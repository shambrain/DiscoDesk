//! DiscoDesk engine core.
//!
//! This crate owns the parts of the engine that are authoritative:
//!
//! * [`state`] — the engine state machine from `ARCHITECTURE.md` §11, with an
//!   explicit transition table.
//! * [`error`] — the typed engine error and its stable [`error::ErrorCode`].
//! * [`Engine`] — the lifecycle owner. It is the single writer of engine state;
//!   everything else (IPC, diagnostics, and later the audio/render/analysis
//!   subsystems) observes or reports into it.
//!
//! # Scope at Gate 0
//!
//! The engine reaches `IdleReady` and stops again. There is no audio capture,
//! no renderer, no desktop host and no analysis: the states for those are
//! declared in [`state::EngineState`] and documented as *not reachable*, rather
//! than being produced by a fake transition.
//!
//! # Concurrency
//!
//! No async runtime is used (see `docs/ADR/0009`). State is guarded by a
//! `std::sync::Mutex`. A poisoned lock is *recovered* rather than propagated:
//! a panic in an unrelated thread must not turn the engine into an
//! unresponsive process, and the state itself remains a valid
//! [`state::EngineState`] because it is only ever written through
//! [`state::EngineState::can_transition_to`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod state;

pub use error::{EngineError, EngineResult, ErrorCode};
pub use state::{EngineState, StateError};

use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

/// A snapshot of engine status, suitable for answering a status query.
///
/// This is deliberately a plain data type with no serialization dependency:
/// the IPC layer decides how it is encoded, so the wire format is not coupled
/// to this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineStatus {
    /// The current state.
    pub state: EngineState,
    /// How long the engine has been running, in milliseconds.
    ///
    /// Zero when the engine is not running.
    pub uptime_ms: u64,
    /// The number of wallpaper surfaces the engine owns.
    ///
    /// Always zero at Gate 0: no desktop host exists yet. It is reported
    /// explicitly rather than omitted so a client can distinguish "no
    /// surfaces" from "unknown".
    pub surface_count: u32,
}

impl EngineStatus {
    /// The engine package version that produced this status.
    #[must_use]
    pub const fn engine_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Whether the engine is in a state that can serve commands.
    #[must_use]
    pub const fn is_serving(&self) -> bool {
        self.state.is_running()
    }
}

/// Mutable engine state, guarded by the owning [`Engine`].
#[derive(Debug)]
struct EngineInner {
    state: EngineState,
    started_at: Option<Instant>,
}

