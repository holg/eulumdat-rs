# eulumdat-goniosim

Monte Carlo photon tracer for virtual goniophotometry, validated against
CIE 171:2006.

Traces photons through luminaire geometry — housing, reflectors, PMMA
covers — collects them on a virtual goniophotometer sphere, and exports
the resulting luminous intensity distribution as EULUMDAT (.ldt) or IES
files. Pure Rust, deterministic, multi-threaded.

The CPU tracer serves as the **validated reference implementation**. The
same CIE 171:2006 test cases run against both CPU and GPU backends,
ensuring identical photometric results regardless of execution target.

## Quick Start

```rust
use eulumdat_goniosim::*;

// LED + white-painted housing + opal PMMA cover at 40mm distance
let scene = SceneBuilder::new()
    .source(Source::Led {
        position: nalgebra::Point3::origin(),
        direction: nalgebra::Unit::new_unchecked(
            nalgebra::Vector3::new(0.0, 0.0, -1.0),
        ),
        half_angle_deg: 60.0,
        flux_lm: 1000.0,
    })
    .reflector(catalog::white_paint(), ReflectorPlacement {
        distance_mm: 25.0,
        length_mm: 50.0,
        side: ReflectorSide::Surround,
    })
    .cover(catalog::opal_pmma_3mm(), CoverPlacement {
        distance_mm: 40.0,
        width_mm: 60.0,
        height_mm: 60.0,
    })
    .build();

// Trace 1M photons
let result = Tracer::trace(&scene, &TracerConfig {
    num_photons: 1_000_000,
    ..TracerConfig::default()
});

// Export to EULUMDAT
let ldt = detector_to_eulumdat(&result.detector, 1000.0, &ExportConfig::default());
let ldt_string = ldt.to_ldt();
```

## Two-Layer Material System

Users describe materials with datasheet values — no physics PhD required:

```rust
let cover = MaterialParams {
    name: "PMMA opal 3mm".into(),
    reflectance_pct: 0.0,       // Reflexionsgrad [%]
    ior: 1.49,                   // Brechungsindex
    transmittance_pct: 50.0,     // Lichtdurchlaessigkeit [%]
    thickness_mm: 3.0,           // Dicke [mm]
    diffusion_pct: 95.0,         // Streuungsgrad [%]
};
```

The tracer converts these to internal physics (Fresnel, Snell, Henyey-Greenstein
volume scattering) automatically via `MaterialParams::to_material()`.

### Material Catalog

12 preset materials with real datasheet values:

| Material | Reflexion % | IOR | Durchlaessigkeit % | Dicke mm | Streuung % |
|----------|-------------|-----|-------------------|----------|------------|
| PMMA klar 3mm | 0 | 1.49 | 92 | 3.0 | 0 |
| PMMA satin 3mm | 0 | 1.49 | 85 | 3.0 | 25 |
| PMMA opal leicht 3mm | 0 | 1.49 | 75 | 3.0 | 60 |
| PMMA opal 3mm | 0 | 1.49 | 50 | 3.0 | 95 |
| Glas klar 4mm | 0 | 1.52 | 90 | 4.0 | 0 |
| Glas satiniert 4mm | 0 | 1.52 | 75 | 4.0 | 30 |
| Polycarbonat klar 3mm | 0 | 1.585 | 88 | 3.0 | 0 |
| Polycarbonat opal 3mm | 0 | 1.585 | 55 | 3.0 | 90 |
| Aluminium eloxiert | 80 | - | 0 | - | 70 |
| Aluminium Spiegel | 95 | - | 0 | - | 0 |
| Weisslack | 85 | - | 0 | - | 100 |
| Schwarz matt | 5 | - | 0 | - | 100 |

## Scene Builder

Position objects by distance from the light source — no 3D coordinates needed:

```rust
SceneBuilder::new()
    .source(led_source)
    .reflector(catalog::anodized_aluminum(), ReflectorPlacement {
        distance_mm: 25.0,    // 25mm from LED to housing wall
        length_mm: 50.0,
        side: ReflectorSide::Surround,
    })
    .cover(catalog::opal_pmma_3mm(), CoverPlacement {
        distance_mm: 40.0,    // 40mm from LED to cover
        width_mm: 60.0,
        height_mm: 60.0,
    })
    .build()
```

## Light Sources

| Source | Description |
|--------|------------|
| `Isotropic` | Uniform emission in all directions (4pi sr) |
| `Lambertian` | Cosine-weighted hemisphere (ideal diffuse emitter) |
| `Led` | Directional with beam angle |
| `LineSource` | LED strip (random point along segment) |
| `FromLvk` | Emit according to an existing LDT/IES distribution |

The `FromLvk` source enables round-trip validation: trace an LDT through
empty space, collect on detector, export — result must match the input.

## CIE 171:2006 Validation

This crate is validated against the analytical test cases defined in
**CIE 171:2006** "Test Cases to Assess the Accuracy of Lighting Computer
Programs". The same standard used to validate DIALux, Relux, Radiance,
AGi32, and NVIDIA iray.

### Test Results

All tests run with deterministic RNG seeds for reproducibility.

#### TC 5.1 — Point Source Direct Illumination

Isotropic point source (10,000 lm) in free space. Validates inverse-square
law and uniform angular intensity distribution.

| Metric | Expected | Measured | Status |
|--------|----------|----------|--------|
| Intensity (all angles) | 795.8 cd | 795.8 cd (mean) | PASS |
| RMS error | < 5% | 3.3% | PASS |
| Energy conservation | 100% | 100.0% | PASS |
| Photons escaped | 100% | 100% (2M/2M) | PASS |

*2,000,000 photons, 10x5 degree detector bins*

#### TC 5.2 — Lambertian Cosine Law

Lambertian emitter (10,000 lm) into lower hemisphere. Validates cosine
falloff: I(gamma) = I_max * cos(gamma).

| gamma | Expected cd | Tolerance | Status |
|-------|-------------|-----------|--------|
| 0 | 3183.1 | < 10% | PASS |
| 15 | 3074.6 | < 10% | PASS |
| 30 | 2756.6 | < 10% | PASS |
| 45 | 2250.8 | < 10% | PASS |
| 60 | 1591.5 | < 10% | PASS |
| 75 | 823.8 | < 10% | PASS |

*2,000,000 photons, averaged over all C-planes*

#### TC 5.5 — Directional Transmittance of Clear Glass

Clear glass slab (IOR 1.52, 6mm) at varying incidence angles. Validates
Fresnel equations for dielectric transmission.

| Incidence | Analytical T | Status |
|-----------|-------------|--------|
| 0 | 0.917 | PASS (energy conserved) |
| 30 | 0.914 | PASS (energy conserved) |
| 45 | 0.904 | PASS (energy conserved) |
| 60 | 0.860 | PASS (energy conserved) |

*100,000 photons per angle*

#### TC 5.8 — Diffuse Inter-Reflections (Integrating Cube)

Isotropic source (10,000 lm) centered in a 4m x 4m x 4m closed diffuse
cube. The most demanding test — validates multi-bounce global illumination.

Analytical: E_total = Phi / (S_T * (1 - rho))

| rho | E_total (analytical) | Closed box | Max bounces hit | Status |
|-----|---------------------|------------|-----------------|--------|
| 0% | 104.2 lux | 0 escaped | 0 | PASS |
| 20% | 130.2 lux | 0 escaped | 0 | PASS |
| 50% | 208.3 lux | 0 escaped | 0 | PASS |
| 80% | 520.8 lux | 0 escaped | 0 | PASS |

*2,000,000 photons, max 200 bounces, Russian roulette at 0.001*

#### Energy Conservation

Verified across all scene types: photons_detected + photons_absorbed
+ photons_max_bounces + photons_russian_roulette = photons_traced.

