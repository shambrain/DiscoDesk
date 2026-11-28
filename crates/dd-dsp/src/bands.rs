//! The six product-facing frequency zones.
//!
//! These are **custom broad frequency groupings**, not Bark critical bands and
//! not instrument stems. A band cannot identify a vocal, a kick or a hi-hat;
//! it measures energy in a frequency range. The names describe the intended
//! visual meaning, not a source separation claim (`ARCHITECTURE.md` §7.3).

/// Number of bands.
pub const BAND_COUNT: usize = 6;

/// One frequency zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// Stable machine name.
    pub key: &'static str,
    /// Lower edge in Hz, inclusive.
    pub low_hz: f32,
    /// Upper edge in Hz, exclusive.
    pub high_hz: f32,
}

/// The six zones, in output order.
pub const BANDS: [Band; BAND_COUNT] = [
    Band {
        key: "sub_bass",
        low_hz: 20.0,
        high_hz: 60.0,
    },
    Band {
        key: "punch",
        low_hz: 60.0,
        high_hz: 150.0,
    },
    Band {
        key: "warmth",
        low_hz: 150.0,
        high_hz: 500.0,
    },
    Band {
        key: "presence",
        low_hz: 500.0,
        high_hz: 2_500.0,
    },
    Band {
        key: "brilliance",
        low_hz: 2_500.0,
        high_hz: 6_000.0,
    },
    Band {
        key: "air",
        low_hz: 6_000.0,
        high_hz: 20_000.0,
    },
];

/// Maps FFT bins onto the six bands for one sample rate.
///
/// Bin ranges are computed once. A band whose range contains no bin (possible
/// for `sub_bass` at a small FFT size or a low sample rate) reports exactly 0
/// rather than borrowing a neighbouring bin, so the frequency resolution limit
/// is visible instead of hidden.
#[derive(Debug, Clone)]
pub struct BandMap {
    ranges: [(usize, usize); BAND_COUNT],
    /// Set when a band has no bins at this resolution.
    empty: [bool; BAND_COUNT],
}

impl BandMap {
    /// Builds the bin mapping.
    ///
    /// `bin_hz` is the spacing between bin centres, `bin_count` the number of
    /// non-negative bins.
    #[must_use]
    pub fn new(bin_hz: f32, bin_count: usize) -> Self {
        let mut ranges = [(0_usize, 0_usize); BAND_COUNT];
        let mut empty = [true; BAND_COUNT];

        for (index, band) in BANDS.iter().enumerate() {
            let first = (band.low_hz / bin_hz).ceil().max(0.0) as usize;
            // Upper edge exclusive: the last included bin centre is < high_hz.
            let last = ((band.high_hz / bin_hz).ceil() as usize)
                .saturating_sub(1)
                .min(bin_count.saturating_sub(1));

            if first <= last && first < bin_count {
                ranges[index] = (first, last);
                empty[index] = false;
            }
        }

        Self { ranges, empty }
    }

    /// Whether a band has no representable bins at this resolution.
    #[must_use]
    pub fn is_empty(&self, index: usize) -> bool {
        self.empty.get(index).copied().unwrap_or(true)
    }

    /// The inclusive bin range for a band, if any.
    #[must_use]
    pub fn range(&self, index: usize) -> Option<(usize, usize)> {
        if self.is_empty(index) {
            None
        } else {
            self.ranges.get(index).copied()
        }
    }

    /// Integrates magnitudes into a per-band amplitude.
    ///
    /// The value is the root of the summed squared magnitudes, i.e. an
    /// energy-equivalent amplitude for the zone. Summing energy rather than
    /// averaging keeps a wide band from being penalised for covering more bins
    /// than a narrow one.
    ///
    /// # Panics
    ///
    /// Panics if `magnitudes` is shorter than the bin count the map was built
    /// for; the length is fixed at construction, so a mismatch is a bug.
    pub fn amplitudes(&self, magnitudes: &[f32], output: &mut [f32; BAND_COUNT]) {
        let empty = self.empty;
        let ranges = self.ranges;
        for ((slot, &is_empty), &(first, last)) in
            output.iter_mut().zip(empty.iter()).zip(ranges.iter())
        {
            if is_empty {
                *slot = 0.0;
                continue;
            }
            let mut energy = 0.0_f32;
            for &magnitude in &magnitudes[first..=last] {
                energy += magnitude * magnitude;
            }
            *slot = energy.sqrt();
        }
    }
}

