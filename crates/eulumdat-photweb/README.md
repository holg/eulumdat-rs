# eulumdat-photweb

[![Crates.io](https://img.shields.io/crates/v/eulumdat-photweb.svg)](https://crates.io/crates/eulumdat-photweb)
[![Documentation](https://docs.rs/eulumdat-photweb/badge.svg)](https://docs.rs/eulumdat-photweb)
[![License](https://img.shields.io/crates/l/eulumdat-photweb.svg)](https://github.com/holg/eulumdat-rs)

Photometric web representation, sampling, and 3D mesh generation for EULUMDAT/IES lighting data.

This crate extends the [`eulumdat`](https://crates.io/crates/eulumdat) library with advanced sampling and 3D visualization capabilities for photometric data.

## Features

- **Bilinear Interpolation**: Sample intensity at any C-plane/gamma angle with smooth interpolation
- **Symmetry Handling**: Automatic expansion based on symmetry type (C0-C180, C90-C270, both planes, vertical axis)
- **Normalized Sampling**: Get intensity values normalized to 0.0-1.0 range
- **3D Mesh Generation**: Generate LDC (Luminous Distribution Curve) solid meshes for 3D visualization
- **Graphics-Ready Output**: Flat arrays for positions, normals, and indices

## Quick Start

```rust
use eulumdat::Eulumdat;
use eulumdat_photweb::PhotometricWeb;

// Load photometric data
let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Create a photometric web
let web = PhotometricWeb::from(&ldt);

// Sample intensity at any angle (with bilinear interpolation)
let intensity = web.sample(45.0, 30.0);  // C=45°, γ=30°
println!("Intensity at C45/G30: {} cd/klm", intensity);

// Sample normalized (0.0 to 1.0, relative to max intensity)
let normalized = web.sample_normalized(45.0, 30.0);
println!("Normalized: {:.2}", normalized);

// Check symmetry and bounds
println!("Max intensity: {} cd/klm", web.max_intensity());
println!("Symmetry: {:?}", web.symmetry());
```

## 3D Mesh Generation

Generate photometric solids (LDC - Luminous Distribution Curve) for 3D visualization:

```rust
use eulumdat::Eulumdat;
use eulumdat_photweb::{PhotometricWeb, LdcMesh};

let ldt = Eulumdat::from_file("luminaire.ldt")?;
let web = PhotometricWeb::from(&ldt);

// Generate mesh with 5° C-plane steps, 5° gamma steps, scale=1.0
let mesh = web.generate_ldc_mesh(5.0, 5.0, 1.0);

println!("Vertices: {}", mesh.vertex_count());
println!("Triangles: {}", mesh.triangle_count());

// Get data for graphics APIs (OpenGL, WebGPU, etc.)
let positions: Vec<f32> = mesh.positions_flat();  // [x0, y0, z0, x1, y1, z1, ...]
let normals: Vec<f32> = mesh.normals_flat();      // [nx0, ny0, nz0, ...]
let indices: &[u32] = &mesh.indices;              // Triangle indices
```

### Coordinate System

The generated mesh uses a Y-up coordinate system:
- **Y axis**: Up (nadir at -Y, zenith at +Y)
- **X-Z plane**: Horizontal
- **C=0°**: Along +Z axis
- **C=90°**: Along +X axis
- **γ=0°**: Nadir (straight down, -Y)
- **γ=90°**: Horizontal (X-Z plane)
- **γ=180°**: Zenith (straight up, +Y)

## Symmetry Support

The `PhotometricWeb` automatically handles all EULUMDAT symmetry types:

| Symmetry | Description | Stored C-planes |
|----------|-------------|-----------------|
| `None` | Full 360° data | 0°-360° |
| `VerticalAxis` | Rotationally symmetric | Single plane |
| `PlaneC0C180` | Mirror across C0-C180 | 0°-180° |
| `PlaneC90C270` | Mirror across C90-C270 | 90°-270° |
| `BothPlanes` | Quarter data | 0°-90° |

You can query any angle regardless of stored symmetry:

```rust
// Even with BothPlanes symmetry (only 0-90° stored),
// you can sample the full 360° range
let i_c0 = web.sample(0.0, 45.0);
let i_c180 = web.sample(180.0, 45.0);
let i_c270 = web.sample(270.0, 45.0);
// These are automatically mirrored from stored data
```

## Use Cases

- **3D Visualization**: Generate meshes for SceneKit, Three.js, Babylon.js, etc.
- **Lighting Simulation**: Sample intensity at arbitrary angles for ray tracing
- **Analysis Tools**: Compute custom metrics by sampling the photometric distribution
- **Export**: Generate mesh data for CAD/BIM software

## Integration Examples

### With wgpu/WebGPU

```rust
let mesh = web.generate_ldc_mesh(5.0, 5.0, 1.0);

// Create vertex buffer
let vertex_data: Vec<f32> = mesh.vertices
    .iter()
    .flat_map(|v| [v.x, v.y, v.z, v.nx, v.ny, v.nz])
    .collect();

// Create index buffer
let index_data: &[u32] = &mesh.indices;
```

### With Three.js (via WASM)

```javascript
const mesh = photweb.generate_ldc_mesh(5.0, 5.0, 1.0);
const geometry = new THREE.BufferGeometry();
geometry.setAttribute('position', new THREE.Float32BufferAttribute(mesh.positions_flat(), 3));
geometry.setAttribute('normal', new THREE.Float32BufferAttribute(mesh.normals_flat(), 3));
geometry.setIndex(Array.from(mesh.indices));
```

## Related Crates

- [`eulumdat`](https://crates.io/crates/eulumdat) - Core EULUMDAT/IES parsing library
- [`eulumdat-cli`](https://crates.io/crates/eulumdat-cli) - Command-line tool
- [`eulumdat-wasm`](https://crates.io/crates/eulumdat-wasm) - WebAssembly bindings

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
