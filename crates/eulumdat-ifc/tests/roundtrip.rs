//! Round trip of the Interior designer's sample room through IFC:
//! design → IFC (bimifc-writer) → import (bimifc-parser) → the same room,
//! and export → import once more → still the same.
//!
//! The sample is the landing page's L-shaped open office (20 × 15 m, 3 m,
//! demo LED at 500 lx, exits at (20, 3.75) and (5, 15)); its IFC is kept in
//! `tests/fixtures/interior_sample.ifc` (our own synthetic data):
//! `cargo run -p eulumdat-ifc --features export --example make_sample_ifc`.
//!
//! Needs the feature `export` (and so the local ../bimifc checkout):
//! `cargo test -p eulumdat-ifc --features export`.
#![cfg(feature = "export")]

use eulumdat_ifc::export::{room_to_ifc, ExportOptions, ExportPhotometry};
use eulumdat_ifc::{Building, PlanOpening, RoomPlan};

mod common;
use common::{interior_sample, SAMPLE};

/// Export, read back, the one room as a plan (+ the photometries found).
fn round_trip(
    plan: &RoomPlan,
    ph: &[ExportPhotometry],
) -> (RoomPlan, Vec<ExportPhotometry>, String) {
    let ifc = room_to_ifc(plan, ph, &ExportOptions::default());
    let b = Building::parse(&ifc).expect("our own IFC reads back");
    assert_eq!(b.rooms.len(), 1, "one room: {:?}", b.warnings);
    let back = b.plan(0);
    let ph = b.photometries.iter().map(ExportPhotometry::from).collect();
    (back, ph, ifc)
}

/// The nearest point of an outline to (x, y).
fn on_outline(poly: &[(f64, f64)], x: f64, y: f64) -> (f64, f64) {
    let n = poly.len();
    let mut best = (f64::INFINITY, (x, y));
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t =
            (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy).max(1e-12)).clamp(0.0, 1.0);
        let q = (a.0 + t * dx, a.1 + t * dy);
        let d = (q.0 - x).hypot(q.1 - y);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// The same room: outline corners, size, height, area; every luminaire
/// at the same place, height and direction with the same LDT; the same
/// exits (compared where they meet the wall).
fn assert_same(a: &RoomPlan, a_ph: &[ExportPhotometry], b: &RoomPlan, b_ph: &[ExportPhotometry]) {
    const MM: f64 = 1e-3;
    assert_eq!(a.name, b.name);
    assert!(
        (a.length - b.length).abs() < MM,
        "length {} vs {}",
        a.length,
        b.length
    );
    assert!(
        (a.width - b.width).abs() < MM,
        "width {} vs {}",
        a.width,
        b.width
    );
    assert!(
        (a.height - b.height).abs() < MM,
        "height {} vs {}",
        a.height,
        b.height
    );
    assert!(
        (a.area - b.area).abs() < 1e-3,
        "area {} vs {}",
        a.area,
        b.area
    );
    assert_eq!(
        a.outline.len(),
        b.outline.len(),
        "{:?} vs {:?}",
        a.outline,
        b.outline
    );
    for p in &a.outline {
        assert!(
            b.outline.iter().any(|q| (p.0 - q.0).hypot(p.1 - q.1) < MM),
            "corner {p:?} missing in {:?}",
            b.outline
        );
    }
    assert_eq!(a.luminaires.len(), b.luminaires.len());
    for l in &a.luminaires {
        let m = b
            .luminaires
            .iter()
            .min_by(|p, q| {
                (p.x - l.x)
                    .hypot(p.y - l.y)
                    .total_cmp(&(q.x - l.x).hypot(q.y - l.y))
            })
            .unwrap();
        assert!((m.x - l.x).hypot(m.y - l.y) < MM, "luminaire {l:?} → {m:?}");
        assert!((m.z - l.z).abs() < MM, "height {l:?} → {m:?}");
        assert!(
            angle_diff(m.rotation_deg, l.rotation_deg) < 0.01,
            "C0 {l:?} → {m:?}"
        );
        assert_eq!(m.emergency, l.emergency, "{l:?} → {m:?}");
        let (pa, pb) = (
            l.photometry.unwrap(),
            m.photometry.expect("photometry kept"),
        );
        assert_eq!(a_ph[pa].ldt, b_ph[pb].ldt, "the LDT travels unchanged");
    }
    assert_eq!(a.doors.len(), b.doors.len());
    for d in &a.doors {
        let wall = on_outline(&a.outline, d.x, d.y);
        let m = b
            .doors
            .iter()
            .map(|e| (e, on_outline(&b.outline, e.x, e.y)))
            .min_by(|p, q| {
                (p.1 .0 - wall.0)
                    .hypot(p.1 .1 - wall.1)
                    .total_cmp(&(q.1 .0 - wall.0).hypot(q.1 .1 - wall.1))
            })
            .unwrap();
        assert!(
            (m.1 .0 - wall.0).hypot(m.1 .1 - wall.1) < 0.01,
            "door {d:?} → {:?}",
            m.0
        );
        assert_eq!(m.0.fire_exit, d.fire_exit, "{d:?} → {:?}", m.0);
        assert_eq!(m.0.exit, d.exit, "{d:?} → {:?}", m.0);
        assert!(
            (m.0.width - d.width).abs() < 0.01,
            "width {d:?} → {:?}",
            m.0
        );
    }
}

