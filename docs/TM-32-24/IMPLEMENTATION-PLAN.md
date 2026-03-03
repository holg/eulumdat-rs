# TM-32-24 BIM Parameters Implementation Plan

## Overview

**ANSI/IES TM-32-24** defines standard BIM (Building Information Modeling) parameters for lighting equipment. This document outlines how to implement TM-32-24 support in the `atla` crate (which handles IES XML format - TM-33).

### Key Standards Referenced
- **TM-32-24**: ANSI/IES standard for BIM lighting parameters (this document)
- **TM-33-23**: IES XML format for photometric data (implemented in `atla` crate)
- **GLDF**: Global Lighting Data Format (XML-based, comprehensive)
- **NEMA BIM 100-2021**: Similar parameter set with GUIDs
- **CEN/TS 17623:2021**: European technical specification for lighting data

### Architecture

```
┌─────────────────────┐     ┌─────────────────────┐
│  LDT/IES text files │     │  IES XML (TM-33)    │
│     (eulumdat)      │     │      (atla)         │
└──────────┬──────────┘     └──────────┬──────────┘
           │                           │
           │    ┌──────────────────┐   │
           └───►│  BimParameters   │◄──┘
                │   (TM-32-24)     │
                └────────┬─────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   Revit Shared     GLDF XML         JSON/CSV
   Parameters       Export           Export
```

## Parameter Groups

TM-32-24 organizes parameters into these groups:

| Group | Required | Optional | Total |
|-------|----------|----------|-------|
| General | 4 | 4 | 8 |
| Photometric | 4 (+4 Revit built-in) | 15 | 23 |
| Mounting | 1 | 0 | 1 |
| Mechanical | 1 | 25 | 26 |
| Electrical | 8 | 28 | 36 |
| Emergency | 0 | 12 | 12 |
| Maintenance | 0 | 8 | 8 |
| Acoustics | 0 | 2 | 2 |

## Mapping to Existing ATLA/TM-33 Fields

### Already Available in `LuminaireOpticalData` struct (atla crate)

| TM-32-24 Parameter | ATLA Field | Notes |
|-------------------|------------|-------|
| Product Name | `header.description` | Direct mapping |
| Manufacturer | `header.manufacturer` | Direct mapping |
| Catalog Number | `header.catalog_number` | Direct mapping |
| Product Documentation Link | `header.more_info_uri` | Direct mapping |
| Color Temperature (CCT) | `emitters[].cct` | Already numeric (Kelvin) |
| Color Rendering Index (CRI) | `emitters[].color_rendering.ra` | Already numeric (0-100) |
| TM-30 Rf/Rg | `emitters[].color_rendering.rf/rg` | Full TM-30 support |
| Watts (Rated Input Power) | `emitters[].input_watts` | Already in struct |
| Power Factor | `emitters[].power_factor` | Already in struct (0-1) |
| Lamp Quantity | `emitters[].quantity` | Already in struct |
| Product Length | `luminaire.dimensions.length` | In mm |
| Product Width | `luminaire.dimensions.width` | In mm |
| Product Height | `luminaire.dimensions.height` | In mm |
| Luminous Area Dimensions | `luminaire.luminous_openings[]` | Shape + dimensions |
| Mounting Type | `luminaire.mounting` | String field |
| .ies File (Photometric Web) | `emitters[].intensity_distribution` | The photometric data itself |

### Available via Calculations (eulumdat crate)

| TM-32-24 Parameter | Calculation Method | Notes |
|-------------------|-------------------|-------|
| BUG Rating | `BugDiagram::from_eulumdat()` | Need to add `from_atla()` |
| Cut-off Angle | `comprehensive_beam_analysis()` | Need to add `from_atla()` |
| Luminous Flux | `total_luminous_flux()` | Already on LuminaireOpticalData |
| CU Table | `calculate_cu_table()` | Need to add `from_atla()` |

### Need New Fields in ATLA

