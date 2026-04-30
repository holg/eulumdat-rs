//! Regression tests for the OXL parser against a real OxyTech-published
//! fixture (`oxytech_pinco_36.oxl`, the LITESTAR sample shipped with the
//! OXL-UK spec PDF in `docs/`).
//!
//! The asserts below are spec-derived invariants — they don't depend on
//! a hand-locked numerical answer, just on what the file actually
//! contains and on the documented mapping in `eulumdat::atla::oxl`.

use eulumdat::atla::oxl;
use eulumdat::atla::types::{IntensityUnits, SymmetryType};

const FIXTURE: &str = include_str!("fixtures/oxytech_pinco_36.oxl");

#[test]
fn pinco_parses_into_a_package_with_one_luminaire() {
    let pkg = oxl::parse(FIXTURE).expect("OXL fixture must parse");

    assert_eq!(pkg.header.litepack_version, "0.0003");
    assert_eq!(pkg.header.product_family.as_deref(), Some("OXL"));
    assert_eq!(
        pkg.header.product_name.as_deref(),
        Some("Pinco 36 LED RA 4000K")
    );
    assert_eq!(pkg.luminaires.len(), 1, "fixture has one Luminaire entry");
}

#[test]
fn pinco_luminaire_has_full_photometric_grid() {
    let pkg = oxl::parse(FIXTURE).expect("OXL fixture must parse");
    let lum = &pkg.luminaires[0];

    // Catalog metadata round-trips through Luminaire/ProductIdentity.
    assert_eq!(lum.header.catalog_number.as_deref(), Some("A12654"));
    assert_eq!(
        lum.header.description.as_deref(),
        Some("Pinco 36 LED RA 4000K")
    );

    // Bounding box is in meters in OXL but in millimeters on our side.
    let dims = lum
        .luminaire
        .as_ref()
        .and_then(|l| l.dimensions.as_ref())
        .expect("luminaire dimensions");
    // OXL: WidthC0C180 = 0.285 m → 285 mm
    assert!(
        (dims.width - 285.0).abs() < 1e-6,
        "width mm = {}",
        dims.width
    );
    // OXL: LengthC90C270 = 0.508 m → 508 mm
    assert!(
        (dims.length - 508.0).abs() < 1e-6,
        "length mm = {}",
        dims.length
    );

    let em = lum.emitters.first().expect("at least one emitter");
    // LampList has Quantity=1, Flux=4247, Power=84.5, CCT=4000, CRI=75
    assert_eq!(em.quantity, 1);
    assert_eq!(em.rated_lumens, Some(4247.0));
    assert_eq!(em.input_watts, Some(84.5));
    assert_eq!(em.cct, Some(4000.0));
    assert_eq!(em.color_rendering.as_ref().and_then(|c| c.ra), Some(75.0));

    let dist = em
        .intensity_distribution
        .as_ref()
        .expect("intensity distribution");

    // Spec / sample says NumCH=36 (C-planes 0..350 step 10), NumGV=91
    // (gamma 0..180 step 2).
    assert_eq!(
        dist.horizontal_angles.len(),
        36,
        "C-plane count drift; got {} angles",
        dist.horizontal_angles.len()
    );
    assert_eq!(
        dist.vertical_angles.len(),
        91,
        "gamma sample count drift; got {} angles",
        dist.vertical_angles.len()
    );

    // C-angles must be sorted numerically.
    for w in dist.horizontal_angles.windows(2) {
        assert!(
            w[0] < w[1],
            "C-angles not numerically sorted: {} >= {}",
            w[0],
            w[1]
        );
    }
    // First C-angle is 0°, last is 350°, step 10° throughout.
    assert!((dist.horizontal_angles[0] - 0.0).abs() < 1e-9);
    assert!((dist.horizontal_angles[35] - 350.0).abs() < 1e-9);

    // FluxUsed (3876.48 lm) is present, so cd values were converted to
    // cd/klm. Pre-conversion we'd see ~135.7 cd at the gamma=0 row, post
    // conversion ≈ 135.7 × 1000 / 3876.48 ≈ 35.0 cd/klm.
    assert_eq!(dist.units, IntensityUnits::CandelaPerKilolumen);
    let i_zero = dist.intensities[0][0];
    assert!(
        (35.0..36.0).contains(&i_zero),
        "first intensity post-conversion = {i_zero} (expected ~35 cd/klm)"
    );

    // The fixture's <SymmetryType>asymmetricCG</SymmetryType> maps to
    // atla `SymmetryType::None` — no symmetry assumed.
    assert_eq!(dist.symmetry, Some(SymmetryType::None));
}

#[test]
fn pinco_round_trips_through_eulumdat_without_panic() {
    // Convert the parsed atla doc → Eulumdat → LDT → re-parse. This
    // catches mapping bugs where our atla output is shaped wrong enough
    // to reject after a writer pass.
    let pkg = oxl::parse(FIXTURE).expect("OXL fixture must parse");
    let lum = &pkg.luminaires[0];

    let ldt = lum.to_eulumdat();
    let ldt_text = ldt.to_ldt();
    eulumdat::Eulumdat::parse(&ldt_text).expect("OXL → atla → LDT should round-trip");
}
