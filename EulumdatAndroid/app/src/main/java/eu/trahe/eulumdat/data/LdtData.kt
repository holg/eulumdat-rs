package eu.trahe.eulumdat.data

/**
 * Data class representing parsed LDT/IES file data.
 * This is a Kotlin-native representation used by the UI.
 */
data class LdtData(
    val luminaireName: String,
    val manufacturer: String,
    val catalogNumber: String,
    val maxIntensity: Double,
    val totalFlux: Double,
    val symmetry: Int,
    val typeIndicator: Int,
    val numCPlanes: Int,
    val numGPlanes: Int,
    val length: Double,
    val width: Double,
    val height: Double,
    val luminousLength: Double,
    val luminousWidth: Double,
    val luminousHeightC0: Double,
    val luminousHeightC90: Double,
    val luminousHeightC180: Double,
    val luminousHeightC270: Double,
    val cAngles: List<Double>,
    val gAngles: List<Double>,
    val intensities: List<List<Double>>,
    val lampSets: List<LampSet>,
    val warnings: List<ValidationIssue>,
    val errors: List<ValidationIssue>,
    // Internal reference to native data for SVG generation
    internal val nativeData: Any?
) {
    val symmetryDescription: String
        get() = when (symmetry) {
            0 -> "None"
            1 -> "Vertical Axis"
            2 -> "C0-C180 Plane"
            3 -> "C90-C270 Plane"
            4 -> "Both Planes"
            else -> "Unknown"
        }

    val typeDescription: String
        get() = when (typeIndicator) {
            1 -> "Point Source (Symmetric)"
            2 -> "Linear Luminaire"
            3 -> "Point Source (Other)"
            else -> "Unknown"
        }

    /**
     * Generate SVG diagram for the given type using native Rust library.
     */
    fun generateSvg(diagramType: DiagramType, isDark: Boolean): String? {
        // Use native library if available
        return generateSvgNative(diagramType, isDark) ?: createPlaceholderSvg(diagramType, isDark)
    }

    private fun createPlaceholderSvg(type: DiagramType, isDark: Boolean): String {
        val bg = if (isDark) "#0f172a" else "#ffffff"
        val fg = if (isDark) "#f1f5f9" else "#1e293b"
        val primary = if (isDark) "#60a5fa" else "#3b82f6"

        return """
            <svg viewBox="0 0 400 400" xmlns="http://www.w3.org/2000/svg">
                <rect width="400" height="400" fill="$bg"/>
                <circle cx="200" cy="200" r="150" fill="none" stroke="$primary" stroke-width="2"/>
                <circle cx="200" cy="200" r="100" fill="none" stroke="$primary" stroke-width="1" opacity="0.5"/>
                <circle cx="200" cy="200" r="50" fill="none" stroke="$primary" stroke-width="1" opacity="0.5"/>
                <line x1="50" y1="200" x2="350" y2="200" stroke="$fg" stroke-width="1" opacity="0.3"/>
                <line x1="200" y1="50" x2="200" y2="350" stroke="$fg" stroke-width="1" opacity="0.3"/>
                <text x="200" y="380" text-anchor="middle" fill="$fg" font-size="14">${type.label}</text>
                <text x="200" y="30" text-anchor="middle" fill="$fg" font-size="12">$luminaireName</text>
            </svg>
        """.trimIndent()
    }
}

data class LampSet(
    val quantity: Int,
    val lampType: String,
    val flux: Double,
    val colorTemp: String,
    val cri: String,
    val wattage: Double
)

data class ValidationIssue(
    val code: String,
    val message: String
)

enum class DiagramType(val label: String) {
    POLAR("Polar"),
    CARTESIAN("Cartesian"),
    BUTTERFLY("Butterfly"),
    BUTTERFLY_3D("3D"),
    HEATMAP("Heatmap"),
    BUG("BUG"),
    LCS("LCS")
}
