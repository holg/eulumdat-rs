package eu.trahe.eulumdat.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import eu.trahe.eulumdat.data.LdtData
import kotlinx.coroutines.delay
import kotlin.math.*

/**
 * 3D Point representation
 */
private data class Point3D(val x: Double, val y: Double, val z: Double) {
    fun rotateX(angle: Double): Point3D {
        val cosA = cos(angle)
        val sinA = sin(angle)
        return Point3D(
            x = x,
            y = y * cosA - z * sinA,
            z = y * sinA + z * cosA
        )
    }

    fun rotateY(angle: Double): Point3D {
        val cosA = cos(angle)
        val sinA = sin(angle)
        return Point3D(
            x = x * cosA + z * sinA,
            y = y,
            z = -x * sinA + z * cosA
        )
    }

    fun rotateZ(angle: Double): Point3D {
        val cosA = cos(angle)
        val sinA = sin(angle)
        return Point3D(
            x = x * cosA - y * sinA,
            y = x * sinA + y * cosA,
            z = z
        )
    }

    fun project(cx: Double, cy: Double, scale: Double): Offset {
        val perspective = 600.0
        val zOffset = 300.0
        val factor = perspective / (perspective + z + zOffset)
        return Offset(
            x = (cx + x * scale * factor).toFloat(),
            y = (cy - y * scale * factor).toFloat()
        )
    }
}

/**
 * Wing data for 3D rendering
 */
private data class Wing(
    val cAngle: Double,
    val points: List<Point3D>,
    val colorHue: Float
)

/**
 * 3D Butterfly Diagram View with touch rotation
 */
@Composable
fun Butterfly3DView(
    ldtData: LdtData,
    isDarkTheme: Boolean,
    modifier: Modifier = Modifier
) {
    var rotationX by remember { mutableStateOf(0.5) }
    var rotationY by remember { mutableStateOf(0.0) }
    var autoRotate by remember { mutableStateOf(true) }
    var isDragging by remember { mutableStateOf(false) }

    // Build wings from LDT data
    val wings = remember(ldtData) { buildWings(ldtData) }
    val maxIntensity = remember(ldtData) {
        ldtData.intensities.flatten().maxOrNull()?.coerceAtLeast(1.0) ?: 1.0
    }

    // Auto-rotation animation
    LaunchedEffect(autoRotate, isDragging) {
        while (autoRotate && !isDragging) {
            delay(16) // ~60fps
            rotationY += 0.01
        }
    }

    // Theme colors
    val bgColor = if (isDarkTheme) Color(0xFF1a1a2e) else Color.White
    val gridColor = if (isDarkTheme) Color(0xFF404060) else Color(0xFFe0e0e0)
    val textColor = if (isDarkTheme) Color(0xFFa0a0a0) else Color(0xFF666666)
    val centerColor = if (isDarkTheme) Color.White else Color(0xFF333333)

    Column(modifier = modifier.fillMaxSize()) {
        // Canvas for 3D rendering
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .background(bgColor)
                .pointerInput(Unit) {
                    detectDragGestures(
                        onDragStart = { isDragging = true },
                        onDragEnd = { isDragging = false },
                        onDragCancel = { isDragging = false },
                        onDrag = { change, dragAmount ->
                            change.consume()
                            rotationY += dragAmount.x * 0.005
                            rotationX += dragAmount.y * 0.005
                            rotationX = rotationX.coerceIn(-1.5, 1.5)
                        }
                    )
                }
        ) {
            Canvas(modifier = Modifier.fillMaxSize()) {
                val cx = size.width / 2
                val cy = size.height / 2
                val scale = minOf(size.width, size.height) / 2 * 0.7

                // Draw grid
                drawGrid(cx.toDouble(), cy.toDouble(), scale.toDouble(), rotationX, rotationY, gridColor)

                // Sort wings by depth for painter's algorithm
                val sortedWings = wings.map { wing ->
                    val avgZ = wing.points.map { p ->
                        p.rotateX(rotationX).rotateY(rotationY).z
                    }.average()
                    wing to avgZ
                }.sortedByDescending { it.second }.map { it.first }

                // Draw wings
                sortedWings.forEach { wing ->
                    drawWing(wing, cx.toDouble(), cy.toDouble(), scale.toDouble(), rotationX, rotationY)
                }

                // Draw center point
                drawCircle(
                    color = centerColor,
                    radius = 4f,
                    center = Offset(cx, cy)
                )
            }

            // Labels overlay
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(12.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        text = "3D Photometric Solid",
                        style = MaterialTheme.typography.labelMedium,
                        color = textColor
                    )
                    Text(
                        text = "Drag to rotate",
                        style = MaterialTheme.typography.labelSmall,
                        color = textColor.copy(alpha = 0.7f)
                    )
                }
                Spacer(modifier = Modifier.weight(1f))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        text = "Max: ${maxIntensity.toInt()} cd/klm",
                        style = MaterialTheme.typography.labelSmall,
                        color = textColor
                    )
                    Text(
                        text = "${ldtData.numCPlanes} C × ${ldtData.numGPlanes} γ",
                        style = MaterialTheme.typography.labelSmall,
                        color = textColor
                    )
                }
            }
        }

        // Controls
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Button(
                onClick = { autoRotate = !autoRotate },
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (autoRotate) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.secondary
                )
            ) {
                Text(if (autoRotate) "Pause" else "Auto")
            }
            OutlinedButton(
                onClick = {
                    rotationX = 0.5
                    rotationY = 0.0
                }
            ) {
                Text("Reset")
            }
        }
    }
}

