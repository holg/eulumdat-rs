import SwiftUI
import EulumdatKit
import UniformTypeIdentifiers

// MARK: - Platform-specific Color Helpers

extension SwiftUI.Color {
    #if os(macOS)
    static var controlBackground: SwiftUI.Color {
        SwiftUI.Color(nsColor: .controlBackgroundColor)
    }

    static var windowBackground: SwiftUI.Color {
        SwiftUI.Color(nsColor: .windowBackgroundColor)
    }
    #else
    static var controlBackground: SwiftUI.Color {
        SwiftUI.Color(uiColor: .secondarySystemBackground)
    }

    static var windowBackground: SwiftUI.Color {
        SwiftUI.Color(uiColor: .systemBackground)
    }
    #endif
}

struct ContentView: View {
    @State private var ldt: Eulumdat?
    @State private var errorMessage: String?
    @State private var isImporting = false
    @State private var isExporting = false
    @State private var selectedTab: AppTab = .diagram
    @State private var selectedDiagram: DiagramType = .polar
    @AppStorage("isDarkTheme") private var isDarkTheme = false
    @State private var showValidation = false
    @State private var isTargeted = false
    @State private var currentFileName: String = ""
    @AppStorage("svgExportSize") private var svgExportSize = 600.0
    @Environment(\.openWindow) private var openWindow

    #if os(macOS)
    // Additional environment for opening windows on macOS
    #endif

    // Unified export state
    @State private var exportDocument: ExportDocument?

    enum ExportType {
        case svg, ies, ldt
    }

    struct ExportDocument {
        let type: ExportType
        let content: String
    }

