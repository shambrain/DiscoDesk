//! The capture runtime: ownership, threading, recovery and telemetry.
//!
//! Two threads:
//!
//! * **capture** owns COM, the WASAPI session and the producer end of the ring.
//!   It re-establishes the session when the default endpoint changes, when the
//!   endpoint is invalidated, or when a read fails.
//! * **dsp** owns the consumer end and the [`dd_dsp::DspEngine`], and publishes
//!   the newest feature frame.
//!
//! Neither thread logs, allocates per packet, or blocks on the other. The ring is
//! bounded and drops are counted.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dd_dsp::{DspConfig, DspEngine, FeatureFrame};
use serde::Serialize;

use crate::backend::{CaptureSession, DeviceInfo, ReadOutcome};
use crate::error::AudioError;
use crate::format::AudioFormat;
use crate::ring::MonoRing;
use crate::stats::{CaptureStats, StatsSnapshot};

/// Samples buffered between capture and DSP. One second at 48 kHz, so a brief
/// stall in the DSP thread cannot lose audio.
const DEFAULT_RING_CAPACITY: usize = 48_000;

/// How long the capture thread waits before retrying after a failure.
const RESTART_BACKOFF: Duration = Duration::from_millis(1000);

/// How long the DSP thread sleeps when the ring is empty.
const DSP_IDLE_SLEEP: Duration = Duration::from_millis(5);

/// Lifecycle of the capture runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureState {
    /// Not started, or stopped.
    Stopped,
    /// Opening the endpoint.
    Starting,
    /// Receiving packets.
    Streaming,
    /// The endpoint reported silence.
    Silent,
    /// Re-establishing after a device change or failure.
    Recovering,
    /// No endpoint is available.
    Unavailable,
    /// Capture failed and is not recoverable on this platform/build.
    Failed,
}

impl CaptureState {
    /// A stable short name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::Starting => "Starting",
            Self::Streaming => "Streaming",
            Self::Silent => "Silent",
            Self::Recovering => "Recovering",
            Self::Unavailable => "Unavailable",
            Self::Failed => "Failed",
        }
    }
}

/// Configuration for a capture runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioRuntimeConfig {
    /// DSP parameters.
    pub dsp: DspConfig,
    /// Ring capacity in samples.
    pub ring_capacity: usize,
}

impl Default for AudioRuntimeConfig {
    fn default() -> Self {
        Self {
            dsp: DspConfig::default(),
            ring_capacity: DEFAULT_RING_CAPACITY,
        }
    }
}

/// Everything observable about the audio path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTelemetry {
    /// Lifecycle state.
    pub state: CaptureState,
    /// The backend actually in use, e.g. `wasapi-loopback`.
    pub backend: Option<String>,
    /// Device name, when known.
    pub device_name: Option<String>,
    /// Device id, when known.
    pub device_id: Option<String>,
    /// Negotiated sample rate.
    pub sample_rate: Option<u32>,
    /// Negotiated channel count.
    pub channels: Option<u16>,
    /// Negotiated encoding name.
    pub encoding: Option<String>,
    /// Counters.
    pub stats: StatsSnapshot,
    /// Ring occupancy in samples.
    pub buffer_fill: usize,
    /// Ring capacity in samples.
    pub buffer_capacity: usize,
    /// The newest feature frame, if any.
    pub frame: Option<FeatureFrame>,
    /// Milliseconds since the newest frame was published.
    pub frame_age_ms: Option<u64>,
    /// Duration of the most recent DSP hop, microseconds.
    pub dsp_last_micros: u64,
    /// Worst DSP hop seen since start, microseconds.
    pub dsp_max_micros: u64,
    /// Frames the DSP has produced.
    pub dsp_frames: u64,
    /// Hop rate in Hz, derived from the negotiated rate.
    pub hop_rate_hz: Option<f32>,
}

