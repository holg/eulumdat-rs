//! Mesh generation for 3D photometric visualizations

use crate::PhotometricWeb;

// ============================================================================
// Color utilities (platform-independent)
// ============================================================================

/// Color mode for 3D mesh visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Heatmap based on intensity (blue -> cyan -> green -> yellow -> red)
    #[default]
    Heatmap,
    /// Rainbow colors based on C-plane angle
    CPlaneRainbow,
    /// Solid color (default blue)
    Solid,
}

/// RGBA color (0.0 - 1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create color from heatmap value (0.0-1.0)
    /// Blue -> Cyan -> Green -> Yellow -> Red
    pub fn from_heatmap(intensity: f32) -> Self {
        let v = intensity.clamp(0.0, 1.0);
        let (r, g, b) = if v < 0.25 {
            let t = v / 0.25;
            (0.0, t, 1.0)
        } else if v < 0.5 {
            let t = (v - 0.25) / 0.25;
            (0.0, 1.0, 1.0 - t)
        } else if v < 0.75 {
            let t = (v - 0.5) / 0.25;
            (t, 1.0, 0.0)
        } else {
            let t = (v - 0.75) / 0.25;
            (1.0, 1.0 - t, 0.0)
        };
        Self::new(r, g, b, 0.9)
    }

    /// Create rainbow color from angle (0-360 degrees)
    pub fn from_c_plane_angle(c_angle: f32) -> Self {
        let hue = c_angle / 360.0;
        let (r, g, b) = hsl_to_rgb(hue, 0.7, 0.5);
        Self::new(r, g, b, 0.9)
    }

    /// Default solid color (semi-transparent blue)
    pub fn solid_default() -> Self {
        Self::new(0.3, 0.5, 0.9, 0.9)
    }
}

/// Convert HSL to RGB (all values 0.0-1.0)
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    }

    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

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

    /// Generate per-vertex colors based on mode.
    ///
    /// Uses the photometric web to sample intensity at each vertex's angle.
    pub fn generate_colors(
        &self,
        web: &PhotometricWeb,
        c_step: f64,
        g_step: f64,
        mode: ColorMode,
    ) -> Vec<Color> {
        let mut colors = Vec::with_capacity(self.vertex_count());

        for gi in 0..self.g_divisions {
            let g_angle = (gi as f64 * g_step).min(180.0);
            for ci in 0..self.c_divisions {
                let c_angle = (ci as f64 * c_step).min(360.0);

                let color = match mode {
                    ColorMode::Heatmap => {
                        let intensity = web.sample_normalized(c_angle, g_angle) as f32;
                        Color::from_heatmap(intensity)
                    }
                    ColorMode::CPlaneRainbow => Color::from_c_plane_angle(c_angle as f32),
                    ColorMode::Solid => Color::solid_default(),
                };
                colors.push(color);
            }
        }
        colors
    }

    /// Get colors as a flat RGBA array [r0, g0, b0, a0, r1, g1, b1, a1, ...]
    pub fn colors_flat(colors: &[Color]) -> Vec<f32> {
        colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect()
    }
}

/// A colored 3D mesh with positions, normals, colors, and indices.
///
/// This is a convenience wrapper that combines `LdcMesh` with per-vertex colors.
#[derive(Debug, Clone)]
pub struct ColoredLdcMesh {
    /// The base mesh (positions, normals, indices)
    pub mesh: LdcMesh,
    /// Per-vertex colors
    pub colors: Vec<Color>,
    /// The color mode used to generate colors
    pub color_mode: ColorMode,
}

impl ColoredLdcMesh {
    /// Generate a colored LDC mesh from a PhotometricWeb.
    ///
    /// # Arguments
    /// * `web` - The photometric web to generate from
    /// * `c_step` - Angle step for C-planes in degrees
    /// * `g_step` - Angle step for gamma in degrees
    /// * `scale` - Scale factor for the mesh
    /// * `color_mode` - How to color the vertices
    pub fn from_photweb(
        web: &PhotometricWeb,
        c_step: f64,
        g_step: f64,
        scale: f32,
        color_mode: ColorMode,
    ) -> Self {
        let mesh = LdcMesh::from_photweb(web, c_step, g_step, scale);
        let colors = mesh.generate_colors(web, c_step, g_step, color_mode);
        Self {
            mesh,
            colors,
            color_mode,
        }
    }

    /// Get vertex positions as a flat array.
    pub fn positions_flat(&self) -> Vec<f32> {
        self.mesh.positions_flat()
    }

    /// Get vertex normals as a flat array.
    pub fn normals_flat(&self) -> Vec<f32> {
        self.mesh.normals_flat()
    }

    /// Get vertex colors as a flat RGBA array.
    pub fn colors_flat(&self) -> Vec<f32> {
        LdcMesh::colors_flat(&self.colors)
    }

    /// Get triangle indices.
    pub fn indices(&self) -> &[u32] {
        &self.mesh.indices
    }

    /// Get vertex count.
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
    }

    /// Get index count.
    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

impl PhotometricWeb {
    /// Generate LDC solid mesh vertices.
    ///
    /// Convenience method that creates an LdcMesh.
    pub fn generate_ldc_mesh(&self, c_step: f64, g_step: f64, scale: f32) -> LdcMesh {
        LdcMesh::from_photweb(self, c_step, g_step, scale)
    }

    /// Generate a colored LDC solid mesh.
    ///
    /// Convenience method that creates a ColoredLdcMesh with positions, normals, colors, and indices.
    pub fn generate_colored_ldc_mesh(
        &self,
        c_step: f64,
        g_step: f64,
        scale: f32,
        color_mode: ColorMode,
    ) -> ColoredLdcMesh {
        ColoredLdcMesh::from_photweb(self, c_step, g_step, scale, color_mode)
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

    #[test]
    fn test_colored_mesh() {
        let web = create_uniform_web();
        let colored = web.generate_colored_ldc_mesh(45.0, 45.0, 1.0, ColorMode::Heatmap);

        // Should have positions, normals, and colors
        assert!(colored.vertex_count() > 0);
        assert_eq!(colored.colors.len(), colored.vertex_count());

        // Flat arrays should have correct lengths
        let positions = colored.positions_flat();
        let normals = colored.normals_flat();
        let colors = colored.colors_flat();

        assert_eq!(positions.len(), colored.vertex_count() * 3);
        assert_eq!(normals.len(), colored.vertex_count() * 3);
        assert_eq!(colors.len(), colored.vertex_count() * 4); // RGBA
    }

    #[test]
    fn test_heatmap_colors() {
        // Low intensity = blue
        let blue = Color::from_heatmap(0.0);
        assert!(blue.b > blue.r && blue.b > blue.g);

        // High intensity = red
        let red = Color::from_heatmap(1.0);
        assert!(red.r > red.g && red.r > red.b);

        // Middle = green-ish
        let mid = Color::from_heatmap(0.5);
        assert!(mid.g > 0.5);
    }

    #[test]
    fn test_c_plane_colors() {
        // C=0° and C=360° should give same color (within tolerance)
        let c0 = Color::from_c_plane_angle(0.0);
        let c360 = Color::from_c_plane_angle(360.0);
        assert!((c0.r - c360.r).abs() < 0.01);
        assert!((c0.g - c360.g).abs() < 0.01);
        assert!((c0.b - c360.b).abs() < 0.01);
    }
}
