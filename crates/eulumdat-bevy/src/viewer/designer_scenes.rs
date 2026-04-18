//! Designer scene geometry for Bevy 3D viewer.
//!
//! Builds Bevy-native 3D scenes from Area Lighting Designer (exterior)
//! and Zonal Cavity Designer (interior) computation results.
//!
//! Coordinate mapping from scene3d (X=east, Y=north, Z=up) to
//! Bevy (X=right, Y=up, Z=forward).

use super::scenes::SceneGeometry;
use super::ViewerSettings;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use eulumdat::area::LuminairePlace;
use eulumdat::diagram::heatmap_color;
use eulumdat::zonal::{CavityResults, LuminaireLayout, Reflectances, Room};

/// Create a heatmap texture from a lux grid.
fn create_heatmap_image(
    lux_grid: &[Vec<f64>],
    max_lux: f64,
    mask: Option<&Vec<Vec<bool>>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let rows = lux_grid.len();
    let cols = if rows > 0 { lux_grid[0].len() } else { 0 };

    let mut data = Vec::with_capacity(rows * cols * 4);
    // Iterate top-to-bottom for texture (row 0 = top of image)
    for row in (0..rows).rev() {
        for col in 0..cols {
            let lux = lux_grid[row][col];
            let normalized = if max_lux > 0.0 {
                lux / max_lux
            } else {
                0.0
            };
            let color = heatmap_color(normalized);

            let alpha = match mask {
                Some(m) if row < m.len() && col < m[row].len() && !m[row][col] => 38, // ~15%
                _ => 200, // ~78%
            };

            data.push(color.r);
            data.push(color.g);
            data.push(color.b);
            data.push(alpha);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: cols.max(1) as u32,
            height: rows.max(1) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });

    images.add(image)
}

/// Convert reflectance (0.0-1.0) to a Bevy color.
fn reflectance_to_bevy_color(rho: f64) -> Color {
    let v = (rho.clamp(0.0, 1.0) * 0.78 + 0.15) as f32; // 0.15..0.93 range
    Color::srgb(v, v, v)
}

/// Build the exterior designer scene from AreaResult data.
pub fn build_designer_exterior(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    settings: &ViewerSettings,
) {
    let (area_width, area_depth, placements, lux_grid, max_lux, mask) =
        match (&settings.area_result, &settings.area_placements) {
            (Some(ar), placements) => (
                ar.area_width as f32,
                ar.area_depth as f32,
                placements.clone(),
                ar.lux_grid.clone(),
                ar.max_lux,
                ar.mask.clone(),
            ),
            _ => {
                // No data — build a placeholder
                build_exterior_placeholder(commands, meshes, materials);
                return;
            }
        };

    let w = area_width;
    let d = area_depth;

    // Ground slab
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.25),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(w, 0.15, d))),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(w / 2.0, -0.075, d / 2.0),
        SceneGeometry,
    ));

    // Heatmap overlay on ground
    if !lux_grid.is_empty() && max_lux > 0.0 {
        let heatmap_handle = create_heatmap_image(&lux_grid, max_lux, mask.as_ref(), images);
        let heatmap_material = materials.add(StandardMaterial {
            base_color_texture: Some(heatmap_handle),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(w, d))),
            MeshMaterial3d(heatmap_material),
            Transform::from_xyz(w / 2.0, 0.01, d / 2.0),
            SceneGeometry,
            NotShadowCaster,
        ));
    }

    // Spawn poles, arms, heads, base plates for each luminaire placement
    for place in &placements {
        let (ex, ey) = place.effective_position();
        let px = ex as f32;
        let pz = ey as f32;
        let mh = place.mounting_height as f32;

        let pole_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.35, 0.38),
            metallic: 0.8,
            perceptual_roughness: 0.4,
            ..default()
        });

        // Base plate
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.05, 0.3))),
            MeshMaterial3d(pole_material.clone()),
            Transform::from_xyz(place.x as f32, 0.025, place.y as f32),
            SceneGeometry,
        ));

        // Vertical pole (from base to mounting height)
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.08, mh))),
            MeshMaterial3d(pole_material.clone()),
            Transform::from_xyz(place.x as f32, mh / 2.0, place.y as f32),
            SceneGeometry,
            NotShadowCaster,
        ));

        // Arm (horizontal, from pole top to effective luminaire position)
        let arm_len = place.arm_length as f32;
        if arm_len > 0.01 {
            let arm_dir = (place.arm_direction as f32).to_radians();
            let arm_cx = place.x as f32 + arm_len / 2.0 * arm_dir.sin();
            let arm_cz = place.y as f32 + arm_len / 2.0 * arm_dir.cos();
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.04, arm_len))),
                MeshMaterial3d(pole_material.clone()),
                Transform::from_xyz(arm_cx, mh - 0.15, arm_cz)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                SceneGeometry,
                NotShadowCaster,
            ));
        }

        // Luminaire head (emissive)
        let head_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.9, 0.7),
            emissive: LinearRgba::new(2.0, 1.8, 1.2, 1.0),
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.08, 0.4))),
            MeshMaterial3d(head_material),
            Transform::from_xyz(px, mh - 0.04, pz),
            SceneGeometry,
            NotShadowCaster,
        ));

        // Light cones (4 triangle meshes, warm yellow, semi-transparent)
        if settings.show_light_cones {
            spawn_light_cone(commands, meshes, materials, Vec3::new(px, mh - 0.08, pz), mh);
        }
    }
}

