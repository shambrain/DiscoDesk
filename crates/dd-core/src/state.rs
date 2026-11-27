//! The engine state machine.
//!
//! # State graph at Gate 0
//!
//! ```text
//! Stopped ──start()──▶ Starting ──▶ IdleReady
//!    ▲                    │            │
//!    │                    ▼            ▼
//!    └────────stop()──── Stopped    ErrorRecoverable
//! ```
//!
//! `IdleReady` is the state in which the engine owns its IPC endpoint and
//! answers status queries. No audio, rendering or analysis subsystem exists
//! at Gate 0, so the states below `IdleReady` in `ARCHITECTURE.md` §11 are
//! **defined but not reachable**. They are part of the contract so that later
//! gates have a place to grow into, and they are explicitly documented as
//! such rather than being reachable through a fake transition.
//!
//! # Rules
//!
//! * Every transition is checked. An illegal transition returns
//!   [`StateError`] and leaves the state untouched.
//! * `start()` is idempotent: starting an already-running engine is not an
//!   error and does not change the state.
//! * `stop()` is idempotent: stopping an already-stopped engine is not an
//!   error.
//! * A permission failure, a lost device or a lost GPU is *not* a crash: the
//!   engine reports the missing capability and continues with a supported
//!   fallback, which is what [`EngineState::ErrorRecoverable`] expresses.

use std::fmt;

/// The state of the engine.
///
/// The variants follow `ARCHITECTURE.md` §11. Those marked *not reachable at
/// Gate 0* are declared so the contract is complete, but no transition
/// produces them yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EngineState {
    /// The engine is not running.
    Stopped,
    /// The engine is starting up.
    Starting,
    /// The engine is running and idle.
    IdleReady,
    /// The engine is capturing audio.
    ///
    /// *Not reachable at Gate 0.* No audio capture subsystem exists yet.
    Listening,
    /// The engine has an active audio source.
    ///
    /// *Not reachable at Gate 0.* No audio capture subsystem exists yet.
    AudioActive,
    /// The engine is rendering to a wallpaper surface.
    ///
    /// *Not reachable at Gate 0.* No renderer or desktop host exists yet.
    Rendering,
    /// The user paused the engine.
    PausedByUser,
    /// A full-screen application covers the display.
    ///
    /// *Not reachable at Gate 0.* No desktop host exists yet.
    PausedByFullscreen,
    /// A power policy paused the engine.
    ///
    /// *Not reachable at Gate 0.* No power integration exists yet.
    PausedByPowerPolicy,
    /// The display went to sleep.
    ///
    /// *Not reachable at Gate 0.* No display lifecycle exists yet.
    PausedByDisplaySleep,
    /// The engine is waiting for an OS permission the user has not granted.
    ///
    /// *Not reachable at Gate 0.* No permission-gated subsystem exists yet.
    WaitingForPermission,
    /// The engine is re-establishing its audio source.
    ///
    /// *Not reachable at Gate 0.* No audio capture subsystem exists yet.
    RecoveringAudio,
    /// The engine is recreating its GPU resources.
    ///
    /// *Not reachable at Gate 0.* No renderer exists yet.
    RecoveringGpu,
    /// The engine is reattaching to the desktop host.
    ///
    /// *Not reachable at Gate 0.* No desktop host exists yet.
    RecoveringDesktopHost,
    /// The engine is running with the deterministic non-AI fallback.
    ///
    /// *Not reachable at Gate 0.* No analysis subsystem exists yet.
    AnalysisFallback,
    /// The engine hit a recoverable error and is recovering.
    ErrorRecoverable,
}

