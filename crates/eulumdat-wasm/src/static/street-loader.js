// Lazy loader for the Street Designer companion app.
// Pattern mirrors bevy-loader.js: single entry point exposed on window,
// imports the WASM module on demand, subsequent calls are idempotent.

let streetLoaded = false;
let streetLoading = false;
let streetLoadPromise = null;

/**
 * Load and initialize the street designer WASM module.
 * @returns {Promise<void>}
 */
async function loadStreetDesigner() {
    if (streetLoaded) {
        console.log("[Street] Already loaded");
        return;
    }
    if (streetLoading && streetLoadPromise) {
        console.log("[Street] Loading in progress, waiting...");
        return streetLoadPromise;
    }

    streetLoading = true;
    console.log("[Street] Loading street designer...");

    streetLoadPromise = (async () => {
        try {
            const cacheBuster = Date.now();
            const mod = await import(`./street/eulumdat-wasm-street.js?v=${cacheBuster}`);
            await mod.default();
            // The #[wasm_bindgen] pub fn mount() in lib.rs is exposed as mod.mount()
            if (typeof mod.mount === "function") {
                mod.mount();
            }
            streetLoaded = true;
            streetLoading = false;
            console.log("[Street] Loaded");
        } catch (error) {
            console.error("[Street] Failed to load:", error);
            streetLoading = false;
            streetLoadPromise = null;
            throw error;
        }
    })();

    return streetLoadPromise;
}

function isStreetDesignerLoaded() {
    return streetLoaded;
}

function isStreetDesignerLoading() {
    return streetLoading;
}

window.loadStreetDesigner = loadStreetDesigner;
window.isStreetDesignerLoaded = isStreetDesignerLoaded;
window.isStreetDesignerLoading = isStreetDesignerLoading;
