#!/bin/bash
# Universal WASM split bundle builder with Brotli pre-compression
#
# Reads configuration from build-config.toml in the same directory.
# Can be used across multiple projects (eulumdat-rs, gldf-rs, acadlisp, etc.)
#
# Usage:
#   ./build-wasm-split.sh          # Build only
#   ./build-wasm-split.sh deploy   # Build and deploy via rsync
#   ./build-wasm-split.sh serve    # Build and serve locally
#   ./build-wasm-split.sh --help   # Show help
#
# The split architecture ensures fast initial page load while still
# providing full 3D visualization and PDF export capabilities when needed.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$SCRIPT_DIR/build-config.toml"

# =============================================================================
# TOML Parser (simple key=value extraction)
# =============================================================================

# Read a value from TOML config
# Usage: toml_get "section.key" [default_value]
toml_get() {
    local key="$1"
    local default="$2"
    local section=""
    local field=""

    # Split key into section and field (e.g., "project.name" -> "project" "name")
    if [[ "$key" == *.* ]]; then
        section="${key%.*}"
        field="${key##*.}"
    else
        field="$key"
    fi

    local in_section=false
    local current_section=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        # Check for section header [section] or [section.subsection]
        if [[ "$line" =~ ^\[([a-zA-Z0-9._-]+)\] ]]; then
            current_section="${BASH_REMATCH[1]}"
            if [[ -z "$section" ]] || [[ "$current_section" == "$section" ]] || [[ "$current_section" == "$section."* ]]; then
                in_section=true
            else
                in_section=false
            fi
            continue
        fi

        # If we're in the right section (or no section specified), look for the field
        if [[ "$in_section" == true ]] || [[ -z "$section" && -z "$current_section" ]]; then
            # Match key = value or key = "value"
            if [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
                echo "${BASH_REMATCH[1]}"
                return 0
            elif [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*([^[:space:]#]+) ]]; then
                echo "${BASH_REMATCH[1]}"
                return 0
            fi
        fi
    done < "$CONFIG_FILE"

    echo "$default"
}

# Read array from TOML (simple format: key = ["a", "b"])
toml_get_array() {
    local key="$1"
    local section="${key%.*}"
    local field="${key##*.}"

    local in_section=false
    local current_section=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        if [[ "$line" =~ ^\[([a-zA-Z0-9._-]+)\] ]]; then
            current_section="${BASH_REMATCH[1]}"
            [[ "$current_section" == "$section" ]] && in_section=true || in_section=false
            continue
        fi

        if [[ "$in_section" == true ]]; then
            if [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*\[(.+)\] ]]; then
                # Extract array elements
                local array_content="${BASH_REMATCH[1]}"
                # Remove quotes and split by comma
                echo "$array_content" | tr ',' '\n' | sed 's/[" ]//g'
                return 0
            fi
        fi
    done < "$CONFIG_FILE"
}

# =============================================================================
# Load Configuration
# =============================================================================

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "ERROR: Configuration file not found: $CONFIG_FILE"
    echo ""
    echo "Create a build-config.toml file with your project settings."
    echo "See the eulumdat-rs repository for an example."
    exit 1
fi

# Project settings
PROJECT_NAME=$(toml_get "project.name" "app")
PROJECT_DISPLAY=$(toml_get "project.display_name" "$PROJECT_NAME")

# Paths (relative to ROOT_DIR)
WASM_CRATE=$(toml_get "paths.wasm_crate" "crates/wasm")
BEVY_CRATE=$(toml_get "paths.bevy_crate" "crates/bevy")
BEVY_OUTPUT_REL=$(toml_get "paths.bevy_output" "target/wasm32-unknown-unknown/web-release")
TYPST_SOURCE_REL=$(toml_get "paths.typst_source" "")
DIST_OUTPUT_REL=$(toml_get "paths.dist_output" "$WASM_CRATE/dist")
TEMPLATES_CRATE=$(toml_get "paths.templates_crate" "")
STREET_CRATE=$(toml_get "paths.street_crate" "")
ASSETS_REL=$(toml_get "paths.assets" "assets")

# Absolute paths
WASM_DIR="$ROOT_DIR/$WASM_CRATE"
BEVY_DIR="$ROOT_DIR/$BEVY_CRATE"
BEVY_OUTPUT="$ROOT_DIR/$BEVY_OUTPUT_REL"
TYPST_SOURCE="$ROOT_DIR/$TYPST_SOURCE_REL"
DIST_DIR="$ROOT_DIR/$DIST_OUTPUT_REL"
ASSETS_DIR="$ROOT_DIR/$ASSETS_REL"
TEMPLATES_DIR="$ROOT_DIR/$TEMPLATES_CRATE"
STREET_DIR="$ROOT_DIR/$STREET_CRATE"

# Bevy settings
BEVY_BINARY=$(toml_get "bevy.binary_name" "${PROJECT_NAME}-3d")
BEVY_FEATURES=$(toml_get_array "bevy.features" | tr '\n' ',' | sed 's/,$//')

# Bundle flags
BUILD_LEPTOS=$(toml_get "bundles.leptos" "true")
BUILD_BEVY=$(toml_get "bundles.bevy" "true")
BUILD_TYPST=$(toml_get "bundles.typst" "false")
BUILD_GMAPS=$(toml_get "bundles.gmaps" "false")
BUILD_TEMPLATES=$(toml_get "bundles.templates" "false")
BUILD_SKYGLOW=$(toml_get "bundles.skyglow" "false")
BUILD_STREET=$(toml_get "bundles.street" "false")

# Deploy settings
DEPLOY_TARGET=$(toml_get "deploy.target" "")
RSYNC_FLAGS=$(toml_get "deploy.rsync_flags" "-avz")
LOCAL_PORT=$(toml_get "deploy.local.port" "8042")
SERVER_CRATE=$(toml_get "deploy.local.server_crate" "")

# Pages
SECRET_PAGE=$(toml_get "pages.secret_export_page" "")
CUSTOM_404_SVG=$(toml_get "pages.custom_404_svg" "")

# Env vars
GMAPS_ENV_KEY=$(toml_get "env.google_maps_key" "GOOGLE_MAPS_API")

# =============================================================================
# Check for tools
# =============================================================================

HAVE_BROTLI=false
if command -v brotli &> /dev/null; then
    HAVE_BROTLI=true
fi

# =============================================================================
# Hash caching for incremental builds
# =============================================================================

CACHE_FILE="$ROOT_DIR/target/.wasm-build-cache"
FORCE_REBUILD=false

# Calculate hash of source files for a crate.
#
# The bundle crate rarely changes in isolation — most edits happen in the
# workspace crates it depends on (eulumdat core, atla, etc.). Hashing only
# the bundle crate's own src/ led to false "unchanged" skips and stale
# deploys. Solution: hash every `crates/*/src/**/*.rs` + every Cargo.toml
# + the root Cargo.lock. Any workspace edit invalidates every bundle's
# cache, which is the correct conservative default.
#
# Usage: calculate_source_hash <crate_dir>
calculate_source_hash() {
    local crate_dir="$1"
    if [[ ! -d "$crate_dir" ]]; then
        echo "0"
        return
    fi
    {
        # All .rs under every crate — captures workspace-wide changes.
        find "$ROOT_DIR/crates" -type f -name "*.rs" 2>/dev/null | sort | xargs cat 2>/dev/null
        # All Cargo.toml under every crate — catches feature / dep edits.
        find "$ROOT_DIR/crates" -type f -name "Cargo.toml" 2>/dev/null | sort | xargs cat 2>/dev/null
        # Lockfile — catches upstream version bumps.
        cat "$ROOT_DIR/Cargo.lock" 2>/dev/null
        # Keep the crate_dir argument honest: if a caller points us at a
        # path outside crates/ (future-proofing) we still mix it in.
        if [[ "$crate_dir" != "$ROOT_DIR/crates/"* ]]; then
            find "$crate_dir" -type f \( -name "*.rs" -o -name "Cargo.toml" \) 2>/dev/null | sort | xargs cat 2>/dev/null
        fi
    } | if command -v md5sum &> /dev/null; then md5sum | cut -c1-16; else md5 -q | cut -c1-16; fi
}

# Get cached hash for a component
# Usage: get_cached_hash <component_name>
get_cached_hash() {
    local component="$1"
    if [[ -f "$CACHE_FILE" ]]; then
        grep "^${component}=" "$CACHE_FILE" 2>/dev/null | cut -d'=' -f2
    fi
}

# Save hash to cache
# Usage: save_hash <component_name> <hash>
save_hash() {
    local component="$1"
    local hash="$2"
    mkdir -p "$(dirname "$CACHE_FILE")"
    # Remove old entry and add new one
    if [[ -f "$CACHE_FILE" ]]; then
        grep -v "^${component}=" "$CACHE_FILE" > "${CACHE_FILE}.tmp" 2>/dev/null || true
        mv "${CACHE_FILE}.tmp" "$CACHE_FILE"
    fi
    echo "${component}=${hash}" >> "$CACHE_FILE"
}

# Check if rebuild is needed
# Usage: needs_rebuild <component_name> <crate_dir>
# Returns 0 (true) if rebuild needed, 1 (false) if cached
needs_rebuild() {
    local component="$1"
    local crate_dir="$2"

    # Blanket force wins over everything.
    if [[ "$FORCE_REBUILD" == "true" ]]; then
        return 0
    fi

    # Selective force: only the named component(s) rebuild; others fall
    # through to the normal hash check.
    if [[ -n "$FORCE_MODULE" ]]; then
        case " $FORCE_MODULE " in
            *" $component "*) return 0 ;;
        esac
    fi

    local current_hash=$(calculate_source_hash "$crate_dir")
    local cached_hash=$(get_cached_hash "$component")

    if [[ "$current_hash" == "$cached_hash" ]] && [[ -n "$cached_hash" ]]; then
        return 1  # No rebuild needed
    fi
    return 0  # Rebuild needed
}

# Names of bundles that support selective `force <module>` rebuilds.
# Keep in sync with the `needs_rebuild` call sites below.
FORCE_MODULE_NAMES=("leptos" "bevy" "street" "skyglow" "templates" "eulumdat-core")

# Map bundle name → cargo crate that should be `cargo clean -p`ed when
# forcing just that module. Without this, `force street` on an unchanged
# tree is a no-op because cargo sees its build artifacts already match
# the sources — no new bytes → no new content hash → nothing re-deploys.
# Modules not listed here (e.g. `eulumdat-core`, which isn't a bundle)
# are ignored.
force_module_crate() {
    case "$1" in
        leptos)    echo "eulumdat-wasm" ;;
        bevy)      echo "eulumdat-bevy" ;;
        street)    echo "eulumdat-wasm-street" ;;
        skyglow)   echo "eulumdat-bevy" ;;
        templates) echo "eulumdat-wasm-templates" ;;
        *)         echo "" ;;
    esac
}

print_force_help() {
    echo "Usage: $0 force [<module>...]"
    echo ""
    echo "Without <module>: rebuild every bundle unconditionally."
    echo "With <module>:    force rebuild only the named bundle(s)."
    echo "                  Multiple modules can be listed, space-separated."
    echo ""
    echo "Available modules:"
    for m in "${FORCE_MODULE_NAMES[@]}"; do
        echo "  - $m"
    done
    echo ""
    echo "Examples:"
    echo "  $0 force                    # rebuild everything"
    echo "  $0 force street             # rebuild just the street designer"
    echo "  $0 force street leptos      # rebuild street + leptos editor"
}