| TM-32-24 Parameter | Implementation | Priority |
|-------------------|----------------|----------|
| **Apparent Power (VA)** | New field on Emitter | High |
| **Voltage Nominal** | New field on Emitter | High |
| **Phase** | New enum + field | Medium |
| **Dimmable** | New field (bool) | Medium |
| **Dimming Method** | New field (String) | Medium |
| **Housing Color** | New field on Luminaire | Medium |
| **Housing Material** | New field on Luminaire | Medium |
| **Weight** | New field on Luminaire | Medium |
| **Melanopic Factor** | New field or calculate from SPD | Low |
| **Flicker (Pst LM)** | New field | Low |
| **Emergency fields** | New Emergency struct | Low |
| **Acoustic fields** | New Acoustics struct | Low |

## Proposed Data Structures

### New `BimParameters` struct

```rust
/// TM-32-24 BIM parameters for lighting equipment
#[derive(Debug, Clone, Default)]
pub struct BimParameters {
    // General
    pub last_updated: Option<String>,           // ISO 8601 timestamp
    pub manufacturer: Option<String>,
    pub manufacturer_url: Option<String>,
    pub product_name: Option<String>,
    pub product_family: Option<String>,
    pub description: Option<String>,

    // Photometric (beyond what's in Eulumdat)
    pub cct_kelvin: Option<u32>,                // Parsed from color_appearance
    pub cri: Option<u8>,                        // Parsed from color_rendering_group
    pub bug_rating: Option<String>,             // "B2 U1 G3"
    pub cutoff_angle: Option<f64>,              // degrees
    pub melanopic_factor: Option<f64>,          // 0.0-2.0 typical
    pub flicker_pst_lm: Option<f64>,
    pub photobiology_class: Option<u8>,         // 1, 2, or 3

    // Electrical
    pub apparent_power_va: Option<f64>,
    pub power_factor: Option<f64>,              // 0.0-1.0
    pub voltage_nominal: Option<f64>,           // V
    pub voltage_type: Option<VoltageType>,      // AC, DC, UC
    pub phase: Option<ElectricalPhase>,
    pub nominal_current: Option<f64>,           // A
    pub nominal_frequency: Option<f64>,         // Hz (50 or 60)
    pub inrush_current: Option<f64>,            // A
    pub dimmable: Option<bool>,
    pub dimming_method: Option<String>,         // "DALI", "0-10V", "Phase", etc.
    pub dimming_range: Option<(f64, f64)>,      // (min%, max%)
    pub color_controllable: Option<bool>,
    pub cct_controllable: Option<bool>,
    pub control_interface: Option<String>,      // "DALI-2", "DMX512", etc.
    pub control_gear: Option<String>,
    pub control_gear_included: Option<bool>,
    pub led_drive_type: Option<LedDriveType>,   // CC, CV, N/A
    pub led_secondary_voltage: Option<f64>,     // V

    // Mechanical
    pub housing_shape: Option<HousingShape>,    // Cuboid, Cylinder
    pub housing_color: Option<String>,          // RAL number or name
    pub housing_material: Option<String>,
    pub diameter: Option<f64>,                  // mm (for cylindrical)
    pub diameter_luminous: Option<f64>,         // mm
    pub weight: Option<f64>,                    // kg
    pub weight_shipping: Option<f64>,           // kg
    pub shipping_length: Option<f64>,           // mm
    pub shipping_width: Option<f64>,            // mm
    pub shipping_height: Option<f64>,           // mm
    pub flexible: Option<bool>,
    pub halogen_free: Option<bool>,
    pub silicone_free: Option<bool>,
    pub sealing_material: Option<String>,
    pub minimum_clearance: Option<f64>,         // mm
    pub drive_over_rated: Option<bool>,
    pub walk_over_rated: Option<bool>,
    pub roll_over_rated: Option<bool>,

    // Mounting
    pub mounting_type: Option<MountingType>,
    pub cutout_diameter: Option<f64>,           // mm
    pub cutout_length: Option<f64>,             // mm
    pub cutout_width: Option<f64>,              // mm
    pub recessed_depth: Option<f64>,            // mm
    pub ceiling_thickness_min: Option<f64>,     // mm
    pub ceiling_thickness_max: Option<f64>,     // mm
    pub covering_insulation_allowed: Option<bool>,

    // Maintenance
    pub light_loss_factor: Option<f64>,         // 0.0-1.0
    pub lamp_lumen_depreciation: Option<f64>,   // LLD factor
    pub luminaire_dirt_depreciation: Option<f64>, // LDD factor
    pub lamp_survival_factor: Option<f64>,      // LSF
    pub burn_in_time: Option<f64>,              // hours
    pub led_module_replaceable: Option<bool>,
    pub light_source_included: Option<bool>,
    pub projected_life_hours: Option<u32>,

    // Emergency
    pub emergency_capable: Option<bool>,
    pub emergency_type: Option<EmergencyType>,
    pub emergency_unit_integrated: Option<bool>,
    pub emergency_luminous_flux: Option<f64>,   // lumens
    pub emergency_light_source_type: Option<String>,
    pub battery_type: Option<String>,
    pub battery_capacity: Option<f64>,          // Ah
    pub battery_voltage: Option<f64>,           // V
    pub battery_exchange_possible: Option<bool>,

    // Environmental
    pub ambient_temperature_range: Option<(f64, f64)>, // (min°C, max°C)
    pub rated_ambient_temperature: Option<f64>, // °C
    pub temperature_on_aperture: Option<f64>,   // °C
    pub relative_humidity_range: Option<(f64, f64)>, // (min%, max%)
    pub epd_available: Option<bool>,            // Environmental Product Declaration
    pub epd_url: Option<String>,

    // Sensor/Detection
    pub with_sensor: Option<bool>,
    pub detector_type: Option<String>,          // PIR, Microwave, etc.
    pub detection_method: Option<String>,
    pub detection_area: Option<String>,         // Shape description
    pub detection_area_adjustable: Option<bool>,
    pub detection_range_adjustable: Option<bool>,
    pub presence_detection_area: Option<f64>,   // m²
    pub radiation_power: Option<f64>,           // W (for active sensors)

    // Acoustics
    pub acoustic_absorption_average: Option<f64>, // NRC value
    pub acoustic_absorption_table: Option<Vec<(f64, f64)>>, // (freq_hz, absorption)

    // Accessories
    pub with_switch: Option<bool>,
    pub with_dimmer: Option<bool>,
    pub with_power_plug: Option<bool>,
    pub with_connecting_cable: Option<bool>,
    pub through_wiring: Option<bool>,
    pub with_starter: Option<bool>,
    pub socket_type: Option<String>,            // E27, GU10, etc.

    // Additional
    pub product_documentation_url: Option<String>,
    pub product_datasheet_url: Option<String>,
    pub circuit_number: Option<String>,
    pub quick_ship: Option<bool>,
    pub quick_ship_terms: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageType {
    AC,
    DC,
    UC, // Universal (AC/DC)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricalPhase {
    SinglePhase,
    ThreePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedDriveType {
    ConstantCurrent,
    ConstantVoltage,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HousingShape {
    Cuboid,
    Cylinder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountingType {
    Ceiling,
    Wall,
    Floor,
    Pendant,
    Recessed,
    Track,
    Surface,
    Pole,
    Bollard,
    InGround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyType {
    Maintained,      // Always on
    NonMaintained,   // On only in emergency
    Combined,        // Part always on, part emergency only
}
```