/// Mutable state shared between the runtime threads and telemetry.
#[derive(Debug)]
struct Shared {
    stats: CaptureStats,
    state: Mutex<CaptureState>,
    device: Mutex<Option<DeviceInfo>>,
    format: Mutex<Option<AudioFormat>>,
    backend: Mutex<Option<String>>,
    latest: Mutex<Option<FeatureFrame>>,
    last_frame_at: Mutex<Option<Instant>>,
    dsp_last_micros: AtomicU64,
    dsp_max_micros: AtomicU64,
    dsp_frames: AtomicU64,
}

impl Shared {
    fn new() -> Self {
        Self {
            stats: CaptureStats::default(),
            state: Mutex::new(CaptureState::Stopped),
            device: Mutex::new(None),
            format: Mutex::new(None),
            backend: Mutex::new(None),
            latest: Mutex::new(None),
            last_frame_at: Mutex::new(None),
            dsp_last_micros: AtomicU64::new(0),
            dsp_max_micros: AtomicU64::new(0),
            dsp_frames: AtomicU64::new(0),
        }
    }

    fn set_state(&self, state: CaptureState) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = state;
        }
    }

    fn state(&self) -> CaptureState {
        self.state
            .lock()
            .map(|guard| *guard)
            .unwrap_or(CaptureState::Failed)
    }

    fn publish(&self, frame: FeatureFrame, duration: Duration) {
        let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
        self.dsp_last_micros.store(micros, Ordering::Relaxed);
        // Track the worst case without a lock: a plain compare-and-max loop.
        let mut current = self.dsp_max_micros.load(Ordering::Relaxed);
        while micros > current {
            match self.dsp_max_micros.compare_exchange_weak(
                current,
                micros,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        self.dsp_frames.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = self.latest.lock() {
            *guard = Some(frame);
        }
        if let Ok(mut guard) = self.last_frame_at.lock() {
            *guard = Some(Instant::now());
        }
    }
}

/// A running capture runtime.
#[derive(Debug)]
pub struct AudioRuntime {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
    ring: Arc<MonoRing>,
    config: AudioRuntimeConfig,
}

impl AudioRuntime {
    /// Starts capture and DSP.
    ///
    /// Returns immediately; the threads report progress through telemetry. On a
    /// platform without capture the runtime starts and reports `Unavailable`
    /// rather than failing to construct.
    #[must_use]
    pub fn start(config: AudioRuntimeConfig) -> Self {
        let shared = Arc::new(Shared::new());
        let stop = Arc::new(AtomicBool::new(false));
        let ring = Arc::new(MonoRing::with_capacity(config.ring_capacity));

        let capture_shared = Arc::clone(&shared);
        let capture_stop = Arc::clone(&stop);
        let capture_ring = Arc::clone(&ring);
        let capture_thread = thread::Builder::new()
            .name("dd-audio-capture".to_owned())
            .spawn(move || capture_loop(capture_shared, capture_stop, capture_ring))
            .expect("the capture thread could not be spawned");

        let dsp_shared = Arc::clone(&shared);
        let dsp_stop = Arc::clone(&stop);
        let dsp_ring = Arc::clone(&ring);
        let dsp_thread = thread::Builder::new()
            .name("dd-audio-dsp".to_owned())
            .spawn(move || dsp_loop(dsp_shared, dsp_stop, dsp_ring, config.dsp))
            .expect("the dsp thread could not be spawned");

        Self {
            shared,
            stop,
            threads: vec![capture_thread, dsp_thread],
            ring,
            config,
        }
    }

    /// The current telemetry snapshot.
    #[must_use]
    pub fn telemetry(&self) -> AudioTelemetry {
        let state = self.shared.state();
        let device = self.shared.device.lock().ok().and_then(|g| g.clone());
        let format = self.shared.format.lock().ok().and_then(|g| g.clone());
        let backend = self.shared.backend.lock().ok().and_then(|g| g.clone());
        let frame = self.shared.latest.lock().ok().and_then(|g| *g);
        let frame_age_ms =
            self.shared.last_frame_at.lock().ok().and_then(|g| {
                g.map(|t| u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX))
            });

        AudioTelemetry {
            state,
            backend,
            device_name: device.as_ref().map(|d| d.name.clone()),
            device_id: device.as_ref().map(|d| d.id.clone()),
            sample_rate: format.as_ref().map(|f| f.sample_rate),
            channels: format.as_ref().map(|f| f.channels),
            encoding: format.as_ref().map(|f| f.encoding.name().to_owned()),
            stats: self.shared.stats.snapshot(),
            buffer_fill: self.ring.len(),
            buffer_capacity: self.ring.capacity(),
            frame,
            frame_age_ms,
            dsp_last_micros: self.shared.dsp_last_micros.load(Ordering::Relaxed),
            dsp_max_micros: self.shared.dsp_max_micros.load(Ordering::Relaxed),
            dsp_frames: self.shared.dsp_frames.load(Ordering::Relaxed),
            hop_rate_hz: format
                .as_ref()
                .map(|f| f.sample_rate as f32 / self.config.dsp.hop_size as f32),
        }
    }

    /// Signals both threads and joins them.
    ///
    /// Bounded by construction: the capture thread wakes at least every 250 ms
    /// and the DSP thread every 5 ms, so both observe the stop flag promptly.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
        self.shared.set_state(CaptureState::Stopped);
    }
}

