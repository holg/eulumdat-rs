// Skyglow Demo loader — Darkness Preservation Simulator
//
// Probes WebGPU at runtime and loads the matching Bevy bundle:
//   - WebGPU available    → ./skyglow/skyglow-demo.js (primary, full fidelity)
//   - WebGPU unavailable  → ./skyglow-webgl2/skyglow-demo.js (degraded fallback)
//   - Neither available   → window.skyglowBackend = 'unsupported',
//                           Leptos shell shows the missing-WebGPU screen
//
// In production, scripts/build-wasm-split.sh emits a hashed copy of this
// loader with hashed module paths baked in. This static file is the
// un-hashed fallback used during `trunk serve` development.

let skyglowLoaded = false;
let skyglowLoading = false;
let skyglowLoadPromise = null;
window.skyglowBackend = null;

async function probeWebGpu() {
    if (!navigator.gpu) return false;
    try {
        const adapter = await navigator.gpu.requestAdapter();
        return !!adapter;
    } catch (_) {
        return false;
    }
}

// `?force=webgl2` or `?force=webgpu` overrides the auto-probe. Useful
// for previewing the fallback experience on a machine that has WebGPU,
// or for writing reply links like "?wasm=skyglow_demo&force=webgl2".
function forcedBackend() {
    try {
        const p = new URLSearchParams(window.location.search).get('force');
        return p === 'webgl2' || p === 'webgpu' ? p : null;
    } catch (_) {
        return null;
    }
}

async function loadSkyglowDemo() {
    if (skyglowLoaded) {
        console.log("[Skyglow] Already loaded");
        return;
    }
    if (skyglowLoading && skyglowLoadPromise) {
        console.log("[Skyglow] Loading in progress, waiting...");
        return skyglowLoadPromise;
    }

    skyglowLoading = true;
    console.log("[Skyglow] Probing WebGPU…");

    skyglowLoadPromise = (async () => {
        const forced = forcedBackend();
        const webgpuOk = forced === 'webgl2' ? false
                       : forced === 'webgpu' ? true
                       : await probeWebGpu();
        const cacheBuster = Date.now();
        let modulePath;

        if (webgpuOk) {
            window.skyglowBackend = 'webgpu';
            modulePath = `./skyglow/skyglow-demo.js?v=${cacheBuster}`;
            console.log(
                forced === 'webgpu'
                    ? "[Skyglow] WebGPU forced via ?force=webgpu"
                    : "[Skyglow] WebGPU available — loading primary bundle"
            );
        } else {
            window.skyglowBackend = 'webgl2';
            modulePath = `./skyglow-webgl2/skyglow-demo.js?v=${cacheBuster}`;
            console.warn(
                (forced === 'webgl2'
                    ? "[Skyglow] WebGL2 forced via ?force=webgl2.\n"
                    : "[Skyglow] WebGPU unavailable — falling back to WebGL2 bundle.\n") +
                "  Reduced fidelity: no Bloom, no IBL, simpler clustered lighting.\n" +
                "  For full quality, use Chrome / Edge / Brave on Windows/Mac, Safari Tahoe, " +
                "or Firefox Nightly with dom.webgpu.enabled."
            );
        }

        try {
            const mod = await import(modulePath);
            await mod.default();
            skyglowLoaded = true;
            skyglowLoading = false;
            console.log("[Skyglow] Demo loaded successfully (" + window.skyglowBackend + ")");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Skyglow] Ignoring control flow exception (not a real error)");
                skyglowLoaded = true;
                skyglowLoading = false;
                return;
            }
            console.error("[Skyglow] Failed to load:", error);
            // If WebGL2 fallback also fails (e.g. bundle missing in dev),
            // mark backend unsupported so the Leptos shell shows the
            // missing-WebGPU screen rather than a generic error.
            if (window.skyglowBackend === 'webgl2') {
                window.skyglowBackend = 'unsupported';
            }
            skyglowLoading = false;
            skyglowLoadPromise = null;
            throw error;
        }
    })();

    return skyglowLoadPromise;
}

function isSkyglowLoaded() { return skyglowLoaded; }
function isSkyglowLoading() { return skyglowLoading; }
function skyglowBackend() { return window.skyglowBackend; }

window.loadSkyglowDemo = loadSkyglowDemo;
window.isSkyglowLoaded = isSkyglowLoaded;
window.isSkyglowLoading = isSkyglowLoading;
window.skyglowBackendName = skyglowBackend;

// Backwards-compatible aliases for the legacy "Obscura" naming.
window.loadObscuraDemo = loadSkyglowDemo;
window.isObscuraLoaded = isSkyglowLoaded;
window.isObscuraLoading = isSkyglowLoading;
