//! In-place iterative radix-2 Cooley-Tukey FFT.
//!
//! Hand-written rather than pulled from a crate so that the exact arithmetic is
//! deterministic, auditable and testable against a naive DFT. The transform is
//! small (2048 points at ~47 hops/s), so the constant factor of a simple
//! implementation is irrelevant here.
//!
//! All twiddle factors and the bit-reversal permutation are precomputed once per
//! size, so a transform performs no allocation.

use std::f32::consts::PI;

/// A single complex sample.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    /// Real part.
    pub re: f32,
    /// Imaginary part.
    pub im: f32,
}

impl Complex {
    /// The squared magnitude, `re^2 + im^2`.
    ///
    /// Squared magnitude avoids a square root when only relative comparisons or
    /// an energy sum are needed.
    #[must_use]
    pub fn norm_sqr(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    /// The magnitude, `sqrt(re^2 + im^2)`.
    #[must_use]
    pub fn magnitude(self) -> f32 {
        self.norm_sqr().sqrt()
    }
}

/// Errors constructing an [`Fft`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FftError {
    /// The transform size must be a power of two.
    #[error("FFT size {0} is not a power of two")]
    NotPowerOfTwo(usize),
    /// The transform size must be at least 2.
    #[error("FFT size {0} is too small; at least 2 is required")]
    TooSmall(usize),
}

/// A reusable forward FFT of a fixed power-of-two size.
#[derive(Debug)]
pub struct Fft {
    size: usize,
    /// Bit-reversed index for each position.
    reversed: Vec<usize>,
    /// Forward twiddles, `e^{-2*pi*i*k/N}` for `k` in `0..size/2`.
    twiddle_re: Vec<f32>,
    twiddle_im: Vec<f32>,
}

impl Fft {
    /// Precomputes the permutation and twiddles for `size`.
    ///
    /// # Errors
    ///
    /// Returns [`FftError`] if `size` is below 2 or is not a power of two.
    pub fn new(size: usize) -> Result<Self, FftError> {
        if size < 2 {
            return Err(FftError::TooSmall(size));
        }
        if !size.is_power_of_two() {
            return Err(FftError::NotPowerOfTwo(size));
        }

        let bits = size.trailing_zeros();
        let mut reversed = Vec::with_capacity(size);
        for index in 0..size {
            // Reverse the low `bits` bits of `index`.
            let mut value = 0_usize;
            for bit in 0..bits {
                if index & (1 << bit) != 0 {
                    value |= 1 << (bits - 1 - bit);
                }
            }
            reversed.push(value);
        }

        let half = size / 2;
        let mut twiddle_re = Vec::with_capacity(half);
        let mut twiddle_im = Vec::with_capacity(half);
        for k in 0..half {
            let angle = -2.0 * PI * (k as f32) / (size as f32);
            twiddle_re.push(angle.cos());
            twiddle_im.push(angle.sin());
        }

        Ok(Self {
            size,
            reversed,
            twiddle_re,
            twiddle_im,
        })
    }

    /// The transform length.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Number of non-negative frequency bins, including DC and Nyquist.
    #[must_use]
    pub fn bin_count(&self) -> usize {
        self.size / 2 + 1
    }

    /// The centre frequency of bin `k` in Hz for `sample_rate`.
    #[must_use]
    pub fn bin_frequency(&self, k: usize, sample_rate: u32) -> f32 {
        k as f32 * sample_rate as f32 / self.size as f32
    }

