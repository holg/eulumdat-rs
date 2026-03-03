#!/bin/bash
# Benchmark comparison script: Native vs WASM
#
# Usage: ./scripts/benchmark-compare.sh

set -e

echo "=== Eulumdat Benchmark: Native vs WASM ==="
echo ""

# Run native benchmark
echo "Running native benchmark..."
NATIVE_JSON=$(cargo bench -p eulumdat --bench native_benchmark 2>/dev/null | grep '{"platform":"native"')

echo ""
echo "Native results:"
echo "$NATIVE_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(f'  Parse LDT:         {data[\"parse_us\"]:>8.2f} µs')
print(f'  Polar diagram:     {data[\"polar_us\"]:>8.2f} µs')
print(f'  Cartesian diagram: {data[\"cartesian_us\"]:>8.2f} µs')
print(f'  IES export:        {data[\"ies_export_us\"]:>8.2f} µs')
print(f'  LDT export:        {data[\"ldt_export_us\"]:>8.2f} µs')
"

echo ""
echo "To run WASM benchmark, open eulumdat.icu in browser and run in console:"
echo ""
echo "  // Run WASM benchmark only:"
echo "  window.runBenchmark()"
echo ""
echo "  // Compare with native results:"
echo "  window.compareBenchmark('$NATIVE_JSON')"
echo ""
echo "The native JSON to paste:"
echo "$NATIVE_JSON"
