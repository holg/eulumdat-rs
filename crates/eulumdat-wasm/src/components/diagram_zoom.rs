//! Zoom and pan wrapper component for diagrams
//! Provides mouse wheel zoom and drag-to-pan functionality

use web_sys::{Element, MouseEvent, WheelEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DiagramZoomProps {
    pub children: Children,
}

pub enum Msg {
    Wheel(WheelEvent),
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp,
    MouseLeave,
    Reset,
}

pub struct DiagramZoom {
    container_ref: NodeRef,
    scale: f64,
    translate_x: f64,
    translate_y: f64,
    dragging: bool,
    last_mouse_x: f64,
    last_mouse_y: f64,
}

impl Component for DiagramZoom {
    type Message = Msg;
    type Properties = DiagramZoomProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            container_ref: NodeRef::default(),
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            dragging: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Wheel(e) => {
                e.prevent_default();
                let delta = e.delta_y();
                let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

                // Clamp scale between 0.5 and 4.0
                let new_scale = (self.scale * zoom_factor).clamp(0.5, 4.0);

                // Get mouse position relative to container
                if let Some(container) = self.container_ref.cast::<Element>() {
                    let rect = container.get_bounding_client_rect();
                    let mouse_x = e.client_x() as f64 - rect.left();
                    let mouse_y = e.client_y() as f64 - rect.top();

                    // Adjust translation to zoom towards mouse position
                    let scale_change = new_scale / self.scale;
                    self.translate_x = mouse_x - (mouse_x - self.translate_x) * scale_change;
                    self.translate_y = mouse_y - (mouse_y - self.translate_y) * scale_change;
                }

                self.scale = new_scale;
                true
            }
            Msg::MouseDown(e) => {
                if e.button() == 0 {
                    // Left mouse button
                    self.dragging = true;
                    self.last_mouse_x = e.client_x() as f64;
                    self.last_mouse_y = e.client_y() as f64;
                }
                false
            }
            Msg::MouseMove(e) => {
                if self.dragging {
                    let dx = e.client_x() as f64 - self.last_mouse_x;
                    let dy = e.client_y() as f64 - self.last_mouse_y;

                    self.translate_x += dx;
                    self.translate_y += dy;

                    self.last_mouse_x = e.client_x() as f64;
                    self.last_mouse_y = e.client_y() as f64;
                    true
                } else {
                    false
                }
            }
            Msg::MouseUp | Msg::MouseLeave => {
                self.dragging = false;
                false
            }
            Msg::Reset => {
                self.scale = 1.0;
                self.translate_x = 0.0;
                self.translate_y = 0.0;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let transform = format!(
            "translate({}px, {}px) scale({})",
            self.translate_x, self.translate_y, self.scale
        );

        let onwheel = ctx.link().callback(Msg::Wheel);
        let onmousedown = ctx.link().callback(Msg::MouseDown);
        let onmousemove = ctx.link().callback(Msg::MouseMove);
        let onmouseup = ctx.link().callback(|_| Msg::MouseUp);
        let onmouseleave = ctx.link().callback(|_| Msg::MouseLeave);
        let onreset = ctx.link().callback(|_| Msg::Reset);

        let is_zoomed = (self.scale - 1.0).abs() > 0.01
            || self.translate_x.abs() > 1.0
            || self.translate_y.abs() > 1.0;

        html! {
            <div
                class="diagram-zoom-container"
                ref={self.container_ref.clone()}
                {onwheel}
                {onmousedown}
                {onmousemove}
                {onmouseup}
                {onmouseleave}
            >
                <div class="diagram-zoom-content" style={format!("transform: {}", transform)}>
                    { for ctx.props().children.iter() }
                </div>
                <div class="diagram-zoom-overlay">
                    <span class="zoom-level">{format!("{:.0}%", self.scale * 100.0)}</span>
                    if is_zoomed {
                        <button class="zoom-reset-btn" onclick={onreset}>{"Reset"}</button>
                    }
                </div>
            </div>
        }
    }
}
