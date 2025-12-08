//! 3D Butterfly Diagram Renderer

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eulumdat::Eulumdat;

/// 3D Point
#[derive(Clone, Copy)]
struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

impl Point3D {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn rotate_x(&self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x,
            y: self.y * cos_a - self.z * sin_a,
            z: self.y * sin_a + self.z * cos_a,
        }
    }

    fn rotate_y(&self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a + self.z * sin_a,
            y: self.y,
            z: -self.x * sin_a + self.z * cos_a,
        }
    }

    fn project(&self, cx: f32, cy: f32, scale: f32) -> Pos2 {
        let perspective = 600.0;
        let z_offset = 300.0;
        let factor = perspective / (perspective + self.z + z_offset);
        Pos2 {
            x: cx + (self.x * scale as f64 * factor) as f32,
            y: cy - (self.y * scale as f64 * factor) as f32,
        }
    }
}

/// Wing data for 3D rendering
struct Wing {
    #[allow(dead_code)]
    c_angle: f64,
    points: Vec<Point3D>,
    hue: f32,
}

/// 3D Butterfly Renderer
pub struct Butterfly3DRenderer {
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub auto_rotate: bool,
    wings: Vec<Wing>,
    max_intensity: f64,
}

impl Butterfly3DRenderer {
    pub fn new() -> Self {
        Self {
            rotation_x: 0.5,
            rotation_y: 0.0,
            auto_rotate: true,
            wings: Vec::new(),
            max_intensity: 1.0,
        }
    }

    pub fn reset_view(&mut self) {
        self.rotation_x = 0.5;
        self.rotation_y = 0.0;
    }

    pub fn update_from_eulumdat(&mut self, ldt: Option<&Eulumdat>) {
        self.wings.clear();

        let Some(ldt) = ldt else { return };

        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return;
        }

        self.max_intensity = ldt.max_intensity().max(1.0);

        // Expand C-planes based on symmetry
        let expanded = self.expand_c_planes(ldt);

