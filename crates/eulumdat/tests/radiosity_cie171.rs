//! CIE 171:2006 — solver-physics validation (separate from EN 12464-1 evaluation).
//!
//! These gate the radiosity *engine*, not the DIALux reporting. They use the
//! analytical interreflection results the standard's diffuse test cases are
//! built on, which we can derive exactly (so no copyrighted table values are
//! reproduced):
//!
//! **Integrating-cavity (Sumpner) result.** For a *closed* room of total
//! surface area `A` and uniform reflectance ρ, where the luminaires deposit a
//! total direct flux Φ onto the surfaces, the steady-state average illuminance
//! over all surfaces is
//!
//! ```text
//!   E_avg_total = (Φ / A) · 1 / (1 − ρ)
//!   E_avg_indirect = (Φ / A) · ρ / (1 − ρ)
//! ```
//!
//! This is exact for the surface average regardless of geometry, because each
//! unit of flux bounces with mean reflectance ρ, summing the geometric series
//! ρ + ρ² + … = ρ/(1−ρ). It is the basis of CIE 171's diffuse-interreflection
//! cases (canonical scene: 4×4×4 m cubic room, isotropic point source).
//!
//! Source for the test-case geometry: CIE 171:2006 diffuse-interreflection
//! cases as described in the public validation literature (Maamari et al.;
//! Mangkuto, "Validation of DIALux 4.12 and evo 4.1 against CIE 171:2006",
//! LEUKOS 2016). The closed-form above is the standard integrating-cavity
//! relation, not a reproduced table.

use eulumdat::{
    compute_form_factors, solve_radiosity, surface_average, RoomMesh, Surface, SurfaceReflectances,
};

/// Directly seed a uniform direct illuminance on every patch (bypassing the LDT
/// photometry), so the test isolates the interreflection solver. `phi` lumens
/// spread over total area `a` gives uniform direct E = phi/a.
fn uniform_direct(mesh: &RoomMesh, phi: f64) -> Vec<f64> {
    let a = mesh.total_area();
    vec![phi / a; mesh.patches.len()]
}

#[test]
fn integrating_cavity_average_matches_analytic() {
    // CIE-style cubic room, uniform diffuse surfaces.
    let side = 4.0;
    let phi = 10_000.0; // lm deposited on surfaces
    let a = 6.0 * side * side; // 96 m²

    for &rho in &[0.2_f64, 0.5, 0.8] {
        let refl = SurfaceReflectances {
            ceiling: rho,
            wall: rho,
            floor: rho,
        };
        let mesh = RoomMesh::new(side, side, side, refl, 12);
        let direct = uniform_direct(&mesh, phi);
        let ff = compute_form_factors(&mesh);
        let res = solve_radiosity(&mesh, &ff, &direct, 1000, 1e-8);

        // Average total illuminance over ALL surfaces.
        let mut flux = 0.0;
        let mut area = 0.0;
        for (p, &e) in mesh.patches.iter().zip(&res.incident) {
            flux += e * p.area;
            area += p.area;
        }
        let e_avg = flux / area;

        let analytic = (phi / a) / (1.0 - rho);
        let rel = (e_avg - analytic).abs() / analytic;
        assert!(
            rel < 0.02,
            "ρ={rho}: solver avg {e_avg:.2}, analytic {analytic:.2} (rel {rel:.4})"
        );
    }
}

#[test]
fn zero_reflectance_gives_direct_only() {
    // ρ=0 ⇒ no interreflection ⇒ surface average equals the direct seed exactly.
    let side = 4.0;
    let phi = 10_000.0;
    let refl = SurfaceReflectances {
        ceiling: 0.0,
        wall: 0.0,
        floor: 0.0,
    };
    let mesh = RoomMesh::new(side, side, side, refl, 10);
    let direct = uniform_direct(&mesh, phi);
    let ff = compute_form_factors(&mesh);
    let res = solve_radiosity(&mesh, &ff, &direct, 50, 1e-9);

    let e_floor = surface_average(&mesh, &res.incident, Surface::Floor);
    let expected = phi / mesh.total_area();
    assert!((e_floor - expected).abs() / expected < 1e-6);
}

#[test]
fn indirect_grows_with_reflectance_per_geometric_series() {
    // The indirect/direct ratio must follow ρ/(1−ρ), the integrating-cavity law.
    let side = 4.0;
    let phi = 10_000.0;
    let a = 6.0 * side * side;
    let direct_level = phi / a;

    for &rho in &[0.3_f64, 0.6, 0.85] {
        let refl = SurfaceReflectances {
            ceiling: rho,
            wall: rho,
            floor: rho,
        };
        let mesh = RoomMesh::new(side, side, side, refl, 12);
        let direct = uniform_direct(&mesh, phi);
        let ff = compute_form_factors(&mesh);
        let res = solve_radiosity(&mesh, &ff, &direct, 1000, 1e-8);

        let mut flux = 0.0;
        let mut area = 0.0;
        for (p, &e) in mesh.patches.iter().zip(&res.incident) {
            flux += e * p.area;
            area += p.area;
        }
        let e_avg = flux / area;
        let indirect = e_avg - direct_level;
        let ratio = indirect / direct_level;
        let analytic_ratio = rho / (1.0 - rho);
        let rel = (ratio - analytic_ratio).abs() / analytic_ratio;
        assert!(
            rel < 0.03,
            "ρ={rho}: indirect/direct {ratio:.3}, analytic {analytic_ratio:.3} (rel {rel:.4})"
        );
    }
}
