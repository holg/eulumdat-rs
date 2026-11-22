//! LDT → ATLA → LDT. The web editor keeps every luminaire as an ATLA
//! document and converts back for the diagrams, designers, validation and
//! the LDT download, so the way back must give the file as loaded.

use eulumdat::atla::LuminaireOpticalData;
use eulumdat::{Eulumdat, PhotometricCalculations, Symmetry};
use std::path::PathBuf;

fn template(name: &str) -> Eulumdat {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("eulumdat-wasm/templates");
    Eulumdat::from_file(dir.join(name)).unwrap()
}

const LDTS: [&str; 4] = [
    "road_luminaire.ldt",
    "projector.ldt",
    "fluorescent_luminaire.ldt",
    "floor_uplight.ldt",
];

#[test]
fn ldt_atla_ldt_is_exact() {
    for name in LDTS {
        let src = template(name);
        let once = LuminaireOpticalData::from_eulumdat(&src).to_eulumdat();
        assert_eq!(once.to_ldt(), src.to_ldt(), "{name}");
        let twice = LuminaireOpticalData::from_eulumdat(&once).to_eulumdat();
        assert_eq!(twice.to_ldt(), src.to_ldt(), "{name}, second trip");
    }
}

/// Without the source copy (an ATLA file, or a document read back from
/// JSON) the plain conversion still gives a valid file with the same table.
#[test]
fn plain_conversion_keeps_symmetry_angles_and_rows() {
    for name in LDTS {
        let src = template(name);
        let mut doc = LuminaireOpticalData::from_eulumdat(&src);
        doc.source_ldt = None;
        let back = doc.to_eulumdat();
        assert_eq!(back.symmetry, src.symmetry, "{name}");
        assert_eq!(back.num_c_planes, src.num_c_planes, "{name}");
        assert_eq!(back.intensities, src.intensities, "{name}");
        assert_eq!(back.c_angles.len(), src.c_angles.len(), "{name}");
        for (a, b) in back.c_angles.iter().zip(&src.c_angles) {
            assert!((a - b).abs() < 1e-6, "{name}: C {a} vs {b}");
        }
        assert!(back.validate_strict().is_ok(), "{name}");
        // the lamp count is the emitter's quantity, not part of the type
        assert_eq!(
            back.lamp_sets[0].lamp_type, src.lamp_sets[0].lamp_type,
            "{name}"
        );
    }
}

/// The road light stores C90…C270 with unequal steps (5° except around
/// C90 / C270). It used to come back as C0–C180 with 72 planes and 27
/// rows: a broken file.
#[test]
fn c90_c270_road_light_stays_c90_c270() {
    let src = template("road_luminaire.ldt");
    assert_eq!(src.symmetry, Symmetry::PlaneC90C270);
    assert_eq!(src.c_plane_distance, 0.0);
    let mut doc = LuminaireOpticalData::from_eulumdat(&src);
    let rows = &doc.emitters[0]
        .intensity_distribution
        .as_ref()
        .unwrap()
        .horizontal_angles;
    assert_eq!(rows.first(), Some(&90.0));
    assert_eq!(rows.last(), Some(&270.0));
    doc.source_ldt = None;
    let back = doc.to_eulumdat();
    assert_eq!(back.symmetry, Symmetry::PlaneC90C270);
    assert_eq!((back.num_c_planes, back.intensities.len()), (52, 27));
    assert_eq!(back.c_plane_distance, 0.0, "unequal steps: Dc = 0");
}

/// The table's own flux agrees with the declared light output ratio; the
/// C90–C270 integral took the widths of the planes C0, C5, … and gave 110 %.
#[test]
fn c90_c270_flux_uses_the_stored_planes() {
    let src = template("road_luminaire.ldt");
    let lor = PhotometricCalculations::total_output(&src) / 10.0;
    assert!(
        (lor - src.light_output_ratio).abs() < 0.5,
        "{lor} vs {}",
        src.light_output_ratio
    );
}

/// An edit on the ATLA side (here the lamp flux) changes the luminaire;
/// what ATLA cannot hold (declared LOR, file name, colour wording, the
/// symmetric angle list) stays as in the file.
#[test]
fn edited_document_keeps_the_ldt_only_values() {
    let src = template("fluorescent_luminaire.ldt");
    let mut doc = LuminaireOpticalData::from_eulumdat(&src);
    let flux = doc.emitters[0].rated_lumens.unwrap();
    doc.emitters[0].rated_lumens = Some(flux * 2.0);
    let back = doc.to_eulumdat();
    assert_ne!(back.to_ldt(), src.to_ldt());
    assert!((back.lamp_sets[0].total_luminous_flux - flux * 2.0).abs() < 1e-6);
    assert_eq!(back.light_output_ratio, src.light_output_ratio);
    assert_eq!(back.file_name, src.file_name);
    assert_eq!(
        back.lamp_sets[0].color_appearance,
        src.lamp_sets[0].color_appearance
    );
    assert_eq!(back.c_angles, src.c_angles);
    assert_eq!(back.symmetry, src.symmetry);
}
