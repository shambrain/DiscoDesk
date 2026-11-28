//! Capture backends.

use crate::error::AudioError;
use crate::format::AudioFormat;

/// Identity of the endpoint being captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// The stable WASAPI endpoint id.
    pub id: String,
    /// The human-readable friendly name, when one could be read.
    pub name: String,
}

/// What a read attempt produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// Real samples were appended (count is mono samples).
    Samples(usize),
    /// The endpoint reported silence; zeros were appended to keep frame timing.
    Silence(usize),
    /// Nothing arrived within the wait window. Not an error.
    Timeout,
}

/// One open capture stream.
pub trait CaptureSession: Send {
    /// The negotiated format.
    fn format(&self) -> &AudioFormat;
    /// The endpoint being captured.
    fn device(&self) -> &DeviceInfo;
    /// The backend name for telemetry.
    fn backend(&self) -> &'static str;
    /// Blocks up to an internal timeout and appends mono samples to `out`.
    fn read_packet(&mut self, out: &mut Vec<f32>) -> Result<ReadOutcome, AudioError>;
    /// Polls whether the OS default render endpoint has changed identity.
    fn default_device_changed(&mut self) -> Result<bool, AudioError>;
}

#[cfg(windows)]
pub mod wasapi;

#[cfg(windows)]
pub use wasapi::open_default_loopback;

#[cfg(not(windows))]
pub mod unsupported;

#[cfg(not(windows))]
pub use unsupported::open_default_loopback;
