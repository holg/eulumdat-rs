import Foundation
import EulumdatKit

// MARK: - Bundle Extension for Resource Access

private extension Bundle {
    /// Returns the resource bundle for the app, working in both SPM and Xcode builds
    static var resourceBundle: Bundle = {
        let bundleName = "EulumdatApp_EulumdatApp"

        let candidates = [
            // SPM build - Bundle.module equivalent
            Bundle.main.resourceURL,
            // Running from Xcode
            Bundle.main.bundleURL,
            // Running as a command line tool
            Bundle.main.bundleURL.deletingLastPathComponent(),
            // SPM build path
            Bundle(for: BundleFinder.self).resourceURL,
        ]

        for candidate in candidates {
            let bundlePath = candidate?.appendingPathComponent(bundleName + ".bundle")
            if let bundle = bundlePath.flatMap(Bundle.init(url:)) {
                return bundle
            }
        }

        // Fallback: try to find Templates directory directly in various locations
        let fm = FileManager.default
        let searchPaths = [
            Bundle.main.bundlePath,
            Bundle.main.resourcePath,
            fm.currentDirectoryPath,
            // Development paths
            fm.currentDirectoryPath + "/.build/debug/\(bundleName).bundle",
            fm.currentDirectoryPath + "/.build/release/\(bundleName).bundle",
        ].compactMap { $0 }

        for path in searchPaths {
            let templatesPath = path + "/Templates"
            if fm.fileExists(atPath: templatesPath) {
                if let bundle = Bundle(path: path) {
                    return bundle
                }
            }
            // Check one level up for bundle
            let bundlePath = path + "/\(bundleName).bundle"
            if fm.fileExists(atPath: bundlePath) {
                if let bundle = Bundle(path: bundlePath) {
                    return bundle
                }
            }
        }

        // Last resort: return main bundle
        return Bundle.main
    }()
}

// Dummy class for bundle finding
private class BundleFinder {}

// MARK: - Luminaire Template

/// Template file format
enum TemplateFormat {
    case ldt
    case atlaXml
}

/// Built-in luminaire templates loaded from real LDT and ATLA XML files
enum LuminaireTemplate: String, CaseIterable, Identifiable {
    // Standard LDT templates
    case downlight
    case projector
    case linear
    case fluorescent
    case roadLuminaire
    case floorUplight
    // Wikipedia example templates
    case wikiBatwing
    case wikiSpotlight
    case wikiFlood
    // ATLA templates with spectral data
    case atlaGrowLight
    case atlaGrowLightRB
    case atlaFluorescent
    case atlaHalogen
    case atlaIncandescent
    case atlaHeatLamp
    case atlaUvBlacklight

    var id: String { rawValue }

    /// Localized display name
    var displayName: String {
        switch self {
        case .downlight: return String(localized: "template.downlight")
        case .projector: return String(localized: "template.projector")
        case .linear: return String(localized: "template.linear")
        case .fluorescent: return String(localized: "template.fluorescent")
        case .roadLuminaire: return String(localized: "template.roadLuminaire")
        case .floorUplight: return String(localized: "template.floorUplight")
        case .wikiBatwing: return String(localized: "template.wikiBatwing")
        case .wikiSpotlight: return String(localized: "template.wikiSpotlight")
        case .wikiFlood: return String(localized: "template.wikiFlood")
        case .atlaGrowLight: return String(localized: "template.atlaGrowLight")
        case .atlaGrowLightRB: return String(localized: "template.atlaGrowLightRB")
        case .atlaFluorescent: return String(localized: "template.atlaFluorescent")
        case .atlaHalogen: return String(localized: "template.atlaHalogen")
        case .atlaIncandescent: return String(localized: "template.atlaIncandescent")
        case .atlaHeatLamp: return String(localized: "template.atlaHeatLamp")
        case .atlaUvBlacklight: return String(localized: "template.atlaUvBlacklight")
        }
    }

    var description: String {
        switch self {
        case .downlight: return String(localized: "template.downlight.desc")
        case .projector: return String(localized: "template.projector.desc")
        case .linear: return String(localized: "template.linear.desc")
        case .fluorescent: return String(localized: "template.fluorescent.desc")
        case .roadLuminaire: return String(localized: "template.roadLuminaire.desc")
        case .floorUplight: return String(localized: "template.floorUplight.desc")
        case .wikiBatwing: return String(localized: "template.wikiBatwing.desc")
        case .wikiSpotlight: return String(localized: "template.wikiSpotlight.desc")
        case .wikiFlood: return String(localized: "template.wikiFlood.desc")
        case .atlaGrowLight: return String(localized: "template.atlaGrowLight.desc")
        case .atlaGrowLightRB: return String(localized: "template.atlaGrowLightRB.desc")
        case .atlaFluorescent: return String(localized: "template.atlaFluorescent.desc")
        case .atlaHalogen: return String(localized: "template.atlaHalogen.desc")
        case .atlaIncandescent: return String(localized: "template.atlaIncandescent.desc")
        case .atlaHeatLamp: return String(localized: "template.atlaHeatLamp.desc")
        case .atlaUvBlacklight: return String(localized: "template.atlaUvBlacklight.desc")
        }
    }

    var icon: String {
        switch self {
        case .downlight: return "light.recessed"
        case .projector: return "light.max"
        case .linear: return "rectangle"
        case .fluorescent: return "lightbulb.led"
        case .roadLuminaire: return "light.beacon.max"
        case .floorUplight: return "lamp.floor"
        case .wikiBatwing: return "light.panel"
        case .wikiSpotlight: return "light.min"
        case .wikiFlood: return "light.flood.fill"
        case .atlaGrowLight, .atlaGrowLightRB: return "leaf.fill"
        case .atlaFluorescent: return "lightbulb.led.wide.fill"
        case .atlaHalogen: return "lightbulb.fill"
        case .atlaIncandescent: return "lightbulb"
        case .atlaHeatLamp: return "flame.fill"
        case .atlaUvBlacklight: return "waveform"
        }
    }

