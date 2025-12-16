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
| [eulumdat-egui](crates/eulumdat-egui) | Cross-platform desktop GUI (Windows, macOS, Linux) |
| [eulumdat-py](crates/eulumdat-py) | Python bindings (PyO3) |
| [eulumdat-ffi](crates/eulumdat-ffi) | FFI bindings (UniFFI) for Swift, Kotlin, etc. |
| [eulumdat-wasm](crates/eulumdat-wasm) | WebAssembly editor |
| [eulumdat-windows-preview](crates/eulumdat-windows-preview) | Windows Shell Preview Handler for File Explorer |

## Applications

| Platform | Description | Status |
|----------|-------------|--------|
| **macOS/iOS** | Native SwiftUI app with **QuickLook extension** | Available |
| **Android** | Native Jetpack Compose app with Material 3 | Available |
| **Desktop (egui)** | Cross-platform GUI (Windows, macOS, Linux) | Available |
| **Windows Preview** | File Explorer preview pane integration | Available |
| **Web** | Browser-based editor via WebAssembly | Beta |

### macOS QuickLook Extension

Preview LDT files directly in Finder with polar diagrams - no need to open an app!

- **Finder Preview**: Select any `.ldt` file and press Space for instant preview
- **Quick Look**: Beautiful polar diagram visualization in Finder's preview pane
- **Universal Binary**: Native support for Apple Silicon and Intel Macs

## Features

- **Parse LDT/IES files** - Full EULUMDAT and IESNA LM-63 format support
- **Write LDT files** - Roundtrip-tested output generation
- **Export to IES** - Convert EULUMDAT to IES format
- **Batch conversion** - Efficient bulk processing of multiple files
- **Validation** - 44 validation constraints based on official specification
- **Symmetry handling** - 5 symmetry types with automatic data expansion
- **Photometric calculations** - CIE flux codes, beam/field angles, spacing criteria, zonal lumens, UGR
- **GLDF export** - Global Lighting Data Format compatible photometric data export
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

### Desktop GUI

```bash
cargo install eulumdat-egui
eulumdat-egui  # Launch the GUI
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

# NEW: Photometric calculations
summary = ldt.photometric_summary()
print(summary.to_text())  # Full report
print(summary.to_compact())  # One-liner

# NEW: Individual calculations
cie = ldt.cie_flux_codes()
print(f"CIE Flux Code: {cie}")  # e.g., "100 77 43 0 0"
print(f"Beam angle: {ldt.beam_angle():.1f}°")
print(f"Field angle: {ldt.field_angle():.1f}°")
print(f"Spacing: {ldt.spacing_criteria()}")  # (S/H C0, S/H C90)

# NEW: GLDF export
gldf = ldt.gldf_data()
print(gldf.to_dict())  # For JSON serialization

# Batch conversion (efficient bulk processing)
from eulumdat import BatchInput, ConversionFormat, batch_convert

inputs = [
    BatchInput("file1.ldt", ldt_content1),
    BatchInput("file2.ldt", ldt_content2),
]
outputs, stats = batch_convert(inputs, ConversionFormat.Ies)
print(f"Converted {stats.successful}/{stats.total} files")
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

# Batch convert multiple files
eulumdat batch input_folder/ -o output_folder/ -f ies

# NEW: Photometric summary (text, compact, or JSON)
eulumdat summary luminaire.ldt
eulumdat summary luminaire.ldt -f json -o summary.json

# NEW: GLDF-compatible export
eulumdat gldf luminaire.ldt --pretty -o gldf_data.json

# NEW: Specific calculations
eulumdat calc luminaire.ldt -t cie-codes      # CIE flux codes
eulumdat calc luminaire.ldt -t beam-angles    # Beam/field angles
eulumdat calc luminaire.ldt -t spacing        # S/H ratios
eulumdat calc luminaire.ldt -t zonal-lumens   # 30° zone distribution
eulumdat calc luminaire.ldt -t all            # Everything
```

### macOS / iOS

Native SwiftUI app available in the [EulumdatApp](EulumdatApp/) directory.

```bash
# Build and run with Xcode
cd EulumdatApp
open EulumdatApp.xcodeproj
```

Features:
- **Universal app** - Single binary for macOS (Intel + Apple Silicon) and iOS
- **QuickLook Extension** - Preview .ldt files directly in Finder (press Space)
- **All diagram types** - Polar, Butterfly, Cartesian, Heatmap, BUG, LCS
- **Interactive 3D view** - Rotate and zoom butterfly diagrams
- **Template library** - Built-in sample luminaires for testing
- **Full validation** - All 44 specification constraints checked
- **Export options** - SVG, IES, LDT formats
- **Intensity table** - Copy as CSV, toggle color highlighting

### Android

Native Jetpack Compose app available in the [EulumdatAndroid](EulumdatAndroid/) directory.

```bash
# Build with Gradle
cd EulumdatAndroid
./gradlew assembleDebug
```

Features:
- **Material 3 design** - Modern Android UI following latest guidelines
- **All diagram types** - Polar, Butterfly, Cartesian, Heatmap, BUG, LCS
- **Interactive 3D view** - Touch gestures to rotate butterfly diagrams
- **Template library** - Sample luminaires included
- **File picker** - Open .ldt/.ies from device storage
- **Share & Export** - SVG export with Android share sheet
- **Multi-architecture** - ARM64, ARMv7, x86_64 native libraries

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
