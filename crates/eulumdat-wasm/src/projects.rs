//! Projects: named sets of luminaires and sensors. The dashboard shows the
//! current project; opening a file adds it to the current project.
//!
//! The built-in **Demo** project ([`DEMO_ID`]) is not stored: the dashboard
//! shows the bundled templates and the demo sensors for it, and it is
//! read-only (a file opened while it is shown starts a new project).
//! User projects live in IndexedDB (`static/projects-store.js`, one JSON
//! record per project) and travel as a project file
//! ([`export_json`] / [`import_json`], `*.candelabra.json`).
//! The id of the current project is kept in localStorage
//! ([`CURRENT_KEY`]); each project remembers the luminaire open in the
//! editor ([`Project::active`], also in the project file), so a reload or
//! opening the project shows it again. The Demo project's last template is
//! kept in localStorage ([`DEMO_LUMINAIRE_KEY`]). The old "recently opened" library
//! (`crate::library`) is moved into a project once ([`from_library`]).

use serde::{Deserialize, Serialize};

use crate::library::{content_id, Library, LibrarySource};

/// Id of the built-in demo project.
pub const DEMO_ID: &str = "demo";
/// localStorage key with the id of the current project.
pub const CURRENT_KEY: &str = "eulumdat_current_project";
/// localStorage key with the id of the template last opened in the Demo.
pub const DEMO_LUMINAIRE_KEY: &str = "eulumdat_demo_luminaire";
/// localStorage flag: the old library was moved into a project.
pub const MIGRATED_KEY: &str = "eulumdat_library_migrated";
/// File name ending of exported projects.
pub const FILE_SUFFIX: &str = ".candelabra.json";
/// `format` value of a project file.
const FILE_FORMAT: &str = "candelabra-project";

/// Luminaire, sensor or Light scene.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Luminaire,
    Sensor,
    /// A saved Light scene (`eulumdat_scene::doc::SceneDoc` as JSON).
    Scene,
}

/// Item id of a saved Light scene: `scene-<doc id>`, so saving it again
/// replaces it (content ids change with every edit).
pub fn scene_item_id(content: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    let id = v.get("id")?.as_str()?;
    Some(format!("scene-{id}"))
}

/// One file of a project (raw text, re-parsed when used).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectItem {
    /// Content hash (`library::content_id`): the same file twice is one
    /// item. A scene: `scene-<doc id>` ([`scene_item_id`]).
    pub id: String,
    /// File name as shown (a GLDF part keeps `x.gldf/ldc/y.ldt`).
    pub name: String,
    pub content: String,
    pub kind: ItemKind,
    pub source: LibrarySource,
    /// `Date.now()` (ms) when added or last opened.
    pub added_at: f64,
}

impl ProjectItem {
    pub fn new(
        name: &str,
        content: String,
        kind: ItemKind,
        source: LibrarySource,
        now: f64,
    ) -> Self {
        let id = match kind {
            ItemKind::Scene => scene_item_id(&content).unwrap_or_else(|| content_id(&content)),
            _ => content_id(&content),
        };
        Self {
            id,
            name: name.to_string(),
            content,
            kind,
            source,
            added_at: now,
        }
    }

    /// GLDF content type of a sensor item (SensXML is sniffed).
    pub fn sensor_content_type(&self) -> &'static str {
        if self.content.contains("<SensXML") {
            "sensor/sensxml"
        } else {
            "sensor/sensldt"
        }
    }
}

/// A named set of luminaires and sensors.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Newest first.
    pub items: Vec<ProjectItem>,
    pub created_at: f64,
    pub modified_at: f64,
    /// Item id of the luminaire open in the editor (see
    /// [`Self::active_luminaire`]).
    #[serde(default)]
    pub active: Option<String>,
}

impl Project {
    /// The built-in demo project (its items are the bundled templates and
    /// the demo sensors, supplied by the UI, not stored here).
    pub fn demo(name: &str) -> Self {
        Self {
            id: DEMO_ID.to_string(),
            name: name.to_string(),
            items: Vec::new(),
            created_at: 0.0,
            modified_at: 0.0,
            active: None,
        }
    }

