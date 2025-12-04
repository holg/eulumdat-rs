import SwiftUI
import EulumdatKit

@main
struct EulumdatApp: App {
    @AppStorage("isDarkTheme") private var isDarkTheme = false
    @AppStorage("defaultDiagram") private var defaultDiagram = "polar"

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        #if os(macOS)
        .windowStyle(.hiddenTitleBar)
        .windowToolbarStyle(.unified(showsTitle: true))
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open LDT File...") {
                    NotificationCenter.default.post(name: .openFile, object: nil)
                }
                .keyboardShortcut("o", modifiers: .command)
            }

            CommandGroup(after: .newItem) {
                Divider()
                Button("Export SVG...") {
                    NotificationCenter.default.post(name: .exportSVG, object: nil)
                }
                .keyboardShortcut("e", modifiers: [.command, .shift])
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

                Button("Heatmap") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "heatmap")
                }
                .keyboardShortcut("4", modifiers: .command)

                Button("BUG Rating") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "bug")
                }
                .keyboardShortcut("5", modifiers: .command)

                Button("LCS Diagram") {
                    NotificationCenter.default.post(name: .selectDiagram, object: "lcs")
                }
                .keyboardShortcut("6", modifiers: .command)

                Divider()

                Toggle("Dark Theme", isOn: $isDarkTheme)
                    .keyboardShortcut("d", modifiers: [.command, .shift])
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
        Settings {
            SettingsView()
        }
        #endif
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let openFile = Notification.Name("openFile")
    static let exportSVG = Notification.Name("exportSVG")
    static let selectDiagram = Notification.Name("selectDiagram")
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
                    Text("Heatmap").tag("heatmap")
                    Text("BUG Rating").tag("bug")
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
#endif
