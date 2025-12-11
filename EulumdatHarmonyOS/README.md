# Eulumdat HarmonyOS

HarmonyOS version of the Eulumdat photometric data viewer, built with Cangjie and ArkTS.

## Overview

This directory contains the HarmonyOS implementation of Eulumdat, providing:

- **Cangjie FFI bindings** - Direct C FFI integration with the Rust library
- **ArkTS UI** - Native HarmonyOS user interface
- **DevEco Studio project** - Ready to open and build

## Architecture

```
EulumdatHarmonyOS/
├── src/                        # Standalone Cangjie sources (CLI test)
│   ├── eulumdat/
│   │   ├── ffi.cj             # C FFI declarations (foreign func)
│   │   ├── types.cj           # Safe Cangjie type wrappers
│   │   └── engine.cj          # High-level Eulumdat API
│   └── main.cj                # CLI test entry point
├── Eulumdat/                   # DevEco Studio project
│   ├── entry/
│   │   ├── src/main/
│   │   │   ├── cangjie/       # Cangjie sources (copied from src/)
│   │   │   ├── ets/           # ArkTS UI layer
│   │   │   │   ├── entryability/
│   │   │   │   ├── model/     # EulumdatEngine.ets (NAPI wrapper)
│   │   │   │   └── pages/     # Index.ets (main UI)
│   │   │   ├── cpp/           # NAPI bridge (C++ ↔ ArkTS)
│   │   │   └── resources/     # Strings, colors, icons
│   │   ├── libs/arm64-v8a/    # Native library (.so)
│   │   └── build-profile.json5
│   └── oh-package.json5
├── libs/arm64-v8a/             # Native library for CLI testing
├── cjpm.toml                   # Cangjie package config
├── eulumdat_ffi.h              # C header for FFI reference
├── build.sh                    # Build automation script
└── README.md                   # This file
```

## Prerequisites

### For Building the Rust Library

- Rust toolchain (rustup)
- For cross-compilation to HarmonyOS: `aarch64-linux-ohos` target

```bash
# Install HarmonyOS target
rustup target add aarch64-linux-ohos
```

### For Building the HarmonyOS App

- DevEco Studio 4.0+ with HarmonyOS SDK
- Cangjie SDK (for standalone CLI testing)

## Quick Start

### 1. Build the Rust FFI Library

```bash
cd EulumdatHarmonyOS
./build.sh release
```

This builds `libeulumdat_harmonyos_ffi.so` and copies it to the appropriate directories.

### 2. Open in DevEco Studio

1. Open DevEco Studio
2. File → Open → Select `EulumdatHarmonyOS/Eulumdat`
3. Wait for project sync
4. Build → Build HAP

### 3. Run on Device/Emulator

1. Connect a HarmonyOS device or start an emulator
2. Run → Run 'entry'

## Cangjie API Usage

```cangjie
import eulumdat.*

// Parse LDT file
let content = readFile("luminaire.ldt")
let ldt = Eulumdat.parseLdt(content)

// Get luminaire information
let info = ldt.info()
println("Name: ${info.luminaireName}")
println("Max Intensity: ${info.maxIntensity} cd/klm")
println("Symmetry: ${info.symmetry.displayName()}")

// Generate SVG diagrams
let polarSvg = ldt.polarSvg(400.0, 400.0, SvgTheme.Light)
let cartesianSvg = ldt.cartesianSvg(500.0, 300.0, 8, SvgTheme.Dark)
let butterflySvg = ldt.butterflySvg(400.0, 400.0, 60.0, SvgTheme.Light)
let heatmapSvg = ldt.heatmapSvg(400.0, 300.0, SvgTheme.Light)

// Validate
let warnings = ldt.validate()
for (w in warnings) {
    println("[${w.severity}] ${w.code}: ${w.message}")
}

// Export
let iesExport = ldt.exportIes()
let ldtExport = ldt.exportLdt()

// Sample intensity at any angle
let intensity = ldt.sampleIntensity(45.0, 30.0)  // C=45°, G=30°
```

## ArkTS API Usage

```typescript
import { EulumdatEngine, SvgTheme } from '../model/EulumdatEngine';

// Get engine instance
const engine = EulumdatEngine.getInstance();

// Parse file
engine.parseLdt(fileContent);

// Check if loaded
if (engine.isLoaded()) {
  // Get info
  const info = engine.getInfo();
  console.log(`Name: ${info.luminaireName}`);

  // Generate SVG
  const svg = engine.polarSvg(400, 400, SvgTheme.Light);

  // Validate
  const warnings = engine.validate();
}
```

## FFI Layer

The FFI layer provides a simple C interface that Cangjie can call directly:

```c
// Parse functions
ParseResult eulumdat_parse_ldt(const char* content);
ParseResult eulumdat_parse_ies(const char* content);

// Information
LuminaireInfo eulumdat_get_info(const EulumdatHandle* handle);

// SVG generation
char* eulumdat_polar_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);
char* eulumdat_cartesian_svg(const EulumdatHandle* handle, double width, double height, uint32_t max_curves, int32_t theme);
char* eulumdat_butterfly_svg(const EulumdatHandle* handle, double width, double height, double tilt_degrees, int32_t theme);
char* eulumdat_heatmap_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);

// Export
char* eulumdat_export_ldt(const EulumdatHandle* handle);
char* eulumdat_export_ies(const EulumdatHandle* handle);

// Validation
ValidationWarningList eulumdat_validate(const EulumdatHandle* handle);

// Memory management
void eulumdat_free(EulumdatHandle* handle);
void eulumdat_string_free(char* s);
```

## Cross-Compilation

### From macOS/Linux to HarmonyOS ARM64

```bash
# Set up HarmonyOS NDK environment
export OHOS_NDK_HOME=/path/to/ohos-sdk/native

# Build with cargo
cargo build --release -p eulumdat-harmonyos-ffi --target aarch64-linux-ohos
```

## Features

- **File Loading**: Open LDT (EULUMDAT) and IES photometric files
- **Luminaire Info**: View manufacturer, dimensions, optical properties
- **Diagrams**: Polar, Cartesian, Butterfly (3D), and Heatmap visualizations
- **Validation**: Check for data quality issues and warnings
- **Export**: Convert between LDT and IES formats
- **Theme Support**: Light and Dark mode for diagrams

## Project Structure Notes

### Why Two Approaches?

The project supports both:

1. **Standalone Cangjie CLI** (`src/` + `cjpm.toml`) - For testing FFI bindings without DevEco Studio
2. **Full HarmonyOS App** (`Eulumdat/`) - Complete app with ArkTS UI

The Cangjie sources are copied from `src/` to `Eulumdat/entry/src/main/cangjie/` during build.

### NAPI Bridge

Since ArkTS cannot directly call C functions, a NAPI bridge (`eulumdat_napi.cpp`) wraps the Rust FFI:

```
ArkTS (TypeScript)
    ↓
NAPI Bridge (C++)
    ↓
Rust FFI (libeulumdat_harmonyos_ffi.so)
    ↓
Rust Core Library (eulumdat)
```

## Related Projects

- **EulumdatApp** - macOS/iOS version (SwiftUI)
- **EulumdatAndroid** - Android version (Jetpack Compose)
- **eulumdat-cli** - Command-line tool
- **eulumdat-wasm** - WebAssembly bindings

## License

MIT OR Apache-2.0
