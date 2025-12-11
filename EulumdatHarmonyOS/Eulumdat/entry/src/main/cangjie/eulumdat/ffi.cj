/**
 * Eulumdat FFI - Direct C bindings for Cangjie
 *
 * This module declares the foreign C functions from libeulumdat_harmonyos_ffi.so
 */
package eulumdat

// ============================================================================
// Opaque handle type
// ============================================================================

/** Opaque handle to a parsed Eulumdat (LDT/IES) file */
public struct CEulumdatHandle {}

// ============================================================================
// C struct definitions
// ============================================================================

/** Result of parsing an LDT/IES file */
public struct CParseResult {
    public var handle: CPointer<CEulumdatHandle>
    public var error: CPointer<UInt8>
}

/** Basic luminaire information */
public struct CLuminaireInfo {
    public var luminaire_name: CPointer<UInt8>
    public var identification: CPointer<UInt8>
    public var luminaire_number: CPointer<UInt8>
    public var file_name: CPointer<UInt8>
    public var date_user: CPointer<UInt8>
    public var measurement_report_number: CPointer<UInt8>
    public var symmetry: Int32
    public var type_indicator: Int32
    public var length: Float64
    public var width: Float64
    public var height: Float64
    public var luminous_area_length: Float64
    public var luminous_area_width: Float64
    public var num_c_planes: UInt32
    public var num_g_planes: UInt32
    public var c_plane_distance: Float64
    public var g_plane_distance: Float64
    public var max_intensity: Float64
    public var total_luminous_flux: Float64
    public var downward_flux_fraction: Float64
    public var light_output_ratio: Float64
}

/** Lamp set information */
public struct CLampSetInfo {
    public var num_lamps: Int32
    public var lamp_type: CPointer<UInt8>
    public var total_luminous_flux: Float64
    public var color_appearance: CPointer<UInt8>
    public var color_rendering_group: CPointer<UInt8>
    public var wattage_with_ballast: Float64
}

/** List of lamp sets */
public struct CLampSetList {
    public var data: CPointer<CLampSetInfo>
    public var len: UInt64
}

/** Validation warning */
public struct CValidationWarning {
    public var code: CPointer<UInt8>
    public var message: CPointer<UInt8>
    public var severity: Int32
}

/** List of validation warnings */
public struct CValidationWarningList {
    public var data: CPointer<CValidationWarning>
    public var len: UInt64
}

// ============================================================================
// Foreign function declarations
// ============================================================================

// Parsing
foreign func eulumdat_parse_ldt(content: CPointer<UInt8>): CParseResult
foreign func eulumdat_parse_ies(content: CPointer<UInt8>): CParseResult

// Memory management
foreign func eulumdat_free(handle: CPointer<CEulumdatHandle>): Unit
foreign func eulumdat_string_free(s: CPointer<UInt8>): Unit

// Information retrieval
foreign func eulumdat_get_info(handle: CPointer<CEulumdatHandle>): CLuminaireInfo
foreign func eulumdat_get_lamp_sets(handle: CPointer<CEulumdatHandle>): CLampSetList
foreign func eulumdat_lamp_set_list_free(list: CLampSetList): Unit

// SVG diagram generation
foreign func eulumdat_polar_svg(
    handle: CPointer<CEulumdatHandle>,
    width: Float64,
    height: Float64,
    theme: Int32
): CPointer<UInt8>

foreign func eulumdat_cartesian_svg(
    handle: CPointer<CEulumdatHandle>,
    width: Float64,
    height: Float64,
    max_curves: UInt32,
    theme: Int32
): CPointer<UInt8>

foreign func eulumdat_butterfly_svg(
    handle: CPointer<CEulumdatHandle>,
    width: Float64,
    height: Float64,
    tilt_degrees: Float64,
    theme: Int32
): CPointer<UInt8>

foreign func eulumdat_heatmap_svg(
    handle: CPointer<CEulumdatHandle>,
    width: Float64,
    height: Float64,
    theme: Int32
): CPointer<UInt8>

// Export
foreign func eulumdat_export_ldt(handle: CPointer<CEulumdatHandle>): CPointer<UInt8>
foreign func eulumdat_export_ies(handle: CPointer<CEulumdatHandle>): CPointer<UInt8>

// Validation
foreign func eulumdat_validate(handle: CPointer<CEulumdatHandle>): CValidationWarningList
foreign func eulumdat_validation_list_free(list: CValidationWarningList): Unit

// Intensity sampling
foreign func eulumdat_sample_intensity(
    handle: CPointer<CEulumdatHandle>,
    c_angle: Float64,
    g_angle: Float64
): Float64

foreign func eulumdat_sample_intensity_normalized(
    handle: CPointer<CEulumdatHandle>,
    c_angle: Float64,
    g_angle: Float64
): Float64

// Helper functions
foreign func eulumdat_symmetry_name(symmetry: Int32): CPointer<UInt8>
foreign func eulumdat_type_indicator_name(type_indicator: Int32): CPointer<UInt8>
