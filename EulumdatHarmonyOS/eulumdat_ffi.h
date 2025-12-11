/**
 * Eulumdat FFI - Simple C API for HarmonyOS/Cangjie
 *
 * This header defines the C interface for the Eulumdat library.
 * Use this with the libeulumdat_harmonyos_ffi.so library.
 */

#ifndef EULUMDAT_FFI_H
#define EULUMDAT_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Opaque handle type
// ============================================================================

/** Opaque handle to a parsed Eulumdat (LDT/IES) file */
typedef struct EulumdatHandle EulumdatHandle;

// ============================================================================
// Result types
// ============================================================================

/** Result of parsing an LDT/IES file */
typedef struct {
    EulumdatHandle* handle;  /** Non-null on success */
    char* error;             /** Non-null on failure, must be freed */
} ParseResult;

/** Basic luminaire information */
typedef struct {
    char* luminaire_name;
    char* identification;
    char* luminaire_number;
    char* file_name;
    char* date_user;
    char* measurement_report_number;
    int32_t symmetry;           /* 0=None, 1=VerticalAxis, 2=PlaneC0C180, 3=PlaneC90C270, 4=BothPlanes */
    int32_t type_indicator;     /* 0=PointSourceSymmetric, 1=Linear, 2=PointSourceOther */
    double length;              /* mm */
    double width;               /* mm */
    double height;              /* mm */
    double luminous_area_length;
    double luminous_area_width;
    uint32_t num_c_planes;
    uint32_t num_g_planes;
    double c_plane_distance;    /* degrees */
    double g_plane_distance;    /* degrees */
    double max_intensity;       /* cd/klm */
    double total_luminous_flux; /* lm */
    double downward_flux_fraction;
    double light_output_ratio;
} LuminaireInfo;

/** Lamp set information */
typedef struct {
    int32_t num_lamps;
    char* lamp_type;
    double total_luminous_flux;
    char* color_appearance;
    char* color_rendering_group;
    double wattage_with_ballast;
} LampSetInfo;

/** List of lamp sets */
typedef struct {
    LampSetInfo* data;
    size_t len;
} LampSetList;

/** Validation warning */
typedef struct {
    char* code;
    char* message;
    int32_t severity;  /* 0=Info, 1=Warning, 2=Error */
} ValidationWarningC;

/** List of validation warnings */
typedef struct {
    ValidationWarningC* data;
    size_t len;
} ValidationWarningList;

// ============================================================================
// Parsing functions
// ============================================================================

/**
 * Parse LDT content from a string
 *
 * @param content Null-terminated UTF-8 string containing LDT file content
 * @return ParseResult with handle on success, error message on failure
 * @note Caller must free handle with eulumdat_free()
 * @note Caller must free error string with eulumdat_string_free()
 */
ParseResult eulumdat_parse_ldt(const char* content);

/**
 * Parse IES content from a string
 *
 * @param content Null-terminated UTF-8 string containing IES file content
 * @return ParseResult with handle on success, error message on failure
 * @note Caller must free handle with eulumdat_free()
 * @note Caller must free error string with eulumdat_string_free()
 */
ParseResult eulumdat_parse_ies(const char* content);

/**
 * Free an Eulumdat handle
 *
 * @param handle Handle to free (null-safe)
 */
void eulumdat_free(EulumdatHandle* handle);

/**
 * Free a string returned by this library
 *
 * @param s String to free (null-safe)
 */
void eulumdat_string_free(char* s);

// ============================================================================
// Information retrieval
// ============================================================================

/**
 * Get basic luminaire information
 *
 * @param handle Valid Eulumdat handle
 * @return LuminaireInfo struct (all string fields must be freed)
 */
LuminaireInfo eulumdat_get_info(const EulumdatHandle* handle);

/**
 * Get lamp sets information
 *
 * @param handle Valid Eulumdat handle
 * @return LampSetList (must be freed with eulumdat_lamp_set_list_free)
 */
LampSetList eulumdat_get_lamp_sets(const EulumdatHandle* handle);

/**
 * Free lamp set list
 *
 * @param list LampSetList to free
 */
void eulumdat_lamp_set_list_free(LampSetList list);

