//! Headless light-scene render: the same renderer the editor tab uses, on an
//! offscreen texture, written as a PPM. A smoke test for the tracer and a
//! way to make documentation images without a window.
//!
//!     cargo run -p eulumdat-rt --bin rt-scene --release -- file.ldt \
//!         --scene street --mode isolux --samples 200 --size 1280x800 --tilt 15 --out out.ppm
//!
//! In `points` mode the per-grid Em / Emin / Uo table is printed. `--cpu`
//! runs the Rust tracer instead of the GPU (use a small `--size`), which is
//! the cross-check between the two paths.

use eulumdat_rt::lightscene::SceneRenderer;
use eulumdat_scene::cpu::CpuTracer;
use eulumdat_scene::sync::scene_from_id;
use eulumdat_scene::viewer::{parse_any, Settings, ViewMode, Viewer};

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let file = args.get(1).filter(|a| !a.starts_with("--")).cloned();
    let samples: u32 = arg(&args, "--samples")
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let out = arg(&args, "--out").unwrap_or_else(|| "rt_scene.ppm".into());
    let (w, h) = arg(&args, "--size")
        .and_then(|s| {
            let (a, b) = s.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((1280u32, 800u32));

    let mut settings = Settings {
        overlay: false,
        ..Default::default()
    };
    let kind = scene_from_id(&arg(&args, "--scene").unwrap_or_default());
    settings.params = eulumdat_scene::scene::SceneParams::defaults(kind);
    settings.mode = ViewMode::from_id(&arg(&args, "--mode").unwrap_or_default());
    settings.isolines = settings.mode == ViewMode::Isolux;
    if let Some(hz) = arg(&args, "--haze").and_then(|v| v.parse().ok()) {
        settings.haze = hz;
    }
    if let Some(t) = arg(&args, "--tilt").and_then(|v| v.parse().ok()) {
        settings.params.tilt = t;
    }
    let mut viewer = Viewer::new(settings);
    // `--l3d body.l3d` draws the real luminaire body; a `.gldf` as the
    // luminaire file brings its own photometry and (first) L3D geometry.
    let mut l3d_bytes = arg(&args, "--l3d").map(|p| std::fs::read(p).expect("read l3d"));
    match file {
        Some(path) if path.to_ascii_lowercase().ends_with(".gldf") => {
            let bytes = std::fs::read(&path).expect("read gldf");
            let x = eulumdat_scene::l3d::gldf_extract(&bytes).expect("no photometry in gldf");
            let phot = x.best_photometry().expect("no photometry in gldf");
            let ldt = parse_any(&phot.name, &phot.text).expect("parse photometry");
            println!(
                "gldf photometry: {} (of {})",
                phot.name,
                x.photometries.len()
            );
            viewer.load(&path, ldt, None);
            if l3d_bytes.is_none() {
                l3d_bytes = x.l3d;
            }
        }
        Some(path) => {
            let text = std::fs::read_to_string(&path).expect("read luminaire file");
            let ldt = parse_any(&path, &text).expect("parse luminaire file");
            viewer.load(&path, ldt, None);
        }
        None => viewer.load_sample(0),
    }
    if let Some(bytes) = l3d_bytes {
        match eulumdat_scene::l3d::mesh_source_from_l3d(&bytes)
            .and_then(|src| eulumdat_scene::mesh::LuminaireMesh::from_source(&src))
        {
            Some(mesh) => {
                println!(
                    "l3d body: {} triangles, {} nodes, bounds {:.3} .. {:.3} m, face {:?}",
                    mesh.tris.len(),
                    mesh.nodes.len(),
                    mesh.bmin,
                    mesh.bmax,
                    mesh.face_half
                );
                viewer.set_mesh(Some(std::sync::Arc::new(mesh)));
            }
            None => println!("l3d body: no usable geometry"),
        }
    }
    viewer.max_pixels = f32::MAX;
    viewer.resize(w as f32, h as f32, 1.0);

    if args.iter().any(|a| a == "--cpu") {
        let mut tracer = CpuTracer::new();
        let per_pass = (w * h) as usize;
        let mut rounds = 0u32;
        loop {
            viewer.update(1.0 / 60.0);
            let input = viewer.frame_input();
            let table = &viewer.photometry.as_ref().expect("luminaire").table;
            tracer.trace_pass(&input, table, per_pass);
            if viewer.settings.mode == ViewMode::Points {
                tracer.probe_pass(&input, table, 16);
                viewer.probe_values = tracer.probe_values().to_vec();
            }
            viewer.samples = tracer.samples();
            viewer.probe_samples = tracer.probe_samples();
            rounds += 1;
            let points_done = viewer.settings.mode != ViewMode::Points
                || tracer.probe_samples() >= samples.min(input.probe_max_samples);
            if tracer.samples() >= samples && points_done || rounds > samples * 4 {
                break;
            }
        }
        println!(
            "cpu: {} samples, {} probe samples, scale {:.0} lx, exposure key {:.0} lx",
            tracer.samples(),
            tracer.probe_samples(),
            viewer.lux_max,
            eulumdat_scene::photometry::camera_key_lux(&viewer.scene)
        );
        for g in viewer.grid_statistics() {
            println!(
                "{:<18} Em {:>7.1}  Emin {:>7.1}  Uo {:.2}",
                g.name, g.em, g.emin, g.uo
            );
        }
        let rgba = tracer.present(&viewer.frame_input().params);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in rgba.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        std::fs::write(&out, ppm).expect("write ppm");
        println!("wrote {out}");
        return;
    }

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("GPU adapter");
    println!("adapter: {}", adapter.get_info().name);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("rt-scene"),
        ..Default::default()
    }))
    .expect("GPU device");

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut renderer = SceneRenderer::new(&device, format);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rt-scene target"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());

    let mut frames = 0u32;
    while frames < samples + 40 {
        viewer.update(1.0 / 60.0);
        let table = viewer.photometry.as_ref().map(|p| &p.table);
        let input = viewer.frame_input();
        renderer.render(&device, &queue, &input, table, &target_view);
        if let Some(values) = renderer.take_probe_values(&device) {
            viewer.probe_values = values;
        }
        viewer.samples = renderer.samples();
        viewer.probe_samples = renderer.probe_samples();
        frames += 1;
        if renderer.samples() >= samples && viewer.settings.mode != ViewMode::Points {
            break;
        }
    }
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    println!(
        "{} samples, {} probe samples, scale {:.0} lx, exposure key {:.0} lx",
        renderer.samples(),
        renderer.probe_samples(),
        viewer.lux_max,
        eulumdat_scene::photometry::camera_key_lux(&viewer.scene)
    );
    for g in viewer.grid_statistics() {
        println!(
            "{:<18} Em {:>7.1}  Emin {:>7.1}  Uo {:.2}",
            g.name, g.em, g.emin, g.uo
        );
    }

    // Read the target back and write a PPM.
    let bytes_per_row = (w * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rt-scene readback"),
        size: (bytes_per_row * h) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (tx, rx) = flume::bounded(1);
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().expect("map").expect("map ok");
    let data = slice.get_mapped_range();
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        let row = &data[(y * bytes_per_row) as usize..];
        for x in 0..w {
            let i = (x * 4) as usize;
            ppm.extend_from_slice(&row[i..i + 3]);
        }
    }
    drop(data);
    readback.unmap();
    std::fs::write(&out, ppm).expect("write ppm");
    println!("wrote {out}");
}