# =============================================================================
# Command line handling
# =============================================================================

ACTION="build"
SKIP_BROTLI=false
if [[ "$1" == "deploy" ]]; then
    ACTION="deploy"
elif [[ "$1" == "serve" ]]; then
    ACTION="serve"
elif [[ "$1" == "servesimple" ]]; then
    ACTION="servesimple"
    SKIP_BROTLI=true
elif [[ "$1" == "force" ]]; then
    ACTION="build"
    shift
    if [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
        print_force_help
        exit 0
    fi
    if [[ $# -eq 0 ]]; then
        # `force` alone → rebuild everything (legacy behavior).
        FORCE_REBUILD=true
    else
        # `force <module> [<module>...]` → selective rebuild. Validate
        # each name against the known module list so typos fail loudly
        # instead of silently doing nothing.
        FORCE_MODULE=""
        for arg in "$@"; do
            valid=false
            for m in "${FORCE_MODULE_NAMES[@]}"; do
                if [[ "$m" == "$arg" ]]; then
                    valid=true
                    break
                fi
                # Bash "in_array" substring check handled in loop.
            done
            if [[ "$valid" == "false" ]]; then
                echo "Error: unknown module '$arg'." >&2
                echo "" >&2
                print_force_help >&2
                exit 2
            fi
            FORCE_MODULE="$FORCE_MODULE $arg"
        done
        FORCE_MODULE="${FORCE_MODULE# }"
        echo "Forcing rebuild of: $FORCE_MODULE"

        # Wipe the cached cargo artifacts + wasm-bindgen output for each
        # forced module so the rebuild genuinely produces fresh bytes.
        # Without this, `force <module>` on an unchanged tree is a no-op:
        # cargo sees the build artifacts as up-to-date, same WASM bytes
        # come out, same content hash, dist/ files untouched.
        for arg in $FORCE_MODULE; do
            crate=$(force_module_crate "$arg")
            if [[ -n "$crate" ]]; then
                echo "  wiping cargo cache for $crate (target: wasm32)"
                cargo clean -p "$crate" --target wasm32-unknown-unknown \
                    >/dev/null 2>&1 || true
            fi
            # Wipe bindgen output too — mapped per bundle.
            case "$arg" in
                street)    rm -rf "$ROOT_DIR/target/street-wasm-bindgen" ;;
                skyglow)   rm -rf "$ROOT_DIR/target/skyglow-wasm-bindgen" ;;
                templates) rm -rf "$ROOT_DIR/target/templates-wasm-bindgen" ;;
                # leptos / bevy do not use this intermediate dir.
            esac
        done
    fi
elif [[ "$1" == "clean" ]]; then
    echo "Cleaning build cache..."
    rm -f "$CACHE_FILE"
    rm -rf "$DIST_DIR"
    echo "Done."
    exit 0
