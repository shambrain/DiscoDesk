//! The DiscoDesk background engine.
//!
//! # What this process does at Gate 0
//!
//! 1. Initialises diagnostics (console only unless file logging is requested).
//! 2. Creates a current-user-only local IPC endpoint and announces it.
//! 3. Serves the versioned command protocol until `ShutdownEngine` arrives.
//!
//! It owns **no wallpaper surface, no audio capture, no renderer and no image
//! analysis**. Commands that depend on those subsystems answer with a
//! structured "not implemented" error naming the gate that will implement
//! them. Nothing here pretends to react to music.
//!
//! # Shutdown
//!
//! The documented graceful path is the `ShutdownEngine` command, which is what
//! Studio uses. The engine then stops its listener, releases the endpoint and
//! exits `0`.
//!
//! Ctrl-C is **not** intercepted at Gate 0: there is no console control
//! handler yet, so Ctrl-C terminates the process. The named pipe is released
//! by the OS when the process exits, so this does not leak the endpoint. This
//! limitation is recorded in `docs/IMPLEMENTATION_STATE.md` rather than being
//! papered over with an untested handler.
//!
//! # Security
//!
//! The endpoint is local-only. On Windows the pipe carries an explicit SDDL
//! descriptor granting only the owning user and `LOCAL SYSTEM`; remote clients
//! are rejected. See `dd_ipc::transport::windows`.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use dd_core::{Engine, EngineState};
use dd_diagnostics::LogOptions;
use dd_ipc::IpcError;
use dd_ipc::frame::Frame;
use dd_ipc::protocol::{
    Command, ErrorPayload, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope,
};
use dd_ipc::transport::{Listener, Transport};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "discodesk-engine",
    version,
    about = "DiscoDesk background engine (Gate 0: IPC and lifecycle only)",
    disable_help_subcommand = true
)]
struct Cli {
    /// Diagnostics filter, for example `info` or `discodesk_engine=debug,info`.
    #[arg(long, value_name = "FILTER", default_value = "info")]
    log_level: String,

    /// Write a rotating local log file into this directory (opt-in).
    ///
    /// Without this flag no file is written at all.
    #[arg(long, value_name = "DIR")]
    log_dir: Option<std::path::PathBuf>,

    /// Print the endpoint path and exit. Does not start the engine.
    #[arg(long)]
    print_endpoint: bool,

