# eulumdat-rt — GPU Ray Tracing Engine for Photometric Simulation

## Vision

A physically correct Monte Carlo photon tracer running on the GPU via wgpu
compute shaders. Produces photometric measurement data (candela, lux, LDT/IES
files) — not screen pixels. Validated against the CPU reference implementation
(`eulumdat-goniosim`) and CIE 171:2006.

Neither Unreal nor Unity can output physically correct photometric data from
ray tracing. Relux, DIALux, and AGi32 use render engines they don't control.
We build the engine AND the physics.

> "The only ray tracer that outputs an LDT file."

## Why Not Use Existing Ray Tracers

### Standard Bevy (rasterization)
- Shadow maps, screen-space reflections, ambient occlusion — all faked
- Fast (60fps) but physically wrong for measurement
- No concept of candela, lux, or photometric output

### Unity / Unreal
- Baked lightmaps (pre-computed, static only) + optional hardware RT
- Unreal Lumen: software RT hybrid — clever but approximate, temporal artifacts
- Both output PICTURES, not MEASUREMENTS — no LDT/IES export possible
- Can't access or modify the core rendering pipeline

### Bevy Solari (bevy_solari)
- Real-time path tracing with ReSTIR DI/GI by JMS55
- First-party Bevy crate at `crates/bevy_solari/`
- **Requires hardware RT** (NVIDIA RTX, `EXPERIMENTAL_RAY_QUERY`)
- Uses `StandardMaterial` — game PBR, not physical optics
- Uses hardware TLAS/BLAS for acceleration — no software fallback
- Denoises and temporally accumulates for visual quality
- Hooks into Bevy's deferred GBuffer — outputs screen pixels
- No macOS/Metal support (hardware RT not available)
- Goal: make Bevy games look as good as Unreal RTX

### Mitsuba / LuxCoreRender (offline)
- Physically correct spectral rendering — the gold standard
- Offline only — minutes to hours per image
- C++ codebases, not embeddable in Rust/WASM
- No real-time interaction, no live parameter adjustment

### eulumdat-rt (ours)
- Monte Carlo photon tracing for MEASUREMENT, not visuals
- **No hardware RT required** — pure compute shaders, runs on any GPU
- Works on Metal (macOS/iOS), Vulkan, DX12, and WebGPU (browser)
- Own material system: IOR, transmittance, diffusion (not game PBR)
- Own software ray intersection (no TLAS/BLAS dependency)
- No denoising — we want raw statistical results
- No temporal tricks — each trace is independent, deterministic
- Output: detector bins → candela → LDT/IES files
- Validates against CIE 171:2006, not "does it look good"

### Comparison Table

|                        | Bevy std | Unity/Unreal | Solari    | Mitsuba   | eulumdat-rt |
|------------------------|----------|-------------|-----------|-----------|-------------|
| Method                 | Raster   | Raster+bake | HW RT     | Offline MC | GPU compute |
| Physically correct     | No       | No          | Approx    | Yes       | Yes         |
| Real-time              | Yes      | Yes         | Yes       | No        | Yes (target)|
| Hardware RT required   | No       | Optional    | **Yes**   | No        | **No**      |
| macOS/Metal            | Yes      | Yes         | **No**    | Yes       | Yes         |
| WebGPU/WASM            | Yes      | No          | No        | No        | **Yes**     |
| Custom materials       | Limited  | Yes         | No (StdMat)| Yes      | Yes         |
| Output photometric data| No       | No          | No        | Possible  | **Yes**     |
| Outputs LDT/IES        | No       | No          | No        | No        | **Yes**     |
| CIE validated          | N/A      | N/A         | N/A       | Partial   | **Yes**     |

The fundamental difference: Solari asks "does the image look physically
plausible?" We ask "is the photometric measurement correct to within 0.1%?"

## Architecture

Bevy plugin crate — own compute pipeline, own material type,
Bevy ECS for entity management, windowing, input, asset loading.
Inspired by Solari's plugin pattern but fundamentally different in purpose:
Solari writes pixels to the screen, we write candela to detector bins.

