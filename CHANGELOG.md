# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2025-12-11

### Added

#### eulumdat (core library)
- **`sample(c_angle, g_angle)`** - Bilinear interpolated intensity sampling at any C/G angle
- **Symmetry-aware sampling** - Automatic expansion for all symmetry types

#### eulumdat-photweb (NEW)
- **PhotometricWeb** struct for advanced intensity sampling
- `sample()` and `sample_normalized()` with bilinear interpolation
- **LdcMesh** for 3D photometric solid mesh generation
- Symmetry handling (None, VerticalAxis, PlaneC0C180, PlaneC90C270, BothPlanes)
- Graphics-ready output (positions, normals, indices)

#### eulumdat-wasm (Web App)
- **IES file import support** - Upload and convert IES files directly in the browser

#### EulumdatHarmonyOS (NEW)
- **HarmonyOS/OpenHarmony port** with Cangjie language bindings
- N-API FFI bridge for native Rust integration
- Full ArkUI interface with all diagram types
- eulumdat-harmonyos-ffi crate for C ABI exports

#### macOS/iOS App
- **Room3DView** - Photometric visualization in 3D room environment

### Fixed
- HarmonyOS app icon (was incorrectly using GeoDB icon)

---

## [0.2.1] - 2025-12-08

### Added

#### eulumdat (core library)
- **Batch conversion API** - New `batch` module for efficient bulk processing of multiple LDT/IES files
- Modular code organization for better maintainability

#### eulumdat-cli
- **`batch` command** - Convert multiple files at once: `eulumdat batch input/ -o output/ -f ies`
- Refactored into modular structure (`cli.rs`, `commands.rs`)

#### eulumdat-egui (NEW)
- **Cross-platform desktop GUI application** for Windows, macOS, and Linux
- All diagram types: Polar, Cartesian, Butterfly, 3D, Heatmap, BUG, LCS
- 7 information tabs: Diagram, General, Dimensions, Lamps, Optical, Intensity, Validation
- Built-in template library for quick testing
- Export to SVG, IES, LDT formats
- Intensity table with **Copy CSV** button and **color toggle**
- Validation tab showing both errors and warnings

#### eulumdat-py (Python bindings)
- **Batch conversion** - `batch_convert()` function for efficient bulk processing
- `BatchInput`, `BatchOutput`, `BatchStats` classes
- `InputFormat` and `ConversionFormat` enums

#### eulumdat-ffi (Swift/Kotlin bindings)
- Batch conversion support
- Modular code organization matching core library

#### eulumdat-windows-preview (NEW)
- **Windows Shell Preview Handler** for File Explorer
- Preview LDT files in Explorer's preview pane (Alt+P)
- Renders polar diagram SVG

#### macOS/iOS App (EulumdatApp)
- **QuickLook Extension** - Preview .ldt files directly in Finder (press Space)
- **Intensity tab improvements**:
  - Copy CSV button for exporting data to spreadsheets
  - Color toggle to show/hide heatmap coloring
- Updated app icon with "LDT/IES" branding
- Fixed app icon transparency for App Store compliance

#### Android App (EulumdatAndroid)
- Interactive 3D butterfly diagram with touch rotation
- Template library matching iOS app
- Updated launcher icon with "LDT/IES" text

#### CI/CD
- All crates published to crates.io (eulumdat, cli, egui, ffi, wasm, py, windows-preview)
- Python wheels published to PyPI for all platforms
- GitHub Actions workflow for building GUI binaries (Windows, macOS, Linux)
- Android APK builds included in releases
- Swift XCFramework builds and releases
- Optional Google Play Store deployment

### Changed
- Improved intensity table layout with smaller, more compact cells
- Validation tab now shows both errors (red) and warnings (yellow)
- Release artifacts categorized by type (cli-*, gui-*, android-*)

### Fixed
- Python bindings error handling compatibility with nightly Rust
- App icons now use solid background (no alpha channel) for App Store compliance
- Swift workflow patching for UniFFI generated bindings
- crates.io publishing with proper version specifications

## [0.2.0] - 2024-11-15

### Added
- Initial release with core parsing, validation, and diagram generation
- CLI tool with info, validate, convert, diagram, and bug commands
- Python bindings via PyO3
- Swift/Kotlin bindings via UniFFI
- WebAssembly editor
- BUG rating calculation (IESNA TM-15-11)
- 6 diagram types: Polar, Cartesian, Butterfly, Heatmap, BUG, LCS
