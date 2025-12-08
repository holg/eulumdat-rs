#!/bin/bash
#
# Build macOS and iOS archives for App Store submission
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"

echo "=== Eulumdat App Archive Builder ==="
echo ""

# Clean build directory
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

cd "$PROJECT_DIR"

# Build macOS archive
echo "Building macOS archive..."
xcodebuild \
    -project EulumdatApp.xcodeproj \
    -scheme EulumdatApp \
    -configuration Release \
    -destination 'generic/platform=macOS' \
    -archivePath "$BUILD_DIR/Eulumdat-macOS.xcarchive" \
    archive \
    2>&1 | grep -E "(error:|warning:|ARCHIVE|BUILD)" || true

if [ -d "$BUILD_DIR/Eulumdat-macOS.xcarchive" ]; then
    echo "✓ macOS archive created: $BUILD_DIR/Eulumdat-macOS.xcarchive"
else
    echo "✗ macOS archive failed"
    exit 1
fi

echo ""

# Build iOS archive
echo "Building iOS archive..."
xcodebuild \
    -project EulumdatApp.xcodeproj \
    -scheme EulumdatApp \
    -configuration Release \
    -destination 'generic/platform=iOS' \
    -archivePath "$BUILD_DIR/Eulumdat-iOS.xcarchive" \
    archive \
    2>&1 | grep -E "(error:|warning:|ARCHIVE|BUILD)" || true

if [ -d "$BUILD_DIR/Eulumdat-iOS.xcarchive" ]; then
    echo "✓ iOS archive created: $BUILD_DIR/Eulumdat-iOS.xcarchive"
else
    echo "✗ iOS archive failed"
    exit 1
fi

echo ""
echo "=== Archives Ready ==="
echo ""
echo "macOS: $BUILD_DIR/Eulumdat-macOS.xcarchive"
echo "iOS:   $BUILD_DIR/Eulumdat-iOS.xcarchive"
echo ""
echo "To upload to App Store Connect:"
echo "  1. Open archives in Xcode Organizer:"
echo "     open $BUILD_DIR/Eulumdat-macOS.xcarchive $BUILD_DIR/Eulumdat-iOS.xcarchive"
echo "  2. Select each archive and click 'Distribute App'"
echo "  3. Choose 'App Store Connect' -> 'Upload'"
echo ""

# Ask to open in Xcode
read -p "Open archives in Xcode Organizer now? [y/N] " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    open "$BUILD_DIR/Eulumdat-macOS.xcarchive" "$BUILD_DIR/Eulumdat-iOS.xcarchive"
fi
