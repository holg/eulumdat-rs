#!/usr/bin/env bash
# Build XCFramework for eulumdat-ffi
# Creates XCFramework from static libraries for iOS/macOS

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target"
SWIFT_DIR="$PROJECT_ROOT/swift"
FFI_CRATE="eulumdat-ffi"
LIB_NAME="libeulumdat_ffi"
XCFRAMEWORK_NAME="eulumdat_ffiFFI.xcframework"

# Generated bindings output directory (contains .swift, .h, and .modulemap)
GENERATED_DIR="$SWIFT_DIR/Sources/Eulumdat"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${YELLOW}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo -e "${YELLOW}=== Building XCFramework for Eulumdat ===${NC}\n"

# 1. Install Rust targets
log_info "Installing Rust targets..."
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
log_success "Rust targets installed"

# 2. Generate Swift bindings
log_info "Generating UniFFI Swift bindings..."
mkdir -p "$GENERATED_DIR"

# First build for macOS to get the dylib for binding generation
cargo build --release --package "$FFI_CRATE" --target aarch64-apple-darwin

cargo run --package "$FFI_CRATE" --bin uniffi-bindgen generate \
    --library "$BUILD_DIR/aarch64-apple-darwin/release/${LIB_NAME}.dylib" \
    --language swift \
    --out-dir "$GENERATED_DIR"

# Patch the Swift file to unconditionally import the FFI module
# (the canImport check doesn't work reliably with SwiftPM binary targets)
# Remove the 3-line block: #if canImport(...) / import ... / #endif
# Using a simpler approach that works reliably across macOS versions
SWIFT_FILE="$GENERATED_DIR/eulumdat_ffi.swift"
if grep -q '#if canImport(eulumdat_ffiFFI)' "$SWIFT_FILE"; then
    # Create a temp file and process line by line
    awk '
    /#if canImport\(eulumdat_ffiFFI\)/ { skip=1; next }
    skip && /^import eulumdat_ffiFFI/ { print; skip=2; next }
    skip==2 && /^#endif/ { skip=0; next }
    { print }
    ' "$SWIFT_FILE" > "${SWIFT_FILE}.tmp" && mv "${SWIFT_FILE}.tmp" "$SWIFT_FILE"
    log_info "Patched Swift bindings to use unconditional import"
fi

log_success "Swift bindings generated"

# 3. Build for all targets
TARGETS=(
    "aarch64-apple-ios"
    "aarch64-apple-ios-sim"
    "x86_64-apple-ios"
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
)

for target in "${TARGETS[@]}"; do
    log_info "Building for $target..."
    cargo build --release --package "$FFI_CRATE" --target "$target"
done
log_success "All targets built"

# 4. Create fat libraries
log_info "Creating fat libraries..."

mkdir -p "$BUILD_DIR/ios-sim-universal"
lipo -create \
    "$BUILD_DIR/aarch64-apple-ios-sim/release/${LIB_NAME}.a" \
    "$BUILD_DIR/x86_64-apple-ios/release/${LIB_NAME}.a" \
    -output "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a"

mkdir -p "$BUILD_DIR/macos-universal"
lipo -create \
    "$BUILD_DIR/aarch64-apple-darwin/release/${LIB_NAME}.a" \
    "$BUILD_DIR/x86_64-apple-darwin/release/${LIB_NAME}.a" \
    -output "$BUILD_DIR/macos-universal/${LIB_NAME}.a"

log_success "Fat libraries created"

# 5. Create XCFramework
log_info "Creating XCFramework..."

rm -rf "$SWIFT_DIR/$XCFRAMEWORK_NAME"

# Create a temporary headers directory with only .h and .modulemap files
HEADERS_TEMP="$BUILD_DIR/headers-temp"
rm -rf "$HEADERS_TEMP"
mkdir -p "$HEADERS_TEMP"
cp "$GENERATED_DIR/eulumdat_ffiFFI.h" "$HEADERS_TEMP/"
# Rename modulemap to standard module.modulemap
cp "$GENERATED_DIR/eulumdat_ffiFFI.modulemap" "$HEADERS_TEMP/module.modulemap"

xcodebuild -create-xcframework \
    -library "$BUILD_DIR/aarch64-apple-ios/release/${LIB_NAME}.a" \
    -headers "$HEADERS_TEMP" \
    -library "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a" \
    -headers "$HEADERS_TEMP" \
    -library "$BUILD_DIR/macos-universal/${LIB_NAME}.a" \
    -headers "$HEADERS_TEMP" \
    -output "$SWIFT_DIR/$XCFRAMEWORK_NAME"

# Clean up temp headers
rm -rf "$HEADERS_TEMP"

log_success "XCFramework created at $SWIFT_DIR/$XCFRAMEWORK_NAME"

# 6. Verify
echo ""
log_info "XCFramework contents:"
ls -la "$SWIFT_DIR/$XCFRAMEWORK_NAME/"

echo -e "\n${GREEN}=== Build complete! ===${NC}"
