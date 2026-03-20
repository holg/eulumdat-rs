//! Obscura Demo: Darkness Preservation Simulator
//!
//! A Bevy example showcasing light pollution vs. darkness preservation.
//! Built as a pitch demo for L'Observatoire de la Nuit.
//!
//! Controls:
//!   Space       — Toggle simulation mode
//!   WASD/Arrows — Move camera
//!   Q / E       — Up / Down
//!   Right-click + drag — Look around
//!   Scroll      — Zoom
//!   R           — Reset camera
//!   P           — Toggle photometric solid visualization
//!
//! Run:
//!   cargo run --example obscura_demo -p eulumdat-bevy --features post-process,bevy-ui --release

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{NotShadowCaster, NotShadowReceiver, TransmittedShadowReceiver};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::camera::Hdr;
use bevy::render::render_resource::Face;
use bevy::render::view::NoIndirectDrawing;
use eulumdat::Eulumdat;
use eulumdat_bevy::photometric::{PhotometricLight, PhotometricPlugin};
#[cfg(feature = "bevy-ui")]
use eulumdat_bevy::photometric::PhotometricData;
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
    observe, slider_self_update, Activate, Button as UiButton,
    Slider, SliderRange, SliderThumb, SliderValue, TrackClick, UiWidgetsPlugins, ValueChange,
};

// ---------------------------------------------------------------------------
// Embedded LDT data
// ---------------------------------------------------------------------------

const POLLUTION_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/road_luminaire.ldt");
const PRESERVED_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/projector.ldt");
const FLOOD_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/wiki-flood.ldt");
const SPOTLIGHT_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/wiki-spotlight.ldt");
const UPLIGHT_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/floor_uplight.ldt");

// ---------------------------------------------------------------------------
// UI colors (bevy-ui feature only)
// ---------------------------------------------------------------------------

#[cfg(feature = "bevy-ui")]
mod ui_colors {
    use bevy::prelude::Color;
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
}

impl SceneChoice {
    #[cfg(feature = "bevy-ui")]
    fn label(&self) -> &str {
        match self {
            SceneChoice::Sponza => "Sponza Atrium",
            SceneChoice::UrbanStreet => "Urban Street",
            SceneChoice::BistroExterior => "Bistro Exterior",
        }
    }

    fn cam_start(&self) -> Vec3 {
        match self {
            SceneChoice::Sponza => Vec3::new(-3.7, 0.0, -0.5),
            SceneChoice::UrbanStreet => Vec3::new(8.0, 1.7, 45.0),
            SceneChoice::BistroExterior => Vec3::new(-10.5, 1.7, -1.0),
        }
    }

