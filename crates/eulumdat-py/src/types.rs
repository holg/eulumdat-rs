//! Core types for Python bindings

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use ::eulumdat as core;
use core::{
    diagram::{
        ButterflyDiagram as CoreButterflyDiagram, CartesianDiagram as CoreCartesianDiagram,
        ConeDiagram as CoreConeDiagram, HeatmapDiagram as CoreHeatmapDiagram,
        PolarDiagram as CorePolarDiagram,
    },
    BugDiagram as CoreBugDiagram, IesExporter, IesParser, PhotometricCalculations,
};

use crate::{
    bug_rating::{BugRating, ZoneLumens},
    calculations::{
        CieFluxCodes, GldfPhotometricData, PhotometricCalcs, PhotometricSummary, UgrParams,
        ZonalLumens30,
    },
    diagram::SvgTheme,
    error::to_py_err,
    validation::ValidationWarning,
};

/// Type indicator for the luminaire.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeIndicator {
    /// Point source with symmetry about the vertical axis (Ityp = 1)
    PointSourceSymmetric = 1,
    /// Linear luminaire (Ityp = 2)
    Linear = 2,
    /// Point source with any other symmetry (Ityp = 3)
    PointSourceOther = 3,
}

#[pymethods]
impl TypeIndicator {
    /// Create from integer value (1-3).
    #[staticmethod]
    fn from_int(value: i32) -> PyResult<Self> {
        match value {
            1 => Ok(Self::PointSourceSymmetric),
            2 => Ok(Self::Linear),
            3 => Ok(Self::PointSourceOther),
            _ => Err(PyValueError::new_err(format!(
                "Invalid type indicator: {} (must be 1-3)",
                value
            ))),
        }
    }

    /// Convert to integer value.
    fn as_int(&self) -> i32 {
        *self as i32
    }

    fn __repr__(&self) -> String {
        match self {
            Self::PointSourceSymmetric => "TypeIndicator.PointSourceSymmetric".to_string(),
            Self::Linear => "TypeIndicator.Linear".to_string(),
            Self::PointSourceOther => "TypeIndicator.PointSourceOther".to_string(),
        }
    }
}

impl From<core::TypeIndicator> for TypeIndicator {
    fn from(t: core::TypeIndicator) -> Self {
        match t {
            core::TypeIndicator::PointSourceSymmetric => Self::PointSourceSymmetric,
            core::TypeIndicator::Linear => Self::Linear,
            core::TypeIndicator::PointSourceOther => Self::PointSourceOther,
        }
    }
}

impl From<TypeIndicator> for core::TypeIndicator {
    fn from(t: TypeIndicator) -> Self {
        match t {
            TypeIndicator::PointSourceSymmetric => Self::PointSourceSymmetric,
            TypeIndicator::Linear => Self::Linear,
            TypeIndicator::PointSourceOther => Self::PointSourceOther,
        }
    }
}

/// Symmetry indicator for the luminaire.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Symmetry {
    /// No symmetry (Isym = 0) - full 360° data required
    None = 0,
    /// Symmetry about the vertical axis (Isym = 1) - only 1 C-plane needed
    VerticalAxis = 1,
    /// Symmetry to plane C0-C180 (Isym = 2) - half the C-planes needed
    PlaneC0C180 = 2,
    /// Symmetry to plane C90-C270 (Isym = 3) - half the C-planes needed
    PlaneC90C270 = 3,
    /// Symmetry to both planes (Isym = 4) - quarter C-planes needed
    BothPlanes = 4,
}