// ============================================================================
// SVG Diagram Generation
// ============================================================================

/**
 * Generate polar diagram SVG
 *
 * @param handle Valid Eulumdat handle
 * @param width SVG width in pixels
 * @param height SVG height in pixels
 * @param theme 0=light, 1=dark
 * @return SVG string (must be freed with eulumdat_string_free)
 */
char* eulumdat_polar_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);

/**
 * Generate cartesian diagram SVG
 *
 * @param handle Valid Eulumdat handle
 * @param width SVG width in pixels
 * @param height SVG height in pixels
 * @param max_curves Maximum number of curves to display
 * @param theme 0=light, 1=dark
 * @return SVG string (must be freed with eulumdat_string_free)
 */
char* eulumdat_cartesian_svg(const EulumdatHandle* handle, double width, double height, uint32_t max_curves, int32_t theme);

/**
 * Generate butterfly (3D) diagram SVG
 *
 * @param handle Valid Eulumdat handle
 * @param width SVG width in pixels
 * @param height SVG height in pixels
 * @param tilt_degrees Tilt angle in degrees
 * @param theme 0=light, 1=dark
 * @return SVG string (must be freed with eulumdat_string_free)
 */
char* eulumdat_butterfly_svg(const EulumdatHandle* handle, double width, double height, double tilt_degrees, int32_t theme);

/**
 * Generate heatmap diagram SVG
 *
 * @param handle Valid Eulumdat handle
 * @param width SVG width in pixels
 * @param height SVG height in pixels
 * @param theme 0=light, 1=dark
 * @return SVG string (must be freed with eulumdat_string_free)
 */
char* eulumdat_heatmap_svg(const EulumdatHandle* handle, double width, double height, int32_t theme);

// ============================================================================
// Export functions
// ============================================================================

/**
 * Export to LDT format string
 *
 * @param handle Valid Eulumdat handle
 * @return LDT format string (must be freed with eulumdat_string_free)
 */
char* eulumdat_export_ldt(const EulumdatHandle* handle);

/**
 * Export to IES format string
 *
 * @param handle Valid Eulumdat handle
 * @return IES format string (must be freed with eulumdat_string_free)
 */
char* eulumdat_export_ies(const EulumdatHandle* handle);

// ============================================================================
// Validation
// ============================================================================

/**
 * Validate the luminaire data and return warnings
 *
 * @param handle Valid Eulumdat handle
 * @return ValidationWarningList (must be freed with eulumdat_validation_list_free)
 */
ValidationWarningList eulumdat_validate(const EulumdatHandle* handle);

/**
 * Free validation warning list
 *
 * @param list ValidationWarningList to free
 */
void eulumdat_validation_list_free(ValidationWarningList list);

// ============================================================================
// Intensity sampling
// ============================================================================

/**
 * Sample intensity at any C and G angle using bilinear interpolation
 *
 * @param handle Valid Eulumdat handle
 * @param c_angle C-plane angle in degrees (0-360)
 * @param g_angle Gamma angle in degrees (0-180)
 * @return Intensity in cd/klm
 */
double eulumdat_sample_intensity(const EulumdatHandle* handle, double c_angle, double g_angle);

/**
 * Sample normalized intensity (0.0 to 1.0) at any C and G angle
 *
 * @param handle Valid Eulumdat handle
 * @param c_angle C-plane angle in degrees (0-360)
 * @param g_angle Gamma angle in degrees (0-180)
 * @return Normalized intensity (0.0 to 1.0)
 */
double eulumdat_sample_intensity_normalized(const EulumdatHandle* handle, double c_angle, double g_angle);

// ============================================================================
// Helper functions
// ============================================================================

/**
 * Get symmetry name as string
 *
 * @param symmetry Symmetry value (0-4)
 * @return Human-readable symmetry name (must be freed)
 */
char* eulumdat_symmetry_name(int32_t symmetry);

/**
 * Get type indicator name as string
 *
 * @param type_indicator Type indicator value (0-2)
 * @return Human-readable type indicator name (must be freed)
 */
char* eulumdat_type_indicator_name(int32_t type_indicator);

#ifdef __cplusplus
}
#endif

#endif /* EULUMDAT_FFI_H */
