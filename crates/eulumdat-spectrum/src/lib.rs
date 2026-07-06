//! Spectral physics for photometric Monte Carlo — a dependency-free leaf crate.
//!
//! This crate is the single home for "what a wavelength does": how to sample a
//! wavelength from a source spectrum, how the eye and other photoreceptors
//! weight it (photopic, scotopic, melanopic, PAR), how a spectrum reads as a
//! colour (CIE colorimetry), how it behaves at night (CIE 191 mesopic), how
//! daylight is coloured across the sky (CIE D-series), and how it bends through
//! optics (Sellmeier dispersion).
//!
//! Like [`eulumdat_daylight`](https://docs.rs), it has **no engine
//! dependencies**, so the CPU tracer (`eulumdat-goniosim`), the GPU tracer
//! (`eulumdat-rt`), the Bevy viewer and the core `eulumdat` crate can all use
//! it without any dependency cycle.
//!
//! # The load-bearing convention
//!
//! Photons carry **radiant** (power-proportional) weights end to end. All
//! photometric/actinic weighting happens at the **detector**, never at
//! emission. A 730 nm far-red photon carries real watts but ≈0 lumens; weight
//! it at emission and you can never recover scotopic, melanopic or PPFD from
//! the same trace. Weight it at detection and every metric is a handful of
//! multiply-adds against the [`lut`] tables. One trace → every quantity.
//!
//! # Example
//! ```
//! use eulumdat_spectrum::{synth, colorimetry, metrics};
//!
//! // Seed a spectrum from a colour temperature, recover it colorimetrically.
//! let spd = synth::synthesize(3000.0);
//! let col = colorimetry::analyze(&spd);
//! assert!((col.cct_k - 3000.0).abs() < 15.0);
//!
//! // The same spectrum, weighted for night vision.
//! let m = metrics::SpectralMetrics::from_spd(&spd);
//! assert!(m.sp_ratio > 1.0 && m.sp_ratio < 1.7); // warm white
//! ```

#![forbid(unsafe_code)]

pub mod colorimetry;
pub mod constants;
pub mod daylight;
pub mod dispersion;
pub mod lut;
pub mod mesopic;
pub mod metrics;
pub mod spd;
pub mod synth;

pub use colorimetry::{analyze, Colorimetry};
pub use dispersion::Sellmeier;
pub use mesopic::{mesopic_coefficient, mesopic_luminance};
pub use metrics::{melanopic_der, sp_ratio, SpectralMetrics};
pub use spd::{Spd, SpectralSampler};