/**
 * Build wing geometry from LDT data
 */
private fun buildWings(ldtData: LdtData): List<Wing> {
    if (ldtData.intensities.isEmpty() || ldtData.gAngles.isEmpty()) {
        return emptyList()
    }

    val maxIntensity = ldtData.intensities.flatten().maxOrNull()?.coerceAtLeast(1.0) ?: 1.0
    val cPlaneData = expandCPlanes(ldtData)

    return cPlaneData.map { (cAngle, intensities) ->
        val cRad = Math.toRadians(cAngle)
        val points = mutableListOf<Point3D>()

        // Start at center
        points.add(Point3D(0.0, 0.0, 0.0))

        // Build points along gamma angles
        ldtData.gAngles.forEachIndexed { j, gAngle ->
            val intensity = intensities.getOrElse(j) { 0.0 }
            val r = intensity / maxIntensity

            val gRad = Math.toRadians(gAngle)

            // Convert spherical to Cartesian
            val x = r * sin(gRad) * cos(cRad)
            val y = r * sin(gRad) * sin(cRad)
            val z = r * cos(gRad)

            points.add(Point3D(x, y, -z)) // Flip Z for display
        }

        val colorHue = ((cAngle / 360.0) * 240.0 + 180.0).toFloat() % 360f

        Wing(cAngle, points, colorHue)
    }
}

/**
 * Expand C-plane data based on symmetry
 */
private fun expandCPlanes(ldtData: LdtData): List<Pair<Double, List<Double>>> {
    if (ldtData.intensities.isEmpty() || ldtData.gAngles.isEmpty()) {
        return emptyList()
    }

    val result = mutableListOf<Pair<Double, List<Double>>>()

    when (ldtData.symmetry) {
        1 -> { // Vertical Axis
            val intensities = ldtData.intensities.firstOrNull() ?: return emptyList()
            for (i in 0 until 12) {
                val cAngle = i * 30.0
                result.add(cAngle to intensities)
            }
        }
        2 -> { // Plane C0-C180
            ldtData.intensities.forEachIndexed { i, intensities ->
                val cAngle = ldtData.cAngles.getOrElse(i) { 0.0 }
                result.add(cAngle to intensities)
                if (cAngle > 0 && cAngle < 180) {
                    result.add(360.0 - cAngle to intensities)
                }
            }
        }
        3 -> { // Plane C90-C270
            ldtData.intensities.forEachIndexed { i, intensities ->
                val cAngle = ldtData.cAngles.getOrElse(i) { 0.0 }
                result.add(cAngle to intensities)
                if (cAngle > 90 && cAngle < 270) {
                    val mirrored = if (cAngle < 180) 90.0 - (cAngle - 90.0) else 270.0 + (270.0 - cAngle)
                    if (mirrored in 0.0..360.0) {
                        result.add(mirrored to intensities)
                    }
                }
            }
        }
        4 -> { // Both Planes
            ldtData.intensities.forEachIndexed { i, intensities ->
                val cAngle = ldtData.cAngles.getOrElse(i) { 0.0 }
                result.add(cAngle to intensities)
                if (cAngle > 0 && cAngle < 90) {
                    result.add(180.0 - cAngle to intensities)
                    result.add(180.0 + cAngle to intensities)
                    result.add(360.0 - cAngle to intensities)
                } else if (abs(cAngle - 90.0) < 0.1) {
                    result.add(270.0 to intensities)
                }
            }
        }
        else -> { // None
            ldtData.intensities.forEachIndexed { i, intensities ->
                val cAngle = ldtData.cAngles.getOrElse(i) { 0.0 }
                result.add(cAngle to intensities)
            }
        }
    }

    return result.sortedBy { it.first }
}

