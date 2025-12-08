//
//  ValidationView.swift
//  EulumdatApp
//
//  Validation tab showing warnings and errors from core validation
//

import SwiftUI
import EulumdatKit

struct ValidationView: View {
    let ldt: Eulumdat
    @State private var warnings: [ValidationWarning] = []
    @State private var errors: [ValidationError] = []

    var body: some View {
        VStack(spacing: 0) {
            // Summary header
            HStack {
                Image(systemName: statusIcon)
                    .font(.title2)
                    .foregroundStyle(statusColor)

                VStack(alignment: .leading, spacing: 4) {
                    Text(statusText)
                        .font(.headline)
                    Text(statusDetail)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }
            .padding()
            .background(statusColor.opacity(0.1))

            Divider()

            // Issues list
            List {
                if !errors.isEmpty {
                    Section {
                        ForEach(Array(errors.enumerated()), id: \.offset) { index, error in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Image(systemName: "xmark.circle.fill")
                                        .foregroundStyle(.red)
                                    Text(error.code)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Text(error.message)
                                    .font(.body)
                            }
                            .padding(.vertical, 4)
                        }
                    } header: {
                        Label("Errors (\(errors.count))", systemImage: "exclamationmark.triangle.fill")
                            .foregroundStyle(.red)
                    }
                }

                if !warnings.isEmpty {
                    Section {
                        ForEach(Array(warnings.enumerated()), id: \.offset) { index, warning in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundStyle(.orange)
                                    Text(warning.code)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Text(warning.message)
                                    .font(.body)
                            }
                            .padding(.vertical, 4)
                        }
                    } header: {
                        Label("Warnings (\(warnings.count))", systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.orange)
                    }
                }

                if errors.isEmpty && warnings.isEmpty {
                    Section {
                        VStack(spacing: 16) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.system(size: 60))
                                .foregroundStyle(.green)

                            Text("No Issues Found")
                                .font(.title3)

                            Text("The file passes all validation checks")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(40)
                    }
                }

                // File details
                Section("File Information") {
                    LabeledContent("Luminaire", value: ldt.luminaireName)
                    LabeledContent("Manufacturer", value: ldt.identification)
                    LabeledContent("Type", value: typeDescription)
                    LabeledContent("Symmetry", value: symmetryDescription)
                }

                Section("Geometry") {
                    LabeledContent("C-Planes", value: "\(ldt.numCPlanes)")
                    LabeledContent("γ-Angles", value: "\(ldt.numGPlanes)")
                    LabeledContent("Dimensions", value: "\(Int(ldt.length))×\(Int(ldt.width))×\(Int(ldt.height)) mm")
                }

                Section("Photometry") {
                    LabeledContent("Max Intensity", value: String(format: "%.1f cd/klm", ldt.maxIntensity))
                    LabeledContent("Total Flux", value: String(format: "%.0f lm", ldt.totalLuminousFlux))
                    LabeledContent("Lamp Sets", value: "\(ldt.lampSets.count)")
                }
            }
        }
        .onAppear {
            runValidation()
        }
    }

    private func runValidation() {
        // Get warnings
        warnings = validateLdt(ldt: ldt)

        // Get errors
        errors = getValidationErrors(ldt: ldt)
    }

    private var statusIcon: String {
        if !errors.isEmpty {
            return "xmark.circle.fill"
        } else if !warnings.isEmpty {
            return "exclamationmark.triangle.fill"
        } else {
            return "checkmark.circle.fill"
        }
    }

    private var statusColor: SwiftUI.Color {
        if !errors.isEmpty {
            return .red
        } else if !warnings.isEmpty {
            return .orange
        } else {
            return .green
        }
    }

    private var statusText: String {
        if !errors.isEmpty {
            return "Validation Failed"
        } else if !warnings.isEmpty {
            return "Validation Passed with Warnings"
        } else {
            return "Validation Passed"
        }
    }

    private var statusDetail: String {
        if !errors.isEmpty {
            return "\(errors.count) error\(errors.count == 1 ? "" : "s"), \(warnings.count) warning\(warnings.count == 1 ? "" : "s")"
        } else if !warnings.isEmpty {
            return "\(warnings.count) warning\(warnings.count == 1 ? "" : "s") found"
        } else {
            return "No issues detected"
        }
    }

    private var typeDescription: String {
        switch ldt.typeIndicator {
        case .pointSourceSymmetric: return "Point Source (Symmetric)"
        case .linear: return "Linear Luminaire"
        case .pointSourceOther: return "Point Source (Other)"
        }
    }

    private var symmetryDescription: String {
        switch ldt.symmetry {
        case .none: return "None"
        case .verticalAxis: return "Vertical Axis"
        case .planeC0c180: return "C0-C180 Plane"
        case .planeC90c270: return "C90-C270 Plane"
        case .bothPlanes: return "Both Planes"
        }
    }
}
