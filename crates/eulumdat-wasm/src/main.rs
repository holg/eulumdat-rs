use eulumdat_wasm::{App, I18nProvider};
use leptos::prelude::*;

fn main() {
    // Better panic messages in the browser console
    console_error_panic_hook::set_once();

    // Mount the app with i18n provider
    leptos::mount::mount_to_body(|| {
        view! {
            <I18nProvider>
                <App />
            </I18nProvider>
        }
    });
}