```
┌─────────────────────────────────────────────────────┐
│  Bevy App                                           │
│  ├── ECS: entities, components, systems             │
│  ├── Windowing, input, asset loading                │
│  └── Standard rendering (for visualization only)    │
├─────────────────────────────────────────────────────┤
│  eulumdat-rt (plugin)                               │
│  ├── RtPlugin: registers systems + resources        │
│  ├── Compute Pipeline (wgpu)                        │
│  │   ├── trace.wgsl     — photon tracing kernel     │
│  │   ├── material.wgsl  — Fresnel, Snell, HG, BL   │
│  │   ├── intersect.wgsl — ray-primitive intersection │
│  │   └── detect.wgsl    — detector binning (atomics) │
│  ├── GPU Buffers                                    │
│  │   ├── Scene geometry (primitives)                │
│  │   ├── Material params                            │
│  │   ├── Photon state (position, direction, energy) │
│  │   ├── Detector bins (atomic u32 array)           │
│  │   └── RNG state (per-workgroup seeds)            │
│  └── Readback + Export                              │
│      ├── Detector bins → Eulumdat → .ldt            │
│      └── Progress reporting to ECS                  │
├─────────────────────────────────────────────────────┤
│  eulumdat-goniosim (CPU reference — validation)     │
│  eulumdat (core — parsing, export, diagrams)        │
└─────────────────────────────────────────────────────┘
```

## Why Not Bevy's Renderer

Bevy's PBR pipeline is designed for game visuals:
- Approximates light for 60fps (screen-space effects, LOD, temporal accumulation)
- Materials output colors (sRGB), not physical quantities
- No energy conservation guarantee (close enough for visuals, not for measurement)
- Can't output photometric data (candela per solid angle)
- Review process blocks changes to core rendering code

Our engine:
- Exact energy conservation (validated to <0.1%)
- Materials output absorption/transmission coefficients
- Output is candela, lux, LDT/IES files
- Deterministic, reproducible (seeded RNG)
- Spectral-ready (wavelength parameter on every photon)
- We own every line — no PRs needed

## Crate Design

```
crates/eulumdat-rt/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Bevy plugin + public API
│   ├── plugin.rs        # RtPlugin: system registration
│   ├── components.rs    # ECS components (RtScene, RtMaterial, RtDetector)
│   ├── resources.rs     # ECS resources (RtPipeline, RtConfig, RtResult)
│   ├── pipeline.rs      # wgpu compute pipeline setup
│   ├── buffers.rs       # GPU buffer creation + upload
│   ├── dispatch.rs      # Compute shader dispatch + readback
│   ├── readback.rs      # Detector bins → Eulumdat
│   └── shaders/
│       ├── trace.wgsl       # Main tracing kernel
│       ├── material.wgsl    # Material evaluation
│       ├── intersect.wgsl   # Ray-primitive intersection
│       ├── detect.wgsl      # Detector binning
│       └── random.wgsl      # PCG/xoshiro RNG
```

### Dependencies

```toml
[dependencies]
bevy = { version = "0.19", default-features = false, features = [
    "bevy_render",   # for wgpu access via RenderDevice/RenderQueue
    "bevy_app",      # for Plugin trait
    "bevy_ecs",      # for systems, resources, components
] }
eulumdat = { workspace = true }
eulumdat-goniosim = { workspace = true }  # shared types + CPU fallback
bytemuck = "1.14"                          # GPU buffer casting
```

No `bevy_pbr`, no `bevy_core_pipeline`. We don't use Bevy's rendering
pipeline at all — only its wgpu device access.

## Compute Shader Architecture

### Workgroup Layout

```
dispatch(num_workgroups, 1, 1)
  each workgroup: 256 invocations
  each invocation: traces 1 photon

total photons per dispatch = num_workgroups * 256
```

Progressive rendering: dispatch repeatedly, accumulating in the detector
buffer. UI shows live-updating LVK between dispatches.

### GPU Buffers

```wgsl
// Scene geometry — flat array of primitives
struct Primitive {
    ptype: u32,           // 0=plane, 1=box, 2=cylinder, 3=sheet
    material_id: u32,
    // 14 floats for geometry params (position, normal, dimensions)
    params: array<f32, 14>,
}

// Materials — flat array
struct Material {
    mtype: u32,           // 0=absorber, 1=diffuse_refl, 2=specular, 3=clear_trans, 4=diffuse_trans
    reflectance: f32,
    ior: f32,
    transmittance: f32,
    scattering_coeff: f32,
    absorption_coeff: f32,
    asymmetry: f32,       // HG g parameter
    thickness: f32,
    min_reflectance: f32,
    _padding: array<f32, 3>,
}

// Detector bins — 2D array, atomic u32 for parallel writes
// Energy stored as fixed-point: value * 1000000 as u32
@group(0) @binding(0) var<storage, read_write> detector_bins: array<atomic<u32>>;

// Scene data
@group(0) @binding(1) var<storage, read> primitives: array<Primitive>;
@group(0) @binding(2) var<storage, read> materials: array<Material>;

// Config
@group(0) @binding(3) var<uniform> config: TraceConfig;

// RNG state — one per invocation
@group(0) @binding(4) var<storage, read_write> rng_state: array<u32>;

struct TraceConfig {
    num_primitives: u32,
    num_materials: u32,
    detector_c_bins: u32,
    detector_g_bins: u32,
    detector_c_res: f32,
    detector_g_res: f32,
    max_bounces: u32,
    rr_threshold: f32,    // Russian roulette
    source_flux: f32,
    seed_offset: u32,     // changes per dispatch for different random paths
}
```

