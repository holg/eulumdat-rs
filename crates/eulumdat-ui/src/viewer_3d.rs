//! 3D viewer for light distribution visualization
//!
//! This module provides an interactive 3D viewer for visualizing photometric
//! data as a 3D surface. The radius at each point represents the intensity
//! in that direction.

use crate::Theme;
use egui::{pos2, vec2, Color32, Pos2, Sense, Stroke, Vec2};
use eulumdat::Eulumdat;
use std::f64::consts::PI;

/// 3D viewer widget
#[derive(Clone)]
pub struct Viewer3D {
    /// Rotation around Y axis (yaw)
    pub rotation_y: f32,
    /// Rotation around X axis (pitch)
    pub rotation_x: f32,
    /// Zoom level
    pub zoom: f32,
    /// Auto-rotate
    pub auto_rotate: bool,
    /// Show grid
    pub show_grid: bool,
    /// Resolution (number of points per axis)
    pub resolution: usize,
    /// Wireframe mode (no bleeding issues)
    pub wireframe: bool,
}

impl Default for Viewer3D {
    fn default() -> Self {
        Self {
            rotation_y: 0.3,
            rotation_x: 0.4,
            zoom: 1.0,
            auto_rotate: true,
            show_grid: true,
            resolution: 36,
            wireframe: true, // Default to wireframe to avoid bleeding
        }
    }
}