/**
 * Draw grid circles and C-plane lines
 */
private fun DrawScope.drawGrid(
    cx: Double,
    cy: Double,
    scale: Double,
    rotationX: Double,
    rotationY: Double,
    gridColor: Color
) {
    // Draw concentric circles
    for (i in 1..4) {
        val r = i / 4.0
        val path = Path()
        var first = true

        for (j in 0..36) {
            val cAngle = j * 10.0
            val cRad = Math.toRadians(cAngle)

            val point = Point3D(r * cos(cRad), r * sin(cRad), 0.0)
                .rotateX(rotationX)
                .rotateY(rotationY)

            val projected = point.project(cx, cy, scale)

            if (first) {
                path.moveTo(projected.x, projected.y)
                first = false
            } else {
                path.lineTo(projected.x, projected.y)
            }
        }
        path.close()

        drawPath(path, gridColor, style = Stroke(width = 1f))
    }

    // Draw C-plane direction lines
    for (i in 0 until 8) {
        val cAngle = i * 45.0
        val cRad = Math.toRadians(cAngle)

        val p1 = Point3D(0.0, 0.0, 0.0)
            .rotateX(rotationX)
            .rotateY(rotationY)
            .project(cx, cy, scale)

        val p2 = Point3D(cos(cRad), sin(cRad), 0.0)
            .rotateX(rotationX)
            .rotateY(rotationY)
            .project(cx, cy, scale)

        drawLine(gridColor, p1, p2, strokeWidth = 1f)
    }
}

/**
 * Draw a single wing
 */
private fun DrawScope.drawWing(
    wing: Wing,
    cx: Double,
    cy: Double,
    scale: Double,
    rotationX: Double,
    rotationY: Double
) {
    if (wing.points.size < 2) return

    val path = Path()
    var first = true

    wing.points.forEach { point ->
        val rotated = point.rotateX(rotationX).rotateY(rotationY)
        val projected = rotated.project(cx, cy, scale)

        if (first) {
            path.moveTo(projected.x, projected.y)
            first = false
        } else {
            path.lineTo(projected.x, projected.y)
        }
    }
    path.close()

    // Fill with semi-transparent color
    val (r, g, b) = hslToRgb(wing.colorHue / 360f, 0.6f, 0.5f)
    val fillColor = Color(r, g, b, alpha = 0.5f)
    drawPath(path, fillColor)

    // Stroke with brighter color
    val (r2, g2, b2) = hslToRgb(wing.colorHue / 360f, 0.7f, 0.6f)
    val strokeColor = Color(r2, g2, b2)
    drawPath(path, strokeColor, style = Stroke(width = 1.5f))
}

/**
 * HSL to RGB conversion
 */
private fun hslToRgb(h: Float, s: Float, l: Float): Triple<Float, Float, Float> {
    val c = (1f - abs(2f * l - 1f)) * s
    val x = c * (1f - abs((h * 6f) % 2f - 1f))
    val m = l - c / 2f

    val (r, g, b) = when ((h * 6f).toInt()) {
        0 -> Triple(c, x, 0f)
        1 -> Triple(x, c, 0f)
        2 -> Triple(0f, c, x)
        3 -> Triple(0f, x, c)
        4 -> Triple(x, 0f, c)
        else -> Triple(c, 0f, x)
    }

    return Triple(r + m, g + m, b + m)
}
