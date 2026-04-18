//! Photometric raytracer demo — uses eulumdat-rt GpuCamera directly.
//!
//! Renders a room scene with LDT luminaire, saves to PPM and opens.

use eulumdat::Eulumdat;
use eulumdat_bevy_rt::scene::types::LightProfile;
use eulumdat_goniosim::catalog;
use eulumdat_rt::{GpuCamera, GpuMaterial, GpuPrimitive};

fn main() {
    // Load LDT file
    let ldt_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            let base = env!("CARGO_MANIFEST_DIR");
            format!("{base}/../eulumdat-egui/assets/templates/road_luminaire.ldt")
        });

    let ldt = Eulumdat::from_file(&ldt_path).expect("Failed to load LDT");
    println!(
        "Loaded LDT: {} — {} lm, {} C x {} G",
        &ldt.luminaire_name,
        ldt.total_luminous_flux() as u32,
        ldt.c_angles.len(),
        ldt.g_angles.len(),
    );

    let profile = LightProfile::from_eulumdat(&ldt);

    // Materials
    let floor_mat = GpuMaterial::from_material_params(&catalog::white_paint());
    let wall_mat = GpuMaterial::from_material_params(&catalog::white_paint());
    let cover_mat = GpuMaterial::from_material_params(&catalog::opal_pmma_3mm());
    let materials = vec![floor_mat, wall_mat, cover_mat];

    // Room: floor, ceiling, back wall, left wall, opal cover
    let room_h: f32 = 1.5;
    let primitives = vec![
        GpuPrimitive::sheet([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], 2.0, 2.0, 0.0, 0),
        GpuPrimitive::sheet([0.0, room_h, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], 2.0, 2.0, 0.0, 1),
        GpuPrimitive::sheet([0.0, room_h/2.0, -2.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 2.0, room_h/2.0, 0.0, 1),
        GpuPrimitive::sheet([-2.0, room_h/2.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 2.0, room_h/2.0, 0.0, 1),
        GpuPrimitive::sheet([0.0, room_h-0.04, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0], 0.2, 0.2, 0.003, 2),
    ];

    let width = 640u32;
    let height = 480u32;
    let spp = 32u32;

    println!("Rendering {}x{} @ {} spp with LDT light pattern...", width, height, spp);

    let mut image = pollster::block_on(async {
        let gpu = GpuCamera::new().await.expect("GPU camera init failed");
        gpu.render_with_lvk(
            width, height, spp,
            [1.8, 0.8, 2.2],
            [0.0, 0.5, 0.0],
            55.0,
            &primitives,
            &materials,
            500.0,
            [0.0, room_h - 0.04, 0.0],
            &profile.lvk_data,
            profile.cdf_c_steps,
            profile.cdf_g_steps,
            profile.cdf_g_max,
            profile.lvk_max_intensity,
        ).await
    });

    // Denoise
    image.denoise(3);

    // Save as PPM
    let output = "/tmp/eulumdat_rt_demo.ppm";
    let bytes = image.to_srgb_bytes_with_exposure(2.0);
    let mut ppm = format!("P6\n{} {}\n255\n", width, height);
    let mut ppm_bytes = ppm.into_bytes();
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            ppm_bytes.push(bytes[i]);
            ppm_bytes.push(bytes[i + 1]);
            ppm_bytes.push(bytes[i + 2]);
        }
    }
    std::fs::write(output, &ppm_bytes).expect("Failed to write PPM");
    println!("Saved to {}", output);

    // Open on macOS
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(output).spawn();
    }

    println!("Done! LDT distribution rendered with full path tracing.");
}
