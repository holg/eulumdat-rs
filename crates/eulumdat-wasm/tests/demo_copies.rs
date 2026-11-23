//! `demo/` holds copies of our synthetic demo files (a published crate can
//! only embed its own files). In the repository they must equal the
//! originals in the `eulumdat` crate; in the packaged crate there is
//! nothing to compare with.

use std::path::Path;

#[test]
fn demo_copies_match_the_originals() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = here.join("../eulumdat/tests/fixtures");
    if !fixtures.is_dir() {
        return;
    }
    let pairs = [
        ("sensors", "demo_ceiling_360.ldt"),
        ("sensors", "demo_corridor.ldt"),
        ("sensors", "demo_ceiling_sensxml.xml"),
        ("sensors", "demo_wall_180_sensxml.xml"),
        ("sensors", "demo_highbay_sensxml.xml"),
        ("sensors", "demo_corridor_sensxml.xml"),
        ("emergency", "demo_em_route.ldt"),
        ("emergency", "demo_em_open.ldt"),
        ("colour_quality", "demo_led_3000k.spd"),
    ];
    for (dir, name) in pairs {
        let original = std::fs::read(fixtures.join(dir).join(name)).unwrap();
        let copy = std::fs::read(here.join("demo").join(name)).unwrap();
        assert!(
            original == copy,
            "demo/{name} differs from fixtures/{dir}/{name}"
        );
    }
}
