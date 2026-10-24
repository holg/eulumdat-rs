import SwiftUI
import EulumdatKit
import UniformTypeIdentifiers

#if os(macOS)
/// Batch conversion view for converting multiple LDT files to IES or other formats
struct BatchConvertView: View {
    @State private var sourceFolder: URL?
    @State private var outputFolder: URL?
    @State private var outputFormat: OutputFormat = .ies
    @State private var isConverting = false
    @State private var conversionResults: [FileConversionResult] = []
    @State private var showResults = false
    @State private var preserveSubfolders = true
    @State private var overwriteExisting = false

    // Themed SVG export.
    @ObservedObject private var presetStore = ThemePresetStore.shared
    /// Selected theme preset name for SVG export; empty = built-in Light.
    @State private var selectedPreset = ""
    /// Which diagram types to render (polar by default).
    @State private var selectedDiagrams: Set<DiagramKind> = [.polar]
    @AppStorage("svgExportSize") private var svgExportSize = 600.0

    enum OutputFormat: String, CaseIterable, Identifiable {
        case ies = "IES (IESNA LM-63)"
        case ldt = "LDT (Normalized)"
        case svg = "Themed SVG diagrams"

        var id: String { rawValue }

        var fileExtension: String {
            switch self {
            case .ies: return "ies"
            case .ldt: return "ldt"
            case .svg: return "svg"
            }
        }

        /// SVG export renders diagrams and needs a theme; text formats don't.
        var isThemed: Bool { self == .svg }
    }

    /// A diagram type the batch export can render, with its themed FFI call.
    enum DiagramKind: String, CaseIterable, Identifiable {
        case polar = "Polar"
        case cartesian = "Cartesian"
        case heatmap = "Heatmap"
        case butterfly = "Butterfly"

        var id: String { rawValue }

        /// Filename suffix appended before ".svg" (e.g. "_polar").
        var fileSuffix: String { "_" + rawValue.lowercased() }

        /// Render this diagram for `ldt` with the given theme.
        func render(ldt: Eulumdat, size: Double, theme: SvgThemeConfig) -> String {
            switch self {
            case .polar:
                return generatePolarSvgThemed(ldt: ldt, width: size, height: size, config: theme)
            case .cartesian:
                return generateCartesianSvgThemed(
                    ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, config: theme)
            case .heatmap:
                return generateHeatmapSvgThemed(
                    ldt: ldt, width: size, height: size * 0.7, config: theme)
            case .butterfly:
                return generateButterflySvgThemed(
                    ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, config: theme)
            }
        }
    }

    /// Sentinel tag for "use the theme currently being edited in the Theme tab".
    private static let currentThemeTag = "__current__"

    /// Resolve the theme to use for SVG export:
    ///   • the live Theme-tab theme (sentinel), or
    ///   • a saved preset by name, or
    ///   • the built-in Light preset (empty selection / fallback).
    private var exportTheme: SvgThemeConfig {
        if selectedPreset == Self.currentThemeTag, let cfg = presetStore.current {
            return cfg
        }
        if !selectedPreset.isEmpty, let cfg = presetStore.config(named: selectedPreset) {
            return cfg
        }
        return themePreset(name: "light")
    }

    /// Diagram kinds to render, in canonical order, honouring the selection.
    private var activeDiagrams: [DiagramKind] {
        DiagramKind.allCases.filter { selectedDiagrams.contains($0) }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .font(.title)
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading) {
                    Text("Batch Convert LDT Files")
                        .font(.headline)
                    Text("Convert to IES/LDT, or export themed SVG diagrams")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }
            .padding()
            .background(Color(nsColor: .windowBackgroundColor))

            Divider()

