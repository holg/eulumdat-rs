//! Bevy 3D Scene Viewer Component
//!
//! This component lazy-loads the Bevy 3D viewer when the user opens the 3D Scene tab.
//! The Bevy WASM (~22MB) is loaded on demand, keeping initial app load fast (~2MB).

use crate::i18n::use_locale;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn is_bevy_loaded() -> bool;

    #[wasm_bindgen(js_name = isBevyLoading)]
    fn is_bevy_loading() -> bool;
}

/// Viewer settings that sync with Bevy via localStorage
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerSettings {
    pub scene_type: u8, // 0=Room, 1=Road, 2=Parking, 3=Outdoor
    pub room_width: f32,
    pub room_length: f32,
    pub room_height: f32,
    pub mounting_height: f32,
    pub pendulum_length: f32,
    pub show_luminaire: bool,
    pub show_photometric_solid: bool,
    pub show_shadows: bool,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        Self {
            scene_type: 0,
            room_width: 4.0,
            room_length: 5.0,
            room_height: 2.8,
            mounting_height: 8.0,
            pendulum_length: 0.3,
            show_luminaire: true,
            show_photometric_solid: false,
            show_shadows: false,
        }
    }
}

impl ViewerSettings {
    /// Convert to JSON string for localStorage
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"scene_type":{},"room_width":{},"room_length":{},"room_height":{},"mounting_height":{},"pendulum_length":{},"show_luminaire":{},"show_photometric_solid":{},"show_shadows":{}}}"#,
            self.scene_type,
            self.room_width,
            self.room_length,
            self.room_height,
            self.mounting_height,
            self.pendulum_length,
            self.show_luminaire,
            self.show_photometric_solid,
            self.show_shadows
        )
    }

    /// Save to localStorage with timestamp to trigger Bevy reload
    pub fn save_to_local_storage(&self) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("eulumdat_viewer_settings", &self.to_json());
                let timestamp = js_sys::Date::now().to_string();
                let _ = storage.set_item("eulumdat_viewer_settings_timestamp", &timestamp);
            }
        }
    }
}

/// Loading state for the Bevy viewer
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BevyLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

