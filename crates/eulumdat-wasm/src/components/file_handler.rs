use js_sys::Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Url};

/// Download a file in the browser
pub fn download_file(filename: &str, content: &str, mime_type: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    // Create blob
    let array = Array::new();
    array.push(&JsValue::from_str(content));

    let options = BlobPropertyBag::new();
    options.set_type(mime_type);

    let blob = match Blob::new_with_str_sequence_and_options(&array, &options) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Create object URL
    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };

    // Create download link
    let a = match document.create_element("a") {
        Ok(e) => e,
        Err(_) => return,
    };

    let _ = a.set_attribute("href", &url);
    let _ = a.set_attribute("download", filename);
    a.set_text_content(Some("download"));

    // Append to body, click, and remove
    if let Some(body) = document.body() {
        let _ = body.append_child(&a);

        if let Ok(html_a) = a.dyn_into::<web_sys::HtmlElement>() {
            html_a.click();
            let _ = body.remove_child(&html_a);
        }
    }

    // Revoke URL
    let _ = Url::revoke_object_url(&url);
}

/// Download an IES file
pub fn download_ies(filename: &str, content: &str) {
    download_file(filename, content, "text/plain");
}

/// Download an ATLA XML file
pub fn download_atla_xml(filename: &str, content: &str) {
    download_file(filename, content, "application/xml");
}

/// Download an ATLA JSON file
pub fn download_atla_json(filename: &str, content: &str) {
    download_file(filename, content, "application/json");
}
