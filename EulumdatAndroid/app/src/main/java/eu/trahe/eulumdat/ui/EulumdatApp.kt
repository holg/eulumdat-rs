@file:OptIn(ExperimentalMaterial3Api::class)

package eu.trahe.eulumdat.ui

import android.content.Context
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.border
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.runtime.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.decode.SvgDecoder
import coil.request.ImageRequest
import eu.trahe.eulumdat.data.LdtData
import eu.trahe.eulumdat.data.DiagramType
import eu.trahe.eulumdat.data.LdtRepository
import eu.trahe.eulumdat.data.LuminaireTemplate
import eu.trahe.eulumdat.data.toImageVector
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EulumdatApp(
    fileUri: Uri?,
    fileName: String?,
    onOpenFile: () -> Unit,
    onClearFile: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var ldtData by remember { mutableStateOf<LdtData?>(null) }
    var currentFileName by remember { mutableStateOf<String?>(null) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var selectedTab by remember { mutableStateOf(Tab.DIAGRAM) }
    var selectedDiagram by remember { mutableStateOf(DiagramType.POLAR) }
    var isDarkTheme by remember { mutableStateOf(false) }

    // Load file when URI changes
    LaunchedEffect(fileUri) {
        if (fileUri != null) {
            isLoading = true
            errorMessage = null
            try {
                ldtData = LdtRepository.loadFromUri(context, fileUri)
                currentFileName = fileName
            } catch (e: Exception) {
                errorMessage = e.message ?: "Failed to load file"
                ldtData = null
                currentFileName = null
            }
            isLoading = false
        }
    }

    // Function to load a template
    fun loadTemplate(template: LuminaireTemplate) {
        scope.launch {
            isLoading = true
            errorMessage = null
            try {
                ldtData = LdtRepository.loadTemplate(context, template)
                currentFileName = template.displayName
            } catch (e: Exception) {
                errorMessage = e.message ?: "Failed to load template"
                ldtData = null
                currentFileName = null
            }
            isLoading = false
        }
    }

    // Function to clear current file/template
    fun clearCurrent() {
        ldtData = null
        currentFileName = null
        errorMessage = null
        onClearFile()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = currentFileName ?: "Eulumdat",
                            style = MaterialTheme.typography.titleMedium
                        )
                        if (ldtData != null) {
                            Text(
                                text = ldtData!!.luminaireName,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                },
                actions = {
                    IconButton(onClick = { isDarkTheme = !isDarkTheme }) {
                        Icon(
                            imageVector = if (isDarkTheme) Icons.Default.LightMode else Icons.Default.DarkMode,
                            contentDescription = "Toggle theme"
                        )
                    }
                    IconButton(onClick = onOpenFile) {
                        Icon(Icons.Default.FolderOpen, contentDescription = "Open file")
                    }
                    if (ldtData != null) {
                        IconButton(onClick = { clearCurrent() }) {
                            Icon(Icons.Default.Close, contentDescription = "Close file")
                        }
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when {
                isLoading -> {
                    LoadingState()
                }
                errorMessage != null -> {
                    ErrorState(message = errorMessage!!, onRetry = onOpenFile)
                }
                ldtData == null -> {
                    EmptyState(
                        onOpenFile = onOpenFile,
                        onTemplateSelected = { loadTemplate(it) }
                    )
                }
                else -> {
                    // Tab bar
                    TabRow(selectedTab, ldtData!!) { tab ->
                        selectedTab = tab
                    }

                    // Content
                    when (selectedTab) {
                        Tab.DIAGRAM -> DiagramTab(
                            ldtData = ldtData!!,
                            selectedDiagram = selectedDiagram,
                            isDarkTheme = isDarkTheme,
                            onDiagramSelected = { selectedDiagram = it }
                        )
                        Tab.GENERAL -> GeneralTab(ldtData!!)
                        Tab.DIMENSIONS -> DimensionsTab(ldtData!!)
                        Tab.LAMPS -> LampsTab(ldtData!!)
                        Tab.INTENSITY -> IntensityTab(ldtData!!)
                        Tab.VALIDATION -> ValidationTab(ldtData!!)
                    }
                }
            }
        }
    }
}

enum class Tab(val title: String, val icon: @Composable () -> Unit) {
    DIAGRAM("Diagram", { Icon(Icons.Default.ShowChart, null) }),
    GENERAL("General", { Icon(Icons.Default.Info, null) }),
    DIMENSIONS("Dimensions", { Icon(Icons.Default.Straighten, null) }),
    LAMPS("Lamps", { Icon(Icons.Default.Lightbulb, null) }),
    INTENSITY("Intensity", { Icon(Icons.Default.GridOn, null) }),
    VALIDATION("Validation", { Icon(Icons.Default.CheckCircle, null) })
}

@Composable
private fun TabRow(
    selectedTab: Tab,
    @Suppress("UNUSED_PARAMETER") ldtData: LdtData,
    onTabSelected: (Tab) -> Unit
) {
    ScrollableTabRow(
        selectedTabIndex = Tab.entries.indexOf(selectedTab),
        edgePadding = 8.dp
    ) {
        Tab.entries.forEach { tab ->
            Tab(
                selected = selectedTab == tab,
                onClick = { onTabSelected(tab) },
                text = { Text(tab.title) },
                icon = tab.icon
            )
        }
    }
}

@Composable
private fun EmptyState(
    onOpenFile: () -> Unit,
    onTemplateSelected: (LuminaireTemplate) -> Unit
) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // Header section
        item {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.padding(vertical = 24.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Lightbulb,
                    contentDescription = null,
                    modifier = Modifier.size(64.dp),
                    tint = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = "Eulumdat Viewer",
                    style = MaterialTheme.typography.headlineMedium,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Open an LDT/IES file or try a template",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(16.dp))
                FilledTonalButton(onClick = onOpenFile) {
                    Icon(Icons.Default.FolderOpen, contentDescription = null)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Open File")
                }
            }
        }

        // Templates section header
        item {
            Text(
                text = "Sample Templates",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp)
            )
        }

        // Template cards
        items(LuminaireTemplate.entries.size) { index ->
            val template = LuminaireTemplate.entries[index]
            TemplateCard(
                template = template,
                onClick = { onTemplateSelected(template) }
            )
        }
    }
}

