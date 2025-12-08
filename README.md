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

## Applications

| Platform | Description |
|----------|-------------|
| **Desktop (egui)** | Cross-platform native app via `cargo install eulumdat-egui` |
| **macOS/iOS** | Native SwiftUI app in [EulumdatApp](EulumdatApp/) |
| **Android** | Native Jetpack Compose app in [EulumdatAndroid](EulumdatAndroid/) |
| **Web** | Browser-based editor via WebAssembly |

## Features

- **Parse LDT/IES files** - Full EULUMDAT and IESNA LM-63 format support
- **Write LDT files** - Roundtrip-tested output generation
- **Export to IES** - Convert EULUMDAT to IES format
- **Batch conversion** - Efficient bulk processing of multiple files
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

### Desktop GUI

```bash
cargo install eulumdat-egui
eulumdat-viewer  # Launch the GUI
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
```

### macOS / iOS

Native SwiftUI app available in the [EulumdatApp](EulumdatApp/) directory.

```bash
# Build and run with Xcode
cd EulumdatApp
open EulumdatApp.xcodeproj
```

Features:
- Universal app (macOS + iOS)
- All diagram types with interactive viewing
- QuickLook extension for .ldt files
- Template library for quick testing
- Export to SVG, IES, LDT
- Intensity table with CSV copy and color toggle

### Android

Native Jetpack Compose app available in the [EulumdatAndroid](EulumdatAndroid/) directory.

```bash
# Build with Gradle
cd EulumdatAndroid
./gradlew assembleDebug
```

Features:
- Material 3 design
- All diagram types including interactive 3D view
- Template library
- File picker for .ldt/.ies files
- Export functionality

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
