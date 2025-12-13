//! Photometric lighting based on Eulumdat data
//!
//! Since Bevy doesn't have native IES/photometric light support,
//! we simulate it using spot lights or by calculating illuminance on surfaces.

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use crate::SceneSettings;
use eulumdat::Eulumdat;

pub struct PhotometricLightPlugin;

impl Plugin for PhotometricLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_lights)
            .add_systems(Update, update_lights);
    }
}

#[derive(Component)]
pub struct PhotometricLight;

#[derive(Component)]
pub struct LuminaireModel;

#[derive(Component)]
pub struct PhotometricSolid;

fn setup_lights(
    mut commands: Commands,
    settings: Res<SceneSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_lights(&mut commands, &settings, &mut meshes, &mut materials);
}

fn update_lights(
    mut commands: Commands,
    settings: Res<SceneSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    lights: Query<Entity, With<PhotometricLight>>,
    luminaires: Query<Entity, With<LuminaireModel>>,
    solids: Query<Entity, With<PhotometricSolid>>,
) {
    if !settings.is_changed() {
        return;
    }

    // Remove existing lights and models
    for entity in lights.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in luminaires.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in solids.iter() {
        commands.entity(entity).despawn_recursive();
    }

    spawn_lights(&mut commands, &settings, &mut meshes, &mut materials);
}

