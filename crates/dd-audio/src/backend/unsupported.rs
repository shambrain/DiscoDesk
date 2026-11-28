//! Non-Windows capture backend.
//!
//! Present so the crate compiles and its pure parts (format conversion, ring,
//! stats) are testable on any host. It never pretends to capture: opening a
//! session returns [`AudioError::UnsupportedPlatform`], and the runtime reports
//! `Unavailable` rather than a stream that silently produces nothing.

use super::{CaptureSession, DeviceInfo, ReadOutcome};
use crate::error::AudioError;
use crate::format::AudioFormat;

/// Always fails: system audio capture is not implemented on this platform.
///
/// # Errors
///
/// Always returns [`AudioError::UnsupportedPlatform`].
pub fn open_default_loopback() -> Result<Box<dyn CaptureSession>, AudioError> {
    Err(AudioError::UnsupportedPlatform)
}

/// A placeholder that can never be constructed, only referenced by type.
#[derive(Debug)]
pub struct UnsupportedSession {
    format: AudioFormat,
    device: DeviceInfo,
}

impl CaptureSession for UnsupportedSession {
    fn format(&self) -> &AudioFormat {
        &self.format
    }

    fn device(&self) -> &DeviceInfo {
        &self.device
    }

    fn backend(&self) -> &'static str {
        "unsupported"
    }

    fn read_packet(&mut self, _out: &mut Vec<f32>) -> Result<ReadOutcome, AudioError> {
        Err(AudioError::UnsupportedPlatform)
    }

    fn default_device_changed(&mut self) -> Result<bool, AudioError> {
        Err(AudioError::UnsupportedPlatform)
    }
}
