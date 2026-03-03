#!/bin/bash
# Build script for eulumdat-plugin WASM

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building Eulumdat Plugin WASM ==="

# Build for wasm32-unknown-unknown target
echo "[1/4] Building WASM..."
cargo build --release --target wasm32-unknown-unknown

# Run wasm-bindgen to generate JS bindings
echo "[2/4] Generating JS bindings..."
mkdir -p dist
wasm-bindgen \
    --target web \
    --out-dir dist \
    --out-name eulumdat_plugin \
    ../../target/wasm32-unknown-unknown/release/eulumdat_plugin.wasm

# Optimize WASM (if wasm-opt is available)
if command -v wasm-opt &> /dev/null; then
    echo "[3/4] Optimizing WASM..."
    wasm-opt -Oz dist/eulumdat_plugin_bg.wasm -o dist/eulumdat_plugin_bg.wasm
else
    echo "[3/4] Skipping WASM optimization (wasm-opt not found)"
fi

# Copy manifest
echo "[4/4] Copying manifest..."
cp manifest.json dist/

# Show sizes
echo ""
echo "=== Build Complete ==="
if [ -f dist/eulumdat_plugin_bg.wasm ]; then
    WASM_SIZE=$(ls -lh dist/eulumdat_plugin_bg.wasm | awk '{print $5}')
    JS_SIZE=$(ls -lh dist/eulumdat_plugin.js | awk '{print $5}')
    echo "  WASM: $WASM_SIZE"
    echo "  JS:   $JS_SIZE"
fi
echo ""
echo "Output: $SCRIPT_DIR/dist/"
echo ""
echo "Files:"
ls -la dist/
