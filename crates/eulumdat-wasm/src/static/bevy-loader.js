// Lazy loader for Bevy 3D Scene Viewer
// This module dynamically loads the Bevy WASM when the user opens the 3D Scene tab

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

/**
 * Load and initialize the Bevy 3D viewer
 * The Bevy app is configured to target canvas#bevy-canvas
 * @returns {Promise<void>}
 */
async function loadBevyViewer() {
    // Already loaded
    if (bevyLoaded) {
        console.log("[Bevy] Already loaded");
        return;
    }

    // Loading in progress, wait for it
    if (bevyLoading && loadPromise) {
        console.log("[Bevy] Loading in progress, waiting...");
        return loadPromise;
    }

    bevyLoading = true;
    console.log("[Bevy] Loading 3D viewer (~22MB)...");

    loadPromise = (async () => {
        try {
            // Dynamically import the Bevy module with cache-busting timestamp
            // Path is relative to the HTML file location
            const cacheBuster = Date.now();
            const bevy = await import(`./bevy/eulumdat-3d.js?v=${cacheBuster}`);

            // Initialize the WASM module
            // This calls wasm.__wbindgen_start() which runs main()
            // The Bevy app targets canvas#bevy-canvas via WindowPlugin config
            await bevy.default();

            bevyLoaded = true;
            bevyLoading = false;
            console.log("[Bevy] 3D viewer loaded successfully");
        } catch (error) {
            // Bevy/WASM uses exceptions for control flow - ignore these "fake" errors
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Bevy] Ignoring control flow exception (not a real error)");
                bevyLoaded = true;
                bevyLoading = false;
                return;
            }

            console.error("[Bevy] Failed to load 3D viewer:", error);
            bevyLoading = false;
            loadPromise = null;
            throw error;
        }
    })();

    return loadPromise;
}

/**
 * Check if Bevy is currently loaded
 * @returns {boolean}
 */
function isBevyLoaded() {
    return bevyLoaded;
}

/**
 * Check if Bevy is currently loading
 * @returns {boolean}
 */
function isBevyLoading() {
    return bevyLoading;
}

// Expose to window for Leptos/WASM to call
window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;
