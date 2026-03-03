//! Google Maps Lighting Designer Component
//!
//! This component lazy-loads the Google Maps API when the user opens the Maps Designer tab.
//! The Maps API is loaded on demand, keeping initial app load fast.
//!
//! Features:
//! - Draw parking lot / area polygon on satellite map
//! - Place luminaires with configurable mounting height, rotation, tilt
//! - Calculate illuminance heatmap using LDT photometric data
//! - Export calculation grid to CSV

use crate::i18n::use_locale;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadGoogleMaps)]
    fn load_google_maps() -> js_sys::Promise;

    #[wasm_bindgen(js_name = initGMapsDesigner)]
    fn init_gmaps_designer(container_id: &str) -> JsValue;

    #[wasm_bindgen(js_name = calculateGMapsHeatmap)]
    fn calculate_gmaps_heatmap(ldt_data: &JsValue, grid_spacing: f64) -> JsValue;

    #[wasm_bindgen(js_name = clearGMapsDesigner)]
    fn clear_gmaps_designer();

    #[wasm_bindgen(js_name = exportGMapsToCsv)]
    fn export_gmaps_to_csv() -> JsValue;

    #[wasm_bindgen(js_name = centerOnUserLocation)]
    fn center_on_user_location();

    #[wasm_bindgen(js_name = isGMapsLoaded)]
    fn is_gmaps_loaded() -> bool;

    #[wasm_bindgen(js_name = isGMapsLoading)]
    fn is_gmaps_loading() -> bool;

    #[wasm_bindgen(js_name = getGMapsLuminaires)]
    fn get_gmaps_luminaires() -> JsValue;

    #[wasm_bindgen(js_name = getGMapsPolygon)]
    fn get_gmaps_polygon() -> JsValue;

    #[wasm_bindgen(js_name = toggleGMapsLuxLabels)]
    fn toggle_gmaps_lux_labels(show: bool);
}

/// Loading state for the Google Maps designer
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GMapsLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

/// Calculation results from heatmap
#[derive(Clone, Debug, Default)]
pub struct CalculationResults {
    pub min_lux: f64,
    pub max_lux: f64,
    pub avg_lux: f64,
    pub uniformity: f64,
    pub point_count: usize,
}

