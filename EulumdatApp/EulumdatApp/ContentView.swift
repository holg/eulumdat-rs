import SwiftUI
import EulumdatKit
import UniformTypeIdentifiers

struct ContentView: View {
    @State private var ldt: Eulumdat?
    @State private var errorMessage: String?
    @State private var isImporting = false
    @State private var isExporting = false
    @State private var selectedDiagram: DiagramType = .polar
    @AppStorage("isDarkTheme") private var isDarkTheme = false
    @State private var showValidation = false
    @State private var isTargeted = false
    @State private var currentFileName: String = ""
    @AppStorage("svgExportSize") private var svgExportSize = 600.0

    enum DiagramType: String, CaseIterable, Identifiable {
        case polar = "Polar"
        case cartesian = "Cartesian"
        case butterfly = "Butterfly"
        case heatmap = "Heatmap"
        case bug = "BUG Rating"
        case lcs = "LCS"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .polar: return "circle.grid.cross"
            case .cartesian: return "chart.xyaxis.line"
            case .butterfly: return "leaf"
            case .heatmap: return "square.grid.3x3.fill"
            case .bug: return "lightbulb.led"
            case .lcs: return "rays"
            }
        }
    }

    var body: some View {
        NavigationStack {
            ZStack {
                if let ldt = ldt {
                    diagramView(ldt: ldt)
                } else {
                    emptyStateView
                }

                // Drop overlay
                if isTargeted {
                    dropOverlay
                }
            }
            .navigationTitle(currentFileName.isEmpty ? "Eulumdat Viewer" : currentFileName)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                toolbarContent
            }
            .fileImporter(
                isPresented: $isImporting,
                allowedContentTypes: [UTType(filenameExtension: "ldt") ?? .data, UTType(filenameExtension: "ies") ?? .data],
                allowsMultipleSelection: false
            ) { result in
                handleFileImport(result)
            }
            .fileExporter(
                isPresented: $isExporting,
                document: SVGDocument(svg: currentSVG),
                contentType: .svg,
                defaultFilename: "\(currentFileName.replacingOccurrences(of: ".ldt", with: ""))_\(selectedDiagram.rawValue.lowercased())"
            ) { result in
                if case .failure(let error) = result {
                    errorMessage = error.localizedDescription
                }
            }
            .alert("Error", isPresented: .constant(errorMessage != nil)) {
                Button("OK") { errorMessage = nil }
            } message: {
                Text(errorMessage ?? "")
            }
            .sheet(isPresented: $showValidation) {
                if let ldt = ldt {
                    ValidationSheet(ldt: ldt)
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .openFile)) { _ in
                isImporting = true
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportSVG)) { _ in
                if ldt != nil {
                    isExporting = true
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .selectDiagram)) { notification in
                if let diagramName = notification.object as? String,
                   let diagram = DiagramType.allCases.first(where: { $0.rawValue.lowercased() == diagramName }) {
                    selectedDiagram = diagram
                }
            }
            .onDrop(of: [.fileURL], isTargeted: $isTargeted) { providers in
                handleDrop(providers: providers)
            }
        }
    }

    // MARK: - Computed Properties

    private var currentSVG: String {
        guard let ldt = ldt else { return "" }
        return generateSVG(ldt: ldt, size: svgExportSize, theme: isDarkTheme ? .dark : .light)
    }

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItem(placement: .primaryAction) {
            Button {
                isImporting = true
            } label: {
                Label("Open", systemImage: "folder")
            }
        }

        if ldt != nil {
            #if os(macOS)
            ToolbarItemGroup(placement: .secondaryAction) {
                Picker("Diagram", selection: $selectedDiagram) {
                    ForEach(DiagramType.allCases) { type in
                        Label(type.rawValue, systemImage: type.icon).tag(type)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 140)

                Toggle(isOn: $isDarkTheme) {
                    Label("Dark", systemImage: isDarkTheme ? "moon.fill" : "sun.max.fill")
                }

                Button {
                    showValidation = true
                } label: {
                    Label("Validate", systemImage: "checkmark.shield")
                }

                Button {
                    isExporting = true
                } label: {
                    Label("Export SVG", systemImage: "square.and.arrow.up")
                }
            }
            #else
            ToolbarItemGroup(placement: .bottomBar) {
                Picker("Diagram", selection: $selectedDiagram) {
                    ForEach(DiagramType.allCases) { type in
                        Label(type.rawValue, systemImage: type.icon).tag(type)
                    }
                }
                .pickerStyle(.segmented)
            }

            ToolbarItemGroup(placement: .secondaryAction) {
                Toggle(isOn: $isDarkTheme) {
                    Label("Dark Theme", systemImage: isDarkTheme ? "moon.fill" : "sun.max.fill")
                }

                Button {
                    showValidation = true
                } label: {
                    Label("Validate", systemImage: "checkmark.shield")
                }

                Button {
                    isExporting = true
                } label: {
                    Label("Export SVG", systemImage: "square.and.arrow.up")
                }
            }
            #endif
        }
    }

    // MARK: - Views

    private var emptyStateView: some View {
        VStack(spacing: 16) {
            Image(systemName: "lightbulb")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("No LDT File Loaded")
                .font(.title2)
                .foregroundStyle(.secondary)

            Text("Open or drag an Eulumdat (.ldt) or IES (.ies) file\nto view light distribution diagrams")
                .font(.callout)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Button {
                isImporting = true
            } label: {
                Label("Open File", systemImage: "folder")
            }
            .buttonStyle(.borderedProminent)
            .padding(.top)

            #if os(macOS)
            Text("or press ⌘O")
                .font(.caption)
                .foregroundStyle(.tertiary)
            #endif
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var dropOverlay: some View {
        ZStack {
            Color.accentColor.opacity(0.2)

            VStack(spacing: 12) {
                Image(systemName: "arrow.down.doc.fill")
                    .font(.system(size: 48))
                Text("Drop LDT or IES file here")
                    .font(.headline)
            }
            .foregroundColor(.accentColor)
        }
        .ignoresSafeArea()
    }

    @ViewBuilder
    private func diagramView(ldt: Eulumdat) -> some View {
        GeometryReader { geometry in
            ScrollView {
                VStack(spacing: 16) {
                    // Info header
                    infoHeader(ldt: ldt)
                        .padding(.horizontal)

                    // Diagram picker for iOS (horizontal scroll)
                    #if os(iOS)
                    diagramPicker
                        .padding(.horizontal)
                    #endif

                    // SVG Diagram
                    let theme: SvgThemeType = isDarkTheme ? .dark : .light
                    let size = min(geometry.size.width - 32, 600)

                    SVGView(svgString: generateSVG(ldt: ldt, size: size, theme: theme))
                        .frame(width: size, height: size * aspectRatio(for: selectedDiagram))
                        .background(isDarkTheme ? Color.black : Color.white)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .shadow(radius: 2)
                        .padding(.horizontal)
                        .animation(.easeInOut(duration: 0.2), value: selectedDiagram)

                    // Additional info
                    detailsView(ldt: ldt)
                        .padding(.horizontal)

                    // Lamp sets
                    if !ldt.lampSets.isEmpty {
                        lampSetsView(ldt: ldt)
                            .padding(.horizontal)
                    }
                }
                .padding(.vertical)
            }
        }
    }

    #if os(iOS)
    private var diagramPicker: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(DiagramType.allCases) { type in
                    Button {
                        withAnimation {
                            selectedDiagram = type
                        }
                    } label: {
                        VStack(spacing: 4) {
                            Image(systemName: type.icon)
                                .font(.title2)
                            Text(type.rawValue)
                                .font(.caption)
                        }
                        .frame(width: 70, height: 60)
                        .background(selectedDiagram == type ? Color.accentColor : Color.secondary.opacity(0.2))
                        .foregroundColor(selectedDiagram == type ? .white : .primary)
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }
    #endif

    private func infoHeader(ldt: Eulumdat) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(ldt.luminaireName.isEmpty ? "Unnamed Luminaire" : ldt.luminaireName)
                .font(.headline)

            if !ldt.luminaireNumber.isEmpty {
                Text(ldt.luminaireNumber)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Divider()

            HStack {
                Label(ldt.identification, systemImage: "building.2")
                Spacer()
                Label("\(Int(ldt.totalLuminousFlux)) lm", systemImage: "lightbulb.fill")
            }
            .font(.subheadline)
            .foregroundStyle(.secondary)
        }
        .padding()
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private func detailsView(ldt: Eulumdat) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Luminaire Details")
                .font(.headline)

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                DetailRow(label: "Type", value: formatTypeIndicator(ldt.typeIndicator))
                DetailRow(label: "Symmetry", value: formatSymmetry(ldt.symmetry))
                DetailRow(label: "Dimensions", value: "\(Int(ldt.length))×\(Int(ldt.width))×\(Int(ldt.height)) mm")
                DetailRow(label: "Max Intensity", value: String(format: "%.1f cd/klm", ldt.maxIntensity))
                DetailRow(label: "LOR", value: String(format: "%.1f%%", ldt.lightOutputRatio))
                DetailRow(label: "C-Planes", value: "\(ldt.numCPlanes) @ \(ldt.cPlaneDistance)°")
                DetailRow(label: "G-Planes", value: "\(ldt.numGPlanes) @ \(ldt.gPlaneDistance)°")
                DetailRow(label: "Downward Flux", value: String(format: "%.1f%%", ldt.downwardFluxFraction))
            }

            Divider()

            bugRatingView(ldt: ldt)
        }
        .padding()
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private func bugRatingView(ldt: Eulumdat) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("BUG Rating (TM-15-11)")
                .font(.headline)

            let bugRating = calculateBugRating(ldt: ldt)

            HStack(spacing: 24) {
                bugRatingItem(label: "Backlight", value: bugRating.b, color: .blue)
                bugRatingItem(label: "Uplight", value: bugRating.u, color: .orange)
                bugRatingItem(label: "Glare", value: bugRating.g, color: .red)
            }
            .frame(maxWidth: .infinity)
        }
    }

    private func bugRatingItem(label: String, value: UInt8, color: SwiftUI.Color) -> some View {
        VStack(spacing: 4) {
            Text("\(label.prefix(1))\(value)")
                .font(.title2.bold())
                .foregroundColor(value > 3 ? color : .primary)
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func lampSetsView(ldt: Eulumdat) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Lamp Configuration")
                .font(.headline)

            ForEach(Array(ldt.lampSets.enumerated()), id: \.offset) { index, lampSet in
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(lampSet.lampType.isEmpty ? "Lamp Set \(index + 1)" : lampSet.lampType)
                            .font(.subheadline.bold())
                        Text("\(lampSet.numLamps) × \(Int(lampSet.totalLuminousFlux)) lm")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    if lampSet.wattageWithBallast > 0 {
                        Text("\(Int(lampSet.wattageWithBallast))W")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 4)

                if index < ldt.lampSets.count - 1 {
                    Divider()
                }
            }
        }
        .padding()
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Helpers

    private func formatTypeIndicator(_ type: TypeIndicator) -> String {
        switch type {
        case .pointSourceSymmetric: return "Point (Symmetric)"
        case .linear: return "Linear"
        case .pointSourceOther: return "Point (Asymmetric)"
        }
    }

    private func formatSymmetry(_ sym: Symmetry) -> String {
        switch sym {
        case .none: return "None"
        case .verticalAxis: return "Vertical Axis"
        case .planeC0c180: return "C0-C180 Plane"
        case .planeC90c270: return "C90-C270 Plane"
        case .bothPlanes: return "Both Planes"
        }
    }

    private func generateSVG(ldt: Eulumdat, size: Double, theme: SvgThemeType) -> String {
        switch selectedDiagram {
        case .polar:
            return generatePolarSvg(ldt: ldt, width: size, height: size, theme: theme)
        case .cartesian:
            return generateCartesianSvg(ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, theme: theme)
        case .butterfly:
            return generateButterflySvg(ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, theme: theme)
        case .heatmap:
            return generateHeatmapSvg(ldt: ldt, width: size, height: size * 0.7, theme: theme)
        case .bug:
            return generateBugSvg(ldt: ldt, width: size, height: size * 0.85, theme: theme)
        case .lcs:
            return generateLcsSvg(ldt: ldt, width: size, height: size, theme: theme)
        }
    }

    private func aspectRatio(for diagram: DiagramType) -> Double {
        switch diagram {
        case .polar, .lcs: return 1.0
        case .bug: return 0.85
        case .butterfly: return 0.8
        case .cartesian: return 0.75
        case .heatmap: return 0.7
        }
    }

    // MARK: - File Handling

    private func handleFileImport(_ result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            guard let url = urls.first else { return }
            loadFile(url: url)
        case .failure(let error):
            errorMessage = error.localizedDescription
        }
    }

    private func handleDrop(providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first else { return false }

        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, error in
            guard let data = item as? Data,
                  let url = URL(dataRepresentation: data, relativeTo: nil) else {
                return
            }

            DispatchQueue.main.async {
                loadFile(url: url)
            }
        }
        return true
    }

    private func loadFile(url: URL) {
        guard url.startAccessingSecurityScopedResource() else {
            errorMessage = "Cannot access file"
            return
        }
        defer { url.stopAccessingSecurityScopedResource() }

        currentFileName = url.lastPathComponent

        do {
            let content = try String(contentsOf: url, encoding: .isoLatin1)
            ldt = try parseLdt(content: content)
        } catch {
            // Try UTF-8 if ISO-Latin1 fails
            do {
                let content = try String(contentsOf: url, encoding: .utf8)
                ldt = try parseLdt(content: content)
            } catch {
                errorMessage = "Failed to parse file: \(error.localizedDescription)"
            }
        }
    }
}