@Composable
private fun TemplateCard(
    template: LuminaireTemplate,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() },
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Icon
            Surface(
                shape = RoundedCornerShape(8.dp),
                color = MaterialTheme.colorScheme.primary.copy(alpha = 0.1f),
                modifier = Modifier.size(48.dp)
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(
                        imageVector = template.icon.toImageVector(),
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(24.dp)
                    )
                }
            }

            // Text content
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = template.displayName,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = template.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            // Arrow
            Icon(
                imageVector = Icons.Default.ChevronRight,
                contentDescription = "Open",
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun LoadingState() {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            CircularProgressIndicator()
            Text(
                text = "Loading...",
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}

@Composable
private fun ErrorState(message: String, onRetry: () -> Unit) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Icon(
                imageVector = Icons.Default.Error,
                contentDescription = null,
                modifier = Modifier.size(64.dp),
                tint = MaterialTheme.colorScheme.error
            )
            Text(
                text = "Error",
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.error
            )
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 32.dp)
            )
            FilledTonalButton(onClick = onRetry) {
                Icon(Icons.Default.Refresh, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Try Again")
            }
        }
    }
}

@Composable
private fun DiagramTab(
    ldtData: LdtData,
    selectedDiagram: DiagramType,
    isDarkTheme: Boolean,
    onDiagramSelected: (DiagramType) -> Unit
) {
    Column(modifier = Modifier.fillMaxSize()) {
        // Diagram type selector
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            DiagramType.entries.forEach { type ->
                FilterChip(
                    selected = selectedDiagram == type,
                    onClick = { onDiagramSelected(type) },
                    label = { Text(type.label) }
                )
            }
        }

        // Diagram content
        if (selectedDiagram == DiagramType.BUTTERFLY_3D) {
            // Real 3D interactive view
            Butterfly3DView(
                ldtData = ldtData,
                isDarkTheme = isDarkTheme,
                modifier = Modifier.fillMaxSize()
            )
        } else {
            // SVG diagram
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp),
                contentAlignment = Alignment.Center
            ) {
                val svgContent = remember(ldtData, selectedDiagram, isDarkTheme) {
                    ldtData.generateSvg(selectedDiagram, isDarkTheme)
                }

                if (svgContent != null) {
                    AsyncImage(
                        model = ImageRequest.Builder(LocalContext.current)
                            .data(svgContent.toByteArray(Charsets.UTF_8))
                            .decoderFactory(SvgDecoder.Factory())
                            .build(),
                        contentDescription = "${selectedDiagram.label} diagram",
                        modifier = Modifier.fillMaxSize()
                    )
                } else {
                    Text(
                        text = "Failed to generate diagram",
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

@Composable
private fun GeneralTab(ldtData: LdtData) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item { SectionHeader("Identification") }
        item { InfoRow("Luminaire Name", ldtData.luminaireName) }
        item { InfoRow("Manufacturer", ldtData.manufacturer) }
        item { InfoRow("Catalog Number", ldtData.catalogNumber) }

        item { Spacer(modifier = Modifier.height(16.dp)) }
        item { SectionHeader("Photometry") }
        item { InfoRow("Max Intensity", "${ldtData.maxIntensity.format(1)} cd/klm") }
        item { InfoRow("Total Luminous Flux", "${ldtData.totalFlux.format(0)} lm") }
        item { InfoRow("Symmetry", ldtData.symmetryDescription) }
        item { InfoRow("Type", ldtData.typeDescription) }

        item { Spacer(modifier = Modifier.height(16.dp)) }
        item { SectionHeader("Angles") }
        item { InfoRow("C-Planes", "${ldtData.numCPlanes}") }
        item { InfoRow("γ-Angles", "${ldtData.numGPlanes}") }
    }
}

@Composable
private fun DimensionsTab(ldtData: LdtData) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item { SectionHeader("Luminaire Dimensions") }
        item { InfoRow("Length", "${ldtData.length.format(0)} mm") }
        item { InfoRow("Width", "${ldtData.width.format(0)} mm") }
        item { InfoRow("Height", "${ldtData.height.format(0)} mm") }

        item { Spacer(modifier = Modifier.height(16.dp)) }
        item { SectionHeader("Luminous Area") }
        item { InfoRow("Length", "${ldtData.luminousLength.format(0)} mm") }
        item { InfoRow("Width", "${ldtData.luminousWidth.format(0)} mm") }
        item { InfoRow("Height C0", "${ldtData.luminousHeightC0.format(0)} mm") }
        item { InfoRow("Height C90", "${ldtData.luminousHeightC90.format(0)} mm") }
        item { InfoRow("Height C180", "${ldtData.luminousHeightC180.format(0)} mm") }
        item { InfoRow("Height C270", "${ldtData.luminousHeightC270.format(0)} mm") }
    }
}

