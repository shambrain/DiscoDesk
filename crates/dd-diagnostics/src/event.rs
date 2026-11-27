//! Privacy-preserving diagnostic events and the event sink abstraction.
//!
//! # Privacy contract
//!
//! This module is the *only* sanctioned way for engine crates to describe
//! something that happened. It exists to make privacy violations structurally
//! difficult:
//!
//! * No raw PCM samples are representable. There is no type for them.
//! * No image bytes are representable. There is no type for them.
//! * File system paths are represented by [`RedactedPath`], which keeps only the
//!   file name, the extension and a non-reversible digest of the *full* path.
//!   The full path is never recoverable from the digest.
//!
//! [`redact_path`] is the single entry point that produces a [`RedactedPath`]
//! and is unit-tested against absolute paths, UNC paths, relative paths and
//! degenerate inputs.

use std::fmt;
use std::path::Path;

/// Maximum number of characters kept from a file name before truncation.
///
/// File names are user-controlled and may themselves be sensitive, so the
/// redaction keeps only a bounded prefix.
pub const MAX_REDACTED_NAME_LEN: usize = 48;

/// Maximum number of characters kept from a path fragment that is not a file
/// name (for example a UNC share name).
pub const MAX_REDACTED_SEGMENT_LEN: usize = 24;

/// A path that has been stripped of its privacy-sensitive parent components.
///
/// The retained information is:
///
/// * the file name (truncated),
/// * the extension (truncated),
/// * a digest of the *whole* original path, so that two different paths can be
///   told apart without revealing either of them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RedactedPath {
    file_name: String,
    extension: Option<String>,
    digest: u64,
    is_absolute: bool,
}

impl RedactedPath {
    /// Returns the (possibly truncated) file name.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the (possibly truncated) extension, if the file had one.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Returns the digest of the original path.
    ///
    /// This is a 64-bit FNV-1a digest. It is a *discriminator*, not a
    /// cryptographic commitment: it is used to tell two paths apart, and it is
    /// explicitly not intended to resist a targeted brute-force search over a
    /// known set of candidate paths.
    #[must_use]
    pub fn digest(&self) -> u64 {
        self.digest
    }

    /// Whether the original path was absolute.
    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }
}

impl fmt::Display for RedactedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{:016x}]", self.file_name, self.digest)?;
        if let Some(ext) = &self.extension {
            write!(f, ".{ext}")?;
        }
        Ok(())
    }
}

/// Truncates a string to `max` characters, appending an ellipsis marker when
/// the string was shortened.
fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// FNV-1a 64-bit digest. Small, dependency-free and deterministic.
///
/// It is deliberately not a cryptographic hash: its only job is to give a
/// stable discriminator for a path that must not be logged.
fn digest_bytes(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Converts a path into its privacy-preserving representation.
///
/// The parent directories are dropped entirely. What remains is the file name,
/// the extension and a digest of the complete path.
///
/// ```
/// use dd_diagnostics::event::redact_path;
///
/// let redacted = redact_path(r"C:\Users\Someone\Pictures\secret.png");
/// assert_eq!(redacted.file_name(), "secret.png");
/// assert_eq!(redacted.extension(), Some("png"));
/// // The parent path is not present in any form.
/// assert!(!format!("{redacted}").contains("Someone"));
/// ```
#[must_use]
pub fn redact_path(path: impl AsRef<Path>) -> RedactedPath {
    let path = path.as_ref();

    // Digest the raw OS string so that two different paths with the same
    // file name remain distinguishable.
    let raw = path.to_string_lossy();
    let digest = digest_bytes(raw.as_bytes());

    let file_name = path
        .file_name()
        .map(|name| truncate(&name.to_string_lossy(), MAX_REDACTED_NAME_LEN))
        .unwrap_or_else(|| "<no-name>".to_owned());

    let extension = path
        .extension()
        .map(|ext| truncate(&ext.to_string_lossy(), MAX_REDACTED_SEGMENT_LEN));

    RedactedPath {
        file_name,
        extension,
        digest,
        is_absolute: path.is_absolute(),
    }
}

/// A structured, privacy-safe description of something that happened.
///
/// The variants are intentionally narrow. There is deliberately no variant that
/// could carry raw PCM, image bytes or an unredacted path.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticEvent {
    /// The engine state machine moved from one state to another.
    EngineStateChanged {
        /// Previous state name.
        from: &'static str,
        /// New state name.
        to: &'static str,
    },
    /// The engine started.
    EngineStarted,
    /// The engine stopped.
    EngineStopped,
    /// An IPC endpoint was created.
    EndpointCreated {
        /// The endpoint path. This is a local, per-user IPC name, not a user
        /// document path, so it is logged verbatim.
        endpoint: String,
    },
    /// An IPC frame was received.
    IpcFrameReceived {
        /// The command name.
        command: &'static str,
        /// Payload size in bytes.
        payload_bytes: usize,
    },
    /// An IPC frame was rejected.
    IpcFrameRejected {
        /// Stable machine-readable reason.
        reason: &'static str,
    },
    /// A recoverable error occurred.
    RecoverableError {
        /// Stable machine-readable reason.
        reason: &'static str,
        /// Optional privacy-safe context.
        context: Option<RedactedPath>,
    },
    /// A file-system-adjacent event. Only the redacted path is retained.
    FileObserved {
        /// What happened to the file.
        action: FileAction,
        /// The redacted path.
        path: RedactedPath,
    },
    /// A free-form note that carries no user data.
    Note {
        /// A short, fixed-vocabulary tag.
        tag: &'static str,
    },
}

