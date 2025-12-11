//! Simple C FFI bindings for Eulumdat - HarmonyOS/Cangjie compatible
//!
//! This crate provides simple C-compatible functions for use with HarmonyOS Cangjie.
//! Unlike the UniFFI-based `eulumdat-ffi`, this uses plain C types for easier integration.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    Eulumdat, Symmetry as CoreSymmetry, TypeIndicator as CoreTypeIndicator,
};

// ============================================================================
// Opaque handle type
// ============================================================================

/// Opaque handle to a parsed Eulumdat (LDT/IES) file
pub struct EulumdatHandle {
    inner: Eulumdat,
}

// ============================================================================
// Result types for C
// ============================================================================

/// Result of parsing an LDT/IES file
#[repr(C)]
pub struct ParseResult {
    pub handle: *mut EulumdatHandle,
    pub error: *mut c_char,
}

/// Basic luminaire information
#[repr(C)]
pub struct LuminaireInfo {
    pub luminaire_name: *mut c_char,
    pub identification: *mut c_char,
    pub luminaire_number: *mut c_char,
    pub file_name: *mut c_char,
    pub date_user: *mut c_char,
    pub measurement_report_number: *mut c_char,
    pub symmetry: i32, // 0=None, 1=VerticalAxis, 2=PlaneC0C180, 3=PlaneC90C270, 4=BothPlanes
    pub type_indicator: i32, // 0=PointSourceSymmetric, 1=Linear, 2=PointSourceOther
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub luminous_area_length: f64,
    pub luminous_area_width: f64,
    pub num_c_planes: u32,
    pub num_g_planes: u32,
    pub c_plane_distance: f64,
    pub g_plane_distance: f64,
    pub max_intensity: f64,
    pub total_luminous_flux: f64,
    pub downward_flux_fraction: f64,
    pub light_output_ratio: f64,
}

/// Lamp set information
#[repr(C)]
pub struct LampSetInfo {
    pub num_lamps: i32,
    pub lamp_type: *mut c_char,
    pub total_luminous_flux: f64,
    pub color_appearance: *mut c_char,
    pub color_rendering_group: *mut c_char,
    pub wattage_with_ballast: f64,
}

/// List of lamp sets
#[repr(C)]
pub struct LampSetList {
    pub data: *mut LampSetInfo,
    pub len: usize,
}

/// Validation warning
#[repr(C)]
pub struct ValidationWarningC {
    pub code: *mut c_char,
    pub message: *mut c_char,
    pub severity: i32, // 0=Info, 1=Warning (all warnings from validate are warnings)
}

/// List of validation warnings
#[repr(C)]
pub struct ValidationWarningList {
    pub data: *mut ValidationWarningC,
    pub len: usize,
}

// ============================================================================
// Helper functions
// ============================================================================

fn string_to_c(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn symmetry_to_int(s: CoreSymmetry) -> i32 {
    match s {
        CoreSymmetry::None => 0,
        CoreSymmetry::VerticalAxis => 1,
        CoreSymmetry::PlaneC0C180 => 2,
        CoreSymmetry::PlaneC90C270 => 3,
        CoreSymmetry::BothPlanes => 4,
    }
}

fn type_indicator_to_int(t: CoreTypeIndicator) -> i32 {
    match t {
        CoreTypeIndicator::PointSourceSymmetric => 0,
        CoreTypeIndicator::Linear => 1,
        CoreTypeIndicator::PointSourceOther => 2,
    }
}

// ============================================================================
// Parsing functions
// ============================================================================

/// Parse LDT content from a string
///
/// # Safety
/// - `content` must be a valid null-terminated UTF-8 string
/// - Caller must free the returned handle with `eulumdat_free`
/// - Caller must free any error string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_parse_ldt(content: *const c_char) -> ParseResult {
    if content.is_null() {
        return ParseResult {
            handle: ptr::null_mut(),
            error: string_to_c("Content is null"),
        };
    }

    let content_str = match CStr::from_ptr(content).to_str() {
        Ok(s) => s,
        Err(_) => {
            return ParseResult {
                handle: ptr::null_mut(),
                error: string_to_c("Invalid UTF-8 content"),
            };
        }
    };

    match Eulumdat::parse(content_str) {
        Ok(ldt) => {
            let handle = Box::new(EulumdatHandle { inner: ldt });
            ParseResult {
                handle: Box::into_raw(handle),
                error: ptr::null_mut(),
            }
        }
        Err(e) => ParseResult {
            handle: ptr::null_mut(),
            error: string_to_c(&e.to_string()),
        },
    }
}

