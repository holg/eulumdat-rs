// Lazy loader for Obscura Demo (Darkness Preservation Simulator)
// This module dynamically loads the Obscura WASM when activated via ?wasm=obscura_demo

let obscuraLoaded = false;
let obscuraLoading = false;
let obscuraLoadPromise = null;

/**
 * Load and initialize the Obscura Demo viewer
 * The Bevy app is configured to target canvas#obscura-canvas
 * @returns {Promise<void>}
 */
async function loadObscuraDemo() {
    if (obscuraLoaded) {
        console.log("[Obscura] Already loaded");
        return;
    }

    if (obscuraLoading && obscuraLoadPromise) {
        console.log("[Obscura] Loading in progress, waiting...");
        return obscuraLoadPromise;
    }

    obscuraLoading = true;
    console.log("[Obscura] Loading Darkness Preservation Simulator...");

    obscuraLoadPromise = (async () => {
        try {
            const cacheBuster = Date.now();
            const mod = await import(`./obscura/obscura-demo.js?v=${cacheBuster}`);
            await mod.default();
            obscuraLoaded = true;
            obscuraLoading = false;
            console.log("[Obscura] Demo loaded successfully");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Obscura] Ignoring control flow exception (not a real error)");
                obscuraLoaded = true;
                obscuraLoading = false;
                return;
            }
            console.error("[Obscura] Failed to load:", error);
            obscuraLoading = false;
            obscuraLoadPromise = null;
            throw error;
        }
    })();

    return obscuraLoadPromise;
}

function isObscuraLoaded() { return obscuraLoaded; }
function isObscuraLoading() { return obscuraLoading; }

window.loadObscuraDemo = loadObscuraDemo;
window.isObscuraLoaded = isObscuraLoaded;
window.isObscuraLoading = isObscuraLoading;
