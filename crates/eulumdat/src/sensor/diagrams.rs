//! Which diagrams apply to a sensor, and rendering them with sensor wording.
//!
//! A sensor LDT holds a detection volume (range in metres per C/γ), so the
//! diagrams that show the *shape* of the stored distribution stay valid:
//! polar (vertical sections), cartesian (range over γ), heatmap (range over
//! C × γ) and the 3D butterfly (the volume itself), plus the sensor's own
//! plan view of the detection areas. Everything built on light thresholds
//! or illuminance (cone, beam angle, isolux, isocandela, BUG, LCS, CU, UGR,
//! street / area / zonal calculations) does not apply.

use super::{DetectionVolume, Motion, SensorPlanSvg, SensorPlanTrace, TRACE_COLORS};
use crate::diagram::{
    ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgLabels, SvgTheme,
};

/// Diagram kinds offered for a sensor LDT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SensorDiagram {
    /// Detection areas on a plane (tangential / radial / presence).
    Plan,
    /// Vertical sections C0-C180 / C90-C270 of the volume.
    Polar,
    /// Range over γ per C-plane.
    Cartesian,
    /// Range over C × γ.
    Heatmap,
    /// The detection volume as a 3D butterfly.
    Volume3d,
}

impl SensorDiagram {
    pub const ALL: [SensorDiagram; 5] = [
        SensorDiagram::Plan,
        SensorDiagram::Polar,
        SensorDiagram::Cartesian,
        SensorDiagram::Heatmap,
        SensorDiagram::Volume3d,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SensorDiagram::Plan => "Detection areas",
            SensorDiagram::Polar => "Polar (range)",
            SensorDiagram::Cartesian => "Cartesian (range)",
            SensorDiagram::Heatmap => "Range heatmap",
            SensorDiagram::Volume3d => "Detection volume (3D)",
        }
    }
}

/// `theme` with the sensor wording (units "m", range axis) but its colours.
pub fn sensor_theme(theme: &SvgTheme) -> SvgTheme {
    let mut t = theme.clone();
    t.labels = SvgLabels::for_sensor();
    t
}

/// Plan view of all motion types the file has, cut `depth` metres below
/// the sensor.
pub fn plan_svg(volume: &DetectionVolume, depth: f64, size: f64, theme: &SvgTheme) -> String {
    plan_svg_with_cut(volume, depth, size, theme, None)
}

/// [`plan_svg`] with the vertical cut at `c_plane` drawn through the sensor.
pub fn plan_svg_with_cut(
    volume: &DetectionVolume,
    depth: f64,
    size: f64,
    theme: &SvgTheme,
    c_plane: Option<f64>,
) -> String {
    let traces: Vec<SensorPlanTrace> = Motion::ALL
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            volume
                .motion_footprint(*m, depth, 1.0)
                .map(|footprint| SensorPlanTrace {
                    footprint,
                    label: format!("{}, {depth:.2} m below the sensor", m.label()),
                    color: TRACE_COLORS[i].to_string(),
                })
        })
        .collect();
    SensorPlanSvg::render_with_cut(&traces, size, theme, c_plane)
}

/// Render one sensor diagram. `depth` is only used by [`SensorDiagram::Plan`].
pub fn render(
    volume: &DetectionVolume,
    kind: SensorDiagram,
    width: f64,
    height: f64,
    depth: f64,
    theme: &SvgTheme,
) -> String {
    render_at_plane(volume, kind, width, height, depth, theme, None)
}

