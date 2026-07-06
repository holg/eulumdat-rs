//! Documents that a sensor's detection-pattern LDT is NOT safely distinguishable
//! from a luminaire's photometric LDT by file content alone.
//!
//! Background: some products (e.g. SLV) ship a sensor's detection-sensitivity
//! pattern in the same EULUMDAT/LDT format used for a luminaire's light
//! distribution. The EULUMDAT format has no "this is a sensor" marker, and at
//! least one real exporter copies the parent luminaire's entire identity and
//! lamp block (manufacturer, product name, lamp type, flux, CRI, color
//! temperature, wattage) verbatim into the sensor file.
//!
//! Consequence: eulumdat-rs cannot — and deliberately does not — try to classify
//! sensor vs. luminaire from the file. That knowledge must come from external
//! context (e.g. a GLDF container's `contentType=sensor/sensldt`). The intuitive
//! "a sensor wouldn't carry CRI/CT" rule is a FALSE NEGATIVE on this exact file,
//! as the assertions below prove.
//!
//! `sensor.ldt` and `wide_luminaire.ldt` are the real SLV OCULUS CW pair taken
//! from gldf-rs `tests/test-create/`. (Note: the separately-named
//! `fixtures/variant2_sensor_wide.ldt` is a normal wide-beam luminaire, NOT a
//! sensor — do not confuse them.)

use eulumdat::Eulumdat;

const SENSOR: &str = include_str!("fixtures/sensor.ldt");
const LUMINAIRE: &str = include_str!("fixtures/wide_luminaire.ldt");

/// Both files parse as valid EULUMDAT — the sensor is not malformed, it is a
/// structurally normal LDT that merely carries a detection pattern.
#[test]
fn both_parse_as_valid_eulumdat() {
    let sensor = Eulumdat::parse(SENSOR).expect("sensor.ldt must parse as valid EULUMDAT");
    let lum = Eulumdat::parse(LUMINAIRE).expect("wide_luminaire.ldt must parse as valid EULUMDAT");

    assert!(!sensor.intensities.is_empty());
    assert!(!lum.intensities.is_empty());
    assert_eq!(sensor.c_angles.len(), lum.c_angles.len());
    assert_eq!(sensor.g_angles.len(), lum.g_angles.len());
}

/// The core finding: the sensor and the luminaire are indistinguishable by their
/// identity and lamp-set blocks. In particular CRI and color temperature — the
/// fields one would expect a sensor to omit — are present and identical on both.
///
/// If this test ever starts FAILING because the two files diverge, that does NOT
/// mean we gained a reliable classifier; a real sensor that happens to differ
/// here is still not provably a sensor from content alone. The test exists to
/// document that the fields are equal on this canonical real-world pair.
#[test]
fn sensor_and_luminaire_share_identical_identity_and_lamp_block() {
    let sensor = Eulumdat::parse(SENSOR).unwrap();
    let lum = Eulumdat::parse(LUMINAIRE).unwrap();

    // Identity / header
    assert_eq!(sensor.identification, lum.identification, "manufacturer");
    assert_eq!(sensor.luminaire_name, lum.luminaire_name, "product name");
    assert_eq!(sensor.luminaire_number, lum.luminaire_number, "product number");
    assert_eq!(sensor.type_indicator, lum.type_indicator, "Ityp");
    assert_eq!(sensor.symmetry, lum.symmetry, "Isym");

    // Lamp-set block — including the CRI/CT one might expect a sensor to lack.
    assert_eq!(sensor.lamp_sets.len(), 1);
    assert_eq!(lum.lamp_sets.len(), 1);
    let s = &sensor.lamp_sets[0];
    let l = &lum.lamp_sets[0];

    assert_eq!(s.num_lamps, l.num_lamps, "num_lamps (both absolute, -1)");
    assert_eq!(s.lamp_type, l.lamp_type, "lamp type description");
    assert_eq!(s.total_luminous_flux, l.total_luminous_flux, "flux (lm)");
    assert_eq!(
        s.color_appearance, l.color_appearance,
        "color temperature — sensor carries the luminaire's bogus CT"
    );
    assert_eq!(
        s.color_rendering_group, l.color_rendering_group,
        "CRI — sensor carries the luminaire's bogus CRI"
    );
    assert_eq!(s.wattage_with_ballast, l.wattage_with_ballast, "wattage");

    // Concretely: CRI and CT are non-empty on the SENSOR, disproving the
    // "missing CRI/CT ⇒ sensor" heuristic.
    assert_eq!(s.color_appearance, "2000K");
    assert_eq!(s.color_rendering_group, "90");
}

/// What DOES differ is only the measured data (the detection lobe vs. the light
/// beam). This is captured for documentation, but is NOT a safe classifier — a
/// narrow-beam luminaire can produce a similar profile. We assert merely that
/// the intensity payloads are not identical, i.e. the files carry different
/// measurements behind an identical identity.
#[test]
fn only_the_measured_distribution_differs() {
    let sensor = Eulumdat::parse(SENSOR).unwrap();
    let lum = Eulumdat::parse(LUMINAIRE).unwrap();

    assert_ne!(
        sensor.intensities, lum.intensities,
        "the detection pattern and the light beam are different measurements"
    );
}
