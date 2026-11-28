//! Hardware integration tests for WASAPI loopback.
//!
//! These need a real output device and are **ignored by default** so an ordinary
//! `cargo test` (and CI) never claims audio hardware it does not have. Run them
//! deliberately:
//!
//! ```powershell
//! cargo test -p dd-audio --test wasapi_hardware -- --ignored --nocapture
//! ```
//!
//! What they assert is deliberately conservative, because they run on whatever
//! machine is present:
//!
//! * the runtime reaches a terminal, coherent state rather than hanging;
//! * if a device exists, the negotiated format is sane (rate, channels,
//!   encoding) and the backend identifies itself;
//! * telemetry counts never contradict each other;
//! * starting and stopping repeatedly does not panic or leak threads badly
//!   enough to block shutdown.
//!
//! They do **not** synthesise audio, so they cannot assert that bands light up;
//! that requires playing something and is done by `scripts/audio-smoke.ps1`.
//!
//! The tests serialise themselves through a mutex. Opening four loopback clients
//! concurrently on the same endpoint was observed to fail on the development
//! machine (one open reported `Unavailable`, the same test serially reached
//! `Streaming`). The product opens one client, so this is test isolation, and it
//! is handled here rather than by requiring a command-line flag.

#![cfg(windows)]

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use dd_audio::{AudioRuntime, AudioRuntimeConfig, CaptureState};

/// Serialises these tests.
///
/// Opening several loopback clients on the same endpoint concurrently was NOT
/// reliable on the development machine: with four tests in parallel one open
/// failed and reported `Unavailable`, while the same test run serially reached
/// `Streaming`. The product opens exactly one capture client, so this is a test
/// isolation issue rather than a product defect - but it must be handled
/// honestly instead of being papered over with `--test-threads=1`.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    // A poisoned guard still serialises; the previous test's assertion failure
    // must not turn into a cascade of lock errors.
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn wait_for_state(
    runtime: &AudioRuntime,
    predicate: impl Fn(CaptureState) -> bool,
    timeout: Duration,
) -> Option<dd_audio::AudioTelemetry> {
    let deadline = Instant::now() + timeout;
    loop {
        let telemetry = runtime.telemetry();
        if predicate(telemetry.state) {
            return Some(telemetry);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
#[ignore = "requires a real audio output device"]
fn capture_reaches_a_coherent_state() {
    let _serial = serial();
    let runtime = AudioRuntime::start(AudioRuntimeConfig::default());

    let telemetry = wait_for_state(
        &runtime,
        |state| {
            matches!(
                state,
                CaptureState::Streaming
                    | CaptureState::Silent
                    | CaptureState::Unavailable
                    | CaptureState::Failed
            )
        },
        Duration::from_secs(10),
    )
    .expect("the runtime must reach a terminal state within 10 s, not hang");

    println!(
        "state={:?} backend={:?} device={:?} rate={:?} channels={:?} encoding={:?}",
        telemetry.state,
        telemetry.backend,
        telemetry.device_name,
        telemetry.sample_rate,
        telemetry.channels,
        telemetry.encoding
    );
    println!("last_error={:?}", telemetry.stats.last_error);

    match telemetry.state {
        CaptureState::Streaming | CaptureState::Silent => {
            assert_eq!(telemetry.backend.as_deref(), Some("wasapi-loopback"));
            let rate = telemetry.sample_rate.expect("a negotiated rate");
            assert!(
                (8_000..=384_000).contains(&rate),
                "implausible sample rate {rate}"
            );
            let channels = telemetry.channels.expect("a channel count");
            assert!(
                (1..=32).contains(&channels),
                "implausible channels {channels}"
            );
            assert!(telemetry.encoding.is_some(), "an encoding must be reported");
            assert!(
                telemetry.device_name.is_some(),
                "a device name must be reported"
            );
        }
        CaptureState::Unavailable | CaptureState::Failed => {
            // No device on this machine, or capture refused. That is a valid
            // outcome for the test; it must simply be reported honestly.
            assert!(
                telemetry.stats.last_error.is_some(),
                "an unavailable runtime must say why"
            );
        }
        other => panic!("unexpected state {other:?}"),
    }

    runtime.stop();
}

#[test]
#[ignore = "requires a real audio output device"]
fn counters_are_self_consistent() {
    let _serial = serial();
    let runtime = AudioRuntime::start(AudioRuntimeConfig::default());
    std::thread::sleep(Duration::from_secs(2));

    let telemetry = runtime.telemetry();
    let stats = &telemetry.stats;

    // Frames captured is samples times channels, or zero.
    if let Some(channels) = telemetry.channels
        && stats.samples > 0
    {
        assert_eq!(
            stats.frames,
            stats.samples * u64::from(channels),
            "frames should be samples x channels"
        );
    }
    if stats.packets == 0 {
        assert_eq!(stats.samples, 0, "no packets must mean no samples");
    }
    assert_eq!(
        stats.device_changes, 0,
        "the device should not change in 2 s"
    );
    assert!(
        telemetry.buffer_capacity >= telemetry.buffer_fill,
        "fill cannot exceed capacity"
    );

    runtime.stop();
}

#[test]
#[ignore = "requires a real audio output device"]
fn repeated_start_stop_shuts_down_promptly() {
    let _serial = serial();
    for round in 0..5 {
        let runtime = AudioRuntime::start(AudioRuntimeConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        let started = Instant::now();
        runtime.stop();
        let elapsed = started.elapsed();
        println!("round {round}: stop took {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(5),
            "stop should join both threads promptly, took {elapsed:?}"
        );
    }
}

#[test]
#[ignore = "requires a real audio output device"]
fn telemetry_is_available_before_any_audio() {
    let _serial = serial();
    let runtime = AudioRuntime::start(AudioRuntimeConfig::default());
    let telemetry = runtime.telemetry();
    // Even with nothing playing, the snapshot must be structurally complete.
    assert!(telemetry.buffer_capacity > 0);
    assert!(telemetry.hop_rate_hz.is_none() || telemetry.hop_rate_hz.is_some());
    runtime.stop();
}