#[pymethods]
impl Symmetry {
    /// Create from integer value (0-4).
    #[staticmethod]
    fn from_int(value: i32) -> PyResult<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::VerticalAxis),
            2 => Ok(Self::PlaneC0C180),
            3 => Ok(Self::PlaneC90C270),
            4 => Ok(Self::BothPlanes),
            _ => Err(PyValueError::new_err(format!(
                "Invalid symmetry: {} (must be 0-4)",
                value
            ))),
        }
    }

    /// Convert to integer value.
    fn as_int(&self) -> i32 {
        *self as i32
    }

    /// Get human-readable description.
    fn description(&self) -> &'static str {
        match self {
            Self::None => "no symmetry",
            Self::VerticalAxis => "symmetry about the vertical axis",
            Self::PlaneC0C180 => "symmetry to plane C0-C180",
            Self::PlaneC90C270 => "symmetry to plane C90-C270",
            Self::BothPlanes => "symmetry to plane C0-C180 and to plane C90-C270",
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Self::None => "Symmetry.None".to_string(),
            Self::VerticalAxis => "Symmetry.VerticalAxis".to_string(),
            Self::PlaneC0C180 => "Symmetry.PlaneC0C180".to_string(),
            Self::PlaneC90C270 => "Symmetry.PlaneC90C270".to_string(),
            Self::BothPlanes => "Symmetry.BothPlanes".to_string(),
        }
    }
}

impl From<core::Symmetry> for Symmetry {
    fn from(s: core::Symmetry) -> Self {
        match s {
            core::Symmetry::None => Self::None,
            core::Symmetry::VerticalAxis => Self::VerticalAxis,
            core::Symmetry::PlaneC0C180 => Self::PlaneC0C180,
            core::Symmetry::PlaneC90C270 => Self::PlaneC90C270,
            core::Symmetry::BothPlanes => Self::BothPlanes,
        }
    }
}

impl From<Symmetry> for core::Symmetry {
    fn from(s: Symmetry) -> Self {
        match s {
            Symmetry::None => Self::None,
            Symmetry::VerticalAxis => Self::VerticalAxis,
            Symmetry::PlaneC0C180 => Self::PlaneC0C180,
            Symmetry::PlaneC90C270 => Self::PlaneC90C270,
            Symmetry::BothPlanes => Self::BothPlanes,
        }
    }
}

/// Lamp set configuration.
#[pyclass]
#[derive(Clone)]
pub struct LampSet {
    /// Number of lamps in this set.
    #[pyo3(get, set)]
    pub num_lamps: i32,
    /// Type of lamps (description string).
    #[pyo3(get, set)]
    pub lamp_type: String,
    /// Total luminous flux of this lamp set in lumens.
    #[pyo3(get, set)]
    pub total_luminous_flux: f64,
    /// Color appearance / color temperature.
    #[pyo3(get, set)]
    pub color_appearance: String,
    /// Color rendering group / CRI.
    #[pyo3(get, set)]
    pub color_rendering_group: String,
    /// Wattage including ballast in watts.
    #[pyo3(get, set)]
    pub wattage_with_ballast: f64,
}

#[pymethods]
impl LampSet {
    #[new]
    #[pyo3(signature = (num_lamps=1, lamp_type="".to_string(), total_luminous_flux=0.0, color_appearance="".to_string(), color_rendering_group="".to_string(), wattage_with_ballast=0.0))]
    fn new(
        num_lamps: i32,
        lamp_type: String,
        total_luminous_flux: f64,
        color_appearance: String,
        color_rendering_group: String,
        wattage_with_ballast: f64,
    ) -> Self {
        Self {
            num_lamps,
            lamp_type,
            total_luminous_flux,
            color_appearance,
            color_rendering_group,
            wattage_with_ballast,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LampSet(num_lamps={}, lamp_type='{}', flux={:.1} lm, wattage={:.1} W)",
            self.num_lamps, self.lamp_type, self.total_luminous_flux, self.wattage_with_ballast
        )
    }
}