    /// An empty project with a fresh id (time + a per-session counter).
    pub fn new(name: &str, now: f64) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: format!(
                "p{:x}-{:x}",
                now as u64,
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
            name: name.to_string(),
            items: Vec::new(),
            created_at: now,
            modified_at: now,
            active: None,
        }
    }

    pub fn is_demo(&self) -> bool {
        self.id == DEMO_ID
    }

    pub fn luminaires(&self) -> impl Iterator<Item = &ProjectItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Luminaire)
    }

    pub fn sensors(&self) -> impl Iterator<Item = &ProjectItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Sensor)
    }

    pub fn scenes(&self) -> impl Iterator<Item = &ProjectItem> {
        self.items.iter().filter(|i| i.kind == ItemKind::Scene)
    }

    /// Add an item at the front; the same content moves to the front with
    /// the new name and time instead of being added twice.
    pub fn add(&mut self, item: ProjectItem) {
        self.items.retain(|i| i.id != item.id);
        self.modified_at = self.modified_at.max(item.added_at);
        self.items.insert(0, item);
    }

    pub fn remove(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
        if self.active.as_deref() == Some(id) {
            self.active = None;
        }
    }

    /// The luminaire to show for this project: the one open last, else
    /// the newest.
    pub fn active_luminaire(&self) -> Option<&ProjectItem> {
        self.active
            .as_deref()
            .and_then(|id| {
                self.items
                    .iter()
                    .find(|i| i.id == id && i.kind == ItemKind::Luminaire)
            })
            .or_else(|| self.luminaires().next())
    }

    /// A copy under a new name and id.
    pub fn duplicate(&self, name: &str, now: f64) -> Self {
        let mut p = Self::new(name, now);
        p.items = self.items.clone();
        p.active = self.active.clone();
        p
    }
}

/// Short description for the project list.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectMeta {
    pub id: String,
    pub name: String,
    pub luminaires: usize,
    pub sensors: usize,
    pub modified_at: f64,
}

impl From<&Project> for ProjectMeta {
    fn from(p: &Project) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            luminaires: p.luminaires().count(),
            sensors: p.sensors().count(),
            modified_at: p.modified_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FileItem {
    name: String,
    kind: ItemKind,
    content: String,
    #[serde(default = "local_file")]
    source: LibrarySource,
}

fn local_file() -> LibrarySource {
    LibrarySource::LocalFile
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    format: String,
    version: u32,
    name: String,
    items: Vec<FileItem>,
    /// Index in `items` of the luminaire open when the file was saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<usize>,
}

/// The project as a file (pretty JSON, the file contents inside).
pub fn export_json(p: &Project) -> String {
    export_json_with(p, true)
}

/// The project as a file, with or without its saved Light scenes.
pub fn export_json_with(p: &Project, scenes: bool) -> String {
    let items: Vec<&ProjectItem> = p
        .items
        .iter()
        .filter(|i| scenes || i.kind != ItemKind::Scene)
        .collect();
    let f = ProjectFile {
        format: FILE_FORMAT.to_string(),
        version: 1,
        name: p.name.clone(),
        items: items
            .iter()
            .map(|i| FileItem {
                name: i.name.clone(),
                kind: i.kind,
                content: i.content.clone(),
                source: i.source.clone(),
            })
            .collect(),
        active: p
            .active
            .as_deref()
            .and_then(|id| items.iter().position(|i| i.id == id)),
    };
    serde_json::to_string_pretty(&f).unwrap_or_default()
}

/// A project from a project file (new id; the order of the file is kept).
pub fn import_json(text: &str, now: f64) -> Result<Project, String> {
    let f: ProjectFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if f.format != FILE_FORMAT {
        return Err(format!("not a project file (format {:?})", f.format));
    }
    let mut p = Project::new(&f.name, now);
    let active = f
        .active
        .and_then(|k| f.items.get(k))
        .map(|it| content_id(&it.content));
    for (k, it) in f.items.into_iter().enumerate().rev() {
        p.add(ProjectItem::new(
            &it.name,
            it.content,
            it.kind,
            it.source,
            now - k as f64,
        ));
    }
    p.active = active.filter(|id| p.items.iter().any(|i| &i.id == id));
    p.modified_at = now;
    Ok(p)
}

/// The old "recently opened" library as a project (luminaire files only;
/// the library never held sensors). `None` when it is empty.
pub fn from_library(lib: &Library, name: &str, now: f64) -> Option<Project> {
    if lib.entries.is_empty() {
        return None;
    }
    let mut p = Project::new(name, now);
    for e in lib.entries.iter().rev() {
        p.add(ProjectItem {
            id: e.id.clone(),
            name: e.name.clone(),
            content: e.content.clone(),
            kind: ItemKind::Luminaire,
            source: e.source.clone(),
            added_at: e.added_at,
        });
    }
    p.modified_at = now;
    Some(p)
}

/// A name not used yet: `base`, `base 2`, `base 3`, …
pub fn unique_name(base: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == base) {
        return base.to_string();
    }
    // "Name 2" counts on ("Name 3"), not "Name 2 2". Larger numbers (a
    // year) are part of the name.
    let (stem, first) = match base.rsplit_once(' ') {
        Some((stem, n)) if !stem.is_empty() => match n.parse::<u32>() {
            Ok(n) if (2..1000).contains(&n) => (stem, n + 1),
            _ => (base, 2),
        },
        _ => (base, 2),
    };
    (first..)
        .map(|n| format!("{stem} {n}"))
        .find(|c| !taken.iter().any(|t| t == c))
        .unwrap_or_else(|| base.to_string())
}

