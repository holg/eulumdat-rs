//! TM-32-24 BIM Parameters for lighting equipment
//!
//! This module implements the ANSI/IES TM-32-24 standard for BIM (Building Information
//! Modeling) parameters for lighting equipment. It provides structured data extraction
//! from IES XML (TM-33) photometric files.
//!
//! # Overview
//!
//! TM-32-24 defines standard parameters organized into groups:
//! - General (manufacturer, product info)
//! - Photometric (CCT, CRI, flux, BUG rating)
//! - Electrical (power, voltage, dimming)
//! - Mechanical (dimensions, weight, materials)
//! - Mounting (type, cutout dimensions)
//! - Emergency (backup power, battery)
//! - Maintenance (depreciation factors)
//! - Acoustics (absorption)
//!
//! # Example
//!
//! ```rust,ignore
//! use atla::{LuminaireOpticalData, bim::BimParameters};
//!
//! let doc = atla::parse_file("luminaire.xml")?;
//! let bim = BimParameters::from_atla(&doc);
//!
//! println!("Manufacturer: {:?}", bim.manufacturer);
//! println!("CCT: {:?} K", bim.cct_kelvin);
//! println!("Power Factor: {:?}", bim.power_factor);
//! ```
//!
//! # References
//!
//! - ANSI/IES TM-32-24: BIM Parameters for Lighting Equipment
//! - NEMA BIM 100-2021: Similar parameter set with GUIDs
//! - CEN/TS 17623:2021: European technical specification

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::LuminaireOpticalData;

// ============================================================================
// Enums
// ============================================================================

/// Electrical voltage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VoltageType {
    /// Alternating current
    #[default]
    AC,
    /// Direct current
    DC,
    /// Universal (AC/DC)
    UC,
}

impl VoltageType {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "AC" => Some(VoltageType::AC),
            "DC" => Some(VoltageType::DC),
            "UC" | "AC/DC" | "UNIVERSAL" => Some(VoltageType::UC),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            VoltageType::AC => "AC",
            VoltageType::DC => "DC",
            VoltageType::UC => "UC",
        }
    }
}

/// Electrical phase type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ElectricalPhase {
    /// Single phase power
    #[default]
    SinglePhase,
    /// Three phase power
    ThreePhase,
}

impl ElectricalPhase {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        if lower.contains("single") || lower == "1" || lower == "1ph" {
            Some(ElectricalPhase::SinglePhase)
        } else if lower.contains("three") || lower == "3" || lower == "3ph" {
            Some(ElectricalPhase::ThreePhase)
        } else {
            None
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ElectricalPhase::SinglePhase => "Single Phase",
            ElectricalPhase::ThreePhase => "Three Phase",
        }
    }
}

/// LED driver type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LedDriveType {
    /// Constant current driver
    ConstantCurrent,
    /// Constant voltage driver
    ConstantVoltage,
    /// Not applicable (non-LED source)
    #[default]
    NotApplicable,
}

impl LedDriveType {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let upper = s.trim().to_uppercase();
        match upper.as_str() {
            "CC" | "CONSTANT CURRENT" | "CONSTANTCURRENT" => Some(LedDriveType::ConstantCurrent),
            "CV" | "CONSTANT VOLTAGE" | "CONSTANTVOLTAGE" => Some(LedDriveType::ConstantVoltage),
            "N/A" | "NA" | "NOT APPLICABLE" => Some(LedDriveType::NotApplicable),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            LedDriveType::ConstantCurrent => "CC",
            LedDriveType::ConstantVoltage => "CV",
            LedDriveType::NotApplicable => "N/A",
        }
    }
}

/// Luminaire housing shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HousingShape {
    /// Rectangular/cuboid housing
    #[default]
    Cuboid,
    /// Cylindrical housing
    Cylinder,
}

impl HousingShape {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        if lower.contains("cuboid")
            || lower.contains("rectangular")
            || lower.contains("square")
            || lower.contains("box")
        {
            Some(HousingShape::Cuboid)
        } else if lower.contains("cylinder")
            || lower.contains("circular")
            || lower.contains("round")
        {
            Some(HousingShape::Cylinder)
        } else {
            None
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            HousingShape::Cuboid => "Cuboid",
            HousingShape::Cylinder => "Cylinder",
        }
    }
}

/// Luminaire mounting type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MountingType {
    /// Ceiling mounted (surface or flush)
    Ceiling,
    /// Wall mounted
    Wall,
    /// Floor standing
    Floor,
    /// Pendant/suspended
    Pendant,
    /// Recessed into ceiling/wall
    Recessed,
    /// Track mounted
    Track,
    /// Surface mounted
    Surface,
    /// Pole mounted (outdoor)
    Pole,
    /// Bollard (outdoor)
    Bollard,
    /// In-ground (outdoor)
    InGround,
}

impl MountingType {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        match lower.as_str() {
            "ceiling" => Some(MountingType::Ceiling),
            "wall" => Some(MountingType::Wall),
            "floor" => Some(MountingType::Floor),
            "pendant" | "suspended" | "hanging" => Some(MountingType::Pendant),
            "recessed" | "embedded" => Some(MountingType::Recessed),
            "track" => Some(MountingType::Track),
            "surface" => Some(MountingType::Surface),
            "pole" | "post" => Some(MountingType::Pole),
            "bollard" => Some(MountingType::Bollard),
            "in-ground" | "inground" | "ground" => Some(MountingType::InGround),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            MountingType::Ceiling => "Ceiling",
            MountingType::Wall => "Wall",
            MountingType::Floor => "Floor",
            MountingType::Pendant => "Pendant",
            MountingType::Recessed => "Recessed",
            MountingType::Track => "Track",
            MountingType::Surface => "Surface",
            MountingType::Pole => "Pole",
            MountingType::Bollard => "Bollard",
            MountingType::InGround => "In-Ground",
        }
    }
}

/// Emergency lighting type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EmergencyType {
    /// Maintained - always on, continues in emergency
    Maintained,
    /// Non-maintained - only on during emergency
    NonMaintained,
    /// Combined - part always on, part emergency only
    Combined,
}