fn spawn_lights(
    commands: &mut Commands,
    settings: &SceneSettings,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    use crate::scene::SceneType;

    // Get luminaire height for positioning calculations
    let lum_height = settings.ldt_data.as_ref()
        .map(|ldt| (ldt.height / 1000.0).max(0.1) as f32)
        .unwrap_or(0.2);

    // Calculate light position based on scene type
    // For pole scenes (Road, Parking, Outdoor), position at end of arm
    // For Room, position at center ceiling
    // Arm is at mounting_height - 0.2, arm radius 0.05, so arm bottom at mounting_height - 0.25
    // Luminaire top should be below arm bottom with small gap
    let arm_bottom = settings.mounting_height - 0.25;
    let luminaire_center_y = arm_bottom - 0.05 - lum_height / 2.0; // 5cm gap below arm

    let light_pos = match settings.scene_type {
        SceneType::Room => Vec3::new(
            settings.room_width / 2.0,
            settings.mounting_height - lum_height / 2.0, // Center of luminaire, hanging from ceiling
            settings.room_length / 2.0,
        ),
        SceneType::Road => {
            // Pole on right sidewalk (w - 0.7), arm extends 0.2m left
            // Lamp hangs directly below arm tip
            Vec3::new(
                settings.room_width - 0.7 - 0.2, // Arm tip position
                luminaire_center_y,
                settings.room_length / 2.0,
            )
        }
        SceneType::Parking | SceneType::Outdoor => {
            // Pole at center, arm extends 0.2m left
            Vec3::new(
                settings.room_width / 2.0 - 0.2, // Arm tip position
                luminaire_center_y,
                settings.room_length / 2.0,
            )
        }
    };

    // Get color temperature, CRI, and luminous flux from LDT data
    let (color_temp, cri, total_flux, lor) = settings.ldt_data.as_ref()
        .map(|ldt| {
            let lamp = ldt.lamp_sets.first();
            let color_temp = lamp
                .map(|l| parse_color_temperature(&l.color_appearance))
                .unwrap_or(4000.0);
            let cri = lamp
                .map(|l| parse_cri_from_group(&l.color_rendering_group))
                .unwrap_or(80.0);
            let total_flux = ldt.total_luminous_flux() as f32;
            let lor = (ldt.light_output_ratio / 100.0) as f32; // Convert % to fraction
            (color_temp, cri, total_flux, lor)
        })
        .unwrap_or((4000.0, 80.0, 1000.0, 1.0));

    // Calculate actual luminaire output
    let luminaire_flux = total_flux * lor;

    let light_color = kelvin_to_rgb(color_temp);
    // Apply CRI-based saturation adjustment (low CRI = more desaturated)
    let light_color = apply_cri_adjustment(light_color, cri);

    // Get downward flux fraction to determine light direction
    let downward_fraction = settings.ldt_data.as_ref()
        .map(|ldt| (ldt.downward_flux_fraction / 100.0) as f32)
        .unwrap_or(1.0);
    let upward_fraction = 1.0 - downward_fraction;

    // Bevy uses lumens for intensity (roughly)
    // Scale factor to make it visible in the scene
    let intensity_scale = 50.0; // Adjust this to taste

    // Main point light for ambient fill
    commands.spawn((
        PointLight {
            color: light_color,
            intensity: luminaire_flux * intensity_scale * 0.3, // 30% as ambient fill
            radius: 0.05,
            range: 50.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(light_pos),
        PhotometricLight,
    ));

    // Add directional lights based on flux distribution
    if let Some(ldt) = &settings.ldt_data {
        let beam_angle = calculate_beam_angle(ldt);

        // Downward spot light (if there's downward flux)
        if downward_fraction > 0.1 {
            // Get luminaire height to position light BELOW the luminaire box
            let lum_height = (ldt.height / 1000.0).max(0.05) as f32;
            // Light source is below the luminaire (light exits from bottom)
            let spot_pos = light_pos - Vec3::Y * (lum_height + 0.05);
            let floor_target = Vec3::new(spot_pos.x, 0.0, spot_pos.z);

            commands.spawn((
                SpotLight {
                    color: light_color,
                    intensity: luminaire_flux * intensity_scale * downward_fraction,
                    range: settings.mounting_height * 4.0,
                    radius: 0.05,
                    inner_angle: beam_angle * 0.5,
                    outer_angle: beam_angle * 1.5,
                    shadows_enabled: settings.show_shadows,
                    ..default()
                },
                Transform::from_translation(spot_pos)
                    .looking_at(floor_target, Vec3::X),
                PhotometricLight,
            ));
        }

        // Upward spot light (for uplights like floor_uplight)
        if upward_fraction > 0.1 {
            let ceiling_target = Vec3::new(light_pos.x, settings.room_height, light_pos.z);
            commands.spawn((
                SpotLight {
                    color: light_color,
                    intensity: luminaire_flux * intensity_scale * upward_fraction,
                    range: settings.room_height * 2.0,
                    radius: 0.05,
                    inner_angle: beam_angle * 0.5,
                    outer_angle: beam_angle * 1.5,
                    shadows_enabled: settings.show_shadows,
                    ..default()
                },
                Transform::from_translation(light_pos)
                    .looking_at(ceiling_target, Vec3::X),
                PhotometricLight,
            ));
        }
    }

    // Luminaire model
    if settings.show_luminaire {
        spawn_luminaire_model(commands, meshes, materials, settings, light_pos, light_color);
    }

    // Photometric solid
    if settings.show_photometric_solid {
        if let Some(ldt) = &settings.ldt_data {
            spawn_photometric_solid(commands, meshes, materials, ldt, light_pos);
        }
    }
}

fn spawn_luminaire_model(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &SceneSettings,
    position: Vec3,
    light_color: Color,
) {
    use crate::scene::SceneType;

    let (width, length, height) = settings.ldt_data.as_ref()
        .map(|ldt| (
            ldt.width / 1000.0,   // mm to m
            ldt.length / 1000.0,  // mm to m (or diameter if width=0)
            (ldt.height / 1000.0).max(0.05),
        ))
        .unwrap_or((0.2, 0.2, 0.05));

    let linear = light_color.to_linear();
    let luminaire_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.3),
        emissive: LinearRgba::new(linear.red * 2.0, linear.green * 2.0, linear.blue * 2.0, 1.0),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    // Determine if cylindrical (width = 0 means length is diameter)
    let is_cylindrical = width < 0.01;

    let (mesh, rotation): (Mesh, Quat) = if is_cylindrical {
        // Cylindrical luminaire: length is diameter, height is the "depth"
        // Bevy Cylinder: axis along Y, circular faces top/bottom
        // For street lamps: rotate so axis is horizontal and PARALLEL to road (Z axis)
        // Then circular face points down toward street
        let diameter = length.max(0.1) as f32;
        let radius = diameter / 2.0;
        let rotation = match settings.scene_type {
            SceneType::Road | SceneType::Parking | SceneType::Outdoor => {
                // Rotate 90° around Z: cylinder axis goes from Y (vertical) to X (perpendicular to road)
                // Length extends toward/away from road, circular ends face along road direction
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
            }
            SceneType::Room => Quat::IDENTITY,
        };
        (
            Cylinder::new(radius, height as f32).into(),
            rotation,
        )
    } else {
        // Rectangular luminaire
        let rotation = match settings.scene_type {
            SceneType::Road | SceneType::Parking | SceneType::Outdoor => {
                // Rotate 90° so length aligns with road
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
            }
            SceneType::Room => Quat::IDENTITY,
        };
        (
            Cuboid::new(width.max(0.1) as f32, height as f32, length.max(0.1) as f32).into(),
            rotation,
        )
    };

    // Position is the luminaire center - no additional offset needed
    // The light source will be spawned BELOW this
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(luminaire_material),
        Transform::from_translation(position)
            .with_rotation(rotation),
        LuminaireModel,
        NotShadowCaster,
    ));
}

