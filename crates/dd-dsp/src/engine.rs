//! The deterministic feature extractor.
//!
//! Feed mono `f32` samples at a known rate; get one [`FeatureFrame`] per hop.
//! The same input and configuration always produce the same output for a given
//! binary — there is no randomness, no time-of-day dependence and no adaptive
//! state that depends on when the process started.
//!
//! # Signal path
//!
//! ```text
//! mono samples -> hop buffer -> sliding window (FFT size, 50% overlap)
//!              -> Hann window -> radix-2 FFT -> magnitude spectrum
//!              -> six band amplitudes -> dBFS -> 0..1 -> envelope followers
//!              -> positive spectral flux -> onset events
//!              -> RMS / peak -> critically damped smoothing
//! ```

use serde::Serialize;

use crate::bands::{BAND_COUNT, BandMap, amplitude_to_unit};
use crate::envelope::EnvelopeFollower;
use crate::fft::{Complex, Fft, FftError};
use crate::flux::{OnsetDetector, SpectralFlux};
use crate::spring::SpringDamper;
use crate::window::HannWindow;

/// Errors constructing a [`DspEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DspError {
    /// The FFT size is not usable.
    #[error("invalid FFT size {0}: expected a power of two of at least 64")]
    InvalidFftSize(usize),
    /// The hop size is not usable.
    #[error("invalid hop size {0}: must be in 1..=fft_size")]
    InvalidHopSize(usize),
    /// The sample rate is not usable.
    #[error("invalid sample rate {0}: must be at least 8000 Hz")]
    InvalidSampleRate(u32),
    /// The underlying FFT could not be built.
    #[error("fft: {0}")]
    Fft(#[from] FftError),
}

/// Tunable analysis parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DspConfig {
    /// FFT size. Must be a power of two.
    pub fft_size: usize,
    /// Samples advanced per analysis frame.
    pub hop_size: usize,
    /// Envelope attack time constant in milliseconds.
    pub attack_ms: f32,
    /// Envelope release time constant in milliseconds.
    pub release_ms: f32,
    /// Normalised flux required to fire an onset.
    pub onset_threshold: f32,
    /// Minimum spacing between onsets, in milliseconds.
    pub onset_refractory_ms: f32,
    /// Level at or below which the frame is considered silent, in dBFS.
    pub silence_floor_dbfs: f32,
    /// Shape of the flux compression curve.
    pub flux_sensitivity: f32,
    /// Time for the smoothed level to settle, in milliseconds.
    pub spring_settle_ms: f32,
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 1024,
            attack_ms: 10.0,
            release_ms: 220.0,
            onset_threshold: 0.35,
            onset_refractory_ms: 120.0,
            silence_floor_dbfs: -60.0,
            flux_sensitivity: 12.0,
            spring_settle_ms: 120.0,
        }
    }
}

impl DspConfig {
    /// Validates the configuration against a sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`DspError`] describing the first invalid parameter.
    pub fn validate(&self, sample_rate: u32) -> Result<(), DspError> {
        if sample_rate < 8_000 {
            return Err(DspError::InvalidSampleRate(sample_rate));
        }
        if self.fft_size < 64 || !self.fft_size.is_power_of_two() {
            return Err(DspError::InvalidFftSize(self.fft_size));
        }
        if self.hop_size == 0 || self.hop_size > self.fft_size {
            return Err(DspError::InvalidHopSize(self.hop_size));
        }
        Ok(())
    }
}

/// One analysis frame. This is the only thing the renderer consumes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFrame {
    /// Caller-supplied timestamp, nanoseconds.
    pub timestamp_ns: u64,
    /// Monotonic frame counter since the engine was created.
    pub frame_index: u64,
    /// The rate the samples were supplied at.
    pub sample_rate: u32,
    /// Root-mean-square level of the analysis window, linear `0..=1`-ish.
    pub level_rms: f32,
    /// Peak absolute sample value in the analysis window.
    pub level_peak: f32,
    /// `level_rms` in dBFS.
    pub loudness_dbfs: f32,
    /// Spring-smoothed level, for motion that has mass.
    pub smoothed_level: f32,
    /// Six band values, already normalised to `0..=1`.
    pub bands: [f32; BAND_COUNT],
    /// The same six bands after attack/release smoothing.
    pub band_envelopes: [f32; BAND_COUNT],
    /// Positive spectral flux, normalised `0..=1`.
    pub spectral_flux: f32,
    /// Whether an onset fired on this frame.
    pub onset: bool,
    /// The flux value at the moment the onset fired, else 0.
    pub onset_strength: f32,
    /// Whether the frame is at or below the silence floor.
    pub is_silent: bool,
}

