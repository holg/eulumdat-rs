import Quartz
import AppKit
import EulumdatKit

class PreviewProvider: QLPreviewProvider, QLPreviewingController {

    func providePreview(for request: QLFileRequest) async throws -> QLPreviewReply {
        let fileExtension = request.fileURL.pathExtension.lowercased()

        // Generate preview with standard size
        let previewSize = CGSize(width: 800, height: 800)

        return QLPreviewReply(dataOfContentType: .png, contentSize: previewSize) { reply in
            do {
                // Parse the photometric file
                let photometric: Photometric

                if fileExtension == "ldt" {
                    photometric = try Photometric.fromLdt(path: request.fileURL.path)
                } else if fileExtension == "ies" {
                    photometric = try Photometric.fromIes(path: request.fileURL.path)
                } else {
                    throw PreviewError.unsupportedFormat
                }

                // Generate polar candela diagram (the most common preview)
                let svgString = try photometric.generateDiagram(type: .polarCandela)

                // Convert SVG to PNG for QuickLook
                guard let pngData = self.renderSVGToPNG(svgString: svgString, size: previewSize) else {
                    throw PreviewError.renderingFailed
                }

                return pngData

            } catch {
                // Generate error image
                return self.createErrorImage(
                    message: "Failed to preview photometric file",
                    detail: error.localizedDescription,
                    size: previewSize
                )
            }
        }
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