/// Parse IES content from a string
///
/// # Safety
/// - `content` must be a valid null-terminated UTF-8 string
/// - Caller must free the returned handle with `eulumdat_free`
/// - Caller must free any error string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_parse_ies(content: *const c_char) -> ParseResult {
    if content.is_null() {
        return ParseResult {
            handle: ptr::null_mut(),
            error: string_to_c("Content is null"),
        };
    }

    let content_str = match CStr::from_ptr(content).to_str() {
        Ok(s) => s,
        Err(_) => {
            return ParseResult {
                handle: ptr::null_mut(),
                error: string_to_c("Invalid UTF-8 content"),
            };
        }
    };

    match eulumdat::IesParser::parse(content_str) {
        Ok(ldt) => {
            let handle = Box::new(EulumdatHandle { inner: ldt });
            ParseResult {
                handle: Box::into_raw(handle),
                error: ptr::null_mut(),
            }
        }
        Err(e) => ParseResult {
            handle: ptr::null_mut(),
            error: string_to_c(&e.to_string()),
        },
    }
}

/// Free an Eulumdat handle
///
/// # Safety
/// - `handle` must be a valid pointer returned by `eulumdat_parse_ldt` or `eulumdat_parse_ies`
/// - Must not be called more than once for the same handle
#[no_mangle]
pub unsafe extern "C" fn eulumdat_free(handle: *mut EulumdatHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Free a string returned by this library
///
/// # Safety
/// - `s` must be a valid pointer returned by this library
/// - Must not be called more than once for the same string
#[no_mangle]
pub unsafe extern "C" fn eulumdat_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ============================================================================
// Information retrieval
// ============================================================================

/// Get basic luminaire information
///
/// # Safety
/// - `handle` must be a valid pointer
/// - Caller must free all string fields with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_get_info(handle: *const EulumdatHandle) -> LuminaireInfo {
    if handle.is_null() {
        return LuminaireInfo {
            luminaire_name: ptr::null_mut(),
            identification: ptr::null_mut(),
            luminaire_number: ptr::null_mut(),
            file_name: ptr::null_mut(),
            date_user: ptr::null_mut(),
            measurement_report_number: ptr::null_mut(),
            symmetry: 0,
            type_indicator: 0,
            length: 0.0,
            width: 0.0,
            height: 0.0,
            luminous_area_length: 0.0,
            luminous_area_width: 0.0,
            num_c_planes: 0,
            num_g_planes: 0,
            c_plane_distance: 0.0,
            g_plane_distance: 0.0,
            max_intensity: 0.0,
            total_luminous_flux: 0.0,
            downward_flux_fraction: 0.0,
            light_output_ratio: 0.0,
        };
    }

    let ldt = &(*handle).inner;

    LuminaireInfo {
        luminaire_name: string_to_c(&ldt.luminaire_name),
        identification: string_to_c(&ldt.identification),
        luminaire_number: string_to_c(&ldt.luminaire_number),
        file_name: string_to_c(&ldt.file_name),
        date_user: string_to_c(&ldt.date_user),
        measurement_report_number: string_to_c(&ldt.measurement_report_number),
        symmetry: symmetry_to_int(ldt.symmetry),
        type_indicator: type_indicator_to_int(ldt.type_indicator),
        length: ldt.length,
        width: ldt.width,
        height: ldt.height,
        luminous_area_length: ldt.luminous_area_length,
        luminous_area_width: ldt.luminous_area_width,
        num_c_planes: ldt.num_c_planes as u32,
        num_g_planes: ldt.num_g_planes as u32,
        c_plane_distance: ldt.c_plane_distance,
        g_plane_distance: ldt.g_plane_distance,
        max_intensity: ldt.max_intensity(),
        total_luminous_flux: ldt.total_luminous_flux(),
        downward_flux_fraction: ldt.downward_flux_fraction,
        light_output_ratio: ldt.light_output_ratio,
    }
}

