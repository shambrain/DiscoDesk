//! Tauri commands exposed to the Studio frontend.
//!
//! These are deliberately thin: they forward to [`EngineClient`] and convert
//! failures into a serialisable error. No engine state is derived here.

use std::sync::Arc;

use crate::engine_client::{EngineClient, EngineView, PipeConnector};
use tauri::State;

/// Application state managed by Tauri.
#[derive(Debug)]
pub struct StudioState {
    /// The single engine client. One client means at most one live connection,
    /// which bounds the Studio's work regardless of how often the UI polls.
    pub client: Arc<EngineClient<PipeConnector>>,
}

/// Returns the current engine status view.
///
/// Runs on a blocking thread because the IPC exchange is synchronous. The
/// frontend guards against overlapping calls, so at most one blocking task is
/// alive at a time.
///
/// # Errors
///
/// Returns an error string only if the blocking task itself cannot be joined
/// (for example because it panicked). Engine conditions such as "not running"
/// or "protocol mismatch" are *not* errors here: they are valid
/// [`EngineView`] values, so the UI can render them as states rather than as a
/// rejected promise.
#[tauri::command]
pub async fn engine_status(state: State<'_, StudioState>) -> Result<EngineView, String> {
    let client = Arc::clone(&state.client);
    tauri::async_runtime::spawn_blocking(move || client.status())
        .await
        .map_err(|err| format!("engine status task failed: {err}"))
}

/// Returns the endpoint the Studio is configured to use, for diagnostics.
#[tauri::command]
#[must_use]
pub fn engine_endpoint(state: State<'_, StudioState>) -> String {
    state.client.endpoint()
}

/// Drops the current connection without affecting the engine.
///
/// The engine keeps running; only this client's pipe handle is released. This
/// exists so the frontend can release the handle when the window closes
/// instead of waiting for process exit.
#[tauri::command]
pub fn engine_disconnect(state: State<'_, StudioState>) {
    state.client.disconnect();
}