// ── Browser storage ─────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod store_js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = projectsAll, catch)]
        pub async fn projects_all() -> Result<JsValue, JsValue>;
        #[wasm_bindgen(js_name = projectsPut, catch)]
        pub async fn projects_put(id: &str, json: &str) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(js_name = projectsDelete, catch)]
        pub async fn projects_delete(id: &str) -> Result<JsValue, JsValue>;
    }
}

/// `static/projects-store.js` is loaded (without it projects are kept in
/// memory only; never call into a missing global).
#[cfg(target_arch = "wasm32")]
fn store_available() -> bool {
    web_sys::window().is_some_and(|w| {
        js_sys::Reflect::get(&w, &"projectsPut".into()).is_ok_and(|f| f.is_function())
    })
}

/// All stored projects (empty if IndexedDB is not available).
#[cfg(target_arch = "wasm32")]
pub async fn load_all() -> Vec<Project> {
    if !store_available() {
        web_sys::console::error_1(&"projects: projects-store.js is not loaded".into());
        return Vec::new();
    }
    let Ok(v) = store_js::projects_all().await else {
        web_sys::console::error_1(&"projects: could not read IndexedDB".into());
        return Vec::new();
    };
    js_sys::Array::from(&v)
        .iter()
        .filter_map(|x| x.as_string())
        .filter_map(|s| serde_json::from_str::<Project>(&s).ok())
        .collect()
}

/// Write a project (not the demo).
#[cfg(target_arch = "wasm32")]
pub async fn save(p: &Project) {
    if p.is_demo() || !store_available() {
        return;
    }
    let Ok(json) = serde_json::to_string(p) else {
        return;
    };
    if let Err(e) = store_js::projects_put(&p.id, &json).await {
        web_sys::console::error_1(&format!("projects: write failed: {e:?}").into());
    }
}

/// Delete a stored project.
#[cfg(target_arch = "wasm32")]
pub async fn delete(id: &str) {
    if !store_available() {
        return;
    }
    if let Err(e) = store_js::projects_delete(id).await {
        web_sys::console::error_1(&format!("projects: delete failed: {e:?}").into());
    }
}

/// Id of the current project from localStorage.
#[cfg(target_arch = "wasm32")]
pub fn load_current_id() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(CURRENT_KEY)
        .ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn save_current_id(id: &str) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(CURRENT_KEY, id);
    }
}

/// Id of the template last opened while the Demo project was shown.
#[cfg(target_arch = "wasm32")]
pub fn load_demo_luminaire() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(DEMO_LUMINAIRE_KEY)
        .ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn save_demo_luminaire(template_id: &str) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(DEMO_LUMINAIRE_KEY, template_id);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_demo_luminaire() -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_demo_luminaire(_template_id: &str) {}