            // Configuration
            Form {
                Section("Source") {
                    HStack {
                        if let folder = sourceFolder {
                            Image(systemName: "folder.fill")
                                .foregroundStyle(.blue)
                            Text(folder.path)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            if let count = countLdtFiles(in: folder) {
                                Text("\(count) LDT files")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        } else {
                            Text("No folder selected")
                                .foregroundStyle(.secondary)
                        }
                        Button("Choose...") {
                            selectSourceFolder()
                        }
                    }
                }

                Section("Output") {
                    Picker("Format", selection: $outputFormat) {
                        ForEach(OutputFormat.allCases) { format in
                            Text(format.rawValue).tag(format)
                        }
                    }

                    if outputFormat.isThemed {
                        Picker("Theme", selection: $selectedPreset) {
                            Text("Light (built-in)").tag("")
                            if presetStore.current != nil {
                                Text("Current theme (from Theme tab)").tag(Self.currentThemeTag)
                            }
                            if !presetStore.names.isEmpty {
                                Divider()
                                ForEach(presetStore.names, id: \.self) { name in
                                    Text(name).tag(name)
                                }
                            }
                        }
                        .help("Edit in the Theme tab and pick “Current theme”, or save named presets there.")

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Diagrams")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            ForEach(DiagramKind.allCases) { kind in
                                Toggle(kind.rawValue, isOn: Binding(
                                    get: { selectedDiagrams.contains(kind) },
                                    set: { on in
                                        if on {
                                            selectedDiagrams.insert(kind)
                                        } else if selectedDiagrams.count > 1 {
                                            // Keep at least one selected.
                                            selectedDiagrams.remove(kind)
                                        }
                                    }
                                ))
                            }
                        }
                    }

                    HStack {
                        if let folder = outputFolder {
                            Image(systemName: "folder.fill")
                                .foregroundStyle(.green)
                            Text(folder.path)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        } else {
                            Text("Same as source folder")
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button("Choose...") {
                            selectOutputFolder()
                        }
                        if outputFolder != nil {
                            Button("Reset") {
                                outputFolder = nil
                            }
                            .buttonStyle(.link)
                        }
                    }
                }

                Section("Options") {
                    Toggle("Preserve subfolder structure", isOn: $preserveSubfolders)
                    Toggle("Overwrite existing files", isOn: $overwriteExisting)
                }

                if !conversionResults.isEmpty {
                    Section("Results") {
                        HStack {
                            let successful = conversionResults.filter { $0.success }.count
                            let failed = conversionResults.count - successful

                            Label("\(successful) successful", systemImage: "checkmark.circle.fill")
                                .foregroundStyle(.green)

                            if failed > 0 {
                                Label("\(failed) failed", systemImage: "xmark.circle.fill")
                                    .foregroundStyle(.red)
                            }

                            Spacer()

                            Button("View Details") {
                                showResults = true
                            }
                        }
                    }
                }
            }
            .formStyle(.grouped)

            Divider()

