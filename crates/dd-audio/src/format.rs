//! Audio format description and PCM conversion.
//!
//! Kept free of the `windows` crate so it can be tested on any host: the WASAPI
//! layer translates `WAVEFORMATEX` into these types once, at negotiation, and
//! everything downstream works on plain data.
//!
//! # Scope
//!
//! Supported encodings are the ones Windows actually delivers in shared-mode
//! loopback: 32-bit float (the usual case), 16-bit PCM, 32-bit PCM and 8-bit
//! PCM. Anything else is rejected at negotiation with a specific error rather
//! than being silently misinterpreted.

use serde::Serialize;

/// A sample encoding in a capture stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleEncoding {
    /// 32-bit IEEE float.
    F32,
    /// 16-bit signed integer.
    I16,
    /// 32-bit signed integer.
    I32,
    /// 8-bit unsigned integer (silence is 128).
    U8,
}

impl SampleEncoding {
    /// Bytes per sample.
    #[must_use]
    pub const fn bytes_per_sample(self) -> usize {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::I16 => 2,
            Self::U8 => 1,
        }
    }

    /// The stable name used on the wire.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::U8 => "u8",
        }
    }
}

/// Errors from format handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    /// The channel count was zero.
    #[error("channel count must be at least 1")]
    NoChannels,
    /// The sample rate was implausibly low.
    #[error("sample rate {0} Hz is not usable")]
    UnsupportedSampleRate(u32),
    /// The bit depth does not match any supported encoding.
    #[error("unsupported sample encoding: {tag} tag, {bits} bits")]
    UnsupportedEncoding {
        /// The `wFormatTag` value.
        tag: u16,
        /// The `wBitsPerSample` value.
        bits: u16,
    },
    /// `nBlockAlign` disagrees with the channel count and sample size.
    #[error("block align {block_align} does not match {channels} channels of {encoding}")]
    InconsistentBlockAlign {
        /// The declared block alignment.
        block_align: u16,
        /// The declared channel count.
        channels: u16,
        /// The derived encoding.
        encoding: &'static str,
    },
    /// A byte buffer did not contain a whole number of frames.
    #[error("buffer of {len} bytes is not a whole number of {block_align}-byte frames")]
    RaggedBuffer {
        /// Buffer length in bytes.
        len: usize,
        /// Frame size in bytes.
        block_align: usize,
    },
    /// The destination buffer was too small.
    #[error("destination buffer is too small")]
    DestinationTooSmall,
}

/// A negotiated capture format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    /// Samples per second per channel.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// How each sample is stored.
    pub encoding: SampleEncoding,
    /// Bytes per interleaved frame.
    pub block_align: u16,
}

impl AudioFormat {
    /// Builds and validates a format.
    ///
    /// # Errors
    ///
    /// Returns [`FormatError`] if the rate, channel count or block alignment is
    /// not self-consistent.
    pub fn new(
        sample_rate: u32,
        channels: u16,
        encoding: SampleEncoding,
        block_align: u16,
    ) -> Result<Self, FormatError> {
        if channels == 0 {
            return Err(FormatError::NoChannels);
        }
        if !(8_000..=384_000).contains(&sample_rate) {
            return Err(FormatError::UnsupportedSampleRate(sample_rate));
        }
        let expected = u32::from(channels) * encoding.bytes_per_sample() as u32;
        if u32::from(block_align) != expected {
            return Err(FormatError::InconsistentBlockAlign {
                block_align,
                channels,
                encoding: encoding.name(),
            });
        }
        Ok(Self {
            sample_rate,
            channels,
            encoding,
            block_align,
        })
    }

    /// Bytes per interleaved frame.
    #[must_use]
    pub const fn frame_bytes(&self) -> usize {
        self.block_align as usize
    }

    /// Decodes interleaved frames into mono `f32` samples by averaging channels.
    ///
    /// Averaging is a documented simplification: Gate 1 analyses the system mix,
    /// and no channel layout weighting is applied. The source buffer must contain
    /// whole frames.
    ///
    /// # Errors
    ///
    /// Returns [`FormatError::RaggedBuffer`] if `bytes` is not a whole number of
    /// frames.
    pub fn decode_mono(&self, bytes: &[u8], out: &mut Vec<f32>) -> Result<(), FormatError> {
        let frame = self.frame_bytes();
        if frame == 0 {
            return Err(FormatError::NoChannels);
        }
        if !bytes.len().is_multiple_of(frame) {
            return Err(FormatError::RaggedBuffer {
                len: bytes.len(),
                block_align: frame,
            });
        }

        out.reserve(bytes.len() / frame);
        let channels = self.channels as usize;
        let scale = 1.0 / channels as f32;

        for frame_bytes in bytes.chunks_exact(frame) {
            let mut sum = 0.0_f32;
            for channel in 0..channels {
                let offset = channel * self.encoding.bytes_per_sample();
                sum += decode_sample(self.encoding, &frame_bytes[offset..]);
            }
            out.push((sum * scale).clamp(-1.0, 1.0));
        }

        Ok(())
    }

