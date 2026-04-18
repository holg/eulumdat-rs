# Eulumdat Android App

Android application for viewing and analyzing photometric data files (LDT/IES).

## Features

- Open LDT (EULUMDAT) and IES photometric files
- View multiple diagram types:
  - Polar diagram
  - Cartesian diagram
  - Butterfly diagram (2D & 3D)
  - Heatmap
  - BUG rating
  - LCS diagram
- View luminaire information:
  - General info (name, manufacturer, photometry)
  - Dimensions (luminaire and luminous area)
  - Lamp data
  - Intensity distribution
  - Validation warnings/errors
- Light/dark theme support
- Material 3 design with dynamic colors

## Building

### Prerequisites

1. **Android Studio** (latest version)
2. **Rust** with Android targets:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
   ```
3. **cargo-ndk**:
   ```bash
   cargo install cargo-ndk
   ```
4. **Android NDK** (install via Android Studio SDK Manager)

### Build Native Libraries

From the project root:

```bash
./scripts/build-android.sh
```

This will:
- Build the Rust library for all Android ABIs (arm64-v8a, armeabi-v7a, x86_64, x86)
- Generate Kotlin bindings using uniffi
- Copy everything to the correct locations in the Android project

### Build the App

1. Open `EulumdatAndroid` in Android Studio
2. Sync Gradle
3. Build and run

Or from command line:

```bash
cd EulumdatAndroid
./gradlew assembleDebug
```

## Project Structure

```
EulumdatAndroid/
├── app/
│   ├── src/main/
│   │   ├── java/eu/trahe/eulumdat/
│   │   │   ├── MainActivity.kt          # Main activity
│   │   │   ├── ui/
│   │   │   │   ├── EulumdatApp.kt       # Main composable
│   │   │   │   └── theme/Theme.kt       # Material 3 theme
│   │   │   └── data/
│   │   │       ├── LdtData.kt           # Data models
│   │   │       └── LdtRepository.kt     # File loading
│   │   ├── jniLibs/                      # Native libraries (generated)
│   │   │   ├── arm64-v8a/
│   │   │   ├── armeabi-v7a/
│   │   │   ├── x86_64/
│   │   │   └── x86/
│   │   └── res/                          # Resources
│   └── build.gradle.kts
├── build.gradle.kts
└── settings.gradle.kts
```

## Tech Stack

- **Kotlin** - Primary language
- **Jetpack Compose** - Modern UI toolkit
- **Material 3** - Design system
- **Coil** - Image loading (SVG support)
- **JNA** - Java Native Access for Rust bindings
- **uniffi** - Rust FFI bindings generation

## Native Library

The app uses the `eulumdat-ffi` Rust crate which provides:
- LDT/IES file parsing
- SVG diagram generation
- Validation
- Format conversion (LDT ↔ IES)

## License

AGPL-3.0-or-later