### GUID Constants

```rust
/// TM-32-24 / NEMA BIM 100-2021 GUIDs for Revit shared parameters
pub mod guids {
    pub const LAST_UPDATED: &str = "8350e45f-5dbb-4391-a637-620154f021c7";
    pub const MANUFACTURER: &str = "9597d401-cfd4-4394-af6f-d84565b83b38";
    pub const MANUFACTURER_URL: &str = "bddc75f4-8e27-4afc-add0-de7358bbf6d3";
    pub const PRODUCT_NAME: &str = "15f5ff4d-f917-45e4-ae77-5521e67f0a27";
    pub const CCT: &str = "0e3d3a2c-30bc-417c-af34-b0c1bbba7ffc";
    pub const CRI: &str = "acfa19a5-870c-4f39-8131-df68636c1cdc";
    pub const APPARENT_POWER: &str = "a1c4f25a-1260-41a5-bac8-97556a57b236";
    pub const LAMP_TYPE: &str = "3cd5049e-0b1e-4cd7-963c-6a59f1b87ee4";
    pub const WATTS: &str = "c9846d7e-34ca-4f59-94ab-3452803f997e";
    pub const VOLTAGE_NOMINAL: &str = "7bcae613-49d5-40a3-a27e-8df1945ce73e";
    pub const PHASE: &str = "652939b9-9afa-4e8c-9ab1-767525e78a49";
    pub const HOUSING_SHAPE: &str = "5f141709-2625-47af-8618-00e1c5e9eac1";
    pub const HOUSING_COLOR: &str = "2f317fc3-3f64-4214-83fd-390823fc9d91";
    pub const HOUSING_MATERIAL: &str = "9704f476-596b-43e9-919a-42f7881c8921";
    pub const DIMMABLE: &str = "4d4881f3-5a77-41eb-a140-e03a297c2351";
    pub const PRODUCT_DEPTH: &str = "038052b7-7d2c-45e2-981f-7dc0b042c255";
    pub const PRODUCT_DIAMETER: &str = "c59a50f1-6e72-4981-ad58-2a65b8fbc846";
    pub const PRODUCT_HEIGHT: &str = "8822b540-105f-47d1-847d-e3df372faa1e";
    pub const PRODUCT_LENGTH: &str = "51782a3a-e0cb-45b4-86b1-2788dd5d6148";
    pub const PRODUCT_WIDTH: &str = "d5e464aa-5c32-4e45-91b5-8db19b6f6ebe";
    pub const MOUNTING_TYPE: &str = "61767f07-f194-447c-b6fb-66f9285a7d39";
    pub const WEIGHT: &str = "5de1affc-b992-4202-ab5f-bf414e76a681";
    pub const BUG_RATING: &str = "1a808da0-3e8d-4e01-82e5-b299efd4e129";
    pub const EMERGENCY_CAPABLE: &str = "04ac16ce-15d8-4607-a43a-1fece8f1cc51";
    pub const EMERGENCY_TYPE: &str = "3602aa1f-59fe-4c06-94f3-fdf66550fa62";
    // ... more GUIDs
}
```