### Main Tracing Kernel (trace.wgsl)

```wgsl
@compute @workgroup_size(256)
fn trace_photons(@builtin(global_invocation_id) id: vec3<u32>) {
    let photon_idx = id.x;

    // Initialize RNG from seed
    var rng = init_rng(photon_idx, config.seed_offset);

    // Sample source direction (from LVK CDF — stored in separate buffer)
    var ray = sample_source(&rng);
    var energy = 1.0;

    // Trace loop
    for (var bounce = 0u; bounce < config.max_bounces; bounce++) {
        // Find nearest intersection
        let hit = intersect_scene(ray);

        if (!hit.valid) {
            // Escaped — record on detector
            record_detector(ray.direction, energy);
            break;
        }

        // Material interaction
        let mat = materials[hit.material_id];
        let interaction = evaluate_material(mat, ray, hit, &rng);

        switch (interaction.type) {
            case ABSORBED: { break; }
            case REFLECTED: {
                ray = interaction.new_ray;
                energy *= interaction.attenuation;
            }
            case TRANSMITTED: {
                ray = interaction.new_ray;
                energy *= interaction.attenuation;
            }
        }

        // Russian roulette
        if (energy < config.rr_threshold) {
            let survive = energy / config.rr_threshold;
            if (random_f32(&rng) > survive) { break; }
            energy = config.rr_threshold;
        }
    }
}
```

### Detector Binning (detect.wgsl)

```wgsl
fn record_detector(direction: vec3<f32>, energy: f32) {
    // Convert direction to (C, gamma) — same convention as CPU
    let gamma = acos(-direction.z);  // 0 = nadir
    let c = atan2(direction.y, direction.x);
    let c_pos = select(c + 2.0 * PI, c, c >= 0.0);

    let ci = u32(c_pos / config.detector_c_res) % config.detector_c_bins;
    let gi = min(u32(gamma / config.detector_g_res + 0.5), config.detector_g_bins - 1);

    let bin_idx = ci * config.detector_g_bins + gi;

    // Atomic add — energy as fixed-point integer (multiply by 1M)
    let energy_fixed = u32(energy * 1000000.0);
    atomicAdd(&detector_bins[bin_idx], energy_fixed);
}
```

### Material Evaluation (material.wgsl)

Port of `eulumdat-goniosim/src/material.rs` to WGSL:

- `fresnel_schlick(cos_theta, eta_ratio) -> f32`
- `reflect(incoming, normal) -> vec3<f32>`
- `refract(incoming, normal, eta) -> vec3<f32>` (with TIR check)
- `random_cosine_hemisphere(normal, rng) -> vec3<f32>`
- `sample_henyey_greenstein(incoming, g, rng) -> vec3<f32>`
- `evaluate_material(mat, ray, hit, rng) -> Interaction`

The thin-sheet model from the CPU version translates directly to WGSL.

## Bevy Integration

### Plugin

```rust
pub struct RtPlugin;

impl Plugin for RtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RtConfig>()
            .init_resource::<RtResult>()
            .add_systems(Update, (
                setup_pipeline,
                dispatch_trace,
                readback_detector,
                update_visualization,
            ).chain());
    }
}
```

### Systems

```rust
/// Creates the compute pipeline and GPU buffers when scene changes.
fn setup_pipeline(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    scene: Res<RtScene>,
    mut pipeline: ResMut<RtPipeline>,
) { ... }

/// Dispatches one batch of photons per frame.
fn dispatch_trace(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<RtPipeline>,
    config: Res<RtConfig>,
    mut result: ResMut<RtResult>,
) {
    // dispatch compute shader
    // increment photon count
    // schedule readback if enough photons accumulated
}

/// Reads detector bins from GPU and converts to Eulumdat.
fn readback_detector(
    render_device: Res<RenderDevice>,
    pipeline: Res<RtPipeline>,
    mut result: ResMut<RtResult>,
) {
    // map readback buffer
    // convert fixed-point bins to f64
    // build Eulumdat via detector_to_eulumdat()
    // update result.ldt, result.svg
}

/// Updates the visualization (polar diagram overlay, detector heatmap).
fn update_visualization(
    result: Res<RtResult>,
    // ... update UI/overlay
) { ... }
```

