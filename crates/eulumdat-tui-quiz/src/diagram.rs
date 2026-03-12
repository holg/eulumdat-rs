use eulumdat::diagram::{heatmap_color, HeatmapDiagram, PolarDiagram};
use eulumdat::Eulumdat;
use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::Category;
use ratatui::prelude::*;
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::*;

const FLUORESCENT_LDT: &str = include_str!("../templates/fluorescent_luminaire.ldt");
const ROAD_LDT: &str = include_str!("../templates/road_luminaire.ldt");
const PROJECTOR_LDT: &str = include_str!("../templates/projector.ldt");

pub struct TemplateLuminaires {
    pub fluorescent: Eulumdat,
    pub road: Eulumdat,
    pub projector: Eulumdat,
}

impl TemplateLuminaires {
    pub fn load() -> Self {
        Self {
            fluorescent: Eulumdat::parse(FLUORESCENT_LDT).expect("fluorescent template"),
            road: Eulumdat::parse(ROAD_LDT).expect("road template"),
            projector: Eulumdat::parse(PROJECTOR_LDT).expect("projector template"),
        }
    }

    /// Returns the diagram spec for a given category (used for non-DiagramReading categories).
    /// Only show diagrams when the questions actually reference or require them.
    pub fn diagram_for_category(&self, category: &Category) -> Option<DiagramSpec> {
        match category {
            Category::CoordinateSystems => Some(self.fluorescent_diagram()),
            Category::Symmetry => Some(self.symmetry_pair()),
            Category::DiagramReading => {
                // DiagramReading uses per-question diagrams; this is the fallback
                Some(self.fluorescent_diagram())
            }
            // DiagramTypes, PhotometricCalc, etc. — questions are theoretical/formula-based,
            // showing a specific diagram would be misleading
            _ => None,
        }
    }

    /// Returns the diagram spec matched to a specific DiagramReading question.
    /// Only show diagrams that are directly relevant and consistent with the question text.
    pub fn diagram_for_question(&self, question_id: u32) -> Option<DiagramSpec> {
        match question_id {
            // Fluorescent polar: curve colors, nadir peak, grid circles, overlapping curves, 90° axis
            17001 | 17002 | 17003 | 17004 | 17010 => Some(self.fluorescent_diagram()),
            // No diagram — question asks about a hypothetical single-curve (Isym=1) luminaire
            // Showing fluorescent (which has both curves) would contradict the question
            17005 => None,
            // Symmetric vs asymmetric side-by-side comparison
            17006 | 17012 => Some(self.symmetry_pair()),
            // Road luminaire (asymmetric throw)
            17007 => Some(self.road_diagram()),
            // Projector (narrow beam spike)
            17008 | 17011 => Some(self.projector_diagram()),
            // Grid reading — fluorescent polar (curve crosses grid circles)
            17009 => Some(self.fluorescent_diagram()),
            // Heatmap questions — fluorescent heatmap
            17013..=17020 => Some(self.fluorescent_heatmap()),
            // 17016: question describes a hypothetical symmetric heatmap pattern —
            // the fluorescent heatmap is BothPlanes symmetric, which is close enough
            // 17017: describes "sharp transition" (narrow beam) — not our fluorescent,
            // but the question is theoretical about what a pattern means
            // Unknown DiagramReading question — no diagram to avoid mismatches
            _ => None,
        }
    }

    fn fluorescent_diagram(&self) -> DiagramSpec {
        DiagramSpec::Single {
            polar: PolarDiagram::from_eulumdat(&self.fluorescent),
            label: DiagramLabel::PolarAndCartesian,
        }
    }

    fn projector_diagram(&self) -> DiagramSpec {
        DiagramSpec::Single {
            polar: PolarDiagram::from_eulumdat(&self.projector),
            label: DiagramLabel::Projector,
        }
    }

    fn road_diagram(&self) -> DiagramSpec {
        DiagramSpec::Single {
            polar: PolarDiagram::from_eulumdat(&self.road),
            label: DiagramLabel::Asymmetric,
        }
    }

    fn symmetry_pair(&self) -> DiagramSpec {
        DiagramSpec::Pair {
            left: PolarDiagram::from_eulumdat(&self.fluorescent),
            left_label: DiagramLabel::Symmetric,
            right: PolarDiagram::from_eulumdat(&self.road),
            right_label: DiagramLabel::Asymmetric,
        }
    }

