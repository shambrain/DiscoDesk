//! Positive spectral flux and onset detection.
//!
//! Spectral flux measures *change* between consecutive magnitude spectra, not
//! loudness. A steady tone produces near-zero flux once the spectrum settles,
//! which is what stops sustained loud music from firing constant transients.
//!
//! This is an onset detector, **not** a beat tracker (`ARCHITECTURE.md` §7.4).
//! It reports that "something started", never a tempo.

/// Half-wave-rectified positive spectral flux with deterministic normalisation.
#[derive(Debug, Clone)]
pub struct SpectralFlux {
    previous: Vec<f32>,
    primed: bool,
    sensitivity: f32,
    last_raw: f32,
    last_normalised: f32,
}

impl SpectralFlux {
    /// Creates an analyser for `bin_count` bins.
    ///
    /// `sensitivity` shapes the compression curve: larger values make small
    /// changes read as larger flux. It is not a gain on the audio.
    #[must_use]
    pub fn new(bin_count: usize, sensitivity: f32) -> Self {
        Self {
            previous: vec![0.0; bin_count],
            primed: false,
            sensitivity: sensitivity.max(0.01),
            last_raw: 0.0,
            last_normalised: 0.0,
        }
    }

    /// Updates with the current magnitudes and returns normalised flux in `0..=1`.
    ///
    /// The first call returns exactly 0.0: with no previous spectrum there is no
    /// change to measure, and reporting a spike would create a false onset on
    /// start-up or after a device change.
    pub fn update(&mut self, magnitudes: &[f32]) -> f32 {
        assert_eq!(
            magnitudes.len(),
            self.previous.len(),
            "magnitude length must match the analyser"
        );

        if !self.primed {
            self.previous.copy_from_slice(magnitudes);
            self.primed = true;
            self.last_raw = 0.0;
            self.last_normalised = 0.0;
            return 0.0;
        }

        let mut positive = 0.0_f32;
        let mut total = 0.0_f32;
        for (index, &magnitude) in magnitudes.iter().enumerate() {
            let delta = magnitude - self.previous[index];
            if delta > 0.0 {
                positive += delta;
            }
            total += magnitude;
            self.previous[index] = magnitude;
        }

        // Relative measure: the fraction of the current magnitude that arrived
        // this frame. Dividing by the bin count instead would dilute a real
        // onset, because almost every bin is empty in normal music - a 600 Hz
        // tone occupies a handful of 1025 bins and would read ~0.02.
        //
        // The small constant keeps silence-from-silence at 0 instead of 0/0.
        let raw = positive / (total + 1e-3);
        self.last_raw = raw;

        let normalised = if raw.is_finite() && raw > 0.0 {
            1.0 - (-self.sensitivity * raw).exp()
        } else {
            0.0
        };
        self.last_normalised = normalised.clamp(0.0, 1.0);
        self.last_normalised
    }

    /// The last unnormalised mean positive increment.
    #[must_use]
    pub fn raw(&self) -> f32 {
        self.last_raw
    }

    /// The last normalised flux.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.last_normalised
    }

    /// Drops the previous spectrum, so the next update returns 0.0.
    ///
    /// Used when the audio source restarts: comparing spectra across a device
    /// change would otherwise produce a large false onset.
    pub fn reset(&mut self) {
        self.primed = false;
        self.last_raw = 0.0;
        self.last_normalised = 0.0;
    }
}

/// Converts flux into discrete onset events with a refractory interval.
#[derive(Debug, Clone)]
pub struct OnsetDetector {
    threshold: f32,
    refractory_frames: u32,
    frames_since_onset: u32,
}

