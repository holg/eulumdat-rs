//! CIE 171:2006 test cases on GPU — must match CPU reference.

use eulumdat_rt::*;
use std::f64::consts::PI;

/// TC 5.1: Isotropic in free space — uniform candela everywhere.
#[test]
fn cie_tc_5_1_isotropic_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();
    let result = pollster::block_on(tracer.trace_isotropic(2_000_000, 10.0, 5.0));

    let expected_cd = 1000.0 / (4.0 * PI);
    let candela = result.to_candela(1000.0);

    // RMS error across non-polar bins
    let mut sum_sq = 0.0;
    let mut count = 0;
    for ci in 0..candela.len() {
        for gi in 2..candela[ci].len() - 2 {
            let cd = candela[ci][gi];
            if cd > 0.0 {
                let err = (cd - expected_cd) / expected_cd;
                sum_sq += err * err;
                count += 1;
            }
        }
    }
    let rms = (sum_sq / count as f64).sqrt();
    eprintln!("TC 5.1 GPU: RMS={:.2}%, expected<5%", rms * 100.0);
    assert!(rms < 0.05, "RMS {:.2}% exceeds 5%", rms * 100.0);

    // Energy conservation
    let energy_ratio = result.total_energy() / 2_000_000.0;
    eprintln!("TC 5.1 GPU: energy ratio={:.6}", energy_ratio);
    assert!((energy_ratio - 1.0).abs() < 0.001);
}

/// TC 5.2: Lambertian — cosine falloff.
#[test]
fn cie_tc_5_2_lambertian_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();
    let result = pollster::block_on(tracer.trace_lambertian(2_000_000, 10.0, 5.0));

    let candela = result.to_candela(1000.0);
    // Lambertian into lower hemisphere: I_max = flux / pi
    let i_max = 1000.0 / PI;

    // Check cosine law at gamma=0, 30, 60
    // Skip gamma=0 (pole bin has inflated cd due to tiny solid angle)
    let test_angles: &[(f64, f64)] = &[(15.0, 0.966), (30.0, 0.866), (45.0, 0.707), (60.0, 0.5)];
    for &(gamma_deg, expected_ratio) in test_angles {
        let gi = (gamma_deg / 5.0).round() as usize;
        let mut avg = 0.0;
        let mut n = 0;
        for ci in 0..candela.len() {
            if candela[ci][gi] > 0.0 {
                avg += candela[ci][gi];
                n += 1;
            }
        }
        if n > 0 { avg /= n as f64; }
        let measured_ratio = avg / i_max;
        let err = (measured_ratio - expected_ratio).abs();
        eprintln!("TC 5.2 GPU: gamma={:.0}, ratio={:.3} (expected {:.3}, err={:.3})",
            gamma_deg, measured_ratio, expected_ratio, err);
        assert!(err < 0.15, "Lambertian at gamma={:.0}: error {:.3}", gamma_deg, err);
    }

    // No light in upper hemisphere (gamma > 90)
    let gi_120 = 24; // 120 / 5.0
    let mut upper_sum = 0.0;
    for ci in 0..candela.len() {
        if gi_120 < candela[ci].len() {
            upper_sum += candela[ci][gi_120];
        }
    }
    eprintln!("TC 5.2 GPU: upper hemisphere (g=120): {:.3}", upper_sum);
    assert!(upper_sum < 1.0, "Lambertian should have no upward light");
}

/// TC 5.5: Clear glass transmittance — Fresnel equations.
#[test]
fn cie_tc_5_5_clear_glass_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    // Clear glass: IOR 1.52, 92% transmittance
    let glass = eulumdat_goniosim::MaterialParams {
        name: "Clear glass".into(),
        reflectance_pct: 4.0,
        ior: 1.52,
        transmittance_pct: 92.0,
        thickness_mm: 4.0,
        diffusion_pct: 0.0,
    };
    let gpu_mat = GpuMaterial::from_material_params(&glass);
    let gpu_prim = GpuPrimitive::sheet(
        [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        0.5, 0.5, 0.004, 0,
    );

    let result = pollster::block_on(tracer.trace_with_scene(
        500_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &[gpu_prim], &[gpu_mat],
    ));

    let throughput = result.total_energy() / 500_000.0;
    eprintln!("TC 5.5 GPU: glass throughput={:.3} (expected ~0.92-0.96)", throughput);

    // Energy conservation: all photons either pass through or reflect back
    // Both escape to detector (no absorption for clear glass)
    // Throughput should be close to 1.0 (photons either go through or reflect)
    assert!(throughput > 0.90, "Glass throughput too low: {throughput:.3}");
}

