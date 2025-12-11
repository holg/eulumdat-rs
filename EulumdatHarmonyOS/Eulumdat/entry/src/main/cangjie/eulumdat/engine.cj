/**
 * Eulumdat Engine - High-level API for working with photometric data
 *
 * This module provides a safe, high-level interface for parsing and
 * visualizing LDT (EULUMDAT) and IES photometric data files.
 */
package eulumdat

import std.collection.ArrayList

/**
 * Eulumdat - Represents a parsed photometric data file
 *
 * This class wraps the native Rust library and provides methods for:
 * - Loading LDT (EULUMDAT) and IES files
 * - Accessing luminaire information
 * - Generating SVG diagrams (polar, cartesian, butterfly, heatmap)
 * - Exporting to LDT/IES formats
 * - Validating photometric data
 */
public class Eulumdat {
    private let handle: CPointer<CEulumdatHandle>
    private var _info: LuminaireInfo?

    /**
     * Parse LDT (EULUMDAT) format content
     *
     * @param content The LDT file content as a string
     * @return Parsed Eulumdat object
     * @throws Exception if parsing fails
     */
    public static func parseLdt(content: String): Eulumdat {
        let cContent = stringToCString(content)
        defer {
            freeCString(cContent)
        }

        unsafe {
            let result = eulumdat_parse_ldt(cContent)

            if (!result.error.isNull()) {
                let errorMsg = cStringToString(result.error)
                eulumdat_string_free(result.error)
                throw Exception("Failed to parse LDT: ${errorMsg}")
            }

            if (result.handle.isNull()) {
                throw Exception("Failed to parse LDT: Unknown error")
            }

            return Eulumdat(result.handle)
        }
    }

    /**
     * Parse IES format content
     *
     * @param content The IES file content as a string
     * @return Parsed Eulumdat object
     * @throws Exception if parsing fails
     */
    public static func parseIes(content: String): Eulumdat {
        let cContent = stringToCString(content)
        defer {
            freeCString(cContent)
        }

        unsafe {
            let result = eulumdat_parse_ies(cContent)

            if (!result.error.isNull()) {
                let errorMsg = cStringToString(result.error)
                eulumdat_string_free(result.error)
                throw Exception("Failed to parse IES: ${errorMsg}")
            }

            if (result.handle.isNull()) {
                throw Exception("Failed to parse IES: Unknown error")
            }

            return Eulumdat(result.handle)
        }
    }

    /** Private constructor - use parseLdt() or parseIes() */
    private init(handle: CPointer<CEulumdatHandle>) {
        this.handle = handle
        this._info = None
    }

    /** Destructor - free native resources */
    public func finalize() {
        unsafe {
            if (!this.handle.isNull()) {
                eulumdat_free(this.handle)
            }
        }
    }

    // ========================================================================
    // Information access
    // ========================================================================

    /** Get luminaire information (cached) */
    public func info(): LuminaireInfo {
        if (let cached = this._info) {
            return cached
        }

        unsafe {
            let cInfo = eulumdat_get_info(this.handle)
            let info = LuminaireInfo.fromC(cInfo)
            this._info = Some(info)
            return info
        }
    }

    /** Get luminaire name */
    public func luminaireName(): String {
        return this.info().luminaireName
    }

    /** Get manufacturer identification */
    public func identification(): String {
        return this.info().identification
    }

    /** Get symmetry type */
    public func symmetry(): Symmetry {
        return this.info().symmetry
    }

    /** Get type indicator */
    public func typeIndicator(): TypeIndicator {
        return this.info().typeIndicator
    }

    /** Get maximum intensity in cd/klm */
    public func maxIntensity(): Float64 {
        return this.info().maxIntensity
    }

    /** Get total luminous flux in lumens */
    public func totalLuminousFlux(): Float64 {
        return this.info().totalLuminousFlux
    }

    /** Get lamp sets */
    public func lampSets(): ArrayList<LampSet> {
        unsafe {
            let cList = eulumdat_get_lamp_sets(this.handle)
            var lampSets = ArrayList<LampSet>()

            if (!cList.data.isNull() && cList.len > 0) {
                for (i in 0..Int64(cList.len)) {
                    let cLampSet = cList.data[i]
                    let lampSet = LampSet(
                        numLamps: cLampSet.num_lamps,
                        lampType: cStringToString(cLampSet.lamp_type),
                        totalLuminousFlux: cLampSet.total_luminous_flux,
                        colorAppearance: cStringToString(cLampSet.color_appearance),
                        colorRenderingGroup: cStringToString(cLampSet.color_rendering_group),
                        wattageWithBallast: cLampSet.wattage_with_ballast
                    )
                    lampSets.append(lampSet)
                }
                eulumdat_lamp_set_list_free(cList)
            }

            return lampSets
        }
    }

    // ========================================================================
    // SVG Diagram Generation
    // ========================================================================

