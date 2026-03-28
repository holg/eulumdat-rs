# eulumdat-goniosim — CPU Monte Carlo Photon Tracer

## Purpose

Pure Rust crate for Monte Carlo photon tracing through luminaire geometry.
No rendering, no Bevy, no GPU — just physics. This is the **reference implementation**
that produces numerically correct results. Any future GPU tracer must validate against it.

> The CPU tracer is the source of truth. It is slow but obviously correct.

## Crate Design

```
crates/eulumdat-goniosim/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API surface
    ├── ray.rs              # Ray, HitRecord, geometric primitives
    ├── source.rs           # Photon emission models
    ├── geometry.rs         # Scene primitives + ray intersection
    ├── material.rs         # MaterialParams (user-facing) + Material enum (internal)
    ├── catalog.rs          # Material catalog (PMMA, glass, aluminum, paint presets)
    ├── tracer.rs           # Monte Carlo loop + Russian roulette
    ├── detector.rs         # Spherical goniophotometer binning
    ├── scene.rs            # Scene + SceneBuilder (distance-based placement)
    ├── export.rs           # Detector → Eulumdat struct → .ldt/.ies
    └── presets.rs          # Ready-made scenes (bare LED, housing, Lichtsaeule)
```

### Dependencies

```toml
[dependencies]
eulumdat = { path = "../eulumdat" }    # LDT/IES export, Eulumdat struct
nalgebra = "0.33"                       # Vec3, transforms, rotations
rand = "0.9"                            # RNG for Monte Carlo sampling
rand_xoshiro = "0.7"                    # Fast, reproducible PRNG
rayon = { version = "1.10", optional = true }

[features]
default = ["parallel"]
parallel = ["rayon"]                    # Multi-threaded tracing
serde = ["dep:serde", "eulumdat/serde"] # Serializable scene descriptions
```

No `wgpu`, no `bevy`, no `web-sys`. Runs on any platform Rust compiles to.

## Core Types

### Ray

```rust
pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Unit<Vector3<f64>>,
}

pub struct HitRecord {
    pub t: f64,                          // distance along ray
    pub point: Point3<f64>,              // hit position
    pub normal: Unit<Vector3<f64>>,      // outward surface normal
    pub front_face: bool,                // ray hit outside (true) or inside (false)
    pub material: MaterialId,
}
```

All geometry in **meters**. Photometric angles in **degrees** at the API boundary,
radians internally.

### Photon

```rust
pub struct Photon {
    pub ray: Ray,
    pub energy: f64,      // relative energy, starts at 1.0, reduced by absorption
    pub wavelength: f64,  // nm (for future spectral rendering, default 555nm)
    pub bounces: u32,
}
```

Energy is tracked multiplicatively. After each interaction the photon's energy is
multiplied by the material's reflectance/transmittance. Russian roulette terminates
low-energy photons probabilistically to avoid bias.

## Modules

### 1. Source Models (`source.rs`)

Each source produces photons: an origin point and a direction sampled from a
distribution, plus total luminous flux for normalization.

```rust
pub enum Source {
    /// Uniform emission in all directions (4pi steradians)
    Isotropic {
        position: Point3<f64>,
        flux_lm: f64,
    },

    /// Cosine-weighted hemisphere (ideal diffuse emitter)
    Lambertian {
        position: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        flux_lm: f64,
    },

    /// Directional LED with beam angle
    Led {
        position: Point3<f64>,
        direction: Unit<Vector3<f64>>,
        half_angle_deg: f64,              // e.g. 60 for 120deg beam
        flux_lm: f64,
    },

    /// Line source (LED strip) — samples random point along segment
    LineSource {
        start: Point3<f64>,
        end: Point3<f64>,
        normal: Unit<Vector3<f64>>,       // emission hemisphere direction
        half_angle_deg: f64,
        flux_lm: f64,
    },

    /// Emit according to an existing LDT/IES distribution (for validation)
    FromLvk {
        position: Point3<f64>,
        orientation: Rotation3<f64>,      // maps LVK C0/gamma0 to world
        eulumdat: Box<Eulumdat>,
        flux_lm: f64,
    },
}
```

