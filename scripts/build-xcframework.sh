#!/usr/bin/env bash
# Build XCFramework for eulumdat-ffi
# This script builds the Rust library for multiple Apple platforms and creates an XCFramework

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target"
SWIFT_DIR="$PROJECT_ROOT/swift"
# Framework name must match the UniFFI-generated module name for Swift imports to work
FRAMEWORK_NAME="eulumdat_ffiFFI"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== Building XCFramework for Eulumdat ===${NC}\n"

# Ensure we have the required Rust targets
echo -e "${YELLOW}Step 1: Installing Rust targets...${NC}"
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
echo -e "${GREEN}✓ Rust targets installed${NC}\n"

# Build for all platforms
echo -e "${YELLOW}Step 2: Building for all platforms...${NC}"

# iOS device (arm64)
echo "  Building for iOS (arm64)..."
cargo build --release --package eulumdat-ffi --target aarch64-apple-ios

# iOS Simulator (arm64 + x86_64)
echo "  Building for iOS Simulator (arm64)..."
cargo build --release --package eulumdat-ffi --target aarch64-apple-ios-sim
echo "  Building for iOS Simulator (x86_64)..."
cargo build --release --package eulumdat-ffi --target x86_64-apple-ios

# macOS (arm64 + x86_64)
echo "  Building for macOS (arm64)..."
cargo build --release --package eulumdat-ffi --target aarch64-apple-darwin
echo "  Building for macOS (x86_64)..."
cargo build --release --package eulumdat-ffi --target x86_64-apple-darwin

echo -e "${GREEN}✓ All platforms built${NC}\n"

# Generate Swift bindings
echo -e "${YELLOW}Step 3: Generating Swift bindings...${NC}"
cargo run --package eulumdat-ffi --bin uniffi-bindgen generate \
    --library "$BUILD_DIR/aarch64-apple-darwin/release/libeulumdat_ffi.dylib" \
    --language swift \
    --out-dir "$SWIFT_DIR/Sources/Eulumdat"
echo -e "${GREEN}✓ Swift bindings generated${NC}\n"

# Create fat libraries for simulator (arm64 + x86_64)
echo -e "${YELLOW}Step 4: Creating fat libraries...${NC}"
mkdir -p "$BUILD_DIR/ios-simulator-fat"
lipo -create \
    "$BUILD_DIR/aarch64-apple-ios-sim/release/libeulumdat_ffi.a" \
    "$BUILD_DIR/x86_64-apple-ios/release/libeulumdat_ffi.a" \
    -output "$BUILD_DIR/ios-simulator-fat/libeulumdat_ffi.a"

mkdir -p "$BUILD_DIR/macos-fat"
lipo -create \
    "$BUILD_DIR/aarch64-apple-darwin/release/libeulumdat_ffi.a" \
    "$BUILD_DIR/x86_64-apple-darwin/release/libeulumdat_ffi.a" \
    -output "$BUILD_DIR/macos-fat/libeulumdat_ffi.a"
echo -e "${GREEN}✓ Fat libraries created${NC}\n"

# Create module maps
echo -e "${YELLOW}Step 5: Creating module maps...${NC}"
create_module_map() {
    local DIR=$1
    mkdir -p "$DIR/Headers"
    cp "$SWIFT_DIR/Sources/Eulumdat/eulumdat_ffiFFI.h" "$DIR/Headers/"

    # Module name matches FRAMEWORK_NAME which matches UniFFI-generated Swift imports
    cat > "$DIR/Headers/module.modulemap" << EOF
framework module ${FRAMEWORK_NAME} {
    umbrella header "eulumdat_ffiFFI.h"
    export *
    module * { export * }
}
EOF
}

# Setup framework directories
for PLATFORM in ios-device ios-simulator macos; do
    FRAMEWORK_DIR="$BUILD_DIR/$PLATFORM/${FRAMEWORK_NAME}.framework"
    mkdir -p "$FRAMEWORK_DIR"
    create_module_map "$FRAMEWORK_DIR"
done

# Copy static libraries
cp "$BUILD_DIR/aarch64-apple-ios/release/libeulumdat_ffi.a" \
   "$BUILD_DIR/ios-device/${FRAMEWORK_NAME}.framework/${FRAMEWORK_NAME}"
cp "$BUILD_DIR/ios-simulator-fat/libeulumdat_ffi.a" \
   "$BUILD_DIR/ios-simulator/${FRAMEWORK_NAME}.framework/${FRAMEWORK_NAME}"
cp "$BUILD_DIR/macos-fat/libeulumdat_ffi.a" \
   "$BUILD_DIR/macos/${FRAMEWORK_NAME}.framework/${FRAMEWORK_NAME}"

echo -e "${GREEN}✓ Module maps created${NC}\n"

# Create XCFramework
echo -e "${YELLOW}Step 6: Creating XCFramework...${NC}"
rm -rf "$SWIFT_DIR/${FRAMEWORK_NAME}.xcframework"
xcodebuild -create-xcframework \
    -framework "$BUILD_DIR/ios-device/${FRAMEWORK_NAME}.framework" \
    -framework "$BUILD_DIR/ios-simulator/${FRAMEWORK_NAME}.framework" \
    -framework "$BUILD_DIR/macos/${FRAMEWORK_NAME}.framework" \
    -output "$SWIFT_DIR/${FRAMEWORK_NAME}.xcframework"
echo -e "${GREEN}✓ XCFramework created${NC}\n"

# Verify
echo -e "${YELLOW}Step 7: Verifying...${NC}"
if [ -d "$SWIFT_DIR/${FRAMEWORK_NAME}.xcframework" ]; then
    echo -e "${GREEN}✓ XCFramework created at: $SWIFT_DIR/${FRAMEWORK_NAME}.xcframework${NC}"
    echo ""
    echo "Contents:"
    ls -la "$SWIFT_DIR/${FRAMEWORK_NAME}.xcframework/"
else
    echo -e "${RED}✗ Failed to create XCFramework${NC}"
    exit 1
fi

echo -e "\n${GREEN}=== Build complete! ===${NC}"
echo -e "Swift package is ready at: $PROJECT_ROOT"
echo -e "Add to your project: .package(url: \"https://github.com/holg/eulumdat-rs\", from: \"0.2.0\")"