/// The engine lifecycle owner.
///
/// The engine is the authoritative owner of its own state. Constructing one
/// does not start anything; call [`Engine::start`].
#[derive(Debug)]
pub struct Engine {
    inner: Mutex<EngineInner>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Creates an engine in [`EngineState::Stopped`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(EngineInner {
                state: EngineState::Stopped,
                started_at: None,
            }),
        }
    }

    /// Locks the inner state, recovering from a poisoned lock.
    ///
    /// A panic while the lock was held cannot leave `EngineInner` in an
    /// invalid state because every write goes through the transition table,
    /// so recovering is safe and keeps the engine observable. The alternative
    /// — propagating the poison — would make every subsequent status query
    /// fail forever after one unrelated panic.
    fn lock_inner(&self) -> MutexGuard<'_, EngineInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!(
                    target: "dd_core.engine",
                    "engine state lock was poisoned; recovering"
                );
                poisoned.into_inner()
            }
        }
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> EngineState {
        self.lock_inner().state
    }

    /// Whether the engine is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.state().is_running()
    }

    /// Starts the engine.
    ///
    /// Transitions `Stopped → Starting → IdleReady`. Starting an engine that
    /// is already running is **not** an error and does not change state, which
    /// makes the call idempotent and safe to retry.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] only if the current state
    /// cannot legally reach `Starting` or `IdleReady`.
    pub fn start(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        match inner.state {
            // Any state in which the engine is already running makes start()
            // a no-op. `ErrorRecoverable` is running: the engine is degraded
            // but alive, and restarting it would be the wrong recovery action.
            EngineState::IdleReady | EngineState::Starting | EngineState::ErrorRecoverable => {
                return Ok(());
            }
            _ => {}
        }
        Self::transition_locked(&mut inner, EngineState::Starting)?;
        inner.started_at = Some(Instant::now());
        Self::transition_locked(&mut inner, EngineState::IdleReady)?;
        Ok(())
    }

    /// Stops the engine.
    ///
    /// Transitions to [`EngineState::Stopped`]. Stopping an engine that is
    /// already stopped is **not** an error.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] only if the current state
    /// cannot legally reach `Stopped`.
    pub fn stop(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        if inner.state == EngineState::Stopped {
            return Ok(());
        }
        Self::transition_locked(&mut inner, EngineState::Stopped)?;
        inner.started_at = None;
        Ok(())
    }

    /// Marks the engine as listening, i.e. owning an audio capture runtime.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] if the current state cannot
    /// reach `Listening`.
    pub fn begin_listening(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        Self::transition_locked(&mut inner, EngineState::Listening)
    }

    /// Marks the engine as actively receiving audio.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] if the current state cannot
    /// reach `AudioActive`.
    pub fn mark_audio_active(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        Self::transition_locked(&mut inner, EngineState::AudioActive)
    }

    /// Returns from a listening state to `IdleReady`, e.g. after audio capture
    /// was stopped while the engine keeps running.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] if the current state cannot
    /// reach `IdleReady`.
    pub fn end_listening(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        Self::transition_locked(&mut inner, EngineState::IdleReady)
    }

    /// Marks a recoverable failure.
    ///
    /// Used when a subsystem reports a condition the engine can survive with a
    /// fallback (a missing permission, a lost device). The engine continues in
    /// [`EngineState::ErrorRecoverable`] and may return to `IdleReady`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] if the current state cannot
    /// reach `ErrorRecoverable`.
    pub fn mark_recoverable(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        Self::transition_locked(&mut inner, EngineState::ErrorRecoverable)
    }

    /// Returns from [`EngineState::ErrorRecoverable`] to
    /// [`EngineState::IdleReady`].
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidTransition`] if the engine is not
    /// recovering.
    pub fn clear_recoverable(&self) -> EngineResult<()> {
        let mut inner = self.lock_inner();
        Self::transition_locked(&mut inner, EngineState::IdleReady)
    }

    /// A status snapshot.
    #[must_use]
    pub fn status(&self) -> EngineStatus {
        let inner = self.lock_inner();
        let uptime_ms = inner.started_at.map_or(0, |started| {
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
        EngineStatus {
            state: inner.state,
            uptime_ms,
            surface_count: 0,
        }
    }

    /// Applies a transition against the table, leaving state untouched on
    /// refusal.
    fn transition_locked(inner: &mut EngineInner, next: EngineState) -> EngineResult<()> {
        let from = inner.state;
        if from == next {
            return Ok(());
        }
        if !from.can_transition_to(next) {
            tracing::debug!(
                target: "dd_core.engine",
                from = from.name(),
                to = next.name(),
                "refused illegal engine transition"
            );
            return Err(EngineError::InvalidTransition { from, to: next });
        }
        inner.state = next;
        tracing::debug!(
            target: "dd_core.engine",
            from = from.name(),
            to = next.name(),
            "engine transition"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_engine_is_stopped() {
        let engine = Engine::new();
        assert_eq!(engine.state(), EngineState::Stopped);
        assert!(!engine.is_running());
    }

    #[test]
    fn start_reaches_idle_ready() {
        let engine = Engine::new();
        engine.start().expect("start should succeed");
        assert_eq!(engine.state(), EngineState::IdleReady);
        assert!(engine.is_running());
    }

    #[test]
    fn start_is_idempotent() {
        let engine = Engine::new();
        engine.start().expect("first start");
        engine.start().expect("second start must not fail");
        assert_eq!(engine.state(), EngineState::IdleReady);
    }

    #[test]
    fn stop_returns_to_stopped() {
        let engine = Engine::new();
        engine.start().expect("start");
        engine.stop().expect("stop");
        assert_eq!(engine.state(), EngineState::Stopped);
        assert!(!engine.is_running());
    }

    #[test]
    fn stop_is_idempotent() {
        let engine = Engine::new();
        engine.stop().expect("stop on a fresh engine must not fail");
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn engine_can_restart_after_stop() {
        let engine = Engine::new();
        engine.start().expect("start");
        engine.stop().expect("stop");
        engine.start().expect("restart");
        assert_eq!(engine.state(), EngineState::IdleReady);
    }

    #[test]
    fn recoverable_round_trip() {
        let engine = Engine::new();
        engine.start().expect("start");
        engine.mark_recoverable().expect("mark recoverable");
        assert_eq!(engine.state(), EngineState::ErrorRecoverable);
        engine.clear_recoverable().expect("clear recoverable");
        assert_eq!(engine.state(), EngineState::IdleReady);
    }

    #[test]
    fn recoverable_is_not_permanent() {
        let engine = Engine::new();
        engine.start().expect("start");
        engine.mark_recoverable().expect("mark recoverable");
        // A recovering engine can still be stopped, which is how shutdown must
        // behave when a subsystem is degraded.
        engine.stop().expect("stop from ErrorRecoverable");
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn an_illegal_transition_leaves_state_untouched() {
        // A stopped engine cannot jump straight to IdleReady.
        let engine = Engine::new();
        let err = engine.clear_recoverable().expect_err("must be refused");
        assert!(matches!(err, EngineError::InvalidTransition { .. }));
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn status_reports_state_and_uptime() {
        let engine = Engine::new();
        let stopped = engine.status();
        assert_eq!(stopped.state, EngineState::Stopped);
        assert_eq!(stopped.uptime_ms, 0);
        assert_eq!(stopped.surface_count, 0);
        assert!(!stopped.is_serving());

        engine.start().expect("start");
        let running = engine.status();
        assert_eq!(running.state, EngineState::IdleReady);
        assert!(running.is_serving());
        assert_eq!(running.surface_count, 0);
    }

    #[test]
    fn concurrent_start_and_stop_do_not_panic() {
        use std::sync::Arc;
        use std::thread;

        let engine = Arc::new(Engine::new());
        let mut handles = Vec::new();
        for _ in 0..8 {
            let engine = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    let _ = engine.start();
                    let _ = engine.stop();
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker thread must not panic");
        }
        // Whatever interleaving occurred, the state must be a legal one.
        assert!(matches!(
            engine.state(),
            EngineState::Stopped | EngineState::IdleReady
        ));
    }

    #[test]
    fn start_from_recoverable_is_idempotent_because_it_is_running() {
        let engine = Engine::new();
        engine.start().expect("start");
        engine.mark_recoverable().expect("mark recoverable");
        engine
            .start()
            .expect("start while recovering must not fail");
        assert_eq!(engine.state(), EngineState::ErrorRecoverable);
    }
}
