//! Skyglow Demo: Darkness Preservation Simulator
//!
//! A Bevy example showcasing skyglow (upward-scattered light pollution)
//! and darkness preservation. The two-mode toggle compares a polluting
//! road luminaire against a fully-shielded preservation-grade fixture
//! at the same target illuminance.
//!
//! Originally a pitch demo for L'Observatoire de la Nuit; the name was
//! generalized to "skyglow" (the IES/IDA term) so the demo works as a
//! vendor-neutral outreach piece for IDA chapters, amateur astronomy
//! clubs, and dark-sky tourism organizations.
//!
//! Controls:
//!   Space       — Toggle simulation mode (pollution ↔ preserved)
//!   WASD        — Move camera
//!   Arrow keys  — Rotate camera (look around)
//!   Q / E       — Up / Down
//!   Right-click + drag — Look around (mouse)
//!   Scroll      — Zoom
//!   R           — Reset camera
//!   P           — Toggle photometric solid visualization
//!   1 / 2 / 3 / 4 — Switch scene (Sponza / Urban / Bistro / Burg Vischering)
//!   Z           — Burg Vischering: raise / lower the drawbridge
//!   [ / ]       — Decrease / increase uplight %
//!   - / =       — Decrease / increase intensity
//!   9 / 0       — Decrease / increase haze
//!
//! Run:
//!   cargo run --example skyglow_demo -p eulumdat-bevy --features post-process,bevy-ui --release

use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
#[cfg(not(feature = "webgl2"))]
use bevy::light::TransmittedShadowReceiver;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{DistanceFog, FogFalloff};
#[cfg(not(feature = "webgl2"))]
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use bevy::render::view::NoIndirectDrawing;
use eulumdat::Eulumdat;
#[cfg(feature = "bevy-ui")]
use eulumdat_bevy::photometric::PhotometricData;
use eulumdat_bevy::photometric::{PhotometricLight, PhotometricPlugin};
use eulumdat_bevy::EulumdatLightBundle;

#[cfg(feature = "bevy-ui")]
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputDispatchPlugin,
};
#[cfg(feature = "bevy-ui")]
use bevy::picking::hover::Hovered;
#[cfg(feature = "bevy-ui")]
use bevy::ui_widgets::{
    observe, slider_self_update, Activate, Button as UiButton, Slider, SliderRange, SliderThumb,
    SliderValue, TrackClick, UiWidgetsPlugins, ValueChange,
};

// ---------------------------------------------------------------------------
// Embedded LDT data
// ---------------------------------------------------------------------------

const POLLUTION_LDT: &str = include_str!("../../eulumdat-wasm/templates/road_luminaire.ldt");
const PRESERVED_LDT: &str = include_str!("../../eulumdat-wasm/templates/projector.ldt");
const FLOOD_LDT: &str = include_str!("../../eulumdat-wasm/templates/wiki-flood.ldt");
const SPOTLIGHT_LDT: &str = include_str!("../../eulumdat-wasm/templates/wiki-spotlight.ldt");
const UPLIGHT_LDT: &str = include_str!("../../eulumdat-wasm/templates/floor_uplight.ldt");

// ---------------------------------------------------------------------------
// UI colors (bevy-ui feature only)
// ---------------------------------------------------------------------------

#[cfg(feature = "bevy-ui")]
mod ui_colors {
    use bevy::prelude::Color;
    // Translucent dashboard backgrounds for the glass-panel feel over
    // the 3D scene. Chrome / Firefox / native compositors render this
    // correctly. Safari WebGPU users see the overlay flicker — that is
    // unrelated to alpha blending (we tested opaque, it still flickers)
    // and lives in the Bloom + HDR + UI render path.
    // See `docs/safari_bloom_flicker.md` for the full diagnostic.
    pub const PANEL_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.85);
    pub const SECTION_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.6);
    pub const SLIDER_TRACK_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
    pub const SLIDER_THUMB_COLOR: Color = Color::srgb(0.35, 0.75, 0.35);
    pub const LABEL_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
    pub const VALUE_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
    pub const BTN_NORMAL: Color = Color::srgb(0.12, 0.12, 0.15);
    pub const BTN_ACTIVE: Color = Color::srgb(0.25, 0.55, 0.35);
}
#[cfg(feature = "bevy-ui")]
use ui_colors::*;

// ---------------------------------------------------------------------------
// Resources & Components
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SceneChoice {
    Sponza,
    UrbanStreet,
    BistroExterior,
    /// Burg Vischering, Lüdinghausen: Geobasis NRW 3D-Mesh (see VISCHERING_*).
    Vischering,
}

impl SceneChoice {
    #[cfg(feature = "bevy-ui")]
    fn label(&self) -> &str {
        match self {
            SceneChoice::Sponza => "Sponza Atrium",
            SceneChoice::UrbanStreet => "Urban Street",
            SceneChoice::BistroExterior => "Bistro Exterior",
            SceneChoice::Vischering => "Burg Vischering",
        }
    }

    fn cam_start(&self) -> Vec3 {
        match self {
            SceneChoice::Sponza => Vec3::new(-3.7, 0.0, -0.5),
            SceneChoice::UrbanStreet => Vec3::new(8.0, 1.7, 45.0),
            SceneChoice::BistroExterior => Vec3::new(-10.5, 1.7, -1.0),
            // South-east bank, a little above the trees: the main castle, the
            // moat and the outer bailey in one view.
            SceneChoice::Vischering => Vec3::new(58.3, 2.3, -8.6),
        }
    }

    fn cam_look_at(&self) -> Vec3 {
        match self {
            SceneChoice::Sponza => Vec3::new(10.0, 4.0, -0.5),
            SceneChoice::UrbanStreet => Vec3::new(0.0, 4.0, 20.0),
            SceneChoice::BistroExterior => Vec3::new(0.0, 3.5, 0.0),
            SceneChoice::Vischering => Vec3::new(22.2, 11.4, -0.7),
        }
    }
}

/// Per-lamp power relative to the Intensity slider (Burg Vischering's floods
/// are sized to their throw); lamps without it follow the slider 1:1.
#[derive(Component, Clone, Copy)]
struct LampScale(f32);

/// The key "moon" light. It is a stylised daylight-level light (15 klx) that
/// the street scenes hide in their shadows; the open castle park would be
/// floodlit by it, so Burg Vischering gets a real night.
#[derive(Component)]
struct MoonLight;

fn moon_illuminance(scene: SceneChoice) -> f32 {
    if scene == SceneChoice::Vischering {
        30.0
    } else {
        15000.0
    }
}

/// Camera exposure per scene: the castle runs on night levels (moon ~30,
/// floods 150-600 in the demo's units), the streets on the default.
fn scene_exposure(scene: SceneChoice) -> bevy::camera::Exposure {
    if scene == SceneChoice::Vischering {
        bevy::camera::Exposure { ev100: 6.0 }
    } else {
        bevy::camera::Exposure::default()
    }
}

/// Haze per scene: the slider's density is tuned for a 30 m street; across
/// the 300 m castle park the same value is an opaque wall.
fn scene_haze_factor(scene: SceneChoice) -> f32 {
    if scene == SceneChoice::Vischering {
        0.12
    } else {
        1.0
    }
}

