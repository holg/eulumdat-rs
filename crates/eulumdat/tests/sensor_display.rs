//! Sensor display mode: when an LDT is known to be a sensor (e.g. via a GLDF
//! container's `contentType=sensor/sensldt`), the angular *shape* diagrams stay
//! geometrically identical to a luminaire's, but the radial/grid axis is a
//! **relative detection-sensitivity response**, not a photometric intensity.
//!
//! `SvgLabels::for_sensor()` is the single chokepoint that relabels every
//! shape diagram (polar, cartesian, butterfly, isocandela, …) at once.
//!
//! Absolute-photometric views (illuminance/lux) are NOT meaningful for a sensor
//! and must not appear; the Cone geometry SVG already carries no lux, which we
//! lock in here.
//!
//! Fixture: the real SLV `sensor.ldt` (a detection lobe stored in cd/klm slots).

use eulumdat::diagram::{CartesianDiagram, ConeDiagram, PolarDiagram, SvgLabels, SvgTheme};
use eulumdat::Eulumdat;

const SENSOR: &str = include_str!("fixtures/sensor.ldt");

fn sensor_ldt() -> Eulumdat {
    Eulumdat::parse(SENSOR).expect("sensor.ldt must parse")
}

#[test]
fn for_sensor_labels_replace_photometric_units() {
    let s = SvgLabels::for_sensor();
    assert_eq!(s.intensity_axis, "Relative sensitivity");
    assert_eq!(s.intensity_unit, "%");
    assert_eq!(s.heatmap_title, "Sensor response (relative)");

    // It overrides ONLY the intensity/response wording; the rest mirrors English
    // (so axis/plane labels etc. are unchanged).
    let en = SvgLabels::english();
    assert_eq!(s.plane_c0_c180, en.plane_c0_c180);
    assert_eq!(s.gamma_axis, en.gamma_axis);
    assert_eq!(s.c_plane_axis, en.c_plane_axis);
}

#[test]
fn polar_sensor_mode_relabels_unit_but_keeps_geometry() {
    let ldt = sensor_ldt();
    let polar = PolarDiagram::from_eulumdat(&ldt);

    let lum_svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());
    let sensor_svg = polar.to_svg(
        500.0,
        500.0,
        &SvgTheme::light().with_labels(SvgLabels::for_sensor()),
    );

    // Sensor mode shows the relative-sensitivity unit, not the photometric one.
    assert!(sensor_svg.contains('%'));
    assert!(
        !sensor_svg.contains("cd/1000lm") && !sensor_svg.contains("cd/klm"),
        "sensor polar must not advertise a photometric unit"
    );
    // The luminaire rendering DOES carry the photometric unit (sanity contrast).
    assert!(lum_svg.contains("cd/1000lm") || lum_svg.contains("cd/klm"));

    // Geometry is identical: same number of plotted polar points (the curve path
    // count is independent of labels). Compare the count of "<path" occurrences.
    let count = |s: &str| s.matches("<path").count();
    assert_eq!(
        count(&lum_svg),
        count(&sensor_svg),
        "relabeling must not change the plotted geometry"
    );
}

#[test]
fn cartesian_sensor_mode_relabels_axis() {
    let ldt = sensor_ldt();
    let cart = CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);

    let sensor_svg = cart.to_svg(
        600.0,
        400.0,
        &SvgTheme::light().with_labels(SvgLabels::for_sensor()),
    );

    assert!(
        sensor_svg.contains("Relative sensitivity"),
        "cartesian sensor mode must use the sensitivity axis label"
    );
    assert!(
        !sensor_svg.contains("Intensity (cd/klm)"),
        "cartesian sensor mode must not use the photometric axis label"
    );
}

#[test]
fn cone_geometry_svg_carries_no_illuminance() {
    // The Cone view that consumers render (ConeDiagram::to_svg) is pure beam/field
    // angle geometry — the detection cone. It must not contain any absolute
    // photometric (lux/lumen) text, which would be meaningless for a sensor and
    // is fabricated from the sensor's cloned-from-luminaire bogus flux.
    let ldt = sensor_ldt();
    let svg = ConeDiagram::from_eulumdat(&ldt, 8.0).to_svg(600.0, 400.0, &SvgTheme::light());

    let lower = svg.to_lowercase();
    assert!(!lower.contains("lux"), "cone SVG must not show lux");
    assert!(!lower.contains(" lm"), "cone SVG must not show lumens");
    assert!(
        !lower.contains("illuminance"),
        "cone SVG must not show an illuminance table"
    );

    // It SHOULD still show the cone — i.e. the beam/field angle geometry exists.
    let cone = ConeDiagram::from_eulumdat(&ldt, 8.0);
    assert!(cone.field_angle > 0.0, "cone must have a real field angle");
}
