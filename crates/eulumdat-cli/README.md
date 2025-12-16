# eulumdat-cli

Command-line tool for working with EULUMDAT (.ldt) and IES (.ies) photometric files.

## Installation

```bash
cargo install eulumdat-cli
```

Or build from source:

```bash
cargo build --release -p eulumdat-cli
```

## Usage

### Display file information

```bash
eulumdat info luminaire.ldt
eulumdat info luminaire.ies --verbose
```

### Validate photometric files

```bash
eulumdat validate luminaire.ldt
eulumdat validate luminaire.ldt --strict
```

### Convert between formats

```bash
# LDT to IES
eulumdat convert luminaire.ldt luminaire.ies

# IES to LDT
eulumdat convert luminaire.ies luminaire.ldt
```

### Generate SVG diagrams

```bash
# Polar diagram (default)
eulumdat diagram luminaire.ldt -o polar.svg

# Butterfly diagram (3D isometric)
eulumdat diagram luminaire.ldt -t butterfly -o butterfly.svg

# Cartesian diagram (intensity vs gamma)
eulumdat diagram luminaire.ldt -t cartesian -o cartesian.svg

# Heatmap diagram
eulumdat diagram luminaire.ldt -t heatmap -o heatmap.svg

# Custom size and dark theme
eulumdat diagram luminaire.ldt -t polar -o polar.svg -W 800 -H 800 --dark
```

### Calculate BUG rating

```bash
# Display BUG rating
eulumdat bug outdoor_luminaire.ldt

# Generate BUG diagram
eulumdat bug outdoor_luminaire.ldt --svg bug.svg
```

### Photometric Summary (v0.3.0+)

```bash
# Full text report
eulumdat summary luminaire.ldt

# Compact one-liner
eulumdat summary luminaire.ldt -f compact

# JSON output
eulumdat summary luminaire.ldt -f json

# Save to file
eulumdat summary luminaire.ldt -f json -o summary.json
```

### GLDF Export (v0.3.0+)

```bash
# GLDF-compatible photometric data as JSON
eulumdat gldf luminaire.ldt

# Pretty-printed JSON
eulumdat gldf luminaire.ldt --pretty

# Save to file
eulumdat gldf luminaire.ldt --pretty -o gldf_data.json
```

### Photometric Calculations (v0.3.0+)

```bash
# CIE flux codes (N1-N5)
eulumdat calc luminaire.ldt -t cie-codes

# Beam and field angles
eulumdat calc luminaire.ldt -t beam-angles

# Spacing criteria (S/H ratios)
eulumdat calc luminaire.ldt -t spacing

# Zonal lumens distribution
eulumdat calc luminaire.ldt -t zonal-lumens

# All calculations
eulumdat calc luminaire.ldt -t all
```

## Commands

| Command | Description |
|---------|-------------|
| `info` | Display luminaire information |
| `validate` | Validate photometric data |
| `convert` | Convert between LDT and IES |
| `diagram` | Generate SVG diagrams |
| `bug` | Calculate BUG rating |
| `summary` | Display photometric summary (v0.3.0+) |
| `gldf` | Export GLDF-compatible data (v0.3.0+) |
| `calc` | Calculate specific values (v0.3.0+) |

## License

MIT OR Apache-2.0
