# ATLA Schema Support

The `atla` crate supports multiple schema versions for luminaire optical data:

| Schema | Root Element | Version | Status |
|--------|--------------|---------|--------|
| **ATLA S001 / TM-33-18** | `<LuminaireOpticalData version="1.0">` | 1.0 | Full support |
| **TM-33-23 (IESTM33-22)** | `<IESTM33-22><Version>1.1</Version>` | 1.1 | Full support |
| **UNI 11733** | `<LuminaireOpticalData>` | 1.0 | Full support (same as S001) |

## Schema Detection

The library automatically detects the schema version from XML content:

```rust
use atla::{detect_schema_version, SchemaVersion};

let xml = r#"<IESTM33-22><Version>1.1</Version></IESTM33-22>"#;
assert_eq!(detect_schema_version(xml), SchemaVersion::Tm3323);

let xml2 = r#"<LuminaireOpticalData version="1.0"></LuminaireOpticalData>"#;
assert_eq!(detect_schema_version(xml2), SchemaVersion::AtlaS001);
```

When parsing, the correct parser is automatically selected:

```rust
use atla;

// Auto-detects format and parses accordingly
let doc = atla::parse(&xml_content)?;
println!("Schema version: {:?}", doc.schema_version);
```

## Schema Differences

### Required Fields

| Field | ATLA S001 | TM-33-23 |
|-------|-----------|----------|
| `Header.Description` | Optional | **Required** |
| `Header.Laboratory` | Optional | **Required** |
| `Header.ReportNumber` | Optional | **Required** |
| `Header.ReportDate` | Optional | **Required** (xs:date format) |
| `Emitter.Description` | Optional | **Required** |
| `Emitter.InputWattage` | Optional | **Required** |
| `CustomData.Name` | N/A | **Required** |
| `CustomData.UniqueIdentifier` | N/A | **Required** |

### Element Name Changes

| Concept | ATLA S001 | TM-33-23 |
|---------|-----------|----------|
| Power input | `InputWatts` | `InputWattage` |
| Goniometer | `Goniometer` | `Gonioradiometer` |
| Goniometer type | `GoniometerType` | `Type` (in Gonioradiometer) |
| Intensity data | `IntensityDistribution` | `LuminousData` |
| Color temperature | `CCT` | `FixedCCT` |

### TM-33-23 Specific Features

- **SymmetryType** (`SymmType`): `Symm _ None`, `Symm _ Bi0`, `Symm _ Bi90`, `Symm _ Quad`, `Symm _ Full`, `Symm _ Arbitrary`
- **Multiplier**: Scaling factor for intensity values
- **AbsolutePhotometry**: Boolean flag for absolute vs relative data
- **AngularSpectralData**: 4D spectral intensity (h, v, wavelength, value)
- **AngularColorData**: CIE x,y chromaticity per angle
- **Multiple CustomData**: Multiple custom data blocks with Name and UniqueIdentifier

## Validation

### Auto-Detection Mode

```rust
use atla::validate;

let result = validate::validate(&doc);  // Uses doc.schema_version
```

### Explicit Schema Validation

```rust
use atla::validate::{validate_with_schema, ValidationSchema};

// Force validation against TM-33-23 rules
let result = validate_with_schema(&doc, ValidationSchema::Tm3323);

// Force validation against S001 rules
let result = validate_with_schema(&doc, ValidationSchema::AtlaS001);
```

### Validation Error Codes

#### Common (All Schemas)
- `E001`: Missing version
- `E002`: No emitters
- `E003`: Negative lumens
- `E004`: Negative watts
- `E005`: CRI out of range
- `E006-E008`: Intensity array issues
- `W001`: Negative dimensions
- `W002`: Zero quantity
- `W003`: CCT out of typical range

