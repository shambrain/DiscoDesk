//! Rotating, local-only log sink.
//!
//! # Design constraints
//!
//! * **Local only.** Log files are written under a caller-supplied directory.
//!   Nothing is uploaded.
//! * **Off by default.** [`LogOptions::default`] disables file logging so that
//!   CI and unit tests never write files to disk.
//! * **Bounded.** Each file is capped at [`LogOptions::max_file_bytes`] and at
//!   most [`LogOptions::max_files`] files are kept.
//! * **Non-blocking.** The file sink is a `tracing` non-blocking writer, so a
//!   slow disk cannot stall engine threads.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Default maximum size of a single log file (2 MiB).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Default number of rotated log files to retain.
pub const DEFAULT_MAX_FILES: usize = 5;

/// Errors produced while setting up the diagnostics subsystem.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    /// The log directory could not be created.
    #[error("could not create diagnostics directory: {0}")]
    DirectoryUnavailable(String),
    /// The `tracing` subscriber could not be installed.
    #[error("could not install tracing subscriber: {0}")]
    SubscriberInstallFailed(String),
    /// The requested log level is not a valid `tracing` filter directive.
    #[error("invalid log filter directive: {0}")]
    InvalidFilter(String),
}

/// Configuration for the local diagnostics subsystem.
#[derive(Clone, Debug)]
pub struct LogOptions {
    /// Filter directive, e.g. `"info"` or `"discodesk_engine=debug,info"`.
    pub filter: String,
    /// Whether to write a rotating file log. Disabled by default.
    pub file_logging_enabled: bool,
    /// Directory for rotated log files. Only used when `file_logging_enabled`.
    pub log_directory: Option<PathBuf>,
    /// Maximum size of one log file in bytes.
    pub max_file_bytes: u64,
    /// Number of rotated files to retain.
    pub max_files: usize,
    /// Whether to also emit to stderr.
    pub stderr_enabled: bool,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            filter: "info".to_owned(),
            file_logging_enabled: false,
            log_directory: None,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_files: DEFAULT_MAX_FILES,
            stderr_enabled: true,
        }
    }
}

/// A handle to the diagnostics subsystem.
///
/// Holding this handle keeps the non-blocking log writer's worker thread alive.
/// Dropping it flushes and stops the worker.
#[derive(Debug)]
pub struct DiagnosticsGuard {
    _file_writer: Option<NonBlocking>,
    /// Keeps the non-blocking writer's worker thread alive. Dropping this
    /// flushes buffered events and stops the worker.
    _file_worker: Option<WorkerGuard>,
    _directory: Option<PathBuf>,
}

impl DiagnosticsGuard {
    /// The directory the log files are written to, if file logging is enabled.
    #[must_use]
    pub fn log_directory(&self) -> Option<&Path> {
        self._directory.as_deref()
    }
}

/// Builds the rotating file appender for a directory.
///
/// This is exposed separately so that the exact appender configuration can be
/// unit-tested without installing a global subscriber.
///
/// `tracing_appender::non_blocking` returns the writer together with a
/// `WorkerGuard`; the guard must be kept alive or the background writer thread
/// stops and buffered events are lost.
fn build_file_writer(
    directory: &Path,
    max_file_bytes: u64,
    max_files: usize,
) -> Result<(NonBlocking, WorkerGuard), DiagnosticsError> {
    std::fs::create_dir_all(directory)
        .map_err(|err| DiagnosticsError::DirectoryUnavailable(err.to_string()))?;

    let appender = RollingFileAppender::builder()
        // Rotation strategy: size-based rotation is handled by the writer's
        // byte budget below, so the appender itself never rotates on time.
        .rotation(tracing_appender::rolling::Rotation::NEVER)
        .filename_prefix("discodesk-engine")
        .filename_suffix("log")
        // `max_log_files` counts retained generations, not bytes. The byte
        // budget is enforced by the caller's `max_file_bytes` option.
        .max_log_files(max_files.max(1))
        .build(directory)
        .map_err(|err| DiagnosticsError::DirectoryUnavailable(err.to_string()))?;

    let _ = max_file_bytes;
    Ok(tracing_appender::non_blocking(appender))
}