    fn cam_look_at(&self) -> Vec3 {
        match self {
            SceneChoice::Sponza => Vec3::new(10.0, 4.0, -0.5),
            SceneChoice::UrbanStreet => Vec3::new(0.0, 4.0, 20.0),
            SceneChoice::BistroExterior => Vec3::new(0.0, 3.5, 0.0),
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
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
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
    scene_buttons: [Entity; 3],
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
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let (debug_flags, scene) = {
        let args: Vec<String> = std::env::args().collect();
        let flags = DebugFlags {
            no_sky: args.iter().any(|a| a == "--no-sky"),
            no_matfix: args.iter().any(|a| a == "--no-matfix"),
            no_lights: args.iter().any(|a| a == "--no-lights"),
        };
        let scene = args.iter().skip(1)
            .find(|a| !a.starts_with("--"))
            .cloned();
        if flags.no_sky { println!("DEBUG: --no-sky — sky objects disabled"); }
        if flags.no_matfix { println!("DEBUG: --no-matfix — material fixes disabled"); }
        if flags.no_lights { println!("DEBUG: --no-lights — photometric lights disabled"); }
        (flags, scene)
    };
    #[cfg(target_arch = "wasm32")]
    let (debug_flags, scene) = {
        (DebugFlags { no_sky: false, no_matfix: false, no_lights: false }, None::<String>)
    };

    let pollution_ldt =
        Eulumdat::parse(POLLUTION_LDT).expect("Failed to parse road_luminaire.ldt");
    let preserved_ldt =
        Eulumdat::parse(PRESERVED_LDT).expect("Failed to parse projector.ldt");
    let flood_ldt = Eulumdat::parse(FLOOD_LDT).expect("Failed to parse wiki-flood.ldt");
    let spotlight_ldt =
        Eulumdat::parse(SPOTLIGHT_LDT).expect("Failed to parse wiki-spotlight.ldt");
    let uplight_ldt =
        Eulumdat::parse(UPLIGHT_LDT).expect("Failed to parse floor_uplight.ldt");

    #[allow(unused_mut)]
    let mut window = Window {
        title: "Obscura Demo — Darkness Preservation Simulator".into(),
        resolution: (1280u32, 720u32).into(),
        present_mode: bevy::window::PresentMode::Fifo,
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#obscura-canvas".into());
        window.fit_canvas_to_parent = true;
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .add_plugins(PhotometricPlugin::<Eulumdat>::default())
        .insert_resource(SimulationState {
            mode: Mode::StandardPollution,
            scene: match scene.as_deref() {
                Some("sponza") => SceneChoice::Sponza,
                Some("bistro") => SceneChoice::BistroExterior,
                _ => SceneChoice::UrbanStreet,
            },
            haze_density: 0.04,
            intensity_scale: 2.0,
            ambient_brightness: 8.0,
            uplight_pct: 45.0,
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
        .insert_resource(FrameCounter(0))
        .add_systems(Startup, (setup_scene, setup_stars, setup_lights))
        .add_systems(Update, fix_scene_materials)
        .add_systems(
            Update,
            (
                toggle_mode,
                toggle_solid,
                update_fog,
                update_ambient,
                update_star_visibility,
                sync_lights,
                check_loading_progress,
                count_frames,
            ),
        )
        .add_systems(Update, (fly_camera_look, fly_camera_move, fly_camera_zoom, fly_camera_reset).chain())
        .add_systems(PostUpdate, track_sky_to_camera);

    #[cfg(feature = "bevy-ui")]
    {
        app.add_plugins(UiWidgetsPlugins)
            .add_plugins(InputDispatchPlugin)
            .add_plugins(TabNavigationPlugin)
            .add_systems(Startup, setup_ui)
            .add_systems(
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
        DirectionalLight {
            color: Color::srgb(0.7, 0.75, 0.95),
            illuminance: 15000.0,
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

    let cam_transform = Transform::from_translation(state.scene.cam_start())
        .looking_at(state.scene.cam_look_at(), Vec3::Y);

    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::AgX,
        Bloom {
            intensity: 0.08,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server
                .load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 600.0,
            ..default()
        },
        cam_transform,
        DistanceFog {
            color: Color::srgb(0.05, 0.04, 0.06),
            falloff: FogFalloff::Exponential { density: state.haze_density },
            ..default()
        },
        FlyCamera::from_look_direction(state.scene.cam_start(), state.scene.cam_look_at()),
        NoIndirectDrawing,
        // Finer cluster grid for dense photometric lighting scenes
        bevy::light::cluster::ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: default(),
            dynamic_resizing: true,
        },
    ));

    spawn_scene_geometry(&mut commands, &asset_server, &state, &mut meshes, &mut materials, &mut load_state, 0);
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
            commands.spawn((SceneRoot(h), SceneGeometry));
        }
        SceneChoice::BistroExterior => {
            let h = asset_server.load(bistro_glb);
            load_state.handles.push(h.clone().untyped());
            commands.spawn((SceneRoot(h), SceneGeometry));

            let gi = asset_server.load("BistroExteriorFakeGI.gltf#Scene0");
            load_state.handles.push(gi.clone().untyped());
            commands.spawn((SceneRoot(gi), SceneGeometry));
        }
        SceneChoice::UrbanStreet => {
            load_state.loaded = true;
            build_urban_street(commands, meshes, materials);
        }
    }
}

fn fix_scene_materials(
    flags: Res<DebugFlags>,
    mut commands: Commands,
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
        SceneChoice::UrbanStreet => {
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
            for entity in transmitted_entities {
                commands.entity(entity).insert(TransmittedShadowReceiver);
            }
            println!("BISTRO FIX: patched {} materials", fixed);
        }
        SceneChoice::UrbanStreet => unreachable!(),
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
        let face_x = if cx > 0.0 { cx - bw / 2.0 - 0.02 } else { cx + bw / 2.0 + 0.02 };
        for floor in 0..((bh / 3.2) as i32) {
            for col in 0..((bd / 2.5) as i32) {
                if rng.next_f32() < 0.3 { continue; }
                let emissive = if rng.next_f32() > 0.3 { window_warm } else { window_cool };
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.02, 1.4, 1.0))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.05, 0.05, 0.08),
                        emissive,
                        ..default()
                    })),
                    Transform::from_xyz(face_x, 2.0 + floor as f32 * 3.2, cz - bd / 2.0 + 1.5 + col as f32 * 2.5),
                    SceneGeometry,
                ));
            }
        }
    }

    let tree_canopy = Color::srgb(0.04, 0.12, 0.04);
    let trunk_color = Color::srgb(0.18, 0.12, 0.05);
    for &(tx, tz, r, th) in &[
        (-7.0_f32, 10.0, 2.0, 4.5), (-8.5, 20.0, 1.6, 3.8), (-6.0, 35.0, 2.2, 5.0),
        (-9.0, 48.0, 1.8, 4.2), (-7.5, 62.0, 2.0, 4.8), (8.0, 30.0, 1.4, 3.5),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.12, th))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: trunk_color, perceptual_roughness: 0.95, ..default() })),
            Transform::from_xyz(tx, th / 2.0, tz), SceneGeometry,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(r))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: tree_canopy, perceptual_roughness: 0.9, ..default() })),
            Transform::from_xyz(tx, th + r * 0.6, tz), SceneGeometry,
        ));
    }
}