#### TM-33-23 Specific
- `TM33-E001`: Header.Description required
- `TM33-E002`: Header.Laboratory required
- `TM33-E003`: Header.ReportNumber required
- `TM33-E004`: Header.ReportDate required
- `TM33-E010`: Emitter.Description required
- `TM33-E011`: Emitter.InputWattage required
- `TM33-E020`: CustomData.Name required
- `TM33-E021`: CustomData.UniqueIdentifier required
- `TM33-E030`: Invalid CIE x value (AngularColorData)
- `TM33-E031`: Invalid CIE y value (AngularColorData)
- `TM33-W001`: Multiplier should be positive
- `TM33-W002`: SymmetryType inconsistent with data
- `TM33-W003`: ReportDate format warning

## Schema Conversion

### S001 to TM-33-23

```rust
use atla::convert::{atla_to_tm33, ConversionPolicy};

// Compatible mode: apply defaults for missing required fields
let (tm33_doc, log) = atla_to_tm33(&s001_doc, ConversionPolicy::Compatible)?;

// Strict mode: error if required fields are missing
let result = atla_to_tm33(&s001_doc, ConversionPolicy::Strict);
```

**Compatible Mode Defaults:**
- `Header.Description` → "Not specified"
- `Header.Laboratory` → "Not specified"
- `Header.ReportNumber` → "UNKNOWN"
- `Header.ReportDate` → Current date (YYYY-MM-DD)
- `Emitter.Description` → "Emitter"

**Note:** `Emitter.InputWattage` has no sensible default and will error even in compatible mode if missing.

### TM-33-23 to S001

```rust
use atla::convert::tm33_to_atla;

let (s001_doc, log) = tm33_to_atla(&tm33_doc);
```

**Data Loss:**
- `AngularSpectralData` → Dropped
- `AngularColorData` → Dropped
- Multiple `CustomData` → First only
- `SymmetryType` → Preserved in field but not in output format
- `Multiplier` → Applied to intensity values

### Conversion Log

Both conversion functions return a log of changes made:

```rust
for entry in &log {
    println!("{}: {:?} ({:?} -> {:?})",
        entry.field,
        entry.action,
        entry.original_value,
        entry.new_value
    );
}
```

**Actions:**
- `Preserved`: Value copied unchanged
- `DefaultApplied`: Missing value filled with default
- `Renamed`: Field name changed (e.g., InputWatts → InputWattage)
- `TypeConverted`: Value type changed (e.g., GTIN string → integer)
- `Dropped`: Data lost in conversion
- `Warning`: Non-fatal issue noted

## Writing Output

### Specify Schema Version

```rust
use atla::xml::write_with_schema;
use atla::SchemaVersion;

// Write as TM-33-23 format
let xml = write_with_schema(&doc, SchemaVersion::Tm3323, Some(2))?;

// Write as S001 format
let xml = write_with_schema(&doc, SchemaVersion::AtlaS001, Some(2))?;
```

### Compact Output

```rust
// No indentation
let xml = write_with_schema(&doc, SchemaVersion::Tm3323, None)?;
```

## CLI Usage

### Validate with Schema Selection

```bash
# Auto-detect schema
eulumdat validate-atla file.xml

# Force TM-33-23 validation
eulumdat validate-atla file.xml --schema-type tm3323

# Force S001 validation
eulumdat validate-atla file.xml --schema-type s001
```

### Convert Between Schemas

```bash
# S001 -> TM-33-23 (compatible mode)
eulumdat atla-convert input.xml output.xml --target tm3323

# TM-33-23 -> S001
eulumdat atla-convert input.xml output.xml --target s001

# Strict mode (error on missing fields)
eulumdat atla-convert input.xml output.xml --target tm3323 --policy strict

# Verbose output (show conversion log)
eulumdat atla-convert input.xml output.xml --target tm3323 --verbose
```

## Test Files

Sample TM-33-23 files are included for testing:

- `crates/atla/tests/samples/tm33-23/minimal.xml` - Minimal valid TM-33-23
- `crates/atla/tests/samples/tm33-23/with_custom_data.xml` - TM-33-23 with multiple CustomData blocks

## References

- [IES TM-33-18](https://www.ies.org/standards/standards-documents/) - Original standard
- [IES TM-33-23](https://www.ies.org/standards/standards-documents/) - Updated standard
- [ATLA S001](http://www.atlasl.org/) - ATLA equivalent
- [UNI 11733:2019](https://www.uni.com/) - Italian national standard