    /// Transforms `buffer` in place. Its length must equal [`Fft::size`].
    ///
    /// # Panics
    ///
    /// Panics if `buffer.len()` is not the transform size. The size is fixed at
    /// construction and checked once, so a mismatch is a programming error, not
    /// a runtime condition.
    pub fn forward_in_place(&self, buffer: &mut [Complex]) {
        assert_eq!(
            buffer.len(),
            self.size,
            "buffer length must equal the FFT size"
        );

        // Bit-reversal permutation.
        for (index, &target) in self.reversed.iter().enumerate() {
            if target > index {
                buffer.swap(index, target);
            }
        }

        // Butterflies, doubling the sub-transform length each pass.
        let mut len = 2;
        while len <= self.size {
            let half = len / 2;
            let step = self.size / len;

            let mut start = 0;
            while start < self.size {
                for offset in 0..half {
                    let twiddle_index = offset * step;
                    let w_re = self.twiddle_re[twiddle_index];
                    let w_im = self.twiddle_im[twiddle_index];

                    let even = buffer[start + offset];
                    let odd = buffer[start + offset + half];

                    let t_re = odd.re * w_re - odd.im * w_im;
                    let t_im = odd.re * w_im + odd.im * w_re;

                    buffer[start + offset] = Complex {
                        re: even.re + t_re,
                        im: even.im + t_im,
                    };
                    buffer[start + offset + half] = Complex {
                        re: even.re - t_re,
                        im: even.im - t_im,
                    };
                }
                start += len;
            }
            len *= 2;
        }
    }

    /// Computes the magnitude of each non-negative frequency bin.
    ///
    /// `samples` must be exactly [`Fft::size`] long and already windowed.
    /// `scratch` must be exactly [`Fft::size`] long. `magnitudes` must be exactly
    /// [`Fft::bin_count`] long.
    ///
    /// `scale` is applied to every magnitude. Use it to normalise for window
    /// coherent gain so that a full-scale sine yields ~1.0 in its bin.
    ///
    /// # Panics
    ///
    /// Panics if any slice length is wrong. These are fixed at construction, so a
    /// mismatch is a programming error.
    pub fn magnitudes(
        &self,
        samples: &[f32],
        scratch: &mut [Complex],
        magnitudes: &mut [f32],
        scale: f32,
    ) {
        assert_eq!(
            samples.len(),
            self.size,
            "sample length must equal the FFT size"
        );
        assert_eq!(
            scratch.len(),
            self.size,
            "scratch length must equal the FFT size"
        );
        assert_eq!(
            magnitudes.len(),
            self.bin_count(),
            "magnitude length must equal the bin count"
        );

        for (slot, &sample) in scratch.iter_mut().zip(samples) {
            *slot = Complex {
                re: sample,
                im: 0.0,
            };
        }

        self.forward_in_place(scratch);

        for (k, magnitude) in magnitudes.iter_mut().enumerate() {
            *magnitude = scratch[k].magnitude() * scale;
        }
    }
}

