import SwiftUI
import EulumdatKit
import UniformTypeIdentifiers

// MARK: - Hex ⇄ Color bridging
//
// `SvgThemeConfig` stores colours as CSS hex strings ("#rrggbb"), but SwiftUI's
// `ColorPicker` works with `Color`. These helpers convert between the two so the
// pickers can drive the config directly.

extension SwiftUI.Color {
    /// Build a Color from a "#rrggbb" / "#rgb" hex string. Returns nil on garbage.
    init?(hex: String) {
        var s = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.hasPrefix("#") { s.removeFirst() }
        // Expand shorthand #rgb → #rrggbb.
        if s.count == 3 {
            s = s.map { "\($0)\($0)" }.joined()
        }
        guard s.count == 6, let value = UInt32(s, radix: 16) else { return nil }
        let r = Double((value >> 16) & 0xFF) / 255.0
        let g = Double((value >> 8) & 0xFF) / 255.0
        let b = Double(value & 0xFF) / 255.0
        self = SwiftUI.Color(red: r, green: g, blue: b)
    }

    /// Serialize back to a "#rrggbb" hex string for the theme config.
    var hexString: String {
        #if os(macOS)
        let native = NSColor(self).usingColorSpace(.sRGB) ?? NSColor(self)
        let r = Int((native.redComponent * 255).rounded())
        let g = Int((native.greenComponent * 255).rounded())
        let b = Int((native.blueComponent * 255).rounded())
        #else
        var rf: CGFloat = 0, gf: CGFloat = 0, bf: CGFloat = 0, af: CGFloat = 0
        UIColor(self).getRed(&rf, green: &gf, blue: &bf, alpha: &af)
        let r = Int((rf * 255).rounded())
        let g = Int((gf * 255).rounded())
        let b = Int((bf * 255).rounded())
        #endif
        return String(format: "#%02x%02x%02x", r, g, b)
    }
}

// MARK: - Theme document (for import/export via fileExporter/fileImporter)

struct ThemeFileDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.init(filenameExtension: "toml") ?? .plainText, .plainText] }
    static var writableContentTypes: [UTType] { [.init(filenameExtension: "toml") ?? .plainText] }

    var text: String

    init(text: String) { self.text = text }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents,
              let str = String(data: data, encoding: .utf8) else {
            throw CocoaError(.fileReadCorruptFile)
        }
        text = str
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}

// MARK: - Theme Editor

/// Live editor for a custom SVG diagram theme: colour wells, curve thickness,
/// font, and a live polar-diagram preview. The edited `SvgThemeConfig` is
/// exposed via `@Binding` so a host tab can hand it to the `*_themed` diagram
/// generators (and to batch export).
struct ThemeEditorView: View {
    /// The luminaire whose polar diagram drives the live preview.
    let ldt: Eulumdat

    /// The theme being edited. Bound so the parent keeps the applied theme.
    @Binding var config: SvgThemeConfig

    // Import/export sheets.
    @State private var isExporting = false
    @State private var isImporting = false
    @State private var exportDocument = ThemeFileDocument(text: "")
    @State private var errorMessage: String?

    // Named presets.
    @ObservedObject private var presetStore = ThemePresetStore.shared
    @State private var isNamingPreset = false
    @State private var newPresetName = ""

    private var appLanguage: String { L10n.currentLanguage }

    private let fontChoices = [
        "system-ui, -apple-system, sans-serif",
        "Helvetica, Arial, sans-serif",
        "Georgia, 'Times New Roman', serif",
        "'SF Mono', Menlo, monospace",
    ]

    var body: some View {
        HStack(spacing: 0) {
            // ── Controls ──────────────────────────────────────────────
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    presetSection
                    Divider()
                    savedPresetsSection
                    Divider()
                    colorSection
                    Divider()
                    curveSection
                    Divider()
                    fontSection
                    Divider()
                    fileSection
                }
                .padding()
                .frame(maxWidth: 340, alignment: .leading)
            }
            #if os(macOS)
            .frame(width: 340)
            #endif
            .background(Color.controlBackground)

            Divider()