/// What happened to an observed file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileAction {
    /// The file was read.
    Read,
    /// The file was written.
    Written,
    /// The file was deleted.
    Deleted,
}

/// The sink that receives [`DiagnosticEvent`]s.
///
/// Implementations must not block for long and must not perform unbounded
/// work; the engine calls this from state-transition paths.
pub trait EventSink: Send + Sync {
    /// Records one event.
    fn emit(&self, event: &DiagnosticEvent);

    /// Records one event together with a human-readable detail string.
    ///
    /// The detail string must itself be privacy-safe: callers are responsible
    /// for redacting anything that could identify a user file.
    fn emit_with_detail(&self, event: &DiagnosticEvent, detail: &str) {
        self.emit(event);
        let _ = detail;
    }
}

/// An event sink that discards everything.
///
/// Useful in tests and in contexts where diagnostics must be fully disabled.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn emit(&self, _event: &DiagnosticEvent) {}
}

/// An event sink that forwards events to `tracing`.
///
/// This is the default sink used by the engine binary. It converts the event
/// into a structured `tracing` event so that the rotating log sink, the
/// console sink and any future sink all observe the same data.
#[derive(Clone, Copy, Debug, Default)]
pub struct TracingEventSink;

impl EventSink for TracingEventSink {
    fn emit(&self, event: &DiagnosticEvent) {
        match event {
            DiagnosticEvent::EngineStateChanged { from, to } => {
                tracing::info!(from = *from, to = *to, "engine state changed");
            }
            DiagnosticEvent::EngineStarted => {
                tracing::info!("engine started");
            }
            DiagnosticEvent::EngineStopped => {
                tracing::info!("engine stopped");
            }
            DiagnosticEvent::EndpointCreated { endpoint } => {
                tracing::info!(endpoint = %endpoint, "ipc endpoint created");
            }
            DiagnosticEvent::IpcFrameReceived {
                command,
                payload_bytes,
            } => {
                tracing::debug!(command = *command, payload_bytes, "ipc frame received");
            }
            DiagnosticEvent::IpcFrameRejected { reason } => {
                tracing::warn!(reason = *reason, "ipc frame rejected");
            }
            DiagnosticEvent::RecoverableError { reason, context } => match context {
                Some(path) => {
                    tracing::warn!(reason = *reason, path = %path, "recoverable error");
                }
                None => {
                    tracing::warn!(reason = *reason, "recoverable error");
                }
            },
            DiagnosticEvent::FileObserved { action, path } => {
                tracing::debug!(action = ?action, path = %path, "file observed");
            }
            DiagnosticEvent::Note { tag } => {
                tracing::debug!(tag = *tag, "note");
            }
        }
    }
}

/// A sink that records events into a bounded, in-memory ring buffer.
///
/// This is intended for tests and for the `GetDiagnosticsSnapshot` command
/// handler. The buffer is bounded so that a long-running engine cannot grow
/// without limit.
#[derive(Debug)]
pub struct BoundedEventBuffer {
    events: std::sync::Mutex<Vec<DiagnosticEvent>>,
    capacity: usize,
}

impl BoundedEventBuffer {
    /// Creates a buffer that keeps at most `capacity` events.
    ///
    /// A capacity of zero is allowed and yields a buffer that stores nothing.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
            capacity,
        }
    }

    /// Returns a snapshot of the retained events, oldest first.
    ///
    /// The lock is held only for the duration of the copy.
    #[must_use]
    pub fn snapshot(&self) -> Vec<DiagnosticEvent> {
        match self.events.lock() {
            Ok(events) => events.clone(),
            // A poisoned mutex means a previous holder panicked while holding
            // it. Returning an empty snapshot is safer than propagating a
            // panic into a runtime path.
            Err(_) => Vec::new(),
        }
    }
}

