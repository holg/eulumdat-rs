# QuickLook Preview Setup for Eulumdat Files

## Overview
To show polar diagram previews in Finder, we need to add a QuickLook extension to the macOS app. This requires converting from Swift Package to Xcode project.

## Step 1: Convert to Xcode Project

```bash
cd /Users/htr/Documents/develeop/rust/eulumdat-rs/EulumdatApp
swift package generate-xcodeproj
```

This creates `EulumdatApp.xcodeproj`.

## Step 2: Add QuickLook Extension in Xcode

1. Open `EulumdatApp.xcodeproj` in Xcode
2. File → New → Target
3. Select "Quick Look Preview Extension" (macOS)
4. Product Name: `EulumdatQuickLook`
5. Click Finish

## Step 3: Configure Extension Info.plist

Edit `EulumdatQuickLook/Info.plist`:

```xml
<key>CFBundleDocumentTypes</key>
<array>
    <dict>
        <key>CFBundleTypeName</key>
        <string>Eulumdat File</string>
        <key>LSItemContentTypes</key>
        <array>
            <string>com.eulumdat.ldt</string>
            <string>com.ies.photometric</string>
        </array>
    </dict>
</array>
<key>QLSupportedContentTypes</key>
<array>
    <string>com.eulumdat.ldt</string>
    <string>com.ies.photometric</string>
</array>
```

## Step 4: Implement PreviewProvider

Replace `EulumdatQuickLook/PreviewProvider.swift`:

```swift
import Quartz
import EulumdatKit

class PreviewProvider: QLPreviewProvider, QLPreviewingController {

    func providePreview(for request: QLFileRequest) async throws -> QLPreviewReply {

        let contentType = request.fileURL.pathExtension.lowercased()

        return try await QLPreviewReply(dataOfContentType: .image, contentSize: CGSize(width: 800, height: 600)) { (reply: QLPreviewReply) in

            do {
                // Parse the photometric file
                let photometric: Photometric
                if contentType == "ldt" {
                    photometric = try Photometric.fromLdt(path: request.fileURL.path)
                } else if contentType == "ies" {
                    photometric = try Photometric.fromIes(path: request.fileURL.path)
                } else {
                    throw NSError(domain: "EulumdatQuickLook", code: 1, userInfo: [NSLocalizedDescriptionKey: "Unsupported file type"])
                }

                // Generate polar diagram SVG
                let svgString = try photometric.generateDiagram(type: .polarCandela)

                // Convert SVG to PNG for QuickLook
                guard let svgData = svgString.data(using: .utf8),
                      let pngData = self.renderSVGToPNG(svgData: svgData, size: CGSize(width: 800, height: 600)) else {
                    throw NSError(domain: "EulumdatQuickLook", code: 2, userInfo: [NSLocalizedDescriptionKey: "Failed to render diagram"])
                }

                return pngData

            } catch {
                // Return error image
                let errorImage = self.createErrorImage(message: "Failed to preview: \(error.localizedDescription)")
                return errorImage.tiffRepresentation ?? Data()
            }
        }
    }

    private func renderSVGToPNG(svgData: Data, size: CGSize) -> Data? {
        // Use WebKit or external library to render SVG to PNG
        // This is a simplified version - you may need to use NSImage + SVG rendering

        guard let svgString = String(data: svgData, encoding: .utf8),
              let nsImage = NSImage(data: svgData) else {
            return nil
        }

        let rep = NSBitmapImageRep(
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
        )

        rep?.size = size

        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep!)

        NSColor.white.setFill()
        NSRect(origin: .zero, size: size).fill()

        nsImage.draw(in: NSRect(origin: .zero, size: size))

        NSGraphicsContext.restoreGraphicsState()

        return rep?.representation(using: .png, properties: [:])
    }

    private func createErrorImage(message: String) -> NSImage {
        let size = CGSize(width: 400, height: 300)
        let image = NSImage(size: size)

        image.lockFocus()

        NSColor.white.setFill()
        NSRect(origin: .zero, size: size).fill()

        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center

        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 14),
            .foregroundColor: NSColor.red,
            .paragraphStyle: paragraph
        ]

        let textRect = NSRect(x: 20, y: size.height / 2 - 20, width: size.width - 40, height: 40)
        message.draw(in: textRect, withAttributes: attrs)

        image.unlockFocus()

        return image
    }
}
```

## Step 5: Link EulumdatKit Framework

1. In Xcode, select the `EulumdatQuickLook` target
2. General → Frameworks and Libraries
3. Add `EulumdatKit` framework

## Step 6: Build and Test

```bash
# Build the app with QuickLook extension
xcodebuild -project EulumdatApp.xcodeproj -scheme EulumdatApp -configuration Release

# Install to Applications
cp -R build/Release/EulumdatApp.app ~/Applications/
```

## Step 7: Test QuickLook Preview

1. Open Finder
2. Navigate to a folder with `.ldt` or `.ies` files
3. Select a file and press Space (QuickLook)
4. You should see the polar diagram preview

## Alternative: Use Thumbnail Provider (macOS 11+)

For macOS 11+, you can also implement a Thumbnail Provider extension that shows thumbnails in Finder icon view:

1. File → New → Target → "Thumbnail Provider Extension"
2. Implement similar logic to generate small preview images

---

## Notes

- QuickLook extensions run in a sandboxed environment
- The extension needs to be signed with the same certificate as the main app
- For App Store distribution, both the app and extension need proper entitlements
- SVG rendering may require additional dependencies (consider using `librsvg` via Homebrew or bundling a rendering library)

## SVG Rendering Options

### Option A: Use WebKit (Simplest)
```swift
import WebKit

func renderSVG(svgData: Data, size: CGSize) -> Data? {
    let webView = WKWebView(frame: CGRect(origin: .zero, size: size))
    // Load SVG and capture as image
    // (requires running in main thread context)
}
```

### Option B: Use rsvg-convert (Most Reliable)
Bundle `rsvg-convert` CLI tool with the extension and call it to convert SVG → PNG.

### Option C: Use Native NSImage (Limited SVG Support)
macOS has limited native SVG support, may not render complex polar diagrams correctly.

---

## Troubleshooting

**Preview not showing:**
- Check Console.app for QuickLook errors
- Verify UTI types match in both app and extension Info.plist
- Run `qlmanage -r` to reset QuickLook cache
- Run `qlmanage -p file.ldt` to test preview directly

**Extension not loading:**
- Ensure extension is properly embedded in the app bundle
- Check code signing: `codesign -dv --entitlements - EulumdatApp.app`
- Verify extension is in `EulumdatApp.app/Contents/PlugIns/`
