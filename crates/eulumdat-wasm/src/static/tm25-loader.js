// TM-25 Ray File Viewer loader
//
// Lazy-loads the Bevy TM-25 viewer bundle (WebGPU only, like the greenhouse
// demo) and provides the file hand-off between the Leptos shell and the
// bundle: `tm25StashFile(file)` stores a File/Blob in IndexedDB, the Bevy
// side polls `tm25TakeFile()` and streams it with Blob.slice. IndexedDB (not
// a JS global) so the file survives the navigation to `?wasm=tm25`.
//
// In production, scripts/build-wasm-split.sh emits a hashed copy of this
// loader with the hashed module path baked in. This static file is the
// un-hashed fallback used during `trunk serve` development.

let tm25Loaded = false;
let tm25Loading = false;
let tm25LoadPromise = null;
window.tm25Backend = null;

const TM25_DB = "eulumdat-tm25";
const TM25_STORE = "pending";

function tm25OpenDb() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(TM25_DB, 1);
        req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains(TM25_STORE)) {
                db.createObjectStore(TM25_STORE);
            }
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

// Store a File (or any structured-clonable value, e.g. a Uint8Array) under
// `key` ("file" = the viewer's input; "gonio" = the ray subset handed to the
// virtual goniometer). Resolves when written.
window.tm25StashFile = async function (file, key = "file") {
    const db = await tm25OpenDb();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(TM25_STORE, "readwrite");
        tx.objectStore(TM25_STORE).put(file, key);
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
    });
};

// Take (and remove) the value stashed under `key`. Resolves to it or null.
window.tm25TakeFile = async function (key = "file") {
    let db;
    try {
        db = await tm25OpenDb();
    } catch (_) {
        return null;
    }
    return new Promise((resolve) => {
        const tx = db.transaction(TM25_STORE, "readwrite");
        const store = tx.objectStore(TM25_STORE);
        const get = store.get(key);
        get.onsuccess = () => {
            const file = get.result || null;
            if (file) store.delete(key);
            resolve(file);
        };
        get.onerror = () => resolve(null);
    });
};

async function probeWebGpuForTm25() {
    if (!navigator.gpu) return false;
    try {
        const adapter = await navigator.gpu.requestAdapter();
        return !!adapter;
    } catch (_) {
        return false;
    }
}

async function loadTm25Viewer() {
    if (tm25Loaded) {
        return;
    }
    if (tm25Loading && tm25LoadPromise) {
        return tm25LoadPromise;
    }
    tm25Loading = true;
    tm25LoadPromise = (async () => {
        const webgpuOk = await probeWebGpuForTm25();
        if (!webgpuOk) {
            window.tm25Backend = 'unsupported';
            tm25Loading = false;
            tm25LoadPromise = null;
            const msg = "WebGPU unavailable — the TM-25 viewer needs WebGPU.";
            console.error("[TM25]", msg);
            throw new Error(msg);
        }
        window.tm25Backend = 'webgpu';
        try {
            const mod = await import('./tm25/tm25-viewer.js');
            await mod.default();
            tm25Loaded = true;
            tm25Loading = false;
            console.log("[TM25] Viewer loaded");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                tm25Loaded = true;
                tm25Loading = false;
                return;
            }
            console.error("[TM25] Failed to load:", error);
            tm25Loading = false;
            tm25LoadPromise = null;
            throw error;
        }
    })();
    return tm25LoadPromise;
}

window.loadTm25Viewer = loadTm25Viewer;
window.isTm25Loaded = () => tm25Loaded;
window.isTm25Loading = () => tm25Loading;
