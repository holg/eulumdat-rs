use eulumdat::{diagram::PolarDiagram, BeamFieldAnalysis};
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

pub fn render_polar(
    area: Rect,
    buf: &mut Buffer,
    polar: &PolarDiagram,
    beam_field: &BeamFieldAnalysis,
    zoom: f64,
    pan: (f64, f64),
    focused: bool,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let scale_max = polar.scale.scale_max;
    let bound = scale_max * 1.15 / zoom;
    let x_min = -bound + pan.0;
    let x_max = bound + pan.0;
    let y_min = -bound + pan.1;
    let y_max = bound + pan.1;

    let grid_values = polar.scale.grid_values.clone();
    let show_c90 = polar.show_c90_c270();

    let c0_label = polar.c0_c180_curve.label.clone();
    let c90_label = polar.c90_c270_curve.label.clone();

    // The PolarPoint coordinates use:
    //   angle_rad = -gamma + PI/2
    //   x = intensity * cos(angle_rad)
    //   y = intensity * sin(angle_rad)
    //
    // This means: gamma=0 (nadir) → y=+intensity (top in canvas Y-up)
    //             gamma=90 → x=+intensity (right)
    //             gamma=180 (zenith) → y=-intensity (bottom)
    //
    // In the SVG, Y is inverted so nadir appears at bottom.
    // In the TUI Canvas (Y-up), we need to NEGATE Y to match the SVG convention:
    //   nadir (0°) at bottom, zenith (180°) at top.
    let c0_pts: Vec<(f64, f64)> = polar
        .c0_c180_curve
        .points
        .iter()
        .map(|p| (p.x, -p.y))
        .collect();
    let c90_pts: Vec<(f64, f64)> = polar
        .c90_c270_curve
        .points
        .iter()
        .map(|p| (p.x, -p.y))
        .collect();

    // Beam/field angle data for arc drawing
    let half_beam = beam_field.beam_angle_ies / 2.0;
    let half_field = beam_field.field_angle_ies / 2.0;
    let beam_threshold = beam_field.beam_threshold_ies;
    let field_threshold = beam_field.field_threshold_ies;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Polar Diagram "),
        )
        .marker(Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            let grid_color = Color::Rgb(40, 40, 55);
            let axis_color = Color::Rgb(60, 60, 80);

            // Grid circles (semi-circles in lower half only, matching SVG)
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

            // Radial lines at 30° increments (matching SVG)
            for i in 0..=6 {
                let angle_deg = i as f64 * 30.0;
                let angle_rad = angle_deg.to_radians();
                // Lines extend from center outward
                // In SVG coords: x = radius * sin(angle), y = radius * cos(angle)
                // But we negated Y, so: y = -radius * cos(angle)
                let r = scale_max;
                let dx = r * angle_rad.sin();
                let dy = -r * angle_rad.cos();
                // Draw both sides (left and right)
                if i == 3 {
                    // 90° horizontal line - draw as axis (thicker)
                    ctx.draw(&CanvasLine {
                        x1: -r,
                        y1: 0.0,
                        x2: r,
                        y2: 0.0,
                        color: axis_color,
                    });
                } else {
                    ctx.draw(&CanvasLine {
                        x1: -dx,
                        y1: dy,
                        x2: dx,
                        y2: dy,
                        color: grid_color,
                    });
                }
            }

            // Vertical axis
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -scale_max,
                x2: 0.0,
                y2: scale_max,
                color: axis_color,
            });

            // Angle labels: matching SVG convention
            // 0° at bottom (nadir), 90° at sides, 180° at top (zenith)
            let label_r = scale_max * 1.08;
            // 0° (nadir) at bottom
            ctx.print(
                0.0,
                -label_r,
                ratatui::text::Line::styled("0\u{00b0}", Style::default().fg(Color::Gray)),
            );
            // 90° at right
            ctx.print(
                label_r,
                0.0,
                ratatui::text::Line::styled("90\u{00b0}", Style::default().fg(Color::Gray)),
            );
            // 180° (zenith) at top
            ctx.print(
                0.0,
                label_r,
                ratatui::text::Line::styled("180\u{00b0}", Style::default().fg(Color::Gray)),
            );
            // 90° at left (same angle, other side)
            ctx.print(
                -label_r,
                0.0,
                ratatui::text::Line::styled("90\u{00b0}", Style::default().fg(Color::Gray)),
            );

            // Intermediate angle labels (30°, 60°, 120°, 150°)
            for &deg in &[30.0_f64, 60.0, 120.0, 150.0] {
                let angle_rad = deg.to_radians();
                let lx = label_r * angle_rad.sin();
                let ly = -label_r * angle_rad.cos();
                // Right side
                ctx.print(
                    lx,
                    ly,
                    ratatui::text::Line::styled(
                        format!("{:.0}\u{00b0}", deg),
                        Style::default().fg(Color::DarkGray),
                    ),
                );
                // Left side
                ctx.print(
                    -lx,
                    ly,
                    ratatui::text::Line::styled(
                        format!("{:.0}\u{00b0}", deg),
                        Style::default().fg(Color::DarkGray),
                    ),
                );
            }

            // Beam angle arc (green, solid) - symmetric around nadir (bottom)
            let beam_color = Color::Rgb(34, 197, 94); // #22c55e
            if half_beam > 0.0 && half_beam <= 90.0 {
                let arc_r = scale_max * 0.85;
                draw_angle_arc(ctx, half_beam, arc_r, beam_color, false);

                // Radial lines from center to arc endpoints
                let angle_rad = half_beam.to_radians();
                let ax = arc_r * angle_rad.sin();
                let ay = -arc_r * angle_rad.cos();
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: ax,
                    y2: ay,
                    color: beam_color,
                });
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: -ax,
                    y2: ay,
                    color: beam_color,
                });

                // Label below arc
                ctx.print(
                    0.0,
                    ay - scale_max * 0.05,
                    ratatui::text::Line::styled(
                        format!("Beam {:.0}\u{00b0}", half_beam * 2.0),
                        Style::default().fg(beam_color),
                    ),
                );
            }

            // Field angle arc (orange, dashed approximation)
            let field_color = Color::Rgb(249, 115, 22); // #f97316
            if half_field > 0.0 && half_field <= 90.0 {
                let arc_r = scale_max * 0.85 - scale_max * 0.12;
                draw_angle_arc(ctx, half_field, arc_r, field_color, true);

                let angle_rad = half_field.to_radians();
                let ax = arc_r * angle_rad.sin();
                let ay = -arc_r * angle_rad.cos();
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: ax,
                    y2: ay,
                    color: field_color,
                });
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: -ax,
                    y2: ay,
                    color: field_color,
                });

                ctx.print(
                    0.0,
                    ay - scale_max * 0.05,
                    ratatui::text::Line::styled(
                        format!("Field {:.0}\u{00b0}", half_field * 2.0),
                        Style::default().fg(field_color),
                    ),
                );
            }

            // 50% threshold circle (beam, green dashed)
            if beam_threshold > 0.0 && beam_threshold < scale_max {
                let segs = 48;
                for i in 0..segs {
                    // Draw every other segment for dashed effect
                    if i % 2 == 1 {
                        continue;
                    }
                    let a1 = 2.0 * std::f64::consts::PI * i as f64 / segs as f64;
                    let a2 = 2.0 * std::f64::consts::PI * (i + 1) as f64 / segs as f64;
                    ctx.draw(&CanvasLine {
                        x1: beam_threshold * a1.cos(),
                        y1: beam_threshold * a1.sin(),
                        x2: beam_threshold * a2.cos(),
                        y2: beam_threshold * a2.sin(),
                        color: beam_color,
                    });
                }
                ctx.print(
                    beam_threshold + scale_max * 0.02,
                    scale_max * 0.02,
                    ratatui::text::Line::styled("50%", Style::default().fg(beam_color)),
                );
            }

            // 10% threshold circle (field, orange dashed)
            if field_threshold > 0.0 && field_threshold < scale_max {
                let segs = 48;
                for i in 0..segs {
                    if i % 2 == 1 {
                        continue;
                    }
                    let a1 = 2.0 * std::f64::consts::PI * i as f64 / segs as f64;
                    let a2 = 2.0 * std::f64::consts::PI * (i + 1) as f64 / segs as f64;
                    ctx.draw(&CanvasLine {
                        x1: field_threshold * a1.cos(),
                        y1: field_threshold * a1.sin(),
                        x2: field_threshold * a2.cos(),
                        y2: field_threshold * a2.sin(),
                        color: field_color,
                    });
                }
                ctx.print(
                    field_threshold + scale_max * 0.02,
                    -scale_max * 0.02,
                    ratatui::text::Line::styled("10%", Style::default().fg(field_color)),
                );
            }

            // C0-C180 curve (blue)
            let c0_color = Color::Rgb(80, 140, 255);
            for w in c0_pts.windows(2) {
                ctx.draw(&CanvasLine {
                    x1: w[0].0,
                    y1: w[0].1,
                    x2: w[1].0,
                    y2: w[1].1,
                    color: c0_color,
                });
            }

            // C90-C270 curve (red)
            if show_c90 && !c90_pts.is_empty() {
                let c90_color = Color::Rgb(255, 100, 100);
                for w in c90_pts.windows(2) {
                    ctx.draw(&CanvasLine {
                        x1: w[0].0,
                        y1: w[0].1,
                        x2: w[1].0,
                        y2: w[1].1,
                        color: c90_color,
                    });
                }
            }

            // Scale labels on grid circles (positioned right of center, below each circle)
            for &r in &grid_values {
                if r > 0.0 {
                    ctx.print(
                        scale_max * 0.03,
                        -r - scale_max * 0.02,
                        ratatui::text::Line::styled(
                            format!("{:.0}", r),
                            Style::default().fg(Color::DarkGray),
                        ),
                    );
                }
            }

            // Legend in bottom-left corner
            let lx = x_min + (x_max - x_min) * 0.02;
            let ly_base = y_min + (y_max - y_min) * 0.15;

            ctx.print(
                lx,
                ly_base,
                ratatui::text::Line::styled(
                    format!("\u{2500} {}", c0_label),
                    Style::default().fg(c0_color),
                ),
            );
            if show_c90 {
                ctx.print(
                    lx,
                    ly_base - (y_max - y_min) * 0.04,
                    ratatui::text::Line::styled(
                        format!("- - {}", c90_label),
                        Style::default().fg(Color::Rgb(255, 100, 100)),
                    ),
                );
            }
            ctx.print(
                lx,
                ly_base - (y_max - y_min) * 0.08,
                ratatui::text::Line::styled("\u{2500} Beam (50%)", Style::default().fg(beam_color)),
            );
            ctx.print(
                lx,
                ly_base - (y_max - y_min) * 0.12,
                ratatui::text::Line::styled("- - Field (10%)", Style::default().fg(field_color)),
            );
        });

    canvas.render(area, buf);
}