            // Actions
            HStack {
                if isConverting {
                    ProgressView()
                        .scaleEffect(0.7)
                    Text("Converting...")
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Button("Convert") {
                    performConversion()
                }
                .buttonStyle(.borderedProminent)
                .disabled(sourceFolder == nil || isConverting)
            }
            .padding()
        }
        .frame(width: 500, height: 450)
        .sheet(isPresented: $showResults) {
            ConversionResultsSheet(results: conversionResults)
        }
    }

    private func selectSourceFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "Select folder containing LDT files"

        if panel.runModal() == .OK {
            sourceFolder = panel.url
        }
    }

    private func selectOutputFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.message = "Select output folder for converted files"

        if panel.runModal() == .OK {
            outputFolder = panel.url
        }
    }

    private func countLdtFiles(in folder: URL) -> Int? {
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(
            at: folder,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return nil }

        var count = 0
        while let url = enumerator.nextObject() as? URL {
            let ext = url.pathExtension.lowercased()
            if ext == "ldt" || ext == "ies" {
                count += 1
            }
        }
        return count
    }

    private func performConversion() {
        guard let source = sourceFolder else { return }

        isConverting = true
        conversionResults = []

        DispatchQueue.global(qos: .userInitiated).async {
            let results = convertFiles(from: source)

            DispatchQueue.main.async {
                conversionResults = results
                isConverting = false
                showResults = true
            }
        }
    }

    private func convertFiles(from sourceFolder: URL) -> [FileConversionResult] {
        let fm = FileManager.default
        var results: [FileConversionResult] = []

        guard let enumerator = fm.enumerator(
            at: sourceFolder,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return results }

        let destination = outputFolder ?? sourceFolder

        // Step 1: Collect all LDT and IES files and read their contents
        var batchInputs: [(url: URL, file: BatchInputFile)] = []

        while let url = enumerator.nextObject() as? URL {
            let ext = url.pathExtension.lowercased()
            guard ext == "ldt" || ext == "ies" else { continue }

            do {
                let content = try String(contentsOf: url, encoding: .utf8)
                // Auto-detect format based on file extension (core will also auto-detect from content)
                let format: InputFormat? = ext == "ies" ? .ies : .ldt
                let inputFile = BatchInputFile(name: url.lastPathComponent, content: content, format: format)
                batchInputs.append((url: url, file: inputFile))
            } catch {
                // Calculate output path for error reporting
                var outputURL: URL
                if preserveSubfolders {
                    let relativePath = url.path.replacingOccurrences(of: sourceFolder.path, with: "")
                    outputURL = destination.appendingPathComponent(relativePath)
                } else {
                    outputURL = destination.appendingPathComponent(url.lastPathComponent)
                }
                outputURL = outputURL.deletingPathExtension().appendingPathExtension(outputFormat.fileExtension)

                results.append(FileConversionResult(
                    inputPath: url.path,
                    outputPath: outputURL.path,
                    success: false,
                    error: "Failed to read file: \(error.localizedDescription)"
                ))
            }
        }

        // Step 2/3: Themed SVG fans out one file per selected diagram type, so
        // it uses its own writer. Text formats keep the 1:1 batch path.
        if outputFormat.isThemed {
            return writeThemedSvgs(batchInputs, sourceFolder: sourceFolder, destination: destination, into: &results)
        }

        let format: ConversionFormat = outputFormat == .ies ? .ies : .ldt
        let batchFiles = batchInputs.map { $0.file }
        let convertedFiles = batchConvertContents(files: batchFiles, format: format)

        // Write converted files to disk (1:1 with inputs).
        for (index, converted) in convertedFiles.enumerated() {
            guard index < batchInputs.count else { continue }
            let originalURL = batchInputs[index].url
            let outputURL = outputPath(for: originalURL, sourceFolder: sourceFolder,
                                       destination: destination, suffix: "")

            if fm.fileExists(atPath: outputURL.path) && !overwriteExisting {
                results.append(FileConversionResult(
                    inputPath: originalURL.path, outputPath: outputURL.path,
                    success: false, error: "File already exists"))
                continue
            }

            if let error = converted.error {
                results.append(FileConversionResult(
                    inputPath: originalURL.path, outputPath: outputURL.path,
                    success: false, error: error))
            } else if let content = converted.content {
                results.append(write(content, to: outputURL, from: originalURL, fm: fm))
            }
        }

        return results
    }

    /// Compute the destination URL for an input, applying subfolder preservation,
    /// an optional filename `suffix` (e.g. "_polar"), and the output extension.
    private func outputPath(for originalURL: URL, sourceFolder: URL, destination: URL, suffix: String) -> URL {
        var outputURL: URL
        if preserveSubfolders {
            let relativePath = originalURL.path.replacingOccurrences(of: sourceFolder.path, with: "")
            outputURL = destination.appendingPathComponent(relativePath)
        } else {
            outputURL = destination.appendingPathComponent(originalURL.lastPathComponent)
        }
        let base = outputURL.deletingPathExtension().lastPathComponent + suffix
        return outputURL.deletingLastPathComponent()
            .appendingPathComponent(base)
            .appendingPathExtension(outputFormat.fileExtension)
    }

    /// Write `content` to `outputURL`, creating directories, into a result.
    private func write(_ content: String, to outputURL: URL, from originalURL: URL, fm: FileManager) -> FileConversionResult {
        do {
            let outputDir = outputURL.deletingLastPathComponent()
            try fm.createDirectory(at: outputDir, withIntermediateDirectories: true)
            try content.write(to: outputURL, atomically: true, encoding: .utf8)
            return FileConversionResult(inputPath: originalURL.path, outputPath: outputURL.path,
                                        success: true, error: nil)
        } catch {
            return FileConversionResult(inputPath: originalURL.path, outputPath: outputURL.path,
                                        success: false, error: "Failed to write file: \(error.localizedDescription)")
        }
    }

    /// Parse each input once, then render + write every selected diagram type
    /// (one SVG per type, filename suffixed with the type name).
    private func writeThemedSvgs(
        _ batchInputs: [(url: URL, file: BatchInputFile)],
        sourceFolder: URL, destination: URL,
        into results: inout [FileConversionResult]
    ) -> [FileConversionResult] {
        let fm = FileManager.default
        let theme = exportTheme
        let size = svgExportSize
        let diagrams = activeDiagrams

        for input in batchInputs {
            // Parse the source once for all diagram types.
            let ldt: Eulumdat
            do {
                if input.file.format == .ies {
                    ldt = try parseIes(content: input.file.content)
                } else if let parsed = try? parseLdt(content: input.file.content) {
                    ldt = parsed
                } else {
                    ldt = try parseIes(content: input.file.content)
                }
            } catch {
                let outURL = outputPath(for: input.url, sourceFolder: sourceFolder,
                                        destination: destination, suffix: diagrams.first?.fileSuffix ?? "")
                results.append(FileConversionResult(
                    inputPath: input.url.path, outputPath: outURL.path,
                    success: false, error: "Failed to parse: \(error.localizedDescription)"))
                continue
            }

            for kind in diagrams {
                let outURL = outputPath(for: input.url, sourceFolder: sourceFolder,
                                        destination: destination, suffix: kind.fileSuffix)
                if fm.fileExists(atPath: outURL.path) && !overwriteExisting {
                    results.append(FileConversionResult(
                        inputPath: input.url.path, outputPath: outURL.path,
                        success: false, error: "File already exists"))
                    continue
                }
                let svg = kind.render(ldt: ldt, size: size, theme: theme)
                results.append(write(svg, to: outURL, from: input.url, fm: fm))
            }
        }

        return results
    }
}

