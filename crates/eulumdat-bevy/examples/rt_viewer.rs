//! Path-traced scene viewer binary — native window or the `?wasm=rt_viewer` bundle.
//!
//!     cargo run --bin rt-viewer -p eulumdat-bevy --features rt-viewer --release -- file.ldt
//!
//! Native: pass an LDT/IES path or drop a file onto the window. Browser: the
//! editor's current luminaire is read from localStorage.
//!
//! Native only: `--screenshot out.png [--frames N]` renders, saves and exits,
//! for documentation images and as a render smoke test. `--scene street|parking|
//! facade|office` and `--mode render|isolux|points` set the start view.

use bevy::prelude::*;
use eulumdat_bevy::rt::{RtInputLocked, RtStartFile, RtStartSettings, RtViewerPlugin};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
struct ShotFlags {
    path: Option<String>,
    frame: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
struct FrameCounter(u32);

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
        title: "Photometric scene viewer".into(),
        resolution: (1280u32, 800u32).into(),
        present_mode: bevy::window::PresentMode::Fifo,
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#rt-canvas".into());
        window.fit_canvas_to_parent = true;
        window.prevent_default_event_handling = false;
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }))
    .add_plugins(RtViewerPlugin)
    .insert_resource(RtStartFile(start_file));

    #[cfg(not(target_arch = "wasm32"))]
    {
        use eulumdat_scene::scene::SceneParams;
        use eulumdat_scene::sync::scene_from_id;
        use eulumdat_scene::viewer::{Settings, ViewMode};
        let args: Vec<String> = std::env::args().collect();
        let value_of = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let mut settings = Settings::default();
        if let Some(s) = value_of("--scene") {
            settings.params = SceneParams::defaults(scene_from_id(&s));
        }
        if let Some(m) = value_of("--mode") {
            settings.mode = ViewMode::from_id(&m);
            settings.isolines = settings.mode == ViewMode::Isolux;
        }
        if let Some(h) = value_of("--haze").and_then(|v| v.parse().ok()) {
            settings.haze = h;
        }
        let shot = value_of("--screenshot");
        app.insert_resource(RtStartSettings(settings))
            .insert_resource(RtInputLocked(shot.is_some()))
            .insert_resource(ShotFlags {
                path: shot,
                frame: value_of("--frames")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(200),
            })
            .init_resource::<FrameCounter>()
            .add_systems(Update, screenshot_system);
    }

    app.run();
}
