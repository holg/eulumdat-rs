//! wgpu compute pipeline for photon tracing.

use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

/// Flatten the exact eulumdat-spectrum 5 nm weighting tables into the GPU LUT
/// layout: 81 nodes × [X, Y, Z, scotopic], as `f32`.
fn spectral_lut_flat() -> Vec<f32> {
    use eulumdat_spectrum::lut::{CMF_X, CMF_Y, CMF_Z, LUT_LEN, V_SCOTOPIC};
    let mut out = Vec::with_capacity(LUT_LEN * 4);
    for i in 0..LUT_LEN {
        out.push(CMF_X[i] as f32);
        out.push(CMF_Y[i] as f32);
        out.push(CMF_Z[i] as f32);
        out.push(V_SCOTOPIC[i] as f32);
    }
    out
}

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
    pub num_primitives: u32,
    pub max_bounces: u32,
    pub rr_threshold: f32,
    pub cdf_g_steps: u32,
    pub cdf_c_steps: u32,
    pub cdf_g_max: f32,
    // Spectral mode flag + wavelength-CDF length. These occupy the two u32 slots
    // that previously padded area_center to its 16-byte boundary, so alignment
    // is unchanged.
    pub spectral_mode: u32,
    pub wl_cdf_len: u32,
    // Area source params (source_type=3)
    pub area_center: [f32; 3],
    pub _pad0: f32,
    pub area_normal: [f32; 3],
    pub _pad1: f32,
    pub area_u_axis: [f32; 3],
    pub area_half_width: f32,
    pub area_half_height: f32,
    pub _pad2: u32,
    pub _pad3: u32,
    pub _pad4: u32,
}

/// GPU primitive — matches GpuPrimitive in WGSL.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuPrimitive {
    pub ptype: u32,
    pub material_id: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub params: [f32; 12],
}

impl GpuPrimitive {
    /// Create a sheet primitive (matches CPU Primitive::Sheet).
    pub fn sheet(
        center: [f32; 3],
        normal: [f32; 3],
        u_axis: [f32; 3],
        half_width: f32,
        half_height: f32,
        thickness: f32,
        material_id: u32,
    ) -> Self {
        Self {
            ptype: 0, // PRIM_SHEET
            material_id,
            _pad0: 0,
            _pad1: 0,
            params: [
                center[0],
                center[1],
                center[2],
                normal[0],
                normal[1],
                normal[2],
                u_axis[0],
                u_axis[1],
                u_axis[2],
                half_width,
                half_height,
                thickness,
            ],
        }
    }
}

/// GPU material — matches GpuMaterial in WGSL.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuMaterial {
    pub mtype: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub reflectance: f32,
    pub ior: f32,
    pub transmittance: f32,
    pub min_reflectance: f32,
    pub absorption_coeff: f32,
    pub scattering_coeff: f32,
    pub asymmetry: f32,
    pub thickness: f32,
}

