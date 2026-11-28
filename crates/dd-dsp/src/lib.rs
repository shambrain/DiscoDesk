//! DiscoDesk deterministic DSP.
//!
//! A small, dependency-light analysis pipeline that turns mono PCM into a
//! compact [`FeatureFrame`]: six musical bands, envelope followers, positive
//! spectral flux, onset events and levels.
//!
//! # Non-claims
//!
//! The six zones are custom broad frequency groupings. They are **not** Bark
//! critical bands, not instrument stems, and cannot identify a vocal, a kick or
//! a hi-hat. Spectral flux is an onset feature, **not** a beat or tempo tracker.
//! See `ARCHITECTURE.md` §7.
//!
//! # Determinism
//!
//! Same input + same configuration + same binary ⇒ identical output. There is no
//! randomness, no wall-clock dependence, and no adaptive state whose value
//! depends on how long the process has been running. Reconstructing the engine
//! is the way to compare sessions.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bands;
pub mod engine;
pub mod envelope;
pub mod fft;
pub mod flux;
pub mod spring;
pub mod window;

pub use bands::{BAND_COUNT, BANDS, Band, BandMap, amplitude_to_unit};
pub use engine::{DspConfig, DspEngine, DspError, FeatureFrame};
pub use envelope::EnvelopeFollower;
pub use fft::{Complex, Fft, FftError};
pub use flux::{OnsetDetector, SpectralFlux};
pub use spring::SpringDamper;
pub use window::HannWindow;