impl EmergencyType {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        if lower.contains("non-maintained") || lower.contains("nonmaintained") {
            Some(EmergencyType::NonMaintained)
        } else if lower.contains("maintained") {
            Some(EmergencyType::Maintained)
        } else if lower.contains("combined") {
            Some(EmergencyType::Combined)
        } else {
            None
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            EmergencyType::Maintained => "Maintained",
            EmergencyType::NonMaintained => "Non-Maintained",
            EmergencyType::Combined => "Combined",
        }
    }
}

// ============================================================================
// TM-32-24 / NEMA BIM GUIDs
// ============================================================================

/// TM-32-24 / NEMA BIM 100-2021 GUIDs for Revit shared parameters
pub mod guids {
    // General
    pub const LAST_UPDATED: &str = "8350e45f-5dbb-4391-a637-620154f021c7";
    pub const MANUFACTURER: &str = "9597d401-cfd4-4394-af6f-d84565b83b38";
    pub const MANUFACTURER_URL: &str = "bddc75f4-8e27-4afc-add0-de7358bbf6d3";
    pub const PRODUCT_NAME: &str = "15f5ff4d-f917-45e4-ae77-5521e67f0a27";
    pub const PRODUCT_FAMILY: &str = "5102b873-5fab-45b7-a2d0-f83ddee8d345";
    pub const DESCRIPTION: &str = "157f9768-92d6-4c2a-b6c6-4021de41bca4";

    // Photometric
    pub const CCT: &str = "0e3d3a2c-30bc-417c-af34-b0c1bbba7ffc";
    pub const CRI: &str = "acfa19a5-870c-4f39-8131-df68636c1cdc";
    pub const BUG_RATING: &str = "1a808da0-3e8d-4e01-82e5-b299efd4e129";

    // Electrical
    pub const APPARENT_POWER: &str = "a1c4f25a-1260-41a5-bac8-97556a57b236";
    pub const LAMP_TYPE: &str = "3cd5049e-0b1e-4cd7-963c-6a59f1b87ee4";
    pub const WATTS: &str = "c9846d7e-34ca-4f59-94ab-3452803f997e";
    pub const VOLTAGE_NOMINAL: &str = "7bcae613-49d5-40a3-a27e-8df1945ce73e";
    pub const PHASE: &str = "652939b9-9afa-4e8c-9ab1-767525e78a49";
    pub const DIMMABLE: &str = "4d4881f3-5a77-41eb-a140-e03a297c2351";
    pub const POWER_FACTOR: &str = "bde8cff9-2162-4d2d-b52a-a0ff1741d33d";

    // Mechanical
    pub const HOUSING_SHAPE: &str = "5f141709-2625-47af-8618-00e1c5e9eac1";
    pub const HOUSING_COLOR: &str = "2f317fc3-3f64-4214-83fd-390823fc9d91";
    pub const HOUSING_MATERIAL: &str = "9704f476-596b-43e9-919a-42f7881c8921";
    pub const PRODUCT_DEPTH: &str = "038052b7-7d2c-45e2-981f-7dc0b042c255";
    pub const PRODUCT_DIAMETER: &str = "c59a50f1-6e72-4981-ad58-2a65b8fbc846";
    pub const PRODUCT_HEIGHT: &str = "8822b540-105f-47d1-847d-e3df372faa1e";
    pub const PRODUCT_LENGTH: &str = "51782a3a-e0cb-45b4-86b1-2788dd5d6148";
    pub const PRODUCT_WIDTH: &str = "d5e464aa-5c32-4e45-91b5-8db19b6f6ebe";
    pub const WEIGHT: &str = "5de1affc-b992-4202-ab5f-bf414e76a681";

    // Mounting
    pub const MOUNTING_TYPE: &str = "61767f07-f194-447c-b6fb-66f9285a7d39";
    pub const CUTOUT_DIAMETER: &str = "48ea3b3c-7921-4716-a6ab-cbb9c4d5e5e1";
    pub const CUTOUT_LENGTH: &str = "b4d75e0a-7fd0-4db4-9abd-42a1e9a3bcfc";
    pub const CUTOUT_WIDTH: &str = "120b879b-b996-457d-ba4e-5684db29e113";
    pub const RECESSED_DEPTH: &str = "e9ed463c-ac3f-46fc-9dd8-f661446876d4";

    // Emergency
    pub const EMERGENCY_CAPABLE: &str = "04ac16ce-15d8-4607-a43a-1fece8f1cc51";
    pub const EMERGENCY_TYPE: &str = "3602aa1f-59fe-4c06-94f3-fdf66550fa62";
    pub const BATTERY_TYPE: &str = "2229ac1a-3802-40a7-89cd-80aeb0177b05";
    pub const BATTERY_CAPACITY: &str = "2b6a2c17-1cf5-44a2-a605-a3acf40c9b3b";
    pub const BATTERY_VOLTAGE: &str = "3258ce83-85d5-40e9-998e-b73fee7e5d87";

    // Maintenance
    pub const LIGHT_LOSS_FACTOR: &str = "9704f476-596b-43e9-919a-42f7881c8921";
    pub const LAMP_LUMEN_DEPRECIATION: &str = "5be0ecbb-d514-4549-80be-b4333a4aa35e";
    pub const LUMINAIRE_DIRT_DEPRECIATION: &str = "0acd978b-9495-4fe4-a8c7-66bacc6243a2";

    // Acoustics
    pub const ACOUSTIC_ABSORPTION_AVG: &str = "8719b75f-0994-47e7-94c8-66cf2a88fc4d";
}

// ============================================================================
// BimParameters struct
// ============================================================================

