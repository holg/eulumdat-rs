#!/usr/bin/env bash
# Build script for Eulumdat HarmonyOS (Cangjie) app
#
# This script:
# 1. Cross-compiles the Rust FFI library for HarmonyOS
# 2. Copies it to libs/ directory
# 3. Builds the Cangjie app (optional)
# 4. Optionally runs the CLI test

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FFI_CRATE="$REPO_ROOT/crates/eulumdat-harmonyos-ffi"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Parse arguments
BUILD_MODE="${1:-release}"  # debug or release
RUN_TEST="${2:-no}"         # run or no

if [[ "$BUILD_MODE" != "debug" && "$BUILD_MODE" != "release" ]]; then
    error "Usage: $0 [debug|release] [run|no]"
fi

info "Building Eulumdat HarmonyOS (Cangjie) app in $BUILD_MODE mode"
echo

# ============================================================================
# Step 1: Build Rust FFI Library for HarmonyOS (cross-compile)
# ============================================================================

info "Step 1: Cross-compiling Rust FFI library for aarch64-unknown-linux-ohos..."

cd "$REPO_ROOT"

# Ensure target is installed
if ! rustup target list --installed | grep -q "aarch64-unknown-linux-ohos"; then
    info "Installing aarch64-unknown-linux-ohos target..."
    rustup target add aarch64-unknown-linux-ohos
fi

if [[ "$BUILD_MODE" == "release" ]]; then
    cargo build --release -p eulumdat-harmonyos-ffi --target aarch64-unknown-linux-ohos
    RUST_LIB="$REPO_ROOT/target/aarch64-unknown-linux-ohos/release/libeulumdat_harmonyos_ffi.so"
else
    cargo build -p eulumdat-harmonyos-ffi --target aarch64-unknown-linux-ohos
    RUST_LIB="$REPO_ROOT/target/aarch64-unknown-linux-ohos/debug/libeulumdat_harmonyos_ffi.so"
fi

if [[ ! -f "$RUST_LIB" ]]; then
    error "Rust library not found at $RUST_LIB"
fi

info "✓ Rust library built: $RUST_LIB"
file "$RUST_LIB"
echo

# ============================================================================
# Step 2: Copy to libs/ directory
# ============================================================================

info "Step 2: Copying library to libs/..."

# For DevEco Studio project
DEVECO_LIBS_DIR="$SCRIPT_DIR/Eulumdat/entry/libs/arm64-v8a"
mkdir -p "$DEVECO_LIBS_DIR"
cp "$RUST_LIB" "$DEVECO_LIBS_DIR/"

# Also copy to standalone CLI libs (for local testing with cjpm)
LIBS_DIR="$SCRIPT_DIR/libs/arm64-v8a"
mkdir -p "$LIBS_DIR"
cp "$RUST_LIB" "$LIBS_DIR/"

info "✓ Library copied to $DEVECO_LIBS_DIR/"
ls -lh "$DEVECO_LIBS_DIR/"
echo

# ============================================================================
# Step 3: Build Cangjie app (optional - requires cjpm)
# ============================================================================

info "Step 3: Building Cangjie app..."

cd "$SCRIPT_DIR"

# Check if cjpm exists
if ! command -v cjpm &> /dev/null; then
    warn "cjpm not found in PATH"
    warn "Please install Cangjie toolchain from: https://cangjie-lang.cn/en/download"
    warn "Skipping Cangjie build"
else
    # Build with cjpm
    cjpm build || warn "cjpm build failed (may need native library for local testing)"
    info "✓ Cangjie app built"
fi
echo

# ============================================================================
# Step 4: Run test (optional)
# ============================================================================

if [[ "$RUN_TEST" == "run" ]]; then
    info "Step 4: Running CLI test..."
    echo

    # Set library path for runtime
    export LD_LIBRARY_PATH="$LIBS_DIR:$LD_LIBRARY_PATH"
    export DYLD_LIBRARY_PATH="$LIBS_DIR:$DYLD_LIBRARY_PATH"

    # Run the binary
    if [[ -f "$SCRIPT_DIR/release/bin/eulumdat_harmonyos" ]]; then
        "$SCRIPT_DIR/release/bin/eulumdat_harmonyos"
    else
        warn "Binary not found at release/bin/eulumdat_harmonyos"
        warn "This is expected if Cangjie toolchain is not installed"
    fi
fi

info "Build complete!"
echo
info "Next steps:"
info "  1. Open EulumdatHarmonyOS/Eulumdat in DevEco Studio"
info "  2. Build and run on HarmonyOS device/emulator"
