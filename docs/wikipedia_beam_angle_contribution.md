# Proposed Contributions to Wikipedia "Beam Angle" Article

## Overview

We've developed open-source tools in **eulumdat-rs** that can generate high-quality, technically accurate SVG illustrations for the Wikipedia "Beam angle" article. These diagrams are generated programmatically from actual photometric data, ensuring accuracy and consistency.

## Current Article Illustrations

The article currently features two polar plots showing:
1. A batwing luminaire with center-beam intensity "much smaller" than maximum (94° IES vs 120° CIE)
2. A batwing luminaire with center-beam intensity "smaller" than maximum (144° IES vs 148° CIE)

These effectively illustrate the difference between IES and CIE beam angle definitions.

## Proposed Improvements

### 1. Enhanced Polar Diagrams with Beam/Field Angle Overlays

Our tool generates polar diagrams that clearly show:
- **Intensity distribution curve** (yellow fill)
- **Beam angle arc** (green, solid) - 50% of I_max
- **Field angle arc** (orange, dashed) - 10% of I_max
- **Both IES and CIE definitions** when they differ significantly
- **50% and 10% threshold circles** for visual reference

**Example files:**
- `polar_spotlight_beam_field.svg` - Standard spotlight distribution
- `polar_batwing_ies_vs_cie.svg` - Batwing showing both definitions
- `polar_wide_flood.svg` - Wide flood distribution

### 2. Cone Diagram (Beam Spread Visualization)

A side-view "electrician's diagram" showing:
- **Luminaire at top** with light source point
- **Beam angle cone** (inner, 50% threshold)
- **Field angle cone** (outer, 10% threshold)
- **Mounting height** with dimension line
- **Beam and field diameters** at floor level
- **NEMA beam classification** (Spot, Flood, etc.)
- **IES definition formula**: `diameter = 2 × height × tan(angle/2)`

**Example files:**
- `cone_diagram_standard.svg` - Basic cone diagram
- `cone_diagram_wikipedia.svg` - Enhanced version with educational annotations

### 3. Technical Accuracy

Our diagrams are generated from actual photometric calculations:

```rust
// IES definition: 50% of MAXIMUM intensity
beam_angle_ies = angle_at_percentage(ldt, 0.5);

// CIE definition: 50% of CENTER-BEAM intensity
beam_angle_cie = angle_at_percentage_of_center(ldt, 0.5);
```

The `BeamFieldAnalysis` struct provides comprehensive data:
- `beam_angle_ies` / `beam_angle_cie`
- `field_angle_ies` / `field_angle_cie`
- `max_intensity` / `center_intensity`
- `is_batwing` (detects when center < max)
- `distribution_type()` - "Standard", "Mild batwing", etc.

### 4. Interactive Web Demo

Our web editor at **https://eulumdat.rs** allows users to:
- Upload any LDT/IES photometric file
- View real-time beam/field angle calculations
- Generate SVG diagrams interactively
- See both IES and CIE definitions compared

## Benefits for Wikipedia

1. **Accuracy**: Diagrams derived from real photometric calculations, not hand-drawn
2. **Consistency**: All diagrams use the same coordinate system and color scheme
3. **Scalability**: SVG format scales perfectly at any resolution
4. **Reproducibility**: Generated from open-source code (MIT licensed)
5. **Educational value**: Clear labels explaining the 50%/10% thresholds
6. **IES vs CIE comparison**: Visually demonstrates why definitions matter

## Suggested Additions to Article

### New Section: "Beam Spread Visualization"

The cone diagram could illustrate:
- How beam angle relates to physical coverage on a surface
- The relationship: `coverage_diameter = 2 × distance × tan(angle/2)`
- Why field angle matters for practical lighting design

### Enhanced "Definitions" Section

Our polar diagrams with overlays could replace or supplement existing images:
- Clearer threshold circles showing 50% and 10% levels
- Color-coded beam (green) vs field (orange) angles
- Side-by-side IES vs CIE comparison for batwing distributions

## Technical Details

### Tool: eulumdat-rs

- **Repository**: https://github.com/your-org/eulumdat-rs
- **License**: MIT (Wikipedia-compatible)
- **Language**: Rust
- **Output**: SVG (vector graphics)

### Diagram Types Available

| Diagram | Use Case |
|---------|----------|
| `PolarDiagram` | Standard photometric polar plot |
| `PolarDiagram::to_svg_with_beam_field_angles()` | With beam/field overlays |
| `ConeDiagram` | Side-view beam spread |
| `ConeDiagram::to_svg_wikipedia()` | Enhanced educational version |
| `CartesianDiagram` | Intensity vs angle graph |

### Running the Examples

```bash
# Generate all Wikipedia SVGs
cargo run -p eulumdat --example generate_wikipedia_svgs

# Output location: screenshots/wikipedia/
```

## Contact

We'd be happy to:
1. Generate custom diagrams matching the article's current style
2. Create additional visualizations (e.g., NEMA types comparison)
3. Provide source code for diagram generation
4. Collaborate on technical accuracy review

---

*Generated with eulumdat-rs - Open-source photometric data tools*
