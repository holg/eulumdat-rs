//! Zoom and pan wrapper component for diagrams
//! Provides mouse wheel zoom and drag-to-pan functionality

use leptos::ev;
use leptos::prelude::*;
use web_sys::Element;

#[component]
pub fn DiagramZoom(children: Children) -> impl IntoView {
    let (scale, set_scale) = signal(1.0_f64);
    let (translate_x, set_translate_x) = signal(0.0_f64);
    let (translate_y, set_translate_y) = signal(0.0_f64);
    let (dragging, set_dragging) = signal(false);
    let (last_mouse_x, set_last_mouse_x) = signal(0.0_f64);
    let (last_mouse_y, set_last_mouse_y) = signal(0.0_f64);

    let container_ref = NodeRef::<leptos::html::Div>::new();

    let on_wheel = move |e: ev::WheelEvent| {
        e.prevent_default();
        let delta = e.delta_y();
        let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

        let current_scale = scale.get();
        let new_scale = (current_scale * zoom_factor).clamp(0.5, 4.0);

        if let Some(container) = container_ref.get() {
            let element: &Element = container.as_ref();
            let rect = element.get_bounding_client_rect();
            let mouse_x = e.client_x() as f64 - rect.left();
            let mouse_y = e.client_y() as f64 - rect.top();

            let scale_change = new_scale / current_scale;
            let tx = translate_x.get();
            let ty = translate_y.get();
            set_translate_x.set(mouse_x - (mouse_x - tx) * scale_change);
            set_translate_y.set(mouse_y - (mouse_y - ty) * scale_change);
        }

        set_scale.set(new_scale);
    };

    let on_mousedown = move |e: ev::MouseEvent| {
        if e.button() == 0 {
            set_dragging.set(true);
            set_last_mouse_x.set(e.client_x() as f64);
            set_last_mouse_y.set(e.client_y() as f64);
        }
    };

    let on_mousemove = move |e: ev::MouseEvent| {
        if dragging.get() {
            let dx = e.client_x() as f64 - last_mouse_x.get();
            let dy = e.client_y() as f64 - last_mouse_y.get();

            set_translate_x.update(|tx| *tx += dx);
            set_translate_y.update(|ty| *ty += dy);

            set_last_mouse_x.set(e.client_x() as f64);
            set_last_mouse_y.set(e.client_y() as f64);
        }
    };

    let on_mouseup = move |_: ev::MouseEvent| {
        set_dragging.set(false);
    };

    let on_mouseleave = move |_: ev::MouseEvent| {
        set_dragging.set(false);
    };

    let on_reset = move |_: ev::MouseEvent| {
        set_scale.set(1.0);
        set_translate_x.set(0.0);
        set_translate_y.set(0.0);
    };

    view! {
        <div
            class="diagram-zoom-container"
            node_ref=container_ref
            on:wheel=on_wheel
            on:mousedown=on_mousedown
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:mouseleave=on_mouseleave
        >
            <div
                class="diagram-zoom-content"
                style=move || format!(
                    "transform: translate({}px, {}px) scale({})",
                    translate_x.get(), translate_y.get(), scale.get()
                )
            >
                {children()}
            </div>
            <div class="diagram-zoom-overlay">
                <span class="zoom-level">{move || format!("{:.0}%", scale.get() * 100.0)}</span>
                {move || {
                    let is_zoomed = (scale.get() - 1.0).abs() > 0.01
                        || translate_x.get().abs() > 1.0
                        || translate_y.get().abs() > 1.0;
                    if is_zoomed {
                        Some(view! {
                            <button class="zoom-reset-btn" on:click=on_reset>"Reset"</button>
                        })
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}
