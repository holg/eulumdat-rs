#!/bin/bash
#
# Clean Rebuild Script
# Performs a complete clean rebuild of the Rust library and XCFramework
#

set -e  # Exit on error

echo "=== Clean Rebuild Script ==="
echo ""

# Get the script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# 1. Clean Rust build artifacts
echo "Step 1: Cleaning Rust build artifacts..."
cargo clean
echo "✓ Rust build cleaned"
echo ""

# 2. Clean XCFramework
echo "Step 2: Removing XCFramework..."
if [ -d "swift/eulumdat_ffiFFI.xcframework" ]; then
    rm -rf swift/eulumdat_ffiFFI.xcframework
    echo "✓ XCFramework removed"
else
    echo "✓ XCFramework already clean"
fi
echo ""

# 3. Clean Swift build artifacts
echo "Step 3: Cleaning Swift build artifacts..."
if [ -d "EulumdatApp/.build" ]; then
    rm -rf EulumdatApp/.build
    echo "✓ Swift build cleaned"
else
    echo "✓ Swift build already clean"
fi
echo ""

# 4. Rebuild XCFramework
echo "Step 4: Rebuilding XCFramework..."
./scripts/build-xcframework.sh
echo ""

# 5. Build Swift app
echo "Step 5: Building Swift app..."
cd EulumdatApp
swift build
cd ..
echo ""

echo "=== Clean Rebuild Complete! ==="
echo ""
echo "✓ All artifacts cleaned"
echo "✓ Rust library rebuilt"
echo "✓ XCFramework regenerated"
echo "✓ Swift app built"
echo ""
echo "You can now run the app from Xcode or with: cd EulumdatApp && .build/debug/EulumdatApp"