impl From<&core::LampSet> for LampSet {
    fn from(ls: &core::LampSet) -> Self {
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

impl From<&LampSet> for core::LampSet {
    fn from(ls: &LampSet) -> Self {
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

/// Main EULUMDAT data structure.
///
/// This class contains all data from an EULUMDAT (LDT) file.
#[pyclass]
pub struct Eulumdat {
    inner: core::Eulumdat,
}

#[pymethods]
impl Eulumdat {
    /// Create a new empty Eulumdat structure.
    #[new]
    fn new() -> Self {
        Self {
            inner: core::Eulumdat::new(),
        }
    }

    /// Parse from a string containing LDT data.
    #[staticmethod]
    fn parse(content: &str) -> PyResult<Self> {
        core::Eulumdat::parse(content)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Load from a file path.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        core::Eulumdat::from_file(path)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Parse from IES format string.
    #[staticmethod]
    fn parse_ies(content: &str) -> PyResult<Self> {
        IesParser::parse(content)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Load from an IES file path.
    #[staticmethod]
    fn from_ies_file(path: &str) -> PyResult<Self> {
        IesParser::parse_file(path)
            .map(|inner| Self { inner })
            .map_err(to_py_err)
    }

    /// Convert to LDT format string.
    fn to_ldt(&self) -> String {
        self.inner.to_ldt()
    }

    /// Export to IES format string.
    fn to_ies(&self) -> String {
        IesExporter::export(&self.inner)
    }

    /// Save to a file path.
    fn save(&self, path: &str) -> PyResult<()> {
        self.inner.save(path).map_err(to_py_err)
    }

    /// Validate the data and return any warnings.
    fn validate(&self) -> Vec<ValidationWarning> {
        self.inner
            .validate()
            .into_iter()
            .map(|w| ValidationWarning {
                code: w.code.to_string(),
                message: w.message,
            })
            .collect()
    }

    // === Identification ===

    /// Identification string.
    #[getter]
    fn identification(&self) -> &str {
        &self.inner.identification
    }

    #[setter]
    fn set_identification(&mut self, value: String) {
        self.inner.identification = value;
    }

    // === Type and Symmetry ===

    /// Type indicator.
    #[getter]
    fn type_indicator(&self) -> TypeIndicator {
        self.inner.type_indicator.into()
    }

    #[setter]
    fn set_type_indicator(&mut self, value: TypeIndicator) {
        self.inner.type_indicator = value.into();
    }

    /// Symmetry indicator.
    #[getter]
    fn symmetry(&self) -> Symmetry {
        self.inner.symmetry.into()
    }

    #[setter]
    fn set_symmetry(&mut self, value: Symmetry) {
        self.inner.symmetry = value.into();
    }

    // === Grid Definition ===

    /// Number of C-planes.
    #[getter]
    fn num_c_planes(&self) -> usize {
        self.inner.num_c_planes
    }

    #[setter]
    fn set_num_c_planes(&mut self, value: usize) {
        self.inner.num_c_planes = value;
    }

    /// Distance between C-planes in degrees.
    #[getter]
    fn c_plane_distance(&self) -> f64 {
        self.inner.c_plane_distance
    }

    #[setter]
    fn set_c_plane_distance(&mut self, value: f64) {
        self.inner.c_plane_distance = value;
    }

    /// Number of gamma angles.
    #[getter]
    fn num_g_planes(&self) -> usize {
        self.inner.num_g_planes
    }

    #[setter]
    fn set_num_g_planes(&mut self, value: usize) {
        self.inner.num_g_planes = value;
    }

    /// Distance between gamma angles in degrees.
    #[getter]
    fn g_plane_distance(&self) -> f64 {
        self.inner.g_plane_distance
    }

    #[setter]
    fn set_g_plane_distance(&mut self, value: f64) {
        self.inner.g_plane_distance = value;
    }

    // === Metadata ===

    /// Measurement report number.
    #[getter]
    fn measurement_report_number(&self) -> &str {
        &self.inner.measurement_report_number
    }

    #[setter]
    fn set_measurement_report_number(&mut self, value: String) {
        self.inner.measurement_report_number = value;
    }

    /// Luminaire name.
    #[getter]
    fn luminaire_name(&self) -> &str {
        &self.inner.luminaire_name
    }

    #[setter]
    fn set_luminaire_name(&mut self, value: String) {
        self.inner.luminaire_name = value;
    }

    /// Luminaire number.
    #[getter]
    fn luminaire_number(&self) -> &str {
        &self.inner.luminaire_number
    }

    #[setter]
    fn set_luminaire_number(&mut self, value: String) {
        self.inner.luminaire_number = value;
    }

    /// File name.
    #[getter]
    fn file_name(&self) -> &str {
        &self.inner.file_name
    }

    #[setter]
    fn set_file_name(&mut self, value: String) {
        self.inner.file_name = value;
    }

    /// Date/user field.
    #[getter]
    fn date_user(&self) -> &str {
        &self.inner.date_user
    }

    #[setter]
    fn set_date_user(&mut self, value: String) {
        self.inner.date_user = value;
    }

    // === Physical Dimensions (in mm) ===

    /// Length/diameter of luminaire (mm).
    #[getter]
    fn length(&self) -> f64 {
        self.inner.length
    }

    #[setter]
    fn set_length(&mut self, value: f64) {
        self.inner.length = value;
    }

    /// Width of luminaire (mm), 0 for circular.
    #[getter]
    fn width(&self) -> f64 {
        self.inner.width
    }

    #[setter]
    fn set_width(&mut self, value: f64) {
        self.inner.width = value;
    }

    /// Height of luminaire (mm).
    #[getter]
    fn height(&self) -> f64 {
        self.inner.height
    }

    #[setter]
    fn set_height(&mut self, value: f64) {
        self.inner.height = value;
    }

    /// Length/diameter of luminous area (mm).
    #[getter]
    fn luminous_area_length(&self) -> f64 {
        self.inner.luminous_area_length
    }

    #[setter]
    fn set_luminous_area_length(&mut self, value: f64) {
        self.inner.luminous_area_length = value;
    }

    /// Width of luminous area (mm), 0 for circular.
    #[getter]
    fn luminous_area_width(&self) -> f64 {
        self.inner.luminous_area_width
    }

    #[setter]
    fn set_luminous_area_width(&mut self, value: f64) {
        self.inner.luminous_area_width = value;
    }

    /// Height of luminous area at C0 plane (mm).
    #[getter]
    fn height_c0(&self) -> f64 {
        self.inner.height_c0
    }

    #[setter]
    fn set_height_c0(&mut self, value: f64) {
        self.inner.height_c0 = value;
    }

    /// Height of luminous area at C90 plane (mm).
    #[getter]
    fn height_c90(&self) -> f64 {
        self.inner.height_c90
    }

    #[setter]
    fn set_height_c90(&mut self, value: f64) {
        self.inner.height_c90 = value;
    }

    /// Height of luminous area at C180 plane (mm).
    #[getter]
    fn height_c180(&self) -> f64 {
        self.inner.height_c180
    }

    #[setter]
    fn set_height_c180(&mut self, value: f64) {
        self.inner.height_c180 = value;
    }

    /// Height of luminous area at C270 plane (mm).
    #[getter]
    fn height_c270(&self) -> f64 {
        self.inner.height_c270
    }

    #[setter]
    fn set_height_c270(&mut self, value: f64) {
        self.inner.height_c270 = value;
    }

    // === Optical Properties ===

    /// Downward flux fraction (DFF) in percent.
    #[getter]
    fn downward_flux_fraction(&self) -> f64 {
        self.inner.downward_flux_fraction
    }

    #[setter]
    fn set_downward_flux_fraction(&mut self, value: f64) {
        self.inner.downward_flux_fraction = value;
    }

    /// Light output ratio of luminaire (LORL) in percent.
    #[getter]
    fn light_output_ratio(&self) -> f64 {
        self.inner.light_output_ratio
    }

    #[setter]
    fn set_light_output_ratio(&mut self, value: f64) {
        self.inner.light_output_ratio = value;
    }

    /// Conversion factor for luminous intensities.
    #[getter]
    fn conversion_factor(&self) -> f64 {
        self.inner.conversion_factor
    }

    #[setter]
    fn set_conversion_factor(&mut self, value: f64) {
        self.inner.conversion_factor = value;
    }

    /// Tilt angle during measurement in degrees.
    #[getter]
    fn tilt_angle(&self) -> f64 {
        self.inner.tilt_angle
    }

    #[setter]
    fn set_tilt_angle(&mut self, value: f64) {
        self.inner.tilt_angle = value;
    }

    // === Lamp Configuration ===

    /// Lamp sets.
    #[getter]
    fn lamp_sets(&self) -> Vec<LampSet> {
        self.inner.lamp_sets.iter().map(LampSet::from).collect()
    }

    #[setter]
    fn set_lamp_sets(&mut self, value: Vec<LampSet>) {
        self.inner.lamp_sets = value.iter().map(core::LampSet::from).collect();
    }

    // === Utilization Factors ===

    /// Direct ratios for room indices.
    #[getter]
    fn direct_ratios(&self) -> Vec<f64> {
        self.inner.direct_ratios.to_vec()
    }

    #[setter]
    fn set_direct_ratios(&mut self, value: Vec<f64>) -> PyResult<()> {
        if value.len() != 10 {
            return Err(PyValueError::new_err(
                "direct_ratios must have exactly 10 values",
            ));
        }
        self.inner.direct_ratios.copy_from_slice(&value);
        Ok(())
    }

    // === Photometric Data ===

    /// C-plane angles in degrees.
    #[getter]
    fn c_angles(&self) -> Vec<f64> {
        self.inner.c_angles.clone()
    }

    #[setter]
    fn set_c_angles(&mut self, value: Vec<f64>) {
        self.inner.c_angles = value;
    }

    /// G-plane (gamma) angles in degrees.
    #[getter]
    fn g_angles(&self) -> Vec<f64> {
        self.inner.g_angles.clone()
    }

    #[setter]
    fn set_g_angles(&mut self, value: Vec<f64>) {
        self.inner.g_angles = value;
    }

    /// Luminous intensity distribution in cd/klm.
    /// Indexed as intensities[c_plane_index][g_plane_index].
    #[getter]
    fn intensities(&self) -> Vec<Vec<f64>> {
        self.inner.intensities.clone()
    }

    #[setter]
    fn set_intensities(&mut self, value: Vec<Vec<f64>>) {
        self.inner.intensities = value;
    }

    // === Computed Properties ===

    /// Get the actual number of C-planes based on symmetry.
    fn actual_c_planes(&self) -> usize {
        self.inner.actual_c_planes()
    }

    /// Get total luminous flux from all lamp sets.
    fn total_luminous_flux(&self) -> f64 {
        self.inner.total_luminous_flux()
    }

    /// Get total wattage from all lamp sets.
    fn total_wattage(&self) -> f64 {
        self.inner.total_wattage()
    }

    /// Get luminous efficacy in lm/W.
    fn luminous_efficacy(&self) -> f64 {
        self.inner.luminous_efficacy()
    }

    /// Get the maximum intensity value.
    fn max_intensity(&self) -> f64 {
        self.inner.max_intensity()
    }

    /// Get the minimum intensity value.
    fn min_intensity(&self) -> f64 {
        self.inner.min_intensity()
    }

    /// Get the average intensity value.
    fn avg_intensity(&self) -> f64 {
        self.inner.avg_intensity()
    }

    /// Get intensity at a specific C and G angle index.
    fn get_intensity(&self, c_index: usize, g_index: usize) -> Option<f64> {
        self.inner.get_intensity(c_index, g_index)
    }

    /// Sample intensity at any C and G angle using bilinear interpolation.
    ///
    /// This method handles symmetry automatically - you can query any angle
    /// in the full 0-360° C range and 0-180° G range regardless of stored symmetry.
    ///
    /// Args:
    ///     c_angle: C-plane angle in degrees (will be normalized to 0-360)
    ///     g_angle: Gamma angle in degrees (will be clamped to 0-180)
    ///
    /// Returns:
    ///     Intensity in cd/klm at the specified angle
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
        self.inner.sample(c_angle, g_angle)
    }

    /// Sample normalized intensity (0.0 to 1.0) at any C and G angle.
    ///
    /// Returns intensity relative to maximum intensity, useful for visualization.
    ///
    /// Args:
    ///     c_angle: C-plane angle in degrees
    ///     g_angle: Gamma angle in degrees
    ///
    /// Returns:
    ///     Normalized intensity (0.0 to 1.0)
    fn sample_normalized(&self, c_angle: f64, g_angle: f64) -> f64 {
        let max = self.inner.max_intensity();
        if max <= 0.0 {
            return 0.0;
        }
        self.inner.sample(c_angle, g_angle) / max
    }

    // === Diagram Generation ===

    /// Generate a polar diagram SVG.
    #[pyo3(signature = (width=500.0, height=500.0, theme=SvgTheme::Light))]
    fn polar_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CorePolarDiagram::from_eulumdat(&self.inner);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a butterfly diagram SVG.
    #[pyo3(signature = (width=500.0, height=400.0, rotation=60.0, theme=SvgTheme::Light))]
    fn butterfly_svg(&self, width: f64, height: f64, rotation: f64, theme: SvgTheme) -> String {
        let diagram = CoreButterflyDiagram::from_eulumdat(&self.inner, width, height, rotation);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a cartesian diagram SVG.
    #[pyo3(signature = (width=600.0, height=400.0, max_curves=8, theme=SvgTheme::Light))]
    fn cartesian_svg(&self, width: f64, height: f64, max_curves: usize, theme: SvgTheme) -> String {
        let diagram = CoreCartesianDiagram::from_eulumdat(&self.inner, width, height, max_curves);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a heatmap diagram SVG.
    #[pyo3(signature = (width=700.0, height=500.0, theme=SvgTheme::Light))]
    fn heatmap_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CoreHeatmapDiagram::from_eulumdat(&self.inner, width, height);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a cone diagram SVG showing beam and field angle spread at a mounting height.
    ///
    /// Args:
    ///     width: SVG width in pixels
    ///     height: SVG height in pixels
    ///     mounting_height: Mounting height in meters
    ///     theme: SVG color theme
    ///
    /// Returns:
    ///     SVG string
    #[pyo3(signature = (width=600.0, height=450.0, mounting_height=3.0, theme=SvgTheme::Light))]
    fn cone_svg(&self, width: f64, height: f64, mounting_height: f64, theme: SvgTheme) -> String {
        let diagram = CoreConeDiagram::from_eulumdat(&self.inner, mounting_height);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a beam angle diagram SVG comparing IES and CIE definitions.
    ///
    /// Shows 50% (beam) and 10% (field) intensity angles with annotations
    /// explaining the differences between IES and CIE standards.
    /// For batwing distributions, shows both main and secondary peaks.
    ///
    /// Args:
    ///     width: SVG width in pixels
    ///     height: SVG height in pixels
    ///     theme: SVG color theme
    ///
    /// Returns:
    ///     SVG string
    #[pyo3(signature = (width=600.0, height=600.0, theme=SvgTheme::Light))]
    fn beam_angle_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CorePolarDiagram::from_eulumdat(&self.inner);
        let analysis = PhotometricCalculations::beam_field_analysis(&self.inner);
        let show_both = analysis.is_batwing;
        diagram.to_svg_with_beam_field_angles(width, height, &theme.to_core(), &analysis, show_both)
    }

    /// Generate a BUG rating diagram SVG.
    #[pyo3(signature = (width=400.0, height=350.0, theme=SvgTheme::Light))]
    fn bug_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CoreBugDiagram::from_eulumdat(&self.inner);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate a LCS (Luminaire Classification System) diagram SVG.
    #[pyo3(signature = (width=510.0, height=315.0, theme=SvgTheme::Light))]
    fn lcs_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CoreBugDiagram::from_eulumdat(&self.inner);
        diagram.to_lcs_svg(width, height, &theme.to_core())
    }

    /// Calculate BUG rating.
    fn bug_rating(&self) -> BugRating {
        let diagram = CoreBugDiagram::from_eulumdat(&self.inner);
        BugRating {
            b: diagram.rating.b,
            u: diagram.rating.u,
            g: diagram.rating.g,
        }
    }

    /// Calculate zone lumens for BUG rating.
    fn zone_lumens(&self) -> ZoneLumens {
        let diagram = CoreBugDiagram::from_eulumdat(&self.inner);
        ZoneLumens {
            bl: diagram.zones.bl,
            bm: diagram.zones.bm,
            bh: diagram.zones.bh,
            bvh: diagram.zones.bvh,
            fl: diagram.zones.fl,
            fm: diagram.zones.fm,
            fh: diagram.zones.fh,
            fvh: diagram.zones.fvh,
            ul: diagram.zones.ul,
            uh: diagram.zones.uh,
        }
    }

    /// Generate a BUG diagram SVG with detailed zone lumens breakdown.
    #[pyo3(signature = (width=600.0, height=400.0, theme=SvgTheme::Light))]
    fn bug_svg_with_details(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let diagram = CoreBugDiagram::from_eulumdat(&self.inner);
        diagram.to_svg_with_details(width, height, &theme.to_core())
    }

    // === Photometric Calculations ===

    /// Calculate complete photometric summary.
    ///
    /// Returns a PhotometricSummary containing all calculated photometric values
    /// including CIE flux codes, beam/field angles, spacing criteria, etc.
    fn photometric_summary(&self) -> PhotometricSummary {
        PhotometricCalcs::photometric_summary(&self.inner)
    }

    /// Calculate GLDF-compatible photometric data.
    ///
    /// Returns data structured for GLDF (Global Lighting Data Format) export.
    fn gldf_data(&self) -> GldfPhotometricData {
        PhotometricCalcs::gldf_data(&self.inner)
    }

    /// Calculate CIE flux codes (N1-N5).
    ///
    /// Returns:
    ///     CieFluxCodes with N1 (DLOR), N2 (0-60°), N3 (0-40°), N4 (ULOR), N5 (90-120°)
    fn cie_flux_codes(&self) -> CieFluxCodes {
        PhotometricCalcs::cie_flux_codes(&self.inner)
    }

    /// Calculate beam angle (50% intensity drop).
    ///
    /// Returns:
    ///     Beam angle in degrees
    fn beam_angle(&self) -> f64 {
        PhotometricCalcs::beam_angle(&self.inner)
    }

    /// Calculate field angle (10% intensity drop).
    ///
    /// Returns:
    ///     Field angle in degrees
    fn field_angle(&self) -> f64 {
        PhotometricCalcs::field_angle(&self.inner)
    }

    /// Calculate spacing criteria (S/H ratios) for both principal planes.
    ///
    /// Returns:
    ///     Tuple of (S/H for C0 plane, S/H for C90 plane)
    fn spacing_criteria(&self) -> (f64, f64) {
        PhotometricCalcs::spacing_criteria(&self.inner)
    }

    /// Calculate zonal lumens in 30° zones.
    ///
    /// Returns:
    ///     ZonalLumens30 with flux percentages in 6 zones from nadir to zenith
    fn zonal_lumens_30(&self) -> ZonalLumens30 {
        PhotometricCalcs::zonal_lumens_30(&self.inner)
    }

    /// Calculate downward flux fraction up to a given arc angle.
    ///
    /// Args:
    ///     arc: Maximum angle from vertical (0° = straight down, 90° = horizontal)
    ///
    /// Returns:
    ///     Percentage of light directed downward (0-100)
    fn downward_flux(&self, arc: f64) -> f64 {
        PhotometricCalcs::downward_flux(&self.inner, arc)
    }

    /// Calculate cut-off angle (where intensity drops below 2.5% of max).
    ///
    /// Returns:
    ///     Cut-off angle in degrees
    fn cut_off_angle(&self) -> f64 {
        PhotometricCalcs::cut_off_angle(&self.inner)
    }

    /// Generate photometric classification code.
    ///
    /// Format: D-N where D=distribution type, N=beam classification
    /// Distribution: D (direct), SD (semi-direct), GD (general diffuse),
    ///               SI (semi-indirect), I (indirect)
    /// Beam: VN (very narrow), N (narrow), M (medium), W (wide), VW (very wide)
    ///
    /// Returns:
    ///     Classification code string (e.g., "D-M" for direct medium beam)
    fn photometric_code(&self) -> String {
        PhotometricCalcs::photometric_code(&self.inner)
    }

    /// Calculate luminaire efficacy (accounting for LOR).
    ///
    /// Differs from lamp efficacy by including light output ratio losses.
    ///
    /// Returns:
    ///     Luminaire efficacy in lm/W
    fn luminaire_efficacy_lor(&self) -> f64 {
        PhotometricCalcs::luminaire_efficacy(&self.inner)
    }

    /// Calculate UGR (Unified Glare Rating) for a room configuration.
    ///
    /// Args:
    ///     params: UgrParams with room geometry and luminaire positions
    ///
    /// Returns:
    ///     UGR value (typically 10-30, lower is better)
    fn calculate_ugr(&self, params: &UgrParams) -> f64 {
        PhotometricCalcs::ugr(&self.inner, params)
    }

    fn __repr__(&self) -> String {
        format!(
            "Eulumdat(name='{}', symmetry={:?}, c_planes={}, g_angles={})",
            self.inner.luminaire_name,
            self.inner.symmetry,
            self.inner.c_angles.len(),
            self.inner.g_angles.len()
        )
    }
}
