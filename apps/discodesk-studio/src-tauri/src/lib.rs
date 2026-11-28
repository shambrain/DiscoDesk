//! DiscoDesk Studio.
//!
//! The Studio is an **on-demand control UI**. It is a separate process from the
//! engine (`docs/ADR/0001`), it talks to the engine only over the versioned
//! local IPC contract (`docs/ADR/0002`, `docs/IPC_PROTOCOL.md`), and it owns no
//! audio capture, DSP, renderer or engine state.
//!
//! Closing the Studio must never stop the engine; nothing in this crate sends
//! `ShutdownEngine`.

pub mod commands;
pub mod engine_client;

use std::sync::Arc;

use commands::StudioState;
use engine_client::{EngineClient, EngineView, PipeConnector};
use tauri::Manager;

/// Runs when a probe reached the engine. Returned so the binary can map it to
/// an exit code without owning the reporting.
pub const PROBE_OK: i32 = 0;
/// Runs when the probe could not reach a healthy engine.
pub const PROBE_FAILED: i32 = 1;

/// Performs one headless status round-trip and prints the view as JSON.
///
/// This exists so that the Studio/engine boundary can be verified by a script
/// against the **real** Studio binary and the **real** engine, rather than only
/// through unit tests with a scripted transport. It also gives support a way to
/// ask "can this machine see the engine?" without opening a window.
pub fn probe(endpoint: Option<String>) -> (i32, String) {
    let connector = match endpoint {
        Some(endpoint) => PipeConnector::at(endpoint),
        None => PipeConnector::default_endpoint(),
    };
    let client = EngineClient::new(connector);
    let view = client.status();

    let rendered = serde_json::to_string_pretty(&view)
        .unwrap_or_else(|err| format!("{{\"kind\":\"error\",\"detail\":\"{err}\"}}"));

    let code = if view.is_connected() {
        PROBE_OK
    } else {
        PROBE_FAILED
    };
    (code, rendered)
}

/// Builds and runs the Tauri application.
///
/// # Panics
///
/// Panics if Tauri cannot start. There is no meaningful recovery: a window that
/// cannot be created is a hard failure, and the process must exit rather than
/// linger invisibly.
pub fn run() {
    let client = Arc::new(EngineClient::new(PipeConnector::default_endpoint()));

    tauri::Builder::default()
        .manage(StudioState { client })
        .invoke_handler(tauri::generate_handler![
            commands::engine_status,
            commands::engine_endpoint,
            commands::engine_disconnect
        ])
        // Deterministic shutdown.
        //
        // Tauri's default is to leave the process running until the webview has
        // finished tearing down, and WebView2 teardown latency is not something
        // this application controls. That made "does closing the window end the
        // Studio?" depend on the machine: locally it exited at once, while on a
        // CI runner the process lingered past a 15 s wait.
        //
        // Exiting explicitly on CloseRequested makes the lifecycle deterministic
        // and independent of the webview. It is safe for the product because the
        // engine is a *different process* (`docs/ADR/0001`): calling exit here
        // cannot stop playback. No Rust thread holds a lock that needs draining:
        // the only state is the `EngineClient` mutex, whose holder is the
        // per-command blocking task, and process exit reclaims it.
        //
        // `exit` is idempotent from the caller's perspective: a second close
        // request cannot arrive once the event loop has been told to stop.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Only the main Studio window exists at Gate 0/1, so any close
                // request is the user closing the Studio.
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("the DiscoDesk Studio window could not be created");
}

/// The view type, re-exported for consumers of this library crate.
pub use engine_client::EngineStatus;
/// Re-exported for consumers of this library crate.
pub use engine_client::EngineView as StatusView;

impl StudioState {
    /// Builds state for an explicit endpoint. Used by tests.
    #[must_use]
    pub fn at_endpoint(endpoint: impl Into<String>) -> Self {
        Self {
            client: Arc::new(EngineClient::new(PipeConnector::at(endpoint))),
        }
    }
}

/// Renders a view as JSON, for tests and diagnostics.
#[must_use]
pub fn render_view(view: &EngineView) -> String {
    serde_json::to_string(view).unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_view_produces_a_tagged_object() {
        let view = EngineView::Unavailable {
            detail: "not running".to_owned(),
        };
        let rendered = render_view(&view);
        assert!(rendered.contains("\"kind\":\"unavailable\""), "{rendered}");
        assert!(rendered.contains("not running"), "{rendered}");
    }

    #[test]
    fn probe_against_an_absent_endpoint_fails_cleanly() {
        // A name that cannot exist: no engine will ever listen on it.
        let endpoint = if cfg!(windows) {
            r"\\.\pipe\discodesk-engine-probe-test-does-not-exist".to_owned()
        } else {
            "/nonexistent/discodesk-engine-probe-test.sock".to_owned()
        };
        let (code, rendered) = probe(Some(endpoint));
        assert_eq!(code, PROBE_FAILED);
        assert!(
            rendered.contains("\"kind\": \"unavailable\"")
                || rendered.contains("\"kind\":\"unavailable\""),
            "{rendered}"
        );
    }
}
