//! bevy_ui overlay for the native shell: the traced image, a status strip,
//! help, the progress bar, a vertical isolux legend, the grid statistics
//! and the measurement points from `Viewer::overlay`.

use super::{samples_done, RtViewer, MAX_SAMPLES};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use eulumdat::PhotometricCalculations;
use eulumdat_scene::measure::{format_lux, heat_ramp};
use eulumdat_scene::photometry::camera_key_lux;
use eulumdat_scene::scene::SceneKind;
use eulumdat_scene::viewer::ViewMode;

#[derive(Component)]
pub struct RtImageNode;
#[derive(Component)]
pub struct RtStatusText;
#[derive(Component)]
pub struct RtStatusStrip;
#[derive(Component)]
pub struct RtHelpText;
#[derive(Component)]
pub struct RtProgressBar;
#[derive(Component)]
pub struct RtLegend;
#[derive(Component)]
pub struct RtLegendText;
#[derive(Component)]
pub struct RtStatsPanel;
#[derive(Component)]
pub struct RtStatsText;
#[derive(Component)]
pub struct RtPointsLayer;
#[derive(Component)]
pub struct RtPointNode;

/// The editor's accent blue.
const ACCENT: Color = Color::srgb(0.376, 0.647, 0.98);
const PANEL: Color = Color::srgba(0.05, 0.06, 0.08, 0.72);
const TEXT: Color = Color::srgba(0.91, 0.93, 0.95, 1.0);
const MUTED: Color = Color::srgba(0.62, 0.68, 0.76, 1.0);

const HELP: &str = "1 street  2 parking  3 facade  4 office   V view  I isolines  G glare  H/Shift+H haze  [ ] exposure\n\
Up/Down height  Left/Right rotation  T/Shift+T tilt  N count  L layout  W finish  P person  , . spacing  K/Shift+K ceiling, facade distance\n\
C/Shift+C colour temp  B bounces  Tab next sample  R reset view  F1 help   drag orbit | right-drag pan | wheel zoom";

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

