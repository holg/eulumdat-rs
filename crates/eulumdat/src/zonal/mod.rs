//! Zonal Cavity Method — interior lighting design.
//!
//! Implements the IES Zonal Cavity Method for calculating the number
//! of luminaires needed to achieve a target illuminance in a rectangular
//! room. Provides point-by-point overlay for uniformity assessment.

mod compute;
mod presets;
mod svg;

pub use compute::{
    compute_zonal, CavityResults, LightLossFactor, LuminaireLayout, Reflectances, Room,
    SolveMode, ZonalResult,
};
pub use presets::{LlfPreset, ReflectancePreset, RoomPreset};
pub use svg::ZonalSvg;
