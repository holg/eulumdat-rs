/**
 * Eulumdat Types - Safe Cangjie wrappers for FFI types
 *
 * This module provides safe Cangjie classes that wrap the raw C structs.
 */
package eulumdat

import std.collection.ArrayList

// ============================================================================
// Enums
// ============================================================================

/** Luminaire symmetry type */
public enum Symmetry {
    | None            // Full 360 data
    | VerticalAxis    // Rotationally symmetric (C0 only)
    | PlaneC0C180     // Mirror across C0-C180
    | PlaneC90C270    // Mirror across C90-C270
    | BothPlanes      // Quarter data, mirror both

    public func displayName(): String {
        match (this) {
            case None => "None (Full 360)"
            case VerticalAxis => "Vertical Axis (C0 only)"
            case PlaneC0C180 => "Plane C0-C180"
            case PlaneC90C270 => "Plane C90-C270"
            case BothPlanes => "Both Planes (Quadrant)"
        }
    }

    public static func fromInt(value: Int32): Symmetry {
        match (value) {
            case 0 => Symmetry.None
            case 1 => Symmetry.VerticalAxis
            case 2 => Symmetry.PlaneC0C180
            case 3 => Symmetry.PlaneC90C270
            case 4 => Symmetry.BothPlanes
            case _ => Symmetry.None
        }
    }
}

/** Luminaire type indicator */
public enum TypeIndicator {
    | PointSourceSymmetric
    | Linear
    | PointSourceOther

    public func displayName(): String {
        match (this) {
            case PointSourceSymmetric => "Point Source (Symmetric)"
            case Linear => "Linear"
            case PointSourceOther => "Point Source (Other)"
        }
    }

    public static func fromInt(value: Int32): TypeIndicator {
        match (value) {
            case 0 => TypeIndicator.PointSourceSymmetric
            case 1 => TypeIndicator.Linear
            case 2 => TypeIndicator.PointSourceOther
            case _ => TypeIndicator.PointSourceSymmetric
        }
    }
}

/** Validation severity level */
public enum Severity {
    | Info      // Informational (not currently used)
    | Warning   // All validation issues are warnings

    public static func fromInt(value: Int32): Severity {
        match (value) {
            case 0 => Severity.Info
            case 1 => Severity.Warning
            case _ => Severity.Warning
        }
    }
}

/** SVG theme */
public enum SvgTheme {
    | Light
    | Dark

    public func toInt(): Int32 {
        match (this) {
            case Light => 0
            case Dark => 1
        }
    }
}

// ============================================================================
// Data classes
// ============================================================================

/** Basic luminaire information */
public class LuminaireInfo {
    public let luminaireName: String
    public let identification: String
    public let luminaireNumber: String
    public let fileName: String
    public let dateUser: String
    public let measurementReportNumber: String
    public let symmetry: Symmetry
    public let typeIndicator: TypeIndicator
    public let length: Float64
    public let width: Float64
    public let height: Float64
    public let luminousAreaLength: Float64
    public let luminousAreaWidth: Float64
    public let numCPlanes: UInt32
    public let numGPlanes: UInt32
    public let cPlaneDistance: Float64
    public let gPlaneDistance: Float64
    public let maxIntensity: Float64
    public let totalLuminousFlux: Float64
    public let downwardFluxFraction: Float64
    public let lightOutputRatio: Float64

    public init(
        luminaireName: String,
        identification: String,
        luminaireNumber: String,
        fileName: String,
        dateUser: String,
        measurementReportNumber: String,
        symmetry: Symmetry,
        typeIndicator: TypeIndicator,
        length: Float64,
        width: Float64,
        height: Float64,
        luminousAreaLength: Float64,
        luminousAreaWidth: Float64,
        numCPlanes: UInt32,
        numGPlanes: UInt32,
        cPlaneDistance: Float64,
        gPlaneDistance: Float64,
        maxIntensity: Float64,
        totalLuminousFlux: Float64,
        downwardFluxFraction: Float64,
        lightOutputRatio: Float64
    ) {
        this.luminaireName = luminaireName
        this.identification = identification
        this.luminaireNumber = luminaireNumber
        this.fileName = fileName
        this.dateUser = dateUser
        this.measurementReportNumber = measurementReportNumber
        this.symmetry = symmetry
        this.typeIndicator = typeIndicator
        this.length = length
        this.width = width
        this.height = height
        this.luminousAreaLength = luminousAreaLength
        this.luminousAreaWidth = luminousAreaWidth
        this.numCPlanes = numCPlanes
        this.numGPlanes = numGPlanes
        this.cPlaneDistance = cPlaneDistance
        this.gPlaneDistance = gPlaneDistance
        this.maxIntensity = maxIntensity
        this.totalLuminousFlux = totalLuminousFlux
        this.downwardFluxFraction = downwardFluxFraction
        this.lightOutputRatio = lightOutputRatio
    }

