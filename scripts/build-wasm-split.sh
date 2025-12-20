#!/bin/bash
# Build script for split WASM bundles
#
# Builds TWO separate WASM bundles:
#   1. Leptos editor (~3MB) - loads immediately on page load
#   2. Bevy 3D viewer (~22MB) - loads on demand when user clicks "3D Scene" tab
#
# Usage:
#   ./build-wasm-split.sh
#
# The split architecture ensures fast initial page load while still
# providing full 3D visualization capabilities when needed.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WASM_DIR="$ROOT_DIR/crates/eulumdat-wasm"
BEVY_DIR="$ROOT_DIR/crates/eulumdat-bevy"
BEVY_OUTPUT="$ROOT_DIR/target/wasm32-unknown-unknown/web-release"

# Help
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0"
    echo ""
    echo "Builds two WASM bundles:"
    echo "  - Leptos editor (~3MB) - loads immediately"
    echo "  - Bevy 3D viewer (~22MB) - loads on demand"
    echo ""
    echo "Output: $WASM_DIR/dist/"
    exit 0
fi

echo "=== Building Eulumdat Split WASM ==="
echo ""
echo "  Bundle 1: Leptos editor (loads immediately)"
echo "  Bundle 2: Bevy 3D viewer (loads on demand)"
echo ""

# Step 1: Build Bevy 3D viewer
echo "[1/4] Building Bevy 3D viewer with bevy-cli..."
cd "$BEVY_DIR"
# Note: standalone feature enables wasm-sync for localStorage polling
# Features flag must come before 'web' subcommand
bevy build --release --features standalone web
echo ""

# Step 2: Build Leptos editor
echo "[2/4] Building Leptos editor with trunk..."
cd "$WASM_DIR"
trunk build --release
echo ""

# Step 3: Add content hashes to Bevy files for cache busting
echo "[3/4] Adding content hashes to Bevy files..."
mkdir -p "$WASM_DIR/dist/bevy"

# Clean old bevy files
rm -f "$WASM_DIR/dist/bevy/"*.js "$WASM_DIR/dist/bevy/"*.wasm

# Calculate short hash (first 16 chars of md5)
JS_HASH=$(md5 -q "$BEVY_OUTPUT/eulumdat-3d.js" | cut -c1-16)
WASM_HASH=$(md5 -q "$BEVY_OUTPUT/eulumdat-3d_bg.wasm" | cut -c1-16)

# Copy with hashed names
cp "$BEVY_OUTPUT/eulumdat-3d.js" "$WASM_DIR/dist/bevy/eulumdat-3d-${JS_HASH}.js"
cp "$BEVY_OUTPUT/eulumdat-3d_bg.wasm" "$WASM_DIR/dist/bevy/eulumdat-3d-${WASM_HASH}_bg.wasm"

# Update the JS file to reference the hashed WASM filename
sed -i '' "s/eulumdat-3d_bg.wasm/eulumdat-3d-${WASM_HASH}_bg.wasm/g" "$WASM_DIR/dist/bevy/eulumdat-3d-${JS_HASH}.js"
echo ""

# Step 4: Generate bevy-loader.js with hashed filenames
echo "[4/4] Generating bevy-loader.js..."

cat > "$WASM_DIR/dist/bevy-loader.js" << EOF
// Lazy loader for Bevy 3D Scene Viewer
// Auto-generated with content hashes for cache busting
//
// The 3D viewer (~22MB) is NOT loaded until the user clicks "3D Scene" tab.
// This keeps the initial page load fast (~3MB for the editor only).

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

async function loadBevyViewer() {
    if (bevyLoaded) {
        console.log("[Bevy] Already loaded");
        return;
    }
    if (bevyLoading && loadPromise) {
        console.log("[Bevy] Loading in progress, waiting...");
        return loadPromise;
    }

    bevyLoading = true;
    console.log("[Bevy] Loading 3D viewer (~22MB)...");

    loadPromise = (async () => {
        try {
            const bevy = await import('./bevy/eulumdat-3d-${JS_HASH}.js');
            await bevy.default();
            bevyLoaded = true;
            bevyLoading = false;
            console.log("[Bevy] 3D viewer loaded successfully");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Bevy] Ignoring control flow exception (not a real error)");
                bevyLoaded = true;
                bevyLoading = false;
                return;
            }
            console.error("[Bevy] Failed to load 3D viewer:", error);
            bevyLoading = false;
            loadPromise = null;
            throw error;
        }
    })();

    return loadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;

console.log("[Bevy] Loader ready (JS: ${JS_HASH}, WASM: ${WASM_HASH})");
EOF

# Summary
echo ""
echo "=== Build Complete ==="
echo ""

LEPTOS_SIZE=$(ls -lh "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | awk '{print $5}' | head -1)
BEVY_SIZE=$(ls -lh "$WASM_DIR/dist/bevy/"*_bg.wasm 2>/dev/null | awk '{print $5}')

echo "Bundle sizes:"
echo "  Leptos editor:  $LEPTOS_SIZE (loads immediately)"
echo "  Bevy 3D viewer: $BEVY_SIZE (loads on demand)"
echo ""
echo "Hashed filenames:"
echo "  eulumdat-3d-${JS_HASH}.js"
echo "  eulumdat-3d-${WASM_HASH}_bg.wasm"
echo ""
echo "Output: $WASM_DIR/dist/"
echo ""
echo "To serve locally:"
echo "  python3 -m http.server 8042 -d $WASM_DIR/dist"
echo "  open http://localhost:8042"