/// [`render`] at one C-plane: polar and cartesian show the pair C / C + 180°
/// (as the luminaire views' C-plane slider), the plan view draws that cut
/// through the sensor; the heatmap and the 3D view ignore it.
pub fn render_at_plane(
    volume: &DetectionVolume,
    kind: SensorDiagram,
    width: f64,
    height: f64,
    depth: f64,
    theme: &SvgTheme,
    c_plane: Option<f64>,
) -> String {
    let theme = sensor_theme(theme);
    let ldt = volume.ldt();
    match (kind, c_plane) {
        (SensorDiagram::Plan, _) => {
            plan_svg_with_cut(volume, depth, width.min(height), &theme, c_plane)
        }
        (SensorDiagram::Polar, Some(c)) => {
            PolarDiagram::from_eulumdat_for_plane(ldt, c).to_svg(width, height, &theme)
        }
        (SensorDiagram::Polar, None) => {
            PolarDiagram::from_eulumdat(ldt).to_svg(width, height, &theme)
        }
        (SensorDiagram::Cartesian, Some(c)) => {
            CartesianDiagram::from_eulumdat_for_plane(ldt, c, width, height)
                .to_svg(width, height, &theme)
        }
        (SensorDiagram::Cartesian, None) => {
            CartesianDiagram::from_eulumdat(ldt, width, height, 8).to_svg(width, height, &theme)
        }
        (SensorDiagram::Heatmap, _) => {
            HeatmapDiagram::from_eulumdat_with_scale(ldt, width, height, 1.0)
                .to_svg(width, height, &theme)
        }
        (SensorDiagram::Volume3d, _) => {
            ButterflyDiagram::from_eulumdat(ldt, width, height, 60.0).to_svg(width, height, &theme)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::volume::tests::box_ldt;

    fn texts(svg: &str) -> String {
        let mut out = String::new();
        let mut rest = svg;
        while let Some(i) = rest.find("<text") {
            let Some(gt) = rest[i..].find('>') else { break };
            let body = &rest[i + gt + 1..];
            let end = body.find("</text>").unwrap_or(0);
            out.push_str(&body[..end]);
            out.push('|');
            rest = &body[end..];
        }
        out
    }

    #[test]
    fn sensor_diagrams_carry_no_photometric_units() {
        let mut ldt = box_ldt(8.0, 4.0, 3.0);
        // a lamp block that would scale a luminaire heatmap
        ldt.lamp_sets.push(crate::LampSet {
            num_lamps: 1,
            lamp_type: "x".into(),
            total_luminous_flux: 600.0,
            color_appearance: String::new(),
            color_rendering_group: String::new(),
            wattage_with_ballast: 0.0,
        });
        ldt.direct_ratios = [0.5, 1.0, 0.25, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 0.0];
        let v = DetectionVolume::new(ldt);
        for kind in SensorDiagram::ALL {
            let svg = render(&v, kind, 500.0, 500.0, 2.0, &SvgTheme::light());
            let t = texts(&svg);
            assert!(svg.starts_with("<svg"), "{kind:?}");
            for bad in [
                "cd/klm",
                "cd/1000lm",
                "Candela",
                " cd<",
                " cd|",
                "lux",
                " lx",
            ] {
                assert!(!t.contains(bad), "{kind:?} shows {bad:?}: {t}");
            }
        }
        // heatmap is not scaled by the (meaningless) 600 lm
        let svg = render(
            &v,
            SensorDiagram::Heatmap,
            500.0,
            500.0,
            2.0,
            &SvgTheme::light(),
        );
        let max = v.ldt().max_intensity();
        assert!(svg.contains(&format!("Max: {max:.2} m")), "{}", texts(&svg));
    }

    #[test]
    fn plan_lists_every_motion_type() {
        let mut ldt = box_ldt(8.0, 4.0, 3.0);
        ldt.direct_ratios = [0.5, 1.0, 0.25, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 0.0];
        let svg = plan_svg(&DetectionVolume::new(ldt), 2.0, 400.0, &SvgTheme::light());
        for m in Motion::ALL {
            assert!(svg.contains(m.label()), "{m:?}");
        }
    }

    /// The C-plane slider: polar / cartesian show the chosen pair, the plan
    /// draws the cut and marks the reach along it.
    #[test]
    fn c_plane_views() {
        // 8 m × 4 m box: C0 and C90 differ.
        let v = DetectionVolume::new(box_ldt(8.0, 4.0, 3.0));
        let theme = SvgTheme::light();
        let default = render(&v, SensorDiagram::Polar, 400.0, 400.0, 2.0, &theme);
        let c0 = render_at_plane(
            &v,
            SensorDiagram::Polar,
            400.0,
            400.0,
            2.0,
            &theme,
            Some(0.0),
        );
        let c90 = render_at_plane(
            &v,
            SensorDiagram::Polar,
            400.0,
            400.0,
            2.0,
            &theme,
            Some(90.0),
        );
        assert_ne!(c0, c90);
        assert_ne!(default, c90);
        assert!(texts(&c90).contains("C90"), "{}", texts(&c90));
        let cart = render_at_plane(
            &v,
            SensorDiagram::Cartesian,
            400.0,
            300.0,
            2.0,
            &theme,
            Some(90.0),
        );
        assert!(cart.starts_with("<svg"));
        let plan = render_at_plane(
            &v,
            SensorDiagram::Plan,
            400.0,
            400.0,
            2.0,
            &theme,
            Some(90.0),
        );
        assert!(plan.contains(r#"stroke-dasharray="6,4""#));
        assert!(texts(&plan).contains("C90"));
        assert!(!render(&v, SensorDiagram::Plan, 400.0, 400.0, 2.0, &theme).contains("6,4"));
    }
}
