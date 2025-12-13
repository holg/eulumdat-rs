//! Heatmap diagram widget for egui

use crate::Theme;
use egui::{pos2, vec2, Rect, Sense};
use eulumdat::{diagram::HeatmapDiagram, Eulumdat};

/// Heatmap diagram widget
pub struct HeatmapWidget;

impl HeatmapWidget {
    /// Show the heatmap diagram
    pub fn show(ui: &mut egui::Ui, ldt: &Eulumdat, theme: &Theme) {
        let available_size = ui.available_size();
        let width = available_size.x.min(800.0);
        let height = (width * 0.5).min(available_size.y - 80.0);

        let (response, painter) = ui.allocate_painter(vec2(width, height), Sense::hover());
        let rect = response.rect;

        // Margins for axes
        let margin = vec2(60.0, 20.0);
        let legend_width = 60.0;
        let plot_rect = Rect::from_min_max(
            rect.min + margin,
            rect.max - vec2(legend_width + 20.0, 40.0),
        );

        // Background
        painter.rect_filled(rect, 0.0, theme.background);

        // Generate heatmap data
        let heatmap = HeatmapDiagram::from_eulumdat(ldt, width as f64, height as f64);

        let num_c = heatmap.c_angles.len();
        let num_g = heatmap.g_angles.len();

        if heatmap.cells.is_empty() || num_c == 0 || num_g == 0 {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No intensity data",
                egui::FontId::proportional(14.0),
                theme.text,
            );
            return;
        }

        // Draw cells
        let cell_width = plot_rect.width() / num_c as f32;
        let cell_height = plot_rect.height() / num_g as f32;

        for cell in &heatmap.cells {
            let x = plot_rect.left() + cell.c_index as f32 * cell_width;
            let y = plot_rect.top() + cell.g_index as f32 * cell_height;
            let cell_rect = Rect::from_min_size(pos2(x, y), vec2(cell_width, cell_height));

            let color = theme.heatmap_color(cell.normalized);
            painter.rect_filled(cell_rect, 0.0, color);
        }

        // Draw axes
        Self::draw_axes(&painter, plot_rect, &heatmap, theme);

        // Draw color legend
        Self::draw_legend(&painter, rect, &heatmap, theme);

        // Tooltip on hover
        if let Some(hover_pos) = response.hover_pos() {
            if plot_rect.contains(hover_pos) {
                let c_idx = ((hover_pos.x - plot_rect.left()) / cell_width) as usize;
                let g_idx = ((hover_pos.y - plot_rect.top()) / cell_height) as usize;

                if let Some(cell) = heatmap
                    .cells
                    .iter()
                    .find(|c| c.c_index == c_idx && c.g_index == g_idx)
                {
                    let tooltip_text = format!(
                        "C: {:.0}°, γ: {:.0}°\nIntensity: {:.1} cd/klm\nCandela: {:.1} cd",
                        cell.c_angle, cell.g_angle, cell.intensity, cell.candela
                    );
                    response.clone().on_hover_text(tooltip_text);
                }
            }
        }
    }

    fn draw_axes(
        painter: &egui::Painter,
        rect: Rect,
        heatmap: &HeatmapDiagram,
        theme: &Theme,
    ) {
        // X axis label (C-planes)
        painter.text(
            pos2(rect.center().x, rect.bottom() + 25.0),
            egui::Align2::CENTER_TOP,
            "C-plane (°)",
            egui::FontId::proportional(11.0),
            theme.text,
        );

        // Y axis label (Gamma)
        painter.text(
            pos2(rect.left() - 40.0, rect.center().y),
            egui::Align2::CENTER_CENTER,
            "γ",
            egui::FontId::proportional(14.0),
            theme.text,
        );

        // X axis tick labels
        let c_ticks = [0.0, 90.0, 180.0, 270.0, 360.0];
        for &c in &c_ticks {
            let x = rect.left() + (c as f32 / 360.0) * rect.width();
            if x <= rect.right() {
                painter.text(
                    pos2(x, rect.bottom() + 5.0),
                    egui::Align2::CENTER_TOP,
                    format!("{:.0}", c),
                    egui::FontId::proportional(9.0),
                    theme.text,
                );
            }
        }

        // Y axis tick labels
        let g_ticks = [0.0, 45.0, 90.0, 135.0, 180.0];
        for &g in &g_ticks {
            let y = rect.top() + (g as f32 / 180.0) * rect.height();
            if y <= rect.bottom() {
                painter.text(
                    pos2(rect.left() - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.0}", g),
                    egui::FontId::proportional(9.0),
                    theme.text,
                );
            }
        }
    }

    fn draw_legend(
        painter: &egui::Painter,
        rect: Rect,
        heatmap: &HeatmapDiagram,
        theme: &Theme,
    ) {
        let legend_x = rect.right() - 50.0;
        let legend_top = rect.top() + 20.0;
        let legend_height = rect.height() - 60.0;
        let legend_width = 20.0;

        // Draw gradient bar
        let num_steps = 50;
        let step_height = legend_height / num_steps as f32;

        for i in 0..num_steps {
            let normalized = 1.0 - (i as f64 / num_steps as f64);
            let color = theme.heatmap_color(normalized);
            let y = legend_top + i as f32 * step_height;
            let step_rect =
                Rect::from_min_size(pos2(legend_x, y), vec2(legend_width, step_height + 1.0));
            painter.rect_filled(step_rect, 0.0, color);
        }

        // Border
        painter.rect_stroke(
            Rect::from_min_size(pos2(legend_x, legend_top), vec2(legend_width, legend_height)),
            0.0,
            egui::Stroke::new(1.0, theme.axis),
        );

        // Labels
        painter.text(
            pos2(legend_x + legend_width + 5.0, legend_top),
            egui::Align2::LEFT_CENTER,
            format!("{:.0}", heatmap.scale.max_intensity),
            egui::FontId::proportional(9.0),
            theme.text,
        );

        painter.text(
            pos2(legend_x + legend_width + 5.0, legend_top + legend_height),
            egui::Align2::LEFT_CENTER,
            "0",
            egui::FontId::proportional(9.0),
            theme.text,
        );

        painter.text(
            pos2(
                legend_x + legend_width / 2.0,
                legend_top + legend_height + 15.0,
            ),
            egui::Align2::CENTER_TOP,
            "cd/klm",
            egui::FontId::proportional(9.0),
            theme.text,
        );
    }
}
