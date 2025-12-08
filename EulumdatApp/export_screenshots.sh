#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   # 1) Default (single bundle)
#   ./export_screenshots.sh
#     -> uses build/Screenshots-macOS.xcresult
#     -> outputs to build/Screenshots-macOS-exported
#
#   # 2) Single specific bundle
#   ./export_screenshots.sh path/to/Result.xcresult [output_dir]
#
#   # 3) Batch mode: pass a directory (e.g. build)
#   ./export_screenshots.sh build
#     -> finds build/Screenshots*.xcresult
#     -> each gets its own "<bundle-name>-exported" directory next to it

DEFAULT_BUNDLE="build/Screenshots-macOS.xcresult"
DEFAULT_OUT_DIR="build/Screenshots-macOS-exported"

maybe_export_bundle() {
  local bundle="$1"
  local out_dir="$2"

  if [[ ! -d "$bundle" ]]; then
    echo "  [skip] Not a directory: $bundle"
    return
  fi

  echo "  Using xcresult:   $bundle"
  echo "  Exporting to dir: $out_dir"

  rm -rf "$out_dir"
  mkdir -p "$out_dir"

  xcrun xcresulttool export attachments \
    --path "$bundle" \
    --output-path "$out_dir"

  echo "  Done."
  echo
}

# --- Argument handling ---

if [[ $# -eq 0 ]]; then
  # No args -> default single bundle
  echo "Mode: single bundle (default)"
  maybe_export_bundle "$DEFAULT_BUNDLE" "$DEFAULT_OUT_DIR"
  exit 0
fi

FIRST_ARG="$1"

# If the first arg is a directory and does NOT end with .xcresult -> batch mode
if [[ -d "$FIRST_ARG" && "${FIRST_ARG##*.}" != "xcresult" ]]; then
  ROOT_DIR="$FIRST_ARG"
  echo "Mode: batch"
  echo "Root dir: $ROOT_DIR"
  echo

  shopt -s nullglob
  found_any=false

  for bundle in "$ROOT_DIR"/Screenshots*.xcresult; do
    found_any=true
    base="$(basename "$bundle")"
    # e.g. Screenshots-macOS.xcresult -> Screenshots-macOS-exported
    out_dir="$(dirname "$bundle")/${base%.xcresult}-exported"
    maybe_export_bundle "$bundle" "$out_dir"
  done

  if [[ "$found_any" = false ]]; then
    echo "No bundles matching ${ROOT_DIR}/Screenshots*.xcresult found."
    exit 1
  fi

  exit 0
fi

# Otherwise: single bundle mode with optional explicit output dir
BUNDLE_PATH="$FIRST_ARG"
OUT_DIR="${2:-}"

if [[ -z "$OUT_DIR" ]]; then
  # If not given, derive "<bundle-name>-exported" next to the bundle
  base="$(basename "$BUNDLE_PATH")"
  OUT_DIR="$(dirname "$BUNDLE_PATH")/${base%.xcresult}-exported"
fi

echo "Mode: single bundle (explicit)"
maybe_export_bundle "$BUNDLE_PATH" "$OUT_DIR"