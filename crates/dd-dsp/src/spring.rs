//! A critically damped spring, solved analytically per step.
//!
//! Used to smooth the overall level so visual motion has mass: a single
//! instantaneous reading should not move the scene, but a sustained change
//! should.
//!
//! # Why analytic rather than explicit integration
//!
//! The obvious implementation — `v += (k*(target - x) - c*v) * dt; x += v * dt`
//! — is only stable when `omega * dt` is small. Here the update interval is the
//! FFT hop (~21.3 ms at 48 kHz) and a usefully snappy spring has
//! `omega = 30..60 rad/s`, giving `omega * dt` of 0.6..1.3. Explicit Euler
//! diverges at those values, so the closed-form critically damped solution is
//! used instead:
//!
//! ```text
//! let a = x0 - target, b = v0 + omega * a, e = exp(-omega * t)
//! x(t) = target + (a + b*t) * e
//! v(t) = (b - omega * (a + b*t)) * e
//! ```
//!
//! This is exact for any `t`, so it cannot become unstable. Stability is tested.

/// A critically damped spring following a moving target.
#[derive(Debug, Clone, Copy)]
pub struct SpringDamper {
    position: f32,
    velocity: f32,
    omega: f32,
    step_seconds: f32,
}

impl SpringDamper {
    /// Creates a spring that settles in roughly `settle_ms`.
    ///
    /// "Settle" is taken as the time to reach ~2% of a step response, which for
    /// a critically damped system is about `5.8 / omega`. Hence
    /// `omega = 5.8 / settle_seconds`.
    #[must_use]
    pub fn new(settle_ms: f32, step_seconds: f32) -> Self {
        let (omega, step) = if settle_ms.is_finite()
            && settle_ms > 0.0
            && step_seconds.is_finite()
            && step_seconds > 0.0
        {
            (5.8 / (settle_ms / 1000.0), step_seconds)
        } else {
            // Degenerate configuration: behave as a pass-through.
            (1.0e6, 1.0e-6)
        };
        Self {
            position: 0.0,
            velocity: 0.0,
            omega,
            step_seconds: step,
        }
    }

    /// Advances one step toward `target` and returns the new position.
    pub fn process(&mut self, target: f32) -> f32 {
        if !target.is_finite() {
            return self.position;
        }
        let a = self.position - target;
        let b = self.velocity + self.omega * a;
        // exp(-omega*t) is in (0, 1] for positive inputs, so this cannot blow up.
        let decay = (-self.omega * self.step_seconds).exp();

        let new_position = target + (a + b * self.step_seconds) * decay;
        let new_velocity = (b - self.omega * (a + b * self.step_seconds)) * decay;

        if new_position.is_finite() && new_velocity.is_finite() {
            self.position = new_position;
            self.velocity = new_velocity;
        }
        self.position
    }

    /// The current position.
    #[must_use]
    pub fn position(&self) -> f32 {
        self.position
    }

    /// The current velocity.
    #[must_use]
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// The angular frequency in rad/s.
    #[must_use]
    pub fn omega(&self) -> f32 {
        self.omega
    }

    /// Jumps to `value` with zero velocity, e.g. on a source reset.
    pub fn reset_to(&mut self, value: f32) {
        self.position = if value.is_finite() { value } else { 0.0 };
        self.velocity = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOP: f32 = 1024.0 / 48_000.0;

    #[test]
    fn converges_to_a_constant_target() {
        let mut spring = SpringDamper::new(120.0, HOP);
        let mut value = 0.0;
        for _ in 0..60 {
            value = spring.process(1.0);
            assert!(value.is_finite());
        }
        assert!((value - 1.0).abs() < 1e-3, "settled at {value}");
    }

    #[test]
    fn is_stable_for_aggressive_settle_times_that_break_explicit_euler() {
        // omega here is ~5.8/0.03 = 193 rad/s, and omega*dt ~ 4.1. An explicit
        // Euler spring would diverge; the analytic solution must not.
        let mut spring = SpringDamper::new(30.0, HOP);
        for _ in 0..2000 {
            let value = spring.process(1.0);
            assert!(value.is_finite(), "diverged: {value}");
            assert!(value.abs() < 10.0, "way overshot: {value}");
        }
        assert!((spring.position() - 1.0).abs() < 1e-2);
    }

    #[test]
    fn does_not_overshoot_a_step_significantly() {
        // Critical damping means no overshoot in the continuous solution.
        let mut spring = SpringDamper::new(200.0, HOP);
        let mut peak: f32 = 0.0;
        for _ in 0..200 {
            peak = peak.max(spring.process(1.0));
        }
        assert!(peak <= 1.001, "overshot to {peak}");
    }

    #[test]
    fn settling_time_is_in_the_right_ballpark() {
        let mut spring = SpringDamper::new(200.0, HOP);
        let mut frames = 0_u32;
        let mut elapsed_ms = 0.0_f32;
        while frames < 1000 {
            let value = spring.process(1.0);
            frames += 1;
            if (value - 1.0).abs() < 0.02 {
                elapsed_ms = frames as f32 * HOP * 1000.0;
                break;
            }
        }
        // Allow a generous band; the point is that it is not instantaneous and
        // not absurdly slow.
        assert!(
            (120.0..500.0).contains(&elapsed_ms),
            "settled in {elapsed_ms} ms, expected roughly 200 ms"
        );
    }

    #[test]
    fn tracks_a_ramp_without_error_accumulation() {
        let mut spring = SpringDamper::new(100.0, HOP);
        let mut target = 0.0_f32;
        let mut position = 0.0;
        for _ in 0..2000 {
            target += 0.001;
            position = spring.process(target);
            assert!(position.is_finite());
        }
        // A critically damped spring lags a ramp by a bounded amount.
        assert!(
            (target - position).abs() < 0.2,
            "target {target} pos {position}"
        );
    }

    #[test]
    fn reset_jumps_without_velocity() {
        let mut spring = SpringDamper::new(120.0, HOP);
        for _ in 0..10 {
            spring.process(1.0);
        }
        spring.reset_to(0.25);
        assert!((spring.position() - 0.25).abs() < 1e-6);
        assert_eq!(spring.velocity(), 0.0);
    }

    #[test]
    fn rejects_non_finite_targets() {
        let mut spring = SpringDamper::new(120.0, HOP);
        let before = spring.position();
        assert_eq!(spring.process(f32::NAN), before);
        assert_eq!(spring.process(f32::INFINITY), before);
    }

    #[test]
    fn degenerate_configuration_is_safe() {
        let mut spring = SpringDamper::new(0.0, 0.0);
        for _ in 0..10 {
            assert!(spring.process(1.0).is_finite());
        }
    }
}
