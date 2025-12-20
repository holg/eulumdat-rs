//! Real 3D Butterfly diagram component with Canvas rendering
//! Features: auto-rotation animation, mouse drag rotation controls

use crate::i18n::use_locale;
use eulumdat::{Eulumdat, Symmetry};
use leptos::ev;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Theme colors for canvas rendering (read from CSS variables)
#[derive(Clone)]
struct ThemeColors {
    bg: String,
    grid: String,
    text: String,
    text_dark: String,
    center: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg: "#ffffff".to_string(),
            grid: "#e0e0e0".to_string(),
            text: "#666666".to_string(),
            text_dark: "#333333".to_string(),
            center: "#333333".to_string(),
        }
    }
}

/// Read CSS variable value from the themed .app element
fn get_css_variable(name: &str) -> Option<String> {
    let js_code = format!(
        "(function() {{ \
            var el = document.querySelector('.app'); \
            if (!el) el = document.documentElement; \
            return getComputedStyle(el).getPropertyValue('{}'); \
        }})()",
        name
    );
    if let Ok(result) = js_sys::eval(&js_code) {
        if let Some(value) = result.as_string() {
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

fn get_theme_colors() -> ThemeColors {
    ThemeColors {
        bg: get_css_variable("--diagram-bg").unwrap_or_else(|| "#ffffff".to_string()),
        grid: get_css_variable("--diagram-grid").unwrap_or_else(|| "#e0e0e0".to_string()),
        text: get_css_variable("--diagram-text-secondary").unwrap_or_else(|| "#666666".to_string()),
        text_dark: get_css_variable("--diagram-text").unwrap_or_else(|| "#333333".to_string()),
        center: get_css_variable("--diagram-text").unwrap_or_else(|| "#333333".to_string()),
    }
}

/// 3D point
#[derive(Clone, Copy)]
struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

impl Point3D {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn rotate_x(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x,
            y: self.y * cos_a - self.z * sin_a,
            z: self.y * sin_a + self.z * cos_a,
        }
    }

    fn rotate_y(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a + self.z * sin_a,
            y: self.y,
            z: -self.x * sin_a + self.z * cos_a,
        }
    }

    fn rotate_z(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a - self.y * sin_a,
            y: self.x * sin_a + self.y * cos_a,
            z: self.z,
        }
    }

    fn project(self, cx: f64, cy: f64, scale: f64) -> (f64, f64) {
        let perspective = 600.0;
        let z_offset = 300.0;
        let factor = perspective / (perspective + self.z + z_offset);
        (cx + self.x * scale * factor, cy - self.y * scale * factor)
    }
}

struct Wing {
    #[allow(dead_code)]
    c_angle: f64,
    points: Vec<Point3D>,
    color_hue: f64,
}

/// Localized strings for canvas rendering
#[derive(Clone, Default)]
struct LocalizedStrings {
    title: String,
    max_label: String,
    drag_hint: String,
}

struct Renderer3D {
    rotation_x: f64,
    rotation_y: f64,
    rotation_z: f64,
    wings: Vec<Wing>,
    max_intensity: f64,
    theme_colors: ThemeColors,
    localized_strings: LocalizedStrings,
}

impl Renderer3D {
    fn new() -> Self {
        Self {
            rotation_x: 0.5,
            rotation_y: 0.0,
            rotation_z: 0.0,
            wings: Vec::new(),
            max_intensity: 100.0,
            theme_colors: ThemeColors::default(),
            localized_strings: LocalizedStrings::default(),
        }
    }

    fn update_localized_strings(&mut self, title: String, max_label: String, drag_hint: String) {
        self.localized_strings.title = title;
        self.localized_strings.max_label = max_label;
        self.localized_strings.drag_hint = drag_hint;
    }

    fn update_theme(&mut self) {
        self.theme_colors = get_theme_colors();
    }

    fn build_wings(&mut self, ldt: &Eulumdat) {
        self.wings.clear();

        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return;
        }

        self.max_intensity = ldt
            .intensities
            .iter()
            .flat_map(|plane| plane.iter())
            .copied()
            .fold(0.0_f64, f64::max)
            .max(1.0);

        let c_plane_data = expand_c_planes(ldt);