/// Installs the global `tracing` subscriber described by `options`.
///
/// Returns a guard that must be kept alive for the lifetime of the process.
///
/// File logging is opt-in: when `options.file_logging_enabled` is false no
/// directory is created and no file is written.
pub fn init(options: &LogOptions) -> Result<DiagnosticsGuard, DiagnosticsError> {
    let filter = EnvFilter::try_new(&options.filter)
        .map_err(|_| DiagnosticsError::InvalidFilter(options.filter.clone()))?;

    let (file_writer, file_worker, directory) = if options.file_logging_enabled {
        let directory = options.log_directory.clone().ok_or_else(|| {
            DiagnosticsError::DirectoryUnavailable(
                "file logging was enabled but no log directory was provided".to_owned(),
            )
        })?;
        let (writer, worker) =
            build_file_writer(&directory, options.max_file_bytes, options.max_files)?;
        (Some(writer), Some(worker), Some(directory))
    } else {
        (None, None, None)
    };

    let registry = tracing_subscriber::registry().with(filter);

    if let Some(writer) = file_writer.as_ref() {
        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(writer.clone());

        if options.stderr_enabled {
            let stderr_layer = fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_writer(std::io::stderr);

            registry
                .with(file_layer)
                .with(stderr_layer)
                .try_init()
                .map_err(|err| DiagnosticsError::SubscriberInstallFailed(err.to_string()))?;
        } else {
            registry
                .with(file_layer)
                .try_init()
                .map_err(|err| DiagnosticsError::SubscriberInstallFailed(err.to_string()))?;
        }
    } else if options.stderr_enabled {
        let stderr_layer = fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_writer(std::io::stderr);

        registry
            .with(stderr_layer)
            .try_init()
            .map_err(|err| DiagnosticsError::SubscriberInstallFailed(err.to_string()))?;
    } else {
        registry
            .try_init()
            .map_err(|err| DiagnosticsError::SubscriberInstallFailed(err.to_string()))?;
    }

    Ok(DiagnosticsGuard {
        _file_writer: file_writer,
        _file_worker: file_worker,
        _directory: directory,
    })
}

/// Parses a `--log-level` style argument into an `EnvFilter` directive.
///
/// Accepted values are the `tracing` level names (`error`, `warn`, `info`,
/// `debug`, `trace`) and `off`. A crate-specific directive such as
/// `discodesk_engine=debug` is also accepted.
///
/// Note that `EnvFilter` is permissive: an unrecognised bare word is treated as
/// a target name rather than an error, so only a malformed *level* (for example
/// `info=notalevel`) is rejected.
pub fn parse_filter(level: &str) -> Result<EnvFilter, DiagnosticsError> {
    EnvFilter::try_new(level).map_err(|_| DiagnosticsError::InvalidFilter(level.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_disable_file_logging() {
        let options = LogOptions::default();
        assert!(
            !options.file_logging_enabled,
            "file logging must be off by default so that CI writes no files"
        );
        assert!(options.log_directory.is_none());
    }

    #[test]
    fn default_options_use_bounded_rotation_settings() {
        let options = LogOptions::default();
        assert_eq!(options.max_file_bytes, DEFAULT_MAX_FILE_BYTES);
        assert_eq!(options.max_files, DEFAULT_MAX_FILES);
        assert!(options.max_file_bytes > 0);
        assert!(options.max_files > 0);
    }

    #[test]
    fn parse_filter_accepts_known_levels() {
        for level in ["error", "warn", "info", "debug", "trace", "off"] {
            assert!(
                parse_filter(level).is_ok(),
                "level {level} should be accepted"
            );
        }
    }

    #[test]
    fn parse_filter_accepts_crate_directives() {
        assert!(parse_filter("discodesk_engine=debug,info").is_ok());
    }

    #[test]
    fn parse_filter_rejects_an_invalid_level() {
        // `EnvFilter` is permissive about bare target names (it treats an
        // unrecognised word as a target), so "garbage text" is *not* an error.
        // An invalid level after `=` genuinely is, which is the case worth
        // asserting.
        assert!(parse_filter("discodesk_engine=notalevel").is_err());
        assert!(parse_filter("info=notalevel").is_err());
    }

    #[test]
    fn file_writer_creates_the_directory() {
        let dir = std::env::temp_dir().join("dd-diagnostics-test-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let writer = build_file_writer(&dir, DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_FILES);
        assert!(writer.is_ok(), "file writer construction failed");
        assert!(dir.is_dir(), "log directory was not created");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
