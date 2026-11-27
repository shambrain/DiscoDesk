//! The engine error taxonomy.
//!
//! Every error the engine can report maps to a stable [`ErrorCode`]. The code
//! is what travels over IPC; the message is for humans and must never carry
//! user data (no absolute paths, no raw PCM, no image contents).

use crate::state::EngineState;

/// A stable, machine-readable error code.
///
/// The numeric values are part of the IPC contract. They must never be reused
/// for a different meaning; new codes are appended with the next free value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    /// A state machine transition was attempted that the machine forbids.
    InvalidTransition = 1000,
    /// The peer spoke a different protocol version.
    ProtocolVersionMismatch = 1001,
    /// A frame or payload exceeded the configured maximum size.
    FrameTooLarge = 1002,
    /// The command name is not part of the protocol.
    UnknownCommand = 1003,
    /// A required capability or OS permission is not available.
    PermissionDenied = 1004,
    /// The referenced resource does not exist.
    NotFound = 1005,
    /// Another instance is already running.
    AlreadyRunning = 1006,
    /// The engine failed for an internal reason.
    Internal = 1007,
    /// The requested capability exists but is not implemented yet.
    ///
    /// This is distinct from `Internal`: it tells the caller that the
    /// behaviour is a known, planned feature rather than a defect.
    NotImplemented = 1008,
    /// The engine is in a state that cannot serve the request.
    InvalidState = 1009,
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
            1000 => Some(Self::InvalidTransition),
            1001 => Some(Self::ProtocolVersionMismatch),
            1002 => Some(Self::FrameTooLarge),
            1003 => Some(Self::UnknownCommand),
            1004 => Some(Self::PermissionDenied),
            1005 => Some(Self::NotFound),
            1006 => Some(Self::AlreadyRunning),
            1007 => Some(Self::Internal),
            1008 => Some(Self::NotImplemented),
            1009 => Some(Self::InvalidState),
            _ => None,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::InvalidTransition => "invalid state transition",
            Self::ProtocolVersionMismatch => "protocol version mismatch",
            Self::FrameTooLarge => "frame too large",
            Self::UnknownCommand => "unknown command",
            Self::PermissionDenied => "permission denied",
            Self::NotFound => "not found",
            Self::AlreadyRunning => "already running",
            Self::Internal => "internal error",
            Self::NotImplemented => "not implemented",
            Self::InvalidState => "invalid engine state",
        };
        f.write_str(text)
    }
}

/// The engine's error type.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// A state machine transition was refused.
    #[error("illegal transition from {from} to {to}")]
    InvalidTransition {
        /// The state the engine is in.
        from: EngineState,
        /// The state that was requested.
        to: EngineState,
    },
    /// The peer spoke a different protocol version.
    #[error("protocol version mismatch: peer sent {found}, this build speaks {expected}")]
    ProtocolVersionMismatch {
        /// The version the peer sent.
        found: u16,
        /// The version this build speaks.
        expected: u16,
    },
    /// A frame or payload exceeded the configured maximum size.
    #[error("frame of {size} bytes exceeds the {limit} byte limit")]
    FrameTooLarge {
        /// The observed size.
        size: usize,
        /// The configured maximum.
        limit: usize,
    },
    /// The command name is not part of the protocol.
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    /// A required capability or OS permission is not available.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// The referenced resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Another instance is already running.
    #[error("already running: {0}")]
    AlreadyRunning(String),
    /// The engine failed for an internal reason.
    #[error("internal error: {0}")]
    Internal(String),
    /// The requested capability exists but is not implemented yet.
    #[error("not implemented: {0}")]
    NotImplemented(String),
    /// The engine is in a state that cannot serve the request.
    #[error("invalid engine state {state} for this request: {reason}")]
    InvalidState {
        /// The state the engine is in.
        state: EngineState,
        /// Why the request could not be served.
        reason: String,
    },
}