## Implementation Phases

### Phase 1: Core Parameter Extraction (High Priority)

1. **Add `BimParameters` struct** to `crates/atla/src/bim.rs`
2. **Implement `from_atla()`** to extract available data from TM-33 XML:
   ```rust
   impl BimParameters {
       /// Extract BIM parameters from IES XML (TM-33) data
       pub fn from_atla(doc: &LuminaireOpticalData) -> Self {
           let mut params = BimParameters::default();

           // Direct mappings from Header
           params.product_name = doc.header.description.clone();
           params.manufacturer = doc.header.manufacturer.clone();
           params.manufacturer_url = doc.header.more_info_uri.clone();
           params.last_updated = doc.header.report_date.clone();

           // Dimensions from Luminaire
           if let Some(lum) = &doc.luminaire {
               if let Some(dims) = &lum.dimensions {
                   params.product_length = Some(dims.length);
                   params.product_width = Some(dims.width);
                   params.product_height = Some(dims.height);
               }
               params.mounting_type = parse_mounting_type(&lum.mounting);
           }

           // Emitter data (use first emitter for primary values)
           if let Some(emitter) = doc.emitters.first() {
               params.cct_kelvin = emitter.cct.map(|c| c as u32);
               params.cri = emitter.color_rendering.as_ref()
                   .and_then(|cr| cr.ra.map(|r| r as u8));
               params.power_factor = emitter.power_factor;
               params.watts = emitter.input_watts;
           }

           // Totals
           params.total_luminous_flux = Some(doc.total_luminous_flux());
           params.total_watts = Some(doc.total_input_watts());

           params
       }
   }
   ```

