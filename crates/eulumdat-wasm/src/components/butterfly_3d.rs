//! Real 3D Butterfly diagram component with Canvas rendering
//! Features: auto-rotation animation, mouse drag rotation controls

use eulumdat::{Eulumdat, Symmetry};
use gloo::timers::callback::Interval;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Butterfly3DProps {
    pub ldt: Eulumdat,
}

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
    // The theme class is on .app div, so we need to read from there
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

/// Get theme colors from CSS variables
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

    /// Rotate around X axis
    fn rotate_x(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x,
            y: self.y * cos_a - self.z * sin_a,
            z: self.y * sin_a + self.z * cos_a,
        }
    }

    /// Rotate around Y axis
    fn rotate_y(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a + self.z * sin_a,
            y: self.y,
            z: -self.x * sin_a + self.z * cos_a,
        }
    }

    /// Rotate around Z axis
    fn rotate_z(self, angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a - self.y * sin_a,
            y: self.x * sin_a + self.y * cos_a,
            z: self.z,
        }
    }

    /// Project to 2D screen coordinates
    fn project(self, cx: f64, cy: f64, scale: f64) -> (f64, f64) {
        // Simple perspective projection
        let perspective = 600.0;
        let z_offset = 300.0;
        let factor = perspective / (perspective + self.z + z_offset);
        (cx + self.x * scale * factor, cy - self.y * scale * factor)
    }
}

/// C-plane wing data for 3D rendering
struct Wing {
    #[allow(dead_code)]
    c_angle: f64,
    points: Vec<Point3D>,
    color_hue: f64,
}

/// State for the 3D renderer
struct Renderer3D {
    rotation_x: f64,
    rotation_y: f64,
    rotation_z: f64,
    auto_rotate: bool,
    dragging: bool,
    last_mouse_x: f64,
    last_mouse_y: f64,
    wings: Vec<Wing>,
    max_intensity: f64,
    theme_colors: ThemeColors,
}

impl Renderer3D {
    fn new() -> Self {
        Self {
            rotation_x: 0.5, // Initial tilt
            rotation_y: 0.0,
            rotation_z: 0.0,
            auto_rotate: true,
            dragging: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            wings: Vec::new(),
            max_intensity: 100.0,
            theme_colors: ThemeColors::default(),
        }
    }

    /// Update theme colors from CSS variables
    fn update_theme(&mut self) {
        self.theme_colors = get_theme_colors();
    }

    /// Build wing geometry from LDT data
    fn build_wings(&mut self, ldt: &Eulumdat) {
        self.wings.clear();

        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
            return;
        }

        // Calculate max intensity
        self.max_intensity = ldt
            .intensities
            .iter()
            .flat_map(|plane| plane.iter())
            .copied()
            .fold(0.0_f64, f64::max)
            .max(1.0);

        // Expand C-planes based on symmetry
        let c_plane_data = expand_c_planes(ldt);

