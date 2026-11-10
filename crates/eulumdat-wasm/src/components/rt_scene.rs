//! Light scene tab: the path-traced viewer on plain wgpu, inside the editor.
//!
//! The editor's current luminaire feeds `eulumdat_scene::viewer::Viewer`
//! directly; `eulumdat_rt::lightscene::SceneRenderer` draws it into the
//! tab's canvas through WebGPU. No separate bundle, no localStorage round
//! trip: settings are signals, the panel reads status straight from the
//! viewer. Measurement points are drawn on a 2D canvas over the picture.
//! Without WebGPU, or on request, the same transport runs on the CPU
//! (`eulumdat_scene::cpu`) at preview size into a 2D canvas.

use eulumdat::Eulumdat;
use eulumdat_rt::lightscene::SceneRenderer;
use eulumdat_scene::cpu::CpuTracer;
use eulumdat_scene::mesh::LuminaireMesh;
use eulumdat_scene::scene::{
    suggest_scene, ObjectId, SceneKind, ScenePlan, GRID_EMPHASIS, GRID_OPEN_AREA, GRID_ROUTE,
    GRID_ROUTE_BAND, GRID_STAIRS, GRID_STAIRS_BAND, MAX_SENSORS,
};
use eulumdat_scene::sync::{scene_id, GridStat, Status, SyncSettings};
use eulumdat_scene::viewer::{ViewMode, Viewer};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use super::skyglow_demo::{detect_browser, render_webgpu_missing};
use crate::i18n::use_locale;

/// GPU side of the tab.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: SceneRenderer,
    /// Set by the device-lost callback; the frame loop then rebuilds the GPU.
    lost: Arc<AtomicBool>,
}

/// Everything the frame loop touches, shared between the loop, the events
/// and the panel.
struct Shared {
    viewer: Viewer,
    gpu: Option<Gpu>,
    canvas: Option<web_sys::HtmlCanvasElement>,
    overlay: Option<web_sys::HtmlCanvasElement>,
    cpu_canvas: Option<web_sys::HtmlCanvasElement>,
    cpu: CpuTracer,
    /// Trace on the CPU (no WebGPU, or chosen in the panel).
    cpu_mode: bool,
    /// Pixels per frame the CPU tracer gets; adapts to the frame time.
    cpu_budget: usize,
    running: bool,
    active: bool,
    last_time: f64,
    /// `performance.now()` of the last frame that ran, for the watchdog.
    last_frame: f64,
    /// A GPU rebuild is under way after a device loss.
    reinit_pending: bool,
    /// Last noteworthy event, shown in the corner of the picture.
    note: String,
    drag: Option<(f32, f32, bool)>,
    /// Pointer drags move objects (luminaires, person, cars, sensors).
    move_mode: bool,
    /// An object is being dragged.
    obj_drag: bool,
    /// GPU resets by the browser ("device lost") so far.
    gpu_resets: u32,
    /// No GPU came back after a reset: the CPU traces.
    gpu_gone: bool,
}

/// Traced pixels at most in CPU mode (preview size).
const CPU_MAX_PIXELS: f32 = 60_000.0;
const GPU_MAX_PIXELS: f32 = 1.7e6;

type SharedRef = Rc<RefCell<Shared>>;
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadState {
    Idle,
    Starting,
    Running,
    Unsupported,
}

/// localStorage key of the last Light scene (a `SceneDoc` as JSON).
const LAST_SCENE_KEY: &str = "eulumdat_light_scene";

fn store_last_scene(json: &str) {
    if let Ok(Some(storage)) = window().local_storage() {
        let _ = storage.set_item(LAST_SCENE_KEY, json);
    }
}

fn load_last_scene() -> Option<eulumdat_scene::doc::SceneDoc> {
    let json = window()
        .local_storage()
        .ok()??
        .get_item(LAST_SCENE_KEY)
        .ok()??;
    serde_json::from_str(&json).ok()
}

/// Wait `ms` milliseconds (browser timer).
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// CSS size of the picture canvas.
fn canvas_css_size(s: &Shared) -> (f32, f32) {
    s.canvas
        .as_ref()
        .map(|c| (c.client_width() as f32, c.client_height() as f32))
        .unwrap_or((800.0, 600.0))
}

fn window() -> web_sys::Window {
    web_sys::window().expect("window")
}

fn has_webgpu() -> bool {
    js_sys::Reflect::get(&window().navigator(), &JsValue::from_str("gpu"))
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

/// The canvas surface target exists only on wasm32; CI lints this crate
/// natively too, so that build gets a stub that never runs.
#[cfg(not(target_arch = "wasm32"))]
async fn init_gpu(_canvas: web_sys::HtmlCanvasElement) -> Result<Gpu, String> {
    Err("WebGPU canvas surfaces exist only in the browser".into())
}

#[cfg(target_arch = "wasm32")]
async fn init_gpu(canvas: web_sys::HtmlCanvasElement) -> Result<Gpu, String> {
    // On the web the default instance is the WebGPU backend.
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| format!("surface: {e}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no WebGPU adapter: {e}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("light scene"),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("device: {e}"))?;
    let w = canvas.width().max(1);
    let h = canvas.height().max(1);
    let mut config = surface
        .get_default_config(&adapter, w, h)
        .ok_or("surface not supported")?;
    config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let srgb = config.format.add_srgb_suffix();
    config.view_formats = vec![srgb];
    surface.configure(&device, &config);
    // wgpu's default handler panics on an uncaptured error, which would kill
    // the frame loop while the rest of the page lives on. Log instead, and
    // note a lost device so the loop can rebuild it.
    device.on_uncaptured_error(Arc::new(|e: wgpu::Error| {
        web_sys::console::error_1(&format!("light scene GPU error: {e}").into());
    }));
    let lost = Arc::new(AtomicBool::new(false));
    let lost2 = lost.clone();
    device.set_device_lost_callback(move |reason, msg| {
        web_sys::console::error_1(&format!("light scene device lost: {reason:?} {msg}").into());
        lost2.store(true, Ordering::Release);
    });
    let renderer = SceneRenderer::new(&device, srgb);
    Ok(Gpu {
        device,
        queue,
        surface,
        config,
        renderer,
        lost,
    })
}

/// Rebuild the GPU after a device loss (or a failed start), once.
fn reinit_gpu(shared: SharedRef) {
    let canvas = {
        let Ok(mut s) = shared.try_borrow_mut() else {
            return;
        };
        if s.reinit_pending {
            return;
        }
        s.reinit_pending = true;
        s.gpu = None;
        s.canvas.clone()
    };
    let Some(canvas) = canvas else {
        return;
    };
    if let Ok(mut s) = shared.try_borrow_mut() {
        s.gpu_resets += 1;
    }
    wasm_bindgen_futures::spawn_local(async move {
        // A browser may refuse a new adapter right after a reset: try again
        // a few times before falling back to the CPU.
        let mut result = Err(String::new());
        for wait_ms in [300, 1500, 4000] {
            sleep_ms(wait_ms).await;
            result = init_gpu(canvas.clone()).await;
            if result.is_ok() {
                break;
            }
        }
        let mut s = shared.borrow_mut();
        s.reinit_pending = false;
        match result {
            Ok(gpu) => {
                s.gpu = Some(gpu);
                s.viewer.reset_token += 1;
                s.viewer.probe_reset_token += 1;
                s.note = "GPU restarted".into();
            }
            Err(e) => {
                web_sys::console::error_1(&format!("light scene: {e}, using the CPU").into());
                s.cpu_mode = true;
                s.gpu_gone = true;
                s.note = String::new();
            }
        }
    });
}

/// One frame: size, update, render, overlay.
fn frame(shared: &SharedRef, now: f64) {
    // Never panic on a borrow collision: skip the frame instead.
    let Ok(mut s) = shared.try_borrow_mut() else {
        return;
    };
    s.last_frame = now;
    if s.gpu
        .as_ref()
        .is_some_and(|g| g.lost.load(Ordering::Acquire))
    {
        drop(s);
        reinit_gpu(shared.clone());
        return;
    }
    let Some(canvas) = s.canvas.clone() else {
        return;
    };
    let dt = if s.last_time > 0.0 {
        ((now - s.last_time) / 1000.0) as f32
    } else {
        1.0 / 60.0
    };
    s.last_time = now;
    s.viewer.note_frame_time(dt * 1000.0);

    let css_w = canvas.client_width() as f32;
    let css_h = canvas.client_height() as f32;
    if css_w < 2.0 || css_h < 2.0 {
        return;
    }
    let dpr = window().device_pixel_ratio() as f32;
    s.viewer.max_pixels = if s.cpu_mode {
        CPU_MAX_PIXELS
    } else {
        GPU_MAX_PIXELS
    };
    let resized = s.viewer.resize(css_w, css_h, dpr);
    let size = s.viewer.size;
    s.viewer.update(dt.min(0.1));

    let input = s.viewer.frame_input();
    if s.cpu_mode {
        cpu_frame(&mut s, &input, dt);
        if let Some(overlay) = s.overlay.clone() {
            draw_overlay(&overlay, &s.viewer, css_w, css_h, dpr, s.move_mode);
        }
        return;
    }
    let Shared { viewer, gpu, .. } = &mut *s;
    let Some(gpu) = gpu.as_mut() else {
        return;
    };
    if resized || canvas.width() != size.x || canvas.height() != size.y {
        canvas.set_width(size.x);
        canvas.set_height(size.y);
        gpu.config.width = size.x;
        gpu.config.height = size.y;
        gpu.surface.configure(&gpu.device, &gpu.config);
    }
    let texture = match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            gpu.surface.configure(&gpu.device, &gpu.config);
            return;
        }
        _ => return,
    };
    let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(gpu.config.view_formats[0]),
        ..Default::default()
    });
    let table = viewer.ies_table();
    gpu.renderer
        .render(&gpu.device, &gpu.queue, &input, table, &view);
    texture.present();
    if let Some(values) = gpu.renderer.take_probe_values(&gpu.device) {
        viewer.probe_values = values;
    }
    viewer.samples = gpu.renderer.samples();
    viewer.probe_samples = gpu.renderer.probe_samples();

    // Sensor areas, object handles and measurement points on the 2D overlay.
    if let Some(overlay) = s.overlay.clone() {
        draw_overlay(&overlay, &s.viewer, css_w, css_h, dpr, s.move_mode);
    }
}

/// One CPU frame: a time-budgeted slice of pixels, a probe pass now and
/// then, and the presented picture onto the 2D preview canvas.
fn cpu_frame(s: &mut Shared, input: &eulumdat_scene::viewer::FrameInput, dt: f32) {
    let Shared {
        viewer,
        cpu,
        cpu_budget,
        cpu_canvas,
        ..
    } = s;
    let Some(table) = viewer.ies_table() else {
        return;
    };
    // Aim for about 30 ms of tracing per frame.
    if dt > 0.05 {
        *cpu_budget = (*cpu_budget * 3 / 4).max(2_000);
    } else if dt < 0.025 {
        *cpu_budget = (*cpu_budget * 5 / 4).min(400_000);
    }
    cpu.trace_pass(input, table, *cpu_budget);
    if input.probe && cpu.frame_count() % 3 == 0 {
        cpu.probe_pass(input, table, 4);
        viewer.probe_values = cpu.probe_values().to_vec();
    }
    viewer.samples = cpu.samples();
    viewer.probe_samples = cpu.probe_samples();
    let Some(canvas) = cpu_canvas.as_ref() else {
        return;
    };
    let size = cpu.size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    if canvas.width() != size.x || canvas.height() != size.y {
        canvas.set_width(size.x);
        canvas.set_height(size.y);
    }
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    let rgba = cpu.present(&input.params);
    if let Ok(image) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&rgba),
        size.x,
        size.y,
    ) {
        let _ = ctx.put_image_data(&image, 0.0, 0.0);
    }
}

