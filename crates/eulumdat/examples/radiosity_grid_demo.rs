//! Reproduce the iesna.eu-vs-DIAL discrepancy table with our radiosity engine
//! on the binding normative (Stockmar) grid, targeting the "DIAL Prüfung"
//! column (DIAL evaluated on the SAME fixed layout).
//!
//! LDTs are read in place from ../light-other-rs/tmp (not committed).
//! Run: cargo run -p eulumdat --example radiosity_grid_demo

use eulumdat::{
    compute_form_factors, direct_illuminance, solve_radiosity, workplane_stats_normative,
    EvaluationStandard, Eulumdat, Luminaire, RoomMesh, SurfaceReflectances, Vec3, WorkPlane,
};

const TMP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../light-other-rs/tmp");
const MF: f64 = 0.80; // DIALux default Wartungsfaktor

struct Row {
    lum: &'static str,
    l: f64,
    w: f64,
    h: f64,
    office: bool, // true=Büro(0.75m), false=Flur(0.2m)
    nx: usize,    // fixed layout columns (along length)
    ny: usize,    // fixed layout rows (along width)
    // DIAL Prüfung reference (same layout):
    dial_e: f64,
    dial_u0: f64,
}

fn rows() -> Vec<Row> {
    // lum, L,W,H, office, nx,ny, dial_e, dial_u0  (target lx omitted; not used)
    vec![
        Row { lum: "Alya",  l:2.0,  w:18.0, h:2.5, office:false, nx:2,  ny:12, dial_e:103.0, dial_u0:0.72 },
        Row { lum: "Alya",  l:3.5,  w:22.0, h:4.0, office:false, nx:2,  ny:23, dial_e:96.0,  dial_u0:0.72 },
        Row { lum: "Alya",  l:2.0,  w:2.0,  h:2.5, office:false, nx:2,  ny:2,  dial_e:88.0,  dial_u0:0.92 },
        Row { lum: "Alya",  l:4.0,  w:5.0,  h:2.5, office:true,  nx:5,  ny:8,  dial_e:464.0, dial_u0:0.84 },
        Row { lum: "Alya",  l:8.0,  w:14.0, h:5.0, office:true,  nx:5,  ny:47, dial_e:444.0, dial_u0:0.73 },
        Row { lum: "Acrux", l:8.0,  w:14.0, h:3.0, office:false, nx:5,  ny:6,  dial_e:86.0,  dial_u0:0.77 },
        Row { lum: "Acrux", l:8.0,  w:14.0, h:5.0, office:false, nx:4,  ny:9,  dial_e:73.0,  dial_u0:0.74 },
        Row { lum: "Acrux", l:8.0,  w:14.0, h:5.0, office:true,  nx:3,  ny:59, dial_e:441.0, dial_u0:0.75 },
        Row { lum: "Acrux", l:8.0,  w:14.0, h:7.0, office:true,  nx:10, ny:23, dial_e:452.0, dial_u0:0.74 },
        Row { lum: "Acrux", l:22.0, w:22.0, h:7.0, office:true,  nx:4,  ny:163,dial_e:421.0, dial_u0:0.67 },
    ]
}

fn lamp_flux(ldt: &Eulumdat) -> f64 {
    // Total luminous flux of the luminaire (lm).
    ldt.total_luminous_flux()
}

fn main() {
    let alya = Eulumdat::from_file(format!("{TMP}/Alya.ldt")).expect("Alya.ldt");
    let acrux = Eulumdat::from_file(format!("{TMP}/Acrux.ldt")).expect("Acrux.ldt");
    println!("Alya  flux = {:.0} lm", lamp_flux(&alya));
    println!("Acrux flux = {:.0} lm", lamp_flux(&acrux));
    println!("MF = {MF}, refl 0.7/0.5/0.2, EN 12464-1:2021 (Stockmar grid)\n");

    println!(
        "{:<6} {:>10} {:>5} {:>6} | {:>7} {:>5} | {:>7} {:>5} | {:>6} {:>6}",
        "Lum", "Room", "Type", "n", "DIAL Ē", "U0", "ours Ē", "U0", "Ē%", "ΔU0"
    );
    println!("{}", "-".repeat(86));

    for r in rows() {
        let ldt = if r.lum == "Alya" { &alya } else { &acrux };
        let flux = lamp_flux(ldt) * MF;

        // Place nx*ny luminaires on a regular grid at ceiling height.
        let mut lums = Vec::with_capacity(r.nx * r.ny);
        for i in 0..r.nx {
            for j in 0..r.ny {
                lums.push(Luminaire {
                    pos: Vec3::new(
                        (i as f64 + 0.5) * (r.l / r.nx as f64),
                        (j as f64 + 0.5) * (r.w / r.ny as f64),
                        r.h,
                    ),
                    flux,
                });
            }
        }

        let refl = SurfaceReflectances { ceiling: 0.7, wall: 0.5, floor: 0.2 };
        // Keep patch count sane for the very long rooms: ~10 divisions on the
        // longest axis is enough for the diffuse field.
        let mesh = RoomMesh::new(r.l, r.w, r.h, refl, 10);
        let ff = compute_form_factors(&mesh);
        let direct = direct_illuminance(&mesh, &lums, ldt);
        let res = solve_radiosity(&mesh, &ff, &direct, 200, 1e-5);

        let plane = if r.office { WorkPlane::Office } else { WorkPlane::Corridor };
        let s = workplane_stats_normative(&mesh, &res, &lums, ldt, plane, EvaluationStandard::En12464_2021);

        let e_pct = 100.0 * s.e_avg / r.dial_e;
        let n = r.nx * r.ny;
        let typ = if r.office { "Büro" } else { "Flur" };
        println!(
            "{:<6} {:>10} {:>5} {:>6} | {:>7.0} {:>5.2} | {:>7.0} {:>5.2} | {:>5.0}% {:>+6.2}",
            r.lum,
            format!("{}x{}x{}", r.l, r.w, r.h),
            typ,
            n,
            r.dial_e,
            r.dial_u0,
            s.e_avg,
            s.u0,
            e_pct,
            s.u0 - r.dial_u0,
        );
    }
    println!("\nĒ% = ours/DIAL Prüfung ·100  (100% = parity).  ΔU0 = ours − DIAL.");
}