The `FromLvk` source uses `eulumdat.sample(c_angle, g_angle)` to importance-sample
directions proportional to the stored intensity distribution. This allows round-trip
validation: trace an LDT through empty space, collect on detector, export — must
reproduce the input LVK within statistical noise.

### 2. Scene Geometry (`geometry.rs`)

Simple analytic primitives with exact ray intersection. No mesh/BVH needed for
typical luminaire scenes (< 20 primitives).

```rust
pub enum Primitive {
    /// Infinite plane (one-sided)
    Plane {
        point: Point3<f64>,
        normal: Unit<Vector3<f64>>,
    },

    /// Axis-aligned box
    Box {
        min: Point3<f64>,
        max: Point3<f64>,
    },

    /// Cylinder (capped or open)
    Cylinder {
        center: Point3<f64>,
        axis: Unit<Vector3<f64>>,
        radius: f64,
        half_height: f64,
        capped: bool,
    },

    /// Flat rectangular sheet (finite plane, for covers/panels)
    Sheet {
        center: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        u_axis: Unit<Vector3<f64>>,       // local X direction
        half_width: f64,                   // extent along u_axis
        half_height: f64,                  // extent along (normal x u_axis)
        thickness: f64,                    // for volume scattering (e.g. 3mm PMMA)
    },

    /// L-profile (two sheets at 90deg — for Lichtsaeule corner profile)
    LProfile {
        corner: Point3<f64>,
        axis: Unit<Vector3<f64>>,          // extrusion direction
        leg_a_dir: Unit<Vector3<f64>>,     // direction of first leg
        leg_a_length: f64,
        leg_b_length: f64,
        height: f64,                       // extrusion length
        thickness: f64,
    },
}

pub struct SceneObject {
    pub primitive: Primitive,
    pub material: MaterialId,
    pub label: String,         // e.g. "PMMA opal 3mm cover" (from MaterialParams.name)
}
```

The role (reflector/transmitter/absorber) is implicit from `MaterialParams`:
transparent materials (`transmittance_pct > 0`) are transmitters, opaque materials
with low reflectance (`< 2%`) are absorbers, the rest are reflectors.

Each primitive implements `fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>`.

### 3. Material System (`material.rs`)

Two-layer design: a **user-facing** `MaterialParams` using datasheet values, and an
**internal** `Material` enum with physics coefficients. Users work with `MaterialParams`;
the tracer converts them to `Material` automatically.

#### User-Facing: MaterialParams

These are the values you find on a material datasheet. A lighting designer can fill
these in without understanding Monte Carlo internals.

```rust
/// Material description using manufacturer datasheet values.
pub struct MaterialParams {
    /// Human-readable name, e.g. "PMMA opal 3mm"
    pub name: String,

    /// Reflexionsgrad [%], 0-100
    /// How much light is reflected at the surface.
    /// For opaque materials: total reflectance (diffuse + specular combined).
    /// For transparent materials: Fresnel reflectance is computed from IOR,
    /// this field is ignored (set to 0).
    pub reflectance_pct: f64,

    /// Brechungsindex (index of refraction)
    /// How much light bends when entering the material.
    /// PMMA: 1.49, glass: 1.52, polycarbonate: 1.585
    /// Set to 0.0 for opaque materials (metal, paint).
    pub ior: f64,

    /// Lichtdurchlässigkeit [%], 0-100 at the given thickness
    /// How much light passes through (measured at normal incidence).
    /// 0 = fully opaque, 92 = clear PMMA 3mm, 50 = heavy opal PMMA 3mm.
    pub transmittance_pct: f64,

    /// Dicke [mm]
    /// Material thickness. Affects volume scattering path length
    /// and Beer-Lambert absorption. Ignored for opaque materials.
    pub thickness_mm: f64,

    /// Streuungsgrad [%], 0-100
    /// Degree of light diffusion/scattering.
    /// 0 = perfectly clear (or mirror-specular for opaque)
    /// 25 = satin/frosted
    /// 60 = light opal
    /// 95 = heavy opal (near-Lambertian exit distribution)
    /// 100 = fully diffuse (matte paint for opaque)
    /// Maps directly to haze values in datasheets (e.g. Evonik Plexiglas).
    pub diffusion_pct: f64,
}
```

#### MaterialParams → Material Conversion

`MaterialParams::to_material()` derives the internal physics:

