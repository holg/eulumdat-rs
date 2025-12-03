# eulumdat-rs Development Notes

**Location:** `../eulumdat-rs` (relative to gldf-rs)
**Last Updated:** 2025-12-03

## Project Overview

Rust library for parsing, writing, and validating Eulumdat (LDT) and IES photometric files. Now refactored as a **Cargo workspace** with:
- **eulumdat-core** - Core library with diagram data generation and SVG rendering
- **eulumdat-ffi** - UniFFI bindings for Swift, Kotlin, Python
- **eulumdat-wasm** - WebAssembly browser editor (thin wrapper over core)

## Project Structure

```
eulumdat-rs/
├── Cargo.toml                    # Workspace root
├── NOTES.md                      # This file
└── crates/
    ├── eulumdat-core/            # Core library
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs            # Public API exports
    │   │   ├── eulumdat.rs       # Core data structures
    │   │   ├── parser.rs         # LDT file parser
    │   │   ├── writer.rs         # LDT file writer
    │   │   ├── validation.rs     # 44 validation checks
    │   │   ├── symmetry.rs       # Symmetry handling
    │   │   ├── calculations.rs   # Photometric calculations
    │   │   ├── ies.rs            # IES export
    │   │   ├── error.rs          # Error types
    │   │   └── diagram/          # Diagram data + SVG generation
    │   │       ├── mod.rs        # Common types (Point2D, DiagramScale)
    │   │       ├── polar.rs      # Polar diagram (C0-C180, C90-C270)
    │   │       ├── butterfly.rs  # 3D butterfly diagram
    │   │       ├── cartesian.rs  # X-Y intensity curves
    │   │       ├── heatmap.rs    # 2D intensity heatmap
    │   │       ├── color.rs      # Color utilities (HSL, heatmap colors)
    │   │       ├── projection.rs # Isometric 3D projection
    │   │       └── svg.rs        # ✨ SVG rendering with themes
    │   ├── examples/
    │   │   └── parse_road.rs
    │   └── tests/
    │       └── integration_test.rs
    │
    ├── eulumdat-ffi/             # FFI bindings
    │   ├── Cargo.toml
    │   ├── src/lib.rs            # UniFFI exports (data + SVG)
    │   └── uniffi-bindgen.rs     # Binding generator
    │
    └── eulumdat-wasm/            # WebAssembly editor (thin wrappers)
        ├── Cargo.toml
        ├── Trunk.toml
        ├── index.html
        ├── src/
        │   ├── main.rs
        │   └── components/
        │       ├── app.rs
        │       ├── polar_diagram.rs      # ~27 lines, uses core.to_svg()
        │       ├── butterfly_diagram.rs  # ~27 lines, uses core.to_svg()
        │       ├── cartesian_diagram.rs  # ~27 lines, uses core.to_svg()
        │       ├── intensity_heatmap.rs  # ~27 lines, uses core.to_svg()
        │       ├── butterfly_3d.rs       # WebGL version
        │       ├── data_table.rs
        │       ├── tabs.rs
        │       ├── validation_panel.rs
        │       ├── diagram_zoom.rs
        │       ├── file_handler.rs
        │       ├── templates.rs
        │       ├── theme.rs
        │       └── bug_rating.rs
        └── templates/
```

## Current Status

### Core Library (`eulumdat-core`)
- **Status:** Complete and functional
- **Tests:** 45 unit tests + 8 integration tests + 5 doc tests = **58 tests passing**
- **Features:**
  - Full LDT parsing with European decimal format
  - LDT file writing (roundtrip tested)
  - IES export (IESNA LM-63-2002)
  - 44 validation constraints (W001-W044)
  - 5 symmetry types with automatic data expansion
  - Photometric calculations
  - **Diagram data generation** (platform-independent):
    - `PolarDiagram` - C0-C180 and C90-C270 curves
    - `ButterflyDiagram` - 3D isometric projection
    - `CartesianDiagram` - Intensity vs gamma curves
    - `HeatmapDiagram` - 2D intensity grid
  - **BUG Rating** (IESNA TM-15-11 & TM-15-07):
    - `ZoneLumens` - Zone lumens calculation
    - `BugRating` - B/U/G rating calculation
    - `BugDiagram` - Complete diagram with SVG generation
  - **SVG rendering** with theming:
    - `to_svg()` methods on all diagram types
    - `SvgTheme::light()`, `SvgTheme::dark()`, `SvgTheme::css_variables()`

### FFI Bindings (`eulumdat-ffi`)
- **Status:** Compiles, ready for binding generation
- **UniFFI 0.28** - Generates Swift, Kotlin, Python bindings
- **Data Exports:**
  - `parse_ldt()` - Parse LDT content
  - `generate_polar_diagram()` - Get polar diagram data
  - `generate_butterfly_diagram()` - Get 3D butterfly data
  - `generate_cartesian_diagram()` - Get cartesian curves
  - `generate_heatmap_diagram()` - Get heatmap cells
- **SVG Exports:**
  - `generate_polar_svg()` - Get polar diagram as SVG string
  - `generate_cartesian_svg()` - Get cartesian diagram as SVG string
  - `generate_heatmap_svg()` - Get heatmap as SVG string
  - `generate_butterfly_svg()` - Get butterfly diagram as SVG string
  - `generate_bug_svg()` - Get BUG rating diagram as SVG (TM-15-11)
  - `generate_lcs_svg()` - Get LCS diagram as SVG (TM-15-07)