fn spawn_photometric_solid(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    ldt: &Eulumdat,
    position: Vec3,
) {
    // Generate mesh vertices from photometric data
    let c_step = 10.0_f64;
    let g_step = 5.0_f64;
    let num_c = (360.0 / c_step) as usize;
    let num_g = (180.0 / g_step) as usize + 1;
    let scale = 0.3_f32;

    let max_intensity = ldt.max_intensity();
    if max_intensity <= 0.0 {
        return;
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for ci in 0..num_c {
        let c_angle = ci as f64 * c_step;
        let c_rad = c_angle.to_radians() as f32;

        for gi in 0..num_g {
            let g_angle = gi as f64 * g_step;
            let normalized = ldt.sample(c_angle, g_angle) / max_intensity;
            let r = normalized as f32 * scale;
            let g_rad = g_angle.to_radians() as f32;

            // Spherical to Cartesian (Y-down for gamma=0)
            let x = r * g_rad.sin() * c_rad.cos();
            let z = r * g_rad.sin() * c_rad.sin();
            let y = -r * g_rad.cos();

            positions.push([x, y, z]);
            normals.push([x, y, z]); // Approximate normals

            // Heatmap color
            let (cr, cg, cb) = heatmap_color(normalized);
            colors.push([cr, cg, cb, 0.7]);
        }
    }

    // Generate indices for triangles
    for c in 0..num_c {
        let next_c = (c + 1) % num_c;
        for g in 0..(num_g - 1) {
            let v0 = (c * num_g + g) as u32;
            let v1 = (next_c * num_g + g) as u32;
            let v2 = (next_c * num_g + (g + 1)) as u32;
            let v3 = (c * num_g + (g + 1)) as u32;

            indices.push(v0);
            indices.push(v1);
            indices.push(v2);
            indices.push(v0);
            indices.push(v2);
            indices.push(v3);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    let solid_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(solid_material),
        Transform::from_translation(position - Vec3::Y * 0.1),
        PhotometricSolid,
    ));
}

/// Parse color temperature from lamp's color appearance string
fn parse_color_temperature(appearance: &str) -> f32 {
    // Try to extract a 4-digit number (typical CCT range 1800-10000)
    let mut digits = String::new();
    for ch in appearance.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                if let Ok(kelvin) = digits.parse::<f32>() {
                    if kelvin >= 1000.0 && kelvin <= 20000.0 {
                        return kelvin;
                    }
                }
            }
        } else {
            digits.clear();
        }
    }
    4000.0 // Default neutral white
}

