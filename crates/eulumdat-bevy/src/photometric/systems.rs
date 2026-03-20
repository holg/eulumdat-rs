//! Bevy systems for photometric lighting.
//!
//! These systems handle:
//! - Spawning Bevy lights from PhotometricLight components
//! - Updating lights when components change
//! - Managing photometric solid and luminaire model entities

#![allow(clippy::type_complexity)]

use super::{
    apply_cri_adjustment, kelvin_to_color, luminaire_material, luminaire_mesh,
    photometric_solid_material, photometric_solid_mesh, BevyLightMarker, LuminaireModel,
    PhotometricData, PhotometricLight, PhotometricMeshResolution, PhotometricSolid,
};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

/// System to spawn Bevy lights for new PhotometricLight entities.
pub fn spawn_photometric_lights<T: PhotometricData>(
    mut commands: Commands,
    query: Query<(Entity, &PhotometricLight<T>, &GlobalTransform), Added<PhotometricLight<T>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, light, global_transform) in query.iter() {
        spawn_lights_for_entity(
            &mut commands,
            entity,
            light,
            global_transform,
            &mut meshes,
            &mut materials,
        );
    }
}

/// System to update Bevy lights when PhotometricLight components change.
/// Skips newly-added entities (handled by `spawn_photometric_lights`).
pub fn update_photometric_lights<T: PhotometricData>(
    mut commands: Commands,
    changed_query: Query<
        (Entity, &PhotometricLight<T>, &GlobalTransform),
        Changed<PhotometricLight<T>>,
    >,
    added: Query<(), Added<PhotometricLight<T>>>,
    bevy_lights: Query<(Entity, &BevyLightMarker<T>)>,
    solids: Query<(Entity, &PhotometricSolid<T>)>,
    models: Query<(Entity, &LuminaireModel<T>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, light, global_transform) in changed_query.iter() {
        // Skip entities that were just added this frame — spawn system handles those
        if added.contains(entity) {
            continue;
        }
        // Despawn old lights, solids, and models for this entity
        for (light_entity, marker) in bevy_lights.iter() {
            if marker.parent == entity {
                commands.entity(light_entity).despawn();
            }
        }
        for (solid_entity, marker) in solids.iter() {
            if marker.parent == entity {
                commands.entity(solid_entity).despawn();
            }
        }
        for (model_entity, marker) in models.iter() {
            if marker.parent == entity {
                commands.entity(model_entity).despawn();
            }
        }

        // Respawn with updated settings
        spawn_lights_for_entity(
            &mut commands,
            entity,
            light,
            global_transform,
            &mut meshes,
            &mut materials,
        );
    }
}

/// System to cleanup Bevy lights when PhotometricLight entities are removed.
pub fn cleanup_photometric_lights<T: PhotometricData>(
    mut commands: Commands,
    mut removed: RemovedComponents<PhotometricLight<T>>,
    bevy_lights: Query<(Entity, &BevyLightMarker<T>)>,
    solids: Query<(Entity, &PhotometricSolid<T>)>,
    models: Query<(Entity, &LuminaireModel<T>)>,
) {
    for removed_entity in removed.read() {
        // Despawn all related entities
        for (light_entity, marker) in bevy_lights.iter() {
            if marker.parent == removed_entity {
                commands.entity(light_entity).despawn();
            }
        }
        for (solid_entity, marker) in solids.iter() {
            if marker.parent == removed_entity {
                commands.entity(solid_entity).despawn();
            }
        }
        for (model_entity, marker) in models.iter() {
            if marker.parent == removed_entity {
                commands.entity(model_entity).despawn();
            }
        }
    }
}

