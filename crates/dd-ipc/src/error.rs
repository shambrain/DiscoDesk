//! Errors produced by the IPC layer.

use std::fmt;

/// Stable, machine-readable error codes.
///
/// The numeric values are part of the wire contract: they are carried in the
/// [`crate::protocol::ErrorPayload`] and must never be reused for a different
/// meaning. New codes are appended with the next free value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    /// A command was syntactically valid but its arguments failed validation.
    InvalidArgument = 1000,
    /// The peer spoke a different protocol version.
    ProtocolVersionMismatch = 1001,
    /// A frame exceeded the configured maximum size.
    FrameTooLarge = 1002,
    /// The frame header did not match the expected magic bytes.
    BadMagic = 1003,
    /// The frame ended before the declared payload was complete.
    TruncatedFrame = 1004,
    /// Bytes remained after a complete frame had been decoded.
    TrailingBytes = 1005,
    /// The command name is not part of the protocol.
    UnknownCommand = 1006,
    /// The operation requires a capability or permission that is not held.
    PermissionDenied = 1007,
    /// The referenced resource does not exist.
    NotFound = 1008,
    /// The requested operation conflicts with an already-running instance.
    AlreadyRunning = 1009,
    /// The peer closed the connection.
    PeerClosed = 1010,
    /// The frame payload was not valid for the requested command.
    MalformedPayload = 1011,
    /// The engine could not carry out the request for an internal reason.
    Internal = 1012,
    /// The local endpoint could not be created or opened.
    EndpointUnavailable = 1013,
    /// The operation was cancelled by the caller.
    Cancelled = 1014,
}

impl ErrorCode {
    /// The numeric wire value of this code.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    /// Returns the code for a numeric wire value, if it is known.
    #[must_use]
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            1000 => Some(Self::InvalidArgument),
            1001 => Some(Self::ProtocolVersionMismatch),
            1002 => Some(Self::FrameTooLarge),
            1003 => Some(Self::BadMagic),
            1004 => Some(Self::TruncatedFrame),
            1005 => Some(Self::TrailingBytes),
            1006 => Some(Self::UnknownCommand),
            1007 => Some(Self::PermissionDenied),
            1008 => Some(Self::NotFound),
            1009 => Some(Self::AlreadyRunning),
            1010 => Some(Self::PeerClosed),
            1011 => Some(Self::MalformedPayload),
            1012 => Some(Self::Internal),
            1013 => Some(Self::EndpointUnavailable),
            1014 => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::InvalidArgument => "invalid argument",
            Self::ProtocolVersionMismatch => "protocol version mismatch",
            Self::FrameTooLarge => "frame too large",
            Self::BadMagic => "bad frame magic",
            Self::TruncatedFrame => "truncated frame",
            Self::TrailingBytes => "trailing bytes after frame",
            Self::UnknownCommand => "unknown command",
            Self::PermissionDenied => "permission denied",
            Self::NotFound => "not found",
            Self::AlreadyRunning => "already running",
            Self::PeerClosed => "peer closed the connection",
            Self::MalformedPayload => "malformed payload",
            Self::Internal => "internal error",
            Self::EndpointUnavailable => "endpoint unavailable",
            Self::Cancelled => "cancelled",
        };
        f.write_str(text)
    }
}

/// The error type used across the IPC crate.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// The frame header did not match the expected magic bytes.
    #[error("frame magic mismatch")]
    BadMagic {
        /// The magic bytes that were actually read.
        found: [u8; 4],
    },
    /// The peer spoke a different protocol version.
    #[error("protocol version mismatch: peer sent {found}, this build speaks {expected}")]
    ProtocolVersionMismatch {
        /// The version in the received frame.
        found: u16,
        /// The version this build speaks.
        expected: u16,
    },
    /// The declared payload length exceeds the configured maximum.
    #[error("frame payload of {declared} bytes exceeds the {limit} byte limit")]
    FrameTooLarge {
        /// The length declared in the frame header.
        declared: usize,
        /// The configured maximum.
        limit: usize,
    },
    /// The frame ended before the declared payload was complete.
    #[error("frame ended after {read} of {expected} declared bytes")]
    TruncatedFrame {
        /// The number of payload bytes actually read.
        read: usize,
        /// The number of payload bytes the header declared.
        expected: usize,
    },
    /// Bytes remained after a complete frame had been decoded.
    #[error("{count} trailing bytes after a complete frame")]
    TrailingBytes {
        /// The number of leftover bytes.
        count: usize,
    },
    /// The frame could not be parsed at all.
    #[error("malformed frame: {0}")]
    MalformedFrame(String),
    /// The payload was not valid for the requested command.
    #[error("malformed payload: {0}")]
    MalformedPayload(String),
    /// A field failed validation.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// The command name is not part of the protocol.
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    /// The peer closed the connection.
    #[error("peer closed the connection")]
    PeerClosed,
    /// An I/O error occurred on the underlying transport.
    #[error("ipc transport i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The local endpoint could not be created or opened.
    #[error("ipc endpoint unavailable: {0}")]
    EndpointUnavailable(String),
    /// A platform-specific error occurred.
    #[error("ipc platform error: {0}")]
    Platform(String),
}