impl EngineError {
    /// The stable error code for this error.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidTransition { .. } => ErrorCode::InvalidTransition,
            Self::ProtocolVersionMismatch { .. } => ErrorCode::ProtocolVersionMismatch,
            Self::FrameTooLarge { .. } => ErrorCode::FrameTooLarge,
            Self::UnknownCommand(_) => ErrorCode::UnknownCommand,
            Self::PermissionDenied(_) => ErrorCode::PermissionDenied,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::AlreadyRunning(_) => ErrorCode::AlreadyRunning,
            Self::Internal(_) => ErrorCode::Internal,
            Self::NotImplemented(_) => ErrorCode::NotImplemented,
            Self::InvalidState { .. } => ErrorCode::InvalidState,
        }
    }
}

/// Convenience alias for engine results.
pub type EngineResult<T> = Result<T, EngineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_round_trip() {
        let codes = [
            ErrorCode::InvalidTransition,
            ErrorCode::ProtocolVersionMismatch,
            ErrorCode::FrameTooLarge,
            ErrorCode::UnknownCommand,
            ErrorCode::PermissionDenied,
            ErrorCode::NotFound,
            ErrorCode::AlreadyRunning,
            ErrorCode::Internal,
            ErrorCode::NotImplemented,
            ErrorCode::InvalidState,
        ];
        for code in codes {
            assert_eq!(ErrorCode::from_u16(code.as_u16()), Some(code));
        }
    }

    #[test]
    fn error_codes_have_distinct_values() {
        let codes = [
            ErrorCode::InvalidTransition,
            ErrorCode::ProtocolVersionMismatch,
            ErrorCode::FrameTooLarge,
            ErrorCode::UnknownCommand,
            ErrorCode::PermissionDenied,
            ErrorCode::NotFound,
            ErrorCode::AlreadyRunning,
            ErrorCode::Internal,
            ErrorCode::NotImplemented,
            ErrorCode::InvalidState,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in codes {
            assert!(seen.insert(code.as_u16()), "duplicate error code {code}");
        }
    }

    #[test]
    fn unknown_wire_values_are_not_mapped() {
        assert_eq!(ErrorCode::from_u16(0), None);
        assert_eq!(ErrorCode::from_u16(999), None);
    }

    #[test]
    fn engine_errors_map_to_the_expected_codes() {
        let cases: Vec<(EngineError, ErrorCode)> = vec![
            (
                EngineError::InvalidTransition {
                    from: EngineState::Stopped,
                    to: EngineState::Rendering,
                },
                ErrorCode::InvalidTransition,
            ),
            (
                EngineError::ProtocolVersionMismatch {
                    found: 2,
                    expected: 1,
                },
                ErrorCode::ProtocolVersionMismatch,
            ),
            (
                EngineError::FrameTooLarge { size: 10, limit: 5 },
                ErrorCode::FrameTooLarge,
            ),
            (
                EngineError::UnknownCommand("x".to_owned()),
                ErrorCode::UnknownCommand,
            ),
            (
                EngineError::PermissionDenied("x".to_owned()),
                ErrorCode::PermissionDenied,
            ),
            (EngineError::NotFound("x".to_owned()), ErrorCode::NotFound),
            (
                EngineError::AlreadyRunning("x".to_owned()),
                ErrorCode::AlreadyRunning,
            ),
            (EngineError::Internal("x".to_owned()), ErrorCode::Internal),
            (
                EngineError::NotImplemented("x".to_owned()),
                ErrorCode::NotImplemented,
            ),
            (
                EngineError::InvalidState {
                    state: EngineState::Stopped,
                    reason: "x".to_owned(),
                },
                ErrorCode::InvalidState,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.code(), expected, "wrong code for {error}");
        }
    }

    #[test]
    fn error_messages_render() {
        let error = EngineError::InvalidTransition {
            from: EngineState::Stopped,
            to: EngineState::Rendering,
        };
        assert!(error.to_string().contains("Stopped"));
        assert!(error.to_string().contains("Rendering"));
    }
}
