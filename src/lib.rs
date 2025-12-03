//! # Eulumdat
//!
//! A Rust library for parsing, writing, and validating Eulumdat (LDT) and IES photometric files.
//!
//! ## Features
//!
//! - Parse LDT (Eulumdat) files
//! - Write LDT files
//! - Export to IES format
//! - Comprehensive validation (41 constraints)
//! - Symmetry-aware data handling with automatic reduction
//! - Photometric calculations (downward flux, utilization factors)
//!
//! ## Example
//!
//! ```rust,no_run
//! use eulumdat::Eulumdat;
//!
//! // Load from file
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//!
//! // Validate
//! let warnings = ldt.validate();
//! for warning in &warnings {
//!     println!("Warning: {}", warning);
//! }
//!
//! // Access data
//! println!("Luminaire: {}", ldt.luminaire_name);
//! println!("Symmetry: {:?}", ldt.symmetry);
//! println!("Total flux: {} lm", ldt.lamp_sets[0].total_luminous_flux);
//! ```

mod error;
mod eulumdat;
mod parser;
mod writer;
mod validation;
mod symmetry;
mod calculations;
mod ies;

pub use error::{Error, Result};
pub use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
pub use validation::{ValidationWarning, ValidationError};
pub use symmetry::SymmetryHandler;
pub use calculations::PhotometricCalculations;
pub use ies::IesExporter;
