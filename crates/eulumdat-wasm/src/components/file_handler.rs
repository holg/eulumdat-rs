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

/// Download an SVG file
pub fn download_svg(filename: &str, content: &str) {
    download_file(filename, content, "image/svg+xml");
}

/// Download binary bytes (e.g., PDF)
pub fn download_bytes(filename: &str, content: &[u8], mime_type: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    // Create Uint8Array from bytes
    let array = js_sys::Uint8Array::from(content);
    let parts = Array::new();
    parts.push(&array.buffer());

    let options = BlobPropertyBag::new();
    options.set_type(mime_type);

    let blob = match Blob::new_with_buffer_source_sequence_and_options(&parts, &options) {
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

/// Export SVG as a raster image (PNG or JPEG) using the browser Canvas API.
///
/// Creates a Blob URL from the SVG, loads it into an `<img>`, draws onto
/// an off-screen `<canvas>`, then triggers a download via `canvas.toDataURL()`.
fn export_svg_as_raster(
    filename: &str,
    svg_content: &str,
    scale: f64,
    mime_type: &str,
    quality: Option<f64>,
) {
    use wasm_bindgen::closure::Closure;

    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    // Create a Blob URL for the SVG (avoids base64 encoding)
    let svg_blob = {
        let parts = Array::new();
        parts.push(&JsValue::from_str(svg_content));
        let opts = BlobPropertyBag::new();
        opts.set_type("image/svg+xml");
        match Blob::new_with_str_sequence_and_options(&parts, &opts) {
            Ok(b) => b,
            Err(_) => return,
        }
    };
    let svg_url = match Url::create_object_url_with_blob(&svg_blob) {
        Ok(u) => u,
        Err(_) => return,
    };

    let (w, h) = parse_svg_dimensions(svg_content).unwrap_or((800.0, 600.0));
    let canvas_w = (w * scale) as u32;
    let canvas_h = (h * scale) as u32;

    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(canvas_w);
    canvas.set_height(canvas_h);

    // Fill white background
    if let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
    {
        ctx.set_fill_style_str("white");
        ctx.fill_rect(0.0, 0.0, canvas_w as f64, canvas_h as f64);
    }

    let img = web_sys::HtmlImageElement::new().unwrap();
    let img_clone = img.clone();
    let filename_owned = filename.to_string();
    let mime_owned = mime_type.to_string();
    let svg_url_clone = svg_url.clone();

    let onload = Closure::once(move || {
        if let Some(ctx) = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        {
            let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
                &img_clone,
                0.0,
                0.0,
                canvas_w as f64,
                canvas_h as f64,
            );
        }

        // Revoke the SVG blob URL now that we've drawn it
        let _ = Url::revoke_object_url(&svg_url_clone);

        // Export canvas as data URL
        let result = if let Some(q) = quality {
            canvas.to_data_url_with_type_and_encoder_options(&mime_owned, &JsValue::from_f64(q))
        } else {
            canvas.to_data_url_with_type(&mime_owned)
        };

        if let Ok(data_url) = result {
            trigger_download_from_url(&data_url, &filename_owned);
        }
    });

    img.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget(); // One-shot closure, acceptable leak
    img.set_src(&svg_url);
}

/// Export SVG as PNG (2x scale for crisp rendering).
pub fn export_svg_as_png(filename: &str, svg_content: &str, scale: f64) {
    export_svg_as_raster(filename, svg_content, scale, "image/png", None);
}

/// Export SVG as JPEG with quality (0.0–1.0).
pub fn export_svg_as_jpeg(filename: &str, svg_content: &str, scale: f64, quality: f64) {
    export_svg_as_raster(filename, svg_content, scale, "image/jpeg", Some(quality));
}

/// Trigger a file download from a data URL.
fn trigger_download_from_url(url: &str, filename: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(a) = document.create_element("a") {
            let _ = a.set_attribute("href", url);
            let _ = a.set_attribute("download", filename);
            if let Some(body) = document.body() {
                let _ = body.append_child(&a);
                if let Ok(html_a) = a.dyn_into::<web_sys::HtmlElement>() {
                    html_a.click();
                    let _ = body.remove_child(&html_a);
                }
            }
        }
    }
}

/// Parse width/height from SVG content (viewBox or width/height attributes).
fn parse_svg_dimensions(svg: &str) -> Option<(f64, f64)> {
    // Try viewBox first: viewBox="0 0 W H"
    if let Some(vb_start) = svg.find("viewBox=\"") {
        let rest = &svg[vb_start + 9..];
        if let Some(vb_end) = rest.find('"') {
            let parts: Vec<&str> = rest[..vb_end].split_whitespace().collect();
            if parts.len() == 4 {
                if let (Ok(w), Ok(h)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    return Some((w, h));
                }
            }
        }
    }
    // Fallback: try width="..." height="..."
    let w = extract_attr(svg, "width")?;
    let h = extract_attr(svg, "height")?;
    Some((w, h))
}

fn extract_attr(svg: &str, attr: &str) -> Option<f64> {
    let needle = format!("{}=\"", attr);
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].trim_end_matches("px").parse().ok()
}

/// Open SVG in a new browser tab
pub fn open_svg_in_new_tab(content: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    // Create blob
    let array = Array::new();
    array.push(&JsValue::from_str(content));

    let options = BlobPropertyBag::new();
    options.set_type("image/svg+xml");

    let blob = match Blob::new_with_str_sequence_and_options(&array, &options) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Create object URL
    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };

    // Open in new tab
    let _ = window.open_with_url_and_target(&url, "_blank");
}