/// Bevy 3D Scene Viewer component
///
/// Renders a canvas and lazy-loads the Bevy WASM when mounted.
/// The Bevy app syncs with the editor via localStorage.
#[component]
pub fn BevySceneViewer() -> impl IntoView {
    let locale = use_locale();
    let (load_state, set_load_state) = signal(BevyLoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let (settings, set_settings) = signal(ViewerSettings::default());
    let (show_controls, set_show_controls) = signal(true);

    // Check if already loaded on mount
    Effect::new(move |_| {
        if is_bevy_loaded() {
            set_load_state.set(BevyLoadState::Loaded);
        } else if is_bevy_loading() {
            set_load_state.set(BevyLoadState::Loading);
        }
    });

    // Save settings to localStorage when they change
    Effect::new(move |_| {
        settings.get().save_to_local_storage();
    });

    // Trigger loading
    let start_loading = move |_| {
        if load_state.get() != BevyLoadState::NotLoaded {
            return;
        }

        set_load_state.set(BevyLoadState::Loading);

        wasm_bindgen_futures::spawn_local(async move {
            let promise = load_bevy_viewer();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;

            match result {
                Ok(_) => {
                    set_load_state.set(BevyLoadState::Loaded);
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    web_sys::console::error_1(&format!("Bevy load error: {}", msg).into());
                    set_error_msg.set(Some(msg));
                    set_load_state.set(BevyLoadState::Error);
                }
            }
        });
    };

    view! {
        <div class="bevy-scene-container" style="width: 100%; height: 100%; position: relative; display: flex;">
            // Control panel (left side)
            <ViewerControlPanel
                settings=settings
                set_settings=set_settings
                show_controls=show_controls
                set_show_controls=set_show_controls
            />

            // Canvas container (right side)
            <div style="flex: 1; position: relative; min-width: 0;">
                // Canvas for Bevy to render into
                <canvas
                    id="bevy-canvas"
                    style="width: 100%; height: 100%; display: block; touch-action: none;"
                />

                // Loading overlay
                {move || {
                    let l = locale.get();
                    match load_state.get() {
                        BevyLoadState::NotLoaded => view! {
                            <div class="bevy-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white; cursor: pointer;
                            " on:click=start_loading>
                                <div style="font-size: 48px; margin-bottom: 20px;">"🎮"</div>
                                <div style="font-size: 18px; margin-bottom: 10px;">{l.ui.bevy_scene.title.clone()}</div>
                                <div style="font-size: 14px; color: #aaa; margin-bottom: 20px;">{l.ui.bevy_scene.click_to_load.clone()}</div>
                                <button style="
                                    padding: 12px 24px; font-size: 16px;
                                    background: #4a9eff; color: white; border: none;
                                    border-radius: 8px; cursor: pointer;
                                ">{l.ui.bevy_scene.load_button.clone()}</button>
                            </div>
                        }.into_any(),

                        BevyLoadState::Loading => view! {
                            <div class="bevy-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white;
                            ">
                                <div class="spinner" style="
                                    width: 50px; height: 50px; border: 4px solid #333;
                                    border-top-color: #4a9eff; border-radius: 50%;
                                    animation: spin 1s linear infinite;
                                "></div>
                                <div style="margin-top: 20px; font-size: 16px;">{l.ui.bevy_scene.loading.clone()}</div>
                                <div style="margin-top: 10px; font-size: 12px; color: #888;">{l.ui.bevy_scene.downloading.clone()}</div>
                            </div>
                            <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                        }.into_any(),

                        BevyLoadState::Loaded => view! {
                            // Bevy is rendering, show controls hint at bottom
                            <div style="
                                position: absolute; bottom: 10px; left: 50%; transform: translateX(-50%);
                                background: rgba(0,0,0,0.6); color: white; padding: 6px 12px;
                                border-radius: 4px; font-size: 12px; pointer-events: none;
                            ">
                                {l.ui.bevy_scene.controls_hint.clone()}
                            </div>
                        }.into_any(),

                        BevyLoadState::Error => view! {
                            <div class="bevy-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white;
                            ">
                                <div style="font-size: 48px; margin-bottom: 20px;">"❌"</div>
                                <div style="font-size: 18px; color: #ff6b6b;">{l.ui.bevy_scene.load_failed.clone()}</div>
                                <div style="margin-top: 10px; font-size: 12px; color: #888; max-width: 400px; text-align: center;">
                                    {move || error_msg.get().unwrap_or_default()}
                                </div>
                                <button
                                    style="
                                        margin-top: 20px; padding: 12px 24px; font-size: 16px;
                                        background: #4a9eff; color: white; border: none;
                                        border-radius: 8px; cursor: pointer;
                                    "
                                    on:click=move |_| {
                                        set_load_state.set(BevyLoadState::NotLoaded);
                                        set_error_msg.set(None);
                                    }
                                >{l.ui.bevy_scene.try_again.clone()}</button>
                            </div>
                        }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}

/// Control panel for adjusting viewer settings
#[component]
fn ViewerControlPanel(
    settings: ReadSignal<ViewerSettings>,
    set_settings: WriteSignal<ViewerSettings>,
    show_controls: ReadSignal<bool>,
    set_show_controls: WriteSignal<bool>,
) -> impl IntoView {
    let locale = use_locale();

    // Helper to update a single field
    let update_field = move |f: fn(&mut ViewerSettings, f32), value: f32| {
        set_settings.update(|s| f(s, value));
    };

    let update_bool = move |f: fn(&mut ViewerSettings, bool), value: bool| {
        set_settings.update(|s| f(s, value));
    };

    view! {
        <div
            class="viewer-control-panel"
            style=move || format!(
                "width: {}; background: var(--bg-secondary, #2a2a2a); border-right: 1px solid var(--border-color, #444); \
                 padding: {}; overflow-y: auto; transition: width 0.2s, padding 0.2s;",
                if show_controls.get() { "240px" } else { "40px" },
                if show_controls.get() { "12px" } else { "8px" }
            )
        >
            // Toggle button
            <button
                style="width: 100%; padding: 6px; margin-bottom: 12px; background: var(--bg-tertiary, #333); \
                       border: 1px solid var(--border-color, #444); border-radius: 4px; color: var(--text-primary, #fff); \
                       cursor: pointer; font-size: 14px;"
                on:click=move |_| set_show_controls.update(|v| *v = !*v)
            >
                {move || if show_controls.get() { "◀ Hide" } else { "▶" }}
            </button>

            // Only show controls when expanded
            {move || {
                if !show_controls.get() {
                    return view! { <div></div> }.into_any();
                }

                let l = locale.get();
                let current = settings.get();

                view! {
                    <div style="display: flex; flex-direction: column; gap: 12px;">
                        // Scene Type
                        <div class="control-group">
                            <label style="font-size: 12px; color: var(--text-secondary, #aaa); margin-bottom: 4px; display: block;">
                                {l.ui.bevy_scene.scene_type.clone()}
                            </label>
                            <select
                                style="width: 100%; padding: 6px; background: var(--bg-tertiary, #333); \
                                       border: 1px solid var(--border-color, #444); border-radius: 4px; \
                                       color: var(--text-primary, #fff);"
                                on:change=move |ev| {
                                    let value: u8 = event_target_value(&ev).parse().unwrap_or(0);
                                    set_settings.update(|s| s.scene_type = value);
                                }
                            >
                                <option value="0" selected=move || current.scene_type == 0>
                                    {l.ui.bevy_scene.scene_room.clone()}
                                </option>
                                <option value="1" selected=move || current.scene_type == 1>
                                    {l.ui.bevy_scene.scene_road.clone()}
                                </option>
                                <option value="2" selected=move || current.scene_type == 2>
                                    {l.ui.bevy_scene.scene_parking.clone()}
                                </option>
                                <option value="3" selected=move || current.scene_type == 3>
                                    {l.ui.bevy_scene.scene_outdoor.clone()}
                                </option>
                            </select>
                        </div>

                        // Dimensions (show different fields based on scene type)
                        {move || {
                            let l = locale.get();
                            let scene_type = settings.get().scene_type;

                            if scene_type == 0 {
                                // Room scene - show room dimensions and pendulum
                                view! {
                                    <div style="display: flex; flex-direction: column; gap: 8px;">
                                        <NumberInput
                                            label=l.ui.bevy_scene.room_width.clone()
                                            value=settings.get().room_width
                                            min=1.0 max=20.0 step=0.5
                                            on_change=move |v| update_field(|s, v| s.room_width = v, v)
                                        />
                                        <NumberInput
                                            label=l.ui.bevy_scene.room_length.clone()
                                            value=settings.get().room_length
                                            min=1.0 max=30.0 step=0.5
                                            on_change=move |v| update_field(|s, v| s.room_length = v, v)
                                        />
                                        <NumberInput
                                            label=l.ui.bevy_scene.room_height.clone()
                                            value=settings.get().room_height
                                            min=2.0 max=10.0 step=0.1
                                            on_change=move |v| update_field(|s, v| s.room_height = v, v)
                                        />
                                        <NumberInput
                                            label=l.ui.bevy_scene.pendulum_length.clone()
                                            value=settings.get().pendulum_length
                                            min=0.0 max=3.0 step=0.1
                                            on_change=move |v| update_field(|s, v| s.pendulum_length = v, v)
                                        />
                                    </div>
                                }.into_any()
                            } else {
                                // Outdoor scenes - show mounting height
                                view! {
                                    <div style="display: flex; flex-direction: column; gap: 8px;">
                                        <NumberInput
                                            label=l.ui.bevy_scene.room_width.clone()
                                            value=settings.get().room_width
                                            min=5.0 max=50.0 step=1.0
                                            on_change=move |v| update_field(|s, v| s.room_width = v, v)
                                        />
                                        <NumberInput
                                            label=l.ui.bevy_scene.room_length.clone()
                                            value=settings.get().room_length
                                            min=10.0 max=100.0 step=5.0
                                            on_change=move |v| update_field(|s, v| s.room_length = v, v)
                                        />
                                        <NumberInput
                                            label=l.ui.bevy_scene.mounting_height.clone()
                                            value=settings.get().mounting_height
                                            min=3.0 max=15.0 step=0.5
                                            on_change=move |v| update_field(|s, v| s.mounting_height = v, v)
                                        />
                                    </div>
                                }.into_any()
                            }
                        }}

                        // Toggle options
                        <div style="display: flex; flex-direction: column; gap: 6px; margin-top: 8px;">
                            <CheckboxInput
                                label=locale.get().ui.bevy_scene.show_luminaire.clone()
                                checked=settings.get().show_luminaire
                                on_change=move |v| update_bool(|s, v| s.show_luminaire = v, v)
                            />
                            <CheckboxInput
                                label=locale.get().ui.bevy_scene.show_solid.clone()
                                checked=settings.get().show_photometric_solid
                                on_change=move |v| update_bool(|s, v| s.show_photometric_solid = v, v)
                            />
                            <CheckboxInput
                                label=locale.get().ui.bevy_scene.show_shadows.clone()
                                checked=settings.get().show_shadows
                                on_change=move |v| update_bool(|s, v| s.show_shadows = v, v)
                            />
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// Number input with label
#[component]
fn NumberInput(
    label: String,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    on_change: impl Fn(f32) + 'static,
) -> impl IntoView {
    view! {
        <div class="number-input" style="display: flex; flex-direction: column; gap: 2px;">
            <label style="font-size: 11px; color: var(--text-secondary, #888);">
                {label}
            </label>
            <input
                type="number"
                value=value
                min=min
                max=max
                step=step
                style="width: 100%; padding: 4px 6px; background: var(--bg-tertiary, #333); \
                       border: 1px solid var(--border-color, #444); border-radius: 3px; \
                       color: var(--text-primary, #fff); font-size: 13px;"
                on:change=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                        on_change(v.clamp(min, max));
                    }
                }
            />
        </div>
    }
}

/// Checkbox input with label
#[component]
fn CheckboxInput(
    label: String,
    checked: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; font-size: 12px; \
                      color: var(--text-primary, #fff);">
            <input
                type="checkbox"
                checked=checked
                style="width: 16px; height: 16px; cursor: pointer;"
                on:change=move |ev| {
                    let target = event_target::<web_sys::HtmlInputElement>(&ev);
                    on_change(target.checked());
                }
            />
            {label}
        </label>
    }
}

/// Standalone Bevy scene for when iframe is still preferred
#[allow(dead_code)]
#[component]
pub fn BevySceneIframe(#[prop(default = 0)] reload_key: u32) -> impl IntoView {
    view! {
        <div class="scene-container">
            <iframe
                src=move || format!("bevy/index.html?r={}", reload_key)
                style="width: 100%; height: 600px; border: none; border-radius: 8px;"
                title="3D Scene Viewer"
            />
        </div>
    }
}
