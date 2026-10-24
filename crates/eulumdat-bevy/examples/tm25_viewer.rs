//! TM-25 ray file viewer binary — native window or the `?wasm=tm25` bundle.
//!
//!     cargo run --bin tm25-viewer -p eulumdat-bevy --features tm25 --release -- file.TM25RAY
//!
//! Native: pass a path or drag a file onto the window. Browser: the Leptos
//! shell stashes the chosen `File` (see `tm25-loader.js`) and this binary
//! streams it through `Blob.slice`.
//!
//! Keys: 1/2/3 toggle rays / points / far-field solid, C colour mode,
//! [ ] ray length, R reset camera. Mouse: left drag orbit, right drag pan,
//! wheel zoom.
//!
//! Native only: `--screenshot out.png [--frames N]` renders, saves and exits,
//! for documentation images and as a render smoke test. `--solid`,
//! `--no-rays`, `--no-points`, `--rays N`, `--ray-length MM` and
//! `--solid-scale MM` set what the shell would otherwise send via
//! localStorage.

use bevy::prelude::*;
use eulumdat_bevy::tm25::{Tm25StartFile, Tm25ViewerPlugin};

/// `--screenshot <path>` and `--frames <n>`, native only.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
struct ShotFlags {
    path: Option<String>,
    frame: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Wait `frame` frames so the streamed rays and the far-field mesh exist,
/// capture the window, then exit once the file is written.
#[cfg(not(target_arch = "wasm32"))]
fn screenshot_system(
    mut commands: Commands,
    flags: Res<ShotFlags>,
    mut frames: ResMut<FrameCounter>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let Some(path) = flags.path.clone() else {
        return;
    };
    frames.0 += 1;
    if frames.0 == flags.frame {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if frames.0 == flags.frame + 40 {
        std::process::exit(0);
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let start_file = std::env::args().nth(1).filter(|a| !a.starts_with("--"));
    #[cfg(target_arch = "wasm32")]
    let start_file: Option<String> = None;

    #[allow(unused_mut)]
    let mut window = Window {
        title: "TM-25 Ray File Viewer".into(),
        resolution: (1280u32, 800u32).into(),
        present_mode: bevy::window::PresentMode::Fifo,
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#tm25-canvas".into());
        window.fit_canvas_to_parent = true;
        window.prevent_default_event_handling = false;
    }

    #[allow(unused_mut)]
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }))
    .add_plugins(Tm25ViewerPlugin)
    .insert_resource(Tm25StartFile(start_file));

    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();
        let has = |name: &str| args.iter().any(|a| a == name);
        let value_of = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        // Native has no localStorage, so the shell's settings never arrive.
        // These flags cover what a documentation image needs.
        let mut settings = eulumdat_bevy::tm25::Tm25Settings::default();
        if has("--solid") {
            settings.show_solid = true;
        }
        if has("--no-rays") {
            settings.show_rays = false;
        }
        if has("--no-points") {
            settings.show_points = false;
        }
        if let Some(v) = value_of("--rays").and_then(|v| v.parse().ok()) {
            settings.display_count = v;
        }
        if let Some(v) = value_of("--ray-length").and_then(|v| v.parse().ok()) {
            settings.ray_length_mm = v;
        }
        if let Some(v) = value_of("--solid-scale").and_then(|v| v.parse().ok()) {
            settings.solid_scale_mm = v;
        }
        app.insert_resource(settings)
            .insert_resource(ShotFlags {
                path: value_of("--screenshot"),
                frame: value_of("--frames")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(150),
            })
            .init_resource::<FrameCounter>()
            .add_systems(Update, screenshot_system);
    }

    app.run();
}
