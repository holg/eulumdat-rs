#!/usr/bin/env bash
# Prepare HarmonyOS release artifacts
#
# This script builds the Rust FFI library and packages it for release.
# Run this locally before creating a GitHub release.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

VERSION="${1:-$(grep '^version = "' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')}"

info "Preparing HarmonyOS release artifacts for version $VERSION"

# Build the Rust library
info "Building Rust FFI library for HarmonyOS..."
cd "$REPO_ROOT"

if ! rustup target list --installed | grep -q "aarch64-unknown-linux-ohos"; then
    info "Installing aarch64-unknown-linux-ohos target..."
    rustup target add aarch64-unknown-linux-ohos
fi

cargo build --release -p eulumdat-harmonyos-ffi --target aarch64-unknown-linux-ohos

# Create release directory
RELEASE_DIR="$REPO_ROOT/release-artifacts/harmonyos"
mkdir -p "$RELEASE_DIR"

# Copy library
SO_FILE="$REPO_ROOT/target/aarch64-unknown-linux-ohos/release/libeulumdat_harmonyos_ffi.so"
if [[ -f "$SO_FILE" ]]; then
    cp "$SO_FILE" "$RELEASE_DIR/"
    info "Copied library to $RELEASE_DIR/"

    # Create a zip with the library and instructions
    cd "$RELEASE_DIR"
    cat > README.txt << EOF
Eulumdat HarmonyOS FFI Library v$VERSION

Installation:
1. Copy libeulumdat_harmonyos_ffi.so to:
   EulumdatHarmonyOS/Eulumdat/entry/libs/arm64-v8a/

2. Open the project in DevEco Studio:
   EulumdatHarmonyOS/Eulumdat/

3. Build and run on your HarmonyOS device or emulator.

For more information, visit:
https://github.com/htr/eulumdat-rs
EOF

    zip -r "harmonyos-eulumdat-ffi-$VERSION.zip" libeulumdat_harmonyos_ffi.so README.txt
    rm README.txt

    info "Created: $RELEASE_DIR/harmonyos-eulumdat-ffi-$VERSION.zip"
else
    warn "Library not found at $SO_FILE"
    warn "Make sure you have the OHOS toolchain configured in .cargo/config.toml"
    exit 1
fi

echo ""
info "Release artifacts ready!"
info "Upload these files to GitHub Releases:"
ls -la "$RELEASE_DIR"/*.zip
