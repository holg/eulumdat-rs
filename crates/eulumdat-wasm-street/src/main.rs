//! Standalone entry point (used when the street designer is loaded as its
//! own HTML page via trunk, not as a lazily-imported module from the main
//! editor).

use eulumdat_wasm_street::StreetApp;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(StreetApp);
}
