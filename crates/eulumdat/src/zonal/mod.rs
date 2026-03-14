//! Zonal Cavity Method — interior lighting design.
//!
//! Implements the IES Zonal Cavity Method for calculating the number
//! of luminaires needed to achieve a target illuminance in a rectangular
//! room. Provides point-by-point overlay for uniformity assessment.

mod compute;
mod presets;
mod svg;

pub use compute::{
    compute_cavity_ratios, compute_ppb_overlay, compute_zonal, effective_cavity_reflectance,
    find_best_layout, interpolate_cu, CavityResults, LightLossFactor, LuminaireLayout, PpbResult,
    Reflectances, Room, SolveMode, ZonalResult,
};
pub use presets::{LlfPreset, ReflectancePreset, RoomPreset};
pub use svg::ZonalSvg;