    /// Template file format
    var format: TemplateFormat {
        switch self {
        case .atlaGrowLight, .atlaGrowLightRB, .atlaFluorescent,
             .atlaHalogen, .atlaIncandescent, .atlaHeatLamp, .atlaUvBlacklight:
            return .atlaXml
        default:
            return .ldt
        }
    }

    /// Whether this template has spectral data
    var hasSpectralData: Bool {
        format == .atlaXml
    }

    /// The filename of the template in the bundle (without extension)
    var fileName: String {
        switch self {
        case .downlight: return "1-1-0"
        case .projector: return "projector"
        case .linear: return "0-2-0"
        case .fluorescent: return "fluorescent_luminaire"
        case .roadLuminaire: return "road_luminaire"
        case .floorUplight: return "floor_uplight"
        case .wikiBatwing: return "wiki-batwing"
        case .wikiSpotlight: return "wiki-spotlight"
        case .wikiFlood: return "wiki-flood"
        case .atlaGrowLight: return "_atla_grow_light"
        case .atlaGrowLightRB: return "_atla_grow_light_rb"
        case .atlaFluorescent: return "_atla_fluorescent"
        case .atlaHalogen: return "_atla_halogen_lamp"
        case .atlaIncandescent: return "_atla_incandescent"
        case .atlaHeatLamp: return "_atla_heat_lamp"
        case .atlaUvBlacklight: return "_atla_uv_blacklight"
        }
    }

    /// File extension for this template
    var fileExtension: String {
        format == .atlaXml ? "xml" : "ldt"
    }

    /// Load the template content from the bundled file
    func loadContent() -> String? {
        let bundle = Bundle.resourceBundle
        let ext = fileExtension

        // Try with subdirectory
        if let url = bundle.url(forResource: fileName, withExtension: ext, subdirectory: "Templates") {
            do {
                return try String(contentsOf: url, encoding: .utf8)
            } catch {
                print("Failed to read template \(fileName): \(error)")
            }
        }

        // Try without subdirectory (flat bundle)
        if let url = bundle.url(forResource: fileName, withExtension: ext) {
            do {
                return try String(contentsOf: url, encoding: .utf8)
            } catch {
                print("Failed to read template \(fileName): \(error)")
            }
        }

        // Try direct path construction
        if let resourcePath = bundle.resourcePath {
            let directPath = (resourcePath as NSString).appendingPathComponent("Templates/\(fileName).\(ext)")
            if FileManager.default.fileExists(atPath: directPath) {
                do {
                    return try String(contentsOfFile: directPath, encoding: .utf8)
                } catch {
                    print("Failed to read template at \(directPath): \(error)")
                }
            }
        }

        print("Template file not found: \(fileName).\(ext) in bundle: \(bundle.bundlePath)")
        return nil
    }

    /// Load the LDT content (for backwards compatibility)
    func loadLdtContent() -> String? {
        if format == .ldt {
            return loadContent()
        }
        // For ATLA templates, we need to convert via AtlaDocument
        return nil
    }

    /// Parse and create Eulumdat from the bundled template
    func createEulumdat() -> Eulumdat? {
        guard let content = loadContent() else {
            return nil
        }

        switch format {
        case .ldt:
            do {
                return try parseLdt(content: content)
            } catch {
                print("Failed to parse LDT template \(fileName): \(error)")
                return nil
            }
        case .atlaXml:
            do {
                let atlaDoc = try AtlaDocument.parseXml(content: content)
                // Convert ATLA to LDT string, then parse
                let ldtContent = atlaDoc.toLdt()
                return try parseLdt(content: ldtContent)
            } catch {
                print("Failed to parse ATLA template \(fileName): \(error)")
                return nil
            }
        }
    }

    /// Create AtlaDocument from the bundled template (for spectral data access)
    func createAtlaDocument() -> AtlaDocument? {
        guard let content = loadContent() else {
            return nil
        }

        switch format {
        case .ldt:
            do {
                return try AtlaDocument.fromLdt(content: content)
            } catch {
                print("Failed to create ATLA from LDT template \(fileName): \(error)")
                return nil
            }
        case .atlaXml:
            do {
                return try AtlaDocument.parseXml(content: content)
            } catch {
                print("Failed to parse ATLA template \(fileName): \(error)")
                return nil
            }
        }
    }
}

// MARK: - Template Manager

/// Helper to manage and list all available templates
struct TemplateManager {
    /// Get all templates with their parsed Eulumdat data
    static func loadAllTemplates() -> [(template: LuminaireTemplate, ldt: Eulumdat)] {
        LuminaireTemplate.allCases.compactMap { template in
            guard let ldt = template.createEulumdat() else { return nil }
            return (template, ldt)
        }
    }

    /// Debug: print all available template files
    static func listBundledTemplates() {
        let bundle = Bundle.resourceBundle
        print("Resource bundle: \(bundle.bundlePath)")
        print("Bundled template files:")
        for template in LuminaireTemplate.allCases {
            if let url = bundle.url(forResource: template.fileName, withExtension: "ldt", subdirectory: "Templates") {
                print("  ✓ \(template.rawValue): \(url.path)")
            } else if let url = bundle.url(forResource: template.fileName, withExtension: "ldt") {
                print("  ✓ \(template.rawValue): \(url.path) (flat)")
            } else {
                print("  ✗ \(template.rawValue): NOT FOUND (\(template.fileName).ldt)")
            }
        }
    }
}
