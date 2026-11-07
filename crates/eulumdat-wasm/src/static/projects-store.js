// Projects store: named sets of luminaires and sensors shown on the
// dashboard (crates/eulumdat-wasm/src/projects.rs). IndexedDB, one record
// per project (JSON string), so projects are not bound to the ~5 MB
// localStorage budget.

const PROJECTS_DB = "candelabra-projects";
const PROJECTS_STORE = "projects";

function projectsOpenDb() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(PROJECTS_DB, 1);
        req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains(PROJECTS_STORE)) {
                db.createObjectStore(PROJECTS_STORE);
            }
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

function projectsTx(mode, fn) {
    return projectsOpenDb().then(
        (db) =>
            new Promise((resolve, reject) => {
                const tx = db.transaction(PROJECTS_STORE, mode);
                const out = fn(tx.objectStore(PROJECTS_STORE));
                tx.oncomplete = () => resolve(out && out.result !== undefined ? out.result : undefined);
                tx.onerror = () => reject(tx.error);
                tx.onabort = () => reject(tx.error);
            })
    );
}

// All projects as JSON strings.
window.projectsAll = function () {
    return projectsTx("readonly", (s) => s.getAll()).then((v) => v || []);
};

// Write one project (JSON string) under its id.
window.projectsPut = function (id, json) {
    return projectsTx("readwrite", (s) => {
        s.put(json, id);
    });
};

// Delete one project.
window.projectsDelete = function (id) {
    return projectsTx("readwrite", (s) => {
        s.delete(id);
    });
};
