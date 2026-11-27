//! DiscoDesk local IPC.
//!
//! This crate defines the **contract** between the `discodesk-engine` process
//! and the `discodesk-studio` process:
//!
//! * [`frame`] — a length-delimited frame codec with a bounded maximum size.
//! * [`protocol`] — the versioned command set and its request/response
//!   envelopes.
//! * [`transport`] — local transports: a Windows named pipe with a
//!   current-user ACL, and a Unix domain socket with restrictive permissions.
//! * [`error`] — the stable error taxonomy shared with the rest of the engine.
//!
//! # Scope at Gate 0
//!
//! Only the contract exists. The engine answers every command that depends on
//! a subsystem with a structured error that says the subsystem is not
//! implemented. Nothing here pretends to do audio capture, rendering or image
//! analysis.
//!
//! # Security
//!
//! The IPC channel is local-only. There is no TCP listener, no HTTP endpoint
//! and no unauthenticated loopback socket. On Windows the pipe is created
//! with an explicit SDDL descriptor that grants access only to the owner and
//! `LOCAL SYSTEM`; the default named-pipe DACL (which grants read access to
//! `Everyone`) is never used.

// The `windows` transport is the only module in the workspace that contains
// `unsafe` code, and every unsafe block there is individually documented with
// its pointer, lifetime, ownership and thread invariants.
#![warn(missing_docs)]

pub mod error;
pub mod frame;
pub mod protocol;
pub mod transport;

pub use error::{ErrorCode, IpcError, IpcResult};
pub use frame::{Frame, MAX_FRAME_BYTES};
pub use protocol::{
    AudioSourceMode, Command, ErrorPayload, FrameRateCap, PROTOCOL_VERSION, RequestEnvelope,
    ResponseEnvelope,
};
pub use transport::{Listener, Transport, connect, listener};
