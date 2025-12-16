#!/bin/bash
# Build both WASM apps and combine them

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Building Eulumdat Bevy 3D Viewer (first) ==="
cd "$ROOT_DIR/crates/eulumdat-bevy"
trunk build --release

echo ""
echo "=== Building Eulumdat WASM Editor ==="
cd "$ROOT_DIR/crates/eulumdat-wasm"
trunk build --release

echo ""
echo "=== Copying Bevy build to dist/bevy/ (after trunk finishes) ==="
mkdir -p "$ROOT_DIR/crates/eulumdat-wasm/dist/bevy"
cp -r "$ROOT_DIR/crates/eulumdat-bevy/dist/"* "$ROOT_DIR/crates/eulumdat-wasm/dist/bevy/"

if [ -f "$ROOT_DIR/crates/eulumdat-wasm/dist/bevy/index.html" ]; then
    echo "Bevy app ready at dist/bevy/index.html"
fi

echo ""
echo "=== Build complete! ==="
echo ""
echo "To serve: python3 -m http.server 8080 -d $ROOT_DIR/crates/eulumdat-wasm/dist"
echo "Then open: http://localhost:8080"
