# Eulumdat macOS/iOS App

A native macOS and iOS application for viewing and editing EULUMDAT (LDT) and IES photometric files.

## Features

- **Multiple Diagram Types**: Polar, Cartesian, Butterfly, 3D, Heatmap, BUG Rating, LCS
- **File Import/Export**: LDT, IES, and SVG formats
- **Batch Conversion**: Convert multiple files at once
- **Validation**: Built-in validation with warnings and errors
- **Templates**: Pre-configured luminaire templates
- **Interactive 3D**: SceneKit-based 3D photometric solid visualization

## Building the App

### Prerequisites

1. **Xcode 15+** (for macOS/iOS development)
2. **Rust** (for building the core library)
3. **Swift Package Manager** (included with Xcode)

### Build Steps

1. **Build the XCFramework** (required before first build):
   ```bash
   cd /path/to/eulumdat-rs
   ./scripts/build-xcframework.sh
   ```

   This script:
   - Installs required Rust targets (iOS, macOS simulator, macOS)
   - Generates UniFFI Swift bindings
   - Builds the Rust library for all platforms
   - Creates the `swift/eulumdat_ffiFFI.xcframework`

2. **Build the Swift App**:
   ```bash
   cd EulumdatApp
   swift build
   ```

   Or open `EulumdatApp.xcodeproj` in Xcode and press ⌘R to build and run.

### Important Notes

⚠️ **XCFramework is NOT committed to git**
- The `swift/eulumdat_ffiFFI.xcframework/` directory is gitignored
- It must be generated locally by running `./scripts/build-xcframework.sh`
- CI/CD pipelines should run this script as part of the build process

⚠️ **User-specific files are gitignored**
- `xcuserdata/` and `*.xcuserstate` (Xcode user settings)
- `.build/` directories (build artifacts)
- See `.gitignore` for complete list

## Development

### Project Structure

```
EulumdatApp/
├── EulumdatApp/
│   ├── ContentView.swift          # Main app UI
│   ├── Butterfly3DView.swift      # 3D SceneKit visualization
│   ├── BatchConvertView.swift     # Batch conversion UI
│   ├── ValidationView.swift       # Validation results
│   ├── Templates.swift            # Luminaire templates
│   └── Resources/
│       └── Templates/             # LDT template files
├── Package.swift                  # Swift package manifest
└── README.md                      # This file
```

### Controls

#### Diagram View (SVG)
- **Cmd+Scroll**: Zoom in/out
- **Cmd+Drag**: Pan when zoomed
- **Double-click**: Open in fullscreen window
- **Cmd+/-/0**: Zoom controls

#### 3D View (SceneKit)
- **Drag**: Rotate camera
- **Cmd+Scroll**: Zoom in/out
- **Cmd+Drag**: Rotate camera
- **Double-click**: Open in fullscreen window

### Keyboard Shortcuts

- **⌘N**: New window
- **⌘O**: Open file
- **⌘⇧B**: Batch convert
- **⌘⇧E**: Export SVG
- **⌘⇧I**: Export IES
- **⌘⇧L**: Export LDT
- **⌘1-7**: Switch diagram types
- **⌘⇧D**: Toggle dark theme

## Troubleshooting

### "Info.plist not found" Error

This means the XCFramework hasn't been built. Run:
```bash
./scripts/build-xcframework.sh
```

### Build Fails with "Cannot find module"

The Swift bindings may be out of date. Rebuild the XCFramework:
```bash
./scripts/build-xcframework.sh
```

### Xcode Can't Find Package Dependencies

Clean and rebuild:
```bash
cd EulumdatApp
rm -rf .build
swift build
```

## License

See the main repository README for license information.
