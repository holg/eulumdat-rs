import SwiftUI
import EulumdatKit

@main
struct EulumdatApp: App {
    @AppStorage("isDarkTheme") private var isDarkTheme = false
    @AppStorage("defaultDiagram") private var defaultDiagram = "polar"

    #if os(macOS)
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    #endif

    var body: some Scene {
        // Main document window - supports multiple instances
        WindowGroup("Eulumdat Editor", id: "editor") {
            ContentView()
        }
        #if os(macOS)
        .windowToolbarStyle(.unified(showsTitle: true))
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Window") {
                    NSApp.sendAction(#selector(NSApplication.newWindowForTab(_:)), to: nil, from: nil)
                }
                .keyboardShortcut("n", modifiers: .command)

                Menu("New from Template") {
                    ForEach(LuminaireTemplate.allCases) { template in
                        Button {
                            NotificationCenter.default.post(name: .newFromTemplate, object: template)
                        } label: {
                            Label(template.rawValue, systemImage: template.icon)
                        }
                    }
                }

                Divider()

                Button("Open LDT/IES File...") {
                    NotificationCenter.default.post(name: .openFile, object: nil)
                }
                .keyboardShortcut("o", modifiers: .command)
            }

            CommandGroup(after: .newItem) {
                Divider()

                Button("Batch Convert...") {
                    NotificationCenter.default.post(name: .openBatchConvert, object: nil)
                }
                .keyboardShortcut("b", modifiers: [.command, .shift])

                Divider()

                Button("Export SVG...") {
                    NotificationCenter.default.post(name: .exportSVG, object: nil)
                }
                .keyboardShortcut("e", modifiers: [.command, .shift])

                Button("Export IES...") {
                    NotificationCenter.default.post(name: .exportIES, object: nil)
                }
                .keyboardShortcut("i", modifiers: [.command, .shift])

                Button("Export LDT...") {
                    NotificationCenter.default.post(name: .exportLDT, object: nil)
                }
                .keyboardShortcut("l", modifiers: [.command, .shift])

                Divider()

                Menu("Export Watch Face") {
                    Button("Dark Style (45mm)") {
                        NotificationCenter.default.post(name: .exportWatchFace, object: WatchFaceExportConfig(style: .dark, size: .watch45mm))
                    }
                    Button("Dark Style (41mm)") {
                        NotificationCenter.default.post(name: .exportWatchFace, object: WatchFaceExportConfig(style: .dark, size: .watch41mm))
                    }
                    Divider()
                    Button("Light Style (45mm)") {
                        NotificationCenter.default.post(name: .exportWatchFace, object: WatchFaceExportConfig(style: .light, size: .watch45mm))
                    }
                    Button("California Style (45mm)") {
                        NotificationCenter.default.post(name: .exportWatchFace, object: WatchFaceExportConfig(style: .california, size: .watch45mm))
                    }
                    Divider()
                    Button("Complication (120×120)") {
                        NotificationCenter.default.post(name: .exportWatchFace, object: WatchFaceExportConfig(style: .complication, size: .complication))
                    }
                }
            }

            CommandMenu("View") {
                Button("Polar Diagram") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "polar")
                }
                .keyboardShortcut("1", modifiers: .command)

                Button("Cartesian Diagram") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "cartesian")
                }
                .keyboardShortcut("2", modifiers: .command)

                Button("Butterfly Diagram") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "butterfly")
                }
                .keyboardShortcut("3", modifiers: .command)

                Button("3D Butterfly") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "3d butterfly")
                }
                .keyboardShortcut("4", modifiers: .command)

                Button("Heatmap") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "heatmap")
                }
                .keyboardShortcut("5", modifiers: .command)

                Button("BUG Rating") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "bug rating")
                }
                .keyboardShortcut("6", modifiers: .command)

                Button("LCS Diagram") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "lcs")
                }
                .keyboardShortcut("7", modifiers: .command)

                Divider()

                Toggle("Dark Theme", isOn: $isDarkTheme)
                    .keyboardShortcut("d", modifiers: [.command, .shift])
            }

            // Window menu commands
            CommandGroup(after: .windowSize) {
                Button("App Store Size (1440×900)") {
                    NotificationCenter.default.post(name: .setAppStoreSize, object: nil)
                }
                .keyboardShortcut("s", modifiers: [.command, .control, .shift])
            }

            CommandGroup(replacing: .help) {
                Button("Eulumdat Format Reference") {
                    if let url = URL(string: "https://paulbourke.net/dataformats/ldt/") {
                        NSWorkspace.shared.open(url)
                    }
                }

                Button("IES TM-15-11 BUG Rating") {
                    if let url = URL(string: "https://www.ies.org/definitions/bug-rating/") {
                        NSWorkspace.shared.open(url)
                    }
                }

                Divider()

                Button("GitHub Repository") {
                    if let url = URL(string: "https://github.com/holg/eulumdat-rs") {
                        NSWorkspace.shared.open(url)
                    }
                }
            }
        }
        #endif

        #if os(macOS)
        // Batch conversion window
        Window("Batch Convert", id: "batch-convert") {
            BatchConvertView()
        }
        .windowStyle(.titleBar)
        .windowResizability(.contentSize)
        .defaultPosition(.center)

        // 3D Viewer window
        Window("3D Photometric Viewer", id: "3d-viewer") {
            Butterfly3DWindowView()
        }
        .windowStyle(.titleBar)
        .defaultSize(width: 1000, height: 800)
        .windowResizability(.contentSize)

        // Diagram Viewer window (for fullscreen diagrams)
        Window("Diagram Viewer", id: "diagram-viewer") {
            DiagramWindowView()
        }
        .windowStyle(.titleBar)
        .defaultSize(width: 1200, height: 900)
        .windowResizability(.contentSize)

        Settings {
            SettingsView()
        }
        #endif
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let openFile = Notification.Name("openFile")
    static let openExternalFile = Notification.Name("openExternalFile")
    static let exportSVG = Notification.Name("exportSVG")
    static let exportIES = Notification.Name("exportIES")
    static let exportLDT = Notification.Name("exportLDT")
    static let exportWatchFace = Notification.Name("exportWatchFace")
    static let selectDiagram = Notification.Name("selectDiagram")
    static let newFromTemplate = Notification.Name("newFromTemplate")
    static let openBatchConvert = Notification.Name("openBatchConvert")
    static let open3DViewer = Notification.Name("open3DViewer")
    static let setAppStoreSize = Notification.Name("setAppStoreSize")
}