// MARK: - Supporting Views

struct DetailRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.callout)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

// MARK: - SVG Document for Export

struct SVGDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.svg] }

    var svg: String

    init(svg: String) {
        self.svg = svg
    }

    init(configuration: ReadConfiguration) throws {
        if let data = configuration.file.regularFileContents {
            svg = String(data: data, encoding: .utf8) ?? ""
        } else {
            svg = ""
        }
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: svg.data(using: .utf8) ?? Data())
    }
}

// MARK: - Validation Sheet

struct ValidationSheet: View {
    let ldt: Eulumdat
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(.green)
                        Text("File parsed successfully")
                    }
                }

                Section("File Information") {
                    LabeledContent("Luminaire", value: ldt.luminaireName)
                    LabeledContent("Manufacturer", value: ldt.identification)
                    LabeledContent("C-Planes", value: "\(ldt.numCPlanes)")
                    LabeledContent("G-Planes", value: "\(ldt.numGPlanes)")
                    LabeledContent("Total Flux", value: "\(Int(ldt.totalLuminousFlux)) lm")
                }

                Section("Photometric Data") {
                    LabeledContent("Intensity Values", value: "\(ldt.intensities.count * (ldt.intensities.first?.count ?? 0))")
                    LabeledContent("Max Intensity", value: String(format: "%.1f cd/klm", ldt.maxIntensity))
                }
            }
            .navigationTitle("Validation")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        #if os(macOS)
        .frame(minWidth: 400, minHeight: 300)
        #endif
    }
}

// MARK: - Preview

#Preview {
    ContentView()
}
