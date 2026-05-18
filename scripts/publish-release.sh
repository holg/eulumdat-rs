#!/usr/bin/env bash
# Publish the 0.7.0 release to crates.io in correct dependency order.
#
# Excluded crates (always):
#   - eulumdat-bevy, eulumdat-bevy-rt    git-dep on holg/bevy.git fork
#   - eulumdat-dev-server, eulumdat-wasm-street, eulumdat-wasm-quiz,
#     eulumdat-wasm-templates, eulumdat-harmonyos-ffi
#     publish = false (internal tooling / WASM bundles)
#
# Python bindings (eulumdat-py, eulumdat-quiz-py) are NOT published
# here — they ship via `maturin publish` to PyPI, not cargo to crates.io.
# Run those separately.
#
# Usage:
#   scripts/publish-release.sh --dry-run        # default: dry-run only
#   scripts/publish-release.sh --execute        # actually publish
#   scripts/publish-release.sh --from <crate>   # resume from a specific crate
#
# Environment:
#   CARGO_REGISTRY_TOKEN  must be set for --execute (or `cargo login`d)
#   PUBLISH_DELAY=20      seconds to sleep between publishes (default 20)
#                         crates.io needs time to propagate the new
#                         version before the next dependent can resolve it.

set -euo pipefail

cd "$(dirname "$0")/.."

# Publish order (each row must already have its deps on crates.io):
ORDER=(
    eulumdat-i18n
    eulumdat
    eulumdat-photweb
    eulumdat-plugin
    eulumdat-typst
    eulumdat-quiz
    eulumdat-goniosim
    eulumdat-rt
    eulumdat-gltf-pack
    eulumdat-ui
    eulumdat-egui
    eulumdat-ffi
    eulumdat-server
    eulumdat-tui
    eulumdat-tui-quiz
    eulumdat-windows-preview
    eulumdat-cli
    eulumdat-wasm
)

MODE="dry-run"
FROM=""
DELAY="${PUBLISH_DELAY:-20}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) MODE="dry-run"; shift ;;
        --execute) MODE="execute"; shift ;;
        --from)    FROM="$2"; shift 2 ;;
        -h|--help)
            sed -n '1,30p' "$0"
            exit 0
            ;;
        *) echo "unknown flag: $1" >&2; exit 1 ;;
    esac
done

# --no-verify needed for eulumdat-wasm — verify step would `cargo build`
# the published source on the host target, which fails because the crate
# is WASM-only (getrandom = { features = ["wasm_js"] }, leptos/csr, etc.).
NO_VERIFY_CRATES=("eulumdat-wasm")

needs_no_verify() {
    local c="$1"
    for x in "${NO_VERIFY_CRATES[@]}"; do
        [[ "$x" == "$c" ]] && return 0
    done
    return 1
}

skip=true
[[ -z "$FROM" ]] && skip=false

echo "── publish-release.sh ─────────────────────────────────────────"
echo "  mode    : $MODE"
echo "  delay   : ${DELAY}s between crates (after successful publish)"
echo "  crates  : ${#ORDER[@]}"
[[ -n "$FROM" ]] && echo "  resume from: $FROM"
echo "──────────────────────────────────────────────────────────────"
echo

for crate in "${ORDER[@]}"; do
    if [[ "$skip" == "true" ]]; then
        if [[ "$crate" == "$FROM" ]]; then
            skip=false
        else
            echo "  ⏭  skip   $crate"
            continue
        fi
    fi

    flags=("--allow-dirty")
    if needs_no_verify "$crate"; then
        flags+=("--no-verify")
    fi

    if [[ "$MODE" == "dry-run" ]]; then
        echo "  🔎 dry-run $crate"
        cargo publish -p "$crate" "${flags[@]}" --dry-run 2>&1 | \
            grep -E "(error|Packaging|Uploading|warning: profiles)" | \
            grep -v "warning: profiles" || true
    else
        echo "  📤 publish $crate"
        if cargo publish -p "$crate" "${flags[@]}"; then
            echo "  ✅ ok      $crate — sleeping ${DELAY}s for index"
            sleep "$DELAY"
        else
            echo "  ❌ FAILED  $crate"
            echo
            echo "Resume with: $0 --execute --from $crate"
            exit 1
        fi
    fi
done

echo
if [[ "$MODE" == "dry-run" ]]; then
    echo "✅ Dry-run complete. Re-run with --execute to publish."
else
    echo "✅ All ${#ORDER[@]} crates published."
    echo
    echo "Next steps (manual):"
    echo "  - maturin publish for eulumdat-py and eulumdat-quiz-py"
    echo "  - git push origin v0.7.0"
    echo "  - eulumdat-bevy stays unpublished until upstream Bevy 0.19"
fi