    /**
     * Generate polar diagram SVG
     *
     * @param width SVG width in pixels
     * @param height SVG height in pixels
     * @param theme Light or Dark theme
     * @return SVG string
     */
    public func polarSvg(width: Float64, height: Float64, theme: SvgTheme): String {
        unsafe {
            let svgPtr = eulumdat_polar_svg(this.handle, width, height, theme.toInt())
            if (svgPtr.isNull()) {
                return ""
            }
            let svg = cStringToString(svgPtr)
            eulumdat_string_free(svgPtr)
            return svg
        }
    }

    /**
     * Generate cartesian diagram SVG
     *
     * @param width SVG width in pixels
     * @param height SVG height in pixels
     * @param maxCurves Maximum number of C-plane curves to show
     * @param theme Light or Dark theme
     * @return SVG string
     */
    public func cartesianSvg(width: Float64, height: Float64, maxCurves: UInt32, theme: SvgTheme): String {
        unsafe {
            let svgPtr = eulumdat_cartesian_svg(this.handle, width, height, maxCurves, theme.toInt())
            if (svgPtr.isNull()) {
                return ""
            }
            let svg = cStringToString(svgPtr)
            eulumdat_string_free(svgPtr)
            return svg
        }
    }

    /**
     * Generate butterfly (3D) diagram SVG
     *
     * @param width SVG width in pixels
     * @param height SVG height in pixels
     * @param tiltDegrees Tilt angle in degrees
     * @param theme Light or Dark theme
     * @return SVG string
     */
    public func butterflySvg(width: Float64, height: Float64, tiltDegrees: Float64, theme: SvgTheme): String {
        unsafe {
            let svgPtr = eulumdat_butterfly_svg(this.handle, width, height, tiltDegrees, theme.toInt())
            if (svgPtr.isNull()) {
                return ""
            }
            let svg = cStringToString(svgPtr)
            eulumdat_string_free(svgPtr)
            return svg
        }
    }

    /**
     * Generate heatmap diagram SVG
     *
     * @param width SVG width in pixels
     * @param height SVG height in pixels
     * @param theme Light or Dark theme
     * @return SVG string
     */
    public func heatmapSvg(width: Float64, height: Float64, theme: SvgTheme): String {
        unsafe {
            let svgPtr = eulumdat_heatmap_svg(this.handle, width, height, theme.toInt())
            if (svgPtr.isNull()) {
                return ""
            }
            let svg = cStringToString(svgPtr)
            eulumdat_string_free(svgPtr)
            return svg
        }
    }

    // ========================================================================
    // Export
    // ========================================================================

    /**
     * Export to LDT (EULUMDAT) format
     *
     * @return LDT format string
     */
    public func exportLdt(): String {
        unsafe {
            let ldtPtr = eulumdat_export_ldt(this.handle)
            if (ldtPtr.isNull()) {
                return ""
            }
            let ldt = cStringToString(ldtPtr)
            eulumdat_string_free(ldtPtr)
            return ldt
        }
    }

    /**
     * Export to IES format
     *
     * @return IES format string
     */
    public func exportIes(): String {
        unsafe {
            let iesPtr = eulumdat_export_ies(this.handle)
            if (iesPtr.isNull()) {
                return ""
            }
            let ies = cStringToString(iesPtr)
            eulumdat_string_free(iesPtr)
            return ies
        }
    }

    // ========================================================================
    // Validation
    // ========================================================================

    /**
     * Validate the photometric data
     *
     * @return List of validation warnings
     */
    public func validate(): ArrayList<ValidationWarning> {
        unsafe {
            let cList = eulumdat_validate(this.handle)
            var warnings = ArrayList<ValidationWarning>()

            if (!cList.data.isNull() && cList.len > 0) {
                for (i in 0..Int64(cList.len)) {
                    let cWarning = cList.data[i]
                    let warning = ValidationWarning(
                        code: cStringToString(cWarning.code),
                        message: cStringToString(cWarning.message),
                        severity: Severity.fromInt(cWarning.severity)
                    )
                    warnings.append(warning)
                }
                eulumdat_validation_list_free(cList)
            }

            return warnings
        }
    }

    /**
     * Check if the data has validation errors
     *
     * @return true if there are any errors
     */
    public func hasErrors(): Bool {
        let warnings = this.validate()
        for (w in warnings) {
            if (w.severity == Severity.Error) {
                return true
            }
        }
        return false
    }

    // ========================================================================
    // Intensity Sampling
    // ========================================================================

    /**
     * Sample intensity at any C and G angle using bilinear interpolation
     *
     * @param cAngle C-plane angle in degrees (0-360)
     * @param gAngle Gamma angle in degrees (0-180)
     * @return Intensity in cd/klm
     */
    public func sampleIntensity(cAngle: Float64, gAngle: Float64): Float64 {
        unsafe {
            return eulumdat_sample_intensity(this.handle, cAngle, gAngle)
        }
    }

    /**
     * Sample normalized intensity (0.0 to 1.0) at any C and G angle
     *
     * @param cAngle C-plane angle in degrees (0-360)
     * @param gAngle Gamma angle in degrees (0-180)
     * @return Normalized intensity (0.0 to 1.0)
     */
    public func sampleIntensityNormalized(cAngle: Float64, gAngle: Float64): Float64 {
        unsafe {
            return eulumdat_sample_intensity_normalized(this.handle, cAngle, gAngle)
        }
    }
}
