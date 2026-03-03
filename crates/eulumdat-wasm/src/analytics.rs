//! Client-side analytics tracking.
//!
//! Sends usage events to the server's /api/track endpoint.
//! Events are sent asynchronously and failures are silently ignored
//! to avoid impacting the user experience.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Track an event with optional data
pub fn track(event: &str, data: Option<serde_json::Value>) {
    let event = event.to_string();
    spawn_local(async move {
        let _ = track_async(&event, data).await;
    });
}

/// Track an event asynchronously
async fn track_async(event: &str, data: Option<serde_json::Value>) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;

    // Build the request body
    let body = serde_json::json!({
        "event": event,
        "data": data,
    });

    // Use fetch API
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body.to_string()));

    let headers = web_sys::Headers::new()?;
    headers.set("Content-Type", "application/json")?;
    opts.set_headers(&headers);

    let request = web_sys::Request::new_with_str_and_init("/api/track", &opts)?;

    // Fire and forget - we don't wait for the response
    let _ = window.fetch_with_request(&request);

    Ok(())
}

// Convenience functions for common events

/// Track file open event
pub fn track_file_open(extension: &str) {
    track(
        "file_open",
        Some(serde_json::json!({
            "extension": extension.to_lowercase()
        })),
    );
}

/// Track export event
pub fn track_export(format: &str) {
    let event = format!("export_{}", format.to_lowercase());
    track(&event, None);
}

/// Track diagram view
pub fn track_diagram_view(diagram_type: &str) {
    track(
        "view_diagram",
        Some(serde_json::json!({
            "type": diagram_type
        })),
    );
}

/// Track 3D viewer load
pub fn track_3d_viewer_load() {
    track("load_3d_viewer", None);
}

/// Track template load
pub fn track_template_load(template_name: &str) {
    track(
        "load_template",
        Some(serde_json::json!({
            "template": template_name
        })),
    );
}
