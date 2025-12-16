#!/usr/bin/env bash
# =============================================================================
# Build and Test SPM Release for EulumdatKit
# =============================================================================
# This script builds the complete Swift Package Manager release:
# 1. Runs Rust tests to ensure core library works
# 2. Builds XCFramework for all Apple platforms
# 3. Runs Swift tests to verify the package works
# 4. Creates a ZIP archive for GitHub release
# 5. Generates checksum for Package.swift binary target
#
# Usage:
#   ./scripts/build-spm-release.sh           # Build and test
#   ./scripts/build-spm-release.sh --skip-rust-tests  # Skip Rust tests
#   ./scripts/build-spm-release.sh --clean   # Clean build (removes all artifacts)
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target"
SWIFT_DIR="$PROJECT_ROOT/swift"
FFI_CRATE="eulumdat-ffi"
LIB_NAME="libeulumdat_ffi"
XCFRAMEWORK_NAME="eulumdat_ffiFFI.xcframework"
GENERATED_DIR="$SWIFT_DIR/Sources/Eulumdat"

# Get version from Cargo.toml
VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
ZIP_NAME="eulumdat_ffiFFI-${VERSION}.xcframework.zip"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

# Parse arguments
SKIP_RUST_TESTS=false
CLEAN_BUILD=false
VERBOSE=false

for arg in "$@"; do
    case $arg in
        --skip-rust-tests)
            SKIP_RUST_TESTS=true
            ;;
        --clean)
            CLEAN_BUILD=true
            ;;
        --verbose|-v)
            VERBOSE=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-rust-tests  Skip running Rust tests"
            echo "  --clean            Clean build (remove all artifacts first)"
            echo "  --verbose, -v      Show verbose output"
            echo "  --help, -h         Show this help message"
            exit 0
            ;;
    esac
done

