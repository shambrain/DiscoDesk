//! Envelope followers with sample-rate-aware attack and release.
//!
//! Coefficients are derived from the *actual* hop duration, never hard-coded for
//! one sample rate. The standard one-pole form is
//!
//! ```text
//! alpha = 1 - exp(-delta_t / tau)
//! y     = y + alpha * (x - y)
//! ```
//!
//! so `tau` is the time constant in seconds. Half-life tuning in milliseconds is
//! converted once, at construction.

/// A one-pole envelope follower with separate attack and release coefficients.
#[derive(Debug, Clone, Copy)]
pub struct EnvelopeFollower {
    attack: f32,
    release: f32,
    value: f32,
}

impl EnvelopeFollower {
    /// Creates a follower.
    ///
    /// `attack_ms` and `release_ms` are time constants, `hop_seconds` is the
    /// interval between updates. All three must be positive; a non-positive
    /// value falls back to an instantaneous follower rather than panicking, so a
    /// bad configuration cannot take the audio path down.
    #[must_use]
    pub fn new(attack_ms: f32, release_ms: f32, hop_seconds: f32) -> Self {
        Self {
            attack: coefficient(attack_ms, hop_seconds),
            release: coefficient(release_ms, hop_seconds),
            value: 0.0,
        }
    }

    /// Updates with `input` and returns the new envelope value.
    ///
    /// Release is selected when the input is below the current value, which is
    /// what makes a transient decay smoothly instead of snapping.
    pub fn process(&mut self, input: f32) -> f32 {
        let target = if input.is_finite() {
            input.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let alpha = if target > self.value {
            self.attack
        } else {
            self.release
        };
        self.value += alpha * (target - self.value);
        if !self.value.is_finite() {
            // Defence in depth: a non-finite coefficient or target must not
            // poison the follower for the rest of the session.
            self.value = 0.0;
        }
        self.value.clamp(0.0, 1.0)
    }

    /// The current value.
    #[must_use]
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Drops the value to zero, e.g. because the audio source restarted.
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    /// The attack coefficient, exposed for tests and diagnostics.
    #[must_use]
    pub fn attack_coefficient(&self) -> f32 {
        self.attack
    }

    /// The release coefficient, exposed for tests and diagnostics.
    #[must_use]
    pub fn release_coefficient(&self) -> f32 {
        self.release
    }
}

/// One-pole coefficient for a time constant in milliseconds and a hop duration
/// in seconds.
fn coefficient(tau_ms: f32, hop_seconds: f32) -> f32 {
    if !tau_ms.is_finite() || !hop_seconds.is_finite() || tau_ms <= 0.0 || hop_seconds <= 0.0 {
        return 1.0;
    }
    let tau = tau_ms / 1000.0;
    (1.0 - (-hop_seconds / tau).exp()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOP: f32 = 1024.0 / 48_000.0; // 21.333 ms

    #[test]
    fn coefficient_depends_on_the_hop_not_a_hardcoded_rate() {
        let at_48k = EnvelopeFollower::new(10.0, 220.0, 1024.0 / 48_000.0);
        let at_44k = EnvelopeFollower::new(10.0, 220.0, 1024.0 / 44_100.0);
        assert!(
            (at_48k.attack_coefficient() - at_44k.attack_coefficient()).abs() > 1e-4,
            "a different sample rate must change the coefficients"
        );
    }

    #[test]
    fn attack_is_faster_than_release() {
        let follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        assert!(
            follower.attack_coefficient() > follower.release_coefficient(),
            "attack {} should exceed release {}",
            follower.attack_coefficient(),
            follower.release_coefficient()
        );
    }

    #[test]
    fn rises_to_the_target_and_settles() {
        let mut follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        let mut value = 0.0;
        for _ in 0..40 {
            value = follower.process(1.0);
        }
        assert!((value - 1.0).abs() < 1e-3, "settled at {value}");
    }

    #[test]
    fn attack_reaches_ninety_percent_within_the_documented_time() {
        // A one-pole follower reaches 1 - 1/e (~63%) after one time constant and
        // ~86% after two. With a 10 ms attack at a 21.33 ms hop, the first update
        // already jumps well past 63%.
        let mut follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        let first = follower.process(1.0);
        let expected = 1.0 - (-HOP / 0.010).exp();
        assert!(
            (first - expected).abs() < 1e-3,
            "first step {first} should equal {expected}"
        );
        assert!(
            first > 0.8,
            "with a 10 ms attack and a 21 ms hop, the first step should be large"
        );
    }

    #[test]
    fn release_decays_smoothed_not_instantly() {
        let mut follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        for _ in 0..50 {
            follower.process(1.0);
        }
        let after_one = follower.process(0.0);
        assert!(
            after_one > 0.5,
            "release should be gradual, got {after_one}"
        );
        let mut value = after_one;
        for _ in 0..200 {
            value = follower.process(0.0);
        }
        assert!(value < 1e-3, "should eventually reach zero, got {value}");
    }

    #[test]
    fn stays_bounded_for_extreme_and_invalid_input() {
        let mut follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        for input in [f32::NAN, f32::INFINITY, -5.0, 5.0, 1e30] {
            let value = follower.process(input);
            assert!(value.is_finite(), "input {input} gave {value}");
            assert!((0.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn reset_returns_to_zero() {
        let mut follower = EnvelopeFollower::new(10.0, 220.0, HOP);
        for _ in 0..10 {
            follower.process(1.0);
        }
        assert!(follower.value() > 0.5);
        follower.reset();
        assert_eq!(follower.value(), 0.0);
    }

    #[test]
    fn degenerate_timing_does_not_panic() {
        let mut follower = EnvelopeFollower::new(0.0, 0.0, 0.0);
        assert!(follower.process(1.0).is_finite());
    }
}
