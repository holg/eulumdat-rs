//! # eulumdat-photweb
//!
//! Photometric web representation and sampling for EULUMDAT/IES photometric data.
//!
//! This crate provides a `PhotometricWeb` structure that represents the full 3D
//! luminous intensity distribution of a light source. It supports:
//!
//! - **Sampling**: Get intensity at any C/G angle with bilinear interpolation
//! - **Normalization**: Sample normalized (0.0-1.0) intensity values
//! - **Symmetry handling**: Automatic expansion based on symmetry type
//! - **Mesh generation**: Generate 3D LDC solid geometry (coming soon)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use eulumdat::Eulumdat;
//! use eulumdat_photweb::PhotometricWeb;
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt")?;
//! let web = PhotometricWeb::from(&ldt);
//!
//! // Sample at any angle
//! let intensity = web.sample(45.0, 30.0);
//! let normalized = web.sample_normalized(45.0, 30.0);
//!
//! // Generate mesh vertices for 3D visualization
//! let vertices = web.generate_ldc_vertices(5.0, 5.0, 1.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod mesh;
mod photweb;

pub use mesh::{LdcMesh, Vertex};
pub use photweb::PhotometricWeb;