/// TM-32-24 BIM parameters for lighting equipment
///
/// This struct contains all standard BIM parameters as defined in ANSI/IES TM-32-24.
/// Parameters are organized into logical groups matching the standard.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BimParameters {
    // ========================================================================
    // General
    // ========================================================================
    /// Last updated timestamp (ISO 8601)
    pub last_updated: Option<String>,
    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Manufacturer website URL
    pub manufacturer_url: Option<String>,
    /// Product name / description
    pub product_name: Option<String>,
    /// Product family/series
    pub product_family: Option<String>,
    /// Detailed description
    pub description: Option<String>,
    /// Catalog/model number
    pub catalog_number: Option<String>,

    // ========================================================================
    // Photometric
    // ========================================================================
    /// Correlated Color Temperature in Kelvin
    pub cct_kelvin: Option<u32>,
    /// Color Rendering Index (Ra, 0-100)
    pub cri: Option<u8>,
    /// TM-30 Fidelity Index (Rf)
    pub tm30_rf: Option<u8>,
    /// TM-30 Gamut Index (Rg)
    pub tm30_rg: Option<u8>,
    /// BUG rating string (e.g., "B2 U1 G3")
    pub bug_rating: Option<String>,
    /// Cut-off angle in degrees
    pub cutoff_angle: Option<f64>,
    /// Total luminous flux in lumens
    pub total_luminous_flux: Option<f64>,
    /// Melanopic factor (for circadian lighting)
    pub melanopic_factor: Option<f64>,
    /// Flicker metric (Pst LM)
    pub flicker_pst_lm: Option<f64>,
    /// Photobiology risk class (1, 2, or 3)
    pub photobiology_class: Option<u8>,

    // ========================================================================
    // Electrical
    // ========================================================================
    /// Total input power in watts
    pub watts: Option<f64>,
    /// Apparent power in volt-amperes
    pub apparent_power_va: Option<f64>,
    /// Power factor (0.0-1.0)
    pub power_factor: Option<f64>,
    /// Nominal voltage in volts
    pub voltage_nominal: Option<f64>,
    /// Voltage type (AC/DC/UC)
    pub voltage_type: Option<VoltageType>,
    /// Electrical phase
    pub phase: Option<ElectricalPhase>,
    /// Nominal current in amperes
    pub nominal_current: Option<f64>,
    /// Nominal frequency in Hz (50 or 60)
    pub nominal_frequency: Option<f64>,
    /// Inrush current in amperes
    pub inrush_current: Option<f64>,
    /// Whether luminaire is dimmable
    pub dimmable: Option<bool>,
    /// Dimming method/protocol (e.g., "DALI", "0-10V")
    pub dimming_method: Option<String>,
    /// Dimming range as (min%, max%)
    pub dimming_range: Option<(f64, f64)>,
    /// Color controllable (tunable white)
    pub color_controllable: Option<bool>,
    /// CCT controllable
    pub cct_controllable: Option<bool>,
    /// Control interface (e.g., "DALI-2", "DMX512")
    pub control_interface: Option<String>,
    /// Control gear description
    pub control_gear: Option<String>,
    /// Control gear included with luminaire
    pub control_gear_included: Option<bool>,
    /// LED drive type
    pub led_drive_type: Option<LedDriveType>,
    /// LED secondary/output voltage
    pub led_secondary_voltage: Option<f64>,
    /// Light source type (e.g., "LED", "Fluorescent")
    pub light_source_type: Option<String>,
    /// Number of light sources/lamps
    pub lamp_quantity: Option<u32>,

    // ========================================================================
    // Mechanical
    // ========================================================================
    /// Housing shape
    pub housing_shape: Option<HousingShape>,
    /// Housing color (RAL number or name)
    pub housing_color: Option<String>,
    /// Housing material
    pub housing_material: Option<String>,
    /// Product length in mm
    pub product_length: Option<f64>,
    /// Product width in mm
    pub product_width: Option<f64>,
    /// Product height in mm
    pub product_height: Option<f64>,
    /// Product diameter in mm (for cylindrical)
    pub product_diameter: Option<f64>,
    /// Luminous area length in mm
    pub luminous_length: Option<f64>,
    /// Luminous area width in mm
    pub luminous_width: Option<f64>,
    /// Luminous area diameter in mm
    pub luminous_diameter: Option<f64>,
    /// Product weight in kg
    pub weight: Option<f64>,
    /// Shipping weight in kg
    pub weight_shipping: Option<f64>,
    /// Shipping length in mm
    pub shipping_length: Option<f64>,
    /// Shipping width in mm
    pub shipping_width: Option<f64>,
    /// Shipping height in mm
    pub shipping_height: Option<f64>,
    /// Flexible/pliable luminaire body
    pub flexible: Option<bool>,
    /// Halogen-free materials
    pub halogen_free: Option<bool>,
    /// Silicone-free materials
    pub silicone_free: Option<bool>,
    /// Sealing material
    pub sealing_material: Option<String>,
    /// Minimum clearance distance in mm
    pub minimum_clearance: Option<f64>,
    /// Drive-over rated (vehicle traffic)
    pub drive_over_rated: Option<bool>,
    /// Walk-over rated (pedestrian traffic)
    pub walk_over_rated: Option<bool>,
    /// Roll-over rated (vehicle roll-over)
    pub roll_over_rated: Option<bool>,

    // ========================================================================
    // Mounting
    // ========================================================================
    /// Mounting type
    pub mounting_type: Option<MountingType>,
    /// Cutout diameter in mm (for recessed circular)
    pub cutout_diameter: Option<f64>,
    /// Cutout length in mm (for recessed rectangular)
    pub cutout_length: Option<f64>,
    /// Cutout width in mm (for recessed rectangular)
    pub cutout_width: Option<f64>,
    /// Required recess depth in mm
    pub recessed_depth: Option<f64>,
    /// Minimum ceiling thickness in mm
    pub ceiling_thickness_min: Option<f64>,
    /// Maximum ceiling thickness in mm
    pub ceiling_thickness_max: Option<f64>,
    /// Covering of insulation allowed
    pub covering_insulation_allowed: Option<bool>,

    // ========================================================================
    // Maintenance
    // ========================================================================
    /// Overall light loss factor (0.0-1.0)
    pub light_loss_factor: Option<f64>,
    /// Lamp lumen depreciation factor (LLD)
    pub lamp_lumen_depreciation: Option<f64>,
    /// Luminaire dirt depreciation factor (LDD)
    pub luminaire_dirt_depreciation: Option<f64>,
    /// Lamp survival factor (LSF)
    pub lamp_survival_factor: Option<f64>,
    /// Burn-in time in hours
    pub burn_in_time: Option<f64>,
    /// LED module replaceable
    pub led_module_replaceable: Option<bool>,
    /// Light source included
    pub light_source_included: Option<bool>,
    /// Projected operational life in hours
    pub projected_life_hours: Option<u32>,

    // ========================================================================
    // Emergency
    // ========================================================================
    /// Emergency lighting capable
    pub emergency_capable: Option<bool>,
    /// Emergency lighting type
    pub emergency_type: Option<EmergencyType>,
    /// Emergency unit integrated
    pub emergency_unit_integrated: Option<bool>,
    /// Emergency mode luminous flux in lumens
    pub emergency_luminous_flux: Option<f64>,
    /// Emergency light source type
    pub emergency_light_source_type: Option<String>,
    /// Battery type
    pub battery_type: Option<String>,
    /// Battery capacity in Ah
    pub battery_capacity: Option<f64>,
    /// Battery voltage in V
    pub battery_voltage: Option<f64>,
    /// Battery exchange possible
    pub battery_exchange_possible: Option<bool>,

    // ========================================================================
    // Environmental
    // ========================================================================
    /// Ambient temperature range (min°C, max°C)
    pub ambient_temperature_range: Option<(f64, f64)>,
    /// Rated ambient temperature in °C
    pub rated_ambient_temperature: Option<f64>,
    /// Temperature on light aperture in °C
    pub temperature_on_aperture: Option<f64>,
    /// Relative humidity range (min%, max%)
    pub relative_humidity_range: Option<(f64, f64)>,
    /// Environmental Product Declaration available
    pub epd_available: Option<bool>,
    /// EPD URL
    pub epd_url: Option<String>,

    // ========================================================================
    // Sensor/Detection
    // ========================================================================
    /// Has integrated sensor
    pub with_sensor: Option<bool>,
    /// Detector type (e.g., "PIR", "Microwave")
    pub detector_type: Option<String>,
    /// Detection method
    pub detection_method: Option<String>,
    /// Detection area shape/description
    pub detection_area: Option<String>,
    /// Detection area adjustable
    pub detection_area_adjustable: Option<bool>,
    /// Detection range adjustable
    pub detection_range_adjustable: Option<bool>,
    /// Presence detection area in m²
    pub presence_detection_area: Option<f64>,
    /// Sensor radiation power in W
    pub radiation_power: Option<f64>,

    // ========================================================================
    // Acoustics
    // ========================================================================
    /// Average acoustic absorption (NRC value)
    pub acoustic_absorption_average: Option<f64>,
    /// Acoustic absorption by frequency (Hz, absorption)
    pub acoustic_absorption_table: Option<Vec<(f64, f64)>>,

    // ========================================================================
    // Accessories
    // ========================================================================
    /// Has integrated switch
    pub with_switch: Option<bool>,
    /// Has integrated dimmer
    pub with_dimmer: Option<bool>,
    /// Has power plug
    pub with_power_plug: Option<bool>,
    /// Has connecting cable
    pub with_connecting_cable: Option<bool>,
    /// Through wiring capability
    pub through_wiring: Option<bool>,
    /// Has starter (for fluorescent)
    pub with_starter: Option<bool>,
    /// Socket type (e.g., "E27", "GU10")
    pub socket_type: Option<String>,

    // ========================================================================
    // Documentation
    // ========================================================================
    /// Product documentation URL
    pub product_documentation_url: Option<String>,
    /// Product datasheet URL
    pub product_datasheet_url: Option<String>,

    // ========================================================================
    // Additional
    // ========================================================================
    /// Circuit number identifier
    pub circuit_number: Option<String>,
    /// Quick ship available
    pub quick_ship: Option<bool>,
    /// Quick ship terms
    pub quick_ship_terms: Option<String>,
}

