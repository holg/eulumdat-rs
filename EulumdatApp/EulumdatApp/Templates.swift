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

/// Built-in luminaire templates loaded from real LDT files
enum LuminaireTemplate: String, CaseIterable, Identifiable {
    case downlight = "Downlight"
    case projector = "Projector"
    case linear = "Linear Luminaire"
    case fluorescent = "Fluorescent Luminaire"
    case roadLuminaire = "Road Luminaire"
    case floorUplight = "Floor Uplight"

    var id: String { rawValue }

    var description: String {
        switch self {
        case .downlight: return "Simple downlight with vertical axis symmetry"
        case .projector: return "CDM-TD 70W spotlight with asymmetric beam"
        case .linear: return "Linear luminaire with C0-C180 symmetry"
        case .fluorescent: return "T16 G5 54W linear luminaire with bilateral symmetry"
        case .roadLuminaire: return "SON-TPP 250W street light with C90-C270 symmetry"
        case .floorUplight: return "HIT-DE 250W floor-standing uplight"
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
        }
    }

    /// The filename of the LDT template in the bundle
    var fileName: String {
        switch self {
        case .downlight: return "1-1-0"
        case .projector: return "projector"
        case .linear: return "0-2-0"
        case .fluorescent: return "fluorescent_luminaire"
        case .roadLuminaire: return "road_luminaire"
        case .floorUplight: return "floor_uplight"
        }
    }

    /// Load the LDT content from the bundled template file
    func loadLdtContent() -> String? {
        let bundle = Bundle.resourceBundle

        // Try with subdirectory
        if let url = bundle.url(forResource: fileName, withExtension: "ldt", subdirectory: "Templates") {
            do {
                return try String(contentsOf: url, encoding: .utf8)
            } catch {
                print("Failed to read template \(fileName): \(error)")
            }
        }

        // Try without subdirectory (flat bundle)
        if let url = bundle.url(forResource: fileName, withExtension: "ldt") {
            do {
                return try String(contentsOf: url, encoding: .utf8)
            } catch {
                print("Failed to read template \(fileName): \(error)")
            }
        }

        // Try direct path construction
        if let resourcePath = bundle.resourcePath {
            let directPath = (resourcePath as NSString).appendingPathComponent("Templates/\(fileName).ldt")
            if FileManager.default.fileExists(atPath: directPath) {
                do {
                    return try String(contentsOfFile: directPath, encoding: .utf8)
                } catch {
                    print("Failed to read template at \(directPath): \(error)")
                }
            }
        }

        print("Template file not found: \(fileName).ldt in bundle: \(bundle.bundlePath)")
        return nil
    }

    /// Parse and create Eulumdat from the bundled template
    func createEulumdat() -> Eulumdat? {
        guard let content = loadLdtContent() else {
            return nil
        }
        do {
            return try parseLdt(content: content)
        } catch {
            print("Failed to parse template \(fileName): \(error)")
            return nil
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