### Components

```rust
/// Tag component: this entity is the photon source (has LDT data).
#[derive(Component)]
pub struct RtSource {
    pub eulumdat: Eulumdat,
}

/// Tag component: this entity is a cover/optical element.
#[derive(Component)]
pub struct RtCover {
    pub material: MaterialParams,
    pub distance_mm: f64,
}

/// Tag component: this entity is the virtual detector sphere.
#[derive(Component)]
pub struct RtDetector {
    pub c_resolution_deg: f64,
    pub g_resolution_deg: f64,
}
```

## Performance Targets

| Metric | CPU (goniosim) | GPU (eulumdat-rt) | Speedup |
|--------|---------------|-------------------|---------|
| Free space (no geometry) | 5M photons/s | 500M photons/s | 100x |
| Simple housing | 1M photons/s | 100M photons/s | 100x |
| Housing + clear PMMA | 500k photons/s | 50M photons/s | 100x |
| Housing + opal PMMA | 100k photons/s | 10M photons/s | 100x |
| Time for 10M photons | 2-100s | 0.02-1s | 100x |

With 100x speedup, real-time interactive exploration becomes possible:
user moves a slider → GPU retraces in <100ms → LVK updates live.

## Validation

Same CIE 171:2006 test cases as the CPU tracer:

```rust
let cpu_result = goniosim::Tracer::trace(&scene, &config);
let gpu_result = rt::GpuTracer::trace(&scene, &config);

let cpu_ldt = detector_to_eulumdat(&cpu_result.detector, flux, &export_config);
let gpu_ldt = detector_to_eulumdat(&gpu_result.detector, flux, &export_config);

let cmp = PhotometricComparison::from_eulumdat(&cpu_ldt, &gpu_ldt, "CPU", "GPU");
assert!(cmp.similarity_score > 0.99);
```

Both backends must produce the same LVK within statistical noise at
sufficient photon count. If they diverge, the GPU shader has a bug.

## Phased Development

### Phase 1: Compute Pipeline MVP

- [ ] wgpu compute pipeline setup via Bevy RenderDevice
- [ ] WGSL port of ray-primitive intersection (plane, sheet)
- [ ] WGSL port of material evaluation (absorber, diffuse reflector)
- [ ] Detector binning with atomic adds
- [ ] Readback and conversion to Eulumdat
- [ ] Validate isotropic free-space test case against CPU

### Phase 2: Full Material Support

- [ ] Fresnel equations in WGSL
- [ ] Clear transmitter (Snell + Beer-Lambert)
- [ ] Diffuse transmitter (thin-sheet model)
- [ ] Mixed reflector
- [ ] CDF-based FromLvk source sampling on GPU
- [ ] Full CIE 171:2006 test suite on GPU

### Phase 3: Visualization

- [ ] Photon trail rendering (instanced line segments)
- [ ] Detector sphere heatmap (vertex colors)
- [ ] Live polar diagram overlay
- [ ] Side-by-side CPU vs GPU comparison

### Phase 4: Advanced Features

- [ ] BVH for complex geometry (mesh import)
- [ ] Spectral rendering (wavelength-dependent materials)
- [ ] Multi-source arrays (LED matrix)
- [ ] WASM/WebGPU support (runs in browser)

## Relationship to Solari

Solari (`bevy_solari`, by JMS55) is Bevy's first-party hardware ray tracing
system. It's the closest architectural reference for a Bevy plugin that runs
its own compute pipeline, but there are fundamental differences:

**What we share with Solari:**
- Bevy plugin pattern — registers systems, uses RenderDevice/RenderQueue
- Pure compute shaders — no render graph involvement
- Own material extraction pipeline

**Where we diverge:**
- Solari requires hardware RT (RTX) — we use software ray intersection
- Solari reads Bevy's deferred GBuffer — we don't render to screen
- Solari uses `StandardMaterial` — we use `MaterialParams` (physical optics)
- Solari denoises for visual quality — we preserve raw statistical data
- Solari outputs pixels — we output photometric measurements
- Solari is desktop-only (no macOS) — we target Metal + WebGPU too

**Architectural lesson from Solari:**
The key insight is that you CAN build a complete ray tracing system as a
Bevy plugin without touching core. Solari proves the pattern works.
We follow it — but for measurement, not visuals.
