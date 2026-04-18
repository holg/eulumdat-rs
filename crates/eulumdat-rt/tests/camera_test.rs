//! Camera rendering test — produces an actual image.

use eulumdat_rt::*;

#[test]
fn render_opal_cover_scene() {
    let camera = pollster::block_on(GpuCamera::new()).expect("GPU camera");

    // Scene: opal PMMA cover at z=-0.04
    let cover = eulumdat_goniosim::catalog::opal_pmma_3mm();
    let gpu_mat = GpuMaterial::from_material_params(&cover);
    let gpu_prim = GpuPrimitive::sheet(
        [0.0, -0.04, 0.0], // center (Bevy Y-up)
        [0.0, 1.0, 0.0],   // normal +Y (facing up)
        [1.0, 0.0, 0.0],   // u_axis
        0.3,
        0.3,   // size
        0.003, // thickness
        0,
    );

    let image = pollster::block_on(camera.render(
        256,
        256,             // resolution
        4,               // samples per pixel
        [0.5, 0.3, 0.5], // camera position
        [0.0, 0.0, 0.0], // look at origin
        60.0,            // FOV degrees
        &[gpu_prim],
        &[gpu_mat],
        100.0,           // source intensity
        [0.0, 0.0, 0.0], // source at origin
    ));

    assert_eq!(image.width, 256);
    assert_eq!(image.height, 256);
    assert_eq!(image.pixels.len(), 256 * 256);

    // Check that we got some non-zero pixels
    let non_zero = image
        .pixels
        .iter()
        .filter(|p| p[0] > 0.001 || p[1] > 0.001 || p[2] > 0.001)
        .count();
    eprintln!(
        "Non-zero pixels: {} / {} ({:.1}%)",
        non_zero,
        image.pixels.len(),
        non_zero as f64 / image.pixels.len() as f64 * 100.0
    );
    assert!(non_zero > 100, "Should have some visible pixels");

    // Save to PPM (simple format, no external deps)
    let bytes = image.to_srgb_bytes();
    eprintln!(
        "Image rendered: {}x{}, {} bytes",
        image.width,
        image.height,
        bytes.len()
    );

    // Write PPM file for visual inspection
    let mut ppm = format!("P6\n{} {}\n255\n", image.width, image.height);
    let ppm_bytes: Vec<u8> = bytes
        .chunks(4)
        .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect();
    let out_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let out_path = format!("{out_dir}/../../tmp/camera_test.ppm");
    std::fs::write(&out_path, {
        let mut data = ppm.into_bytes();
        data.extend_from_slice(&ppm_bytes);
        data
    })
    .ok(); // Don't fail if tmp doesn't exist
    eprintln!("Wrote {out_path}");
}

#[test]
fn render_empty_scene() {
    let camera = pollster::block_on(GpuCamera::new()).expect("GPU camera");

    let image = pollster::block_on(camera.render(
        64,
        64,
        1,
        [0.0, 0.0, 2.0],
        [0.0, 0.0, 0.0],
        60.0,
        &[],
        &[],
        0.0,
        [0.0, 0.0, 0.0],
    ));

    assert_eq!(image.pixels.len(), 64 * 64);
    // Empty scene = sky gradient, should have non-zero blue-ish pixels
    let has_color = image.pixels.iter().any(|p| p[2] > 0.01);
    eprintln!("Empty scene has sky: {has_color}");
    assert!(has_color, "Empty scene should show sky gradient");
}
