# atla

Rust library for parsing, writing, and converting ATLA S001 / ANSI/IES TM-33 / UNI 11733 luminaire optical data files.

## Overview

The ATLA S001 standard (also published as ANSI/IES TM-33-18 and UNI 11733:2019) is a modern XML/JSON format for luminaire optical data, designed to eventually replace the legacy IES LM-63 and EULUMDAT formats.

### Key Features

- **Spectral data support** - Full spectral power distribution (SPD)
- **Multiple intensity metrics** - Luminous, radiant, photon, and spectral
- **Data provenance** - Track whether data is measured or simulated
- **Color metrics** - CCT, CRI (Ra, R9), and TM-30 (Rf, Rg)
- **Extensible** - Custom data fields for application-specific needs
- **JSON format** - 90% smaller files compared to XML (ATLA S001-A)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
atla = "0.3"

# Enable JSON support
atla = { version = "0.3", features = ["json"] }

# Enable EULUMDAT conversion
atla = { version = "0.3", features = ["eulumdat"] }

# Enable all features
atla = { version = "0.3", features = ["xml", "json", "serde", "eulumdat"] }
```

## Usage

### Parse XML

```rust
use atla::{xml, LuminaireOpticalData};

let xml = r#"<?xml version="1.0"?>
<LuminaireOpticalData version="1.0">
    <Header>
        <Manufacturer>ACME Lighting</Manufacturer>
        <CatalogNumber>LED-100</CatalogNumber>
    </Header>
    <Emitter>
        <Quantity>1</Quantity>
        <RatedLumens>1000</RatedLumens>
        <CCT>4000</CCT>
    </Emitter>
</LuminaireOpticalData>"#;

let doc = xml::parse(xml)?;
println!("Manufacturer: {:?}", doc.header.manufacturer);
println!("Total flux: {} lm", doc.total_luminous_flux());
```

### Parse JSON

```rust
use atla::json;

let json = r#"{
    "version": "1.0",
    "header": {
        "manufacturer": "ACME Lighting",
        "catalogNumber": "LED-100"
    },
    "emitters": [{
        "quantity": 1,
        "ratedLumens": 1000,
        "cct": 4000
    }]
}"#;

let doc = json::parse(json)?;
```

### Auto-detect Format

```rust
use atla;

// Automatically detects XML or JSON based on content
let doc = atla::parse(&content)?;

// Or from file (detects from extension)
let doc = atla::parse_file("luminaire.xml")?;
```

### Convert to/from EULUMDAT

With the `eulumdat` feature enabled:

```rust
use atla::LuminaireOpticalData;
use eulumdat::Eulumdat;

// LDT -> ATLA
let ldt = Eulumdat::from_file("luminaire.ldt")?;
let atla = LuminaireOpticalData::from_eulumdat(&ldt);

// ATLA -> LDT
let ldt_back = atla.to_eulumdat();
```

## Standards

This crate implements the following equivalent standards:

| Standard | Organization | Year |
|----------|-------------|------|
| ATLA S001 | All Things Lighting Association | 2018 |
| ATLA S001-A | All Things Lighting Association | 2024 |
| ANSI/IES TM-33-18 | Illuminating Engineering Society | 2018 |
| ANSI/IES TM-33-23 | Illuminating Engineering Society | 2023 |
| UNI 11733:2019 | Ente Italiano di Normazione | 2019 |

The ATLA S001 specification is freely available from [allthingslighting.org](https://www.allthingslighting.org/lighting-standards-2/).

## License

MIT OR Apache-2.0