pub fn setup_ui(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<super::RtAssets>,
) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode::new(assets.output.clone()),
        GlobalZIndex(-1),
        RtImageNode,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
            border: UiRect::bottom(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(ACCENT),
        GlobalZIndex(2),
        RtStatusStrip,
        children![(Text::new(""), font(12.5), TextColor(TEXT), RtStatusText)],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            bottom: Val::Px(16.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            border: UiRect::left(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(ACCENT),
        GlobalZIndex(2),
        children![(Text::new(HELP), font(10.5), TextColor(MUTED), RtHelpText)],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)),
        GlobalZIndex(2),
        children![(
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(ACCENT),
            RtProgressBar,
        )],
    ));

    // Vertical legend on the right: the ramp as a 1 × 256 texture, max at the top.
    let mut bytes = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let c = heat_ramp(1.0 - i as f32 / 255.0);
        bytes.extend_from_slice(&[
            (c.x * 255.0) as u8,
            (c.y * 255.0) as u8,
            (c.z * 255.0) as u8,
            255,
        ]);
    }
    let ramp = images.add(Image::new(
        Extent3d {
            width: 1,
            height: 256,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Percent(30.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        Visibility::Hidden,
        GlobalZIndex(2),
        RtLegend,
        children![
            (
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(220.0),
                    ..default()
                },
                ImageNode::new(ramp),
            ),
            (
                Node {
                    height: Val::Px(220.0),
                    ..default()
                },
                Text::new(""),
                font(10.5),
                TextColor(MUTED),
                RtLegendText,
            ),
        ],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(80.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
            border: UiRect::right(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        BorderColor::all(ACCENT),
        Visibility::Hidden,
        GlobalZIndex(2),
        RtStatsPanel,
        children![(Text::new(""), font(11.5), TextColor(TEXT), RtStatsText)],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
        GlobalZIndex(1),
        RtPointsLayer,
    ));
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|v| format!("{v:.0} deg"))
        .unwrap_or_else(|| "-".into())
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_status(
    viewer: Res<RtViewer>,
    mut texts: ParamSet<(
        Query<&mut Text, With<RtStatusText>>,
        Query<&mut Text, With<RtLegendText>>,
        Query<&mut Text, With<RtStatsText>>,
    )>,
    mut bars: Query<&mut Node, With<RtProgressBar>>,
    mut help: Query<&mut Visibility, (With<RtHelpText>, Without<RtLegend>, Without<RtStatsPanel>)>,
    mut legend: Query<
        &mut Visibility,
        (With<RtLegend>, Without<RtHelpText>, Without<RtStatsPanel>),
    >,
    mut stats: Query<&mut Visibility, (With<RtStatsPanel>, Without<RtHelpText>, Without<RtLegend>)>,
    mut strip: Query<
        &mut Visibility,
        (
            With<RtStatusStrip>,
            Without<RtHelpText>,
            Without<RtLegend>,
            Without<RtStatsPanel>,
        ),
    >,
) {
    let v = &viewer.0;
    let settings = &v.settings;
    let p = &settings.params;
    let mut line = match &v.photometry {
        Some(ph) => {
            let beam = PhotometricCalculations::beam_angle(&ph.ldt);
            let field = PhotometricCalculations::field_angle(&ph.ldt);
            format!(
                "{}  |  {} lm  |  peak {:.0} cd  |  beam {} / field {}  |  throw C{:.0} ({:.0} %)\n",
                ph.name,
                ph.ldt.luminous_flux() as i64,
                ph.table.max_cd,
                fmt_opt(beam),
                fmt_opt(field),
                ph.throw.c,
                ph.throw.asymmetry * 100.0
            )
        }
        None => "no luminaire\n".to_string(),
    };
    line += &format!(
        "{}  |  {}{}  |  height {:.2} m  |  tilt {:.0} deg  |  rotation {:.0} deg{}  |  {} lum",
        p.kind.label(),
        settings.mode.label(),
        if settings.isolines && settings.mode != ViewMode::Isolux {
            " + isolines"
        } else {
            ""
        },
        p.height,
        p.tilt,
        p.rotation,
        if settings.auto_rotate { " (auto)" } else { "" },
        p.count
    );
    match p.kind {
        SceneKind::Street => {
            line += &format!(
                "  |  {}  |  spacing {:.0} m",
                p.street_layout.label(),
                p.spacing
            );
        }
        SceneKind::Office => line += &format!("  |  ceiling {:.2} m", p.ceiling_height),
        SceneKind::Greenhouse => line += &format!("  |  canopy {:.2} m", p.canopy_height),
        SceneKind::Facade => line += &format!("  |  distance {:.2} m", p.wall_distance),
        SceneKind::Parking => {
            if p.count > 1 {
                line += &format!("  |  spacing {:.1} m", p.spacing);
            }
        }
    }
    line += &format!(
        "\n{:.0} K  |  haze {:.0} %  |  glare {}  |  {:+.1} EV  |  {} bounces  |  key {:.0} lx  |  scale {:.0} lx  |  {}/{} spp  |  {} x {}",
        settings.cct,
        settings.haze * 100.0,
        if settings.glare { "on" } else { "off" },
        settings.exposure_ev,
        settings.bounces,
        camera_key_lux(&v.scene),
        v.lux_max,
        samples_done(v),
        MAX_SAMPLES,
        v.size.x,
        v.size.y
    );
    if let Ok(mut t) = texts.p0().single_mut() {
        if t.0 != line {
            t.0 = line;
        }
    }
    if let Ok(mut t) = texts.p1().single_mut() {
        let m = v.lux_max;
        let s = format!(
            "{:.0} lx\n\n\n\n{:.0}\n\n\n\n{:.0}\n\n\n\n{:.0}\n\n\n\n0",
            m,
            m * 0.75,
            m * 0.5,
            m * 0.25
        );
        if t.0 != s {
            t.0 = s;
        }
    }
    if settings.mode == ViewMode::Points {
        if let Ok(mut t) = texts.p2().single_mut() {
            let stats = v.grid_statistics();
            let s = if stats.is_empty() {
                "measuring...".to_string()
            } else {
                let mut out = String::from("Grid              Em     Emin    Uo\n");
                for g in stats {
                    out += &format!(
                        "{:<16} {:>6} {:>7}   {:.2}\n",
                        g.name,
                        format_lux(g.em),
                        format_lux(g.emin),
                        g.uo
                    );
                }
                out.trim_end().to_string()
            };
            if t.0 != s {
                t.0 = s;
            }
        }
    }
    let progress = (samples_done(v) as f32 / MAX_SAMPLES as f32).min(1.0);
    for mut bar in bars.iter_mut() {
        bar.width = Val::Percent(if progress >= 1.0 || !settings.overlay {
            0.0
        } else {
            progress * 100.0
        });
    }
    let vis = |on: bool| {
        if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    };
    let overlay = settings.overlay;
    for mut vv in help.iter_mut() {
        *vv = vis(overlay && settings.show_help);
    }
    for mut vv in legend.iter_mut() {
        *vv = vis(overlay && settings.mode == ViewMode::Isolux);
    }
    for mut vv in stats.iter_mut() {
        *vv = vis(overlay && settings.mode == ViewMode::Points);
    }
    for mut vv in strip.iter_mut() {
        *vv = vis(overlay);
    }
}

/// Measurement points: value pills in the ramp colour, small dots elsewhere.
pub fn update_points(
    mut commands: Commands,
    viewer: Res<RtViewer>,
    window: Single<&Window, With<PrimaryWindow>>,
    layer: Query<Entity, With<RtPointsLayer>>,
    old: Query<Entity, With<RtPointNode>>,
) {
    for e in old.iter() {
        commands.entity(e).despawn();
    }
    let Ok(layer) = layer.single() else {
        return;
    };
    for p in viewer.0.overlay(window.width(), window.height()) {
        let [r, g, b] = p.color;
        match p.text {
            None => {
                let rad = p.radius;
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(p.x - rad),
                        top: Val::Px(p.y - rad),
                        width: Val::Px(2.0 * rad),
                        height: Val::Px(2.0 * rad),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(r, g, b, 0.95)),
                    BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                    Pickable::IGNORE,
                    ChildOf(layer),
                    RtPointNode,
                ));
            }
            Some(text) => {
                let text_color = if p.light_text {
                    Color::srgba(0.96, 0.97, 0.98, 1.0)
                } else {
                    Color::srgba(0.05, 0.05, 0.06, 1.0)
                };
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(p.x - p.radius),
                        top: Val::Px(p.y - 7.5),
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(0.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(r, g, b, 0.92)),
                    BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                    Pickable::IGNORE,
                    ChildOf(layer),
                    RtPointNode,
                    children![(Text::new(text), font(10.5), TextColor(text_color))],
                ));
            }
        }
    }
}
