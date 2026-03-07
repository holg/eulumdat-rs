use eulumdat::diagram::CartesianDiagram;
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

const CURVE_COLORS: [Color; 8] = [
    Color::Rgb(80, 140, 255),  // blue
    Color::Rgb(255, 100, 100), // red
    Color::Rgb(100, 255, 100), // green
    Color::Rgb(255, 200, 50),  // yellow
    Color::Rgb(200, 100, 255), // purple
    Color::Rgb(100, 255, 255), // cyan
    Color::Rgb(255, 150, 80),  // orange
    Color::Rgb(200, 200, 200), // gray
];

pub fn render_cartesian(
    area: Rect,
    buf: &mut Buffer,
    cartesian: &CartesianDiagram,
    zoom: f64,
    pan: (f64, f64),
    focused: bool,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let max_gamma = cartesian.max_gamma;
    let scale_max = cartesian.scale.scale_max;

    let x_range = max_gamma / zoom;
    let y_range = scale_max / zoom;
    let x_center = max_gamma / 2.0 + pan.0;
    let y_center = scale_max / 2.0 + pan.1;

    let x_min = x_center - x_range / 2.0;
    let x_max = x_center + x_range / 2.0;
    let y_min = y_center - y_range / 2.0;
    let y_max = y_center + y_range / 2.0;

    let x_ticks = cartesian.x_ticks.clone();
    let y_ticks = cartesian.y_ticks.clone();

    // Collect curve data for the closure
    #[allow(clippy::type_complexity)]
    let curves_data: Vec<(Vec<(f64, f64)>, String, Color)> = cartesian
        .curves
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let pts: Vec<(f64, f64)> = c.points.iter().map(|p| (p.gamma, p.intensity)).collect();
            let color = CURVE_COLORS[i % CURVE_COLORS.len()];
            (pts, c.label.clone(), color)
        })
        .collect();

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Cartesian Diagram "),
        )
        .marker(Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            let grid_color = Color::Rgb(40, 40, 55);
            let axis_color = Color::Rgb(60, 60, 80);

            // Grid lines
            for &x in &x_ticks {
                ctx.draw(&CanvasLine {
                    x1: x,
                    y1: 0.0,
                    x2: x,
                    y2: scale_max,
                    color: grid_color,
                });
            }
            for &y in &y_ticks {
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: y,
                    x2: max_gamma,
                    y2: y,
                    color: grid_color,
                });
            }

            // Axes
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: max_gamma,
                y2: 0.0,
                color: axis_color,
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: scale_max,
                color: axis_color,
            });

            // Curves
            for (pts, _label, color) in &curves_data {
                for w in pts.windows(2) {
                    ctx.draw(&CanvasLine {
                        x1: w[0].0,
                        y1: w[0].1,
                        x2: w[1].0,
                        y2: w[1].1,
                        color: *color,
                    });
                }
            }

            // X-axis labels
            for &x in &x_ticks {
                ctx.print(
                    x,
                    y_min + (y_max - y_min) * 0.02,
                    ratatui::text::Line::styled(
                        format!("{:.0}\u{00b0}", x),
                        Style::default().fg(Color::DarkGray),
                    ),
                );
            }

            // Y-axis labels
            for &y in &y_ticks {
                if y > 0.0 {
                    ctx.print(
                        x_min + (x_max - x_min) * 0.01,
                        y,
                        ratatui::text::Line::styled(
                            format!("{:.0}", y),
                            Style::default().fg(Color::DarkGray),
                        ),
                    );
                }
            }

            // Legend
            let legend_x = x_min + (x_max - x_min) * 0.02;
            for (i, (_pts, label, color)) in curves_data.iter().enumerate() {
                ctx.print(
                    legend_x,
                    y_max - (y_max - y_min) * (0.05 + 0.05 * i as f64),
                    ratatui::text::Line::styled(
                        format!("\u{2500} {}", label),
                        Style::default().fg(*color),
                    ),
                );
            }
        });

    canvas.render(area, buf);
}
