//! Camera ray tracing — renders images from the same physics engine.

use crate::pipeline::{GpuMaterial, GpuPrimitive};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

/// Camera configuration — matches CameraConfig in camera.wgsl.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraConfig {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub cam_pos: [f32; 3],
    pub _pad0: f32,
    pub cam_forward: [f32; 3],
    pub _pad1: f32,
    pub cam_right: [f32; 3],
    pub _pad2: f32,
    pub cam_up: [f32; 3],
    pub fov_tan: f32,
    pub num_primitives: u32,
    pub seed_offset: u32,
    pub source_intensity: f32,
    pub source_radius: f32,
}

/// Result from a camera render — RGB pixels.
pub struct CameraImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 3]>, // linear RGB, HDR
}

impl CameraImage {
    /// Convert to 8-bit sRGB bytes (for saving as PNG/BMP).
    pub fn to_srgb_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for pixel in &self.pixels {
            // Reinhard tone mapping
            let mapped = [
                pixel[0] / (1.0 + pixel[0]),
                pixel[1] / (1.0 + pixel[1]),
                pixel[2] / (1.0 + pixel[2]),
            ];
            // Linear to sRGB
            for c in &mapped {
                let srgb = if *c <= 0.0031308 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                bytes.push((srgb.clamp(0.0, 1.0) * 255.0) as u8);
            }
            bytes.push(255); // alpha
        }
        bytes
    }
}

/// GPU camera renderer — uses the same device as GpuTracer.
pub struct GpuCamera {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuCamera {
    /// Create a new GPU camera renderer.
    pub async fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..default()
            })
            .await
            .map_err(|e| format!("No GPU: {e}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("eulumdat-rt-camera"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Device: {e}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("camera.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/camera.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[
                // pixels (read_write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // config (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // primitives (read storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // materials (read storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera_pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("camera_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("trace_camera"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self { device, queue, pipeline, bind_group_layout })
    }

    /// Render an image of the scene.
    pub async fn render(
        &self,
        width: u32,
        height: u32,
        samples_per_pixel: u32,
        camera_pos: [f32; 3],
        look_at: [f32; 3],
        fov_degrees: f32,
        primitives: &[GpuPrimitive],
        materials: &[GpuMaterial],
        source_intensity: f32,
    ) -> CameraImage {
        // Compute camera basis vectors
        let pos = glam::Vec3::from(camera_pos);
        let target = glam::Vec3::from(look_at);
        let forward = (target - pos).normalize();
        let world_up = glam::Vec3::Y;
        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward).normalize();

        let config = CameraConfig {
            width,
            height,
            samples_per_pixel,
            max_bounces: 8,
            cam_pos: camera_pos,
            _pad0: 0.0,
            cam_forward: forward.to_array(),
            _pad1: 0.0,
            cam_right: right.to_array(),
            _pad2: 0.0,
            cam_up: up.to_array(),
            fov_tan: (fov_degrees.to_radians() / 2.0).tan(),
            num_primitives: primitives.len() as u32,
            seed_offset: 42,
            source_intensity,
            source_radius: 0.02,
        };

        let total_pixels = width * height;
        // 4 u32 per pixel: R, G, B, sample_count
        let pixel_buffer_size = (total_pixels * 4) as u64 * 4;

        let config_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam_config"),
            contents: bytemuck::bytes_of(&config),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let pixel_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixel_buf"),
            size: pixel_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buf"),
            size: pixel_buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Dummy buffers if no geometry
        let dummy_prim = GpuPrimitive { ptype: 0, material_id: 0, _pad0: 0, _pad1: 0, params: [0.0; 12] };
        let dummy_mat = GpuMaterial { mtype: 0, _pad0: 0, _pad1: 0, _pad2: 0,
            reflectance: 0.0, ior: 1.0, transmittance: 0.0, min_reflectance: 0.0,
            absorption_coeff: 0.0, scattering_coeff: 0.0, asymmetry: 0.0, thickness: 0.0 };

        let prim_data: Vec<GpuPrimitive> = if primitives.is_empty() { vec![dummy_prim] } else { primitives.to_vec() };
        let mat_data: Vec<GpuMaterial> = if materials.is_empty() { vec![dummy_mat] } else { materials.to_vec() };

        let prim_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam_prims"),
            contents: bytemuck::cast_slice(&prim_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let mat_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam_mats"),
            contents: bytemuck::cast_slice(&mat_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cam_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: pixel_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: config_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: prim_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: mat_buf.as_entire_binding() },
            ],
        });

        // Dispatch: 16x16 workgroups covering the image
        let wg_x = (width + 15) / 16;
        let wg_y = (height + 15) / 16;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cam_encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cam_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        encoder.copy_buffer_to_buffer(&pixel_buf, 0, &readback_buf, 0, pixel_buffer_size);
        self.queue.submit(Some(encoder.finish()));

        // Readback
        let slice = readback_buf.slice(..);
        let (tx, rx) = flume::bounded(1);
        slice.map_async(wgpu::MapMode::Read, move |r| { tx.send(r).unwrap(); });
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok();
        rx.recv_async().await.unwrap().unwrap();

        let data = slice.get_mapped_range();
        let raw: &[u32] = bytemuck::cast_slice(&data);

        let mut pixels = vec![[0.0f32; 3]; total_pixels as usize];
        for i in 0..total_pixels as usize {
            let r = raw[i * 4] as f32 / 1000.0;
            let g = raw[i * 4 + 1] as f32 / 1000.0;
            let b = raw[i * 4 + 2] as f32 / 1000.0;
            let count = raw[i * 4 + 3].max(1) as f32;
            pixels[i] = [r / count, g / count, b / count];
        }

        drop(data);
        readback_buf.unmap();

        CameraImage { width, height, pixels }
    }
}

fn default() -> wgpu::RequestAdapterOptions<'static, 'static> {
    wgpu::RequestAdapterOptions::default()
}