/// Build the interior designer scene from Room + zonal cavity data.
pub fn build_designer_interior(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    settings: &ViewerSettings,
) {
    let room = match &settings.designer_room {
        Some(r) => r,
        None => {
            build_interior_placeholder(commands, meshes, materials);
            return;
        }
    };

    let l = room.length as f32;
    let w = room.width as f32;
    let h = room.height as f32;
    let wp_h = room.workplane_height as f32;
    let sus = room.suspension_length as f32;

    let refl = settings
        .designer_reflectances
        .as_ref()
        .cloned()
        .unwrap_or_else(|| Reflectances::new(0.80, 0.50, 0.20));

    // Floor
    let floor_material = materials.add(StandardMaterial {
        base_color: reflectance_to_bevy_color(refl.floor),
        perceptual_roughness: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, w))),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(l / 2.0, 0.0, w / 2.0),
        SceneGeometry,
    ));

    // Ceiling
    let ceiling_color = reflectance_to_bevy_color(refl.ceiling);
    let ceiling_material = materials.add(StandardMaterial {
        base_color: ceiling_color.with_alpha(0.25),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, w))),
        MeshMaterial3d(ceiling_material),
        Transform::from_xyz(l / 2.0, h, w / 2.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
        SceneGeometry,
    ));

    let wall_color = reflectance_to_bevy_color(refl.wall);

    // Back wall (z=0) — more opaque for depth
    let back_wall_mat = materials.add(StandardMaterial {
        base_color: wall_color.with_alpha(0.60),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, h))),
        MeshMaterial3d(back_wall_mat),
        Transform::from_xyz(l / 2.0, h / 2.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Left wall (x=0) — more opaque
    let left_wall_mat = materials.add(StandardMaterial {
        base_color: wall_color.with_alpha(0.60),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, h))),
        MeshMaterial3d(left_wall_mat),
        Transform::from_xyz(0.0, h / 2.0, w / 2.0)
            .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Front wall (z=w) — transparent for visibility
    let front_wall_mat = materials.add(StandardMaterial {
        base_color: wall_color.with_alpha(0.15),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, h))),
        MeshMaterial3d(front_wall_mat),
        Transform::from_xyz(l / 2.0, h / 2.0, w)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Right wall (x=l) — transparent for visibility
    let right_wall_mat = materials.add(StandardMaterial {
        base_color: wall_color.with_alpha(0.15),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, h))),
        MeshMaterial3d(right_wall_mat),
        Transform::from_xyz(l, h / 2.0, w / 2.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Workplane heatmap (if PPB result available)
    if let Some(ppb) = &settings.designer_ppb {
        if !ppb.lux_grid.is_empty() && ppb.max_lux > 0.0 {
            let heatmap_handle = create_heatmap_image(&ppb.lux_grid, ppb.max_lux, None, images);
            let heatmap_material = materials.add(StandardMaterial {
                base_color_texture: Some(heatmap_handle),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Mesh3d(meshes.add(Plane3d::default().mesh().size(l, w))),
                MeshMaterial3d(heatmap_material),
                Transform::from_xyz(l / 2.0, wp_h + 0.005, w / 2.0),
                SceneGeometry,
                NotShadowCaster,
            ));
        }
    }

    // Luminaire grid
    if let Some(layout) = &settings.designer_layout {
        let lum_y = h - sus;

        let head_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.9, 0.7),
            emissive: LinearRgba::new(2.0, 1.8, 1.2, 1.0),
            ..default()
        });

        let cable_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.2),
            metallic: 0.3,
            perceptual_roughness: 0.6,
            ..default()
        });

        for row in 0..layout.rows {
            for col in 0..layout.cols {
                let lx = layout.offset_x as f32 + row as f32 * layout.spacing_x as f32;
                let lz = layout.offset_y as f32 + col as f32 * layout.spacing_y as f32;

                // Luminaire head
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.6, 0.06, 0.3))),
                    MeshMaterial3d(head_material.clone()),
                    Transform::from_xyz(lx, lum_y, lz),
                    SceneGeometry,
                    NotShadowCaster,
                ));

                // Suspension rod
                if sus > 0.01 {
                    let rod_y = h - sus / 2.0;
                    commands.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.02, sus))),
                        MeshMaterial3d(cable_material.clone()),
                        Transform::from_xyz(lx, rod_y, lz),
                        SceneGeometry,
                        NotShadowCaster,
                    ));
                }

                // Light cones
                if settings.show_light_cones {
                    spawn_light_cone(
                        commands,
                        meshes,
                        materials,
                        Vec3::new(lx, lum_y - 0.03, lz),
                        lum_y,
                    );
                }
            }
        }
    }

    // Cavity zone overlays (on back + left walls)
    if settings.show_cavities {
        if let Some(cavity) = &settings.designer_cavity {
            spawn_cavity_zones(commands, meshes, materials, room, cavity);
        }
    }
}

