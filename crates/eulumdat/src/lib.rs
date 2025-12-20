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

pub mod batch;
pub mod bug_rating;
mod calculations;
pub mod diagram;
mod error;
mod eulumdat;
mod ies;
mod parser;
mod symmetry;
mod validation;
mod writer;

pub use batch::{BatchInput, BatchOutput, BatchStats, ConversionFormat, InputFormat};
pub use bug_rating::{BugDiagram, BugRating, ZoneLumens};
pub use calculations::{
    BeamFieldAnalysis, CieFluxCodes, GldfPhotometricData, IesMetadata, PhotometricCalculations,
    PhotometricSummary, UgrParams, UgrTableValues, ZonalLumens30,
};
pub use error::{Error, Result};
pub use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
pub use ies::{
    validate_ies, validate_ies_strict, FileGenerationType, IesData, IesExportOptions, IesExporter,
    IesParser, IesValidationSeverity, IesValidationWarning, IesVersion, LampPosition,
    LuminousShape, PhotometricType, TiltData, UnitType,
};
pub use symmetry::SymmetryHandler;
pub use validation::{validate, validate_strict, ValidationError, ValidationWarning};