impl EventSink for BoundedEventBuffer {
    fn emit(&self, event: &DiagnosticEvent) {
        let Ok(mut events) = self.events.lock() else {
            return;
        };
        if self.capacity == 0 {
            return;
        }
        if events.len() >= self.capacity {
            // Drop the oldest event to make room. This keeps the buffer bounded
            // without unbounded growth.
            events.remove(0);
        }
        events.push(event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_parent_directories_from_windows_path() {
        let redacted = redact_path(r"C:\Users\Someone\Pictures\holiday.png");
        assert_eq!(redacted.file_name(), "holiday.png");
        assert_eq!(redacted.extension(), Some("png"));
        assert!(redacted.is_absolute());
        let rendered = redacted.to_string();
        assert!(
            !rendered.contains("Someone"),
            "redacted path leaked the user directory: {rendered}"
        );
        assert!(
            !rendered.contains("Pictures"),
            "redacted path leaked the parent directory: {rendered}"
        );
        assert!(
            !rendered.contains("C:\\"),
            "redacted path leaked the drive root: {rendered}"
        );
    }

    #[test]
    fn redacts_parent_directories_from_unc_path() {
        let redacted = redact_path(r"\\server\share\private\doc.pdf");
        assert_eq!(redacted.file_name(), "doc.pdf");
        let rendered = redacted.to_string();
        assert!(!rendered.contains("server"), "UNC host leaked: {rendered}");
        assert!(!rendered.contains("share"), "UNC share leaked: {rendered}");
    }

    #[test]
    fn redacts_relative_path() {
        let redacted = redact_path("images/wallpaper.jpg");
        assert_eq!(redacted.file_name(), "wallpaper.jpg");
        assert!(!redacted.is_absolute());
    }

    #[test]
    fn distinct_paths_produce_distinct_digests() {
        let a = redact_path(r"C:\a\one.png");
        let b = redact_path(r"C:\b\one.png");
        assert_eq!(a.file_name(), b.file_name());
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn identical_paths_produce_identical_digests() {
        let a = redact_path(r"C:\a\one.png");
        let b = redact_path(r"C:\a\one.png");
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn long_file_names_are_truncated() {
        let long_name = "x".repeat(200);
        let redacted = redact_path(format!(r"C:\dir\{long_name}.png"));
        assert!(
            redacted.file_name().chars().count() <= MAX_REDACTED_NAME_LEN,
            "file name was not truncated"
        );
    }

    #[test]
    fn path_without_file_name_is_handled() {
        let redacted = redact_path(r"C:\");
        assert_eq!(redacted.file_name(), "<no-name>");
        assert_eq!(redacted.extension(), None);
    }

    #[test]
    fn empty_path_is_handled() {
        let redacted = redact_path("");
        assert_eq!(redacted.file_name(), "<no-name>");
    }

    #[test]
    fn bounded_buffer_drops_oldest_events() {
        let buffer = BoundedEventBuffer::new(2);
        buffer.emit(&DiagnosticEvent::Note { tag: "a" });
        buffer.emit(&DiagnosticEvent::Note { tag: "b" });
        buffer.emit(&DiagnosticEvent::Note { tag: "c" });
        let events = buffer.snapshot();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            DiagnosticEvent::Note { tag: "b" },
            "oldest event was not dropped"
        );
        assert_eq!(events[1], DiagnosticEvent::Note { tag: "c" });
    }

    #[test]
    fn zero_capacity_buffer_stores_nothing() {
        let buffer = BoundedEventBuffer::new(0);
        buffer.emit(&DiagnosticEvent::EngineStarted);
        assert!(buffer.snapshot().is_empty());
    }

    #[test]
    fn null_sink_accepts_events() {
        NullEventSink.emit(&DiagnosticEvent::EngineStarted);
    }

    #[test]
    fn tracing_sink_accepts_events() {
        TracingEventSink.emit(&DiagnosticEvent::EngineStarted);
        TracingEventSink.emit(&DiagnosticEvent::EngineStateChanged {
            from: "Stopped",
            to: "Starting",
        });
        TracingEventSink.emit(&DiagnosticEvent::EndpointCreated {
            endpoint: r"\\.\pipe\discodesk-test".to_owned(),
        });
        TracingEventSink.emit(&DiagnosticEvent::IpcFrameReceived {
            command: "GetEngineStatus",
            payload_bytes: 12,
        });
        TracingEventSink.emit(&DiagnosticEvent::IpcFrameRejected {
            reason: "bad-magic",
        });
        TracingEventSink.emit(&DiagnosticEvent::RecoverableError {
            reason: "device-lost",
            context: Some(redact_path(r"C:\x\y.png")),
        });
        TracingEventSink.emit(&DiagnosticEvent::FileObserved {
            action: FileAction::Read,
            path: redact_path(r"C:\x\y.png"),
        });
    }
}
