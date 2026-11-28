//! DiscoDesk audio capture.
//!
//! Windows: WASAPI shared-mode **system/output loopback** on the default render
//! endpoint. Other platforms compile and report `UnsupportedPlatform` rather
//! than pretending to capture.
//!
//! # Honest capability statement
//!
//! This is the system mix. It is not per-application capture; that requires
//! `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` on Windows 10 build 20348 or
//! later, and is a separate, capability-gated feature (`ARCHITECTURE.md` §6.2).
//!
//! # Layers
//!
//! * [`format`] — `WAVEFORMATEX` interpretation and PCM-to-mono conversion. Pure.
//! * [`ring`] — the bounded, lock-free handoff from capture to DSP.
//! * [`stats`] — counters; sample loss is always reported.
//! * [`backend`] — the platform session.
//! * [`runtime`] — threads, recovery, and telemetry assembly.
//!
//! # Real-time contract
//!
//! The capture path never logs, never does file or network I/O, never allocates
//! once its buffers are warm, and never blocks on the DSP thread. Overflow is
//! counted ([`stats::StatsSnapshot::dropped_samples`]).

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod backend;
pub mod error;
pub mod format;
pub mod ring;
pub mod runtime;
pub mod stats;

pub use backend::{CaptureSession, DeviceInfo, ReadOutcome};
pub use error::AudioError;
pub use format::{AudioFormat, FormatError, SampleEncoding};
pub use ring::MonoRing;
pub use runtime::{AudioRuntime, AudioRuntimeConfig, AudioTelemetry, CaptureState, unix_time_ns};
pub use stats::{CaptureStats, StatsSnapshot};