/// Convert color temperature (Kelvin) to RGB
/// Using Tanner Helland's algorithm
fn kelvin_to_rgb(kelvin: f32) -> Color {
    let temp = kelvin / 100.0;
    let r: f32;
    let g: f32;
    let b: f32;

    // Red
    if temp <= 66.0 {
        r = 255.0;
    } else {
        let x = temp - 60.0;
        r = (329.698727446 * x.powf(-0.1332047592)).clamp(0.0, 255.0);
    }

    // Green
    if temp <= 66.0 {
        g = (99.4708025861 * temp.ln() - 161.1195681661).clamp(0.0, 255.0);
    } else {
        let x = temp - 60.0;
        g = (288.1221695283 * x.powf(-0.0755148492)).clamp(0.0, 255.0);
    }

    // Blue
    if temp >= 66.0 {
        b = 255.0;
    } else if temp <= 19.0 {
        b = 0.0;
    } else {
        let x = temp - 10.0;
        b = (138.5177312231 * x.ln() - 305.0447927307).clamp(0.0, 255.0);
    }

    Color::srgb(r / 255.0, g / 255.0, b / 255.0)
}

/// Calculate approximate beam angle from photometric data
fn calculate_beam_angle(ldt: &Eulumdat) -> f32 {
    let max_intensity = ldt.max_intensity();
    if max_intensity <= 0.0 {
        return std::f32::consts::FRAC_PI_4; // 45 degrees default
    }

    let half_max = max_intensity * 0.5;

    // Find the gamma angle where intensity drops to 50% (beam angle definition)
    for g in 0..90 {
        let intensity = ldt.sample(0.0, g as f64);
        if intensity < half_max {
            return (g as f32).to_radians();
        }
    }

    std::f32::consts::FRAC_PI_2 // 90 degrees if not found
}

/// Heatmap color for visualization
fn heatmap_color(value: f64) -> (f32, f32, f32) {
    let v = value.clamp(0.0, 1.0) as f32;

    if v < 0.25 {
        let t = v / 0.25;
        (0.0, t, 1.0) // Blue to Cyan
    } else if v < 0.5 {
        let t = (v - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - t) // Cyan to Green
    } else if v < 0.75 {
        let t = (v - 0.5) / 0.25;
        (t, 1.0, 0.0) // Green to Yellow
    } else {
        let t = (v - 0.75) / 0.25;
        (1.0, 1.0 - t, 0.0) // Yellow to Red
    }
}

/// Parse CRI (Color Rendering Index) from color rendering group string
/// Groups: 1A (≥90), 1B (80-89), 2A (70-79), 2B (60-69), 3 (40-59), 4 (<40)
fn parse_cri_from_group(group: &str) -> f32 {
    let group = group.trim().to_uppercase();
    match group.as_str() {
        "1A" | "1" => 95.0,
        "1B" => 85.0,
        "2A" | "2" => 75.0,
        "2B" => 65.0,
        "3" => 50.0,
        "4" => 30.0,
        _ => {
            // Try to parse as a number
            group.parse::<f32>().unwrap_or(80.0)
        }
    }
}

/// Apply CRI-based color adjustment
/// Low CRI lights render colors less accurately, simulated by desaturation
fn apply_cri_adjustment(color: Color, cri: f32) -> Color {
    // CRI 100 = full saturation, CRI 0 = grayscale
    // We use a gentler curve: CRI 90+ = full sat, CRI 50 = ~70% sat
    let saturation_factor = if cri >= 90.0 {
        1.0
    } else {
        // Linear interpolation from CRI 50 (0.7) to CRI 90 (1.0)
        let t = ((cri - 50.0) / 40.0).clamp(0.0, 1.0);
        0.7 + 0.3 * t
    };

    // Convert to linear RGB, desaturate, convert back
    let linear = color.to_linear();
    let luminance = 0.2126 * linear.red + 0.7152 * linear.green + 0.0722 * linear.blue;

    let r = luminance + (linear.red - luminance) * saturation_factor;
    let g = luminance + (linear.green - luminance) * saturation_factor;
    let b = luminance + (linear.blue - luminance) * saturation_factor;

    Color::linear_rgb(r, g, b)
}
