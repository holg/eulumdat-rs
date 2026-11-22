//! The committed Interior sample (`tests/fixtures/interior_sample.ifc`,
//! written by `examples/make_sample_ifc.rs` with bimifc-writer) imports
//! as the room it was made from. Needs no feature and no sibling
//! checkout, so it runs everywhere (CI against crates.io bimifc too).

use eulumdat_ifc::Building;

#[test]
fn the_interior_sample_imports() {
    let text = include_str!("fixtures/interior_sample.ifc");
    let b = Building::parse(text).unwrap();
    assert_eq!(b.rooms.len(), 1, "{:?}", b.warnings);
    let p = b.plan(0);
    assert_eq!(p.name, "Open Office");
    assert!((p.length - 20.0).abs() < 1e-3 && (p.width - 15.0).abs() < 1e-3);
    assert!((p.height - 3.0).abs() < 1e-3);
    assert!((p.area - 225.0).abs() < 1e-3, "the L: 300 m² minus 75 m²");
    assert_eq!(p.outline.len(), 6);
    // 60 downlights (zonal layout, 500 lx) + 11 emergency luminaires
    let (normal, em): (Vec<_>, Vec<_>) = p.luminaires.iter().partition(|l| !l.emergency);
    assert_eq!((normal.len(), em.len()), (60, 11));
    assert!(normal
        .iter()
        .all(|l| (l.z - 3.0).abs() < 1e-3 && l.photometry == Some(0)));
    assert!(em.iter().all(|l| l.photometry == Some(1)));
    assert!(
        em.iter().any(|l| (l.rotation_deg - 90.0).abs() < 0.01),
        "turned along the route"
    );
    assert_eq!(b.photometries.len(), 2);
    assert_eq!((b.photometries[0].count, b.photometries[1].count), (60, 11));
    let exits: Vec<_> = p.doors.iter().filter(|d| d.fire_exit && d.exit).collect();
    assert_eq!(exits.len(), 2, "{:?}", p.doors);
}
