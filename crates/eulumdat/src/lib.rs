//! # eulumdat
//!
//! A Rust library for parsing, writing, and validating **EULUMDAT (LDT)** and **IES** photometric files.
//!
//! ## About EULUMDAT
//!
//! EULUMDAT (European Lumen Data format) is a photometric data file format used primarily in Europe
//! for specifying luminous intensity distributions from light sources such as lamps and luminaires.
//! The format was proposed by Axel Stockmar (Light Consult Inc., Berlin) in 1990 and uses the `.ldt`
//! file extension.
//!
//! The format is documented in:
//! - Stockmar, A. W. (1990): "EULUMDAT - ein Leuchtendatenformat für den europäischen Beleuchtungsplaner",
//!   Tagungsband Licht '90, pp. 641-644.
//! - Stockmar, A. W. (1998): "EULUMDAT/2 - Extended Version of a Well Established Luminaire Data Format",
//!   CIBSE National Lighting Conference 1998, pp. 353-362.
//!
//! ### EULUMDAT vs IES
//!
//! | Feature | EULUMDAT (.ldt) | IES (.ies) |
//! |---------|-----------------|------------|
//! | Region | Europe | North America |
//! | Standard | Stockmar 1990 | IESNA LM-63-2002 |
//! | Angle convention | C-planes from 0° | Horizontal/vertical angles |
//! | Symmetry | 5 types (Isym 0-4) | 5 types |
//!
//! ## Features
//!
//! - **Parse LDT files** - Full EULUMDAT format support with European decimal handling (comma as separator)
//! - **Write LDT files** - Roundtrip-tested output generation
//! - **Export to IES** - IESNA LM-63-2002 format export
//! - **Validation** - 44 validation constraints with detailed warnings
//! - **Symmetry handling** - 5 symmetry types with automatic data expansion
//! - **Photometric calculations** - Downward flux, beam angles, utilization factors
//! - **BUG Rating** - IESNA TM-15-11 Backlight-Uplight-Glare calculations
//! - **Diagram generation** - Platform-independent data for visualizations
//!
//! ## EULUMDAT File Structure
//!
//! The EULUMDAT format is a plain ASCII text file with the following structure:
//!
//! | Line | Field | Description |
//! |------|-------|-------------|
//! | 1 | Identification | Company/database/version (max 78 chars) |
//! | 2 | Ityp | Type indicator (0-3) |
//! | 3 | Isym | Symmetry indicator (0-4) |
//! | 4 | Mc | Number of C-planes |
//! | 5 | Dc | Distance between C-planes (°) |
//! | 6 | Ng | Number of gamma angles per C-plane |
//! | 7 | Dg | Distance between gamma angles (°) |
//! | 8-12 | Metadata | Certificate, luminaire info, filename |
//! | 13-21 | Dimensions | Luminaire and luminous area dimensions (mm) |
//! | 22-26 | Photometric | DFF, LORL, conversion factor, tilt |
//! | 26a-26b | Lamp sets | Number of lamp sets + lamp data |
//! | 27 | Direct ratios | 10 utilization factor values |
//! | 28 | C-angles | Mc angle values |
//! | 29 | G-angles | Ng angle values |
//! | 30+ | Intensities | Mc × Ng intensity values (cd/klm) |
//!
//! ### Type Indicators (Ityp)
//!
//! - **0**: Point source with no symmetry
//! - **1**: Point source with vertical axis symmetry
//! - **2**: Linear luminaire
//! - **3**: Point source with other symmetry types
//!
//! ### Symmetry Indicators (Isym)
//!
//! - **0**: No symmetry - all C-planes from 0° to 360°
//! - **1**: Vertical axis symmetry - single C-plane
//! - **2**: C0-C180 plane symmetry - C-planes from 0° to 180°
//! - **3**: C90-C270 plane symmetry - C-planes from 90° to 270°
//! - **4**: Both plane symmetry - C-planes from 0° to 90°
//!
//! ## BUG Rating System
//!
//! This crate implements the IESNA TM-15-11 BUG (Backlight-Uplight-Glare) rating system for
//! evaluating outdoor luminaire optical performance related to light trespass, sky glow, and
//! high-angle brightness control.
//!
//! The BUG rating divides the luminaire's light distribution into zones:
//! - **Backlight (B0-B5)**: Light emitted behind the luminaire
//! - **Uplight (U0-U5)**: Light emitted above horizontal (contributing to sky glow)
//! - **Glare (G0-G5)**: Forward high-angle light (causing visual discomfort)
//!
//! Lower ratings indicate better control of unwanted light. The system is incorporated in:
//! - IDA/IES Model Lighting Ordinance
//! - LEED v4 Light Pollution Reduction credit
//! - California Title 24 Energy Code
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, IesExporter};
//!
//! // Parse from file
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//!
//! // Access photometric data
//! println!("Luminaire: {}", ldt.luminaire_name);
//! println!("Symmetry: {:?}", ldt.symmetry);
//! println!("C-planes: {}", ldt.c_angles.len());
//! println!("Gamma angles: {}", ldt.g_angles.len());
//!
//! // Validate the data
//! for warning in ldt.validate() {
//!     println!("Warning [{}]: {}", warning.code, warning.message);
//! }
//!
//! // Export to IES format
//! let ies_content = IesExporter::export(&ldt);
//! std::fs::write("luminaire.ies", ies_content)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Diagram Generation
//!
//! The library provides platform-independent diagram data structures that can be rendered
//! to SVG or used with any graphics library:
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, diagram::*};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//!
//! // Polar diagram (C0-C180 and C90-C270 curves)
//! let polar = PolarDiagram::from_eulumdat(&ldt);
//! let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());
//!
//! // Butterfly diagram (3D isometric projection)
//! let butterfly = ButterflyDiagram::from_eulumdat(&ldt, 500.0, 400.0, 60.0);
//! let svg = butterfly.to_svg(500.0, 400.0, &SvgTheme::dark());
//!
//! // Cartesian diagram (intensity vs gamma, max 8 curves)
//! let cartesian = CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
//!
//! // Heatmap diagram (2D intensity grid)
//! let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## BUG Rating Calculation
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, BugDiagram, diagram::SvgTheme};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//!
//! // Calculate BUG rating
//! let bug = BugDiagram::from_eulumdat(&ldt);
//! println!("BUG Rating: {}", bug.rating); // e.g., "B2 U1 G3"
//!
//! // Generate TM-15-11 BUG diagram
//! let bug_svg = bug.to_svg(400.0, 350.0, &SvgTheme::light());
//!
//! // Generate TM-15-07 LCS (Luminaire Classification System) diagram
//! let lcs_svg = bug.to_lcs_svg(510.0, 315.0, &SvgTheme::light());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## References
//!
//! - [EULUMDAT File Format (Paul Bourke)](https://paulbourke.net/dataformats/ldt/)
//! - [AGi32 EULUMDAT Documentation](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/eulumdat_file_format.htm)
//! - [DIALux EULUMDAT Format](https://evo.support-en.dial.de/support/solutions/articles/9000074164-description-of-the-eulumdat-format)
//! - [IESNA LM-63-2002 Standard](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/iesna_lm-63_format.htm)
//! - [IES TM-15-11 BUG Ratings](https://www.ies.org/wp-content/uploads/2017/03/TM-15-11BUGRatingsAddendum.pdf)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Float functions the photometry needs. With `std` the inherent `f64` methods
/// win (method resolution prefers them); without `std` this trait supplies them
/// via libm.
pub mod math {
    pub trait F64Ext {
        fn sin(self) -> f64;
        fn cos(self) -> f64;
        fn tan(self) -> f64;
        fn asin(self) -> f64;
        fn acos(self) -> f64;
        fn atan(self) -> f64;
        fn atan2(self, x: f64) -> f64;
        fn sqrt(self) -> f64;
        fn cbrt(self) -> f64;
        fn powf(self, y: f64) -> f64;
        fn powi(self, n: i32) -> f64;
        fn ln(self) -> f64;
        fn log10(self) -> f64;
        fn log2(self) -> f64;
        fn exp(self) -> f64;
        fn exp2(self) -> f64;
        fn floor(self) -> f64;
        fn ceil(self) -> f64;
        fn round(self) -> f64;
        fn trunc(self) -> f64;
        fn fract(self) -> f64;
        fn hypot(self, y: f64) -> f64;
        fn mul_add(self, a: f64, b: f64) -> f64;
        fn round_ties_even(self) -> f64;
        fn rem_euclid(self, m: f64) -> f64;
    }
    impl F64Ext for f64 {
        #[inline]
        fn sin(self) -> f64 {
            libm::sin(self)
        }
        #[inline]
        fn cos(self) -> f64 {
            libm::cos(self)
        }
        #[inline]
        fn tan(self) -> f64 {
            libm::tan(self)
        }
        #[inline]
        fn asin(self) -> f64 {
            libm::asin(self)
        }
        #[inline]
        fn acos(self) -> f64 {
            libm::acos(self)
        }
        #[inline]
        fn atan(self) -> f64 {
            libm::atan(self)
        }
        #[inline]
        fn atan2(self, x: f64) -> f64 {
            libm::atan2(self, x)
        }
        #[inline]
        fn sqrt(self) -> f64 {
            libm::sqrt(self)
        }
        #[inline]
        fn cbrt(self) -> f64 {
            libm::cbrt(self)
        }
        #[inline]
        fn powf(self, y: f64) -> f64 {
            libm::pow(self, y)
        }
        #[inline]
        fn powi(self, n: i32) -> f64 {
            libm::pow(self, n as f64)
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
        fn log2(self) -> f64 {
            libm::log2(self)
        }
        #[inline]
        fn exp(self) -> f64 {
            libm::exp(self)
        }
        #[inline]
        fn exp2(self) -> f64 {
            libm::exp2(self)
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
        fn trunc(self) -> f64 {
            libm::trunc(self)
        }
        #[inline]
        fn fract(self) -> f64 {
            self - libm::trunc(self)
        }
        #[inline]
        fn hypot(self, y: f64) -> f64 {
            libm::hypot(self, y)
        }
        #[inline]
        fn mul_add(self, a: f64, b: f64) -> f64 {
            libm::fma(self, a, b)
        }
        #[inline]
        fn round_ties_even(self) -> f64 {
            libm::rint(self)
        }
        #[inline]
        fn rem_euclid(self, m: f64) -> f64 {
            let r = self % m;
            if r < 0.0 {
                r + m.abs()
            } else {
                r
            }
        }
    }
}

/// What every module needs from `alloc` (a no-op under `std`).
#[allow(unused_imports)]
pub(crate) mod prelude {
    pub use crate::math::F64Ext;
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
}

#[cfg(feature = "std")]
pub mod area;
#[cfg(feature = "std")]
pub mod atla;
#[cfg(feature = "std")]
pub mod batch;
#[cfg(feature = "std")]
pub mod bug_rating;
mod calculations;
#[cfg(feature = "std")]
pub mod compare;
pub mod diagram;
mod error;
mod eulumdat;
#[cfg(feature = "std")]
mod ies;
#[cfg(feature = "std")]
pub mod iesna_classification;
#[cfg(feature = "std")]
pub mod interpolate;
mod parser;
#[cfg(feature = "std")]
pub mod radiosity;
#[cfg(feature = "std")]
pub mod road_luminance;
#[cfg(feature = "std")]
pub mod scene3d;
#[cfg(feature = "std")]
pub mod standards;
#[cfg(feature = "std")]
pub mod street;
mod symmetry;
pub mod type_b_conversion;
pub mod units;
mod validation;
mod writer;
#[cfg(feature = "std")]
pub mod zonal;

#[cfg(feature = "std")]
pub use atla::colorimetry::{analyze as analyze_spd, Colorimetry};
#[cfg(feature = "std")]
pub use atla::spd_loader::{
    get_warnings as spd_warnings, load as load_spd, parse as parse_spd, to_atla as spd_to_atla,
    LoadedSpd, ReferenceMetrics, SpdLoadError,
};
#[cfg(feature = "std")]
pub use batch::{BatchInput, BatchOutput, BatchStats, ConversionFormat, InputFormat};
#[cfg(feature = "std")]
pub use bug_rating::{BugDiagram, BugRating, LcsZonePercentages, LightingZone, ZoneLumens};
pub use calculations::{
    format_angle, BeamFieldAnalysis, BeamFieldSections, CandelaEntry, CandelaTabulation,
    CieFluxCodes, ComprehensiveBeamAnalysis, CuTable, DistributionType, GldfPhotometricData,
    IesMetadata, LightDirection, NemaClassification, PeakDirection, PhotometricCalculations,
    PhotometricSummary, SectionBeamField, UgrParams, UgrTable, UgrTableValues, ZonalLumens30,
    CU_RCR_VALUES, CU_REFLECTANCES, UGR_REFLECTANCES, UGR_ROOM_SIZES,
};
#[cfg(feature = "std")]
pub use compare::{ComparisonMetric, PhotometricComparison, Significance};
#[cfg(feature = "std")]
pub use iesna_classification::{
    classify as iesna_classify, Applicability as IesnaApplicability, CutoffClass,
    IesnaClassification, LateralType, LongitudinalClass,
};
#[cfg(feature = "std")]
pub use radiosity::{
    compute_form_factors, direct_illuminance, illuminance_at_point, radiosity_cu, solve_radiosity,
    surface_average, workplane_stats, workplane_stats_at_height, workplane_stats_normative,
    EvaluationStandard, FormFactors, Luminaire, Patch, RadiosityResult, RoomMesh, Surface,
    SurfaceReflectances, Vec3, WorkPlane, WorkplaneStats,
};
// i18n re-exports for comparison are available via PhotometricComparison methods
pub use error::{Error, Result};
pub use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
#[cfg(feature = "std")]
pub use ies::{
    validate_ies, validate_ies_strict, FileGenerationType, IesData, IesExportOptions, IesExporter,
    IesImportOptions, IesParser, IesValidationSeverity, IesValidationWarning, IesVersion,
    LampPosition, LuminousShape, PhotometricType, TiltData, UnitType,
};
pub use symmetry::SymmetryHandler;
pub use type_b_conversion::TypeBConversion;
pub use units::UnitSystem;
pub use validation::{validate, validate_strict, ValidationError, ValidationWarning};
#[cfg(feature = "i18n")]
pub use validation::{validate_strict_with_locale, validate_with_locale};