    /** Convert from C struct */
    internal static func fromC(c: CLuminaireInfo): LuminaireInfo {
        let info = LuminaireInfo(
            luminaireName: cStringToString(c.luminaire_name),
            identification: cStringToString(c.identification),
            luminaireNumber: cStringToString(c.luminaire_number),
            fileName: cStringToString(c.file_name),
            dateUser: cStringToString(c.date_user),
            measurementReportNumber: cStringToString(c.measurement_report_number),
            symmetry: Symmetry.fromInt(c.symmetry),
            typeIndicator: TypeIndicator.fromInt(c.type_indicator),
            length: c.length,
            width: c.width,
            height: c.height,
            luminousAreaLength: c.luminous_area_length,
            luminousAreaWidth: c.luminous_area_width,
            numCPlanes: c.num_c_planes,
            numGPlanes: c.num_g_planes,
            cPlaneDistance: c.c_plane_distance,
            gPlaneDistance: c.g_plane_distance,
            maxIntensity: c.max_intensity,
            totalLuminousFlux: c.total_luminous_flux,
            downwardFluxFraction: c.downward_flux_fraction,
            lightOutputRatio: c.light_output_ratio
        )

        // Free the C strings
        unsafe {
            eulumdat_string_free(c.luminaire_name)
            eulumdat_string_free(c.identification)
            eulumdat_string_free(c.luminaire_number)
            eulumdat_string_free(c.file_name)
            eulumdat_string_free(c.date_user)
            eulumdat_string_free(c.measurement_report_number)
        }

        return info
    }
}

/** Lamp set information */
public class LampSet {
    public let numLamps: Int32
    public let lampType: String
    public let totalLuminousFlux: Float64
    public let colorAppearance: String
    public let colorRenderingGroup: String
    public let wattageWithBallast: Float64

    public init(
        numLamps: Int32,
        lampType: String,
        totalLuminousFlux: Float64,
        colorAppearance: String,
        colorRenderingGroup: String,
        wattageWithBallast: Float64
    ) {
        this.numLamps = numLamps
        this.lampType = lampType
        this.totalLuminousFlux = totalLuminousFlux
        this.colorAppearance = colorAppearance
        this.colorRenderingGroup = colorRenderingGroup
        this.wattageWithBallast = wattageWithBallast
    }
}

/** Validation warning */
public class ValidationWarning {
    public let code: String
    public let message: String
    public let severity: Severity

    public init(code: String, message: String, severity: Severity) {
        this.code = code
        this.message = message
        this.severity = severity
    }
}

// ============================================================================
// String conversion helpers
// ============================================================================

/** Convert C string (null-terminated UTF-8) to Cangjie String */
internal func cStringToString(ptr: CPointer<UInt8>): String {
    if (ptr.isNull()) {
        return ""
    }

    unsafe {
        var bytes = ArrayList<UInt8>()
        var i: Int64 = 0

        while (true) {
            let ch = ptr[i]
            if (ch == 0) {
                break
            }
            bytes.append(ch)
            i += 1
        }

        if (bytes.isEmpty()) {
            return ""
        }

        return String.fromUtf8(bytes.toArray())
    }
}

/** Convert Cangjie String to C string (null-terminated UTF-8) */
internal func stringToCString(s: String): CPointer<UInt8> {
    let bytes = s.toUtf8()
    let len = bytes.size

    unsafe {
        // Allocate memory for string + null terminator
        let ptr = libc.malloc(UInt64(len + 1)) as CPointer<UInt8>
        if (ptr.isNull()) {
            return CPointer<UInt8>()
        }

        // Copy bytes
        for (i in 0..len) {
            ptr[Int64(i)] = bytes[i]
        }

        // Null terminator
        ptr[Int64(len)] = 0

        return ptr
    }
}

/** Free a C string allocated by stringToCString */
internal func freeCString(ptr: CPointer<UInt8>): Unit {
    if (!ptr.isNull()) {
        unsafe {
            libc.free(ptr as CPointer<Unit>)
        }
    }
}

// ============================================================================
// libc bindings for memory allocation
// ============================================================================

foreign func malloc(size: UInt64): CPointer<Unit>
foreign func free(ptr: CPointer<Unit>): Unit

/** Namespace for libc functions */
internal struct libc {
    static func malloc(size: UInt64): CPointer<Unit> {
        return malloc(size)
    }

    static func free(ptr: CPointer<Unit>): Unit {
        free(ptr)
    }
}
