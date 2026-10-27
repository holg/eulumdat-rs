//! Bindings for the tm25.eu viewer page: the TM-25 report as Typst source
//! from the JSON the Bevy viewer publishes; the page then compiles it with
//! the Typst bundle the editor already ships.

use wasm_bindgen::prelude::*;

/// Typst source of the report. `exports` and `luminance` may be empty.
#[wasm_bindgen]
pub fn tm25_report_typst(
    status: &str,
    exports: &str,
    luminance: &str,
    file_name: &str,
) -> Result<String, JsError> {
    eulumdat_typst::tm25_report::typst_from_json(status, exports, luminance, file_name)
        .map_err(|e| JsError::new(&e))
}