impl EngineState {
    /// The stable wire name of this state.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::Starting => "Starting",
            Self::IdleReady => "IdleReady",
            Self::Listening => "Listening",
            Self::AudioActive => "AudioActive",
            Self::Rendering => "Rendering",
            Self::PausedByUser => "PausedByUser",
            Self::PausedByFullscreen => "PausedByFullscreen",
            Self::PausedByPowerPolicy => "PausedByPowerPolicy",
            Self::PausedByDisplaySleep => "PausedByDisplaySleep",
            Self::WaitingForPermission => "WaitingForPermission",
            Self::RecoveringAudio => "RecoveringAudio",
            Self::RecoveringGpu => "RecoveringGpu",
            Self::RecoveringDesktopHost => "RecoveringDesktopHost",
            Self::AnalysisFallback => "AnalysisFallback",
            Self::ErrorRecoverable => "ErrorRecoverable",
        }
    }

    /// Whether the engine has reached a state it cannot leave on its own.
    ///
    /// Only [`EngineState::Stopped`] is terminal at Gate 0. The recovery
    /// states are *not* terminal: the engine leaves them by itself once the
    /// underlying problem is resolved.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped)
    }

    /// Whether this state means the engine is paused.
    #[must_use]
    pub const fn is_paused(self) -> bool {
        matches!(
            self,
            Self::PausedByUser
                | Self::PausedByFullscreen
                | Self::PausedByPowerPolicy
                | Self::PausedByDisplaySleep
        )
    }

    /// Whether the engine is running, i.e. not stopped.
    #[must_use]
    pub const fn is_running(self) -> bool {
        !matches!(self, Self::Stopped)
    }

    /// Whether this state is reachable at Gate 0.
    ///
    /// The states below `IdleReady` describe subsystems that do not exist yet.
    /// They are declared for contract completeness and are not produced by any
    /// transition.
    #[must_use]
    pub const fn is_reachable_at_gate_0(self) -> bool {
        matches!(self, Self::Stopped | Self::Starting | Self::IdleReady | Self::ErrorRecoverable)
    }

    /// Whether a transition from `self` to `next` is permitted.
    #[must_use]
    pub const fn can_transition_to(self, next: EngineState) -> bool {
        matches!(
            (self, next),
            (Self::Stopped, Self::Starting)
                | (Self::Starting, Self::IdleReady)
                | (Self::Starting, Self::ErrorRecoverable)
                | (Self::IdleReady, Self::ErrorRecoverable)
                | (Self::ErrorRecoverable, Self::IdleReady)
                | (Self::ErrorRecoverable, Self::Stopped)
                | (Self::Starting, Self::Stopped)
                | (Self::IdleReady, Self::Stopped)
        )
    }
}

impl fmt::Display for EngineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The error returned when a state transition is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("illegal transition from {from} to {to}")]
pub struct StateError {
    /// The state the engine is in.
    pub from: EngineState,
    /// The state that was requested.
    pub to: EngineState,
}

