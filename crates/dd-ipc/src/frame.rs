//! Length-delimited frame codec.
//!
//! # Wire format
//!
//! ```text
//! +--------+--------+-------------------+---------------------+
//! | magic  | version| length (u32 LE)   | payload (length B)  |
//! | 4 B    | 2 B LE | 4 B LE            |                     |
//! +--------+--------+-------------------+---------------------+
//! ```
//!
//! * `magic` — the four ASCII bytes `D`, `D`, `F`, `1` (`b"DDF1"`).
//! * `version` — the protocol version, `u16` little-endian.
//! * `length` — the payload length in bytes, `u32` little-endian.
//! * `payload` — exactly `length` bytes.
//!
//! # Safety properties
//!
//! * The declared length is validated against [`MAX_FRAME_BYTES`] *before* any
//!   buffer is allocated, so a hostile peer cannot cause a large allocation.
//! * Decoding reads the header first and only then allocates the payload
//!   buffer, sized from the already-validated length.
//! * Trailing bytes after a complete frame are reported, not silently ignored.
//!
//! The codec is hand-rolled rather than derived from `serde` on purpose: the
//! header is a fixed 10 bytes and the payload is an opaque byte string that
//! the envelope layer interprets. Keeping the header out of the serializer
//! avoids a JSON round-trip in the hot path and keeps the allocation count
//! predictable.

use crate::error::{IpcError, IpcResult};

/// The four magic bytes that begin every frame: `b"DDF1"`.
pub const FRAME_MAGIC: [u8; 4] = *b"DDF1";

/// Size of the fixed frame header in bytes.
pub const FRAME_HEADER_BYTES: usize = 10;

/// Maximum accepted payload size in bytes (1 MiB).
///
/// A frame whose header declares a larger payload is rejected before any
/// allocation happens.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// A decoded frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The protocol version the sender speaks.
    pub version: u16,
    /// The payload bytes.
    pub payload: Vec<u8>,
}

/// Encodes a payload into a complete frame.
///
/// Returns [`IpcError::FrameTooLarge`] if `payload` exceeds
/// [`MAX_FRAME_BYTES`]. The check happens before the frame buffer is grown.
pub fn encode(version: u16, payload: &[u8]) -> IpcResult<Vec<u8>> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(IpcError::FrameTooLarge {
            declared: payload.len(),
            limit: MAX_FRAME_BYTES,
        });
    }

    let length = u32::try_from(payload.len()).map_err(|_| IpcError::FrameTooLarge {
        declared: payload.len(),
        limit: MAX_FRAME_BYTES,
    })?;

    let mut buffer = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
    buffer.extend_from_slice(&FRAME_MAGIC);
    buffer.extend_from_slice(&version.to_le_bytes());
    buffer.extend_from_slice(&length.to_le_bytes());
    buffer.extend_from_slice(payload);
    Ok(buffer)
}

/// Decodes exactly one frame from the start of `bytes`.
///
/// * [`IpcError::BadMagic`] — the header does not start with [`FRAME_MAGIC`].
/// * [`IpcError::ProtocolVersionMismatch`] — the frame version differs from
///   `expected_version`.
/// * [`IpcError::FrameTooLarge`] — the declared length exceeds
///   [`MAX_FRAME_BYTES`]. This is checked before allocation.
/// * [`IpcError::TruncatedFrame`] — the buffer ends before the declared payload
///   is complete.
/// * [`IpcError::TrailingBytes`] — bytes remain after the frame.
///
/// On success the returned frame and the number of bytes consumed are given.
pub fn decode(bytes: &[u8], expected_version: u16) -> IpcResult<(Frame, usize)> {
    if bytes.len() < FRAME_HEADER_BYTES {
        return Err(IpcError::TruncatedFrame {
            read: bytes.len(),
            expected: FRAME_HEADER_BYTES,
        });
    }

    let mut magic = [0_u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != FRAME_MAGIC {
        return Err(IpcError::BadMagic { found: magic });
    }

    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != expected_version {
        return Err(IpcError::ProtocolVersionMismatch {
            found: version,
            expected: expected_version,
        });
    }

    let length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(IpcError::FrameTooLarge {
            declared: length,
            limit: MAX_FRAME_BYTES,
        });
    }

    let end = FRAME_HEADER_BYTES + length;
    if bytes.len() < end {
        return Err(IpcError::TruncatedFrame {
            read: bytes.len().saturating_sub(FRAME_HEADER_BYTES),
            expected: length,
        });
    }

    let payload = bytes[FRAME_HEADER_BYTES..end].to_vec();

    let consumed = end;
    if bytes.len() > consumed {
        return Err(IpcError::TrailingBytes {
            count: bytes.len() - consumed,
        });
    }

    Ok((Frame { version, payload }, consumed))
}

