//! Projects in the app: the current project (what the dashboard shows),
//! the stored projects, and the dashboard's project bar and sensor rows.
//! The model and the storage live in `crate::projects`.
//!
//! Every opened file goes through [`ProjectsCtx::add`]: into the current
//! project, or, while the read-only Demo project is shown, into a new
//! "Untitled project". The app's sensor list (`SensorFiles`) is rebuilt
//! from the current project ([`ProjectsCtx::sensor_files`]).

use std::cell::Cell;

use leptos::prelude::*;

use super::sensor_view::{demo_sensors, SensorFile, SensorFiles};
use crate::i18n::use_locale;
use crate::library::LibrarySource;
use crate::projects::{self, ItemKind, Project, ProjectItem, ProjectMeta, DEMO_ID};

thread_local! {
    static CTX: Cell<Option<ProjectsCtx>> = const { Cell::new(None) };
}

/// The app's projects (set up once in `App`).
pub fn ctx() -> Option<ProjectsCtx> {
    CTX.with(|c| c.get())
}

fn now() -> f64 {
    js_sys::Date::now()
}

/// Projects state (signals, `Copy`).
#[derive(Clone, Copy)]
pub struct ProjectsCtx {
    /// Stored (user) projects, newest change first.
    pub projects: RwSignal<Vec<Project>>,
    /// `DEMO_ID` or the id of a stored project.
    pub current_id: RwSignal<String>,
    /// Localised names for new projects ("Untitled project", "My files",
    /// "copy"), kept current by `App`.
    pub names: RwSignal<(String, String, String)>,
    /// Bumped when the editor should open the current project's luminaire
    /// ([`Self::active_item`]; Demo: the template opened last): after a
    /// reload, when the user picks a project, when a project file is
    /// opened. `App` does the loading.
    pub open_active: RwSignal<u32>,
}

