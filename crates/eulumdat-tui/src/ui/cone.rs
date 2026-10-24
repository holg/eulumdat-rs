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
    // Floor x of the left (C+180) and right (C) edge; (0, 0) when the edge
    // does not reach the floor
    let foot = |edges: Option<(f64, f64)>| {
        edges
            .filter(|(l, r)| l.abs() < 89.0 && r.abs() < 89.0)
            .map(|(l, r)| (h * l.to_radians().tan(), h * r.to_radians().tan()))
            .unwrap_or((0.0, 0.0))
    };
    let (beam_l, beam_r) = foot(cone.beam_edges);
    let (field_l, field_r) = foot(cone.field_edges);

    // Canvas bounds: luminaire at top, floor at bottom
    let x_extent = field_l.abs().max(field_r).max(h * 0.5) * 1.3;
    let y_extent = h * 1.2;

    let x_range = x_extent * 2.0 / zoom;
    let y_range = y_extent / zoom;
    let x_center = pan.0;
    let y_center = h / 2.0 + pan.1;

    let x_min = x_center - x_range / 2.0;
    let x_max = x_center + x_range / 2.0;
    let y_min = y_center - y_range / 2.0;
    let y_max = y_center + y_range / 2.0;

    let beam_angle = eulumdat::format_angle(cone.beam_angle);
    let field_angle = eulumdat::format_angle(cone.field_angle);
    let diam = |d: Option<f64>| d.map_or_else(|| "—".to_string(), |d| format!("{d:.1}m"));
    let beam_diam = diam(cone.beam_diameter);
    let field_diam = diam(cone.field_diameter);
    let section = format!(
        "C{:.0}\u{2013}C{:.0}",
        cone.section_c,
        (cone.section_c + 180.0) % 360.0
    );

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
                x2: beam_l,
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
                x2: field_l,
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
                x1: beam_l,
                y1: 0.0,
                x2: beam_r,
                y2: 0.0,
                color: beam_color,
            });

            // Field diameter line on floor
            ctx.draw(&CanvasLine {
                x1: field_l,
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
                    format!("Beam {}: {} / \u{00d8}{}", section, beam_angle, beam_diam),
                    Style::default().fg(beam_color),
                ),
            );
            ctx.print(
                field_r * 0.5,
                -h * 0.12,
                ratatui::text::Line::styled(
                    format!(
                        "Field {}: {} / \u{00d8}{}",
                        section, field_angle, field_diam
                    ),
                    Style::default().fg(field_color),
                ),
            );
        });

    canvas.render(area, buf);
}