/// Returns the declared payload length of the frame that starts at `bytes[0]`.
///
/// This performs only header validation: magic, version and the size limit. It
/// never allocates and never reads the payload, so a caller can size a read
/// buffer safely.
pub fn peek_payload_len(bytes: &[u8], expected_version: u16) -> IpcResult<usize> {
    if bytes.len() < FRAME_HEADER_BYTES {
        return Err(IpcError::TruncatedFrame {
            read: bytes.len(),
            expected: FRAME_HEADER_BYTES,
        });
    }

    let mut magic = [0_u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != FRAME_MAGIC {
        return Err(IpcError::BadMagic { found: magic });
    }

    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != expected_version {
        return Err(IpcError::ProtocolVersionMismatch {
            found: version,
            expected: expected_version,
        });
    }

    let length = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(IpcError::FrameTooLarge {
            declared: length,
            limit: MAX_FRAME_BYTES,
        });
    }

    Ok(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_an_empty_payload() {
        let encoded = encode(1, &[]).expect("encode should succeed");
        let (frame, consumed) = decode(&encoded, 1).expect("decode should succeed");
        assert_eq!(frame.version, 1);
        assert!(frame.payload.is_empty());
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn round_trips_a_payload() {
        let payload = b"{\"hello\":\"world\"}";
        let encoded = encode(1, payload).expect("encode should succeed");
        let (frame, consumed) = decode(&encoded, 1).expect("decode should succeed");
        assert_eq!(frame.payload, payload);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn header_has_the_documented_size() {
        let encoded = encode(1, &[1, 2, 3]).expect("encode should succeed");
        assert_eq!(encoded.len(), FRAME_HEADER_BYTES + 3);
        assert_eq!(&encoded[0..4], &FRAME_MAGIC);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut encoded = encode(1, b"payload").expect("encode should succeed");
        encoded[0] = b'X';
        match decode(&encoded, 1) {
            Err(IpcError::BadMagic { found }) => assert_ne!(found, FRAME_MAGIC),
            other => panic!("expected BadMagic, got {other:?}"),
        }
    }

    #[test]
    fn rejects_version_mismatch() {
        let encoded = encode(2, b"payload").expect("encode should succeed");
        match decode(&encoded, 1) {
            Err(IpcError::ProtocolVersionMismatch { found, expected }) => {
                assert_eq!(found, 2);
                assert_eq!(expected, 1);
            }
            other => panic!("expected ProtocolVersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn rejects_oversized_declared_length_before_allocating() {
        // Craft a header that claims a huge payload but supplies none.
        let mut header = Vec::new();
        header.extend_from_slice(&FRAME_MAGIC);
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        match decode(&header, 1) {
            Err(IpcError::FrameTooLarge { declared, limit }) => {
                assert_eq!(declared, u32::MAX as usize);
                assert_eq!(limit, MAX_FRAME_BYTES);
            }
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn encode_rejects_an_oversized_payload() {
        let payload = vec![0_u8; MAX_FRAME_BYTES + 1];
        match encode(1, &payload) {
            Err(IpcError::FrameTooLarge { declared, limit }) => {
                assert_eq!(declared, MAX_FRAME_BYTES + 1);
                assert_eq!(limit, MAX_FRAME_BYTES);
            }
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn accepts_a_payload_exactly_at_the_limit() {
        let payload = vec![7_u8; MAX_FRAME_BYTES];
        let encoded = encode(1, &payload).expect("encode should succeed");
        let (frame, _) = decode(&encoded, 1).expect("decode should succeed");
        assert_eq!(frame.payload.len(), MAX_FRAME_BYTES);
    }

    #[test]
    fn rejects_truncated_frames() {
        let encoded = encode(1, b"0123456789").expect("encode should succeed");
        let truncated = &encoded[..encoded.len() - 4];
        match decode(truncated, 1) {
            Err(IpcError::TruncatedFrame { read, expected }) => {
                assert_eq!(read, 6);
                assert_eq!(expected, 10);
            }
            other => panic!("expected TruncatedFrame, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_truncated_header() {
        match decode(&[0_u8; 4], 1) {
            Err(IpcError::TruncatedFrame { read, expected }) => {
                assert_eq!(read, 4);
                assert_eq!(expected, FRAME_HEADER_BYTES);
            }
            other => panic!("expected TruncatedFrame, got {other:?}"),
        }
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut encoded = encode(1, b"payload").expect("encode should succeed");
        encoded.extend_from_slice(b"extra");
        match decode(&encoded, 1) {
            Err(IpcError::TrailingBytes { count }) => assert_eq!(count, 5),
            other => panic!("expected TrailingBytes, got {other:?}"),
        }
    }

    #[test]
    fn peek_reports_the_declared_length_without_the_payload() {
        let encoded = encode(1, b"0123456789").expect("encode should succeed");
        assert_eq!(
            peek_payload_len(&encoded, 1).expect("peek should succeed"),
            10
        );
    }

    #[test]
    fn peek_rejects_an_oversized_length() {
        let mut header = Vec::new();
        header.extend_from_slice(&FRAME_MAGIC);
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&(MAX_FRAME_BYTES as u32 + 1).to_le_bytes());
        assert!(matches!(
            peek_payload_len(&header, 1),
            Err(IpcError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn peek_rejects_a_version_mismatch() {
        let encoded = encode(7, b"x").expect("encode should succeed");
        assert!(matches!(
            peek_payload_len(&encoded, 1),
            Err(IpcError::ProtocolVersionMismatch { .. })
        ));
    }

    #[test]
    fn encoded_frames_start_with_the_magic_bytes() {
        let encoded = encode(1, b"x").expect("encode should succeed");
        assert_eq!(&encoded[0..4], &FRAME_MAGIC);
    }
}
