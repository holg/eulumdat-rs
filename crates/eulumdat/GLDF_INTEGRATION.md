# GLDF Integration Guide - Photometric Data from Eulumdat

This document explains how to extract GLDF-compatible photometric properties from LDT/IES files using the `eulumdat` crate.

## Quick Start

```rust
use eulumdat::{Eulumdat, GldfPhotometricData};

// Parse LDT file
let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Get all GLDF-compatible photometric data
let gldf_data = GldfPhotometricData::from_eulumdat(&ldt);

// Export as key-value pairs for XML generation
let properties = gldf_data.to_gldf_properties();
for (key, value) in properties {
    println!("{}: {}", key, value);
}
```

## Available Types

### `GldfPhotometricData`

Primary struct for GLDF export. Contains all photometric properties defined in the GLDF specification.

```rust
pub struct GldfPhotometricData {
    pub cie_flux_code: String,              // e.g., "92 68 42 8 3"
    pub light_output_ratio: f64,            // Total LOR (%)
    pub luminous_efficacy: f64,             // Luminaire efficacy (lm/W)
    pub downward_flux_fraction: f64,        // DFF (%)
    pub downward_light_output_ratio: f64,   // DLOR (%)
    pub upward_light_output_ratio: f64,     // ULOR (%)
    pub luminaire_luminance: f64,           // At 65deg viewing angle (cd/m2)
    pub cut_off_angle: f64,                 // Where intensity < 2.5% (degrees)
    pub ugr_4h_8h_705020: Option<UgrTableValues>, // UGR for standard room
    pub photometric_code: String,           // e.g., "D-M" (Direct-Medium)
    pub tenth_peak_divergence: (f64, f64),  // Field angles C0/C90 (degrees)
    pub half_peak_divergence: (f64, f64),   // Beam angles C0/C90 (degrees)
    pub light_distribution_bug_rating: String, // e.g., "B2-U0-G1"
}
```

### `PhotometricSummary`

Extended summary with additional calculated values useful for reports.

```rust
pub struct PhotometricSummary {
    // Flux and efficiency
    pub total_lamp_flux: f64,      // Total rated lamp flux (lm)
    pub calculated_flux: f64,       // Flux from intensity integration (lm)
    pub lor: f64,                   // Light Output Ratio (%)
    pub dlor: f64,                  // Downward LOR (%)
    pub ulor: f64,                  // Upward LOR (%)

    // Efficacy
    pub lamp_efficacy: f64,         // Lamp efficacy (lm/W)
    pub luminaire_efficacy: f64,    // System efficacy (lm/W)
    pub total_wattage: f64,         // Total power (W)

    // CIE Flux Codes
    pub cie_flux_codes: CieFluxCodes, // N1-N5 values

    // Beam characteristics
    pub beam_angle: f64,            // 50% intensity angle (degrees)
    pub field_angle: f64,           // 10% intensity angle (degrees)

    // Intensity statistics
    pub max_intensity: f64,         // Peak intensity (cd/klm)
    pub min_intensity: f64,         // Minimum intensity (cd/klm)
    pub avg_intensity: f64,         // Average intensity (cd/klm)

    // Spacing criterion
    pub spacing_c0: f64,            // S/H ratio for C0 plane
    pub spacing_c90: f64,           // S/H ratio for C90 plane

    // Zonal lumens
    pub zonal_lumens: ZonalLumens30, // Flux in 30deg zones
}
```

## GLDF Property Mapping

The following table shows how `GldfPhotometricData` properties map to GLDF XML elements:

| GLDF Property | `GldfPhotometricData` Field | Unit | Description |
|---------------|----------------------------|------|-------------|
| `CIEFluxCode` | `cie_flux_code` | - | Space-separated N1-N5 values (e.g., "92 68 42 8 3") |
| `LightOutputRatio` | `light_output_ratio` | % | Total efficiency of luminaire |
| `LuminousEfficacy` | `luminous_efficacy` | lm/W | Luminaire output per watt |
| `DownwardFluxFraction` | `downward_flux_fraction` | % | Percentage of light below horizontal |
| `DownwardLightOutputRatio` | `downward_light_output_ratio` | % | DLOR = N1 x LOR / 100 |
| `UpwardLightOutputRatio` | `upward_light_output_ratio` | % | ULOR = N4 x LOR / 100 |
| `LuminaireLuminance` | `luminaire_luminance` | cd/m2 | Average luminance at 65deg |
| `CutOffAngle` | `cut_off_angle` | deg | Angle where I < 2.5% of max |
| `UGR4H8H705020` | `ugr_4h_8h_705020` | - | Crosswise and endwise UGR |
| `PhotometricCode` | `photometric_code` | - | Distribution-Beam classification |
| `TenthPeakDivergence` | `tenth_peak_divergence` | deg | Field angles (10% intensity) |
| `HalfPeakDivergence` | `half_peak_divergence` | deg | Beam angles (50% intensity) |
| `LightDistributionBUGRating` | `light_distribution_bug_rating` | - | BUG rating (e.g., "B2-U0-G1") |

## Property Definitions

### CIE Flux Code (`cie_flux_code`)

Five values representing percentage of lamp flux in angular zones:
- **N1**: Flux in lower hemisphere (0-90deg) - equivalent to DLOR
- **N2**: Flux within 0-60deg zone
- **N3**: Flux within 0-40deg zone
- **N4**: Flux in upper hemisphere (90-180deg) - equivalent to ULOR
- **N5**: Flux in 90-120deg zone (near-horizontal uplight)

Example: `"92 68 42 8 3"` means:
- 92% of flux goes downward (N1)
- 68% within 60deg of nadir (N2)
- 42% within 40deg of nadir (N3)
- 8% goes upward (N4)
- 3% is near-horizontal uplight (N5)

### Light Output Ratio (`light_output_ratio`)

Total efficiency of the luminaire:
```
LOR = (Luminaire output flux / Lamp rated flux) x 100%
```

Read directly from LDT file field or calculated from intensities.

### Luminous Efficacy (`luminous_efficacy`)

System efficacy accounting for LOR:
```
Luminaire Efficacy = (Lamp flux x LOR / 100) / System Wattage
```

This differs from lamp efficacy which doesn't account for luminaire losses.

### Downward/Upward Light Output Ratio

Derived from CIE flux codes and LOR:
```
DLOR = N1 x LOR / 100
ULOR = N4 x LOR / 100
```

### Luminaire Luminance (`luminaire_luminance`)

Average luminance at 65deg viewing angle (standard UGR calculation angle):
```
L = I / A_projected
```
Where:
- I = intensity at 65deg (average of C0 and C90)
- A_projected = luminous area x cos(65deg)

### Cut-off Angle (`cut_off_angle`)

The angle from nadir where intensity drops below 2.5% of maximum. Used for glare assessment. For example, a luminaire with 70deg cut-off has no direct light visible above 70deg from nadir.

### UGR Table Values (`ugr_4h_8h_705020`)

Unified Glare Rating calculated for a standard reference room:
- Room size: 4H x 8H (where H = mounting height)
- Reflectances: Ceiling 70%, Walls 50%, Floor 20%
- Two values: **Crosswise** (looking across short axis) and **Endwise** (looking along long axis)

Lower UGR = less glare. Typical targets:
- Offices: UGR <= 19
- Industrial: UGR <= 25
- Corridors: UGR <= 28

### Photometric Code (`photometric_code`)

Classification using distribution type and beam width:

**Distribution types:**
- `D` = Direct (DLOR >= 90%)
- `SD` = Semi-direct (60-90%)
- `GD` = General diffuse (40-60%)
- `SI` = Semi-indirect (10-40%)
- `I` = Indirect (< 10%)

**Beam widths:**
- `VN` = Very narrow (< 20deg)
- `N` = Narrow (20-30deg)
- `M` = Medium (30-45deg)
- `W` = Wide (45-60deg)
- `VW` = Very wide (> 60deg)

Example: `"D-M"` = Direct distribution with Medium beam width

### Peak Divergence Angles

- **Half Peak (Beam angle)**: Where intensity drops to 50% of maximum
- **Tenth Peak (Field angle)**: Where intensity drops to 10% of maximum

Both reported for C0-C180 and C90-C270 planes:
```
half_peak_divergence: (beam_C0, beam_C90)
tenth_peak_divergence: (field_C0, field_C90)
```

### BUG Rating (`light_distribution_bug_rating`)