impl ProjectsCtx {
    /// Set up the projects and load the stored ones. `force_demo`: a link
    /// asked for demo content, show the Demo project whatever was current.
    /// `restore`: open the project's luminaire once loaded (not when a link
    /// brings its own luminaire).
    pub fn init(force_demo: bool, restore: bool) -> Self {
        let ctx = Self {
            projects: RwSignal::new(Vec::new()),
            current_id: RwSignal::new(DEMO_ID.to_string()),
            names: RwSignal::new(("Untitled project".into(), "My files".into(), "copy".into())),
            open_active: RwSignal::new(0),
        };
        CTX.with(|c| c.set(Some(ctx)));
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let mut stored = projects::load_all().await;
            let mut start = if force_demo {
                None
            } else {
                projects::load_current_id()
            };
            // The old "recently opened" list becomes a project, once.
            if !projects::library_migrated() {
                let lib = crate::library::Library::load();
                let name = ctx.names.get_untracked().1;
                if let Some(p) = projects::from_library(&lib, &name, now()) {
                    projects::save(&p).await;
                    if start.is_none() && !force_demo {
                        start = Some(p.id.clone());
                    }
                    stored.push(p);
                }
                projects::set_library_migrated();
            }
            // Files opened while loading are already in the list: merge.
            ctx.projects.update(|v| {
                for p in stored {
                    if !v.iter().any(|x| x.id == p.id) {
                        v.push(p);
                    }
                }
                v.sort_by(|a, b| b.modified_at.total_cmp(&a.modified_at));
            });
            // Nothing opened while loading: show the stored current project
            // and its luminaire.
            let still_demo = ctx.current_id.get_untracked() == DEMO_ID;
            if let Some(id) = start {
                if still_demo
                    && ctx
                        .projects
                        .with_untracked(|v| v.iter().any(|p| p.id == id))
                {
                    ctx.current_id.set(id);
                }
            }
            if still_demo && restore {
                ctx.request_open();
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (force_demo, restore);
        ctx
    }

    /// The Demo project is shown (tracked).
    pub fn is_demo(&self) -> bool {
        self.current_id.get() == DEMO_ID
    }

    /// Run `f` on the current project (tracked); `None` for the Demo.
    pub fn with_current<R>(&self, f: impl FnOnce(&Project) -> R) -> Option<R> {
        let id = self.current_id.get();
        if id == DEMO_ID {
            return None;
        }
        self.projects.with(|v| v.iter().find(|p| p.id == id).map(f))
    }

    /// Name of the current project (tracked), `None` for the Demo.
    pub fn current_name(&self) -> Option<String> {
        self.with_current(|p| p.name.clone())
    }

    /// Change the current stored project and write it.
    fn mutate(&self, f: impl FnOnce(&mut Project)) {
        self.update_current(true, f);
    }

    /// `touch`: count it as a change (modification time, list order).
    fn update_current(&self, touch: bool, f: impl FnOnce(&mut Project)) {
        let id = self.current_id.get_untracked();
        if id == DEMO_ID {
            return;
        }
        let mut saved = None;
        self.projects.update(|v| {
            if let Some(p) = v.iter_mut().find(|p| p.id == id) {
                f(p);
                if touch {
                    p.modified_at = now();
                }
                saved = Some(p.clone());
            }
            if touch {
                v.sort_by(|a, b| b.modified_at.total_cmp(&a.modified_at));
            }
        });
        if let Some(p) = saved {
            persist(p);
        }
    }

    /// Ask the editor to open the current project's luminaire.
    pub fn request_open(&self) {
        self.open_active.update(|n| *n = n.wrapping_add(1));
    }

    /// The luminaire of the current project to show (untracked): the one
    /// open last, else the newest. `None` for the Demo.
    pub fn active_item(&self) -> Option<ProjectItem> {
        self.with_current_untracked(|p| p.active_luminaire().cloned())
            .flatten()
    }

    /// Id of that luminaire (tracked; the dashboard selects its row).
    pub fn active_id(&self) -> Option<String> {
        self.with_current(|p| p.active_luminaire().map(|i| i.id.clone()))
            .flatten()
    }

    /// The editor now shows this item of the current project.
    pub fn set_active(&self, item_id: &str) {
        let known = self
            .with_current_untracked(|p| {
                p.active.as_deref() != Some(item_id)
                    && p.items
                        .iter()
                        .any(|i| i.id == item_id && i.kind == ItemKind::Luminaire)
            })
            .unwrap_or(false);
        if known {
            let id = item_id.to_string();
            self.update_current(false, |p| p.active = Some(id));
        }
    }

    fn taken_names(&self) -> Vec<String> {
        self.projects
            .with_untracked(|v| v.iter().map(|p| p.name.clone()).collect())
    }

    /// Store a new project and make it current.
    fn start(&self, p: Project) {
        let id = p.id.clone();
        persist(p.clone());
        self.projects.update(|v| v.insert(0, p));
        self.switch(&id);
    }

    /// Add an opened file to the current project; while the Demo is shown
    /// a new "Untitled project" is started with it.
    /// An added luminaire is the one the editor shows.
    pub fn add(&self, name: &str, content: String, kind: ItemKind, source: LibrarySource) {
        let item = ProjectItem::new(name, content, kind, source, now());
        let shown = (kind == ItemKind::Luminaire).then(|| item.id.clone());
        let add = move |p: &mut Project| {
            p.add(item);
            if shown.is_some() {
                p.active = shown;
            }
        };
        if self.current_id.get_untracked() == DEMO_ID {
            let base = self.names.get_untracked().0;
            let mut p = Project::new(&projects::unique_name(&base, &self.taken_names()), now());
            add(&mut p);
            self.start(p);
        } else {
            self.mutate(add);
        }
    }

    /// Save a Light scene (JSON `SceneDoc`) in the current project; the same
    /// scene again replaces it. While the Demo is shown a new project starts.
    pub fn put_scene(&self, name: &str, json: String) {
        self.add(name, json, ItemKind::Scene, LibrarySource::LocalFile);
    }

    /// The scene item `scene-<doc id>` is in the current project (untracked).
    pub fn has_scene(&self, doc_id: &str) -> bool {
        let id = format!("scene-{doc_id}");
        self.with_current_untracked(|p| p.items.iter().any(|i| i.id == id))
            .unwrap_or(false)
    }

    /// The demo sensors in the current project (the Demo has them already).
    pub fn add_demo_sensors(&self) {
        if self.current_id.get_untracked() == DEMO_ID {
            return;
        }
        for d in demo_sensors().into_iter().rev() {
            if let Some(content) = demo_sensor_text(&d.name) {
                self.add(&d.name, content, ItemKind::Sensor, LibrarySource::Template);
            }
        }
    }

    pub fn remove(&self, id: &str) {
        let id = id.to_string();
        self.mutate(|p| p.remove(&id));
    }

    /// Remove every luminaire of the current project (sensors stay).
    pub fn remove_luminaires(&self) {
        self.mutate(|p| p.items.retain(|i| i.kind != ItemKind::Luminaire));
    }

    pub fn switch(&self, id: &str) {
        self.current_id.set(id.to_string());
        #[cfg(target_arch = "wasm32")]
        projects::save_current_id(id);
    }

    /// The user picked a project: show it and open its luminaire.
    pub fn choose(&self, id: &str) {
        if self.current_id.get_untracked() != id {
            self.switch(id);
            self.request_open();
        }
    }

    pub fn new_project(&self) {
        let base = self.names.get_untracked().0;
        self.start(Project::new(
            &projects::unique_name(&base, &self.taken_names()),
            now(),
        ));
    }

    pub fn rename(&self, name: &str) {
        let name = name.trim().to_string();
        if !name.is_empty() {
            self.mutate(|p| p.name = name);
        }
    }

    /// A copy of the current project under "`name` copy".
    pub fn save_as_copy(&self) {
        let Some(p) = self.with_current_untracked(|p| p.clone()) else {
            return;
        };
        let suffix = self.names.get_untracked().2;
        let name = projects::unique_name(&format!("{} {suffix}", p.name), &self.taken_names());
        self.start(p.duplicate(&name, now()));
    }

    fn with_current_untracked<R>(&self, f: impl FnOnce(&Project) -> R) -> Option<R> {
        let id = self.current_id.get_untracked();
        self.projects
            .with_untracked(|v| v.iter().find(|p| p.id == id).map(f))
    }

    /// Delete the current project and show the Demo.
    pub fn delete_current(&self) {
        let id = self.current_id.get_untracked();
        if id == DEMO_ID {
            return;
        }
        self.projects.update(|v| v.retain(|p| p.id != id));
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move { projects::delete(&id).await });
        self.switch(DEMO_ID);
    }

