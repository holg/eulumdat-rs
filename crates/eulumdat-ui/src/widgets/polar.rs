//! Polar diagram widget for egui

use crate::Theme;
use egui::{pos2, vec2, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use eulumdat::{diagram::PolarDiagram, Eulumdat};

/// Polar diagram widget
pub struct PolarWidget;

impl PolarWidget {
    /// Show the polar diagram
    pub fn show(ui: &mut egui::Ui, ldt: &Eulumdat, theme: &Theme) {
        let available_size = ui.available_size();
        let size = available_size.min_elem().min(600.0);
        let (response, painter) =
            ui.allocate_painter(Vec2::splat(size), Sense::hover());

        let rect = response.rect;
        let center = rect.center();
        let radius = (size / 2.0) * 0.85;

        // Background
        painter.rect_filled(rect, 0.0, theme.background);

        // Generate diagram data
        let polar = PolarDiagram::from_eulumdat(ldt);

        // Draw grid circles
        Self::draw_grid(&painter, center, radius, &polar, theme);

        // Draw angle labels
        Self::draw_angle_labels(&painter, center, radius, theme);

        // Draw intensity curves
        Self::draw_curve(
            &painter,
            center,
            radius,
            &polar.c0_c180_curve.points,
            polar.scale.scale_max,
            theme.primary_curve,
        );

        if polar.show_c90_c270() {
            Self::draw_curve(
                &painter,
                center,
                radius,
                &polar.c90_c270_curve.points,
                polar.scale.scale_max,
                theme.secondary_curve,
            );
        }

        // Legend
        Self::draw_legend(ui, &polar, theme);
    }

    fn draw_grid(
        painter: &egui::Painter,
        center: Pos2,
        radius: f32,
        polar: &PolarDiagram,
        theme: &Theme,
    ) {
        let grid_stroke = Stroke::new(1.0, theme.grid);
        let axis_stroke = Stroke::new(1.5, theme.axis);

        // Concentric circles for intensity scale
        for (i, &value) in polar.scale.grid_values.iter().enumerate() {
            let r = radius * (value / polar.scale.scale_max) as f32;
            painter.circle_stroke(center, r, grid_stroke);

            // Label on right side
            let label_pos = center + vec2(r + 5.0, 0.0);
            painter.text(
                label_pos,
                egui::Align2::LEFT_CENTER,
                format!("{:.0}", value),
                egui::FontId::proportional(10.0),
                theme.text,
            );
        }

        // Radial lines for angles (every 30°)
        for angle_deg in (0..360).step_by(30) {
            let angle_rad = (angle_deg as f32 - 90.0).to_radians();
            let outer = center + radius * vec2(angle_rad.cos(), angle_rad.sin());
            painter.line_segment([center, outer], grid_stroke);
        }

        // Horizontal and vertical axes (stronger)
        painter.line_segment(
            [center - vec2(radius, 0.0), center + vec2(radius, 0.0)],
            axis_stroke,
        );
        painter.line_segment(
            [center - vec2(0.0, radius), center + vec2(0.0, radius)],
            axis_stroke,
        );
    }

    fn draw_angle_labels(painter: &egui::Painter, center: Pos2, radius: f32, theme: &Theme) {
        let labels = [
            (0, "0°", egui::Align2::CENTER_BOTTOM),
            (90, "90°", egui::Align2::LEFT_CENTER),
            (180, "180°", egui::Align2::CENTER_TOP),
            (270, "90°", egui::Align2::RIGHT_CENTER),
        ];

        for (angle_deg, label, align) in labels {
            let angle_rad = (angle_deg as f32 - 90.0).to_radians();
            let pos = center + (radius + 15.0) * vec2(angle_rad.cos(), angle_rad.sin());
            painter.text(
                pos,
                align,
                label,
                egui::FontId::proportional(12.0),
                theme.text,
            );
        }
    }

    fn draw_curve(
        painter: &egui::Painter,
        center: Pos2,
        radius: f32,
        points: &[eulumdat::diagram::PolarPoint],
        scale_max: f64,
        color: Color32,
    ) {
        if points.is_empty() {
            return;
        }

        let stroke = Stroke::new(2.0, color);
        let screen_points: Vec<Pos2> = points
            .iter()
            .map(|p| {
                // Convert from diagram coordinates to screen coordinates
                // In polar diagram: gamma 0° = down (nadir), gamma 180° = up (zenith)
                let r = (p.intensity / scale_max) as f32 * radius;
                let angle_rad = (p.gamma as f32 - 90.0).to_radians();
                center + r * vec2(angle_rad.cos(), angle_rad.sin())
            })
            .collect();

        // Draw as connected line segments
        for pair in screen_points.windows(2) {
            painter.line_segment([pair[0], pair[1]], stroke);
        }
    }

    fn draw_legend(ui: &mut egui::Ui, polar: &PolarDiagram, theme: &Theme) {
        ui.horizontal(|ui| {
            // C0-C180 legend
            let (rect, _) = ui.allocate_exact_size(vec2(20.0, 3.0), Sense::hover());
            ui.painter().rect_filled(rect, 0.0, theme.primary_curve);
            ui.label("C0-C180");

            if polar.show_c90_c270() {
                ui.add_space(20.0);
                let (rect, _) = ui.allocate_exact_size(vec2(20.0, 3.0), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, theme.secondary_curve);
                ui.label("C90-C270");
            }

            ui.add_space(20.0);
            ui.label(format!("Max: {:.0} cd/klm", polar.scale.max_intensity));
        });
    }
}
