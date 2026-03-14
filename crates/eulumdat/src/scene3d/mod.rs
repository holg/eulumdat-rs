//! SVG-based axonometric 3D scene projection.
//!
//! Shared by both the Area Lighting Designer (exterior scenes) and the
//! Zonal Cavity Designer (interior scenes). Renders to clean SVG strings
//! that export/print directly.

pub mod exterior;
pub mod interior;

pub use exterior::build_exterior_scene;
pub use interior::build_interior_scene;

/// Camera for axonometric projection.
#[derive(Debug, Clone)]
pub struct SceneCamera {
    /// Horizontal rotation in degrees (0° = front, 90° = right)
    pub azimuth: f64,
    /// Vertical tilt in degrees (0° = side, 90° = top-down)
    pub elevation: f64,
    /// Zoom factor (pixels per meter)
    pub scale: f64,
    /// SVG center point
    pub center: (f64, f64),
}

/// Preset camera positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPreset {
    /// azimuth=30°, elevation=30° — default isometric
    FrontRight,
    /// azimuth=150°, elevation=30°
    FrontLeft,
    /// azimuth=0°, elevation=85° — nearly plan view
    TopDown,
    /// azimuth=30°, elevation=15° — eye-level perspective
    LowAngle,
    /// azimuth=0°, elevation=0° — side cutaway (for interior cavity view)
    Section,
}

impl CameraPreset {
    pub fn all() -> &'static [CameraPreset] {
        &[
            Self::FrontRight,
            Self::FrontLeft,
            Self::TopDown,
            Self::LowAngle,
            Self::Section,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::FrontRight => "Front-Right",
            Self::FrontLeft => "Front-Left",
            Self::TopDown => "Top-Down",
            Self::LowAngle => "Low Angle",
            Self::Section => "Section",
        }
    }

    /// Create a SceneCamera for this preset, centered in the given SVG viewport.
    pub fn to_camera(&self, svg_width: f64, svg_height: f64, scene_scale: f64) -> SceneCamera {
        let (az, el) = match self {
            Self::FrontRight => (30.0, 30.0),
            Self::FrontLeft => (150.0, 30.0),
            Self::TopDown => (0.0, 85.0),
            Self::LowAngle => (30.0, 15.0),
            Self::Section => (0.0, 0.0),
        };
        SceneCamera {
            azimuth: az,
            elevation: el,
            scale: scene_scale,
            center: (svg_width / 2.0, svg_height / 2.0),
        }
    }
}

/// A 3D polygon face for z-sorted SVG rendering.
#[derive(Debug, Clone)]
pub struct SceneFace {
    /// World-space vertex positions (x, y, z)
    pub vertices: Vec<(f64, f64, f64)>,
    /// SVG fill color
    pub fill: String,
    /// SVG stroke color
    pub stroke: String,
    /// Stroke width
    pub stroke_width: f64,
    /// Opacity (0.0–1.0)
    pub opacity: f64,
    /// Optional stroke dasharray (e.g. "6,3")
    pub dash: Option<String>,
    /// Optional text label at the face centroid
    pub label: Option<(String, f64)>,
}

impl SceneFace {
    pub fn quad(
        v0: (f64, f64, f64),
        v1: (f64, f64, f64),
        v2: (f64, f64, f64),
        v3: (f64, f64, f64),
        fill: &str,
        stroke: &str,
        opacity: f64,
    ) -> Self {
        Self {
            vertices: vec![v0, v1, v2, v3],
            fill: fill.to_string(),
            stroke: stroke.to_string(),
            stroke_width: 1.0,
            opacity,
            dash: None,
            label: None,
        }
    }

    pub fn with_stroke_width(mut self, w: f64) -> Self {
        self.stroke_width = w;
        self
    }

    pub fn with_dash(mut self, d: &str) -> Self {
        self.dash = Some(d.to_string());
        self
    }

    pub fn with_label(mut self, text: &str, font_size: f64) -> Self {
        self.label = Some((text.to_string(), font_size));
        self
    }
}

/// Project a 3D point to 2D SVG coordinates using axonometric projection.
///
/// Convention: X = east, Y = north, Z = up.
/// SVG Y-axis is flipped (top-left origin).
pub fn project(camera: &SceneCamera, p: (f64, f64, f64)) -> (f64, f64) {
    let az = camera.azimuth.to_radians();
    let el = camera.elevation.to_radians();
    let (x, y, z) = p;
    let sx = (x * az.cos() - y * az.sin()) * camera.scale;
    let sy = (-(x * az.sin() + y * az.cos()) * el.sin() + z * el.cos()) * camera.scale;
    (camera.center.0 + sx, camera.center.1 - sy)
}

/// Compute view-space depth for z-sorting (larger = farther from camera).
fn view_depth(camera: &SceneCamera, p: (f64, f64, f64)) -> f64 {
    let az = camera.azimuth.to_radians();
    let el = camera.elevation.to_radians();
    let (x, y, z) = p;
    (x * az.sin() + y * az.cos()) * el.cos() + z * el.sin()
}

/// Average depth of a face for painter's algorithm sorting.
fn face_depth(camera: &SceneCamera, face: &SceneFace) -> f64 {
    if face.vertices.is_empty() {
        return 0.0;
    }
    let sum: f64 = face.vertices.iter().map(|&v| view_depth(camera, v)).sum();
    sum / face.vertices.len() as f64
}

