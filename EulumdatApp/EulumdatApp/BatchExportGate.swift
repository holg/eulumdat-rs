import SwiftUI
import StoreKit

#if os(macOS)

/// Gates `BatchConvertView` behind the one-time batch-export unlock.
/// Shows a paywall when locked; the real batch UI once purchased/restored.
struct BatchExportGate: View {
    @ObservedObject private var store = StoreManager.shared

    var body: some View {
        Group {
            if store.batchUnlocked {
                BatchConvertView()
            } else {
                paywall
            }
        }
        .task {
            await store.loadProducts()
            await store.refreshEntitlements()
        }
    }

    private var paywall: some View {
        VStack(spacing: 20) {
            Image(systemName: "lock.badge.plus")
                .font(.system(size: 56))
                .foregroundStyle(.tint)

            Text("Batch Export")
                .font(.title.bold())

            Text("Convert whole folders to IES, LDT, or themed SVG diagrams in one pass. A one-time purchase unlocks batch export on every device signed into your Apple ID.")
                .font(.body)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)

            VStack(alignment: .leading, spacing: 8) {
                featureRow("Folder-based batch conversion")
                featureRow("IES · LDT · themed SVG output")
                featureRow("Apply saved themes across every file")
            }
            .padding(.vertical, 4)

            Button {
                Task { await store.purchaseBatchExport() }
            } label: {
                if store.isPurchasing {
                    ProgressView().controlSize(.small)
                } else {
                    Text("Unlock for \(store.batchPriceText)")
                        .font(.headline)
                        .frame(maxWidth: 260)
                }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(store.isPurchasing || store.batchProduct == nil)

            Button("Restore Purchase") {
                Task { await store.restorePurchases() }
            }
            .buttonStyle(.plain)
            .font(.callout)
            .foregroundStyle(.tint)
            .disabled(store.isPurchasing)

            if let msg = store.errorMessage {
                Text(msg)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func featureRow(_ text: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "checkmark.circle.fill")
                .foregroundStyle(.green)
            Text(text)
                .font(.callout)
        }
    }
}

#endif