impl GpuMaterial {
    /// Convert from eulumdat-goniosim MaterialParams.
    pub fn from_material_params(params: &eulumdat_goniosim::MaterialParams) -> Self {
        use eulumdat_goniosim::Material;
        let mat = params.to_material();
        match mat {
            Material::Absorber => Self {
                mtype: 0,
                reflectance: 0.0,
                ior: 1.0,
                transmittance: 0.0,
                min_reflectance: 0.0,
                absorption_coeff: 0.0,
                scattering_coeff: 0.0,
                asymmetry: 0.0,
                thickness: 0.0,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
            Material::DiffuseReflector { reflectance } => Self {
                mtype: 1,
                reflectance: reflectance as f32,
                ior: 1.0,
                transmittance: 0.0,
                min_reflectance: 0.0,
                absorption_coeff: 0.0,
                scattering_coeff: 0.0,
                asymmetry: 0.0,
                thickness: 0.0,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
            Material::SpecularReflector { reflectance } => Self {
                mtype: 2,
                reflectance: reflectance as f32,
                ior: 1.0,
                transmittance: 0.0,
                min_reflectance: 0.0,
                absorption_coeff: 0.0,
                scattering_coeff: 0.0,
                asymmetry: 0.0,
                thickness: 0.0,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
            Material::ClearTransmitter {
                ior,
                transmittance,
                min_reflectance,
                ..
            } => Self {
                mtype: 4,
                reflectance: 0.0,
                ior: ior as f32,
                transmittance: transmittance as f32,
                min_reflectance: min_reflectance as f32,
                absorption_coeff: 0.0,
                scattering_coeff: 0.0,
                asymmetry: 0.0,
                thickness: 0.0,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
            Material::DiffuseTransmitter {
                ior,
                scattering_coeff,
                absorption_coeff,
                asymmetry,
                thickness,
                min_reflectance,
                ..
            } => Self {
                mtype: 5,
                reflectance: 0.0,
                ior: ior as f32,
                transmittance: 0.0,
                min_reflectance: min_reflectance as f32,
                absorption_coeff: absorption_coeff as f32,
                scattering_coeff: scattering_coeff as f32,
                asymmetry: asymmetry as f32,
                thickness: thickness as f32,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
            Material::MixedReflector {
                reflectance,
                specular_fraction: _,
            } => Self {
                mtype: 3,
                reflectance: reflectance as f32,
                ior: 1.0,
                transmittance: 0.0,
                min_reflectance: 0.0,
                absorption_coeff: 0.0,
                scattering_coeff: 0.0,
                asymmetry: 0.0,
                thickness: 0.0,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            },
        }
    }
}

/// Source type enum (matches WGSL switch).
#[derive(Clone, Copy, Debug)]
pub enum SourceType {
    Isotropic = 0,
    Lambertian = 1,
    FromLvk = 2,
    AreaSource = 3,
}

/// Angularly-integrated spectral channels from a GPU spectral trace: summed
/// tristimulus and scotopic weight over all detector bins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSpectralChannels {
    /// Summed CIE X.
    pub x: f64,
    /// Summed CIE Y (photopic).
    pub y: f64,
    /// Summed CIE Z.
    pub z: f64,
    /// Summed scotopic weight.
    pub scotopic: f64,
}

impl GpuSpectralChannels {
    /// Integrated correlated colour temperature (K), via the shared spectrum
    /// crate's chromaticity → CCT path. `None` if no light collected.
    pub fn cct_k(&self) -> Option<f64> {
        let sum = self.x + self.y + self.z;
        if sum <= 0.0 {
            return None;
        }
        Some(eulumdat_spectrum::colorimetry::from_chromaticity(self.x / sum, self.y / sum).cct_k)
    }

    /// Integrated scotopic/photopic ratio (Km′/Km · S/Y).
    pub fn sp_ratio(&self) -> f64 {
        if self.y <= 0.0 {
            return 0.0;
        }
        (eulumdat_spectrum::constants::KM_SCOTOPIC * self.scotopic)
            / (eulumdat_spectrum::constants::KM_PHOTOPIC * self.y)
    }
}

/// Result from a GPU trace — detector bins as f64, plus optional spectral
/// channels when the trace ran in spectral mode.
pub struct GpuDetectorResult {
    bins: Vec<Vec<f64>>,
    num_c: usize,
    num_g: usize,
    c_res: f64,
    g_res: f64,
    channels: Option<GpuSpectralChannels>,
}

impl GpuDetectorResult {
    /// Total detected energy.
    pub fn total_energy(&self) -> f64 {
        self.bins.iter().flat_map(|row| row.iter()).sum()
    }

    /// Get bins as `[c][g]` array.
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

    /// Integrated spectral channels, if the trace ran in spectral mode.
    pub fn channels(&self) -> Option<GpuSpectralChannels> {
        self.channels
    }

    /// Convert to candela (same formula as CPU detector).
    pub fn to_candela(&self, source_flux_lm: f64) -> Vec<Vec<f64>> {
        let total = self.total_energy();
        if total <= 0.0 {
            return self.bins.clone();
        }

        let flux_per_energy = source_flux_lm / total;
        let dc_rad = self.c_res.to_radians();

        let mut candela = vec![vec![0.0; self.num_g]; self.num_c];
        #[allow(clippy::needless_range_loop)]
        for ci in 0..self.num_c {
            for gi in 0..self.num_g {
                let g_rad = (gi as f64 * self.g_res).to_radians();
                let g_lo = (g_rad - self.g_res.to_radians() / 2.0).max(0.0);
                let g_hi = (g_rad + self.g_res.to_radians() / 2.0).min(std::f64::consts::PI);
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
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                // primitives: storage buffer (read)
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
                // materials: storage buffer (read)
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
                // cdf_data: storage buffer (read) for FromLvk source
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
                // wl_cdf: storage buffer (read) — source-spectrum wavelength CDF
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // channel_bins: storage buffer (read_write) — spectral channels
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // spectral_lut: storage buffer (read) — 81×vec4 weighting LUT
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
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
        self.trace(
            num_photons,
            c_res_deg,
            g_res_deg,
            SourceType::Isotropic,
            1000.0,
        )
        .await
    }

    /// Trace an isotropic source carrying a spectrum. Each photon's wavelength is
    /// sampled on the GPU from the SPD's CDF, and the weighted channel bins
    /// (X,Y,Z,scotopic) are accumulated. The returned result's
    /// [`GpuDetectorResult::channels`] gives the integrated CCT and S/P — the
    /// GPU analogue of the CPU `WeightedChannels` path, for parity validation.
    pub async fn trace_isotropic_spectral(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        spd: &eulumdat_spectrum::Spd,
    ) -> GpuDetectorResult {
        let num_c = (360.0 / c_res_deg).round() as u32;
        let num_g = (180.0 / g_res_deg).round() as u32 + 1;

        // Build the flattened wavelength CDF: [w0,c0,w1,c1,…].
        let wls = spd.wavelengths();
        let vals = spd.values();
        let mut wl_cdf: Vec<f32> = Vec::with_capacity(wls.len() * 2);
        let mut cum = 0.0f64;
        let mut cdf_vals = vec![0.0f64; wls.len()];
        for i in 1..wls.len() {
            cum += 0.5 * (vals[i - 1] + vals[i]) * (wls[i] - wls[i - 1]);
            cdf_vals[i] = cum;
        }
        if cum > 0.0 {
            for c in &mut cdf_vals {
                *c /= cum;
            }
        }
        for i in 0..wls.len() {
            wl_cdf.push(wls[i] as f32);
            wl_cdf.push(cdf_vals[i] as f32);
        }

        let config = GpuTracerConfig {
            detector_c_bins: num_c,
            detector_g_bins: num_g,
            detector_c_res: c_res_deg,
            detector_g_res: g_res_deg,
            seed_offset: 42,
            num_photons,
            source_type: SourceType::Isotropic as u32,
            source_flux: 1000.0,
            num_primitives: 0,
            max_bounces: 4,
            rr_threshold: 0.01,
            cdf_g_steps: 0,
            cdf_c_steps: 0,
            cdf_g_max: 0.0,
            spectral_mode: 1,
            wl_cdf_len: wls.len() as u32,
            area_center: [0.0; 3],
            _pad0: 0.0,
            area_normal: [0.0, 0.0, -1.0],
            _pad1: 0.0,
            area_u_axis: [1.0, 0.0, 0.0],
            area_half_width: 0.0,
            area_half_height: 0.0,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
        };

        self.dispatch_config(config, num_c, num_g, &[], &[], &[], &wl_cdf)
            .await
    }

    /// Trace photons from a Lambertian source in free space.
    pub async fn trace_lambertian(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
    ) -> GpuDetectorResult {
        self.trace(
            num_photons,
            c_res_deg,
            g_res_deg,
            SourceType::Lambertian,
            1000.0,
        )
        .await
    }

    /// Trace with scene geometry and materials.
    #[allow(clippy::too_many_arguments)]
    pub async fn trace_with_scene(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        source_type: SourceType,
        source_flux: f32,
        primitives: &[GpuPrimitive],
        materials: &[GpuMaterial],
    ) -> GpuDetectorResult {
        self.trace_inner(
            num_photons,
            c_res_deg,
            g_res_deg,
            source_type,
            source_flux,
            primitives,
            materials,
            &[],
            0,
            0,
            0.0,
            50,
            0.01,
        )
        .await
    }

    /// Trace from a rectangular diffuse area source in free space.
    #[allow(clippy::too_many_arguments)]
    pub async fn trace_area_source(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        source_flux: f32,
        center: [f32; 3],
        normal: [f32; 3],
        u_axis: [f32; 3],
        half_width: f32,
        half_height: f32,
    ) -> GpuDetectorResult {
        let num_c = (360.0 / c_res_deg).round() as u32;
        let num_g = (180.0 / g_res_deg).round() as u32 + 1;

        let config = GpuTracerConfig {
            detector_c_bins: num_c,
            detector_g_bins: num_g,
            detector_c_res: c_res_deg,
            detector_g_res: g_res_deg,
            seed_offset: 42,
            num_photons,
            source_type: SourceType::AreaSource as u32,
            source_flux,
            num_primitives: 0,
            max_bounces: 50,
            rr_threshold: 0.01,
            cdf_g_steps: 0,
            cdf_c_steps: 0,
            cdf_g_max: 0.0,
            spectral_mode: 0,
            wl_cdf_len: 0,
            area_center: center,
            _pad0: 0.0,
            area_normal: normal,
            _pad1: 0.0,
            area_u_axis: u_axis,
            area_half_width: half_width,
            area_half_height: half_height,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
        };

        self.dispatch_config(config, num_c, num_g, &[], &[], &[], &[])
            .await
    }

    /// Trace from an LDT source (FromLvk) with optional cover geometry.
    #[allow(clippy::too_many_arguments)]
    pub async fn trace_from_lvk(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        source_flux: f32,
        cdf: &eulumdat_goniosim::source::LvkCdf,
        primitives: &[GpuPrimitive],
        materials: &[GpuMaterial],
    ) -> GpuDetectorResult {
        // Flatten CDF data: marginal_g (g_steps) + conditional_c (g_steps * c_steps)
        let g_steps = cdf.g_steps;
        let c_steps = cdf.c_steps;
        let mut cdf_flat = Vec::with_capacity(g_steps + g_steps * c_steps);
        // Marginal CDF
        for v in &cdf.marginal_g {
            cdf_flat.push(*v as f32);
        }
        // Conditional CDFs (flattened)
        for row in &cdf.conditional_c {
            for v in row {
                cdf_flat.push(*v as f32);
            }
        }

        self.trace_inner(
            num_photons,
            c_res_deg,
            g_res_deg,
            SourceType::FromLvk,
            source_flux,
            primitives,
            materials,
            &cdf_flat,
            g_steps as u32,
            c_steps as u32,
            cdf.g_max as f32,
            50,
            0.01,
        )
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
        self.trace_inner(
            num_photons,
            c_res_deg,
            g_res_deg,
            source_type,
            source_flux,
            &[],
            &[],
            &[],
            0,
            0,
            0.0,
            1,
            0.01,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn trace_inner(
        &self,
        num_photons: u32,
        c_res_deg: f32,
        g_res_deg: f32,
        source_type: SourceType,
        source_flux: f32,
        primitives_data: &[GpuPrimitive],
        materials_data: &[GpuMaterial],
        cdf_data: &[f32],
        cdf_g_steps: u32,
        cdf_c_steps: u32,
        cdf_g_max: f32,
        max_bounces: u32,
        rr_threshold: f32,
    ) -> GpuDetectorResult {
        let num_c = (360.0 / c_res_deg).round() as u32;
        let num_g = (180.0 / g_res_deg).round() as u32 + 1;

        let config = GpuTracerConfig {
            detector_c_bins: num_c,
            detector_g_bins: num_g,
            detector_c_res: c_res_deg,
            detector_g_res: g_res_deg,
            seed_offset: 42,
            num_photons,
            source_type: source_type as u32,
            source_flux,
            num_primitives: primitives_data.len() as u32,
            max_bounces,
            rr_threshold,
            cdf_g_steps,
            cdf_c_steps,
            cdf_g_max,
            spectral_mode: 0,
            wl_cdf_len: 0,
            area_center: [0.0; 3],
            _pad0: 0.0,
            area_normal: [0.0, 0.0, -1.0],
            _pad1: 0.0,
            area_u_axis: [1.0, 0.0, 0.0],
            area_half_width: 0.0,
            area_half_height: 0.0,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
        };

        self.dispatch_config(
            config,
            num_c,
            num_g,
            primitives_data,
            materials_data,
            cdf_data,
            &[],
        )
        .await
    }

    /// Core dispatch: creates GPU buffers, runs compute, reads back results.
    ///
    /// `wl_cdf` is the flattened source-spectrum wavelength CDF ([w,c,w,c,…]);
    /// pass `&[]` for a monochromatic run. When `config.spectral_mode == 1`, the
    /// weighted channel bins (X,Y,Z,scotopic) are read back into
    /// [`GpuDetectorResult::channels`].
    async fn dispatch_config(
        &self,
        config: GpuTracerConfig,
        num_c: u32,
        num_g: u32,
        primitives_data: &[GpuPrimitive],
        materials_data: &[GpuMaterial],
        cdf_data: &[f32],
        wl_cdf: &[f32],
    ) -> GpuDetectorResult {
        let total_bins = num_c * num_g;
        let num_photons = config.num_photons;
        let c_res_deg = config.detector_c_res;
        let g_res_deg = config.detector_g_res;

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

        // Primitive + material buffers (need at least 1 element for wgpu)
        let dummy_prim = GpuPrimitive {
            ptype: 0,
            material_id: 0,
            _pad0: 0,
            _pad1: 0,
            params: [0.0; 12],
        };
        let dummy_mat = GpuMaterial {
            mtype: 0,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            reflectance: 0.0,
            ior: 1.0,
            transmittance: 0.0,
            min_reflectance: 0.0,
            absorption_coeff: 0.0,
            scattering_coeff: 0.0,
            asymmetry: 0.0,
            thickness: 0.0,
        };

        let prim_buf_data: Vec<GpuPrimitive> = if primitives_data.is_empty() {
            vec![dummy_prim]
        } else {
            primitives_data.to_vec()
        };
        let mat_buf_data: Vec<GpuMaterial> = if materials_data.is_empty() {
            vec![dummy_mat]
        } else {
            materials_data.to_vec()
        };

        let primitives_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("primitives_buffer"),
                contents: bytemuck::cast_slice(&prim_buf_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let materials_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("materials_buffer"),
                contents: bytemuck::cast_slice(&mat_buf_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // CDF buffer
        let cdf_buf_data: Vec<f32> = if cdf_data.is_empty() {
            vec![0.0]
        } else {
            cdf_data.to_vec()
        };
        let cdf_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cdf_buffer"),
                contents: bytemuck::cast_slice(&cdf_buf_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // Wavelength CDF buffer (dummy single element when non-spectral).
        let wl_buf_data: Vec<f32> = if wl_cdf.is_empty() {
            vec![0.0, 0.0]
        } else {
            wl_cdf.to_vec()
        };
        let wl_cdf_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wl_cdf_buffer"),
                contents: bytemuck::cast_slice(&wl_buf_data),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // Channel bins: 4 atomics (X,Y,Z,scotopic) per direction bin.
        let spectral = config.spectral_mode == 1;
        let channel_len = if spectral {
            (total_bins as u64) * 4
        } else {
            1
        };
        let channel_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("channel_buffer"),
            size: channel_len * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let channel_readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("channel_readback"),
            size: channel_len * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Spectral LUT: 81 nodes × [X, Y, Z, scotopic] at 5 nm, from the exact
        // eulumdat-spectrum tables so GPU colorimetry matches the CPU.
        let lut_data = spectral_lut_flat();
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spectral_lut_buffer"),
                contents: bytemuck::cast_slice(&lut_data),
                usage: wgpu::BufferUsages::STORAGE,
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: primitives_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: materials_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: cdf_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wl_cdf_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: channel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute
        let workgroup_size = 256u32;
        let num_workgroups = num_photons.div_ceil(workgroup_size);

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
        if spectral {
            encoder.copy_buffer_to_buffer(
                &channel_buffer,
                0,
                &channel_readback,
                0,
                channel_len * 4,
            );
        }

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
        #[allow(clippy::needless_range_loop)]
        for ci in 0..num_c as usize {
            for gi in 0..num_g as usize {
                let idx = ci * num_g as usize + gi;
                bins[ci][gi] = raw_bins[idx] as f64 / 1_000.0;
            }
        }

        drop(data);
        readback_buffer.unmap();

        // Read back the spectral channels, if any.
        let channels = if spectral {
            let slice = channel_readback.slice(..);
            let (tx, rx) = flume::bounded(1);
            slice.map_async(wgpu::MapMode::Read, move |r| {
                tx.send(r).unwrap();
            });
            self.device.poll(wgpu::PollType::wait_indefinitely()).ok();
            rx.recv_async().await.unwrap().unwrap();
            let cdata = slice.get_mapped_range();
            let raw: &[u32] = bytemuck::cast_slice(&cdata);
            let mut agg = [0.0f64; 4];
            let n = (total_bins as usize) * 4;
            for i in 0..n {
                agg[i % 4] += raw[i] as f64 / 1_000.0;
            }
            drop(cdata);
            channel_readback.unmap();
            Some(GpuSpectralChannels {
                x: agg[0],
                y: agg[1],
                z: agg[2],
                scotopic: agg[3],
            })
        } else {
            None
        };

        GpuDetectorResult {
            bins,
            num_c: num_c as usize,
            num_g: num_g as usize,
            c_res: c_res_deg as f64,
            g_res: g_res_deg as f64,
            channels,
        }
    }
}
