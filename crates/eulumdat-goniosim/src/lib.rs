//! eulumdat-goniosim — CPU Monte Carlo photon tracer for virtual goniophotometry.
//!
//! Pure Rust crate for tracing photons through luminaire geometry and collecting
//! them on a virtual goniophotometer sphere. This is the **reference implementation**
//! that produces numerically correct results. Any future GPU tracer must validate
//! against it.
//!
//! # Two-Layer Material System
//!
//! Users work with [`MaterialParams`] using manufacturer datasheet values
//! (reflectance %, IOR, transmittance %, thickness, diffusion %).
//! The tracer converts them to internal physics representations automatically.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat_goniosim::*;
//!
//! // Build a scene: LED + white housing + opal PMMA cover
//! let scene = SceneBuilder::new()
//!     .source(Source::Led {
//!         position: nalgebra::Point3::origin(),
//!         direction: nalgebra::Unit::new_unchecked(
//!             nalgebra::Vector3::new(0.0, 0.0, -1.0),
//!         ),
//!         half_angle_deg: 60.0,
//!         flux_lm: 1000.0,
//!     })
//!     .reflector(catalog::white_paint(), ReflectorPlacement {
//!         distance_mm: 25.0,
//!         length_mm: 50.0,
//!         side: ReflectorSide::Surround,
//!     })
//!     .cover(catalog::opal_pmma_3mm(), CoverPlacement {
//!         distance_mm: 40.0,
//!         width_mm: 60.0,
//!         height_mm: 60.0,
//!     })
//!     .build();
//!
//! // Trace 1M photons
//! let config = TracerConfig {
//!     num_photons: 1_000_000,
//!     ..TracerConfig::default()
//! };
//! let result = Tracer::trace(&scene, &config);
//!
//! // Export to EULUMDAT
//! let ldt = detector_to_eulumdat(
//!     &result.detector,
//!     1000.0,
//!     &ExportConfig::default(),
//! );
//! let ldt_string = ldt.to_ldt();
//! ```

pub mod catalog;
pub mod detector;
pub mod export;
pub mod geometry;
pub mod material;
pub mod ray;
pub mod scene;
pub mod source;
pub mod tracer;

/// Material index type.
pub type MaterialId = usize;

// Re-export dependencies for downstream crates (e.g. eulumdat-wasm)
pub use nalgebra;
pub use rand;
pub use rand_xoshiro;

// Re-exports for convenience
pub use catalog::material_catalog;
pub use detector::Detector;
pub use export::{
    detector_to_eulumdat, detector_to_eulumdat_at_angles, detector_to_eulumdat_with_lamp_flux,
    ExportConfig,
};
pub use geometry::{Primitive, SceneObject};
pub use material::{Interaction, Material, MaterialParams};
pub use ray::{HitRecord, Photon, Ray};
pub use scene::{
    bare_isotropic, bare_lambertian, led_housing_with_cover, led_with_housing,
    roundtrip_validation, CoverPlacement, ReflectorPlacement, ReflectorSide, Scene, SceneBuilder,
};
pub use source::Source;
pub use tracer::{
    PhotonTrail, ProgressInfo, Tracer, TracerConfig, TracerResult, TracerStats, TrailEvent,
    TrailPoint,
};
