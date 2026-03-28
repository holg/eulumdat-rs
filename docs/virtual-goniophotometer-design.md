# Virtual Goniophotometer — Architecture Concept

## Vision

A browser-based (WASM) visual Monte Carlo photon tracer that shows **in real-time** how a Luminous Intensity Distribution (LVK/LID) is built up from first principles. Not a certified measurement tool — a **demonstration and educational instrument** that makes the invisible visible.

> "Don't explain what an LDT file is. Show how one is born."

## The Demo Experience

1. User sees a 3D scene: an LED source, optional housing geometry, optional cover material (clear/opal PMMA)
2. Click **"Trace"** — photons start flying from the source
3. Photons interact with geometry: reflect off housing walls, transmit/refract through PMMA, scatter in diffuse materials
4. A translucent **virtual goniophotometer sphere** surrounds the luminaire, collecting arriving photons
5. A **polar LVK diagram** renders in real-time, building up from noise to a smooth curve as more photons accumulate
6. When converged: **"Export → EULUMDAT"** — click — download the `.ldt` file

The entire pipeline from physics to file format, visualized in one interactive experience.

## Why This Matters

The lighting industry has a communication problem:

- **Manufacturers** generate LDT/IES files but most users treat them as black boxes
- **Planners** import files into DIALux/Relux without understanding what's inside
- **Everyone** talks about LVK curves but few understand how they're created
- **Custom luminaires** (like Christian's Lichtsäulen) fall through the cracks because nobody owns the measurement step

This demo bridges the gap. It's marketing, education, and a technology demonstrator in one.

## Architecture

### Stack

```
┌─────────────────────────────────────┐
│          Browser (WASM)             │
├─────────────────────────────────────┤
│  Bevy App                           │
│  ├── Rendering Pipeline (visual)    │
│  │   ├── 3D Scene (luminaire, gonio)│
│  │   ├── Photon particle trails     │
│  │   └── Real-time LVK polar plot   │
│  ├── Compute Pipeline (simulation)  │
│  │   ├── wgpu compute shaders       │
│  │   ├── Monte Carlo photon tracer  │
│  │   └── Spherical detector binning │
│  └── UI (bevy_egui or Bevy UI)      │
│      ├── Source parameters           │
│      ├── Material editor             │
│      ├── Simulation controls         │
│      └── Export button               │
├─────────────────────────────────────┤
│  eulumdat-rs         (LDT export)   │
│  gldf-rs             (GLDF export)  │
│  l3d-rs              (L3D geometry) │
└─────────────────────────────────────┘
```

### Module Breakdown

#### 1. Photon Source Model (`src/source/`)

Defines how photons are emitted. Each source type produces rays with origin + direction + wavelength/color.

```
Sources:
├── Lambertian      — ideal diffuse emitter (cosine-weighted hemisphere)
├── Isotropic       — uniform in all directions (point source)
├── IES/LDT Import  — emit according to existing LVK (for validation)
└── LED Model       — directional with beam angle + spatial extent
    ├── Batwing
    ├── Lambertian
    └── Custom (from datasheet parameters)

Parameters:
├── Total luminous flux (lm)
├── Color temperature (K) → spectral distribution (for vis only)
├── Beam angle (for directional sources)
└── Physical extent (point / line / area)
```

For the Lichtsäulen demo: a **line source** (LED strip) with a directional emission profile matching typical PROLED strips (~120° beam angle, Lambertian-ish).

#### 2. Scene Geometry (`src/geometry/`)

Simple parametric geometry, not a full CAD system. Enough to model common luminaire housings.

```
Primitives:
├── Plane           — reflective back panel, walls
├── Box             — housing, niche
├── Cylinder        — round luminaires
├── L-Profile       — corner luminaires (like the Lichtsäule!)
└── Cover Sheet     — flat or curved plate (the PMMA)

Each primitive has:
├── Transform (position, rotation, scale)
├── Material reference
└── Role: Emitter | Reflector | Transmitter | Absorber | Detector
```

The scene for the Lichtsäulen demo:

```
Scene: "Lichtsäule Eckprofil"
├── LED Strip (line source, 120° beam)
│   └── Position: corner, facing outward diagonally
├── Aluminum housing (L-profile, 2 panels at 90°)
│   └── Material: matte white paint (ρ ≈ 0.85)
├── PMMA Cover (curved sheet, 85mm wide)
│   └── Material: opal PMMA (see below)
│   └── Distance from LED: 17mm
└── Brass edge trim (absorber, negligible)
```

#### 3. Material System (`src/material/`)

This is where the physics lives. Each material defines what happens when a photon hits a surface.

```
Material Types:
├── Specular Reflector
│   └── reflectance ρ, Fresnel equations for angle dependence
├── Diffuse Reflector (Lambertian)
│   └── reflectance ρ, scattered into hemisphere
├── Clear Transmitter
│   └── IOR (PMMA: 1.49), Fresnel reflection at both surfaces
│   └── Beer-Lambert absorption (negligible for thin PMMA in visible)
├── Diffuse Transmitter (OPAL PMMA — the hard one)
│   └── IOR, bulk scattering model
│   └── Henyey-Greenstein phase function (asymmetry parameter g)
│   └── Scattering coefficient μs, absorption coefficient μa
│   └── Thickness-dependent transmission
└── Absorber
    └── absorptance α = 1

Key Material Presets:
├── "Clear PMMA 3mm"    → IOR 1.49, τ ≈ 0.92, no scattering
├── "Opal PMMA 3mm"     → IOR 1.49, τ ≈ 0.50-0.85 (varies), heavy scattering
├── "Satin PMMA 3mm"    → IOR 1.49, surface scattering, moderate diffusion
├── "White paint"        → ρ ≈ 0.85, Lambertian
├── "Aluminum anodized"  → ρ ≈ 0.80, mostly specular
└── "Mirror"             → ρ ≈ 0.95, specular
```

**The opal PMMA model is the most complex part.** In reality, opal PMMA contains embedded scattering particles. A volume scattering model (similar to subsurface scattering in PBR) with Henyey-Greenstein phase function is the standard approach. For the demo, we can start with a simplified model and iterate.

#### 4. Monte Carlo Photon Tracer (`src/tracer/`)

The simulation core. Runs on GPU via wgpu compute shaders.

```
Algorithm per photon:
1. EMIT: Sample origin + direction from source model
2. INTERSECT: Find nearest geometry hit (ray-primitive intersection)
3. INTERACT: Based on material at hit point:
   ├── Reflect (specular or diffuse)
   ├── Transmit (with Fresnel, refraction)
   ├── Scatter (volume scattering in diffuse materials)
   └── Absorb (photon dies)
4. Continue until:
   ├── Photon exits the scene → record on detector sphere
   ├── Photon is absorbed
   └── Max bounces exceeded (e.g. 50)
5. Repeat for N photons (10k → 10M, progressive)

GPU Architecture:
├── Compute shader: trace batch of photons (e.g. 4096 per dispatch)
├── Storage buffer: detector sphere bins (θ × φ grid)
├── Storage buffer: photon trail points (for visualization)
└── Readback: periodically copy detector bins to CPU for LVK display
```

**Photon trail visualization:** Store the first N photon paths (e.g. 1000) with their interaction points and types. Render as particle trails with color-coding:
- 🟡 Yellow: emission
- 🔵 Blue: reflection
- 🟢 Green: transmission
- 🔴 Red: absorption
- ⚪ White: detected on sphere

#### 5. Virtual Goniophotometer (`src/detector/`)

A spherical detector that collects photons, exactly like a real goniophotometer.

```
Detector Sphere:
├── Resolution: e.g. 1° × 1° (360 × 180 = 64,800 bins)
├── Coordinate system: C-plane convention (C0-C360, γ0-γ180)
│   └── Standard photometric coordinate system per CIE
├── Accumulator: count photons per bin, weight by energy
└── Normalization: convert counts → candela (cd) values

LVK Generation:
├── Sum all bins → total flux (validation against source flux)
├── Extract C-planes at desired intervals (C0, C15, C30, ... C345)
├── Smooth/interpolate if needed
└── Ready for export
```

The detector sphere can be rendered semi-transparently in the 3D scene, with heat-map coloring showing where photons are arriving. This is the money shot.

#### 6. LVK Export (`src/export/`)

Here your existing crates shine.

```
Export Pipeline:
├── Detector bins → LVK data (cd values per C-plane and γ-angle)
├── eulumdat-rs → .ldt file
│   ├── Header metadata (from UI: luminaire name, manufacturer, etc.)
│   ├── Geometry (from scene primitives)
│   └── Photometric data (from detector)
├── IES export → .ies file (same data, different format)
└── gldf-rs → .gldf package
    ├── EULUMDAT embedded
    ├── l3d-rs → 3D model from scene geometry
    └── Metadata (from UI)
```

#### 7. UI / Controls (`src/ui/`)

```
Panels:
├── Source Panel
│   ├── Type selector (Lambertian / LED / Import LVK)
│   ├── Flux (lm)
│   ├── Beam angle
│   └── Color temperature (visual only)
├── Scene Panel
│   ├── Preset scenes (bare LED, LED+housing, Lichtsäule, ...)
│   ├── Geometry parameters (dimensions, distances)
│   └── Material assignments
├── Material Panel
│   ├── Preset materials
│   ├── Custom: reflectance, IOR, scattering params
│   └── Visual preview swatch
├── Simulation Panel
│   ├── Start / Pause / Reset
│   ├── Photon count (current / target)
│   ├── Convergence indicator
│   └── Speed slider (photons per frame)
├── Visualization Panel
│   ├── Show/hide: photon trails, detector sphere, LVK overlay
│   ├── Trail density slider
│   └── Detector sphere opacity
├── LVK Display (2D polar plot overlay)
│   ├── C-plane selector
│   ├── Live updating curve
│   └── Reference LVK overlay (import .ldt for comparison)
└── Export Panel
    ├── EULUMDAT (.ldt)
    ├── IES (.ies)
    ├── GLDF (.gldf)
    └── Metadata fields (luminaire name, manufacturer, etc.)
```

## Phased Development

### Phase 1: Core Raytracer + Visualization (MVP)

**Goal:** Photons fly, hit things, LVK builds up. Looks impressive.

- [ ] Bevy app scaffold with WASM target
- [ ] Basic primitives (plane, box)
- [ ] Lambertian + specular reflection materials
- [ ] CPU-based Monte Carlo tracer (GPU comes later)
- [ ] Photon trail rendering (instanced line segments or particles)
- [ ] Spherical detector with basic binning
- [ ] 2D polar plot (egui or custom Bevy UI)
- [ ] EULUMDAT export via eulumdat-rs

**Deliverable:** "Look, photons bounce off a reflector and we get an LVK curve" — already demo-worthy.

### Phase 2: Transmissive Materials + Real Scenes

**Goal:** Model real luminaires with covers.

- [ ] Clear PMMA: Fresnel + refraction (Snell's law)
- [ ] Opal PMMA: volume scattering (Henyey-Greenstein)
- [ ] L-profile geometry (for Lichtsäule demo)
- [ ] Line source (LED strip)
- [ ] Preset scene: "Lichtsäule Eckprofil"
- [ ] LDT/IES import for source profiles
- [ ] Reference LVK overlay (compare simulated vs. measured)

**Deliverable:** The Lichtsäule demo. Show how opal PMMA transforms a directional LED into a diffuse column of light.

### Phase 3: GPU Compute + Polish

**Goal:** Fast enough for interactive exploration. Production-quality visuals.

- [ ] wgpu compute shader tracer
- [ ] Progressive rendering (start coarse, refine)
- [ ] Detector sphere heat-map visualization
- [ ] Scene editor (drag geometry, change materials)
- [ ] GLDF export via gldf-rs + l3d-rs
- [ ] Hosted on eulumdat.icu or dedicated subdomain

**Deliverable:** Full interactive tool. Marketing gold for conference demos and Show HN.

### Phase 4 (Future): Validation + Extended Features

- [ ] Compare against measured LVKs from real luminaires
- [ ] Spectral rendering (wavelength-dependent materials)
- [ ] Fluorescent/phosphor materials
- [ ] Import STL/OBJ for arbitrary geometry
- [ ] Multi-LED arrays (matrix sources)
- [ ] PDF report generation with LVK plots

## Technical Considerations

### WASM Constraints

- **No threads in baseline WASM** — compute shader approach sidesteps this (GPU parallelism)
- **Memory:** Monte Carlo with 1M photons × ~32 bytes/photon = 32 MB — fine
- **wgpu in WASM:** Works via WebGPU API. Requires WebGPU-capable browser (Chrome, Edge, Firefox Nightly). Fallback to WebGL2 for rendering only (no compute).
- **File download:** Use `web-sys` / `js-sys` for Blob + download trigger

### Performance Targets

- **Rendering:** 60 FPS for 3D scene + photon trails (Bevy handles this)
- **Simulation:** 100k–1M photons/second on GPU (progressive)
- **Convergence:** Typical luminaire LVK converges at ~500k–1M photons
- **Interactive:** User changes material → clear detector → re-trace in <5 seconds

### Accuracy vs. Speed Tradeoffs

This is a demo, not a certified tool. But it should be **physically plausible**:

- Fresnel equations: exact (trivial computation)
- Snell's law refraction: exact
- Lambertian scattering: exact (cosine-weighted hemisphere sampling)
- Opal PMMA scattering: approximate (Henyey-Greenstein is standard but simplified)
- Energy conservation: enforced at every interaction (reflection + transmission + absorption = 1)
- Validation: compare against known analytical cases (e.g., Lambertian emitter → must produce cosine LVK)

### Crate Dependencies (Rust)

```toml
[dependencies]
bevy = "0.15"                  # or latest stable
bevy_egui = "0.33"             # UI panels
eulumdat-rs = { path = "..." } # LDT export
gldf-rs = { path = "..." }    # GLDF export
l3d-rs = { path = "..." }     # 3D model generation
nalgebra = "0.33"              # Vector math
rand = "0.8"                   # RNG for Monte Carlo
bytemuck = "1.14"              # GPU buffer casting

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = ["Blob", "Url", "HtmlAnchorElement"] }
js-sys = "0.3"
```

## Marketing Angle

### Show HN Post

> **Show HN: Virtual Goniophotometer — Watch how lighting data files are born (Rust/WASM)**
>
> I build open-source tools for the lighting industry (eulumdat-rs, gldf-rs).
> A recurring frustration: nobody understands what's inside an .ldt or .ies file,
> even the people who use them daily.
>
> So I built a visual Monte Carlo photon tracer that shows, in real-time,
> how a luminous intensity distribution curve emerges from ray-material interactions.
> Photons fly out of an LED, bounce off housing walls, scatter through a diffuser,
> and accumulate on a virtual goniophotometer sphere. The LVK builds up before
> your eyes, and you can export the result as an industry-standard EULUMDAT file.
>
> Runs entirely in the browser via WASM + WebGPU.
>
> Try it: [link]

### Conference Demo

Perfect for LiTG (Deutsche Lichttechnische Gesellschaft) events, PLDC, or even the Lichtplanerstammtisch where you met Christian. Live demo: "This is what happens inside your custom luminaire."

### Integration with Existing Tools

- **eulumdat.icu:** Add as a feature tab — "Simulate" alongside "View" and "Edit"
- **gldf.icu:** Same — show how the photometric data in a GLDF was (or could be) generated
- **Consulting:** "Here's what your Lichtsäule looks like optically" — powerful visualization for client conversations

## Naming Ideas

- **PhotonForge** — forging light data from first principles
- **LumenTrace** — tracing lumens through geometry
- **GonioSim** — virtual goniophotometer simulator
- **RayForge** — similar to PhotonForge
- **Candela** — the unit itself, simple and direct