/// Render a list of scene faces to an SVG string.
///
/// Performs z-sort (painter's algorithm: farthest first) and projects all
/// vertices through the camera.
pub fn render_scene_svg(
    faces: &[SceneFace],
    camera: &SceneCamera,
    svg_width: f64,
    svg_height: f64,
    bg: &str,
) -> String {
    // Sort indices by depth (farthest first = ascending depth)
    let mut indices: Vec<usize> = (0..faces.len()).collect();
    indices.sort_by(|&a, &b| {
        face_depth(camera, &faces[a])
            .partial_cmp(&face_depth(camera, &faces[b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" preserveAspectRatio="xMidYMid meet">"#
    );
    svg.push_str(&format!(
        r#"<rect width="{svg_width}" height="{svg_height}" fill="{bg}"/>"#
    ));

    for &idx in &indices {
        let face = &faces[idx];
        if face.vertices.is_empty() {
            continue;
        }

        let projected: Vec<(f64, f64)> = face.vertices.iter().map(|&v| project(camera, v)).collect();

        // Build points string
        let points: String = projected
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        let dash_attr = face
            .dash
            .as_ref()
            .map(|d| format!(r#" stroke-dasharray="{d}""#))
            .unwrap_or_default();

        svg.push_str(&format!(
            r#"<polygon points="{points}" fill="{}" stroke="{}" stroke-width="{:.1}" opacity="{:.2}"{dash_attr}/>"#,
            face.fill, face.stroke, face.stroke_width, face.opacity
        ));

        // Label at centroid
        if let Some((ref text, font_size)) = face.label {
            let cx: f64 = projected.iter().map(|p| p.0).sum::<f64>() / projected.len() as f64;
            let cy: f64 = projected.iter().map(|p| p.1).sum::<f64>() / projected.len() as f64;
            svg.push_str(&format!(
                r#"<text x="{cx:.1}" y="{cy:.1}" fill="{}" font-size="{font_size:.0}" text-anchor="middle" dominant-baseline="middle">{text}</text>"#,
                face.stroke
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Compute an appropriate scene scale to fit a bounding box in the SVG viewport.
///
/// The bounding box dimensions are centered around origin (scenes use centered geometry).
pub fn fit_scale(
    bbox_x: f64,
    bbox_y: f64,
    bbox_z: f64,
    svg_width: f64,
    svg_height: f64,
    camera: &SceneCamera,
) -> f64 {
    // Scene geometry is centered around origin, so half-extents
    let hx = bbox_x / 2.0;
    let hy = bbox_y / 2.0;
    // Z goes from 0 to bbox_z
    let corners = [
        (-hx, -hy, 0.0),
        (hx, -hy, 0.0),
        (-hx, hy, 0.0),
        (hx, hy, 0.0),
        (-hx, -hy, bbox_z),
        (hx, -hy, bbox_z),
        (-hx, hy, bbox_z),
        (hx, hy, bbox_z),
    ];

    // Use unit scale camera to measure raw extent
    let unit_cam = SceneCamera {
        scale: 1.0,
        center: (0.0, 0.0),
        ..*camera
    };

    let projected: Vec<(f64, f64)> = corners.iter().map(|&c| project(&unit_cam, c)).collect();
    let min_x = projected.iter().map(|p| p.0).fold(f64::MAX, f64::min);
    let max_x = projected.iter().map(|p| p.0).fold(f64::MIN, f64::max);
    let min_y = projected.iter().map(|p| p.1).fold(f64::MAX, f64::min);
    let max_y = projected.iter().map(|p| p.1).fold(f64::MIN, f64::max);

    let extent_x = (max_x - min_x).max(0.1);
    let extent_y = (max_y - min_y).max(0.1);

    let margin = 0.85; // leave 15% margin
    let scale_x = svg_width * margin / extent_x;
    let scale_y = svg_height * margin / extent_y;
    scale_x.min(scale_y)
}

/// Helper: reflectance to grayscale RGB string.
pub fn reflectance_to_rgb(rho: f64) -> String {
    let v = (rho.clamp(0.0, 1.0) * 200.0 + 40.0).round() as u8; // 40..240 range
    format!("rgb({v},{v},{v})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_top_down() {
        let cam = CameraPreset::TopDown.to_camera(500.0, 400.0, 20.0);
        let p = project(&cam, (5.0, 0.0, 0.0));
        // Top-down: az≈0, el≈85°. X should map mostly to screen X.
        assert!((p.0 - 250.0).abs() < 200.0, "x={}", p.0);
    }

    #[test]
    fn test_project_origin_at_center() {
        let cam = SceneCamera {
            azimuth: 0.0,
            elevation: 45.0,
            scale: 10.0,
            center: (250.0, 200.0),
        };
        let p = project(&cam, (0.0, 0.0, 0.0));
        assert!((p.0 - 250.0).abs() < 0.01);
        assert!((p.1 - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_render_empty() {
        let cam = CameraPreset::FrontRight.to_camera(500.0, 400.0, 20.0);
        let svg = render_scene_svg(&[], &cam, 500.0, 400.0, "#fff");
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_fit_scale_positive() {
        let cam = CameraPreset::FrontRight.to_camera(500.0, 400.0, 1.0);
        let s = fit_scale(10.0, 8.0, 3.0, 500.0, 400.0, &cam);
        assert!(s > 0.0, "scale = {s}");
        assert!(s < 100.0, "scale = {s}");
    }
}
