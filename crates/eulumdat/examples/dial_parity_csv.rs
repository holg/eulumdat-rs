//! Emit the DIAL discrepancy table in the ORIGINAL sheet format (tab-separated,
//! German formatting), with a new "iesna.eu NEU" block appended showing our
//! radiosity-measured values, ready to paste back into the comparison sheet.
//!
//! Run: cargo run -q -p eulumdat --example dial_parity_csv

use eulumdat::{
    compute_form_factors, direct_illuminance, solve_radiosity, workplane_stats_normative, Eulumdat,
    EvaluationStandard, Luminaire, RoomMesh, SurfaceReflectances, Vec3, WorkPlane,
};

const MF: f64 = 0.80;
const DIVISIONS: usize = 16;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

fn ldt_for(key: &str) -> Eulumdat {
    let file = match key {
        "alya" => "alya_359lm.ldt",
        "acrux" => "acrux_452lm.ldt",
        other => panic!("unknown luminaire {other}"),
    };
    Eulumdat::from_file(fixture(file)).unwrap()
}

/// Display flux per the original sheet labels ("Alya 359lm", "Acrux 452lm").
fn produkt(key: &str) -> &'static str {
    match key {
        "alya" => "Alya 359lm",
        "acrux" => "Acrux 452lm",
        _ => unreachable!(),
    }
}

/// German decimal: dot -> comma, drop trailing ".0" (2.0 -> "2", 2.5 -> "2,5").
fn de(v: f64) -> String {
    let s = if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    };
    s.replace('.', ",")
}

/// U0 with two decimals, German comma, trailing zero trimmed (0.70 -> "0,7").
fn de_u0(v: f64) -> String {
    let mut s = format!("{v:.2}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s.replace('.', ",")
}

fn main() {
    let csv = std::fs::read_to_string(fixture("dial_discrepancy.csv")).unwrap();

    let mut out = String::new();
    // Semicolon-separated (German decimals use comma, so ';' avoids collision),
    // with a fourth block for our new engine. Imports as columns in Numbers/Excel.
    out.push_str(
        "Produkt;Raum;Ziel;;Ergebnis iesna.eu (old deploy!);;;;Dial Prüfung;;;;;iesna.eu NEU (Radiosity);;;\n",
    );

    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("lum,") {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        let lum = f[0];
        let (l, w, h): (f64, f64, f64) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
        );
        let target: f64 = f[4].parse().unwrap();
        let typ = if f[5] == "buero" { "Büro" } else { "Flur" };
        let (nx, ny): (usize, usize) = (f[6].parse().unwrap(), f[7].parse().unwrap());
        let n = nx * ny;
        let iesna_e: f64 = f[8].parse().unwrap();
        let iesna_u0: f64 = f[9].parse().unwrap();
        let dial_e: f64 = f[10].parse().unwrap();
        let dial_u0: f64 = f[11].parse().unwrap();

        // Run our engine on the same fixed layout.
        let ldt = ldt_for(lum);
        let flux = ldt.total_luminous_flux() * MF;
        let mut lums = Vec::with_capacity(n);
        for i in 0..nx {
            for j in 0..ny {
                lums.push(Luminaire {
                    pos: Vec3::new(
                        (i as f64 + 0.5) * (l / nx as f64),
                        (j as f64 + 0.5) * (w / ny as f64),
                        h,
                    ),
                    flux,
                });
            }
        }
        let refl = SurfaceReflectances {
            ceiling: 0.7,
            wall: 0.5,
            floor: 0.2,
        };
        let mesh = RoomMesh::new(l, w, h, refl, DIVISIONS);
        let ff = compute_form_factors(&mesh);
        let direct = direct_illuminance(&mesh, &lums, &ldt);
        let res = solve_radiosity(&mesh, &ff, &direct, 250, 1e-6);
        let plane = if typ == "Büro" {
            WorkPlane::Office
        } else {
            WorkPlane::Corridor
        };
        let s = workplane_stats_normative(
            &mesh,
            &res,
            &lums,
            &ldt,
            plane,
            EvaluationStandard::En12464_2021,
        );
        let ours_pct = (100.0 * s.e_avg / dial_e).round();

        let room = format!("{}x{}x{}", de(l), de(w), de(h));
        let layout = format!("{nx}x{ny}");
        let ziel = format!("{}lx", de(target));

        // Mirror the original column order. Original "ratio" was DIAL/iesna_old.
        let old_ratio = (100.0 * dial_e / iesna_e).round();

        out.push_str(&format!(
            // Produkt;Raum;Ziel;Typ | [old: layout;n;E;U0] | [dial: layout;n;E;U0;ratio%] | [NEU: layout;n;E;U0;vs-dial%]
            "{produkt};{room};{ziel};{typ};{layout};{n};{ie};{ieu};;{layout};{n};{de};{du};{old_ratio:.0} %;{layout};{n};{oe};{ou};{ours_pct:.0} %\n",
            produkt = produkt(lum),
            ie = de(iesna_e),
            ieu = de_u0(iesna_u0),
            de = de(dial_e),
            du = de_u0(dial_u0),
            oe = s.e_avg.round() as i64,
            ou = de_u0(s.u0),
        ));
    }

    let path = format!(
        "{}/../../docs/dial_parity_measured.csv",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::write(&path, &out).expect("write csv");
    print!("{out}");
    eprintln!("\n-> wrote {path}");
}
