//! FFI-compatible types for Eulumdat data structures

use eulumdat::{
    Eulumdat as CoreEulumdat, LampSet as CoreLampSet, Symmetry as CoreSymmetry,
    TypeIndicator as CoreTypeIndicator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Symmetry {
    None,
    VerticalAxis,
    PlaneC0C180,
    PlaneC90C270,
    BothPlanes,
}

impl From<CoreSymmetry> for Symmetry {
    fn from(s: CoreSymmetry) -> Self {
        match s {
            CoreSymmetry::None => Symmetry::None,
            CoreSymmetry::VerticalAxis => Symmetry::VerticalAxis,
            CoreSymmetry::PlaneC0C180 => Symmetry::PlaneC0C180,
            CoreSymmetry::PlaneC90C270 => Symmetry::PlaneC90C270,
            CoreSymmetry::BothPlanes => Symmetry::BothPlanes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TypeIndicator {
    PointSourceSymmetric,
    Linear,
    PointSourceOther,
}

impl From<CoreTypeIndicator> for TypeIndicator {
    fn from(t: CoreTypeIndicator) -> Self {
        match t {
            CoreTypeIndicator::PointSourceSymmetric => TypeIndicator::PointSourceSymmetric,
            CoreTypeIndicator::Linear => TypeIndicator::Linear,
            CoreTypeIndicator::PointSourceOther => TypeIndicator::PointSourceOther,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LampSet {
    pub num_lamps: i32,
    pub lamp_type: String,
    pub total_luminous_flux: f64,
    pub color_appearance: String,
    pub color_rendering_group: String,
    pub wattage_with_ballast: f64,
}

impl From<&CoreLampSet> for LampSet {
    fn from(ls: &CoreLampSet) -> Self {
        Self {
            num_lamps: ls.num_lamps,
            lamp_type: ls.lamp_type.clone(),
            total_luminous_flux: ls.total_luminous_flux,
            color_appearance: ls.color_appearance.clone(),
            color_rendering_group: ls.color_rendering_group.clone(),
            wattage_with_ballast: ls.wattage_with_ballast,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Eulumdat {
    // Identification
    pub identification: String,
    pub type_indicator: TypeIndicator,
    pub symmetry: Symmetry,

    // Grid
    pub num_c_planes: u32,
    pub c_plane_distance: f64,
    pub num_g_planes: u32,
    pub g_plane_distance: f64,

    // Metadata
    pub measurement_report_number: String,
    pub luminaire_name: String,
    pub luminaire_number: String,
    pub file_name: String,
    pub date_user: String,

    // Physical dimensions (mm)
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub luminous_area_length: f64,
    pub luminous_area_width: f64,
    pub height_c0: f64,
    pub height_c90: f64,
    pub height_c180: f64,
    pub height_c270: f64,

    // Optical properties
    pub downward_flux_fraction: f64,
    pub light_output_ratio: f64,
    pub conversion_factor: f64,
    pub tilt_angle: f64,

    // Lamp configuration
    pub lamp_sets: Vec<LampSet>,

    // Utilization factors
    pub direct_ratios: Vec<f64>,

    // Photometric data
    pub c_angles: Vec<f64>,
    pub g_angles: Vec<f64>,
    pub intensities: Vec<Vec<f64>>,

    // Computed values
    pub max_intensity: f64,
    pub total_luminous_flux: f64,
}

impl From<&CoreEulumdat> for Eulumdat {
    fn from(ldt: &CoreEulumdat) -> Self {
        Self {
            identification: ldt.identification.clone(),
            type_indicator: ldt.type_indicator.into(),
            symmetry: ldt.symmetry.into(),
            num_c_planes: ldt.num_c_planes as u32,
            c_plane_distance: ldt.c_plane_distance,
            num_g_planes: ldt.num_g_planes as u32,
            g_plane_distance: ldt.g_plane_distance,
            measurement_report_number: ldt.measurement_report_number.clone(),
            luminaire_name: ldt.luminaire_name.clone(),
            luminaire_number: ldt.luminaire_number.clone(),
            file_name: ldt.file_name.clone(),
            date_user: ldt.date_user.clone(),
            length: ldt.length,
            width: ldt.width,
            height: ldt.height,
            luminous_area_length: ldt.luminous_area_length,
            luminous_area_width: ldt.luminous_area_width,
            height_c0: ldt.height_c0,
            height_c90: ldt.height_c90,
            height_c180: ldt.height_c180,
            height_c270: ldt.height_c270,
            downward_flux_fraction: ldt.downward_flux_fraction,
            light_output_ratio: ldt.light_output_ratio,
            conversion_factor: ldt.conversion_factor,
            tilt_angle: ldt.tilt_angle,
            lamp_sets: ldt.lamp_sets.iter().map(|ls| ls.into()).collect(),
            direct_ratios: ldt.direct_ratios.to_vec(),
            c_angles: ldt.c_angles.clone(),
            g_angles: ldt.g_angles.clone(),
            intensities: ldt.intensities.clone(),
            max_intensity: ldt.max_intensity(),
            total_luminous_flux: ldt.total_luminous_flux(),
        }
    }
}

/// Helper to convert FFI Eulumdat back to core type
pub fn to_core_eulumdat(ldt: &Eulumdat) -> CoreEulumdat {
    CoreEulumdat {
        identification: ldt.identification.clone(),
        type_indicator: match ldt.type_indicator {
            TypeIndicator::PointSourceSymmetric => CoreTypeIndicator::PointSourceSymmetric,
            TypeIndicator::Linear => CoreTypeIndicator::Linear,
            TypeIndicator::PointSourceOther => CoreTypeIndicator::PointSourceOther,
        },
        symmetry: match ldt.symmetry {
            Symmetry::None => CoreSymmetry::None,
            Symmetry::VerticalAxis => CoreSymmetry::VerticalAxis,
            Symmetry::PlaneC0C180 => CoreSymmetry::PlaneC0C180,
            Symmetry::PlaneC90C270 => CoreSymmetry::PlaneC90C270,
            Symmetry::BothPlanes => CoreSymmetry::BothPlanes,
        },
        num_c_planes: ldt.num_c_planes as usize,
        c_plane_distance: ldt.c_plane_distance,
        num_g_planes: ldt.num_g_planes as usize,
        g_plane_distance: ldt.g_plane_distance,
        measurement_report_number: ldt.measurement_report_number.clone(),
        luminaire_name: ldt.luminaire_name.clone(),
        luminaire_number: ldt.luminaire_number.clone(),
        file_name: ldt.file_name.clone(),
        date_user: ldt.date_user.clone(),
        length: ldt.length,
        width: ldt.width,
        height: ldt.height,
        luminous_area_length: ldt.luminous_area_length,
        luminous_area_width: ldt.luminous_area_width,
        height_c0: ldt.height_c0,
        height_c90: ldt.height_c90,
        height_c180: ldt.height_c180,
        height_c270: ldt.height_c270,
        downward_flux_fraction: ldt.downward_flux_fraction,
        light_output_ratio: ldt.light_output_ratio,
        conversion_factor: ldt.conversion_factor,
        tilt_angle: ldt.tilt_angle,
        lamp_sets: ldt
            .lamp_sets
            .iter()
            .map(|ls| CoreLampSet {
                num_lamps: ls.num_lamps,
                lamp_type: ls.lamp_type.clone(),
                total_luminous_flux: ls.total_luminous_flux,
                color_appearance: ls.color_appearance.clone(),
                color_rendering_group: ls.color_rendering_group.clone(),
                wattage_with_ballast: ls.wattage_with_ballast,
            })
            .collect(),
        direct_ratios: ldt.direct_ratios.clone().try_into().unwrap_or([0.0; 10]),
        c_angles: ldt.c_angles.clone(),
        g_angles: ldt.g_angles.clone(),
        intensities: ldt.intensities.clone(),
    }
}

/// Parse LDT content and return an Eulumdat object
#[uniffi::export]
pub fn parse_ldt(content: String) -> Result<Eulumdat, crate::error::EulumdatError> {
    CoreEulumdat::parse(&content)
        .map(|ldt| (&ldt).into())
        .map_err(|e| crate::error::EulumdatError::ParseError(e.to_string()))
}

/// Parse IES content and return an Eulumdat object
#[uniffi::export]
pub fn parse_ies(content: String) -> Result<Eulumdat, crate::error::EulumdatError> {
    eulumdat::IesParser::parse(&content)
        .map(|ldt| (&ldt).into())
        .map_err(|e| crate::error::EulumdatError::ParseError(e.to_string()))
}

/// Export Eulumdat data to LDT format string
#[uniffi::export]
pub fn export_ldt(ldt: &Eulumdat) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    core_ldt.to_ldt()
}

/// Export Eulumdat data to IES format string
#[uniffi::export]
pub fn export_ies(ldt: &Eulumdat) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    eulumdat::IesExporter::export(&core_ldt)
}
