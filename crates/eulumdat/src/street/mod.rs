//! Street / roadway lighting layouts and illuminance computation.
//!
//! A [`StreetLayout`] describes a linear road segment: lane widths, pole
//! spacing, arrangement (single side / opposite / staggered), mounting
//! height, and optional overhang/tilt. The layout is translated into a set
//! of [`crate::area::LuminairePlace`] records and fed into the existing
//! area-compute pipeline to produce an [`crate::area::AreaResult`]. The
//! resulting grid is what the regional compliance standards (RP-8,
//! EN 13201, CJJ 45) check against.
//!
//! This module owns only the layout math. Visualization (3D street scene,
//! plan views) lives in the WASM companion crate.
//!
//! ```no_run
//! use eulumdat::Eulumdat;
//! use eulumdat::street::{StreetLayout, Arrangement};
//!
//! let ldt = Eulumdat::from_file("road.ldt").unwrap();
//! let layout = StreetLayout {
//!     length_m: 120.0,
//!     lane_width_m: 3.5,
//!     num_lanes: 2,
//!     pole_spacing_m: 30.0,
//!     arrangement: Arrangement::Staggered,
//!     mounting_height_m: 10.0,
//!     overhang_m: 1.5,
//!     tilt_deg: 0.0,
//!     pole_offset_m: 0.5,
//! };
//! let result = layout.compute(&ldt, 1.0);
//! println!("avg = {:.1} lux, uniformity = {:.2}", result.avg_lux, result.uniformity_min_avg);
//! ```

pub mod layout;

pub use layout::{Arrangement, StreetLayout};