```rust
impl MaterialParams {
    pub fn to_material(&self) -> Material {
        // ... conversion logic, see rules below
    }
}
```

**Conversion rules:**

| Condition | Material variant | Derived from |
|-----------|-----------------|--------------|
| `transmittance_pct == 0, diffusion_pct == 0` | `SpecularReflector` | `reflectance = reflectance_pct / 100` |
| `transmittance_pct == 0, diffusion_pct == 100` | `DiffuseReflector` | `reflectance = reflectance_pct / 100` |
| `transmittance_pct == 0, 0 < diffusion < 100` | `MixedReflector` | `specular_fraction = 1.0 - diffusion_pct / 100` |
| `transmittance_pct > 0, diffusion_pct < 5` | `ClearTransmitter` | `ior` used directly, `tau = transmittance_pct / 100` |
| `transmittance_pct > 0, diffusion_pct >= 5` | `DiffuseTransmitter` | See volume scattering derivation below |
| `reflectance_pct < 2, transmittance_pct == 0` | `Absorber` | Near-zero reflectance = black hole |

**Volume scattering derivation** (for `DiffuseTransmitter`):

Given user-facing `transmittance_pct`, `thickness_mm`, and `diffusion_pct`:

```
thickness [m] = thickness_mm / 1000

# Total attenuation from Beer-Lambert law:
# tau = exp(-mu_t * thickness),  where mu_t = mu_s + mu_a
mu_t = -ln(transmittance_pct / 100) / thickness

# Split between scattering and absorption based on diffusion:
# High diffusion = mostly scattering, low absorption
# albedo (single-scatter) = mu_s / mu_t
albedo = 0.5 + 0.5 * (diffusion_pct / 100)    # range 0.5..1.0
mu_s = mu_t * albedo
mu_a = mu_t * (1 - albedo)

# Henyey-Greenstein asymmetry parameter:
# Low diffusion = forward-biased scattering (g near 1)
# High diffusion = near-isotropic scattering (g near 0)
g = 0.9 * (1.0 - diffusion_pct / 100)         # range 0.9..0.0
```

#### Internal: Material Enum

The tracer works with these. Users never construct them directly.

```rust
pub(crate) enum Material {
    Absorber,
    DiffuseReflector { reflectance: f64 },
    SpecularReflector { reflectance: f64 },
    MixedReflector { reflectance: f64, specular_fraction: f64 },
    ClearTransmitter { ior: f64, transmittance: f64 },
    DiffuseTransmitter {
        ior: f64,
        scattering_coeff: f64,     // mu_s [1/m]
        absorption_coeff: f64,     // mu_a [1/m]
        asymmetry: f64,            // HG g parameter
        thickness: f64,            // slab thickness [m]
    },
}
```

#### Material Interaction Logic

For each material type, the tracer calls `interact(photon, hit) -> Interaction`:

```rust
pub enum Interaction {
    Absorbed,
    Reflected { new_ray: Ray, attenuation: f64 },
    Transmitted { new_ray: Ray, attenuation: f64 },
}
```

