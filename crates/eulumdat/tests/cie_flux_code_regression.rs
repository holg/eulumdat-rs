//! Regression test for the CIE 52-1982 flux code formula.
//!
//! The reference luminaire is the LEDVANCE BIOLUX HCL DL DN150 S 20W TW ZB
//! recessed downlight. Per `docs/check_cie_flux_code.md`, the published
//! GLDF flux code for this product is **`95 100 100 100 100`** — meaning:
//!
//! - 95 % of downward flux is in the inner 41.4° cone (focused beam),
//! - all of it is inside 60°,
//! - all of it is inside 75°,
//! - all luminaire flux flows downward (DLOR = 100 %),
//! - LOR = 100 % (luminaire flux equals lamp flux).
//!
//! That matches the textbook narrow-beam recessed-downlight signature and
//! is what a correct implementation must produce. An earlier draft of
//! `cie_flux_codes` returned `100 100 93 0 0` because of three bugs in the
//! formula (wrong cone angles, wrong basis, wrong fields for N4/N5); this
//! test guards against the bugs returning.

use eulumdat::{Eulumdat, PhotometricCalculations};

const FIXTURE: &str = include_str!("fixtures/biolux_dn150.ldt");
const SENSOR_WIDE: &str = include_str!("fixtures/variant2_sensor_wide.ldt");

#[test]
fn biolux_dn150_matches_published_flux_code() {
    let ldt = Eulumdat::parse(FIXTURE).expect("biolux_dn150 fixture must parse");
    let codes = PhotometricCalculations::cie_flux_codes(&ldt);

    // Display rounds via banker's rounding and joins with spaces.
    let formatted = codes.to_string();
    assert_eq!(
        formatted, "95 100 100 100 100",
        "CIE flux code drift — got `{formatted}`, expected `95 100 100 100 100` per CIE 52-1982 \
         applied to the published GLDF reference (BIOLUX HCL DL DN150). \
         See docs/check_cie_flux_code.md."
    );
}

#[test]
fn biolux_dn150_invariants_hold() {
    let ldt = Eulumdat::parse(FIXTURE).expect("biolux_dn150 fixture must parse");
    let codes = PhotometricCalculations::cie_flux_codes(&ldt);

    // Spec-derived invariants — independent of the specific luminaire.
    // N1, N2, N3 are cumulative fractions of *downward* flux on
    // increasing cones (0–41.4 ⊂ 0–60 ⊂ 0–75), so monotonically
    // non-decreasing and bounded to [0, 100].
    assert!(
        codes.n1 >= 0.0 && codes.n1 <= 100.0,
        "n1 out of range: {}",
        codes.n1
    );
    assert!(
        codes.n2 >= 0.0 && codes.n2 <= 100.0,
        "n2 out of range: {}",
        codes.n2
    );
    assert!(
        codes.n3 >= 0.0 && codes.n3 <= 100.0,
        "n3 out of range: {}",
        codes.n3
    );
    assert!(codes.n1 <= codes.n2 + 1e-9, "expected n1 ≤ n2");
    assert!(codes.n2 <= codes.n3 + 1e-9, "expected n2 ≤ n3");

    // N4 is DLOR — % of total flux that flows downward.
    assert!(
        codes.n4 >= 0.0 && codes.n4 <= 100.0,
        "n4 (DLOR) out of range: {}",
        codes.n4
    );

    // N5 is LOR — must equal the LDT's stored light_output_ratio exactly.
    assert!(
        (codes.n5 - ldt.light_output_ratio).abs() < 1e-9,
        "n5 should equal ldt.light_output_ratio ({}) — got {}",
        ldt.light_output_ratio,
        codes.n5
    );
}

/// Regression for the 41.4° boundary-truncation rounding bug.
///
/// The downward-flux integrator at `calculations.rs::downward_for_plane`
/// uses trapezoidal integration. When the cone half-angle (41.4°) falls
/// between two gamma samples (e.g. 40° and 42.5°), the trapezoid must
/// average `I(40°)` with the *interpolated* `I(41.4°)` — not with the
/// far-endpoint `I(42.5°)`. On a steeply-falling shoulder this mistake
/// pulls N1 just below the .5 rounding boundary.
///
/// For this OCULUS CW wide-beam fixture the true N1 is 68.5251 → rounds
/// to **69**. The buggy integrator returns 68.4417 → rounds to **68**.
#[test]
fn variant2_sensor_wide_boundary_rounding() {
    let ldt = Eulumdat::parse(SENSOR_WIDE).expect("variant2 fixture must parse");
    let codes = PhotometricCalculations::cie_flux_codes(&ldt);
    let formatted = codes.to_string();
    assert_eq!(
        formatted, "69 95 99 100 100",
        "41.4° boundary interpolation regression — N1 should be 69 (true value 68.525), \
         not 68 (the value produced when the trapezoid averages I[40°] with I[42.5°] \
         instead of with the linearly-interpolated I(41.4°)). got `{formatted}`"
    );
}
