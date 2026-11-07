//! The SLV "OCULUS CW" pair from the GLDF consortium's old Tria 2 demo:
//! `sensor.ldt` sat in the container's `sensor/` folder, but it is the
//! photometry of a luminaire (a copy of `wide_luminaire.ldt`'s product), not a
//! detection pattern. That is why the two files share their identity and lamp
//! block, and why they once looked "indistinguishable".
//!
//! Real sensor LDTs (detection volume in metres, sensor parameters in lines
//! 33–42) are recognisable: `eulumdat::sensor::is_sensor_ldt` finds all
//! 1 187 sensor LDTs of the Relux Net corpus and none of 34 916 luminaire
//! LDTs (see `src/sensor/detect.rs`). Both files here are luminaires, and
//! the check says so. Declared context (GLDF `contentType="sensor/…"`) still
//! decides first.
//!
//! (`fixtures/variant2_sensor_wide.ldt` is a normal wide-beam luminaire whose
//! name contains "sensor".)

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

/// The two files share their identity and lamp-set blocks (CRI and colour
/// temperature included): the "sensor" is a copy of a luminaire file. So
/// identity / lamp-block rules ("no CRI ⇒ sensor") say nothing; the sensor
/// fingerprint lives in the table values and lines 33–42.
#[test]
fn sensor_and_luminaire_share_identical_identity_and_lamp_block() {
    let sensor = Eulumdat::parse(SENSOR).unwrap();
    let lum = Eulumdat::parse(LUMINAIRE).unwrap();

    // Identity / header
    assert_eq!(sensor.identification, lum.identification, "manufacturer");
    assert_eq!(sensor.luminaire_name, lum.luminaire_name, "product name");
    assert_eq!(
        sensor.luminaire_number, lum.luminaire_number,
        "product number"
    );
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

    // CRI and CT are filled in on the file from the sensor folder, so a
    // "missing CRI/CT ⇒ sensor" rule would be meaningless.
    assert_eq!(s.color_appearance, "2000K");
    assert_eq!(s.color_rendering_group, "90");
}

/// The intensity tables differ (two measurements behind one identity), but
/// both are light distributions in cd/klm, not ranges in metres.
#[test]
fn only_the_measured_distribution_differs() {
    let sensor = Eulumdat::parse(SENSOR).unwrap();
    let lum = Eulumdat::parse(LUMINAIRE).unwrap();

    assert_ne!(
        sensor.intensities, lum.intensities,
        "the detection pattern and the light beam are different measurements"
    );
}

/// Neither file has the sensor LDT fingerprint: both are luminaires.
#[test]
fn neither_file_is_a_sensor_ldt() {
    for text in [SENSOR, LUMINAIRE] {
        let l = Eulumdat::parse(text).unwrap();
        assert!(!eulumdat::sensor::is_sensor_ldt(&l));
    }
    let demo = Eulumdat::parse(include_str!("fixtures/sensors/demo_ceiling_360.ldt")).unwrap();
    assert!(eulumdat::sensor::is_sensor_ldt(&demo));
}
