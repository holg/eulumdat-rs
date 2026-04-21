// Lazy loader for the Street Designer companion app.
// Pattern mirrors bevy-loader.js: single entry point exposed on window,
// imports the WASM module on demand, subsequent calls are idempotent.
//
// After the first successful load the module stays cached on `window`, so
// re-entering the tab (which unmounts and re-mounts #street-root) only
// triggers a cheap mount() call — no re-download, no re-init.

let streetModule = null;
let streetInitialized = false;
let streetLoading = false;
let streetLoadPromise = null;

/**
 * Load (if needed) and mount the street designer into #street-root.
 * Safe to call multiple times — subsequent calls just re-mount.
 * @returns {Promise<void>}
 */
async function loadStreetDesigner() {
    if (streetInitialized && streetModule) {
        // Already in memory — just mount into whatever #street-root exists now.
        if (typeof streetModule.mount === "function") {
            streetModule.mount();
        }
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
            streetModule = mod;
            if (typeof mod.mount === "function") {
                mod.mount();
            }
            streetInitialized = true;
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
    return streetInitialized;
}

function isStreetDesignerLoading() {
    return streetLoading;
}

window.loadStreetDesigner = loadStreetDesigner;
window.isStreetDesignerLoaded = isStreetDesignerLoaded;
window.isStreetDesignerLoading = isStreetDesignerLoading;