fn update_moon(
    mut commands: Commands,
    state: Res<SimulationState>,
    mut moon: Query<&mut DirectionalLight, With<MoonLight>>,
    cams: Query<Entity, With<FlyCamera>>,
) {
    if state.is_changed() {
        for mut m in moon.iter_mut() {
            m.illuminance = moon_illuminance(state.scene);
        }
        for cam in cams.iter() {
            commands.entity(cam).insert(scene_exposure(state.scene));
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    StandardPollution,
    PreservedDarkness,
}

#[derive(Resource)]
struct SimulationState {
    mode: Mode,
    scene: SceneChoice,
    haze_density: f32,
    intensity_scale: f32,
    ambient_brightness: f32,
    uplight_pct: f32,
    show_solid: bool,
    lights_dirty: bool,
    pollution_ldt: Eulumdat,
    preserved_ldt: Eulumdat,
    flood_ldt: Eulumdat,
    spotlight_ldt: Eulumdat,
    uplight_ldt: Eulumdat,
}

impl SimulationState {
    fn active_ldt(&self) -> &Eulumdat {
        match self.mode {
            Mode::StandardPollution => &self.pollution_ldt,
            Mode::PreservedDarkness => &self.preserved_ldt,
        }
    }
}

#[derive(Component)]
struct StreetLamp;

#[derive(Component)]
struct SkyObject(Vec3);

#[derive(Component)]
struct Star(f32);

#[derive(Component)]
struct Moon;

#[derive(Component)]
struct Planet(f32);

#[derive(Resource, Default)]
struct MaterialFixState(Option<SceneChoice>);

#[derive(Resource, Default)]
struct PreviousScene(Option<SceneChoice>);

#[derive(Resource, Default)]
struct SceneLoadState {
    handles: Vec<UntypedHandle>,
    loaded: bool,
    start_frame: u32,
}

#[derive(Resource, Default)]
struct FrameCounter(u32);

#[derive(Component)]
struct FlyCamera {
    speed: f32,
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl FlyCamera {
    fn from_look_direction(from: Vec3, to: Vec3) -> Self {
        let dir = (to - from).normalize();
        let yaw = dir.z.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
        let pitch = (-dir.y).asin();
        Self {
            speed: 5.0,
            sensitivity: 0.003,
            pitch,
            yaw,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (u32::MAX as f32)
    }

    #[allow(dead_code)]
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

// ---------------------------------------------------------------------------
// UI entity references (bevy-ui feature only)
// ---------------------------------------------------------------------------

#[cfg(feature = "bevy-ui")]
#[derive(Resource)]
#[allow(dead_code)]
struct UiEntities {
    mode_text: Entity,
    hints_text: Entity,
    scene_buttons: [Entity; 4],
    ldt_name: Entity,
    ldt_manufacturer: Entity,
    ldt_flux: Entity,
    ldt_cct: Entity,
    ldt_lor: Entity,
    uplight_slider: Entity,
    grade_text: Entity,
    grade_bar: Entity,
    intensity_slider: Entity,
    ambient_slider: Entity,
    haze_slider: Entity,
    loading_overlay: Entity,
    loading_text: Entity,
}

#[cfg(feature = "bevy-ui")]
#[derive(Component)]
struct DashboardPanel;

#[cfg(feature = "bevy-ui")]
#[derive(Component)]
struct SceneButton(SceneChoice);

#[cfg(feature = "bevy-ui")]
#[derive(Component)]
struct DashboardSliderThumb;

#[cfg(feature = "bevy-ui")]
#[derive(Component, Clone, Copy)]
enum SliderBinding {
    Uplight,
    Intensity,
    Ambient,
    Haze,
}

// ---------------------------------------------------------------------------
// Debug flags (CLI: --no-sky, --no-matfix, --no-lights)
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct DebugFlags {
    no_sky: bool,
    no_matfix: bool,
    no_lights: bool,
    /// Native automation: `--shot=<file.png>` saves one frame once the scene
    /// has loaded and exits; `--cam=x,y,z,lx,ly,lz` pins the camera;
    /// `--preserved` starts in the preserved-darkness mode.
    shot: Option<String>,
    cam: Option<(Vec3, Vec3)>,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let (debug_flags, scene) = {
        let args: Vec<String> = std::env::args().collect();
        let value = |key: &str| {
            args.iter()
                .find_map(|a| a.strip_prefix(key).map(str::to_string))
        };
        let flags = DebugFlags {
            no_sky: args.iter().any(|a| a == "--no-sky"),
            no_matfix: args.iter().any(|a| a == "--no-matfix"),
            no_lights: args.iter().any(|a| a == "--no-lights"),
            shot: value("--shot="),
            cam: value("--cam=").and_then(|v| {
                let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
                (n.len() == 6).then(|| (Vec3::new(n[0], n[1], n[2]), Vec3::new(n[3], n[4], n[5])))
            }),
        };
        let scene = args.iter().skip(1).find(|a| !a.starts_with("--")).cloned();
        if flags.no_sky {
            println!("DEBUG: --no-sky — sky objects disabled");
        }
        if flags.no_matfix {
            println!("DEBUG: --no-matfix — material fixes disabled");
        }
        if flags.no_lights {
            println!("DEBUG: --no-lights — photometric lights disabled");
        }
        (flags, scene)
    };
    #[cfg(target_arch = "wasm32")]
    let (debug_flags, scene) = {
        // Read scene from URL: ?wasm=skyglow_demo&scene=sponza (or bistro)
        #[cfg(feature = "wasm-bindgen")]
        let scene: Option<String> =
            js_sys::eval("new URLSearchParams(window.location.search).get('scene')")
                .ok()
                .and_then(|v| v.as_string());
        #[cfg(not(feature = "wasm-bindgen"))]
        let scene: Option<String> = None;
        (
            DebugFlags {
                no_sky: false,
                no_matfix: false,
                no_lights: false,
                shot: None,
                cam: None,
            },
            scene,
        )
    };

    #[cfg(not(target_arch = "wasm32"))]
    let start_preserved = std::env::args().any(|a| a == "--preserved");
    #[cfg(target_arch = "wasm32")]
    let start_preserved = false;
    #[cfg(not(target_arch = "wasm32"))]
    let start_drawbridge_up = std::env::args().any(|a| a == "--drawbridge-up");
    #[cfg(target_arch = "wasm32")]
    let start_drawbridge_up = false;
    let pollution_ldt = Eulumdat::parse(POLLUTION_LDT).expect("Failed to parse road_luminaire.ldt");
    let preserved_ldt = Eulumdat::parse(PRESERVED_LDT).expect("Failed to parse projector.ldt");
    let flood_ldt = Eulumdat::parse(FLOOD_LDT).expect("Failed to parse wiki-flood.ldt");
    let spotlight_ldt = Eulumdat::parse(SPOTLIGHT_LDT).expect("Failed to parse wiki-spotlight.ldt");
    let uplight_ldt = Eulumdat::parse(UPLIGHT_LDT).expect("Failed to parse floor_uplight.ldt");

    #[allow(unused_mut)]
    let mut window = Window {
        title: "Skyglow Demo — Darkness Preservation Simulator".into(),
        resolution: (1280u32, 720u32).into(),
        present_mode: bevy::window::PresentMode::Fifo,
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#skyglow-canvas".into());
        window.fit_canvas_to_parent = true;
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }))
    .add_plugins(PhotometricPlugin::<Eulumdat>::default())
    .insert_resource(SimulationState {
        mode: if start_preserved {
            Mode::PreservedDarkness
        } else {
            Mode::StandardPollution
        },
        scene: match scene.as_deref() {
            Some("sponza") => SceneChoice::Sponza,
            Some("bistro") => SceneChoice::BistroExterior,
            Some("vischering") => SceneChoice::Vischering,
            _ => SceneChoice::UrbanStreet,
        },
        // Same values as the Space toggle sets for each mode.
        haze_density: if start_preserved { 0.005 } else { 0.04 },
        intensity_scale: if start_preserved { 0.5 } else { 2.0 },
        ambient_brightness: if start_preserved { 2.0 } else { 8.0 },
        uplight_pct: if start_preserved { 3.0 } else { 45.0 },
        show_solid: false,
        lights_dirty: false,
        pollution_ldt,
        preserved_ldt,
        flood_ldt,
        spotlight_ldt,
        uplight_ldt,
    })
    .insert_resource(debug_flags)
    .init_resource::<MaterialFixState>()
    .init_resource::<PreviousScene>()
    .init_resource::<SceneLoadState>()
    .init_resource::<PendingCameraReset>()
    .insert_resource(Drawbridge {
        up: start_drawbridge_up,
        angle: if start_drawbridge_up {
            VISCHERING_DRAWBRIDGE_MAX_DEG
        } else {
            0.0
        },
    })
    .insert_resource(FrameCounter(0))
    .add_systems(Startup, (setup_scene, setup_stars, setup_lights))
    .add_systems(Update, fix_scene_materials)
    .add_systems(
        Update,
        (
            toggle_mode,
            toggle_solid,
            keyboard_shortcuts,
            update_fog,
            update_ambient,
            update_star_visibility,
            update_moon,
            animate_drawbridge,
            sync_lights,
            check_loading_progress,
            count_frames,
            #[cfg(not(target_arch = "wasm32"))]
            auto_shot,
        ),
    )
    .add_systems(
        Update,
        (
            fly_camera_look,
            fly_camera_move,
            fly_camera_zoom,
            fly_camera_reset,
        )
            .chain(),
    )
    .add_systems(PostUpdate, track_sky_to_camera);

    #[cfg(feature = "bevy-ui")]
    {
        // Bevy 0.19 pulls the widget/focus plugins into DefaultPlugins on some
        // feature combinations — add only what's missing (as greenhouse_demo).
        if !app.is_plugin_added::<bevy::ui_widgets::popover::PopoverPlugin>() {
            app.add_plugins(UiWidgetsPlugins);
        }
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        app.add_systems(Startup, setup_ui).add_systems(
            Update,
            (
                update_ui_from_state,
                update_scene_button_visuals,
                update_slider_visuals,
                update_loading_overlay,
            ),
        );
    }

    app.run();
}

// ---------------------------------------------------------------------------
// Scene setup
// ---------------------------------------------------------------------------

#[derive(Component)]
struct SceneGeometry;

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    state: Res<SimulationState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_state: ResMut<SceneLoadState>,
    mut cluster_settings: ResMut<bevy::light::cluster::GlobalClusterSettings>,
    flags: Res<DebugFlags>,
) {
    // Pre-allocate GPU cluster buffers for the Bistro scene's ~40 lights
    // to avoid resize flickering in the first few frames.
    if let Some(ref mut gpu) = cluster_settings.gpu_clustering {
        gpu.initial_z_slice_list_capacity = 4096;
        gpu.initial_index_list_capacity = 524288;
    }

    commands.insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)));

    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.7, 0.7, 0.9),
        brightness: state.ambient_brightness,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        MoonLight,
        DirectionalLight {
            color: Color::srgb(0.7, 0.75, 0.95),
            illuminance: moon_illuminance(state.scene),
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.1,
            shadow_normal_bias: 0.2,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::PI * 0.35,
            -std::f32::consts::PI * 0.13,
            0.0,
        )),
    ));

    let (cam_start, cam_look) = flags
        .cam
        .unwrap_or((state.scene.cam_start(), state.scene.cam_look_at()));
    let cam_transform = Transform::from_translation(cam_start).looking_at(cam_look, Vec3::Y);

    // Camera spawn is split between WebGPU and WebGL2 paths because:
    // - Bloom needs `rgba16float` storage textures (WebGPU only)
    // - EnvironmentMapLight reads KTX2 zstd cubemaps via compute decoder
    //   (WebGPU only)
    // - The dense ClusterConfig::FixedZ { total: 4096 } overflows
    //   WebGL2's smaller storage-buffer limits
    //
    // The WebGL2 build trades these for a simpler camera + ambient ramp
    // and shows a "reduced fidelity" banner via `setup_fallback_banner`.
    let mut cam = commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::AgX,
        cam_transform,
        DistanceFog {
            color: Color::srgb(0.05, 0.04, 0.06),
            falloff: FogFalloff::Exponential {
                density: state.haze_density * scene_haze_factor(state.scene),
            },
            ..default()
        },
        FlyCamera::from_look_direction(cam_start, cam_look),
        NoIndirectDrawing,
    ));

    #[cfg(not(feature = "webgl2"))]
    {
        cam.insert((
            Bloom {
                intensity: 0.08,
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 600.0,
                ..default()
            },
            // Finer cluster grid for dense photometric lighting scenes
            bevy::light::cluster::ClusterConfig::FixedZ {
                total: 4096,
                z_slices: 24,
                z_config: default(),
                dynamic_resizing: true,
            },
        ));
    }
    #[cfg(feature = "webgl2")]
    {
        // WebGL2 fallback: attach an ambient term to the camera so the
        // scene isn't near-black where direct lights miss. Bevy 0.19's
        // `AmbientLight` is a per-camera component, not a global
        // resource — `cam.insert(...)` is the right place.
        let _ = &asset_server;
        cam.insert(AmbientLight {
            color: Color::srgb(0.55, 0.6, 0.75),
            brightness: 220.0,
            ..default()
        });
    }

    spawn_scene_geometry(
        &mut commands,
        &asset_server,
        &state,
        &mut meshes,
        &mut materials,
        &mut load_state,
        0,
    );
}

