use eulumdat_wasm::App;
use leptos::prelude::*;

fn main() {
    // Better panic messages in the browser console
    console_error_panic_hook::set_once();

    // Mount the app to the document body
    leptos::mount::mount_to_body(App);
}
