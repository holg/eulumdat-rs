#!/bin/bash
# Full benchmark comparison: Native, Python, and WASM
#
# Usage: ./scripts/benchmark_all.sh
#
# WASM results must be obtained manually by running in browser console:
#   window.runBenchmark()           // Simple
#   window.runBenchmarkChallenging() // Challenging

set -e

cd "$(dirname "$0")/.."

echo "============================================================"
echo "=== Eulumdat Benchmark Suite: Native vs Python vs WASM ==="
echo "============================================================"
echo ""

# Run native benchmark
echo ">>> Running Native Rust benchmark..."
echo ""
cargo bench -p eulumdat --bench native_benchmark 2>/dev/null

echo ""
echo "============================================================"
echo ""

# Run Python benchmark
echo ">>> Running Python (PyO3) benchmark..."
echo ""

# Check if module is built
if ! python3 -c "import eulumdat" 2>/dev/null; then
    echo "Building Python module..."
    maturin develop --release -m crates/eulumdat-py/Cargo.toml 2>/dev/null
fi

python3 scripts/benchmark_python.py

echo ""
echo "============================================================"
echo ""

# WASM instructions
echo ">>> WASM Benchmark Instructions"
echo ""
echo "1. Start local server (if not running):"
echo "   python3 -m http.server 8042 -d crates/eulumdat-wasm/dist"
echo ""
echo "2. Open browser: http://localhost:8042"
echo ""
echo "3. Open browser console (F12) and run:"
echo ""
echo "   // Simple benchmark:"
echo "   window.runBenchmark()"
echo ""
echo "   // Challenging benchmark:"
echo "   window.runBenchmarkChallenging()"
echo ""
echo "   // Or both:"
echo "   window.runBenchmarkFull()"
echo ""
echo "============================================================"
echo ""
echo "Copy the WASM JSON results below for comparison:"
echo ""
echo "Native Simple:      {\"platform\":\"native\",\"type\":\"simple\",...}"
echo "Native Challenging: {\"platform\":\"native\",\"type\":\"challenging\",...}"
echo "Python Simple:      {\"platform\":\"python\",\"type\":\"simple\",...}"
echo "Python Challenging: {\"platform\":\"python\",\"type\":\"challenging\",...}"
echo "WASM Simple:        (paste from browser console)"
echo "WASM Challenging:   (paste from browser console)"
echo ""