fn spawn_scene_geometry(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    state: &SimulationState,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    load_state: &mut SceneLoadState,
    frame: u32,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let (sponza_glb, bistro_glb) = ("Sponza_gpu.glb#Scene0", "BistroExterior_gpu.glb#Scene0");
    #[cfg(target_arch = "wasm32")]
    let (sponza_glb, bistro_glb) = ("Sponza_web.glb#Scene0", "BistroExterior_web.glb#Scene0");

    load_state.handles.clear();
    load_state.loaded = false;
    load_state.start_frame = frame;

    match state.scene {
        SceneChoice::Sponza => {
            let h = asset_server.load(sponza_glb);
            load_state.handles.push(h.clone().untyped());
            commands.spawn((WorldAssetRoot(h), SceneGeometry));
        }
        SceneChoice::BistroExterior => {
            let h = asset_server.load(bistro_glb);
            load_state.handles.push(h.clone().untyped());
            commands.spawn((WorldAssetRoot(h), SceneGeometry));

            let gi = asset_server.load("BistroExteriorFakeGI.gltf#Scene0");
            load_state.handles.push(gi.clone().untyped());
            commands.spawn((WorldAssetRoot(gi), SceneGeometry));
        }
        SceneChoice::UrbanStreet => {
            load_state.loaded = true;
            build_urban_street(commands, meshes, materials);
        }
        SceneChoice::Vischering => {
            // Same file natively and on the web: JPEG textures, ~50 MB.
            let h = asset_server.load("Vischering.glb#Scene0");
            load_state.handles.push(h.clone().untyped());
            commands.spawn((WorldAssetRoot(h), SceneGeometry));
            // The GLB carries the water (Vischering_Water: each body at its
            // LiDAR level, the moat above the meadows). Below all of it a dark,
            // glossy catch-all plane fills photogrammetry holes and the land
            // beyond the mesh.
            commands.spawn((
                Mesh3d(meshes.add(Plane3d::default().mesh().size(1200.0, 1200.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    // A moat, not a lawn: dark green-black and glossy, so the
                    // floods and lanterns glint in it.
                    base_color: Color::srgb(0.006, 0.012, 0.012),
                    perceptual_roughness: 0.06,
                    reflectance: 0.5,
                    ..default()
                })),
                Transform::from_xyz(0.0, VISCHERING_WATER_Y, 0.0),
                SceneGeometry,
            ));
        }
    }
}

fn fix_scene_materials(
    flags: Res<DebugFlags>,
    #[cfg_attr(feature = "webgl2", allow(unused_mut, unused_variables))] mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimulationState>,
    mut fix_state: ResMut<MaterialFixState>,
    mesh_entities: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
) {
    if flags.no_matfix {
        fix_state.0 = Some(state.scene);
        return;
    }
    if fix_state.0 == Some(state.scene) {
        return;
    }
    let threshold = match state.scene {
        SceneChoice::Sponza => 20,
        SceneChoice::BistroExterior => 50,
        // The generated glTF already carries the right materials.
        SceneChoice::UrbanStreet | SceneChoice::Vischering => {
            fix_state.0 = Some(state.scene);
            return;
        }
    };
    if materials.len() < threshold {
        return;
    }

    match state.scene {
        SceneChoice::Sponza => {
            let mut fixed = 0u32;
            for (_id, mat) in materials.iter_mut() {
                mat.flip_normal_map_y = true;
                mat.metallic = 0.0;
                mat.perceptual_roughness = 0.5;
                fixed += 1;
            }
            println!("SPONZA FIX: patched {} materials", fixed);
        }
        SceneChoice::BistroExterior => {
            let mut transmitted_entities = Vec::new();
            for (entity, mat_handle) in mesh_entities.iter() {
                if let Some(mat) = materials.get(&mat_handle.0) {
                    if matches!(mat.alpha_mode, AlphaMode::Mask(_)) {
                        transmitted_entities.push(entity);
                    }
                }
            }
            let mut fixed = 0u32;
            for (_id, mat) in materials.iter_mut() {
                mat.flip_normal_map_y = true;
                match mat.alpha_mode {
                    AlphaMode::Mask(_) => {
                        mat.diffuse_transmission = 0.6;
                        mat.double_sided = true;
                        mat.cull_mode = None;
                        mat.thickness = 0.2;
                    }
                    AlphaMode::Opaque => {
                        mat.double_sided = false;
                        mat.cull_mode = Some(Face::Back);
                    }
                    _ => {}
                }
                fixed += 1;
            }
            // TransmittedShadowReceiver needs storage textures for the
            // transmitted-shadow ray probe — not available in WebGL2.
            #[cfg(not(feature = "webgl2"))]
            for entity in transmitted_entities {
                commands.entity(entity).insert(TransmittedShadowReceiver);
            }
            #[cfg(feature = "webgl2")]
            let _ = transmitted_entities;
            println!("BISTRO FIX: patched {} materials", fixed);
        }
        SceneChoice::UrbanStreet | SceneChoice::Vischering => unreachable!(),
    }
    fix_state.0 = Some(state.scene);
}

fn build_urban_street(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let road_length = 120.0_f32;
    let road_width = 7.0_f32;
    let sidewalk_width = 2.5_f32;

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.04, 0.06, 0.03),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.01, road_length / 2.0),
        SceneGeometry,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(road_width, road_length))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.08),
            metallic: 0.15,
            perceptual_roughness: 0.55,
            reflectance: 0.3,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, road_length / 2.0),
        SceneGeometry,
    ));

    let sidewalk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.35),
        perceptual_roughness: 0.75,
        ..default()
    });
    for sign in [-1.0_f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(sidewalk_width, 0.12, road_length))),
            MeshMaterial3d(sidewalk_mat.clone()),
            Transform::from_xyz(
                sign * (road_width / 2.0 + sidewalk_width / 2.0),
                0.06,
                road_length / 2.0,
            ),
            SceneGeometry,
        ));
    }

    let building_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.12),
        perceptual_roughness: 0.85,
        ..default()
    });
    let buildings = [
        (15.0_f32, 15.0, 10.0, 14.0, 8.0),
        (14.0, 40.0, 8.0, 10.0, 10.0),
        (16.0, 70.0, 12.0, 18.0, 7.0),
        (13.0, 95.0, 7.0, 8.0, 9.0),
        (-14.0, 25.0, 9.0, 12.0, 8.0),
        (-15.0, 55.0, 11.0, 16.0, 9.0),
        (-13.0, 85.0, 8.0, 10.0, 7.0),
    ];
    let window_warm = LinearRgba::new(1.8, 1.3, 0.5, 1.0);
    let window_cool = LinearRgba::new(0.8, 0.9, 1.6, 1.0);
    let mut rng = Rng::new(12345);

    for &(cx, cz, bw, bh, bd) in &buildings {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(bw, bh, bd))),
            MeshMaterial3d(building_mat.clone()),
            Transform::from_xyz(cx, bh / 2.0, cz),
            SceneGeometry,
        ));
        let face_x = if cx > 0.0 {
            cx - bw / 2.0 - 0.02
        } else {
            cx + bw / 2.0 + 0.02
        };
        for floor in 0..((bh / 3.2) as i32) {
            for col in 0..((bd / 2.5) as i32) {
                if rng.next_f32() < 0.3 {
                    continue;
                }
                let emissive = if rng.next_f32() > 0.3 {
                    window_warm
                } else {
                    window_cool
                };
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.02, 1.4, 1.0))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.05, 0.05, 0.08),
                        emissive,
                        ..default()
                    })),
                    Transform::from_xyz(
                        face_x,
                        2.0 + floor as f32 * 3.2,
                        cz - bd / 2.0 + 1.5 + col as f32 * 2.5,
                    ),
                    SceneGeometry,
                ));
            }
        }
    }

    let tree_canopy = Color::srgb(0.04, 0.12, 0.04);
    let trunk_color = Color::srgb(0.18, 0.12, 0.05);
    for &(tx, tz, r, th) in &[
        (-7.0_f32, 10.0, 2.0, 4.5),
        (-8.5, 20.0, 1.6, 3.8),
        (-6.0, 35.0, 2.2, 5.0),
        (-9.0, 48.0, 1.8, 4.2),
        (-7.5, 62.0, 2.0, 4.8),
        (8.0, 30.0, 1.4, 3.5),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.12, th))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: trunk_color,
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_xyz(tx, th / 2.0, tz),
            SceneGeometry,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(r))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: tree_canopy,
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(tx, th + r * 0.6, tz),
            SceneGeometry,
        ));
    }
}