    /// Decodes a single interleaved frame's worth of samples into `out`.
    ///
    /// Used by tests and by callers that need per-channel data; production
    /// analysis uses [`AudioFormat::decode_mono`].
    ///
    /// # Errors
    ///
    /// Returns [`FormatError::DestinationTooSmall`] if `out` cannot hold
    /// `self.channels` samples.
    pub fn decode_frame(&self, frame: &[u8], out: &mut [f32]) -> Result<(), FormatError> {
        if out.len() < self.channels as usize || frame.len() < self.frame_bytes() {
            return Err(FormatError::DestinationTooSmall);
        }
        for (channel, slot) in out.iter_mut().enumerate().take(self.channels as usize) {
            let offset = channel * self.encoding.bytes_per_sample();
            *slot = decode_sample(self.encoding, &frame[offset..]);
        }
        Ok(())
    }
}

/// Decodes one sample. The slice is guaranteed to hold enough bytes by callers.
fn decode_sample(encoding: SampleEncoding, bytes: &[u8]) -> f32 {
    match encoding {
        SampleEncoding::F32 => {
            let raw = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if raw.is_finite() { raw } else { 0.0 }
        }
        SampleEncoding::I16 => {
            let raw = i16::from_le_bytes([bytes[0], bytes[1]]);
            // 32768 keeps -32768 mapping to exactly -1.0.
            f32::from(raw) / 32_768.0
        }
        SampleEncoding::I32 => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            raw as f32 / 2_147_483_648.0
        }
        SampleEncoding::U8 => {
            // Unsigned 8-bit is centred on 128.
            (f32::from(bytes[0]) - 128.0) / 128.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames_i16(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn rejects_empty_channel_counts_and_bad_rates() {
        assert_eq!(
            AudioFormat::new(48_000, 0, SampleEncoding::F32, 0),
            Err(FormatError::NoChannels)
        );
        assert_eq!(
            AudioFormat::new(100, 2, SampleEncoding::F32, 8),
            Err(FormatError::UnsupportedSampleRate(100))
        );
    }

    #[test]
    fn rejects_inconsistent_block_align() {
        assert_eq!(
            AudioFormat::new(48_000, 2, SampleEncoding::F32, 4),
            Err(FormatError::InconsistentBlockAlign {
                block_align: 4,
                channels: 2,
                encoding: "f32",
            })
        );
    }

    #[test]
    fn stereo_float_is_averaged_to_mono() {
        let format = AudioFormat::new(48_000, 2, SampleEncoding::F32, 8).expect("valid");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0.5_f32.to_le_bytes());
        bytes.extend_from_slice(&(-0.5_f32).to_le_bytes());
        bytes.extend_from_slice(&0.25_f32.to_le_bytes());
        bytes.extend_from_slice(&0.75_f32.to_le_bytes());

        let mut out = Vec::new();
        format.decode_mono(&bytes, &mut out).expect("valid");
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sixteen_bit_extremes_map_to_full_scale() {
        let format = AudioFormat::new(44_100, 1, SampleEncoding::I16, 2).expect("valid");
        let bytes = frames_i16(&[0, 16_384, -16_384, 32_767, -32_768]);
        let mut out = Vec::new();
        format.decode_mono(&bytes, &mut out).expect("valid");

        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-3);
        assert!((out[2] + 0.5).abs() < 1e-3);
        assert!((out[3] - 1.0).abs() < 1e-3);
        // -32768 / 32768 is exactly -1.0.
        assert!((out[4] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn unsigned_eight_bit_is_centred() {
        let format = AudioFormat::new(48_000, 1, SampleEncoding::U8, 1).expect("valid");
        let mut out = Vec::new();
        format.decode_mono(&[128, 255, 0], &mut out).expect("valid");
        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - (127.0 / 128.0)).abs() < 1e-6);
        assert!((out[2] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn thirty_two_bit_pcm_scales() {
        let format = AudioFormat::new(48_000, 1, SampleEncoding::I32, 4).expect("valid");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(&1_073_741_824_i32.to_le_bytes()); // 0.25 * 2^32
        let mut out = Vec::new();
        format.decode_mono(&bytes, &mut out).expect("valid");
        assert!((out[0]).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn non_finite_float_samples_become_zero() {
        let format = AudioFormat::new(48_000, 1, SampleEncoding::F32, 4).expect("valid");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&f32::NAN.to_le_bytes());
        bytes.extend_from_slice(&f32::INFINITY.to_le_bytes());
        let mut out = Vec::new();
        format.decode_mono(&bytes, &mut out).expect("valid");
        assert_eq!(out, vec![0.0, 0.0]);
    }

    #[test]
    fn a_ragged_buffer_is_rejected() {
        let format = AudioFormat::new(48_000, 2, SampleEncoding::F32, 8).expect("valid");
        let mut out = Vec::new();
        assert_eq!(
            format.decode_mono(&[0_u8; 5], &mut out),
            Err(FormatError::RaggedBuffer {
                len: 5,
                block_align: 8
            })
        );
    }

    #[test]
    fn an_empty_buffer_yields_no_samples() {
        let format = AudioFormat::new(48_000, 2, SampleEncoding::F32, 8).expect("valid");
        let mut out = Vec::new();
        format.decode_mono(&[], &mut out).expect("valid");
        assert!(out.is_empty());
    }

    #[test]
    fn five_one_channel_layout_averages_all_channels() {
        let format = AudioFormat::new(48_000, 6, SampleEncoding::F32, 24).expect("valid");
        let mut bytes = Vec::new();
        for value in [0.1_f32, 0.1, 0.1, 0.1, 0.1, 0.1] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let mut out = Vec::new();
        format.decode_mono(&bytes, &mut out).expect("valid");
        assert_eq!(out.len(), 1);
        assert!((out[0] - 0.1).abs() < 1e-6);
    }
}