impl Drop for AudioRuntime {
    fn drop(&mut self) {
        // If `stop` was not called explicitly, do the same thing: signal and
        // join. Leaving detached threads would outlive the engine.
        self.stop.store(true, Ordering::Release);
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

/// The capture thread body.
fn capture_loop(shared: Arc<Shared>, stop: Arc<AtomicBool>, ring: Arc<MonoRing>) {
    #[cfg(windows)]
    let _com = match crate::backend::wasapi::initialise_com() {
        Ok(guard) => Some(guard),
        Err(err) => {
            shared.stats.set_error(err.to_string());
            shared.set_state(CaptureState::Failed);
            return;
        }
    };

    let mut packet = Vec::with_capacity(16_384);
    let mut first_attempt = true;

    while !stop.load(Ordering::Acquire) {
        if !first_attempt {
            shared.set_state(CaptureState::Recovering);
        }
        first_attempt = false;

        match crate::backend::open_default_loopback() {
            Ok(mut session) => {
                shared.stats.clear_error();
                publish_session(&shared, session.as_ref());
                shared.set_state(CaptureState::Streaming);

                let end_reason = run_session(&shared, &stop, &ring, session.as_mut(), &mut packet);
                shared.stats.restarts.fetch_add(1, Ordering::Relaxed);

                if stop.load(Ordering::Acquire) {
                    break;
                }

                match end_reason {
                    SessionEnd::DeviceChanged => {
                        shared.stats.device_changes.fetch_add(1, Ordering::Relaxed);
                        shared
                            .stats
                            .set_error("the default output device changed; reconnecting");
                    }
                    SessionEnd::Failed(err) => {
                        shared.stats.set_error(err.to_string());
                        shared.set_state(if err.is_recoverable() {
                            CaptureState::Recovering
                        } else {
                            CaptureState::Failed
                        });
                    }
                    SessionEnd::Stopped => break,
                }
            }
            Err(err) => {
                shared.stats.set_error(err.to_string());
                shared.set_state(if err.is_recoverable() {
                    CaptureState::Unavailable
                } else {
                    CaptureState::Failed
                });
            }
        }

        // Back off before retrying, but stay responsive to stop.
        let deadline = Instant::now() + RESTART_BACKOFF;
        while Instant::now() < deadline && !stop.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(50));
        }
    }

    shared.set_state(CaptureState::Stopped);
}

/// Why a session loop ended.
enum SessionEnd {
    Stopped,
    DeviceChanged,
    Failed(AudioError),
}

