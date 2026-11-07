//! Plan view of sensor footprints (metres), comparable to the plan view
//! Relux draws for sensor files.

use super::Footprint;
use crate::diagram::SvgTheme;
use core::fmt::Write;

/// One footprint to draw, with its legend text and colour.
#[derive(Debug, Clone)]
pub struct SensorPlanTrace {
    pub footprint: Footprint,
    pub label: String,
    pub color: String,
}

/// Plan view: sensor at the centre, rings in metres, one filled outline per
/// trace, C0 to the right, C90 up.
pub struct SensorPlanSvg;

/// Default colours (yellow / green / blue, the order Relux uses).
pub const TRACE_COLORS: [&str; 4] = ["#d4b106", "#16a34a", "#2563eb", "#9333ea"];

fn nice_step(max: f64) -> f64 {
    let raw = (max / 4.0).max(0.25);
    let p = 10f64.powf(raw.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if raw <= m * p {
            return m * p;
        }
    }
    10.0 * p
}

/// Reach of a footprint towards the C angle `c_deg` (linear between the
/// tabulated C-planes).
fn reach_along(fp: &Footprint, c_deg: f64) -> f64 {
    let n = fp.reach.len();
    if n == 0 || fp.c_step_deg <= 0.0 {
        return 0.0;
    }
    let pos = c_deg.rem_euclid(360.0) / fp.c_step_deg;
    let i = (pos.floor() as usize) % n;
    let f = pos - pos.floor();
    fp.reach[i] * (1.0 - f) + fp.reach[(i + 1) % n] * f
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl SensorPlanSvg {
    pub fn render(traces: &[SensorPlanTrace], size: f64, theme: &SvgTheme) -> String {
        Self::render_with_cut(traces, size, theme, None)
    }

    /// [`Self::render`] with the vertical cut C / C + 180° drawn through the
    /// sensor (the plane the polar and cartesian views show), and the reach
    /// of each trace along it marked.
    pub fn render_with_cut(
        traces: &[SensorPlanTrace],
        size: f64,
        theme: &SvgTheme,
        c_plane: Option<f64>,
    ) -> String {
        Self::render_styled(traces, &[], size, theme, c_plane)
    }

    /// [`Self::render_with_cut`] where `dashed[i]` draws trace `i` as a
    /// dashed outline without fill (the second sensor of a comparison).
    pub fn render_styled(
        traces: &[SensorPlanTrace],
        dashed: &[bool],
        size: f64,
        theme: &SvgTheme,
        c_plane: Option<f64>,
    ) -> String {
        let is_dashed = |i: usize| dashed.get(i).copied().unwrap_or(false);
        let max = traces
            .iter()
            .map(|t| t.footprint.max_range)
            .fold(0.0, f64::max)
            .max(1.0);
        let step = nice_step(max);
        let rings = (max / step).ceil().max(1.0) as usize;
        let extent = step * rings as f64;
        let legend_h = 18.0 * traces.len() as f64 + 10.0;
        let (w, h) = (size, size + legend_h);
        let cx = size / 2.0;
        let cy = size / 2.0;
        let scale = (size / 2.0 - 24.0) / extent;

        let mut s = String::new();
        let _ = write!(
            s,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" font-family="{}">"#,
            esc(&theme.font_family)
        );
        let _ = write!(
            s,
            r#"<rect width="{w}" height="{h}" fill="{}"/>"#,
            theme.background
        );
        // rings and labels
        for i in 1..=rings {
            let r = i as f64 * step * scale;
            let _ = write!(
                s,
                r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="none" stroke="{}" stroke-width="1"/>"#,
                theme.grid
            );
            let _ = write!(
                s,
                r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}">{} m</text>"#,
                cx + 3.0,
                cy + r - 3.0,
                theme.text_secondary,
                (i as f64 * step * 100.0).round() / 100.0
            );
        }
        // axes every 30°
        for k in 0..12 {
            let a = (k as f64 * 30.0).to_radians();
            let r = extent * scale;
            let _ = write!(
                s,
                r#"<line x1="{cx:.1}" y1="{cy:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="{}"/>"#,
                cx + r * a.cos(),
                cy - r * a.sin(),
                if k % 3 == 0 { &theme.axis } else { &theme.grid },
                if k % 3 == 0 { 1.0 } else { 0.5 }
            );
        }
        let _ = write!(
            s,
            r#"<text x="{:.1}" y="{:.1}" font-size="10" fill="{}">C0</text><text x="{:.1}" y="{:.1}" font-size="10" fill="{}">C90</text>"#,
            cx + extent * scale - 18.0,
            cy - 4.0,
            theme.text_secondary,
            cx + 4.0,
            cy - extent * scale + 10.0,
            theme.text_secondary
        );
        // footprints, largest first so smaller ones stay visible
        let mut order: Vec<usize> = (0..traces.len()).collect();
        order.sort_by(|&a, &b| {
            traces[b]
                .footprint
                .area_m2
                .partial_cmp(&traces[a].footprint.area_m2)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        for &i in &order {
            let t = &traces[i];
            if t.footprint.is_empty() {
                continue;
            }
            let mut d = String::new();
            for (k, (x, y)) in t.footprint.outline.iter().enumerate() {
                let _ = write!(
                    d,
                    "{}{:.1},{:.1}",
                    if k == 0 { "M" } else { "L" },
                    cx + x * scale,
                    cy - y * scale
                );
            }
            d.push('Z');
            let style = if is_dashed(i) {
                r#"fill-opacity="0" stroke-dasharray="7,4""#
            } else {
                r#"fill-opacity="0.18""#
            };
            let _ = write!(
                s,
                r#"<path d="{d}" fill="{c}" {style} stroke="{c}" stroke-width="{}"/>"#,
                theme.curve_width.max(1.5),
                c = t.color
            );
        }
        if let Some(c) = c_plane {
            let r = extent * scale;
            let (a, b) = (c.to_radians(), (c + 180.0).to_radians());
            let _ = write!(
                s,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.5" stroke-dasharray="6,4"/>"#,
                cx + r * b.cos(),
                cy - r * b.sin(),
                cx + r * a.cos(),
                cy - r * a.sin(),
                theme.text
            );
            let _ = write!(
                s,
                r#"<text x="{:.1}" y="{:.1}" font-size="11" font-weight="600" fill="{}" text-anchor="middle">C{:.0}</text>"#,
                (cx + (r + 12.0) * a.cos()).clamp(16.0, size - 16.0),
                (cy - (r + 12.0) * a.sin() + 4.0).clamp(12.0, size - 4.0),
                theme.text,
                c.rem_euclid(360.0)
            );
            // reach of every trace along the cut, both directions
            for (i, t) in traces.iter().enumerate() {
                if t.footprint.is_empty() {
                    continue;
                }
                // filled dot = solid trace, ring = dashed trace
                let (fill, stroke) = if is_dashed(i) {
                    (theme.background.as_str(), t.color.as_str())
                } else {
                    (t.color.as_str(), theme.background.as_str())
                };
                for ang in [c, c + 180.0] {
                    let d = reach_along(&t.footprint, ang) * scale;
                    let rad = ang.to_radians();
                    let _ = write!(
                        s,
                        r#"<circle cx="{:.1}" cy="{:.1}" r="3.5" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#,
                        cx + d * rad.cos(),
                        cy - d * rad.sin(),
                    );
                }
            }
        }
        let _ = write!(
            s,
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="3" fill="{}"/>"#,
            theme.text
        );
        // legend
        for (i, t) in traces.iter().enumerate() {
            let y = size + 6.0 + 18.0 * i as f64;
            let swatch = if is_dashed(i) {
                r#"fill-opacity="0" stroke-dasharray="4,2""#
            } else {
                r#"fill-opacity="0.35""#
            };
            let _ = write!(
                s,
                r#"<rect x="12" y="{:.1}" width="14" height="10" fill="{c}" {swatch} stroke="{c}"/><text x="32" y="{:.1}" font-size="12" fill="{}">{} · max {:.2} m · {:.1} m²</text>"#,
                y,
                y + 9.5,
                theme.text,
                esc(&t.label),
                t.footprint.max_range,
                t.footprint.area_m2,
                c = t.color
            );
        }
        s.push_str("</svg>");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::DetectionVolume;

    #[test]
    fn renders_traces_and_legend() {
        let v = DetectionVolume::new(crate::sensor::volume::tests::box_ldt(10.0, 1.5, 4.25));
        let traces = vec![SensorPlanTrace {
            footprint: v.footprint(2.0, 2.0),
            label: "walking, 2.0 m below".into(),
            color: TRACE_COLORS[0].into(),
        }];
        let svg = SensorPlanSvg::render(&traces, 400.0, &SvgTheme::light());
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains("walking, 2.0 m below"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("10 m") || svg.contains("12 m") || svg.contains("5 m"));
    }

    #[test]
    fn nice_steps() {
        assert_eq!(nice_step(10.7), 5.0);
        assert_eq!(nice_step(4.0), 1.0);
        assert_eq!(nice_step(20.0), 5.0);
    }
}