/// The old library was already moved into a project.
#[cfg(target_arch = "wasm32")]
pub fn library_migrated() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(MIGRATED_KEY).ok().flatten())
        .is_some()
}

#[cfg(target_arch = "wasm32")]
pub fn set_library_migrated() {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(MIGRATED_KEY, "1");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryEntry;

    fn item(name: &str, content: &str, kind: ItemKind, t: f64) -> ProjectItem {
        ProjectItem::new(name, content.into(), kind, LibrarySource::LocalFile, t)
    }

    #[test]
    fn add_dedups_by_content_and_refronts() {
        let mut p = Project::new("x", 1.0);
        p.add(item("a.ldt", "AAA", ItemKind::Luminaire, 2.0));
        p.add(item("s.ldt", "SSS", ItemKind::Sensor, 3.0));
        p.add(item("a-again.ldt", "AAA", ItemKind::Luminaire, 4.0));
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].name, "a-again.ldt");
        assert_eq!(p.luminaires().count(), 1);
        assert_eq!(p.sensors().count(), 1);
        assert_eq!(p.modified_at, 4.0);
        p.remove(&p.items[0].id.clone());
        assert_eq!(p.items.len(), 1);
    }

    #[test]
    fn export_import_round_trip() {
        let mut p = Project::new("Project X", 10.0);
        p.add(item("lum.ldt", "LUM", ItemKind::Luminaire, 11.0));
        p.add(item("pir.xml", "<SensXML/>", ItemKind::Sensor, 12.0));
        let json = export_json(&p);
        assert!(json.contains("candelabra-project"));
        let q = import_json(&json, 20.0).unwrap();
        assert_ne!(q.id, p.id);
        assert_eq!(q.name, "Project X");
        let names: Vec<_> = q.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["pir.xml", "lum.ldt"]);
        assert_eq!(q.items[0].kind, ItemKind::Sensor);
        assert_eq!(q.items[0].sensor_content_type(), "sensor/sensxml");
        assert!(import_json(
            "{\"format\":\"other\",\"version\":1,\"name\":\"\",\"items\":[]}",
            0.0
        )
        .is_err());
        assert!(import_json("not json", 0.0).is_err());
    }

    #[test]
    fn active_luminaire_is_remembered_and_travels_in_the_file() {
        let mut p = Project::new("Project X", 10.0);
        assert!(p.active_luminaire().is_none());
        p.add(item("a.ldt", "AAA", ItemKind::Luminaire, 11.0));
        p.add(item("b.ldt", "BBB", ItemKind::Luminaire, 12.0));
        p.add(item("s.xml", "<SensXML/>", ItemKind::Sensor, 13.0));
        // nothing chosen: the newest luminaire
        assert_eq!(p.active_luminaire().unwrap().name, "b.ldt");
        let a = p
            .items
            .iter()
            .find(|i| i.name == "a.ldt")
            .unwrap()
            .id
            .clone();
        p.active = Some(a.clone());
        assert_eq!(p.active_luminaire().unwrap().name, "a.ldt");
        // a sensor id is not a luminaire
        let s = p.items[0].id.clone();
        p.active = Some(s);
        assert_eq!(p.active_luminaire().unwrap().name, "b.ldt");
        p.active = Some(a.clone());

        let json = export_json(&p);
        assert!(json.contains("\"active\": 2"), "{json}");
        let q = import_json(&json, 20.0).unwrap();
        assert_eq!(q.active.as_deref(), Some(a.as_str()));
        assert_eq!(q.active_luminaire().unwrap().name, "a.ldt");
        assert_eq!(p.duplicate("copy", 30.0).active, p.active);

        // older files without the field still open
        let old = json.replace(",\n  \"active\": 2", "");
        assert!(!old.contains("active"));
        assert_eq!(import_json(&old, 21.0).unwrap().active, None);

        // removing the open luminaire falls back to the newest
        p.remove(&a);
        assert_eq!(p.active, None);
        assert_eq!(p.active_luminaire().unwrap().name, "b.ldt");
    }

    #[test]
    fn stored_projects_without_active_still_load() {
        let p = Project::new("x", 1.0);
        let mut v: serde_json::Value = serde_json::to_value(&p).unwrap();
        v.as_object_mut().unwrap().remove("active");
        let q: Project = serde_json::from_value(v).unwrap();
        assert_eq!(q.active, None);
    }

    #[test]
    fn library_becomes_a_project() {
        assert!(from_library(&Library::default(), "My files", 1.0).is_none());
        let mut lib = Library::default();
        for (i, n) in ["old.ldt", "new.ldt"].iter().enumerate() {
            lib.push(LibraryEntry {
                id: content_id(n),
                name: n.to_string(),
                content: n.to_string(),
                source: LibrarySource::LocalFile,
                added_at: i as f64,
            });
        }
        let p = from_library(&lib, "My files", 5.0).unwrap();
        let names: Vec<_> = p.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["new.ldt", "old.ldt"]);
        assert!(p.items.iter().all(|i| i.kind == ItemKind::Luminaire));
    }

    #[test]
    fn unique_names() {
        let taken = vec![
            "Untitled project".to_string(),
            "Untitled project 2".to_string(),
        ];
        assert_eq!(
            unique_name("Untitled project", &taken),
            "Untitled project 3"
        );
        assert_eq!(unique_name("Other", &taken), "Other");
        // a numbered name counts on
        assert_eq!(
            unique_name("Untitled project 2", &taken),
            "Untitled project 3"
        );
        let years = vec!["Lab 2026".to_string()];
        assert_eq!(unique_name("Lab 2026", &years), "Lab 2026 2");
    }

    #[test]
    fn demo_is_special() {
        assert!(Project::demo("Demo").is_demo());
        assert!(!Project::new("x", 1.0).is_demo());
        assert_ne!(Project::new("a", 1.0).id, DEMO_ID);
    }

    #[test]
    fn scenes_keep_their_id_and_travel_in_the_file() {
        let mut p = Project::new("x", 1.0);
        let doc = |name: &str, x: f32| {
            format!(
                r#"{{"id":"s1","name":"{name}","settings":{{"scene":"street"}},"moves":[["Person",[{x},1.0]]]}}"#
            )
        };
        p.add(item("Street", &doc("Street", 1.0), ItemKind::Scene, 2.0));
        assert_eq!(p.items[0].id, "scene-s1");
        // saved again after an edit: the same item, new content
        p.add(item(
            "Street 2",
            &doc("Street 2", 3.0),
            ItemKind::Scene,
            3.0,
        ));
        assert_eq!(p.scenes().count(), 1);
        assert_eq!(p.items[0].name, "Street 2");
        assert!(p.items[0].content.contains("3"));
        assert_eq!(p.luminaires().count(), 0);
        let q = import_json(&export_json(&p), 9.0).unwrap();
        assert_eq!(q.scenes().count(), 1);
        assert_eq!(q.items[0].id, "scene-s1");
        assert_eq!(q.items[0].kind, ItemKind::Scene);
    }

    #[test]
    fn a_project_file_can_leave_the_scenes_out() {
        let mut p = Project::new("x", 1.0);
        p.add(item("a.ldt", "LDT A", ItemKind::Luminaire, 1.0));
        p.add(item(
            "Street",
            r#"{"id":"s1","name":"Street"}"#,
            ItemKind::Scene,
            2.0,
        ));
        p.add(item("b.ldt", "LDT B", ItemKind::Luminaire, 3.0));
        // the open luminaire sits behind the scene in the list
        p.active = Some(p.luminaires().last().unwrap().id.clone());
        let active_name = p.active_luminaire().unwrap().name.clone();
        let q = import_json(&export_json_with(&p, false), 9.0).unwrap();
        assert_eq!(q.scenes().count(), 0);
        assert_eq!(q.luminaires().count(), 2);
        assert_eq!(q.active_luminaire().unwrap().name, active_name);
        let r = import_json(&export_json_with(&p, true), 9.0).unwrap();
        assert_eq!(r.scenes().count(), 1);
    }
}
