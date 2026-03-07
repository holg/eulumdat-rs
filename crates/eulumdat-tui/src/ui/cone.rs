use eulumdat::diagram::ConeDiagram;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, Borders, Widget,
    },
};

pub fn render_cone(
    area: Rect,
    buf: &mut Buffer,
    cone: &ConeDiagram,
    zoom: f64,
    pan: (f64, f64),
    focused: bool,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let h = cone.mounting_height;
    let field_r = cone.field_diameter / 2.0;

    // Canvas bounds: luminaire at top, floor at bottom
    let x_extent = field_r.max(h * 0.5) * 1.3;
    let y_extent = h * 1.2;

    let x_range = x_extent * 2.0 / zoom;
    let y_range = y_extent / zoom;
    let x_center = pan.0;
    let y_center = h / 2.0 + pan.1;

    let x_min = x_center - x_range / 2.0;
    let x_max = x_center + x_range / 2.0;
    let y_min = y_center - y_range / 2.0;
    let y_max = y_center + y_range / 2.0;

    let beam_r = cone.beam_diameter / 2.0;
    let beam_angle = cone.half_beam_angle;
    let field_angle = cone.half_field_angle;
    let beam_diam = cone.beam_diameter;
    let field_diam = cone.field_diameter;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Cone Diagram "),
        )
        .marker(Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            let beam_color = Color::Rgb(80, 140, 255);
            let field_color = Color::Rgb(60, 80, 120);
            let floor_color = Color::Rgb(100, 100, 100);
            let luminaire_color = Color::Rgb(255, 200, 50);

            // Floor line
            ctx.draw(&CanvasLine {
                x1: x_min,
                y1: 0.0,
                x2: x_max,
                y2: 0.0,
                color: floor_color,
            });

            // Luminaire point at top
            let lum_half = h * 0.02;
            ctx.draw(&CanvasLine {
                x1: -lum_half,
                y1: h,
                x2: lum_half,
                y2: h,
                color: luminaire_color,
            });

            // Beam cone lines
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: h,
                x2: -beam_r,
                y2: 0.0,
                color: beam_color,
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: h,
                x2: beam_r,
                y2: 0.0,
                color: beam_color,
            });

            // Field cone lines
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: h,
                x2: -field_r,
                y2: 0.0,
                color: field_color,
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: h,
                x2: field_r,
                y2: 0.0,
                color: field_color,
            });

            // Beam diameter line on floor
            ctx.draw(&CanvasLine {
                x1: -beam_r,
                y1: 0.0,
                x2: beam_r,
                y2: 0.0,
                color: beam_color,
            });

            // Field diameter line on floor
            ctx.draw(&CanvasLine {
                x1: -field_r,
                y1: -h * 0.02,
                x2: field_r,
                y2: -h * 0.02,
                color: field_color,
            });

            // Annotations
            ctx.print(
                0.0,
                h * 1.05,
                ratatui::text::Line::styled(
                    format!("h = {:.1} m", h),
                    Style::default().fg(Color::Gray),
                ),
            );
            ctx.print(
                beam_r * 0.5,
                -h * 0.06,
                ratatui::text::Line::styled(
                    format!(
                        "Beam: {:.1}\u{00b0} / \u{00d8}{:.1}m",
                        beam_angle * 2.0,
                        beam_diam
                    ),
                    Style::default().fg(beam_color),
                ),
            );
            ctx.print(
                field_r * 0.5,
                -h * 0.12,
                ratatui::text::Line::styled(
                    format!(
                        "Field: {:.1}\u{00b0} / \u{00d8}{:.1}m",
                        field_angle * 2.0,
                        field_diam
                    ),
                    Style::default().fg(field_color),
                ),
            );
        });

    canvas.render(area, buf);
}