/// Google Maps Lighting Designer component
///
/// Renders a map and allows users to:
/// 1. Draw a polygon defining the area (parking lot, etc.)
/// 2. Place luminaires on the map
/// 3. Calculate illuminance heatmap using the loaded LDT file
#[component]
pub fn MapsDesigner(
    /// The current LDT data as JSON string (from localStorage)
    #[prop(optional)]
    _ldt_json: Option<String>,
) -> impl IntoView {
    let locale = use_locale();
    let (load_state, set_load_state) = signal(GMapsLoadState::NotLoaded);
    let (error_msg, set_error_msg) = signal::<Option<String>>(None);
    let (results, set_results) = signal::<Option<CalculationResults>>(None);
    let (grid_spacing, set_grid_spacing) = signal(1.0_f64);
    let (luminaire_count, set_luminaire_count) = signal(0_usize);
    let (has_polygon, set_has_polygon) = signal(false);
    let (show_controls, set_show_controls) = signal(true);
    let (show_labels, set_show_labels) = signal(true);

    // Check if already loaded on mount
    Effect::new(move |_| {
        if is_gmaps_loaded() {
            set_load_state.set(GMapsLoadState::Loaded);
        } else if is_gmaps_loading() {
            set_load_state.set(GMapsLoadState::Loading);
        }
    });

    // Listen for map events from JavaScript
    Effect::new(move |_| {
        if load_state.get() != GMapsLoadState::Loaded {
            return;
        }

        // Set up event listeners for polygon and luminaire changes
        let window = web_sys::window().unwrap();

        // Polygon events
        let polygon_handler = Closure::wrap(Box::new(move |_: web_sys::CustomEvent| {
            set_has_polygon.set(true);
        }) as Box<dyn FnMut(_)>);

        let _ = window.add_event_listener_with_callback(
            "gmaps-polygon-complete",
            polygon_handler.as_ref().unchecked_ref(),
        );
        polygon_handler.forget();

        let polygon_update_handler = Closure::wrap(Box::new(move |_: web_sys::CustomEvent| {
            set_has_polygon.set(true);
        }) as Box<dyn FnMut(_)>);

        let _ = window.add_event_listener_with_callback(
            "gmaps-polygon-updated",
            polygon_update_handler.as_ref().unchecked_ref(),
        );
        polygon_update_handler.forget();

        // Luminaire events
        let luminaire_handler = Closure::wrap(Box::new(move |_: web_sys::CustomEvent| {
            let luminaires = get_gmaps_luminaires();
            if let Ok(arr) = luminaires.dyn_into::<js_sys::Array>() {
                set_luminaire_count.set(arr.length() as usize);
            }
        }) as Box<dyn FnMut(_)>);

        for event in &[
            "gmaps-luminaire-added",
            "gmaps-luminaire-removed",
            "gmaps-luminaire-updated",
            "gmaps-luminaire-moved",
        ] {
            let _ = window.add_event_listener_with_callback(
                event,
                luminaire_handler.as_ref().unchecked_ref(),
            );
        }
        luminaire_handler.forget();
    });

    // Trigger loading
    let start_loading = move |_| {
        if load_state.get() != GMapsLoadState::NotLoaded {
            return;
        }

        set_load_state.set(GMapsLoadState::Loading);

        wasm_bindgen_futures::spawn_local(async move {
            let promise = load_google_maps();
            let result = wasm_bindgen_futures::JsFuture::from(promise).await;

            match result {
                Ok(_) => {
                    // Initialize the map
                    init_gmaps_designer("gmaps-container");
                    set_load_state.set(GMapsLoadState::Loaded);
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    web_sys::console::error_1(&format!("GMaps load error: {}", msg).into());
                    set_error_msg.set(Some(msg));
                    set_load_state.set(GMapsLoadState::Error);
                }
            }
        });
    };

    // Calculate heatmap
    let calculate = move |_| {
        // Get LDT data from localStorage
        let window = web_sys::window().unwrap();
        let storage = window.local_storage().ok().flatten();

        let ldt_json = storage
            .as_ref()
            .and_then(|s| s.get_item("eulumdat_current_json").ok().flatten());

        if let Some(json) = ldt_json {
            // Parse JSON to get intensities for JS calculation
            if let Ok(ldt_value) = js_sys::JSON::parse(&json) {
                let result = calculate_gmaps_heatmap(&ldt_value, grid_spacing.get());

                // Parse results
                if !result.is_null() && !result.is_undefined() {
                    let get_num = |key: &str| -> f64 {
                        js_sys::Reflect::get(&result, &JsValue::from_str(key))
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0)
                    };

                    set_results.set(Some(CalculationResults {
                        min_lux: get_num("minLux"),
                        max_lux: get_num("maxLux"),
                        avg_lux: get_num("avgLux"),
                        uniformity: get_num("uniformity"),
                        point_count: get_num("pointCount") as usize,
                    }));
                }
            }
        } else {
            web_sys::console::warn_1(&"No LDT data in localStorage".into());
        }
    };

    // Export to CSV
    let export_csv = move |_| {
        let csv = export_gmaps_to_csv();
        if let Some(csv_str) = csv.as_string() {
            // Create download
            let blob =
                web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&csv_str.into())).unwrap();
            let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

            let document = web_sys::window().unwrap().document().unwrap();
            let a = document.create_element("a").unwrap();
            a.set_attribute("href", &url).unwrap();
            a.set_attribute("download", "illuminance_grid.csv").unwrap();
            a.dyn_ref::<web_sys::HtmlElement>().unwrap().click();

            web_sys::Url::revoke_object_url(&url).unwrap();
        }
    };

    // Clear all
    let clear_all = move |_| {
        clear_gmaps_designer();
        set_results.set(None);
        set_luminaire_count.set(0);
        set_has_polygon.set(false);
    };

    view! {
        <div class="gmaps-designer-container" style="width: 100%; height: 100%; position: relative; display: flex;">
            // Control panel (left side)
            <div
                class="gmaps-control-panel"
                style=move || format!(
                    "width: {}; background: var(--bg-secondary, #2a2a2a); border-right: 1px solid var(--border-color, #444); \
                     padding: {}; overflow-y: auto; transition: width 0.2s, padding 0.2s; display: flex; flex-direction: column; gap: 12px;",
                    if show_controls.get() { "280px" } else { "40px" },
                    if show_controls.get() { "12px" } else { "8px" }
                )
            >
                // Toggle button
                <button
                    style="width: 100%; padding: 6px; background: var(--bg-tertiary, #333); \
                           border: 1px solid var(--border-color, #444); border-radius: 4px; color: var(--text-primary, #fff); \
                           cursor: pointer; font-size: 14px;"
                    on:click=move |_| set_show_controls.update(|v| *v = !*v)
                >
                    {move || if show_controls.get() { "◀ Hide" } else { "▶" }}
                </button>

                {move || {
                    if !show_controls.get() {
                        return view! { <div></div> }.into_any();
                    }

                    let _l = locale.get();

                    view! {
                        <div style="display: flex; flex-direction: column; gap: 12px;">
                            // Instructions
                            <div style="font-size: 12px; color: var(--text-secondary, #aaa); padding: 8px; background: var(--bg-tertiary, #333); border-radius: 4px;">
                                <div style="font-weight: bold; margin-bottom: 4px;">"Instructions:"</div>
                                <ol style="margin: 0; padding-left: 16px; line-height: 1.6;">
                                    <li>"Draw a polygon (parking lot area)"</li>
                                    <li>"Place luminaires (markers)"</li>
                                    <li>"Click Calculate to see heatmap"</li>
                                </ol>
                            </div>

                            // Status
                            <div style="font-size: 12px; padding: 8px; background: var(--bg-tertiary, #333); border-radius: 4px;">
                                <div style="display: flex; justify-content: space-between; margin-bottom: 4px;">
                                    <span>"Area defined:"</span>
                                    <span style=move || if has_polygon.get() { "color: #4caf50;" } else { "color: #f44336;" }>
                                        {move || if has_polygon.get() { "✓" } else { "✗" }}
                                    </span>
                                </div>
                                <div style="display: flex; justify-content: space-between;">
                                    <span>"Luminaires:"</span>
                                    <span>{move || luminaire_count.get()}</span>
                                </div>
                            </div>

                            // Grid spacing
                            <div style="display: flex; flex-direction: column; gap: 4px;">
                                <label style="font-size: 11px; color: var(--text-secondary, #888);">
                                    "Grid Spacing (m)"
                                </label>
                                <input
                                    type="number"
                                    value=move || grid_spacing.get()
                                    min="0.5"
                                    max="5.0"
                                    step="0.5"
                                    style="width: 100%; padding: 6px; background: var(--bg-tertiary, #333); \
                                           border: 1px solid var(--border-color, #444); border-radius: 4px; \
                                           color: var(--text-primary, #fff);"
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_grid_spacing.set(v.clamp(0.5, 5.0));
                                        }
                                    }
                                />
                            </div>

                            // Action buttons
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                <button
                                    style="width: 100%; padding: 10px; background: #4caf50; color: white; \
                                           border: none; border-radius: 4px; cursor: pointer; font-size: 14px; font-weight: bold;"
                                    disabled=move || !has_polygon.get() || luminaire_count.get() == 0
                                    on:click=calculate
                                >
                                    "Calculate Heatmap"
                                </button>

                                <button
                                    style="width: 100%; padding: 8px; background: var(--bg-tertiary, #555); color: white; \
                                           border: 1px solid var(--border-color, #666); border-radius: 4px; cursor: pointer;"
                                    on:click=move |_| center_on_user_location()
                                >
                                    "📍 My Location"
                                </button>

                                <button
                                    style="width: 100%; padding: 8px; background: var(--bg-tertiary, #555); color: white; \
                                           border: 1px solid var(--border-color, #666); border-radius: 4px; cursor: pointer;"
                                    disabled=move || results.get().is_none()
                                    on:click=export_csv
                                >
                                    "📥 Export CSV"
                                </button>

                                <button
                                    style=move || format!("width: 100%; padding: 8px; background: {}; color: white; \
                                           border: 1px solid var(--border-color, #666); border-radius: 4px; cursor: pointer;",
                                           if show_labels.get() { "#2196F3" } else { "var(--bg-tertiary, #555)" })
                                    disabled=move || results.get().is_none()
                                    on:click=move |_| {
                                        let new_state = !show_labels.get();
                                        set_show_labels.set(new_state);
                                        toggle_gmaps_lux_labels(new_state);
                                    }
                                >
                                    {move || if show_labels.get() { "🔢 Hide Values" } else { "🔢 Show Values" }}
                                </button>

                                <button
                                    style="width: 100%; padding: 8px; background: #f44336; color: white; \
                                           border: none; border-radius: 4px; cursor: pointer;"
                                    on:click=clear_all
                                >
                                    "🗑 Clear All"
                                </button>
                            </div>

                            // Results
                            {move || {
                                results.get().map(|r| view! {
                                    <div style="padding: 12px; background: var(--bg-tertiary, #333); border-radius: 4px; font-size: 12px;">
                                        <div style="font-weight: bold; margin-bottom: 8px; color: var(--text-primary, #fff);">
                                            "Calculation Results"
                                        </div>
                                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
                                            <div>
                                                <div style="color: var(--text-secondary, #888);">"Min"</div>
                                                <div style="font-size: 16px; font-weight: bold;">{format!("{:.1} lux", r.min_lux)}</div>
                                            </div>
                                            <div>
                                                <div style="color: var(--text-secondary, #888);">"Max"</div>
                                                <div style="font-size: 16px; font-weight: bold;">{format!("{:.1} lux", r.max_lux)}</div>
                                            </div>
                                            <div>
                                                <div style="color: var(--text-secondary, #888);">"Average"</div>
                                                <div style="font-size: 16px; font-weight: bold;">{format!("{:.1} lux", r.avg_lux)}</div>
                                            </div>
                                            <div>
                                                <div style="color: var(--text-secondary, #888);">"Uniformity (U₀)"</div>
                                                <div style="font-size: 16px; font-weight: bold;">{format!("{:.2}", r.uniformity)}</div>
                                            </div>
                                        </div>
                                        <div style="margin-top: 8px; color: var(--text-tertiary, #666); font-size: 10px;">
                                            {format!("{} calculation points", r.point_count)}
                                        </div>

                                        // Uniformity assessment
                                        {
                                            let bg_color = if r.uniformity >= 0.4 { "#1b5e20" } else if r.uniformity >= 0.25 { "#f57f17" } else { "#b71c1c" };
                                            let msg = if r.uniformity >= 0.4 {
                                                "✓ Good uniformity (U₀ ≥ 0.40)"
                                            } else if r.uniformity >= 0.25 {
                                                "⚠ Acceptable uniformity (U₀ ≥ 0.25)"
                                            } else {
                                                "✗ Poor uniformity - add more luminaires"
                                            };
                                            view! {
                                                <div style=format!("margin-top: 8px; padding: 6px; border-radius: 4px; font-size: 11px; background: {};", bg_color)>
                                                    {msg}
                                                </div>
                                            }
                                        }
                                    </div>
                                })
                            }}

                            // Legend
                            <div style="padding: 8px; background: var(--bg-tertiary, #333); border-radius: 4px; font-size: 11px;">
                                <div style="font-weight: bold; margin-bottom: 8px;">"Heatmap Legend"</div>
                                <div style="display: flex; height: 16px; border-radius: 4px; overflow: hidden;">
                                    <div style="flex: 1; background: #00008b;"></div>
                                    <div style="flex: 1; background: #0000ff;"></div>
                                    <div style="flex: 1; background: #00ffff;"></div>
                                    <div style="flex: 1; background: #00ff00;"></div>
                                    <div style="flex: 1; background: #ffff00;"></div>
                                    <div style="flex: 1; background: #ff8000;"></div>
                                    <div style="flex: 1; background: #ff0000;"></div>
                                </div>
                                <div style="display: flex; justify-content: space-between; margin-top: 4px; color: var(--text-secondary, #888);">
                                    <span>"Low"</span>
                                    <span>"High"</span>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>

            // Map container (right side)
            <div style="flex: 1; position: relative; min-width: 0;">
                // Container for Google Maps
                <div
                    id="gmaps-container"
                    style="width: 100%; height: 100%;"
                />

                // Loading overlay
                {move || {
                    let _l = locale.get();
                    match load_state.get() {
                        GMapsLoadState::NotLoaded => view! {
                            <div class="gmaps-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white; cursor: pointer;
                            " on:click=start_loading>
                                <div style="font-size: 48px; margin-bottom: 20px;">"🗺️"</div>
                                <div style="font-size: 18px; margin-bottom: 10px;">"Lighting Designer"</div>
                                <div style="font-size: 14px; color: #aaa; margin-bottom: 20px; text-align: center; max-width: 400px;">
                                    "Design outdoor lighting on real satellite maps. Draw your parking lot, place luminaires, and calculate illuminance."
                                </div>
                                <button style="
                                    padding: 12px 24px; font-size: 16px;
                                    background: #4a9eff; color: white; border: none;
                                    border-radius: 8px; cursor: pointer;
                                ">"Load Google Maps"</button>
                            </div>
                        }.into_any(),

                        GMapsLoadState::Loading => view! {
                            <div class="gmaps-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white;
                            ">
                                <div class="spinner" style="
                                    width: 50px; height: 50px; border: 4px solid #333;
                                    border-top-color: #4a9eff; border-radius: 50%;
                                    animation: spin 1s linear infinite;
                                "></div>
                                <div style="margin-top: 20px; font-size: 16px;">"Loading Google Maps..."</div>
                            </div>
                            <style>"@keyframes spin { to { transform: rotate(360deg); } }"</style>
                        }.into_any(),

                        GMapsLoadState::Loaded => view! {
                            // Map is loaded, show drawing hint
                            <div style="
                                position: absolute; top: 10px; left: 50%; transform: translateX(-50%);
                                background: rgba(0,0,0,0.7); color: white; padding: 8px 16px;
                                border-radius: 4px; font-size: 12px; pointer-events: none;
                            ">
                                "Use toolbar: Polygon to draw area, Marker to place luminaires"
                            </div>
                        }.into_any(),

                        GMapsLoadState::Error => view! {
                            <div class="gmaps-overlay" style="
                                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                                display: flex; flex-direction: column; align-items: center; justify-content: center;
                                background: rgba(0,0,0,0.8); color: white;
                            ">
                                <div style="font-size: 48px; margin-bottom: 20px;">"❌"</div>
                                <div style="font-size: 18px; color: #ff6b6b;">"Failed to load Google Maps"</div>
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
                                        set_load_state.set(GMapsLoadState::NotLoaded);
                                        set_error_msg.set(None);
                                    }
                                >"Try Again"</button>
                            </div>
                        }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}