impl FeatureFrame {
    /// A silent frame, used as the initial and post-reset value.
    #[must_use]
    pub const fn silent(sample_rate: u32, timestamp_ns: u64) -> Self {
        Self {
            timestamp_ns,
            frame_index: 0,
            sample_rate,
            level_rms: 0.0,
            level_peak: 0.0,
            loudness_dbfs: -120.0,
            smoothed_level: 0.0,
            bands: [0.0; BAND_COUNT],
            band_envelopes: [0.0; BAND_COUNT],
            spectral_flux: 0.0,
            onset: false,
            onset_strength: 0.0,
            is_silent: true,
        }
    }

    /// Whether every numeric field is finite. A frame that fails this must never
    /// reach the renderer.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.level_rms.is_finite()
            && self.level_peak.is_finite()
            && self.loudness_dbfs.is_finite()
            && self.smoothed_level.is_finite()
            && self.spectral_flux.is_finite()
            && self.onset_strength.is_finite()
            && self.bands.iter().all(|v| v.is_finite())
            && self.band_envelopes.iter().all(|v| v.is_finite())
    }
}

/// The analysis engine. Owns all buffers; performs no allocation while running.
#[derive(Debug)]
pub struct DspEngine {
    sample_rate: u32,
    config: DspConfig,
    fft: Fft,
    window: HannWindow,
    /// `fft_size` most recent samples, oldest first.
    history: Vec<f32>,
    /// Working buffer of `fft_size` windowed samples.
    windowed: Vec<f32>,
    /// `hop_size` samples waiting to be analysed.
    pending: Vec<f32>,
    scratch: Vec<Complex>,
    magnitudes: Vec<f32>,
    band_map: BandMap,
    band_amplitudes: [f32; BAND_COUNT],
    envelopes: [EnvelopeFollower; BAND_COUNT],
    flux: SpectralFlux,
    onset: OnsetDetector,
    spring: SpringDamper,
    hop_seconds: f32,
    magnitude_scale: f32,
    frame_index: u64,
    produced: u64,
}

impl DspEngine {
    /// Builds an engine for `sample_rate`.
    ///
    /// # Errors
    ///
    /// Returns [`DspError`] if the configuration or the sample rate is unusable.
    pub fn new(sample_rate: u32, config: DspConfig) -> Result<Self, DspError> {
        config.validate(sample_rate)?;

        let fft = Fft::new(config.fft_size)?;
        let window = HannWindow::new(config.fft_size);
        let hop_seconds = config.hop_size as f32 / sample_rate as f32;
        let hop_ms = hop_seconds * 1000.0;
        let bin_hz = sample_rate as f32 / config.fft_size as f32;
        let bin_count = fft.bin_count();

        // Scale so a full-scale sine reads ~1.0 in its peak bin.
        let magnitude_scale = 2.0 / (config.fft_size as f32 * window.coherent_gain());

        Ok(Self {
            sample_rate,
            config,
            window,
            history: vec![0.0; config.fft_size],
            windowed: vec![0.0; config.fft_size],
            pending: Vec::with_capacity(config.hop_size),
            scratch: vec![Complex::default(); config.fft_size],
            magnitudes: vec![0.0; bin_count],
            band_map: BandMap::new(bin_hz, bin_count),
            band_amplitudes: [0.0; BAND_COUNT],
            envelopes: [EnvelopeFollower::new(config.attack_ms, config.release_ms, hop_seconds);
                BAND_COUNT],
            flux: SpectralFlux::new(bin_count, config.flux_sensitivity),
            onset: OnsetDetector::new(config.onset_threshold, config.onset_refractory_ms, hop_ms),
            spring: SpringDamper::new(config.spring_settle_ms, hop_seconds),
            fft,
            hop_seconds,
            magnitude_scale,
            frame_index: 0,
            produced: 0,
        })
    }

    /// The configured sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The hop duration in seconds.
    #[must_use]
    pub fn hop_seconds(&self) -> f32 {
        self.hop_seconds
    }

    /// The hop rate in frames per second.
    #[must_use]
    pub fn hop_rate_hz(&self) -> f32 {
        1.0 / self.hop_seconds
    }

    /// How many frames have been produced.
    #[must_use]
    pub fn frames_produced(&self) -> u64 {
        self.produced
    }

    /// The bin spacing in Hz, exposed for diagnostics.
    #[must_use]
    pub fn bin_hz(&self) -> f32 {
        self.sample_rate as f32 / self.config.fft_size as f32
    }

