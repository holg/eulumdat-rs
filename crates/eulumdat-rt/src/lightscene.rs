//! Light scene renderer: the `eulumdat-scene` tracer on plain wgpu.
//!
//! One bind group, three compute pipelines (`trace`, `probe`, `present`)
//! and a blit of the presented image into whatever texture the host hands
//! in: a canvas surface on the web, an offscreen texture for the headless
//! `rt-scene` binary. Accumulation lives in storage buffers; the probe
//! values come back through an asynchronous buffer map so a frame never
//! waits on the GPU.

use eulumdat_scene::gpu::{Params, PROBE_PASS_SAMPLES};
use eulumdat_scene::photometry::IesTable;
use eulumdat_scene::scene::{MAX_LAMPS, MAX_PRIMS};
use eulumdat_scene::viewer::FrameInput;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var t_img: texture_2d<f32>;
@group(0) @binding(1) var s_img: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VOut {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var o: VOut;
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(t_img, s_img, in.uv).rgb, 1.0);
}
"#;

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(16),
        usage,
        mapped_at_creation: false,
    })
}

struct Output {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

pub struct SceneRenderer {
    layout: wgpu::BindGroupLayout,
    trace: wgpu::ComputePipeline,
    probe: wgpu::ComputePipeline,
    present: wgpu::ComputePipeline,
    blit: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    boxes: wgpu::Buffer,
    lamps: wgpu::Buffer,
    color: Option<wgpu::Buffer>,
    lux: Option<wgpu::Buffer>,
    output: Option<Output>,
    ies: wgpu::Texture,
    ies_view: wgpu::TextureView,
    ies_version: u64,
    probe_pos: Option<wgpu::Buffer>,
    probe_out: Option<wgpu::Buffer>,
    probe_staging: Option<wgpu::Buffer>,
    probe_count: usize,
    probes_version: u64,
    mesh: wgpu::Buffer,
    mesh_version: u64,
    bind_group: Option<wgpu::BindGroup>,
    blit_group: Option<wgpu::BindGroup>,
    size: [u32; 2],
    samples: u32,
    probe_samples: u32,
    frame: u32,
    reset_token: u64,
    probe_reset_token: u64,
    map_in_flight: bool,
    map_done: Arc<AtomicBool>,
    probe_dirty: bool,
}

impl SceneRenderer {
    /// `target_format` is the format of the texture views handed to
    /// [`render`](Self::render); on the web use the sRGB view format of the
    /// canvas so the linear output is encoded on write.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lightscene trace.wgsl"),
            source: wgpu::ShaderSource::Wgsl(eulumdat_scene::TRACE_WGSL.into()),
        });
        let storage = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lightscene bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1, true),
                storage(2, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                storage(4, false),
                storage(5, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                storage(7, true),
                storage(8, false),
                storage(9, true),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lightscene pl"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let compute = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let trace = compute("trace");
        let probe = compute("probe");
        let present = compute("present");

        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lightscene blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lightscene blit bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let blit_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lightscene blit pl"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lightscene blit"),
            layout: Some(&blit_pl),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lightscene blit sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let rw = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let params = buffer(
            device,
            "lightscene params",
            std::mem::size_of::<Params>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let boxes = buffer(device, "lightscene prims", (MAX_PRIMS * 96) as u64, rw);
        let lamps = buffer(device, "lightscene lamps", (MAX_LAMPS * 64) as u64, rw);
        let mesh = buffer(device, "lightscene mesh", 32, rw);
        let (ies, ies_view) = Self::ies_texture(device, 1, 1);

        Self {
            layout,
            trace,
            probe,
            present,
            blit,
            blit_layout,
            sampler,
            params,
            boxes,
            lamps,
            color: None,
            lux: None,
            output: None,
            ies,
            ies_view,
            ies_version: 0,
            probe_pos: None,
            probe_out: None,
            probe_staging: None,
            probe_count: 0,
            probes_version: 0,
            mesh,
            mesh_version: 0,
            bind_group: None,
            blit_group: None,
            size: [0, 0],
            samples: 0,
            probe_samples: 0,
            frame: 0,
            reset_token: 0,
            probe_reset_token: 0,
            map_in_flight: false,
            map_done: Arc::new(AtomicBool::new(false)),
            probe_dirty: false,
        }
    }

    fn ies_texture(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lightscene ies"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    /// Photometry samples accumulated so far.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Illuminance samples per measurement point so far.
    pub fn probe_samples(&self) -> u32 {
        self.probe_samples
    }

    /// Upload the frame's data, run the passes that are due and blit the
    /// presented image into `target`. `table` is read only when the
    /// `ies_version` in `input` changed.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &FrameInput,
        table: Option<&IesTable>,
        target: &wgpu::TextureView,
    ) {
        let mut rebind = self.bind_group.is_none();
        let size = [input.size.x.max(1), input.size.y.max(1)];
        if size != self.size || self.color.is_none() {
            let px = (size[0] as u64) * (size[1] as u64);
            let rw = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
            self.color = Some(buffer(device, "lightscene acc color", px * 16, rw));
            self.lux = Some(buffer(device, "lightscene acc lux", px * 16, rw));
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("lightscene output"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.output = Some(Output {
                _texture: texture,
                view,
            });
            self.size = size;
            self.samples = 0;
            rebind = true;
            self.blit_group = None;
        }
        if input.ies_version != self.ies_version {
            if let Some(t) = table {
                let (tex, view) = Self::ies_texture(device, t.c_size, t.rows());
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &t.to_bytes(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(t.c_size * 4),
                        rows_per_image: Some(t.rows()),
                    },
                    wgpu::Extent3d {
                        width: t.c_size,
                        height: t.rows(),
                        depth_or_array_layers: 1,
                    },
                );
                self.ies = tex;
                self.ies_view = view;
                self.ies_version = input.ies_version;
                rebind = true;
            }
        }
        if self.probe_pos.is_none() || input.probes_version != self.probes_version {
            let n = input.probes.len();
            let bytes: &[u8] = bytemuck::cast_slice(input.probes.as_slice());
            let pos = buffer(
                device,
                "lightscene probe pos",
                bytes.len() as u64,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            );
            if !bytes.is_empty() {
                queue.write_buffer(&pos, 0, bytes);
            }
            let out_size = (n.max(1) * 16) as u64;
            self.probe_pos = Some(pos);
            self.probe_out = Some(buffer(
                device,
                "lightscene probe out",
                out_size,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            ));
            // A map in flight belongs to the old layout; let it complete and drop it.
            self.probe_staging = Some(buffer(
                device,
                "lightscene probe staging",
                out_size,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ));
            self.map_in_flight = false;
            self.map_done.store(false, Ordering::Relaxed);
            self.probe_count = n;
            self.probes_version = input.probes_version;
            self.probe_samples = 0;
            rebind = true;
        }
        if input.mesh_version != self.mesh_version {
            let rows: Vec<[f32; 4]> = input.mesh.as_ref().map(|m| m.packed()).unwrap_or_default();
            let bytes: &[u8] = bytemuck::cast_slice(rows.as_slice());
            self.mesh = buffer(
                device,
                "lightscene mesh",
                (bytes.len() as u64).max(32),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            );
            if !bytes.is_empty() {
                queue.write_buffer(&self.mesh, 0, bytes);
            }
            self.mesh_version = input.mesh_version;
            rebind = true;
        }
        if input.reset_token != self.reset_token {
            self.reset_token = input.reset_token;
            self.samples = 0;
        }
        if input.probe_reset_token != self.probe_reset_token {
            self.probe_reset_token = input.probe_reset_token;
            self.probe_samples = 0;
        }

        let mut boxes = input.prims.clone();
        boxes.truncate(MAX_PRIMS);
        let mut lamps = input.lamps.clone();
        lamps.truncate(MAX_LAMPS);
        if !boxes.is_empty() {
            queue.write_buffer(&self.boxes, 0, bytemuck::cast_slice(&boxes));
        }
        if !lamps.is_empty() {
            queue.write_buffer(&self.lamps, 0, bytemuck::cast_slice(&lamps));
        }
        let do_trace = input.trace && self.samples < input.max_samples;
        let do_probe =
            input.probe && self.probe_count > 0 && self.probe_samples < input.probe_max_samples;

        let mut params = input.params;
        params.counts.z = lamps.len() as u32;
        params.counts.w = boxes.len() as u32;
        params.frame.x = self.frame;
        params.frame.y = self.probe_count as u32;
        params.frame.z = PROBE_PASS_SAMPLES;
        params.f0.x = 1.0 / (self.samples as f32 + 1.0);
        params.f0.y =
            PROBE_PASS_SAMPLES as f32 / (self.probe_samples as f32 + PROBE_PASS_SAMPLES as f32);
        params.size.x = size[0];
        params.size.y = size[1];
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));

        if rebind {
            let output = self.output.as_ref().unwrap();
            self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lightscene bg"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.boxes.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.lamps.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.ies_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.color.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.lux.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&output.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: self.probe_pos.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: self.probe_out.as_ref().unwrap().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: self.mesh.as_entire_binding(),
                    },
                ],
            }));
        }
        if self.blit_group.is_none() {
            let output = self.output.as_ref().unwrap();
            self.blit_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lightscene blit bg"),
                layout: &self.blit_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }

        // The trace runs in bands of rows, one submission each: a browser
        // resets the GPU ("device lost") when one command buffer runs too
        // long. The band's first row goes to the shader in `flags.y`; the
        // uniform write before each submission is ordered with it.
        // Extra passes per frame reuse the frame's blend factor, so passes
        // beyond the first are weighted like the first. Fine for convergence.
        if do_trace {
            let passes = input
                .passes_per_frame
                .max(1)
                .min((input.max_samples - self.samples).max(1));
            let bands = input.bands.clamp(1, size[1].max(1));
            let band_rows = size[1].div_ceil(bands).div_ceil(8) * 8;
            for _ in 0..passes {
                let mut row0 = 0;
                while row0 < size[1] {
                    let rows = band_rows.min(size[1] - row0);
                    params.flags.y = row0;
                    queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("lightscene trace band"),
                        });
                    {
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("lightscene trace"),
                            timestamp_writes: None,
                        });
                        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
                        pass.set_pipeline(&self.trace);
                        pass.dispatch_workgroups(size[0].div_ceil(8), rows.div_ceil(8), 1);
                    }
                    queue.submit(Some(encoder.finish()));
                    row0 += rows;
                }
            }
            self.samples += passes;
            params.flags.y = 0;
            queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("lightscene frame"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lightscene compute"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            if do_probe {
                pass.set_pipeline(&self.probe);
                pass.dispatch_workgroups((self.probe_count as u32).div_ceil(64), 1, 1);
                self.probe_samples += PROBE_PASS_SAMPLES;
                self.probe_dirty = true;
            }
            pass.set_pipeline(&self.present);
            pass.dispatch_workgroups(size[0].div_ceil(8), size[1].div_ceil(8), 1);
        }
        // Copy the point values out while no map is pending.
        let mut start_map = false;
        if self.probe_dirty && !self.map_in_flight {
            if let (Some(out), Some(staging)) = (&self.probe_out, &self.probe_staging) {
                encoder.copy_buffer_to_buffer(
                    out,
                    0,
                    staging,
                    0,
                    (self.probe_count.max(1) * 16) as u64,
                );
                start_map = true;
                self.probe_dirty = false;
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lightscene blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blit);
            pass.set_bind_group(0, self.blit_group.as_ref().unwrap(), &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        self.frame = self.frame.wrapping_add(1);

        if start_map {
            let done = self.map_done.clone();
            done.store(false, Ordering::Relaxed);
            self.map_in_flight = true;
            self.probe_staging.as_ref().unwrap().slice(..).map_async(
                wgpu::MapMode::Read,
                move |r| {
                    if r.is_ok() {
                        done.store(true, Ordering::Release);
                    }
                },
            );
        }
    }

    /// New measurement point values, if the asynchronous readback finished.
    /// Call once per frame after [`render`](Self::render).
    pub fn take_probe_values(&mut self, device: &wgpu::Device) -> Option<Vec<f32>> {
        if !self.map_in_flight {
            return None;
        }
        let _ = device.poll(wgpu::PollType::Poll);
        if !self.map_done.load(Ordering::Acquire) {
            return None;
        }
        self.map_in_flight = false;
        let staging = self.probe_staging.as_ref()?;
        let values = {
            let data = staging.slice(..).get_mapped_range();
            let raw: &[f32] = bytemuck::cast_slice(&data);
            raw.chunks(4)
                .take(self.probe_count)
                .map(|c| c[0])
                .collect::<Vec<f32>>()
        };
        staging.unmap();
        Some(values)
    }
}