@Composable
private fun LampsTab(ldtData: LdtData) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item { SectionHeader("Lamp Sets (${ldtData.lampSets.size})") }

        ldtData.lampSets.forEachIndexed { index, lamp ->
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant
                    )
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Text(
                            text = "Lamp Set ${index + 1}",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold
                        )
                        InfoRow("Quantity", "${lamp.quantity}")
                        InfoRow("Type", lamp.lampType)
                        InfoRow("Flux", "${lamp.flux.format(0)} lm")
                        InfoRow("Color Temperature", "${lamp.colorTemp} K")
                        InfoRow("CRI", lamp.cri)
                        InfoRow("Power", "${lamp.wattage.format(1)} W")
                    }
                }
            }
        }
    }
}

@Composable
private fun IntensityTab(ldtData: LdtData) {
    Column(modifier = Modifier.fillMaxSize()) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Intensities (cd/klm)",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = "Max: ${ldtData.maxIntensity.format(1)} cd/klm",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }

        // Heatmap grid
        if (ldtData.intensities.isNotEmpty() && ldtData.cAngles.isNotEmpty() && ldtData.gAngles.isNotEmpty()) {
            IntensityHeatmapGrid(ldtData)
        } else {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        Icons.Default.GridOn,
                        contentDescription = null,
                        modifier = Modifier.size(48.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "No Intensity Data",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        // Footer with legend
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "${ldtData.numCPlanes} C-planes × ${ldtData.numGPlanes} γ-angles",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            // Color scale legend
            Row(
                horizontalArrangement = Arrangement.spacedBy(2.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("0", style = MaterialTheme.typography.labelSmall)
                (0..9).forEach { i ->
                    Box(
                        modifier = Modifier
                            .size(12.dp)
                            .background(heatmapColor(i / 9.0))
                    )
                }
                Text("max", style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}

@Composable
private fun IntensityHeatmapGrid(ldtData: LdtData) {
    val cellWidth = 56.dp
    val cellHeight = 28.dp
    val headerWidth = 48.dp

    Box(
        modifier = Modifier
            .fillMaxSize()
            .horizontalScroll(rememberScrollState())
    ) {
        LazyColumn(
            modifier = Modifier.fillMaxSize()
        ) {
            // Header row with C-angles
            item {
                Row {
                    // Corner cell
                    Box(
                        modifier = Modifier
                            .width(headerWidth)
                            .height(cellHeight)
                            .background(MaterialTheme.colorScheme.surfaceVariant)
                            .border(0.5.dp, MaterialTheme.colorScheme.outline),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "γ\\C",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.SemiBold
                        )
                    }
                    // C-angle headers
                    ldtData.cAngles.forEach { cAngle ->
                        Box(
                            modifier = Modifier
                                .width(cellWidth)
                                .height(cellHeight)
                                .background(MaterialTheme.colorScheme.surfaceVariant)
                                .border(0.5.dp, MaterialTheme.colorScheme.outline),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = "C${cAngle.toInt()}°",
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.Medium
                            )
                        }
                    }
                }
            }

            // Data rows
            items(ldtData.gAngles.size) { gIndex ->
                val gAngle = ldtData.gAngles[gIndex]
                Row {
                    // Row header (γ-angle)
                    Box(
                        modifier = Modifier
                            .width(headerWidth)
                            .height(cellHeight)
                            .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.7f))
                            .border(0.5.dp, MaterialTheme.colorScheme.outline),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "${gAngle.toInt()}°",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Medium
                        )
                    }
                    // Intensity values
                    ldtData.cAngles.forEachIndexed { cIndex, _ ->
                        val intensity = if (cIndex < ldtData.intensities.size && gIndex < ldtData.intensities[cIndex].size) {
                            ldtData.intensities[cIndex][gIndex]
                        } else 0.0
                        val normalized = if (ldtData.maxIntensity > 0) intensity / ldtData.maxIntensity else 0.0
                        val bgColor = heatmapColor(normalized)
                        val textColor = if (normalized > 0.5) Color.White else MaterialTheme.colorScheme.onSurface

                        Box(
                            modifier = Modifier
                                .width(cellWidth)
                                .height(cellHeight)
                                .background(bgColor)
                                .border(0.5.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = formatIntensity(intensity),
                                style = MaterialTheme.typography.labelSmall,
                                color = textColor
                            )
                        }
                    }
                }
            }
        }
    }
}

