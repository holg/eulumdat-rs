# eulumdat-rs

A Rust workspace for parsing, writing, and analyzing **EULUMDAT (LDT)** and **IES** photometric files.

[![Crates.io](https://img.shields.io/crates/v/eulumdat.svg)](https://crates.io/crates/eulumdat)
[![Documentation](https://docs.rs/eulumdat/badge.svg)](https://docs.rs/eulumdat)
[![License](https://img.shields.io/crates/l/eulumdat.svg)](https://github.com/holg/eulumdat-rs#license)

## Crates

| Crate | Description |
|-------|-------------|
| [eulumdat](crates/eulumdat) | Core library for parsing, validation, and calculations |
| [eulumdat-cli](crates/eulumdat-cli) | Command-line tool |
| [eulumdat-py](crates/eulumdat-py) | Python bindings (PyO3) |
| [eulumdat-ffi](crates/eulumdat-ffi) | FFI bindings (UniFFI) for Swift, Kotlin, etc. |
| [eulumdat-wasm](crates/eulumdat-wasm) | WebAssembly editor |

## Features

- **Parse LDT/IES files** - Full EULUMDAT and IESNA LM-63 format support
- **Write LDT files** - Roundtrip-tested output generation
- **Export to IES** - Convert EULUMDAT to IES format
- **Validation** - 44 validation constraints based on official specification
- **Symmetry handling** - 5 symmetry types with automatic data expansion
- **Photometric calculations** - Downward flux, beam angles, utilization factors
- **BUG Rating** - IESNA TM-15-11 Backlight-Uplight-Glare calculations
- **Diagram generation** - Polar, Butterfly, Cartesian, Heatmap with SVG export

## Installation

### Rust Library

```toml
[dependencies]
eulumdat = "0.2"
```

### Command-Line Tool

```bash
cargo install eulumdat-cli
```

### Python

```bash
pip install eulumdat
```

### Swift (SPM)

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/holg/eulumdat-rs", from: "0.2.0")
]
```

## Quick Start

### Rust

```rust
use eulumdat::{Eulumdat, BugDiagram, diagram::SvgTheme};

// Parse from file
let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Access data
println!("Luminaire: {}", ldt.luminaire_name);
println!("Max intensity: {:.1} cd/klm", ldt.max_intensity());

// Validate
for warning in ldt.validate() {
    println!("[{}] {}", warning.code, warning.message);
}

// Generate polar diagram SVG
let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
let svg = polar.to_svg(500.0, 500.0, &SvgTheme::light());

// Calculate BUG rating
let bug = BugDiagram::from_eulumdat(&ldt);
println!("BUG Rating: {}", bug.rating);
```

### Python

```python
import eulumdat

# Parse from file or string
ldt = eulumdat.Eulumdat.from_file("luminaire.ldt")

# Access data
print(f"Luminaire: {ldt.luminaire_name}")
print(f"Max intensity: {ldt.max_intensity():.1f} cd/klm")

# Generate SVG diagrams
polar_svg = ldt.polar_svg(width=500, height=500)
bug_svg = ldt.bug_svg()

# Calculate BUG rating
rating = ldt.bug_rating()
print(f"BUG Rating: {rating}")
```

### Swift

```swift
import Eulumdat

// Parse LDT content
let ldt = try parseLdt(content: ldtString)

// Access data
print("Luminaire: \(ldt.luminaireName)")
print("Max intensity: \(ldt.maxIntensity) cd/klm")

// Generate SVG diagrams
let polarSvg = generatePolarSvg(ldt: ldt, width: 500, height: 500, theme: .light)
let bugSvg = generateBugSvg(ldt: ldt, width: 400, height: 350, theme: .dark)

// Calculate BUG rating
let rating = calculateBugRating(ldt: ldt)
print("BUG Rating: B\(rating.b) U\(rating.u) G\(rating.g)")
```

### CLI

```bash
# Display file information
eulumdat info luminaire.ldt

# Validate
eulumdat validate luminaire.ldt

# Convert LDT to IES
eulumdat convert luminaire.ldt output.ies

# Generate diagram
eulumdat diagram luminaire.ldt -t polar -o polar.svg

# Calculate BUG rating
eulumdat bug outdoor_luminaire.ldt --svg bug.svg
```

## Diagram Types

| Type | Description |
|------|-------------|
| Polar | Classic intensity distribution (C0-C180, C90-C270 curves) |
| Butterfly | 3D isometric projection |
| Cartesian | Intensity vs gamma angle |
| Heatmap | 2D intensity color map |
| BUG | IESNA TM-15-11 zone visualization |
| LCS | TM-15-07 Luminaire Classification System |

## References

- [EULUMDAT Format (Paul Bourke)](https://paulbourke.net/dataformats/ldt/)
- [IESNA LM-63-2002](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/iesna_lm-63_format.htm)
- [IES TM-15-11 BUG Ratings](https://www.ies.org/wp-content/uploads/2017/03/TM-15-11BUGRatingsAddendum.pdf)

## License

MIT OR Apache-2.0
