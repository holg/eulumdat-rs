// Path-traced scene viewer loader
//
// Lazy-loads the Bevy rt-viewer bundle (WebGPU only: the tracer runs as
// compute shaders). The bundle reads the editor's current luminaire from
// localStorage["eulumdat_current_ldt"] and re-reads it when
// localStorage["eulumdat_ldt_timestamp"] changes.
//
// In production, scripts/build-wasm-split.sh emits a hashed copy of this
// loader with the hashed module path baked in. This static file is the
// un-hashed fallback used during `trunk serve` development.

let rtViewerLoaded = false;
let rtViewerLoading = false;
let rtViewerLoadPromise = null;
window.rtViewerBackend = null;

async function probeWebGpuForRtViewer() {
    if (!navigator.gpu) return false;
    try {
        const adapter = await navigator.gpu.requestAdapter();
        return !!adapter;
    } catch (_) {
        return false;
    }
}

async function loadRtViewer() {
    if (rtViewerLoaded) {
        return;
    }
    if (rtViewerLoading && rtViewerLoadPromise) {
        return rtViewerLoadPromise;
    }
    rtViewerLoading = true;
    rtViewerLoadPromise = (async () => {
        const webgpuOk = await probeWebGpuForRtViewer();
        if (!webgpuOk) {
            window.rtViewerBackend = 'unsupported';
            rtViewerLoading = false;
            rtViewerLoadPromise = null;
            const msg = "WebGPU unavailable — the scene viewer needs WebGPU.";
            console.error("[RtViewer]", msg);
            throw new Error(msg);
        }
        window.rtViewerBackend = 'webgpu';
        try {
            const mod = await import('./rt-viewer/rt-viewer.js');
            await mod.default();
            rtViewerLoaded = true;
            rtViewerLoading = false;
            console.log("[RtViewer] Viewer loaded");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                rtViewerLoaded = true;
                rtViewerLoading = false;
                return;
            }
            console.error("[RtViewer] Failed to load:", error);
            rtViewerLoading = false;
            rtViewerLoadPromise = null;
            throw error;
        }
    })();
    return rtViewerLoadPromise;
}

window.loadRtViewer = loadRtViewer;
window.isRtViewerLoaded = () => rtViewerLoaded;
window.isRtViewerLoading = () => rtViewerLoading;
