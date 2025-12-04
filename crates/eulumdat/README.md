# eulumdat

A Rust library for parsing, writing, and validating **EULUMDAT (LDT)** and **IES** photometric files.

[![Crates.io](https://img.shields.io/crates/v/eulumdat.svg)](https://crates.io/crates/eulumdat)
[![Documentation](https://docs.rs/eulumdat/badge.svg)](https://docs.rs/eulumdat)
[![License](https://img.shields.io/crates/l/eulumdat.svg)](https://github.com/holg/eulumdat-rs#license)

## About EULUMDAT

**EULUMDAT** (European Lumen Data format) is a photometric data file format used primarily in Europe for specifying luminous intensity distributions from light sources. The format was proposed by [Axel Stockmar](https://paulbourke.net/dataformats/ldt/) (Light Consult Inc., Berlin) in 1990 and uses the `.ldt` file extension.

While the **.IES** format (IESNA LM-63) is predominantly used in North America, EULUMDAT/LDT is the standard across Europe. Both formats describe the same photometric data but use different conventions.

### Format Comparison

| Feature | EULUMDAT (.ldt) | IES (.ies) |
|---------|-----------------|------------|
| Region | Europe | North America |
| Standard | Stockmar 1990 | IESNA LM-63-2002 |
| Angle convention | C-planes (0°-360°) | Horizontal/Vertical |
| Decimal separator | Comma or period | Period |
| Typical C-planes | 24 (interior), 36 (road) | Varies |

## Features

- **Parse LDT files** - Full EULUMDAT format support with European decimal handling
- **Write LDT files** - Roundtrip-tested output generation
- **Export to IES** - IESNA LM-63-2002 format export
- **Validation** - 44 validation constraints with detailed warnings
- **Symmetry handling** - 5 symmetry types with automatic data expansion
- **Photometric calculations** - Downward flux, beam angles, utilization factors
- **BUG Rating** - IESNA TM-15-11 Backlight-Uplight-Glare calculations
- **Diagram generation** - Platform-independent data for visualizations:
  - Polar diagrams (C0-C180, C90-C270 curves)
  - Butterfly diagrams (3D isometric projection)
  - Cartesian diagrams (intensity vs gamma)
  - Heatmap diagrams (2D intensity grid)
- **SVG rendering** - Built-in SVG generation with light/dark theming

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
eulumdat = "0.2"
```

With serde support:

```toml
[dependencies]
eulumdat = { version = "0.2", features = ["serde"] }
```

## Quick Start

```rust
use eulumdat::{Eulumdat, IesExporter};

// Parse from file
let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Access photometric data
println!("Luminaire: {}", ldt.luminaire_name);
println!("Symmetry: {:?}", ldt.symmetry);
println!("C-planes: {}", ldt.c_angles.len());
println!("Gamma angles: {}", ldt.g_angles.len());
println!("Max intensity: {} cd/klm",
    ldt.intensities.iter().flatten().cloned().fold(0.0_f64, f64::max));

// Validate the data
for warning in ldt.validate() {
    println!("Warning [{}]: {}", warning.code, warning.message);
}

// Export to IES format
let ies = IesExporter::export(&ldt);
std::fs::write("luminaire.ies", ies)?;
```

## EULUMDAT File Structure

The EULUMDAT format is a plain ASCII text file:

| Line | Field | Description |
|------|-------|-------------|
| 1 | Identification | Company/database/version (max 78 chars) |
| 2 | Ityp | Type indicator (0=point, 1=vertical axis, 2=linear, 3=other) |
| 3 | Isym | Symmetry indicator (0-4, see below) |
| 4 | Mc | Number of C-planes |
| 5 | Dc | Distance between C-planes (°), 0 if non-equidistant |
| 6 | Ng | Number of gamma angles per C-plane |
| 7 | Dg | Distance between gamma angles (°) |
| 8-12 | Metadata | Certificate, luminaire info, filename |
| 13-21 | Dimensions | Luminaire and luminous area dimensions (mm) |
| 22-26 | Photometric | DFF, LORL, conversion factor, tilt |
| 26a-b | Lamp sets | Number of lamp sets + lamp data |
| 27 | Direct ratios | 10 utilization factor values |
| 28 | C-angles | Mc angle values starting at 0° |
| 29 | G-angles | Ng angle values starting at 0° |
| 30+ | Intensities | Mc × Ng intensity values (cd/klm) |

### Symmetry Types (Isym)

| Value | Name | C-plane Range | Data Coverage |
|-------|------|---------------|---------------|
| 0 | None | 0° - 360° | Full distribution |
| 1 | Vertical axis | Single plane | Rotationally symmetric |
| 2 | C0-C180 | 0° - 180° | Mirror at C0-C180 plane |
| 3 | C90-C270 | 90° - 270° | Mirror at C90-C270 plane |
| 4 | Both planes | 0° - 90° | Quadrant symmetry |

## Diagram Generation

Generate platform-independent diagram data for visualization:

```rust
use eulumdat::{Eulumdat, diagram::*};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Polar diagram - classic intensity distribution view
let polar = PolarDiagram::from_eulumdat(&ldt);
let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());

// Butterfly diagram - 3D isometric projection
let butterfly = ButterflyDiagram::from_eulumdat(&ldt, 500.0, 400.0, 60.0);
let svg = butterfly.to_svg(500.0, 400.0, &SvgTheme::dark());

// Cartesian diagram - intensity vs gamma angle (max 8 curves)
let cartesian = CartesianDiagram::from_eulumdat(&ldt, 600.0, 400.0, 8);
let svg = cartesian.to_svg(600.0, 400.0, &SvgTheme::light());

// Heatmap diagram - 2D intensity color map
let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
let svg = heatmap.to_svg(700.0, 500.0, &SvgTheme::dark());
```

### Theming

```rust
use eulumdat::diagram::SvgTheme;

// Predefined themes
let light = SvgTheme::light();
let dark = SvgTheme::dark();

// CSS variables for dynamic theming (web applications)
let css_vars = SvgTheme::css_variables();
```

## BUG Rating (IESNA TM-15-11)

Calculate and visualize the **B**acklight-**U**plight-**G**lare rating for outdoor luminaires:

```rust
use eulumdat::{Eulumdat, BugDiagram, diagram::SvgTheme};

let ldt = Eulumdat::from_file("outdoor_luminaire.ldt")?;

// Calculate BUG rating
let bug = BugDiagram::from_eulumdat(&ldt);
println!("BUG Rating: {}", bug.rating); // e.g., "B2 U0 G3"

// Access zone lumens
println!("Backlight zones: BL={:.0} BM={:.0} BH={:.0} BVH={:.0}",
    bug.zones.bl, bug.zones.bm, bug.zones.bh, bug.zones.bvh);
println!("Uplight zones: UL={:.0} UH={:.0}",
    bug.zones.ul, bug.zones.uh);

// Generate TM-15-11 BUG visualization
let bug_svg = bug.to_svg(400.0, 350.0, &SvgTheme::light());

// Generate TM-15-07 LCS (Luminaire Classification System) diagram
let lcs_svg = bug.to_lcs_svg(510.0, 315.0, &SvgTheme::light());
```

### BUG Rating Scale

| Rating | Meaning |
|--------|---------|
| B0-B1 | Excellent backlight control |
| B2 | Good backlight control |
| B3-B5 | Increasing backlight |
| U0 | No uplight (full cutoff) |
| U1-U5 | Increasing uplight |
| G0-G1 | Excellent glare control |
| G2-G5 | Increasing high-angle glare |

The BUG system is used in:
- IDA/IES Model Lighting Ordinance
- LEED v4 Light Pollution Reduction credit
- California Title 24 Energy Code (2022)

## FFI Bindings

For Swift, Kotlin, and Python bindings, see [eulumdat-ffi](https://crates.io/crates/eulumdat-ffi).

## References

### EULUMDAT Format
- [Paul Bourke's EULUMDAT Documentation](https://paulbourke.net/dataformats/ldt/)
- [AGi32 EULUMDAT File Format](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/eulumdat_file_format.htm)
- [DIALux EULUMDAT Format Description](https://evo.support-en.dial.de/support/solutions/articles/9000074164-description-of-the-eulumdat-format)
- Stockmar, A. W. (1990): "EULUMDAT - ein Leuchtendatenformat für den europäischen Beleuchtungsplaner", Tagungsband Licht '90, pp. 641-644.
- Stockmar, A. W. (1998): "EULUMDAT/2 - Extended Version of a Well Established Luminaire Data Format", CIBSE National Lighting Conference 1998, pp. 353-362.

### IES Format
- [AGi32 IESNA LM-63 Format](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/iesna_lm-63_format.htm)
- ANSI/IESNA LM-63-2002: Standard File Format for Electronic Transfer of Photometric Data

### BUG Rating
- [IES TM-15-11 BUG Ratings Addendum (PDF)](https://www.ies.org/wp-content/uploads/2017/03/TM-15-11BUGRatingsAddendum.pdf)
- [NaturaLED BUG Ratings Explained](https://naturaled.com/bug-ratings-explained/)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
