package eu.trahe.eulumdat.data

import android.content.Context
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.ui.graphics.vector.ImageVector

/**
 * Built-in luminaire templates loaded from bundled LDT files in assets.
 */
enum class LuminaireTemplate(
    val displayName: String,
    val description: String,
    val fileName: String,
    val icon: TemplateIcon
) {
    DOWNLIGHT(
        displayName = "Downlight",
        description = "Simple downlight with vertical axis symmetry",
        fileName = "1-1-0.ldt",
        icon = TemplateIcon.DOWNLIGHT
    ),
    PROJECTOR(
        displayName = "Projector",
        description = "CDM-TD 70W spotlight with asymmetric beam",
        fileName = "projector.ldt",
        icon = TemplateIcon.PROJECTOR
    ),
    LINEAR(
        displayName = "Linear Luminaire",
        description = "Linear luminaire with C0-C180 symmetry",
        fileName = "0-2-0.ldt",
        icon = TemplateIcon.LINEAR
    ),
    FLUORESCENT(
        displayName = "Fluorescent Luminaire",
        description = "T16 G5 54W linear luminaire with bilateral symmetry",
        fileName = "fluorescent_luminaire.ldt",
        icon = TemplateIcon.FLUORESCENT
    ),
    ROAD_LUMINAIRE(
        displayName = "Road Luminaire",
        description = "SON-TPP 250W street light with C90-C270 symmetry",
        fileName = "road_luminaire.ldt",
        icon = TemplateIcon.ROAD
    ),
    FLOOR_UPLIGHT(
        displayName = "Floor Uplight",
        description = "HIT-DE 250W floor-standing uplight",
        fileName = "floor_uplight.ldt",
        icon = TemplateIcon.UPLIGHT
    );

    /**
     * Load the LDT content from bundled assets.
     */
    fun loadContent(context: Context): String? {
        return try {
            context.assets.open("templates/$fileName").bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            null
        }
    }
}

/**
 * Icons for template types (using Material icons as approximations)
 */
enum class TemplateIcon {
    DOWNLIGHT,
    PROJECTOR,
    LINEAR,
    FLUORESCENT,
    ROAD,
    UPLIGHT
}

/**
 * Helper to get ImageVector for template icon
 */
fun TemplateIcon.toImageVector(): ImageVector = when (this) {
    TemplateIcon.DOWNLIGHT -> Icons.Default.Highlight
    TemplateIcon.PROJECTOR -> Icons.Default.FlashlightOn
    TemplateIcon.LINEAR -> Icons.Default.LinearScale
    TemplateIcon.FLUORESCENT -> Icons.Default.Lightbulb
    TemplateIcon.ROAD -> Icons.Default.WbTwilight
    TemplateIcon.UPLIGHT -> Icons.Default.VerticalAlignTop
}
