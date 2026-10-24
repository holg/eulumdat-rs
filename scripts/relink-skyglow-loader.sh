#!/usr/bin/env bash
# Point dist/index.html at the most recent already-built, content-hashed
# skyglow loader when the skyglow bundle is disabled in build-config.toml
# (bundles.skyglow = false). build-wasm-split.sh only rewrites loader
# references for bundles it built, so a fresh Leptos build would otherwise
# ship the un-hashed dev loader (which imports a non-existent
# ./skyglow/skyglow-demo.js). Run after build-wasm-split.sh, then run
# build-wasm-split.sh once more so index.html.br is refreshed.
set -euo pipefail
DIST="$(cd "$(dirname "$0")/.." && pwd)/crates/eulumdat-wasm/dist"
LOADER=$(ls -t "$DIST"/skyglow-loader-*.js 2>/dev/null | head -1 || true)
if [[ -z "$LOADER" ]]; then
    echo "relink-skyglow-loader: no hashed skyglow loader in $DIST — nothing to do"
    exit 0
fi
NAME=$(basename "$LOADER")
if grep -q "$NAME" "$DIST/index.html"; then
    echo "relink-skyglow-loader: index.html already references $NAME"
    exit 0
fi
if [[ "$(uname)" == "Darwin" ]]; then
    sed -i '' "s|skyglow-loader\.js\"|$NAME\"|g; s|skyglow-loader-[a-f0-9]*\.js\"|$NAME\"|g" "$DIST/index.html"
else
    sed -i "s|skyglow-loader\.js\"|$NAME\"|g; s|skyglow-loader-[a-f0-9]*\.js\"|$NAME\"|g" "$DIST/index.html"
fi
rm -f "$DIST/index.html.br"
echo "relink-skyglow-loader: index.html -> $NAME"
