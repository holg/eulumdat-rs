//! Mesh generation for 3D photometric visualizations

use crate::PhotometricWeb;

/// A 3D vertex with position and normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Z coordinate
    pub z: f32,
    /// Normal X component
    pub nx: f32,
    /// Normal Y component
    pub ny: f32,
    /// Normal Z component
    pub nz: f32,
}

impl Vertex {
    /// Create a new vertex with position only (normal will be computed later).
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            nx: 0.0,
            ny: 0.0,
            nz: 0.0,
        }
    }

    /// Create a vertex with position and normal.
    pub fn with_normal(x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32) -> Self {
        Self {
            x,
            y,
            z,
            nx,
            ny,
            nz,
        }
    }
}

/// A 3D mesh representing the LDC (Luminous Distribution Curve) solid.
///
/// This is the "photometric solid" - a 3D surface where distance from
/// center equals intensity at that angle.
#[derive(Debug, Clone)]
pub struct LdcMesh {
    /// Vertex positions and normals
    pub vertices: Vec<Vertex>,
    /// Triangle indices (3 per triangle)
    pub indices: Vec<u32>,
    /// Number of C-plane divisions
    pub c_divisions: usize,
    /// Number of gamma divisions
    pub g_divisions: usize,
}

impl LdcMesh {
    /// Generate an LDC solid mesh from a PhotometricWeb.
    ///
    /// # Arguments
    /// * `web` - The photometric web to generate from
    /// * `c_step` - Angle step for C-planes in degrees (e.g., 5.0 for smooth, 15.0 for fast)
    /// * `g_step` - Angle step for gamma in degrees
    /// * `scale` - Scale factor for the mesh (1.0 = normalized intensity as radius)
    ///
    /// # Coordinate System
    /// - Y axis points up (nadir at -Y, zenith at +Y)
    /// - X-Z plane is horizontal
    /// - C=0° is along +Z axis, C=90° is along +X axis
    pub fn from_photweb(web: &PhotometricWeb, c_step: f64, g_step: f64, scale: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Calculate grid dimensions
        let c_count = (360.0 / c_step).ceil() as usize + 1;
        let g_count = (180.0 / g_step).ceil() as usize + 1;

        // Generate vertices
        for gi in 0..g_count {
            let g_angle = (gi as f64 * g_step).min(180.0);
            let g_rad = g_angle.to_radians();

            for ci in 0..c_count {
                let c_angle = (ci as f64 * c_step).min(360.0);
                let c_rad = c_angle.to_radians();

                // Get normalized intensity as radius
                let radius = web.sample_normalized(c_angle, g_angle) as f32 * scale;

                // Spherical to Cartesian conversion
                // gamma = 0 is nadir (-Y), gamma = 90 is horizontal, gamma = 180 is zenith (+Y)
                let sin_g = g_rad.sin() as f32;
                let cos_g = g_rad.cos() as f32;
                let sin_c = c_rad.sin() as f32;
                let cos_c = c_rad.cos() as f32;

                let x = radius * sin_g * sin_c;
                let y = -radius * cos_g; // Negative because gamma=0 is down
                let z = radius * sin_g * cos_c;

                // Normal points outward (same direction as position for a sphere-like surface)
                let len = (x * x + y * y + z * z).sqrt();
                let (nx, ny, nz) = if len > 0.0001 {
                    (x / len, y / len, z / len)
                } else {
                    (0.0, -1.0, 0.0) // Default normal pointing down for degenerate case
                };

                vertices.push(Vertex::with_normal(x, y, z, nx, ny, nz));
            }
        }

        // Generate triangle indices
        // Connect vertices in a grid pattern
        for gi in 0..g_count - 1 {
            for ci in 0..c_count - 1 {
                let i00 = (gi * c_count + ci) as u32;
                let i01 = (gi * c_count + ci + 1) as u32;
                let i10 = ((gi + 1) * c_count + ci) as u32;
                let i11 = ((gi + 1) * c_count + ci + 1) as u32;

                // Two triangles per quad
                // Triangle 1: i00, i10, i01
                indices.push(i00);
                indices.push(i10);
                indices.push(i01);

                // Triangle 2: i01, i10, i11
                indices.push(i01);
                indices.push(i10);
                indices.push(i11);
            }
        }

        Self {
            vertices,
            indices,
            c_divisions: c_count,
            g_divisions: g_count,
        }
    }