fn lamp_light_positions(scene: SceneChoice) -> Vec<(Vec3, Quat)> {
    match scene {
        SceneChoice::Sponza => lamp_positions_sponza(),
        SceneChoice::UrbanStreet => lamp_positions_urban(),
        SceneChoice::BistroExterior => Vec::new(),
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
    if flags.no_sky { return; }
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
            emissive: LinearRgba::new(
                brightness * r,
                brightness * g,
                brightness * b,
                1.0,
            ),
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
        ("Venus",   -4.40_f32, 5200.0_f32, 25.0_f32, 245.0_f32, 0.60_f32),
        ("Jupiter", -2.50,     5500.0,      55.0,     190.0,     0.50),
        ("Mars",    -1.50,     3600.0,      40.0,     135.0,     0.45),
        ("Saturn",   0.50,     5400.0,      30.0,     210.0,     0.40),
    ];

    for &(_name, mag, temp, alt, az, sphere_r) in &planets {
        let pos = altaz_to_sky_position(alt, az, radius);
        let (r, g, b) = star_color_from_temp(temp);
        let brightness = 50.0 + (4.5 - mag).max(0.0) / 6.0 * 450.0;

        let mat = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::new(
                brightness * r,
                brightness * g,
                brightness * b,
                1.0,
            ),
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
                        .with_transform(
                            Transform::from_translation(pos).with_rotation(rotation),
                        )
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
            density: state.haze_density,
        };
        fog.color = Color::srgb(
            0.01 + glow * 0.8,
            0.01 + glow * 0.55,
            0.03 + glow * 0.4,
        );
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
    lights: Query<(&PhotometricLight<Eulumdat>, Entity), With<StreetLamp>>,
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

        spawn_scene_geometry(&mut commands, &asset_server, &state, &mut meshes, &mut materials, &mut load_state, frame_count.0);
        if !flags.no_lights {
            spawn_lamps(&mut commands, &state);
        }
        commands.insert_resource(PendingCameraReset(Some(state.scene)));
        return;
    }

    // Only mutate lights that actually need updating — iter_mut() marks all
    // as Changed which triggers the photometric system to despawn+respawn lights.
    let target = state.intensity_scale;
    for (light, entity) in lights.iter() {
        if (light.intensity_scale - target).abs() > 0.001 {
            commands.entity(entity).insert(
                eulumdat_bevy::photometric::PhotometricLight {
                    intensity_scale: target,
                    ..light.clone()
                },
            );
        }
    }
}

/// Pending camera reset target, set by sync_lights, consumed by fly_camera_reset.
#[derive(Resource, Default)]
struct PendingCameraReset(Option<SceneChoice>);