IESNA TM-15-11 Backlight-Uplight-Glare rating for outdoor luminaires:
- **B** (Backlight): 0-5, light behind luminaire
- **U** (Uplight): 0-5, light above horizontal
- **G** (Glare): 0-5, high-angle forward light

Example: `"B2-U0-G1"` = Low backlight, no uplight, very low glare

## Usage Examples

### Generate GLDF XML Properties

```rust
use eulumdat::{Eulumdat, GldfPhotometricData};

let ldt = Eulumdat::from_file("luminaire.ldt")?;
let gldf = GldfPhotometricData::from_eulumdat(&ldt);

// Generate XML elements
let props = gldf.to_gldf_properties();
let mut xml = String::from("<Photometry>\n");
for (key, value) in props {
    xml.push_str(&format!("  <{}>{}</{}>\n", key, value, key));
}
xml.push_str("</Photometry>");
```

### Access Individual Calculations

```rust
use eulumdat::{Eulumdat, PhotometricCalculations};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// CIE Flux Codes
let cie = PhotometricCalculations::cie_flux_codes(&ldt);
println!("CIE: {} {} {} {} {}", cie.n1, cie.n2, cie.n3, cie.n4, cie.n5);

// Beam angles
let beam = PhotometricCalculations::beam_angle(&ldt);
let field = PhotometricCalculations::field_angle(&ldt);
println!("Beam: {:.1}deg, Field: {:.1}deg", beam, field);

// Per-plane beam angles
let beam_c0 = PhotometricCalculations::beam_angle_for_plane(&ldt, 0.0);
let beam_c90 = PhotometricCalculations::beam_angle_for_plane(&ldt, 90.0);

// Spacing criterion
let (s_h_c0, s_h_c90) = PhotometricCalculations::spacing_criteria(&ldt);

// Luminaire luminance at custom angle
let luminance = PhotometricCalculations::luminaire_luminance(&ldt, 75.0);

// Custom UGR calculation
let ugr_params = UgrParams {
    room_length: 10.0,
    room_width: 6.0,
    mounting_height: 3.0,
    eye_height: 1.2,
    observer_x: 5.0,
    observer_y: 3.0,
    luminaire_positions: vec![(3.0, 3.0), (7.0, 3.0)],
    rho_ceiling: 0.7,
    rho_wall: 0.5,
    rho_floor: 0.2,
    illuminance: 500.0,
};
let ugr = PhotometricCalculations::ugr(&ldt, &ugr_params);
```

### Text Reports

```rust
use eulumdat::{Eulumdat, GldfPhotometricData, PhotometricSummary};

let ldt = Eulumdat::from_file("luminaire.ldt")?;

// Full GLDF report
let gldf = GldfPhotometricData::from_eulumdat(&ldt);
println!("{}", gldf.to_text());

// Extended summary with zonal lumens
let summary = PhotometricSummary::from_eulumdat(&ldt);
println!("{}", summary.to_text());

// Compact one-liner
println!("{}", summary.to_compact());
```

## Data Flow for GLDF Creation

```
LDT/IES File
    |
    v
Eulumdat::from_file() or IesParser::parse()
    |
    v
Eulumdat struct (raw photometric data)
    |
    +---> GldfPhotometricData::from_eulumdat()
    |         |
    |         v
    |     to_gldf_properties() --> XML generation
    |
    +---> PhotometricSummary::from_eulumdat()
    |         |
    |         v
    |     to_key_value() --> Reports, displays
    |
    +---> Individual calculations via PhotometricCalculations::*()
```

## Notes

1. **Intensity Units**: Internal calculations use cd/klm (candelas per kilolumen). Actual candelas are obtained by multiplying by total lamp flux / 1000.

2. **Symmetry Handling**: All calculations properly handle LDT symmetry types (none, vertical axis, C0-C180, C90-C270, both planes).

3. **IES Support**: IES files parsed via `IesParser::parse()` produce an `Eulumdat` struct with equivalent data, so all GLDF calculations work identically.

4. **Missing Data**: If required LDT fields are missing (e.g., luminous area for luminance calculation), methods return 0.0 or sensible defaults.

5. **BUG Rating**: Calculated per IESNA TM-15-11, requires intensity data up to at least 90deg gamma for accurate results.