impl BimParameters {
    /// Create a new empty BimParameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract BIM parameters from IES XML (TM-33) data
    ///
    /// This is the primary extraction method. TM-33 XML contains structured
    /// data that maps well to TM-32-24 parameters.
    pub fn from_atla(doc: &LuminaireOpticalData) -> Self {
        let mut params = BimParameters {
            manufacturer: doc.header.manufacturer.clone(),
            product_name: doc.header.description.clone(),
            catalog_number: doc.header.catalog_number.clone(),
            manufacturer_url: doc.header.more_info_uri.clone(),
            last_updated: doc.header.report_date.clone(),
            ..Default::default()
        };

        // Dimensions - from Luminaire
        if let Some(lum) = &doc.luminaire {
            if let Some(dims) = &lum.dimensions {
                params.product_length = Some(dims.length);
                params.product_width = Some(dims.width);
                params.product_height = Some(dims.height);
            }

            // Luminous opening dimensions
            if let Some(opening) = lum.luminous_openings.first() {
                params.luminous_length = Some(opening.dimensions.length);
                params.luminous_width = opening.dimensions.width;
                if opening.shape == crate::LuminousOpeningShape::Circular {
                    params.luminous_diameter = Some(opening.dimensions.length);
                }
            }

            // Mounting type
            if let Some(mounting) = &lum.mounting {
                params.mounting_type = MountingType::parse(mounting);
            }
        }

        // Emitter data - use first emitter for primary values
        if let Some(emitter) = doc.emitters.first() {
            // CCT
            params.cct_kelvin = emitter.cct.map(|c| c as u32);

            // CRI and TM-30
            if let Some(cr) = &emitter.color_rendering {
                params.cri = cr.ra.map(|r| r as u8);
                params.tm30_rf = cr.rf.map(|r| r as u8);
                params.tm30_rg = cr.rg.map(|r| r as u8);
            }

            // Power
            params.watts = emitter.input_watts;
            params.power_factor = emitter.power_factor;

            // Lamp quantity
            params.lamp_quantity = Some(emitter.quantity);
        }

        // Totals across all emitters
        let total_flux = doc.total_luminous_flux();
        if total_flux > 0.0 {
            params.total_luminous_flux = Some(total_flux);
        }

        let total_watts = doc.total_input_watts();
        if total_watts > 0.0 && params.watts.is_none() {
            params.watts = Some(total_watts);
        }

        // Calculate apparent power if we have watts and power factor
        if let (Some(watts), Some(pf)) = (params.watts, params.power_factor) {
            if pf > 0.0 {
                params.apparent_power_va = Some(watts / pf);
            }
        }

        // Extract BIM parameters from CustomData sections
        for item in &doc.custom_data_items {
            // Parse the raw content for BIM parameters
            extract_bim_from_custom_data(&mut params, &item.raw_content);
        }

        params
    }
}

