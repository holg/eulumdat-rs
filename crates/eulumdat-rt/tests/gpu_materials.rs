//! GPU material tests — verify each material type against CPU reference.

use eulumdat_rt::*;

fn gpu_throughput(material: &eulumdat_goniosim::MaterialParams, num_photons: u32) -> f64 {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();
    let gpu_mat = GpuMaterial::from_material_params(material);
    let gpu_prim = GpuPrimitive::sheet(
        [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
        0.5, 0.5, material.thickness_mm as f32 / 1000.0, 0,
    );
    let result = pollster::block_on(tracer.trace_with_scene(
        num_photons, 10.0, 5.0, SourceType::Isotropic, 1000.0,
        &[gpu_prim], &[gpu_mat],
    ));
    result.total_energy() / num_photons as f64
}

fn cpu_throughput(material: &eulumdat_goniosim::MaterialParams, num_photons: u64) -> f64 {
    use eulumdat_goniosim::*;
    let mut scene = Scene::new();
    scene.add_source(Source::Isotropic {
        position: nalgebra::Point3::origin(),
        flux_lm: 1000.0,
    });
    let mat_id = scene.add_material(material.clone());
    scene.add_object(
        Primitive::Sheet {
            center: nalgebra::Point3::new(0.0, 0.0, -0.04),
            normal: nalgebra::Vector3::z_axis(),
            u_axis: nalgebra::Vector3::x_axis(),
            half_width: 0.5, half_height: 0.5,
            thickness: material.thickness_mm / 1000.0,
        },
        mat_id, "cover",
    );
    let config = TracerConfig {
        num_photons,
        detector_c_resolution: 10.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..TracerConfig::default()
    };
    let result = Tracer::trace(&scene, &config);
    result.stats.total_energy_detected / result.stats.total_energy_emitted
}

#[test]
fn clear_pmma_gpu_vs_cpu() {
    let mat = eulumdat_goniosim::catalog::clear_pmma_3mm();
    let gpu = gpu_throughput(&mat, 500_000);
    let cpu = cpu_throughput(&mat, 500_000);
    eprintln!("Clear PMMA: GPU={:.3}, CPU={:.3}, ratio={:.3}", gpu, cpu, gpu / cpu);
    assert!((gpu / cpu - 1.0).abs() < 0.15, "GPU/CPU ratio too far: {:.3}", gpu / cpu);
}

#[test]
fn satin_pmma_gpu_vs_cpu() {
    let mat = eulumdat_goniosim::catalog::satin_pmma_3mm();
    let gpu = gpu_throughput(&mat, 500_000);
    let cpu = cpu_throughput(&mat, 500_000);
    eprintln!("Satin PMMA: GPU={:.3}, CPU={:.3}, ratio={:.3}", gpu, cpu, gpu / cpu);
    assert!((gpu / cpu - 1.0).abs() < 0.15, "GPU/CPU ratio too far: {:.3}", gpu / cpu);
}

#[test]
fn opal_pmma_gpu_vs_cpu() {
    let mat = eulumdat_goniosim::catalog::opal_pmma_3mm();
    let gpu = gpu_throughput(&mat, 500_000);
    let cpu = cpu_throughput(&mat, 500_000);
    eprintln!("Opal PMMA: GPU={:.3}, CPU={:.3}, ratio={:.3}", gpu, cpu, gpu / cpu);
    assert!((gpu / cpu - 1.0).abs() < 0.15, "GPU/CPU ratio too far: {:.3}", gpu / cpu);
}

#[test]
fn clear_glass_gpu_vs_cpu() {
    let mat = eulumdat_goniosim::catalog::clear_glass_4mm();
    let gpu = gpu_throughput(&mat, 500_000);
    let cpu = cpu_throughput(&mat, 500_000);
    eprintln!("Clear glass: GPU={:.3}, CPU={:.3}, ratio={:.3}", gpu, cpu, gpu / cpu);
    assert!((gpu / cpu - 1.0).abs() < 0.15, "GPU/CPU ratio too far: {:.3}", gpu / cpu);
}

#[test]
fn matte_black_gpu_vs_cpu() {
    let mat = eulumdat_goniosim::catalog::matte_black();
    let gpu = gpu_throughput(&mat, 500_000);
    let cpu = cpu_throughput(&mat, 500_000);
    eprintln!("Matte black: GPU={:.3}, CPU={:.3}, ratio={:.3}", gpu, cpu, gpu / cpu);
    // Matte black: 5% reflectance. ~50% of isotropic photons miss the cover
    // (go upward), so throughput ≈ 0.5 + 0.5*0.05 = 0.525
    assert!(gpu < 0.70, "Matte black throughput too high: GPU={:.3}", gpu);
    assert!((gpu / cpu - 1.0).abs() < 0.15, "GPU/CPU ratio too far: {:.3}", gpu / cpu);
}

/// Cover always reduces throughput (never increases).
#[test]
fn cover_always_reduces() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();
    let covers = [
        ("Clear PMMA", eulumdat_goniosim::catalog::clear_pmma_3mm()),
        ("Satin PMMA", eulumdat_goniosim::catalog::satin_pmma_3mm()),
        ("Opal PMMA", eulumdat_goniosim::catalog::opal_pmma_3mm()),
        ("Clear glass", eulumdat_goniosim::catalog::clear_glass_4mm()),
        ("Matte black", eulumdat_goniosim::catalog::matte_black()),
    ];

    for (name, mat) in &covers {
        let gpu_mat = GpuMaterial::from_material_params(mat);
        let gpu_prim = GpuPrimitive::sheet(
            [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
            0.5, 0.5, mat.thickness_mm as f32 / 1000.0, 0,
        );
        let result = pollster::block_on(tracer.trace_with_scene(
            200_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
            &[gpu_prim], &[gpu_mat],
        ));
        let throughput = result.total_energy() / 200_000.0;
        eprintln!("  {name}: throughput={throughput:.3}");
        assert!(throughput < 1.0, "{name}: throughput {throughput:.3} >= 1.0 — cover must reduce light");
    }
}

/// Transmittance directly controls absorption.
#[test]
fn transmittance_controls_absorption() {
    let tracer = pollster::block_on(GpuTracer::new()).unwrap();

    let mut prev_throughput = 1.0f64;
    for &trans in &[90.0, 50.0, 20.0, 5.0] {
        let mat = eulumdat_goniosim::MaterialParams {
            name: format!("Custom {trans}%"),
            reflectance_pct: 4.0,
            ior: 1.49,
            transmittance_pct: trans,
            thickness_mm: 3.0,
            diffusion_pct: 50.0,
        };
        let gpu_mat = GpuMaterial::from_material_params(&mat);
        let gpu_prim = GpuPrimitive::sheet(
            [0.0, 0.0, -0.04], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0],
            0.5, 0.5, 0.003, 0,
        );
        let result = pollster::block_on(tracer.trace_with_scene(
            200_000, 10.0, 5.0, SourceType::Isotropic, 1000.0,
            &[gpu_prim], &[gpu_mat],
        ));
        let throughput = result.total_energy() / 200_000.0;
        eprintln!("  Trans={trans}%: throughput={throughput:.3}");
        assert!(
            throughput <= prev_throughput + 0.05,
            "Lower transmittance should reduce throughput: {trans}% gave {throughput:.3} > prev {prev_throughput:.3}"
        );
        prev_throughput = throughput;
    }
}