    /// Download the current project as a project file.
    pub fn export_current(&self) {
        self.export_current_with(true);
    }

    /// Saved Light scenes in the current project.
    pub fn scene_count(&self) -> usize {
        let id = self.current_id.get();
        self.projects.with(|v| {
            v.iter()
                .find(|p| p.id == id)
                .map_or(0, |p| p.scenes().count())
        })
    }

    /// Download the current project, with or without its saved scenes.
    pub fn export_current_with(&self, scenes: bool) {
        if let Some((name, json)) =
            self.with_current_untracked(|p| (p.name.clone(), projects::export_json_with(p, scenes)))
        {
            let safe: String = name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            super::file_handler::download_file(
                &format!("{safe}{}", projects::FILE_SUFFIX),
                &json,
                "application/json",
            );
        }
    }

    /// Open a project file as a new project, with the luminaire that was
    /// open when it was saved.
    pub fn import_text(&self, text: &str) -> Result<(), String> {
        let mut p = projects::import_json(text, now())?;
        p.name = projects::unique_name(&p.name, &self.taken_names());
        self.start(p);
        self.request_open();
        Ok(())
    }

    /// Ids of the sensors shown (tracked): what `sensor_files` depends on.
    pub fn sensor_key(&self) -> (bool, Vec<String>) {
        if self.is_demo() {
            (true, Vec::new())
        } else {
            (
                false,
                self.with_current(|p| p.sensors().map(|i| i.id.clone()).collect())
                    .unwrap_or_default(),
            )
        }
    }

    /// The sensors of the current project, parsed (untracked). Names are
    /// made unique ("name (2)"): the pickers select by name.
    pub fn sensor_files(&self) -> Vec<SensorFile> {
        if self.current_id.get_untracked() == DEMO_ID {
            return demo_sensors();
        }
        self.with_current_untracked(|p| {
            unique_sensor_names(p)
                .into_iter()
                .filter_map(|(name, i)| {
                    SensorFile::from_bytes(&name, i.sensor_content_type(), i.content.as_bytes())
                })
                .collect()
        })
        .unwrap_or_default()
    }

    /// (shown name, item id) of the current project's sensors (tracked;
    /// empty for the Demo, whose sensors cannot be removed).
    pub fn sensor_item_ids(&self) -> Vec<(String, String)> {
        self.with_current(|p| {
            unique_sensor_names(p)
                .into_iter()
                .map(|(name, i)| (name, i.id.clone()))
                .collect()
        })
        .unwrap_or_default()
    }