        for (c_angle, intensities) in c_plane_data {
            let c_rad = c_angle.to_radians();
            let mut points = Vec::new();

            // Start at center (origin)
            points.push(Point3D::new(0.0, 0.0, 0.0));

            // Build points along gamma angles
            for (j, &g_angle) in ldt.g_angles.iter().enumerate() {
                let intensity = intensities.get(j).copied().unwrap_or(0.0);
                let r = intensity / self.max_intensity; // Normalize to 0-1

                let g_rad = g_angle.to_radians();

                // Convert spherical to Cartesian
                // gamma=0 is nadir (down), gamma=90 is horizontal, gamma=180 is zenith (up)
                let x = r * g_rad.sin() * c_rad.cos();
                let y = r * g_rad.sin() * c_rad.sin();
                let z = r * g_rad.cos(); // Down is positive Z

                points.push(Point3D::new(x, y, -z)); // Flip Z for display
            }

            let color_hue = (c_angle / 360.0) * 240.0 + 180.0;

            self.wings.push(Wing {
                c_angle,
                points,
                color_hue: color_hue % 360.0,
            });
        }
    }

    /// Render the 3D butterfly to canvas
    fn render(&self, ctx: &CanvasRenderingContext2d, width: f64, height: f64) {
        let cx = width / 2.0;
        let cy = height / 2.0;
        let scale = (width.min(height) / 2.0) * 0.7;

        // Clear background using theme color
        ctx.set_fill_style_str(&self.theme_colors.bg);
        ctx.fill_rect(0.0, 0.0, width, height);

        // Draw grid circles
        self.draw_grid(ctx, cx, cy, scale);

        // Sort wings by average Z depth for painter's algorithm
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

        // Draw wings from back to front
        for (idx, _) in wing_depths {
            self.draw_wing(ctx, &self.wings[idx], cx, cy, scale);
        }

        // Draw center point
        ctx.begin_path();
        ctx.set_fill_style_str(&self.theme_colors.center);
        let _ = ctx.arc(cx, cy, 4.0, 0.0, std::f64::consts::TAU);
        ctx.fill();

        // Draw labels
        self.draw_labels(ctx, width, height);
    }

    fn draw_grid(&self, ctx: &CanvasRenderingContext2d, cx: f64, cy: f64, scale: f64) {
        ctx.set_stroke_style_str(&self.theme_colors.grid);
        ctx.set_line_width(1.0);

        // Draw concentric circles at gamma=90 (horizontal plane)
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

        // Draw C-plane direction lines
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

            // Draw C-plane label
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

        // Fill with semi-transparent color based on hue
        let (r, g, b) = hsl_to_rgb(wing.color_hue / 360.0, 0.6, 0.5);
        ctx.set_fill_style_str(&format!("rgba({}, {}, {}, 0.5)", r, g, b));
        ctx.fill();

        // Stroke with slightly brighter color
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
        let _ = ctx.fill_text("3D Butterfly Diagram", 15.0, 15.0);

        ctx.set_fill_style_str(&self.theme_colors.text);
        ctx.set_font("11px system-ui, sans-serif");
        ctx.set_text_align("right");
        let _ = ctx.fill_text("cd/klm", width - 15.0, height - 20.0);

        ctx.set_text_align("left");
        let _ = ctx.fill_text(
            &format!("Max: {:.0}", self.max_intensity),
            15.0,
            height - 20.0,
        );

        // Instructions
        ctx.set_font("10px system-ui, sans-serif");
        ctx.set_text_align("right");
        ctx.set_text_baseline("top");
        let _ = ctx.fill_text("Drag to rotate", width - 15.0, 15.0);
    }
}