private fun formatIntensity(value: Double): String {
    return when {
        value >= 1000 -> "%.0f".format(value)
        value >= 100 -> "%.0f".format(value)
        else -> "%.1f".format(value)
    }
}

private fun heatmapColor(normalized: Double): Color {
    val value = normalized.coerceIn(0.0, 1.0)
    val r: Float
    val g: Float
    val b: Float

    when {
        value < 0.25 -> {
            val t = (value / 0.25).toFloat()
            r = 0f; g = t; b = 1f
        }
        value < 0.5 -> {
            val t = ((value - 0.25) / 0.25).toFloat()
            r = 0f; g = 1f; b = 1f - t
        }
        value < 0.75 -> {
            val t = ((value - 0.5) / 0.25).toFloat()
            r = t; g = 1f; b = 0f
        }
        else -> {
            val t = ((value - 0.75) / 0.25).toFloat()
            r = 1f; g = 1f - t; b = 0f
        }
    }

    return Color(r, g, b)
}

@Composable
private fun ValidationTab(ldtData: LdtData) {
    val warnings = ldtData.warnings
    val errors = ldtData.errors

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        // Status header
        item {
            val (icon, color, text) = when {
                errors.isNotEmpty() -> Triple(Icons.Default.Error, MaterialTheme.colorScheme.error, "Validation Failed")
                warnings.isNotEmpty() -> Triple(Icons.Default.Warning, MaterialTheme.colorScheme.tertiary, "Passed with Warnings")
                else -> Triple(Icons.Default.CheckCircle, MaterialTheme.colorScheme.primary, "Validation Passed")
            }

            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = color.copy(alpha = 0.1f)
                )
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Icon(icon, null, tint = color)
                    Text(text, fontWeight = FontWeight.Bold, color = color)
                }
            }
        }

        if (errors.isNotEmpty()) {
            item { SectionHeader("Errors (${errors.size})") }
            errors.forEach { error ->
                item {
                    ValidationItem(
                        code = error.code,
                        message = error.message,
                        isError = true
                    )
                }
            }
        }

        if (warnings.isNotEmpty()) {
            item { SectionHeader("Warnings (${warnings.size})") }
            warnings.forEach { warning ->
                item {
                    ValidationItem(
                        code = warning.code,
                        message = warning.message,
                        isError = false
                    )
                }
            }
        }

        if (errors.isEmpty() && warnings.isEmpty()) {
            item {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(32.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Icon(
                            Icons.Default.CheckCircle,
                            null,
                            modifier = Modifier.size(64.dp),
                            tint = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text("No issues found", style = MaterialTheme.typography.titleMedium)
                    }
                }
            }
        }
    }
}

@Composable
private fun ValidationItem(code: String, message: String, isError: Boolean) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(
                if (isError) MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f)
                else MaterialTheme.colorScheme.tertiaryContainer.copy(alpha = 0.3f)
            )
            .padding(12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Icon(
            if (isError) Icons.Default.Error else Icons.Default.Warning,
            null,
            tint = if (isError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.tertiary
        )
        Column {
            Text(
                text = code,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(text = message, style = MaterialTheme.typography.bodyMedium)
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleSmall,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(vertical = 8.dp)
    )
}

@Composable
private fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium
        )
    }
}

private fun Double.format(decimals: Int): String = "%.${decimals}f".format(this)
