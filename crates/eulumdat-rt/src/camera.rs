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
    pub lvk_c_steps: u32,
    pub lvk_g_steps: u32,
    pub lvk_g_max: f32,
    pub lvk_max_intensity: f32,
}

/// Result from a camera render — RGB pixels.
pub struct CameraImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 3]>, // linear RGB, HDR
}

impl CameraImage {
    /// Apply edge-preserving denoise (bilateral filter).
    /// `strength` controls the spatial radius (3-7 recommended).
    pub fn denoise(&mut self, strength: u32) {
        let radius = strength.min(10) as i32;
        let sigma_space = radius as f32 * 0.5;
        let sigma_color = 0.15f32; // how different colors can be before edge is detected
        let src = self.pixels.clone();
        let w = self.width as i32;
        let h = self.height as i32;

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let center = src[idx];
                let mut sum = [0.0f32; 3];
                let mut weight_sum = 0.0f32;

                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < 0 || nx >= w || ny < 0 || ny >= h { continue; }

                        let ni = (ny * w + nx) as usize;
                        let neighbor = src[ni];

                        // Spatial weight (Gaussian)
                        let dist2 = (dx * dx + dy * dy) as f32;
                        let w_space = (-dist2 / (2.0 * sigma_space * sigma_space)).exp();

                        // Color weight (edge-preserving)
                        let cdiff = (center[0] - neighbor[0]).powi(2)
                            + (center[1] - neighbor[1]).powi(2)
                            + (center[2] - neighbor[2]).powi(2);
                        let w_color = (-cdiff / (2.0 * sigma_color * sigma_color)).exp();

                        let w = w_space * w_color;
                        sum[0] += neighbor[0] * w;
                        sum[1] += neighbor[1] * w;
                        sum[2] += neighbor[2] * w;
                        weight_sum += w;
                    }
                }

                if weight_sum > 0.0 {
                    self.pixels[idx] = [
                        sum[0] / weight_sum,
                        sum[1] / weight_sum,
                        sum[2] / weight_sum,
                    ];
                }
            }
        }
    }

    /// Convert to 8-bit sRGB bytes (for saving as PNG/BMP).
    pub fn to_srgb_bytes(&self) -> Vec<u8> {
        self.to_srgb_bytes_with_exposure(1.0)
    }

    /// Convert with exposure adjustment (1.0 = default, 2.0 = brighter).
    pub fn to_srgb_bytes_with_exposure(&self, exposure: f32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for pixel in &self.pixels {
            let exposed = [
                pixel[0] * exposure,
                pixel[1] * exposure,
                pixel[2] * exposure,
            ];
            // ACES filmic tone mapping (more natural than Reinhard)
            let mapped = [
                aces_tonemap(exposed[0]),
                aces_tonemap(exposed[1]),
                aces_tonemap(exposed[2]),
            ];
            // Linear to sRGB gamma
            for c in &mapped {
                let srgb = if *c <= 0.0031308 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                bytes.push((srgb.clamp(0.0, 1.0) * 255.0) as u8);
            }
            bytes.push(255);
        }
        bytes
    }
}

/// ACES filmic tone mapping curve.
fn aces_tonemap(x: f32) -> f32 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
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
                // lvk_data (read storage — light emission pattern)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
    ///
    /// `lvk_data`: optional flat array of intensity values [c0g0, c0g1, ..., c1g0, ...] for
    /// LDT-based light emission. If empty, uses uniform emission.
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
        self.render_with_lvk(width, height, samples_per_pixel, camera_pos, look_at, fov_degrees,
            primitives, materials, source_intensity, &[], 0, 0, 0.0, 0.0).await
    }

    /// Render with LDT-based light emission pattern.
    pub async fn render_with_lvk(
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
        lvk_data: &[f32],
        lvk_c_steps: u32,
        lvk_g_steps: u32,
        lvk_g_max: f32,
        lvk_max_intensity: f32,
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
            lvk_c_steps,
            lvk_g_steps,
            lvk_g_max,
            lvk_max_intensity,
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

        // LVK buffer
        let lvk_buf_data: Vec<f32> = if lvk_data.is_empty() { vec![1.0] } else { lvk_data.to_vec() };
        let lvk_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cam_lvk"),
            contents: bytemuck::cast_slice(&lvk_buf_data),
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
                wgpu::BindGroupEntry { binding: 4, resource: lvk_buf.as_entire_binding() },
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
