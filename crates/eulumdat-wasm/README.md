# Eulumdat WASM Editor

A browser-based Eulumdat (LDT/IES) photometric file editor built with Rust and WebAssembly.

## Features

- **Open/Save LDT files** - Load and save Eulumdat format files
- **Export to IES** - Convert to IESNA LM-63-2002 format
- **Drag & Drop** - Drop LDT files directly onto the editor
- **Live Validation** - Real-time validation with 44 constraint checks
- **Polar Diagram** - Interactive visualization of intensity distribution
- **Editable Data Table** - Edit intensity values directly
- **Clipboard Support** - Copy data to Excel/Calc

## Tabs

1. **General** - Identification, type indicator, symmetry, metadata
2. **Dimensions** - Physical dimensions and optical properties
3. **Lamp Sets** - Configure up to 20 lamp sets
4. **Direct Ratios** - Utilization factors for room indices

## Building

### Prerequisites

- Rust with `wasm32-unknown-unknown` target
- Trunk (`cargo install trunk`)

### Development

```bash
cd eulumdat-wasm
trunk serve
```

Open http://127.0.0.1:8080 in your browser.

### Production Build

```bash
trunk build --release
```

Output will be in the `dist/` directory.

## Technology Stack

- **Yew** - Rust/WASM web framework
- **Gloo** - Browser API bindings
- **eulumdat** - Core parsing library

## Screenshots

The editor provides:
- Multi-tab interface for editing all Eulumdat fields
- Real-time polar diagram visualization
- Validation panel showing warnings and errors
- Summary panel with calculated metrics (flux, efficacy, beam angle)

## License

MIT OR Apache-2.0
