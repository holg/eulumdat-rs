package eu.trahe.eulumdat.data

import android.content.Context
import android.net.Uri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.eulumdat_ffi.*
import java.io.BufferedReader
import java.io.InputStreamReader

/**
 * Repository for loading and parsing LDT/IES files using the native Rust library.
 */
object LdtRepository {

    /**
     * Load a built-in template from assets.
     */
    suspend fun loadTemplate(context: Context, template: LuminaireTemplate): LdtData = withContext(Dispatchers.IO) {
        val content = template.loadContent(context)
            ?: throw IllegalArgumentException("Cannot load template: ${template.displayName}")

        val nativeLdt = parseLdt(content)
        val warnings = validateLdt(nativeLdt)
        val errors = getValidationErrors(nativeLdt)

        convertToLdtData(nativeLdt, warnings, errors)
    }

    /**
     * Load LDT/IES file from a content URI using the native Rust parser.
     */
    suspend fun loadFromUri(context: Context, uri: Uri): LdtData = withContext(Dispatchers.IO) {
        val content = readFileContent(context, uri)
        val extension = uri.lastPathSegment?.substringAfterLast('.', "ldt")?.lowercase() ?: "ldt"

        val nativeLdt = when (extension) {
            "ies" -> parseIes(content)
            else -> parseLdt(content)
        }

        // Get validation warnings and errors
        val warnings = validateLdt(nativeLdt)
        val errors = getValidationErrors(nativeLdt)

        convertToLdtData(nativeLdt, warnings, errors)
    }

    private fun readFileContent(context: Context, uri: Uri): String {
        return context.contentResolver.openInputStream(uri)?.use { inputStream ->
            BufferedReader(InputStreamReader(inputStream, Charsets.ISO_8859_1)).readText()
        } ?: throw IllegalArgumentException("Cannot read file")
    }

    /**
     * Convert native Eulumdat to our LdtData class
     */
    private fun convertToLdtData(
        native: Eulumdat,
        warnings: List<ValidationWarning>,
        errors: List<uniffi.eulumdat_ffi.ValidationError>
    ): LdtData {
        // Calculate max intensity from intensities array
        val maxIntensity = native.intensities.flatten().maxOrNull() ?: 0.0

        // Calculate total flux from lamp sets
        val totalFlux = native.lampSets.sumOf { it.totalLuminousFlux * it.numLamps }

        return LdtData(
            luminaireName = native.luminaireName,
            manufacturer = native.identification,
            catalogNumber = native.luminaireNumber,
            maxIntensity = maxIntensity,
            totalFlux = totalFlux,
            symmetry = native.symmetry.ordinal,
            typeIndicator = native.typeIndicator.ordinal + 1,
            numCPlanes = native.numCPlanes.toInt(),
            numGPlanes = native.numGPlanes.toInt(),
            length = native.length,
            width = native.width,
            height = native.height,
            luminousLength = native.luminousAreaLength,
            luminousWidth = native.luminousAreaWidth,
            luminousHeightC0 = native.heightC0,
            luminousHeightC90 = native.heightC90,
            luminousHeightC180 = native.heightC180,
            luminousHeightC270 = native.heightC270,
            cAngles = native.cAngles,
            gAngles = native.gAngles,
            intensities = native.intensities,
            lampSets = native.lampSets.map { lamp ->
                LampSet(
                    quantity = lamp.numLamps,
                    lampType = lamp.lampType,
                    flux = lamp.totalLuminousFlux,
                    colorTemp = lamp.colorAppearance,
                    cri = lamp.colorRenderingGroup,
                    wattage = lamp.wattageWithBallast
                )
            },
            warnings = warnings.map { ValidationIssue(it.code, it.message) },
            errors = errors.map { ValidationIssue(it.code, it.message) },
            nativeData = native
        )
    }
}

/**
 * Extension to generate SVG from LdtData using native library
 */
fun LdtData.generateSvgNative(diagramType: DiagramType, isDark: Boolean): String? {
    val native = nativeData as? Eulumdat ?: return null
    val theme = if (isDark) SvgThemeType.DARK else SvgThemeType.LIGHT
    val size = 500.0

    return try {
        when (diagramType) {
            DiagramType.POLAR -> generatePolarSvg(native, size, size, theme)
            DiagramType.CARTESIAN -> generateCartesianSvg(native, size, size * 0.75, 8u, theme)
            DiagramType.BUTTERFLY -> generateButterflySvg(native, size, size * 0.8, 60.0, theme)
            DiagramType.BUTTERFLY_3D -> null // Handled by Butterfly3DView composable
            DiagramType.HEATMAP -> generateHeatmapSvg(native, size, size * 0.7, theme)
            DiagramType.BUG -> generateBugSvg(native, size, size * 0.85, theme)
            DiagramType.LCS -> generateLcsSvg(native, size, size, theme)
        }
    } catch (e: Exception) {
        null
    }
}
