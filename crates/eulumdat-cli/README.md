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

## Commands

| Command | Description |
|---------|-------------|
| `info` | Display luminaire information |
| `validate` | Validate photometric data |
| `convert` | Convert between LDT and IES |
| `diagram` | Generate SVG diagrams |
| `bug` | Calculate BUG rating |

## License

MIT OR Apache-2.0
