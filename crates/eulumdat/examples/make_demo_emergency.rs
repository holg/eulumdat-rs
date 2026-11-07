//! Writes the synthetic demo emergency luminaires (our own numbers, safe to
//! commit and to show): an escape-route optic that throws along C0 / C180
//! and a wide anti-panic optic.
//!
//! ```text
//! cargo run -p eulumdat --example make_demo_emergency -- crates/eulumdat/tests/fixtures/emergency
//! ```

use eulumdat::{Eulumdat, LampSet, PhotometricCalculations, Symmetry, TypeIndicator};

struct Spec {
    file: &'static str,
    name: &'static str,
    number: &'static str,
    flux: f64,
    watts: f64,
    symmetry: Symmetry,
    /// relative intensity at (C, γ) in degrees
    shape: fn(f64, f64) -> f64,
}

/// Escape-route optic: a lobe at γ ≈ 62° along C0 / C180, narrow across.
fn route(c: f64, g: f64) -> f64 {
    if g >= 90.0 {
        return 0.0;
    }
    let along = c.to_radians().cos().abs().powi(8);
    let lobe = (-((g - 62.0) / 13.0).powi(2)).exp();
    let down = g.to_radians().cos().powf(2.0);
    3.2 * lobe * along + down
}

/// Anti-panic optic: wide batwing, rotationally symmetric.
fn open(_c: f64, g: f64) -> f64 {
    if g >= 90.0 {
        return 0.0;
    }
    let (s, co) = (g.to_radians().sin(), g.to_radians().cos());
    co.powf(0.35) * (1.0 + 2.2 * s.powi(4))
}

fn build(s: &Spec) -> Eulumdat {
    let c_angles: Vec<f64> = (0..24).map(|k| k as f64 * 15.0).collect();
    let g_angles: Vec<f64> = (0..=36).map(|k| k as f64 * 5.0).collect();
    let rows: Vec<f64> = match s.symmetry {
        Symmetry::VerticalAxis => vec![0.0],
        _ => c_angles.clone(),
    };
    let intensities: Vec<Vec<f64>> = rows
        .iter()
        .map(|&c| g_angles.iter().map(|&g| (s.shape)(c, g)).collect())
        .collect();
    let mut l = Eulumdat {
        identification: "Candelabra demo (synthetic, not a product)".into(),
        type_indicator: if s.symmetry == Symmetry::VerticalAxis {
            TypeIndicator::PointSourceSymmetric
        } else {
            TypeIndicator::PointSourceOther
        },
        symmetry: s.symmetry,
        num_c_planes: 24,
        c_plane_distance: 15.0,
        num_g_planes: 37,
        g_plane_distance: 5.0,
        measurement_report_number: "synthetic".into(),
        luminaire_name: s.name.into(),
        luminaire_number: s.number.into(),
        file_name: s.file.into(),
        date_user: "2026-11-07 / Candelabra".into(),
        length: 120.0,
        width: 120.0,
        height: 40.0,
        luminous_area_length: 60.0,
        luminous_area_width: 60.0,
        downward_flux_fraction: 100.0,
        light_output_ratio: 100.0,
        conversion_factor: 1.0,
        lamp_sets: vec![LampSet {
            // absolute photometry: the flux is the luminaire's
            num_lamps: -1,
            lamp_type: "LED, emergency mode".into(),
            total_luminous_flux: s.flux,
            color_appearance: "4000K".into(),
            color_rendering_group: "1B".into(),
            wattage_with_ballast: s.watts,
        }],
        c_angles,
        g_angles,
        intensities,
        ..Default::default()
    };
    // scale the table so it carries exactly 1000 lm per 1000 lm (LOR 100 %)
    let per_klm = PhotometricCalculations::calculated_luminous_flux(&l);
    let k = 1000.0 / per_klm;
    for row in &mut l.intensities {
        for v in row.iter_mut() {
            *v = (*v * k * 10.0).round() / 10.0;
        }
    }
    l.direct_ratios = PhotometricCalculations::calculate_direct_ratios(&l, "1.25");
    l
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "crates/eulumdat/tests/fixtures/emergency".into());
    std::fs::create_dir_all(&dir).unwrap();
    let specs = [
        Spec {
            file: "demo_em_route.ldt",
            name: "Demo emergency luminaire, escape-route optic",
            number: "DEMO-EM-ROUTE",
            flux: 180.0,
            watts: 2.5,
            symmetry: Symmetry::None,
            shape: route,
        },
        Spec {
            file: "demo_em_open.ldt",
            name: "Demo emergency luminaire, anti-panic optic",
            number: "DEMO-EM-OPEN",
            flux: 220.0,
            watts: 2.5,
            symmetry: Symmetry::VerticalAxis,
            shape: open,
        },
    ];
    for s in &specs {
        let l = build(s);
        let text = l.to_ldt();
        let back = Eulumdat::parse(&text).expect("reads back");
        for (c, g) in [
            (0.0, 0.0),
            (0.0, 60.0),
            (90.0, 60.0),
            (45.0, 30.0),
            (180.0, 65.0),
        ] {
            let (a, b) = (l.sample(c, g), back.sample(c, g));
            assert!((a - b).abs() < 0.11, "{} at C{c} γ{g}: {a} vs {b}", s.file);
        }
        let flux = eulumdat::emergency::luminaire_flux_lm(&back);
        assert!(
            (flux - s.flux).abs() / s.flux < 0.01,
            "{}: {flux} lm",
            s.file
        );
        let path = format!("{dir}/{}", s.file);
        std::fs::write(&path, text).unwrap();
        println!(
            "{path}: {:.0} lm, I max {:.0} cd/klm",
            flux,
            back.max_intensity()
        );
    }
}