    fn fluorescent_heatmap(&self) -> DiagramSpec {
        DiagramSpec::Heatmap {
            heatmap: HeatmapDiagram::from_eulumdat(&self.fluorescent, 400.0, 300.0),
            label: DiagramLabel::Heatmap,
        }
    }
}

pub enum DiagramLabel {
    PolarAndCartesian,
    Symmetric,
    Asymmetric,
    Projector,
    Heatmap,
}

impl DiagramLabel {
    pub fn text<'a>(&self, locale: &'a QuizLocale) -> &'a str {
        match self {
            Self::PolarAndCartesian => &locale.ui.polar_diagram,
            Self::Symmetric => &locale.ui.symmetric,
            Self::Asymmetric => &locale.ui.asymmetric,
            Self::Projector => &locale.ui.projector_narrow,
            Self::Heatmap => locale.ui.heatmap.as_deref().unwrap_or("Heatmap"),
        }
    }
}

pub enum DiagramSpec {
    Single {
        polar: PolarDiagram,
        label: DiagramLabel,
    },
    Pair {
        left: PolarDiagram,
        left_label: DiagramLabel,
        right: PolarDiagram,
        right_label: DiagramLabel,
    },
    Heatmap {
        heatmap: HeatmapDiagram,
        label: DiagramLabel,
    },
}

/// Draw a diagram spec into the given area.
pub fn draw_diagram(frame: &mut Frame, area: Rect, spec: &DiagramSpec, locale: &QuizLocale) {
    match spec {
        DiagramSpec::Single { polar, label } => {
            let label_text = label.text(locale);
            draw_polar_canvas(frame, area, polar, label_text);
        }
        DiagramSpec::Pair {
            left,
            left_label,
            right,
            right_label,
        } => {
            let halves =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);
            draw_polar_canvas(frame, halves[0], left, left_label.text(locale));
            draw_polar_canvas(frame, halves[1], right, right_label.text(locale));
        }
        DiagramSpec::Heatmap { heatmap, label } => {
            let label_text = label.text(locale);
            draw_heatmap(frame, area, heatmap, label_text);
        }
    }
}

/// Render a single polar diagram into the area using braille canvas.
fn draw_polar_canvas(frame: &mut Frame, area: Rect, polar: &PolarDiagram, title: &str) {
    let scale_max = polar.scale.scale_max;
    if scale_max <= 0.0 {
        return;
    }

    let bound = scale_max * 1.15;

    let grid_color = Color::Rgb(50, 50, 65);
    let axis_color = Color::Rgb(70, 70, 90);
    let c0_color = Color::Rgb(80, 140, 255);
    let c90_color = Color::Rgb(255, 100, 100);

    let grid_values = polar.scale.grid_values.clone();
    let c0_points: Vec<(f64, f64)> = polar
        .c0_c180_curve
        .points
        .iter()
        .map(|p| (p.x, p.y))
        .collect();
    let c90_points: Vec<(f64, f64)> = polar
        .c90_c270_curve
        .points
        .iter()
        .map(|p| (p.x, p.y))
        .collect();
    let show_c90 = polar.show_c90_c270();
    let c0_label = polar.c0_c180_curve.label.clone();
    let c90_label = polar.c90_c270_curve.label.clone();

    let canvas = Canvas::default()
        .block(
            Block::default()
                .title_top(format!(" {} ", title))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::DarkGray)),
        )
        .marker(Marker::Braille)
        .x_bounds([-bound, bound])
        .y_bounds([-bound, bound])
        .paint(move |ctx| {
            let steps = 72;

            // Grid circles
            for &r in &grid_values {
                for i in 0..steps {
                    let a0 = i as f64 * std::f64::consts::TAU / steps as f64;
                    let a1 = (i + 1) as f64 * std::f64::consts::TAU / steps as f64;
                    ctx.draw(&CanvasLine {
                        x1: r * a0.cos(),
                        y1: r * a0.sin(),
                        x2: r * a1.cos(),
                        y2: r * a1.sin(),
                        color: grid_color,
                    });
                }
            }

            // Axes (vertical and horizontal)
            let axis_len = scale_max * 1.05;
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -axis_len,
                x2: 0.0,
                y2: axis_len,
                color: axis_color,
            });
            ctx.draw(&CanvasLine {
                x1: -axis_len,
                y1: 0.0,
                x2: axis_len,
                y2: 0.0,
                color: axis_color,
            });

            // Angle labels
            ctx.print(1.0, axis_len * 0.98, ratatui::text::Line::from("0\u{b0}"));
            ctx.print(
                1.0,
                -axis_len * 0.98,
                ratatui::text::Line::from("180\u{b0}"),
            );
            ctx.print(
                axis_len * 0.88,
                -bound * 0.05,
                ratatui::text::Line::from("90\u{b0}"),
            );

            // C0-C180 curve
            draw_curve(ctx, &c0_points, c0_color);

            // C90-C270 curve
            if show_c90 {
                draw_curve(ctx, &c90_points, c90_color);
            }

            // Legend
            let legend_y = -bound * 0.85;
            ctx.print(
                -bound * 0.9,
                legend_y,
                ratatui::text::Line::from(Span::styled(
                    format!("\u{2500} {}", c0_label),
                    Style::new().fg(c0_color),
                )),
            );
            if show_c90 {
                ctx.print(
                    -bound * 0.9,
                    legend_y - bound * 0.1,
                    ratatui::text::Line::from(Span::styled(
                        format!("\u{2500} {}", c90_label),
                        Style::new().fg(c90_color),
                    )),
                );
            }
        });

    frame.render_widget(canvas, area);
}

