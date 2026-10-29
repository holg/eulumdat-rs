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
use eulumdat_scene::scene::{suggest_scene, SceneKind};
use eulumdat_scene::sync::{scene_id, Status, SyncSettings};
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
    wasm_bindgen_futures::spawn_local(async move {
        let result = init_gpu(canvas).await;
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
                s.note = "GPU unavailable, tracing on the CPU".into();
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
            draw_overlay(&overlay, &s.viewer, css_w, css_h, dpr);
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
    let table = viewer.photometry.as_ref().map(|p| &p.table);
    gpu.renderer
        .render(&gpu.device, &gpu.queue, &input, table, &view);
    texture.present();
    if let Some(values) = gpu.renderer.take_probe_values(&gpu.device) {
        viewer.probe_values = values;
    }
    viewer.samples = gpu.renderer.samples();
    viewer.probe_samples = gpu.renderer.probe_samples();

    // Measurement points on the 2D overlay.
    if let Some(overlay) = s.overlay.clone() {
        draw_overlay(&overlay, &s.viewer, css_w, css_h, dpr);
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
    let Some(table) = viewer.photometry.as_ref().map(|p| &p.table) else {
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
    let (link_settings, link_scene) = use_context::<super::app::RtLinkParams>()
        .map(|p| {
            let pairs = p.0.get_value();
            let mut s = SyncSettings::default();
            let any = s.apply_query(&pairs);
            (
                if any { s } else { SyncSettings::default() },
                pairs.iter().any(|(k, _)| k == "scene"),
            )
        })
        .unwrap_or((SyncSettings::default(), false));
    let (settings, set_settings) = signal(link_settings);
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
                set_settings.set(SyncSettings::for_scene(kind));
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
                let (st, n, stalled) = shared_sv.with_value(|sh| {
                    let Ok(s) = sh.try_borrow() else {
                        return (None, None, false);
                    };
                    let stalled = s.last_frame > 0.0 && now - s.last_frame > 1500.0;
                    (Some(s.viewer.status()), Some(s.note.clone()), stalled)
                });
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

    let t = Memo::new(move |_| use_locale().get().ui.light_scene.clone());
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
            <RtControlPanel settings=settings set_settings=set_settings status=status webgpu=webgpu scene_touched=scene_touched />

            <div style="flex: 1; position: relative; min-width: 0; background: #07080a; border-radius: 0 8px 8px 0; overflow: hidden;">
                <canvas
                    node_ref=canvas_ref
                    style="width: 100%; height: 100%; display: block; touch-action: none; cursor: grab;"
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
                    }.into_any(),
                    LoadState::Unsupported => render_webgpu_missing(browser.get_value()).into_any(),
                }}
                {move || (!webgpu.get() && load_state.get() == LoadState::Running).then(|| view! {
                    <div style="position: absolute; top: 8px; right: 8px; background: rgba(0,0,0,0.6); color: #ccc; font-size: 11px; padding: 4px 8px; border-radius: 6px; pointer-events: none;">
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
) -> impl IntoView {
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

                    <Section title=Signal::derive(move || t.get().step_scene)>
                        <div style="display: flex; gap: 4px;">
                            {[SceneKind::Street, SceneKind::Parking, SceneKind::Facade, SceneKind::Office, SceneKind::Greenhouse].into_iter().map(|kind| {
                                let id = scene_id(kind);
                                let label = move || {
                                    let tt = t.get();
                                    match kind { SceneKind::Street => tt.scene_street, SceneKind::Parking => tt.scene_parking, SceneKind::Facade => tt.scene_facade, SceneKind::Office => tt.scene_office, SceneKind::Greenhouse => tt.scene_greenhouse }
                                };
                                view! {
                                    <button style=move || seg_style(settings.get().scene == id)
                                        on:click=move |_| { scene_touched.set_value(true); upd(&|s| { if s.scene != id { *s = SyncSettings::for_scene(kind); } }) }>
                                        {label}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        <div style=desc_style>
                            {move || {
                                let tt = t.get();
                                match settings.get().scene.as_str() { "parking" => tt.desc_parking, "facade" => tt.desc_facade, "office" => tt.desc_office, "greenhouse" => tt.desc_greenhouse, _ => tt.desc_street }
                            }}
                        </div>
                    </Section>

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
                        <Check label=Signal::derive(move || t.get().person_for_scale) checked=Signal::derive(move || settings.get().show_figure) on_change=move |v| upd(&|s| s.show_figure = v) />
                    </Section>

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
