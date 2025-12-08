#!/bin/bash
#
# Build Eulumdat Android app for Play Store deployment
#
# Usage:
#   ./build-android.sh          # Build debug APK
#   ./build-android.sh release  # Build release AAB for Play Store
#   ./build-android.sh apk      # Build release APK
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ANDROID_PROJECT="$PROJECT_DIR/EulumdatAndroid"
JNI_LIBS_DIR="$ANDROID_PROJECT/app/src/main/jniLibs"
BUILD_TYPE="${1:-debug}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${YELLOW}[INFO]${NC} $1"; }
ok() { echo -e "${GREEN}[OK]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

echo "=== Eulumdat Android Build ==="
echo ""

# Check prerequisites
command -v cargo-ndk >/dev/null 2>&1 || error "cargo-ndk not installed. Run: cargo install cargo-ndk"

# Check for Android SDK
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [ ! -d "$ANDROID_HOME" ]; then
    error "Android SDK not found at $ANDROID_HOME. Please install Android Studio."
fi

# Find NDK
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$(ls -d "$ANDROID_HOME/ndk/"* 2>/dev/null | tail -1)}"
if [ ! -d "$ANDROID_NDK_HOME" ]; then
    error "Android NDK not found. Please install it via Android Studio SDK Manager."
fi

info "Android SDK: $ANDROID_HOME"
info "Android NDK: $ANDROID_NDK_HOME"

# Ensure Android targets are installed
info "Checking Rust Android targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android 2>/dev/null || true

cd "$PROJECT_DIR"

# Step 1: Build native libraries
echo ""
info "Step 1/3: Building native libraries..."

TARGETS=(
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "x86_64-linux-android"
    "i686-linux-android"
)

for target in "${TARGETS[@]}"; do
    info "  Building for $target..."
    cargo ndk -t $target -o "$JNI_LIBS_DIR" build --release -p eulumdat-ffi 2>&1 | grep -E "(Compiling eulumdat|Finished)" || true
done

ok "Native libraries built"

# Step 2: Generate Kotlin bindings
echo ""
info "Step 2/3: Generating Kotlin bindings..."
KOTLIN_OUT_DIR="$ANDROID_PROJECT/app/src/main/java"
mkdir -p "$KOTLIN_OUT_DIR"

cargo run -p eulumdat-ffi --bin uniffi-bindgen generate \
    --library target/aarch64-linux-android/release/libeulumdat_ffi.so \
    --language kotlin \
    --out-dir "$KOTLIN_OUT_DIR" 2>&1 | grep -v "Unable to auto-format" || true

if [ -f "$KOTLIN_OUT_DIR/uniffi/eulumdat_ffi/eulumdat_ffi.kt" ]; then
    ok "Kotlin bindings generated"
else
    error "Failed to generate Kotlin bindings"
fi

# Step 3: Build Android app
echo ""
info "Step 3/3: Building Android app ($BUILD_TYPE)..."
cd "$ANDROID_PROJECT"

# Make gradlew executable
chmod +x gradlew 2>/dev/null || true

# Download gradle wrapper if missing
if [ ! -f "gradlew" ]; then
    info "Downloading Gradle wrapper..."
    gradle wrapper --gradle-version 8.2
fi

case "$BUILD_TYPE" in
    release|aab)
        info "Building release AAB (Android App Bundle) for Play Store..."
        ./gradlew bundleRelease --no-daemon
        AAB_PATH="$ANDROID_PROJECT/app/build/outputs/bundle/release/app-release.aab"
        if [ -f "$AAB_PATH" ]; then
            ok "Release AAB built: $AAB_PATH"
            echo ""
            echo "To upload to Play Store:"
            echo "  1. Sign the AAB with your release keystore"
            echo "  2. Upload to Google Play Console"
        else
            error "AAB build failed"
        fi
        ;;
    apk)
        info "Building release APK..."
        ./gradlew assembleRelease --no-daemon
        APK_PATH="$ANDROID_PROJECT/app/build/outputs/apk/release/app-release-unsigned.apk"
        if [ -f "$APK_PATH" ]; then
            ok "Release APK built: $APK_PATH"
            echo ""
            echo "Note: APK is unsigned. Sign with:"
            echo "  apksigner sign --ks your-keystore.jks --out app-release.apk $APK_PATH"
        else
            error "APK build failed"
        fi
        ;;
    debug|*)
        info "Building debug APK..."
        ./gradlew assembleDebug --no-daemon
        APK_PATH="$ANDROID_PROJECT/app/build/outputs/apk/debug/app-debug.apk"
        if [ -f "$APK_PATH" ]; then
            ok "Debug APK built: $APK_PATH"
            echo ""
            echo "Install on connected device:"
            echo "  adb install $APK_PATH"
        else
            error "Debug APK build failed"
        fi
        ;;
esac

# Show build summary
echo ""
echo "=== Build Summary ==="
echo "Native libraries:"
find "$JNI_LIBS_DIR" -name "*.so" -exec ls -lh {} \; 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}'
echo ""
echo "Output:"
find "$ANDROID_PROJECT/app/build/outputs" \( -name "*.apk" -o -name "*.aab" \) -exec ls -lh {} \; 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}'
echo ""
ok "Android build complete!"
