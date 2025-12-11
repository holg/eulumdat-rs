/**
 * Eulumdat NAPI Bridge
 *
 * This module bridges between ArkTS and the Rust FFI library.
 * It provides JavaScript-callable functions that wrap the C API.
 */

#include <napi/native_api.h>
#include <string>
#include <cstring>

// ============================================================================
// Rust FFI declarations
// ============================================================================

extern "C" {

struct EulumdatHandle;

struct ParseResult {
    EulumdatHandle* handle;
    char* error;
};

struct LuminaireInfo {
    char* luminaire_name;
    char* identification;
    char* luminaire_number;
    char* file_name;
    char* date_user;
    char* measurement_report_number;
    int32_t symmetry;
    int32_t type_indicator;
    double length;
    double width;
    double height;
    double luminous_area_length;
    double luminous_area_width;
    uint32_t num_c_planes;
    uint32_t num_g_planes;
    double c_plane_distance;
    double g_plane_distance;
    double max_intensity;
    double total_luminous_flux;
    double downward_flux_fraction;
    double light_output_ratio;
};

struct LampSetInfo {
    int32_t num_lamps;
    char* lamp_type;
    double total_luminous_flux;
    char* color_appearance;
    char* color_rendering_group;
    double wattage_with_ballast;
};

struct LampSetList {
    LampSetInfo* data;
    size_t len;
};

struct ValidationWarningC {
    char* code;
    char* message;
    int32_t severity;
};

struct ValidationWarningList {
    ValidationWarningC* data;
    size_t len;
};

// Parsing
ParseResult eulumdat_parse_ldt(const char* content);
ParseResult eulumdat_parse_ies(const char* content);

// Memory management
void eulumdat_free(EulumdatHandle* handle);
void eulumdat_string_free(char* s);

// Information
LuminaireInfo eulumdat_get_info(const EulumdatHandle* handle);
LampSetList eulumdat_get_lamp_sets(const EulumdatHandle* handle);
void eulumdat_lamp_set_list_free(LampSetList list);

// SVG generation
char* eulumdat_polar_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);
char* eulumdat_cartesian_svg(const EulumdatHandle* handle, double width, double height, uint32_t max_curves, int32_t theme);
char* eulumdat_butterfly_svg(const EulumdatHandle* handle, double width, double height, double tilt_degrees, int32_t theme);
char* eulumdat_heatmap_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);

// Export
char* eulumdat_export_ldt(const EulumdatHandle* handle);
char* eulumdat_export_ies(const EulumdatHandle* handle);

// Validation
ValidationWarningList eulumdat_validate(const EulumdatHandle* handle);
void eulumdat_validation_list_free(ValidationWarningList list);

// Intensity sampling
double eulumdat_sample_intensity(const EulumdatHandle* handle, double c_angle, double g_angle);
double eulumdat_sample_intensity_normalized(const EulumdatHandle* handle, double c_angle, double g_angle);

// Helpers
char* eulumdat_symmetry_name(int32_t symmetry);
char* eulumdat_type_indicator_name(int32_t type_indicator);

} // extern "C"

// ============================================================================
// Global state
// ============================================================================

static EulumdatHandle* g_handle = nullptr;

// ============================================================================
// Helper functions
// ============================================================================

static napi_value CreateStringValue(napi_env env, const char* str) {
    napi_value result;
    if (str == nullptr) {
        napi_get_undefined(env, &result);
        return result;
    }
    napi_create_string_utf8(env, str, NAPI_AUTO_LENGTH, &result);
    return result;
}

static std::string GetStringArg(napi_env env, napi_value arg) {
    size_t len = 0;
    napi_get_value_string_utf8(env, arg, nullptr, 0, &len);

    std::string result(len, '\0');
    napi_get_value_string_utf8(env, arg, &result[0], len + 1, &len);
    return result;
}

// ============================================================================
// NAPI functions
// ============================================================================

/**
 * parseLdt(content: string): boolean
 * Parse LDT content and store the handle globally.
 */