fn lamp_light_positions(scene: SceneChoice) -> Vec<(Vec3, Quat)> {
    match scene {
        SceneChoice::Sponza => lamp_positions_sponza(),
        SceneChoice::UrbanStreet => lamp_positions_urban(),
        SceneChoice::BistroExterior | SceneChoice::Vischering => Vec::new(),
    }
}

fn lamp_positions_sponza() -> Vec<(Vec3, Quat)> {
    let mut positions = Vec::new();
    let lamp_y = 5.0;
    let corridor_z = 0.0;

    for i in 0..8 {
        let x = -8.0 + i as f32 * 2.5;
        positions.push((Vec3::new(x, lamp_y, corridor_z), Quat::IDENTITY));
    }
    let gallery_y = 8.0;
    for i in 0..4 {
        let x = -6.0 + i as f32 * 4.0;
        positions.push((Vec3::new(x, gallery_y, 3.5), Quat::IDENTITY));
        positions.push((Vec3::new(x, gallery_y, -3.5), Quat::IDENTITY));
    }
    positions
}

fn lamp_positions_urban() -> Vec<(Vec3, Quat)> {
    let h = 8.0;
    let road_width = 7.0_f32;
    let sidewalk_width = 2.5_f32;
    let right_x = road_width / 2.0 + sidewalk_width + 0.3;
    let left_x = -right_x;
    let tilt = 15.0_f32.to_radians();
    let pole_spacing = 25.0;
    let road_length = 120.0;
    let arm_length = 1.5;

    let num_poles = ((road_length / pole_spacing) as i32).max(1);
    let mut positions = Vec::new();
    for i in 0..num_poles {
        let z = pole_spacing / 2.0 + i as f32 * pole_spacing;
        let (base_x, arm_sign) = if i % 2 == 0 {
            (right_x, -1.0_f32)
        } else {
            (left_x, 1.0_f32)
        };
        let light_x = base_x + arm_sign * arm_length;
        positions.push((
            Vec3::new(light_x, h - 0.3, z),
            Quat::from_rotation_z(-arm_sign * tilt),
        ));
    }
    positions
}

#[derive(Clone, Copy)]
enum BistroLuminaire {
    StreetLight,
    WallLight,
    Spotlight,
    Lantern,
}

struct BistroPlacement {
    pos: Vec3,
    rotation: Quat,
    luminaire: BistroLuminaire,
}

fn lamp_placements_bistro() -> Vec<BistroPlacement> {
    let mut placements = Vec::new();

    for &(x, y, z) in &[
        (-6.93, 6.92, -6.70),
        (5.98, 6.93, -34.01),
        (78.92, 6.93, 54.97),
        (56.16, 6.93, 29.31),
        (-33.31, 6.93, -29.18),
        (-15.44, 6.93, 3.34),
        (-3.34, 6.93, 7.44),
        (-2.81, 6.93, 16.06),
        (12.50, 6.93, 13.85),
        (34.53, 6.93, 30.11),
        (53.29, 6.93, 38.56),
        (62.08, 6.93, 54.31),
    ] {
        placements.push(BistroPlacement {
            pos: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
            luminaire: BistroLuminaire::StreetLight,
        });
    }

    for &(x, y, z) in &[
        (-21.44, 6.33, 1.94),
        (-27.61, 6.33, -6.06),
        (28.84, 7.04, -66.51),
        (28.25, 7.04, -55.05),
        (14.37, 7.15, -32.13),
        (48.01, 6.35, 21.60),
        (39.14, 7.24, 37.07),
        (33.22, 6.35, 20.41),
        (-32.27, 6.33, -11.50),
        (-39.34, 6.31, -18.89),
        (-45.62, 5.14, -27.51),
        (-43.03, 5.14, -33.70),
        (-28.37, 6.33, -23.91),
        (-19.10, 6.33, -15.39),
        (-13.49, 6.37, -18.26),
        (-3.39, 6.63, -29.25),
    ] {
        placements.push(BistroPlacement {
            pos: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
            luminaire: BistroLuminaire::WallLight,
        });
    }

    for &(x, y, z) in &[
        (-26.19, 11.98, -22.00),
        (15.72, 5.55, -33.45),
        (-20.12, 13.68, 3.93),
        (-26.09, 13.68, -3.01),
        (-29.69, 13.67, -9.11),
    ] {
        placements.push(BistroPlacement {
            pos: Vec3::new(x, y, z),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            luminaire: BistroLuminaire::Spotlight,
        });
    }

    for &(x, y, z) in &[
        (7.33, 4.13, 5.27),
        (2.91, 4.14, 3.52),
        (-1.79, 4.13, -7.54),
        (-0.07, 4.08, -11.55),
        (11.71, 4.09, 6.69),
    ] {
        placements.push(BistroPlacement {
            pos: Vec3::new(x, y, z),
            rotation: Quat::IDENTITY,
            luminaire: BistroLuminaire::Lantern,
        });
    }

    placements
}

// ---------------------------------------------------------------------------
// Burg Vischering (Lüdinghausen)
// ---------------------------------------------------------------------------
//
// Mesh: Geobasis NRW 3D-Mesh 2024 (dl-de/zero-2.0), cut to 320 x 320 m around
// the castle and written as assets/Vischering.glb by
// gldf-unreal-demo/Scripts/geodata/nrw_vischering.py, which also derives the
// sites below from the NRW LiDAR (ground, water) and LoD2 (walls, bridges).
// glTF frame: Y up, metres, x = east, z = south, origin = castle at ground level.

/// Façade lighting is specified as illuminance on the wall: each flood gets
/// the power that puts this much on its wall (CIE 94: ~30-100 lx for façades
/// in dark/medium surroundings; the polluting setup overdoes it and aims past
/// the walls). In this demo's light units, see `flood_scale`.
const VISCHERING_WALL_LUX_PRESERVED: f32 = 150.0;
const VISCHERING_WALL_LUX_POLLUTING: f32 = 600.0;
/// Drawbridge (glTF node "Vischering_Drawbridge", origin on the hinge):
/// rotation axis in the scene frame; positive angles lift the free end.
const VISCHERING_DRAWBRIDGE_AXIS: [f32; 3] = [0.3591, 0.0000, -0.9333];
const VISCHERING_DRAWBRIDGE_MAX_DEG: f32 = 75.0;

/// Drawbridge state: target up/down and the current angle (degrees).
#[derive(Resource, Default)]
struct Drawbridge {
    up: bool,
    angle: f32,
}

/// Z raises / lowers the drawbridge; it swings at 20°/s about its hinge.
fn animate_drawbridge(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut bridge: ResMut<Drawbridge>,
    mut nodes: Query<(&Name, &mut Transform)>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        bridge.up = !bridge.up;
    }
    let target = if bridge.up {
        VISCHERING_DRAWBRIDGE_MAX_DEG
    } else {
        0.0
    };
    let step = 20.0 * time.delta_secs();
    let before = bridge.angle;
    bridge.angle += (target - bridge.angle).clamp(-step, step);
    if (bridge.angle - before).abs() < 1e-4 && !bridge.is_changed() {
        return;
    }
    let axis = Vec3::from(VISCHERING_DRAWBRIDGE_AXIS).normalize_or(Vec3::Z);
    for (name, mut t) in nodes.iter_mut() {
        if name.as_str() == "Vischering_Drawbridge" {
            t.rotation = Quat::from_axis_angle(axis, bridge.angle.to_radians());
        }
    }
}

/// Catch-all water plane, under the lowest water body (written by nrw_vischering.py).
const VISCHERING_WATER_Y: f32 = -2.14;

/// Façade floodlights on open ground with a clear view of their wall (lawns
/// and paths across the moat, courtyards): (position, aim point on the wall).
/// In the photogrammetric mesh trees are solid, so sites under a canopy are
/// left out by the generator.
const VISCHERING_FLOODS: &[([f32; 3], [f32; 3])] = &[
    ([-31.31, 0.90, -42.14], [-35.15, 3.72, -43.28]),
    ([21.78, 0.23, 37.30], [17.73, 4.91, 15.67]),
    ([16.91, 1.75, 0.69], [16.76, 4.91, 4.68]),
    ([54.21, 0.42, 12.44], [27.38, 4.91, 9.43]),
    ([-6.99, 0.23, 32.33], [2.88, 4.91, 19.74]),
    ([-51.93, 1.16, -64.40], [-36.49, 3.75, -60.22]),
    ([-24.77, 1.28, -26.94], [3.77, 4.91, -17.70]),
    ([-29.60, 1.04, -19.20], [0.11, 4.91, 15.92]),
    ([-111.59, 0.35, -4.68], [-111.00, 3.63, -8.63]),
    ([-31.61, 1.01, -12.35], [-5.12, 4.91, -3.27]),
    ([-30.81, 0.94, -49.13], [-29.01, 3.71, -52.70]),
    ([-24.21, 1.00, -45.20], [-20.28, 3.67, -48.29]),
    ([-120.10, 1.33, -12.54], [-116.14, 3.63, -11.94]),
    ([-43.59, 1.51, -15.27], [-43.86, 2.08, -11.28]),
    ([9.82, 1.85, -2.13], [9.21, 9.76, 1.83]),
    ([21.00, 1.67, 1.00], [9.04, 15.81, 3.45]),
];