/// Spawn a downward light cone visualization (4 triangle faces).
fn spawn_light_cone(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    top: Vec3,
    height_above_ground: f32,
) {
    let cone_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.7, 0.12),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    // Cone spreads from top point down to ground, 30-degree half-angle
    let spread = height_above_ground * 0.577; // tan(30°) ≈ 0.577
    let bottom_y = 0.0;

    let corners = [
        Vec3::new(top.x - spread, bottom_y, top.z - spread),
        Vec3::new(top.x + spread, bottom_y, top.z - spread),
        Vec3::new(top.x + spread, bottom_y, top.z + spread),
        Vec3::new(top.x - spread, bottom_y, top.z + spread),
    ];

    for i in 0..4 {
        let c0 = corners[i];
        let c1 = corners[(i + 1) % 4];

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        let positions = vec![
            [top.x, top.y, top.z],
            [c0.x, c0.y, c0.z],
            [c1.x, c1.y, c1.z],
        ];
        let normals = vec![[0.0, 1.0, 0.0]; 3];
        let uvs = vec![[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2]);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(indices);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(cone_material.clone()),
            SceneGeometry,
            NotShadowCaster,
        ));
    }
}

/// Spawn cavity zone overlays on the back and left walls.
fn spawn_cavity_zones(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    room: &Room,
    cavity: &CavityResults,
) {
    let l = room.length as f32;
    let w = room.width as f32;
    let h = room.height as f32;
    let wp_h = room.workplane_height as f32;
    let sus = room.suspension_length as f32;

    // Ceiling cavity: from ceiling to luminaire plane (blue tint)
    if sus > 0.01 {
        let cc_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.3, 0.5, 1.0, 0.12),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        // On back wall (z=0.01)
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(l, sus))),
            MeshMaterial3d(cc_mat.clone()),
            Transform::from_xyz(l / 2.0, h - sus / 2.0, 0.01)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
        // On left wall (x=0.01)
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(w, sus))),
            MeshMaterial3d(cc_mat),
            Transform::from_xyz(0.01, h - sus / 2.0, w / 2.0)
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
    }

    // Room cavity: from luminaire plane to workplane (green tint)
    let rc_height = (h - sus - wp_h).max(0.0);
    if rc_height > 0.01 {
        let rc_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.3, 1.0, 0.5, 0.10),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        let rc_center_y = wp_h + rc_height / 2.0;
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(l, rc_height))),
            MeshMaterial3d(rc_mat.clone()),
            Transform::from_xyz(l / 2.0, rc_center_y, 0.01)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(w, rc_height))),
            MeshMaterial3d(rc_mat),
            Transform::from_xyz(0.01, rc_center_y, w / 2.0)
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
    }

    // Floor cavity: from workplane to floor (orange tint)
    if wp_h > 0.01 {
        let fc_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.6, 0.2, 0.10),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(l, wp_h))),
            MeshMaterial3d(fc_mat.clone()),
            Transform::from_xyz(l / 2.0, wp_h / 2.0, 0.01)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(w, wp_h))),
            MeshMaterial3d(fc_mat),
            Transform::from_xyz(0.01, wp_h / 2.0, w / 2.0)
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            SceneGeometry,
            NotShadowCaster,
        ));
    }

    // Info text labels could use cavity ratios (rcr, ccr, fcr) but
    // deferred to egui panel for now — no 3D text labels.
    let _ = cavity; // suppress unused warning, ratios used above for zone sizing
}