/// Helper function to spawn all light-related entities for a PhotometricLight.
fn spawn_lights_for_entity<T: PhotometricData>(
    commands: &mut Commands,
    parent_entity: Entity,
    light: &PhotometricLight<T>,
    global_transform: &GlobalTransform,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let data = &light.data;
    let position = global_transform.translation();
    let rotation = global_transform.to_scale_rotation_translation().1;

    // Calculate light parameters
    let total_flux = data.total_flux() as f32;
    let lor = data.light_output_ratio() as f32;
    let luminaire_flux = total_flux * lor;

    // Get color from data or use default
    let color_temp = data.color_temperature().unwrap_or(4000.0);
    let cri = data.cri().unwrap_or(80.0);
    let light_color = apply_cri_adjustment(kelvin_to_color(color_temp), cri);

    let downward_fraction = data.downward_fraction() as f32;
    let upward_fraction = data.upward_fraction() as f32;
    let beam_angle = data.beam_angle() as f32;

    // Intensity scaling (Bevy uses lumens-like units)
    let intensity_scale = 50.0 * light.intensity_scale;

    // Get luminaire dimensions
    let (_, _, _height) = data.dimensions();

    // Calculate light direction based on luminaire rotation
    // Default up direction is +Y, apply luminaire rotation
    let up_direction = rotation * Vec3::Y;

    // Spawn ambient point light (30% of intensity)
    commands.spawn((
        PointLight {
            color: light_color,
            intensity: luminaire_flux * intensity_scale * 0.3,
            radius: 0.05,
            range: 50.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(position),
        BevyLightMarker::<T>::new(parent_entity),
    ));

    // Spawn downward spot lights (if significant downward flux)
    // For asymmetric luminaires (like road lights), we create multiple angled spots
    // that follow the luminaire's rotation to illuminate in the correct direction
    if downward_fraction > 0.1 {
        let spot_pos = position;

        // Get the luminaire's local axes (rotated by parent transform)
        // After 90° Y rotation for road scene:
        // - local_x points along road (was Z before rotation)
        // - local_z points across road (was -X before rotation)
        let local_x = rotation * Vec3::X; // After rotation: along road
        let local_z = rotation * Vec3::Z; // After rotation: across road

        // Main downward spot (30% of total)
        // Calculate target based on luminaire rotation - light points along -Y in local space
        // After rotation, the "down" direction becomes rotation * -Y
        let down_dir = rotation * (-Vec3::Y);
        let main_target = spot_pos + down_dir * position.y.max(10.0);
        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: luminaire_flux * intensity_scale * downward_fraction * 0.3,
                range: position.y * 3.0,
                radius: 0.05,
                inner_angle: beam_angle * 0.2,
                outer_angle: beam_angle * 0.6,
                shadow_maps_enabled: light.shadow_maps_enabled,
                ..default()
            },
            Transform::from_translation(spot_pos).looking_at(main_target, local_z),
            BevyLightMarker::<T>::new(parent_entity),
        ));

        // Side spots pointing in local Z directions (perpendicular to main beam)
        // These provide wider coverage
        let side_intensity = luminaire_flux * intensity_scale * downward_fraction * 0.35;

        // Side spot in positive local Z direction
        // Target combines the down direction with a side offset
        let side_offset = local_z * 8.0;
        let target_across_pos = spot_pos + down_dir * position.y.max(10.0) * 0.5 + side_offset;
        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: side_intensity,
                range: position.y * 4.0,
                radius: 0.05,
                inner_angle: 0.3, // ~17 degrees
                outer_angle: 0.8, // ~46 degrees
                shadow_maps_enabled: light.shadow_maps_enabled,
                ..default()
            },
            Transform::from_translation(spot_pos).looking_at(target_across_pos, local_x),
            BevyLightMarker::<T>::new(parent_entity),
        ));

        // Across road - negative Z direction (toward sidewalk)
        let target_across_neg = Vec3::new(
            spot_pos.x - local_z.x * 4.0,
            0.0,
            spot_pos.z - local_z.z * 4.0,
        );
        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: side_intensity * 0.5, // Less light toward sidewalk
                range: position.y * 3.0,
                radius: 0.05,
                inner_angle: 0.2,
                outer_angle: 0.6,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(spot_pos).looking_at(target_across_neg, local_x),
            BevyLightMarker::<T>::new(parent_entity),
        ));
    }

    // Spawn upward spot light (if significant upward flux)
    if upward_fraction > 0.1 {
        let target = position + up_direction * 10.0;
        let forward = rotation * Vec3::Z;
        let up_hint = if forward.dot(up_direction).abs() > 0.99 {
            rotation * Vec3::X
        } else {
            forward
        };

        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: luminaire_flux * intensity_scale * upward_fraction,
                range: 20.0,
                radius: 0.05,
                inner_angle: beam_angle * 0.5,
                outer_angle: beam_angle * 1.5,
                shadow_maps_enabled: light.shadow_maps_enabled,
                ..default()
            },
            Transform::from_translation(position).looking_at(target, up_hint),
            BevyLightMarker::<T>::new(parent_entity),
        ));
    }

    // Spawn luminaire model (apply rotation)
    if light.show_model {
        let mesh = luminaire_mesh(data);
        let material = luminaire_material(light_color);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_translation(position).with_rotation(rotation),
            LuminaireModel::<T>::new(parent_entity),
            NotShadowCaster,
        ));
    }

    // Spawn photometric solid (apply rotation)
    if light.show_solid {
        let mesh = photometric_solid_mesh(data, PhotometricMeshResolution::Medium, 0.3);
        let material = photometric_solid_material();

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_translation(position - Vec3::Y * 0.1).with_rotation(rotation),
            PhotometricSolid::<T>::new(parent_entity),
        ));
    }
}
