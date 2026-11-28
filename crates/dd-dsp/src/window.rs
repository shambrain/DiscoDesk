//! Analysis window.

use std::f32::consts::PI;

/// A Hann window of `size` points, with its coherent gain.
///
/// The coherent gain is the mean of the window taps. Dividing a magnitude
/// spectrum by `size * gain` makes a full-scale sine read ~1.0 in its bin,
/// which is what makes the dBFS mapping in [`crate::bands`] physically
/// meaningful instead of arbitrary.
#[derive(Debug, Clone)]
pub struct HannWindow {
    taps: Vec<f32>,
    coherent_gain: f32,
}

impl HannWindow {
    /// Builds a periodic Hann window.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0: a zero-length analysis window cannot be used and
    /// the caller is expected to have validated its configuration.
    #[must_use]
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "window size must be positive");
        let taps: Vec<f32> = (0..size)
            .map(|n| {
                let x = n as f32 / size as f32;
                0.5 - 0.5 * (2.0 * PI * x).cos()
            })
            .collect();
        let sum: f32 = taps.iter().sum();
        Self {
            coherent_gain: sum / size as f32,
            taps,
        }
    }

    /// The window taps.
    #[must_use]
    pub fn taps(&self) -> &[f32] {
        &self.taps
    }

    /// The mean of the taps.
    #[must_use]
    pub fn coherent_gain(&self) -> f32 {
        self.coherent_gain
    }

    /// Multiplies `input` by the window into `output`.
    ///
    /// # Panics
    ///
    /// Panics if the slice lengths differ from the window size.
    pub fn apply(&self, input: &[f32], output: &mut [f32]) {
        assert_eq!(input.len(), self.taps.len());
        assert_eq!(output.len(), self.taps.len());
        for ((out, &sample), &tap) in output.iter_mut().zip(input).zip(&self.taps) {
            *out = sample * tap;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_near_zero_and_the_middle_peaks() {
        let window = HannWindow::new(64);
        let taps = window.taps();
        assert!(taps[0].abs() < 1e-6);
        assert!(taps[32] > 0.99);
    }

    #[test]
    fn coherent_gain_is_one_half() {
        // A periodic Hann window has a mean of exactly 0.5.
        for size in [8_usize, 64, 2048] {
            let window = HannWindow::new(size);
            assert!(
                (window.coherent_gain() - 0.5).abs() < 1e-4,
                "size {size} gain {}",
                window.coherent_gain()
            );
        }
    }

    #[test]
    fn symmetry_of_the_periodic_window() {
        let window = HannWindow::new(64);
        let taps = window.taps();
        // A periodic (DFT-even) Hann satisfies w[n] == w[N-n] for n in 1..N.
        for n in 1..64 {
            assert!(
                (taps[n] - taps[64 - n]).abs() < 1e-5,
                "n {n}: {} vs {}",
                taps[n],
                taps[64 - n]
            );
        }
    }
}
