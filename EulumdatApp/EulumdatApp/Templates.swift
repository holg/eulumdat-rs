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
    case ies
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
    // TM-33-23 horticultural templates
    case tm33Minimal
    case tm33CustomData
    case tm33HortLed
    case tm33FarRed
    case tm33Uv
    case tm33Seedling
    // TM-32-24 BIM templates
    case tm32OfficeDownlight
    case tm32RoadLuminaire
    // Floodlight templates (FL MAX LUM)
    case fl600wSym30
    case fl600wSym60
    case fl900wSym30
    case fl900wSym60
    case fl900wAsym
    case fl1200wSym10
    case fl1200wSym30
    case fl1200wSym60
    case fl1200wAsym
    // AEC IES templates
    case aecItalo
    case aecMaxwellCie
    case aecMaxwellIesna

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
        case .tm33Minimal: return "TM-33-23 Minimal"
        case .tm33CustomData: return "TM-33-23 Custom Data"
        case .tm33HortLed: return "TM-33-23 Horticultural LED"
        case .tm33FarRed: return "TM-33-23 Far-Red (730nm)"
        case .tm33Uv: return "TM-33-23 UV-A/B Supplemental"
        case .tm33Seedling: return "TM-33-23 Seedling/Clone"
        case .tm32OfficeDownlight: return "TM-32-24 Office Downlight (BIM)"
        case .tm32RoadLuminaire: return "TM-32-24 Road Luminaire (BIM)"
        case .fl600wSym30: return "FL MAX LUM 600W SYM 30°"
        case .fl600wSym60: return "FL MAX LUM 600W SYM 60°"
        case .fl900wSym30: return "FL MAX LUM 900W SYM 30°"
        case .fl900wSym60: return "FL MAX LUM 900W SYM 60°"
        case .fl900wAsym: return "FL MAX LUM 900W ASYM 50×110°"
        case .fl1200wSym10: return "FL MAX LUM 1200W SYM 10°"
        case .fl1200wSym30: return "FL MAX LUM 1200W SYM 30°"
        case .fl1200wSym60: return "FL MAX LUM 1200W SYM 60°"
        case .fl1200wAsym: return "FL MAX LUM 1200W ASYM 50×110°"
        case .aecItalo: return "AEC: ITALO 1 5P5"
        case .aecMaxwellCie: return "AEC: Maxwell 8-T4 (CIE)"
        case .aecMaxwellIesna: return "AEC: Maxwell 8-T4 (IESNA)"
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
        case .tm33Minimal: return "Minimal valid TM-33-23 document with all required fields"
        case .tm33CustomData: return "TM-33-23 with multiple CustomData blocks and extended fields"
        case .tm33HortLed: return "600W full spectrum LED panel with PPFD metrics and spectral data"
        case .tm33FarRed: return "120W far-red supplemental for flowering enhancement"
        case .tm33Uv: return "60W UV-A/UV-B for secondary metabolite enhancement"
        case .tm33Seedling: return "200W high-blue LED for seedling and clone propagation"
        case .tm32OfficeDownlight: return "6\" LED downlight with complete TM-32-24 BIM parameters"
        case .tm32RoadLuminaire: return "150W LED road luminaire with emergency backup and full BIM data"
        case .fl600wSym30: return "600W floodlight with symmetric 30° beam"
        case .fl600wSym60: return "600W floodlight with symmetric 60° beam"
        case .fl900wSym30: return "900W floodlight with symmetric 30° beam"
        case .fl900wSym60: return "900W floodlight with symmetric 60° beam"
        case .fl900wAsym: return "900W floodlight with asymmetric 50×110° beam"
        case .fl1200wSym10: return "1200W floodlight with symmetric 10° narrow beam"
        case .fl1200wSym30: return "1200W floodlight with symmetric 30° beam"
        case .fl1200wSym60: return "1200W floodlight with symmetric 60° beam"
        case .fl1200wAsym: return "1200W floodlight with asymmetric 50×110° beam"
        case .aecItalo: return "ITALO 1 5P5 S05 architectural luminaire (IES format)"
        case .aecMaxwellCie: return "Maxwell 8-T4 LUXEON 5050 measured photometry (CIE format)"
        case .aecMaxwellIesna: return "Maxwell 8-T4 LUXEON 5050 measured photometry (IESNA format)"
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
        case .wikiFlood: return "light.max"
        case .atlaGrowLight, .atlaGrowLightRB: return "leaf.fill"
        case .atlaFluorescent: return "lightbulb.led.wide.fill"
        case .atlaHalogen: return "lightbulb.fill"
        case .atlaIncandescent: return "lightbulb"
        case .atlaHeatLamp: return "flame.fill"
        case .atlaUvBlacklight: return "waveform"
        case .tm33Minimal, .tm33CustomData: return "doc.text"
        case .tm33HortLed, .tm33Seedling: return "leaf.fill"
        case .tm33FarRed: return "sun.max.fill"
        case .tm33Uv: return "waveform"
        case .tm32OfficeDownlight: return "building.2"
        case .tm32RoadLuminaire: return "road.lanes"
        case .fl600wSym30, .fl600wSym60, .fl900wSym30, .fl900wSym60,
             .fl1200wSym10, .fl1200wSym30, .fl1200wSym60: return "lightbulb.max"
        case .fl900wAsym, .fl1200wAsym: return "lightbulb.max.fill"
        case .aecItalo, .aecMaxwellCie, .aecMaxwellIesna: return "building.columns"
        }
    }

    /// Template file format
    var format: TemplateFormat {
        switch self {
        case .atlaGrowLight, .atlaGrowLightRB, .atlaFluorescent,
             .atlaHalogen, .atlaIncandescent, .atlaHeatLamp, .atlaUvBlacklight,
             .tm33Minimal, .tm33CustomData, .tm33HortLed, .tm33FarRed, .tm33Uv, .tm33Seedling,
             .tm32OfficeDownlight, .tm32RoadLuminaire:
            return .atlaXml
        case .aecItalo, .aecMaxwellCie, .aecMaxwellIesna:
            return .ies
        default:
            return .ldt
        }
    }

    /// Whether this template has spectral data
    var hasSpectralData: Bool {
        format == .atlaXml && self != .tm33Minimal && self != .tm33CustomData
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
        case .tm33Minimal: return "tm-33-23_minimal"
        case .tm33CustomData: return "tm-33-23_with_custom_data"
        case .tm33HortLed: return "tm-33-23_horticultural_led"
        case .tm33FarRed: return "tm-33-23_far_red_supplemental"
        case .tm33Uv: return "tm-33-23_uv_supplemental"
        case .tm33Seedling: return "tm-33-23_seedling_propagation"
        case .tm32OfficeDownlight: return "tm-32-24_office_downlight_bim"
        case .tm32RoadLuminaire: return "tm-32-24_road_luminaire_bim"
        case .fl600wSym30: return "4058075580596_FL_MAX_LUM_600W_757_SYM_30_WAL"
        case .fl600wSym60: return "4058075580602_FL_MAX_LUM_600W_757_SYM_60_WAL"
        case .fl900wSym30: return "4058075580633_FL_MAX_LUM_900W_757_SYM_30_WAL"
        case .fl900wSym60: return "4058075580640_FL_MAX_LUM_900W_757_SYM_60_WAL"
        case .fl900wAsym: return "4058075580657_FL_MAX_LUM_900W_757_ASYM_50X110_WAL"
        case .fl1200wSym10: return "4058075580664_FL_MAX_LUM_1200W_757_SYM_10_WAL"
        case .fl1200wSym30: return "4058075580671_FL_MAX_LUM_1200W_757_SYM_30_WAL"
        case .fl1200wSym60: return "4058075580688_FL_MAX_LUM_1200W_757_SYM_60_WAL"
        case .fl1200wAsym: return "4058075580695_FL_MAX_LUM_1200W_757_ASYM_50X110WAL"
        case .aecItalo: return "ITALO 1 5P5 S05 3.140-3M"
        case .aecMaxwellCie: return "S01.01.02.354_MAXWELL-8-T4 LUXEON 5050 Square with glass-MEASURED_CIE"
        case .aecMaxwellIesna: return "S01.01.02.354_MAXWELL-8-T4 LUXEON 5050 Square with glass-MEASURED_IESNA"
        }
    }

    /// File extension for this template
    var fileExtension: String {
        switch self {
        case .aecItalo, .aecMaxwellCie: return "IES"
        case .aecMaxwellIesna: return "ies"
        default:
            switch format {
            case .atlaXml: return "xml"
            case .ies: return "ies"
            case .ldt: return "ldt"
            }
        }
    }

    /// Load the template content from the bundled file
    func loadContent() -> String? {
        let bundle = Bundle.resourceBundle
        let ext = fileExtension

        // Find the URL
        var url: URL?

        // Try with subdirectory
        url = bundle.url(forResource: fileName, withExtension: ext, subdirectory: "Templates")

        // Try without subdirectory (flat bundle)
        if url == nil {
            url = bundle.url(forResource: fileName, withExtension: ext)
        }

        // Try direct path construction
        if url == nil, let resourcePath = bundle.resourcePath {
            let directPath = (resourcePath as NSString).appendingPathComponent("Templates/\(fileName).\(ext)")
            if FileManager.default.fileExists(atPath: directPath) {
                url = URL(fileURLWithPath: directPath)
            }
        }

        guard let fileUrl = url else {
            print("Template file not found: \(fileName).\(ext) in bundle: \(bundle.bundlePath)")
            return nil
        }

        // Try ISO Latin-1 first (handles LDT/IES files with extended chars), then UTF-8
        if let content = try? String(contentsOf: fileUrl, encoding: .isoLatin1) {
            return content
        }
        if let content = try? String(contentsOf: fileUrl, encoding: .utf8) {
            return content
        }

        print("Failed to read template \(fileName).\(ext)")
        return nil
    }

    /// Load the LDT content (for backwards compatibility)
    func loadLdtContent() -> String? {
        if format == .ldt || format == .ies {
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
        case .ies:
            do {
                return try parseIes(content: content)
            } catch {
                print("Failed to parse IES template \(fileName): \(error)")
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
        case .ies:
            do {
                return try AtlaDocument.fromIes(content: content)
            } catch {
                print("Failed to create ATLA from IES template \(fileName): \(error)")
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
            let ext = template.fileExtension
            if let url = bundle.url(forResource: template.fileName, withExtension: ext, subdirectory: "Templates") {
                print("  ✓ \(template.rawValue): \(url.path)")
            } else if let url = bundle.url(forResource: template.fileName, withExtension: ext) {
                print("  ✓ \(template.rawValue): \(url.path) (flat)")
            } else {
                print("  ✗ \(template.rawValue): NOT FOUND (\(template.fileName).\(ext))")
            }
        }
    }
}