**Fresnel equations** (ClearTransmitter and DiffuseTransmitter):
- Compute reflectance R(theta) from Fresnel equations given IOR and incident angle
- With probability R: reflect (specular). With probability 1-R: transmit (refract via Snell's law)
- Both surfaces of a sheet are modeled: entry and exit, each with Fresnel

**Henyey-Greenstein phase function** (DiffuseTransmitter):
- Inside the volume, photon travels a random distance sampled from exp(-mu_t * d)
  where mu_t = mu_s + mu_a
- At each scattering event, new direction sampled from HG distribution:
  cos(theta) = (1/(2g)) * (1 + g^2 - ((1-g^2)/(1-g+2g*xi))^2)
- Photon exits when it reaches a surface boundary, subject to internal Fresnel

#### Material Catalog

Common materials with real datasheet values. Users pick from this list or use as
starting points for custom materials.

```rust
pub fn material_catalog() -> Vec<MaterialParams>;
```

| Name | Reflexion % | Brechungsindex | Durchlässigkeit % | Dicke mm | Streuung % |
|------|-------------|----------------|-------------------|----------|------------|
| PMMA klar 3mm | 0 | 1.49 | 92 | 3.0 | 0 |
| PMMA satin 3mm | 0 | 1.49 | 85 | 3.0 | 25 |
| PMMA opal leicht 3mm | 0 | 1.49 | 75 | 3.0 | 60 |
| PMMA opal 3mm | 0 | 1.49 | 50 | 3.0 | 95 |
| Glas klar 4mm | 0 | 1.52 | 90 | 4.0 | 0 |
| Glas satiniert 4mm | 0 | 1.52 | 75 | 4.0 | 30 |
| Polycarbonat klar 3mm | 0 | 1.585 | 88 | 3.0 | 0 |
| Polycarbonat opal 3mm | 0 | 1.585 | 55 | 3.0 | 90 |
| Aluminium eloxiert | 80 | 0.0 | 0 | 0.0 | 70 |
| Aluminium Spiegel | 95 | 0.0 | 0 | 0.0 | 0 |
| Weisslack | 85 | 0.0 | 0 | 0.0 | 100 |
| Schwarz matt | 5 | 0.0 | 0 | 0.0 | 100 |

Note: For transparent materials, `reflectance_pct` is 0 because Fresnel reflection
is computed from `ior` — the IOR already determines how much light reflects at each
surface. Setting both would double-count.

#### Preset Constructors

```rust
pub fn clear_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA klar 3mm".into(),
        reflectance_pct: 0.0,
        ior: 1.49,
        transmittance_pct: 92.0,
        thickness_mm: 3.0,
        diffusion_pct: 0.0,
    }
}

pub fn opal_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA opal 3mm".into(),
        reflectance_pct: 0.0,
        ior: 1.49,
        transmittance_pct: 50.0,
        thickness_mm: 3.0,
        diffusion_pct: 95.0,
    }
}

pub fn white_paint() -> MaterialParams {
    MaterialParams {
        name: "Weisslack".into(),
        reflectance_pct: 85.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    }
}

pub fn anodized_aluminum() -> MaterialParams {
    MaterialParams {
        name: "Aluminium eloxiert".into(),
        reflectance_pct: 80.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 70.0,
    }
}

pub fn mirror_aluminum() -> MaterialParams {
    MaterialParams {
        name: "Aluminium Spiegel".into(),
        reflectance_pct: 95.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 0.0,
    }
}

pub fn matte_black() -> MaterialParams {
    MaterialParams {
        name: "Schwarz matt".into(),
        reflectance_pct: 5.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    }
}
```

### 4. Monte Carlo Tracer (`tracer.rs`)

The simulation core. Traces photons one by one (or in parallel via Rayon).

```rust
pub struct TracerConfig {
    pub num_photons: u64,                  // total photons to trace
    pub max_bounces: u32,                  // max interactions per photon (default 50)
    pub russian_roulette_threshold: f64,   // energy below which RR kicks in (default 0.01)
    pub seed: u64,                         // RNG seed for reproducibility
}

pub struct TracerResult {
    pub detector: Detector,
    pub stats: TracerStats,
    pub trails: Vec<PhotonTrail>,          // optional, first N paths for visualization
}

pub struct TracerStats {
    pub photons_traced: u64,
    pub photons_detected: u64,             // reached detector sphere
    pub photons_absorbed: u64,
    pub photons_max_bounces: u64,          // terminated by bounce limit
    pub photons_russian_roulette: u64,     // terminated by RR
    pub total_energy_emitted: f64,
    pub total_energy_detected: f64,        // should approach emitted * (1 - absorbed fraction)
    pub elapsed: Duration,
}
```

#### Algorithm

```
for each photon:
    1. Sample origin + direction from Source
    2. Set energy = 1.0, bounces = 0
    3. Loop:
       a. Find nearest intersection with scene geometry
       b. If no hit → photon escapes → record on Detector sphere
       c. Get material at hit point
       d. Call material.interact(photon, hit):
          - Absorbed → break
          - Reflected → update ray, energy *= attenuation
          - Transmitted → update ray, energy *= attenuation
       e. bounces += 1
       f. If bounces > max_bounces → break
       g. Russian roulette: if energy < threshold →
          survive with probability energy/threshold, boost energy
          or terminate
    4. If photon escaped scene → detector.record(direction, energy)
```

#### Parallelism

With the `parallel` feature (default), photon batches are distributed across threads
via Rayon. Each thread has its own RNG (seeded deterministically from batch index)
and local detector accumulator. After all batches complete, accumulators are merged.

```rust
impl Tracer {
    /// Trace all photons. Returns result with filled detector.
    pub fn trace(&self, scene: &Scene, config: &TracerConfig) -> TracerResult;

    /// Trace with progress callback (called per batch).
    /// Useful for UI progress bars.
    pub fn trace_with_progress(
        &self,
        scene: &Scene,
        config: &TracerConfig,
        callback: impl Fn(ProgressInfo) + Send + Sync,
    ) -> TracerResult;
}

pub struct ProgressInfo {
    pub photons_done: u64,
    pub photons_total: u64,
    pub photons_per_second: f64,
    pub current_stats: TracerStats,
}
```

#### Photon Trails (for visualization)

The first N photons (configurable, e.g. 1000) record their full path for
visualization by the Bevy frontend.

```rust
pub struct PhotonTrail {
    pub points: Vec<TrailPoint>,
}

pub struct TrailPoint {
    pub position: Point3<f64>,
    pub event: TrailEvent,
}

pub enum TrailEvent {
    Emitted,
    Reflected,
    Transmitted,
    Scattered,       // volume scattering inside diffuse material
    Absorbed,
    Detected,        // reached detector sphere
}
```

### 5. Spherical Detector (`detector.rs`)

A virtual goniophotometer sphere. Collects photons that escape the scene and bins
them by direction.

```rust
pub struct Detector {
    bins: Vec<Vec<f64>>,          // [c_index][g_index] accumulated energy
    counts: Vec<Vec<u64>>,        // [c_index][g_index] photon count
    c_resolution_deg: f64,        // e.g. 1.0 for 1deg bins
    g_resolution_deg: f64,
    num_c: usize,                 // 360 / c_resolution
    num_g: usize,                 // 180 / g_resolution + 1
    total_energy: f64,
}

impl Detector {
    pub fn new(c_resolution_deg: f64, g_resolution_deg: f64) -> Self;

    /// Record an escaping photon by its world-space direction and energy.
    pub fn record(&mut self, direction: &Vector3<f64>, energy: f64);

    /// Convert accumulated bins to candela values.
    /// cd = (energy_in_bin / solid_angle_of_bin) * (source_flux / total_energy)
    pub fn to_candela(&self, source_flux_lm: f64) -> Vec<Vec<f64>>;

    /// Total detected flux (for energy conservation validation).
    pub fn total_flux(&self, source_flux_lm: f64) -> f64;

    /// Merge another detector's data (for parallel accumulation).
    pub fn merge(&mut self, other: &Detector);

    /// Extract C-planes at the intervals needed for Eulumdat export.
    /// E.g. from 1deg detector bins, extract every 5deg for a standard LDT.
    pub fn resample(&self, c_step_deg: f64, g_step_deg: f64) -> Detector;
}
```

#### Direction → Bin Mapping

The detector uses CIE photometric coordinates:
- **C-angle**: azimuth in the horizontal plane. C0 = front (+X), C90 = right (+Y),
  measured clockwise when viewed from above.
- **Gamma angle**: from nadir. gamma=0 = straight down (-Z), gamma=90 = horizontal,
  gamma=180 = straight up (+Z).

```rust
fn direction_to_cg(dir: &Vector3<f64>) -> (f64, f64) {
    let gamma = dir.z.acos().to_degrees();         // 0=down, 180=up (if -Z is down)
    let c = dir.y.atan2(dir.x).to_degrees();       // azimuth
    let c = if c < 0.0 { c + 360.0 } else { c };
    (c, gamma)
}
```

The coordinate convention must match `eulumdat-rs` exactly. The nadir/zenith
convention (gamma=0 = down) follows the EULUMDAT/CIE standard.

#### Solid Angle Correction

Bins near the poles subtend less solid angle than bins at the equator.
The candela normalization must account for this:

```
solid_angle(g) = |sin(g)| * dg * dc   (in radians)
```

Bins at gamma=0 and gamma=180 are degenerate (zero solid angle) — these are
handled as small caps.

### 6. Scene Description (`scene.rs`)

Combines sources, geometry, and materials into a traceable scene.

```rust
pub struct Scene {
    pub sources: Vec<Source>,
    pub objects: Vec<SceneObject>,
    pub materials: Vec<Material>,          // indexed by MaterialId (internal)
    pub material_params: Vec<MaterialParams>, // user-facing, parallel to materials
}

pub type MaterialId = usize;

impl Scene {
    pub fn new() -> Self;
    pub fn add_source(&mut self, source: Source);

    /// Add a material from user-facing params. Converts internally.
    pub fn add_material(&mut self, params: MaterialParams) -> MaterialId;

    pub fn add_object(&mut self, primitive: Primitive, material: MaterialId, label: &str) -> usize;

    /// Find nearest intersection of ray with any scene object.
    pub fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
}
```

#### Scene Builder

For common luminaire configurations, the `SceneBuilder` provides a high-level API
where users specify materials and distances without computing 3D coordinates.

```rust
pub struct SceneBuilder {
    scene: Scene,
    source_position: Point3<f64>,    // origin, typically (0, 0, 0)
    source_direction: Unit<Vector3<f64>>, // emission direction, typically -Z (down)
}

impl SceneBuilder {
    /// Start building a scene with a source at the origin.
    pub fn new() -> Self;

    /// Set the light source.
    pub fn source(mut self, source: Source) -> Self;

    /// Add a reflector/housing surface at a given distance from the source.
    /// Distance is measured perpendicular to the source direction.
    pub fn reflector(
        mut self,
        material: MaterialParams,
        placement: ReflectorPlacement,
    ) -> Self;

    /// Add a transmissive cover (PMMA, glass) at a given distance from the source.
    /// Distance is measured along the source emission direction.
    pub fn cover(
        mut self,
        material: MaterialParams,
        placement: CoverPlacement,
    ) -> Self;

    /// Build the final scene.
    pub fn build(self) -> Scene;
}

/// Placement of a transmissive cover relative to the light source.
pub struct CoverPlacement {
    /// Abstand zur Lichtquelle [mm] — along emission axis
    pub distance_mm: f64,
    /// Cover width [mm]
    pub width_mm: f64,
    /// Cover height [mm]
    pub height_mm: f64,
}

/// Placement of a reflector surface relative to the light source.
pub struct ReflectorPlacement {
    /// Abstand zur Lichtquelle [mm] — perpendicular to emission axis
    pub distance_mm: f64,
    /// Reflector length along emission axis [mm]
    pub length_mm: f64,
    /// Which side of the source (Left, Right, Back, or Surround for cylinder)
    pub side: ReflectorSide,
}

pub enum ReflectorSide {
    Left,
    Right,
    Back,
    Surround,  // cylindrical reflector around the source
}
```

**Example: LED + housing + opal cover**

```rust
let scene = SceneBuilder::new()
    .source(Source::Led {
        position: Point3::origin(),
        direction: -Vector3::z_axis(),
        half_angle_deg: 60.0,
        flux_lm: 1000.0,
    })
    .reflector(white_paint(), ReflectorPlacement {
        distance_mm: 25.0,
        length_mm: 50.0,
        side: ReflectorSide::Surround,
    })
    .cover(opal_pmma_3mm(), CoverPlacement {
        distance_mm: 40.0,    // 40mm below LED
        width_mm: 60.0,
        height_mm: 60.0,
    })
    .build();
```

This builds the geometry automatically: LED at origin pointing down, white-painted
cylindrical housing 25mm radius x 50mm tall, opal PMMA sheet 40mm below the LED.

### 7. Export (`export.rs`)

Converts detector output into an `Eulumdat` struct ready for `.ldt` export.

```rust
pub struct ExportConfig {
    pub c_step_deg: f64,               // C-plane interval (default 15.0)
    pub g_step_deg: f64,               // gamma interval (default 1.0 or 5.0)
    pub symmetry: Option<Symmetry>,    // force symmetry, or auto-detect
    pub luminaire_name: String,
    pub manufacturer: String,
    pub luminaire_dimensions_mm: (f64, f64, f64),  // L, W, H
    pub luminous_area_mm: (f64, f64),              // La, Wa
}

/// Build an Eulumdat struct from detector data.
pub fn detector_to_eulumdat(
    detector: &Detector,
    source_flux_lm: f64,
    config: &ExportConfig,
) -> Eulumdat;
```

This function:
1. Resamples detector bins to the requested C/G resolution
2. Converts energy bins to cd/klm values
3. Fills all `Eulumdat` fields (metadata from `ExportConfig`, photometric data from detector)
4. Optionally detects and applies symmetry (if measured data is symmetric within tolerance)
5. Sets `lamp_sets`, `downward_flux_fraction`, `light_output_ratio` from computed values
6. Result passes `validate_strict()` — ready for `to_ldt()`

### 8. Presets (`presets.rs`)

Ready-made scenes for demos and testing. All use `MaterialParams` and `SceneBuilder`.

```rust
/// Bare Lambertian emitter in free space.
/// Expected result: cosine LVK (analytical solution known).
/// Primary validation case — no geometry, no materials.
pub fn bare_lambertian(flux_lm: f64) -> Scene;

/// Bare isotropic point source.
/// Expected result: uniform sphere (constant cd in all directions).
pub fn bare_isotropic(flux_lm: f64) -> Scene;

/// LED with reflector housing.
/// Housing: white paint (Reflexion 85%, Streuung 100%).
/// Shows how housing shapes the beam.
pub fn led_with_housing(flux_lm: f64, beam_angle_deg: f64) -> Scene {
    SceneBuilder::new()
        .source(Source::Led { ..., flux_lm })
        .reflector(white_paint(), ReflectorPlacement {
            distance_mm: 25.0,
            length_mm: 50.0,
            side: ReflectorSide::Surround,
        })
        .build()
}

/// LED + housing + clear PMMA cover at configurable distance.
/// Shows Fresnel losses (~8% for PMMA at normal incidence).
pub fn led_housing_clear_cover(flux_lm: f64, cover_distance_mm: f64) -> Scene {
    SceneBuilder::new()
        .source(Source::Led { ..., flux_lm })
        .reflector(white_paint(), ReflectorPlacement {
            distance_mm: 25.0,
            length_mm: cover_distance_mm + 10.0,
            side: ReflectorSide::Surround,
        })
        .cover(clear_pmma_3mm(), CoverPlacement {
            distance_mm: cover_distance_mm,
            width_mm: 60.0,
            height_mm: 60.0,
        })
        .build()
}

/// LED + housing + opal PMMA cover.
/// Shows how diffuser transforms beam into soft distribution.
/// User can swap material to see effect of different Streuungsgrad values.
pub fn led_housing_opal_cover(flux_lm: f64, cover: MaterialParams) -> Scene {
    SceneBuilder::new()
        .source(Source::Led { ..., flux_lm })
        .reflector(anodized_aluminum(), ReflectorPlacement {
            distance_mm: 25.0,
            length_mm: 50.0,
            side: ReflectorSide::Surround,
        })
        .cover(cover, CoverPlacement {
            distance_mm: 40.0,
            width_mm: 60.0,
            height_mm: 60.0,
        })
        .build()
}

/// Lichtsaeule corner profile.
/// LED strip in L-profile aluminum housing with opal PMMA cover.
pub fn lichtsaeule_eckprofil(
    flux_lm: f64,
    length_m: f64,
    housing: MaterialParams,     // e.g. anodized_aluminum()
    cover: MaterialParams,       // e.g. opal_pmma_3mm()
    cover_distance_mm: f64,      // distance from LED strip to cover
) -> Scene;

/// Load an existing LDT as source, trace through empty space.
/// Result must match input within statistical noise.
/// This is the round-trip validation test.
pub fn roundtrip_validation(ldt: &Eulumdat) -> Scene;

/// Custom scene: user provides all parameters.
/// This is what the UI calls when users configure their own simulation.
pub fn custom(
    source: Source,
    housing: MaterialParams,
    housing_distance_mm: f64,
    cover: Option<(MaterialParams, f64)>,  // (material, distance_mm)
) -> Scene;
```

## Validation Strategy

The CPU tracer is the reference. It must be provably correct before anything else
is built on top of it.

### Analytical Test Cases

| Test | Input | Expected Output | Tolerance |
|------|-------|-----------------|-----------|
| Isotropic in free space | Point source, 1000 lm | Constant ~79.6 cd everywhere | < 2% RMS at 1M photons |
| Lambertian in free space | Cosine emitter, 1000 lm | I(gamma) = I_max * cos(gamma) | < 2% RMS at 1M photons |
| Perfect mirror | 45deg mirror, point source | All light redirected to mirror angle | < 1% flux loss |
| Clear PMMA slab | Normal incidence, IOR 1.49 | ~8% Fresnel loss (two surfaces) | < 0.5% |
| Energy conservation | Any scene | detected + absorbed = emitted | < 0.1% |

### Round-trip Test

```
input.ldt → FromLvk source → trace empty space → detector → export.ldt → compare
```

Uses `PhotometricComparison::from_eulumdat(&input, &output, ...)` from the existing
`eulumdat` crate. Similarity score must be > 99% at sufficient photon count.

### Convergence Test

Run same scene at 10k, 100k, 1M, 10M photons. RMS difference between successive
runs must decrease as 1/sqrt(N) — the expected Monte Carlo convergence rate.

## Integration Points

### With `eulumdat` crate

- `Eulumdat::new()` + public fields → construct from detector data
- `Eulumdat::sample(c, g)` → importance sampling for `FromLvk` source
- `Eulumdat::to_ldt()` → export result
- `PhotometricComparison` → validate round-trip
- `validate_strict()` → verify exported struct is well-formed

### With Bevy (visualization, separate crate)

The Bevy integration consumes `eulumdat-goniosim` as a dependency.
The simulation crate provides data; Bevy renders it.

```rust
// Bevy system: kick off simulation
fn start_trace(scene: Res<GonioScene>, mut commands: Commands) {
    let config = TracerConfig { num_photons: 1_000_000, ..default() };
    // spawn async task or run in thread
    let result = Tracer::new().trace(&scene.0, &config);
    commands.insert_resource(TraceResult(result));
}

// Bevy system: render photon trails
fn render_trails(result: Res<TraceResult>, mut gizmos: Gizmos) {
    for trail in &result.0.trails {
        for window in trail.points.windows(2) {
            let color = match window[1].event {
                TrailEvent::Reflected => Color::BLUE,
                TrailEvent::Transmitted => Color::GREEN,
                TrailEvent::Absorbed => Color::RED,
                TrailEvent::Detected => Color::WHITE,
                _ => Color::YELLOW,
            };
            gizmos.line(window[0].position.cast(), window[1].position.cast(), color);
        }
    }
}

// Bevy system: update detector heatmap on sphere mesh
fn update_detector_mesh(result: Res<TraceResult>, mut meshes: ResMut<Assets<Mesh>>) {
    let candela = result.0.detector.to_candela(source_flux);
    // map candela values to vertex colors on sphere mesh
}
```

### With future GPU tracer

The GPU tracer will implement the same `Scene` → `TracerResult` interface.
Validation:

```rust
let cpu_result = CpuTracer::trace(&scene, &config);
let gpu_result = GpuTracer::trace(&scene, &config);

// Compare detector outputs
let cpu_ldt = detector_to_eulumdat(&cpu_result.detector, flux, &export_config);
let gpu_ldt = detector_to_eulumdat(&gpu_result.detector, flux, &export_config);
let cmp = PhotometricComparison::from_eulumdat(&cpu_ldt, &gpu_ldt, "CPU", "GPU");
assert!(cmp.similarity_score > 0.99);
```

## Performance Expectations

On a modern CPU (M-series Mac, Ryzen, etc.) with Rayon parallelism:

| Scene complexity | Photons/sec (est.) | Time for 1M photons |
|------------------|--------------------|---------------------|
| Free space (no geometry) | 5-10M | < 0.2s |
| Simple housing (5 surfaces) | 1-3M | 0.3-1s |
| Housing + clear PMMA | 500k-1M | 1-2s |
| Housing + opal PMMA (volume scattering) | 100k-500k | 2-10s |

Opal PMMA is the bottleneck — each photon scatters many times inside the volume
before exiting. This is expected and matches real goniophotometer measurement times
(opal diffusers produce noisier data).

## Non-Goals

- **Not a certified measurement tool** — this is educational/demonstrative
- **No mesh geometry** — analytic primitives only (no STL/OBJ import in v1)
- **No spectral rendering** — monochromatic (555nm), color is visual only
- **No polarization** — unpolarized light only
- **No GPU compute** — that's a separate crate later, validated against this one
- **No UI** — that's Bevy's job