/// Get lamp sets information
///
/// # Safety
/// - `handle` must be a valid pointer
/// - Caller must free with `eulumdat_lamp_set_list_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_get_lamp_sets(handle: *const EulumdatHandle) -> LampSetList {
    if handle.is_null() {
        return LampSetList {
            data: ptr::null_mut(),
            len: 0,
        };
    }

    let ldt = &(*handle).inner;
    let len = ldt.lamp_sets.len();

    if len == 0 {
        return LampSetList {
            data: ptr::null_mut(),
            len: 0,
        };
    }

    let mut lamp_sets: Vec<LampSetInfo> = ldt
        .lamp_sets
        .iter()
        .map(|ls| LampSetInfo {
            num_lamps: ls.num_lamps,
            lamp_type: string_to_c(&ls.lamp_type),
            total_luminous_flux: ls.total_luminous_flux,
            color_appearance: string_to_c(&ls.color_appearance),
            color_rendering_group: string_to_c(&ls.color_rendering_group),
            wattage_with_ballast: ls.wattage_with_ballast,
        })
        .collect();

    let data = lamp_sets.as_mut_ptr();
    std::mem::forget(lamp_sets);

    LampSetList { data, len }
}

/// Free lamp set list
///
/// # Safety
/// - Must be called with a valid LampSetList returned by `eulumdat_get_lamp_sets`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_lamp_set_list_free(list: LampSetList) {
    if !list.data.is_null() && list.len > 0 {
        let lamp_sets = Vec::from_raw_parts(list.data, list.len, list.len);
        for ls in lamp_sets {
            eulumdat_string_free(ls.lamp_type);
            eulumdat_string_free(ls.color_appearance);
            eulumdat_string_free(ls.color_rendering_group);
        }
    }
}

// ============================================================================
// SVG Diagram Generation
// ============================================================================

/// Generate polar diagram SVG
///
/// # Safety
/// - `handle` must be a valid pointer
/// - `theme` must be 0 (light) or 1 (dark)
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_polar_svg(
    handle: *const EulumdatHandle,
    width: f64,
    height: f64,
    theme: i32,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let svg_theme = if theme == 1 {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
    };

    let polar = PolarDiagram::from_eulumdat(ldt);
    let svg = polar.to_svg(width, height, &svg_theme);
    string_to_c(&svg)
}

/// Generate cartesian diagram SVG
///
/// # Safety
/// - `handle` must be a valid pointer
/// - `theme` must be 0 (light) or 1 (dark)
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_cartesian_svg(
    handle: *const EulumdatHandle,
    width: f64,
    height: f64,
    max_curves: u32,
    theme: i32,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let svg_theme = if theme == 1 {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
    };

    let cartesian = CartesianDiagram::from_eulumdat(ldt, width, height, max_curves as usize);
    let svg = cartesian.to_svg(width, height, &svg_theme);
    string_to_c(&svg)
}

/// Generate butterfly (3D) diagram SVG
///
/// # Safety
/// - `handle` must be a valid pointer
/// - `theme` must be 0 (light) or 1 (dark)
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_butterfly_svg(
    handle: *const EulumdatHandle,
    width: f64,
    height: f64,
    tilt_degrees: f64,
    theme: i32,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let svg_theme = if theme == 1 {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
    };

    let butterfly = ButterflyDiagram::from_eulumdat(ldt, width, height, tilt_degrees);
    let svg = butterfly.to_svg(width, height, &svg_theme);
    string_to_c(&svg)
}

/// Generate heatmap diagram SVG
///
/// # Safety
/// - `handle` must be a valid pointer
/// - `theme` must be 0 (light) or 1 (dark)
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_heatmap_svg(
    handle: *const EulumdatHandle,
    width: f64,
    height: f64,
    theme: i32,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let svg_theme = if theme == 1 {
        SvgTheme::dark()
    } else {
        SvgTheme::light()
    };

    let heatmap = HeatmapDiagram::from_eulumdat(ldt, width, height);
    let svg = heatmap.to_svg(width, height, &svg_theme);
    string_to_c(&svg)
}

// ============================================================================
// Export functions
// ============================================================================