3. **Implement `from_eulumdat()`** for LDT/IES text file support:
   ```rust
   impl BimParameters {
       /// Extract BIM parameters from EULUMDAT/LDT data
       /// Note: LDT format has less metadata than IES XML
       pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
           let mut params = BimParameters::default();

           // Direct mappings
           params.product_name = Some(ldt.luminaire_name.clone());
           params.manufacturer = Some(ldt.identification.clone());

           // Dimensions
           params.product_length = Some(ldt.length);
           params.product_width = Some(ldt.width);
           params.product_height = Some(ldt.height);

           // Parse CCT from lamp_sets (string like "3000K")
           if let Some(lamp) = ldt.lamp_sets.first() {
               params.cct_kelvin = parse_cct(&lamp.color_appearance);
               params.cri = parse_cri(&lamp.color_rendering_group);
           }

           // Calculate totals
           params.total_watts = Some(ldt.total_wattage());

           // BUG rating (requires photometric data)
           let bug = BugDiagram::from_eulumdat(ldt);
           params.bug_rating = Some(bug.rating.to_string());

           params
       }
   }
   ```

4. **Add helper functions** for parsing LDT string fields:
   ```rust
   fn parse_cct(color_appearance: &str) -> Option<u32> {
       // "3000K" -> 3000, "4000 K" -> 4000, etc.
       let s = color_appearance.trim().to_uppercase();
       s.strip_suffix('K')
           .or_else(|| s.strip_suffix(" K"))
           .and_then(|n| n.trim().parse().ok())
   }

   fn parse_cri(cri_group: &str) -> Option<u8> {
       match cri_group.trim() {
           "1A" => Some(95), // ≥90, use midpoint
           "1B" => Some(85), // 80-89
           "2A" => Some(75), // 70-79
           "2B" => Some(65), // 60-69
           "3" => Some(50),  // 40-59
           "4" => Some(30),  // <40
           _ => None,
       }
   }

   fn parse_mounting_type(mounting: &Option<String>) -> Option<MountingType> {
       mounting.as_ref().and_then(|m| {
           match m.to_lowercase().as_str() {
               "recessed" => Some(MountingType::Recessed),
               "surface" => Some(MountingType::Surface),
               "pendant" => Some(MountingType::Pendant),
               "track" => Some(MountingType::Track),
               "wall" => Some(MountingType::Wall),
               "floor" => Some(MountingType::Floor),
               "ceiling" => Some(MountingType::Ceiling),
               "pole" => Some(MountingType::Pole),
               "bollard" => Some(MountingType::Bollard),
               "in-ground" | "inground" => Some(MountingType::InGround),
               _ => None,
           }
       })
   }
   ```

### Phase 2: Revit Shared Parameters Export

1. **Generate Revit shared parameters file**:
   ```rust
   impl BimParameters {
       pub fn to_revit_shared_params(&self) -> String {
           // Generate ANNEX_A.txt format
       }
   }
   ```

2. **Export individual parameter values**:
   ```rust
   pub fn to_revit_family_params(&self) -> Vec<(String, String, String)> {
       // Returns (GUID, name, value) tuples
   }
   ```

### Phase 3: GLDF Integration

1. **Map to GLDF XML elements**:
   - `ProductMetaData.Description` → `description`
   - `DescriptivePhotometry.LightDistribution.BUG-Rating` → `bug_rating`
   - `SimpleGeometry.Cuboid.*` → dimensions
   - `ControlGears.ControlGear` → control gear info
   - etc.

