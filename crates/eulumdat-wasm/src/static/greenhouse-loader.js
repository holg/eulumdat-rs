// Greenhouse Demo loader — "Vom Schweinestall zum Gewächshaus"
//
// Lazy-loads the Bevy greenhouse bundle (WebGPU only: the physically based
// atmosphere and bloom passes need compute shaders + storage textures).
//   - WebGPU available    → ./greenhouse/greenhouse-demo.js
//   - WebGPU unavailable  → window.greenhouseBackend = 'unsupported',
//                           the Leptos shell shows the missing-WebGPU screen
//
// In production, scripts/build-wasm-split.sh emits a hashed copy of this
// loader with the hashed module path baked in. This static file is the
// un-hashed fallback used during `trunk serve` development.

let greenhouseLoaded = false;
let greenhouseLoading = false;
let greenhouseLoadPromise = null;
window.greenhouseBackend = null;

async function probeWebGpuForGreenhouse() {
    if (!navigator.gpu) return false;
    try {
        const adapter = await navigator.gpu.requestAdapter();
        return !!adapter;
    } catch (_) {
        return false;
    }
}

async function loadGreenhouseDemo() {
    if (greenhouseLoaded) {
        console.log("[Greenhouse] Already loaded");
        return;
    }
    if (greenhouseLoading && greenhouseLoadPromise) {
        console.log("[Greenhouse] Loading in progress, waiting...");
        return greenhouseLoadPromise;
    }

    greenhouseLoading = true;
    console.log("[Greenhouse] Probing WebGPU…");

    greenhouseLoadPromise = (async () => {
        const webgpuOk = await probeWebGpuForGreenhouse();
        if (!webgpuOk) {
            window.greenhouseBackend = 'unsupported';
            greenhouseLoading = false;
            greenhouseLoadPromise = null;
            const msg = "WebGPU unavailable — the greenhouse demo needs WebGPU.";
            console.error("[Greenhouse]", msg);
            throw new Error(msg);
        }
        window.greenhouseBackend = 'webgpu';
        try {
            const mod = await import('./greenhouse/greenhouse-demo.js');
            await mod.default();
            greenhouseLoaded = true;
            greenhouseLoading = false;
            console.log("[Greenhouse] Demo loaded successfully");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Greenhouse] Ignoring control flow exception (not a real error)");
                greenhouseLoaded = true;
                greenhouseLoading = false;
                return;
            }
            console.error("[Greenhouse] Failed to load:", error);
            greenhouseLoading = false;
            greenhouseLoadPromise = null;
            throw error;
        }
    })();

    return greenhouseLoadPromise;
}

window.loadGreenhouseDemo = loadGreenhouseDemo;
window.isGreenhouseLoaded = () => greenhouseLoaded;
window.isGreenhouseLoading = () => greenhouseLoading;

console.log("[Greenhouse] Loader ready (dev, un-hashed)");