/// Draw an arc symmetric around nadir (bottom of diagram, y negative direction).
/// half_angle: half-angle in degrees from nadir
/// arc_r: radius of the arc in world coords
/// dashed: if true, skip every other segment
fn draw_angle_arc(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    half_angle: f64,
    arc_r: f64,
    color: Color,
    dashed: bool,
) {
    // Arc spans from -half_angle to +half_angle around nadir (270° in math coords)
    // In our coord system (Y negated): nadir is at angle 270° = -PI/2
    // Left endpoint: gamma = half_angle → SVG: sin(half_angle), -cos(half_angle)
    // Right endpoint: gamma = half_angle → SVG: -sin(half_angle), -cos(half_angle)

    let segments = 24;
    let start_angle = -half_angle; // degrees from nadir, going left
    let end_angle = half_angle; // degrees from nadir, going right

    for i in 0..segments {
        if dashed && i % 2 == 1 {
            continue;
        }
        let t1 = start_angle + (end_angle - start_angle) * i as f64 / segments as f64;
        let t2 = start_angle + (end_angle - start_angle) * (i + 1) as f64 / segments as f64;

        // Convert gamma-from-nadir to canvas coords
        // gamma from nadir: x = r * sin(gamma), y = -r * cos(gamma)
        let x1 = arc_r * t1.to_radians().sin();
        let y1 = -arc_r * t1.to_radians().cos();
        let x2 = arc_r * t2.to_radians().sin();
        let y2 = -arc_r * t2.to_radians().cos();

        ctx.draw(&CanvasLine {
            x1,
            y1,
            x2,
            y2,
            color,
        });
    }
}
