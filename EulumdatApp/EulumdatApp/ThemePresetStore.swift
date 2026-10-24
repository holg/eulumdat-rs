import SwiftUI
import EulumdatKit

/// Persistent store of named custom SVG themes.
///
/// Presets are kept as TOML strings (the same format the Theme editor exports),
/// so persistence reuses the round-trip-tested `themeConfigToToml` /
/// `themeConfigFromToml` FFI functions and needs no `Codable` on the FFI struct.
/// Backed by `UserDefaults` under a single dictionary key.
@MainActor
final class ThemePresetStore: ObservableObject {
    static let shared = ThemePresetStore()

    private let defaultsKey = "eulumdat.themePresets"
    private let currentKey = "eulumdat.themeCurrent"

    /// name → TOML string.
    @Published private(set) var presets: [String: String] = [:]

    /// The theme currently being edited in the Theme tab, as a TOML string.
    /// Published live so Batch export can offer it without an explicit save.
    /// `nil` until the Theme tab has been opened at least once.
    @Published private(set) var currentToml: String?

    private init() {
        load()
    }

    /// Remember the theme the user is editing right now (called by the Theme
    /// editor on every change) so other views can apply it immediately.
    func setCurrent(_ config: SvgThemeConfig) {
        currentToml = themeConfigToToml(config: config)
        UserDefaults.standard.set(currentToml, forKey: currentKey)
    }

    /// The current in-editor theme, decoded, or nil if none/unparseable.
    var current: SvgThemeConfig? {
        guard let toml = currentToml else { return nil }
        return try? themeConfigFromToml(toml: toml)
    }

    /// Preset names, sorted for stable menus.
    var names: [String] {
        presets.keys.sorted()
    }

    /// Save (or overwrite) a preset under `name`.
    func save(_ config: SvgThemeConfig, as name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        presets[trimmed] = themeConfigToToml(config: config)
        persist()
    }

    /// Load a preset by name, or nil if absent / unparseable.
    func config(named name: String) -> SvgThemeConfig? {
        guard let toml = presets[name] else { return nil }
        return try? themeConfigFromToml(toml: toml)
    }

    /// Delete a preset.
    func delete(_ name: String) {
        presets.removeValue(forKey: name)
        persist()
    }

    // MARK: - Persistence

    private func load() {
        if let dict = UserDefaults.standard.dictionary(forKey: defaultsKey) as? [String: String] {
            presets = dict
        }
        currentToml = UserDefaults.standard.string(forKey: currentKey)
    }

    private func persist() {
        UserDefaults.standard.set(presets, forKey: defaultsKey)
    }
}