/// Lanterns at the bridge ends (ground position; the head sits 4 m up).
const VISCHERING_LANTERNS: &[[f32; 3]] = &[
    [13.91, 1.91, -9.66],
    [-24.64, 1.08, -24.48],
    [-41.58, 1.33, -20.40],
];

/// Castle floodlighting, the skyglow way: in the polluting mode wide floods
/// are aimed past the walls (light spills over the roofs into the sky); in
/// the preserved mode narrow spots stop at 45 % of the wall height.
fn spawn_vischering_lamps(commands: &mut Commands, state: &SimulationState) {
    use eulumdat_bevy::photometric::PhotometricData as _;
    let polluting = state.mode == Mode::StandardPollution;
    let flood = if polluting {
        state.flood_ldt.clone()
    } else {
        state.spotlight_ldt.clone()
    };
    // eulumdat-bevy's main beam: flux · LOR · 50 · downward · 0.3 · scale, and
    // Bevy spreads a spot's power like a point light (I = P / 4π), so the
    // scale for E lux at d metres is E · d² · 4π / (flux · LOR · 15 · down).
    let (wall_lux, mode_default) = if polluting {
        (VISCHERING_WALL_LUX_POLLUTING, 2.0)
    } else {
        (VISCHERING_WALL_LUX_PRESERVED, 0.5)
    };
    let beam =
        (flood.total_flux() * flood.light_output_ratio() * flood.downward_fraction()) as f32 * 15.0;
    let per_slider = 1.0 / mode_default; // slider at its mode default → exactly wall_lux
    for &(p, a) in VISCHERING_FLOODS {
        let pos = Vec3::from(p) + Vec3::Y * 0.5; // housing on the mesh surface
        let aim = Vec3::from(a) + if polluting { Vec3::Y * 6.0 } else { Vec3::ZERO };
        let dir = (aim - pos).normalize_or(Vec3::NEG_Y);
        let mult = wall_lux * (aim - pos).length_squared() * 4.0 * std::f32::consts::PI
            / beam.max(1e-3)
            * per_slider;
        commands.spawn((
            LampScale(mult),
            EulumdatLightBundle::new(flood.clone())
                .with_transform(
                    Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Y, dir)),
                )
                .with_intensity_scale(state.intensity_scale * mult)
                // On the ground: the height-based default reach would be ~1 m.
                .with_range((aim - pos).length() * 2.0 + 10.0)
                .with_model(false)
                .with_solid(state.show_solid),
            StreetLamp,
        ));
    }
    for &p in VISCHERING_LANTERNS {
        commands.spawn((
            EulumdatLightBundle::new(state.active_ldt().clone())
                .with_transform(Transform::from_translation(Vec3::from(p) + Vec3::Y * 4.0))
                .with_intensity_scale(state.intensity_scale)
                .with_model(true)
                .with_solid(state.show_solid),
            StreetLamp,
        ));
    }
}

// ---------------------------------------------------------------------------
// Stars
// ---------------------------------------------------------------------------

const BRIGHT_STARS_JSON: &str = include_str!("../data/bright_stars.json");

fn altaz_to_sky_position(alt_deg: f32, az_deg: f32, radius: f32) -> Vec3 {
    let alt = alt_deg.to_radians();
    let az = az_deg.to_radians();
    let cos_alt = alt.cos();
    Vec3::new(
        radius * cos_alt * az.sin(),
        radius * alt.sin(),
        radius * cos_alt * az.cos(),
    )
}

fn star_color_from_temp(temp: f32) -> (f32, f32, f32) {
    if temp > 10000.0 {
        (0.7, 0.75, 1.0)
    } else if temp > 7500.0 {
        (0.8, 0.88, 1.0)
    } else if temp > 6000.0 {
        (1.0, 0.96, 0.9)
    } else if temp > 5000.0 {
        (1.0, 0.85, 0.4)
    } else if temp > 4000.0 {
        (1.0, 0.65, 0.2)
    } else {
        (1.0, 0.4, 0.2)
    }
}

#[derive(serde::Deserialize)]
struct CatalogStar {
    #[allow(dead_code)]
    name: String,
    alt: f64,
    az: f64,
    mag: f64,
    temp: f64,
}

#[derive(serde::Deserialize)]
struct StarCatalog {
    #[allow(dead_code)]
    location: serde_json::Value,
    #[allow(dead_code)]
    time: String,
    stars: Vec<CatalogStar>,
}

fn setup_stars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    flags: Res<DebugFlags>,
) {
    if flags.no_sky {
        return;
    }
    let star_mesh = meshes.add(Sphere::new(0.25));
    let radius = 90.0_f32;

    let catalog: StarCatalog =
        serde_json::from_str(BRIGHT_STARS_JSON).expect("Failed to parse star catalog");

    for star in &catalog.stars {
        let alt = star.alt as f32;
        if alt < 10.0 {
            continue;
        }

        let mag = star.mag as f32;
        let pos = altaz_to_sky_position(alt, star.az as f32, radius);

        let brightness = 50.0 + (4.5 - mag).max(0.0) / 6.0 * 450.0;
        let (r, g, b) = star_color_from_temp(star.temp as f32);

        let star_mat = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::new(brightness * r, brightness * g, brightness * b, 1.0),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            fog_enabled: false,
            ..default()
        });

        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat),
            Transform::from_translation(pos),
            Visibility::Visible,
            Star(mag),
            SkyObject(pos),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    let moon_pos = altaz_to_sky_position(35.0, 200.0, radius);
    let moon_mat = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::new(300.0, 320.0, 400.0, 1.0),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        fog_enabled: false,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(2.5))),
        MeshMaterial3d(moon_mat),
        Transform::from_translation(moon_pos),
        Visibility::Visible,
        Moon,
        SkyObject(moon_pos),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    let planets = [
        (
            "Venus", -4.40_f32, 5200.0_f32, 25.0_f32, 245.0_f32, 0.60_f32,
        ),
        ("Jupiter", -2.50, 5500.0, 55.0, 190.0, 0.50),
        ("Mars", -1.50, 3600.0, 40.0, 135.0, 0.45),
        ("Saturn", 0.50, 5400.0, 30.0, 210.0, 0.40),
    ];

    for &(_name, mag, temp, alt, az, sphere_r) in &planets {
        let pos = altaz_to_sky_position(alt, az, radius);
        let (r, g, b) = star_color_from_temp(temp);
        let brightness = 50.0 + (4.5 - mag).max(0.0) / 6.0 * 450.0;

        let mat = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::new(brightness * r, brightness * g, brightness * b, 1.0),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            fog_enabled: false,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(sphere_r))),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            Visibility::Visible,
            Planet(mag),
            SkyObject(pos),
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

// ---------------------------------------------------------------------------
// Lights
// ---------------------------------------------------------------------------

fn setup_lights(flags: Res<DebugFlags>, mut commands: Commands, state: Res<SimulationState>) {
    if !flags.no_lights {
        spawn_lamps(&mut commands, &state);
    }
}