struct FileConversionResult: Identifiable {
    let id = UUID()
    let inputPath: String
    let outputPath: String
    let success: Bool
    let error: String?

    var inputFileName: String {
        URL(fileURLWithPath: inputPath).lastPathComponent
    }

    var outputFileName: String {
        URL(fileURLWithPath: outputPath).lastPathComponent
    }
}

struct ConversionResultsSheet: View {
    let results: [FileConversionResult]
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Conversion Results")
                    .font(.headline)
                Spacer()

                let successful = results.filter { $0.success }.count
                let failed = results.count - successful

                HStack(spacing: 12) {
                    Label("\(successful)", systemImage: "checkmark.circle.fill")
                        .foregroundStyle(.green)
                    Label("\(failed)", systemImage: "xmark.circle.fill")
                        .foregroundStyle(failed > 0 ? .red : .secondary)
                }
            }
            .padding()

            Divider()

            // Results list
            List(results) { result in
                HStack {
                    Image(systemName: result.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundStyle(result.success ? .green : .red)

                    VStack(alignment: .leading, spacing: 2) {
                        Text(result.inputFileName)
                            .font(.body)
                        if result.success {
                            Text("→ \(result.outputFileName)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        } else if let error = result.error {
                            Text(error)
                                .font(.caption)
                                .foregroundStyle(.red)
                        }
                    }

                    Spacer()

                    if result.success {
                        Button {
                            NSWorkspace.shared.selectFile(result.outputPath, inFileViewerRootedAtPath: "")
                        } label: {
                            Image(systemName: "folder")
                        }
                        .buttonStyle(.borderless)
                        .help("Show in Finder")
                    }
                }
            }

            Divider()

            // Footer
            HStack {
                Button("Open Output Folder") {
                    if let firstSuccess = results.first(where: { $0.success }) {
                        let folderURL = URL(fileURLWithPath: firstSuccess.outputPath).deletingLastPathComponent()
                        NSWorkspace.shared.open(folderURL)
                    }
                }
                .disabled(!results.contains { $0.success })

                Spacer()

                Button("Done") {
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
            }
            .padding()
        }
        .frame(width: 500, height: 400)
    }
}

#Preview {
    BatchConvertView()
}
#endif