impl Viewer3D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the 3D viewer (static version for simple use)
    pub fn show(ui: &mut egui::Ui, ldt: &Eulumdat, theme: &Theme) {
        // Use a stateful viewer stored in egui's memory
        let id = ui.id().with("viewer_3d_state");
        let mut state = ui.data_mut(|d| d.get_temp::<Viewer3D>(id).unwrap_or_default());
        state.show_interactive(ui, ldt, theme);
        ui.data_mut(|d| d.insert_temp(id, state));
    }

    /// Show with controls
    pub fn show_interactive(&mut self, ui: &mut egui::Ui, ldt: &Eulumdat, theme: &Theme) {
        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.wireframe, "Wireframe");
            ui.checkbox(&mut self.auto_rotate, "Auto-rotate");
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.add(egui::Slider::new(&mut self.zoom, 0.5..=2.0).text("Zoom"));
            ui.add(
                egui::Slider::new(&mut self.resolution, 12..=72)
                    .text("Res")
                    .step_by(6.0),
            );
            if ui.button("Reset").clicked() {
                self.rotation_y = 0.3;
                self.rotation_x = 0.4;
            }
        });

        ui.separator();

        // Canvas
        let available_size = ui.available_size();
        let size = available_size.min_elem().min(600.0);
        let (response, painter) = ui.allocate_painter(Vec2::splat(size), Sense::drag());

        let rect = response.rect;
        let center = rect.center();

        // Background
        painter.rect_filled(rect, 0.0, theme.background);

        // Handle drag for rotation
        if response.dragged() {
            let delta = response.drag_delta();
            self.rotation_y += delta.x * 0.01;
            self.rotation_x += delta.y * 0.01;
            self.rotation_x = self.rotation_x.clamp(-PI as f32 / 2.0, PI as f32 / 2.0);
        }

        // Auto-rotate
        if self.auto_rotate {
            self.rotation_y += 0.005;
            ui.ctx().request_repaint();
        }

        // Generate and render 3D mesh
        let scale = (size / 2.0) * 0.7 * self.zoom;
        let max_intensity = ldt.max_intensity();

        if max_intensity <= 0.0 {
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "No intensity data",
                egui::FontId::proportional(14.0),
                theme.text,
            );
            return;
        }

        // Draw grid at horizontal plane
        if self.show_grid {
            self.draw_grid(&painter, center, scale, theme);
        }

        // Generate points on the intensity surface
        let mut polygons = Vec::new();

        let c_step = 360.0 / self.resolution as f64;
        let g_step = 180.0 / (self.resolution / 2) as f64;

        for c_idx in 0..self.resolution {
            let c1 = c_idx as f64 * c_step;
            let c2 = ((c_idx + 1) % self.resolution) as f64 * c_step;

            for g_idx in 0..(self.resolution / 2) {
                let g1 = g_idx as f64 * g_step;
                let g2 = (g_idx + 1) as f64 * g_step;

                // Get intensities at corners
                let i1 = ldt.sample(c1, g1) / max_intensity;
                let i2 = ldt.sample(c2, g1) / max_intensity;
                let i3 = ldt.sample(c2, g2) / max_intensity;
                let i4 = ldt.sample(c1, g2) / max_intensity;

                // Convert to 3D points
                let p1 = self.spherical_to_screen(c1, g1, i1, center, scale);
                let p2 = self.spherical_to_screen(c2, g1, i2, center, scale);
                let p3 = self.spherical_to_screen(c2, g2, i3, center, scale);
                let p4 = self.spherical_to_screen(c1, g2, i4, center, scale);

                // Use centroid depth for painter's algorithm sorting
                let avg_z = (p1.2 + p2.2 + p3.2 + p4.2) / 4.0;

                // Color based on average intensity
                let avg_intensity = (i1 + i2 + i3 + i4) / 4.0;
                let color = theme.heatmap_color(avg_intensity);

                polygons.push((
                    avg_z,
                    [
                        pos2(p1.0, p1.1),
                        pos2(p2.0, p2.1),
                        pos2(p3.0, p3.1),
                        pos2(p4.0, p4.1),
                    ],
                    color,
                ));
            }
        }

        // Sort by depth (painter's algorithm - back to front)
        polygons.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if self.wireframe {
            // Wireframe mode - draw only edges (no bleeding issues)
            for (_, points, color) in &polygons {
                let stroke = Stroke::new(1.0, *color);
                for i in 0..4 {
                    painter.line_segment([points[i], points[(i + 1) % 4]], stroke);
                }
            }
        } else {
            // Filled mode - expand polygons slightly to reduce gaps
            for (_, points, color) in &polygons {
                // Calculate centroid
                let cx = (points[0].x + points[1].x + points[2].x + points[3].x) / 4.0;
                let cy = (points[0].y + points[1].y + points[2].y + points[3].y) / 4.0;

                // Expand each vertex slightly outward from centroid (1.5 pixels)
                let expand = 1.5;
                let expanded: Vec<Pos2> = points
                    .iter()
                    .map(|p| {
                        let dx = p.x - cx;
                        let dy = p.y - cy;
                        let len = (dx * dx + dy * dy).sqrt().max(0.001);
                        pos2(p.x + dx / len * expand, p.y + dy / len * expand)
                    })
                    .collect();

                painter.add(egui::Shape::convex_polygon(expanded, *color, Stroke::NONE));
            }
        }

        // Draw axes
        self.draw_axes(&painter, center, scale, theme);

        // Info
        ui.horizontal(|ui| {
            ui.label(format!("Max intensity: {:.0} cd/klm", max_intensity));
            ui.label("|");
            ui.label("Drag to rotate");
        });
    }

    /// Convert spherical coordinates to screen position
    fn spherical_to_screen(
        &self,
        c_angle: f64,
        g_angle: f64,
        radius: f64,
        center: Pos2,
        scale: f32,
    ) -> (f32, f32, f32) {
        let c_rad = c_angle.to_radians();
        let g_rad = g_angle.to_radians();
        let r = radius as f32;

        // Spherical to Cartesian (Y-up, Z-forward)
        let x = r * (g_rad.sin() as f32) * (c_rad.cos() as f32);
        let z = r * (g_rad.sin() as f32) * (c_rad.sin() as f32);
        let y = r * (g_rad.cos() as f32);

        // Apply rotations
        let (x, z) = self.rotate_y(x, z);
        let (y, z) = self.rotate_x(y, z);

        // Simple perspective projection
        let perspective = 3.0;
        let factor = perspective / (perspective + z + 1.0);

        let screen_x = center.x + x * scale * factor;
        let screen_y = center.y - y * scale * factor;

        (screen_x, screen_y, z)
    }

    fn rotate_y(&self, x: f32, z: f32) -> (f32, f32) {
        let cos = self.rotation_y.cos();
        let sin = self.rotation_y.sin();
        (x * cos - z * sin, x * sin + z * cos)
    }

    fn rotate_x(&self, y: f32, z: f32) -> (f32, f32) {
        let cos = self.rotation_x.cos();
        let sin = self.rotation_x.sin();
        (y * cos - z * sin, y * sin + z * cos)
    }

    fn draw_grid(&self, painter: &egui::Painter, center: Pos2, scale: f32, theme: &Theme) {
        let grid_stroke = Stroke::new(0.5, theme.grid);

        // Horizontal plane grid at gamma = 90°
        let grid_radius = 1.0;
        let num_circles = 4;
        let num_radials = 8;

        // Concentric circles
        for i in 1..=num_circles {
            let r = grid_radius * (i as f32 / num_circles as f32);
            let mut points = Vec::new();

            for c in 0..=36 {
                let c_angle = (c as f64 * 10.0).to_radians();
                let x = r * c_angle.cos() as f32;
                let z = r * c_angle.sin() as f32;
                let y = 0.0_f32;

                let (x, z) = self.rotate_y(x, z);
                let (y, z) = self.rotate_x(y, z);

                let perspective = 3.0;
                let factor = perspective / (perspective + z + 1.0);

                points.push(pos2(
                    center.x + x * scale * factor,
                    center.y - y * scale * factor,
                ));
            }

            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], grid_stroke);
            }
        }

        // Radial lines
        for i in 0..num_radials {
            let c_angle = (i as f64 * 360.0 / num_radials as f64).to_radians();
            let x1 = 0.0_f32;
            let z1 = 0.0_f32;
            let x2 = grid_radius * c_angle.cos() as f32;
            let z2 = grid_radius * c_angle.sin() as f32;

            let (x1, z1) = self.rotate_y(x1, z1);
            let (y1, _) = self.rotate_x(0.0, z1);
            let (x2, z2) = self.rotate_y(x2, z2);
            let (y2, z2) = self.rotate_x(0.0, z2);

            let perspective = 3.0;
            let f1 = perspective / (perspective + 1.0);
            let f2 = perspective / (perspective + z2 + 1.0);

            painter.line_segment(
                [
                    pos2(center.x + x1 * scale * f1, center.y - y1 * scale * f1),
                    pos2(center.x + x2 * scale * f2, center.y - y2 * scale * f2),
                ],
                grid_stroke,
            );
        }
    }

    fn draw_axes(&self, painter: &egui::Painter, center: Pos2, scale: f32, theme: &Theme) {
        let axis_len = 1.2_f32;
        let perspective = 3.0;

        // X axis (red) - C0
        let (x, z) = self.rotate_y(axis_len, 0.0);
        let (y, z) = self.rotate_x(0.0, z);
        let f = perspective / (perspective + z + 1.0);
        painter.line_segment(
            [
                center,
                pos2(center.x + x * scale * f, center.y - y * scale * f),
            ],
            Stroke::new(2.0, Color32::from_rgb(200, 80, 80)),
        );

        // Y axis (green) - Up (nadir direction)
        let (x, z) = self.rotate_y(0.0, 0.0);
        let (y, z) = self.rotate_x(axis_len, z);
        let f = perspective / (perspective + z + 1.0);
        painter.line_segment(
            [
                center,
                pos2(center.x + x * scale * f, center.y - y * scale * f),
            ],
            Stroke::new(2.0, Color32::from_rgb(80, 200, 80)),
        );

        // Z axis (blue) - C90
        let (x, z) = self.rotate_y(0.0, axis_len);
        let (y, z) = self.rotate_x(0.0, z);
        let f = perspective / (perspective + z + 1.0);
        painter.line_segment(
            [
                center,
                pos2(center.x + x * scale * f, center.y - y * scale * f),
            ],
            Stroke::new(2.0, Color32::from_rgb(80, 80, 200)),
        );
    }
}