elif [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  (none)            Build WASM bundles (incremental - skips unchanged)"
    echo "  force             Force rebuild all bundles (ignore cache)"
    echo "  force <module>    Force rebuild only the named module(s)"
    echo "  force -h          List available module names for selective force"
    echo "  deploy            Build and deploy via rsync to configured target"
    echo "  serve             Build and start local development server (with Brotli)"
    echo "  servesimple       Build and serve locally (skip Brotli, use python3)"
    echo "  clean             Remove build cache and dist directory"
    echo "  --help            Show this help"
    echo ""
    echo "Configuration: $CONFIG_FILE"
    echo ""
    echo "Project: $PROJECT_DISPLAY"
    echo "Output:  $DIST_DIR"
    if [[ -n "$DEPLOY_TARGET" ]]; then
        echo "Deploy:  $DEPLOY_TARGET"
    fi
    exit 0
fi

# =============================================================================
# Build Process
# =============================================================================

echo "=== Building $PROJECT_DISPLAY Split WASM ==="
echo ""
if [[ "$BUILD_LEPTOS" == "true" ]]; then
    echo "  Bundle 1: Leptos editor (loads immediately)"
fi
if [[ "$BUILD_BEVY" == "true" ]]; then
    echo "  Bundle 2: Bevy 3D viewer (loads on demand)"
fi
if [[ "$BUILD_TYPST" == "true" ]]; then
    echo "  Bundle 3: Typst PDF compiler (loads on demand)"
fi
if [[ "$BUILD_TEMPLATES" == "true" ]]; then
    echo "  Bundle 4: Templates (loads on demand)"
fi
if [[ "$BUILD_SKYGLOW" == "true" ]]; then
    echo "  Bundle 5: Skyglow Demo (loads on demand, ?wasm=skyglow_demo)"
fi
if [[ "$BUILD_STREET" == "true" ]]; then
    echo "  Bundle 6: Street Designer (loads on demand)"
fi
if [[ "$HAVE_BROTLI" == "true" ]]; then
    echo ""
    echo "  Brotli pre-compression: enabled"
fi
echo ""

STEP=1
TOTAL_STEPS=9

# Track what was built (for later steps)
BEVY_BUILT=false
LEPTOS_BUILT=false

# -----------------------------------------------------------------------------
# Step 1: Build Bevy 3D viewer
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Also check eulumdat core lib since bevy depends on it
    BEVY_NEEDS_BUILD=false
    if needs_rebuild "bevy" "$BEVY_DIR"; then
        BEVY_NEEDS_BUILD=true
    elif needs_rebuild "eulumdat-core" "$ROOT_DIR/crates/eulumdat"; then
        BEVY_NEEDS_BUILD=true
    elif [[ ! -f "$BEVY_OUTPUT/${BEVY_BINARY}.js" ]]; then
        BEVY_NEEDS_BUILD=true
    fi

    if [[ "$BEVY_NEEDS_BUILD" == "true" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Building Bevy 3D viewer..."
        cd "$BEVY_DIR"

        FEATURE_FLAG=""
        if [[ -n "$BEVY_FEATURES" ]]; then
            FEATURE_FLAG="--features $BEVY_FEATURES"
        fi

        if command -v bevy &> /dev/null; then
            echo "  Using bevy-cli..."
            # Answer 'n' to wasm-opt install prompt - we run wasm-opt ourselves afterwards
            echo "n" | bevy build --release --bin "$BEVY_BINARY" $FEATURE_FLAG web || true

            if [[ ! -f "$BEVY_OUTPUT/${BEVY_BINARY}.js" ]]; then
                echo "  bevy-cli didn't produce output, falling back to cargo..."
                cargo build --release --bin "$BEVY_BINARY" $FEATURE_FLAG --target wasm32-unknown-unknown
                wasm-bindgen --out-dir "$BEVY_OUTPUT" --target web \
                    "$ROOT_DIR/target/wasm32-unknown-unknown/release/${BEVY_BINARY}.wasm"
            fi
        else
            echo "  bevy-cli not found, using cargo + wasm-bindgen..."
            cargo build --release --bin "$BEVY_BINARY" $FEATURE_FLAG --target wasm32-unknown-unknown
            mkdir -p "$BEVY_OUTPUT"
            wasm-bindgen --out-dir "$BEVY_OUTPUT" --target web \
                "$ROOT_DIR/target/wasm32-unknown-unknown/release/${BEVY_BINARY}.wasm"
        fi

        if command -v wasm-opt &> /dev/null && [[ -f "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm" ]]; then
            echo "  Running wasm-opt..."
            wasm-opt -Oz -o "$BEVY_OUTPUT/${BEVY_BINARY}_bg_opt.wasm" "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm"
            mv "$BEVY_OUTPUT/${BEVY_BINARY}_bg_opt.wasm" "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm"
        fi

        # Save hashes after successful build
        save_hash "bevy" "$(calculate_source_hash "$BEVY_DIR")"
        save_hash "eulumdat-core" "$(calculate_source_hash "$ROOT_DIR/crates/eulumdat")"
        BEVY_BUILT=true
        echo ""
    else
        echo "[$STEP/$TOTAL_STEPS] Bevy 3D viewer: unchanged, skipping build"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 1.5: Build Skyglow Demo (from same Bevy crate, different binary)
#
# Two variants:
#   - WebGPU (primary, full fidelity) → target/wasm32-.../release/
#     and target/skyglow-wasm-bindgen/
#   - WebGL2 (fallback, degraded) → target/wasm32-.../release-webgl2/
#     and target/skyglow-webgl2-wasm-bindgen/
#
# The Leptos shell + skyglow-loader.js probe `navigator.gpu` at runtime
# and load whichever module the user's browser supports. Most users
# only download the bundle that matches their browser.
# -----------------------------------------------------------------------------
SKYGLOW_BINARY="skyglow-demo"
SKYGLOW_OUTPUT="$ROOT_DIR/target/wasm32-unknown-unknown/release"
SKYGLOW_BINDGEN_OUTPUT="$ROOT_DIR/target/skyglow-wasm-bindgen"
# WebGL2 sibling — same source, different cargo features. Built into a
# separate target dir (--target-dir) so its release artifacts don't
# clobber the WebGPU build's `release/skyglow-demo.wasm`.
SKYGLOW_WEBGL2_TARGET_DIR="$ROOT_DIR/target/skyglow-webgl2"
SKYGLOW_WEBGL2_OUTPUT="$SKYGLOW_WEBGL2_TARGET_DIR/wasm32-unknown-unknown/release"
SKYGLOW_WEBGL2_BINDGEN_OUTPUT="$ROOT_DIR/target/skyglow-webgl2-wasm-bindgen"
SKYGLOW_BUILT=false
if [[ "$BUILD_SKYGLOW" == "true" ]]; then
    SKYGLOW_NEEDS_BUILD=false
    if needs_rebuild "skyglow" "$BEVY_DIR"; then
        SKYGLOW_NEEDS_BUILD=true
    elif [[ ! -f "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" ]]; then
        SKYGLOW_NEEDS_BUILD=true
    elif [[ ! -f "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" ]]; then
        SKYGLOW_NEEDS_BUILD=true
    fi

    if [[ "$SKYGLOW_NEEDS_BUILD" == "true" ]]; then
        echo "[1.5/$TOTAL_STEPS] Building Skyglow Demo for WASM (WebGPU + WebGL2)..."
        cd "$ROOT_DIR"

        # ── WebGPU build (primary) ───────────────────────────────────────
        echo "  WebGPU bundle..."
        cargo build --release --target wasm32-unknown-unknown \
            --bin skyglow-demo \
            --features bevy-ui,post-process,wasm-bindgen,js-sys \
            -p eulumdat-bevy

        mkdir -p "$SKYGLOW_BINDGEN_OUTPUT"
        wasm-bindgen --out-dir "$SKYGLOW_BINDGEN_OUTPUT" --target web \
            "$SKYGLOW_OUTPUT/${SKYGLOW_BINARY}.wasm"

        if command -v wasm-opt &> /dev/null && [[ -f "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" ]]; then
            echo "    Running wasm-opt (WebGPU)..."
            ORIG_SIZE=$(ls -lh "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" | awk '{print $5}')
            wasm-opt -Oz -o "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg_opt.wasm" \
                "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm"
            mv "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg_opt.wasm" \
                "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm"
            OPT_SIZE=$(ls -lh "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" | awk '{print $5}')
            echo "    Size: $ORIG_SIZE -> $OPT_SIZE"
        fi

        # ── WebGL2 build (fallback) ──────────────────────────────────────
        # --target-dir keeps it separate from the WebGPU build so the
        # release artifacts don't collide; --no-default-features drops
        # `webgpu` so the swap to `webgl2` is unambiguous.
        echo "  WebGL2 bundle..."
        cargo build --release --target wasm32-unknown-unknown \
            --target-dir "$SKYGLOW_WEBGL2_TARGET_DIR" \
            --bin skyglow-demo \
            --no-default-features \
            --features viewer,bevy-ui,post-process,wasm-bindgen,js-sys,webgl2 \
            -p eulumdat-bevy

        mkdir -p "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT"
        wasm-bindgen --out-dir "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT" --target web \
            "$SKYGLOW_WEBGL2_OUTPUT/${SKYGLOW_BINARY}.wasm"

        if command -v wasm-opt &> /dev/null && [[ -f "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" ]]; then
            echo "    Running wasm-opt (WebGL2)..."
            ORIG_SIZE=$(ls -lh "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" | awk '{print $5}')
            wasm-opt -Oz -o "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg_opt.wasm" \
                "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm"
            mv "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg_opt.wasm" \
                "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm"
            OPT_SIZE=$(ls -lh "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" | awk '{print $5}')
            echo "    Size: $ORIG_SIZE -> $OPT_SIZE"
        fi

        save_hash "skyglow" "$(calculate_source_hash "$BEVY_DIR")"
        SKYGLOW_BUILT=true
        echo ""
    else
        echo "[1.5/$TOTAL_STEPS] Skyglow Demo: unchanged, skipping build"
    fi
fi

# -----------------------------------------------------------------------------
# Step 1.7: Build Street Designer (lazy-loaded companion, same pattern as skyglow)
# -----------------------------------------------------------------------------
STREET_CRATE_NAME="eulumdat-wasm-street"
STREET_OUTPUT="$ROOT_DIR/target/wasm32-unknown-unknown/release"
STREET_BINDGEN_OUTPUT="$ROOT_DIR/target/street-wasm-bindgen"
STREET_BUILT=false
if [[ "$BUILD_STREET" == "true" ]]; then
    STREET_NEEDS_BUILD=false
    if needs_rebuild "street" "$STREET_DIR"; then
        STREET_NEEDS_BUILD=true
    elif [[ ! -f "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" ]]; then
        STREET_NEEDS_BUILD=true
    fi

    if [[ "$STREET_NEEDS_BUILD" == "true" ]]; then
        echo "[1.7/$TOTAL_STEPS] Building Street Designer for WASM..."
        cd "$ROOT_DIR"

        # Build the street designer library as a cdylib for wasm32
        cargo build --release --target wasm32-unknown-unknown \
            --lib \
            -p "$STREET_CRATE_NAME"

        # Run wasm-bindgen to generate JS glue
        mkdir -p "$STREET_BINDGEN_OUTPUT"
        STREET_CRATE_UNDER=$(echo "$STREET_CRATE_NAME" | tr '-' '_')
        wasm-bindgen --out-dir "$STREET_BINDGEN_OUTPUT" --target web \
            "$STREET_OUTPUT/${STREET_CRATE_UNDER}.wasm"

        # wasm-bindgen outputs use underscores (eulumdat_wasm_street.js);
        # normalize to hyphenated name so downstream steps can assume a
        # consistent filename.
        if [[ -f "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_UNDER}.js" ]] && \
           [[ ! -f "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" ]]; then
            mv "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_UNDER}.js" \
                "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js"
            mv "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_UNDER}_bg.wasm" \
                "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm"
            # Rewrite the JS to match the renamed WASM
            if [[ "$(uname)" == "Darwin" ]]; then
                sed -i '' "s/${STREET_CRATE_UNDER}_bg.wasm/${STREET_CRATE_NAME}_bg.wasm/g" \
                    "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js"
            else
                sed -i "s/${STREET_CRATE_UNDER}_bg.wasm/${STREET_CRATE_NAME}_bg.wasm/g" \
                    "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js"
            fi
        fi

        # Optimize with wasm-opt if available
        if command -v wasm-opt &> /dev/null && [[ -f "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" ]]; then
            echo "  Running wasm-opt..."
            ORIG_SIZE=$(ls -lh "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" | awk '{print $5}')
            wasm-opt -Oz -o "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg_opt.wasm" \
                "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm"
            mv "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg_opt.wasm" \
                "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm"
            OPT_SIZE=$(ls -lh "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" | awk '{print $5}')
            echo "  Size: $ORIG_SIZE -> $OPT_SIZE"
        fi

        save_hash "street" "$(calculate_source_hash "$STREET_DIR")"
        STREET_BUILT=true
        echo ""
    else
        echo "[1.7/$TOTAL_STEPS] Street Designer: unchanged, skipping build"
    fi
fi

# -----------------------------------------------------------------------------
# Step 2: Build Leptos editor
# -----------------------------------------------------------------------------
if [[ "$BUILD_LEPTOS" == "true" ]]; then
    LEPTOS_NEEDS_BUILD=false
    if needs_rebuild "leptos" "$WASM_DIR"; then
        LEPTOS_NEEDS_BUILD=true
    elif needs_rebuild "eulumdat-core" "$ROOT_DIR/crates/eulumdat"; then
        LEPTOS_NEEDS_BUILD=true
    elif [[ ! -d "$DIST_DIR" ]] || [[ -z "$(ls -A "$DIST_DIR"/*.wasm 2>/dev/null)" ]]; then
        LEPTOS_NEEDS_BUILD=true
    fi

    if [[ "$LEPTOS_NEEDS_BUILD" == "true" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Building Leptos editor with trunk..."
        cd "$WASM_DIR"
        trunk build --release

        # Save hash after successful build
        save_hash "leptos" "$(calculate_source_hash "$WASM_DIR")"
        LEPTOS_BUILT=true
        echo ""
    else
        echo "[$STEP/$TOTAL_STEPS] Leptos editor: unchanged, skipping build"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 3: Add content hashes to Bevy files
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Calculate current hashes from source files
    if command -v md5sum &> /dev/null; then
        JS_HASH=$(md5sum "$BEVY_OUTPUT/${BEVY_BINARY}.js" | cut -c1-16)
        WASM_HASH=$(md5sum "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm" | cut -c1-16)
    else
        JS_HASH=$(md5 -q "$BEVY_OUTPUT/${BEVY_BINARY}.js" | cut -c1-16)
        WASM_HASH=$(md5 -q "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm" | cut -c1-16)
    fi

    # Check if hashed files already exist
    BEVY_JS_TARGET="$DIST_DIR/bevy/${BEVY_BINARY}-${JS_HASH}.js"
    BEVY_WASM_TARGET="$DIST_DIR/bevy/${BEVY_BINARY}-${WASM_HASH}_bg.wasm"

    if [[ -f "$BEVY_JS_TARGET" ]] && [[ -f "$BEVY_WASM_TARGET" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Bevy files: unchanged (hash match), skipping"
    else
        echo "[$STEP/$TOTAL_STEPS] Adding content hashes to Bevy files..."
        mkdir -p "$DIST_DIR/bevy"

        # Remove old hashed files (different hash)
        rm -f "$DIST_DIR/bevy/"*.js "$DIST_DIR/bevy/"*.wasm "$DIST_DIR/bevy/"*.br

        cp "$BEVY_OUTPUT/${BEVY_BINARY}.js" "$BEVY_JS_TARGET"
        cp "$BEVY_OUTPUT/${BEVY_BINARY}_bg.wasm" "$BEVY_WASM_TARGET"

        # Update JS to reference hashed WASM
        if [[ "$(uname)" == "Darwin" ]]; then
            sed -i '' "s/${BEVY_BINARY}_bg.wasm/${BEVY_BINARY}-${WASM_HASH}_bg.wasm/g" "$BEVY_JS_TARGET"
        else
            sed -i "s/${BEVY_BINARY}_bg.wasm/${BEVY_BINARY}-${WASM_HASH}_bg.wasm/g" "$BEVY_JS_TARGET"
        fi
        echo "  ${BEVY_BINARY}-${JS_HASH}.js"
        echo "  ${BEVY_BINARY}-${WASM_HASH}_bg.wasm"
    fi
fi
((STEP++))
echo ""

# -----------------------------------------------------------------------------
# Step 3.5: Add content hashes to Skyglow files (WebGPU + WebGL2)
# -----------------------------------------------------------------------------
hash_md5() {
    if command -v md5sum &> /dev/null; then
        md5sum "$1" | cut -c1-16
    else
        md5 -q "$1" | cut -c1-16
    fi
}
sed_inplace() {
    if [[ "$(uname)" == "Darwin" ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

SKYGLOW_JS_HASH=""
SKYGLOW_WASM_HASH=""
SKYGLOW_WEBGL2_JS_HASH=""
SKYGLOW_WEBGL2_WASM_HASH=""
if [[ "$BUILD_SKYGLOW" == "true" ]] && [[ -f "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" ]]; then
    SKYGLOW_JS_HASH=$(hash_md5 "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js")
    SKYGLOW_WASM_HASH=$(hash_md5 "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm")
    SKYGLOW_JS_TARGET="$DIST_DIR/skyglow/${SKYGLOW_BINARY}-${SKYGLOW_JS_HASH}.js"
    SKYGLOW_WASM_TARGET="$DIST_DIR/skyglow/${SKYGLOW_BINARY}-${SKYGLOW_WASM_HASH}_bg.wasm"

    # WebGL2 sibling — same naming pattern under skyglow-webgl2/.
    SKYGLOW_WEBGL2_AVAILABLE=false
    if [[ -f "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" ]]; then
        SKYGLOW_WEBGL2_AVAILABLE=true
        SKYGLOW_WEBGL2_JS_HASH=$(hash_md5 "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js")
        SKYGLOW_WEBGL2_WASM_HASH=$(hash_md5 "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm")
    fi
    SKYGLOW_WEBGL2_JS_TARGET="$DIST_DIR/skyglow-webgl2/${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_JS_HASH}.js"
    SKYGLOW_WEBGL2_WASM_TARGET="$DIST_DIR/skyglow-webgl2/${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_WASM_HASH}_bg.wasm"

    UNCHANGED=true
    [[ -f "$SKYGLOW_JS_TARGET" ]] && [[ -f "$SKYGLOW_WASM_TARGET" ]] || UNCHANGED=false
    if [[ "$SKYGLOW_WEBGL2_AVAILABLE" == "true" ]]; then
        [[ -f "$SKYGLOW_WEBGL2_JS_TARGET" ]] && [[ -f "$SKYGLOW_WEBGL2_WASM_TARGET" ]] || UNCHANGED=false
    fi

    if [[ "$UNCHANGED" == "true" ]]; then
        echo "[3.5/$TOTAL_STEPS] Skyglow files: unchanged (hash match), skipping"
    else
        echo "[3.5/$TOTAL_STEPS] Adding content hashes to Skyglow files..."

        # ── WebGPU primary ──────────────────────────────────────────────
        mkdir -p "$DIST_DIR/skyglow"
        rm -f "$DIST_DIR/skyglow/"*.js "$DIST_DIR/skyglow/"*.wasm "$DIST_DIR/skyglow/"*.br
        cp "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" "$SKYGLOW_JS_TARGET"
        cp "$SKYGLOW_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" "$SKYGLOW_WASM_TARGET"
        sed_inplace "s/${SKYGLOW_BINARY}_bg.wasm/${SKYGLOW_BINARY}-${SKYGLOW_WASM_HASH}_bg.wasm/g" "$SKYGLOW_JS_TARGET"
        echo "  WebGPU:  ${SKYGLOW_BINARY}-${SKYGLOW_JS_HASH}.js"
        echo "           ${SKYGLOW_BINARY}-${SKYGLOW_WASM_HASH}_bg.wasm"

        # ── WebGL2 fallback ─────────────────────────────────────────────
        if [[ "$SKYGLOW_WEBGL2_AVAILABLE" == "true" ]]; then
            mkdir -p "$DIST_DIR/skyglow-webgl2"
            rm -f "$DIST_DIR/skyglow-webgl2/"*.js "$DIST_DIR/skyglow-webgl2/"*.wasm "$DIST_DIR/skyglow-webgl2/"*.br
            cp "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}.js" "$SKYGLOW_WEBGL2_JS_TARGET"
            cp "$SKYGLOW_WEBGL2_BINDGEN_OUTPUT/${SKYGLOW_BINARY}_bg.wasm" "$SKYGLOW_WEBGL2_WASM_TARGET"
            sed_inplace "s/${SKYGLOW_BINARY}_bg.wasm/${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_WASM_HASH}_bg.wasm/g" "$SKYGLOW_WEBGL2_JS_TARGET"
            echo "  WebGL2:  ${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_JS_HASH}.js"
            echo "           ${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_WASM_HASH}_bg.wasm"
        fi
    fi
fi
echo ""

# -----------------------------------------------------------------------------
# Step 3.7: Add content hashes to Street Designer files
# -----------------------------------------------------------------------------
STREET_JS_HASH=""
STREET_WASM_HASH=""
if [[ "$BUILD_STREET" == "true" ]] && [[ -f "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" ]]; then
    if command -v md5sum &> /dev/null; then
        STREET_JS_HASH=$(md5sum "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" | cut -c1-16)
        STREET_WASM_HASH=$(md5sum "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" | cut -c1-16)
    else
        STREET_JS_HASH=$(md5 -q "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" | cut -c1-16)
        STREET_WASM_HASH=$(md5 -q "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" | cut -c1-16)
    fi

    STREET_JS_TARGET="$DIST_DIR/street/${STREET_CRATE_NAME}-${STREET_JS_HASH}.js"
    STREET_WASM_TARGET="$DIST_DIR/street/${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm"

    if [[ -f "$STREET_JS_TARGET" ]] && [[ -f "$STREET_WASM_TARGET" ]]; then
        echo "[3.7/$TOTAL_STEPS] Street files: unchanged (hash match), skipping"
    else
        echo "[3.7/$TOTAL_STEPS] Adding content hashes to Street Designer files..."
        mkdir -p "$DIST_DIR/street"

        rm -f "$DIST_DIR/street/"*.js "$DIST_DIR/street/"*.wasm "$DIST_DIR/street/"*.br

        cp "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}.js" "$STREET_JS_TARGET"
        cp "$STREET_BINDGEN_OUTPUT/${STREET_CRATE_NAME}_bg.wasm" "$STREET_WASM_TARGET"

        # Update JS to reference hashed WASM
        if [[ "$(uname)" == "Darwin" ]]; then
            sed -i '' "s/${STREET_CRATE_NAME}_bg.wasm/${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm/g" "$STREET_JS_TARGET"
        else
            sed -i "s/${STREET_CRATE_NAME}_bg.wasm/${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm/g" "$STREET_JS_TARGET"
        fi
        echo "  ${STREET_CRATE_NAME}-${STREET_JS_HASH}.js"
        echo "  ${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm"
    fi
fi
echo ""

# -----------------------------------------------------------------------------
# Step 4: Optimize and hash Typst files
# -----------------------------------------------------------------------------
if [[ "$BUILD_TYPST" == "true" ]] && [[ -d "$TYPST_SOURCE" ]]; then
    # Calculate hash of source to check if we need to rebuild
    if command -v md5sum &> /dev/null; then
        TYPST_SRC_HASH=$(md5sum "$TYPST_SOURCE/typst_wasm_bg.wasm" | cut -c1-16)
    else
        TYPST_SRC_HASH=$(md5 -q "$TYPST_SOURCE/typst_wasm_bg.wasm" | cut -c1-16)
    fi

    # Check if output with this hash already exists
    EXISTING_TYPST_WASM=$(ls "$DIST_DIR/typst/typst_wasm-"*"_bg.wasm" 2>/dev/null | head -1)
    if [[ -n "$EXISTING_TYPST_WASM" ]]; then
        EXISTING_TYPST_HASH=$(basename "$EXISTING_TYPST_WASM" | sed 's/typst_wasm-\([^_]*\)_bg\.wasm/\1/')
    fi

    # Extract JS hash for later use
    if command -v md5sum &> /dev/null; then
        TYPST_JS_HASH=$(md5sum "$TYPST_SOURCE/typst_wasm.js" | cut -c1-16)
    else
        TYPST_JS_HASH=$(md5 -q "$TYPST_SOURCE/typst_wasm.js" | cut -c1-16)
    fi

    TYPST_JS_TARGET="$DIST_DIR/typst/typst_wasm-${TYPST_JS_HASH}.js"
    if [[ -f "$TYPST_JS_TARGET" ]] && [[ -n "$EXISTING_TYPST_WASM" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Typst files: unchanged (hash match), skipping"
        TYPST_WASM_HASH="$EXISTING_TYPST_HASH"
    else
        echo "[$STEP/$TOTAL_STEPS] Optimizing Typst WASM with wasm-opt..."
        mkdir -p "$DIST_DIR/typst"

        rm -f "$DIST_DIR/typst/"*.js "$DIST_DIR/typst/"*.wasm "$DIST_DIR/typst/"*.br

        TYPST_WASM_OPTIMIZED="$DIST_DIR/typst/typst_wasm_optimized.wasm"
        if command -v wasm-opt &> /dev/null; then
            echo "  Running wasm-opt -Oz..."
            ORIG_SIZE=$(ls -lh "$TYPST_SOURCE/typst_wasm_bg.wasm" | awk '{print $5}')
            wasm-opt -Oz -o "$TYPST_WASM_OPTIMIZED" "$TYPST_SOURCE/typst_wasm_bg.wasm"
            OPT_SIZE=$(ls -lh "$TYPST_WASM_OPTIMIZED" | awk '{print $5}')
            echo "  Size: $ORIG_SIZE -> $OPT_SIZE"
        else
            echo "  wasm-opt not found, copying unoptimized..."
            cp "$TYPST_SOURCE/typst_wasm_bg.wasm" "$TYPST_WASM_OPTIMIZED"
        fi

        if command -v md5sum &> /dev/null; then
            TYPST_WASM_HASH=$(md5sum "$TYPST_WASM_OPTIMIZED" | cut -c1-16)
        else
            TYPST_WASM_HASH=$(md5 -q "$TYPST_WASM_OPTIMIZED" | cut -c1-16)
        fi

        cp "$TYPST_SOURCE/typst_wasm.js" "$TYPST_JS_TARGET"
        mv "$TYPST_WASM_OPTIMIZED" "$DIST_DIR/typst/typst_wasm-${TYPST_WASM_HASH}_bg.wasm"

        if [[ "$(uname)" == "Darwin" ]]; then
            sed -i '' "s/typst_wasm_bg.wasm/typst_wasm-${TYPST_WASM_HASH}_bg.wasm/g" "$TYPST_JS_TARGET"
        else
            sed -i "s/typst_wasm_bg.wasm/typst_wasm-${TYPST_WASM_HASH}_bg.wasm/g" "$TYPST_JS_TARGET"
        fi
        echo "  typst_wasm-${TYPST_JS_HASH}.js"
        echo "  typst_wasm-${TYPST_WASM_HASH}_bg.wasm"
    fi
else
    echo "[$STEP/$TOTAL_STEPS] Skipping Typst (not configured or source not found)..."
fi
((STEP++))
echo ""

# -----------------------------------------------------------------------------
# Step 4.5: Build and hash Templates WASM module
# -----------------------------------------------------------------------------
TEMPLATES_JS_HASH=""
TEMPLATES_WASM_HASH=""
if [[ "$BUILD_TEMPLATES" == "true" ]] && [[ -d "$TEMPLATES_DIR" ]]; then
    TEMPLATES_NEEDS_BUILD=false
    if needs_rebuild "templates" "$TEMPLATES_DIR"; then
        TEMPLATES_NEEDS_BUILD=true
    elif [[ ! -d "$DIST_DIR/templates" ]] || [[ -z "$(ls -A "$DIST_DIR/templates/"*.wasm 2>/dev/null)" ]]; then
        TEMPLATES_NEEDS_BUILD=true
    fi

    TEMPLATES_WASM_PACK_OUT="$TEMPLATES_DIR/pkg"

    if [[ "$TEMPLATES_NEEDS_BUILD" == "true" ]]; then
        echo "[${STEP}/$TOTAL_STEPS] Building Templates WASM module..."
        cd "$TEMPLATES_DIR"

        wasm-pack build --target web --release --out-dir pkg

        if command -v wasm-opt &> /dev/null && [[ -f "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" ]]; then
            echo "  Running wasm-opt..."
            ORIG_SIZE=$(ls -lh "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" | awk '{print $5}')
            wasm-opt -Oz -o "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg_opt.wasm" "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm"
            mv "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg_opt.wasm" "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm"
            OPT_SIZE=$(ls -lh "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" | awk '{print $5}')
            echo "  Size: $ORIG_SIZE -> $OPT_SIZE"
        fi

        save_hash "templates" "$(calculate_source_hash "$TEMPLATES_DIR")"
    else
        echo "[${STEP}/$TOTAL_STEPS] Templates WASM: unchanged, skipping build"
    fi

    # Hash and copy templates files to dist/templates/
    if command -v md5sum &> /dev/null; then
        TEMPLATES_JS_HASH=$(md5sum "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates.js" | cut -c1-16)
        TEMPLATES_WASM_HASH=$(md5sum "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" | cut -c1-16)
    else
        TEMPLATES_JS_HASH=$(md5 -q "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates.js" | cut -c1-16)
        TEMPLATES_WASM_HASH=$(md5 -q "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" | cut -c1-16)
    fi

    TEMPLATES_JS_TARGET="$DIST_DIR/templates/eulumdat_wasm_templates-${TEMPLATES_JS_HASH}.js"
    TEMPLATES_WASM_TARGET="$DIST_DIR/templates/eulumdat_wasm_templates-${TEMPLATES_WASM_HASH}_bg.wasm"

    if [[ -f "$TEMPLATES_JS_TARGET" ]] && [[ -f "$TEMPLATES_WASM_TARGET" ]]; then
        echo "  Templates files: unchanged (hash match), skipping copy"
    else
        mkdir -p "$DIST_DIR/templates"
        rm -f "$DIST_DIR/templates/"*.js "$DIST_DIR/templates/"*.wasm "$DIST_DIR/templates/"*.br

        cp "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates.js" "$TEMPLATES_JS_TARGET"
        cp "$TEMPLATES_WASM_PACK_OUT/eulumdat_wasm_templates_bg.wasm" "$TEMPLATES_WASM_TARGET"

        # Update JS to reference hashed WASM
        if [[ "$(uname)" == "Darwin" ]]; then
            sed -i '' "s/eulumdat_wasm_templates_bg.wasm/eulumdat_wasm_templates-${TEMPLATES_WASM_HASH}_bg.wasm/g" "$TEMPLATES_JS_TARGET"
        else
            sed -i "s/eulumdat_wasm_templates_bg.wasm/eulumdat_wasm_templates-${TEMPLATES_WASM_HASH}_bg.wasm/g" "$TEMPLATES_JS_TARGET"
        fi
        echo "  eulumdat_wasm_templates-${TEMPLATES_JS_HASH}.js"
        echo "  eulumdat_wasm_templates-${TEMPLATES_WASM_HASH}_bg.wasm"
    fi
else
    echo "[${STEP}/$TOTAL_STEPS] Skipping Templates (not configured)..."
fi
((STEP++))
echo ""

# -----------------------------------------------------------------------------
# Step 5: Generate bevy-loader.js
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Check if loader already exists with correct hash reference
    EXISTING_LOADER=$(ls "$DIST_DIR/bevy-loader-"*.js 2>/dev/null | head -1)
    if [[ -n "$EXISTING_LOADER" ]] && grep -q "${BEVY_BINARY}-${JS_HASH}.js" "$EXISTING_LOADER" 2>/dev/null; then
        echo "[$STEP/$TOTAL_STEPS] bevy-loader.js: unchanged, skipping"
        BEVY_LOADER_HASH=$(echo "$EXISTING_LOADER" | sed 's/.*bevy-loader-\([^.]*\)\.js/\1/')
    else
        echo "[$STEP/$TOTAL_STEPS] Generating bevy-loader.js..."
        # Remove old loaders
        rm -f "$DIST_DIR/bevy-loader-"*.js "$DIST_DIR/bevy-loader-"*.js.br

        cat > "$DIST_DIR/bevy-loader-temp.js" << EOF
// Lazy loader for Bevy 3D Scene Viewer
// Auto-generated with content hashes for cache busting

let bevyLoaded = false;
let bevyLoading = false;
let bevyLoadPromise = null;

async function loadBevyViewer() {
    if (bevyLoaded) {
        console.log("[Bevy] Already loaded");
        return;
    }
    if (bevyLoading && bevyLoadPromise) {
        console.log("[Bevy] Loading in progress, waiting...");
        return bevyLoadPromise;
    }

    bevyLoading = true;
    console.log("[Bevy] Loading 3D viewer...");

    bevyLoadPromise = (async () => {
        try {
            const bevy = await import('./bevy/${BEVY_BINARY}-${JS_HASH}.js');
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
            bevyLoadPromise = null;
            throw error;
        }
    })();

    return bevyLoadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;

console.log("[Bevy] Loader ready (JS: ${JS_HASH}, WASM: ${WASM_HASH})");
EOF

        if command -v md5sum &> /dev/null; then
            BEVY_LOADER_HASH=$(md5sum "$DIST_DIR/bevy-loader-temp.js" | cut -c1-16)
        else
            BEVY_LOADER_HASH=$(md5 -q "$DIST_DIR/bevy-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/bevy-loader-temp.js" "$DIST_DIR/bevy-loader-${BEVY_LOADER_HASH}.js"
        echo "  bevy-loader-${BEVY_LOADER_HASH}.js"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 6: Generate typst-loader.js
# -----------------------------------------------------------------------------
if [[ "$BUILD_TYPST" == "true" ]] && [[ -d "$TYPST_SOURCE" ]]; then
    # Check if loader already exists with correct hash reference
    EXISTING_TYPST_LOADER=$(ls "$DIST_DIR/typst-loader-"*.js 2>/dev/null | head -1)
    if [[ -n "$EXISTING_TYPST_LOADER" ]] && grep -q "typst_wasm-${TYPST_JS_HASH}.js" "$EXISTING_TYPST_LOADER" 2>/dev/null; then
        echo "[$STEP/$TOTAL_STEPS] typst-loader.js: unchanged, skipping"
        TYPST_LOADER_HASH=$(echo "$EXISTING_TYPST_LOADER" | sed 's/.*typst-loader-\([^.]*\)\.js/\1/')
    else
        echo "[$STEP/$TOTAL_STEPS] Generating typst-loader.js..."
        rm -f "$DIST_DIR/typst-loader-"*.js "$DIST_DIR/typst-loader-"*.js.br

        cat > "$DIST_DIR/typst-loader-temp.js" << EOF
// Typst WASM loader for PDF compilation
// Auto-generated with content hashes for cache busting

let typstModule = null;
let typstInitPromise = null;

async function initTypst() {
    if (typstModule) return typstModule;
    if (typstInitPromise) return typstInitPromise;

    typstInitPromise = (async () => {
        try {
            console.log('[Typst] Loading PDF compiler...');
            const module = await import('./typst/typst_wasm-${TYPST_JS_HASH}.js');
            await module.default();
            typstModule = module;
            console.log('[Typst] PDF compiler loaded successfully');
            return module;
        } catch (e) {
            console.error('[Typst] Failed to load:', e);
            typstInitPromise = null;
            throw e;
        }
    })();

    return typstInitPromise;
}

window.compileTypstToPdf = async function(typstSource) {
    const module = await initTypst();
    try {
        const pdfBytes = module.compile_to_pdf(typstSource);
        return pdfBytes;
    } catch (e) {
        console.error('[Typst] Compilation error:', e);
        throw new Error('Typst compilation failed: ' + e);
    }
};

window.isTypstLoaded = function() { return typstModule !== null; };
window.preloadTypst = async function() { await initTypst(); };

console.log("[Typst] Loader ready (JS: ${TYPST_JS_HASH}, WASM: ${TYPST_WASM_HASH})");
EOF

        if command -v md5sum &> /dev/null; then
            TYPST_LOADER_HASH=$(md5sum "$DIST_DIR/typst-loader-temp.js" | cut -c1-16)
        else
            TYPST_LOADER_HASH=$(md5 -q "$DIST_DIR/typst-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/typst-loader-temp.js" "$DIST_DIR/typst-loader-${TYPST_LOADER_HASH}.js"
        echo "  typst-loader-${TYPST_LOADER_HASH}.js"
    fi
else
    echo "[$STEP/$TOTAL_STEPS] Skipping typst-loader.js (not configured)..."
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 6.5: Generate gmaps-loader.js (if configured)
# -----------------------------------------------------------------------------
if [[ "$BUILD_GMAPS" == "true" ]] && [[ -f "$WASM_DIR/src/static/gmaps-loader.js" ]]; then
    # Calculate expected hash from source file (with API key substitution)
    GMAPS_API_KEY=""
    if [[ -f "$ROOT_DIR/.env" ]]; then
        GMAPS_API_KEY=$(grep "^${GMAPS_ENV_KEY}=" "$ROOT_DIR/.env" | cut -d'=' -f2)
    fi

    # Create temp content to check hash
    if [[ -n "$GMAPS_API_KEY" ]]; then
        GMAPS_EXPECTED_CONTENT=$(sed "s/__GMAPS_API_KEY__/${GMAPS_API_KEY}/g" "$WASM_DIR/src/static/gmaps-loader.js")
    else
        GMAPS_EXPECTED_CONTENT=$(cat "$WASM_DIR/src/static/gmaps-loader.js")
    fi

    if command -v md5sum &> /dev/null; then
        GMAPS_EXPECTED_HASH=$(echo "$GMAPS_EXPECTED_CONTENT" | md5sum | cut -c1-16)
    else
        GMAPS_EXPECTED_HASH=$(echo "$GMAPS_EXPECTED_CONTENT" | md5 -q | cut -c1-16)
    fi

    # Check if loader with this hash exists
    if [[ -f "$DIST_DIR/gmaps-loader-${GMAPS_EXPECTED_HASH}.js" ]]; then
        echo "[6.5/$TOTAL_STEPS] gmaps-loader.js: unchanged, skipping"
        GMAPS_LOADER_HASH="$GMAPS_EXPECTED_HASH"
    else
        echo "[6.5/$TOTAL_STEPS] Generating gmaps-loader.js with API key..."
        rm -f "$DIST_DIR/gmaps-loader-"*.js "$DIST_DIR/gmaps-loader-"*.js.br

        echo "$GMAPS_EXPECTED_CONTENT" > "$DIST_DIR/gmaps-loader-${GMAPS_EXPECTED_HASH}.js"
        GMAPS_LOADER_HASH="$GMAPS_EXPECTED_HASH"

        if [[ -n "$GMAPS_API_KEY" ]]; then
            echo "  API key injected from .env"
        else
            echo "  WARNING: No $GMAPS_ENV_KEY key found in .env"
        fi
        echo "  gmaps-loader-${GMAPS_LOADER_HASH}.js"
    fi
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 6.7: Generate templates-loader.js (if configured)
# -----------------------------------------------------------------------------
TEMPLATES_LOADER_HASH=""
if [[ "$BUILD_TEMPLATES" == "true" ]] && [[ -n "$TEMPLATES_JS_HASH" ]]; then
    EXISTING_TEMPLATES_LOADER=$(ls "$DIST_DIR/templates-loader-"*.js 2>/dev/null | head -1)
    if [[ -n "$EXISTING_TEMPLATES_LOADER" ]] && grep -q "eulumdat_wasm_templates-${TEMPLATES_JS_HASH}.js" "$EXISTING_TEMPLATES_LOADER" 2>/dev/null; then
        echo "[6.7/$TOTAL_STEPS] templates-loader.js: unchanged, skipping"
        TEMPLATES_LOADER_HASH=$(echo "$EXISTING_TEMPLATES_LOADER" | sed 's/.*templates-loader-\([^.]*\)\.js/\1/')
    else
        echo "[6.7/$TOTAL_STEPS] Generating templates-loader.js..."
        rm -f "$DIST_DIR/templates-loader-"*.js "$DIST_DIR/templates-loader-"*.js.br

        cat > "$DIST_DIR/templates-loader-temp.js" << EOF
// Templates WASM loader for lazy-loaded template content
// Auto-generated with content hashes for cache busting

let templatesModule = null;
let templatesInitPromise = null;

async function initTemplates() {
    if (templatesModule) return templatesModule;
    if (templatesInitPromise) return templatesInitPromise;

    templatesInitPromise = (async () => {
        try {
            console.log('[Templates] Loading templates module...');
            const module = await import('./templates/eulumdat_wasm_templates-${TEMPLATES_JS_HASH}.js');
            await module.default();
            templatesModule = module;
            console.log('[Templates] Module loaded successfully');
            return module;
        } catch (e) {
            console.error('[Templates] Failed to load:', e);
            templatesInitPromise = null;
            throw e;
        }
    })();

    return templatesInitPromise;
}

window.getTemplateContent = async function(id) {
    const module = await initTemplates();
    return module.get_template_content(id);
};

window.isTemplatesLoaded = function() {
    return templatesModule !== null;
};

window.preloadTemplates = async function() {
    await initTemplates();
};

console.log("[Templates] Loader ready (JS: ${TEMPLATES_JS_HASH}, WASM: ${TEMPLATES_WASM_HASH})");
EOF

        if command -v md5sum &> /dev/null; then
            TEMPLATES_LOADER_HASH=$(md5sum "$DIST_DIR/templates-loader-temp.js" | cut -c1-16)
        else
            TEMPLATES_LOADER_HASH=$(md5 -q "$DIST_DIR/templates-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/templates-loader-temp.js" "$DIST_DIR/templates-loader-${TEMPLATES_LOADER_HASH}.js"
        echo "  templates-loader-${TEMPLATES_LOADER_HASH}.js"
    fi
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 6.8: Generate skyglow-loader.js (if configured)
# -----------------------------------------------------------------------------
SKYGLOW_LOADER_HASH=""
if [[ "$BUILD_SKYGLOW" == "true" ]] && [[ -n "$SKYGLOW_JS_HASH" ]]; then
    # Cache check — only regenerate the loader if either bundle's JS hash
    # changed. We grep for both because either side rotating means stale.
    EXISTING_SKYGLOW_LOADER=$(ls "$DIST_DIR/skyglow-loader-"*.js 2>/dev/null | head -1)
    LOADER_FRESH=false
    if [[ -n "$EXISTING_SKYGLOW_LOADER" ]] \
        && grep -q "${SKYGLOW_BINARY}-${SKYGLOW_JS_HASH}.js" "$EXISTING_SKYGLOW_LOADER" 2>/dev/null; then
        if [[ "$SKYGLOW_WEBGL2_AVAILABLE" != "true" ]] \
            || grep -q "${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_JS_HASH}.js" "$EXISTING_SKYGLOW_LOADER" 2>/dev/null; then
            LOADER_FRESH=true
        fi
    fi

    if [[ "$LOADER_FRESH" == "true" ]]; then
        echo "[6.8/$TOTAL_STEPS] skyglow-loader.js: unchanged, skipping"
        SKYGLOW_LOADER_HASH=$(echo "$EXISTING_SKYGLOW_LOADER" | sed 's/.*skyglow-loader-\([^.]*\)\.js/\1/')
    else
        echo "[6.8/$TOTAL_STEPS] Generating skyglow-loader.js..."
        rm -f "$DIST_DIR/skyglow-loader-"*.js "$DIST_DIR/skyglow-loader-"*.js.br

        # Two paths embedded; runtime probe picks one. The webgl2 path is
        # rendered as a JS string literal (or `null`) so we can use
        # `if (webgl2Path)` directly in the loader.
        SKYGLOW_WEBGPU_PATH="./skyglow/${SKYGLOW_BINARY}-${SKYGLOW_JS_HASH}.js"
        if [[ "$SKYGLOW_WEBGL2_AVAILABLE" == "true" ]]; then
            SKYGLOW_WEBGL2_PATH_JS="\"./skyglow-webgl2/${SKYGLOW_BINARY}-${SKYGLOW_WEBGL2_JS_HASH}.js\""
            SKYGLOW_WEBGL2_HASH_LINE="; WebGL2 JS: ${SKYGLOW_WEBGL2_JS_HASH}, WASM: ${SKYGLOW_WEBGL2_WASM_HASH}"
        else
            SKYGLOW_WEBGL2_PATH_JS="null"
            SKYGLOW_WEBGL2_HASH_LINE=""
        fi

        cat > "$DIST_DIR/skyglow-loader-temp.js" << EOF
// Skyglow Demo loader — Darkness Preservation Simulator
// Auto-generated by scripts/build-wasm-split.sh with content hashes for
// cache busting. Probes WebGPU at runtime and loads the matching Bevy
// bundle (WebGPU primary, WebGL2 fallback).

let skyglowLoaded = false;
let skyglowLoading = false;
let skyglowLoadPromise = null;
// 'webgpu' | 'webgl2' | 'unsupported' — set during the probe; the Leptos
// shell reads window.skyglowBackend to show the right banner.
window.skyglowBackend = null;

async function probeWebGpu() {
    if (!navigator.gpu) return false;
    try {
        const adapter = await navigator.gpu.requestAdapter();
        return !!adapter;
    } catch (_) {
        return false;
    }
}

// ?force=webgl2 / ?force=webgpu overrides the auto-probe. Useful for
// previewing the fallback experience or writing reply links like
// "?wasm=skyglow_demo&force=webgl2".
function forcedBackend() {
    try {
        const p = new URLSearchParams(window.location.search).get('force');
        return p === 'webgl2' || p === 'webgpu' ? p : null;
    } catch (_) {
        return null;
    }
}

async function loadSkyglowDemo() {
    if (skyglowLoaded) {
        console.log("[Skyglow] Already loaded");
        return;
    }
    if (skyglowLoading && skyglowLoadPromise) {
        console.log("[Skyglow] Loading in progress, waiting...");
        return skyglowLoadPromise;
    }

    skyglowLoading = true;
    console.log("[Skyglow] Probing WebGPU…");

    skyglowLoadPromise = (async () => {
        const forced = forcedBackend();
        const webgpuOk = forced === 'webgl2' ? false
                       : forced === 'webgpu' ? true
                       : await probeWebGpu();
        const webgl2Path = ${SKYGLOW_WEBGL2_PATH_JS};
        let modulePath;

        if (webgpuOk) {
            window.skyglowBackend = 'webgpu';
            modulePath = '${SKYGLOW_WEBGPU_PATH}';
            console.log(
                forced === 'webgpu'
                    ? "[Skyglow] WebGPU forced via ?force=webgpu"
                    : "[Skyglow] WebGPU available — loading primary bundle"
            );
        } else if (webgl2Path) {
            window.skyglowBackend = 'webgl2';
            modulePath = webgl2Path;
            console.warn(
                (forced === 'webgl2'
                    ? "[Skyglow] WebGL2 forced via ?force=webgl2.\\n"
                    : "[Skyglow] WebGPU unavailable — falling back to WebGL2 bundle.\\n") +
                "  Reduced fidelity: no Bloom, no IBL, simpler clustered lighting.\\n" +
                "  For full quality: enable WebGPU (Chrome / Edge / Brave on Windows or Mac, " +
                "Safari Tahoe, Firefox Nightly with dom.webgpu.enabled)."
            );
        } else {
            window.skyglowBackend = 'unsupported';
            skyglowLoading = false;
            skyglowLoadPromise = null;
            const msg = "WebGPU unavailable and no WebGL2 fallback bundle was built.";
            console.error("[Skyglow]", msg);
            throw new Error(msg);
        }

        try {
            const mod = await import(modulePath);
            await mod.default();
            skyglowLoaded = true;
            skyglowLoading = false;
            console.log("[Skyglow] Demo loaded successfully (" + window.skyglowBackend + ")");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Skyglow] Ignoring control flow exception (not a real error)");
                skyglowLoaded = true;
                skyglowLoading = false;
                return;
            }
            console.error("[Skyglow] Failed to load:", error);
            skyglowLoading = false;
            skyglowLoadPromise = null;
            throw error;
        }
    })();

    return skyglowLoadPromise;
}

function isSkyglowLoaded() { return skyglowLoaded; }
function isSkyglowLoading() { return skyglowLoading; }
function skyglowBackend() { return window.skyglowBackend; }

window.loadSkyglowDemo = loadSkyglowDemo;
window.isSkyglowLoaded = isSkyglowLoaded;
window.isSkyglowLoading = isSkyglowLoading;
window.skyglowBackendName = skyglowBackend;

// Backwards-compatible aliases for the legacy "Obscura" naming.
window.loadObscuraDemo = loadSkyglowDemo;
window.isObscuraLoaded = isSkyglowLoaded;
window.isObscuraLoading = isSkyglowLoading;

console.log("[Skyglow] Loader ready " +
    "(WebGPU JS: ${SKYGLOW_JS_HASH}, WASM: ${SKYGLOW_WASM_HASH}${SKYGLOW_WEBGL2_HASH_LINE})");
EOF

        if command -v md5sum &> /dev/null; then
            SKYGLOW_LOADER_HASH=$(md5sum "$DIST_DIR/skyglow-loader-temp.js" | cut -c1-16)
        else
            SKYGLOW_LOADER_HASH=$(md5 -q "$DIST_DIR/skyglow-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/skyglow-loader-temp.js" "$DIST_DIR/skyglow-loader-${SKYGLOW_LOADER_HASH}.js"
        echo "  skyglow-loader-${SKYGLOW_LOADER_HASH}.js"
    fi
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 6.85: Generate street-loader.js (if configured)
# -----------------------------------------------------------------------------
STREET_LOADER_HASH=""
if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_JS_HASH" ]]; then
    EXISTING_STREET_LOADER=$(ls "$DIST_DIR/street-loader-"*.js 2>/dev/null | head -1)
    if [[ -n "$EXISTING_STREET_LOADER" ]] && grep -q "${STREET_CRATE_NAME}-${STREET_JS_HASH}.js" "$EXISTING_STREET_LOADER" 2>/dev/null; then
        echo "[6.85/$TOTAL_STEPS] street-loader.js: unchanged, skipping"
        STREET_LOADER_HASH=$(echo "$EXISTING_STREET_LOADER" | sed 's/.*street-loader-\([^.]*\)\.js/\1/')
    else
        echo "[6.85/$TOTAL_STEPS] Generating street-loader.js..."
        rm -f "$DIST_DIR/street-loader-"*.js "$DIST_DIR/street-loader-"*.js.br

        cat > "$DIST_DIR/street-loader-temp.js" << EOF
// Street Designer loader — multi-luminaire RP-8 / EN 13201 / CJJ 45 / MLO compliance
// Auto-generated with content hashes for cache busting.

let streetModule = null;
let streetInitialized = false;
let streetLoading = false;
let streetLoadPromise = null;

async function loadStreetDesigner() {
    if (streetInitialized && streetModule) {
        // Cached — re-mount into whatever #street-root exists now.
        if (typeof streetModule.mount === "function") {
            streetModule.mount();
        }
        return;
    }
    if (streetLoading && streetLoadPromise) {
        console.log("[Street] Loading in progress, waiting...");
        return streetLoadPromise;
    }

    streetLoading = true;
    console.log("[Street] Loading street designer...");

    streetLoadPromise = (async () => {
        try {
            const mod = await import('./street/${STREET_CRATE_NAME}-${STREET_JS_HASH}.js');
            await mod.default();
            streetModule = mod;
            if (typeof mod.mount === "function") {
                mod.mount();
            }
            streetInitialized = true;
            streetLoading = false;
            console.log("[Street] Loaded successfully");
        } catch (error) {
            console.error("[Street] Failed to load:", error);
            streetLoading = false;
            streetLoadPromise = null;
            throw error;
        }
    })();

    return streetLoadPromise;
}

function isStreetDesignerLoaded() { return streetInitialized; }
function isStreetDesignerLoading() { return streetLoading; }

window.loadStreetDesigner = loadStreetDesigner;
window.isStreetDesignerLoaded = isStreetDesignerLoaded;
window.isStreetDesignerLoading = isStreetDesignerLoading;

console.log("[Street] Loader ready (JS: ${STREET_JS_HASH}, WASM: ${STREET_WASM_HASH})");
EOF

        if command -v md5sum &> /dev/null; then
            STREET_LOADER_HASH=$(md5sum "$DIST_DIR/street-loader-temp.js" | cut -c1-16)
        else
            STREET_LOADER_HASH=$(md5 -q "$DIST_DIR/street-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/street-loader-temp.js" "$DIST_DIR/street-loader-${STREET_LOADER_HASH}.js"
        echo "  street-loader-${STREET_LOADER_HASH}.js"
    fi
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 6.6: Update index.html with hashed loader filenames
# -----------------------------------------------------------------------------
# Only update if needed (check if already has correct hashes)
INDEX_NEEDS_UPDATE=false

if [[ "$BUILD_BEVY" == "true" ]] && [[ -n "$BEVY_LOADER_HASH" ]]; then
    if ! grep -q "bevy-loader-${BEVY_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi
if [[ "$BUILD_TYPST" == "true" ]] && [[ -n "$TYPST_LOADER_HASH" ]]; then
    if ! grep -q "typst-loader-${TYPST_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi
if [[ "$BUILD_GMAPS" == "true" ]] && [[ -n "$GMAPS_LOADER_HASH" ]]; then
    if ! grep -q "gmaps-loader-${GMAPS_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi
if [[ "$BUILD_TEMPLATES" == "true" ]] && [[ -n "$TEMPLATES_LOADER_HASH" ]]; then
    if ! grep -q "templates-loader-${TEMPLATES_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi
if [[ "$BUILD_SKYGLOW" == "true" ]] && [[ -n "$SKYGLOW_LOADER_HASH" ]]; then
    if ! grep -q "skyglow-loader-${SKYGLOW_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi
if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_LOADER_HASH" ]]; then
    if ! grep -q "street-loader-${STREET_LOADER_HASH}.js" "$DIST_DIR/index.html" 2>/dev/null; then
        INDEX_NEEDS_UPDATE=true
    fi
fi

if [[ "$INDEX_NEEDS_UPDATE" == "true" ]]; then
    echo "[6.6/$TOTAL_STEPS] Updating index.html with hashed loader filenames..."

    SED_INPLACE=(-i '')
    [[ "$(uname)" != "Darwin" ]] && SED_INPLACE=(-i)

    if [[ "$BUILD_BEVY" == "true" ]] && [[ -n "$BEVY_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|bevy-loader.js\"|bevy-loader-${BEVY_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|bevy-loader.js?v=[0-9]*\"|bevy-loader-${BEVY_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|bevy-loader-[a-f0-9]*.js\"|bevy-loader-${BEVY_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    if [[ "$BUILD_TYPST" == "true" ]] && [[ -n "$TYPST_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|typst-loader.js\"|typst-loader-${TYPST_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|typst-loader.js?v=[0-9]*\"|typst-loader-${TYPST_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|typst-loader-[a-f0-9]*.js\"|typst-loader-${TYPST_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    if [[ "$BUILD_GMAPS" == "true" ]] && [[ -n "$GMAPS_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|gmaps-loader.js\"|gmaps-loader-${GMAPS_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|gmaps-loader.js?v=[0-9]*\"|gmaps-loader-${GMAPS_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|gmaps-loader-[a-f0-9]*.js\"|gmaps-loader-${GMAPS_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    if [[ "$BUILD_TEMPLATES" == "true" ]] && [[ -n "$TEMPLATES_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|templates-loader.js\"|templates-loader-${TEMPLATES_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|templates-loader.js?v=[0-9]*\"|templates-loader-${TEMPLATES_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|templates-loader-[a-f0-9]*.js\"|templates-loader-${TEMPLATES_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    if [[ "$BUILD_SKYGLOW" == "true" ]] && [[ -n "$SKYGLOW_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|skyglow-loader.js\"|skyglow-loader-${SKYGLOW_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|skyglow-loader.js?v=[0-9]*\"|skyglow-loader-${SKYGLOW_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|skyglow-loader-[a-f0-9]*.js\"|skyglow-loader-${SKYGLOW_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_LOADER_HASH" ]]; then
        sed "${SED_INPLACE[@]}" "s|street-loader.js\"|street-loader-${STREET_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|street-loader.js?v=[0-9]*\"|street-loader-${STREET_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
        sed "${SED_INPLACE[@]}" "s|street-loader-[a-f0-9]*.js\"|street-loader-${STREET_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    fi

    echo "  Updated loader references"
else
    echo "[6.6/$TOTAL_STEPS] index.html: unchanged, skipping"
fi
echo ""

# -----------------------------------------------------------------------------
# Step 6.9: Copy packed GLB scenes + env maps into dist/assets/
# -----------------------------------------------------------------------------
if [[ "$BUILD_SKYGLOW" == "true" ]]; then
    mkdir -p "$DIST_DIR/assets/environment_maps"
    # Copy self-contained GLB scenes (no external texture files needed)
    cp -f "$BEVY_DIR/assets/Sponza_web.glb" "$DIST_DIR/assets/" 2>/dev/null || true
    cp -f "$BEVY_DIR/assets/BistroExterior_web.glb" "$DIST_DIR/assets/" 2>/dev/null || true
    cp -f "$BEVY_DIR/assets/BistroExteriorFakeGI.gltf" "$DIST_DIR/assets/" 2>/dev/null || true
    # Copy environment maps (required for IBL lighting)
    cp -f "$BEVY_DIR/assets/environment_maps/"*.ktx2 "$DIST_DIR/assets/environment_maps/" 2>/dev/null || true
    echo "[6.9/$TOTAL_STEPS] Copied packed GLB scenes + env maps into dist/assets/"
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 7: Pre-compress with Brotli (only files missing .br)
# -----------------------------------------------------------------------------
if [[ "$SKIP_BROTLI" == "true" ]]; then
    echo "[$((STEP++))/$TOTAL_STEPS] Skipping Brotli (servesimple mode)"
elif [[ "$HAVE_BROTLI" == "true" ]]; then
    FILES_TO_COMPRESS=()

    # Only add files that don't have an up-to-date .br version
    for f in "$DIST_DIR/"*.wasm "$DIST_DIR/bevy/"*.wasm "$DIST_DIR/typst/"*.wasm "$DIST_DIR/templates/"*.wasm "$DIST_DIR/skyglow/"*.wasm "$DIST_DIR/skyglow-webgl2/"*.wasm "$DIST_DIR/street/"*.wasm \
             "$DIST_DIR/"*.js "$DIST_DIR/bevy/"*.js "$DIST_DIR/typst/"*.js "$DIST_DIR/templates/"*.js "$DIST_DIR/skyglow/"*.js "$DIST_DIR/skyglow-webgl2/"*.js "$DIST_DIR/street/"*.js \
             "$DIST_DIR/"*.css; do
        if [[ -f "$f" ]]; then
            # Skip non-hashed loader files (we use hashed versions)
            basename_f=$(basename "$f")
            if [[ "$basename_f" == "bevy-loader.js" ]] || \
               [[ "$basename_f" == "typst-loader.js" ]] || \
               [[ "$basename_f" == "gmaps-loader.js" ]] || \
               [[ "$basename_f" == "templates-loader.js" ]] || \
               [[ "$basename_f" == "skyglow-loader.js" ]] || \
               [[ "$basename_f" == "street-loader.js" ]]; then
                continue
            fi
            # Skip if .br exists and is at least as new as source
            if [[ -f "${f}.br" ]]; then
                src_time=$(stat -f '%m' "$f" 2>/dev/null || stat -c '%Y' "$f" 2>/dev/null)
                br_time=$(stat -f '%m' "${f}.br" 2>/dev/null || stat -c '%Y' "${f}.br" 2>/dev/null)
                if [[ "$br_time" -ge "$src_time" ]]; then
                    continue
                fi
            fi
            FILES_TO_COMPRESS+=("$f")
        fi
    done

    if [[ ${#FILES_TO_COMPRESS[@]} -eq 0 ]]; then
        echo "[$((STEP++))/$TOTAL_STEPS] Brotli: all files up-to-date, skipping"
    else
        echo "[$((STEP++))/$TOTAL_STEPS] Compressing ${#FILES_TO_COMPRESS[@]} files with Brotli..."
        if command -v nproc &> /dev/null; then
            NCPU=$(nproc)
        elif command -v sysctl &> /dev/null; then
            NCPU=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)
        else
            NCPU=4
        fi
        printf '%s\n' "${FILES_TO_COMPRESS[@]}" | xargs -P "$NCPU" -I {} brotli -f -q 11 {}
        echo "  Done!"
    fi
else
    echo "[$((STEP++))/$TOTAL_STEPS] Brotli not found, skipping pre-compression."
    echo "  Install with: brew install brotli"
fi
echo ""

# -----------------------------------------------------------------------------
# Step 8: Create additional pages
# -----------------------------------------------------------------------------
echo "[$TOTAL_STEPS/$TOTAL_STEPS] Creating additional pages for static deployment..."

if [[ -n "$SECRET_PAGE" ]]; then
    cp "$DIST_DIR/index.html" "$DIST_DIR/$SECRET_PAGE"
    echo "  Created $SECRET_PAGE (enables PDF/Typst export)"
fi

if [[ -n "$CUSTOM_404_SVG" ]] && [[ -f "$ASSETS_DIR/$CUSTOM_404_SVG" ]]; then
    cat > "$DIST_DIR/404.html" << 'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            background: #070810;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            color: #c7f8ff;
            padding: 2rem;
        }
        .container { max-width: 1400px; width: 100%; text-align: center; }
        .svg-container { width: 100%; max-width: 1000px; margin: 0 auto 2rem; }
        .svg-container svg { width: 100%; height: auto; }
        .message { opacity: 0.8; margin-bottom: 2rem; }
        .home-link {
            display: inline-block;
            padding: 0.75rem 2rem;
            background: linear-gradient(135deg, #22d8ff 0%, #9ff7ff 100%);
            color: #070810;
            text-decoration: none;
            border-radius: 8px;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .home-link:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 20px rgba(34, 216, 255, 0.4);
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="svg-container">
HTMLEOF
    cat "$ASSETS_DIR/$CUSTOM_404_SVG" >> "$DIST_DIR/404.html"
    cat >> "$DIST_DIR/404.html" << 'HTMLEOF'
        </div>
        <p class="message">The page you're looking for seems to be in the dark.</p>
        <a href="./" class="home-link">← Back to Editor</a>
    </div>
</body>
</html>
HTMLEOF
    echo "  Created 404.html with custom SVG"
fi
echo ""

# -----------------------------------------------------------------------------
# Step 9: Build Quiz app and copy into dist
# -----------------------------------------------------------------------------
QUIZ_DIR="$ROOT_DIR/crates/eulumdat-wasm-quiz"
if [[ -d "$QUIZ_DIR" ]]; then
    echo "[9/$TOTAL_STEPS] Building Quiz app..."
    cd "$QUIZ_DIR"
    trunk build --release 2>&1 | tail -5

    # Trunk always outputs index.html regardless of target setting.
    # Rename it to quiz.html before copying to avoid overwriting the main app.
    if [[ -d "$QUIZ_DIR/dist" ]]; then
        # Ensure quiz.html exists (Trunk's post_build hook should rename, but be safe)
        if [[ -f "$QUIZ_DIR/dist/index.html" ]]; then
            mv "$QUIZ_DIR/dist/index.html" "$QUIZ_DIR/dist/quiz.html"
        fi
        # Copy everything except index.html (should not exist after mv, but be safe)
        for f in "$QUIZ_DIR/dist/"*; do
            [[ "$(basename "$f")" == "index.html" ]] && continue
            cp "$f" "$DIST_DIR/" 2>/dev/null || true
        done
        QUIZ_WASM=$(ls "$QUIZ_DIR/dist/"*_bg.wasm 2>/dev/null | head -1)
        QUIZ_SIZE=$(ls -lh "$QUIZ_WASM" 2>/dev/null | awk '{print $5}')
        echo "  Quiz app copied to dist/ ($QUIZ_SIZE)"

        # Brotli-compress quiz files if available
        if [[ "$HAVE_BROTLI" == "true" ]]; then
            for f in "$DIST_DIR"/eulumdat-wasm-quiz*; do
                if [[ -f "$f" ]] && [[ ! "$f" =~ \.br$ ]] && [[ ! -f "$f.br" ]]; then
                    brotli -f -q 11 "$f"
                fi
            done
            if [[ -f "$DIST_DIR/quiz.html" ]] && [[ ! -f "$DIST_DIR/quiz.html.br" ]]; then
                brotli -f -q 11 "$DIST_DIR/quiz.html"
            fi
            echo "  Quiz files Brotli-compressed"
        fi
    else
        echo "  WARNING: Quiz dist/ not found after build"
    fi
    cd "$ROOT_DIR"
else
    echo "[9/$TOTAL_STEPS] Quiz app: skipped (crate not found)"
fi
echo ""

# =============================================================================
# Summary
# =============================================================================

echo "=== Build Complete ==="
echo ""

# Show what was rebuilt vs cached
echo "Build status:"
if [[ "$BEVY_BUILT" == "true" ]]; then
    echo "  Bevy 3D viewer:     REBUILT"
elif [[ "$BUILD_BEVY" == "true" ]]; then
    echo "  Bevy 3D viewer:     cached (unchanged)"
fi
if [[ "$LEPTOS_BUILT" == "true" ]]; then
    echo "  Leptos editor:      REBUILT"
elif [[ "$BUILD_LEPTOS" == "true" ]]; then
    echo "  Leptos editor:      cached (unchanged)"
fi
echo ""

LEPTOS_WASM=$(ls "$DIST_DIR/"eulumdat-wasm-*_bg.wasm 2>/dev/null | head -1)
BEVY_WASM_FILE=$(ls "$DIST_DIR/bevy/"*_bg.wasm 2>/dev/null | head -1)
TYPST_WASM_FILE=$(ls "$DIST_DIR/typst/"*_bg.wasm 2>/dev/null | head -1)
TEMPLATES_WASM_FILE=$(ls "$DIST_DIR/templates/"*_bg.wasm 2>/dev/null | head -1)
QUIZ_WASM_FILE=$(ls "$DIST_DIR/"eulumdat-wasm-quiz-*_bg.wasm 2>/dev/null | head -1)

LEPTOS_SIZE=$(ls -lh "$LEPTOS_WASM" 2>/dev/null | awk '{print $5}')
BEVY_SIZE=$(ls -lh "$BEVY_WASM_FILE" 2>/dev/null | awk '{print $5}')
TYPST_SIZE=$(ls -lh "$TYPST_WASM_FILE" 2>/dev/null | awk '{print $5}')
TEMPLATES_SIZE=$(ls -lh "$TEMPLATES_WASM_FILE" 2>/dev/null | awk '{print $5}')
QUIZ_SIZE=$(ls -lh "$QUIZ_WASM_FILE" 2>/dev/null | awk '{print $5}')

echo "Bundle sizes (raw / compressed):"
if [[ "$HAVE_BROTLI" == "true" ]]; then
    LEPTOS_BR=$(ls -lh "${LEPTOS_WASM}.br" 2>/dev/null | awk '{print $5}')
    BEVY_BR=$(ls -lh "${BEVY_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')
    TYPST_BR=$(ls -lh "${TYPST_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')
    TEMPLATES_BR=$(ls -lh "${TEMPLATES_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')

    [[ -n "$LEPTOS_SIZE" ]] && echo "  Leptos editor:      $LEPTOS_SIZE -> $LEPTOS_BR (loads immediately)"
    [[ -n "$BEVY_SIZE" ]] && echo "  Bevy 3D viewer:     $BEVY_SIZE -> $BEVY_BR (loads on demand)"
    [[ -n "$TYPST_SIZE" ]] && echo "  Typst PDF compiler: $TYPST_SIZE -> $TYPST_BR (loads on demand)"
    [[ -n "$TEMPLATES_SIZE" ]] && echo "  Templates:          $TEMPLATES_SIZE -> $TEMPLATES_BR (loads on demand)"
    if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_WASM_HASH" ]]; then
        STREET_WASM_FILE_DIST="$DIST_DIR/street/${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm"
        STREET_SIZE=$(ls -lh "$STREET_WASM_FILE_DIST" 2>/dev/null | awk '{print $5}')
        STREET_BR=$(ls -lh "${STREET_WASM_FILE_DIST}.br" 2>/dev/null | awk '{print $5}')
        [[ -n "$STREET_SIZE" ]] && echo "  Street Designer:    $STREET_SIZE -> $STREET_BR (loads on demand)"
    fi
    if [[ -n "$QUIZ_SIZE" ]]; then
        QUIZ_BR=$(ls -lh "${QUIZ_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')
        echo "  Quiz app:           $QUIZ_SIZE -> $QUIZ_BR (quiz.html)"
    fi
else
    [[ -n "$LEPTOS_SIZE" ]] && echo "  Leptos editor:      $LEPTOS_SIZE (loads immediately)"
    [[ -n "$BEVY_SIZE" ]] && echo "  Bevy 3D viewer:     $BEVY_SIZE (loads on demand)"
    [[ -n "$TYPST_SIZE" ]] && echo "  Typst PDF compiler: $TYPST_SIZE (loads on demand)"
    [[ -n "$TEMPLATES_SIZE" ]] && echo "  Templates:          $TEMPLATES_SIZE (loads on demand)"
    if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_WASM_HASH" ]]; then
        STREET_WASM_FILE_DIST="$DIST_DIR/street/${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm"
        STREET_SIZE_U=$(ls -lh "$STREET_WASM_FILE_DIST" 2>/dev/null | awk '{print $5}')
        [[ -n "$STREET_SIZE_U" ]] && echo "  Street Designer:    $STREET_SIZE_U (loads on demand)"
    fi
    [[ -n "$QUIZ_SIZE" ]] && echo "  Quiz app:           $QUIZ_SIZE (quiz.html)"
fi
echo ""

if [[ "$BUILD_BEVY" == "true" ]]; then
    echo "Hashed filenames:"
    echo "  Bevy:      ${BEVY_BINARY}-${JS_HASH}.js / ${BEVY_BINARY}-${WASM_HASH}_bg.wasm"
fi
if [[ "$BUILD_TYPST" == "true" ]] && [[ -n "$TYPST_JS_HASH" ]]; then
    echo "  Typst:     typst_wasm-${TYPST_JS_HASH}.js / typst_wasm-${TYPST_WASM_HASH}_bg.wasm"
fi
if [[ "$BUILD_TEMPLATES" == "true" ]] && [[ -n "$TEMPLATES_JS_HASH" ]]; then
    echo "  Templates: eulumdat_wasm_templates-${TEMPLATES_JS_HASH}.js / eulumdat_wasm_templates-${TEMPLATES_WASM_HASH}_bg.wasm"
fi
if [[ "$BUILD_STREET" == "true" ]] && [[ -n "$STREET_JS_HASH" ]]; then
    echo "  Street:    ${STREET_CRATE_NAME}-${STREET_JS_HASH}.js / ${STREET_CRATE_NAME}-${STREET_WASM_HASH}_bg.wasm"
fi
echo ""
echo "Output: $DIST_DIR"
echo ""

# =============================================================================
# Deploy / Serve
# =============================================================================

if [[ "$ACTION" == "deploy" ]]; then
    echo ""
    if [[ -z "$DEPLOY_TARGET" ]]; then
        echo "ERROR: No deploy target configured in build-config.toml"
        echo "Add [deploy] section with target = \"user@host:/path/\""
        exit 1
    fi

    echo "=== Deploying to $DEPLOY_TARGET ==="
    echo "Running: rsync $RSYNC_FLAGS $DIST_DIR/ $DEPLOY_TARGET"
    echo ""
    rsync $RSYNC_FLAGS "$DIST_DIR/" "$DEPLOY_TARGET"
    echo ""
    echo "Deploy complete!"

elif [[ "$ACTION" == "serve" ]]; then
    echo ""
    echo "=== Starting local server on port $LOCAL_PORT ==="
    echo ""

    if [[ -n "$SERVER_CRATE" ]]; then
        echo "Running: cargo run -p $SERVER_CRATE -- -p $LOCAL_PORT --dist $DIST_DIR"
        echo "Open: http://localhost:$LOCAL_PORT"
        echo ""
        cargo run -p "$SERVER_CRATE" -- -p "$LOCAL_PORT" --dist "$DIST_DIR"
    else
        echo "Running: python3 -m http.server $LOCAL_PORT -d $DIST_DIR"
        echo "Open: http://localhost:$LOCAL_PORT"
        echo ""
        python3 -m http.server "$LOCAL_PORT" -d "$DIST_DIR"
    fi

elif [[ "$ACTION" == "servesimple" ]]; then
    echo ""
    echo "=== Starting simple local server on port $LOCAL_PORT (no Brotli) ==="
    echo ""
    echo "Running: python3 -m http.server $LOCAL_PORT -d $DIST_DIR"
    echo "Open: http://localhost:$LOCAL_PORT"
    echo ""
    python3 -m http.server "$LOCAL_PORT" -d "$DIST_DIR"

else
    # Just show instructions
    if [[ -n "$DEPLOY_TARGET" ]]; then
        echo "To deploy to $DEPLOY_TARGET:"
        echo "  $0 deploy"
        echo ""
    fi
    echo "To serve locally:"
    if [[ -n "$SERVER_CRATE" ]]; then
        echo "  $0 serve"
        echo "  # or: cargo run -p $SERVER_CRATE -- -p $LOCAL_PORT --dist $DIST_DIR"
    else
        echo "  $0 serve"
        echo "  # or: python3 -m http.server $LOCAL_PORT -d $DIST_DIR"
    fi
    echo "  open http://localhost:$LOCAL_PORT"
fi
