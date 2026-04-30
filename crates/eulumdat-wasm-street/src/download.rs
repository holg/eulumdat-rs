//! Browser file-download helpers used by the export buttons.
//!
//! All entry points take an in-memory string and a filename, build a
//! `Blob`, hang it off a hidden `<a download>`, click it, and revoke the
//! object URL. Errors are logged to the console rather than surfaced —
//! a failed download is a UX issue, not a data-integrity one.

use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Blob, BlobPropertyBag, HtmlAnchorElement, Url};

fn save_blob(content: &str, filename: &str, mime: &str) -> Result<(), JsValue> {
    let parts = Array::new();
    parts.push(&JsValue::from_str(content));

    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &opts)?;

    let url = Url::create_object_url_with_blob(&blob)?;

    let document = window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let anchor: HtmlAnchorElement = document
        .create_element("a")?
        .dyn_into()
        .map_err(|_| JsValue::from_str("anchor cast failed"))?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none")?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&anchor)?;
    anchor.click();
    body.remove_child(&anchor)?;
    Url::revoke_object_url(&url)?;
    Ok(())
}

pub fn save_svg(svg: &str, filename: &str) {
    if let Err(e) = save_blob(svg, filename, "image/svg+xml") {
        web_sys::console::error_1(&e);
    }
}

pub fn save_csv(csv: &str, filename: &str) {
    if let Err(e) = save_blob(csv, filename, "text/csv") {
        web_sys::console::error_1(&e);
    }
}

pub fn save_json(json: &str, filename: &str) {
    if let Err(e) = save_blob(json, filename, "application/json") {
        web_sys::console::error_1(&e);
    }
}

/// Save a binary blob (e.g., PDF bytes) as a download. Mirrors `save_blob`
/// but keeps the bytes intact instead of treating them as a string.
fn save_bytes(bytes: &[u8], filename: &str, mime: &str) -> Result<(), JsValue> {
    let view = Uint8Array::from(bytes);
    let parts = Array::new();
    parts.push(&view);

    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &opts)?;

    let url = Url::create_object_url_with_blob(&blob)?;

    let document = window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let anchor: HtmlAnchorElement = document
        .create_element("a")?
        .dyn_into()
        .map_err(|_| JsValue::from_str("anchor cast failed"))?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none")?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&anchor)?;
    anchor.click();
    body.remove_child(&anchor)?;
    Url::revoke_object_url(&url)?;
    Ok(())
}

pub fn save_pdf(pdf: &[u8], filename: &str) {
    if let Err(e) = save_bytes(pdf, filename, "application/pdf") {
        web_sys::console::error_1(&e);
    }
}