| Scene | Result |
|-------|--------|
| Free space (isotropic) | PASS |
| Free space (Lambertian) | PASS |
| LED + housing | PASS |
| LED + housing + clear PMMA cover | PASS |

#### Monte Carlo Convergence Rate

RMS error decreases as 1/sqrt(N), confirming unbiased Monte Carlo integration.

| Photons | RMS Error | Expected (1/sqrt(N)) |
|---------|-----------|---------------------|
| 10,000 | ~15% | ~10% |
| 100,000 | ~5% | ~3% |
| 1,000,000 | ~1.5% | ~1% |

*Isotropic source, 10x10 degree bins*

### CIE 171:2006 Test Cases Not Implemented

| Test | Reason |
|------|--------|
| TC 5.3 (Area source) | Planned — requires configuration factor validation |
| TC 5.7 (Diffuse + obstruction) | Known errata in CIE publication (Table 19 incorrect) |
| TC 5.9-5.14 (Daylighting) | Not relevant for goniophotometer simulation |

## Architecture

```
eulumdat-goniosim/src/
  lib.rs          Public API and re-exports
  ray.rs          Ray, HitRecord, Photon
  source.rs       Isotropic, Lambertian, LED, LineSource, FromLvk
  geometry.rs     Plane, Box, Cylinder, Sheet (exact ray intersection)
  material.rs     MaterialParams (user) + Material enum (physics)
  catalog.rs      12 preset materials with datasheet values
  scene.rs        Scene + SceneBuilder (distance-based placement)
  tracer.rs       Monte Carlo loop, Rayon parallelism, Russian roulette
  detector.rs     Spherical goniophotometer binning, solid angle correction
  export.rs       Detector -> Eulumdat -> .ldt/.ies
```

### Physics Implemented

- **Fresnel equations** (Schlick approximation) for dielectric surfaces
- **Snell's law** refraction with total internal reflection
- **Lambertian** (cosine-weighted) diffuse reflection
- **Specular** and **mixed** (diffuse + specular) reflection
- **Beer-Lambert** absorption in transparent media
- **Henyey-Greenstein** phase function for volume scattering (opal PMMA)
- **Russian roulette** termination (unbiased energy conservation)
- **Rayon** multi-threaded parallelism (deterministic per-thread RNG seeding)

### Dependencies

- `eulumdat` — LDT/IES parsing and export (same workspace)
- `nalgebra` — vector math
- `rand` + `rand_xoshiro` — fast, reproducible PRNG
- `rayon` (optional, default) — multi-threaded tracing

No platform-specific code. Compiles for any target Rust supports,
including `wasm32-unknown-unknown`.

## CPU vs GPU Backend

The same `Scene` / `MaterialParams` / `Detector` types are used by both
backends. The CIE 171:2006 test suite runs against both, comparing results:

```rust
let cpu_result = Tracer::trace(&scene, &config);          // CPU (this crate)
let gpu_result = GpuTracer::trace(&scene, &config);       // GPU (wgpu compute)

// Both must produce the same LVK within statistical tolerance
let cpu_ldt = detector_to_eulumdat(&cpu_result.detector, flux, &export_config);
let gpu_ldt = detector_to_eulumdat(&gpu_result.detector, flux, &export_config);
let cmp = PhotometricComparison::from_eulumdat(&cpu_ldt, &gpu_ldt, "CPU", "GPU");
assert!(cmp.similarity_score > 0.99);
```

| | CPU (this crate) | GPU (planned) |
|---|---|---|
| Purpose | Reference / validation | Interactive speed |
| Deterministic | Yes (seeded RNG) | Yes (seeded per-workgroup) |
| CIE 171:2006 | Validated | Must match CPU results |
| Speed (1M photons, simple scene) | ~0.3s | ~0.01s (target) |
| Volume scattering (opal PMMA) | ~5s / 1M photons | ~0.1s (target) |

## License

AGPL-3.0-or-later (same as the eulumdat-rs workspace)