/// Expand C-plane data based on symmetry type (same as SVG version)
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
                if let Some(&c_angle) = ldt.c_angles.get(c_start + i) {
                    result.push((c_angle, intensities.clone()));
                    if c_angle > 90.0 && c_angle < 270.0 {
                        let mirrored = if c_angle < 180.0 {
                            90.0 - (c_angle - 90.0)
                        } else {
                            270.0 + (270.0 - c_angle)
                        };
                        if (0.0..=360.0).contains(&mirrored) {
                            result.push((mirrored, intensities.clone()));
                        }
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

/// HSL to RGB conversion
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

pub enum Msg {
    Tick,
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(#[allow(dead_code)] MouseEvent),
    MouseLeave,
    ToggleAutoRotate,
    ResetView,
}

pub struct Butterfly3D {
    canvas_ref: NodeRef,
    renderer: Rc<RefCell<Renderer3D>>,
    _interval: Option<Interval>,
    start_time: Option<f64>,
}

impl Component for Butterfly3D {
    type Message = Msg;
    type Properties = Butterfly3DProps;

    fn create(ctx: &Context<Self>) -> Self {
        let renderer = Rc::new(RefCell::new(Renderer3D::new()));

        // Build wings from LDT data
        renderer.borrow_mut().build_wings(&ctx.props().ldt);

        // Set up animation interval
        let link = ctx.link().clone();
        let interval = Interval::new(16, move || {
            link.send_message(Msg::Tick);
        });

        Self {
            canvas_ref: NodeRef::default(),
            renderer,
            _interval: Some(interval),
            start_time: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                let mut renderer = self.renderer.borrow_mut();

                // Update theme colors from CSS variables (in case theme changed)
                renderer.update_theme();

                // Get current time for animation
                if let Some(window) = web_sys::window() {
                    if let Ok(performance) = window.performance().ok_or(()) {
                        let now = performance.now();

                        if self.start_time.is_none() {
                            self.start_time = Some(now);
                        }

                        // Auto-rotate for first 3 seconds, then slow down
                        if renderer.auto_rotate && !renderer.dragging {
                            let elapsed = now - self.start_time.unwrap_or(now);
                            let speed = if elapsed < 3000.0 {
                                0.015 // Fast initial rotation
                            } else {
                                0.003 // Slow continuous rotation
                            };
                            renderer.rotation_y += speed;
                        }
                    }
                }

                // Render
                if let Some(canvas) = self.canvas_ref.cast::<HtmlCanvasElement>() {
                    if let Ok(Some(context)) = canvas.get_context("2d") {
                        if let Ok(ctx2d) = context.dyn_into::<CanvasRenderingContext2d>() {
                            let width = canvas.width() as f64;
                            let height = canvas.height() as f64;
                            renderer.render(&ctx2d, width, height);
                        }
                    }
                }
                false // Don't re-render Yew component, canvas handles rendering
            }
            Msg::MouseDown(e) => {
                let mut renderer = self.renderer.borrow_mut();
                renderer.dragging = true;
                renderer.last_mouse_x = e.client_x() as f64;
                renderer.last_mouse_y = e.client_y() as f64;
                false
            }
            Msg::MouseMove(e) => {
                let mut renderer = self.renderer.borrow_mut();
                if renderer.dragging {
                    let dx = e.client_x() as f64 - renderer.last_mouse_x;
                    let dy = e.client_y() as f64 - renderer.last_mouse_y;

                    renderer.rotation_y += dx * 0.01;
                    renderer.rotation_x += dy * 0.01;

                    // Clamp X rotation
                    renderer.rotation_x = renderer.rotation_x.clamp(-1.5, 1.5);

                    renderer.last_mouse_x = e.client_x() as f64;
                    renderer.last_mouse_y = e.client_y() as f64;
                }
                false
            }
            Msg::MouseUp(_) => {
                self.renderer.borrow_mut().dragging = false;
                false
            }
            Msg::MouseLeave => {
                self.renderer.borrow_mut().dragging = false;
                false
            }
            Msg::ToggleAutoRotate => {
                let mut renderer = self.renderer.borrow_mut();
                renderer.auto_rotate = !renderer.auto_rotate;
                true
            }
            Msg::ResetView => {
                let mut renderer = self.renderer.borrow_mut();
                renderer.rotation_x = 0.5;
                renderer.rotation_y = 0.0;
                renderer.rotation_z = 0.0;
                self.start_time = None;
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        // Rebuild wings if LDT data changed
        self.renderer.borrow_mut().build_wings(&ctx.props().ldt);
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let auto_rotate = self.renderer.borrow().auto_rotate;

        html! {
            <div class="butterfly-3d-container">
                <canvas
                    ref={self.canvas_ref.clone()}
                    class="butterfly-3d-canvas"
                    width="800"
                    height="600"
                    onmousedown={ctx.link().callback(Msg::MouseDown)}
                    onmousemove={ctx.link().callback(Msg::MouseMove)}
                    onmouseup={ctx.link().callback(Msg::MouseUp)}
                    onmouseleave={ctx.link().callback(|_| Msg::MouseLeave)}
                />
                <div class="butterfly-3d-controls">
                    <button onclick={ctx.link().callback(|_| Msg::ToggleAutoRotate)}>
                        {if auto_rotate { "Pause" } else { "Auto" }}
                    </button>
                    <button onclick={ctx.link().callback(|_| Msg::ResetView)}>
                        {"Reset"}
                    </button>
                </div>
            </div>
        }
    }
}
