# eulumdat

A Rust library for parsing, writing, and validating Eulumdat (LDT) and IES photometric files.

## Features

- **Parse LDT files** - Full support for Eulumdat format with European number format (comma decimal separator)
- **Write LDT files** - Generate valid Eulumdat files
- **Export to IES** - Convert to IESNA LM-63-2002 format
- **Comprehensive validation** - 44 validation checks based on the official specification
- **Symmetry handling** - Automatic data reduction/expansion for 5 symmetry types
- **Photometric calculations** - Downward flux, beam angle, utilization factors

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
eulumdat = "0.1"
```

## Usage

### Parsing LDT files

```rust
use eulumdat::Eulumdat;

// From file
let ldt = Eulumdat::from_file("luminaire.ldt")?;

// From string
let ldt = Eulumdat::parse(content)?;

// Access data
println!("Luminaire: {}", ldt.luminaire_name);
println!("Symmetry: {:?}", ldt.symmetry);
println!("Total flux: {} lm", ldt.total_luminous_flux());
```

### Validation

```rust
use eulumdat::Eulumdat;

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Get warnings
let warnings = ldt.validate();
for warning in &warnings {
    println!("Warning: {}", warning);
}

// Strict validation (returns error if critical issues found)
ldt.validate_strict()?;
```

### Writing LDT files

```rust
use eulumdat::Eulumdat;

let ldt = Eulumdat::from_file("input.ldt")?;
ldt.save("output.ldt")?;

// Or get as string
let content = ldt.to_ldt();
```

### Export to IES

```rust
use eulumdat::{Eulumdat, IesExporter};

let ldt = Eulumdat::from_file("luminaire.ldt")?;
let ies = IesExporter::export(&ldt);
std::fs::write("luminaire.ies", ies)?;
```

### Photometric calculations

```rust
use eulumdat::{Eulumdat, PhotometricCalculations};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Beam and field angles
let beam = PhotometricCalculations::beam_angle(&ldt);
let field = PhotometricCalculations::field_angle(&ldt);

// Downward flux fraction at 90° (hemisphere)
let dff = PhotometricCalculations::downward_flux(&ldt, 90.0);

// Utilization factors
let ratios = PhotometricCalculations::calculate_direct_ratios(&ldt, "1.00");
```

### Symmetry handling

```rust
use eulumdat::{Eulumdat, SymmetryHandler};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Get intensity at any angle (handles symmetry automatically)
let intensity = SymmetryHandler::get_intensity_at(&ldt, 45.0, 30.0);

// Expand symmetric data to full distribution
let full_data = SymmetryHandler::expand_to_full(&ldt);

// Generate polar diagram points
let points = SymmetryHandler::generate_polar_points(&ldt, 0);
```

## Symmetry Types

| Type | Description | Data Reduction |
|------|-------------|----------------|
| 0 | No symmetry | None (full 360°) |
| 1 | Vertical axis | 360x (1 C-plane) |
| 2 | C0-C180 plane | 2x |
| 3 | C90-C270 plane | 2x |
| 4 | Both planes | 4x |

## Validation Codes

The library validates against 44 constraints. Warning codes:

- `W001-W006`: Type, symmetry, grid dimension validation
- `W007-W011`: String field length validation
- `W012-W018`: Physical dimension validation
- `W019-W022`: Optical property validation
- `W023-W030`: Lamp set validation
- `W031`: Direct ratio validation
- `W032-W036`: Angle validation
- `W037-W039`: Symmetry-specific plane requirements
- `W040-W044`: Intensity data validation

## Optional Features

- `serde` - Enable serialization/deserialization support

```toml
[dependencies]
eulumdat = { version = "0.1", features = ["serde"] }
```

## License

MIT OR Apache-2.0

## Credits

Ported from [QLumEdit](https://github.com/kstrug/QLumEdit) by Krzysztof Strugiński.