#[test]
fn interior_sample_survives_export_and_import() {
    let (plan, ph) = interior_sample();
    assert!(plan.luminaires.len() > 10, "{}", plan.luminaires.len());
    let (back, back_ph, _) = round_trip(&plan, &ph);
    eprintln!(
        "{} · {:.1} × {:.1} × {:.1} m · {:.1} m² · {} luminaires · {} exits",
        back.name,
        back.length,
        back.width,
        back.height,
        back.area,
        back.luminaires.len(),
        back.doors.iter().filter(|d| d.exit).count()
    );
    assert_same(&plan, &ph, &back, &back_ph);
    // export the imported room again: nothing drifts
    let (again, again_ph, _) = round_trip(&back, &back_ph);
    assert_same(&back, &back_ph, &again, &again_ph);
}

#[test]
fn turned_luminaires_and_a_window_survive() {
    let (mut plan, ph) = interior_sample();
    for (i, l) in plan.luminaires.iter_mut().enumerate() {
        l.rotation_deg = [0.0, 30.0, 90.0, 225.0][i % 4];
        l.z = 2.7;
    }
    plan.doors[1].fire_exit = false; // an ordinary door to the outside
    plan.windows.push(PlanOpening {
        x: 0.0,
        y: 7.0,
        width: 2.4,
        height: 1.5,
        sill: 0.9,
        fire_exit: false,
        exit: false,
        name: "Window 1".into(),
    });
    let (back, back_ph, _) = round_trip(&plan, &ph);
    assert_same(&plan, &ph, &back, &back_ph);
    // the sample's 11 emergency luminaires turned too, still emergency
    assert_eq!(back.luminaires.iter().filter(|l| l.emergency).count(), 11);
    assert_eq!(back_ph.len(), 2);
    assert_eq!(back.windows.len(), 1);
    let w = &back.windows[0];
    assert!(
        (w.sill - 0.9).abs() < 0.01 && (w.height - 1.5).abs() < 0.01,
        "{w:?}"
    );
    assert!((w.width - 2.4).abs() < 0.01, "{w:?}");
}

#[test]
fn committed_sample_is_current() {
    let (plan, ph) = interior_sample();
    let committed = std::fs::read_to_string(SAMPLE).unwrap_or_else(|_| {
        panic!("{SAMPLE} missing: cargo run -p eulumdat-ifc --features export --example make_sample_ifc")
    });
    let b = Building::parse(&committed).unwrap();
    let from_file = b.plan(0);
    let file_ph: Vec<ExportPhotometry> =
        b.photometries.iter().map(ExportPhotometry::from).collect();
    // GlobalIds of luminaires and the header time differ per export, so
    // the file is compared by what it holds, not byte for byte
    assert_same(&plan, &ph, &from_file, &file_ph);
}