fn draw_curve(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    points: &[(f64, f64)],
    color: Color,
) {
    if points.len() < 2 {
        return;
    }
    for i in 0..points.len() - 1 {
        ctx.draw(&CanvasLine {
            x1: points[i].0,
            y1: points[i].1,
            x2: points[i + 1].0,
            y2: points[i + 1].1,
            color,
        });
    }
    // Close the curve
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        ctx.draw(&CanvasLine {
            x1: last.0,
            y1: last.1,
            x2: first.0,
            y2: first.1,
            color,
        });
    }
}

/// Render a heatmap using half-block characters (▀) with RGB colors.
///
/// Each terminal cell displays two heatmap rows: top half via fg color,
/// bottom half via bg color, using the upper-half-block character.
fn draw_heatmap(frame: &mut Frame, area: Rect, heatmap: &HeatmapDiagram, title: &str) {
    if heatmap.is_empty() {
        return;
    }

    let block = Block::default()
        .title_top(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 10 || inner.height < 5 {
        return;
    }

    // Reserve space: left margin for gamma labels, bottom for C-plane labels, right for legend
    let label_left = 5u16;
    let label_bottom = 2u16;
    let legend_width = 6u16;

    let plot_x = inner.x + label_left;
    let plot_y = inner.y;
    let plot_w = inner.width.saturating_sub(label_left + legend_width + 1);
    let plot_h = inner.height.saturating_sub(label_bottom);

    if plot_w < 4 || plot_h < 2 {
        return;
    }

    let num_c = heatmap.c_angles.len();
    let num_g = heatmap.g_angles.len();

    // Each terminal row shows 2 gamma rows (using ▀ half-blocks)
    let gamma_rows_visible = (plot_h as usize) * 2;

    // Build the intensity grid for sampling
    // We sample into the heatmap's normalized data
    let max_intensity = heatmap.scale.max_intensity;

    let buf = frame.buffer_mut();

    for ty in 0..plot_h {
        for tx in 0..plot_w {
            let cell_x = (plot_x + tx) as usize;
            let cell_y = (plot_y + ty) as usize;

            if cell_x >= buf.area().width as usize || cell_y >= buf.area().height as usize {
                continue;
            }

            // Map terminal x to C-plane index
            let c_idx = (tx as usize * num_c) / plot_w as usize;
            let c_idx = c_idx.min(num_c.saturating_sub(1));

            // Map terminal y to two gamma indices (top and bottom half of cell)
            let g_top = (ty as usize * 2 * num_g) / gamma_rows_visible;
            let g_bot = ((ty as usize * 2 + 1) * num_g) / gamma_rows_visible;
            let g_top = g_top.min(num_g.saturating_sub(1));
            let g_bot = g_bot.min(num_g.saturating_sub(1));

            // Get intensity values and normalize
            let get_normalized = |ci: usize, gi: usize| -> f64 {
                if ci < num_c && gi < num_g && ci < heatmap.c_angles.len() {
                    if let Some(cell) = heatmap.get_cell(ci, gi) {
                        return cell.normalized;
                    }
                }
                0.0
            };

            let norm_top = get_normalized(c_idx, g_top);
            let norm_bot = get_normalized(c_idx, g_bot);

            let color_top = heatmap_color(norm_top);
            let color_bot = heatmap_color(norm_bot);

            let fg = Color::Rgb(color_top.r, color_top.g, color_top.b);
            let bg = Color::Rgb(color_bot.r, color_bot.g, color_bot.b);

            let cell = &mut buf[(cell_x as u16, cell_y as u16)];
            cell.set_char('\u{2580}'); // ▀ upper half block
            cell.set_fg(fg);
            cell.set_bg(bg);
        }
    }

    // Y-axis labels (gamma angles)
    let gamma_label_step = if num_g > 10 { num_g / 4 } else { 1 };
    for (gi, &g_angle) in heatmap.g_angles.iter().enumerate() {
        if gi % gamma_label_step != 0 {
            continue;
        }
        let ty = (gi * gamma_rows_visible) / (num_g * 2);
        if ty < plot_h as usize {
            let label = format!("{:>3.0}°", g_angle);
            let y = plot_y + ty as u16;
            let x = inner.x;
            if y < inner.y + inner.height {
                for (i, ch) in label.chars().enumerate() {
                    let lx = x + i as u16;
                    if lx < plot_x {
                        let cell = &mut buf[(lx, y)];
                        cell.set_char(ch);
                        cell.set_fg(Color::DarkGray);
                        cell.set_bg(Color::Reset);
                    }
                }
            }
        }
    }

    // X-axis labels (C-plane angles)
    let c_label_y = plot_y + plot_h;
    if c_label_y < inner.y + inner.height {
        let c_label_step = if num_c > 8 { num_c / 4 } else { 1 };
        for (ci, &c_angle) in heatmap.c_angles.iter().enumerate() {
            if ci % c_label_step != 0 {
                continue;
            }
            let tx = (ci * plot_w as usize) / num_c;
            let label = format!("{:.0}°", c_angle);
            for (i, ch) in label.chars().enumerate() {
                let lx = plot_x + tx as u16 + i as u16;
                if lx < plot_x + plot_w {
                    let cell = &mut buf[(lx, c_label_y)];
                    cell.set_char(ch);
                    cell.set_fg(Color::DarkGray);
                    cell.set_bg(Color::Reset);
                }
            }
        }
    }

    // Color legend bar on the right
    let legend_x = plot_x + plot_w + 1;
    if legend_x + 2 < inner.x + inner.width && plot_h >= 4 {
        for ty in 0..plot_h {
            // Map from top (max) to bottom (min)
            let norm_top = 1.0 - (ty as f64 * 2.0) / (plot_h as f64 * 2.0);
            let norm_bot = 1.0 - ((ty as f64 * 2.0 + 1.0) / (plot_h as f64 * 2.0));
            let ct = heatmap_color(norm_top.max(0.0));
            let cb = heatmap_color(norm_bot.max(0.0));

            let cell = &mut buf[(legend_x, plot_y + ty)];
            cell.set_char('\u{2580}');
            cell.set_fg(Color::Rgb(ct.r, ct.g, ct.b));
            cell.set_bg(Color::Rgb(cb.r, cb.g, cb.b));
        }

        // Legend labels: max at top, 0 at bottom
        if max_intensity > 0.0 {
            let max_label = format!("{:.0}", max_intensity);
            for (i, ch) in max_label.chars().enumerate() {
                let lx = legend_x + 1 + i as u16;
                if lx < inner.x + inner.width {
                    let cell = &mut buf[(lx, plot_y)];
                    cell.set_char(ch);
                    cell.set_fg(Color::DarkGray);
                    cell.set_bg(Color::Reset);
                }
            }
            let cell = &mut buf[(legend_x + 1, plot_y + plot_h - 1)];
            cell.set_char('0');
            cell.set_fg(Color::DarkGray);
            cell.set_bg(Color::Reset);
        }
    }
}