impl StateError {
    /// Builds the error for a refused transition.
    #[must_use]
    pub const fn new(from: EngineState, to: EngineState) -> Self {
        Self { from, to }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_0_startup_path_is_legal() {
        assert!(EngineState::Stopped.can_transition_to(EngineState::Starting));
        assert!(EngineState::Starting.can_transition_to(EngineState::IdleReady));
    }

    #[test]
    fn the_gate_0_shutdown_path_is_legal() {
        assert!(EngineState::Starting.can_transition_to(EngineState::Stopped));
        assert!(EngineState::IdleReady.can_transition_to(EngineState::Stopped));
    }

    #[test]
    fn the_recovery_path_is_legal() {
        assert!(EngineState::Starting.can_transition_to(EngineState::ErrorRecoverable));
        assert!(EngineState::IdleReady.can_transition_to(EngineState::ErrorRecoverable));
        assert!(EngineState::ErrorRecoverable.can_transition_to(EngineState::IdleReady));
        assert!(EngineState::ErrorRecoverable.can_transition_to(EngineState::Stopped));
    }

    #[test]
    fn illegal_transitions_are_refused() {
        assert!(!EngineState::Stopped.can_transition_to(EngineState::IdleReady));
        assert!(!EngineState::Stopped.can_transition_to(EngineState::Rendering));
        assert!(!EngineState::Stopped.can_transition_to(EngineState::Starting) == false);
        assert!(!EngineState::IdleReady.can_transition_to(EngineState::Starting));
        assert!(!EngineState::IdleReady.can_transition_to(EngineState::Rendering));
        assert!(!EngineState::Starting.can_transition_to(EngineState::Rendering));
        assert!(!EngineState::Stopped.can_transition_to(EngineState::Stopped));
    }

    #[test]
    fn only_stopped_is_terminal() {
        assert!(EngineState::Stopped.is_terminal());
        assert!(!EngineState::Starting.is_terminal());
        assert!(!EngineState::IdleReady.is_terminal());
        assert!(!EngineState::ErrorRecoverable.is_terminal());
        assert!(!EngineState::Rendering.is_terminal());
    }

    #[test]
    fn the_paused_states_are_the_documented_four() {
        for state in [
            EngineState::PausedByUser,
            EngineState::PausedByFullscreen,
            EngineState::PausedByPowerPolicy,
            EngineState::PausedByDisplaySleep,
        ] {
            assert!(state.is_paused(), "{state} should be paused");
        }

        for state in [
            EngineState::Stopped,
            EngineState::Starting,
            EngineState::IdleReady,
            EngineState::Listening,
            EngineState::AudioActive,
            EngineState::Rendering,
            EngineState::WaitingForPermission,
            EngineState::RecoveringAudio,
            EngineState::RecoveringGpu,
            EngineState::RecoveringDesktopHost,
            EngineState::AnalysisFallback,
            EngineState::ErrorRecoverable,
        ] {
            assert!(!state.is_paused(), "{state} should not be paused");
        }
    }

    #[test]
    fn only_the_gate_0_states_are_reachable() {
        for state in [
            EngineState::Stopped,
            EngineState::Starting,
            EngineState::IdleReady,
            EngineState::ErrorRecoverable,
        ] {
            assert!(state.is_reachable_at_gate_0(), "{state} is reachable");
        }

        for state in [
            EngineState::Listening,
            EngineState::AudioActive,
            EngineState::Rendering,
            EngineState::PausedByUser,
            EngineState::PausedByFullscreen,
            EngineState::PausedByPowerPolicy,
            EngineState::PausedByDisplaySleep,
            EngineState::WaitingForPermission,
            EngineState::RecoveringAudio,
            EngineState::RecoveringGpu,
            EngineState::RecoveringDesktopHost,
            EngineState::AnalysisFallback,
        ] {
            assert!(
                !state.is_reachable_at_gate_0(),
                "{state} must not be reachable at Gate 0"
            );
        }
    }

    #[test]
    fn state_names_are_unique() {
        let states = [
            EngineState::Stopped,
            EngineState::Starting,
            EngineState::IdleReady,
            EngineState::Listening,
            EngineState::AudioActive,
            EngineState::Rendering,
            EngineState::PausedByUser,
            EngineState::PausedByFullscreen,
            EngineState::PausedByPowerPolicy,
            EngineState::PausedByDisplaySleep,
            EngineState::WaitingForPermission,
            EngineState::RecoveringAudio,
            EngineState::RecoveringGpu,
            EngineState::RecoveringDesktopHost,
            EngineState::AnalysisFallback,
            EngineState::ErrorRecoverable,
        ];
        let mut names: Vec<&str> = states.iter().map(|s| s.name()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate state name");
    }

    #[test]
    fn state_error_reports_both_states() {
        let error = StateError::new(EngineState::Stopped, EngineState::Rendering);
        assert_eq!(error.from, EngineState::Stopped);
        assert_eq!(error.to, EngineState::Rendering);
        assert!(error.to_string().contains("Stopped"));
        assert!(error.to_string().contains("Rendering"));
    }
}