fn spawn_lamps(commands: &mut Commands, state: &SimulationState) {
    let intensity_scale = state.intensity_scale;

    match state.scene {
        SceneChoice::Vischering => spawn_vischering_lamps(commands, state),
        SceneChoice::BistroExterior => {
            for placement in lamp_placements_bistro() {
                let ldt = match placement.luminaire {
                    BistroLuminaire::StreetLight => state.active_ldt().clone(),
                    BistroLuminaire::WallLight => state.flood_ldt.clone(),
                    BistroLuminaire::Spotlight => state.spotlight_ldt.clone(),
                    BistroLuminaire::Lantern => state.uplight_ldt.clone(),
                };
                commands.spawn((
                    EulumdatLightBundle::new(ldt)
                        .with_transform(
                            Transform::from_translation(placement.pos)
                                .with_rotation(placement.rotation),
                        )
                        .with_intensity_scale(intensity_scale)
                        .with_model(false)
                        .with_solid(state.show_solid),
                    StreetLamp,
                ));
            }
        }
        _ => {
            let ldt = state.active_ldt().clone();
            for (pos, rotation) in lamp_light_positions(state.scene) {
                commands.spawn((
                    EulumdatLightBundle::new(ldt.clone())
                        .with_transform(Transform::from_translation(pos).with_rotation(rotation))
                        .with_intensity_scale(intensity_scale)
                        .with_model(true)
                        .with_solid(state.show_solid),
                    StreetLamp,
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

fn count_frames(mut counter: ResMut<FrameCounter>) {
    counter.0 += 1;
}

/// Returns true if the UI panel is currently being hovered or pressed.
#[cfg(feature = "bevy-ui")]
fn ui_wants_pointer(panel: &Query<&Interaction, With<DashboardPanel>>) -> bool {
    panel.iter().any(|i| *i != Interaction::None)
}

fn toggle_mode(
    flags: Res<DebugFlags>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SimulationState>,
    lamps: Query<Entity, With<StreetLamp>>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    state.mode = match state.mode {
        Mode::StandardPollution => {
            state.haze_density = 0.005;
            state.intensity_scale = 0.5;
            state.ambient_brightness = 2.0;
            state.uplight_pct = 3.0;
            Mode::PreservedDarkness
        }
        Mode::PreservedDarkness => {
            state.haze_density = 0.04;
            state.intensity_scale = 1.0;
            state.ambient_brightness = 8.0;
            state.uplight_pct = 45.0;
            Mode::StandardPollution
        }
    };

    for entity in lamps.iter() {
        commands.entity(entity).despawn();
    }

    if !flags.no_lights {
        spawn_lamps(&mut commands, &state);
    }
}

/// Keyboard shortcuts for scene switching and slider adjustments.
///   1 = Sponza, 2 = Urban Street, 3 = Bistro Exterior, 4 = Burg Vischering
///   [ = decrease uplight %, ] = increase uplight %
///   - = decrease intensity, = = increase intensity
///   9 = decrease haze, 0 = increase haze
fn keyboard_shortcuts(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<SimulationState>) {
    let mut scene_changed = false;

    // Scene switching: 1, 2, 3, 4
    if keys.just_pressed(KeyCode::Digit1) && state.scene != SceneChoice::Sponza {
        state.scene = SceneChoice::Sponza;
        scene_changed = true;
    }
    if keys.just_pressed(KeyCode::Digit2) && state.scene != SceneChoice::UrbanStreet {
        state.scene = SceneChoice::UrbanStreet;
        scene_changed = true;
    }
    if keys.just_pressed(KeyCode::Digit3) && state.scene != SceneChoice::BistroExterior {
        state.scene = SceneChoice::BistroExterior;
        scene_changed = true;
    }
    if keys.just_pressed(KeyCode::Digit4) && state.scene != SceneChoice::Vischering {
        state.scene = SceneChoice::Vischering;
        scene_changed = true;
    }

    if scene_changed {
        state.lights_dirty = true;
    }

    // Uplight adjustment: [ and ]
    if keys.just_pressed(KeyCode::BracketLeft) {
        state.uplight_pct = (state.uplight_pct - 10.0).max(0.0);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        state.uplight_pct = (state.uplight_pct + 10.0).min(100.0);
    }

    // Intensity adjustment: - and =
    if keys.just_pressed(KeyCode::Minus) {
        state.intensity_scale = (state.intensity_scale - 0.2).max(0.05);
    }
    if keys.just_pressed(KeyCode::Equal) {
        state.intensity_scale = (state.intensity_scale + 0.2).min(3.0);
    }

    // Haze adjustment: 9 and 0
    if keys.just_pressed(KeyCode::Digit9) {
        state.haze_density = (state.haze_density - 0.01).max(0.001);
    }
    if keys.just_pressed(KeyCode::Digit0) {
        state.haze_density = (state.haze_density + 0.01).min(0.15);
    }
}

fn toggle_solid(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SimulationState>,
    mut lights: Query<&mut PhotometricLight<Eulumdat>>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }
    state.show_solid = !state.show_solid;
    for mut light in lights.iter_mut() {
        light.show_solid = state.show_solid;
    }
}

fn update_fog(state: Res<SimulationState>, mut fog_query: Query<&mut DistanceFog>) {
    let uplight_frac = state.uplight_pct / 100.0;
    let glow = uplight_frac * 0.6;

    for mut fog in fog_query.iter_mut() {
        fog.falloff = FogFalloff::Exponential {
            density: state.haze_density * scene_haze_factor(state.scene),
        };
        fog.color = Color::srgb(0.01 + glow * 0.8, 0.01 + glow * 0.55, 0.03 + glow * 0.4);
    }
}

fn update_ambient(state: Res<SimulationState>, mut commands: Commands) {
    if !state.is_changed() {
        return;
    }
    let uplight_frac = state.uplight_pct / 100.0;
    let clear_color = match state.mode {
        Mode::StandardPollution => {
            let glow = uplight_frac * 0.08;
            Color::srgb(0.005 + glow, 0.005 + glow * 0.7, 0.015 + glow * 0.5)
        }
        Mode::PreservedDarkness => Color::srgb(0.002, 0.002, 0.008),
    };
    commands.insert_resource(ClearColor(clear_color));
    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.7, 0.7, 0.9),
        brightness: state.ambient_brightness,
        affects_lightmapped_meshes: true,
    });
}

fn sync_lights(
    flags: Res<DebugFlags>,
    mut state: ResMut<SimulationState>,
    mut commands: Commands,
    lights: Query<(&PhotometricLight<Eulumdat>, Entity, Option<&LampScale>), With<StreetLamp>>,
    all_lamps: Query<Entity, With<StreetLamp>>,
    scene_geo: Query<Entity, With<SceneGeometry>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut fix_state: ResMut<MaterialFixState>,
    mut prev_scene: ResMut<PreviousScene>,
    mut load_state: ResMut<SceneLoadState>,
    frame_count: Res<FrameCounter>,
) {
    if state.lights_dirty {
        state.lights_dirty = false;

        fix_state.0 = None;

        for entity in scene_geo.iter() {
            commands.entity(entity).despawn();
        }
        for entity in all_lamps.iter() {
            commands.entity(entity).despawn();
        }

        if prev_scene.0.is_some_and(|s| s != SceneChoice::UrbanStreet) {
            let mesh_count = meshes.len();
            let mat_count = materials.len();
            let img_count = images.len();

            let mesh_ids: Vec<_> = meshes.ids().collect();
            for id in mesh_ids {
                meshes.remove(id);
            }
            let mat_ids: Vec<_> = materials.ids().collect();
            for id in mat_ids {
                materials.remove(id);
            }
            let img_ids: Vec<_> = images.ids().collect();
            for id in img_ids {
                images.remove(id);
            }

            println!(
                "Released {} meshes, {} materials, {} images",
                mesh_count, mat_count, img_count
            );
        }
        prev_scene.0 = Some(state.scene);

        spawn_scene_geometry(
            &mut commands,
            &asset_server,
            &state,
            &mut meshes,
            &mut materials,
            &mut load_state,
            frame_count.0,
        );
        if !flags.no_lights {
            spawn_lamps(&mut commands, &state);
        }
        commands.insert_resource(PendingCameraReset(Some(state.scene)));
        return;
    }

    // Only mutate lights that actually need updating — iter_mut() marks all
    // as Changed which triggers the photometric system to despawn+respawn lights.
    for (light, entity, lamp_scale) in lights.iter() {
        let target = state.intensity_scale * lamp_scale.map_or(1.0, |s| s.0);
        if (light.intensity_scale - target).abs() > 0.001 * target.max(1.0) {
            commands
                .entity(entity)
                .insert(eulumdat_bevy::photometric::PhotometricLight {
                    intensity_scale: target,
                    ..light.clone()
                });
        }
    }
}

/// Pending camera reset target, set by sync_lights, consumed by fly_camera_reset.
#[derive(Resource, Default)]
struct PendingCameraReset(Option<SceneChoice>);

/// `--shot=<file.png>`: pin the camera, wait until the scene is loaded and
/// has rendered for ~3 s, save one frame, exit (README / test screenshots).
#[cfg(not(target_arch = "wasm32"))]
fn auto_shot(
    mut commands: Commands,
    flags: Res<DebugFlags>,
    load: Res<SceneLoadState>,
    frames: Res<FrameCounter>,
    state: Res<SimulationState>,
    mut pending: ResMut<PendingCameraReset>,
    mut stage: Local<(u32, u32)>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let Some(path) = &flags.shot else { return };
    pending.0 = Some(state.scene); // the fly camera must not drift
    if !load.loaded {
        return;
    }
    if stage.0 == 0 {
        stage.0 = frames.0;
    } else if stage.1 == 0 && frames.0 > stage.0 + 180 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        stage.1 = frames.0;
    } else if stage.1 != 0 && frames.0 > stage.1 + 30 {
        exit.write(AppExit::Success);
    }
}

fn check_loading_progress(asset_server: Res<AssetServer>, mut load_state: ResMut<SceneLoadState>) {
    if load_state.loaded || load_state.handles.is_empty() {
        return;
    }
    let all_loaded = load_state
        .handles
        .iter()
        .all(|h| asset_server.is_loaded_with_dependencies(h.id()));
    if all_loaded {
        load_state.loaded = true;
    }
}

fn update_star_visibility(
    flags: Res<DebugFlags>,
    state: Res<SimulationState>,
    mut stars: Query<(&mut Visibility, &Star)>,
    mut moon: Query<&mut Visibility, (With<Moon>, Without<Star>, Without<Planet>)>,
    mut planets: Query<(&mut Visibility, &Planet), (Without<Star>, Without<Moon>)>,
) {
    if flags.no_sky || !state.is_changed() {
        return;
    }

    match state.mode {
        Mode::PreservedDarkness => {
            for (mut v, _) in stars.iter_mut() {
                *v = Visibility::Visible;
            }
            for mut v in moon.iter_mut() {
                *v = Visibility::Visible;
            }
            for (mut v, _) in planets.iter_mut() {
                *v = Visibility::Visible;
            }
        }
        Mode::StandardPollution => {
            let limiting_mag = 4.5 - (state.uplight_pct / 100.0) * 6.5;

            for (mut v, star) in stars.iter_mut() {
                *v = if star.0 < limiting_mag {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
            for mut v in moon.iter_mut() {
                *v = Visibility::Visible;
            }
            for (mut v, planet) in planets.iter_mut() {
                *v = if planet.0 < limiting_mag {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

fn track_sky_to_camera(
    flags: Res<DebugFlags>,
    cam: Query<&Transform, With<FlyCamera>>,
    mut sky: Query<(&mut Transform, &SkyObject), Without<FlyCamera>>,
) {
    if flags.no_sky {
        return;
    }
    let Ok(cam_tf) = cam.single() else { return };
    let cam_pos = cam_tf.translation;
    for (mut tf, offset) in sky.iter_mut() {
        tf.translation = cam_pos + offset.0;
    }
}

// ---------------------------------------------------------------------------
// bevy_ui Dashboard (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "bevy-ui")]
fn setup_ui(mut commands: Commands, state: Res<SimulationState>) {
    let ldt = state.active_ldt();

    // -- Loading overlay (centered, initially hidden) --
    let loading_text = commands
        .spawn((
            Text::new("Loading..."),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(VALUE_COLOR),
        ))
        .id();

    let loading_overlay = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(20.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                ))
                .add_child(loading_text);
        })
        .id();

    // -- Main dashboard panel --
    let mut mode_text = Entity::PLACEHOLDER;
    let mut hints_text = Entity::PLACEHOLDER;
    let mut scene_btns = [Entity::PLACEHOLDER; 4];
    let mut ldt_name = Entity::PLACEHOLDER;
    let mut ldt_manufacturer = Entity::PLACEHOLDER;
    let mut ldt_flux = Entity::PLACEHOLDER;
    let mut ldt_cct = Entity::PLACEHOLDER;
    let mut ldt_lor = Entity::PLACEHOLDER;
    let mut uplight_slider = Entity::PLACEHOLDER;
    let mut grade_text = Entity::PLACEHOLDER;
    let mut grade_bar = Entity::PLACEHOLDER;
    let mut intensity_slider = Entity::PLACEHOLDER;
    let mut ambient_slider = Entity::PLACEHOLDER;
    let mut haze_slider = Entity::PLACEHOLDER;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(310.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Interaction::default(),
            DashboardPanel,
            TabGroup::default(),
        ))
        .with_children(|panel| {
            // Title
            panel.spawn((
                Text::new("Skyglow Analysis"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));

            // Mode heading
            mode_text = panel
                .spawn((
                    Text::new("STANDARD POLLUTION"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.35, 0.35)),
                ))
                .id();

            // Hints
            hints_text = panel
                .spawn((
                    Text::new("[Space] toggle  [P] solid  [R] reset  [Z] drawbridge"),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ))
                .id();

            // Separator
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Scene selector
            panel.spawn((
                Text::new("Scene"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));

            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|row| {
                    let scenes = [
                        (SceneChoice::Sponza, "Sponza"),
                        (SceneChoice::UrbanStreet, "Urban"),
                        (SceneChoice::BistroExterior, "Bistro"),
                        (SceneChoice::Vischering, "Vischering"),
                    ];
                    for (i, (choice, label)) in scenes.iter().enumerate() {
                        let is_active = *choice == state.scene;
                        scene_btns[i] = row
                            .spawn((
                                Node {
                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                UiButton,
                                Hovered::default(),
                                SceneButton(*choice),
                                BackgroundColor(if is_active { BTN_ACTIVE } else { BTN_NORMAL }),
                                observe(on_scene_button_activate),
                                children![(
                                    Text::new(*label),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(VALUE_COLOR),
                                )],
                            ))
                            .id();
                    }
                });

            // Separator
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Active luminaire info
            panel.spawn((
                Text::new("Active Luminaire"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));

            panel
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(6.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(SECTION_BG),
                ))
                .with_children(|info| {
                    ldt_name = spawn_kv_row(info, "Name:", &ldt.luminaire_name);
                    ldt_manufacturer = spawn_kv_row(info, "Mfr:", &ldt.identification);
                    ldt_flux = spawn_kv_row(
                        info,
                        "Flux:",
                        &format!("{:.0} lm", ldt.total_luminous_flux()),
                    );
                    ldt_cct = spawn_kv_row(
                        info,
                        "CCT:",
                        &ldt.color_temperature()
                            .map_or("N/A".into(), |c| format!("{:.0} K", c)),
                    );
                    ldt_lor = spawn_kv_row(
                        info,
                        "LOR:",
                        &format!("{:.1}%", ldt.light_output_ratio * 100.0),
                    );
                });

            // Separator
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Environmental impact
            panel.spawn((
                Text::new("Environmental Impact"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));

            uplight_slider = spawn_labeled_slider(
                panel,
                "Uplight %",
                state.uplight_pct,
                0.0,
                100.0,
                SliderBinding::Uplight,
            );

            // Grade display
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|col| {
                    grade_text = col
                        .spawn((
                            Text::new("Sky Glow: D  (Poor)"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.86, 0.47, 0.2)),
                        ))
                        .id();

                    // Progress bar
                    col.spawn((
                        Node {
                            height: Val::Px(6.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    ))
                    .with_children(|track| {
                        grade_bar = track
                            .spawn((
                                Node {
                                    width: Val::Percent(45.0),
                                    height: Val::Percent(100.0),
                                    border_radius: BorderRadius::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.86, 0.47, 0.2)),
                            ))
                            .id();
                    });
                });

            // Separator
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Lighting controls
            panel.spawn((
                Text::new("Lighting"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));
            intensity_slider = spawn_labeled_slider(
                panel,
                "Intensity",
                state.intensity_scale,
                0.05,
                3.0,
                SliderBinding::Intensity,
            );
            ambient_slider = spawn_labeled_slider(
                panel,
                "Ambient",
                state.ambient_brightness,
                0.0,
                500.0,
                SliderBinding::Ambient,
            );

            // Separator
            panel.spawn((
                Node {
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Atmosphere
            panel.spawn((
                Text::new("Atmosphere"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
            ));
            haze_slider = spawn_labeled_slider(
                panel,
                "Haze",
                state.haze_density,
                0.001,
                0.15,
                SliderBinding::Haze,
            );
        });

    commands.insert_resource(UiEntities {
        mode_text,
        hints_text,
        scene_buttons: scene_btns,
        ldt_name,
        ldt_manufacturer,
        ldt_flux,
        ldt_cct,
        ldt_lor,
        uplight_slider,
        grade_text,
        grade_bar,
        intensity_slider,
        ambient_slider,
        haze_slider,
        loading_overlay,
        loading_text,
    });
}

/// Helper: spawn a key-value row, returns the value text entity.
#[cfg(feature = "bevy-ui")]
fn spawn_kv_row(parent: &mut ChildSpawnerCommands, key: &str, value: &str) -> Entity {
    let mut val_entity = Entity::PLACEHOLDER;
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(key),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(LABEL_COLOR),
                Node {
                    width: Val::Px(40.0),
                    ..default()
                },
            ));
            val_entity = row
                .spawn((
                    Text::new(value),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(VALUE_COLOR),
                ))
                .id();
        });
    val_entity
}

/// Helper: spawn a labeled horizontal slider, returns the slider entity.
#[cfg(feature = "bevy-ui")]
fn spawn_labeled_slider(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    initial: f32,
    min: f32,
    max: f32,
    binding: SliderBinding,
) -> Entity {
    let mut slider_entity = Entity::PLACEHOLDER;
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(format!("{label}: {initial:.2}")),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(VALUE_COLOR),
            ));

            slider_entity = col
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Stretch,
                        height: Val::Px(14.0),
                        width: Val::Px(280.0),
                        ..default()
                    },
                    Hovered::default(),
                    Slider {
                        track_click: TrackClick::Snap,
                        ..Default::default()
                    },
                    SliderValue(initial),
                    SliderRange::new(min, max),
                    binding,
                    TabIndex(0),
                    observe(slider_self_update),
                    observe(on_slider_change),
                    Children::spawn((
                        Spawn((
                            Node {
                                height: Val::Px(6.0),
                                border_radius: BorderRadius::all(Val::Px(3.0)),
                                ..default()
                            },
                            BackgroundColor(SLIDER_TRACK_COLOR),
                        )),
                        Spawn((
                            Node {
                                display: Display::Flex,
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                right: Val::Px(14.0),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            children![(
                                DashboardSliderThumb,
                                SliderThumb,
                                Node {
                                    display: Display::Flex,
                                    width: Val::Px(14.0),
                                    height: Val::Px(14.0),
                                    position_type: PositionType::Absolute,
                                    left: percent(0),
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(SLIDER_THUMB_COLOR),
                            )],
                        )),
                    )),
                ))
                .id();
        });
    slider_entity
}

/// Observer: called when a scene button is activated.
#[cfg(feature = "bevy-ui")]
fn on_scene_button_activate(
    activate: On<Activate>,
    scene_btns: Query<&SceneButton>,
    mut state: ResMut<SimulationState>,
) {
    if let Ok(btn) = scene_btns.get(activate.entity) {
        if state.scene != btn.0 {
            state.scene = btn.0;
            state.lights_dirty = true;
        }
    }
}

/// Observer: called when a slider value changes, updates SimulationState.
#[cfg(feature = "bevy-ui")]
fn on_slider_change(
    value_change: On<ValueChange<f32>>,
    bindings: Query<&SliderBinding>,
    mut state: ResMut<SimulationState>,
) {
    if let Ok(binding) = bindings.get(value_change.source) {
        match binding {
            SliderBinding::Uplight => state.uplight_pct = value_change.value,
            SliderBinding::Intensity => state.intensity_scale = value_change.value,
            SliderBinding::Ambient => state.ambient_brightness = value_change.value,
            SliderBinding::Haze => state.haze_density = value_change.value,
        }
    }
}

/// Update text labels and colors when SimulationState changes.
#[cfg(feature = "bevy-ui")]
fn update_ui_from_state(
    state: Res<SimulationState>,
    ui: Res<UiEntities>,
    mut texts: Query<&mut Text>,
    mut colors: Query<&mut TextColor>,
    mut bg_colors: Query<&mut BackgroundColor>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    if !state.is_changed() {
        return;
    }

    // Mode heading
    if let Ok(mut text) = texts.get_mut(ui.mode_text) {
        let (label, color) = match state.mode {
            Mode::StandardPollution => ("STANDARD POLLUTION", Color::srgb(1.0, 0.35, 0.35)),
            Mode::PreservedDarkness => ("PRESERVED DARKNESS", Color::srgb(0.35, 0.78, 1.0)),
        };
        **text = label.into();
        if let Ok(mut tc) = colors.get_mut(ui.mode_text) {
            tc.0 = color;
        }
    }

    // LDT info
    let ldt = state.active_ldt();
    if let Ok(mut t) = texts.get_mut(ui.ldt_name) {
        **t = ldt.luminaire_name.clone();
    }
    if let Ok(mut t) = texts.get_mut(ui.ldt_manufacturer) {
        **t = ldt.identification.clone();
    }
    if let Ok(mut t) = texts.get_mut(ui.ldt_flux) {
        **t = format!("{:.0} lm", ldt.total_luminous_flux());
    }
    if let Ok(mut t) = texts.get_mut(ui.ldt_cct) {
        **t = ldt
            .color_temperature()
            .map_or("N/A".into(), |c| format!("{:.0} K", c));
    }
    if let Ok(mut t) = texts.get_mut(ui.ldt_lor) {
        **t = format!("{:.1}%", ldt.light_output_ratio * 100.0);
    }

    // Sky glow grade
    let score = state.uplight_pct / 100.0;
    let (grade, bar_color) = if score < 0.05 {
        ("A  (Excellent)", Color::srgb(0.2, 0.78, 0.2))
    } else if score < 0.15 {
        ("B  (Good)", Color::srgb(0.47, 0.78, 0.2))
    } else if score < 0.30 {
        ("C  (Moderate)", Color::srgb(0.86, 0.7, 0.2))
    } else if score < 0.50 {
        ("D  (Poor)", Color::srgb(0.86, 0.47, 0.2))
    } else {
        ("F  (Severe)", Color::srgb(0.86, 0.2, 0.2))
    };

    if let Ok(mut t) = texts.get_mut(ui.grade_text) {
        **t = format!("Sky Glow: {grade}");
    }
    if let Ok(mut tc) = colors.get_mut(ui.grade_text) {
        tc.0 = bar_color;
    }

    // Progress bar width + color
    if let Ok(mut node) = nodes.get_mut(ui.grade_bar) {
        node.width = Val::Percent(state.uplight_pct);
    }
    if let Ok(mut bg) = bg_colors.get_mut(ui.grade_bar) {
        bg.0 = bar_color;
    }

    // Sync slider values back (when mode toggle changes them externally)
    commands
        .entity(ui.uplight_slider)
        .insert(SliderValue(state.uplight_pct));
    commands
        .entity(ui.intensity_slider)
        .insert(SliderValue(state.intensity_scale));
    commands
        .entity(ui.ambient_slider)
        .insert(SliderValue(state.ambient_brightness));
    commands
        .entity(ui.haze_slider)
        .insert(SliderValue(state.haze_density));
}

/// Update scene button background colors to highlight active scene.
#[cfg(feature = "bevy-ui")]
fn update_scene_button_visuals(
    state: Res<SimulationState>,
    ui: Res<UiEntities>,
    mut bg: Query<&mut BackgroundColor>,
    scene_btns: Query<&SceneButton>,
) {
    if !state.is_changed() {
        return;
    }
    for &btn_entity in &ui.scene_buttons {
        if let Ok(btn) = scene_btns.get(btn_entity) {
            if let Ok(mut bg) = bg.get_mut(btn_entity) {
                bg.0 = if btn.0 == state.scene {
                    BTN_ACTIVE
                } else {
                    BTN_NORMAL
                };
            }
        }
    }
}

/// Update slider thumb positions and labels when slider values change.
#[cfg(feature = "bevy-ui")]
fn update_slider_visuals(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange, &SliderBinding),
        (Changed<SliderValue>, With<Slider>),
    >,
    children_q: Query<&Children>,
    mut thumbs: Query<&mut Node, With<DashboardSliderThumb>>,
    // Update the label text (the sibling Text above the slider)
    parents: Query<&ChildOf>,
    mut texts: Query<&mut Text>,
    children_list: Query<&Children>,
) {
    for (slider_ent, value, range, binding) in sliders.iter() {
        // Update thumb position
        let position = range.thumb_position(value.0) * 100.0;
        for child in children_q.iter_descendants(slider_ent) {
            if let Ok(mut thumb_node) = thumbs.get_mut(child) {
                thumb_node.left = percent(position);
            }
        }

        // Update the label text (sibling of slider in the parent column)
        let label_name = match binding {
            SliderBinding::Uplight => "Uplight %",
            SliderBinding::Intensity => "Intensity",
            SliderBinding::Ambient => "Ambient",
            SliderBinding::Haze => "Haze",
        };

        // Walk up to parent, then find sibling Text
        if let Ok(child_of) = parents.get(slider_ent) {
            let parent = child_of.parent();
            if let Ok(siblings) = children_list.get(parent) {
                for sibling in siblings.iter() {
                    if sibling != slider_ent {
                        if let Ok(mut text) = texts.get_mut(sibling) {
                            **text = format!("{label_name}: {:.2}", value.0);
                        }
                    }
                }
            }
        }
    }
}

