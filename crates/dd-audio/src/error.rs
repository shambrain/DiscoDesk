//! Audio capture errors.
//!
//! Messages are built from descriptions, never from OS strings that could embed
//! a device path or a user name. The numeric categories exist so telemetry and
//! the Studio can distinguish "no device" from "device went away" from "this
//! build does not support capture".

/// A capture failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioError {
    /// System audio capture is not implemented on this platform.
    #[error("system audio capture is not implemented on this platform")]
    UnsupportedPlatform,
    /// No default render endpoint could be found.
    #[error("no default output device is available")]
    NoDefaultDevice,
    /// The endpoint's format is not one this build can decode.
    #[error("unsupported endpoint format: {0}")]
    UnsupportedFormat(String),
    /// COM or device activation failed.
    #[error("could not activate the audio endpoint: {0}")]
    Activation(String),
    /// Stream initialisation failed.
    #[error("could not initialise the capture stream: {0}")]
    Initialization(String),
    /// Reading from the capture client failed.
    #[error("capture read failed: {0}")]
    Capture(String),
    /// The endpoint was invalidated (device removed, driver reset, service
    /// restart). Recovery is expected; this is not a fatal condition.
    #[error("the audio endpoint was invalidated")]
    DeviceInvalidated,
    /// The runtime was asked to stop.
    #[error("capture was stopped")]
    Stopped,
}

impl AudioError {
    /// Whether the runtime should attempt to re-establish capture.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::DeviceInvalidated
                | Self::NoDefaultDevice
                | Self::Capture(_)
                | Self::Activation(_)
                | Self::Initialization(_)
        )
    }

    /// A stable short category for telemetry.
    #[must_use]
    pub const fn category(&self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::NoDefaultDevice => "no_default_device",
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::Activation(_) => "activation_failed",
            Self::Initialization(_) => "initialization_failed",
            Self::Capture(_) => "capture_failed",
            Self::DeviceInvalidated => "device_invalidated",
            Self::Stopped => "stopped",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoverability_matches_the_documented_set() {
        assert!(AudioError::DeviceInvalidated.is_recoverable());
        assert!(AudioError::NoDefaultDevice.is_recoverable());
        assert!(!AudioError::UnsupportedPlatform.is_recoverable());
        assert!(!AudioError::UnsupportedFormat("x".into()).is_recoverable());
        assert!(!AudioError::Stopped.is_recoverable());
    }

    #[test]
    fn categories_are_stable_strings() {
        assert_eq!(
            AudioError::DeviceInvalidated.category(),
            "device_invalidated"
        );
        assert_eq!(AudioError::NoDefaultDevice.category(), "no_default_device");
    }
}