        for (c_angle, intensities) in c_plane_data {
            let c_rad = c_angle.to_radians();
            let mut points = Vec::new();

            points.push(Point3D::new(0.0, 0.0, 0.0));

            for (j, &g_angle) in ldt.g_angles.iter().enumerate() {
                let intensity = intensities.get(j).copied().unwrap_or(0.0);
                let r = intensity / self.max_intensity;

                let g_rad = g_angle.to_radians();

                let x = r * g_rad.sin() * c_rad.cos();
                let y = r * g_rad.sin() * c_rad.sin();
                let z = r * g_rad.cos();

                points.push(Point3D::new(x, y, -z));
            }

            let color_hue = (c_angle / 360.0) * 240.0 + 180.0;

            self.wings.push(Wing {
                c_angle,
                points,
                color_hue: color_hue % 360.0,
            });
        }
    }

    fn render(&self, ctx: &CanvasRenderingContext2d, width: f64, height: f64) {
        let cx = width / 2.0;
        let cy = height / 2.0;
        let scale = (width.min(height) / 2.0) * 0.7;

        ctx.set_fill_style_str(&self.theme_colors.bg);
        ctx.fill_rect(0.0, 0.0, width, height);

        self.draw_grid(ctx, cx, cy, scale);

        let mut wing_depths: Vec<(usize, f64)> = self
            .wings
            .iter()
            .enumerate()
            .map(|(i, wing)| {
                let avg_z: f64 = wing
                    .points
                    .iter()
                    .map(|p| {
                        let rotated = p
                            .rotate_x(self.rotation_x)
                            .rotate_y(self.rotation_y)
                            .rotate_z(self.rotation_z);
                        rotated.z
                    })
                    .sum::<f64>()
                    / wing.points.len() as f64;
                (i, avg_z)
            })
            .collect();

        wing_depths.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (idx, _) in wing_depths {
            self.draw_wing(ctx, &self.wings[idx], cx, cy, scale);
        }

        ctx.begin_path();
        ctx.set_fill_style_str(&self.theme_colors.center);
        let _ = ctx.arc(cx, cy, 4.0, 0.0, std::f64::consts::TAU);
        ctx.fill();

        self.draw_labels(ctx, width, height);
    }

    fn draw_grid(&self, ctx: &CanvasRenderingContext2d, cx: f64, cy: f64, scale: f64) {
        ctx.set_stroke_style_str(&self.theme_colors.grid);
        ctx.set_line_width(1.0);

        for i in 1..=4 {
            let r = (i as f64) / 4.0;
            ctx.begin_path();

            for j in 0..=36 {
                let c_angle = (j as f64) * 10.0;
                let c_rad = c_angle.to_radians();

                let point = Point3D::new(r * c_rad.cos(), r * c_rad.sin(), 0.0);

                let rotated = point
                    .rotate_x(self.rotation_x)
                    .rotate_y(self.rotation_y)
                    .rotate_z(self.rotation_z);

                let (px, py) = rotated.project(cx, cy, scale);

                if j == 0 {
                    ctx.move_to(px, py);
                } else {
                    ctx.line_to(px, py);
                }
            }
            ctx.close_path();
            ctx.stroke();
        }

        for i in 0..8 {
            let c_angle = (i as f64) * 45.0;
            let c_rad = c_angle.to_radians();

            ctx.begin_path();

            let p1 = Point3D::new(0.0, 0.0, 0.0)
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .rotate_z(self.rotation_z);
            let (x1, y1) = p1.project(cx, cy, scale);

            let p2 = Point3D::new(c_rad.cos(), c_rad.sin(), 0.0)
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .rotate_z(self.rotation_z);
            let (x2, y2) = p2.project(cx, cy, scale);

            ctx.move_to(x1, y1);
            ctx.line_to(x2, y2);
            ctx.stroke();

            let label_p = Point3D::new(c_rad.cos() * 1.1, c_rad.sin() * 1.1, 0.0)
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .rotate_z(self.rotation_z);
            let (lx, ly) = label_p.project(cx, cy, scale);

            ctx.set_fill_style_str(&self.theme_colors.text);
            ctx.set_font("10px system-ui, sans-serif");
            ctx.set_text_align("center");
            ctx.set_text_baseline("middle");
            let _ = ctx.fill_text(&format!("C{}", (c_angle as i32)), lx, ly);
        }
    }

    fn draw_wing(&self, ctx: &CanvasRenderingContext2d, wing: &Wing, cx: f64, cy: f64, scale: f64) {
        if wing.points.len() < 2 {
            return;
        }

        ctx.begin_path();

        for (i, point) in wing.points.iter().enumerate() {
            let rotated = point
                .rotate_x(self.rotation_x)
                .rotate_y(self.rotation_y)
                .rotate_z(self.rotation_z);

            let (px, py) = rotated.project(cx, cy, scale);

            if i == 0 {
                ctx.move_to(px, py);
            } else {
                ctx.line_to(px, py);
            }
        }

        ctx.close_path();

        let (r, g, b) = hsl_to_rgb(wing.color_hue / 360.0, 0.6, 0.5);
        ctx.set_fill_style_str(&format!("rgba({}, {}, {}, 0.5)", r, g, b));
        ctx.fill();

        let (r, g, b) = hsl_to_rgb(wing.color_hue / 360.0, 0.7, 0.6);
        ctx.set_stroke_style_str(&format!("rgb({}, {}, {})", r, g, b));
        ctx.set_line_width(1.5);
        ctx.stroke();
    }

    fn draw_labels(&self, ctx: &CanvasRenderingContext2d, width: f64, height: f64) {
        ctx.set_fill_style_str(&self.theme_colors.text_dark);
        ctx.set_font("12px system-ui, sans-serif");
        ctx.set_text_align("left");
        ctx.set_text_baseline("top");
        let title = if self.localized_strings.title.is_empty() {
            "3D Butterfly Diagram".to_string()
        } else {
            self.localized_strings.title.clone()
        };
        let _ = ctx.fill_text(&title, 15.0, 15.0);

        ctx.set_fill_style_str(&self.theme_colors.text);
        ctx.set_font("11px system-ui, sans-serif");
        ctx.set_text_align("right");
        let _ = ctx.fill_text("cd/klm", width - 15.0, height - 20.0);

        ctx.set_text_align("left");
        let max_label = if self.localized_strings.max_label.is_empty() {
            "Max".to_string()
        } else {
            self.localized_strings.max_label.clone()
        };
        let _ = ctx.fill_text(
            &format!("{}: {:.0}", max_label, self.max_intensity),
            15.0,
            height - 20.0,
        );

        ctx.set_font("10px system-ui, sans-serif");
        ctx.set_text_align("right");
        ctx.set_text_baseline("top");
        let drag_hint = if self.localized_strings.drag_hint.is_empty() {
            "Drag to rotate".to_string()
        } else {
            self.localized_strings.drag_hint.clone()
        };
        let _ = ctx.fill_text(&drag_hint, width - 15.0, 15.0);
    }
}

