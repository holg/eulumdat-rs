//
//  PreviewProvider.swift
//  LdtPreview
//
//  Created by Holger Trahe on 06.12.25.
//

import Quartz
import AppKit
import EulumdatKit

class PreviewProvider: QLPreviewProvider, QLPreviewingController {

    func providePreview(for request: QLFilePreviewRequest) async throws -> QLPreviewReply {
        let fileExtension = request.fileURL.pathExtension.lowercased()
        let previewSize = CGSize(width: 800, height: 900)

        // Try SVG directly first
        do {
            // Read file content - try ISO Latin-1 first (common for LDT), then UTF-8
            let content: String
            if let isoContent = try? String(contentsOf: request.fileURL, encoding: .isoLatin1) {
                content = isoContent
            } else {
                content = try String(contentsOf: request.fileURL, encoding: .utf8)
            }

            // Parse the photometric file
            let eulumdat: Eulumdat

            if fileExtension == "ldt" {
                eulumdat = try parseLdt(content: content)
            } else if fileExtension == "ies" {
                eulumdat = try parseIes(content: content)
            } else {
                throw PreviewError.unsupportedFormat
            }

            // Generate combined SVG with info panel and polar diagram
            let svgString = generatePreviewSvg(eulumdat: eulumdat, fileName: request.fileURL.lastPathComponent)

            // Return SVG directly - QuickLook can render SVG natively on macOS
            guard let svgData = svgString.data(using: .utf8) else {
                throw PreviewError.renderingFailed
            }

            return QLPreviewReply(dataOfContentType: UTType.svg, contentSize: previewSize) { reply in
                reply.stringEncoding = .utf8
                return svgData
            }

        } catch let error {
            // Return error as PNG
            return QLPreviewReply(dataOfContentType: .png, contentSize: previewSize) { reply in
                return self.createErrorImage(
                    message: "Preview Error",
                    detail: "\(error)",
                    size: previewSize
                )
            }
        }
    }

    // MARK: - Preview SVG Generation