        for (c_angle, intensities) in expanded {
            let c_rad = c_angle.to_radians();
            let mut points = vec![Point3D::new(0.0, 0.0, 0.0)];

            for (j, g_angle) in ldt.g_angles.iter().enumerate() {
                let intensity = intensities.get(j).copied().unwrap_or(0.0);
                let r = intensity / self.max_intensity;

                let g_rad = g_angle.to_radians();

                // Convert spherical to Cartesian
                let x = r * g_rad.sin() * c_rad.cos();
                let y = r * g_rad.sin() * c_rad.sin();
                let z = r * g_rad.cos();

                points.push(Point3D::new(x, y, -z));
            }

            let hue = ((c_angle / 360.0) * 240.0 + 180.0) as f32 % 360.0;
            self.wings.push(Wing {
                c_angle,
                points,
                hue,
            });
        }
    }

    fn expand_c_planes(&self, ldt: &Eulumdat) -> Vec<(f64, Vec<f64>)> {
        let mut result = Vec::new();

        match ldt.symmetry {
            eulumdat::Symmetry::VerticalAxis => {
                // Rotationally symmetric - replicate C0 around
                if let Some(intensities) = ldt.intensities.first() {
                    for i in 0..12 {
                        result.push((i as f64 * 30.0, intensities.clone()));
                    }
                }
            }
            eulumdat::Symmetry::PlaneC0C180 => {
                // Mirror across C0-C180
                for (i, intensities) in ldt.intensities.iter().enumerate() {
                    let c_angle = ldt.c_angles.get(i).copied().unwrap_or(0.0);
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 0.0 && c_angle < 180.0 {
                        result.push((360.0 - c_angle, intensities.clone()));
                    }
                }
            }
            eulumdat::Symmetry::PlaneC90C270 => {
                // Mirror across C90-C270
                for (i, intensities) in ldt.intensities.iter().enumerate() {
                    let c_angle = ldt.c_angles.get(i).copied().unwrap_or(0.0);
                    result.push((c_angle, intensities.clone()));
                }
            }
            eulumdat::Symmetry::BothPlanes => {
                // Quarter data - mirror both ways
                for (i, intensities) in ldt.intensities.iter().enumerate() {
                    let c_angle = ldt.c_angles.get(i).copied().unwrap_or(0.0);
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 0.0 && c_angle < 90.0 {
                        result.push((180.0 - c_angle, intensities.clone()));
                        result.push((180.0 + c_angle, intensities.clone()));
                        result.push((360.0 - c_angle, intensities.clone()));
                    } else if (c_angle - 90.0).abs() < 0.1 {
                        result.push((270.0, intensities.clone()));
                    }
                }
            }
            eulumdat::Symmetry::None => {
                // Full data
                for (i, intensities) in ldt.intensities.iter().enumerate() {
                    let c_angle = ldt.c_angles.get(i).copied().unwrap_or(0.0);
                    result.push((c_angle, intensities.clone()));
                }
            }
        }

        result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        result
    }

    pub fn render(&self, painter: &egui::Painter, rect: Rect, dark_theme: bool) {
        let cx = rect.center().x;
        let cy = rect.center().y;
        let scale = rect.width().min(rect.height()) / 2.5;

        let grid_color = if dark_theme {
            Color32::from_rgb(64, 64, 96)
        } else {
            Color32::from_rgb(224, 224, 224)
        };

        // Draw grid
        self.draw_grid(painter, cx, cy, scale, grid_color);

        // Sort wings by depth (painter's algorithm)
        let mut sorted_wings: Vec<(&Wing, f64)> = self
            .wings
            .iter()
            .map(|wing| {
                let avg_z: f64 = wing
                    .points
                    .iter()
                    .map(|p| p.rotate_x(self.rotation_x).rotate_y(self.rotation_y).z)
                    .sum::<f64>()
                    / wing.points.len() as f64;
                (wing, avg_z)
            })
            .collect();

        sorted_wings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Draw wings
        for (wing, _) in sorted_wings {
            self.draw_wing(painter, wing, cx, cy, scale);
        }

        // Draw center point
        let center_color = if dark_theme {
            Color32::WHITE
        } else {
            Color32::BLACK
        };
        painter.circle_filled(Pos2::new(cx, cy), 3.0, center_color);
    }

    fn draw_grid(&self, painter: &egui::Painter, cx: f32, cy: f32, scale: f32, color: Color32) {
        // Draw concentric circles
        for i in 1..=4 {
            let r = i as f64 / 4.0;
            let mut points = Vec::new();

            for j in 0..=36 {
                let c_angle = j as f64 * 10.0;
                let c_rad = c_angle.to_radians();

                let point = Point3D::new(r * c_rad.cos(), r * c_rad.sin(), 0.0)
                    .rotate_x(self.rotation_x)
                    .rotate_y(self.rotation_y);

                points.push(point.project(cx, cy, scale));
            }

            for i in 0..points.len() - 1 {
                painter.line_segment([points[i], points[i + 1]], Stroke::new(1.0, color));
            }
        }

        // Draw C-plane direction lines
        for i in 0..8 {
            let c_angle = i as f64 * 45.0;
            let c_rad = c_angle.to_radians();

            let p1 = Point3D::new(0.0, 0.0, 0.0)
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .project(cx, cy, scale);

            let p2 = Point3D::new(c_rad.cos(), c_rad.sin(), 0.0)
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .project(cx, cy, scale);

            painter.line_segment([p1, p2], Stroke::new(1.0, color));
        }
    }

    fn draw_wing(&self, painter: &egui::Painter, wing: &Wing, cx: f32, cy: f32, scale: f32) {
        if wing.points.len() < 2 {
            return;
        }

        let mut projected: Vec<Pos2> = wing
            .points
            .iter()
            .map(|p| {
                p.rotate_x(self.rotation_x)
                    .rotate_y(self.rotation_y)
                    .project(cx, cy, scale)
            })
            .collect();

        // Close the polygon
        if let Some(first) = projected.first() {
            projected.push(*first);
        }

        // Convert hue to RGB
        let (r, g, b) = hsl_to_rgb(wing.hue / 360.0, 0.6, 0.5);
        let fill_color = Color32::from_rgba_unmultiplied(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            128,
        );

        let (r2, g2, b2) = hsl_to_rgb(wing.hue / 360.0, 0.7, 0.6);
        let stroke_color =
            Color32::from_rgb((r2 * 255.0) as u8, (g2 * 255.0) as u8, (b2 * 255.0) as u8);

        // Draw filled polygon
        if projected.len() >= 3 {
            // Use triangle fan for convex-ish polygons
            let center = projected[0];
            for i in 1..projected.len() - 1 {
                let tri = [center, projected[i], projected[i + 1]];
                painter.add(egui::Shape::convex_polygon(
                    tri.to_vec(),
                    fill_color,
                    Stroke::NONE,
                ));
            }
        }

        // Draw outline
        for i in 0..projected.len() - 1 {
            painter.line_segment(
                [projected[i], projected[i + 1]],
                Stroke::new(1.5, stroke_color),
            );
        }
    }
}

impl Default for Butterfly3DRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// HSL to RGB conversion
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r + m, g + m, b + m)
}
