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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Float functions this crate needs. With `std` the inherent `f64` methods win
/// (method resolution prefers them); without `std` this trait supplies them via
/// libm, so the spectral maths runs unchanged on a bare-metal target.
pub mod math {
    pub trait F64Ext {
        fn sqrt(self) -> f64;
        fn exp(self) -> f64;
        fn ln(self) -> f64;
        fn log10(self) -> f64;
        fn powf(self, n: f64) -> f64;
        fn powi(self, n: i32) -> f64;
        fn floor(self) -> f64;
        fn ceil(self) -> f64;
        fn round(self) -> f64;
        fn hypot(self, y: f64) -> f64;
    }

    impl F64Ext for f64 {
        #[inline]
        fn sqrt(self) -> f64 {
            libm::sqrt(self)
        }
        #[inline]
        fn exp(self) -> f64 {
            libm::exp(self)
        }
        #[inline]
        fn ln(self) -> f64 {
            libm::log(self)
        }
        #[inline]
        fn log10(self) -> f64 {
            libm::log10(self)
        }
        #[inline]
        fn powf(self, n: f64) -> f64 {
            libm::pow(self, n)
        }
        #[inline]
        fn powi(self, n: i32) -> f64 {
            libm::pow(self, n as f64)
        }
        #[inline]
        fn floor(self) -> f64 {
            libm::floor(self)
        }
        #[inline]
        fn ceil(self) -> f64 {
            libm::ceil(self)
        }
        #[inline]
        fn round(self) -> f64 {
            libm::round(self)
        }
        #[inline]
        fn hypot(self, y: f64) -> f64 {
            libm::hypot(self, y)
        }
    }
}

/// What every module needs from `alloc` plus the math fallbacks (a no-op under `std`).
#[allow(unused_imports)]
pub(crate) mod prelude {
    pub use crate::math::F64Ext;
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

pub mod colorimetry;
pub mod constants;
pub mod daylight;
pub mod dispersion;
pub mod lut;
pub mod mesopic;
pub mod metrics;
pub mod spd;
pub mod synth;
pub mod uv;

pub use colorimetry::{analyze, Colorimetry};
pub use dispersion::Sellmeier;
pub use mesopic::{mesopic_coefficient, mesopic_luminance};
pub use metrics::{melanopic_der, sp_ratio, SpectralMetrics};
pub use spd::{Spd, SpectralSampler};
pub use uv::{ActionSpectrum, ACTINIC_LIMIT_J_PER_M2};