/// Converts a linear amplitude to a 0..1 value on a dBFS scale.
///
/// `floor_dbfs` (a negative number, e.g. -60.0) maps to 0.0 and 0 dBFS maps to
/// 1.0. Values outside the range are clamped. Silence, negatives, NaN and
/// infinity all map to exactly 0.0 so a broken input can never produce motion.
#[must_use]
pub fn amplitude_to_unit(amplitude: f32, floor_dbfs: f32) -> f32 {
    if !amplitude.is_finite() || amplitude <= 0.0 || !floor_dbfs.is_finite() || floor_dbfs >= 0.0 {
        return 0.0;
    }
    let db = 20.0 * amplitude.log10();
    if !db.is_finite() {
        return 0.0;
    }
    ((db - floor_dbfs) / -floor_dbfs).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_48k_2048() -> BandMap {
        // 48000 / 2048 = 23.4375 Hz per bin, 1025 non-negative bins.
        BandMap::new(48_000.0 / 2048.0, 1025)
    }

    #[test]
    fn every_band_has_bins_at_48k_2048() {
        let map = map_48k_2048();
        for (index, band) in BANDS.iter().enumerate() {
            assert!(
                !map.is_empty(index),
                "band {index} ({}) has no bins",
                band.key
            );
        }
    }

    #[test]
    fn band_ranges_do_not_overlap_and_are_ordered() {
        let map = map_48k_2048();
        for index in 1..BAND_COUNT {
            let (_, previous_last) = map.range(index - 1).expect("non-empty");
            let (first, _) = map.range(index).expect("non-empty");
            assert!(
                first > previous_last,
                "band {} starts at bin {first} but band {} ends at {previous_last}",
                index,
                index - 1
            );
        }
    }

    #[test]
    fn a_100hz_tone_lands_in_punch_not_sub_bass() {
        let map = map_48k_2048();
        let mut magnitudes = vec![0.0_f32; 1025];
        let bin = (100.0_f32 / (48_000.0_f32 / 2048.0)).round() as usize;
        magnitudes[bin] = 1.0;

        let mut bands = [0.0_f32; BAND_COUNT];
        map.amplitudes(&magnitudes, &mut bands);
        assert!(bands[1] > 0.9, "punch should carry it, got {}", bands[1]);
        assert!(
            bands[0] < 0.01,
            "sub-bass should be empty, got {}",
            bands[0]
        );
    }

    #[test]
    fn a_40hz_tone_lands_in_sub_bass() {
        let map = map_48k_2048();
        let mut magnitudes = vec![0.0_f32; 1025];
        let bin = (40.0_f32 / (48_000.0_f32 / 2048.0)).round() as usize;
        magnitudes[bin] = 1.0;

        let mut bands = [0.0_f32; BAND_COUNT];
        map.amplitudes(&magnitudes, &mut bands);
        assert!(bands[0] > 0.9, "sub-bass should carry it, got {}", bands[0]);
    }

    #[test]
    fn a_10khz_tone_lands_in_air() {
        let map = map_48k_2048();
        let mut magnitudes = vec![0.0_f32; 1025];
        let bin = (10_000.0_f32 / (48_000.0_f32 / 2048.0)).round() as usize;
        magnitudes[bin] = 1.0;

        let mut bands = [0.0_f32; BAND_COUNT];
        map.amplitudes(&magnitudes, &mut bands);
        assert!(bands[5] > 0.9, "air should carry it, got {}", bands[5]);
    }

    #[test]
    fn a_band_without_bins_reports_zero_rather_than_borrowing() {
        // 8 kHz sample rate, 256-point FFT: 31.25 Hz bins. sub_bass is 20-60 Hz
        // so it has bin 1 (31.25 Hz) only; test a configuration where a band is
        // genuinely empty: 20-60 Hz with 62.5 Hz bins has no bin below 60.
        let map = BandMap::new(62.5, 33);
        assert!(
            map.is_empty(0),
            "sub_bass should be empty at 62.5 Hz spacing"
        );
        let mut bands = [1.0_f32; BAND_COUNT];
        map.amplitudes(&[1.0_f32; 33], &mut bands);
        assert_eq!(bands[0], 0.0);
    }

    #[test]
    fn unit_mapping_is_monotonic_and_clamped() {
        assert_eq!(amplitude_to_unit(0.0, -60.0), 0.0);
        assert_eq!(amplitude_to_unit(-1.0, -60.0), 0.0);
        assert_eq!(amplitude_to_unit(f32::NAN, -60.0), 0.0);
        assert_eq!(amplitude_to_unit(f32::INFINITY, -60.0), 0.0);
        assert!((amplitude_to_unit(1.0, -60.0) - 1.0).abs() < 1e-6);
        // -60 dBFS is exactly the floor.
        assert!(amplitude_to_unit(0.001, -60.0).abs() < 1e-5);
        // Half amplitude is about -6 dBFS, i.e. 0.9 of the range.
        let half = amplitude_to_unit(0.5, -60.0);
        assert!((half - 0.9).abs() < 1e-3, "half was {half}");
        // Loud input clamps rather than exceeding 1.0.
        assert_eq!(amplitude_to_unit(8.0, -60.0), 1.0);
    }
}