fn draw_overlay(
    canvas: &web_sys::HtmlCanvasElement,
    viewer: &Viewer,
    css_w: f32,
    css_h: f32,
    dpr: f32,
    move_mode: bool,
) {
    let w = (css_w * dpr) as u32;
    let h = (css_h * dpr) as u32;
    if canvas.width() != w || canvas.height() != h {
        canvas.set_width(w);
        canvas.set_height(h);
    }
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    ctx.set_transform(dpr as f64, 0.0, 0.0, dpr as f64, 0.0, 0.0)
        .ok();
    ctx.clear_rect(0.0, 0.0, css_w as f64, css_h as f64);
    draw_emergency(&ctx, viewer, css_w, css_h);
    draw_sensors(&ctx, viewer, css_w, css_h, move_mode);
    // The grab points as data (CSS pixels): for automation and tests.
    if move_mode {
        let list: Vec<String> = viewer
            .object_handles(css_w, css_h)
            .iter()
            .map(|h| format!("{:?}:{:.0}:{:.0}:{}", h.id, h.x, h.y, h.active as u8))
            .collect();
        let _ = canvas.set_attribute("data-handles", &list.join(" "));
    } else {
        let _ = canvas.remove_attribute("data-handles");
    }
    if viewer.settings.mode != ViewMode::Points {
        return;
    }
    ctx.set_font("600 10.5px ui-sans-serif, system-ui, sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    ctx.set_line_width(1.0);
    for p in viewer.overlay(css_w, css_h) {
        let [r, g, b] = p.color;
        let fill = format!(
            "rgba({},{},{},0.93)",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8
        );
        ctx.set_fill_style_str(&fill);
        ctx.set_stroke_style_str("rgba(0,0,0,0.7)");
        match &p.text {
            None => {
                ctx.begin_path();
                ctx.arc(
                    p.x as f64,
                    p.y as f64,
                    p.radius as f64,
                    0.0,
                    std::f64::consts::TAU,
                )
                .ok();
                ctx.fill();
                ctx.stroke();
            }
            Some(text) => {
                let half_w = p.radius as f64;
                let half_h = 7.5;
                ctx.begin_path();
                ctx.round_rect_with_f64(
                    p.x as f64 - half_w,
                    p.y as f64 - half_h,
                    2.0 * half_w,
                    2.0 * half_h,
                    7.5,
                )
                .ok();
                ctx.fill();
                ctx.stroke();
                ctx.set_fill_style_str(if p.light_text { "#f5f7fa" } else { "#0d0d10" });
                ctx.fill_text(text, p.x as f64, p.y as f64 + 0.5).ok();
            }
        }
    }
}

/// Trace a list of polygons / polylines into the current path.
fn trace_paths(ctx: &web_sys::CanvasRenderingContext2d, paths: &[Vec<(f32, f32)>], close: bool) {
    for path in paths {
        for (k, (x, y)) in path.iter().enumerate() {
            if k == 0 {
                ctx.move_to(*x as f64, *y as f64);
            } else {
                ctx.line_to(*x as f64, *y as f64);
            }
        }
        if close {
            ctx.close_path();
        }
    }
}

/// The plan's emergency markings: escape routes as a safety-green band
/// with a dashed centre line, the open area's border, and the points of
/// emphasis with their vertical illuminance (green ≥ 5 lx, red below).
fn draw_emergency(
    ctx: &web_sys::CanvasRenderingContext2d,
    viewer: &Viewer,
    css_w: f32,
    css_h: f32,
) {
    let Some(m) = viewer.emergency_marks(css_w, css_h) else {
        return;
    };
    // one path: overlapping segments do not stack up
    ctx.begin_path();
    trace_paths(ctx, &m.bands, true);
    ctx.set_fill_style_str("rgba(0,148,68,0.42)");
    ctx.fill();
    let dash = |a: f64, b: f64| js_sys::Array::of2(&a.into(), &b.into());
    ctx.begin_path();
    trace_paths(ctx, &m.centre_lines, false);
    ctx.set_stroke_style_str("rgba(255,255,255,0.9)");
    ctx.set_line_width(1.5);
    ctx.set_line_dash(&dash(8.0, 6.0)).ok();
    ctx.stroke();
    ctx.begin_path();
    trace_paths(ctx, &m.open_area, false);
    ctx.set_stroke_style_str("rgba(52,211,153,0.85)");
    ctx.set_line_width(1.5);
    ctx.set_line_dash(&dash(3.0, 4.0)).ok();
    ctx.stroke();
    ctx.set_line_dash(&js_sys::Array::new()).ok();
    // the plan's standard (China has no vertical minimum: neutral rings)
    let need = viewer
        .settings
        .params
        .plan
        .as_ref()
        .and_then(|p| p.emergency.as_ref())
        .map_or(5.0, |e| e.standard.vertical_lx);
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    for p in &m.points {
        let (x, y) = (p.x as f64, p.y as f64);
        let ring = match p.lux {
            Some(e) if need <= 0.0 || e >= need => {
                if need > 0.0 {
                    "rgba(74,222,128,0.95)"
                } else {
                    "rgba(226,232,240,0.9)"
                }
            }
            Some(_) => "rgba(248,113,113,0.95)",
            None => "rgba(226,232,240,0.9)",
        };
        ctx.begin_path();
        ctx.arc(x, y, 12.0, 0.0, std::f64::consts::TAU).ok();
        ctx.set_fill_style_str("rgba(10,12,16,0.72)");
        ctx.fill();
        ctx.set_stroke_style_str(ring);
        ctx.set_line_width(2.0);
        ctx.stroke();
        ctx.set_font("13px ui-sans-serif, system-ui, sans-serif");
        ctx.set_fill_style_str("#fff");
        ctx.fill_text(&p.symbol, x, y + 1.0).ok();
        if let Some(e) = p.lux {
            let text = format!("{} lx", fmt_lux(e));
            ctx.set_font("600 10.5px ui-sans-serif, system-ui, sans-serif");
            let w = text.len() as f64 * 6.0 + 8.0;
            ctx.begin_path();
            ctx.round_rect_with_f64(x - w / 2.0, y + 15.0, w, 15.0, 7.5)
                .ok();
            ctx.set_fill_style_str("rgba(10,12,16,0.8)");
            ctx.fill();
            ctx.set_fill_style_str(ring);
            ctx.fill_text(&text, x, y + 23.0).ok();
        }
    }
}

/// Detection areas of the sensors on the ground (green while they detect
/// something) and, in move mode, the grab points of the movable objects.
fn draw_sensors(
    ctx: &web_sys::CanvasRenderingContext2d,
    viewer: &Viewer,
    css_w: f32,
    css_h: f32,
    move_mode: bool,
) {
    // Indoors the walls stop detection: clip the areas to the room.
    let clip = viewer.room_outline_screen(css_w, css_h);
    if let Some(room) = clip.as_ref() {
        ctx.save();
        ctx.begin_path();
        for (k, (x, y)) in room.iter().enumerate() {
            if k == 0 {
                ctx.move_to(*x as f64, *y as f64);
            } else {
                ctx.line_to(*x as f64, *y as f64);
            }
        }
        ctx.close_path();
        ctx.clip();
    }
    // Fill each group (detecting / idle) as one path: overlapping areas do
    // not stack up to a solid colour.
    let outlines = viewer.sensor_outlines(css_w, css_h);
    for detecting in [false, true] {
        let group: Vec<_> = outlines
            .iter()
            .filter(|o| o.detecting == detecting)
            .collect();
        if group.is_empty() {
            continue;
        }
        ctx.begin_path();
        for o in &group {
            for (k, (x, y)) in o.points.iter().enumerate() {
                if k == 0 {
                    ctx.move_to(*x as f64, *y as f64);
                } else {
                    ctx.line_to(*x as f64, *y as f64);
                }
            }
            ctx.close_path();
        }
        let (fill, stroke) = if detecting {
            ("rgba(34,197,94,0.20)", "rgba(74,222,128,0.95)")
        } else {
            ("rgba(124,58,237,0.10)", "rgba(167,139,250,0.85)")
        };
        ctx.set_fill_style_str(fill);
        ctx.fill();
        ctx.set_stroke_style_str(stroke);
        ctx.set_line_width(1.5);
        ctx.set_line_dash(&js_sys::Array::of2(&6.0.into(), &4.0.into()))
            .ok();
        ctx.stroke();
        ctx.set_line_dash(&js_sys::Array::new()).ok();
    }
    if clip.is_some() {
        ctx.restore();
    }
    // The detection volumes in 3D (optional): rings at several heights and
    // lines from each sensor down to the floor.
    if viewer.settings.show_sensor_volume {
        ctx.set_line_width(1.0);
        for v in viewer.sensor_volume_lines(css_w, css_h) {
            ctx.set_stroke_style_str(if v.detecting {
                "rgba(74,222,128,0.75)"
            } else {
                "rgba(167,139,250,0.65)"
            });
            ctx.begin_path();
            for line in &v.lines {
                for (k, (x, y)) in line.iter().enumerate() {
                    if k == 0 {
                        ctx.move_to(*x as f64, *y as f64);
                    } else {
                        ctx.line_to(*x as f64, *y as f64);
                    }
                }
            }
            ctx.stroke();
        }
    }
    if !move_mode {
        return;
    }
    let dragging = viewer.dragging();
    ctx.set_font("13px ui-sans-serif, system-ui, sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    for h in viewer.object_handles(css_w, css_h) {
        let (icon, ring) = match h.id {
            ObjectId::Luminaire(_) => (
                "💡",
                if h.active {
                    "rgba(250,204,21,0.95)"
                } else {
                    "rgba(148,163,184,0.9)"
                },
            ),
            ObjectId::Person => (
                "🚶",
                if h.active {
                    "rgba(74,222,128,0.95)"
                } else {
                    "rgba(226,232,240,0.9)"
                },
            ),
            ObjectId::Car(_) => (
                "🚗",
                if h.active {
                    "rgba(74,222,128,0.95)"
                } else {
                    "rgba(226,232,240,0.9)"
                },
            ),
            ObjectId::Sensor(_) => (
                "📡",
                if h.active {
                    "rgba(74,222,128,0.95)"
                } else {
                    "rgba(167,139,250,0.95)"
                },
            ),
        };
        let r = if dragging == Some(h.id) { 15.0 } else { 12.0 };
        ctx.begin_path();
        ctx.arc(h.x as f64, h.y as f64, r, 0.0, std::f64::consts::TAU)
            .ok();
        ctx.set_fill_style_str("rgba(10,12,16,0.72)");
        ctx.fill();
        ctx.set_stroke_style_str(ring);
        ctx.set_line_width(2.0);
        ctx.stroke();
        ctx.set_fill_style_str("#fff");
        ctx.fill_text(icon, h.x as f64, h.y as f64 + 1.0).ok();
    }
}

/// Keep requesting frames while the tab is active.
fn run_loop(shared: SharedRef) {
    {
        let mut s = shared.borrow_mut();
        if s.running || !s.active {
            return;
        }
        s.running = true;
        s.last_time = 0.0;
    }
    let cb: FrameCallback = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    let shared2 = shared.clone();
    *cb.borrow_mut() = Some(Closure::new(move |now: f64| {
        let active = shared2.borrow().active;
        if !active {
            shared2.borrow_mut().running = false;
            return;
        }
        frame(&shared2, now);
        if let Some(c) = cb2.borrow().as_ref() {
            let _ = window().request_animation_frame(c.as_ref().unchecked_ref());
        }
    }));
    let guard = cb.borrow();
    if let Some(c) = guard.as_ref() {
        let _ = window().request_animation_frame(c.as_ref().unchecked_ref());
    }
}

fn fmt_lux(e: f32) -> String {
    eulumdat_scene::measure::format_lux(e)
}

/// The light scene tab. `active` pauses the tracer while the tab is hidden;
/// `ldc` is the editor's current luminaire.
#[component]
pub fn RtSceneViewer(active: Signal<bool>, ldc: ReadSignal<Eulumdat>) -> impl IntoView {
    let shared: SharedRef = Rc::new(RefCell::new(Shared {
        viewer: Viewer::new(eulumdat_scene::viewer::Settings {
            overlay: false,
            ..Default::default()
        }),
        gpu: None,
        canvas: None,
        overlay: None,
        cpu_canvas: None,
        cpu: CpuTracer::new(),
        cpu_mode: false,
        cpu_budget: 20_000,
        running: false,
        active: false,
        last_time: 0.0,
        last_frame: 0.0,
        reinit_pending: false,
        note: String::new(),
        drag: None,
        move_mode: false,
        obj_drag: false,
        gpu_resets: 0,
        gpu_gone: false,
    }));
    let (note, set_note) = signal(String::new());
    let shared_sv = StoredValue::new_local(shared.clone());
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let overlay_ref = NodeRef::<leptos::html::Canvas>::new();
    let cpu_ref = NodeRef::<leptos::html::Canvas>::new();
    let (webgpu, set_webgpu) = signal(true);
    let (restart, set_restart) = signal(0u32);
    provide_context(set_restart);
    Effect::new(move |_| {
        if restart.get() == 0 {
            return;
        }
        shared_sv.with_value(|sh| {
            let lost = {
                let Ok(mut s) = sh.try_borrow_mut() else {
                    return;
                };
                s.viewer.reset_token += 1;
                s.viewer.probe_reset_token += 1;
                s.running = false;
                s.note = "tracer restarted".into();
                s.gpu.is_none()
                    || s.gpu
                        .as_ref()
                        .is_some_and(|g| g.lost.load(Ordering::Acquire))
            };
            if lost && webgpu.get_untracked() {
                reinit_gpu(sh.clone());
            }
            run_loop(sh.clone());
        });
    });
    let (load_state, set_load_state) = signal(LoadState::Idle);
    // A deep link (`#tab=light-scene&scene=…&mode=…`) sets the first
    // settings and counts as the visitor's own scene choice.
    let (link_settings, link_scene, link_any) = use_context::<super::app::RtLinkParams>()
        .map(|p| {
            let pairs = p.0.get_value();
            let mut s = SyncSettings::default();
            let any = s.apply_query(&pairs);
            (
                if any { s } else { SyncSettings::default() },
                pairs.iter().any(|(k, _)| k == "scene"),
                any,
            )
        })
        .unwrap_or((SyncSettings::default(), false, false));
    // The editor starts on Balanced unless a link asks for a quality.
    let link_settings = {
        let mut s = link_settings;
        let asked = use_context::<super::app::RtLinkParams>()
            .is_some_and(|p| p.0.get_value().iter().any(|(k, _)| k == "quality"));
        if !asked {
            s.apply_quality("balanced");
        }
        s
    };
    let (settings, set_settings) = signal(link_settings);
    let t = Memo::new(move |_| use_locale().get().ui.light_scene.clone());
    let (status, set_status) = signal(Status::default());
    let browser = StoredValue::new(detect_browser());

    // The editor's luminaire, whenever it changes.
    // Only a real change of the luminaire reloads it: the editor's signal
    // also ticks on round trips that leave the data as it was, and a reload
    // restarts the picture and resets exposure and colour temperature.
    let last_ldt = StoredValue::new(String::new());
    let scene_touched = StoredValue::new(link_scene);
    Effect::new(move |_| {
        let ldt = ldc.get();
        let text = ldt.to_ldt();
        if last_ldt.with_value(|t| *t == text) {
            return;
        }
        last_ldt.set_value(text);
        // Until the visitor picks a scene, follow the luminaire: an office
        // light in the street looks wrong and vice versa.
        if !scene_touched.get_value() {
            let kind = suggest_scene(&ldt);
            if settings.get_untracked().scene != scene_id(kind) {
                set_settings.update(|s| *s = s.switched_to(kind));
            }
        }
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.load("editor luminaire", ldt, None);
            }
        });
    });

    // The luminaire body from a GLDF container (L3D model), if the app has one.
    let body = use_context::<RwSignal<Option<Arc<LuminaireMesh>>>>();
    Effect::new(move |_| {
        let mesh = body.map(|b| b.get()).unwrap_or_default();
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.set_mesh(mesh);
            }
        });
    });

    // Panel settings → viewer; the compute choice stays in the tab.
    let ppfd = use_context::<super::app::PpfdFactor>();
    Effect::new(move |_| {
        let mut s = settings.get();
        s.overlay = false;
        s.active = true;
        s.ppfd_factor = ppfd.and_then(|c| c.0.get());
        let cpu = s.compute == "cpu" || !webgpu.get();
        shared_sv.with_value(|sh| {
            let Ok(mut sh) = sh.try_borrow_mut() else {
                return;
            };
            sh.viewer.apply_sync(&s);
            if sh.cpu_mode != cpu {
                sh.cpu_mode = cpu;
                // A fresh start on the other tracer.
                sh.viewer.reset_token += 1;
                sh.viewer.probe_reset_token += 1;
            }
        });
    });

    // A plan from the Interior / Outdoor designer: the "Planned" scene,
    // applied when it arrives and when the tab mounts with one waiting.
    let plan_ctx = use_context::<super::app::LightScenePlan>();
    let doc_ctx = use_context::<super::app::LightSceneDoc>();
    // The scene being worked on: its document id, name, whether it is in
    // the project (then changes are saved there automatically), and the
    // plan of a planned scene (for the panel's "Plan" button).
    let new_doc_id = || format!("{:x}", js_sys::Date::now() as u64);
    let scene_doc_id = RwSignal::new(new_doc_id());
    let scene_name = RwSignal::new(String::new());
    let scene_saved = RwSignal::new(false);
    let saved_kind = StoredValue::new(String::new());
    let current_plan = RwSignal::new(None::<Arc<ScenePlan>>);
    // A plan from a designer: a new (unsaved) scene. Taken from the app,
    // so a later visit restores the last scene instead.
    Effect::new(move |_| {
        let Some(plan) = plan_ctx.and_then(|c| c.0.get()) else {
            return;
        };
        if let Some(c) = plan_ctx {
            c.0.set(None);
        }
        let n = plan.sensors.len().min(MAX_SENSORS) as u32;
        let height = plan.sensor_height;
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.set_plan(Some(plan.clone()));
            }
        });
        current_plan.set(Some(plan.clone()));
        scene_doc_id.set(new_doc_id());
        scene_saved.set(false);
        scene_name.set(plan.name.clone());
        scene_touched.set_value(true);
        set_settings.update(|s| {
            *s = s.switched_to(SceneKind::Planned);
            s.sensors = n;
            s.sensor_height = height;
            // the designed lighting first: the sensors only detect until
            // the user turns their control on (else nobody in a zone =
            // every luminaire at the idle level)
            s.sensor_dim = false;
            // the mode the designer showed; arriving in emergency mode
            // replays the power failure (the eye adapts again)
            if plan.emergency.is_some() {
                s.emergency_mode = plan.emergency_mode;
                if plan.emergency_mode {
                    s.emergency_replay = s.emergency_replay.wrapping_add(1);
                }
            }
        });
    });
    // A saved scene (project menu) or the last scene of this browser.
    let sensor_choice = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let Some(doc) = doc_ctx.and_then(|c| c.0.get()) else {
            return;
        };
        if let Some(c) = doc_ctx {
            c.0.set(None);
        }
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.apply_doc(&doc);
            }
        });
        current_plan.set(doc.plan.clone().map(Arc::new));
        scene_doc_id.set(doc.id.clone());
        scene_name.set(doc.name.clone());
        let saved = super::projects_ui::ctx().is_some_and(|p| p.has_scene(&doc.id));
        scene_saved.set(saved);
        saved_kind.set_value(doc.settings.scene.clone());
        if let Some(f) = doc.sensor_file.clone() {
            sensor_choice.set(Some(f));
        }
        scene_touched.set_value(true);
        set_settings.set(doc.settings.clone());
    });
    // Nothing asked for and no link: the last scene comes back.
    if !link_any
        && plan_ctx.is_none_or(|c| c.0.get_untracked().is_none())
        && doc_ctx.is_some_and(|c| c.0.get_untracked().is_none())
    {
        if let Some(doc) = load_last_scene() {
            if let Some(c) = doc_ctx {
                c.0.set(Some(Arc::new(doc)));
            }
        }
    }
    // The scene as a document (for the browser and the project).
    let build_doc = move || -> Option<(String, String)> {
        let sync = settings.get_untracked();
        let id = scene_doc_id.get_untracked();
        let name = scene_name.get_untracked();
        let lum = super::projects_ui::ctx()
            .and_then(|p| p.active_item())
            .map(|i| i.id);
        let doc = shared_sv.with_value(|sh| {
            sh.try_borrow()
                .ok()
                .map(|s| s.viewer.scene_doc(&sync, &id, &name, lum.clone()))
        })?;
        serde_json::to_string(&doc).ok().map(|j| (name, j))
    };
    let default_name = move || {
        let tt = t.get_untracked();
        match current_plan.get_untracked() {
            Some(p) if settings.get_untracked().scene == "planned" => p.name.clone(),
            _ => match settings.get_untracked().scene.as_str() {
                "parking" => tt.scene_parking,
                "facade" => tt.scene_facade,
                "office" => tt.scene_office,
                "greenhouse" => tt.scene_greenhouse,
                _ => tt.scene_street,
            },
        }
    };
    let on_save_scene = Callback::new(move |_| {
        if scene_name.get_untracked().trim().is_empty() {
            scene_name.set(default_name());
        }
        scene_saved.set(true);
        saved_kind.set_value(settings.get_untracked().scene);
        if let (Some(p), Some((name, json))) = (super::projects_ui::ctx(), build_doc()) {
            store_last_scene(&json);
            p.put_scene(&name, json);
        }
    });

    // Sensors: the file comes from the app's sensor list (the current
    // project's sensors, the demos), the first one unless picked.
    let sensor_files = use_context::<super::sensor_view::SensorFiles>().map(|f| f.0);
    let sensor_names = Memo::new(move |_| {
        sensor_files
            .map(|f| f.with(|v| v.iter().map(|x| x.name.clone()).collect::<Vec<_>>()))
            .unwrap_or_default()
    });
    Effect::new(move |_| {
        let wanted = settings.with(|s| s.sensors > 0);
        let choice = sensor_choice.get();
        let file = if wanted {
            sensor_files.and_then(|f| {
                f.with(|v| {
                    v.iter()
                        .find(|x| Some(&x.name) == choice.as_ref())
                        .or_else(|| v.first())
                        .map(|x| (x.name.clone(), Arc::new(x.data.to_core())))
                })
            })
        } else {
            None
        };
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.set_sensor_file(file);
            }
        });
    });
    let on_load_demo = Callback::new(move |_| {
        if let Some(f) = sensor_files {
            super::sensor_view::add_demo(f);
        }
    });
    // Move mode: drags on the picture move objects instead of the camera.
    let move_mode = RwSignal::new(false);
    Effect::new(move |_| {
        let on = move_mode.get();
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.move_mode = on;
            }
        });
    });
    let on_reset_moves = Callback::new(move |_| {
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.reset_moves();
            }
        });
    });

    // Tab visibility starts and pauses the frame loop; the first activation boots the GPU.
    Effect::new(move |_| {
        let on = active.get();
        shared_sv.with_value(|sh| {
            sh.borrow_mut().active = on;
            if on {
                run_loop(sh.clone());
            }
        });
        if on && load_state.get_untracked() == LoadState::Idle {
            let (Some(canvas), Some(overlay), Some(cpu_canvas)) = (
                canvas_ref.get_untracked(),
                overlay_ref.get_untracked(),
                cpu_ref.get_untracked(),
            ) else {
                return;
            };
            let canvas: web_sys::HtmlCanvasElement = canvas.unchecked_into();
            let overlay: web_sys::HtmlCanvasElement = overlay.unchecked_into();
            let cpu_canvas: web_sys::HtmlCanvasElement = cpu_canvas.unchecked_into();
            let sh = shared_sv.get_value();
            {
                let mut s = sh.borrow_mut();
                s.canvas = Some(canvas.clone());
                s.overlay = Some(overlay);
                s.cpu_canvas = Some(cpu_canvas);
            }
            if !has_webgpu() {
                // No WebGPU: the CPU tracer carries the tab.
                set_webgpu.set(false);
                sh.borrow_mut().cpu_mode = true;
                set_load_state.set(LoadState::Running);
                run_loop(sh.clone());
                return;
            }
            set_load_state.set(LoadState::Starting);
            wasm_bindgen_futures::spawn_local(async move {
                match init_gpu(canvas).await {
                    Ok(gpu) => {
                        sh.borrow_mut().gpu = Some(gpu);
                        set_load_state.set(LoadState::Running);
                        run_loop(sh.clone());
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("light scene: {e}, using the CPU").into(),
                        );
                        set_webgpu.set(false);
                        sh.borrow_mut().cpu_mode = true;
                        set_load_state.set(LoadState::Running);
                        run_loop(sh.clone());
                    }
                }
            });
        }
    });

    // Status for the panel, a few times per second while running.
    let seen_resets = StoredValue::new(0u32);
    // Autosave: a second after the last change the scene goes to this
    // browser, and into the project when it was saved there.
    let last_fp = StoredValue::new(0u64);
    let changed_at = StoredValue::new(0.0f64);
    let dirty = StoredValue::new(false);
    Effect::new(move |_| {
        if load_state.get() != LoadState::Running {
            return;
        }
        let handle = set_interval_with_handle(
            move || {
                if !active.get_untracked() {
                    return;
                }
                let now = window().performance().map(|p| p.now()).unwrap_or(0.0);
                let (st, n, stalled, resets, gone) = shared_sv.with_value(|sh| {
                    let Ok(s) = sh.try_borrow() else {
                        return (None, None, false, 0, false);
                    };
                    let stalled = s.last_frame > 0.0 && now - s.last_frame > 1500.0;
                    (
                        Some(s.viewer.status()),
                        Some(s.note.clone()),
                        stalled,
                        s.gpu_resets,
                        s.gpu_gone,
                    )
                });
                // After a GPU reset: go down to Quick so it does not happen
                // again, and say why.
                if resets > seen_resets.get_value() {
                    seen_resets.set_value(resets);
                    set_settings.update(|s| s.apply_quality("quick"));
                }
                // Another scene type: a new scene, not yet in the project.
                let kind_now = settings.get_untracked().scene;
                if scene_saved.get_untracked() && saved_kind.get_value() != kind_now {
                    scene_saved.set(false);
                    scene_doc_id.set(new_doc_id());
                    scene_name.set(String::new());
                }
                // Saved in a project that loaded later (IndexedDB).
                if !scene_saved.get_untracked()
                    && super::projects_ui::ctx()
                        .is_some_and(|p| p.has_scene(&scene_doc_id.get_untracked()))
                {
                    scene_saved.set(true);
                    saved_kind.set_value(kind_now.clone());
                }
                let fp = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    shared_sv.with_value(|sh| {
                        if let Ok(s) = sh.try_borrow() {
                            s.viewer.doc_fingerprint().hash(&mut h);
                        }
                    });
                    serde_json::to_string(&settings.get_untracked())
                        .unwrap_or_default()
                        .hash(&mut h);
                    scene_name.get_untracked().hash(&mut h);
                    h.finish()
                };
                if fp != last_fp.get_value() {
                    last_fp.set_value(fp);
                    changed_at.set_value(now);
                    dirty.set_value(true);
                } else if dirty.get_value() && now - changed_at.get_value() > 1000.0 {
                    dirty.set_value(false);
                    if let Some((name, json)) = build_doc() {
                        store_last_scene(&json);
                        if scene_saved.get_untracked() {
                            if let Some(p) = super::projects_ui::ctx() {
                                p.put_scene(&name, json);
                            }
                        }
                    }
                }
                let n = if gone {
                    Some(t.get_untracked().gpu_gone_note)
                } else if resets > 0
                    && n.as_deref()
                        .is_some_and(|n| n.is_empty() || n == "GPU restarted")
                {
                    Some(t.get_untracked().gpu_reset_note)
                } else {
                    n
                };
                if let Some(st) = st {
                    set_status.set(st);
                }
                if let Some(n) = n {
                    set_note.set(n);
                }
                // Watchdog: a frame loop that died (an exception inside a frame
                // breaks the requestAnimationFrame chain) is started again.
                if stalled {
                    shared_sv.with_value(|sh| {
                        if let Ok(mut s) = sh.try_borrow_mut() {
                            s.running = false;
                            s.last_frame = now;
                            s.note = "frame loop restarted".into();
                        }
                        run_loop(sh.clone());
                    });
                }
            },
            std::time::Duration::from_millis(350),
        );
        if let Ok(h) = handle {
            on_cleanup(move || h.clear());
        }
    });

    // Camera input on the picture.
    let on_down = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        let pan = ev.button() == 2 || ev.button() == 1 || ev.shift_key();
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                // Move mode: grab the object under the pointer, if any.
                if s.move_mode && !pan {
                    let (w, h) = canvas_css_size(&s);
                    if s.viewer
                        .begin_drag(ev.offset_x() as f32, ev.offset_y() as f32, w, h)
                    {
                        s.obj_drag = true;
                        s.drag = None;
                        return;
                    }
                }
                s.drag = Some((ev.client_x() as f32, ev.client_y() as f32, pan));
            }
        });
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = target.set_pointer_capture(ev.pointer_id());
        }
    };
    let on_move = move |ev: web_sys::PointerEvent| {
        shared_sv.with_value(|sh| {
            let Ok(mut s) = sh.try_borrow_mut() else {
                return;
            };
            if s.obj_drag {
                let (w, h) = canvas_css_size(&s);
                s.viewer
                    .drag_to(ev.offset_x() as f32, ev.offset_y() as f32, w, h);
                return;
            }
            let Some((lx, ly, pan)) = s.drag else {
                return;
            };
            let (x, y) = (ev.client_x() as f32, ev.client_y() as f32);
            let (dx, dy) = (x - lx, y - ly);
            s.drag = Some((x, y, pan));
            let css_h = s
                .canvas
                .as_ref()
                .map(|c| c.client_height() as f32)
                .unwrap_or(600.0);
            if pan {
                s.viewer.pan(dx, dy, css_h);
            } else {
                s.viewer.orbit(dx, dy);
            }
        });
    };
    let on_up = move |_ev: web_sys::PointerEvent| {
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.drag = None;
                if s.obj_drag {
                    s.obj_drag = false;
                    s.viewer.end_drag();
                }
            }
        });
    };
    let on_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();
        let dy = ev.delta_y() as f32 * if ev.delta_mode() == 1 { 40.0 } else { 1.0 };
        shared_sv.with_value(|sh| {
            if let Ok(mut s) = sh.try_borrow_mut() {
                s.viewer.zoom((-dy * 0.0015).clamp(-1.0, 1.0).exp());
            }
        });
    };

    let (intro_open, set_intro_open) = signal(!intro_seen());
    let (remember, set_remember) = signal(true);
    let close_intro = move || {
        if remember.get_untracked() {
            set_intro_seen();
        }
        set_intro_open.set(false);
    };

    view! {
        <div class="rt-scene" style="width: 100%; height: 100%; display: flex; min-height: 0;">
            <RtControlPanel settings=settings set_settings=set_settings status=status webgpu=webgpu scene_touched=scene_touched
                sensor_names=sensor_names sensor_choice=sensor_choice on_load_demo=on_load_demo plan=Signal::from(current_plan)
                scene_name=scene_name scene_saved=scene_saved on_save_scene=on_save_scene />

            <div style="flex: 1; position: relative; min-width: 0; background: #07080a; border-radius: 0 8px 8px 0; overflow: hidden;">
                <canvas
                    node_ref=canvas_ref
                    style=move || format!("width: 100%; height: 100%; display: block; touch-action: none; cursor: {};", if move_mode.get() { "move" } else { "grab" })
                    on:pointerdown=on_down
                    on:pointermove=on_move
                    on:pointerup=on_up
                    on:pointercancel=on_up
                    on:wheel=on_wheel
                    on:contextmenu=move |ev: web_sys::MouseEvent| ev.prevent_default()
                />
                <canvas
                    node_ref=cpu_ref
                    style=move || format!(
                        "position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; image-rendering: auto; display: {};",
                        if settings.get().compute == "cpu" || !webgpu.get() { "block" } else { "none" }
                    )
                />
                <canvas
                    node_ref=overlay_ref
                    style="position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none;"
                />

                {move || match load_state.get() {
                    LoadState::Idle | LoadState::Starting => view! {
                        <div style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: rgba(7,8,10,0.92); color: white; text-align: center; padding: 24px;">
                            <div style="font-size: 20px; font-weight: 300; margin-bottom: 8px;">"Light scene"</div>
                            <div style="font-size: 13px; color: #9aa4b2; max-width: 520px; line-height: 1.5;">
                                {move || t.get().starting}
                            </div>
                        </div>
                    }.into_any(),
                    LoadState::Running => view! {
                        // Progress: thin bar along the bottom edge plus a readable pill.
                        <div style="position: absolute; left: 0; right: 0; bottom: 0; height: 3px; background: rgba(255,255,255,0.06); pointer-events: none;">
                            <div style=move || {
                                let s = status.get();
                                let p = if s.max_samples > 0 { s.samples as f32 / s.max_samples as f32 } else { 0.0 };
                                format!("height: 100%; width: {:.1}%; background: var(--primary-color); opacity: {};", p * 100.0, if p >= 1.0 { 0 } else { 1 })
                            }></div>
                        </div>
                        <div style="position: absolute; bottom: 10px; left: 8px; background: rgba(0,0,0,0.6); color: #ccc; font-size: 11px; padding: 4px 8px; border-radius: 6px; pointer-events: none; font-variant-numeric: tabular-nums;">
                            {move || {
                                let s = status.get();
                                let tt = t.get();
                                if settings.get().mode == "points" && s.probe_max_samples > 0 && s.probe_samples < s.probe_max_samples {
                                    fill(&tt.measuring_points, &[("{n}", s.point_count.to_string())])
                                } else if s.max_samples > 0 && s.samples < s.max_samples {
                                    fill(&tt.refining, &[("{pct}", format!("{:.0}", 100.0 * s.samples as f32 / s.max_samples as f32))])
                                } else {
                                    tt.complete
                                }
                            }}
                        </div>
                        // What the current view means and how to move: one line, centred.
                        <div style="position: absolute; bottom: 10px; left: 50%; transform: translateX(-50%); max-width: 70%; background: rgba(0,0,0,0.55); color: #d6dbe3; font-size: 11px; padding: 4px 10px; border-radius: 6px; pointer-events: none; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                            {move || {
                                let tt = t.get();
                                match settings.get().mode.as_str() {
                                    "isolux" => {
                                        let st = status.get();
                                        if st.ppfd_factor > 0.0 {
                                            fill(&tt.hint_ppfd, &[("{step}", format!("{:.0}", st.iso_step * st.ppfd_factor))])
                                        } else {
                                            fill(&tt.hint_isolux, &[("{step}", format!("{:.0}", st.iso_step))])
                                        }
                                    }
                                    "points" => tt.hint_points,
                                    _ => tt.hint_render,
                                }
                            }}
                        </div>
                        <button
                            title=move || t.get().help
                            aria-label=move || t.get().help
                            style="position: absolute; top: 8px; left: 8px; width: 26px; height: 26px; border-radius: 13px; border: 1px solid rgba(255,255,255,0.25); background: rgba(0,0,0,0.55); color: #fff; font-size: 14px; cursor: pointer;"
                            on:click=move |_| set_intro_open.set(true)
                        >"?"</button>
                        // Move luminaires, the person, cars and sensors by hand.
                        <div style="position: absolute; top: 8px; right: 8px; display: flex; gap: 6px; align-items: center;">
                            {move || (status.get().moved > 0).then(|| view! {
                                <button style="padding: 5px 10px; border-radius: 14px; border: 1px solid rgba(255,255,255,0.25); background: rgba(0,0,0,0.55); color: #fff; font-size: 11.5px; cursor: pointer;"
                                    on:click=move |_| on_reset_moves.run(())>
                                    {move || format!("↺ {}", t.get().reset_moves)}
                                </button>
                            })}
                            <button
                                title=move || t.get().move_hint
                                style=move || format!(
                                    "padding: 5px 12px; border-radius: 14px; border: 1px solid {}; background: {}; color: #fff; font-size: 11.5px; cursor: pointer;",
                                    if move_mode.get() { "var(--primary-color)" } else { "rgba(255,255,255,0.25)" },
                                    if move_mode.get() { "var(--primary-color)" } else { "rgba(0,0,0,0.55)" }
                                )
                                on:click=move |_| move_mode.update(|m| *m = !*m)
                            >{move || t.get().move_objects}</button>
                        </div>
                        // What the sensors see.
                        {move || {
                            let st = status.get();
                            (st.sensors > 0).then(|| {
                                let tt = t.get();
                                let any = st.sensors_detecting.iter().any(|d| *d);
                                // detection is tested in 3D: a sensor with nothing on the
                                // floor can still see a person at chest height
                                let text = if !any && st.sensor_diameter.is_none() {
                                    fill(&tt.status_no_area, &[("{h}", format!("{:.1}", st.sensor_height))])
                                } else if any {
                                    let mut what = Vec::new();
                                    if st.person_detected { what.push(tt.status_person.clone()); }
                                    if st.cars_detected > 0 { what.push(fill(&tt.status_cars, &[("{n}", st.cars_detected.to_string())])); }
                                    fill(&tt.status_detected, &[
                                        ("{what}", what.join(", ")),
                                        ("{on}", st.lamps_on.to_string()),
                                        ("{n}", st.lamps.to_string()),
                                    ])
                                } else if !settings.get().sensor_dim {
                                    tt.status_dim_off.clone()
                                } else {
                                    let idle = settings.get_untracked().sensor_idle;
                                    fill(&tt.status_idle, &[("{pct}", format!("{:.0}", idle * 100.0))])
                                };
                                view! {
                                    <div style=format!(
                                        "position: absolute; top: 44px; right: 8px; max-width: 60%; background: rgba(0,0,0,0.62); color: {}; font-size: 11.5px; padding: 5px 10px; border-radius: 8px; pointer-events: none; border: 1px solid {};",
                                        if any { "#bbf7d0" } else { "#e2e8f0" },
                                        if any { "rgba(74,222,128,0.6)" } else { "rgba(255,255,255,0.15)" }
                                    )>
                                        {format!("📡 {text}")}
                                    </div>
                                }
                            })
                        }}
                        {move || move_mode.get().then(|| view! {
                            <div style="position: absolute; top: 40px; left: 8px; max-width: 45%; background: rgba(0,0,0,0.55); color: #d6dbe3; font-size: 11px; padding: 4px 10px; border-radius: 6px; pointer-events: none;">
                                {move || t.get().move_hint}
                            </div>
                        })}
                    }.into_any(),
                    LoadState::Unsupported => render_webgpu_missing(browser.get_value()).into_any(),
                }}
                {move || (!webgpu.get() && load_state.get() == LoadState::Running).then(|| view! {
                    <div style="position: absolute; bottom: 36px; left: 8px; background: rgba(0,0,0,0.6); color: #ccc; font-size: 11px; padding: 4px 8px; border-radius: 6px; pointer-events: none;">
                        {move || t.get().no_webgpu}
                    </div>
                })}
                {move || (!note.get().is_empty()).then(|| view! {
                    <div style="position: absolute; bottom: 36px; right: 8px; background: rgba(0,0,0,0.6); color: #f0c060; font-size: 11px; padding: 4px 8px; border-radius: 6px; pointer-events: none;">
                        {note.get()}
                    </div>
                })}

                // First-visit guide. The tracer keeps running behind it so the
                // picture is already forming when the visitor closes the card.
                {move || (intro_open.get() && load_state.get() == LoadState::Running).then(|| {
                    let tt = t.get();
                    let name = {
                        let n = status.get().name;
                        if n.is_empty() { "the loaded luminaire".to_string() } else { n }
                    };
                    let step = |n: &str, title: String, body: String| view! {
                        <div style="display: flex; gap: 12px; align-items: flex-start;">
                            <div style="flex: none; width: 26px; height: 26px; border-radius: 13px; background: var(--primary-color); color: #fff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center;">{n.to_string()}</div>
                            <div>
                                <div style="font-size: 13px; font-weight: 600; color: #fff; margin-bottom: 2px;">{title}</div>
                                <div style="font-size: 12px; color: #b4bcc8; line-height: 1.45;">{body}</div>
                            </div>
                        </div>
                    };
                    view! {
                        <div style="position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; background: rgba(7,8,10,0.6); padding: 16px;"
                            on:click=move |_| close_intro()>
                            <div style="background: rgba(18,20,26,0.96); border: 1px solid rgba(255,255,255,0.12); border-radius: 12px; padding: 22px 24px; max-width: 540px; width: 100%; color: #fff; box-shadow: 0 12px 40px rgba(0,0,0,0.5); display: flex; flex-direction: column; gap: 14px;"
                                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()>
                                <div style="font-size: 19px; font-weight: 300;">{tt.intro_title.clone()}</div>
                                <div style="font-size: 12.5px; color: #c8ced8; line-height: 1.5;">{fill(&tt.intro_lead, &[("{name}", name)])}</div>
                                {step("1", tt.intro_step1_title.clone(), tt.intro_step1.clone())}
                                {step("2", tt.intro_step2_title.clone(), tt.intro_step2.clone())}
                                {step("3", tt.intro_step3_title.clone(), tt.intro_step3.clone())}
                                <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 4px; flex-wrap: wrap;">
                                    <label style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; color: #9aa4b2; cursor: pointer;">
                                        <input type="checkbox" prop:checked=move || remember.get()
                                            on:change=move |ev| set_remember.set(event_target::<web_sys::HtmlInputElement>(&ev).checked()) />
                                        {tt.intro_remember.clone()}
                                    </label>
                                    <button style="padding: 8px 18px; background: var(--primary-color); border: none; border-radius: 6px; color: #fff; font-size: 13px; cursor: pointer;"
                                        on:click=move |_| close_intro()>
                                        {tt.intro_start.clone()}
                                    </button>
                                </div>
                            </div>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

const INTRO_KEY: &str = "eulumdat_rt_intro_seen";

fn intro_seen() -> bool {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(INTRO_KEY).ok().flatten())
        .is_some()
}

fn set_intro_seen() {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item(INTRO_KEY, "1");
    }
}

/// Replace `{key}` placeholders in a translated string.
fn fill(template: &str, pairs: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in pairs {
        out = out.replace(k, v);
    }
    out
}

fn height_range(s: &SyncSettings) -> (f32, f32) {
    match s.scene.as_str() {
        "parking" => (3.0, 14.0),
        "facade" => (0.2, 9.0),
        "office" => (1.2, (s.ceiling_height - 0.12).max(1.2)),
        "greenhouse" => (s.canopy_height + 0.2, 3.85),
        _ => (4.0, 14.0),
    }
}

fn count_options(s: &SyncSettings) -> &'static [u32] {
    match s.scene.as_str() {
        "street" => &[1, 2, 3, 4],
        "parking" => &[1, 2, 3, 4, 6],
        "facade" => &[1, 2, 3, 4, 5],
        _ => &[1, 2, 4, 6],
    }
}

#[component]
fn RtControlPanel(
    settings: ReadSignal<SyncSettings>,
    set_settings: WriteSignal<SyncSettings>,
    status: ReadSignal<Status>,
    webgpu: ReadSignal<bool>,
    scene_touched: StoredValue<bool>,
    sensor_names: Memo<Vec<String>>,
    sensor_choice: RwSignal<Option<String>>,
    on_load_demo: Callback<()>,
    plan: Signal<Option<Arc<ScenePlan>>>,
    scene_name: RwSignal<String>,
    scene_saved: RwSignal<bool>,
    on_save_scene: Callback<()>,
) -> impl IntoView {
    let ui_sensor = Memo::new(move |_| use_locale().get().ui.sensor.clone());
    let t = Memo::new(move |_| use_locale().get().ui.light_scene.clone());
    let (open, set_open) = signal(true);
    let upd = move |f: &dyn Fn(&mut SyncSettings)| set_settings.update(|s| f(s));
    let seg_style = |on: bool| {
        format!(
            "flex: 1; padding: 5px 4px; font-size: 11px; cursor: pointer; border: 1px solid var(--border); background: {}; color: {}; border-radius: 4px;",
            if on { "var(--primary-color)" } else { "var(--surface-elevated)" },
            if on { "#fff" } else { "var(--text-primary)" }
        )
    };
    let desc_style = "font-size: 10.5px; color: var(--text-secondary); line-height: 1.4;";
    // memos: the status changes every few hundred ms
    let show_emergency = Memo::new(move |_| {
        let st = status.get();
        settings.get().scene == "planned" && (st.emergency_lamps > 0 || !st.routes.is_empty())
    });

    view! {
        <div
            class="viewer-control-panel"
            style=move || format!(
                "width: {}; background: var(--surface); border-right: 1px solid var(--border); padding: {}; overflow-y: auto; font-size: 12px; transition: width 0.2s;",
                if open.get() { "260px" } else { "36px" },
                if open.get() { "10px 12px" } else { "8px 4px" }
            )
        >
            <button
                style="width: 100%; padding: 5px; margin-bottom: 10px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer; font-size: 13px;"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                {move || if open.get() { "◀" } else { "▶" }}
            </button>

            {move || open.get().then(|| view! {
                <div style="display: flex; flex-direction: column; gap: 10px;">
                    <div style="font-size: 11px; color: var(--text-secondary); line-height: 1.4;">
                        {move || {
                            let s = status.get();
                            let tt = t.get();
                            if s.name.is_empty() { tt.waiting } else {
                                let body = if s.body_tris > 0 { format!(" · L3D body, {} triangles", s.body_tris) } else { String::new() };
                                fill(&tt.summary, &[
                                    ("{name}", s.name.clone()),
                                    ("{lm}", (s.lumens as i64).to_string()),
                                    ("{cd}", format!("{:.0}", s.peak_cd)),
                                    ("{c}", format!("{:.0}", s.throw_c)),
                                    ("{pct}", format!("{:.0}", s.asymmetry * 100.0)),
                                ]) + &body
                            }
                        }}
                    </div>

                    <div style="display: flex; flex-direction: column; gap: 4px;">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().quality}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {["quick", "balanced", "high"].into_iter().map(|id| view! {
                                    <button style=move || seg_style(settings.get().quality == id)
                                        on:click=move |_| upd(&|s| s.apply_quality(id))>
                                        {move || { let tt = t.get(); match id { "quick" => tt.quality_quick, "high" => tt.quality_high, _ => tt.quality_balanced } }}
                                    </button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style=desc_style>
                            {move || { let tt = t.get(); match settings.get().quality.as_str() { "quick" => tt.quality_desc_quick, "high" => tt.quality_desc_high, _ => tt.quality_desc_balanced } }}
                        </div>
                        {move || (settings.get().quality != "quick").then(|| view! {
                            <Check label=Signal::derive(move || t.get().auto_quick) checked=Signal::derive(move || settings.get().auto_quick)
                                on_change=move |v| upd(&|s| s.auto_quick = v) />
                        })}
                    </div>

                    <Section title=Signal::derive(move || t.get().step_scene)>
                        <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                            {[SceneKind::Street, SceneKind::Parking, SceneKind::Facade, SceneKind::Office, SceneKind::Greenhouse].into_iter().map(|kind| {
                                let id = scene_id(kind);
                                let label = move || {
                                    let tt = t.get();
                                    match kind { SceneKind::Street => tt.scene_street, SceneKind::Parking => tt.scene_parking, SceneKind::Facade => tt.scene_facade, SceneKind::Office => tt.scene_office, SceneKind::Greenhouse => tt.scene_greenhouse, SceneKind::Planned => tt.scene_planned }
                                };
                                view! {
                                    <button style=move || seg_style(settings.get().scene == id)
                                        on:click=move |_| { scene_touched.set_value(true); upd(&|s| { if s.scene != id { *s = s.switched_to(kind); } }) }>
                                        {label}
                                    </button>
                                }
                            }).collect_view()}
                            {move || plan.get().map(|pl| {
                                let n = pl.sensors.len().min(MAX_SENSORS) as u32;
                                let h = pl.sensor_height;
                                view! {
                                    <button style=move || seg_style(settings.get().scene == "planned")
                                        on:click=move |_| { scene_touched.set_value(true); upd(&|s| { if s.scene != "planned" { *s = s.switched_to(SceneKind::Planned); s.sensors = n; s.sensor_height = h; } }) }>
                                        {move || t.get().scene_planned}
                                    </button>
                                }
                            })}
                        </div>
                        <div style=desc_style>
                            {move || {
                                let tt = t.get();
                                match settings.get().scene.as_str() {
                                    "parking" => tt.desc_parking,
                                    "facade" => tt.desc_facade,
                                    "office" => tt.desc_office,
                                    "greenhouse" => tt.desc_greenhouse,
                                    "planned" => plan.get().map(|pl| {
                                        let from = if pl.indoor { tt.from_interior.clone() } else { tt.from_outdoor.clone() };
                                        let mut d = fill(&tt.desc_planned, &[("{from}", from), ("{name}", pl.name.clone())]);
                                        if let Some(note) = pl.note.as_ref() {
                                            d.push(' ');
                                            d.push_str(note);
                                        }
                                        d
                                    }).unwrap_or_default(),
                                    _ => tt.desc_street,
                                }
                            }}
                        </div>
                        // back to the designer the plan came from
                        {move || {
                            let pl = plan.get().filter(|_| settings.get().scene == "planned")?;
                            let back = use_context::<super::app::OpenDesigner>()?;
                            let indoor = pl.indoor;
                            Some(view! {
                                <button style="padding: 5px 8px; font-size: 11px; cursor: pointer; border: 1px solid var(--border); background: var(--surface-elevated); color: var(--text-primary); border-radius: 4px; text-align: left;"
                                    on:click=move |_| back.0.run(indoor)>
                                    {move || {
                                        let tt = t.get();
                                        let from = if indoor { tt.from_interior.clone() } else { tt.from_outdoor.clone() };
                                        fill(&tt.back_to_designer, &[("{from}", from)])
                                    }}
                                </button>
                            })
                        }}
                                            // The scene: name, save in the project (then autosaved)
                        <div style="display: flex; gap: 4px; align-items: center;">
                            <input type="text" style="flex: 1; min-width: 0; font-size: 11px; padding: 4px 6px;"
                                placeholder=move || t.get().scene_name
                                prop:value=move || scene_name.get()
                                on:change=move |ev| scene_name.set(event_target_value(&ev)) />
                            {move || (!scene_saved.get()).then(|| view! {
                                <button style="padding: 5px 8px; background: var(--primary-color); border: none; border-radius: 4px; color: #fff; cursor: pointer; font-size: 11px; white-space: nowrap;"
                                    on:click=move |_| on_save_scene.run(())>{move || t.get().save_scene}</button>
                            })}
                        </div>
                        <div style=desc_style>
                            {move || {
                                let tt = t.get();
                                if scene_saved.get() {
                                    let project = super::projects_ui::ctx().and_then(|p| p.current_name()).unwrap_or_default();
                                    fill(&tt.scene_saved, &[("{project}", project)])
                                } else {
                                    tt.scene_unsaved
                                }
                            }}
                        </div>
                    </Section>

                    {move || show_emergency.get().then(|| view! { <EmergencyPanel settings set_settings status plan /> })}

                    <Section title=Signal::derive(move || t.get().step_view)>
                        <div style="display: flex; gap: 4px;">
                            {["render", "isolux", "points"].into_iter().map(|id| view! {
                                <button style=move || seg_style(settings.get().mode == id)
                                    on:click=move |_| upd(&|s| { s.mode = id.into(); })>
                                    {move || { let tt = t.get(); match id { "isolux" => tt.mode_isolux, "points" => tt.mode_points, _ => tt.mode_render } }}
                                </button>
                            }).collect_view()}
                        </div>
                        <div style=desc_style>
                            {move || {
                                let tt = t.get();
                                match settings.get().mode.as_str() { "isolux" => tt.mode_isolux_desc, "points" => tt.mode_points_desc, _ => tt.mode_render_desc }
                            }}
                        </div>
                        <Check label=Signal::derive(move || t.get().isolines_on_photo) checked=Signal::derive(move || settings.get().isolines) on_change=move |v| upd(&|s| s.isolines = v) />
                        {move || (settings.get().mode == "isolux").then(|| {
                            let s = status.get();
                            view! {
                                <div style="display: flex; gap: 8px; align-items: stretch; margin-top: 4px;">
                                    <div style="width: 10px; height: 110px; border-radius: 3px; background: linear-gradient(to top, #440154, #3b528b 25%, #21918c 50%, #5ec962 75%, #fde725);"></div>
                                    <div style="display: flex; flex-direction: column; justify-content: space-between; font-size: 10px; color: var(--text-secondary); font-variant-numeric: tabular-nums;">
                                        <span>{if s.ppfd_factor > 0.0 { format!("{:.0} {}", s.lux_max * s.ppfd_factor, t.get().unit_ppfd) } else { format!("{:.0} lx", s.lux_max) }}</span>
                                        <span>{format!("{:.0}", s.lux_max * 0.5 * if s.ppfd_factor > 0.0 { s.ppfd_factor } else { 1.0 })}</span>
                                        <span>"0"</span>
                                    </div>
                                    <div style="font-size: 10px; color: var(--text-secondary); align-self: flex-end;">{fill(&t.get().legend_lines, &[("{step}", format!("{:.0}", s.iso_step * if s.ppfd_factor > 0.0 { s.ppfd_factor } else { 1.0 }))])}</div>
                                </div>
                            }
                        })}
                        {move || (settings.get().mode == "points").then(|| {
                            let s = status.get();
                            let tt = t.get();
                            view! {
                                <table style="width: 100%; font-size: 10.5px; border-collapse: collapse; margin-top: 4px; font-variant-numeric: tabular-nums;">
                                    <thead><tr style="color: var(--text-secondary);"><th style="text-align: left;">{tt.col_grid.clone()}</th><th style="text-align: right;">{tt.col_em.clone()}</th><th style="text-align: right;">{tt.col_emin.clone()}</th><th style="text-align: right;">{tt.col_uo.clone()}</th>{(s.ppfd_factor > 0.0).then(|| view! { <th style="text-align: right;">{tt.dli.clone()}</th> })}</tr></thead>
                                    <tbody>
                                        {if s.grids.is_empty() {
                                            view! { <tr><td colspan="5" style="color: var(--text-secondary);">{tt.measuring.clone()}</td></tr> }.into_any()
                                        } else {
                                            let k = if s.ppfd_factor > 0.0 { s.ppfd_factor } else { 1.0 };
                                            let hours = s.photoperiod;
                                            let f = s.ppfd_factor;
                                            s.grids.iter().map(|g| view! {
                                                <tr><td>{g.name.clone()}</td><td style="text-align: right;">{fmt_lux(g.em * k)}</td><td style="text-align: right;">{fmt_lux(g.emin * k)}</td><td style="text-align: right;">{format!("{:.2}", g.uo)}</td>{(f > 0.0).then(|| view! { <td style="text-align: right;">{format!("{:.1}", g.em * f * hours * 3600.0 / 1.0e6)}</td> })}</tr>
                                            }).collect_view().into_any()
                                        }}
                                    </tbody>
                                </table>
                                <div style=desc_style>
                                    {if s.ppfd_factor > 0.0 {
                                        let note = if s.ppfd_from_spectrum { fill(&tt.ppfd_from_spd, &[("{f}", format!("{:.4}", s.ppfd_factor))]) } else { fill(&tt.ppfd_assumed, &[("{f}", format!("{:.4}", s.ppfd_factor))]) };
                                        format!("{} {} {}", tt.unit_ppfd, note, tt.dli_explain)
                                    } else { tt.stats_explain.clone() }}
                                </div>
                            }
                        })}
                    </Section>

                    <Section title=Signal::derive(move || t.get().step_placement)>
                        // A plan brings its own placement.
                        {move || (settings.get().scene != "planned").then(|| view! {
                        {move || {
                            let s = settings.get();
                            let tt = t.get();
                            let (lo, hi) = height_range(&s);
                            view! {
                                <Range label=if s.scene == "office" || s.scene == "greenhouse" { tt.height_above_floor } else { tt.mounting_height } unit="m" value=s.height min=lo max=hi step=0.05
                                    on_change=move |v| upd(&|s| s.height = v) />
                            }
                        }}
                        {move || {
                            let s = settings.get();
                            let tt = t.get();
                            match s.scene.as_str() {
                                "office" => view! { <Range label=tt.ceiling_height unit="m" value=s.ceiling_height min=2.4 max=4.5 step=0.05 on_change=move |v| upd(&|s| { s.ceiling_height = v; s.height = s.height.min(v - 0.12); }) /> }.into_any(),
                                "facade" => view! { <Range label=tt.distance_to_facade unit="m" value=s.wall_distance min=0.2 max=6.0 step=0.05 on_change=move |v| upd(&|s| s.wall_distance = v) /> }.into_any(),
                                "greenhouse" => view! {
                                    <Range label=tt.canopy_height unit="m" value=s.canopy_height min=0.5 max=2.2 step=0.05 on_change=move |v| upd(&|s| { s.canopy_height = v; s.height = s.height.max(v + 0.2); }) />
                                    <Range label=tt.photoperiod unit="h" value=s.photoperiod min=6.0 max=24.0 step=0.5 on_change=move |v| upd(&|s| s.photoperiod = v) />
                                }.into_any(),
                                _ => view! { <div></div> }.into_any(),
                            }
                        }}
                        <Range label=t.get_untracked().tilt unit="°" value=settings.get_untracked().tilt min=-90.0 max=90.0 step=1.0 on_change=move |v| upd(&|s| s.tilt = v) />
                        <Check label=Signal::derive(move || t.get().aim_auto) checked=Signal::derive(move || settings.get().rotation.is_none())
                            on_change=move |v| upd(&|s| { s.rotation = if v { None } else { Some(status.get_untracked().rotation) }; }) />
                        {move || settings.get().rotation.map(|r| view! {
                            <Range label=t.get().rotation unit="°" value=r min=-180.0 max=180.0 step=1.0 on_change=move |v| upd(&|s| s.rotation = Some(v)) />
                        })}
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().luminaires}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {move || count_options(&settings.get()).iter().map(|&n| view! {
                                    <button style=move || seg_style(settings.get().count == n) on:click=move |_| upd(&|s| s.count = n)>{n.to_string()}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        {move || {
                            let s = settings.get();
                            let (lo, hi, step) = match s.scene.as_str() { "street" => (12.0, 60.0, 1.0), "parking" => (8.0, 40.0, 1.0), _ => (0.5, 12.0, 0.1) };
                            (s.count > 1 && s.scene != "office").then(|| view! {
                                <Range label=t.get().spacing unit="m" value=s.spacing min=lo max=hi step=step on_change=move |v| upd(&|s| s.spacing = v) />
                            })
                        }}
                        {move || (settings.get().scene == "street").then(|| view! {
                            <div style="display: flex; gap: 4px;">
                                {["single", "opposite", "staggered"].into_iter().map(|id| view! {
                                    <button style=move || seg_style(settings.get().street_layout == id) on:click=move |_| upd(&|s| s.street_layout = id.into())>
                                        {move || { let tt = t.get(); match id { "opposite" => tt.layout_opposite, "staggered" => tt.layout_staggered, _ => tt.layout_single } }}
                                    </button>
                                }).collect_view()}
                            </div>
                        })}
                        {move || matches!(settings.get().scene.as_str(), "street" | "facade").then(|| view! {
                            <div style="display: flex; gap: 4px;">
                                {["render", "brick"].into_iter().map(|id| view! {
                                    <button style=move || seg_style(settings.get().wall_finish == id) on:click=move |_| upd(&|s| s.wall_finish = id.into())>
                                        {move || { let tt = t.get(); if id == "brick" { tt.facade_brick } else { tt.facade_render } }}
                                    </button>
                                }).collect_view()}
                            </div>
                        })}
                        })}
                        <Check label=Signal::derive(move || t.get().person_for_scale) checked=Signal::derive(move || settings.get().show_figure) on_change=move |v| upd(&|s| s.show_figure = v) />
                    </Section>

                    <Section title=Signal::derive(move || t.get().step_sensors)>
                        <div style=desc_style>{move || t.get().sensors_desc}</div>
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().sensor_count}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {move || {
                                    let cur = settings.get().sensors;
                                    let mut opts = vec![0u32, 1, 2, 3, 4, 6];
                                    if !opts.contains(&cur) {
                                        opts.push(cur.min(MAX_SENSORS as u32));
                                    }
                                    opts.into_iter().map(|n| view! {
                                        <button style=move || seg_style(settings.get().sensors == n) on:click=move |_| upd(&|s| s.sensors = n)>{n.to_string()}</button>
                                    }).collect_view()
                                }}
                            </div>
                        </div>
                        {move || (settings.get().sensors > 0).then(|| {
                            let names = sensor_names.get();
                            let tt = t.get();
                            if names.is_empty() {
                                return view! {
                                    <div style=desc_style>{tt.sensor_none.clone()}</div>
                                    <button style="padding: 6px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer;"
                                        on:click=move |_| on_load_demo.run(())>{tt.sensor_load_demo.clone()}</button>
                                }.into_any();
                            }
                            let current = sensor_choice.get().filter(|c| names.contains(c)).or_else(|| names.first().cloned());
                            view! {
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <span style="color: var(--text-secondary); width: 70px;">{tt.sensor_file.clone()}</span>
                                    <select style="flex: 1; min-width: 0; font-size: 11px;"
                                        on:change=move |ev| sensor_choice.set(Some(event_target_value(&ev)))>
                                        {names.into_iter().map(|n| {
                                            let sel = current.as_deref() == Some(n.as_str());
                                            let value = n.clone();
                                            view! { <option value=value selected=sel>{n}</option> }
                                        }).collect_view()}
                                    </select>
                                </div>
                            }.into_any()
                        })}
                        {move || {
                            let s = settings.get();
                            (s.sensors > 0 && s.scene != "office").then(|| {
                                let h = if s.sensor_height > 0.0 { s.sensor_height } else { status.get_untracked().sensor_height.max(1.0) };
                                view! {
                                    <Range label=t.get().sensor_height unit="m" value=h min=1.0 max=12.0 step=0.1
                                        on_change=move |v| upd(&|s| s.sensor_height = v) />
                                }
                            })
                        }}
                        {move || (settings.get().sensors > 0).then(|| view! {
                            <div style="display: flex; align-items: center; gap: 8px;">
                                <span style="color: var(--text-secondary); width: 70px;">{move || t.get().sensor_motion}</span>
                                <div style="display: flex; gap: 3px; flex: 1;">
                                    {["tangential", "radial", "presence"].into_iter().map(|id| view! {
                                        <button style=move || seg_style(settings.get().sensor_motion == id) on:click=move |_| upd(&|s| s.sensor_motion = id.into())>
                                            {move || { let u = ui_sensor.get(); match id { "radial" => u.radial, "presence" => u.presence, _ => u.tangential } }}
                                        </button>
                                    }).collect_view()}
                                </div>
                            </div>
                            <Check label=Signal::derive(move || t.get().sensor_dim)
                                checked=Signal::derive(move || settings.get().sensor_dim)
                                on_change=move |v| upd(&|s| s.sensor_dim = v) />
                            {move || (!settings.get().sensor_dim).then(|| view! {
                                <div style=desc_style>{move || t.get().sensor_dim_off_note}</div>
                            })}
                            <div style=move || format!("display: {}; flex-direction: column; gap: 3px;", if settings.get().sensor_dim { "flex" } else { "none" })>
                                {["together", "nearest"].into_iter().map(|id| view! {
                                    <button style=move || seg_style(settings.get().sensor_control == id) on:click=move |_| upd(&|s| s.sensor_control = id.into())>
                                        {move || { let tt = t.get(); if id == "nearest" { tt.control_nearest } else { tt.control_together } }}
                                    </button>
                                }).collect_view()}
                            </div>
                            <div style=move || format!("display: {}; align-items: center; gap: 8px;", if settings.get().sensor_dim { "flex" } else { "none" })>
                                <span style="color: var(--text-secondary); width: 70px;">{move || t.get().idle_level}</span>
                                <div style="display: flex; gap: 3px; flex: 1;">
                                    {[0.0f32, 0.1, 0.3, 0.5].into_iter().map(|lv| view! {
                                        <button style=move || seg_style((settings.get().sensor_idle - lv).abs() < 1e-3) on:click=move |_| upd(&|s| s.sensor_idle = lv)>
                                            {move || if lv == 0.0 { t.get().idle_off } else { format!("{:.0} %", lv * 100.0) }}
                                        </button>
                                    }).collect_view()}
                                </div>
                            </div>
                            <Check label=Signal::derive(move || t.get().sensor_volume_3d)
                                checked=Signal::derive(move || settings.get().sensor_volume)
                                on_change=move |v| upd(&|s| s.sensor_volume = v) />
                            <div style=desc_style>
                                {move || {
                                    let st = status.get();
                                    let tt = t.get();
                                    match st.sensor_diameter {
                                        Some(d) => fill(&tt.sensor_area, &[("{d}", format!("{d:.1}")), ("{h}", format!("{:.1}", st.sensor_height))]),
                                        None => fill(&tt.status_no_area, &[("{h}", format!("{:.1}", st.sensor_height))]),
                                    }
                                }}
                            </div>
                        })}
                    </Section>

                    <MoonPanel settings set_settings status />

                    <Section title=Signal::derive(move || t.get().light_camera) collapsed=true>
                        <Check label=Signal::derive(move || format!("{}{}", t.get().cct_from_file, status.get().cct_file.map(|c| format!(" ({c:.0} K)")).unwrap_or_default()))
                            checked=Signal::derive(move || settings.get().cct.is_none())
                            on_change=move |v| upd(&|s| { s.cct = if v { None } else { Some(status.get_untracked().cct.max(2000.0)) }; }) />
                        {move || settings.get().cct.map(|c| view! {
                            <Range label=t.get().cct unit="K" value=c min=2000.0 max=6500.0 step=50.0 on_change=move |v| upd(&|s| s.cct = Some(v)) />
                        })}
                        <Range label=t.get_untracked().haze unit="%" value=settings.get_untracked().haze * 100.0 min=0.0 max=100.0 step=1.0 on_change=move |v| upd(&|s| s.haze = v / 100.0) />
                        <Range label=t.get_untracked().exposure unit="EV" value=settings.get_untracked().exposure_ev min=-4.0 max=4.0 step=0.1 on_change=move |v| upd(&|s| s.exposure_ev = v) />
                        <Check label=Signal::derive(move || t.get().glare) checked=Signal::derive(move || settings.get().glare) on_change=move |v| upd(&|s| s.glare = v) />
                        <Check label=Signal::derive(move || t.get().denoise) checked=Signal::derive(move || settings.get().denoise) on_change=move |v| upd(&|s| s.denoise = v) />
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().bounces}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {[1u32, 2, 3, 4, 6].into_iter().map(|n| view! {
                                    <button style=move || seg_style(settings.get().bounces == n) on:click=move |_| upd(&|s| s.bounces = n)>{n.to_string()}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style="display: flex; gap: 4px;">
                            <button style="flex: 1; padding: 6px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer;"
                                on:click=move |_| upd(&|s| s.reset_view += 1)>
                                {move || t.get().reset_view}
                            </button>
                            <button style="flex: 1; padding: 6px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer;"
                                title=move || t.get().restart_tip
                                on:click=move |_| { if let Some(r) = use_context::<WriteSignal<u32>>() { r.update(|v| *v += 1); } }>
                                {move || t.get().restart}
                            </button>
                        </div>
                        <div style=desc_style>{move || t.get().nav_hint}</div>
                    </Section>

                    <Section title=Signal::derive(move || t.get().advanced) collapsed=true>
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().compute}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {[("gpu", "GPU"), ("cpu", "CPU")].into_iter().map(|(id, label)| view! {
                                    <button style=move || {
                                        let on = settings.get().compute == id || (id == "cpu" && !webgpu.get());
                                        format!("{} opacity: {};", seg_style(on), if id == "gpu" && !webgpu.get() { 0.4 } else { 1.0 })
                                    } disabled=move || id == "gpu" && !webgpu.get()
                                        on:click=move |_| upd(&|s| s.compute = id.into())>{label}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().per_pixel}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {[64u32, 256, 768, 2048].into_iter().map(|n| view! {
                                    <button style=move || seg_style(settings.get().samples == n) on:click=move |_| upd(&|s| s.samples = n)>{n.to_string()}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().per_point}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {[256u32, 1024, 4096].into_iter().map(|n| view! {
                                    <button style=move || seg_style(settings.get().point_samples == n) on:click=move |_| upd(&|s| s.point_samples = n)>{n.to_string()}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="color: var(--text-secondary); width: 70px;">{move || t.get().per_frame}</span>
                            <div style="display: flex; gap: 3px; flex: 1;">
                                {[1u32, 2, 4].into_iter().map(|n| view! {
                                    <button style=move || seg_style(settings.get().passes == n) on:click=move |_| upd(&|s| s.passes = n)>{n.to_string()}</button>
                                }).collect_view()}
                            </div>
                        </div>
                        <div style=desc_style>
                            {move || {
                                let s = status.get();
                                fill(&t.get().samples_line, &[
                                    ("{s}", s.samples.to_string()), ("{smax}", s.max_samples.to_string()),
                                    ("{p}", s.probe_samples.to_string()), ("{pmax}", s.probe_max_samples.to_string()),
                                    ("{key}", format!("{:.0}", s.key_lux)),
                                ])
                            }}
                            <br/>
                            {move || t.get().cpu_note}
                        </div>
                    </Section>
                </div>
            })}
        </div>
    }
}

/// The moon over an outdoor scene: on / off, the night, the place (globe),
/// north in the scene, luminaires off, and what the moon gives.
#[component]
fn MoonPanel(
    settings: ReadSignal<SyncSettings>,
    set_settings: WriteSignal<SyncSettings>,
    status: ReadSignal<Status>,
) -> impl IntoView {
    use crate::components::location_picker::{GlobePicker, PickedPlace};
    let t = Memo::new(move |_| crate::i18n::use_locale().get().ui.light_scene.clone());
    let upd = move |f: &dyn Fn(&mut SyncSettings)| set_settings.update(|s| f(s));
    let desc_style = "font-size: 10.5px; color: var(--text-secondary); line-height: 1.4;";
    let place = RwSignal::new(PickedPlace::load_last());
    let globe = RwSignal::new(false);
    let dst_key = |d: eulumdat_daylight::DstRule| match d {
        eulumdat_daylight::DstRule::Eu => "eu",
        eulumdat_daylight::DstRule::Us => "us",
        eulumdat_daylight::DstRule::SouthernEu => "south",
        eulumdat_daylight::DstRule::None => "none",
    };
    let set_place = move |p: &PickedPlace| {
        let (utc, dst) = p.clock();
        let (lat, lon) = (p.lat, p.lon);
        upd(&|s| {
            s.moon_lat = lat;
            s.moon_lon = lon;
            s.moon_utc = utc as f32;
            s.moon_dst = dst_key(dst).into();
        });
    };
    // switched on: tonight at the last place picked on the globe
    let switch_on = move |on: bool| {
        if on {
            let d = js_sys::Date::new_0();
            let today = format!(
                "{:04}-{:02}-{:02}",
                d.get_full_year(),
                d.get_month() + 1,
                d.get_date()
            );
            upd(&|s| {
                s.moon = true;
                s.moon_date = today.clone();
            });
            if let Some(p) = place.get_untracked() {
                set_place(&p);
            }
        } else {
            upd(&|s| s.moon = false);
        }
    };
    let phase_icon = |f: f32, waxing: bool| match (f, waxing) {
        (f, _) if f < 0.04 => "🌑",
        (f, _) if f > 0.96 => "🌕",
        (f, true) if f < 0.4 => "🌒",
        (f, true) if f < 0.6 => "🌓",
        (_, true) => "🌔",
        (f, false) if f < 0.4 => "🌘",
        (f, false) if f < 0.6 => "🌗",
        _ => "🌖",
    };
    let lx = |v: f32| crate::components::moon_outdoor::lx(f64::from(v));
    view! {
        <Section title=Signal::derive(move || t.get().moon_title) collapsed=true>
            <Check label=Signal::derive(move || t.get().moon_on) checked=Signal::derive(move || settings.get().moon)
                on_change=move |v| switch_on(v) />
            <div style=desc_style>{move || t.get().moon_desc}</div>
            {move || (!status.get().outdoor).then(|| view! { <div style=desc_style>{move || t.get().moon_indoor}</div> })}
            {move || settings.get().moon.then(|| view! {
                <div style="display: flex; align-items: center; gap: 6px; font-size: 11px;">
                    <span style="color: var(--text-secondary); width: 70px;">{move || t.get().moon_date}</span>
                    <input type="date" style="flex: 1; padding: 3px 5px; background: var(--surface-elevated); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px;"
                        prop:value=move || settings.get().moon_date
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            if eulumdat_scene::moon::parse_date(&v).is_some() {
                                upd(&|s| s.moon_date = v.clone());
                            }
                        } />
                    <button style="padding: 3px 6px; font-size: 10.5px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer;"
                        on:click=move |_| {
                            if let Some(d) = settings.get_untracked().moon_time().and_then(|mt| eulumdat_scene::moon::next_full_moon(&mt)) {
                                upd(&|s| s.moon_date = d.clone());
                            }
                        }>{move || t.get().moon_full}</button>
                </div>
                <Range label=t.get_untracked().moon_time unit="h" value=settings.get_untracked().moon_hour min=0.0 max=23.75 step=0.25
                    on_change=move |v| upd(&|s| s.moon_hour = v) />
                <div style="display: flex; align-items: center; gap: 6px; font-size: 11px;">
                    <span style="flex: 1;">{move || fill(&t.get().moon_place, &[("{place}", place.get().map(|p| p.label()).unwrap_or_else(|| format!("{:.2}°, {:.2}°", settings.get().moon_lat, settings.get().moon_lon)))])}</span>
                    <button style="padding: 3px 6px; font-size: 10.5px; background: var(--surface-elevated); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); cursor: pointer;"
                        on:click=move |_| globe.set(true)>{move || t.get().moon_pick}</button>
                </div>
                <GlobePicker open=globe
                    at=Signal::derive(move || (settings.get().moon_lat, settings.get().moon_lon))
                    on_pick=Callback::new(move |p: PickedPlace| { set_place(&p); place.set(Some(p)); }) />
                <Range label=t.get_untracked().moon_north unit="°" value=settings.get_untracked().north_deg min=0.0 max=355.0 step=5.0
                    on_change=move |v| upd(&|s| s.north_deg = v) />
                <Check label=Signal::derive(move || t.get().moon_lamps_off) checked=Signal::derive(move || settings.get().lamps_off)
                    on_change=move |v| upd(&|s| s.lamps_off = v) />
                <div style="font-size: 11px; color: #79c0ff; font-variant-numeric: tabular-nums;">
                    {move || {
                        use eulumdat_scene::moon::SkyKind;
                        let tt = t.get();
                        match status.get().sky {
                            Some(sk) if sk.kind == SkyKind::Sun => fill(&tt.sun_readout, &[
                                ("{alt}", format!("{:.0}", sk.sun_altitude_deg)),
                                ("{lx}", lx(sk.horizontal_lux)),
                                ("{beam}", lx(sk.beam_lux)),
                                ("{sky}", lx(sk.sky_lux)),
                            ]),
                            Some(sk) if sk.kind == SkyKind::Twilight => fill(&tt.twilight_readout, &[
                                ("{alt}", format!("{:.0}", sk.sun_altitude_deg)),
                                ("{lx}", lx(sk.horizontal_lux)),
                            ]),
                            Some(sk) if sk.moon.altitude_deg > 0.0 => {
                                let m = sk.moon;
                                fill(&tt.moon_readout, &[
                                    ("{phase}", phase_icon(m.illuminated, m.waxing).to_string()),
                                    ("{pct}", format!("{:.0}", m.illuminated * 100.0)),
                                    ("{alt}", format!("{:.0}", m.altitude_deg)),
                                    ("{lx}", lx(m.horizontal_lux)),
                                    ("{beam}", lx(m.normal_lux)),
                                    ("{sky}", lx(m.sky_lux)),
                                ])
                            }
                            Some(sk) => format!("{} {}", tt.moon_below, fill(&tt.night_readout, &[("{lx}", lx(sk.horizontal_lux))])),
                            None => String::new(),
                        }
                    }}
                </div>
            })}
        </Section>
    }
}

/// Emergency lighting of a planned scene: emergency mode, the plan's
/// markings on / off (off = as a person escaping sees it), walking a route
/// at eye height, and the EN 1838 check of the measured points.
#[component]
fn EmergencyPanel(
    settings: ReadSignal<SyncSettings>,
    set_settings: WriteSignal<SyncSettings>,
    status: ReadSignal<Status>,
    plan: Signal<Option<Arc<ScenePlan>>>,
) -> impl IntoView {
    let t = Memo::new(move |_| use_locale().get().ui.light_scene.clone());
    let upd = move |f: &dyn Fn(&mut SyncSettings)| set_settings.update(|s| f(s));
    let desc_style = "font-size: 10.5px; color: var(--text-secondary); line-height: 1.4;";
    let btn = "padding: 4px 8px; font-size: 11px; cursor: pointer; border: 1px solid var(--border); background: var(--surface-elevated); color: var(--text-primary); border-radius: 4px;";
    let seg_style = |on: bool| {
        format!(
            "flex: 1; padding: 4px; font-size: 11px; cursor: pointer; border: 1px solid var(--border); background: {}; color: {}; border-radius: 4px;",
            if on { "var(--primary-color)" } else { "var(--surface-elevated)" },
            if on { "#fff" } else { "var(--text-primary)" }
        )
    };
    // position along the walked route: the viewer's while it walks on
    let walk_pos = move || status.get().walk.or(settings.get().walk).unwrap_or(0.0);
    let routes = Memo::new(move |_| status.get().routes);
    let playing = Memo::new(move |_| status.get().walk_playing);
    let walking = Memo::new(move |_| status.get().walk.is_some());
    // in steps of 1 %: the text changes, the panel does not rebuild per tick
    let adaptation = Memo::new(move |_| (status.get().adaptation * 100.0).round() / 100.0);
    // what the plan's standard requires (EN 1838 when the plan says nothing)
    let standard = Memo::new(move |_| {
        plan.get()
            .and_then(|p| p.emergency.as_ref().map(|e| e.standard.clone()))
            .unwrap_or_default()
    });
    let checks = move || {
        let tt = t.get();
        let st = status.get();
        let s = settings.get();
        if !s.emergency_mode {
            return view! { <div style=desc_style>{tt.em_off_hint.clone()}</div> }.into_any();
        }
        if s.mode != "points" {
            return view! {
                <button style=btn on:click=move |_| upd(&|s| s.mode = "points".into())>{tt.em_measure.clone()}</button>
            }
            .into_any();
        }
        let req = standard.get();
        let find = |name: &str| st.grids.iter().find(|g| g.name == name).cloned();
        // a grid's minimum together with its band edges (the central band)
        let with_band = |centre: &str, band: &str| match (find(centre), find(band)) {
            (Some(c), Some(b)) => Some((c.emin.min(b.emin), c.emax.max(b.emax))),
            (Some(c), None) => Some((c.emin, c.emax)),
            _ => None,
        };
        // (label, min, max, average, required min, max ratio, required
        // average; 0 = no such rule)
        let mut checks: Vec<(String, f32, f32, f32, f32, f32, f32)> = Vec::new();
        let route_avg = |c: &GridStat, b: Option<&GridStat>| match b {
            // the band grid holds both edges: weigh it twice
            Some(b) => (c.em + 2.0 * b.em) / 3.0,
            None => c.em,
        };
        let band_ratio = if req.ratio_over_band {
            req.route_ratio
        } else {
            0.0
        };
        if let Some(g) = find(GRID_ROUTE).filter(|_| req.route_centre_lx > 0.0) {
            // EN 1838: the centre line (+ its ratio)
            let ratio = if req.ratio_over_band {
                0.0
            } else {
                req.route_ratio
            };
            checks.push((
                tt.em_check_route.clone(),
                g.emin,
                g.emax,
                0.0,
                req.route_centre_lx,
                ratio,
                0.0,
            ));
        }
        if let Some((mn, mx)) =
            with_band(GRID_ROUTE, GRID_ROUTE_BAND).filter(|_| req.route_band_lx > 0.0)
        {
            let avg =
                find(GRID_ROUTE).map_or(0.0, |c| route_avg(&c, find(GRID_ROUTE_BAND).as_ref()));
            checks.push((
                tt.em_check_band.clone(),
                mn,
                mx,
                avg,
                req.route_band_lx,
                band_ratio,
                req.route_avg_lx,
            ));
        }
        if let Some((mn, mx)) = with_band(GRID_STAIRS, GRID_STAIRS_BAND) {
            checks.push((
                tt.em_check_stairs.clone(),
                mn,
                mx,
                0.0,
                req.stairs_lx,
                0.0,
                0.0,
            ));
        }
        if let Some(g) = find(GRID_OPEN_AREA) {
            checks.push((
                if req.area_border_m > 0.0 {
                    tt.em_check_open.clone()
                } else {
                    tt.em_check_room.clone()
                },
                g.emin,
                g.emax,
                g.em,
                req.area_lx,
                req.area_ratio,
                req.area_avg_lx,
            ));
        }
        if let Some(g) = find(GRID_EMPHASIS).filter(|_| req.vertical_lx > 0.0) {
            checks.push((
                tt.em_check_points.clone(),
                g.emin,
                g.emax,
                0.0,
                req.vertical_lx,
                0.0,
                0.0,
            ));
        }
        let rows: Vec<(String, bool, String)> = checks
            .into_iter()
            .map(|(label, emin, emax, avg, min, ratio, avg_req)| {
                let mut ok = emin >= min;
                let mut text = fill(
                    &tt.em_check_min,
                    &[("{v}", fmt_lux(emin)), ("{req}", format!("{min}"))],
                );
                if avg_req > 0.0 {
                    ok &= avg >= avg_req;
                    text.push_str(" · ");
                    text.push_str(&fill(
                        &tt.em_check_avg,
                        &[("{v}", fmt_lux(avg)), ("{req}", format!("{avg_req}"))],
                    ));
                }
                if ratio > 0.0 {
                    let v = if emin > 0.0 {
                        emax / emin
                    } else {
                        f32::INFINITY
                    };
                    ok &= v <= ratio;
                    let shown = if v.is_finite() {
                        format!("{v:.1}")
                    } else {
                        "∞".into()
                    };
                    text.push_str(" · ");
                    text.push_str(&fill(
                        &tt.em_check_ratio,
                        &[("{v}", shown), ("{req}", format!("{ratio}"))],
                    ));
                }
                (label, ok, text)
            })
            .collect();
        if rows.is_empty() {
            return view! { <div style=desc_style>{tt.measuring.clone()}</div> }.into_any();
        }
        view! {
            <div style="display: flex; flex-direction: column; gap: 4px; font-size: 11px; font-variant-numeric: tabular-nums;">
                {rows.into_iter().map(|(label, ok, text)| view! {
                    <div>
                        <span style=if ok { "color: #16a34a; font-weight: 600;" } else { "color: #dc2626; font-weight: 600;" }>{if ok { "✓ " } else { "✗ " }}</span>
                        <span>{label}</span>
                        <div style="color: var(--text-secondary); padding-left: 14px;">{text}</div>
                    </div>
                }).collect_view()}
                <div style=desc_style>{tt.em_check_note.clone()}</div>
            </div>
        }
        .into_any()
    };
    view! {
        <Section title=Signal::derive(move || format!("{} · {}", t.get().em_title, standard.get().name))>
            <Check label=Signal::derive(move || t.get().em_mode) checked=Signal::derive(move || settings.get().emergency_mode)
                on_change=move |v| upd(&|s| s.emergency_mode = v) />
            <div style=desc_style>{move || t.get().em_mode_desc}</div>
            {move || settings.get().emergency_mode.then(|| view! {
                <div style="display: flex; flex-direction: column; gap: 3px;">
                    <div style="font-size: 11px;">
                        {move || {
                            let a = adaptation.get();
                            let tt = t.get();
                            if a < 0.98 {
                                fill(&tt.em_adapting, &[("{pct}", format!("{:.0}", a * 100.0))])
                            } else {
                                fill(&tt.em_adapted, &[("{stops}", format!("{:.0}", a * eulumdat_scene::viewer::EMERGENCY_ADAPT_STOPS))])
                            }
                        }}
                    </div>
                    <button style=btn on:click=move |_| upd(&|s| s.emergency_replay = s.emergency_replay.wrapping_add(1))>
                        {move || t.get().em_replay}
                    </button>
                </div>
            })}
            <Check label=Signal::derive(move || t.get().em_marks) checked=Signal::derive(move || settings.get().emergency_marks)
                on_change=move |v| upd(&|s| s.emergency_marks = v) />
            {move || (!settings.get().emergency_marks).then(|| view! { <div style=desc_style>{move || t.get().em_marks_off}</div> })}
            {move || {
                let n = routes.with(|r| r.len());
                (n > 0).then(|| {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 4px; margin-top: 4px;">
                            <span style="font-size: 11.5px;">{move || t.get().em_walk}</span>
                            {(n > 1).then(|| view! {
                                <div style="display: flex; gap: 3px; flex-wrap: wrap;">
                                    {(0..n).map(|i| view! {
                                        <button style=move || seg_style(settings.get().walk_route as usize == i)
                                            on:click=move |_| upd(&|s| { s.walk_route = i as u32; s.walk = Some(0.0); s.walk_play = false; })>
                                            {move || fill(&t.get().em_route, &[("{n}", (i + 1).to_string())])}
                                        </button>
                                    }).collect_view()}
                                </div>
                            })}
                            <input type="range" min="0" max="1" step="0.005" style="width: 100%;"
                                aria-label=move || t.get().em_walk
                                prop:value=move || walk_pos().to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                        upd(&|s| { s.walk = Some(v); s.walk_play = false; });
                                    }
                                } />
                            <div style=desc_style>
                                {move || {
                                    let len = routes.with(|r| r.get(settings.get().walk_route as usize).copied().unwrap_or(0.0));
                                    fill(&t.get().em_walk_at, &[("{m}", format!("{:.1}", walk_pos() * len)), ("{len}", format!("{len:.1}"))])
                                }}
                            </div>
                            <div style="display: flex; gap: 4px;">
                                {move || {
                                    let playing = playing.get();
                                    view! {
                                        <button style=btn on:click=move |_| {
                                            let pos = status.get_untracked().walk.or(settings.get_untracked().walk).unwrap_or(0.0);
                                            if playing {
                                                upd(&|s| { s.walk = Some(pos); s.walk_play = false; });
                                            } else {
                                                let from = if pos >= 0.999 { 0.0 } else { pos };
                                                upd(&|s| { s.walk = Some(from); s.walk_play = true; });
                                            }
                                        }>
                                            {move || if playing { t.get().em_walk_pause } else { t.get().em_walk_play }}
                                        </button>
                                    }
                                }}
                                {move || walking.get().then(|| view! {
                                    <button style=btn on:click=move |_| upd(&|s| { s.walk = None; s.walk_play = false; })>
                                        {move || t.get().em_walk_exit}
                                    </button>
                                })}
                            </div>
                        </div>
                    }
                })
            }}
            {checks}
        </Section>
    }
}

#[component]
fn Section(
    title: Signal<String>,
    #[prop(default = false)] collapsed: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let (open, set_open) = signal(!collapsed);
    view! {
        <div style="display: flex; flex-direction: column; gap: 6px; padding-top: 6px; border-top: 1px solid var(--border);">
            <button
                style="display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 0; background: none; border: none; cursor: pointer; font-size: 10px; letter-spacing: 2px; text-transform: uppercase; color: var(--text-secondary); text-align: left;"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                <span>{move || title.get()}</span>
                <span style="letter-spacing: 0;">{move || if open.get() { "−" } else { "+" }}</span>
            </button>
            {move || open.get().then(|| children())}
        </div>
    }
}

#[component]
fn Range(
    label: String,
    unit: &'static str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    on_change: impl Fn(f32) + 'static,
) -> impl IntoView {
    let (shown, set_shown) = signal(value);
    let digits = if step >= 1.0 {
        0
    } else if step >= 0.1 {
        1
    } else {
        2
    };
    let aria = label.clone();
    view! {
        <div class="control-group" style="display: flex; flex-direction: column; gap: 2px;">
            <div style="display: flex; justify-content: space-between; font-size: 11px;">
                <span style="color: var(--text-secondary);">{label}</span>
                <span style="font-variant-numeric: tabular-nums;">{move || format!("{:.*} {}", digits, shown.get(), unit)}</span>
            </div>
            <input type="range" min=min max=max step=step value=value aria-label=aria style="width: 100%;"
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                        set_shown.set(v);
                        on_change(v);
                    }
                } />
        </div>
    }
}

#[component]
fn Check(
    label: Signal<String>,
    checked: Signal<bool>,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 11.5px;">
            <input type="checkbox" prop:checked=move || checked.get() style="width: 14px; height: 14px;"
                on:change=move |ev| on_change(event_target::<web_sys::HtmlInputElement>(&ev).checked()) />
            <span>{move || label.get()}</span>
        </label>
    }
}
