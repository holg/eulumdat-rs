use eulumdat::diagram::ButterflyDiagram;
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

pub fn render_butterfly(
    area: Rect,
    buf: &mut Buffer,
    butterfly: &ButterflyDiagram,
    zoom: f64,
    pan: (f64, f64),
    focused: bool,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let scale_max = butterfly.scale.scale_max;
    let bound = scale_max * 1.3 / zoom;

    let x_min = -bound + pan.0;
    let x_max = bound + pan.0;
    let y_min = -bound + pan.1;
    let y_max = bound + pan.1;

    // Collect wing data
    let wings_data: Vec<(Vec<(f64, f64)>, Color)> = butterfly
        .wings
        .iter()
        .map(|w| {
            let pts: Vec<(f64, f64)> = w.points.iter().map(|p| (p.x, p.y)).collect();
            let color = Color::Rgb(w.stroke_color.r, w.stroke_color.g, w.stroke_color.b);
            (pts, color)
        })
        .collect();

    // Grid circles
    let grid_values = butterfly.scale.grid_values.clone();

    // C-plane lines
    let c_lines: Vec<(f64, f64, f64, f64)> = butterfly
        .c_plane_lines
        .iter()
        .map(|(_, start, end)| (start.x, start.y, end.x, end.y))
        .collect();

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Butterfly Diagram "),
        )
        .marker(Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            let grid_color = Color::Rgb(40, 40, 55);
            let cplane_color = Color::Rgb(50, 50, 65);

            // Grid circles
            let segments = 72;
            for &r in &grid_values {
                if r <= 0.0 {
                    continue;
                }
                for i in 0..segments {
                    let a1 = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
                    let a2 = 2.0 * std::f64::consts::PI * (i + 1) as f64 / segments as f64;
                    ctx.draw(&CanvasLine {
                        x1: r * a1.cos(),
                        y1: r * a1.sin(),
                        x2: r * a2.cos(),
                        y2: r * a2.sin(),
                        color: grid_color,
                    });
                }
            }

            // C-plane lines
            for &(x1, y1, x2, y2) in &c_lines {
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: cplane_color,
                });
            }

            // Wing outlines
            for (pts, color) in &wings_data {
                for w in pts.windows(2) {
                    ctx.draw(&CanvasLine {
                        x1: w[0].0,
                        y1: w[0].1,
                        x2: w[1].0,
                        y2: w[1].1,
                        color: *color,
                    });
                }
                // Close the wing
                if pts.len() > 2 {
                    let first = pts[0];
                    let last = pts[pts.len() - 1];
                    ctx.draw(&CanvasLine {
                        x1: last.0,
                        y1: last.1,
                        x2: first.0,
                        y2: first.1,
                        color: *color,
                    });
                }
            }

            // Scale labels
            for &r in &grid_values {
                if r > 0.0 {
                    ctx.print(
                        r * 0.05,
                        r + scale_max * 0.02,
                        ratatui::text::Line::styled(
                            format!("{:.0}", r),
                            Style::default().fg(Color::DarkGray),
                        ),
                    );
                }
            }
        });

    canvas.render(area, buf);
}
