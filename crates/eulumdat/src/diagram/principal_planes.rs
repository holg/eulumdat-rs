//! Principal-planes (PV / PC) side-by-side polar diagram.
//!
//! In road-lighting practice, the two principal planes of a luminaire's
//! intensity distribution are conventionally shown next to each other
//! in technical specs:
//!
//! - **PV** (*Plane Vertical* / longitudinal — the C0–C180 cut, along
//!   the road axis). Shows how the beam stretches up and down the road.
//! - **PC** (*Plane Crosswise* / transverse — the C90–C270 cut, across
//!   the road). Shows how the beam spreads side-to-side toward the
//!   sidewalks and the opposing curb.
//!
//! Both halves are rendered as polar plots at the same intensity scale,
//! so the eye can compare longitudinal vs. transverse spread directly
//! — the classic "is this an asymmetric road luminaire or a symmetric
//! downlight?" read.
//!
//! The output is a single self-contained `<svg>` with two side-by-side
//! polar panels and a small caption identifying which plane is which.
//! It reuses [`PolarDiagram::from_eulumdat_for_plane`] for the heavy
//! lifting and just composites the two halves.
//!
//! # Why "PV / PC" and not "PV / PC chart"?
//!
//! "PV/PC" is the road-lighting industry's name for the principal-plane
//! intensity diagram. We renamed our earlier Pareto-front layout chart
//! away from this acronym (it's now
//! [`crate::street::svg::layout_tradeoff_chart`]) so this name can mean
//! what road-lighting engineers expect.
//!
//! # Example
//!
//! ```no_run
//! use eulumdat::Eulumdat;
//! use eulumdat::diagram::{principal_planes_svg, SvgTheme};
//!
//! let ldt = Eulumdat::from_file("road_luminaire.ldt").unwrap();
//! let svg = principal_planes_svg(&ldt, 800.0, 420.0, &SvgTheme::default());
//! assert!(svg.contains("<svg"));
//! ```

use super::polar::PolarDiagram;
use super::svg::SvgTheme;
use crate::Eulumdat;

/// C-plane angle for the longitudinal cut (along the road).
const PV_PLANE_DEG: f64 = 0.0;
/// C-plane angle for the transverse cut (across the road).
const PC_PLANE_DEG: f64 = 90.0;

/// Render both principal planes (PV and PC) as side-by-side polar plots.
///
/// `width` is the total canvas width; each half panel gets half of it
/// minus a small gutter. `height` is the canvas height of both panels.
/// The two halves use the **same** intensity scale (the larger of the
/// two peaks) so they're visually comparable.
pub fn principal_planes_svg(ldt: &Eulumdat, width: f64, height: f64, theme: &SvgTheme) -> String {
    let gutter = 12.0;
    let panel_w = ((width - gutter) / 2.0).max(120.0);
    let panel_h = height.max(120.0);
    let panel_size = panel_w.min(panel_h);

    let pv = PolarDiagram::from_eulumdat_for_plane(ldt, PV_PLANE_DEG);
    let pc = PolarDiagram::from_eulumdat_for_plane(ldt, PC_PLANE_DEG);

    // Use the larger peak as the shared scale so the two halves are
    // comparable. We tweak each PolarDiagram's `scale.scale_max` to the
    // shared value, keeping its grid_values as-is (PolarDiagram picks
    // them adaptively in `from_eulumdat_for_plane`; for shared scaling
    // we override the cap so neither half clips above its own peak).
    let mut pv = pv;
    let mut pc = pc;
    let shared_max = pv.scale.scale_max.max(pc.scale.scale_max);
    pv.scale.scale_max = shared_max;
    pc.scale.scale_max = shared_max;

    let pv_inner = inner_svg(&pv.to_svg(panel_size, panel_size, theme));
    let pc_inner = inner_svg(&pc.to_svg(panel_size, panel_size, theme));

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" font-family="{font}">"#,
        font = theme.font_family
    ));
    svg.push_str(&format!(
        r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
        theme.background
    ));

    // Left panel: PV (along road)
    let left_x = (panel_w - panel_size) / 2.0;
    svg.push_str(&format!(
        r#"<g transform="translate({left_x:.2},0)">{pv_inner}</g>"#,
    ));
    // Caption.
    svg.push_str(&format!(
        r#"<text x="{x:.1}" y="{y:.1}" fill="{c}" font-size="13" font-weight="bold" text-anchor="middle">PV — along road (C0–C180)</text>"#,
        x = panel_w / 2.0,
        y = 18.0,
        c = theme.text,
    ));

    // Right panel: PC (across road)
    let right_x_origin = panel_w + gutter;
    let right_x = right_x_origin + (panel_w - panel_size) / 2.0;
    svg.push_str(&format!(
        r#"<g transform="translate({right_x:.2},0)">{pc_inner}</g>"#,
    ));
    svg.push_str(&format!(
        r#"<text x="{x:.1}" y="{y:.1}" fill="{c}" font-size="13" font-weight="bold" text-anchor="middle">PC — across road (C90–C270)</text>"#,
        x = right_x_origin + panel_w / 2.0,
        y = 18.0,
        c = theme.text,
    ));

    svg.push_str("</svg>");
    svg
}

/// Strip the outer `<svg ...>` wrapper from a self-contained polar SVG
/// so it can be re-embedded inside a `<g>` group. Matches up to the
/// first `>` after `<svg`, plus the closing `</svg>`.
fn inner_svg(s: &str) -> String {
    let start = s
        .find("<svg")
        .and_then(|i| s[i..].find('>').map(|j| i + j + 1));
    let end = s.rfind("</svg>");
    match (start, end) {
        (Some(a), Some(b)) if a <= b => s[a..b].to_string(),
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn road_ldt() -> Eulumdat {
        let p = "../eulumdat-wasm/templates/road_luminaire.ldt";
        let content = std::fs::read_to_string(p).expect("template must exist");
        Eulumdat::parse(&content).expect("template must parse")
    }

    #[test]
    fn principal_planes_svg_emits_both_panels() {
        let ldt = road_ldt();
        let svg = principal_planes_svg(&ldt, 800.0, 420.0, &SvgTheme::default());

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Both captions visible.
        assert!(svg.contains("PV"), "PV caption missing: {}", &svg[..200]);
        assert!(svg.contains("PC"), "PC caption missing");
        assert!(svg.contains("C0–C180"), "PV plane label missing");
        assert!(svg.contains("C90–C270"), "PC plane label missing");
        // At least one polar curve from each half (rough sanity).
        assert!(
            svg.matches("<path").count() >= 2,
            "expected ≥2 curves, got {}",
            svg.matches("<path").count()
        );
    }

    #[test]
    fn inner_svg_strips_wrapper() {
        let s = r#"<svg xmlns="..." viewBox="0 0 10 10"><circle/></svg>"#;
        assert_eq!(inner_svg(s), "<circle/>");
    }

    #[test]
    fn inner_svg_passes_through_when_no_wrapper() {
        let s = "no svg here";
        assert_eq!(inner_svg(s), s);
    }
}