    enum AppTab: String, CaseIterable, Identifiable {
        case general = "General"
        case dimensions = "Dimensions"
        case lamps = "Lamp Sets"
        case optical = "Optical"
        case intensity = "Intensity"
        case diagram = "Diagram"
        case validation = "Validation"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .general: return "info.circle"
            case .dimensions: return "ruler"
            case .lamps: return "lightbulb"
            case .optical: return "sun.max"
            case .intensity: return "chart.bar"
            case .diagram: return "circle.grid.cross"
            case .validation: return "checkmark.shield"
            }
        }
    }

    enum DiagramType: String, CaseIterable, Identifiable {
        case polar = "Polar"
        case cartesian = "Cartesian"
        case butterfly = "Butterfly"
        case butterfly3D = "3D"
        case room3D = "Room"
        case heatmap = "Heatmap"
        case bug = "BUG"
        case lcs = "LCS"

        var id: String { rawValue }
    }

    var body: some View {
        ZStack {
            NavigationStack { contentVStack }
            Color.clear.background(notifications)
        }
        .fileImporter(isPresented: $isImporting, allowedContentTypes: [.ldt, .ies], allowsMultipleSelection: false, onCompletion: handleFileImport)
        .background(exporterViews)
    }

    @ViewBuilder
    private var exporterViews: some View {
        if let doc = exportDocument {
            switch doc.type {
            case .svg:
                Color.clear.fileExporter(
                    isPresented: $isExporting,
                    document: SVGDocument(svg: doc.content),
                    contentType: .svgExport,
                    defaultFilename: svgExportFilename
                ) { result in
                    print("DEBUG: SVG fileExporter completion: \(result)")
                    handleExportResult(result)
                    exportDocument = nil
                }
            case .ies:
                Color.clear.fileExporter(
                    isPresented: $isExporting,
                    document: IESDocument(content: doc.content),
                    contentType: .ies,
                    defaultFilename: iesExportFilename
                ) { result in
                    print("DEBUG: IES fileExporter completion: \(result)")
                    handleExportResult(result)
                    exportDocument = nil
                }
            case .ldt:
                Color.clear.fileExporter(
                    isPresented: $isExporting,
                    document: LDTDocument(content: doc.content),
                    contentType: .ldt,
                    defaultFilename: currentFileName.replacingOccurrences(of: ".ldt", with: "_modified")
                ) { result in
                    print("DEBUG: LDT fileExporter completion: \(result)")
                    handleExportResult(result)
                    exportDocument = nil
                }
            }
        }
    }

    @ViewBuilder
    private var notifications: some View {
        Color.clear
            .onReceive(NotificationCenter.default.publisher(for: .openFile)) { _ in isImporting = true }
            .onReceive(NotificationCenter.default.publisher(for: .openExternalFile)) { notification in
                if let url = notification.object as? URL {
                    loadExternalFile(url: url)
                }
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportSVG)) { _ in
                print("DEBUG: Received exportSVG notification")
                triggerSVGExport()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportIES)) { _ in
                print("DEBUG: Received exportIES notification")
                triggerIESExport()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportLDT)) { _ in
                print("DEBUG: Received exportLDT notification")
                triggerLDTExport()
            }
            .onReceive(NotificationCenter.default.publisher(for: .newFromTemplate)) { n in handleTemplate(n) }
            .onReceive(NotificationCenter.default.publisher(for: .openBatchConvert)) { _ in openWindow(id: "batch-convert") }
            #if os(macOS)
            .onReceive(NotificationCenter.default.publisher(for: .setAppStoreSize)) { _ in
                setWindowToAppStoreSize()
            }
            .onReceive(NotificationCenter.default.publisher(for: .exportWatchFace)) { notification in
                if let config = notification.object as? WatchFaceExportConfig {
                    exportWatchFace(config: config)
                }
            }
            #endif
    }

    #if os(macOS)
    /// Sets the window to App Store screenshot size (1440×900)
    private func setWindowToAppStoreSize() {
        guard let window = NSApplication.shared.keyWindow else { return }
        let targetSize = NSSize(width: 1440, height: 900)
        let targetOrigin = NSPoint(x: 50, y: 50)
        window.setFrame(NSRect(origin: targetOrigin, size: targetSize), display: true, animate: false)
    }
    #endif

    private var contentVStack: some View {
        VStack(spacing: 0) {
            tabBar
            if let ldt = Binding($ldt) {
                tabContent(ldt: ldt)
            } else {
                emptyStateView
            }
        }
        .navigationTitle(currentFileName.isEmpty ? "Eulumdat Editor" : currentFileName)
        #if os(macOS)
        .navigationSubtitle(ldt != nil ? "\(ldt!.luminaireName)" : "")
        #endif
        .toolbar { toolbarContent }
        .alert("Error", isPresented: .constant(errorMessage != nil)) {
            Button("OK") { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "")
        }
        .onDrop(of: [.fileURL], isTargeted: $isTargeted, perform: handleDrop)
    }

    private func handleTemplate(_ notification: Notification) {
        if let template = notification.object as? LuminaireTemplate {
            ldt = template.createEulumdat()
            currentFileName = template.rawValue + ".ldt"
            selectedTab = .diagram
        }
    }


    // MARK: - Tab Bar

    private var tabBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(AppTab.allCases) { tab in
                    tabButton(tab)
                }
            }
            .padding(.horizontal, 8)
        }
        .background(Color.controlBackground)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }

    private func tabButton(_ tab: AppTab) -> some View {
        Button {
            withAnimation(.easeInOut(duration: 0.15)) {
                selectedTab = tab
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: tab.icon)
                    .font(.system(size: 12))
                Text(tab.rawValue)
                    .font(.system(size: 13))
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(selectedTab == tab ? Color.accentColor.opacity(0.15) : Color.clear)
            )
            .foregroundColor(selectedTab == tab ? .accentColor : .secondary)
        }
        .buttonStyle(.plain)
        .disabled(ldt == nil && tab != .diagram)
        .accessibilityIdentifier("Tab_\(tab.rawValue)")
    }

    // MARK: - Tab Content

    @ViewBuilder
    private func tabContent(ldt: Binding<Eulumdat>) -> some View {
        switch selectedTab {
        case .general:
            GeneralTabView(ldt: ldt)
        case .dimensions:
            DimensionsTabView(ldt: ldt)
        case .lamps:
            LampSetsTabView(ldt: ldt)
        case .optical:
            OpticalTabView(ldt: ldt)
        case .intensity:
            IntensityTabView(ldt: ldt)
        case .diagram:
            DiagramTabView(ldt: ldt.wrappedValue, selectedDiagram: $selectedDiagram, isDarkTheme: $isDarkTheme)
        case .validation:
            ValidationView(ldt: ldt.wrappedValue)
        }
    }

    // MARK: - Empty State

    private var emptyStateView: some View {
        VStack(spacing: 24) {
            Image(systemName: "lightbulb")
                .font(.system(size: 80))
                .foregroundStyle(.tertiary)

            VStack(spacing: 8) {
                Text("No LDT File Loaded")
                    .font(.title2)
                    .fontWeight(.medium)

                Text("Open an LDT or IES file, or create from template")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            HStack(spacing: 16) {
                Button {
                    isImporting = true
                } label: {
                    Label("Open File", systemImage: "folder")
                }
                .buttonStyle(.borderedProminent)

                Menu {
                    ForEach(LuminaireTemplate.allCases) { template in
                        Button(template.rawValue) {
                            ldt = template.createEulumdat()
                            currentFileName = template.rawValue + ".ldt"
                        }
                    }
                } label: {
                    Label("New from Template", systemImage: "doc.badge.plus")
                }
                .menuStyle(.borderlessButton)
            }

            if isTargeted {
                Text("Drop file here")
                    .font(.headline)
                    .foregroundColor(.accentColor)
                    .padding()
                    .background(
                        RoundedRectangle(cornerRadius: 12)
                            .strokeBorder(style: StrokeStyle(lineWidth: 2, dash: [8]))
                            .foregroundColor(.accentColor)
                    )
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.windowBackground)
        .onTapGesture(count: 2) {
            isImporting = true
        }
    }

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                isImporting = true
            } label: {
                Label("Open", systemImage: "folder")
            }

            if ldt != nil {
                Menu {
                    Button("Export SVG...") {
                        print("DEBUG: Export SVG button clicked")
                        triggerSVGExport()
                    }
                    Button("Export IES...") {
                        print("DEBUG: Export IES button clicked")
                        triggerIESExport()
                    }
                    Button("Export LDT...") {
                        print("DEBUG: Export LDT button clicked")
                        triggerLDTExport()
                    }
                } label: {
                    Label("Export", systemImage: "square.and.arrow.up")
                }

                Toggle(isOn: $isDarkTheme) {
                    Label("Dark", systemImage: isDarkTheme ? "moon.fill" : "sun.max.fill")
                }
            }
        }
    }

    // MARK: - Computed Properties

    private var svgExportFilename: String {
        let baseName = currentFileName
            .replacingOccurrences(of: ".ldt", with: "")
            .replacingOccurrences(of: ".LDT", with: "")
        let diagramName = selectedDiagram.rawValue.lowercased().replacingOccurrences(of: " ", with: "_")
        let name = baseName.isEmpty ? "diagram" : baseName
        return "\(name)_\(diagramName)"
    }

    private var iesExportFilename: String {
        let baseName = currentFileName
            .replacingOccurrences(of: ".ldt", with: "")
            .replacingOccurrences(of: ".LDT", with: "")
        return baseName.isEmpty ? "export" : baseName
    }

    private func triggerSVGExport() {
        guard let ldt = ldt else {
            print("DEBUG: Cannot export - no LDT loaded")
            return
        }
        print("DEBUG: Generating SVG for export")
        let svg = generateSVG(ldt: ldt, size: svgExportSize, theme: isDarkTheme ? .dark : .light)
        print("DEBUG: Generated SVG length: \(svg.count) characters")
        exportDocument = ExportDocument(type: .svg, content: svg)
        print("DEBUG: Set exportDocument to SVG")
        DispatchQueue.main.async {
            print("DEBUG: Setting isExporting = true")
            self.isExporting = true
        }
    }

    private func triggerIESExport() {
        guard let ldt = ldt else { return }
        print("DEBUG: Generating IES for export")
        let ies = exportIes(ldt: ldt)
        print("DEBUG: Generated IES length: \(ies.count) characters")
        exportDocument = ExportDocument(type: .ies, content: ies)
        print("DEBUG: Set exportDocument to IES")
        isExporting = true
    }

    private func triggerLDTExport() {
        guard let ldt = ldt else { return }
        print("DEBUG: Generating LDT for export")
        let ldtContent = exportLdt(ldt: ldt)
        print("DEBUG: Generated LDT length: \(ldtContent.count) characters")
        exportDocument = ExportDocument(type: .ldt, content: ldtContent)
        print("DEBUG: Set exportDocument to LDT")
        isExporting = true
    }

    #if os(macOS)
    /// Export watch face SVG as PNG file
    private func exportWatchFace(config: WatchFaceExportConfig) {
        guard let ldt = ldt else {
            errorMessage = "No luminaire data loaded"
            return
        }

        // Generate SVG
        let svg: String
        if config.size == .complication {
            svg = generateComplicationSvg(ldt: ldt, size: config.size.width)
        } else {
            svg = generatePhotosFaceSvg(ldt: ldt, width: config.size.width, height: config.size.height, style: config.style)
        }

        // Convert SVG to PNG
        guard let svgData = svg.data(using: .utf8),
              let image = NSImage(data: svgData) else {
            // Fallback: save as SVG if PNG conversion fails
            saveWatchFaceSVG(svg: svg, config: config)
            return
        }

        // Create PNG representation
        guard let tiffData = image.tiffRepresentation,
              let bitmap = NSBitmapImageRep(data: tiffData),
              let pngData = bitmap.representation(using: .png, properties: [:]) else {
            saveWatchFaceSVG(svg: svg, config: config)
            return
        }

        // Save dialog
        let savePanel = NSSavePanel()
        savePanel.allowedContentTypes = [.png]
        savePanel.nameFieldStringValue = "\(config.size.filename)_\(styleName(config.style)).png"
        savePanel.title = "Export Watch Face"
        savePanel.message = "Save the watch face image for Apple Watch Photos face"

        savePanel.begin { response in
            if response == .OK, let url = savePanel.url {
                do {
                    try pngData.write(to: url)
                } catch {
                    DispatchQueue.main.async {
                        self.errorMessage = "Failed to save: \(error.localizedDescription)"
                    }
                }
            }
        }
    }

    private func saveWatchFaceSVG(svg: String, config: WatchFaceExportConfig) {
        let savePanel = NSSavePanel()
        savePanel.allowedContentTypes = [.svg]
        savePanel.nameFieldStringValue = "\(config.size.filename)_\(styleName(config.style)).svg"
        savePanel.title = "Export Watch Face (SVG)"

        savePanel.begin { response in
            if response == .OK, let url = savePanel.url {
                do {
                    try svg.write(to: url, atomically: true, encoding: .utf8)
                } catch {
                    DispatchQueue.main.async {
                        self.errorMessage = "Failed to save: \(error.localizedDescription)"
                    }
                }
            }
        }
    }

    private func styleName(_ style: WatchFaceStyleType) -> String {
        switch style {
        case .dark: return "dark"
        case .light: return "light"
        case .minimal: return "minimal"
        case .complication: return "complication"
        case .california: return "california"
        }
    }
    #endif

    private func generateSVG(ldt: Eulumdat, size: Double, theme: SvgThemeType) -> String {
        switch selectedDiagram {
        case .polar:
            return generatePolarSvg(ldt: ldt, width: size, height: size, theme: theme)
        case .cartesian:
            return generateCartesianSvg(ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, theme: theme)
        case .butterfly, .butterfly3D, .room3D:
            return generateButterflySvg(ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, theme: theme)
        case .heatmap:
            return generateHeatmapSvg(ldt: ldt, width: size, height: size * 0.7, theme: theme)
        case .bug:
            return generateBugSvg(ldt: ldt, width: size, height: size * 0.85, theme: theme)
        case .lcs:
            return generateLcsSvg(ldt: ldt, width: size, height: size, theme: theme)
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

    private func handleExportResult(_ result: Result<URL, Error>) {
        if case .failure(let error) = result {
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

        loadFileContents(url: url)
    }

    private func loadExternalFile(url: URL) {
        // Files opened via open command or Finder don't need security-scoped access
        loadFileContents(url: url)
    }

    private func loadFileContents(url: URL) {
        currentFileName = url.lastPathComponent
        let isIesFile = url.pathExtension.lowercased() == "ies"

        do {
            // Try ISO Latin-1 first (common for LDT files), then UTF-8
            let content: String
            if let isoContent = try? String(contentsOf: url, encoding: .isoLatin1) {
                content = isoContent
            } else {
                content = try String(contentsOf: url, encoding: .utf8)
            }

            if isIesFile {
                ldt = try parseIes(content: content)
            } else {
                ldt = try parseLdt(content: content)
            }
            selectedTab = .diagram
        } catch {
            errorMessage = "Failed to parse file: \(error.localizedDescription)"
        }
    }
}

// MARK: - General Tab

struct GeneralTabView: View {
    @Binding var ldt: Eulumdat

    var body: some View {
        Form {
            Section("Identification") {
                TextField("Manufacturer/ID", text: $ldt.identification)
                TextField("Luminaire Name", text: $ldt.luminaireName)
                TextField("Luminaire Number", text: $ldt.luminaireNumber)
                TextField("File Name", text: $ldt.fileName)
                TextField("Date/User", text: $ldt.dateUser)
                TextField("Report Number", text: $ldt.measurementReportNumber)
            }

            Section("Type") {
                Picker("Type Indicator", selection: $ldt.typeIndicator) {
                    Text("Point Source (Symmetric)").tag(TypeIndicator.pointSourceSymmetric)
                    Text("Linear").tag(TypeIndicator.linear)
                    Text("Point Source (Other)").tag(TypeIndicator.pointSourceOther)
                }

                Picker("Symmetry", selection: $ldt.symmetry) {
                    Text("None (Full Data)").tag(Symmetry.none)
                    Text("Vertical Axis").tag(Symmetry.verticalAxis)
                    Text("Plane C0-C180").tag(Symmetry.planeC0c180)
                    Text("Plane C90-C270").tag(Symmetry.planeC90c270)
                    Text("Both Planes").tag(Symmetry.bothPlanes)
                }
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Dimensions Tab

struct DimensionsTabView: View {
    @Binding var ldt: Eulumdat

    var body: some View {
        Form {
            Section("Luminaire Dimensions (mm)") {
                LabeledContent("Length") {
                    TextField("", value: $ldt.length, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                }
                LabeledContent("Width") {
                    TextField("", value: $ldt.width, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                }
                LabeledContent("Height") {
                    TextField("", value: $ldt.height, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                }
            }

            Section("Luminous Area (mm)") {
                LabeledContent("Length") {
                    TextField("", value: $ldt.luminousAreaLength, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                }
                LabeledContent("Width") {
                    TextField("", value: $ldt.luminousAreaWidth, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                }
            }

            Section("Height to Luminous Area (mm)") {
                LabeledContent("C0") {
                    TextField("", value: $ldt.heightC0, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("C90") {
                    TextField("", value: $ldt.heightC90, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("C180") {
                    TextField("", value: $ldt.heightC180, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("C270") {
                    TextField("", value: $ldt.heightC270, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Lamp Sets Tab

struct LampSetsTabView: View {
    @Binding var ldt: Eulumdat

    var body: some View {
        List {
            ForEach(Array(ldt.lampSets.enumerated()), id: \.offset) { index, _ in
                LampSetSection(
                    lampSet: Binding(
                        get: { ldt.lampSets[index] },
                        set: { ldt.lampSets[index] = $0 }
                    ),
                    index: index,
                    onDelete: { ldt.lampSets.remove(at: index) }
                )
            }

            Section {
                Button {
                    ldt.lampSets.append(LampSet(
                        numLamps: 1,
                        lampType: "LED",
                        totalLuminousFlux: 1000,
                        colorAppearance: "3000",
                        colorRenderingGroup: "1A",
                        wattageWithBallast: 10
                    ))
                } label: {
                    Label("Add Lamp Set", systemImage: "plus.circle")
                }
            }
        }
    }
}

struct LampSetSection: View {
    @Binding var lampSet: LampSet
    let index: Int
    let onDelete: () -> Void

    var body: some View {
        Section("Lamp Set \(index + 1)") {
            LabeledContent("Number of Lamps") {
                TextField("", value: $lampSet.numLamps, format: .number)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 60)
            }
            LabeledContent("Type") {
                TextField("", text: $lampSet.lampType)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 150)
            }
            LabeledContent("Luminous Flux (lm)") {
                TextField("", value: $lampSet.totalLuminousFlux, format: .number)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 100)
            }
            LabeledContent("Color Temp (K)") {
                TextField("", text: $lampSet.colorAppearance)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
            }
            LabeledContent("CRI Group") {
                TextField("", text: $lampSet.colorRenderingGroup)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 60)
            }
            LabeledContent("Wattage (W)") {
                TextField("", value: $lampSet.wattageWithBallast, format: .number)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
            }

            Button(role: .destructive, action: onDelete) {
                Label("Remove Lamp Set", systemImage: "trash")
            }
        }
    }
}

// MARK: - Optical Tab

struct OpticalTabView: View {
    @Binding var ldt: Eulumdat

    var body: some View {
        Form {
            Section("Light Output") {
                LabeledContent("Light Output Ratio (%)") {
                    TextField("", value: $ldt.lightOutputRatio, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("Downward Flux (%)") {
                    TextField("", value: $ldt.downwardFluxFraction, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("Tilt Angle (°)") {
                    TextField("", value: $ldt.tiltAngle, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
                LabeledContent("Conversion Factor") {
                    TextField("", value: $ldt.conversionFactor, format: .number)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 80)
                }
            }

            Section("Computed Values") {
                LabeledContent("Max Intensity", value: String(format: "%.1f cd/klm", ldt.maxIntensity))
                LabeledContent("Total Flux", value: String(format: "%.0f lm", ldt.totalLuminousFlux))
            }

            Section("Direct Ratios (Room Index k)") {
                let indices = ["0.60", "0.80", "1.00", "1.25", "1.50", "2.00", "2.50", "3.00", "4.00", "5.00"]
                ForEach(Array(ldt.directRatios.enumerated()), id: \.offset) { index, _ in
                    if index < indices.count {
                        LabeledContent("k = \(indices[index])") {
                            TextField("", value: Binding(
                                get: { ldt.directRatios[index] },
                                set: { ldt.directRatios[index] = $0 }
                            ), format: .number)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 80)
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
    }
}

// MARK: - Intensity Tab

struct IntensityTabView: View {
    @Binding var ldt: Eulumdat
    @State private var showColors = true

    var body: some View {
        VStack(spacing: 0) {
            // Header with toolbar
            HStack {
                Text("Intensities (cd/klm)")
                    .font(.headline)
                Spacer()

                // Copy CSV button
                Button {
                    let csv = generateIntensityCSV(ldt: ldt)
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(csv, forType: .string)
                    #else
                    UIPasteboard.general.string = csv
                    #endif
                } label: {
                    Label("Copy CSV", systemImage: "doc.on.clipboard")
                        .font(.caption)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)

                Divider()
                    .frame(height: 20)

                // Color toggle
                Toggle(isOn: $showColors) {
                    Text("Colors")
                        .font(.caption)
                }
                .toggleStyle(.switch)
                .controlSize(.small)

                Divider()
                    .frame(height: 20)

                Text("Max: \(ldt.maxIntensity, specifier: "%.1f")")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .padding()
            .background(Color.controlBackground)

            Divider()

            // Heatmap table
            if !ldt.intensities.isEmpty && !ldt.cAngles.isEmpty && !ldt.gAngles.isEmpty {
                IntensityHeatmapGrid(ldt: ldt, showColors: showColors)
            } else {
                VStack {
                    Image(systemName: "tablecells")
                        .font(.largeTitle)
                        .foregroundStyle(.secondary)
                    Text("No Intensity Data")
                        .font(.headline)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }

            Divider()

            // Footer
            HStack {
                Text("\(ldt.numCPlanes) C-planes × \(ldt.numGPlanes) γ-angles = \(ldt.numCPlanes * ldt.numGPlanes) values")
                Spacer()
                // Color scale legend (only show when colors enabled)
                if showColors {
                    HStack(spacing: 2) {
                        Text("0")
                            .font(.caption2)
                        ForEach(0..<10, id: \.self) { i in
                            Rectangle()
                                .fill(heatmapColor(normalized: Double(i) / 9.0))
                                .frame(width: 12, height: 12)
                        }
                        Text("max")
                            .font(.caption2)
                    }
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding()
            .background(Color.controlBackground)
        }
    }

    /// Generate tab-separated CSV from intensity data
    private func generateIntensityCSV(ldt: Eulumdat) -> String {
        var csv = ""

        // Header row: gamma, C0, C15, C30, ...
        csv += "gamma"
        for cAngle in ldt.cAngles {
            csv += "\tC\(Int(cAngle))"
        }
        csv += "\n"

        // Data rows
        for (gIdx, gAngle) in ldt.gAngles.enumerated() {
            csv += "\(Int(gAngle))"
            for cIdx in 0..<ldt.cAngles.count {
                let intensity = (cIdx < ldt.intensities.count && gIdx < ldt.intensities[cIdx].count)
                    ? ldt.intensities[cIdx][gIdx]
                    : 0.0
                csv += String(format: "\t%.1f", intensity)
            }
            csv += "\n"
        }

        return csv
    }
}

/// Grid view showing intensity values with heatmap coloring
struct IntensityHeatmapGrid: View {
    let ldt: Eulumdat
    let showColors: Bool
    private let cellWidth: CGFloat = 52
    private let cellHeight: CGFloat = 22
    private let headerWidth: CGFloat = 45

    var body: some View {
        ScrollView([.horizontal, .vertical], showsIndicators: true) {
            VStack(alignment: .leading, spacing: 0) {
                // Header row with C-angles
                HStack(spacing: 0) {
                    // Corner cell (γ \ C)
                    Text("γ \\ C")
                        .font(.system(size: 10, weight: .semibold, design: .monospaced))
                        .frame(width: headerWidth, height: cellHeight)
                        .background(Color.secondary.opacity(0.15))
                        .border(Color.secondary.opacity(0.3), width: 0.5)

                    // C-angle headers
                    ForEach(Array(ldt.cAngles.enumerated()), id: \.offset) { _, cAngle in
                        Text("\(Int(cAngle))")
                            .font(.system(size: 9, weight: .medium, design: .monospaced))
                            .frame(width: cellWidth, height: cellHeight)
                            .background(Color.secondary.opacity(0.15))
                            .border(Color.secondary.opacity(0.3), width: 0.5)
                    }
                }

                // Data rows
                ForEach(Array(ldt.gAngles.enumerated()), id: \.offset) { gIndex, gAngle in
                    HStack(spacing: 0) {
                        // Row header (γ-angle)
                        Text("\(Int(gAngle))")
                            .font(.system(size: 10, weight: .medium, design: .monospaced))
                            .frame(width: headerWidth, height: cellHeight)
                            .background(Color.secondary.opacity(0.1))
                            .border(Color.secondary.opacity(0.3), width: 0.5)

                        // Intensity values
                        ForEach(Array(ldt.cAngles.enumerated()), id: \.offset) { cIndex, _ in
                            let intensity = getIntensity(cIndex: cIndex, gIndex: gIndex)
                            let normalized = ldt.maxIntensity > 0 ? intensity / ldt.maxIntensity : 0

                            if showColors {
                                Text(formatIntensity(intensity))
                                    .font(.system(size: 9, design: .monospaced))
                                    .foregroundColor(textColor(for: normalized))
                                    .frame(width: cellWidth, height: cellHeight)
                                    .background(heatmapColor(normalized: normalized))
                                    .border(Color.secondary.opacity(0.2), width: 0.5)
                            } else {
                                Text(formatIntensity(intensity))
                                    .font(.system(size: 9, design: .monospaced))
                                    .frame(width: cellWidth, height: cellHeight)
                                    .border(Color.secondary.opacity(0.2), width: 0.5)
                            }
                        }
                    }
                }
            }
            .padding(8)
        }
    }

    private func getIntensity(cIndex: Int, gIndex: Int) -> Double {
        guard cIndex < ldt.intensities.count,
              gIndex < ldt.intensities[cIndex].count else {
            return 0
        }
        return ldt.intensities[cIndex][gIndex]
    }

    private func formatIntensity(_ value: Double) -> String {
        if value >= 10000 {
            return String(format: "%.0f", value)
        } else if value >= 1000 {
            return String(format: "%.0f", value)
        } else if value >= 100 {
            return String(format: "%.1f", value)
        } else {
            return String(format: "%.1f", value)
        }
    }

    private func textColor(for normalized: Double) -> SwiftUI.Color {
        // Use dark text for light backgrounds, light text for dark backgrounds
        return normalized > 0.5 ? .white : .primary
    }
}

/// Heatmap color function (blue -> cyan -> green -> yellow -> red)
func heatmapColor(normalized: Double) -> SwiftUI.Color {
    let value = max(0, min(1, normalized))

    let r: Double
    let g: Double
    let b: Double

    if value < 0.25 {
        // Blue to Cyan
        let t = value / 0.25
        r = 0
        g = t
        b = 1
    } else if value < 0.5 {
        // Cyan to Green
        let t = (value - 0.25) / 0.25
        r = 0
        g = 1
        b = 1 - t
    } else if value < 0.75 {
        // Green to Yellow
        let t = (value - 0.5) / 0.25
        r = t
        g = 1
        b = 0
    } else {
        // Yellow to Red
        let t = (value - 0.75) / 0.25
        r = 1
        g = 1 - t
        b = 0
    }

    return SwiftUI.Color(red: r, green: g, blue: b)
}

// MARK: - macOS Zoomable Scroll Container

#if os(macOS)
import AppKit

/// A container that wraps content in an NSScrollView with Cmd+scroll zoom and Cmd+drag pan
struct ZoomableScrollContainer<Content: View>: NSViewRepresentable {
    @Binding var zoomScale: CGFloat
    var onDoubleClick: () -> Void
    let content: Content

    init(zoomScale: Binding<CGFloat>, onDoubleClick: @escaping () -> Void, @ViewBuilder content: () -> Content) {
        self._zoomScale = zoomScale
        self.onDoubleClick = onDoubleClick
        self.content = content()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = ZoomableNSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.scrollerStyle = .overlay
        scrollView.drawsBackground = false
        scrollView.backgroundColor = .clear

        // Create hosting view for SwiftUI content
        let hostingView = NSHostingView(rootView: content)
        hostingView.translatesAutoresizingMaskIntoConstraints = false

        // Create a flipped document view (required for proper scrolling)
        let documentView = FlippedView()
        documentView.translatesAutoresizingMaskIntoConstraints = false
        documentView.addSubview(hostingView)

        scrollView.documentView = documentView

        // Pin hosting view to document view
        NSLayoutConstraint.activate([
            hostingView.leadingAnchor.constraint(equalTo: documentView.leadingAnchor),
            hostingView.trailingAnchor.constraint(equalTo: documentView.trailingAnchor),
            hostingView.topAnchor.constraint(equalTo: documentView.topAnchor),
            hostingView.bottomAnchor.constraint(equalTo: documentView.bottomAnchor)
        ])

        // Store callbacks
        scrollView.onZoom = { delta in
            DispatchQueue.main.async {
                let zoomDelta = delta > 0 ? 1.1 : 0.9
                context.coordinator.parent.zoomScale = min(max(context.coordinator.parent.zoomScale * zoomDelta, 0.5), 5.0)
            }
        }
        scrollView.onDoubleClick = onDoubleClick

        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let zoomableScrollView = scrollView as? ZoomableNSScrollView else { return }

        // Update callbacks
        zoomableScrollView.onDoubleClick = onDoubleClick

        // Update content
        if let documentView = scrollView.documentView,
           let hostingView = documentView.subviews.first as? NSHostingView<Content> {
            hostingView.rootView = content
        }
    }

    class Coordinator {
        var parent: ZoomableScrollContainer

        init(_ parent: ZoomableScrollContainer) {
            self.parent = parent
        }
    }
}

/// A flipped NSView for proper scroll coordinate system
class FlippedView: NSView {
    override var isFlipped: Bool { true }
}

/// Custom NSScrollView that handles Cmd+scroll for zoom and Cmd+drag for pan
class ZoomableNSScrollView: NSScrollView {
    var onZoom: ((CGFloat) -> Void)?
    var onDoubleClick: (() -> Void)?

    private var isDragging = false
    private var lastDragLocation: NSPoint?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setupGestures()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupGestures()
    }

    private func setupGestures() {
        // Add double-click gesture
        let doubleClick = NSClickGestureRecognizer(target: self, action: #selector(handleDoubleClick))
        doubleClick.numberOfClicksRequired = 2
        self.addGestureRecognizer(doubleClick)

        // Add tracking area
        let trackingArea = NSTrackingArea(
            rect: bounds,
            options: [.activeInKeyWindow, .inVisibleRect, .mouseMoved, .cursorUpdate, .mouseEnteredAndExited],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
    }

    @objc private func handleDoubleClick() {
        onDoubleClick?()
    }

    override func scrollWheel(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            // Cmd+scroll = zoom
            onZoom?(event.scrollingDeltaY)
        } else {
            // Normal scroll
            super.scrollWheel(with: event)
        }
    }

    override func mouseDown(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            // Start Cmd+drag pan
            isDragging = true
            lastDragLocation = event.locationInWindow
            NSCursor.closedHand.push()
        } else {
            super.mouseDown(with: event)
        }
    }

    override func mouseDragged(with event: NSEvent) {
        if isDragging, let lastLocation = lastDragLocation {
            let currentLocation = event.locationInWindow
            let deltaX = currentLocation.x - lastLocation.x
            let deltaY = currentLocation.y - lastLocation.y

            // Pan the scroll view (inverted for natural mouse movement)
            var newOrigin = contentView.bounds.origin
            newOrigin.x -= deltaX  // Inverted: mouse moves right, content moves left (revealing content on right)
            newOrigin.y += deltaY  // Natural: mouse moves down, content moves down

            // Clamp to valid bounds
            let contentSize = documentView?.frame.size ?? .zero
            let visibleSize = contentView.bounds.size
            newOrigin.x = max(0, min(newOrigin.x, max(0, contentSize.width - visibleSize.width)))
            newOrigin.y = max(0, min(newOrigin.y, max(0, contentSize.height - visibleSize.height)))

            contentView.scroll(to: newOrigin)
            reflectScrolledClipView(contentView)

            lastDragLocation = currentLocation
        } else {
            super.mouseDragged(with: event)
        }
    }

    override func mouseUp(with event: NSEvent) {
        if isDragging {
            isDragging = false
            lastDragLocation = nil
            NSCursor.pop()
        } else {
            super.mouseUp(with: event)
        }
    }

    override func cursorUpdate(with event: NSEvent) {
        if NSEvent.modifierFlags.contains(.command) && !isDragging {
            NSCursor.openHand.set()
        }
    }

    override func flagsChanged(with event: NSEvent) {
        if event.modifierFlags.contains(.command) && !isDragging {
            NSCursor.openHand.push()
        } else if !isDragging {
            NSCursor.pop()
        }
        super.flagsChanged(with: event)
    }

    override func mouseExited(with event: NSEvent) {
        if !isDragging {
            NSCursor.arrow.set()
        }
        super.mouseExited(with: event)
    }
}
#endif

// MARK: - Diagram Tab

struct DiagramTabView: View {
    let ldt: Eulumdat
    @Binding var selectedDiagram: ContentView.DiagramType
    @Binding var isDarkTheme: Bool
    @State private var isFullscreen = false
    @State private var showDiagramInfo = true
    @State private var zoomScale: CGFloat = 1.0
    @State private var lastZoomScale: CGFloat = 1.0
    @State private var scrollOffset: CGSize = .zero
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(spacing: 0) {
            // Info summary at top
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(ldt.luminaireName)
                        .font(.headline)
                        .lineLimit(1)
                    Text(ldt.identification)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                Label("\(ldt.maxIntensity, specifier: "%.0f") cd/klm", systemImage: "sun.max.fill")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Label("\(ldt.totalLuminousFlux, specifier: "%.0f") lm", systemImage: "lightbulb.fill")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
            .background(Color.controlBackground)

            Divider()

            // Diagram type picker
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 4) {
                    ForEach(ContentView.DiagramType.allCases) { type in
                        Button {
                            selectedDiagram = type
                        } label: {
                            Text(type.rawValue)
                                .font(.system(size: 12, weight: selectedDiagram == type ? .semibold : .regular))
                                .padding(.horizontal, 12)
                                .padding(.vertical, 6)
                                .background(
                                    RoundedRectangle(cornerRadius: 6)
                                        .fill(selectedDiagram == type ? Color.accentColor : Color.secondary.opacity(0.1))
                                )
                                .foregroundColor(selectedDiagram == type ? .white : .primary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
            }
            .background(Color.controlBackground)

            Divider()

            // Diagram content
            GeometryReader { geometry in
                #if os(iOS)
                // On iOS, use more available space - especially for iPad
                let availableWidth = geometry.size.width - 32
                let availableHeight = geometry.size.height - 32
                let ratio = aspectRatio(for: selectedDiagram)
                // Calculate size based on aspect ratio to fill available space
                let size = min(availableWidth, availableHeight / ratio)
                #else
                // On macOS, scale with window size
                let availableWidth = geometry.size.width - 32
                let availableHeight = geometry.size.height - 32
                let ratio = aspectRatio(for: selectedDiagram)
                let size = min(availableWidth, availableHeight / ratio)
                #endif

                #if os(macOS)
                // macOS: Use custom ZoomableScrollContainer for proper Cmd+drag panning
                ZStack {
                    ZoomableScrollContainer(zoomScale: $zoomScale, onDoubleClick: {
                        // Always open in a new window on double-click
                        DiagramWindowModel.shared.ldt = ldt
                        DiagramWindowModel.shared.selectedDiagram = selectedDiagram
                        DiagramWindowModel.shared.isDarkTheme = isDarkTheme
                        openWindow(id: "diagram-viewer")
                    }) {
                        HStack {
                            Spacer(minLength: 0)
                            VStack {
                                Spacer(minLength: 0)
                                diagramContent(size: size)
                                Spacer(minLength: 0)
                            }
                            Spacer(minLength: 0)
                        }
                        .frame(minWidth: geometry.size.width, minHeight: geometry.size.height)
                        .padding()
                    }
                    .accessibilityIdentifier("DiagramScrollView")

                    // Overlay for size/zoom info (non-interactive)
                    VStack {
                        HStack {
                            Spacer()
                            VStack(alignment: .trailing, spacing: 4) {
                                Text("\(Int(size * zoomScale))×\(Int(size * aspectRatio(for: selectedDiagram) * zoomScale))px")
                                    .font(.caption)
                                    .monospacedDigit()
                                Text("\(Int(zoomScale * 100))%")
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                                    .accessibilityIdentifier("ZoomPercentage")
                                if zoomScale > 1.0 {
                                    Text("⌘+drag to pan")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                        .accessibilityIdentifier("PanHint")
                                }
                            }
                            .padding(8)
                            .background(.ultraThinMaterial)
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                            .padding(8)
                        }
                        Spacer()
                    }
                    .allowsHitTesting(false)
                }
                #else
                // iOS: Use standard ScrollView with gestures
                ScrollView([.horizontal, .vertical], showsIndicators: true) {
                    ZStack(alignment: .topTrailing) {
                        diagramContent(size: size)

                        // Size overlay
                        VStack(alignment: .trailing, spacing: 4) {
                            Text("\(Int(size * zoomScale))×\(Int(size * aspectRatio(for: selectedDiagram) * zoomScale))px")
                                .font(.caption)
                                .monospacedDigit()
                            Text("\(Int(zoomScale * 100))%")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .accessibilityIdentifier("ZoomPercentage")
                        }
                        .padding(8)
                        .background(.ultraThinMaterial)
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                        .padding(8)
                        .allowsHitTesting(false)
                    }
                    .padding()
                }
                .onTapGesture(count: 2) {
                    isFullscreen.toggle()
                }
                .gesture(
                    MagnificationGesture()
                        .onChanged { value in
                            let delta = value / lastZoomScale
                            lastZoomScale = value
                            zoomScale = min(max(zoomScale * delta, 0.5), 5.0)
                        }
                        .onEnded { _ in
                            lastZoomScale = 1.0
                        }
                )
                .fullScreenCover(isPresented: $isFullscreen) {
                    DiagramFullscreenView(
                        ldt: ldt,
                        selectedDiagram: $selectedDiagram,
                        isDarkTheme: $isDarkTheme,
                        isPresented: $isFullscreen
                    )
                }
                #endif

                // Zoom controls for macOS
                #if os(macOS)
                VStack {
                    Spacer()
                    HStack {
                        Spacer()
                        HStack(spacing: 8) {
                            Button {
                                zoomScale = max(zoomScale - 0.25, 0.5)
                            } label: {
                                Image(systemName: "minus.magnifyingglass")
                            }
                            .keyboardShortcut("-", modifiers: .command)

                            Button {
                                zoomScale = 1.0
                            } label: {
                                Text("\(Int(zoomScale * 100))%")
                                    .monospacedDigit()
                                    .frame(minWidth: 50)
                            }
                            .keyboardShortcut("0", modifiers: .command)

                            Button {
                                zoomScale = min(zoomScale + 0.25, 5.0)
                            } label: {
                                Image(systemName: "plus.magnifyingglass")
                            }
                            .keyboardShortcut("+", modifiers: .command)

                            Divider()
                                .frame(height: 20)

                            Button {
                                isFullscreen.toggle()
                            } label: {
                                Image(systemName: "arrow.up.left.and.arrow.down.right")
                            }
                            .keyboardShortcut("f", modifiers: .command)
                            .help("Fullscreen (⌘F)")
                            .accessibilityIdentifier("FullscreenButton")
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 8)
                        .background(.ultraThinMaterial)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .padding()
                    }
                }
                #endif
            }
        }
    }

    @ViewBuilder
    private func diagramContent(size: CGFloat) -> some View {
        if selectedDiagram == .butterfly3D {
            Butterfly3DView(ldt: ldt, isDarkTheme: $isDarkTheme)
                .frame(width: size * zoomScale, height: size * 0.8 * zoomScale)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .shadow(radius: 2)
                .accessibilityIdentifier("Diagram3DView")
        } else if selectedDiagram == .room3D {
            Room3DView(ldt: ldt, isDarkTheme: $isDarkTheme)
                .frame(width: size * zoomScale, height: size * 0.8 * zoomScale)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .accessibilityIdentifier("DiagramRoom3DView")
        } else {
            let theme: SvgThemeType = isDarkTheme ? .dark : .light
            SVGView(svgString: generateSVG(type: selectedDiagram, ldt: ldt, size: size * zoomScale, theme: theme))
                .frame(width: size * zoomScale, height: size * aspectRatio(for: selectedDiagram) * zoomScale)
                .background(isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .shadow(radius: 2)
                .accessibilityIdentifier("DiagramSVGView")
        }
    }

    private func generateSVG(type: ContentView.DiagramType, ldt: Eulumdat, size: Double, theme: SvgThemeType) -> String {
        switch type {
        case .polar:
            return generatePolarSvg(ldt: ldt, width: size, height: size, theme: theme)
        case .cartesian:
            return generateCartesianSvg(ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, theme: theme)
        case .butterfly, .butterfly3D, .room3D:
            return generateButterflySvg(ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, theme: theme)
        case .heatmap:
            return generateHeatmapSvg(ldt: ldt, width: size, height: size * 0.7, theme: theme)
        case .bug:
            return generateBugSvg(ldt: ldt, width: size, height: size * 0.85, theme: theme)
        case .lcs:
            return generateLcsSvg(ldt: ldt, width: size, height: size, theme: theme)
        }
    }

    private func aspectRatio(for diagram: ContentView.DiagramType) -> Double {
        switch diagram {
        case .polar, .lcs: return 1.0
        case .bug: return 0.85
        case .butterfly, .butterfly3D, .room3D: return 0.8
        case .cartesian: return 0.75
        case .heatmap: return 0.7
        }
    }
}


// MARK: - BUG Rating Edit View

struct BugRatingEditView: View {
    let ldt: Eulumdat
    @Binding var overrideEnabled: Bool
    @Binding var overrideB: UInt8
    @Binding var overrideU: UInt8
    @Binding var overrideG: UInt8

    private var calculatedRating: BugRatingData {
        calculateBugRating(ldt: ldt)
    }

    private var displayB: UInt8 { overrideEnabled ? overrideB : calculatedRating.b }
    private var displayU: UInt8 { overrideEnabled ? overrideU : calculatedRating.u }
    private var displayG: UInt8 { overrideEnabled ? overrideG : calculatedRating.g }

    var body: some View {
        VStack(spacing: 16) {
            // Main BUG rating display
            HStack(spacing: 20) {
                bugRatingInput("B", subtitle: "Backlight", value: displayB, binding: $overrideB)
                bugRatingInput("U", subtitle: "Uplight", value: displayU, binding: $overrideU)
                bugRatingInput("G", subtitle: "Glare", value: displayG, binding: $overrideG)
            }
            .padding(.vertical, 8)

            // Combined rating display
            HStack {
                Text("Combined Rating:")
                    .foregroundStyle(.secondary)
                Text("B\(displayB) U\(displayU) G\(displayG)")
                    .font(.headline)
                    .fontWeight(.bold)
                    .foregroundStyle(ratingColor(max(displayB, displayU, displayG)))
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(ratingBackgroundColor(max(displayB, displayU, displayG)))
                    .clipShape(RoundedRectangle(cornerRadius: 6))
            }

            if overrideEnabled {
                // Show calculated vs override comparison
                HStack {
                    Text("Calculated:")
                        .foregroundStyle(.secondary)
                    Text("B\(calculatedRating.b) U\(calculatedRating.u) G\(calculatedRating.g)")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    Spacer()
                    Button("Reset to Calculated") {
                        overrideB = calculatedRating.b
                        overrideU = calculatedRating.u
                        overrideG = calculatedRating.g
                    }
                    .buttonStyle(.borderless)
                    .font(.caption)
                }
            }

            // Rating legend
            Divider()
            HStack(spacing: 16) {
                ratingLegendItem(0...1, label: "Excellent", color: .green)
                ratingLegendItem(2...2, label: "Good", color: .yellow)
                ratingLegendItem(3...3, label: "Fair", color: .orange)
                ratingLegendItem(4...5, label: "Poor", color: .red)
            }
            .font(.caption2)
        }
    }

    @ViewBuilder
    private func bugRatingInput(_ label: String, subtitle: String, value: UInt8, binding: Binding<UInt8>) -> some View {
        VStack(spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)

            if overrideEnabled {
                // Editable stepper
                HStack(spacing: 0) {
                    Button {
                        if binding.wrappedValue > 0 { binding.wrappedValue -= 1 }
                    } label: {
                        Image(systemName: "minus")
                            .frame(width: 24, height: 24)
                    }
                    .buttonStyle(.bordered)

                    Text("\(value)")
                        .font(.title)
                        .fontWeight(.bold)
                        .frame(width: 40)
                        .foregroundStyle(ratingColor(value))

                    Button {
                        if binding.wrappedValue < 5 { binding.wrappedValue += 1 }
                    } label: {
                        Image(systemName: "plus")
                            .frame(width: 24, height: 24)
                    }
                    .buttonStyle(.bordered)
                }
            } else {
                // Read-only display
                Text("\(value)")
                    .font(.title)
                    .fontWeight(.bold)
                    .foregroundStyle(ratingColor(value))
            }

            Text(subtitle)
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .frame(width: 100)
        .padding()
        .background(ratingBackgroundColor(value).opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .strokeBorder(ratingColor(value).opacity(0.5), lineWidth: 2)
        )
    }

    private func ratingLegendItem(_ range: ClosedRange<UInt8>, label: String, color: SwiftUI.Color) -> some View {
        HStack(spacing: 4) {
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
            Text("\(range.lowerBound)-\(range.upperBound): \(label)")
                .foregroundStyle(.secondary)
        }
    }

    private func ratingColor(_ value: UInt8) -> SwiftUI.Color {
        switch value {
        case 0...1: return .green
        case 2: return .yellow
        case 3: return .orange
        default: return .red
        }
    }

    private func ratingBackgroundColor(_ value: UInt8) -> SwiftUI.Color {
        switch value {
        case 0...1: return SwiftUI.Color.green.opacity(0.15)
        case 2: return SwiftUI.Color.yellow.opacity(0.15)
        case 3: return SwiftUI.Color.orange.opacity(0.15)
        default: return SwiftUI.Color.red.opacity(0.15)
        }
    }
}

// MARK: - Custom UTTypes

extension UTType {
    static var ldt: UTType {
        UTType(filenameExtension: "ldt", conformingTo: .text) ?? UTType(exportedAs: "com.eulumdat.ldt")
    }

    static var ies: UTType {
        UTType(filenameExtension: "ies", conformingTo: .text) ?? UTType(exportedAs: "com.ies.photometric")
    }

    static var svgExport: UTType {
        UTType(filenameExtension: "svg", conformingTo: .xml) ?? UTType(exportedAs: "public.svg-image")
    }
}

// MARK: - Document Types

struct SVGDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.svgExport] }
    static var writableContentTypes: [UTType] { [.svgExport] }
    var svg: String

    init(svg: String) { self.svg = svg }

    init(configuration: ReadConfiguration) throws {
        svg = configuration.file.regularFileContents.flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        print("DEBUG: SVGDocument fileWrapper called, svg length: \(svg.count)")
        return FileWrapper(regularFileWithContents: svg.data(using: .utf8) ?? Data())
    }
}

struct IESDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.ies] }
    static var writableContentTypes: [UTType] { [.ies] }
    var content: String

    init(content: String) { self.content = content }

    init(configuration: ReadConfiguration) throws {
        content = configuration.file.regularFileContents.flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        print("DEBUG: IESDocument fileWrapper called, content length: \(content.count)")
        return FileWrapper(regularFileWithContents: content.data(using: .utf8) ?? Data())
    }
}

struct LDTDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.ldt] }
    static var writableContentTypes: [UTType] { [.ldt] }
    var content: String

    init(content: String) { self.content = content }

    init(configuration: ReadConfiguration) throws {
        content = configuration.file.regularFileContents.flatMap { String(data: $0, encoding: .utf8) } ?? ""
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: content.data(using: .utf8) ?? Data())
    }
}

// MARK: - Diagram Window View (Standalone Window)

#if os(macOS)
struct DiagramWindowView: View {
    @ObservedObject var model = DiagramWindowModel.shared
    @State private var zoomScale: CGFloat = 1.0
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        if let ldt = model.ldt {
            // Room3D has its own controls, so render it without zoom container/overlays
            if model.selectedDiagram == .room3D {
                Room3DView(ldt: ldt, isDarkTheme: $model.isDarkTheme)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .accessibilityIdentifier("WindowDiagramRoom3D")
                    .navigationTitle("Room - \(ldt.luminaireName)")
                    .navigationSubtitle("Max: \(Int(ldt.maxIntensity)) cd/klm • Total: \(Int(ldt.totalLuminousFlux)) lm")
                    .toolbar {
                        ToolbarItem(placement: .primaryAction) {
                            Toggle(isOn: $model.isDarkTheme) {
                                Label("Dark Theme", systemImage: model.isDarkTheme ? "moon.fill" : "sun.max.fill")
                            }
                        }
                    }
            } else {
                GeometryReader { geometry in
                    let availableWidth = geometry.size.width - 80
                    let availableHeight = geometry.size.height - 80
                    let ratio = aspectRatio(for: model.selectedDiagram)
                    let baseSize = min(availableWidth, availableHeight / ratio)

                    // Use ZoomableScrollContainer for pan and zoom support
                    ZStack {
                        // Background
                        (model.isDarkTheme ? Color.black : Color.white)
                            .ignoresSafeArea()

                        ZoomableScrollContainer(zoomScale: $zoomScale, onDoubleClick: {
                            // Double-click does nothing in fullscreen window
                        }) {
                            if model.selectedDiagram == .butterfly3D {
                                Butterfly3DView(ldt: ldt, isDarkTheme: $model.isDarkTheme)
                                    .frame(width: baseSize * zoomScale, height: baseSize * ratio * zoomScale)
                                    .clipShape(RoundedRectangle(cornerRadius: 12))
                                    .accessibilityIdentifier("WindowDiagram3D")
                            } else {
                                let theme: SvgThemeType = model.isDarkTheme ? .dark : .light
                                SVGView(svgString: generateSVG(type: model.selectedDiagram, ldt: ldt, size: baseSize * zoomScale, theme: theme))
                                    .frame(width: baseSize * zoomScale, height: baseSize * ratio * zoomScale)
                                    .background(model.isDarkTheme ? Color.black : Color.white)
                                    .clipShape(RoundedRectangle(cornerRadius: 12))
                                    .shadow(radius: 4)
                                    .accessibilityIdentifier("WindowDiagramSVG")
                            }
                        }

                        // Zoom info overlay (top-right, non-interactive)
                        VStack {
                            HStack {
                                Spacer()
                                VStack(alignment: .trailing, spacing: 4) {
                                    Text("\(Int(baseSize * zoomScale))×\(Int(baseSize * ratio * zoomScale))px")
                                        .font(.caption)
                                        .monospacedDigit()
                                    Text("\(Int(zoomScale * 100))%")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    if zoomScale > 1.0 {
                                        Text("⌘+drag to pan")
                                            .font(.caption2)
                                            .foregroundStyle(.secondary)
                                    }
                                }
                                .padding(8)
                                .background(.ultraThinMaterial)
                                .clipShape(RoundedRectangle(cornerRadius: 6))
                                .padding(8)
                            }
                            Spacer()
                        }
                        .allowsHitTesting(false)

                        // Zoom controls (bottom center)
                        VStack {
                            Spacer()
                            HStack(spacing: 12) {
                                Button {
                                    zoomScale = max(zoomScale - 0.25, 0.5)
                                } label: {
                                    Image(systemName: "minus.magnifyingglass")
                                        .font(.title2)
                                }
                                .keyboardShortcut("-", modifiers: .command)

                                Text("\(Int(zoomScale * 100))%")
                                    .monospacedDigit()
                                    .frame(minWidth: 60)
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 4)
                                    .background(.ultraThinMaterial)
                                    .clipShape(RoundedRectangle(cornerRadius: 6))

                                Button {
                                    zoomScale = min(zoomScale + 0.25, 5.0)
                                } label: {
                                    Image(systemName: "plus.magnifyingglass")
                                        .font(.title2)
                                }
                                .keyboardShortcut("+", modifiers: .command)

                                Divider()
                                    .frame(height: 20)

                                Button {
                                    zoomScale = 1.0
                                } label: {
                                    Text("Reset")
                                }
                                .keyboardShortcut("0", modifiers: .command)
                            }
                            .padding()
                            .background(.ultraThinMaterial)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                            .padding()
                        }
                    }
                }
                .navigationTitle("\(model.selectedDiagram.rawValue) - \(ldt.luminaireName)")
                .navigationSubtitle("Max: \(Int(ldt.maxIntensity)) cd/klm • Total: \(Int(ldt.totalLuminousFlux)) lm")
                .toolbar {
                    ToolbarItem(placement: .primaryAction) {
                        Toggle(isOn: $model.isDarkTheme) {
                            Label("Dark Theme", systemImage: model.isDarkTheme ? "moon.fill" : "sun.max.fill")
                        }
                    }
                }
            }
        } else {
            VStack(spacing: 16) {
                Image(systemName: "circle.grid.cross")
                    .font(.system(size: 80))
                    .foregroundStyle(.tertiary)
                Text("No diagram loaded")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
            .frame(minWidth: 800, minHeight: 600)
        }
    }

    private func generateSVG(type: ContentView.DiagramType, ldt: Eulumdat, size: Double, theme: SvgThemeType) -> String {
        switch type {
        case .polar:
            return generatePolarSvg(ldt: ldt, width: size, height: size, theme: theme)
        case .cartesian:
            return generateCartesianSvg(ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, theme: theme)
        case .butterfly, .butterfly3D, .room3D:
            return generateButterflySvg(ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, theme: theme)
        case .heatmap:
            return generateHeatmapSvg(ldt: ldt, width: size, height: size * 0.7, theme: theme)
        case .bug:
            return generateBugSvg(ldt: ldt, width: size, height: size * 0.85, theme: theme)
        case .lcs:
            return generateLcsSvg(ldt: ldt, width: size, height: size, theme: theme)
        }
    }

    private func aspectRatio(for diagram: ContentView.DiagramType) -> Double {
        switch diagram {
        case .polar, .lcs: return 1.0
        case .bug: return 0.85
        case .butterfly, .butterfly3D, .room3D: return 0.8
        case .cartesian: return 0.75
        case .heatmap: return 0.7
        }
    }
}
#endif

// MARK: - iOS Fullscreen Diagram View

#if os(iOS)
struct DiagramFullscreenView: View {
    let ldt: Eulumdat
    @Binding var selectedDiagram: ContentView.DiagramType
    @Binding var isDarkTheme: Bool
    @Binding var isPresented: Bool
    @State private var zoomScale: CGFloat = 1.0

    var body: some View {
        // Room3D has its own controls, render without navigation chrome
        if selectedDiagram == .room3D {
            ZStack {
                Room3DView(ldt: ldt, isDarkTheme: $isDarkTheme)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .ignoresSafeArea()

                // Minimal close button overlay
                VStack {
                    HStack {
                        Button {
                            isPresented = false
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .font(.title)
                                .foregroundStyle(.white.opacity(0.8))
                                .background(Circle().fill(.black.opacity(0.3)))
                        }
                        .padding()
                        Spacer()
                    }
                    Spacer()
                }
            }
            .background(Color.black)
        } else {
            NavigationStack {
                GeometryReader { geometry in
                    let size = min(geometry.size.width, geometry.size.height) - 40

                    ScrollView([.horizontal, .vertical], showsIndicators: true) {
                        VStack {
                            Spacer(minLength: 0)
                            HStack {
                                Spacer(minLength: 0)
                                diagramContent(size: size)
                                Spacer(minLength: 0)
                            }
                            Spacer(minLength: 0)
                        }
                        .frame(minWidth: geometry.size.width, minHeight: geometry.size.height)
                    }
                }
                .background(isDarkTheme ? Color.black : Color.white)
                .navigationTitle(ldt.luminaireName)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Done") {
                            isPresented = false
                        }
                    }
                    ToolbarItem(placement: .primaryAction) {
                        Toggle(isOn: $isDarkTheme) {
                            Image(systemName: isDarkTheme ? "moon.fill" : "sun.max.fill")
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func diagramContent(size: CGFloat) -> some View {
        if selectedDiagram == .butterfly3D {
            Butterfly3DView(ldt: ldt, isDarkTheme: $isDarkTheme)
                .frame(width: size * zoomScale, height: size * 0.8 * zoomScale)
                .clipShape(RoundedRectangle(cornerRadius: 12))
        } else {
            let theme: SvgThemeType = isDarkTheme ? .dark : .light
            SVGView(svgString: generateSVG(type: selectedDiagram, ldt: ldt, size: size * zoomScale, theme: theme))
                .frame(width: size * zoomScale, height: size * aspectRatio(for: selectedDiagram) * zoomScale)
                .background(isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
        }
    }

    private func generateSVG(type: ContentView.DiagramType, ldt: Eulumdat, size: Double, theme: SvgThemeType) -> String {
        switch type {
        case .polar:
            return generatePolarSvg(ldt: ldt, width: size, height: size, theme: theme)
        case .cartesian:
            return generateCartesianSvg(ldt: ldt, width: size, height: size * 0.75, maxCurves: 8, theme: theme)
        case .butterfly, .butterfly3D, .room3D:
            return generateButterflySvg(ldt: ldt, width: size, height: size * 0.8, tiltDegrees: 60, theme: theme)
        case .heatmap:
            return generateHeatmapSvg(ldt: ldt, width: size, height: size * 0.7, theme: theme)
        case .bug:
            return generateBugSvg(ldt: ldt, width: size, height: size * 0.85, theme: theme)
        case .lcs:
            return generateLcsSvg(ldt: ldt, width: size, height: size, theme: theme)
        }
    }

    private func aspectRatio(for diagram: ContentView.DiagramType) -> Double {
        switch diagram {
        case .polar, .lcs: return 1.0
        case .bug: return 0.85
        case .butterfly, .butterfly3D, .room3D: return 0.8
        case .cartesian: return 0.75
        case .heatmap: return 0.7
        }
    }
}
#endif
