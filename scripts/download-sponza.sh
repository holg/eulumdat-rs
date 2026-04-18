#!/bin/bash
# Download the Sponza glTF scene from Khronos glTF-Sample-Assets
# and environment maps from bevy_bistro_scene for the obscura_demo example.

set -e
cd "$(dirname "$0")/.."

SPONZA_DIR="assets/sponza"
ENVMAP_DIR="assets/environment_maps"
SPONZA_BASE="https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models/Sponza/glTF"

# --- Sponza scene ---
if [ -f "$SPONZA_DIR/Sponza.gltf" ]; then
    echo "Sponza already downloaded in $SPONZA_DIR"
else
    echo "Downloading Sponza scene (~50 MB)..."
    mkdir -p "$SPONZA_DIR"

    # Get file list from GitHub API
    FILES=$(curl -s "https://api.github.com/repos/KhronosGroup/glTF-Sample-Assets/contents/Models/Sponza/glTF" | python3 -c "
import sys, json
for f in json.load(sys.stdin):
    print(f['name'])
")

    total=$(echo "$FILES" | wc -l | tr -d ' ')
    i=0
    for f in $FILES; do
        i=$((i + 1))
        printf "\r  [%d/%d] %s" "$i" "$total" "$f"
        curl -sL "$SPONZA_BASE/$f" -o "$SPONZA_DIR/$f"
    done
    echo ""
    echo "Sponza downloaded to $SPONZA_DIR"
fi

# --- Environment maps ---
if [ -f "$ENVMAP_DIR/pisa_specular_rgb9e5_zstd.ktx2" ]; then
    echo "Environment maps already downloaded in $ENVMAP_DIR"
else
    echo "Downloading environment maps..."
    mkdir -p "$ENVMAP_DIR"
    BISTRO_BASE="https://raw.githubusercontent.com/DGriffin91/bevy_bistro_scene/main/assets/environment_maps"
    curl -sL "$BISTRO_BASE/pisa_diffuse_rgb9e5_zstd.ktx2" -o "$ENVMAP_DIR/pisa_diffuse_rgb9e5_zstd.ktx2"
    curl -sL "$BISTRO_BASE/pisa_specular_rgb9e5_zstd.ktx2" -o "$ENVMAP_DIR/pisa_specular_rgb9e5_zstd.ktx2"
    echo "Environment maps downloaded to $ENVMAP_DIR"
fi

echo ""
echo "Ready! Run the demo with:"
echo "  cargo run --example obscura_demo --features egui-ui,post-process"