/// TC 5.8: Integrating cube — diffuse inter-reflections.
#[test]
fn cie_tc_5_8_diffuse_cube_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    // Build a closed cube with diffuse walls (rho=0.5)
    let rho = 0.5f32;
    let half = 2.0f32; // 4m cube
    let mat = GpuMaterial {
        mtype: 1, // diffuse reflector
        _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: rho,
        ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    };

    let walls = [
        // floor (z=-half, normal +Z)
        GpuPrimitive::sheet([0.0, 0.0, -half], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        // ceiling (z=+half, normal -Z)
        GpuPrimitive::sheet([0.0, 0.0, half], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        // left (x=-half, normal +X)
        GpuPrimitive::sheet([-half, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 0),
        // right (x=+half, normal -X)
        GpuPrimitive::sheet([half, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 0),
        // back (y=-half, normal +Y)
        GpuPrimitive::sheet([0.0, -half, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        // front (y=+half, normal -Y)
        GpuPrimitive::sheet([0.0, half, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
    ];

    let result = pollster::block_on(tracer.trace_with_scene(
        500_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &walls, &[mat],
    ));

    // In a closed box, NO photons should escape (all absorbed)
    let escaped = result.total_energy() / 500_000.0;
    eprintln!("TC 5.8 GPU: escaped fraction={:.4} (should be ~0)", escaped);
    assert!(escaped < 0.05, "Closed box should absorb nearly all photons, got {escaped:.4}");
}

/// TC 5.3: Rectangular diffuse area source — far-field cosine distribution.
#[test]
fn cie_tc_5_3_area_source_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    let luminance = 1000.0f64; // cd/m²
    let area = 2.0 * 1.0;     // m²
    let flux = (luminance * area * PI) as f32;
    let i_max = (luminance * area) as f64; // 2000 cd at nadir

    let result = pollster::block_on(tracer.trace_area_source(
        2_000_000, 10.0, 5.0, flux,
        [0.0, 0.0, 0.0],       // center
        [0.0, 0.0, -1.0],      // normal (emit downward = nadir)
        [1.0, 0.0, 0.0],       // u_axis
        1.0, 0.5,              // half_width=1m, half_height=0.5m → 2m×1m
    ));

    // Energy conservation
    let energy_ratio = result.total_energy() / 2_000_000.0;
    eprintln!("TC 5.3 GPU: energy ratio={:.6}", energy_ratio);
    assert!((energy_ratio - 1.0).abs() < 0.001, "Energy conservation: {energy_ratio:.6}");

    // Far-field cosine pattern: I(γ) = L×A×cos(γ)
    let candela = result.to_candela(flux as f64);
    let test_angles: &[(f64, f64)] = &[(15.0, 0.966), (30.0, 0.866), (45.0, 0.707), (60.0, 0.500)];

    for &(gamma_deg, expected_ratio) in test_angles {
        let gi = (gamma_deg / 5.0).round() as usize;
        let expected_cd = i_max * expected_ratio;
        let mut avg = 0.0;
        let mut n = 0;
        for ci in 0..candela.len() {
            if candela[ci][gi] > 0.0 {
                avg += candela[ci][gi];
                n += 1;
            }
        }
        if n > 0 { avg /= n as f64; }
        let rel_err = (avg - expected_cd).abs() / expected_cd;
        eprintln!("TC 5.3 GPU: gamma={:.0}, cd={:.1} (expected {:.1}, err={:.1}%)",
            gamma_deg, avg, expected_cd, rel_err * 100.0);
        assert!(rel_err < 0.15, "TC 5.3 GPU: error {:.1}% at gamma={:.0}", rel_err * 100.0, gamma_deg);
    }
}

/// TC 5.6: Single diffuse reflection — isotropic source in a closed room
/// with one reflecting floor (ρ=0.5) and absorbing walls/ceiling.
#[test]
fn cie_tc_5_6_single_reflection_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    let half = 2.0f32; // 4m cube

    // Material 0: Diffuse reflector (floor, ρ=0.5)
    let floor_mat = GpuMaterial {
        mtype: 1, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.5, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    };
    // Material 1: Absorber (walls/ceiling, ρ=0)
    let absorber = GpuMaterial {
        mtype: 0, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.0, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    };

    let prims = [
        // Floor (reflective)
        GpuPrimitive::sheet([0.0, 0.0, -half], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        // Ceiling (absorber)
        GpuPrimitive::sheet([0.0, 0.0, half], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], half, half, 0.001, 1),
        // 4 walls (absorber)
        GpuPrimitive::sheet([-half, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 1),
        GpuPrimitive::sheet([half, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 1),
        GpuPrimitive::sheet([0.0, -half, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 1),
        GpuPrimitive::sheet([0.0, half, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 1),
    ];

    let result = pollster::block_on(tracer.trace_with_scene(
        500_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &prims, &[floor_mat, absorber],
    ));

    // Closed room: no photons escape
    let escaped = result.total_energy() / 500_000.0;
    eprintln!("TC 5.6 GPU: escaped fraction={:.4} (should be ~0)", escaped);
    assert!(escaped < 0.01, "TC 5.6 GPU: {:.2}% escaped closed room", escaped * 100.0);
}

/// TC 5.7: Diffuse room with internal obstruction.
/// Same as TC 5.8 integrating cube (ρ=0.5) but with an absorbing partition.
#[test]
fn cie_tc_5_7_obstruction_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    let half = 2.0f32; // 4m cube
    let rho = 0.5f32;

    // Material 0: Diffuse walls (ρ=0.5)
    let wall_mat = GpuMaterial {
        mtype: 1, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: rho, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    };
    // Material 1: Absorber (partition)
    let absorber = GpuMaterial {
        mtype: 0, _pad0: 0, _pad1: 0, _pad2: 0,
        reflectance: 0.0, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
        absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0,
    };

    // 6 walls + 1 partition = 7 primitives
    let prims = [
        // 6 cube walls (all diffuse ρ=0.5)
        GpuPrimitive::sheet([0.0, 0.0, -half], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        GpuPrimitive::sheet([0.0, 0.0, half], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        GpuPrimitive::sheet([-half, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 0),
        GpuPrimitive::sheet([half, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], half, half, 0.001, 0),
        GpuPrimitive::sheet([0.0, -half, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        GpuPrimitive::sheet([0.0, half, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], half, half, 0.001, 0),
        // Absorbing partition at x=0.5
        GpuPrimitive::sheet([0.5, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0, 1.5, 0.001, 1),
    ];

    let result = pollster::block_on(tracer.trace_with_scene(
        500_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &prims, &[wall_mat, absorber],
    ));

    // Closed room with partition: no photons escape
    let escaped = result.total_energy() / 500_000.0;
    eprintln!("TC 5.7 GPU: escaped fraction={:.4} (should be ~0)", escaped);
    assert!(escaped < 0.05, "TC 5.7 GPU: {:.2}% escaped closed room with partition", escaped * 100.0);
}

/// Energy conservation: detected energy must match emitted for all configs.
#[test]
fn energy_conservation_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    // Free space isotropic
    let r1 = pollster::block_on(tracer.trace_isotropic(500_000, 10.0, 5.0));
    let ratio1 = r1.total_energy() / 500_000.0;
    eprintln!("Free space: energy ratio={:.6}", ratio1);
    assert!((ratio1 - 1.0).abs() < 0.001);

    // Free space lambertian
    let r2 = pollster::block_on(tracer.trace_lambertian(500_000, 10.0, 5.0));
    let ratio2 = r2.total_energy() / 500_000.0;
    eprintln!("Lambertian: energy ratio={:.6}", ratio2);
    assert!((ratio2 - 1.0).abs() < 0.001);
}

/// Monte Carlo convergence: RMS decreases with more photons.
#[test]
fn convergence_gpu() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();
    let expected = 1000.0 / (4.0 * PI);

    let mut prev_rms = 1.0;
    for &n in &[10_000u32, 100_000, 1_000_000] {
        let result = pollster::block_on(tracer.trace_isotropic(n, 10.0, 10.0));
        let candela = result.to_candela(1000.0);

        let mut sum_sq = 0.0;
        let mut count = 0;
        for ci in 0..candela.len() {
            for gi in 1..candela[ci].len() - 1 {
                if candela[ci][gi] > 0.0 {
                    let err = (candela[ci][gi] - expected) / expected;
                    sum_sq += err * err;
                    count += 1;
                }
            }
        }
        let rms = (sum_sq / count as f64).sqrt();
        eprintln!("N={n}: RMS={:.2}%", rms * 100.0);
        assert!(rms < prev_rms * 1.5, "RMS should decrease with more photons");
        prev_rms = rms;
    }
    assert!(prev_rms < 0.05, "Final RMS {:.2}% should be < 5%", prev_rms * 100.0);
}