/// Extract BIM parameters from CustomData raw content
fn extract_bim_from_custom_data(params: &mut BimParameters, content: &str) {
    // Simple XML tag extraction - look for common TM-32-24 fields
    // This handles formats like: <BUGRating guid="...">B2 U0 G2</BUGRating>

    // BUG Rating
    if let Some(value) = extract_xml_value(content, "BUGRating") {
        params.bug_rating = Some(value);
    }

    // Dimmable
    if let Some(value) = extract_xml_value(content, "Dimmable") {
        params.dimmable = match value.to_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        };
    }

    // Dimming method
    if let Some(value) = extract_xml_value(content, "DimmingMethod") {
        params.dimming_method = Some(value);
    }

    // Projected life hours
    if let Some(value) = extract_xml_value(content, "ProjectedLifeHours") {
        params.projected_life_hours = value.parse().ok();
    }

    // Voltage
    if let Some(value) = extract_xml_value(content, "VoltageNominal") {
        params.voltage_nominal = value.parse().ok();
    }

    // Housing material
    if let Some(value) = extract_xml_value(content, "HousingMaterial") {
        params.housing_material = Some(value);
    }

    // Housing color
    if let Some(value) = extract_xml_value(content, "HousingColor") {
        params.housing_color = Some(value);
    }

    // Weight
    if let Some(value) = extract_xml_value(content, "Weight") {
        params.weight = value.parse().ok();
    }

    // Emergency capable
    if let Some(value) = extract_xml_value(content, "EmergencyCapable") {
        params.emergency_capable = match value.to_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        };
    }

    // Emergency luminous flux
    if let Some(value) = extract_xml_value(content, "EmergencyLuminousFlux") {
        params.emergency_luminous_flux = value.parse().ok();
    }

    // Battery type
    if let Some(value) = extract_xml_value(content, "BatteryType") {
        params.battery_type = Some(value);
    }

    // Mounting type
    if let Some(value) = extract_xml_value(content, "MountingType") {
        params.mounting_type = MountingType::parse(&value);
    }

    // Control interface
    if let Some(value) = extract_xml_value(content, "ControlInterface") {
        params.control_interface = Some(value);
    }

    // Cutoff angle
    if let Some(value) = extract_xml_value(content, "CutoffAngle") {
        params.cutoff_angle = value.parse().ok();
    }
}