/// Show/hide loading overlay.
#[cfg(feature = "bevy-ui")]
fn update_loading_overlay(
    ui: Res<UiEntities>,
    load_state: Res<SceneLoadState>,
    state: Res<SimulationState>,
    frame_count: Res<FrameCounter>,
    mut visibility: Query<&mut Visibility>,
    mut texts: Query<&mut Text>,
) {
    let should_show = !load_state.loaded && !load_state.handles.is_empty();
    if let Ok(mut vis) = visibility.get_mut(ui.loading_overlay) {
        *vis = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if should_show {
        let elapsed = frame_count.0.saturating_sub(load_state.start_frame);
        let dots = ".".repeat((elapsed as usize / 15) % 4);
        if let Ok(mut text) = texts.get_mut(ui.loading_text) {
            **text = format!("Loading {}{dots}", state.scene.label());
        }
    }
}

// ---------------------------------------------------------------------------
// Fly camera
// ---------------------------------------------------------------------------

fn fly_camera_look(
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
    accumulated: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    #[cfg(feature = "bevy-ui")] panel: Query<&Interaction, With<DashboardPanel>>,
) {
    // Arrow keys rotate the camera (same as right-click drag)
    // This enables keyboard-only camera control for automated recording.
    let mut key_delta = Vec2::ZERO;
    let rotate_speed = 1.5; // radians per second
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::ArrowLeft) {
        key_delta.x -= rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        key_delta.x += rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        key_delta.y -= rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        key_delta.y += rotate_speed * dt / 0.003;
    }

    // Mouse look (right-click drag)
    let mouse_delta = if mouse_button.pressed(MouseButton::Right) {
        #[cfg(feature = "bevy-ui")]
        if ui_wants_pointer(&panel) {
            Vec2::ZERO
        } else {
            accumulated.delta
        }
        #[cfg(not(feature = "bevy-ui"))]
        {
            accumulated.delta
        }
    } else {
        Vec2::ZERO
    };

    let delta = mouse_delta + key_delta;
    if delta == Vec2::ZERO {
        return;
    }

    for (mut transform, mut cam) in query.iter_mut() {
        cam.yaw -= delta.x * cam.sensitivity;
        cam.pitch -= delta.y * cam.sensitivity;
        cam.pitch = cam.pitch.clamp(-1.5, 1.5);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    }
}