/// Export to LDT format string
///
/// # Safety
/// - `handle` must be a valid pointer
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_export_ldt(handle: *const EulumdatHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let ldt_string = ldt.to_ldt();
    string_to_c(&ldt_string)
}

/// Export to IES format string
///
/// # Safety
/// - `handle` must be a valid pointer
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_export_ies(handle: *const EulumdatHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let ldt = &(*handle).inner;
    let ies_string = eulumdat::IesExporter::export(ldt);
    string_to_c(&ies_string)
}

// ============================================================================
// Validation
// ============================================================================

/// Validate the luminaire data and return warnings
///
/// # Safety
/// - `handle` must be a valid pointer
/// - Caller must free with `eulumdat_validation_list_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_validate(handle: *const EulumdatHandle) -> ValidationWarningList {
    if handle.is_null() {
        return ValidationWarningList {
            data: ptr::null_mut(),
            len: 0,
        };
    }

    let ldt = &(*handle).inner;
    let warnings = eulumdat::validate(ldt);

    if warnings.is_empty() {
        return ValidationWarningList {
            data: ptr::null_mut(),
            len: 0,
        };
    }

    let mut warning_list: Vec<ValidationWarningC> = warnings
        .iter()
        .map(|w| {
            // All warnings from validate() have severity=1 (Warning)
            ValidationWarningC {
                code: string_to_c(w.code),
                message: string_to_c(&w.message),
                severity: 1, // Warning
            }
        })
        .collect();

    let len = warning_list.len();
    let data = warning_list.as_mut_ptr();
    std::mem::forget(warning_list);

    ValidationWarningList { data, len }
}

/// Free validation warning list
///
/// # Safety
/// - Must be called with a valid ValidationWarningList returned by `eulumdat_validate`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_validation_list_free(list: ValidationWarningList) {
    if !list.data.is_null() && list.len > 0 {
        let warnings = Vec::from_raw_parts(list.data, list.len, list.len);
        for w in warnings {
            eulumdat_string_free(w.code);
            eulumdat_string_free(w.message);
        }
    }
}

// ============================================================================
// Intensity sampling
// ============================================================================

/// Sample intensity at any C and G angle using bilinear interpolation
///
/// # Safety
/// - `handle` must be a valid pointer
#[no_mangle]
pub unsafe extern "C" fn eulumdat_sample_intensity(
    handle: *const EulumdatHandle,
    c_angle: f64,
    g_angle: f64,
) -> f64 {
    if handle.is_null() {
        return 0.0;
    }

    let ldt = &(*handle).inner;
    ldt.sample(c_angle, g_angle)
}

/// Sample normalized intensity (0.0 to 1.0) at any C and G angle
///
/// # Safety
/// - `handle` must be a valid pointer
#[no_mangle]
pub unsafe extern "C" fn eulumdat_sample_intensity_normalized(
    handle: *const EulumdatHandle,
    c_angle: f64,
    g_angle: f64,
) -> f64 {
    if handle.is_null() {
        return 0.0;
    }

    let ldt = &(*handle).inner;
    let intensity = ldt.sample(c_angle, g_angle);
    let max = ldt.max_intensity();

    if max > 0.0 {
        intensity / max
    } else {
        0.0
    }
}

// ============================================================================
// Symmetry name helper
// ============================================================================

/// Get symmetry name as string
///
/// # Safety
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_symmetry_name(symmetry: i32) -> *mut c_char {
    let name = match symmetry {
        0 => "None (Full 360°)",
        1 => "Vertical Axis (C0 only)",
        2 => "Plane C0-C180",
        3 => "Plane C90-C270",
        4 => "Both Planes (Quadrant)",
        _ => "Unknown",
    };
    string_to_c(name)
}

/// Get type indicator name as string
///
/// # Safety
/// - Caller must free the returned string with `eulumdat_string_free`
#[no_mangle]
pub unsafe extern "C" fn eulumdat_type_indicator_name(type_indicator: i32) -> *mut c_char {
    let name = match type_indicator {
        0 => "Point Source (Symmetric)",
        1 => "Linear",
        2 => "Point Source (Other)",
        _ => "Unknown",
    };
    string_to_c(name)
}