    private func generatePreviewSvg(eulumdat: Eulumdat, fileName: String) -> String {
        let width = 800
        let height = 900
        let diagramSize = 650
        let infoHeight = 100
        let padding = 20

        // Generate the polar diagram SVG (we'll extract just the content)
        let polarSvg = generatePolarSvg(ldt: eulumdat, width: Double(diagramSize), height: Double(diagramSize), theme: .light)

        // Extract SVG content (remove outer svg tags to embed)
        let polarContent = extractSvgContent(from: polarSvg)

        // Format values
        let maxIntensity = String(format: "%.0f", eulumdat.maxIntensity)
        let totalFlux = String(format: "%.0f", eulumdat.totalLuminousFlux)
        let luminaireName = escapeXml(eulumdat.luminaireName)
        let manufacturer = escapeXml(eulumdat.identification)
        let fileNameEscaped = escapeXml(fileName)

        // Determine symmetry description
        let symmetryDesc: String
        switch eulumdat.symmetry {
        case .none: symmetryDesc = "Full (C0-C360)"
        case .verticalAxis: symmetryDesc = "Rotational"
        case .planeC0c180: symmetryDesc = "Plane C0-C180"
        case .planeC90c270: symmetryDesc = "Plane C90-C270"
        case .bothPlanes: symmetryDesc = "Quadrant"
        }

        // Calculate diagram offset to center it
        let diagramX = (width - diagramSize) / 2
        let diagramY = infoHeight + padding

        return """
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 \(width) \(height)" width="\(width)" height="\(height)">
          <defs>
            <style>
              .info-bg { fill: #f8f9fa; }
              .info-border { fill: none; stroke: #dee2e6; stroke-width: 1; }
              .title { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif; font-size: 18px; font-weight: 600; fill: #212529; }
              .subtitle { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; font-size: 13px; fill: #6c757d; }
              .label { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; font-size: 11px; fill: #868e96; }
              .value { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Display', sans-serif; font-size: 15px; font-weight: 500; fill: #212529; }
              .unit { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; font-size: 11px; fill: #868e96; }
              .filename { font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', sans-serif; font-size: 10px; fill: #adb5bd; }
              .metric-box { fill: #ffffff; stroke: #e9ecef; stroke-width: 1; rx: 6; }
            </style>
          </defs>

          <!-- Background -->
          <rect width="\(width)" height="\(height)" fill="#ffffff"/>

          <!-- Info Panel Background -->
          <rect x="0" y="0" width="\(width)" height="\(infoHeight)" class="info-bg"/>
          <line x1="0" y1="\(infoHeight)" x2="\(width)" y2="\(infoHeight)" stroke="#dee2e6" stroke-width="1"/>

          <!-- Luminaire Info (left side) -->
          <g transform="translate(\(padding), \(padding))">
            <text class="title" y="18">\(luminaireName.isEmpty ? "Unknown Luminaire" : luminaireName)</text>
            <text class="subtitle" y="38">\(manufacturer.isEmpty ? "Unknown Manufacturer" : manufacturer)</text>
            <text class="filename" y="55">\(fileNameEscaped)</text>
          </g>

          <!-- Metrics (right side) -->
          <g transform="translate(\(width - 320), \(padding - 5))">
            <!-- Max Intensity Box -->
            <rect x="0" y="0" width="90" height="70" class="metric-box"/>
            <text class="label" x="45" y="18" text-anchor="middle">Max Intensity</text>
            <text class="value" x="45" y="42" text-anchor="middle">\(maxIntensity)</text>
            <text class="unit" x="45" y="58" text-anchor="middle">cd/klm</text>

            <!-- Total Flux Box -->
            <rect x="100" y="0" width="90" height="70" class="metric-box"/>
            <text class="label" x="145" y="18" text-anchor="middle">Total Flux</text>
            <text class="value" x="145" y="42" text-anchor="middle">\(totalFlux)</text>
            <text class="unit" x="145" y="58" text-anchor="middle">lm</text>

            <!-- Symmetry Box -->
            <rect x="200" y="0" width="90" height="70" class="metric-box"/>
            <text class="label" x="245" y="18" text-anchor="middle">Symmetry</text>
            <text class="value" x="245" y="42" text-anchor="middle" style="font-size: 12px;">\(symmetryDesc)</text>
            <text class="unit" x="245" y="58" text-anchor="middle">\(eulumdat.numCPlanes)C × \(eulumdat.numGPlanes)γ</text>
          </g>

          <!-- Polar Diagram -->
          <g transform="translate(\(diagramX), \(diagramY))">
            \(polarContent)
          </g>
        </svg>
        """
    }

    private func extractSvgContent(from svg: String) -> String {
        // Remove the outer <svg> tags to get just the content
        var content = svg

        // Remove opening svg tag
        if let startRange = content.range(of: "<svg[^>]*>", options: .regularExpression) {
            content = String(content[startRange.upperBound...])
        }

        // Remove closing svg tag
        if let endRange = content.range(of: "</svg>", options: .backwards) {
            content = String(content[..<endRange.lowerBound])
        }

        return content
    }

    private func escapeXml(_ string: String) -> String {
        var result = string
        result = result.replacingOccurrences(of: "&", with: "&amp;")
        result = result.replacingOccurrences(of: "<", with: "&lt;")
        result = result.replacingOccurrences(of: ">", with: "&gt;")
        result = result.replacingOccurrences(of: "\"", with: "&quot;")
        result = result.replacingOccurrences(of: "'", with: "&apos;")
        return result
    }

    // MARK: - SVG Rendering