fn fly_camera_move(
    mut query: Query<(&mut Transform, &FlyCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut transform, cam) in query.iter_mut() {
        let mut dir = Vec3::ZERO;
        let fwd = transform.forward();
        let fwd_flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let right = transform.right();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        // WASD for movement, Arrow keys for camera rotation (see fly_camera_look)
        if keys.pressed(KeyCode::KeyW) {
            dir += fwd_flat;
        }
        if keys.pressed(KeyCode::KeyS) {
            dir -= fwd_flat;
        }
        if keys.pressed(KeyCode::KeyA) {
            dir -= right_flat;
        }
        if keys.pressed(KeyCode::KeyD) {
            dir += right_flat;
        }
        if keys.pressed(KeyCode::KeyQ) {
            dir += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyE) {
            dir -= Vec3::Y;
        }

        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            cam.speed * 3.0
        } else {
            cam.speed
        };

        if dir != Vec3::ZERO {
            transform.translation += dir.normalize() * speed * time.delta_secs();
        }
    }
}

fn fly_camera_zoom(
    mut query: Query<(&mut Transform, &FlyCamera)>,
    accumulated: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    #[cfg(feature = "bevy-ui")] panel: Query<&Interaction, With<DashboardPanel>>,
) {
    #[cfg(feature = "bevy-ui")]
    if ui_wants_pointer(&panel) {
        return;
    }
    let dy = accumulated.delta.y;
    if dy.abs() > 0.0 {
        for (mut transform, cam) in query.iter_mut() {
            let fwd = transform.forward();
            transform.translation += fwd * dy * cam.speed * 0.5;
        }
    }
}

fn fly_camera_reset(
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<SimulationState>,
    mut pending: ResMut<PendingCameraReset>,
    flags: Res<DebugFlags>,
) {
    let should_reset = keys.just_pressed(KeyCode::KeyR) || pending.0.is_some();
    if !should_reset {
        return;
    }
    pending.0 = None;
    let (start, look) = flags
        .cam
        .unwrap_or((state.scene.cam_start(), state.scene.cam_look_at()));
    for (mut transform, mut cam) in query.iter_mut() {
        *transform = Transform::from_translation(start).looking_at(look, Vec3::Y);
        let new_cam = FlyCamera::from_look_direction(start, look);
        cam.yaw = new_cam.yaw;
        cam.pitch = new_cam.pitch;
    }
}
