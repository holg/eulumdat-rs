# Eulumdat Benchmark Results: Native vs Python vs WASM

**Date:** 2024-12-24 (Heiligabend!)
**Machine:** Apple Silicon (M1/M2)
**Test file:** `Preliminar Interlab IPT_fotometria 1.ies` (361 × 263 = 94,943 intensity values)

## Summary

| Platform | Simple Parse | Challenging Parse | Notes |
|----------|--------------|-------------------|-------|
| **Native Rust** | 12.5 µs | 3.7 ms | Baseline |
| **Python (PyO3)** | 12.7 µs | 3.9 ms | ~3% overhead |
| **WASM-WASI (wasmer)** | 13.8 µs | 4.8 ms | ~28% overhead |
| **WASM-Browser (Chrome)** | 25.4 µs | 4.3 ms | ~15-100% overhead |

## Detailed Results

### Simple LDT (24 C-planes × 19 gamma angles = 456 values)

| Operation | Native (µs) | Python (µs) | WASM-WASI (µs) | Browser (µs) |
|-----------|-------------|-------------|----------------|--------------|
| Parse LDT | 12.46 | 12.66 | 13.82 | 25.40 |
| Polar diagram | 26.18 | 27.22 | 46.64 | 48.80 |
| Cartesian diagram | 53.67 | 55.59 | 92.64 | 80.50 |
| IES export | 19.09 | 18.13 | 32.23 | 27.90 |
| LDT export | 7.80 | 8.05 | 10.83 | 9.50 |

**Ratios vs Native (Simple):**

| Operation | Python | WASM-WASI | Browser |
|-----------|--------|-----------|---------|
| Parse LDT | **1.02x** | **1.11x** | **2.04x** |
| Polar diagram | **1.04x** | **1.78x** | **1.86x** |
| Cartesian diagram | **1.04x** | **1.73x** | **1.50x** |
| IES export | **0.95x** ✨ | **1.69x** | **1.46x** |
| LDT export | **1.03x** | **1.39x** | **1.22x** |

### Challenging IES (361 C-planes × 263 gamma angles = 94,943 values)

| Operation | Native (µs) | Python (µs) | WASM-WASI (µs) | Browser (µs) |
|-----------|-------------|-------------|----------------|--------------|
| Parse IES | 3,729 | 3,856 | 4,775 | 4,295 |
| Polar diagram | 166 | 173 | 332 | 251 |
| Cartesian diagram | 373 | 389 | 773 | 674 |
| IES export | 7,624 | 7,057 | 15,090 | 11,225 |
| LDT export | 11,208 | 9,848 | 17,101 | 12,839 |

**Ratios vs Native (Challenging):**

| Operation | Python | WASM-WASI | Browser |
|-----------|--------|-----------|---------|
| Parse IES | **1.03x** | **1.28x** | **1.15x** |
| Polar diagram | **1.04x** | **2.00x** | **1.51x** |
| Cartesian diagram | **1.04x** | **2.07x** | **1.81x** |
| IES export | **0.93x** ✨ | **1.98x** | **1.47x** |
| LDT export | **0.88x** ✨ | **1.53x** | **1.15x** |

## Key Insights

### Python (PyO3) Bindings
- **Essentially zero overhead** (~3-4%)
- Some operations slightly faster in Python (measurement noise)
- PyO3's performance is remarkable
- Perfect for scripting, automation, and data science workflows

### WASM-WASI (wasmer) - Pure WASM runtime
- **1.1x - 2.1x slower** than native
- No browser overhead - pure WASM execution
- String operations (export) have ~1.5-2x overhead
- Diagram generation ~1.8-2x overhead
- This is the "true" WASM overhead

### WASM-Browser (Chrome V8)
- **1.15x - 2x slower** than native
- Sometimes **faster** than WASM-WASI for exports (V8 JIT optimizations!)
- DOM/browser overhead adds ~2x for small operations
- Large operations amortize the overhead better

### Surprising Finding: Browser vs WASI
For large export operations, **Chrome V8 is faster than wasmer**:
- IES export: Browser 11.2ms vs WASI 15.1ms (V8 wins by 25%)
- LDT export: Browser 12.8ms vs WASI 17.1ms (V8 wins by 25%)

V8's JIT compiler and optimized string handling outperforms wasmer's AOT compilation for string-heavy operations!

## Total Processing Time (Parse + All Operations)

| Platform | Simple Total | Challenging Total |
|----------|--------------|-------------------|
| Native Rust | ~120 µs | ~23 ms |
| Python (PyO3) | ~122 µs | ~21 ms |
| WASM-WASI | ~196 µs | ~38 ms |
| Browser | ~192 µs | ~29 ms |

The challenging file (95,000 values, 10,566 lines) that "makes other tools struggle" completes in **under 40ms** on all platforms.

## Running the Benchmarks

```bash
# Native Rust
cargo bench -p eulumdat --bench native_benchmark

# Python (requires maturin)
source /path/to/venv/bin/activate
maturin develop --release -m crates/eulumdat-py/Cargo.toml
python3 scripts/benchmark_python.py

# WASM-WASI (requires wasmer)
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1 -p eulumdat --example wasm_benchmark
wasmer run target/wasm32-wasip1/release/examples/wasm_benchmark.wasm

# WASM-Browser (requires playwright)
./scripts/build-wasm-split.sh
python3 -m http.server 8042 -d crates/eulumdat-wasm/dist &
python3 scripts/benchmark_wasm.py
```

## Conclusion

| Use Case | Recommended Platform | Why |
|----------|---------------------|-----|
| **CLI/Batch processing** | Native Rust | Maximum performance |
| **Python scripting** | PyO3 bindings | Zero overhead, great ergonomics |
| **Server-side WASM** | WASM-WASI | Sandboxed, portable |
| **Browser tools** | WASM-Browser | Interactive, V8 optimized |

The Rust core + multi-platform bindings architecture delivers excellent performance across all targets. Even the "slowest" platform (WASM-WASI) processes 95,000 intensity values in under 40ms.

Frohe Weihnachten! 🎄
