//! The demo LED (synthetic, `tests/fixtures/colour_quality/make_demo_led.py`)
//! that the editor ships as template `demo-led-3000k` and the landing page
//! quotes. If these numbers move, update the landing text
//! (`crates/eulumdat-wasm/landing/src/gen_landing.py`, keys `cq_*`).
#![cfg(all(feature = "xml", feature = "json"))]

use eulumdat::atla::{spd_loader, ColourQuality};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/colour_quality/demo_led_3000k.spd"
);
const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../eulumdat-wasm/templates/demo_led_3000k.xml"
);

fn check(q: &ColourQuality, what: &str) {
    let tm = &q.tm30;
    let c = q.cri.as_ref().expect("CRI");
    let near = |v: f64, want: f64, tol: f64, name: &str| {
        assert!(
            (v - want).abs() <= tol,
            "{what}: {name} {v} (want {want} ± {tol})"
        );
    };
    near(tm.cct, 3021.0, 5.0, "CCT");
    near(tm.duv, 0.0, 0.0005, "Duv");
    near(tm.rf, 82.6, 0.2, "Rf");
    near(tm.rg, 98.1, 0.2, "Rg");
    near(c.ra, 81.7, 0.2, "Ra");
    near(c.r9(), 9.9, 0.5, "R9");
    near(c.re, 75.5, 0.3, "Re");
    // The story of the landing page: reds (bin 1) about 11 % duller, and the
    // card labels them.
    near(tm.rcs_hue[0], -10.9, 0.3, "Rcs,h1");
    assert!(q.callout_bins().contains(&0), "{what}: red not labelled");
}

#[test]
fn demo_spectrum_numbers() {
    let spd = spd_loader::load(std::path::Path::new(FIXTURE))
        .expect("demo spectrum")
        .spd;
    check(&ColourQuality::from_spd(&spd).unwrap(), "fixture");
}

#[test]
fn editor_template_carries_the_same_spectrum() {
    let xml = std::fs::read_to_string(TEMPLATE).expect("template");
    let doc = eulumdat::atla::xml::parse(&xml).expect("ATLA");
    let spd = doc
        .emitters
        .iter()
        .find_map(|e| e.spectral_distribution.clone())
        .expect("spectrum in the template");
    check(&ColourQuality::from_spd(&spd).unwrap(), "template");
    assert!(doc
        .header
        .description
        .as_deref()
        .unwrap_or("")
        .contains("synthetic"));
}
