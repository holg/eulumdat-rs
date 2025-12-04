# eulumdat

Python bindings for the [eulumdat-rs](https://github.com/holg/eulumdat-rs) Rust library.

Parse, write, and analyze **EULUMDAT (LDT)** and **IES** photometric files with high performance.

## Installation

```bash
pip install eulumdat
```

## Quick Start

```python
import eulumdat

# Parse an LDT file
ldt = eulumdat.Eulumdat.from_file("luminaire.ldt")

# Access photometric data
print(f"Luminaire: {ldt.luminaire_name}")
print(f"Symmetry: {ldt.symmetry}")
print(f"Max intensity: {ldt.max_intensity()} cd/klm")
print(f"Total flux: {ldt.total_luminous_flux()} lm")

# Validate the data
for warning in ldt.validate():
    print(f"Warning [{warning.code}]: {warning.message}")

# Generate SVG diagrams
polar_svg = ldt.polar_svg(width=500, height=500)
butterfly_svg = ldt.butterfly_svg(width=500, height=400)
cartesian_svg = ldt.cartesian_svg(width=600, height=400)
heatmap_svg = ldt.heatmap_svg(width=700, height=500)

# Calculate BUG rating (IESNA TM-15-11)
rating = ldt.bug_rating()
print(f"BUG Rating: {rating}")  # e.g., "B2 U1 G3"

# Export to IES format
ies_content = ldt.to_ies()
```

## IES Format Support

```python
# Parse IES files
ldt = eulumdat.Eulumdat.parse_ies(ies_content)
# or from file
ldt = eulumdat.Eulumdat.from_ies_file("luminaire.ies")

# Export to IES
ies_output = ldt.to_ies()
```

## Diagram Themes

```python
# Light theme (default)
svg = ldt.polar_svg(theme=eulumdat.SvgTheme.Light)

# Dark theme
svg = ldt.polar_svg(theme=eulumdat.SvgTheme.Dark)

# CSS variables for dynamic theming
svg = ldt.polar_svg(theme=eulumdat.SvgTheme.CssVariables)
```

## License

MIT OR Apache-2.0