fn expand_c_planes(ldt: &Eulumdat) -> Vec<(f64, Vec<f64>)> {
    if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let c_start = match ldt.symmetry {
        Symmetry::PlaneC90C270 => ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0),
        _ => 0,
    };

    match ldt.symmetry {
        Symmetry::VerticalAxis => {
            let intensities = &ldt.intensities[0];
            for i in 0..12 {
                let c_angle = i as f64 * 30.0;
                result.push((c_angle, intensities.clone()));
            }
        }
        Symmetry::PlaneC0C180 => {
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 0.0 && c_angle < 180.0 {
                        result.push((360.0 - c_angle, intensities.clone()));
                    }
                }
            }
        }
        Symmetry::PlaneC90C270 => {
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(i) {
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 0.0 && c_angle < 180.0 {
                        let mirrored = 360.0 - c_angle;
                        result.push((mirrored, intensities.clone()));
                    }
                }
            }
        }
        Symmetry::BothPlanes => {
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 0.0 && c_angle < 90.0 {
                        result.push((180.0 - c_angle, intensities.clone()));
                        result.push((180.0 + c_angle, intensities.clone()));
                        result.push((360.0 - c_angle, intensities.clone()));
                    } else if (c_angle - 90.0).abs() < 0.1 {
                        result.push((270.0, intensities.clone()));
                    }
                }
            }
        }
        Symmetry::None => {
            for (i, intensities) in ldt.intensities.iter().enumerate() {
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push((c_angle, intensities.clone()));
                }
            }
        }
    }

    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    result
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[component]
pub fn Butterfly3D(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let renderer = Rc::new(RefCell::new(Renderer3D::new()));

    let (auto_rotate, set_auto_rotate) = signal(true);
    let (dragging, set_dragging) = signal(false);
    let (last_mouse_x, set_last_mouse_x) = signal(0.0_f64);
    let (last_mouse_y, set_last_mouse_y) = signal(0.0_f64);
    let (start_time, set_start_time) = signal::<Option<f64>>(None);

    // Build wings when LDT changes
    Effect::new({
        let renderer = renderer.clone();
        move |_| {
            let ldt = ldt.get();
            renderer.borrow_mut().build_wings(&ldt);
        }
    });

    // Update localized strings when locale changes
    Effect::new({
        let renderer = renderer.clone();
        move |_| {
            let l = locale.get();
            renderer.borrow_mut().update_localized_strings(
                l.ui.diagram.title_3d.clone(),
                l.ui.butterfly.max.clone(),
                l.ui.butterfly.drag_hint.clone(),
            );
        }
    });

    // Animation loop
    Effect::new({
        let renderer = renderer.clone();
        move |_| {
            let renderer = renderer.clone();

            let animate = Rc::new(RefCell::new(
                None::<wasm_bindgen::closure::Closure<dyn Fn()>>,
            ));
            let animate_clone = animate.clone();

            *animate.borrow_mut() = Some(wasm_bindgen::closure::Closure::new({
                let renderer = renderer.clone();
                let animate = animate_clone.clone();

                move || {
                    let mut r = renderer.borrow_mut();
                    r.update_theme();

                    if auto_rotate.get() && !dragging.get() {
                        if let Some(window) = web_sys::window() {
                            if let Ok(performance) = window.performance().ok_or(()) {
                                let now = performance.now();

                                if start_time.get().is_none() {
                                    set_start_time.set(Some(now));
                                }

                                let elapsed = now - start_time.get().unwrap_or(now);
                                let speed = if elapsed < 3000.0 { 0.015 } else { 0.003 };
                                r.rotation_y += speed;
                            }
                        }
                    }

                    if let Some(canvas) = canvas_ref.get() {
                        let canvas: &HtmlCanvasElement = canvas.as_ref();
                        if let Ok(Some(context)) = canvas.get_context("2d") {
                            if let Ok(ctx2d) = context.dyn_into::<CanvasRenderingContext2d>() {
                                let width = canvas.width() as f64;
                                let height = canvas.height() as f64;
                                r.render(&ctx2d, width, height);
                            }
                        }
                    }

                    if let Some(window) = web_sys::window() {
                        if let Some(closure) = animate.borrow().as_ref() {
                            let _ =
                                window.request_animation_frame(closure.as_ref().unchecked_ref());
                        }
                    }
                }
            }));

            if let Some(window) = web_sys::window() {
                if let Some(closure) = animate.borrow().as_ref() {
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                }
            }
        }
    });

    let on_mousedown = {
        let _renderer = renderer.clone();
        move |e: ev::MouseEvent| {
            set_dragging.set(true);
            set_last_mouse_x.set(e.client_x() as f64);
            set_last_mouse_y.set(e.client_y() as f64);
        }
    };

    let on_mousemove = {
        let renderer = renderer.clone();
        move |e: ev::MouseEvent| {
            if dragging.get() {
                let dx = e.client_x() as f64 - last_mouse_x.get();
                let dy = e.client_y() as f64 - last_mouse_y.get();

                let mut r = renderer.borrow_mut();
                r.rotation_y += dx * 0.01;
                r.rotation_x += dy * 0.01;
                r.rotation_x = r.rotation_x.clamp(-1.5, 1.5);

                set_last_mouse_x.set(e.client_x() as f64);
                set_last_mouse_y.set(e.client_y() as f64);
            }
        }
    };

    let on_mouseup = move |_: ev::MouseEvent| {
        set_dragging.set(false);
    };

    let on_mouseleave = move |_: ev::MouseEvent| {
        set_dragging.set(false);
    };

    let on_toggle = move |_: ev::MouseEvent| {
        set_auto_rotate.update(|v| *v = !*v);
    };

    let on_reset = {
        let renderer = renderer.clone();
        move |_: ev::MouseEvent| {
            let mut r = renderer.borrow_mut();
            r.rotation_x = 0.5;
            r.rotation_y = 0.0;
            r.rotation_z = 0.0;
            set_start_time.set(None);
        }
    };

    view! {
        <div class="butterfly-3d-container">
            <canvas
                node_ref=canvas_ref
                class="butterfly-3d-canvas"
                width="800"
                height="600"
                on:mousedown=on_mousedown
                on:mousemove=on_mousemove
                on:mouseup=on_mouseup
                on:mouseleave=on_mouseleave
            />
            <div class="butterfly-3d-controls">
                <button on:click=on_toggle>
                    {move || {
                        let l = locale.get();
                        if auto_rotate.get() {
                            l.ui.butterfly.pause.clone()
                        } else {
                            l.ui.butterfly.auto.clone()
                        }
                    }}
                </button>
                <button on:click=on_reset>
                    {move || locale.get().ui.butterfly.reset.clone()}
                </button>
            </div>
        </div>
    }
}