/// Placeholder exterior scene when no area result data is present.
fn build_exterior_placeholder(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Simple ground plane
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.3),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(ground),
        Transform::from_xyz(10.0, 0.0, 10.0),
        SceneGeometry,
    ));
}

/// Placeholder interior scene when no room data is present.
fn build_interior_placeholder(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Simple room box (4x5x2.8)
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.7),
        perceptual_roughness: 0.8,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(4.0, 5.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(2.0, 0.0, 2.5),
        SceneGeometry,
    ));

    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.85, 0.85, 0.5),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });
    // Back wall
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(4.0, 2.8))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_xyz(2.0, 1.4, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));
    // Left wall
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 2.8))),
        MeshMaterial3d(wall_mat),
        Transform::from_xyz(0.0, 1.4, 2.5)
            .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));
}

/// Calculate luminaire transforms for the exterior designer scene.
pub fn calculate_exterior_transforms(
    placements: &[LuminairePlace],
) -> Vec<super::controls::LuminaireTransform> {
    placements
        .iter()
        .map(|p| {
            let (ex, ey) = p.effective_position();
            let tilt_rad = (p.tilt_angle as f32).to_radians();
            let rot_rad = (p.rotation as f32).to_radians();
            super::controls::LuminaireTransform {
                position: Vec3::new(ex as f32, p.mounting_height as f32 - 0.04, ey as f32),
                rotation: Quat::from_rotation_y(-rot_rad) * Quat::from_rotation_x(tilt_rad),
            }
        })
        .collect()
}

/// Calculate luminaire transforms for the interior designer scene.
pub fn calculate_interior_transforms(
    room: &Room,
    layout: &LuminaireLayout,
) -> Vec<super::controls::LuminaireTransform> {
    let lum_y = (room.height - room.suspension_length) as f32;
    let mut transforms = Vec::with_capacity(layout.count);
    for row in 0..layout.rows {
        for col in 0..layout.cols {
            let lx = layout.offset_x as f32 + row as f32 * layout.spacing_x as f32;
            let lz = layout.offset_y as f32 + col as f32 * layout.spacing_y as f32;
            transforms.push(super::controls::LuminaireTransform {
                position: Vec3::new(lx, lum_y, lz),
                rotation: Quat::IDENTITY,
            });
        }
    }
    transforms
}
