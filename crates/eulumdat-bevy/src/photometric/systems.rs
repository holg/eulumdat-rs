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
use bevy::pbr::NotShadowCaster;
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
pub fn update_photometric_lights<T: PhotometricData>(
    mut commands: Commands,
    changed_query: Query<
        (Entity, &PhotometricLight<T>, &GlobalTransform),
        Changed<PhotometricLight<T>>,
    >,
    bevy_lights: Query<(Entity, &BevyLightMarker<T>)>,
    solids: Query<(Entity, &PhotometricSolid<T>)>,
    models: Query<(Entity, &LuminaireModel<T>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, light, global_transform) in changed_query.iter() {
        // Despawn old lights, solids, and models for this entity
        for (light_entity, marker) in bevy_lights.iter() {
            if marker.parent == entity {
                commands.entity(light_entity).despawn_recursive();
            }
        }
        for (solid_entity, marker) in solids.iter() {
            if marker.parent == entity {
                commands.entity(solid_entity).despawn_recursive();
            }
        }
        for (model_entity, marker) in models.iter() {
            if marker.parent == entity {
                commands.entity(model_entity).despawn_recursive();
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
                commands.entity(light_entity).despawn_recursive();
            }
        }
        for (solid_entity, marker) in solids.iter() {
            if marker.parent == removed_entity {
                commands.entity(solid_entity).despawn_recursive();
            }
        }
        for (model_entity, marker) in models.iter() {
            if marker.parent == removed_entity {
                commands.entity(model_entity).despawn_recursive();
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
    let (_, _, height) = data.dimensions();

    // Spawn ambient point light (30% of intensity)
    commands.spawn((
        PointLight {
            color: light_color,
            intensity: luminaire_flux * intensity_scale * 0.3,
            radius: 0.05,
            range: 50.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(position),
        BevyLightMarker::<T>::new(parent_entity),
    ));

    // Spawn downward spot light (if significant downward flux)
    if downward_fraction > 0.1 {
        let spot_pos = position - Vec3::Y * (height + 0.05);
        let floor_target = Vec3::new(spot_pos.x, 0.0, spot_pos.z);

        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: luminaire_flux * intensity_scale * downward_fraction,
                range: position.y * 4.0,
                radius: 0.05,
                inner_angle: beam_angle * 0.5,
                outer_angle: beam_angle * 1.5,
                shadows_enabled: light.shadows_enabled,
                ..default()
            },
            Transform::from_translation(spot_pos).looking_at(floor_target, Vec3::X),
            BevyLightMarker::<T>::new(parent_entity),
        ));
    }

    // Spawn upward spot light (if significant upward flux)
    if upward_fraction > 0.1 {
        let ceiling_target = Vec3::new(position.x, position.y + 10.0, position.z);

        commands.spawn((
            SpotLight {
                color: light_color,
                intensity: luminaire_flux * intensity_scale * upward_fraction,
                range: 20.0,
                radius: 0.05,
                inner_angle: beam_angle * 0.5,
                outer_angle: beam_angle * 1.5,
                shadows_enabled: light.shadows_enabled,
                ..default()
            },
            Transform::from_translation(position).looking_at(ceiling_target, Vec3::X),
            BevyLightMarker::<T>::new(parent_entity),
        ));
    }

    // Spawn luminaire model
    if light.show_model {
        let mesh = luminaire_mesh(data);
        let material = luminaire_material(light_color);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_translation(position),
            LuminaireModel::<T>::new(parent_entity),
            NotShadowCaster,
        ));
    }

    // Spawn photometric solid
    if light.show_solid {
        let mesh = photometric_solid_mesh(data, PhotometricMeshResolution::Medium, 0.3);
        let material = photometric_solid_material();

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_translation(position - Vec3::Y * 0.1),
            PhotometricSolid::<T>::new(parent_entity),
        ));
    }
}
