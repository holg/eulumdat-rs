import SwiftUI
import EulumdatKit
import UniformTypeIdentifiers

struct CompareView: View {
    let ldt: Eulumdat
    @Binding var isDarkTheme: Bool
    @State private var ldtB: Eulumdat?
    @State private var fileBName: String = ""
    @State private var isImportingB = false
    @State private var selectedMode: CompareMode = .polarOverlay
    @State private var cPlaneA: Double = 0
    @State private var cPlaneB: Double = 0
    @State private var linkSliders: Bool = true
    @State private var isFullscreen = false
    @AppStorage("mountingHeight") private var mountingHeight = 3.0
    @AppStorage("tiltAngle") private var tiltAngle = 0.0
    @AppStorage("areaSize") private var areaSize = 20.0
    @Environment(\.openWindow) private var openWindow

    private var appLanguage: String { L10n.currentLanguage }

    private var currentLanguage: Language {
        switch appLanguage {
        case "de": return .german
        case "zh": return .chinese
        case "fr": return .french
        case "it": return .italian
        case "ru": return .russian
        case "es": return .spanish
        case "pt-BR": return .portugueseBrazil
        default: return .english
        }
    }

    enum CompareMode: String, CaseIterable, Identifiable {
        case polarOverlay = "Polar Overlay"
        case cartesianOverlay = "Cartesian Overlay"
        case heatmapSideBySide = "Heatmap"
        case butterflySideBySide = "Butterfly"
        case coneSideBySide = "Cone"
        case isoluxSideBySide = "Isolux"
        case isocandelaSideBySide = "Isocandela"
        case floodlightSideBySide = "Floodlight"
        case bugSideBySide = "BUG"
        case lcsSideBySide = "LCS"

        var id: String { rawValue }
    }

    var body: some View {
        if ldtB == nil {
            emptyState
                .fileImporter(isPresented: $isImportingB, allowedContentTypes: [.ldt, .ies, .xml], allowsMultipleSelection: false, onCompletion: handleFileBImport)
        } else {
            compareContent
                .fileImporter(isPresented: $isImportingB, allowedContentTypes: [.ldt, .ies, .xml], allowsMultipleSelection: false, onCompletion: handleFileBImport)
        }
    }

    // MARK: - Empty State