    /// Luminaires, sensors and saved Light scenes of the current project
    /// with their kind (tracked): the Demo lists the bundled templates and
    /// the demo sensors.
    /// Sensor names are the ones of the app's sensor list.
    pub fn entries(&self) -> Vec<ProjectEntry> {
        if self.is_demo() {
            let mut v: Vec<ProjectEntry> = super::templates::ALL_TEMPLATES
                .iter()
                .enumerate()
                .map(|(i, t)| ProjectEntry {
                    name: t.name.to_string(),
                    kind: ItemKind::Luminaire,
                    source: EntrySource::Template(i),
                })
                .collect();
            v.extend(demo_sensors().into_iter().map(|d| ProjectEntry {
                name: d.name,
                kind: ItemKind::Sensor,
                source: EntrySource::DemoSensor,
            }));
            return v;
        }
        self.with_current(|p| {
            let mut v: Vec<ProjectEntry> = p
                .luminaires()
                .map(|i| ProjectEntry {
                    name: i.name.clone(),
                    kind: ItemKind::Luminaire,
                    source: EntrySource::Item(i.id.clone()),
                })
                .collect();
            v.extend(
                unique_sensor_names(p)
                    .into_iter()
                    .map(|(name, i)| ProjectEntry {
                        name,
                        kind: ItemKind::Sensor,
                        source: EntrySource::Item(i.id.clone()),
                    }),
            );
            v.extend(p.scenes().map(|i| ProjectEntry {
                name: i.name.clone(),
                kind: ItemKind::Scene,
                source: EntrySource::Item(i.id.clone()),
            }));
            v
        })
        .unwrap_or_default()
    }

    /// A stored item of the current project (untracked).
    pub fn item(&self, id: &str) -> Option<ProjectItem> {
        self.with_current_untracked(|p| p.items.iter().find(|i| i.id == id).cloned())
            .flatten()
    }

    /// Short list for the project selector (tracked).
    pub fn metas(&self) -> Vec<ProjectMeta> {
        self.projects
            .with(|v| v.iter().map(ProjectMeta::from).collect())
    }
}

/// Where a project entry comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum EntrySource {
    /// Bundled template (index in `templates::ALL_TEMPLATES`), Demo project.
    Template(usize),
    /// A demo sensor (Demo project; in the app's sensor list by name).
    DemoSensor,
    /// A stored item of a user project (item id).
    Item(String),
}

/// One luminaire or sensor of the current project, flagged with its kind:
/// opening it needs no detection.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectEntry {
    pub name: String,
    pub kind: ItemKind,
    pub source: EntrySource,
}

/// Selection state of the dashboard's sensor rows (shared with the
/// dashboard sidebar's compare button).
#[derive(Clone, Copy)]
pub struct SensorSelection {
    pub selected: RwSignal<Option<String>>,
    pub expanded: RwSignal<Option<String>>,
    pub compare: RwSignal<Option<String>>,
}

impl SensorSelection {
    pub fn new() -> Self {
        Self {
            selected: RwSignal::new(None),
            expanded: RwSignal::new(None),
            compare: RwSignal::new(None),
        }
    }

    /// (A, B) when two different sensors are marked (tracked).
    pub fn pair(&self) -> Option<(String, String)> {
        match (self.selected.get(), self.compare.get()) {
            (Some(a), Some(b)) if a != b => Some((a, b)),
            _ => None,
        }
    }
}

impl Default for SensorSelection {
    fn default() -> Self {
        Self::new()
    }
}

/// The project's sensors with names made unique: a second file of the same
/// name (different content) is shown as "name (2)".
fn unique_sensor_names(p: &Project) -> Vec<(String, &ProjectItem)> {
    let mut seen: Vec<String> = Vec::new();
    p.sensors()
        .map(|i| {
            let mut name = i.name.clone();
            let mut n = 2;
            while seen.contains(&name) {
                name = format!("{} ({n})", i.name);
                n += 1;
            }
            seen.push(name.clone());
            (name, i)
        })
        .collect()
}

fn persist(p: Project) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move { projects::save(&p).await });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = p;
}

/// Text of a demo sensor (for adding it to a project).
fn demo_sensor_text(name: &str) -> Option<String> {
    super::sensor_view::demo_sensor_sources()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
}