    private func renderSVGToPNG(svgString: String, size: CGSize) -> Data? {
        // Create temporary SVG file
        let tempDir = FileManager.default.temporaryDirectory
        let svgURL = tempDir.appendingPathComponent(UUID().uuidString).appendingPathExtension("svg")
        let pngURL = tempDir.appendingPathComponent(UUID().uuidString).appendingPathExtension("png")

        defer {
            try? FileManager.default.removeItem(at: svgURL)
            try? FileManager.default.removeItem(at: pngURL)
        }

        do {
            // Write SVG to temp file
            try svgString.write(to: svgURL, atomically: true, encoding: .utf8)

            // Use rsvg-convert if available (most reliable)
            if let rsvgPath = findRsvgConvert() {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: rsvgPath)
                process.arguments = [
                    "-w", String(Int(size.width)),
                    "-h", String(Int(size.height)),
                    "-o", pngURL.path,
                    svgURL.path
                ]

                try process.run()
                process.waitUntilExit()

                if process.terminationStatus == 0 {
                    return try Data(contentsOf: pngURL)
                }
            }

            // Fallback: Try using NSImage (limited SVG support)
            return renderSVGWithNSImage(svgString: svgString, size: size)

        } catch {
            return nil
        }
    }

    private func findRsvgConvert() -> String? {
        let paths = [
            "/opt/homebrew/bin/rsvg-convert",
            "/usr/local/bin/rsvg-convert",
            "/opt/local/bin/rsvg-convert"
        ]

        for path in paths {
            if FileManager.default.fileExists(atPath: path) {
                return path
            }
        }

        return nil
    }

    private func renderSVGWithNSImage(svgString: String, size: CGSize) -> Data? {
        guard let svgData = svgString.data(using: .utf8) else { return nil }

        // Create bitmap context
        guard let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int(size.width),
            pixelsHigh: Int(size.height),
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ) else { return nil }

        bitmap.size = size

        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)

        // White background
        NSColor.white.setFill()
        NSRect(origin: .zero, size: size).fill()

        // Try to render SVG
        if let nsImage = NSImage(data: svgData) {
            nsImage.draw(in: NSRect(origin: .zero, size: size))
        }

        NSGraphicsContext.restoreGraphicsState()

        return bitmap.representation(using: .png, properties: [:])
    }

    // MARK: - Test Image Generation

    private func createTestImage(text: String, size: CGSize) -> Data {
        guard let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int(size.width),
            pixelsHigh: Int(size.height),
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ) else { return Data() }

        bitmap.size = size

        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)

        // White background
        NSColor.white.setFill()
        NSRect(origin: .zero, size: size).fill()

        // Draw text
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center

        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 24),
            .foregroundColor: NSColor.black,
            .paragraphStyle: paragraph
        ]

        let textRect = NSRect(x: 40, y: size.height / 2 - 40, width: size.width - 80, height: 80)
        text.draw(in: textRect, withAttributes: attrs)

        NSGraphicsContext.restoreGraphicsState()

        return bitmap.representation(using: .png, properties: [:]) ?? Data()
    }

    // MARK: - Error Image Generation

    private func createErrorImage(message: String, detail: String, size: CGSize) -> Data {
        guard let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int(size.width),
            pixelsHigh: Int(size.height),
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ) else { return Data() }

        bitmap.size = size

        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)

        // Light gray background
        NSColor(white: 0.95, alpha: 1.0).setFill()
        NSRect(origin: .zero, size: size).fill()

        // Draw error message
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center

        let titleAttrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.boldSystemFont(ofSize: 18),
            .foregroundColor: NSColor.darkGray,
            .paragraphStyle: paragraph
        ]

        let detailAttrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 12),
            .foregroundColor: NSColor.gray,
            .paragraphStyle: paragraph
        ]

        let titleRect = NSRect(x: 40, y: size.height / 2, width: size.width - 80, height: 30)
        let detailRect = NSRect(x: 40, y: size.height / 2 - 50, width: size.width - 80, height: 80)

        message.draw(in: titleRect, withAttributes: titleAttrs)
        detail.draw(in: detailRect, withAttributes: detailAttrs)

        NSGraphicsContext.restoreGraphicsState()

        return bitmap.representation(using: .png, properties: [:]) ?? Data()
    }
}

// MARK: - Error Types

enum PreviewError: Error, LocalizedError {
    case unsupportedFormat
    case renderingFailed

    var errorDescription: String? {
        switch self {
        case .unsupportedFormat:
            return "Unsupported file format"
        case .renderingFailed:
            return "Failed to render diagram"
        }
    }
}