impl IpcError {
    /// The stable error code for this error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::BadMagic { .. } => ErrorCode::BadMagic,
            Self::ProtocolVersionMismatch { .. } => ErrorCode::ProtocolVersionMismatch,
            Self::FrameTooLarge { .. } => ErrorCode::FrameTooLarge,
            Self::TruncatedFrame { .. } => ErrorCode::TruncatedFrame,
            Self::TrailingBytes { .. } => ErrorCode::TrailingBytes,
            Self::MalformedFrame(_) => ErrorCode::MalformedPayload,
            Self::MalformedPayload(_) => ErrorCode::MalformedPayload,
            Self::InvalidArgument(_) => ErrorCode::InvalidArgument,
            Self::UnknownCommand(_) => ErrorCode::UnknownCommand,
            Self::PeerClosed => ErrorCode::PeerClosed,
            Self::Io(_) => ErrorCode::Internal,
            Self::EndpointUnavailable(_) => ErrorCode::EndpointUnavailable,
            Self::Platform(_) => ErrorCode::Internal,
        }
    }
}

/// Convenience alias for IPC results.
pub type IpcResult<T> = Result<T, IpcError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_round_trip_through_their_wire_value() {
        let codes = [
            ErrorCode::InvalidArgument,
            ErrorCode::ProtocolVersionMismatch,
            ErrorCode::FrameTooLarge,
            ErrorCode::BadMagic,
            ErrorCode::TruncatedFrame,
            ErrorCode::TrailingBytes,
            ErrorCode::UnknownCommand,
            ErrorCode::PermissionDenied,
            ErrorCode::NotFound,
            ErrorCode::AlreadyRunning,
            ErrorCode::Internal,
            ErrorCode::PeerClosed,
            ErrorCode::MalformedPayload,
            ErrorCode::EndpointUnavailable,
            ErrorCode::Cancelled,
        ];
        for code in codes {
            assert_eq!(ErrorCode::from_u16(code.as_u16()), Some(code));
        }
    }

    #[test]
    fn unknown_wire_value_is_not_mapped() {
        assert_eq!(ErrorCode::from_u16(0), None);
        assert_eq!(ErrorCode::from_u16(999), None);
        assert_eq!(ErrorCode::from_u16(65535), None);
    }

    #[test]
    fn error_codes_have_distinct_values() {
        let codes = [
            ErrorCode::InvalidArgument,
            ErrorCode::ProtocolVersionMismatch,
            ErrorCode::FrameTooLarge,
            ErrorCode::BadMagic,
            ErrorCode::TruncatedFrame,
            ErrorCode::TrailingBytes,
            ErrorCode::UnknownCommand,
            ErrorCode::PermissionDenied,
            ErrorCode::NotFound,
            ErrorCode::AlreadyRunning,
            ErrorCode::Internal,
            ErrorCode::PeerClosed,
            ErrorCode::MalformedPayload,
            ErrorCode::EndpointUnavailable,
            ErrorCode::Cancelled,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in codes {
            assert!(seen.insert(code.as_u16()), "duplicate error code {code}");
        }
    }

    #[test]
    fn ipc_errors_map_to_the_expected_codes() {
        assert_eq!(
            IpcError::BadMagic { found: [0; 4] }.code(),
            ErrorCode::BadMagic
        );
        assert_eq!(
            IpcError::ProtocolVersionMismatch {
                found: 2,
                expected: 1
            }
            .code(),
            ErrorCode::ProtocolVersionMismatch
        );
        assert_eq!(
            IpcError::FrameTooLarge {
                declared: 10,
                limit: 5
            }
            .code(),
            ErrorCode::FrameTooLarge
        );
        assert_eq!(
            IpcError::TruncatedFrame {
                read: 1,
                expected: 2
            }
            .code(),
            ErrorCode::TruncatedFrame
        );
        assert_eq!(
            IpcError::TrailingBytes { count: 3 }.code(),
            ErrorCode::TrailingBytes
        );
        assert_eq!(
            IpcError::UnknownCommand("x".to_owned()).code(),
            ErrorCode::UnknownCommand
        );
        assert_eq!(
            IpcError::InvalidArgument("x".to_owned()).code(),
            ErrorCode::InvalidArgument
        );
        assert_eq!(IpcError::PeerClosed.code(), ErrorCode::PeerClosed);
    }

    #[test]
    fn error_codes_render_readable_text() {
        assert_eq!(
            ErrorCode::ProtocolVersionMismatch.to_string(),
            "protocol version mismatch"
        );
        assert!(!ErrorCode::FrameTooLarge.to_string().is_empty());
    }
}