/// Copies the session's identity and format into shared state.
fn publish_session(shared: &Shared, session: &dyn CaptureSession) {
    if let Ok(mut guard) = shared.device.lock() {
        *guard = Some(session.device().clone());
    }
    if let Ok(mut guard) = shared.format.lock() {
        *guard = Some(session.format().clone());
    }
    if let Ok(mut guard) = shared.backend.lock() {
        *guard = Some(session.backend().to_owned());
    }
}

/// How many zero samples represent `elapsed` of silence, capped so a long gap
/// cannot flood the ring.
///
/// WASAPI loopback delivers **no packets** while nothing is being rendered, so
/// without this the DSP would never see another hop and envelopes would freeze
/// at their last value - the wallpaper would keep reacting to audio that stopped.
fn silence_samples(elapsed: Duration, sample_rate: u32, cap_samples: usize) -> usize {
    let samples = elapsed.as_nanos().saturating_mul(u128::from(sample_rate)) / 1_000_000_000_u128;
    usize::try_from(samples)
        .unwrap_or(cap_samples)
        .min(cap_samples)
}

/// Runs one session until it ends.
fn run_session(
    shared: &Shared,
    stop: &AtomicBool,
    ring: &MonoRing,
    session: &mut dyn CaptureSession,
    packet: &mut Vec<f32>,
) -> SessionEnd {
    let sample_rate = session.format().sample_rate;
    let cap_samples = (sample_rate as usize).saturating_mul(2);
    let zeros: Vec<f32> = vec![0.0; cap_samples];
    let mut last_packet = Instant::now();

    loop {
        if stop.load(Ordering::Acquire) {
            return SessionEnd::Stopped;
        }

        match session.default_device_changed() {
            Ok(true) => return SessionEnd::DeviceChanged,
            Ok(false) => {}
            Err(err) => return SessionEnd::Failed(err),
        }

        packet.clear();
        match session.read_packet(packet) {
            Ok(ReadOutcome::Samples(count)) => {
                last_packet = Instant::now();
                shared.stats.packets.fetch_add(1, Ordering::Relaxed);
                shared
                    .stats
                    .samples
                    .fetch_add(count as u64, Ordering::Relaxed);
                let format = session.format().clone();
                shared.stats.frames.fetch_add(
                    (count as u64).saturating_mul(u64::from(format.channels)),
                    Ordering::Relaxed,
                );
                let accepted = ring.push_slice(packet);
                if accepted < packet.len() {
                    shared
                        .stats
                        .dropped_samples
                        .fetch_add((packet.len() - accepted) as u64, Ordering::Relaxed);
                }
                shared.set_state(CaptureState::Streaming);
            }
            Ok(ReadOutcome::Silence(count)) => {
                last_packet = Instant::now();
                shared.stats.packets.fetch_add(1, Ordering::Relaxed);
                shared.stats.silent_packets.fetch_add(1, Ordering::Relaxed);
                shared
                    .stats
                    .samples
                    .fetch_add(count as u64, Ordering::Relaxed);
                let accepted = ring.push_slice(packet);
                if accepted < packet.len() {
                    shared
                        .stats
                        .dropped_samples
                        .fetch_add((packet.len() - accepted) as u64, Ordering::Relaxed);
                }
                shared.set_state(CaptureState::Silent);
            }
            Ok(ReadOutcome::Timeout) => {
                // Nothing playing, or a rate-limited event: not an error. If
                // the gap is long enough that no packets are coming, synthesise
                // silence so the DSP keeps its cadence and decays to zero.
                let elapsed = last_packet.elapsed();
                if elapsed >= Duration::from_millis(100) {
                    let needed = silence_samples(elapsed, sample_rate, cap_samples);
                    if needed > 0 {
                        let accepted = ring.push_slice(&zeros[..needed]);
                        if accepted < needed {
                            shared
                                .stats
                                .dropped_samples
                                .fetch_add((needed - accepted) as u64, Ordering::Relaxed);
                        }
                    }
                    last_packet = Instant::now();
                    shared.set_state(CaptureState::Silent);
                }
            }
            Err(err) => return SessionEnd::Failed(err),
        }
    }
}