/// Project selector and actions above the dashboard table.
#[component]
pub fn ProjectBar() -> impl IntoView {
    let locale = use_locale();
    let Some(ctx) = ctx() else {
        return ().into_any();
    };
    let confirm_delete = RwSignal::new(false);
    let import_error = RwSignal::new(None::<String>);
    let import_ref = NodeRef::<leptos::html::Input>::new();
    let on_import = move |_| {
        let Some(input) = import_ref.get() else {
            return;
        };
        let Some(file) = input.files().and_then(|l| l.get(0)) else {
            return;
        };
        input.set_value("");
        let name = file.name();
        wasm_bindgen_futures::spawn_local(async move {
            let text = gloo_file::futures::read_as_text(&file.into())
                .await
                .unwrap_or_default();
            if ctx.import_text(&text).is_err() {
                let tpl = locale
                    .get_untracked()
                    .dashboard
                    .projects
                    .import_error
                    .clone();
                import_error.set(Some(tpl.replace("{name}", &name)));
            } else {
                import_error.set(None);
            }
        });
    };

    view! {
        <div class="project-bar">
            <span class="project-label">{move || format!("📁 {}", locale.get().dashboard.projects.project)}</span>
            <select class="project-select" on:change=move |ev| {
                confirm_delete.set(false);
                ctx.choose(&event_target_value(&ev));
            }>
                <option value=DEMO_ID selected=move || ctx.is_demo()>
                    {move || locale.get().dashboard.projects.demo.clone()}
                </option>
                {move || ctx.metas().into_iter().map(|m| {
                    let id = m.id.clone();
                    let counts = locale.get().dashboard.projects.counts
                        .replace("{l}", &m.luminaires.to_string())
                        .replace("{s}", &m.sensors.to_string());
                    view! {
                        <option value=m.id.clone() selected=move || ctx.current_id.get() == id>
                            {format!("{} ({counts})", m.name)}
                        </option>
                    }
                }).collect_view()}
            </select>
            {move || (!ctx.is_demo()).then(|| view! {
                <input class="project-name" type="text"
                    title=move || locale.get().dashboard.projects.name_hint.clone()
                    prop:value=move || ctx.current_name().unwrap_or_default()
                    on:change=move |ev| ctx.rename(&event_target_value(&ev))
                />
            })}
            <button class="project-btn" on:click=move |_| ctx.new_project()>
                {move || format!("＋ {}", locale.get().dashboard.projects.new)}
            </button>
            {move || (!ctx.is_demo()).then(|| view! {
                <button class="project-btn" on:click=move |_| ctx.save_as_copy()>
                    {move || locale.get().dashboard.projects.save_as.clone()}
                </button>
                <button class="project-btn" on:click=move |_| ctx.export_current()>
                    {move || format!("⬇ {}", locale.get().dashboard.projects.export)}
                </button>
            })}
            <label class="project-btn">
                {move || format!("⬆ {}", locale.get().dashboard.projects.import)}
                <input type="file" accept=".json" style="display:none" node_ref=import_ref on:change=on_import/>
            </label>
            {move || (!ctx.is_demo()).then(|| view! {
                <button class=move || if confirm_delete.get() { "project-btn danger" } else { "project-btn" }
                    on:click=move |_| {
                        if confirm_delete.get_untracked() {
                            confirm_delete.set(false);
                            ctx.delete_current();
                        } else {
                            confirm_delete.set(true);
                        }
                    }
                >{move || {
                    let l = locale.get();
                    if confirm_delete.get() {
                        l.dashboard.projects.confirm_delete
                            .replace("{name}", &ctx.current_name().unwrap_or_default())
                    } else {
                        format!("🗑 {}", l.dashboard.projects.delete)
                    }
                }}</button>
            })}
        </div>
        {move || import_error.get().map(|e| view! { <p class="project-note project-error">{format!("⚠ {e}")}</p> })}
        {move || {
            let l = locale.get();
            if ctx.is_demo() {
                Some(view! { <p class="project-note">{l.dashboard.projects.demo_note.clone()}</p> }.into_any())
            } else if ctx.with_current(|p| p.items.is_empty()).unwrap_or(false) {
                Some(view! { <p class="project-note">{l.dashboard.projects.empty.clone()}</p> }.into_any())
            } else {
                None
            }
        }}
    }
    .into_any()
}

