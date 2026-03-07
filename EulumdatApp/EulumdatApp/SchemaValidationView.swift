import SwiftUI
import EulumdatKit

struct SchemaValidationView: View {
    let ldt: Eulumdat
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
        List {
            // Schema validation cards
            Section(L10n.string("schema.schemaValidation", language: appLanguage)) {
                let s001 = validateSchemaS001(ldt: ldt)
                let tm33 = validateSchemaTm33(ldt: ldt)
                let tm32 = validateSchemaTm32(ldt: ldt)

                LazyVGrid(columns: [
                    GridItem(.flexible(), spacing: 12),
                    GridItem(.flexible(), spacing: 12),
                    GridItem(.flexible(), spacing: 12),
                ], spacing: 12) {
                    schemaCard(
                        name: L10n.string("schema.s001", language: appLanguage),
                        result: s001
                    )
                    schemaCard(
                        name: L10n.string("schema.tm33", language: appLanguage),
                        result: tm33
                    )
                    schemaCard(
                        name: L10n.string("schema.tm32", language: appLanguage),
                        result: tm32
                    )
                }
                .padding(.vertical, 4)

                // Expandable details per schema
                if !s001.errors.isEmpty || !s001.warnings.isEmpty {
                    schemaDetails(name: L10n.string("schema.s001", language: appLanguage), result: s001)
                }
                if !tm33.errors.isEmpty || !tm33.warnings.isEmpty {
                    schemaDetails(name: L10n.string("schema.tm33", language: appLanguage), result: tm33)
                }
                if !tm32.errors.isEmpty || !tm32.warnings.isEmpty {
                    schemaDetails(name: L10n.string("schema.tm32", language: appLanguage), result: tm32)
                }
            }

            // LDT/IES validation
            Section(L10n.string("schema.ldtValidation", language: appLanguage)) {
                let warnings = validateLdtLocalized(ldt: ldt, language: currentLanguage)
                let errors = getValidationErrorsLocalized(ldt: ldt, language: currentLanguage)

                statusRow(errors: errors, warnings: warnings)

                if !errors.isEmpty {
                    ForEach(Array(errors.enumerated()), id: \.offset) { _, error in
                        HStack(spacing: 8) {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(.red)
                                .font(.caption)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(error.code)
                                    .font(.caption.bold().monospaced())
                                Text(error.message)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                }

                if !warnings.isEmpty {
                    ForEach(Array(warnings.enumerated()), id: \.offset) { _, warning in
                        HStack(spacing: 8) {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundColor(.orange)
                                .font(.caption)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(warning.code)
                                    .font(.caption.bold().monospaced())
                                Text(warning.message)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }

            // File info section
            Section(L10n.string("schema.fileInfo", language: appLanguage)) {
                LabeledContent("Luminaire", value: ldt.luminaireName)
                LabeledContent("Manufacturer", value: ldt.identification)
                LabeledContent("Type", value: typeDescription)
                LabeledContent("Symmetry", value: symmetryDescription)
                LabeledContent("C-Planes", value: "\(ldt.numCPlanes)")
                LabeledContent("G-Angles", value: "\(ldt.numGPlanes)")
                LabeledContent("Max Intensity", value: String(format: "%.1f cd/klm", ldt.maxIntensity))
                LabeledContent("Total Flux", value: String(format: "%.0f lm", ldt.totalLuminousFlux))
            }
        }
    }

    // MARK: - Schema Card

    private func schemaCard(name: String, result: SchemaValidationResult) -> some View {
        VStack(spacing: 6) {
            Text(name)
                .font(.caption.bold())
                .lineLimit(1)

            Image(systemName: result.isValid ? "checkmark.circle.fill" : (result.errors.isEmpty ? "exclamationmark.triangle.fill" : "xmark.circle.fill"))
                .font(.title2)
                .foregroundColor(cardColor(result))

            Text(result.isValid
                ? L10n.string("schema.valid", language: appLanguage)
                : (result.errors.isEmpty
                    ? "\(result.warnings.count) \(L10n.string("schema.warnings", language: appLanguage))"
                    : L10n.string("schema.invalid", language: appLanguage)))
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .background(
            RoundedRectangle(cornerRadius: 10)
                .fill(cardColor(result).opacity(0.1))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(cardColor(result).opacity(0.3), lineWidth: 1)
        )
    }

    private func cardColor(_ result: SchemaValidationResult) -> SwiftUI.Color {
        if result.isValid && result.warnings.isEmpty { return .green }
        if result.isValid { return .orange }
        return .red
    }

    // MARK: - Schema Details

    private func schemaDetails(name: String, result: SchemaValidationResult) -> some View {
        DisclosureGroup("\(name) Details") {
            if !result.errors.isEmpty {
                ForEach(Array(result.errors.enumerated()), id: \.offset) { _, msg in
                    HStack(spacing: 8) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.red)
                            .font(.caption)
                        VStack(alignment: .leading) {
                            Text(msg.code).font(.caption.bold().monospaced())
                            Text(msg.message).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            if !result.warnings.isEmpty {
                ForEach(Array(result.warnings.enumerated()), id: \.offset) { _, msg in
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.orange)
                            .font(.caption)
                        VStack(alignment: .leading) {
                            Text(msg.code).font(.caption.bold().monospaced())
                            Text(msg.message).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
    }

    // MARK: - Status Row

    private func statusRow(errors: [ValidationError], warnings: [ValidationWarning]) -> some View {
        HStack(spacing: 8) {
            Image(systemName: errors.isEmpty && warnings.isEmpty
                ? "checkmark.circle.fill"
                : (errors.isEmpty ? "exclamationmark.triangle.fill" : "xmark.circle.fill"))
                .foregroundColor(errors.isEmpty && warnings.isEmpty ? .green : (errors.isEmpty ? .orange : .red))

            if errors.isEmpty && warnings.isEmpty {
                Text("All validation checks passed")
                    .font(.subheadline)
                    .foregroundColor(.green)
            } else {
                Text("\(errors.count) error(s), \(warnings.count) warning(s)")
                    .font(.subheadline)
                    .foregroundColor(errors.isEmpty ? .orange : .red)
            }
        }
    }

    // MARK: - Descriptions

    private var typeDescription: String {
        switch ldt.typeIndicator {
        case .pointSourceSymmetric: return "Point Source (Symmetric)"
        case .linear: return "Linear"
        case .pointSourceOther: return "Point Source (Other)"
        }
    }

    private var symmetryDescription: String {
        switch ldt.symmetry {
        case .none: return "No symmetry"
        case .verticalAxis: return "Vertical Axis"
        case .planeC0c180: return "Plane C0-C180"
        case .planeC90c270: return "Plane C90-C270"
        case .bothPlanes: return "Both Planes"
        }
    }
}