static napi_value ParseLdt(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value argv[1];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    if (argc < 1) {
        napi_throw_error(env, nullptr, "Missing content argument");
        return nullptr;
    }

    std::string content = GetStringArg(env, argv[0]);

    // Free previous handle if any
    if (g_handle != nullptr) {
        eulumdat_free(g_handle);
        g_handle = nullptr;
    }

    ParseResult result = eulumdat_parse_ldt(content.c_str());

    if (result.error != nullptr) {
        napi_throw_error(env, nullptr, result.error);
        eulumdat_string_free(result.error);
        return nullptr;
    }

    g_handle = result.handle;

    napi_value success;
    napi_get_boolean(env, true, &success);
    return success;
}

/**
 * parseIes(content: string): boolean
 * Parse IES content and store the handle globally.
 */
static napi_value ParseIes(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value argv[1];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    if (argc < 1) {
        napi_throw_error(env, nullptr, "Missing content argument");
        return nullptr;
    }

    std::string content = GetStringArg(env, argv[0]);

    // Free previous handle if any
    if (g_handle != nullptr) {
        eulumdat_free(g_handle);
        g_handle = nullptr;
    }

    ParseResult result = eulumdat_parse_ies(content.c_str());

    if (result.error != nullptr) {
        napi_throw_error(env, nullptr, result.error);
        eulumdat_string_free(result.error);
        return nullptr;
    }

    g_handle = result.handle;

    napi_value success;
    napi_get_boolean(env, true, &success);
    return success;
}

/**
 * isLoaded(): boolean
 * Check if a file is currently loaded.
 */
static napi_value IsLoaded(napi_env env, napi_callback_info info) {
    napi_value result;
    napi_get_boolean(env, g_handle != nullptr, &result);
    return result;
}

/**
 * getInfo(): LuminaireInfo
 * Get luminaire information from the loaded file.
 */
static napi_value GetInfo(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    LuminaireInfo cInfo = eulumdat_get_info(g_handle);

    napi_value result;
    napi_create_object(env, &result);

    // Set all properties
    napi_value value;

    napi_set_named_property(env, result, "luminaireName", CreateStringValue(env, cInfo.luminaire_name));
    napi_set_named_property(env, result, "identification", CreateStringValue(env, cInfo.identification));
    napi_set_named_property(env, result, "luminaireNumber", CreateStringValue(env, cInfo.luminaire_number));
    napi_set_named_property(env, result, "fileName", CreateStringValue(env, cInfo.file_name));
    napi_set_named_property(env, result, "dateUser", CreateStringValue(env, cInfo.date_user));
    napi_set_named_property(env, result, "measurementReportNumber", CreateStringValue(env, cInfo.measurement_report_number));

    napi_create_int32(env, cInfo.symmetry, &value);
    napi_set_named_property(env, result, "symmetry", value);

    napi_create_int32(env, cInfo.type_indicator, &value);
    napi_set_named_property(env, result, "typeIndicator", value);

    napi_create_double(env, cInfo.length, &value);
    napi_set_named_property(env, result, "length", value);

    napi_create_double(env, cInfo.width, &value);
    napi_set_named_property(env, result, "width", value);

    napi_create_double(env, cInfo.height, &value);
    napi_set_named_property(env, result, "height", value);

    napi_create_double(env, cInfo.max_intensity, &value);
    napi_set_named_property(env, result, "maxIntensity", value);

    napi_create_double(env, cInfo.total_luminous_flux, &value);
    napi_set_named_property(env, result, "totalLuminousFlux", value);

    napi_create_double(env, cInfo.downward_flux_fraction, &value);
    napi_set_named_property(env, result, "downwardFluxFraction", value);

    napi_create_double(env, cInfo.light_output_ratio, &value);
    napi_set_named_property(env, result, "lightOutputRatio", value);

    napi_create_uint32(env, cInfo.num_c_planes, &value);
    napi_set_named_property(env, result, "numCPlanes", value);

    napi_create_uint32(env, cInfo.num_g_planes, &value);
    napi_set_named_property(env, result, "numGPlanes", value);

    // Free C strings
    eulumdat_string_free(cInfo.luminaire_name);
    eulumdat_string_free(cInfo.identification);
    eulumdat_string_free(cInfo.luminaire_number);
    eulumdat_string_free(cInfo.file_name);
    eulumdat_string_free(cInfo.date_user);
    eulumdat_string_free(cInfo.measurement_report_number);

    return result;
}

