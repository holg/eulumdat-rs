//! Prepare accumulation texture for progressive rendering.

use super::extract::ExtractedPhotometricCamera;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::TextureFormat,
    texture::TextureCache,
    renderer::RenderDevice,
};

/// Per-camera accumulation texture for progressive path tracing.
#[derive(Component)]
pub struct TracerAccumulationTexture(pub bevy_render::texture::CachedTexture);

pub fn prepare_tracer_accumulation_texture(
    mut commands: Commands,
    cameras: Query<(Entity, &ExtractedCamera, &ExtractedPhotometricCamera)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
) {
    for (entity, camera, _settings) in cameras.iter() {
        let Some(viewport) = camera.physical_viewport_size else {
            continue;
        };

        let texture = texture_cache.get(
            &render_device,
            bevy_render::render_resource::TextureDescriptor {
                label: Some("photometric_accumulation"),
                size: bevy_render::render_resource::Extent3d {
                    width: viewport.x,
                    height: viewport.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: bevy_render::render_resource::TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: bevy_render::render_resource::TextureUsages::STORAGE_BINDING
                    | bevy_render::render_resource::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert(TracerAccumulationTexture(texture));
    }
}
