//! DiscoDesk diagnostics.
//!
//! This crate provides two things and nothing else:
//!
//! 1. [`event`] — privacy-preserving diagnostic events and the [`event::EventSink`]
//!    abstraction. No raw PCM, no image bytes and no absolute user paths are
//!    representable.
//! 2. [`log`] — a rotating, local-only `tracing` file sink that is **off by
//!    default** so that unit tests and CI never write files to disk.
//!
//! Engine crates should depend on this crate instead of talking to `tracing`
//! directly for anything that could carry user data.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod event;
pub mod log;

pub use event::{
    BoundedEventBuffer, DiagnosticEvent, EventSink, FileAction, NullEventSink, RedactedPath,
    TracingEventSink, redact_path,
};
pub use log::{DiagnosticsError, DiagnosticsGuard, LogOptions, init, parse_filter};
