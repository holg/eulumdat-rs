#!/bin/bash
# Build single WASM binary: main editor with templates embedded + typst PDF export
#
# Templates are compiled directly into the WASM via include_str!() — no separate module.
# Typst WASM compiler is loaded on-demand from dist/typst/ for PDF export.
#
# Usage:
#   ./scripts/build-single.sh          # Build release
#   ./scripts/build-single.sh serve    # Build debug + serve with hot-reload
#   ./scripts/build-single.sh deploy   # Build release and deploy via rsync

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WASM_DIR="$ROOT_DIR/crates/eulumdat-wasm"
DIST_DIR="$WASM_DIR/dist"
MODE="${1:-release}"

echo "=== Eulumdat WASM Editor (single binary) ==="
echo ""

case "$MODE" in
  serve)
    echo "→ Building + serving (debug, hot-reload) on http://127.0.0.1:8044"
    cd "$WASM_DIR"
    trunk serve
    ;;

  deploy)
    echo "→ Building release..."
    cd "$WASM_DIR"
    trunk build --release

    DEPLOY_TARGET=$(grep '^target' "$SCRIPT_DIR/build-config.toml" | head -1 | sed 's/.*= *"//' | sed 's/".*//')
    if [ -z "$DEPLOY_TARGET" ]; then
      echo "ERROR: No deploy target in build-config.toml"
      exit 1
    fi

    echo "→ Deploying to $DEPLOY_TARGET ..."
    rsync -avz "$DIST_DIR/" "$DEPLOY_TARGET/"
    echo ""
    echo "Deploy complete."
    ;;

  *)
    echo "→ Building release..."
    cd "$WASM_DIR"
    trunk build --release
    echo ""
    echo "Build complete: $DIST_DIR"
    echo "To serve: python3 -m http.server 8042 -d $DIST_DIR"
    ;;
esac
