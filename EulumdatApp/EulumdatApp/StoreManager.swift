import Foundation
import StoreKit

/// StoreKit 2 manager for the one-time "Batch Export" unlock (non-consumable).
///
/// The product is a single non-consumable IAP. Once purchased, the entitlement
/// is derived from `Transaction.currentEntitlements` (Apple's source of truth),
/// so it restores automatically on any device signed into the same Apple ID.
@MainActor
final class StoreManager: ObservableObject {
    static let shared = StoreManager()

    /// The product identifier — must match the one created in App Store Connect
    /// and in the local `Products.storekit` test file.
    static let batchExportProductID = "eu.trahe.eulumdat.batchexport"

    /// The loaded product (nil until `loadProducts()` succeeds).
    @Published private(set) var batchProduct: Product?

    /// Whether the batch-export feature is unlocked.
    @Published private(set) var batchUnlocked = false

    /// A purchase or restore is in flight.
    @Published private(set) var isPurchasing = false

    /// Last user-facing error, if any.
    @Published var errorMessage: String?

    private var updatesTask: Task<Void, Never>?

    private init() {
        // Listen for transactions that arrive outside an explicit purchase call
        // (Ask-to-Buy approvals, restores, purchases on other devices).
        updatesTask = Task { [weak self] in
            for await update in Transaction.updates {
                await self?.handle(transactionResult: update)
            }
        }
        Task { await refreshEntitlements() }
    }

    deinit { updatesTask?.cancel() }

    /// Localized price string for UI, e.g. "€50.00". Falls back to a plain "€50".
    var batchPriceText: String {
        batchProduct?.displayPrice ?? "€50"
    }

    /// Load products from the store (call on view appear).
    func loadProducts() async {
        do {
            let products = try await Product.products(for: [Self.batchExportProductID])
            batchProduct = products.first
        } catch {
            errorMessage = "Couldn’t load the store: \(error.localizedDescription)"
        }
    }

    /// Purchase the batch-export unlock.
    func purchaseBatchExport() async {
        guard let product = batchProduct else {
            errorMessage = "Product not available yet. Try again in a moment."
            return
        }
        isPurchasing = true
        defer { isPurchasing = false }
        do {
            let result = try await product.purchase()
            switch result {
            case .success(let verification):
                await handle(transactionResult: verification)
            case .userCancelled:
                break
            case .pending:
                // Ask-to-Buy / SCA — the entitlement arrives later via updates.
                errorMessage = "Purchase pending approval."
            @unknown default:
                break
            }
        } catch {
            errorMessage = "Purchase failed: \(error.localizedDescription)"
        }
    }

    /// Restore purchases (App Store syncs entitlements; then re-check).
    func restorePurchases() async {
        isPurchasing = true
        defer { isPurchasing = false }
        do {
            try await AppStore.sync()
            await refreshEntitlements()
            if !batchUnlocked {
                errorMessage = "No previous purchase found to restore."
            }
        } catch {
            errorMessage = "Restore failed: \(error.localizedDescription)"
        }
    }

    /// Recompute the entitlement from current entitlements.
    func refreshEntitlements() async {
        for await entitlement in Transaction.currentEntitlements {
            if case .verified(let transaction) = entitlement,
               transaction.productID == Self.batchExportProductID,
               transaction.revocationDate == nil {
                batchUnlocked = true
                return
            }
        }
        batchUnlocked = false
    }

    /// Apply a verified transaction and finish it.
    private func handle(transactionResult: VerificationResult<Transaction>) async {
        guard case .verified(let transaction) = transactionResult else {
            // StoreKit couldn't verify the transaction signature — ignore it.
            errorMessage = "Could not verify the purchase."
            return
        }
        if transaction.productID == Self.batchExportProductID,
           transaction.revocationDate == nil {
            batchUnlocked = true
        }
        await transaction.finish()
    }
}
