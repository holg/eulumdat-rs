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

    enum OutputFormat: String, CaseIterable, Identifiable {
        case ies = "IES (IESNA LM-63)"
        case ldt = "LDT (Normalized)"

        var id: String { rawValue }

        var fileExtension: String {
            switch self {
            case .ies: return "ies"
            case .ldt: return "ldt"
            }
        }
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
                    Text("Convert multiple EULUMDAT files to IES format")
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

        // Step 2: Batch convert all files at once using Rust core function
        let format: ConversionFormat = outputFormat == .ies ? .ies : .ldt
        let batchFiles = batchInputs.map { $0.file }
        let convertedFiles = batchConvertContents(files: batchFiles, format: format)

        // Step 3: Write converted files to disk
        for (index, converted) in convertedFiles.enumerated() {
            guard index < batchInputs.count else { continue }
            let originalURL = batchInputs[index].url

            // Calculate output path
            var outputURL: URL
            if preserveSubfolders {
                let relativePath = originalURL.path.replacingOccurrences(of: sourceFolder.path, with: "")
                outputURL = destination.appendingPathComponent(relativePath)
            } else {
                outputURL = destination.appendingPathComponent(originalURL.lastPathComponent)
            }
            outputURL = outputURL.deletingPathExtension().appendingPathExtension(outputFormat.fileExtension)

            // Check if file exists
            if fm.fileExists(atPath: outputURL.path) && !overwriteExisting {
                results.append(FileConversionResult(
                    inputPath: originalURL.path,
                    outputPath: outputURL.path,
                    success: false,
                    error: "File already exists"
                ))
                continue
            }

            // Handle conversion result
            if let error = converted.error {
                results.append(FileConversionResult(
                    inputPath: originalURL.path,
                    outputPath: outputURL.path,
                    success: false,
                    error: error
                ))
            } else if let content = converted.content {
                do {
                    // Create output directory if needed
                    let outputDir = outputURL.deletingLastPathComponent()
                    try fm.createDirectory(at: outputDir, withIntermediateDirectories: true)

                    // Write output
                    try content.write(to: outputURL, atomically: true, encoding: .utf8)

                    results.append(FileConversionResult(
                        inputPath: originalURL.path,
                        outputPath: outputURL.path,
                        success: true,
                        error: nil
                    ))
                } catch {
                    results.append(FileConversionResult(
                        inputPath: originalURL.path,
                        outputPath: outputURL.path,
                        success: false,
                        error: "Failed to write file: \(error.localizedDescription)"
                    ))
                }
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