    /// Clears all analysis state, e.g. after a device change.
    ///
    /// Keeps the configuration and buffers; the next frame is treated as the
    /// first one seen, so no false onset is produced by the discontinuity.
    pub fn reset(&mut self) {
        self.history.fill(0.0);
        self.pending.clear();
        self.magnitudes.fill(0.0);
        self.band_amplitudes = [0.0; BAND_COUNT];
        for envelope in &mut self.envelopes {
            envelope.reset();
        }
        self.flux.reset();
        self.spring.reset_to(0.0);
    }

    /// Pushes mono samples and returns the most recent frame if a hop completed.
    ///
    /// Non-finite input samples are replaced with 0.0 rather than propagating
    /// NaN into the spectra; the frame is always finite.
    pub fn push(&mut self, samples: &[f32], timestamp_ns: u64) -> Option<FeatureFrame> {
        let mut latest = None;

        for &sample in samples {
            let value = if sample.is_finite() { sample } else { 0.0 };
            self.pending.push(value);
            // `pending` is reserved to `hop_size`, so this push cannot allocate.
            if self.pending.len() == self.config.hop_size {
                latest = Some(self.run_hop(timestamp_ns));
                self.pending.clear();
            }
        }

        latest
    }

    /// Runs one analysis hop over the current window.
    fn run_hop(&mut self, timestamp_ns: u64) -> FeatureFrame {
        let fft_size = self.config.fft_size;
        let hop = self.config.hop_size;

        // Slide the history window: drop the oldest `hop` samples, append the
        // new ones.
        self.history.copy_within(hop.., 0);
        let tail = fft_size - hop;
        self.history[tail..].copy_from_slice(&self.pending);

        // Time-domain level over the raw window.
        let mut sum_squares = 0.0_f32;
        let mut peak = 0.0_f32;
        for &value in &self.history {
            sum_squares += value * value;
            let magnitude = value.abs();
            if magnitude > peak {
                peak = magnitude;
            }
        }
        let rms = (sum_squares / fft_size as f32).sqrt();
        let loudness_dbfs = if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -120.0
        };
        let is_silent = loudness_dbfs <= self.config.silence_floor_dbfs;

        // Spectrum.
        self.window.apply(&self.history, &mut self.windowed);
        self.fft.magnitudes(
            &self.windowed,
            &mut self.scratch,
            &mut self.magnitudes,
            self.magnitude_scale,
        );

        // Bands and their envelopes.
        self.band_map
            .amplitudes(&self.magnitudes, &mut self.band_amplitudes);
        let floor = self.config.silence_floor_dbfs;
        let mut bands = [0.0_f32; BAND_COUNT];
        let mut band_envelopes = [0.0_f32; BAND_COUNT];
        for index in 0..BAND_COUNT {
            let unit = amplitude_to_unit(self.band_amplitudes[index], floor);
            bands[index] = unit;
            band_envelopes[index] = self.envelopes[index].process(unit);
        }

        // Transients.
        let flux = self.flux.update(&self.magnitudes);
        let onset_strength = self.onset.update(flux);
        let onset = onset_strength.is_some();

        self.frame_index += 1;
        self.produced += 1;

        let mut frame = FeatureFrame {
            timestamp_ns,
            frame_index: self.frame_index,
            sample_rate: self.sample_rate,
            level_rms: rms,
            level_peak: peak,
            loudness_dbfs,
            smoothed_level: self.spring.process(rms),
            bands,
            band_envelopes,
            spectral_flux: flux,
            onset,
            onset_strength: onset_strength.unwrap_or(0.0),
            is_silent,
        };