    /// Override the endpoint path or pipe name.
    ///
    /// Supplying this bypasses the per-user default. The endpoint still
    /// carries the current-user security descriptor.
    #[arg(long, value_name = "PATH")]
    endpoint: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("discodesk-engine: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<ExitCode, String> {
    let endpoint = cli.endpoint.clone().unwrap_or_else(default_endpoint);

    if cli.print_endpoint {
        println!("{endpoint}");
        return Ok(ExitCode::SUCCESS);
    }

    let options = LogOptions {
        filter: cli.log_level.clone(),
        file_logging_enabled: cli.log_dir.is_some(),
        log_directory: cli.log_dir.clone(),
        ..LogOptions::default()
    };
    // The guard must outlive the serve loop; dropping it flushes the writer.
    let _diagnostics = dd_diagnostics::init(&options).map_err(|err| err.to_string())?;

    let engine = Engine::new();
    engine.start().map_err(|err| err.to_string())?;

    let mut listener = dd_ipc::listener(&endpoint).map_err(|err| match err {
        IpcError::EndpointUnavailable(detail) => {
            format!("could not create the IPC endpoint at {endpoint}: {detail}")
        }
        other => format!("could not create the IPC endpoint at {endpoint}: {other}"),
    })?;

    // This line is the readiness signal a supervisor or a test waits for.
    println!("engine ready, endpoint: {endpoint}");
    println!(
        "protocol version {PROTOCOL_VERSION}; state {}",
        engine.state().name()
    );

    tracing::info!(
        target: "discodesk_engine",
        endpoint = %endpoint,
        protocol_version = PROTOCOL_VERSION,
        "engine listening"
    );

    let outcome = serve(&engine, listener.as_mut());

    // Always attempt a clean stop, even if serving failed.
    if let Err(err) = engine.stop() {
        tracing::warn!(target: "discodesk_engine", error = %err, "engine stop failed");
    }
    drop(listener);

    match outcome {
        Ok(()) => {
            tracing::info!(target: "discodesk_engine", "engine shut down");
            println!("engine stopped");
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => Err(format!("serving failed: {err}")),
    }
}

/// The default endpoint for the current user.
///
/// Delegated to `dd_ipc` so the Studio computes exactly the same name. Two
/// independent derivations would let the two processes open different pipes
/// while both appearing correct.
fn default_endpoint() -> String {
    dd_ipc::default_endpoint_path()
}

/// Accepts and serves connections until a client requests shutdown.
fn serve(engine: &Engine, listener: &mut dyn Listener) -> Result<(), IpcError> {
    loop {
        let mut transport = match listener.accept() {
            Ok(transport) => transport,
            Err(err) => {
                // A failed accept is recoverable: log it and keep serving so a
                // single bad client cannot take the engine down.
                tracing::warn!(target: "discodesk_engine", error = %err, "accept failed");
                continue;
            }
        };

        match serve_connection(engine, transport.as_mut()) {
            Ok(ConnectionOutcome::Shutdown) => return Ok(()),
            Ok(ConnectionOutcome::Closed) => {}
            Err(err) => {
                // Same reasoning as above: a protocol error on one connection
                // must not stop the engine.
                tracing::warn!(target: "discodesk_engine", error = %err, "connection ended");
            }
        }
    }
}

/// What a finished connection asked the engine to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionOutcome {
    /// Keep serving other clients.
    Closed,
    /// The client asked the engine to shut down.
    Shutdown,
}

/// Serves one connection.
fn serve_connection(
    engine: &Engine,
    transport: &mut dyn Transport,
) -> Result<ConnectionOutcome, IpcError> {
    loop {
        let frame = match transport.recv_frame() {
            Ok(frame) => frame,
            Err(IpcError::PeerClosed) => return Ok(ConnectionOutcome::Closed),
            Err(err @ IpcError::ProtocolVersionMismatch { .. }) => {
                // The frame carried a version this build does not speak. We
                // cannot correlate a reply because the request was never
                // decoded, so close and let the client report the mismatch.
                // An envelope-level version mismatch *is* answered below.
                tracing::warn!(
                    target: "discodesk_engine.ipc",
                    error = %err,
                    "closing connection: frame protocol version mismatch"
                );
                return Ok(ConnectionOutcome::Closed);
            }
            Err(err) => return Err(err),
        };

        let request: RequestEnvelope = match serde_json::from_slice(&frame.payload) {
            Ok(request) => request,
            Err(err) => {
                // We cannot recover a request id from malformed JSON.
                let payload = ErrorPayload::new(
                    dd_ipc::ErrorCode::MalformedPayload,
                    format!("request envelope was not valid JSON: {err}"),
                );
                send_response(transport, &ResponseEnvelope::err(0, &payload))?;
                return Ok(ConnectionOutcome::Closed);
            }
        };

        let request_id = request.request_id;

        // `validate` performs the protocol-version check and the per-command
        // payload validation. Both are answerable and correlate to the client's
        // request id.
        let command = match request.validate() {
            Ok(command) => command,
            Err(err) => {
                send_response(transport, &ResponseEnvelope::from_error(request_id, &err))?;
                // A version mismatch is unrecoverable: the rest of this
                // connection may use a different wire contract, so close.
                // Every other validation failure (unknown command, bad
                // argument, malformed payload) leaves the connection usable,
                // which is what docs/IPC_PROTOCOL.md documents.
                if matches!(err, IpcError::ProtocolVersionMismatch { .. }) {
                    return Ok(ConnectionOutcome::Closed);
                }
                continue;
            }
        };

        tracing::debug!(
            target: "discodesk_engine.ipc",
            command = command.name(),
            request_id,
            "handling command"
        );

        let (response, shutdown) = handle(engine, command, request_id);
        send_response(transport, &response)?;

        if shutdown {
            return Ok(ConnectionOutcome::Shutdown);
        }
    }
}

/// Encodes and sends a response frame.
fn send_response(
    transport: &mut dyn Transport,
    response: &ResponseEnvelope,
) -> Result<(), IpcError> {
    let bytes = serde_json::to_vec(response)
        .map_err(|err| IpcError::MalformedPayload(format!("could not encode response: {err}")))?;
    let frame = Frame {
        version: PROTOCOL_VERSION,
        payload: bytes,
    };
    transport.send_frame(&frame)
}

/// Answers one command.
///
/// Returns the response and whether the engine should shut down afterwards.
fn handle(engine: &Engine, command: Command, request_id: u64) -> (ResponseEnvelope, bool) {
    match command {
        Command::GetEngineStatus => {
            let status = engine.status();
            let body = json!({
                "state": status.state.name(),
                "engineVersion": status.engine_version(),
                "protocolVersion": PROTOCOL_VERSION,
                "uptimeMs": status.uptime_ms,
                "surfaceCount": status.surface_count,
                "isRunning": status.state.is_running(),
                "paused": status.state.is_paused(),
            });
            (ResponseEnvelope::ok(request_id, Some(body)), false)
        }

        Command::GetPlatformCapabilities => {
            // Gate 0 has not probed any capability. Reporting "unknown" is the
            // honest answer: it is not the same claim as "unsupported", and
            // `docs/ADR/0003` requires the two to stay distinguishable.
            let body = json!({
                "probed": false,
                "os": std::env::consts::OS,
                "note": "capability probing is not implemented at Gate 0",
                "wallpaperHosting": "unknown",
                "systemAudioCapture": "unknown",
                "perAppAudioCapture": "unknown",
                "wallpaperChangeDetection": "unknown",
            });
            (ResponseEnvelope::ok(request_id, Some(body)), false)
        }

        Command::GetDiagnosticsSnapshot => {
            // No event ring buffer is wired at Gate 0. Returning an empty list
            // plus an explicit explanation is honest; inventing events would
            // not be.
            let body = json!({
                "events": [],
                "note": "diagnostic event buffering is not wired at Gate 0; \
                         use --log-dir to opt into a rotating file log",
            });
            (ResponseEnvelope::ok(request_id, Some(body)), false)
        }

        Command::ShutdownEngine => {
            let body = json!({
                "state": EngineState::Stopped.name(),
                "message": "engine is shutting down",
            });
            (ResponseEnvelope::ok(request_id, Some(body)), true)
        }

        // Everything else belongs to a later gate. The answer names the gate so
        // a client can show something accurate instead of a generic failure.
        other => {
            let gate = gate_for(other);
            let payload = ErrorPayload::new(
                dd_ipc::ErrorCode::NotImplemented,
                format!(
                    "{} is not implemented at Gate 0; it belongs to {gate}",
                    other.name()
                ),
            );
            (ResponseEnvelope::err(request_id, &payload), false)
        }
    }
}

/// The gate that will implement a command.
#[must_use]
const fn gate_for(command: Command) -> &'static str {
    match command {
        Command::GetAudioSources | Command::SetAudioSource => "Gate 1 (audio capture)",
        Command::Pause | Command::Resume | Command::TogglePause => {
            "Gate 1 (engine lifecycle beyond idle)"
        }
        Command::SetEffectSettings | Command::SetVibePreset | Command::SetPerformancePolicy => {
            "Gate 2 (renderer) and Gate 4 (Studio presets)"
        }
        Command::ImportWallpaper | Command::CancelJob => "Gate 5 (image analysis and cache)",
        Command::ApplyPackage => "M3 (package format)",
        _ => "a later gate",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_defined_answer() {
        // Guards against a new `Command` variant silently falling through to
        // the catch-all without a gate mapping.
        for command in Command::all() {
            let engine = Engine::new();
            engine.start().expect("engine should start");
            let (response, shutdown) = handle(&engine, *command, 7);
            assert_eq!(response.request_id, 7);
            assert_eq!(response.version, PROTOCOL_VERSION);
            match command {
                Command::GetEngineStatus
                | Command::GetPlatformCapabilities
                | Command::GetDiagnosticsSnapshot
                | Command::ShutdownEngine => assert!(response.is_ok(), "{command} should succeed"),
                _ => assert!(!response.is_ok(), "{command} should be refused"),
            }
            assert_eq!(shutdown, matches!(command, Command::ShutdownEngine));
        }
    }

    #[test]
    fn status_reports_idle_ready() {
        let engine = Engine::new();
        engine.start().expect("start");
        let (response, _) = handle(&engine, Command::GetEngineStatus, 1);
        let body = response.result.expect("status body");
        assert_eq!(body["state"], json!("IdleReady"));
        assert_eq!(body["isRunning"], json!(true));
        assert_eq!(body["surfaceCount"], json!(0));
        assert_eq!(body["protocolVersion"], json!(PROTOCOL_VERSION));
    }

    #[test]
    fn capabilities_are_reported_as_unknown_not_supported() {
        let engine = Engine::new();
        let (response, _) = handle(&engine, Command::GetPlatformCapabilities, 2);
        let body = response.result.expect("capability body");
        assert_eq!(body["probed"], json!(false));
        assert_eq!(body["wallpaperHosting"], json!("unknown"));
        assert_eq!(body["systemAudioCapture"], json!("unknown"));
        assert_eq!(body["perAppAudioCapture"], json!("unknown"));
    }

    #[test]
    fn deferred_commands_name_their_gate() {
        let engine = Engine::new();
        let (response, _) = handle(&engine, Command::GetAudioSources, 3);
        let error = response.error.expect("deferred command must be refused");
        assert_eq!(error.error_code(), Some(dd_ipc::ErrorCode::NotImplemented));
        assert!(
            error.message.contains("Gate 1"),
            "message should name the gate, got: {}",
            error.message
        );
    }

    #[test]
    fn default_endpoint_is_non_empty_and_local() {
        let endpoint = default_endpoint();
        assert!(!endpoint.is_empty());
        assert!(endpoint.contains(dd_ipc::ENDPOINT_STEM));
        #[cfg(windows)]
        assert!(endpoint.starts_with(r"\\.\pipe\"));
    }
}