/// Extract value from an XML element tag
fn extract_xml_value(content: &str, tag: &str) -> Option<String> {
    // Match patterns like: <TagName>value</TagName> or <TagName attr="...">value</TagName>
    let open_pattern = format!("<{}", tag);

    if let Some(start_idx) = content.find(&open_pattern) {
        // Find the end of the opening tag
        let remaining = &content[start_idx..];
        if let Some(tag_end) = remaining.find('>') {
            let value_start = start_idx + tag_end + 1;

            // Find closing tag
            let close_pattern = format!("</{}>", tag);
            if let Some(close_idx) = content[value_start..].find(&close_pattern) {
                let value = content[value_start..value_start + close_idx].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

impl BimParameters {
    /// Check if any photometric parameters are set
    pub fn has_photometric_data(&self) -> bool {
        self.cct_kelvin.is_some()
            || self.cri.is_some()
            || self.total_luminous_flux.is_some()
            || self.bug_rating.is_some()
    }

    /// Check if any electrical parameters are set
    pub fn has_electrical_data(&self) -> bool {
        self.watts.is_some()
            || self.power_factor.is_some()
            || self.voltage_nominal.is_some()
            || self.dimmable.is_some()
    }

    /// Check if any dimensional parameters are set
    pub fn has_dimensional_data(&self) -> bool {
        self.product_length.is_some()
            || self.product_width.is_some()
            || self.product_height.is_some()
            || self.product_diameter.is_some()
    }

    /// Get a summary string of key parameters
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(name) = &self.product_name {
            parts.push(name.clone());
        }

        if let Some(cct) = self.cct_kelvin {
            parts.push(format!("{}K", cct));
        }

        if let Some(cri) = self.cri {
            parts.push(format!("CRI {}", cri));
        }

        if let Some(watts) = self.watts {
            parts.push(format!("{:.0}W", watts));
        }

        if let Some(flux) = self.total_luminous_flux {
            parts.push(format!("{:.0} lm", flux));
        }

        parts.join(" | ")
    }

    /// Format BIM parameters as a multi-line text report
    pub fn to_text_report(&self) -> String {
        let mut lines = Vec::new();

        lines.push("═══════════════════════════════════════════════════════════════".to_string());
        lines.push("                    TM-32-24 BIM PARAMETERS                    ".to_string());
        lines.push("═══════════════════════════════════════════════════════════════".to_string());

        // General
        if self.manufacturer.is_some()
            || self.product_name.is_some()
            || self.catalog_number.is_some()
        {
            lines.push(
                "\n┌─ GENERAL ─────────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = &self.manufacturer {
                lines.push(format!("│ Manufacturer:        {}", v));
            }
            if let Some(v) = &self.product_name {
                lines.push(format!("│ Product Name:        {}", v));
            }
            if let Some(v) = &self.catalog_number {
                lines.push(format!("│ Catalog Number:      {}", v));
            }
            if let Some(v) = &self.product_family {
                lines.push(format!("│ Product Family:      {}", v));
            }
            if let Some(v) = &self.description {
                lines.push(format!("│ Description:         {}", v));
            }
            if let Some(v) = &self.last_updated {
                lines.push(format!("│ Last Updated:        {}", v));
            }
        }

        // Photometric
        if self.has_photometric_data() {
            lines.push(
                "\n┌─ PHOTOMETRIC ──────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = self.cct_kelvin {
                lines.push(format!("│ CCT:                 {} K", v));
            }
            if let Some(v) = self.cri {
                lines.push(format!("│ CRI (Ra):            {}", v));
            }
            if let Some(rf) = self.tm30_rf {
                let rg_str = self
                    .tm30_rg
                    .map(|rg| format!(" / Rg {}", rg))
                    .unwrap_or_default();
                lines.push(format!("│ TM-30:               Rf {}{}", rf, rg_str));
            }
            if let Some(v) = self.total_luminous_flux {
                lines.push(format!("│ Luminous Flux:       {:.0} lm", v));
            }
            if let Some(v) = &self.bug_rating {
                lines.push(format!("│ BUG Rating:          {}", v));
            }
            if let Some(v) = self.cutoff_angle {
                lines.push(format!("│ Cut-off Angle:       {:.0}°", v));
            }
            if let Some(v) = self.melanopic_factor {
                lines.push(format!("│ Melanopic Factor:    {:.2}", v));
            }
        }

        // Electrical
        if self.has_electrical_data() {
            lines.push(
                "\n┌─ ELECTRICAL ───────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = self.watts {
                lines.push(format!("│ Power:               {:.1} W", v));
            }
            if let Some(v) = self.apparent_power_va {
                lines.push(format!("│ Apparent Power:      {:.1} VA", v));
            }
            if let Some(v) = self.power_factor {
                lines.push(format!("│ Power Factor:        {:.2}", v));
            }
            if let Some(v) = self.voltage_nominal {
                let vtype = self
                    .voltage_type
                    .map(|t| format!(" {}", t.as_str()))
                    .unwrap_or_default();
                lines.push(format!("│ Voltage:             {:.0} V{}", v, vtype));
            }
            if let Some(v) = &self.phase {
                lines.push(format!("│ Phase:               {}", v.as_str()));
            }
            if let Some(v) = self.dimmable {
                let method = self
                    .dimming_method
                    .as_ref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default();
                lines.push(format!(
                    "│ Dimmable:            {}{}",
                    if v { "Yes" } else { "No" },
                    method
                ));
            }
            if let Some(v) = &self.control_interface {
                lines.push(format!("│ Control Interface:   {}", v));
            }
            if let Some(v) = &self.light_source_type {
                lines.push(format!("│ Light Source:        {}", v));
            }
            if let Some(v) = self.lamp_quantity {
                lines.push(format!("│ Lamp Quantity:       {}", v));
            }
        }

        // Mechanical / Dimensions
        if self.has_dimensional_data() || self.weight.is_some() || self.housing_shape.is_some() {
            lines.push(
                "\n┌─ MECHANICAL ───────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = &self.housing_shape {
                lines.push(format!("│ Housing Shape:       {}", v.as_str()));
            }
            if let Some(v) = &self.housing_color {
                lines.push(format!("│ Housing Color:       {}", v));
            }
            if let Some(v) = &self.housing_material {
                lines.push(format!("│ Housing Material:    {}", v));
            }
            if self.product_length.is_some()
                || self.product_width.is_some()
                || self.product_height.is_some()
            {
                let l = self
                    .product_length
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or("-".to_string());
                let w = self
                    .product_width
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or("-".to_string());
                let h = self
                    .product_height
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or("-".to_string());
                lines.push(format!("│ Dimensions (LxWxH):  {} × {} × {} mm", l, w, h));
            }
            if let Some(v) = self.product_diameter {
                lines.push(format!("│ Diameter:            {:.0} mm", v));
            }
            if let Some(v) = self.weight {
                lines.push(format!("│ Weight:              {:.2} kg", v));
            }
        }

        // Mounting
        if self.mounting_type.is_some()
            || self.cutout_diameter.is_some()
            || self.recessed_depth.is_some()
        {
            lines.push(
                "\n┌─ MOUNTING ─────────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = &self.mounting_type {
                lines.push(format!("│ Mounting Type:       {}", v.as_str()));
            }
            if let Some(v) = self.cutout_diameter {
                lines.push(format!("│ Cutout Diameter:     {:.0} mm", v));
            }
            if self.cutout_length.is_some() || self.cutout_width.is_some() {
                let l = self
                    .cutout_length
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or("-".to_string());
                let w = self
                    .cutout_width
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or("-".to_string());
                lines.push(format!("│ Cutout (LxW):        {} × {} mm", l, w));
            }
            if let Some(v) = self.recessed_depth {
                lines.push(format!("│ Recessed Depth:      {:.0} mm", v));
            }
        }

        // Maintenance
        if self.light_loss_factor.is_some() || self.projected_life_hours.is_some() {
            lines.push(
                "\n┌─ MAINTENANCE ──────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = self.light_loss_factor {
                lines.push(format!("│ Light Loss Factor:   {:.2}", v));
            }
            if let Some(v) = self.lamp_lumen_depreciation {
                lines.push(format!("│ Lamp Lumen Depr.:    {:.2}", v));
            }
            if let Some(v) = self.luminaire_dirt_depreciation {
                lines.push(format!("│ Luminaire Dirt Depr.:{:.2}", v));
            }
            if let Some(v) = self.projected_life_hours {
                lines.push(format!("│ Projected Life:      {} hours", v));
            }
            if let Some(v) = self.led_module_replaceable {
                lines.push(format!(
                    "│ LED Replaceable:     {}",
                    if v { "Yes" } else { "No" }
                ));
            }
        }

        // Emergency
        if self.emergency_capable.is_some() {
            lines.push(
                "\n┌─ EMERGENCY ────────────────────────────────────────────────────".to_string(),
            );
            if let Some(v) = self.emergency_capable {
                lines.push(format!(
                    "│ Emergency Capable:   {}",
                    if v { "Yes" } else { "No" }
                ));
            }
            if let Some(v) = &self.emergency_type {
                lines.push(format!("│ Emergency Type:      {}", v.as_str()));
            }
            if let Some(v) = self.emergency_luminous_flux {
                lines.push(format!("│ Emergency Flux:      {:.0} lm", v));
            }
            if let Some(v) = &self.battery_type {
                lines.push(format!("│ Battery Type:        {}", v));
            }
            if let Some(v) = self.battery_capacity {
                lines.push(format!("│ Battery Capacity:    {:.1} Ah", v));
            }
        }

        // Environmental
        if self.ambient_temperature_range.is_some() || self.rated_ambient_temperature.is_some() {
            lines.push(
                "\n┌─ ENVIRONMENTAL ────────────────────────────────────────────────".to_string(),
            );
            if let Some((min, max)) = self.ambient_temperature_range {
                lines.push(format!(
                    "│ Ambient Temp Range:  {:.0}°C to {:.0}°C",
                    min, max
                ));
            }
            if let Some(v) = self.rated_ambient_temperature {
                lines.push(format!("│ Rated Ambient Temp:  {:.0}°C", v));
            }
        }

        lines.push("\n═══════════════════════════════════════════════════════════════".to_string());

        lines.join("\n")
    }

    /// Format BIM parameters as key-value pairs for table display
    /// Returns Vec of (group, key, value, unit) tuples
    pub fn to_table_rows(&self) -> Vec<(&'static str, &'static str, String, &'static str)> {
        let mut rows = Vec::new();

        // General
        if let Some(v) = &self.manufacturer {
            rows.push(("General", "Manufacturer", v.clone(), ""));
        }
        if let Some(v) = &self.product_name {
            rows.push(("General", "Product Name", v.clone(), ""));
        }
        if let Some(v) = &self.catalog_number {
            rows.push(("General", "Catalog Number", v.clone(), ""));
        }
        if let Some(v) = &self.product_family {
            rows.push(("General", "Product Family", v.clone(), ""));
        }

        // Photometric
        if let Some(v) = self.cct_kelvin {
            rows.push(("Photometric", "CCT", v.to_string(), "K"));
        }
        if let Some(v) = self.cri {
            rows.push(("Photometric", "CRI (Ra)", v.to_string(), ""));
        }
        if let Some(v) = self.tm30_rf {
            rows.push(("Photometric", "TM-30 Rf", v.to_string(), ""));
        }
        if let Some(v) = self.tm30_rg {
            rows.push(("Photometric", "TM-30 Rg", v.to_string(), ""));
        }
        if let Some(v) = self.total_luminous_flux {
            rows.push(("Photometric", "Luminous Flux", format!("{:.0}", v), "lm"));
        }
        if let Some(v) = &self.bug_rating {
            rows.push(("Photometric", "BUG Rating", v.clone(), ""));
        }
        if let Some(v) = self.cutoff_angle {
            rows.push(("Photometric", "Cut-off Angle", format!("{:.0}", v), "°"));
        }

        // Electrical
        if let Some(v) = self.watts {
            rows.push(("Electrical", "Power", format!("{:.1}", v), "W"));
        }
        if let Some(v) = self.apparent_power_va {
            rows.push(("Electrical", "Apparent Power", format!("{:.1}", v), "VA"));
        }
        if let Some(v) = self.power_factor {
            rows.push(("Electrical", "Power Factor", format!("{:.2}", v), ""));
        }
        if let Some(v) = self.voltage_nominal {
            rows.push(("Electrical", "Voltage", format!("{:.0}", v), "V"));
        }
        if let Some(v) = &self.voltage_type {
            rows.push(("Electrical", "Voltage Type", v.as_str().to_string(), ""));
        }
        if let Some(v) = &self.phase {
            rows.push(("Electrical", "Phase", v.as_str().to_string(), ""));
        }
        if let Some(v) = self.dimmable {
            rows.push((
                "Electrical",
                "Dimmable",
                (if v { "Yes" } else { "No" }).to_string(),
                "",
            ));
        }
        if let Some(v) = &self.dimming_method {
            rows.push(("Electrical", "Dimming Method", v.clone(), ""));
        }
        if let Some(v) = &self.control_interface {
            rows.push(("Electrical", "Control Interface", v.clone(), ""));
        }
        if let Some(v) = &self.light_source_type {
            rows.push(("Electrical", "Light Source", v.clone(), ""));
        }
        if let Some(v) = self.lamp_quantity {
            rows.push(("Electrical", "Lamp Quantity", v.to_string(), ""));
        }

        // Mechanical
        if let Some(v) = &self.housing_shape {
            rows.push(("Mechanical", "Housing Shape", v.as_str().to_string(), ""));
        }
        if let Some(v) = &self.housing_color {
            rows.push(("Mechanical", "Housing Color", v.clone(), ""));
        }
        if let Some(v) = &self.housing_material {
            rows.push(("Mechanical", "Housing Material", v.clone(), ""));
        }
        if let Some(v) = self.product_length {
            rows.push(("Mechanical", "Length", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.product_width {
            rows.push(("Mechanical", "Width", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.product_height {
            rows.push(("Mechanical", "Height", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.product_diameter {
            rows.push(("Mechanical", "Diameter", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.weight {
            rows.push(("Mechanical", "Weight", format!("{:.2}", v), "kg"));
        }

        // Mounting
        if let Some(v) = &self.mounting_type {
            rows.push(("Mounting", "Mounting Type", v.as_str().to_string(), ""));
        }
        if let Some(v) = self.cutout_diameter {
            rows.push(("Mounting", "Cutout Diameter", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.cutout_length {
            rows.push(("Mounting", "Cutout Length", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.cutout_width {
            rows.push(("Mounting", "Cutout Width", format!("{:.0}", v), "mm"));
        }
        if let Some(v) = self.recessed_depth {
            rows.push(("Mounting", "Recessed Depth", format!("{:.0}", v), "mm"));
        }

        // Maintenance
        if let Some(v) = self.light_loss_factor {
            rows.push(("Maintenance", "Light Loss Factor", format!("{:.2}", v), ""));
        }
        if let Some(v) = self.lamp_lumen_depreciation {
            rows.push(("Maintenance", "Lamp Lumen Depr.", format!("{:.2}", v), ""));
        }
        if let Some(v) = self.luminaire_dirt_depreciation {
            rows.push((
                "Maintenance",
                "Luminaire Dirt Depr.",
                format!("{:.2}", v),
                "",
            ));
        }
        if let Some(v) = self.projected_life_hours {
            rows.push(("Maintenance", "Projected Life", v.to_string(), "hours"));
        }
        if let Some(v) = self.led_module_replaceable {
            rows.push((
                "Maintenance",
                "LED Replaceable",
                (if v { "Yes" } else { "No" }).to_string(),
                "",
            ));
        }

        // Emergency
        if let Some(v) = self.emergency_capable {
            rows.push((
                "Emergency",
                "Emergency Capable",
                (if v { "Yes" } else { "No" }).to_string(),
                "",
            ));
        }
        if let Some(v) = &self.emergency_type {
            rows.push(("Emergency", "Emergency Type", v.as_str().to_string(), ""));
        }
        if let Some(v) = self.emergency_luminous_flux {
            rows.push(("Emergency", "Emergency Flux", format!("{:.0}", v), "lm"));
        }
        if let Some(v) = &self.battery_type {
            rows.push(("Emergency", "Battery Type", v.clone(), ""));
        }
        if let Some(v) = self.battery_capacity {
            rows.push(("Emergency", "Battery Capacity", format!("{:.1}", v), "Ah"));
        }

        // Environmental
        if let Some((min, max)) = self.ambient_temperature_range {
            rows.push((
                "Environmental",
                "Ambient Temp Range",
                format!("{:.0} to {:.0}", min, max),
                "°C",
            ));
        }
        if let Some(v) = self.rated_ambient_temperature {
            rows.push((
                "Environmental",
                "Rated Ambient Temp",
                format!("{:.0}", v),
                "°C",
            ));
        }

        rows
    }

    /// Export to CSV format
    pub fn to_csv(&self) -> String {
        let mut lines = vec!["Group,Parameter,Value,Unit".to_string()];

        for (group, key, value, unit) in self.to_table_rows() {
            // Escape values containing commas or quotes
            let escaped_value = if value.contains(',') || value.contains('"') {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value
            };
            lines.push(format!("{},{},{},{}", group, key, escaped_value, unit));
        }

        lines.join("\n")
    }

    /// Count how many parameters are populated
    pub fn populated_count(&self) -> usize {
        self.to_table_rows().len()
    }

    /// Get list of parameter groups that have data
    pub fn populated_groups(&self) -> Vec<&'static str> {
        let rows = self.to_table_rows();
        let mut groups: Vec<&'static str> = rows.iter().map(|(g, _, _, _)| *g).collect();
        groups.dedup();
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voltage_type_parse() {
        assert_eq!(VoltageType::parse("AC"), Some(VoltageType::AC));
        assert_eq!(VoltageType::parse("dc"), Some(VoltageType::DC));
        assert_eq!(VoltageType::parse("UC"), Some(VoltageType::UC));
        assert_eq!(VoltageType::parse("AC/DC"), Some(VoltageType::UC));
        assert_eq!(VoltageType::parse("invalid"), None);
    }

    #[test]
    fn test_mounting_type_parse() {
        assert_eq!(
            MountingType::parse("recessed"),
            Some(MountingType::Recessed)
        );
        assert_eq!(MountingType::parse("Pendant"), Some(MountingType::Pendant));
        assert_eq!(MountingType::parse("SURFACE"), Some(MountingType::Surface));
        assert_eq!(
            MountingType::parse("in-ground"),
            Some(MountingType::InGround)
        );
    }

    #[test]
    fn test_bim_parameters_default() {
        let params = BimParameters::new();
        assert!(params.manufacturer.is_none());
        assert!(params.cct_kelvin.is_none());
        assert!(!params.has_photometric_data());
        assert!(!params.has_electrical_data());
    }

    #[test]
    fn test_bim_summary() {
        let mut params = BimParameters::new();
        params.product_name = Some("Test Luminaire".to_string());
        params.cct_kelvin = Some(4000);
        params.cri = Some(90);
        params.watts = Some(50.0);
        params.total_luminous_flux = Some(5000.0);

        let summary = params.summary();
        assert!(summary.contains("Test Luminaire"));
        assert!(summary.contains("4000K"));
        assert!(summary.contains("CRI 90"));
        assert!(summary.contains("50W"));
        assert!(summary.contains("5000 lm"));
    }

    #[test]
    fn test_led_drive_type_parse() {
        assert_eq!(
            LedDriveType::parse("CC"),
            Some(LedDriveType::ConstantCurrent)
        );
        assert_eq!(
            LedDriveType::parse("constant voltage"),
            Some(LedDriveType::ConstantVoltage)
        );
        assert_eq!(
            LedDriveType::parse("N/A"),
            Some(LedDriveType::NotApplicable)
        );
    }

    #[test]
    fn test_bim_text_report() {
        let mut params = BimParameters::new();
        params.manufacturer = Some("Test Manufacturer".to_string());
        params.product_name = Some("LED Downlight".to_string());
        params.catalog_number = Some("DL-100".to_string());
        params.cct_kelvin = Some(4000);
        params.cri = Some(90);
        params.watts = Some(30.0);
        params.power_factor = Some(0.95);
        params.total_luminous_flux = Some(3000.0);
        params.product_length = Some(180.0);
        params.product_width = Some(180.0);
        params.product_height = Some(100.0);
        params.mounting_type = Some(MountingType::Recessed);

        let report = params.to_text_report();

        assert!(report.contains("TM-32-24 BIM PARAMETERS"));
        assert!(report.contains("Test Manufacturer"));
        assert!(report.contains("LED Downlight"));
        assert!(report.contains("4000 K"));
        assert!(report.contains("CRI (Ra):            90"));
        assert!(report.contains("30.0 W"));
        assert!(report.contains("3000 lm"));
        assert!(report.contains("Recessed"));
    }

    #[test]
    fn test_bim_table_rows() {
        let mut params = BimParameters::new();
        params.manufacturer = Some("Test".to_string());
        params.cct_kelvin = Some(4000);
        params.watts = Some(30.0);

        let rows = params.to_table_rows();

        assert!(rows.len() >= 3);
        assert!(rows
            .iter()
            .any(|(g, k, _, _)| *g == "General" && *k == "Manufacturer"));
        assert!(rows
            .iter()
            .any(|(g, k, _, u)| *g == "Photometric" && *k == "CCT" && *u == "K"));
        assert!(rows
            .iter()
            .any(|(g, k, _, u)| *g == "Electrical" && *k == "Power" && *u == "W"));
    }

    #[test]
    fn test_bim_csv_export() {
        let mut params = BimParameters::new();
        params.manufacturer = Some("Test, Inc.".to_string()); // Contains comma
        params.cct_kelvin = Some(4000);

        let csv = params.to_csv();

        assert!(csv.starts_with("Group,Parameter,Value,Unit"));
        assert!(csv.contains("\"Test, Inc.\"")); // Should be quoted
        assert!(csv.contains("Photometric,CCT,4000,K"));
    }

    #[test]
    fn test_bim_populated_count() {
        let mut params = BimParameters::new();
        assert_eq!(params.populated_count(), 0);

        params.manufacturer = Some("Test".to_string());
        params.cct_kelvin = Some(4000);
        params.watts = Some(30.0);

        assert_eq!(params.populated_count(), 3);
    }

    #[test]
    fn test_bim_populated_groups() {
        let mut params = BimParameters::new();
        params.manufacturer = Some("Test".to_string());
        params.cct_kelvin = Some(4000);
        params.watts = Some(30.0);
        params.mounting_type = Some(MountingType::Recessed);

        let groups = params.populated_groups();

        assert!(groups.contains(&"General"));
        assert!(groups.contains(&"Photometric"));
        assert!(groups.contains(&"Electrical"));
        assert!(groups.contains(&"Mounting"));
    }
}