/// The sensors of the current project as dashboard rows, like the
/// luminaire rows: a click selects a sensor and opens its diagrams below
/// the row (click again to fold), Ctrl/⌘ + click marks a second sensor, and
/// "Compare" opens both in the Compare tab.
#[component]
pub fn SensorRows(
    /// Open the full sensor page (index in the app's sensor list).
    on_open: Callback<usize>,
    /// Compare two sensors (names in the app's sensor list).
    on_compare: Callback<(String, String)>,
    /// Selection, shared with the dashboard (sidebar compare button).
    sel: SensorSelection,
    /// A sensor row was clicked (the dashboard tracks which list was used
    /// last, for its compare button).
    #[prop(optional)]
    on_touch: Option<Callback<()>>,
) -> impl IntoView {
    let locale = use_locale();
    let files = use_context::<SensorFiles>()
        .map(|f| f.0)
        .unwrap_or_else(|| RwSignal::new(Vec::new()));
    let ctx = ctx();
    let (selected, expanded, compare) = (sel.selected, sel.expanded, sel.compare);
    // Choices of a project that was left are dropped.
    Effect::new(move |_| {
        let names: Vec<String> = files.with(|v| v.iter().map(|f| f.name.clone()).collect());
        for sig in [selected, expanded, compare] {
            if sig.get_untracked().is_some_and(|n| !names.contains(&n)) {
                sig.set(None);
            }
        }
    });
    let pair = Memo::new(move |_| sel.pair());

    move || {
        let list = files.get();
        if list.is_empty() {
            return None;
        }
        let l = locale.get();
        let p = l.dashboard.projects.clone();
        let ids = ctx.map(|c| c.sensor_item_ids()).unwrap_or_default();
        let rows = list
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                let core = f.data.to_core();
                let h = core.default_height();
                let range = core
                    .areas(h, false)
                    .into_iter()
                    .filter(|a| a.detected())
                    .map(|a| a.max_range())
                    .fold(0.0, f64::max);
                let kind = if core.is_sensxml() {
                    p.sensxml.clone()
                } else {
                    p.sensor_ldt.clone()
                };
                let info = if range > 0.0 {
                    p.range
                        .replace("{r}", &format!("{range:.1}"))
                        .replace("{h}", &format!("{h:.1}"))
                } else {
                    String::new()
                };
                let remove_id = ids
                    .iter()
                    .find(|(n, _)| *n == f.name)
                    .map(|(_, id)| id.clone());
                let remove_title = p.remove.clone();
                let name = f.name.clone();
                let (n1, n2, n3, n4, n5) =
                    (name.clone(), name.clone(), name.clone(), name.clone(), name.clone());
                view! {
                    <div
                        class=move || {
                            let mut c = String::from("sensor-row");
                            if selected.get().as_deref() == Some(n1.as_str()) {
                                c.push_str(" selected");
                            }
                            if compare.get().as_deref() == Some(n1.as_str()) {
                                c.push_str(" compare-target");
                            }
                            c
                        }
                        on:click=move |ev: leptos::ev::MouseEvent| {
                            if let Some(t) = on_touch {
                                t.run(());
                            }
                            if ev.ctrl_key() || ev.meta_key() {
                                let again = compare.get_untracked().as_deref() == Some(n2.as_str());
                                compare.set(if again { None } else { Some(n2.clone()) });
                            } else if selected.get_untracked().as_deref() == Some(n2.as_str()) {
                                let open = expanded.get_untracked().as_deref() == Some(n2.as_str());
                                expanded.set(if open { None } else { Some(n2.clone()) });
                            } else {
                                selected.set(Some(n2.clone()));
                                expanded.set(Some(n2.clone()));
                            }
                        }
                    >
                        <span class=move || if expanded.get().as_deref() == Some(n3.as_str()) { "expand-arrow open" } else { "expand-arrow" }>"▶"</span>
                        <span class="sensor-row-icon">"📡"</span>
                        <span class="sensor-row-name">{f.name.clone()}</span>
                        <span class="sensor-row-kind">{kind}</span>
                        <span class="sensor-row-info">{info}</span>
                        {remove_id.map(|id| view! {
                            <button class="library-remove-btn" title=remove_title
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    if let Some(c) = ctx {
                                        c.remove(&id);
                                    }
                                }
                            >"×"</button>
                        })}
                    </div>
                    {move || (expanded.get().as_deref() == Some(n4.as_str())).then(|| {
                        let data = files.with_untracked(|v| v.iter().find(|x| x.name == n5).map(|x| x.data.clone()));
                        data.map(|d| view! { <SensorDetail data=d index=i on_open=on_open/> })
                    })}
                }
            })
            .collect_view();
        let count = files.with(|v| v.len());
        Some(view! {
            <div class="dashboard-sensors">
                <div class="dashboard-sensors-head">
                    <h4>{format!("📡 {} ({count})", p.sensors_title)}</h4>
                    <span class="dashboard-hint">{p.compare_hint.clone()}</span>
                    {move || pair.get().map(|(a, b)| {
                        let label = locale.get().dashboard.projects.compare_selected
                            .replace("{a}", &a)
                            .replace("{b}", &b);
                        view! {
                            <button class="project-btn sensor-compare-btn"
                                on:click=move |_| on_compare.run((a.clone(), b.clone()))
                            >{label}</button>
                        }
                    })}
                </div>
                {rows}
            </div>
        })
    }
}

