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

## Photometric Calculations (v0.3.0+)

```python
# Complete photometric summary
summary = ldt.photometric_summary()
print(summary.to_text())      # Full multi-line report
print(summary.to_compact())   # One-line summary
data = summary.to_dict()      # For JSON serialization

# CIE flux codes (N1-N5)
cie = ldt.cie_flux_codes()
print(f"CIE Flux Code: {cie}")  # e.g., "100 77 43 0 0"
print(f"N1 (DLOR): {cie.n1}%")
print(f"N4 (ULOR): {cie.n4}%")

# Beam characteristics
print(f"Beam angle (50%): {ldt.beam_angle():.1f}°")
print(f"Field angle (10%): {ldt.field_angle():.1f}°")
print(f"Cut-off angle: {ldt.cut_off_angle():.1f}°")

# Spacing criteria (S/H ratios)
s_c0, s_c90 = ldt.spacing_criteria()
print(f"S/H ratio: {s_c0:.2f} (C0), {s_c90:.2f} (C90)")

# Zonal lumens (30° zones)
zones = ldt.zonal_lumens_30()
print(f"Downward: {zones.downward_total():.1f}%")
print(f"Upward: {zones.upward_total():.1f}%")

# Photometric classification code
print(f"Classification: {ldt.photometric_code()}")  # e.g., "D-M"

# Downward flux to specific angle
flux_60 = ldt.downward_flux(60.0)  # % within 60° of nadir
```

## GLDF Export (v0.3.0+)

```python
# Get GLDF-compatible photometric data
gldf = ldt.gldf_data()
print(gldf.cie_flux_code)
print(gldf.light_output_ratio)
print(gldf.bug_rating)

# Export as dictionary for JSON
gldf_dict = gldf.to_dict()
import json
json.dumps(gldf_dict)
```

## UGR Calculation (v0.3.0+)

```python
# Standard office room
params = eulumdat.UgrParams.standard_office()
ugr = ldt.calculate_ugr(params)
print(f"UGR: {ugr:.1f}")

# Custom room configuration
params = eulumdat.UgrParams(
    room_length=10.0,
    room_width=6.0,
    mounting_height=3.0,
    eye_height=1.2,
    observer_x=5.0,
    observer_y=3.0
)
params.add_luminaire(2.5, 2.0)
params.add_luminaire(7.5, 2.0)
params.add_luminaire(2.5, 4.0)
params.add_luminaire(7.5, 4.0)
ugr = ldt.calculate_ugr(params)
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

# BUG diagram with detailed zone lumens table
detailed_bug = ldt.bug_svg_with_details(width=600, height=400)
```

## License

MIT OR Apache-2.0