/**
 * polarSvg(width: number, height: number, theme: number): string
 * Generate polar diagram SVG.
 */
static napi_value PolarSvg(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    size_t argc = 3;
    napi_value argv[3];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    double width = 400, height = 400;
    int32_t theme = 0;

    if (argc >= 1) napi_get_value_double(env, argv[0], &width);
    if (argc >= 2) napi_get_value_double(env, argv[1], &height);
    if (argc >= 3) napi_get_value_int32(env, argv[2], &theme);

    char* svg = eulumdat_polar_svg(g_handle, width, height, theme);
    napi_value result = CreateStringValue(env, svg);
    eulumdat_string_free(svg);

    return result;
}

/**
 * cartesianSvg(width: number, height: number, maxCurves: number, theme: number): string
 * Generate cartesian diagram SVG.
 */
static napi_value CartesianSvg(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    size_t argc = 4;
    napi_value argv[4];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    double width = 500, height = 300;
    uint32_t maxCurves = 8;
    int32_t theme = 0;

    if (argc >= 1) napi_get_value_double(env, argv[0], &width);
    if (argc >= 2) napi_get_value_double(env, argv[1], &height);
    if (argc >= 3) napi_get_value_uint32(env, argv[2], &maxCurves);
    if (argc >= 4) napi_get_value_int32(env, argv[3], &theme);

    char* svg = eulumdat_cartesian_svg(g_handle, width, height, maxCurves, theme);
    napi_value result = CreateStringValue(env, svg);
    eulumdat_string_free(svg);

    return result;
}

/**
 * butterflySvg(width: number, height: number, tiltDegrees: number, theme: number): string
 * Generate butterfly diagram SVG.
 */
static napi_value ButterflySvg(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    size_t argc = 4;
    napi_value argv[4];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    double width = 400, height = 400, tiltDegrees = 60.0;
    int32_t theme = 0;

    if (argc >= 1) napi_get_value_double(env, argv[0], &width);
    if (argc >= 2) napi_get_value_double(env, argv[1], &height);
    if (argc >= 3) napi_get_value_double(env, argv[2], &tiltDegrees);
    if (argc >= 4) napi_get_value_int32(env, argv[3], &theme);

    char* svg = eulumdat_butterfly_svg(g_handle, width, height, tiltDegrees, theme);
    napi_value result = CreateStringValue(env, svg);
    eulumdat_string_free(svg);

    return result;
}

/**
 * heatmapSvg(width: number, height: number, theme: number): string
 * Generate heatmap diagram SVG.
 */
static napi_value HeatmapSvg(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    size_t argc = 3;
    napi_value argv[3];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    double width = 400, height = 300;
    int32_t theme = 0;

    if (argc >= 1) napi_get_value_double(env, argv[0], &width);
    if (argc >= 2) napi_get_value_double(env, argv[1], &height);
    if (argc >= 3) napi_get_value_int32(env, argv[2], &theme);

    char* svg = eulumdat_heatmap_svg(g_handle, width, height, theme);
    napi_value result = CreateStringValue(env, svg);
    eulumdat_string_free(svg);

    return result;
}

/**
 * exportLdt(): string
 * Export to LDT format.
 */
static napi_value ExportLdt(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    char* ldt = eulumdat_export_ldt(g_handle);
    napi_value result = CreateStringValue(env, ldt);
    eulumdat_string_free(ldt);

    return result;
}

/**
 * exportIes(): string
 * Export to IES format.
 */
static napi_value ExportIes(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    char* ies = eulumdat_export_ies(g_handle);
    napi_value result = CreateStringValue(env, ies);
    eulumdat_string_free(ies);

    return result;
}

/**
 * validate(): ValidationWarning[]
 * Validate the loaded file.
 */