/// Diagrams and numbers of one sensor below its dashboard row: detection
/// areas at the reference height, polar and cartesian of the walking
/// volume (ranges in m), a table per motion type.
#[component]
fn SensorDetail(
    data: super::sensor_view::SensorData,
    index: usize,
    on_open: Callback<usize>,
) -> impl IntoView {
    use eulumdat::diagram::SvgTheme;
    use eulumdat::sensor::{diagrams, Motion, SensorDiagram};
    let locale = use_locale();
    let core = data.to_core();
    let h = core.default_height();
    let volume = core.motion_volume(Motion::Tangential, false);
    move || {
        let l = locale.get();
        let ls = l.ui.sensor.clone();
        let theme = diagrams::sensor_theme(&SvgTheme::css_variables_with_locale(&l));
        let label = |m: Motion| super::zonal_sensors::motion_label(&ls, m);
        let plan = core.plan_svg_with_labels(h, false, 340.0, &theme, &label);
        let (polar, cart) = match &volume {
            Some(v) => (
                diagrams::render(v, SensorDiagram::Polar, 340.0, 340.0, h, &theme),
                diagrams::render(v, SensorDiagram::Cartesian, 420.0, 300.0, h, &theme),
            ),
            None => (String::new(), String::new()),
        };
        let rows = core
            .areas(h, false)
            .into_iter()
            .map(|a| {
                let q = match a.quality {
                    Some(eulumdat::sensor::Quality::Interpolated) => " (i)",
                    Some(eulumdat::sensor::Quality::Extrapolated) => " (e)",
                    _ => "",
                };
                view! {
                    <tr>
                        <td>{label(a.motion)}</td>
                        <td>{if a.detected() { format!("{:.2} m{q}", a.max_range()) } else { ls.not_detected.clone() }}</td>
                        <td>{if a.detected() { format!("{:.1} m²", a.area_m2()) } else { "–".into() }}</td>
                        <td>{if a.detected() { format!("{:.2} m", a.diameter()) } else { "–".into() }}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <div class="luminaire-detail sensor-detail">
                <div class="detail-diagram">
                    <h4>{format!("{} · {h:.1} m", ls.view_plan)}</h4>
                    <div class="sensor-detail-svg" inner_html=plan></div>
                </div>
                {(!polar.is_empty()).then(|| view! {
                    <div class="detail-diagram">
                        <h4>{format!("{} · {}", ls.view_polar, label(Motion::Tangential))}</h4>
                        <div class="sensor-detail-svg" inner_html=polar.clone()></div>
                    </div>
                })}
                {(!cart.is_empty()).then(|| view! {
                    <div class="detail-diagram">
                        <h4>{ls.view_cartesian.clone()}</h4>
                        <div class="sensor-detail-svg" inner_html=cart.clone()></div>
                    </div>
                })}
                <div class="detail-diagram sensor-detail-table">
                    <table class="area-stats-table">
                        <thead><tr>
                            <th>{ls.motion.clone()}</th>
                            <th>{ls.max_range.clone()}</th>
                            <th>{ls.area.clone()}</th>
                            <th>{ls.diameter.clone()}</th>
                        </tr></thead>
                        <tbody>{rows}</tbody>
                    </table>
                    <button class="project-btn" on:click=move |_| on_open.run(index)>
                        {format!("📡 {}", l.dashboard.projects.open_sensor)}
                    </button>
                </div>
            </div>
        }
    }
}