    /// Get vertex positions as a flat array [x0, y0, z0, x1, y1, z1, ...].
    ///
    /// Useful for graphics APIs that expect interleaved or separate position data.
    pub fn positions_flat(&self) -> Vec<f32> {
        self.vertices.iter().flat_map(|v| [v.x, v.y, v.z]).collect()
    }

    /// Get vertex normals as a flat array [nx0, ny0, nz0, nx1, ny1, nz1, ...].
    pub fn normals_flat(&self) -> Vec<f32> {
        self.vertices
            .iter()
            .flat_map(|v| [v.nx, v.ny, v.nz])
            .collect()
    }

    /// Get the number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Get the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

impl PhotometricWeb {
    /// Generate LDC solid mesh vertices.
    ///
    /// Convenience method that creates an LdcMesh.
    pub fn generate_ldc_mesh(&self, c_step: f64, g_step: f64, scale: f32) -> LdcMesh {
        LdcMesh::from_photweb(self, c_step, g_step, scale)
    }

    /// Generate just the vertex positions for the LDC solid.
    ///
    /// Returns a vector of (x, y, z) tuples.
    pub fn generate_ldc_vertices(
        &self,
        c_step: f64,
        g_step: f64,
        scale: f32,
    ) -> Vec<(f32, f32, f32)> {
        let mesh = self.generate_ldc_mesh(c_step, g_step, scale);
        mesh.vertices.iter().map(|v| (v.x, v.y, v.z)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eulumdat::Symmetry;

    fn create_uniform_web() -> PhotometricWeb {
        // Uniform intensity in all directions = perfect sphere
        PhotometricWeb::new(
            vec![0.0, 90.0, 180.0, 270.0],
            vec![0.0, 45.0, 90.0, 135.0, 180.0],
            vec![
                vec![100.0, 100.0, 100.0, 100.0, 100.0],
                vec![100.0, 100.0, 100.0, 100.0, 100.0],
                vec![100.0, 100.0, 100.0, 100.0, 100.0],
                vec![100.0, 100.0, 100.0, 100.0, 100.0],
            ],
            Symmetry::None,
        )
    }

    #[test]
    fn test_ldc_mesh_generation() {
        let web = create_uniform_web();
        let mesh = web.generate_ldc_mesh(45.0, 45.0, 1.0);

        // Should have vertices and indices
        assert!(mesh.vertex_count() > 0);
        assert!(mesh.triangle_count() > 0);

        // Indices should be valid
        for &idx in &mesh.indices {
            assert!((idx as usize) < mesh.vertex_count());
        }
    }

    #[test]
    fn test_uniform_sphere_radii() {
        let web = create_uniform_web();
        let mesh = web.generate_ldc_mesh(30.0, 30.0, 1.0);

        // For uniform intensity, all vertices should be at approximately same distance from origin
        for v in &mesh.vertices {
            let r = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
            // Allow some tolerance for edge cases (poles)
            if r > 0.01 {
                assert!((r - 1.0).abs() < 0.01, "Expected radius ~1.0, got {}", r);
            }
        }
    }

    #[test]
    fn test_nadir_zenith_positions() {
        let web = create_uniform_web();
        let mesh = web.generate_ldc_mesh(90.0, 90.0, 1.0);

        // Find vertex at gamma=0 (nadir) - should be at (0, -1, 0) for normalized
        let nadir = mesh.vertices.iter().find(|v| v.y < -0.9);
        assert!(nadir.is_some(), "Should have nadir vertex");

        // Find vertex at gamma=180 (zenith) - should be at (0, +1, 0) for normalized
        let zenith = mesh.vertices.iter().find(|v| v.y > 0.9);
        assert!(zenith.is_some(), "Should have zenith vertex");
    }

    #[test]
    fn test_flat_arrays() {
        let web = create_uniform_web();
        let mesh = web.generate_ldc_mesh(90.0, 90.0, 1.0);

        let positions = mesh.positions_flat();
        let normals = mesh.normals_flat();

        assert_eq!(positions.len(), mesh.vertex_count() * 3);
        assert_eq!(normals.len(), mesh.vertex_count() * 3);
    }
}