        // Guarantee the contract even if a future change introduces a NaN.
        if !frame.is_finite() {
            frame = FeatureFrame::silent(self.sample_rate, timestamp_ns);
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn engine() -> DspEngine {
        DspEngine::new(RATE, DspConfig::default()).expect("default config is valid")
    }

    /// Feeds `samples` and returns every frame produced.
    fn frames(engine: &mut DspEngine, samples: &[f32]) -> Vec<FeatureFrame> {
        let mut out = Vec::new();
        let hop = engine.config.hop_size;
        for chunk in samples.chunks(hop) {
            if let Some(frame) = engine.push(chunk, 0) {
                out.push(frame);
            }
        }
        out
    }

    fn sine(frequency: f32, amplitude: f32, samples: usize, rate: u32) -> Vec<f32> {
        (0..samples)
            .map(|n| {
                let t = n as f32 / rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * amplitude
            })
            .collect()
    }

    #[test]
    fn rejects_invalid_configurations() {
        assert!(matches!(
            DspEngine::new(
                48_000,
                DspConfig {
                    fft_size: 1000,
                    ..Default::default()
                }
            ),
            Err(DspError::InvalidFftSize(1000))
        ));
        assert!(matches!(
            DspEngine::new(
                48_000,
                DspConfig {
                    hop_size: 0,
                    ..Default::default()
                }
            ),
            Err(DspError::InvalidHopSize(0))
        ));
        assert!(matches!(
            DspEngine::new(
                48_000,
                DspConfig {
                    hop_size: 4096,
                    ..Default::default()
                }
            ),
            Err(DspError::InvalidHopSize(4096))
        ));
        assert!(matches!(
            DspEngine::new(4_000, DspConfig::default()),
            Err(DspError::InvalidSampleRate(4_000))
        ));
    }

    #[test]
    fn hop_and_bin_geometry_match_the_documented_numbers() {
        let engine = engine();
        assert!((engine.hop_seconds() - 1024.0 / 48_000.0).abs() < 1e-9);
        assert!((engine.hop_rate_hz() - 46.875).abs() < 1e-3);
        assert!((engine.bin_hz() - 23.4375).abs() < 1e-4);
    }

    #[test]
    fn silence_produces_silent_finite_frames() {
        let mut engine = engine();
        let produced = frames(&mut engine, &vec![0.0_f32; 48_000]);
        assert!(
            produced.len() > 20,
            "expected many frames, got {}",
            produced.len()
        );
        for frame in &produced {
            assert!(frame.is_finite());
            assert!(frame.is_silent);
            assert_eq!(frame.bands, [0.0; BAND_COUNT]);
            assert_eq!(frame.band_envelopes, [0.0; BAND_COUNT]);
            assert_eq!(frame.spectral_flux, 0.0);
            assert!(!frame.onset);
        }
    }

    /// Feeds a tone long enough for the envelopes to settle, then returns the
    /// last frame.
    fn settled_tone(frequency: f32) -> FeatureFrame {
        let mut engine = engine();
        let samples = sine(frequency, 0.5, RATE as usize, RATE);
        let produced = frames(&mut engine, &samples);
        *produced.last().expect("frames were produced")
    }

    #[test]
    fn each_zone_receives_its_own_tone() {
        // (frequency, expected dominant band index)
        let cases = [
            (40.0_f32, 0_usize),
            (100.0, 1),
            (250.0, 2),
            (1_000.0, 3),
            (4_000.0, 4),
            (10_000.0, 5),
        ];

        for (frequency, expected) in cases {
            let frame = settled_tone(frequency);
            let (dominant, value) =
                frame
                    .bands
                    .iter()
                    .enumerate()
                    .fold(
                        (0, 0.0_f32),
                        |acc, (i, &v)| {
                            if v > acc.1 { (i, v) } else { acc }
                        },
                    );
            assert_eq!(
                dominant,
                expected,
                "{frequency} Hz should dominate band {expected} ({}), bands were {:?}",
                crate::bands::BANDS[expected].key,
                frame.bands
            );
            assert!(
                value > 0.5,
                "{frequency} Hz produced only {value} in its band"
            );
        }
    }

    #[test]
    fn a_tone_does_not_light_up_distant_bands() {
        let frame = settled_tone(1_000.0);
        // Presence is 500-2500 Hz; sub-bass and air must stay dark.
        assert!(frame.bands[0] < 0.2, "sub-bass {}", frame.bands[0]);
        assert!(frame.bands[5] < 0.2, "air {}", frame.bands[5]);
    }

    #[test]
    fn values_are_always_bounded_and_finite() {
        let mut engine = engine();
        // Deliberately hostile input.
        let mut samples = Vec::new();
        samples.extend(sine(50.0, 1.0, 4_800, RATE));
        samples.extend(vec![f32::NAN; 1_024]);
        samples.extend(vec![f32::INFINITY; 1_024]);
        samples.extend(vec![-1.0e9; 1_024]);
        samples.extend(sine(12_000.0, 1.0, 4_800, RATE));
        samples.extend(vec![0.0; 4_800]);

        for frame in frames(&mut engine, &samples) {
            assert!(frame.is_finite(), "frame was not finite: {frame:?}");
            for value in frame.bands.iter().chain(frame.band_envelopes.iter()) {
                assert!((0.0..=1.0).contains(value), "value out of range: {value}");
            }
            assert!((0.0..=1.0).contains(&frame.spectral_flux));
            assert!(frame.level_rms >= 0.0);
        }
    }

    #[test]
    fn output_is_deterministic_for_identical_input() {
        let samples = sine(440.0, 0.4, 24_000, RATE);
        let mut a = engine();
        let mut b = engine();
        let first = frames(&mut a, &samples);
        let second = frames(&mut b, &samples);
        assert_eq!(first.len(), second.len());
        assert_eq!(first, second, "identical input must give identical frames");
    }

    #[test]
    fn different_sample_rates_scale_the_band_mapping() {
        // A 1 kHz tone must land in "presence" at 44.1 kHz too, which only works
        // if the band map is derived from the real rate.
        let mut engine = DspEngine::new(44_100, DspConfig::default()).expect("valid");
        let samples = sine(1_000.0, 0.5, 44_100, 44_100);
        let produced = frames(&mut engine, &samples);
        let last = produced.last().expect("frames");
        let (dominant, _) =
            last.bands.iter().enumerate().fold(
                (0, 0.0_f32),
                |acc, (i, &v)| if v > acc.1 { (i, v) } else { acc },
            );
        assert_eq!(dominant, 3, "bands were {:?}", last.bands);
    }

    #[test]
    fn an_impulse_fires_an_onset_but_a_steady_tone_does_not() {
        // Steady tone: after settling, no onsets.
        let mut steady = engine();
        let tone = sine(600.0, 0.5, RATE as usize, RATE);
        let steady_frames = frames(&mut steady, &tone);
        let late_onsets = steady_frames
            .iter()
            .skip(steady_frames.len() / 2)
            .filter(|f| f.onset)
            .count();
        assert_eq!(late_onsets, 0, "a steady tone must not keep firing onsets");

        // Silence, then a burst: an onset must fire at the burst.
        let mut burst = engine();
        let mut samples = vec![0.0_f32; 8_192];
        samples.extend(sine(600.0, 0.9, 4_096, RATE));
        let burst_frames = frames(&mut burst, &samples);
        assert!(
            burst_frames.iter().any(|f| f.onset),
            "a sudden burst should produce an onset"
        );
    }

    #[test]
    fn onset_strength_is_only_set_when_an_onset_fires() {
        let mut engine = engine();
        let mut samples = vec![0.0_f32; 8_192];
        samples.extend(sine(800.0, 0.9, 4_096, RATE));
        for frame in frames(&mut engine, &samples) {
            if frame.onset {
                assert!(frame.onset_strength > 0.0);
            } else {
                assert_eq!(frame.onset_strength, 0.0);
            }
        }
    }

    #[test]
    fn silence_after_energy_returns_to_a_stable_baseline() {
        let mut engine = engine();
        let mut samples = sine(200.0, 0.7, 24_000, RATE);
        // Three seconds of silence: a 220 ms release needs several time constants
        // to reach the noise floor, and the assertion below requires it to.
        samples.extend(vec![0.0_f32; 3 * RATE as usize]);
        let produced = frames(&mut engine, &samples);
        let tail = &produced[produced.len() - 20..];
        for frame in tail {
            assert!(frame.is_silent, "should be silent: {frame:?}");
            assert_eq!(frame.bands, [0.0; BAND_COUNT]);
            assert!(
                frame.band_envelopes.iter().all(|v| *v < 1e-3),
                "envelopes should have decayed: {:?}",
                frame.band_envelopes
            );
        }
    }

    #[test]
    fn reset_clears_state_between_sessions() {
        let mut engine = engine();
        let tone = sine(500.0, 0.8, 24_000, RATE);
        let produced = frames(&mut engine, &tone);
        assert!(produced.last().expect("frames").bands[3] > 0.1);
        engine.reset();
        let after = frames(&mut engine, &vec![0.0_f32; 4_096]);
        for frame in after {
            assert_eq!(frame.bands, [0.0; BAND_COUNT]);
        }
    }

    #[test]
    fn arbitrary_chunk_sizes_produce_the_same_frames() {
        // The engine must be insensitive to how the capture layer splits data.
        let samples = sine(300.0, 0.5, 12_000, RATE);
        let mut even = engine();
        let frames_even = frames(&mut even, &samples);

        let mut odd = engine();
        let mut frames_odd = Vec::new();
        for chunk in samples.chunks(333) {
            if let Some(frame) = odd.push(chunk, 0) {
                frames_odd.push(frame);
            }
        }

        assert_eq!(frames_even.len(), frames_odd.len());
        assert_eq!(frames_even, frames_odd);
    }

    #[test]
    fn a_silent_frame_is_constructible_and_finite() {
        let frame = FeatureFrame::silent(RATE, 42);
        assert!(frame.is_finite());
        assert!(frame.is_silent);
        assert_eq!(frame.timestamp_ns, 42);
    }
}
