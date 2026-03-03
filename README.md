# eulumdat-rs

A Rust workspace for parsing, writing, and analyzing photometric files: **EULUMDAT (LDT)**, **IES (LM-63)**, and **ATLA/TM-33** formats.

**Internally uses ATLA-S001 as the unified photometric data model** - the most comprehensive and permissive format that supports spectral data, greenhouse metrics, and seamless conversion between all formats.

[![Crates.io](https://img.shields.io/crates/v/eulumdat.svg)](https://crates.io/crates/eulumdat)
[![Documentation](https://docs.rs/eulumdat/badge.svg)](https://docs.rs/eulumdat)
[![License](https://img.shields.io/crates/l/eulumdat.svg)](https://github.com/holg/eulumdat-rs#license)

**Live demo:** [eulumdat.icu](https://eulumdat.icu)

## Crates

| Crate | Description |
|-------|-------------|
| [atla](crates/atla) | ATLA-S001/TM-33-23 unified photometric data model with spectral + BIM support |
| [eulumdat](crates/eulumdat) | Core library: parsing, validation, calculations, diagrams, comparison |
| [eulumdat-cli](crates/eulumdat-cli) | Command-line tool |
| [eulumdat-typst](crates/eulumdat-typst) | Typst report generation (single-file + comparison reports) |
| [eulumdat-wasm](crates/eulumdat-wasm) | WebAssembly editor with Google Maps designer |
| [eulumdat-bevy](crates/eulumdat-bevy) | 3D scene viewer (Bevy engine, native + WASM) |
| [eulumdat-egui](crates/eulumdat-egui) | Cross-platform desktop GUI (Windows, macOS, Linux) |
| [eulumdat-py](crates/eulumdat-py) | Python bindings (PyO3) |
| [eulumdat-ffi](crates/eulumdat-ffi) | FFI bindings (UniFFI) for Swift, Kotlin, etc. |
| [eulumdat-server](crates/eulumdat-server) | REST API server for photometric analysis |
| [eulumdat-plugin](crates/eulumdat-plugin) | Plugin system for custom analysis engines |
| [eulumdat-windows-preview](crates/eulumdat-windows-preview) | Windows Shell Preview Handler for File Explorer |

## Applications

| Platform | Description | Status |
|----------|-------------|--------|
| **Web** | Browser-based editor with 3D viewer, Maps designer, BIM panel | [eulumdat.icu](https://eulumdat.icu) |
| **macOS/iOS** | Native SwiftUI app with **QuickLook extension** | Available |
| **Android** | Native Jetpack Compose app with Material 3 | Available |
| **Desktop (egui)** | Cross-platform GUI (Windows, macOS, Linux) | Available |
| **Windows Preview** | File Explorer preview pane integration | Available |
| **HarmonyOS** | OpenHarmony/HarmonyOS app (ArkTS + native Rust) | Available |

## What's New in 0.5.0

### TM-32-24 BIM Parameters

Full ANSI/IES TM-32-24 Building Information Modeling support, extracting **100+ BIM fields** across 13 categories from TM-33-23 XML files:

- **Photometric** - CCT, CRI, TM-30 Rf/Rg, BUG rating, cutoff angle, melanopic factor
- **Electrical** - watts, power factor, voltage, DALI/DMX dimming, LED drive type
- **Mechanical** - housing dimensions, weight, material, IP rating, IK rating
- **Mounting** - ceiling/wall/floor/pendant/recessed/track/pole, cutout dimensions
- **Emergency** - maintained/non-maintained, battery type/capacity
- **Acoustics** - NRC value, frequency-band absorption table
- **Sensors** - PIR/microwave detection, coverage area
- **Maintenance** - LLD, LDD, LSF, projected lamp life

Cross-references **NEMA BIM 100-2021** GUIDs for direct Revit shared parameter import. Export as CSV or text report.

### New Diagram Types

| Type | Description |
|------|-------------|
| **Isocandela** | Type B H/V contour plot with marching-squares contour lines at 10-90% of I_max |
| **Isolux Footprint** | Ground-plane lux heatmap with adjustable mounting height (3-30m), tilt (0-80°), and area size |
| **Floodlight V-H** | H-plane + V-plane Cartesian curves (Type B) with NEMA beam classification badge |

These join the existing set: Polar, Butterfly, Cartesian, Heatmap, BUG, LCS, Cone, Beam Angle, Spectral, Greenhouse.

### Google Maps Lighting Designer

Place luminaires on satellite imagery, draw coverage polygons, and compute point-by-point illuminance from actual photometric data:

- Bilinear interpolation on the intensity grid, inverse-square law with cosine correction
- Configurable mounting height, tilt, and rotation per luminaire
- Adjustable grid spacing (0.5-5m)
- Color-coded heatmap overlay with lux value labels
- Results: min/max/avg lux, uniformity U_0
- Export grid data as CSV
- Everything runs client-side in the browser

### 36-Metric Comparison Engine

Compare two photometric files across **36 weighted metrics** with significance classification:

- Flux & efficiency (8), IES/CIE beam angles (4), intensity stats (3)
- Zonal lumens 0-180° (6), CIE flux codes N1-N5 (5), spacing criteria (2)
- BUG ratings B/U/G (3), physical dimensions L/W/H (3)
- Weighted similarity score with overlay diagrams (Polar + Cartesian)

### Comparison Report Export (PDF/Typst)

One-click PDF export via Typst compiled to WASM (no server):

- Similarity score badge with deviation summary
- Polar + Cartesian overlay diagrams
- Color-coded metrics table (green/yellow/orange/red)
- Side-by-side luminaire info, heatmaps, and 3D butterfly diagrams

### Typst Report Generation

Generate publication-ready photometric reports via `eulumdat-typst`:

- Single-file report: luminaire info, all diagrams, CU/UGR tables, candela tabulation
- Comparison report: overlay diagrams, metrics table, side-by-side analysis
- CLI: `eulumdat report luminaire.ldt report.typ`

## Features

- **Parse LDT/IES/ATLA files** - EULUMDAT, IESNA LM-63-2019, TM-33-23 XML/JSON
- **Write & convert** - Roundtrip LDT, IES, ATLA XML, ATLA JSON
- **Batch conversion** - Efficient bulk processing with progress reporting
- **Validation** - 44 LDT + 71 IES + TM-33-23/TM-32-24 validation rules
- **Symmetry handling** - 5 symmetry types with automatic data expansion
- **Photometric calculations** - CIE flux codes, beam/field angles, spacing criteria, zonal lumens, UGR, CU tables, NEMA classification
- **BUG Rating** - IESNA TM-15-11 Backlight-Uplight-Glare calculations
- **TM-32-24 BIM** - 100+ building information modeling parameters
- **12 diagram types** - Polar, Butterfly, Cartesian, Heatmap, BUG, LCS, Cone, Beam Angle, Spectral, Greenhouse, Isocandela, Isolux, Floodlight V-H
- **File comparison** - 36-metric similarity scoring with overlay diagrams and PDF report export
- **Google Maps designer** - Real-world illuminance simulation on satellite imagery
- **3D scene viewer** - Interactive Bevy-based room/road/parking/outdoor scenes with photometric lighting
- **Report generation** - Typst-based PDF reports (single-file + comparison)
- **GLDF integration** - Full DescriptivePhotometry population from IES/LDT
- **REST API** - Server mode for web service integration

## ATLA-S001 & TM-33-23 Support

The library internally uses **ATLA-S001** (Advanced Transfer Language for photometric Applications) as the unified data model:

- **Spectral data** - Full wavelength-based intensity distributions (380-780nm)
- **Horticultural metrics** - PPF, PPFD, YPF, phytochrome ratios for grow lights
- **TM-33-23 compatibility** - IES TM-33-23 XML format
- **TM-32-24 BIM** - Building information modeling parameters
- **SPDX detection** - Identify spectral power distribution data in XML files
- **Lossless conversion** - Convert between LDT, IES, and ATLA without data loss
- **Schema conversion** - Convert between ATLA-S001 and TM-33-23 schemas

## Installation

### Rust Library

```toml
[dependencies]
eulumdat = "0.5"
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
    .package(url: "https://github.com/holg/eulumdat-rs", from: "0.5.0")
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

// Compare two files (36 metrics)
let ldt_b = Eulumdat::from_file("luminaire_b.ldt")?;
let cmp = eulumdat::PhotometricComparison::from_eulumdat(&ldt, &ldt_b, "A", "B");
println!("Similarity: {:.1}%", cmp.similarity_score * 100.0);
for m in cmp.significant_metrics(eulumdat::Significance::Minor) {
    println!("{}: {:+.1}% ({})", m.name, m.delta_percent, m.significance);
}

// Generate overlay diagram
let polar_a = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
let polar_b = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt_b);
let overlay = eulumdat::diagram::PolarDiagram::to_overlay_svg(
    &polar_a, &polar_b, 500.0, 500.0, &SvgTheme::light(), "A", "B",
);
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

# Photometric calculations
summary = ldt.photometric_summary()
print(summary.to_text())  # Full report
print(summary.to_compact())  # One-liner

# Individual calculations
cie = ldt.cie_flux_codes()
print(f"CIE Flux Code: {cie}")  # e.g., "100 77 43 0 0"
print(f"Beam angle: {ldt.beam_angle():.1f}°")
print(f"Field angle: {ldt.field_angle():.1f}°")
print(f"Spacing: {ldt.spacing_criteria()}")  # (S/H C0, S/H C90)

# GLDF export
gldf = ldt.gldf_data()
print(gldf.to_dict())  # For JSON serialization

# Batch conversion
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

# Convert between formats (LDT, IES, ATLA XML, ATLA JSON)
eulumdat convert luminaire.ldt output.ies
eulumdat convert luminaire.xml output.ldt
eulumdat convert luminaire.ldt output.xml

# Generate diagrams (12 types)
eulumdat diagram luminaire.ldt -t polar -o polar.svg
eulumdat diagram luminaire.ldt -t isocandela -o isocandela.svg
eulumdat diagram luminaire.ldt -t isolux -o isolux.svg -m 10.0 --tilt 30
eulumdat diagram luminaire.ldt -t floodlight-vh -o floodlight.svg --log-scale
eulumdat diagram luminaire.ldt -t heatmap -o heatmap.svg --dark

# Calculate BUG rating
eulumdat bug outdoor_luminaire.ldt --svg bug.svg

# Batch convert multiple files
eulumdat batch input_folder/ -o output_folder/ -f ies

# Photometric summary (text, compact, or JSON)
eulumdat summary luminaire.ldt
eulumdat summary luminaire.ldt -f json -o summary.json

# Specific calculations
eulumdat calc luminaire.ldt -t cie-codes      # CIE flux codes
eulumdat calc luminaire.ldt -t beam-angles    # Beam/field angles
eulumdat calc luminaire.ldt -t nema           # NEMA floodlight classification
eulumdat calc luminaire.ldt -t cu-table       # Coefficient of Utilization
eulumdat calc luminaire.ldt -t ugr-table      # Unified Glare Rating
eulumdat calc luminaire.ldt -t all            # Everything

# Compare two files (36 metrics)
eulumdat compare file_a.ldt file_b.ldt                    # Text table
eulumdat compare file_a.ldt file_b.ldt -f json            # JSON output
eulumdat compare file_a.ldt file_b.ldt -f csv             # CSV output
eulumdat compare file_a.ldt file_b.ldt --significant-only # Only >= 5% deltas
eulumdat compare file_a.ldt file_b.ies -d polar -o cmp.svg   # Polar overlay SVG

# Generate photometric report (Typst)
eulumdat report luminaire.ldt report.typ
eulumdat report luminaire.ldt report.typ --cu-table --ugr-table --candela-table
eulumdat report luminaire.ldt report.typ --compact --paper letter

# Validate ATLA XML
eulumdat validate-atla document.xml
eulumdat validate-atla document.xml --schema-type tm3323 --xsd

# Convert between ATLA schema versions
eulumdat atla-convert s001.xml tm33.xml --target tm3323 --verbose

# GLDF-compatible export
eulumdat gldf luminaire.ldt --pretty -o gldf_data.json
```

### macOS / iOS

Native SwiftUI app available in the [EulumdatApp](EulumdatApp/) directory.

```bash
cd EulumdatApp && open EulumdatApp.xcodeproj
```

Features: Universal binary, QuickLook Extension, all diagram types, interactive 3D view, template library, validation, SVG/IES/LDT export.

### Android

Native Jetpack Compose app available in the [EulumdatAndroid](EulumdatAndroid/) directory.

```bash
cd EulumdatAndroid && ./gradlew assembleDebug
```

Features: Material 3 design, all diagram types, interactive 3D view, file picker, share & export, ARM64/ARMv7/x86_64.

### Web (WASM)

Browser-based editor at [eulumdat.icu](https://eulumdat.icu) or build locally:

```bash
# Build split bundles (Leptos editor + lazy-loaded Bevy 3D viewer)
./scripts/build-wasm-split.sh

# Serve locally
python3 -m http.server 8042 -d crates/eulumdat-wasm/dist
```

Features: All diagrams, 3D scene viewer (lazy-loaded), Google Maps designer, TM-32-24 BIM panel, file comparison with PDF export, template library, dark/light theme.

## Diagram Types

| Type | Description |
|------|-------------|
| Polar | Classic intensity distribution (C0-C180, C90-C270 curves) |
| Butterfly | 3D isometric projection |
| Cartesian | Intensity vs gamma angle |
| Heatmap | 2D intensity color map |
| BUG | IESNA TM-15-11 zone visualization |
| LCS | TM-15-07 Luminaire Classification System |
| Cone | Beam/field angle spread at mounting height |
| Beam Angle | IES vs CIE beam angle comparison |
| Spectral | Spectral power distribution (ATLA/TM-33 input) |
| Greenhouse | PPFD distribution for horticultural lighting |
| **Isocandela** | Type B H/V contour plot with iso-intensity lines |
| **Isolux** | Ground-plane illuminance footprint with lux contours |
| **Floodlight V-H** | H-plane + V-plane curves with NEMA classification |
| Polar Overlay | Two files overlaid on one polar diagram (comparison) |
| Cartesian Overlay | Two files overlaid on one cartesian diagram (comparison) |

## LM-63-2019 IES Support

Full support for the latest ANSI/IES LM-63-2019 standard:

```rust
use eulumdat::{IesParser, IesData, FileGenerationType, LuminousShape, IesMetadata};

// Parse IES with full metadata
let ies_data: IesData = IesParser::parse_to_ies_data(content)?;

// Access LM-63-2019 specific fields
println!("Version: {:?}", ies_data.version);           // IesVersion::Lm63_2019
println!("Test Lab: {}", ies_data.test_lab);           // From [TESTLAB]
println!("Issue Date: {}", ies_data.issue_date);       // From [ISSUEDATE]
println!("Accredited: {}", ies_data.file_generation_type.is_accredited());
println!("Shape: {:?}", ies_data.luminous_shape);      // Circular, Rectangular, etc.

// TILT data if present
if let Some(tilt) = &ies_data.tilt_data {
    println!("Lamp geometry: {}", tilt.lamp_geometry);
    println!("Tilt angles: {:?}", tilt.angles);
}

// Convert to IesMetadata for GLDF integration
let meta = IesMetadata::from_ies_data(&ies_data);
let (shape, width_mm, length_mm, diameter_mm) = meta.to_gldf_emitter_geometry();
```

## References

- [EULUMDAT Format (Paul Bourke)](https://paulbourke.net/dataformats/ldt/)
- [ANSI/IES LM-63-2019](https://www.ies.org/product/approved-method-ies-standard-file-format-for-the-electronic-transfer-of-photometric-data-and-related-information/) - Current IES standard
- [IESNA LM-63-2002](https://docs.agi32.com/PhotometricToolbox/Content/Open_Tool/iesna_lm-63_format.htm) - Legacy format reference
- [IES TM-33-23](https://www.ies.org/product/ies-standard-format-for-the-electronic-transfer-of-luminaire-optical-data/) - Luminaire optical data format
- [IES TM-32-24](https://www.ies.org/) - BIM parameters for lighting equipment
- [ATLA-S001](https://github.com/holg/eulumdat-rs/blob/main/docs/ATLA-S001.pdf) - Advanced Transfer Language for photometric Applications
- [IES TM-15-11 BUG Ratings](https://www.ies.org/wp-content/uploads/2017/03/TM-15-11BUGRatingsAddendum.pdf)
- [NEMA BIM 100-2021](https://www.nema.org/) - BIM shared parameters for Revit

## License

AGPL-3.0-or-later

This project is licensed under the GNU Affero General Public License v3.0 or later. See [LICENSE](LICENSE) for details.