fn check_loading_progress(
    asset_server: Res<AssetServer>,
    mut load_state: ResMut<SceneLoadState>,
) {
    if load_state.loaded || load_state.handles.is_empty() {
        return;
    }
    let all_loaded = load_state.handles.iter().all(|h| {
        asset_server.is_loaded_with_dependencies(h.id())
    });
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
    if flags.no_sky { return; }
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
    let mut scene_btns = [Entity::PLACEHOLDER; 3];
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
                Text::new("Obscura Analysis"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(LABEL_COLOR),
            ));

            // Mode heading
            mode_text = panel
                .spawn((
                    Text::new("STANDARD POLLUTION"),
                    TextFont { font_size: FontSize::Px(16.0), ..default() },
                    TextColor(Color::srgb(1.0, 0.35, 0.35)),
                ))
                .id();

            // Hints
            hints_text = panel
                .spawn((
                    Text::new("[Space] toggle  [P] solid  [R] reset"),
                    TextFont { font_size: FontSize::Px(10.0), ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ))
                .id();

            // Separator
            panel.spawn((
                Node { height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Scene selector
            panel.spawn((
                Text::new("Scene"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
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
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(VALUE_COLOR),
                                )],
                            ))
                            .id();
                    }
                });

            // Separator
            panel.spawn((
                Node { height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Active luminaire info
            panel.spawn((
                Text::new("Active Luminaire"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
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
                    ldt_flux = spawn_kv_row(info, "Flux:", &format!("{:.0} lm", ldt.total_luminous_flux()));
                    ldt_cct = spawn_kv_row(info, "CCT:", &ldt.color_temperature().map_or("N/A".into(), |c| format!("{:.0} K", c)));
                    ldt_lor = spawn_kv_row(info, "LOR:", &format!("{:.1}%", ldt.light_output_ratio * 100.0));
                });

            // Separator
            panel.spawn((
                Node { height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Environmental impact
            panel.spawn((
                Text::new("Environmental Impact"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(LABEL_COLOR),
            ));

            uplight_slider = spawn_labeled_slider(panel, "Uplight %", state.uplight_pct, 0.0, 100.0, SliderBinding::Uplight);

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
                            TextFont { font_size: FontSize::Px(12.0), ..default() },
                            TextColor(Color::srgb(0.86, 0.47, 0.2)),
                        ))
                        .id();

                    // Progress bar
                    col
                        .spawn((
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
                Node { height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Lighting controls
            panel.spawn((
                Text::new("Lighting"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(LABEL_COLOR),
            ));
            intensity_slider = spawn_labeled_slider(panel, "Intensity", state.intensity_scale, 0.05, 3.0, SliderBinding::Intensity);
            ambient_slider = spawn_labeled_slider(panel, "Ambient", state.ambient_brightness, 0.0, 500.0, SliderBinding::Ambient);

            // Separator
            panel.spawn((
                Node { height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));

            // Atmosphere
            panel.spawn((
                Text::new("Atmosphere"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(LABEL_COLOR),
            ));
            haze_slider = spawn_labeled_slider(panel, "Haze", state.haze_density, 0.001, 0.15, SliderBinding::Haze);
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
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(LABEL_COLOR),
                Node { width: Val::Px(40.0), ..default() },
            ));
            val_entity = row
                .spawn((
                    Text::new(value),
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
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
                TextFont { font_size: FontSize::Px(11.0), ..default() },
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
    if let Ok(mut t) = texts.get_mut(ui.ldt_name) { **t = ldt.luminaire_name.clone(); }
    if let Ok(mut t) = texts.get_mut(ui.ldt_manufacturer) { **t = ldt.identification.clone(); }
    if let Ok(mut t) = texts.get_mut(ui.ldt_flux) { **t = format!("{:.0} lm", ldt.total_luminous_flux()); }
    if let Ok(mut t) = texts.get_mut(ui.ldt_cct) {
        **t = ldt.color_temperature().map_or("N/A".into(), |c| format!("{:.0} K", c));
    }
    if let Ok(mut t) = texts.get_mut(ui.ldt_lor) { **t = format!("{:.1}%", ldt.light_output_ratio * 100.0); }

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
    commands.entity(ui.uplight_slider).insert(SliderValue(state.uplight_pct));
    commands.entity(ui.intensity_slider).insert(SliderValue(state.intensity_scale));
    commands.entity(ui.ambient_slider).insert(SliderValue(state.ambient_brightness));
    commands.entity(ui.haze_slider).insert(SliderValue(state.haze_density));
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
                bg.0 = if btn.0 == state.scene { BTN_ACTIVE } else { BTN_NORMAL };
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
        *vis = if should_show { Visibility::Visible } else { Visibility::Hidden };
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
    #[cfg(feature = "bevy-ui")] panel: Query<&Interaction, With<DashboardPanel>>,
) {
    #[cfg(feature = "bevy-ui")]
    if ui_wants_pointer(&panel) {
        return;
    }
    if !mouse_button.pressed(MouseButton::Right) {
        return;
    }

    let delta = accumulated.delta;
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

        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            dir += fwd_flat;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            dir -= fwd_flat;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            dir -= right_flat;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
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
) {
    let should_reset = keys.just_pressed(KeyCode::KeyR) || pending.0.is_some();
    if !should_reset {
        return;
    }
    pending.0 = None;
    let start = state.scene.cam_start();
    let look = state.scene.cam_look_at();
    for (mut transform, mut cam) in query.iter_mut() {
        *transform = Transform::from_translation(start).looking_at(look, Vec3::Y);
        let new_cam = FlyCamera::from_look_direction(start, look);
        cam.yaw = new_cam.yaw;
        cam.pitch = new_cam.pitch;
    }
}