2. **Create GLDF export module** (separate crate):
   ```rust
   // In gldf-rs crate
   pub fn from_bim_parameters(params: &BimParameters) -> GldfProduct {
       // Map TM-32-24 to GLDF structure
   }
   ```

### Phase 4: Extended Parameters

Add fields that require additional data sources:
- Melanopic factor (requires spectral data)
- Flicker metrics (requires temporal measurement data)
- Acoustic absorption (requires measurement data)
- Detection areas (requires sensor specifications)

## File Structure

```
crates/atla/src/
├── lib.rs              # Add: pub mod bim;
├── bim.rs              # NEW: BimParameters, enums, GUIDs
├── bim/
│   ├── mod.rs          # Module exports
│   ├── parameters.rs   # BimParameters struct
│   ├── enums.rs        # VoltageType, MountingType, etc.
│   ├── guids.rs        # TM-32-24/NEMA GUIDs
│   ├── parsers.rs      # CCT, CRI, etc. parsing
│   └── revit.rs        # Revit export functions
└── ...

crates/eulumdat/src/
├── bim_compat.rs       # NEW: from_eulumdat() adapter
└── ...                 # (wrapper that converts to atla first)
```

### Design Decision: Location of BimParameters

The `BimParameters` struct lives in the `atla` crate because:
1. TM-32-24 is designed for IES XML (TM-33) format
2. ATLA/TM-33 has richer metadata than LDT/IES text formats
3. The mapping from TM-33 to TM-32-24 is more direct

For LDT/IES text files, use the conversion path:
```rust
// Option 1: Convert LDT to ATLA first, then extract BIM
let ldt = Eulumdat::from_file("luminaire.ldt")?;
let atla = LuminaireOpticalData::from_eulumdat(&ldt);
let bim = BimParameters::from_atla(&atla);

// Option 2: Direct convenience function in eulumdat crate
let ldt = Eulumdat::from_file("luminaire.ldt")?;
let bim = BimParameters::from_eulumdat(&ldt); // uses atla internally
```

## Testing Strategy

1. **Unit tests** for parsers:
   ```rust
   #[test]
   fn test_parse_cct() {
       assert_eq!(parse_cct("3000K"), Some(3000));
       assert_eq!(parse_cct("4000 K"), Some(4000));
       assert_eq!(parse_cct("warm white"), None);
   }
   ```

2. **Integration tests** with real LDT files:
   ```rust
   #[test]
   fn test_bim_from_road_luminaire() {
       let ldt = Eulumdat::from_file("tests/files/road_luminaire.ldt").unwrap();
       let bim = BimParameters::from_eulumdat(&ldt);
       assert!(bim.bug_rating.is_some());
       assert!(bim.cct_kelvin.unwrap() > 2000);
   }
   ```

3. **Validation tests** against GLDF reference files

## CLI Integration

Add BIM export command to `eulumdat-cli`:

```bash
# Export BIM parameters as JSON
eulumdat bim luminaire.ldt --format json

# Export as Revit shared parameters
eulumdat bim luminaire.ldt --format revit

# Export specific parameters
eulumdat bim luminaire.ldt --params cct,cri,bug_rating
```

## Web/WASM Integration

Add BIM panel to the web editor:
- Display all extractable TM-32-24 parameters
- Show which parameters are missing/unavailable
- Allow manual entry of additional parameters
- Export to JSON/CSV

## Timeline Considerations

This implementation can be done incrementally:
1. Start with Phase 1 (core extraction) - most immediately useful
2. Add Revit export when BIM integration is needed
3. Add GLDF integration when that format support is added
4. Extended parameters can be added as data becomes available

## References

- TM-32-24 PDF: `docs/TM-32-24/TM-32-24.pdf`
- Annex A (Revit params): `docs/TM-32-24/ANNEX_A.txt`
- Annex B (mappings): `docs/TM-32-24/TM-32-24_AnnexB.xlsx`
- GLDF specification: https://gldf.io
- NEMA BIM 100-2021: Referenced in TM-32-24
