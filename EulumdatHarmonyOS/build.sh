#!/usr/bin/env bash
#
# Eulumdat HarmonyOS Build Script
#
# This script builds the Rust FFI library and prepares it for HarmonyOS integration.
#
# Usage:
#   ./build.sh [release|debug] [run]
#
# Examples:
#   ./build.sh              # Build release
#   ./build.sh debug        # Build debug
#   ./build.sh release run  # Build and run CLI test

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FFI_CRATE="$REPO_ROOT/crates/eulumdat-harmonyos-ffi"

# Default to release build
BUILD_TYPE="${1:-release}"
RUN_TEST="${2:-}"

echo "=== Eulumdat HarmonyOS Build ==="
echo ""
echo "Repository: $REPO_ROOT"
echo "Build type: $BUILD_TYPE"
echo ""

# ============================================================================
# Step 1: Build Rust FFI library
# ============================================================================

echo "Step 1: Building Rust FFI library..."

cd "$REPO_ROOT"

if [ "$BUILD_TYPE" = "debug" ]; then
    cargo build -p eulumdat-harmonyos-ffi
    RUST_LIB="$REPO_ROOT/target/debug/libeulumdat_harmonyos_ffi"
else
    cargo build --release -p eulumdat-harmonyos-ffi
    RUST_LIB="$REPO_ROOT/target/release/libeulumdat_harmonyos_ffi"
fi

# Detect platform and library extension
case "$(uname -s)" in
    Darwin)
        RUST_LIB_EXT=".dylib"
        TARGET_EXT=".so"  # HarmonyOS uses .so
        ;;
    Linux)
        RUST_LIB_EXT=".so"
        TARGET_EXT=".so"
        ;;
    MINGW*|CYGWIN*|MSYS*)
        RUST_LIB_EXT=".dll"
        TARGET_EXT=".dll"
        ;;
    *)
        echo "Unknown platform: $(uname -s)"
        exit 1
        ;;
esac

RUST_LIB_FULL="${RUST_LIB}${RUST_LIB_EXT}"

if [ ! -f "$RUST_LIB_FULL" ]; then
    echo "ERROR: Library not found: $RUST_LIB_FULL"
    exit 1
fi

echo "  Built: $RUST_LIB_FULL"
echo ""

# ============================================================================
# Step 2: Copy to libs directory
# ============================================================================

echo "Step 2: Copying library to app..."

# For standalone CLI testing
LIBS_DIR="$SCRIPT_DIR/libs/arm64-v8a"
mkdir -p "$LIBS_DIR"
cp "$RUST_LIB_FULL" "$LIBS_DIR/libeulumdat_harmonyos_ffi${TARGET_EXT}"

# For DevEco Studio project
DEVECO_LIBS_DIR="$SCRIPT_DIR/Eulumdat/entry/libs/arm64-v8a"
mkdir -p "$DEVECO_LIBS_DIR"
cp "$RUST_LIB_FULL" "$DEVECO_LIBS_DIR/libeulumdat_harmonyos_ffi${TARGET_EXT}"

echo "  Copied to: $LIBS_DIR"
echo "  Copied to: $DEVECO_LIBS_DIR"
echo ""

# ============================================================================
# Step 3: Copy Cangjie sources to DevEco project
# ============================================================================

echo "Step 3: Copying Cangjie sources to DevEco project..."

CANGJIE_SRC="$SCRIPT_DIR/src"
DEVECO_CANGJIE="$SCRIPT_DIR/Eulumdat/entry/src/main/cangjie"

mkdir -p "$DEVECO_CANGJIE"
cp -r "$CANGJIE_SRC/eulumdat" "$DEVECO_CANGJIE/"
cp -r "$CANGJIE_SRC/ui" "$DEVECO_CANGJIE/" 2>/dev/null || true

echo "  Copied Cangjie sources"
echo ""

# ============================================================================
# Step 4: Build standalone CLI (optional)
# ============================================================================

if command -v cjpm &> /dev/null; then
    echo "Step 4: Building Cangjie CLI..."

    cd "$SCRIPT_DIR"

    if [ "$BUILD_TYPE" = "debug" ]; then
        cjpm build || echo "  Note: cjpm build failed (expected if not on HarmonyOS SDK)"
    else
        cjpm build --release || echo "  Note: cjpm build failed (expected if not on HarmonyOS SDK)"
    fi

    echo ""
else
    echo "Step 4: Skipping Cangjie CLI build (cjpm not found)"
    echo "  Note: Install HarmonyOS SDK and cjpm for Cangjie development"
    echo ""
fi

# ============================================================================
# Step 5: Run test (optional)
# ============================================================================

if [ "$RUN_TEST" = "run" ]; then
    echo "Step 5: Running CLI test..."

    CLI_BIN="$SCRIPT_DIR/release/bin/eulumdat_harmonyos"

    if [ -f "$CLI_BIN" ]; then
        export LD_LIBRARY_PATH="$LIBS_DIR:${LD_LIBRARY_PATH:-}"
        export DYLD_LIBRARY_PATH="$LIBS_DIR:${DYLD_LIBRARY_PATH:-}"

        "$CLI_BIN"
    else
        echo "  CLI binary not found: $CLI_BIN"
        echo "  Note: Build with cjpm first"
    fi

    echo ""
fi

# ============================================================================
# Done
# ============================================================================

echo "=== Build Complete ==="
echo ""
echo "Next steps:"
echo "  1. Open EulumdatHarmonyOS/Eulumdat in DevEco Studio"
echo "  2. Build and run on HarmonyOS device/emulator"
echo ""
echo "For ARM64 cross-compilation (Linux/macOS to HarmonyOS):"
echo "  rustup target add aarch64-linux-ohos"
echo "  cargo build --release -p eulumdat-harmonyos-ffi --target aarch64-linux-ohos"
echo ""
