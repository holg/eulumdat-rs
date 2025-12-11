/**
 * Eulumdat HarmonyOS - CLI Test Entry Point
 *
 * This is a standalone test application to verify the Cangjie FFI bindings
 * work correctly before integrating with DevEco Studio.
 */
package eulumdat_harmonyos

import eulumdat.*

main() {
    println("=== Eulumdat HarmonyOS (Cangjie) ===")
    println("")

    // Sample LDT content for testing
    let sampleLdt = """
Company Identification
1
4
1
13
5
3
Report No. 12345
Luminaire Name Test
Luminaire Number 001
File Name test.ldt
Date User 2024-01-01
100
50
80
80
40
0
0
0
0
0.85
0.75
1.0
0.0
1
1 100W LED 5000lm ww 1 100
1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0
0 90 180 270
0 5 10 15 20 25 30 35 40 45 50 55 60
500 495 485 470 450 425 395 360 320 275 225 170 110
500 490 475 455 430 400 365 325 280 230 175 115 50
500 495 485 470 450 425 395 360 320 275 225 170 110
500 490 475 455 430 400 365 325 280 230 175 115 50
"""

    let db = try {
        Eulumdat.parseLdt(sampleLdt)
    } catch (e: Exception) {
        println("ERROR: ${e.message}")
        return
    }

    // Display luminaire information
    let info = db.info()
    println("Luminaire Information:")
    println("  Name: ${info.luminaireName}")
    println("  Manufacturer: ${info.identification}")
    println("  Symmetry: ${info.symmetry.displayName()}")
    println("  Type: ${info.typeIndicator.displayName()}")
    println("  Dimensions: ${info.length}mm x ${info.width}mm x ${info.height}mm")
    println("  Max Intensity: ${formatNumber(info.maxIntensity, 1)} cd/klm")
    println("  Total Flux: ${formatNumber(info.totalLuminousFlux, 1)} lm")
    println("  Light Output Ratio: ${formatPercent(info.lightOutputRatio)}")
    println("")

    // Display lamp sets
    let lampSets = db.lampSets()
    println("Lamp Sets (${lampSets.size()}):")
    for (ls in lampSets) {
        println("  - ${ls.numLamps}x ${ls.lampType}")
        println("    Flux: ${formatNumber(ls.totalLuminousFlux, 0)} lm")
        println("    Power: ${formatNumber(ls.wattageWithBallast, 1)} W")
    }
    println("")

    // Validation
    let warnings = db.validate()
    println("Validation (${warnings.size()} issues):")
    if (warnings.isEmpty()) {
        println("  No issues found")
    } else {
        for (w in warnings) {
            let severityIcon = match (w.severity) {
                case Info => "INFO"
                case Warning => "WARN"
            }
            println("  [${severityIcon}] ${w.code}: ${w.message}")
        }
    }
    println("")

    // Sample intensity
    println("Intensity Samples:")
    println("  At C0, G0 (nadir): ${formatNumber(db.sampleIntensity(0.0, 0.0), 1)} cd/klm")
    println("  At C0, G45: ${formatNumber(db.sampleIntensity(0.0, 45.0), 1)} cd/klm")
    println("  At C0, G90 (horizontal): ${formatNumber(db.sampleIntensity(0.0, 90.0), 1)} cd/klm")
    println("")

    // Generate SVG (just show length to verify it works)
    let polarSvg = db.polarSvg(400.0, 400.0, SvgTheme.Light)
    println("SVG Generation:")
    println("  Polar diagram: ${polarSvg.size()} bytes")

    let cartesianSvg = db.cartesianSvg(500.0, 300.0, 8, SvgTheme.Light)
    println("  Cartesian diagram: ${cartesianSvg.size()} bytes")

    let butterflySvg = db.butterflySvg(400.0, 400.0, 60.0, SvgTheme.Dark)
    println("  Butterfly diagram: ${butterflySvg.size()} bytes")

    let heatmapSvg = db.heatmapSvg(400.0, 300.0, SvgTheme.Light)
    println("  Heatmap diagram: ${heatmapSvg.size()} bytes")
    println("")

    // Export
    let ldtExport = db.exportLdt()
    let iesExport = db.exportIes()
    println("Export:")
    println("  LDT format: ${ldtExport.size()} bytes")
    println("  IES format: ${iesExport.size()} bytes")
    println("")

    println("=== Test Complete ===")
}

// Helper functions
func formatNumber(value: Float64, decimals: Int32): String {
    if (decimals == 0) {
        return "${Int64(value)}"
    } else if (decimals == 1) {
        let rounded = Float64(Int64(value * 10.0)) / 10.0
        return "${rounded}"
    } else {
        let rounded = Float64(Int64(value * 100.0)) / 100.0
        return "${rounded}"
    }
}

func formatPercent(value: Float64): String {
    let percent = value * 100.0
    return "${formatNumber(percent, 1)}%"
}