/// A naive `O(n^2)` DFT, used only by tests as the oracle for the fast path.
#[cfg(test)]
pub(crate) fn naive_dft(input: &[Complex]) -> Vec<Complex> {
    let n = input.len();
    let mut out = vec![Complex::default(); n];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut sum_re = 0.0_f32;
        let mut sum_im = 0.0_f32;
        for (t, sample) in input.iter().enumerate() {
            let angle = -2.0 * PI * (k * t) as f32 / n as f32;
            let (s, c) = angle.sin_cos();
            sum_re += sample.re * c - sample.im * s;
            sum_im += sample.re * s + sample.im * c;
        }
        *slot = Complex {
            re: sum_re,
            im: sum_im,
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn rejects_non_power_of_two_and_tiny_sizes() {
        assert!(matches!(Fft::new(0), Err(FftError::TooSmall(0))));
        assert!(matches!(Fft::new(1), Err(FftError::TooSmall(1))));
        assert!(matches!(Fft::new(3), Err(FftError::NotPowerOfTwo(3))));
        assert!(matches!(Fft::new(1000), Err(FftError::NotPowerOfTwo(1000))));
        assert!(Fft::new(2048).is_ok());
    }

    #[test]
    fn matches_the_naive_dft_for_all_small_sizes() {
        for size in [2_usize, 4, 8, 16, 32, 64] {
            let fft = Fft::new(size).expect("power of two");
            // A deterministic, non-symmetric input.
            let input: Vec<Complex> = (0..size)
                .map(|i| {
                    let x = i as f32;
                    Complex {
                        re: (0.7 * x).sin() + 0.3 * (0.11 * x).cos(),
                        im: 0.2 * (0.37 * x).sin(),
                    }
                })
                .collect();

            let expected = naive_dft(&input);
            let mut actual = input.clone();
            fft.forward_in_place(&mut actual);

            // Both sides are f32, so compare with a relative tolerance: absolute
            // 1e-4 is below f32 resolution once magnitudes reach ~10.
            for k in 0..size {
                let re_tolerance = 1e-3 * expected[k].re.abs().max(1.0);
                let im_tolerance = 1e-3 * expected[k].im.abs().max(1.0);
                assert!(
                    (actual[k].re - expected[k].re).abs() <= re_tolerance,
                    "size {size} bin {k} real: {} vs {}",
                    actual[k].re,
                    expected[k].re
                );
                assert!(
                    (actual[k].im - expected[k].im).abs() <= im_tolerance,
                    "size {size} bin {k} imag: {} vs {}",
                    actual[k].im,
                    expected[k].im
                );
            }
        }
    }

    #[test]
    fn dc_input_concentrates_all_energy_in_bin_zero() {
        let size = 64;
        let fft = Fft::new(size).expect("power of two");
        let mut buffer = vec![Complex { re: 1.0, im: 0.0 }; size];
        fft.forward_in_place(&mut buffer);

        assert!(close(buffer[0].re, size as f32, 1e-3));
        for (k, value) in buffer.iter().enumerate().skip(1) {
            assert!(
                value.magnitude() < 1e-3,
                "bin {k} should be empty, got {}",
                value.magnitude()
            );
        }
    }

    #[test]
    fn a_full_scale_sine_normalises_to_unit_magnitude() {
        // A Hann window loses half its energy, so the coherent gain is 0.5.
        let size = 2048;
        let sample_rate = 48_000;
        let fft = Fft::new(size).expect("power of two");
        let bin = 100;
        let frequency = fft.bin_frequency(bin, sample_rate);

        let window: Vec<f32> = (0..size)
            .map(|n| {
                let x = n as f32 / size as f32;
                0.5 - 0.5 * (2.0 * PI * x).cos()
            })
            .collect();
        let gain: f32 = window.iter().sum::<f32>() / size as f32;

        let samples: Vec<f32> = (0..size)
            .map(|n| {
                let t = n as f32 / sample_rate as f32;
                (2.0 * PI * frequency * t).sin() * window[n]
            })
            .collect();

        let mut scratch = vec![Complex::default(); size];
        let mut magnitudes = vec![0.0_f32; fft.bin_count()];
        fft.magnitudes(
            &samples,
            &mut scratch,
            &mut magnitudes,
            2.0 / (size as f32 * gain),
        );

        // The tone is not bin-aligned to the window, so allow spectral leakage.
        let peak = magnitudes.iter().fold(0.0_f32, |a, b| a.max(*b));
        assert!(
            close(peak, 1.0, 0.05),
            "expected ~1.0 peak magnitude, got {peak}"
        );
    }

    #[test]
    fn bin_frequencies_follow_the_size_and_rate() {
        let fft = Fft::new(2048).expect("power of two");
        let spacing = fft.bin_frequency(1, 48_000);
        assert!(close(spacing, 23.4375, 1e-4), "spacing was {spacing}");
        assert!(close(fft.bin_frequency(1024, 48_000), 24_000.0, 1e-3));
    }

    #[test]
    fn transform_is_deterministic() {
        let fft = Fft::new(256).expect("power of two");
        let input: Vec<Complex> = (0..256)
            .map(|i| Complex {
                re: (i as f32 * 0.13).sin(),
                im: 0.0,
            })
            .collect();

        let mut a = input.clone();
        let mut b = input.clone();
        fft.forward_in_place(&mut a);
        fft.forward_in_place(&mut b);

        assert_eq!(a, b, "identical input must give bit-identical output");
    }
}