- **BUG Rating:**
  - `calculate_bug_rating()` - Get B/U/G rating values
  - `generate_bug_diagram()` - Get complete BUG diagram data

### WASM Editor (`eulumdat-wasm`)
- **Status:** Working browser-based editor
- **Framework:** Yew 0.21 with CSR
- **Architecture:** Thin wrappers (~27 lines each) calling `core.to_svg()`

## Build Commands

```bash
# Full workspace
cd eulumdat-rs
cargo build --workspace
cargo test --workspace

# Core library only
cargo build --package eulumdat-core
cargo test --package eulumdat-core

# FFI library
cargo build --package eulumdat-ffi --release

# Generate Swift bindings
cargo run --package eulumdat-ffi --bin uniffi-bindgen -- \
  generate --library target/release/libeulumdat_ffi.dylib \
  --language swift --out-dir ./bindings/swift

# WASM editor
cd crates/eulumdat-wasm
trunk serve        # Development server
trunk build        # Production build
```

## SVG Generation Usage

```rust
use eulumdat_core::{Eulumdat, diagram::*};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Generate complete SVG strings (ready to render)
let polar = PolarDiagram::from_eulumdat(&ldt);
let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());

// Dark theme
let svg_dark = polar.to_svg(500.0, 500.0, &SvgTheme::dark());

// CSS variables for dynamic theming (web)
let svg_css = polar.to_svg(500.0, 500.0, &SvgTheme::css_variables());

// Other diagram types
let cartesian = CartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);
let svg = cartesian.to_svg(500.0, 380.0, &SvgTheme::light());

let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
let svg = heatmap.to_svg(700.0, 500.0, &SvgTheme::light());

let butterfly = ButterflyDiagram::from_eulumdat(&ldt, 500.0, 450.0, 60.0);
let svg = butterfly.to_svg(500.0, 450.0, &SvgTheme::light());
```

## FFI Usage (Swift Example)

```swift
import EulumdatFFI

// Parse LDT file
let content = try String(contentsOfFile: "luminaire.ldt")
let ldt = try parseLdt(content: content)

// Generate SVG directly (recommended for rendering)
let polarSvg = generatePolarSvg(ldt: ldt, width: 500, height: 500, theme: .light)
// Use polarSvg in WKWebView or SVG rendering library

// Or get raw data for custom rendering
let polar = generatePolarDiagram(ldt: ldt)
for point in polar.c0C180Curve.points {
    print("γ=\(point.gamma)° I=\(point.intensity)")
}
```

## Key Types

```rust
// Core data structure
pub struct Eulumdat {
    pub luminaire_name: String,
    pub symmetry: Symmetry,
    pub c_angles: Vec<f64>,
    pub g_angles: Vec<f64>,
    pub intensities: Vec<Vec<f64>>,  // [C-plane][gamma] cd/klm
    // ...
}

// SVG Theme configuration
pub struct SvgTheme {
    pub background: String,
    pub surface: String,
    pub grid: String,
    pub axis: String,
    pub text: String,
    pub curve_c0_c180: String,
    pub curve_c90_c270: String,
    // ...
}

// Diagram types (all have to_svg() method)
pub struct PolarDiagram {
    pub c0_c180_curve: PolarCurve,
    pub c90_c270_curve: PolarCurve,
    pub scale: DiagramScale,
}

pub struct ButterflyDiagram {
    pub wings: Vec<ButterflyWing>,
    pub grid_circles: Vec<Vec<Point2D>>,
}

pub struct CartesianDiagram {
    pub curves: Vec<CartesianCurve>,
    pub x_ticks: Vec<f64>,
    pub y_ticks: Vec<f64>,
}

pub struct HeatmapDiagram {
    pub cells: Vec<HeatmapCell>,
    pub legend_entries: Vec<(f64, Color, f64)>,
}
```

## Architecture Benefits

1. **Platform Independence** - All logic in core, platform code just renders SVG strings
2. **Reusability** - Same SVG output for WASM, iOS, Android, desktop
3. **FFI Ready** - UniFFI generates type-safe bindings with SVG functions
4. **Testable** - All diagram logic unit tested (26 diagram tests)
5. **Theming** - Light/dark/CSS variable themes built-in

## Refactoring Summary (2025-12-03)

Before refactoring:
- WASM components: ~1600 lines of duplicate diagram/SVG logic
- FFI: Data-only exports

After refactoring:
- Core: Added `svg.rs` module with `to_svg()` methods and `SvgTheme`
- WASM components: ~108 lines total (thin wrappers)
- FFI: Added `generate_*_svg()` functions

**Code reduction: ~1500 lines removed from WASM, logic centralized in core**

## Next Steps / TODO

1. **iOS/Android apps** - Use eulumdat-ffi SVG generation
2. **IES Parser** - Currently only export; add full IES parsing
3. **Publish crates** - eulumdat-core to crates.io
4. **WASM improvements:**
   - Data table editing
   - Undo/redo support
   - Multiple file comparison

## Dependencies

### eulumdat-core
- `anyhow` - Error handling
- `serde` (optional) - Serialization

### eulumdat-ffi
- `uniffi` 0.28 - FFI binding generation
- `thiserror` - Error types (required by UniFFI)

### eulumdat-wasm
- `yew` 0.21 - Web framework
- `gloo` - Browser APIs
- `web-sys` - Web API bindings

## Credits

Ported from [QLumEdit](https://github.com/kstrug/QLumEdit) by Krzysztof Strugiński.