// MARK: - Watch Face Export Configuration

enum WatchFaceSize {
    case watch45mm  // 396×484
    case watch41mm  // 368×448
    case complication  // 120×120

    var width: UInt32 {
        switch self {
        case .watch45mm: return 396
        case .watch41mm: return 368
        case .complication: return 120
        }
    }

    var height: UInt32 {
        switch self {
        case .watch45mm: return 484
        case .watch41mm: return 448
        case .complication: return 120
        }
    }

    var filename: String {
        switch self {
        case .watch45mm: return "watchface_45mm"
        case .watch41mm: return "watchface_41mm"
        case .complication: return "complication"
        }
    }
}

struct WatchFaceExportConfig {
    let style: WatchFaceStyleType
    let size: WatchFaceSize
}

// MARK: - Shared 3D Viewer Data

class Viewer3DModel: ObservableObject {
    static let shared = Viewer3DModel()
    @Published var currentLDT: Eulumdat?

    private init() {}
}

// MARK: - Shared Diagram Viewer Data

class DiagramWindowModel: ObservableObject {
    static let shared = DiagramWindowModel()
    @Published var ldt: Eulumdat?
    @Published var selectedDiagram: ContentView.DiagramType = .polar
    @Published var isDarkTheme: Bool = false

    private init() {}
}

// MARK: - Settings View (macOS)

#if os(macOS)
struct SettingsView: View {
    @AppStorage("isDarkTheme") private var isDarkTheme = false
    @AppStorage("defaultDiagram") private var defaultDiagram = "polar"
    @AppStorage("svgExportSize") private var svgExportSize = 600.0

    var body: some View {
        Form {
            Section("Appearance") {
                Toggle("Use Dark Theme for Diagrams", isOn: $isDarkTheme)
            }

            Section("Default Diagram") {
                Picker("Default diagram type", selection: $defaultDiagram) {
                    Text("Polar").tag("polar")
                    Text("Cartesian").tag("cartesian")
                    Text("Butterfly").tag("butterfly")
                    Text("3D Butterfly").tag("3d butterfly")
                    Text("Heatmap").tag("heatmap")
                    Text("BUG Rating").tag("bug rating")
                    Text("LCS").tag("lcs")
                }
            }

            Section("Export") {
                Slider(value: $svgExportSize, in: 400...1200, step: 100) {
                    Text("SVG Export Size: \(Int(svgExportSize))px")
                }
            }

            Section("About") {
                LabeledContent("Version", value: "0.2.0")
                LabeledContent("Library", value: "eulumdat-rs")

                Link("View on GitHub", destination: URL(string: "https://github.com/holg/eulumdat-rs")!)
            }
        }
        .formStyle(.grouped)
        .frame(width: 400, height: 350)
    }
}

// MARK: - App Delegate (macOS)

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        // Standard macOS behavior: quit when last window closes
        return true
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        // Handle files opened via Finder, open command, or drag-and-drop onto dock icon
        for url in urls {
            let ext = url.pathExtension.lowercased()
            if ext == "ldt" || ext == "ies" {
                NotificationCenter.default.post(name: .openExternalFile, object: url)
                break // Only open the first valid file
            }
        }
    }
}
#endif
