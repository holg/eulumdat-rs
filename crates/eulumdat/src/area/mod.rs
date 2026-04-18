//! Area lighting designer — multi-luminaire illuminance computation.
//!
//! Places multiple luminaires on a ground plane and computes combined
//! illuminance with statistics (min/avg/max, uniformity ratios).

mod compute;
pub mod layout;
mod optimize;
mod polygon;
mod svg;

pub use compute::{
    compute_area_illuminance, compute_area_illuminance_mixed, compute_area_illuminance_polygon,
    compute_wall_illuminance, AreaResult, LuminairePlace,
};
pub use layout::{ArrangementType, GridPreset, PoleConfig};
pub use optimize::{optimize_spacing, OptimizationCriteria, OptimizationRow};
pub use polygon::AreaPolygon;
pub use svg::{AreaSvg, ContourOverlay};