            // ── Live preview ──────────────────────────────────────────
            GeometryReader { geo in
                let size = min(geo.size.width, geo.size.height) - 32
                VStack {
                    Spacer(minLength: 0)
                    SVGView(svgString: previewSvg(size: max(size, 200)))
                        .frame(width: max(size, 200), height: max(size, 200))
                        .background(previewBackground)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .shadow(radius: 2)
                    Spacer(minLength: 0)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .alert("Theme error", isPresented: .constant(errorMessage != nil)) {
            Button("OK") { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "")
        }
        .fileExporter(
            isPresented: $isExporting,
            document: exportDocument,
            contentType: .plainText,
            defaultFilename: "theme"
        ) { result in
            if case .failure(let err) = result { errorMessage = err.localizedDescription }
        }
        .fileImporter(
            isPresented: $isImporting,
            allowedContentTypes: [.plainText, .init(filenameExtension: "toml") ?? .plainText],
            allowsMultipleSelection: false
        ) { result in
            handleImport(result)
        }
        // Publish the live theme so Batch export can apply it without a save.
        // Single-parameter onChange for iOS 16 compatibility.
        .onAppear { presetStore.setCurrent(config) }
        .onChange(of: config) { newValue in
            presetStore.setCurrent(newValue)
        }
    }

    // MARK: - Sections

    private var presetSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            sectionHeader("Preset")
            HStack {
                ForEach(["light", "dark", "css"], id: \.self) { name in
                    Button(name.capitalized) {
                        config = themePreset(name: name)
                    }
                    .buttonStyle(.bordered)
                }
            }
        }
    }

    private var savedPresetsSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            sectionHeader("Saved presets")

            HStack {
                Menu {
                    if presetStore.names.isEmpty {
                        Text("No saved presets")
                    }
                    ForEach(presetStore.names, id: \.self) { name in
                        Button(name) {
                            if let loaded = presetStore.config(named: name) {
                                config = loaded
                            }
                        }
                    }
                } label: {
                    Label("Load", systemImage: "tray.and.arrow.down")
                }
                .disabled(presetStore.names.isEmpty)

                Button {
                    newPresetName = ""
                    isNamingPreset = true
                } label: {
                    Label("Save as…", systemImage: "plus")
                }
                .buttonStyle(.bordered)
            }

            if !presetStore.names.isEmpty {
                Menu {
                    ForEach(presetStore.names, id: \.self) { name in
                        Button(role: .destructive) {
                            presetStore.delete(name)
                        } label: {
                            Label("Delete “\(name)”", systemImage: "trash")
                        }
                    }
                } label: {
                    Label("Manage…", systemImage: "slider.horizontal.3")
                        .font(.caption)
                }
                .menuStyle(.borderlessButton)
            }
        }
        .alert("Save preset", isPresented: $isNamingPreset) {
            TextField("Preset name", text: $newPresetName)
            Button("Save") {
                presetStore.save(config, as: newPresetName)
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Name this theme so you can reuse it and apply it in batch export.")
        }
    }

    private var colorSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("Colours")
            colorWell("Background", get: config.background, set: { config.background = $0 })
            colorWell("Surface", get: config.surface, set: { config.surface = $0 })
            colorWell("Grid", get: config.grid, set: { config.grid = $0 })
            colorWell("Axis", get: config.axis, set: { config.axis = $0 })
            colorWell("Text", get: config.text, set: { config.text = $0 })
            colorWell("C0–C180 curve", get: config.curveC0, set: { config.curveC0 = $0 })
            colorWell("C90–C270 curve", get: config.curveC90, set: { config.curveC90 = $0 })
        }
    }

    private var curveSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("Curves")

            VStack(alignment: .leading, spacing: 2) {
                Text("Line thickness: \(thicknessValue, specifier: "%.1f") px")
                    .font(.caption).foregroundStyle(.secondary)
                Slider(value: thicknessBinding, in: 0.5...8, step: 0.5)
            }
        }
    }

    private var fontSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            sectionHeader("Font")

            // Curated presets. A custom font chosen below is added as an extra
            // tag so the picker can display it as the current selection.
            Picker("Font family", selection: fontBinding) {
                ForEach(fontPickerChoices, id: \.self) { family in
                    Text(fontDisplayName(family)).tag(family)
                }
            }
            .labelsHidden()
            .pickerStyle(.menu)

            FontChooserButton { family in
                // Use the raw family name, with a generic fallback appended so
                // the SVG degrades gracefully where the font isn't installed.
                config.fontFamily = "\(family), sans-serif"
            }

            Text("Current: \(config.fontFamily ?? fontChoices[0])")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
    }

    /// Curated fonts plus the current custom font (if any) so the menu can show
    /// it selected instead of falling back to a blank row.
    private var fontPickerChoices: [String] {
        let current = config.fontFamily ?? fontChoices[0]
        return fontChoices.contains(current) ? fontChoices : fontChoices + [current]
    }

    private var fileSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            sectionHeader("Theme file")
            HStack {
                Button {
                    exportDocument = ThemeFileDocument(text: themeConfigToToml(config: config))
                    isExporting = true
                } label: {
                    Label("Export…", systemImage: "square.and.arrow.up")
                }
                .buttonStyle(.bordered)

                Button {
                    isImporting = true
                } label: {
                    Label("Import…", systemImage: "square.and.arrow.down")
                }
                .buttonStyle(.bordered)
            }
        }
    }

    // MARK: - Reusable controls

    private func sectionHeader(_ title: String) -> some View {
        Text(title.uppercased())
            .font(.caption.bold())
            .foregroundStyle(.secondary)
    }

    /// A labelled colour well bound to one optional hex field of the config.
    /// `nil`/unparseable falls back to a neutral grey so the well is usable.
    private func colorWell(_ label: String, get: String?, set: @escaping (String?) -> Void) -> some View {
        let binding = Binding<SwiftUI.Color>(
            get: { get.flatMap { SwiftUI.Color(hex: $0) } ?? SwiftUI.Color.gray },
            set: { set($0.hexString) }
        )
        return ColorPicker(label, selection: binding, supportsOpacity: false)
            .font(.callout)
    }

    // MARK: - Derived bindings

    /// Curve stroke width. Backed by the config's `curveWidth` field, which the
    /// core renderer applies inline so it shows in every renderer (default 2.5).
    private var thicknessValue: Double {
        config.curveWidth ?? 2.5
    }

    private var thicknessBinding: Binding<Double> {
        Binding(
            get: { thicknessValue },
            set: { config.curveWidth = $0 }
        )
    }

    private var fontBinding: Binding<String> {
        Binding(
            get: { config.fontFamily ?? fontChoices[0] },
            set: { config.fontFamily = $0 }
        )
    }

    // MARK: - Preview

    private func previewSvg(size: CGFloat) -> String {
        generatePolarSvgThemed(ldt: ldt, width: Double(size), height: Double(size), config: config)
    }

    private var previewBackground: SwiftUI.Color {
        config.background.flatMap { SwiftUI.Color(hex: $0) } ?? SwiftUI.Color.white
    }

    private func fontDisplayName(_ family: String) -> String {
        // Curated presets get friendly names.
        if family == fontChoices[0] { return "System" }
        if family.contains("Helvetica") { return "Helvetica / Arial" }
        if family.contains("Georgia") { return "Serif (Georgia)" }
        if family.contains("mono") || family.contains("Menlo") { return "Monospace" }
        // Custom font: show its first (primary) family token.
        let primary = family
            .split(separator: ",").first
            .map { $0.trimmingCharacters(in: CharacterSet(charactersIn: " '\"")) } ?? family
        return primary.isEmpty ? family : primary
    }

    // MARK: - Import

    private func handleImport(_ result: Result<[URL], Error>) {
        guard case .success(let urls) = result, let url = urls.first else { return }
        guard url.startAccessingSecurityScopedResource() else { return }
        defer { url.stopAccessingSecurityScopedResource() }
        do {
            let text = try String(contentsOf: url, encoding: .utf8)
            config = try themeConfigFromToml(toml: text)
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

// MARK: - Native font chooser

#if os(macOS)
/// A "Choose font…" button that opens the system font panel and reports the
/// selected font family name.
struct FontChooserButton: View {
    let onPick: (String) -> Void

    var body: some View {
        Button {
            FontPanelController.shared.present(onPick: onPick)
        } label: {
            Label("Choose font…", systemImage: "textformat")
        }
        .buttonStyle(.bordered)
    }
}

/// Bridges `NSFontPanel` (which reports changes through the responder chain)
/// to a simple callback.
final class FontPanelController: NSObject, NSWindowDelegate {
    static let shared = FontPanelController()
    private var onPick: ((String) -> Void)?

    func present(onPick: @escaping (String) -> Void) {
        self.onPick = onPick
        let manager = NSFontManager.shared
        manager.target = self
        manager.action = #selector(changeFont(_:))
        let panel = manager.fontPanel(true)
        panel?.makeKeyAndOrderFront(nil)
    }

    @objc func changeFont(_ sender: Any?) {
        let manager = NSFontManager.shared
        let converted = manager.convert(NSFont.systemFont(ofSize: 14))
        // familyName is the user-facing family (e.g. "Georgia").
        if let family = converted.familyName {
            onPick?(family)
        }
    }

    /// Validate menu items so the font panel stays enabled.
    @objc func validModesForFontPanel(_ fontPanel: NSFontPanel) -> UInt {
        UInt(NSFontPanel.ModeMask.face.rawValue | NSFontPanel.ModeMask.collection.rawValue)
    }
}

#else
import UIKit

/// A "Choose font…" button that presents `UIFontPickerViewController`.
struct FontChooserButton: View {
    let onPick: (String) -> Void
    @State private var showing = false

    var body: some View {
        Button {
            showing = true
        } label: {
            Label("Choose font…", systemImage: "textformat")
        }
        .buttonStyle(.bordered)
        .sheet(isPresented: $showing) {
            FontPickerRepresentable(onPick: onPick)
        }
    }
}

struct FontPickerRepresentable: UIViewControllerRepresentable {
    let onPick: (String) -> Void

    func makeUIViewController(context: Context) -> UIFontPickerViewController {
        let config = UIFontPickerViewController.Configuration()
        let picker = UIFontPickerViewController(configuration: config)
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ controller: UIFontPickerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator { Coordinator(onPick: onPick) }

    final class Coordinator: NSObject, UIFontPickerViewControllerDelegate {
        let onPick: (String) -> Void
        init(onPick: @escaping (String) -> Void) { self.onPick = onPick }

        func fontPickerViewControllerDidPickFont(_ viewController: UIFontPickerViewController) {
            if let descriptor = viewController.selectedFontDescriptor {
                let font = UIFont(descriptor: descriptor, size: 14)
                onPick(font.familyName)
            }
            viewController.dismiss(animated: true)
        }

        func fontPickerViewControllerDidCancel(_ viewController: UIFontPickerViewController) {
            viewController.dismiss(animated: true)
        }
    }
}
#endif
