//! Cartesian diagram widget for egui

use crate::Theme;
use egui::{pos2, vec2, Pos2, Rect, Sense, Stroke, Vec2};
use eulumdat::{diagram::CartesianDiagram, Eulumdat};

/// Cartesian diagram widget
pub struct CartesianWidget;

impl CartesianWidget {
    /// Show the cartesian diagram
    pub fn show(ui: &mut egui::Ui, ldt: &Eulumdat, theme: &Theme) {
        let available_size = ui.available_size();
        let width = available_size.x.min(800.0);
        let height = (width * 0.6).min(available_size.y - 50.0);

        let (response, painter) = ui.allocate_painter(vec2(width, height), Sense::hover());
        let rect = response.rect;

        // Margins for axes
        let margin = vec2(60.0, 40.0);
        let plot_rect = Rect::from_min_max(
            rect.min + margin,
            rect.max - vec2(20.0, 30.0),
        );

        // Background
        painter.rect_filled(rect, 0.0, theme.background);

        // Generate diagram data (max 8 curves)
        let cartesian = CartesianDiagram::from_eulumdat(ldt, width as f64, height as f64, 8);

        // Draw grid
        Self::draw_grid(&painter, plot_rect, &cartesian, theme);

        // Draw curves
        for (i, curve) in cartesian.curves.iter().enumerate() {
            let color = theme.c_plane_color(curve.c_angle, cartesian.curves.len());
            Self::draw_curve(&painter, plot_rect, curve, &cartesian, color);
        }

        // Draw axes labels
        Self::draw_axes(&painter, plot_rect, &cartesian, theme);

        // Legend
        Self::draw_legend(ui, &cartesian, theme);
    }

    fn draw_grid(
        painter: &egui::Painter,
        rect: Rect,
        cartesian: &CartesianDiagram,
        theme: &Theme,
    ) {
        let grid_stroke = Stroke::new(1.0, theme.grid);

        // Vertical grid lines (gamma angles)
        for gamma in (0..=180).step_by(30) {
            let x = rect.left() + (gamma as f32 / 180.0) * rect.width();
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                grid_stroke,
            );
        }

        // Horizontal grid lines (intensity)
        let num_lines = 5;
        for i in 0..=num_lines {
            let y = rect.bottom() - (i as f32 / num_lines as f32) * rect.height();
            painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                grid_stroke,
            );
        }
    }

    fn draw_curve(
        painter: &egui::Painter,
        rect: Rect,
        curve: &eulumdat::diagram::CartesianCurve,
        cartesian: &CartesianDiagram,
        color: egui::Color32,
    ) {
        if curve.points.is_empty() {
            return;
        }

        let stroke = Stroke::new(2.0, color);
        let max_intensity = cartesian.scale.scale_max;

        let screen_points: Vec<Pos2> = curve
            .points
            .iter()
            .map(|p| {
                let x = rect.left() + (p.gamma as f32 / 180.0) * rect.width();
                let y = rect.bottom() - (p.intensity as f32 / max_intensity as f32) * rect.height();
                pos2(x, y.max(rect.top()))
            })
            .collect();

        for pair in screen_points.windows(2) {
            painter.line_segment([pair[0], pair[1]], stroke);
        }
    }

    fn draw_axes(
        painter: &egui::Painter,
        rect: Rect,
        cartesian: &CartesianDiagram,
        theme: &Theme,
    ) {
        let axis_stroke = Stroke::new(1.5, theme.axis);

        // X axis
        painter.line_segment(
            [pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom())],
            axis_stroke,
        );

        // Y axis
        painter.line_segment(
            [pos2(rect.left(), rect.top()), pos2(rect.left(), rect.bottom())],
            axis_stroke,
        );

        // X axis labels (gamma)
        for gamma in (0..=180).step_by(30) {
            let x = rect.left() + (gamma as f32 / 180.0) * rect.width();
            painter.text(
                pos2(x, rect.bottom() + 15.0),
                egui::Align2::CENTER_TOP,
                format!("{}°", gamma),
                egui::FontId::proportional(10.0),
                theme.text,
            );
        }

        // Y axis labels (intensity)
        let num_labels = 5;
        for i in 0..=num_labels {
            let value = cartesian.scale.scale_max * (i as f64 / num_labels as f64);
            let y = rect.bottom() - (i as f32 / num_labels as f32) * rect.height();
            painter.text(
                pos2(rect.left() - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{:.0}", value),
                egui::FontId::proportional(10.0),
                theme.text,
            );
        }

        // Axis titles
        painter.text(
            pos2(rect.center().x, rect.bottom() + 30.0),
            egui::Align2::CENTER_TOP,
            "Gamma (°)",
            egui::FontId::proportional(12.0),
            theme.text,
        );
    }

    fn draw_legend(ui: &mut egui::Ui, cartesian: &CartesianDiagram, theme: &Theme) {
        ui.horizontal_wrapped(|ui| {
            for curve in &cartesian.curves {
                let color = theme.c_plane_color(curve.c_angle, cartesian.curves.len());
                let (rect, _) = ui.allocate_exact_size(vec2(15.0, 3.0), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, color);
                ui.label(format!("C{:.0}°", curve.c_angle));
                ui.add_space(10.0);
            }
        });
    }
}
