//! Zoom and pan wrapper component for diagrams
//! Provides mouse wheel zoom and drag-to-pan functionality

use crate::i18n::use_locale;
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Element;

#[component]
pub fn DiagramZoom(children: Children) -> impl IntoView {
    let locale = use_locale();
    let (scale, set_scale) = signal(1.0_f64);
    let (translate_x, set_translate_x) = signal(0.0_f64);
    let (translate_y, set_translate_y) = signal(0.0_f64);
    let (dragging, set_dragging) = signal(false);
    let (last_mouse_x, set_last_mouse_x) = signal(0.0_f64);
    let (last_mouse_y, set_last_mouse_y) = signal(0.0_f64);

    let container_ref = NodeRef::<leptos::html::Div>::new();

    let on_wheel = move |e: ev::WheelEvent| {
        // Don't zoom when scrolling on controls
        if let Some(target) = e.target() {
            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                let tag = el.tag_name().to_uppercase();
                if matches!(tag.as_str(), "INPUT" | "SELECT") {
                    return;
                }
            }
        }
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
            // Don't start drag on interactive controls (sliders, buttons, inputs)
            if let Some(target) = e.target() {
                if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                    let tag = el.tag_name().to_uppercase();
                    if matches!(tag.as_str(), "INPUT" | "BUTTON" | "SELECT" | "LABEL") {
                        return;
                    }
                }
            }
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

    let on_dblclick = move |_: ev::MouseEvent| {
        // Find SVG inside the container and open in a new browser tab via Blob URL
        if let Some(container) = container_ref.get() {
            let element: &Element = container.as_ref();
            if let Some(svg_el) = element.query_selector("svg").ok().flatten() {
                let svg_html = svg_el.outer_html();
                if let Some(window) = web_sys::window() {
                    // Create a Blob from the SVG content
                    let parts = js_sys::Array::new();
                    parts.push(&wasm_bindgen::JsValue::from_str(&svg_html));
                    let opts = web_sys::BlobPropertyBag::new();
                    opts.set_type("image/svg+xml");
                    if let Ok(blob) =
                        web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)
                    {
                        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                            let _ = window.open_with_url_and_target(&url, "_blank");
                        }
                    }
                }
            }
        }
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
            on:dblclick=on_dblclick
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
                            <button class="zoom-reset-btn" on:click=on_reset>{move || locale.get().ui.actions.reset.clone()}</button>
                        })
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}