static napi_value Validate(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    ValidationWarningList cList = eulumdat_validate(g_handle);

    napi_value result;
    napi_create_array_with_length(env, cList.len, &result);

    for (size_t i = 0; i < cList.len; i++) {
        ValidationWarningC* cWarning = &cList.data[i];

        napi_value warning;
        napi_create_object(env, &warning);

        napi_set_named_property(env, warning, "code", CreateStringValue(env, cWarning->code));
        napi_set_named_property(env, warning, "message", CreateStringValue(env, cWarning->message));

        napi_value severity;
        napi_create_int32(env, cWarning->severity, &severity);
        napi_set_named_property(env, warning, "severity", severity);

        napi_set_element(env, result, i, warning);
    }

    eulumdat_validation_list_free(cList);

    return result;
}

/**
 * sampleIntensity(cAngle: number, gAngle: number): number
 * Sample intensity at given angles.
 */
static napi_value SampleIntensity(napi_env env, napi_callback_info info) {
    if (g_handle == nullptr) {
        napi_throw_error(env, nullptr, "No file loaded");
        return nullptr;
    }

    size_t argc = 2;
    napi_value argv[2];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    double cAngle = 0, gAngle = 0;
    if (argc >= 1) napi_get_value_double(env, argv[0], &cAngle);
    if (argc >= 2) napi_get_value_double(env, argv[1], &gAngle);

    double intensity = eulumdat_sample_intensity(g_handle, cAngle, gAngle);

    napi_value result;
    napi_create_double(env, intensity, &result);
    return result;
}

/**
 * symmetryName(symmetry: number): string
 * Get human-readable symmetry name.
 */
static napi_value SymmetryName(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value argv[1];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    int32_t symmetry = 0;
    if (argc >= 1) napi_get_value_int32(env, argv[0], &symmetry);

    char* name = eulumdat_symmetry_name(symmetry);
    napi_value result = CreateStringValue(env, name);
    eulumdat_string_free(name);

    return result;
}

/**
 * typeIndicatorName(typeIndicator: number): string
 * Get human-readable type indicator name.
 */
static napi_value TypeIndicatorName(napi_env env, napi_callback_info info) {
    size_t argc = 1;
    napi_value argv[1];
    napi_get_cb_info(env, info, &argc, argv, nullptr, nullptr);

    int32_t typeIndicator = 0;
    if (argc >= 1) napi_get_value_int32(env, argv[0], &typeIndicator);

    char* name = eulumdat_type_indicator_name(typeIndicator);
    napi_value result = CreateStringValue(env, name);
    eulumdat_string_free(name);

    return result;
}

// ============================================================================
// Module registration
// ============================================================================

EXTERN_C_START
static napi_value Init(napi_env env, napi_value exports) {
    napi_property_descriptor desc[] = {
        { "parseLdt", nullptr, ParseLdt, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "parseIes", nullptr, ParseIes, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "isLoaded", nullptr, IsLoaded, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "getInfo", nullptr, GetInfo, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "polarSvg", nullptr, PolarSvg, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "cartesianSvg", nullptr, CartesianSvg, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "butterflySvg", nullptr, ButterflySvg, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "heatmapSvg", nullptr, HeatmapSvg, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "exportLdt", nullptr, ExportLdt, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "exportIes", nullptr, ExportIes, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "validate", nullptr, Validate, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "sampleIntensity", nullptr, SampleIntensity, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "symmetryName", nullptr, SymmetryName, nullptr, nullptr, nullptr, napi_default, nullptr },
        { "typeIndicatorName", nullptr, TypeIndicatorName, nullptr, nullptr, nullptr, napi_default, nullptr },
    };

    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}
EXTERN_C_END

static napi_module eulumdatModule = {
    .nm_version = 1,
    .nm_flags = 0,
    .nm_filename = nullptr,
    .nm_register_func = Init,
    .nm_modname = "eulumdat_napi",
    .nm_priv = nullptr,
    .reserved = { 0 },
};

extern "C" __attribute__((constructor)) void RegisterEulumdatModule(void) {
    napi_module_register(&eulumdatModule);
}
