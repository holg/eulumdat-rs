import SwiftUI
import EulumdatKit

struct BimPanelView: View {
    let ldt: Eulumdat
    private var appLanguage: String { L10n.currentLanguage }

    var body: some View {
        let hasBim = hasBimData(ldt: ldt)

        if hasBim {
            bimContent
        } else {
            emptyState
        }
    }

    // MARK: - Empty State

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "building.2")
                .font(.system(size: 60))
                .foregroundStyle(.tertiary)

            Text(L10n.string("bim.noData", language: appLanguage))
                .font(.title3)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Text(L10n.string("bim.info", language: appLanguage))
                .font(.caption)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 400)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.windowBackground)
    }

    // MARK: - BIM Content

    private var bimContent: some View {
        let bimData = getBimParameters(ldt: ldt)

        return List {
            Section {
                HStack {
                    Image(systemName: "building.2.fill")
                        .foregroundColor(.accentColor)
                    Text(L10n.string("bim.title", language: appLanguage))
                        .font(.headline)
                    Spacer()
                    Text(bimData.summary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            // Group rows by group name
            let grouped = Dictionary(grouping: bimData.rows, by: { $0.group })
            let sortedGroups = grouped.keys.sorted()

            ForEach(sortedGroups, id: \.self) { group in
                Section(group) {
                    if let rows = grouped[group] {
                        ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                            HStack {
                                Text(row.key)
                                    .font(.subheadline)
                                Spacer()
                                HStack(spacing: 4) {
                                    Text(row.value)
                                        .font(.subheadline.monospacedDigit())
                                        .foregroundStyle(.primary)
                                    if !row.unit.isEmpty {
                                        Text(row.unit)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Export section
            Section {
                #if os(macOS)
                HStack(spacing: 12) {
                    Button {
                        exportCSV(bimData)
                    } label: {
                        Label(L10n.string("bim.exportCSV", language: appLanguage), systemImage: "tablecells")
                    }

                    Button {
                        exportReport(bimData)
                    } label: {
                        Label(L10n.string("bim.exportReport", language: appLanguage), systemImage: "doc.text")
                    }
                }
                #else
                ShareLink(item: bimData.csv, preview: SharePreview("BIM Parameters CSV")) {
                    Label(L10n.string("bim.exportCSV", language: appLanguage), systemImage: "tablecells")
                }
                ShareLink(item: bimData.textReport, preview: SharePreview("BIM Parameters Report")) {
                    Label(L10n.string("bim.exportReport", language: appLanguage), systemImage: "doc.text")
                }
                #endif
            }
        }
    }

    #if os(macOS)
    private func exportCSV(_ bimData: BimData) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.commaSeparatedText]
        panel.nameFieldStringValue = "bim_parameters.csv"
        panel.begin { response in
            if response == .OK, let url = panel.url {
                try? bimData.csv.write(to: url, atomically: true, encoding: .utf8)
            }
        }
    }

    private func exportReport(_ bimData: BimData) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.plainText]
        panel.nameFieldStringValue = "bim_parameters.txt"
        panel.begin { response in
            if response == .OK, let url = panel.url {
                try? bimData.textReport.write(to: url, atomically: true, encoding: .utf8)
            }
        }
    }
    #endif
}
