//! Compare the first room of two IFC files the way the designers see it:
//! size, outline, luminaires (position, height, C0, emergency, photometry
//! by flux and peak), exits. Exit code 1 on a difference.
//! `cargo run -p eulumdat-ifc --example compare_ifc -- a.ifc b.ifc`

use eulumdat_ifc::{Building, RoomPlan};

fn load(path: &str) -> (Building, RoomPlan) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let b = Building::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert!(!b.rooms.is_empty(), "{path}: no room");
    let p = b.plan(0);
    (b, p)
}

fn photometry(b: &Building, i: Option<usize>) -> Option<(f64, f64)> {
    let l = eulumdat::Eulumdat::parse(&b.photometries[i?].ldt).ok()?;
    Some((l.luminous_flux(), l.max_intensity()))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (ba, a) = load(&args[0]);
    let (bb, b) = load(&args[1]);
    let mut diffs = Vec::new();
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "DIFF" });
        if !ok {
            diffs.push(what);
        }
    };
    check(
        a.name == b.name,
        format!("room name {:?} / {:?}", a.name, b.name),
    );
    let near = |x: f64, y: f64, t: f64| (x - y).abs() < t;
    check(
        near(a.length, b.length, 1e-3)
            && near(a.width, b.width, 1e-3)
            && near(a.height, b.height, 1e-3),
        format!(
            "size {:.3} × {:.3} × {:.3} / {:.3} × {:.3} × {:.3}",
            a.length, a.width, a.height, b.length, b.width, b.height
        ),
    );
    check(
        near(a.area, b.area, 1e-3),
        format!("area {:.3} / {:.3}", a.area, b.area),
    );
    check(
        a.outline.len() == b.outline.len()
            && a.outline.iter().all(|p| {
                b.outline
                    .iter()
                    .any(|q| (p.0 - q.0).hypot(p.1 - q.1) < 1e-3)
            }),
        format!("outline {} / {} corners", a.outline.len(), b.outline.len()),
    );
    for em in [false, true] {
        let la: Vec<_> = a.luminaires.iter().filter(|l| l.emergency == em).collect();
        let lb: Vec<_> = b.luminaires.iter().filter(|l| l.emergency == em).collect();
        let kind = if em {
            "emergency luminaires"
        } else {
            "luminaires"
        };
        check(
            la.len() == lb.len(),
            format!("{kind}: {} / {}", la.len(), lb.len()),
        );
        let mut worst = (0.0f64, 0.0f64, 0.0f64);
        for l in &la {
            if let Some(m) = lb.iter().min_by(|p, q| {
                (p.x - l.x)
                    .hypot(p.y - l.y)
                    .total_cmp(&(q.x - l.x).hypot(q.y - l.y))
            }) {
                let d = (m.x - l.x).hypot(m.y - l.y);
                let r = (m.rotation_deg - l.rotation_deg).rem_euclid(360.0);
                worst = (
                    worst.0.max(d),
                    worst.1.max((m.z - l.z).abs()),
                    worst.2.max(r.min(360.0 - r)),
                );
            }
        }
        check(
            worst.0 < 1e-3 && worst.1 < 1e-3 && worst.2 < 0.01,
            format!(
                "{kind}: worst position {:.4} m, height {:.4} m, C0 {:.3}°",
                worst.0, worst.1, worst.2
            ),
        );
        if let (Some(l), Some(m)) = (la.first(), lb.first()) {
            let (pa, pb) = (photometry(&ba, l.photometry), photometry(&bb, m.photometry));
            let same = match (pa, pb) {
                (Some(x), Some(y)) => near(x.0, y.0, x.0 * 0.005) && near(x.1, y.1, x.1 * 0.005),
                (None, None) => true,
                _ => false,
            };
            check(
                same,
                format!("{kind}: photometry (flux, peak cd/klm) {pa:?} / {pb:?}"),
            );
        }
    }
    let exits = |p: &RoomPlan| p.doors.iter().filter(|d| d.exit).count();
    check(
        exits(&a) == exits(&b),
        format!("exits {} / {}", exits(&a), exits(&b)),
    );
    if !diffs.is_empty() {
        std::process::exit(1);
    }
}
