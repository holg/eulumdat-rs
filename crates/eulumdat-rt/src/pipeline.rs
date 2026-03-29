//! wgpu compute pipeline for photon tracing.

use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

/// GPU trace configuration — matches TraceConfig in WGSL.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuTracerConfig {
    pub detector_c_bins: u32,
    pub detector_g_bins: u32,
    pub detector_c_res: f32,
    pub detector_g_res: f32,
    pub seed_offset: u32,
    pub num_photons: u32,
    pub source_type: u32,
    pub source_flux: f32,
}

/// Source type enum (matches WGSL switch).
#[derive(Clone, Copy, Debug)]
pub enum SourceType {
    Isotropic = 0,
    Lambertian = 1,
}

/// Result from a GPU trace — detector bins as f64.
pub struct GpuDetectorResult {
    bins: Vec<Vec<f64>>,
    num_c: usize,
    num_g: usize,
    c_res: f64,
    g_res: f64,
}

impl GpuDetectorResult {
    /// Total detected energy.
    pub fn total_energy(&self) -> f64 {
        self.bins.iter().flat_map(|row| row.iter()).sum()
    }

    /// Get bins as [c][g] array.
    pub fn bins(&self) -> &Vec<Vec<f64>> {
        &self.bins
    }

    /// Number of C-bins.
    pub fn num_c(&self) -> usize {
        self.num_c
    }

    /// Number of gamma-bins.
    pub fn num_g(&self) -> usize {
        self.num_g
    }

    /// Convert to candela (same formula as CPU detector).
    pub fn to_candela(&self, source_flux_lm: f64) -> Vec<Vec<f64>> {
        let total = self.total_energy();
        if total <= 0.0 {
            return self.bins.clone();
        }

        let flux_per_energy = source_flux_lm / total;
        let dc_rad = (self.c_res as f64).to_radians();

        let mut candela = vec![vec![0.0; self.num_g]; self.num_c];
        for ci in 0..self.num_c {
            for gi in 0..self.num_g {
                let g_rad = (gi as f64 * self.g_res).to_radians();
                let g_lo = (g_rad - self.g_res.to_radians() / 2.0).max(0.0);
                let g_hi = (g_rad + self.g_res.to_radians() / 2.0)
                    .min(std::f64::consts::PI);
                let solid_angle = dc_rad * (g_lo.cos() - g_hi.cos()).abs();
                if solid_angle > 0.0 {
                    candela[ci][gi] = self.bins[ci][gi] * flux_per_energy / solid_angle;
                }
            }
        }
        candela
    }
}

/// The GPU photon tracer.
pub struct GpuTracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuTracer {
    /// Create a new GPU tracer, requesting a wgpu device.
    pub async fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("No GPU adapter found: {e}"))?;

        log::info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("eulumdat-rt"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Failed to create device: {e}"))?;

        // Load shader
        let shader_source = include_str!("shaders/trace.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("trace.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source)),
        });

        // Bind group layout
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rt_bind_group_layout"),
                entries: &[
                    // detector_bins: storage buffer (read_write)
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
                    // config: uniform buffer
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
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rt_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rt_trace_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("trace_photons"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    /// Trace photons from an isotropic source in free space.
    pub async fn trace_isotropic(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
    ) -> GpuDetectorResult {
        self.trace(num_photons, c_res_deg, g_res_deg, SourceType::Isotropic, 1000.0)
            .await
    }

    /// Trace photons from a Lambertian source in free space.
    pub async fn trace_lambertian(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
    ) -> GpuDetectorResult {
        self.trace(num_photons, c_res_deg, g_res_deg, SourceType::Lambertian, 1000.0)
            .await
    }

    /// Core trace dispatch.
    async fn trace(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        source_type: SourceType,
        source_flux: f32,
    ) -> GpuDetectorResult {
        let num_c = (360.0 / c_res_deg).round() as u32;
        let num_g = (180.0 / g_res_deg).round() as u32 + 1;
        let total_bins = num_c * num_g;

        // Config uniform
        let config = GpuTracerConfig {
            detector_c_bins: num_c,
            detector_g_bins: num_g,
            detector_c_res: c_res_deg,
            detector_g_res: g_res_deg,
            seed_offset: 42,
            num_photons,
            source_type: source_type as u32,
            source_flux,
        };

        let config_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("config_buffer"),
                contents: bytemuck::bytes_of(&config),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Detector buffer (zeros)
        let detector_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("detector_buffer"),
            size: (total_bins as u64) * 4, // u32 per bin
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Readback buffer
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buffer"),
            size: (total_bins as u64) * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rt_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: detector_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute
        let workgroup_size = 256u32;
        let num_workgroups = (num_photons + workgroup_size - 1) / workgroup_size;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rt_encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rt_trace_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Copy detector to readback
        encoder.copy_buffer_to_buffer(
            &detector_buffer,
            0,
            &readback_buffer,
            0,
            (total_bins as u64) * 4,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let buffer_slice = readback_buffer.slice(..);
        let (tx, rx) = flume::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok();
        rx.recv_async().await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let raw_bins: &[u32] = bytemuck::cast_slice(&data);

        // Convert fixed-point u32 back to f64
        let mut bins = vec![vec![0.0f64; num_g as usize]; num_c as usize];
        for ci in 0..num_c as usize {
            for gi in 0..num_g as usize {
                let idx = ci * num_g as usize + gi;
                bins[ci][gi] = raw_bins[idx] as f64 / 1_000_000.0;
            }
        }

        drop(data);
        readback_buffer.unmap();

        GpuDetectorResult {
            bins,
            num_c: num_c as usize,
            num_g: num_g as usize,
            c_res: c_res_deg as f64,
            g_res: g_res_deg as f64,
        }
    }
}
