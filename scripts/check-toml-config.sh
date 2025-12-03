#!/usr/bin/env bash
# File: scripts/check-toml-config.sh

set -euo pipefail

echo "=== Checking TOML Configuration ==="
echo ""

# Show taplo config
echo "Current taplo.toml configuration:"
cat taplo.toml
echo ""

# Test the conflict
echo "=== Testing for conflicts ==="
echo ""

# Save current state
cp Cargo.toml Cargo.toml.backup

echo "1. Running cargo-sort..."
cargo-sort -wg
echo ""

echo "2. Checking if taplo is happy after cargo-sort..."
if taplo format --check; then
    echo "✓ No conflict - taplo is happy after cargo-sort"
    rm Cargo.toml.backup
else
    echo "✗ CONFLICT DETECTED - taplo wants to change files after cargo-sort"
    echo ""
    echo "Running taplo format to see what it wants to change..."
    taplo format
    echo ""
    echo "=== Showing differences ==="
    echo "Diff between cargo-sort output and taplo output:"
    diff -u Cargo.toml.backup Cargo.toml || true
    echo ""
    
    # Restore backup
    mv Cargo.toml.backup Cargo.toml
    
    echo "=== Solution ==="
    echo "The simplest solution is to exclude Cargo.toml from taplo formatting."
    echo "Update taplo.toml with:"
    echo ""
    echo 'exclude = ["config/**", "target/**", "**/Cargo.toml"]'
    echo ""
    exit 1
fi

echo ""
echo "✓ All checks passed - no conflicts detected!"