impl OnsetDetector {
    /// Creates a detector.
    ///
    /// `threshold` is compared against normalised flux. `refractory_ms` and
    /// `hop_ms` together set how many frames must pass before another onset can
    /// fire, which prevents one attack from producing a burst of events.
    #[must_use]
    pub fn new(threshold: f32, refractory_ms: f32, hop_ms: f32) -> Self {
        let frames = if hop_ms > 0.0 {
            (refractory_ms / hop_ms).round().max(1.0) as u32
        } else {
            1
        };
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            refractory_frames: frames,
            // Start "ready": the first eligible frame may fire.
            frames_since_onset: frames,
        }
    }

    /// Returns the onset strength (`flux`) when an onset fires, otherwise `None`.
    pub fn update(&mut self, flux: f32) -> Option<f32> {
        if !flux.is_finite() {
            return None;
        }
        if self.frames_since_onset < self.refractory_frames {
            self.frames_since_onset = self.frames_since_onset.saturating_add(1);
            return None;
        }
        if flux >= self.threshold && flux > 0.0 {
            self.frames_since_onset = 0;
            return Some(flux);
        }
        None
    }

    /// The threshold in normalised flux units.
    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// How many frames must separate two onsets.
    #[must_use]
    pub fn refractory_frames(&self) -> u32 {
        self.refractory_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_update_is_always_zero() {
        let mut flux = SpectralFlux::new(16, 12.0);
        let magnitudes = vec![0.5_f32; 16];
        assert_eq!(flux.update(&magnitudes), 0.0);
    }

    #[test]
    fn a_steady_spectrum_produces_no_flux() {
        let mut flux = SpectralFlux::new(16, 12.0);
        let magnitudes = vec![0.5_f32; 16];
        let _ = flux.update(&magnitudes);
        for _ in 0..20 {
            assert!(flux.update(&magnitudes) < 1e-6);
        }
    }

    #[test]
    fn a_rising_spectrum_produces_positive_flux() {
        let mut flux = SpectralFlux::new(16, 12.0);
        let _ = flux.update(&[0.0_f32; 16]);
        let rising = flux.update(&[0.5_f32; 16]);
        assert!(rising > 0.5, "rising flux was {rising}");
    }

    #[test]
    fn a_falling_spectrum_produces_no_flux() {
        let mut flux = SpectralFlux::new(16, 12.0);
        let _ = flux.update(&[0.9_f32; 16]);
        // Energy dropping is not an onset.
        assert_eq!(flux.update(&[0.1_f32; 16]), 0.0);
    }

    #[test]
    fn flux_is_bounded_and_finite() {
        let mut flux = SpectralFlux::new(8, 12.0);
        let _ = flux.update(&[0.0_f32; 8]);
        for _ in 0..10 {
            let value = flux.update(&[1000.0_f32; 8]);
            assert!(value.is_finite());
            assert!((0.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn reset_makes_the_next_update_zero() {
        let mut flux = SpectralFlux::new(8, 12.0);
        let _ = flux.update(&[0.0_f32; 8]);
        assert!(flux.update(&[1.0_f32; 8]) > 0.0);
        flux.reset();
        assert_eq!(flux.update(&[1.0_f32; 8]), 0.0);
    }

    #[test]
    fn refractory_prevents_a_burst_from_one_attack() {
        // 100 ms refractory at a 21.33 ms hop is 5 frames.
        let mut detector = OnsetDetector::new(0.3, 100.0, 1000.0 / 46.875);
        assert_eq!(detector.refractory_frames(), 5);

        let first = detector.update(0.9);
        assert!(first.is_some(), "the first strong frame should fire");

        // `refractory_frames` is the number of frames that must be suppressed.
        for frame in 0..5 {
            assert!(
                detector.update(0.9).is_none(),
                "suppressed frame {frame} should not fire"
            );
        }
        // The frame after the refractory window is eligible again.
        assert!(detector.update(0.9).is_some());
    }

    #[test]
    fn weak_flux_never_fires() {
        let mut detector = OnsetDetector::new(0.5, 100.0, 21.33);
        for _ in 0..50 {
            assert!(detector.update(0.2).is_none());
        }
    }

    #[test]
    fn non_finite_flux_cannot_fire() {
        let mut detector = OnsetDetector::new(0.1, 10.0, 21.33);
        assert!(detector.update(f32::NAN).is_none());
        assert!(detector.update(f32::INFINITY).is_none());
    }

    #[test]
    fn sensitivity_increases_response_to_the_same_change() {
        let mut gentle = SpectralFlux::new(8, 2.0);
        let mut sensitive = SpectralFlux::new(8, 30.0);

        // A 4% rise on an already non-zero spectrum. Rising from silence is a
        // 100% relative change and saturates both, which measures nothing.
        let baseline = vec![0.5_f32; 8];
        let _ = gentle.update(&baseline);
        let _ = sensitive.update(&baseline);

        let a = gentle.update(&[0.52_f32; 8]);
        let b = sensitive.update(&[0.52_f32; 8]);
        assert!(b > a, "sensitive {b} should exceed gentle {a}");
        assert!(a < 0.5 && b < 1.0, "a={a} b={b}");
    }
}