# Logging functions
log_step() { echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"; echo -e "${BOLD}${CYAN}▶ $1${NC}"; }
log_info() { echo -e "${YELLOW}  ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}  ✓${NC} $1"; }
log_error() { echo -e "${RED}  ✗${NC} $1"; }
log_detail() { if $VERBOSE; then echo -e "${MAGENTA}    →${NC} $1"; fi; }

# Timer
START_TIME=$(date +%s)
step_time() {
    local now=$(date +%s)
    local elapsed=$((now - START_TIME))
    echo -e "${MAGENTA}  ⏱${NC} Step completed in ${elapsed}s"
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking Prerequisites"

    local missing=()

    # Check Rust
    if ! command -v rustc &> /dev/null; then
        missing+=("rustc (Rust compiler)")
    else
        log_success "Rust $(rustc --version | cut -d' ' -f2)"
    fi

    # Check Cargo
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    else
        log_success "Cargo available"
    fi

    # Check rustup
    if ! command -v rustup &> /dev/null; then
        missing+=("rustup")
    else
        log_success "Rustup available"
    fi

    # Check Xcode tools
    if ! command -v xcodebuild &> /dev/null; then
        missing+=("xcodebuild (Xcode Command Line Tools)")
    else
        log_success "Xcode $(xcodebuild -version | head -1 | cut -d' ' -f2)"
    fi

    # Check Swift
    if ! command -v swift &> /dev/null; then
        missing+=("swift")
    else
        log_success "Swift $(swift --version 2>&1 | head -1 | sed 's/.*version \([0-9.]*\).*/\1/')"
    fi

    # Check lipo
    if ! command -v lipo &> /dev/null; then
        missing+=("lipo")
    else
        log_success "lipo available"
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing prerequisites:"
        for item in "${missing[@]}"; do
            echo "    - $item"
        done
        exit 1
    fi

    step_time
}

# Clean build artifacts
clean_build() {
    log_step "Cleaning Build Artifacts"

    log_info "Removing XCFramework..."
    rm -rf "$SWIFT_DIR/$XCFRAMEWORK_NAME"

    log_info "Removing universal libraries..."
    rm -rf "$BUILD_DIR/ios-sim-universal"
    rm -rf "$BUILD_DIR/macos-universal"
    rm -rf "$BUILD_DIR/headers-temp"

    log_info "Removing ZIP artifacts..."
    rm -f "$SWIFT_DIR/$ZIP_NAME"
    rm -f "$SWIFT_DIR"/*.xcframework.zip

    if [ "$CLEAN_BUILD" = true ]; then
        log_info "Removing all Rust build artifacts..."
        cargo clean
    fi

    log_success "Clean complete"
    step_time
}

# Run Rust tests
run_rust_tests() {
    if [ "$SKIP_RUST_TESTS" = true ]; then
        log_step "Skipping Rust Tests (--skip-rust-tests)"
        return
    fi

    log_step "Running Rust Tests"

    log_info "Testing core eulumdat library..."
    cargo test --package eulumdat --release
    log_success "Core library tests passed"

    log_info "Testing FFI crate..."
    cargo test --package eulumdat-ffi --release
    log_success "FFI crate tests passed"

    step_time
}

# Install Rust targets
install_rust_targets() {
    log_step "Installing Rust Targets"

    local TARGETS=(
        "aarch64-apple-ios"
        "aarch64-apple-ios-sim"
        "x86_64-apple-ios"
        "aarch64-apple-darwin"
        "x86_64-apple-darwin"
    )

    for target in "${TARGETS[@]}"; do
        log_detail "Adding target: $target"
    done

    rustup target add "${TARGETS[@]}" 2>/dev/null || true
    log_success "All Apple targets installed"

    step_time
}

# Generate Swift bindings
generate_swift_bindings() {
    log_step "Generating Swift Bindings"

    mkdir -p "$GENERATED_DIR"

    log_info "Building FFI crate for macOS (to generate bindings)..."
    cargo build --release --package "$FFI_CRATE" --target aarch64-apple-darwin

    log_info "Running UniFFI bindgen..."
    cargo run --package "$FFI_CRATE" --bin uniffi-bindgen generate \
        --library "$BUILD_DIR/aarch64-apple-darwin/release/${LIB_NAME}.dylib" \
        --language swift \
        --out-dir "$GENERATED_DIR"

    log_info "Patching Swift bindings (removing canImport check)..."
    SWIFT_FILE="$GENERATED_DIR/eulumdat_ffi.swift"
    awk '
    /#if canImport\(eulumdat_ffiFFI\)/ { skip=1; next }
    skip && /^import eulumdat_ffiFFI/ { print; skip=2; next }
    skip==2 && /^#endif/ { skip=0; next }
    { print }
    ' "$SWIFT_FILE" > "${SWIFT_FILE}.tmp" && mv "${SWIFT_FILE}.tmp" "$SWIFT_FILE"

    # Verify patch
    if grep -q '#if canImport(eulumdat_ffiFFI)' "$SWIFT_FILE"; then
        log_error "Failed to patch Swift bindings!"
        exit 1
    fi

    log_success "Swift bindings generated and patched"

    # Show generated files
    log_detail "Generated: eulumdat_ffi.swift ($(wc -l < "$GENERATED_DIR/eulumdat_ffi.swift") lines)"
    log_detail "Generated: eulumdat_ffiFFI.h ($(wc -l < "$GENERATED_DIR/eulumdat_ffiFFI.h") lines)"
    log_detail "Generated: eulumdat_ffiFFI.modulemap"

    step_time
}

# Build for all targets
build_all_targets() {
    log_step "Building for All Apple Platforms"

    local TARGETS=(
        "aarch64-apple-ios"
        "aarch64-apple-ios-sim"
        "x86_64-apple-ios"
        "aarch64-apple-darwin"
        "x86_64-apple-darwin"
    )

    local total=${#TARGETS[@]}
    local current=0

    for target in "${TARGETS[@]}"; do
        ((current++))
        log_info "[$current/$total] Building for $target..."
        cargo build --release --package "$FFI_CRATE" --target "$target"

        # Verify output
        local lib_path="$BUILD_DIR/$target/release/${LIB_NAME}.a"
        if [ ! -f "$lib_path" ]; then
            log_error "Build failed: $lib_path not found"
            exit 1
        fi
        log_detail "Built: $(du -h "$lib_path" | cut -f1)"
    done

    log_success "All targets built successfully"
    step_time
}

# Create fat libraries
create_fat_libraries() {
    log_step "Creating Universal (Fat) Libraries"

    # iOS Simulator universal (arm64 + x86_64)
    log_info "Creating iOS Simulator universal library..."
    mkdir -p "$BUILD_DIR/ios-sim-universal"
    lipo -create \
        "$BUILD_DIR/aarch64-apple-ios-sim/release/${LIB_NAME}.a" \
        "$BUILD_DIR/x86_64-apple-ios/release/${LIB_NAME}.a" \
        -output "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a"
    log_detail "iOS Simulator: $(du -h "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a" | cut -f1)"

    # macOS universal (arm64 + x86_64)
    log_info "Creating macOS universal library..."
    mkdir -p "$BUILD_DIR/macos-universal"
    lipo -create \
        "$BUILD_DIR/aarch64-apple-darwin/release/${LIB_NAME}.a" \
        "$BUILD_DIR/x86_64-apple-darwin/release/${LIB_NAME}.a" \
        -output "$BUILD_DIR/macos-universal/${LIB_NAME}.a"
    log_detail "macOS: $(du -h "$BUILD_DIR/macos-universal/${LIB_NAME}.a" | cut -f1)"

    # Verify architectures
    log_info "Verifying architectures..."
    log_detail "iOS Simulator archs: $(lipo -archs "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a")"
    log_detail "macOS archs: $(lipo -archs "$BUILD_DIR/macos-universal/${LIB_NAME}.a")"

    log_success "Universal libraries created"
    step_time
}

# Create XCFramework
create_xcframework() {
    log_step "Creating XCFramework"

    rm -rf "$SWIFT_DIR/$XCFRAMEWORK_NAME"

    # Create temporary headers directory
    local HEADERS_TEMP="$BUILD_DIR/headers-temp"
    rm -rf "$HEADERS_TEMP"
    mkdir -p "$HEADERS_TEMP"
    cp "$GENERATED_DIR/eulumdat_ffiFFI.h" "$HEADERS_TEMP/"
    cp "$GENERATED_DIR/eulumdat_ffiFFI.modulemap" "$HEADERS_TEMP/module.modulemap"

    log_info "Bundling XCFramework with 3 slices..."
    xcodebuild -create-xcframework \
        -library "$BUILD_DIR/aarch64-apple-ios/release/${LIB_NAME}.a" \
        -headers "$HEADERS_TEMP" \
        -library "$BUILD_DIR/ios-sim-universal/${LIB_NAME}.a" \
        -headers "$HEADERS_TEMP" \
        -library "$BUILD_DIR/macos-universal/${LIB_NAME}.a" \
        -headers "$HEADERS_TEMP" \
        -output "$SWIFT_DIR/$XCFRAMEWORK_NAME"

    rm -rf "$HEADERS_TEMP"

    # Verify XCFramework structure
    log_info "XCFramework structure:"
    for slice in "$SWIFT_DIR/$XCFRAMEWORK_NAME"/*/; do
        local slice_name=$(basename "$slice")
        if [ "$slice_name" != "Info.plist" ]; then
            local lib_size=$(du -h "$slice/${LIB_NAME}.a" 2>/dev/null | cut -f1 || echo "N/A")
            echo -e "    ${CYAN}$slice_name${NC} ($lib_size)"
        fi
    done

    log_success "XCFramework created at swift/$XCFRAMEWORK_NAME"
    step_time
}

# Run Swift tests
run_swift_tests() {
    log_step "Running Swift Tests"

    cd "$SWIFT_DIR"

    log_info "Building Swift package..."
    swift build
    log_success "Swift package builds successfully"

    log_info "Running tests..."
    swift test
    log_success "All Swift tests passed"

    cd "$PROJECT_ROOT"
    step_time
}

# Create release ZIP
create_release_zip() {
    log_step "Creating Release ZIP"

    cd "$SWIFT_DIR"

    # Remove old ZIPs
    rm -f *.xcframework.zip

    log_info "Compressing XCFramework..."
    zip -r -q "$ZIP_NAME" "$XCFRAMEWORK_NAME"

    local zip_size=$(du -h "$ZIP_NAME" | cut -f1)
    log_success "Created: $ZIP_NAME ($zip_size)"

    # Generate checksum
    log_info "Generating checksum..."
    local checksum=$(swift package compute-checksum "$ZIP_NAME")
    echo "$checksum" > "${ZIP_NAME}.sha256"
    log_success "Checksum: $checksum"

    cd "$PROJECT_ROOT"
    step_time
}

# Print summary
print_summary() {
    local END_TIME=$(date +%s)
    local TOTAL_TIME=$((END_TIME - START_TIME))

    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${GREEN}✓ SPM Release Build Complete!${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "  ${BOLD}Version:${NC}     $VERSION"
    echo -e "  ${BOLD}Total Time:${NC}  ${TOTAL_TIME}s"
    echo ""
    echo -e "  ${BOLD}Artifacts:${NC}"
    echo -e "    • swift/$XCFRAMEWORK_NAME"
    echo -e "    • swift/$ZIP_NAME"
    echo -e "    • swift/${ZIP_NAME}.sha256"
    echo ""
    echo -e "  ${BOLD}XCFramework Slices:${NC}"
    echo -e "    • ios-arm64 (iPhone/iPad devices)"
    echo -e "    • ios-arm64_x86_64-simulator (Simulator)"
    echo -e "    • macos-arm64_x86_64 (macOS Universal)"
    echo ""
    echo -e "  ${BOLD}Next Steps:${NC}"
    echo -e "    1. Create git tag: ${CYAN}git tag v$VERSION${NC}"
    echo -e "    2. Push tag: ${CYAN}git push origin v$VERSION${NC}"
    echo -e "    3. Upload ${CYAN}swift/$ZIP_NAME${NC} to GitHub Release"
    echo ""
    echo -e "  ${BOLD}For binary distribution, add to Package.swift:${NC}"
    echo -e "    ${CYAN}.binaryTarget("
    echo -e "        name: \"eulumdat_ffiFFI\","
    echo -e "        url: \"https://github.com/holg/eulumdat-rs/releases/download/v$VERSION/$ZIP_NAME\","
    echo -e "        checksum: \"$(cat "$SWIFT_DIR/${ZIP_NAME}.sha256" 2>/dev/null || echo '<checksum>')\""
    echo -e "    )${NC}"
    echo ""
}

# Main execution
main() {
    echo -e "${BOLD}${YELLOW}"
    echo "╔═══════════════════════════════════════════════════════════════════════════╗"
    echo "║           EulumdatKit SPM Release Builder v$VERSION                        ║"
    echo "╚═══════════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    check_prerequisites

    if [ "$CLEAN_BUILD" = true ]; then
        clean_build
    fi

    run_rust_tests
    install_rust_targets
    generate_swift_bindings
    build_all_targets
    create_fat_libraries
    create_xcframework
    run_swift_tests
    create_release_zip
    print_summary
}

# Run main
main