    private var emptyState: some View {
        VStack(spacing: 20) {
            Image(systemName: "arrow.left.arrow.right")
                .font(.system(size: 60))
                .foregroundStyle(.tertiary)

            Text(L10n.string("compare.emptyTitle", language: appLanguage))
                .font(.title3)
                .foregroundStyle(.secondary)

            Text(L10n.string("compare.emptyHint", language: appLanguage))
                .font(.caption)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)

            HStack(spacing: 16) {
                Button {
                    isImportingB = true
                } label: {
                    Label(L10n.string("compare.browse", language: appLanguage), systemImage: "folder")
                }
                .buttonStyle(.borderedProminent)

                Menu {
                    ForEach(LuminaireTemplate.allCases) { template in
                        Button(template.displayName) {
                            if let newLdt = template.createEulumdat() {
                                ldtB = newLdt
                                fileBName = template.displayName
                            }
                        }
                    }
                } label: {
                    Label(L10n.string("compare.selectTemplate", language: appLanguage), systemImage: "doc.badge.plus")
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.windowBackground)
    }

    // MARK: - Compare Content

    private var compareContent: some View {
        VStack(spacing: 0) {
            // Header with File B info and mode picker
            HStack(spacing: 12) {
                Label(fileBName, systemImage: "doc.fill")
                    .font(.subheadline)
                    .lineLimit(1)

                Spacer()

                Button {
                    ldtB = nil
                    fileBName = ""
                } label: {
                    Label(L10n.string("compare.clear", language: appLanguage), systemImage: "xmark.circle")
                        .font(.caption)
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
            .background(Color.controlBackground)

            Divider()

            // Mode picker
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 4) {
                    ForEach(CompareMode.allCases) { mode in
                        Button {
                            selectedMode = mode
                        } label: {
                            Text(mode.rawValue)
                                .font(.system(size: 11, weight: selectedMode == mode ? .semibold : .regular))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 5)
                                .background(
                                    RoundedRectangle(cornerRadius: 5)
                                        .fill(selectedMode == mode ? Color.accentColor : Color.secondary.opacity(0.1))
                                )
                                .foregroundColor(selectedMode == mode ? .white : .primary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 6)
            }
            .background(Color.controlBackground)

            // C-plane sliders for overlay modes
            if selectedMode == .polarOverlay || selectedMode == .cartesianOverlay {
                cPlaneControls
            }

            Divider()

            // Diagram + Metrics
            GeometryReader { geometry in
                let size = min(geometry.size.width - 32, geometry.size.height - 180)
                ScrollView {
                    VStack(spacing: 16) {
                        diagramView(size: max(size, 200))
                            .padding(.top, 8)
                            #if os(macOS)
                            .onTapGesture(count: 2) {
                                openCompareWindow()
                            }
                            #else
                            .onTapGesture(count: 2) {
                                isFullscreen = true
                            }
                            #endif
                        metricsTable
                    }
                    .padding(.horizontal)
                }
            }
        }
        #if os(iOS)
        .fullScreenCover(isPresented: $isFullscreen) {
            if let b = ldtB {
                CompareFullscreenView(
                    ldtA: ldt,
                    ldtB: b,
                    selectedMode: $selectedMode,
                    isDarkTheme: $isDarkTheme,
                    isPresented: $isFullscreen,
                    fileBName: fileBName,
                    cPlaneA: $cPlaneA,
                    cPlaneB: $cPlaneB,
                    linkSliders: $linkSliders
                )
            }
        }
        #endif
    }

    // MARK: - Open Compare Window (macOS)

    private func openCompareWindow() {
        #if os(macOS)
        let model = CompareWindowModel.shared
        model.ldtA = ldt
        model.ldtB = ldtB
        model.selectedMode = selectedMode
        model.isDarkTheme = isDarkTheme
        model.fileBName = fileBName
        model.cPlaneA = cPlaneA
        model.cPlaneB = cPlaneB
        model.linkSliders = linkSliders
        openWindow(id: "compare-viewer")
        #endif
    }

    // MARK: - C-Plane Controls

    private var cPlaneControls: some View {
        HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 2) {
                Text(L10n.string("compare.cPlaneA", language: appLanguage))
                    .font(.caption2).foregroundStyle(.secondary)
                Slider(value: $cPlaneA, in: 0...360, step: 5) {
                    Text("\(Int(cPlaneA))°")
                }
                .frame(maxWidth: 200)
                .onChange(of: cPlaneA) { _, newVal in
                    if linkSliders { cPlaneB = newVal }
                }
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(L10n.string("compare.cPlaneB", language: appLanguage))
                    .font(.caption2).foregroundStyle(.secondary)
                Slider(value: $cPlaneB, in: 0...360, step: 5) {
                    Text("\(Int(cPlaneB))°")
                }
                .frame(maxWidth: 200)
            }

            Toggle(L10n.string("compare.linkSliders", language: appLanguage), isOn: $linkSliders)
                .font(.caption)
                .toggleStyle(.switch)
                .labelsHidden()

            Image(systemName: linkSliders ? "link" : "link.badge.plus")
                .font(.caption)
                .foregroundStyle(.secondary)

            Spacer()
        }
        .padding(.horizontal)
        .padding(.vertical, 6)
        .background(Color.controlBackground)
    }

    // MARK: - Diagram View

    @ViewBuilder
    private func diagramView(size: CGFloat) -> some View {
        let theme: SvgThemeType = isDarkTheme ? .dark : .light
        guard let b = ldtB else { return AnyView(EmptyView()) }

        switch selectedMode {
        case .polarOverlay:
            let svg = generatePolarOverlaySvg(
                ldtA: ldt, ldtB: b, width: Double(size), height: Double(size),
                theme: theme, labelA: "A", labelB: fileBName,
                cPlaneA: cPlaneA > 0 ? cPlaneA : nil, cPlaneB: cPlaneB > 0 ? cPlaneB : nil
            )
            return AnyView(
                SVGView(svgString: svg)
                    .frame(width: size, height: size)
                    .background(isDarkTheme ? Color.black : Color.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .shadow(radius: 2)
            )
        case .cartesianOverlay:
            let svg = generateCartesianOverlaySvg(
                ldtA: ldt, ldtB: b, width: Double(size), height: Double(size * 0.75),
                theme: theme, labelA: "A", labelB: fileBName,
                cPlaneA: cPlaneA > 0 ? cPlaneA : nil, cPlaneB: cPlaneB > 0 ? cPlaneB : nil
            )
            return AnyView(
                SVGView(svgString: svg)
                    .frame(width: size, height: size * 0.75)
                    .background(isDarkTheme ? Color.black : Color.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                    .shadow(radius: 2)
            )
        default:
            // Side-by-side diagrams
            return AnyView(sideBySideDiagrams(size: size, theme: theme, ldtB: b))
        }
    }

    private func sideBySideDiagrams(size: CGFloat, theme: SvgThemeType, ldtB: Eulumdat) -> some View {
        let halfSize = (size - 16) / 2
        let lang = currentLanguage
        return HStack(spacing: 16) {
            svgDiagram(ldt: ldt, size: halfSize, theme: theme, lang: lang)
            svgDiagram(ldt: ldtB, size: halfSize, theme: theme, lang: lang)
        }
    }

    private func svgDiagram(ldt: Eulumdat, size: CGFloat, theme: SvgThemeType, lang: Language) -> some View {
        let s = Double(size)
        let svg: String
        let h: CGFloat

        switch selectedMode {
        case .heatmapSideBySide:
            svg = generateHeatmapSvgLocalized(ldt: ldt, width: s, height: s * 0.7, theme: theme, language: lang)
            h = size * 0.7
        case .butterflySideBySide:
            svg = generateButterflySvgLocalized(ldt: ldt, width: s, height: s * 0.8, tiltDegrees: 60, theme: theme, language: lang)
            h = size * 0.8
        case .coneSideBySide:
            svg = generateConeSvgLocalized(ldt: ldt, width: s, height: s * 0.8, mountingHeight: mountingHeight, theme: theme, language: lang)
            h = size * 0.8
        case .isoluxSideBySide:
            svg = generateIsoluxSvgLocalized(ldt: ldt, width: s, height: s, mountingHeight: mountingHeight, tiltAngle: tiltAngle, areaSize: areaSize, theme: theme, language: lang)
            h = size
        case .isocandelaSideBySide:
            svg = generateIsocandelaSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .floodlightSideBySide:
            svg = generateFloodlightCartesianSvgLocalized(ldt: ldt, width: s, height: s * 0.75, logScale: false, theme: theme, language: lang)
            h = size * 0.75
        case .bugSideBySide:
            svg = generateBugSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .lcsSideBySide:
            svg = generateLcsSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        default:
            svg = generatePolarSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        }

        return SVGView(svgString: svg)
            .frame(width: size, height: h)
            .background(isDarkTheme ? Color.black : Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .shadow(radius: 1)
    }

    // MARK: - Metrics Table

    private var metricsTable: some View {
        Group {
            if let b = ldtB {
                let result = comparePhotometricLocalized(
                    ldtA: ldt, ldtB: b,
                    labelA: "A", labelB: fileBName,
                    language: currentLanguage
                )

                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text(L10n.string("compare.similarity", language: appLanguage))
                            .font(.subheadline.bold())
                        Text("\(result.similarityScore * 100, specifier: "%.1f")%")
                            .font(.subheadline.monospacedDigit())
                            .foregroundColor(similarityColor(result.similarityScore))
                    }

                    // Metrics grid
                    LazyVGrid(columns: [
                        GridItem(.flexible(minimum: 120)),
                        GridItem(.fixed(80), alignment: .trailing),
                        GridItem(.fixed(80), alignment: .trailing),
                        GridItem(.fixed(70), alignment: .trailing),
                        GridItem(.fixed(60), alignment: .trailing),
                    ], spacing: 4) {
                        // Header
                        Text(L10n.string("compare.metric", language: appLanguage)).font(.caption.bold())
                        Text("A").font(.caption.bold())
                        Text("B").font(.caption.bold())
                        Text(L10n.string("compare.delta", language: appLanguage)).font(.caption.bold())
                        Text("%").font(.caption.bold())

                        ForEach(Array(result.metrics.enumerated()), id: \.offset) { _, metric in
                            Text(metric.name)
                                .font(.caption)
                                .lineLimit(1)
                            Text(formatValue(metric.valueA, unit: metric.unit))
                                .font(.caption.monospacedDigit())
                            Text(formatValue(metric.valueB, unit: metric.unit))
                                .font(.caption.monospacedDigit())
                            Text(formatValue(metric.delta, unit: metric.unit))
                                .font(.caption.monospacedDigit())
                                .foregroundColor(significanceColor(metric.significance))
                            Text("\(metric.deltaPercent, specifier: "%.1f")%")
                                .font(.caption.monospacedDigit())
                                .foregroundColor(significanceColor(metric.significance))
                        }
                    }
                    .padding()
                    .background(Color.controlBackground)
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .padding(.bottom)
            }
        }
    }

    // MARK: - Helpers

    private func similarityColor(_ score: Double) -> SwiftUI.Color {
        if score >= 0.9 { return .green }
        if score >= 0.7 { return .orange }
        return .red
    }

    private func significanceColor(_ significance: SignificanceLevel) -> SwiftUI.Color {
        switch significance {
        case .negligible: return .secondary
        case .minor: return .primary
        case .moderate: return .orange
        case .major: return .red
        }
    }

    private func formatValue(_ value: Double, unit: String) -> String {
        if abs(value) >= 1000 {
            return String(format: "%.0f", value)
        } else if abs(value) >= 10 {
            return String(format: "%.1f", value)
        } else {
            return String(format: "%.2f", value)
        }
    }

    // MARK: - File Handling

    private func handleFileBImport(_ result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            guard let url = urls.first else { return }
            guard url.startAccessingSecurityScopedResource() else { return }
            defer { url.stopAccessingSecurityScopedResource() }

            fileBName = url.lastPathComponent
            let ext = url.pathExtension.lowercased()

            do {
                let content: String
                if let isoContent = try? String(contentsOf: url, encoding: .isoLatin1) {
                    content = isoContent
                } else {
                    content = try String(contentsOf: url, encoding: .utf8)
                }

                switch ext {
                case "ies":
                    ldtB = try parseIes(content: content)
                case "xml":
                    let doc = try AtlaDocument.parseXml(content: content)
                    ldtB = try parseLdt(content: doc.toLdt())
                default:
                    ldtB = try parseLdt(content: content)
                }
            } catch {
                ldtB = nil
                fileBName = ""
            }
        case .failure:
            break
        }
    }
}

// MARK: - Compare Window View (macOS)

#if os(macOS)
struct CompareWindowView: View {
    @ObservedObject var model = CompareWindowModel.shared
    @State private var zoomScale: CGFloat = 1.0
    @AppStorage("mountingHeight") private var mountingHeight = 3.0
    @AppStorage("tiltAngle") private var tiltAngle = 0.0
    @AppStorage("areaSize") private var areaSize = 20.0

    private var appLanguage: String { L10n.currentLanguage }

    private var currentLanguage: Language {
        switch appLanguage {
        case "de": return .german
        case "zh": return .chinese
        case "fr": return .french
        case "it": return .italian
        case "ru": return .russian
        case "es": return .spanish
        case "pt-BR": return .portugueseBrazil
        default: return .english
        }
    }

    var body: some View {
        if let a = model.ldtA, let b = model.ldtB {
            VStack(spacing: 0) {
                // C-plane controls for overlay modes
                if model.selectedMode == .polarOverlay || model.selectedMode == .cartesianOverlay {
                    HStack(spacing: 16) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(L10n.string("compare.cPlaneA", language: appLanguage))
                                .font(.caption2).foregroundStyle(.secondary)
                            Slider(value: $model.cPlaneA, in: 0...360, step: 5) {
                                Text("\(Int(model.cPlaneA))°")
                            }
                            .frame(maxWidth: 200)
                            .onChange(of: model.cPlaneA) { _, newVal in
                                if model.linkSliders { model.cPlaneB = newVal }
                            }
                        }

                        VStack(alignment: .leading, spacing: 2) {
                            Text(L10n.string("compare.cPlaneB", language: appLanguage))
                                .font(.caption2).foregroundStyle(.secondary)
                            Slider(value: $model.cPlaneB, in: 0...360, step: 5) {
                                Text("\(Int(model.cPlaneB))°")
                            }
                            .frame(maxWidth: 200)
                        }

                        Toggle(L10n.string("compare.linkSliders", language: appLanguage), isOn: $model.linkSliders)
                            .font(.caption)
                            .toggleStyle(.switch)
                            .labelsHidden()

                        Image(systemName: model.linkSliders ? "link" : "link.badge.plus")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        Spacer()
                    }
                    .padding(.horizontal)
                    .padding(.vertical, 6)
                    .background(Color.controlBackground)

                    Divider()
                }

                // Diagram
                GeometryReader { geometry in
                    let availableWidth = geometry.size.width - 80
                    let availableHeight = geometry.size.height - 80
                    let ratio = aspectRatio(for: model.selectedMode)
                    let baseSize = min(availableWidth, availableHeight / ratio)

                    ZStack {
                        (model.isDarkTheme ? Color.black : Color.white)
                            .ignoresSafeArea()

                        ZoomableScrollContainer(zoomScale: $zoomScale, onDoubleClick: {}) {
                            compareDiagramContent(ldtA: a, ldtB: b, size: baseSize * zoomScale)
                        }

                        // Zoom info
                        VStack {
                            HStack {
                                Spacer()
                                VStack(alignment: .trailing, spacing: 4) {
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

                        // Zoom controls
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
            }
            .navigationTitle("\(model.selectedMode.rawValue) - A vs \(model.fileBName)")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Toggle(isOn: $model.isDarkTheme) {
                        Label(L10n.string("fullscreen.darkTheme", language: appLanguage), systemImage: model.isDarkTheme ? "moon.fill" : "sun.max.fill")
                    }
                }
            }
        } else {
            VStack(spacing: 16) {
                Image(systemName: "arrow.left.arrow.right")
                    .font(.system(size: 80))
                    .foregroundStyle(.tertiary)
                Text("No comparison loaded")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
            .frame(minWidth: 800, minHeight: 600)
        }
    }

    @ViewBuilder
    private func compareDiagramContent(ldtA: Eulumdat, ldtB: Eulumdat, size: CGFloat) -> some View {
        let theme: SvgThemeType = model.isDarkTheme ? .dark : .light
        let lang = currentLanguage

        switch model.selectedMode {
        case .polarOverlay:
            let svg = generatePolarOverlaySvg(
                ldtA: ldtA, ldtB: ldtB, width: Double(size), height: Double(size),
                theme: theme, labelA: "A", labelB: model.fileBName,
                cPlaneA: model.cPlaneA > 0 ? model.cPlaneA : nil,
                cPlaneB: model.cPlaneB > 0 ? model.cPlaneB : nil
            )
            SVGView(svgString: svg)
                .frame(width: size, height: size)
                .background(model.isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .shadow(radius: 4)

        case .cartesianOverlay:
            let svg = generateCartesianOverlaySvg(
                ldtA: ldtA, ldtB: ldtB, width: Double(size), height: Double(size * 0.75),
                theme: theme, labelA: "A", labelB: model.fileBName,
                cPlaneA: model.cPlaneA > 0 ? model.cPlaneA : nil,
                cPlaneB: model.cPlaneB > 0 ? model.cPlaneB : nil
            )
            SVGView(svgString: svg)
                .frame(width: size, height: size * 0.75)
                .background(model.isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .shadow(radius: 4)

        default:
            // Side-by-side at full window size
            let halfSize = (size - 24) / 2
            HStack(spacing: 24) {
                svgWindowDiagram(ldt: ldtA, size: halfSize, theme: theme, lang: lang)
                svgWindowDiagram(ldt: ldtB, size: halfSize, theme: theme, lang: lang)
            }
        }
    }

    private func svgWindowDiagram(ldt: Eulumdat, size: CGFloat, theme: SvgThemeType, lang: Language) -> some View {
        let s = Double(size)
        let svg: String
        let h: CGFloat

        switch model.selectedMode {
        case .heatmapSideBySide:
            svg = generateHeatmapSvgLocalized(ldt: ldt, width: s, height: s * 0.7, theme: theme, language: lang)
            h = size * 0.7
        case .butterflySideBySide:
            svg = generateButterflySvgLocalized(ldt: ldt, width: s, height: s * 0.8, tiltDegrees: 60, theme: theme, language: lang)
            h = size * 0.8
        case .coneSideBySide:
            svg = generateConeSvgLocalized(ldt: ldt, width: s, height: s * 0.8, mountingHeight: mountingHeight, theme: theme, language: lang)
            h = size * 0.8
        case .isoluxSideBySide:
            svg = generateIsoluxSvgLocalized(ldt: ldt, width: s, height: s, mountingHeight: mountingHeight, tiltAngle: tiltAngle, areaSize: areaSize, theme: theme, language: lang)
            h = size
        case .isocandelaSideBySide:
            svg = generateIsocandelaSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .floodlightSideBySide:
            svg = generateFloodlightCartesianSvgLocalized(ldt: ldt, width: s, height: s * 0.75, logScale: false, theme: theme, language: lang)
            h = size * 0.75
        case .bugSideBySide:
            svg = generateBugSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .lcsSideBySide:
            svg = generateLcsSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        default:
            svg = generatePolarSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        }

        return SVGView(svgString: svg)
            .frame(width: size, height: h)
            .background(model.isDarkTheme ? Color.black : Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(radius: 2)
    }

    private func aspectRatio(for mode: CompareView.CompareMode) -> Double {
        switch mode {
        case .polarOverlay, .isoluxSideBySide, .lcsSideBySide: return 1.0
        case .bugSideBySide, .isocandelaSideBySide: return 0.85
        case .butterflySideBySide, .coneSideBySide: return 0.8
        case .cartesianOverlay, .floodlightSideBySide: return 0.75
        case .heatmapSideBySide: return 0.7
        }
    }
}
#endif

// MARK: - Compare Fullscreen View (iOS)

#if os(iOS)
struct CompareFullscreenView: View {
    let ldtA: Eulumdat
    let ldtB: Eulumdat
    @Binding var selectedMode: CompareView.CompareMode
    @Binding var isDarkTheme: Bool
    @Binding var isPresented: Bool
    let fileBName: String
    @Binding var cPlaneA: Double
    @Binding var cPlaneB: Double
    @Binding var linkSliders: Bool
    @AppStorage("mountingHeight") private var mountingHeight = 3.0
    @AppStorage("tiltAngle") private var tiltAngle = 0.0
    @AppStorage("areaSize") private var areaSize = 20.0

    private var appLanguage: String { L10n.currentLanguage }

    private var currentLanguage: Language {
        switch appLanguage {
        case "de": return .german
        case "zh": return .chinese
        case "fr": return .french
        case "it": return .italian
        case "ru": return .russian
        case "es": return .spanish
        case "pt-BR": return .portugueseBrazil
        default: return .english
        }
    }

    var body: some View {
        NavigationStack {
            GeometryReader { geometry in
                let size = min(geometry.size.width, geometry.size.height) - 40

                ScrollView([.horizontal, .vertical], showsIndicators: true) {
                    VStack {
                        Spacer(minLength: 0)
                        HStack {
                            Spacer(minLength: 0)
                            compareDiagramContent(size: size)
                            Spacer(minLength: 0)
                        }
                        Spacer(minLength: 0)
                    }
                    .frame(minWidth: geometry.size.width, minHeight: geometry.size.height)
                }
            }
            .background(isDarkTheme ? Color.black : Color.white)
            .navigationTitle("\(selectedMode.rawValue)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.string("fullscreen.done", language: appLanguage)) {
                        isPresented = false
                    }
                }
                if selectedMode == .polarOverlay || selectedMode == .cartesianOverlay {
                    ToolbarItem(placement: .principal) {
                        HStack(spacing: 8) {
                            Text("C°")
                                .font(.caption)
                            Slider(value: $cPlaneA, in: 0...360, step: 5)
                                .frame(width: 100)
                                .onChange(of: cPlaneA) { _, newVal in
                                    if linkSliders { cPlaneB = newVal }
                                }
                            Text(String(format: "%.0f°", cPlaneA))
                                .font(.caption.monospacedDigit())
                        }
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

    @ViewBuilder
    private func compareDiagramContent(size: CGFloat) -> some View {
        let theme: SvgThemeType = isDarkTheme ? .dark : .light
        let lang = currentLanguage

        switch selectedMode {
        case .polarOverlay:
            let svg = generatePolarOverlaySvg(
                ldtA: ldtA, ldtB: ldtB, width: Double(size), height: Double(size),
                theme: theme, labelA: "A", labelB: fileBName,
                cPlaneA: cPlaneA > 0 ? cPlaneA : nil,
                cPlaneB: cPlaneB > 0 ? cPlaneB : nil
            )
            SVGView(svgString: svg)
                .frame(width: size, height: size)
                .background(isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))

        case .cartesianOverlay:
            let svg = generateCartesianOverlaySvg(
                ldtA: ldtA, ldtB: ldtB, width: Double(size), height: Double(size * 0.75),
                theme: theme, labelA: "A", labelB: fileBName,
                cPlaneA: cPlaneA > 0 ? cPlaneA : nil,
                cPlaneB: cPlaneB > 0 ? cPlaneB : nil
            )
            SVGView(svgString: svg)
                .frame(width: size, height: size * 0.75)
                .background(isDarkTheme ? Color.black : Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))

        default:
            let halfSize = (size - 16) / 2
            HStack(spacing: 16) {
                svgFullscreenDiagram(ldt: ldtA, size: halfSize, theme: theme, lang: lang)
                svgFullscreenDiagram(ldt: ldtB, size: halfSize, theme: theme, lang: lang)
            }
        }
    }

    private func svgFullscreenDiagram(ldt: Eulumdat, size: CGFloat, theme: SvgThemeType, lang: Language) -> some View {
        let s = Double(size)
        let svg: String
        let h: CGFloat

        switch selectedMode {
        case .heatmapSideBySide:
            svg = generateHeatmapSvgLocalized(ldt: ldt, width: s, height: s * 0.7, theme: theme, language: lang)
            h = size * 0.7
        case .butterflySideBySide:
            svg = generateButterflySvgLocalized(ldt: ldt, width: s, height: s * 0.8, tiltDegrees: 60, theme: theme, language: lang)
            h = size * 0.8
        case .coneSideBySide:
            svg = generateConeSvgLocalized(ldt: ldt, width: s, height: s * 0.8, mountingHeight: mountingHeight, theme: theme, language: lang)
            h = size * 0.8
        case .isoluxSideBySide:
            svg = generateIsoluxSvgLocalized(ldt: ldt, width: s, height: s, mountingHeight: mountingHeight, tiltAngle: tiltAngle, areaSize: areaSize, theme: theme, language: lang)
            h = size
        case .isocandelaSideBySide:
            svg = generateIsocandelaSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .floodlightSideBySide:
            svg = generateFloodlightCartesianSvgLocalized(ldt: ldt, width: s, height: s * 0.75, logScale: false, theme: theme, language: lang)
            h = size * 0.75
        case .bugSideBySide:
            svg = generateBugSvgLocalized(ldt: ldt, width: s, height: s * 0.85, theme: theme, language: lang)
            h = size * 0.85
        case .lcsSideBySide:
            svg = generateLcsSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        default:
            svg = generatePolarSvgLocalized(ldt: ldt, width: s, height: s, theme: theme, language: lang)
            h = size
        }

        return SVGView(svgString: svg)
            .frame(width: size, height: h)
            .background(isDarkTheme ? Color.black : Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}
#endif