/// The DSP thread body.
fn dsp_loop(shared: Arc<Shared>, stop: Arc<AtomicBool>, ring: Arc<MonoRing>, config: DspConfig) {
    let epoch = Instant::now();
    let mut engine: Option<DspEngine> = None;
    let mut engine_rate: Option<u32> = None;
    let mut buffer = [0.0_f32; 2048];

    loop {
        let stopping = stop.load(Ordering::Acquire);

        let rate = shared
            .format
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|f| f.sample_rate));

        if let Some(rate) = rate
            && engine_rate != Some(rate)
        {
            match DspEngine::new(rate, config) {
                Ok(fresh) => {
                    engine = Some(fresh);
                    engine_rate = Some(rate);
                }
                Err(err) => {
                    shared
                        .stats
                        .set_error(format!("dsp initialisation failed: {err}"));
                    engine = None;
                    engine_rate = None;
                }
            }
        }

        let read = ring.pop_into(&mut buffer);
        if read == 0 {
            if stopping {
                break;
            }
            thread::sleep(DSP_IDLE_SLEEP);
            continue;
        }

        let Some(active) = engine.as_mut() else {
            // No engine yet (no format negotiated): drop the samples, do not
            // pretend to analyse them.
            continue;
        };

        let timestamp_ns = u64::try_from(epoch.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let started = Instant::now();
        if let Some(frame) = active.push(&buffer[..read], timestamp_ns) {
            shared.publish(frame, started.elapsed());
        }
    }
}

/// Wall-clock nanoseconds, for external consumers that need an absolute stamp.
#[must_use]
pub fn unix_time_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_states_have_stable_names() {
        assert_eq!(CaptureState::Streaming.name(), "Streaming");
        assert_eq!(CaptureState::Unavailable.name(), "Unavailable");
    }

    #[test]
    fn default_config_is_the_documented_one() {
        let config = AudioRuntimeConfig::default();
        assert_eq!(config.dsp.fft_size, 2048);
        assert_eq!(config.dsp.hop_size, 1024);
        assert_eq!(config.ring_capacity, 48_000);
    }

    #[test]
    fn starting_and_stopping_the_runtime_is_clean() {
        let runtime = AudioRuntime::start(AudioRuntimeConfig::default());
        // Telemetry is available immediately, whatever the platform does.
        let telemetry = runtime.telemetry();
        assert!(telemetry.buffer_capacity > 0);
        runtime.stop();
    }

    #[test]
    fn repeat_start_stop_does_not_panic() {
        for _ in 0..5 {
            let runtime = AudioRuntime::start(AudioRuntimeConfig {
                dsp: DspConfig::default(),
                ring_capacity: 4_096,
            });
            let _ = runtime.telemetry();
            runtime.stop();
        }
    }

    #[test]
    fn silence_samples_are_proportional_to_elapsed_time() {
        // 100 ms at 48 kHz is 4800 samples.
        assert_eq!(
            silence_samples(Duration::from_millis(100), 48_000, 96_000),
            4_800
        );
        // 21.333 ms is one hop at 48 kHz; integer truncation can land one short.
        let hop = silence_samples(Duration::from_nanos(21_333_333), 48_000, 96_000);
        assert!((1023..=1024).contains(&hop), "hop was {hop}");
        // 100 ms at 44.1 kHz.
        assert_eq!(
            silence_samples(Duration::from_millis(100), 44_100, 88_200),
            4_410
        );
    }

    #[test]
    fn silence_samples_are_capped() {
        assert_eq!(
            silence_samples(Duration::from_secs(60), 48_000, 96_000),
            96_000
        );
    }

    #[test]
    fn unix_time_is_nonzero() {
        assert!(unix_time_ns() > 0);
    }
}
