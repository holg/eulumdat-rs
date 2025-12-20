#!/usr/bin/env python3
"""
Generate platform-specific localization files from JSON source of truth.

Usage:
    python scripts/generate-localizations.py [--swift] [--kotlin] [--harmonyos]

    Without arguments, generates all platforms.

Source: locales/*.json
Outputs:
    - Swift:     EulumdatApp/EulumdatApp/Localizable.xcstrings
    - Kotlin:    EulumdatAndroid/app/src/main/res/values-*/strings.xml
    - HarmonyOS: EulumdatHarmonyOS/entry/src/main/resources/*/element/string.json
"""

import json
import os
import sys
from pathlib import Path
from typing import Dict, Any
from datetime import datetime

# Project root
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
LOCALES_DIR = PROJECT_ROOT / "locales"

# Output paths
SWIFT_OUTPUT = PROJECT_ROOT / "EulumdatApp" / "EulumdatApp" / "Localizable.xcstrings"
KOTLIN_OUTPUT_DIR = PROJECT_ROOT / "EulumdatAndroid" / "app" / "src" / "main" / "res"
HARMONYOS_OUTPUT_DIR = PROJECT_ROOT / "EulumdatHarmonyOS" / "entry" / "src" / "main" / "resources"

# Language code mappings
SWIFT_LANG_CODES = {
    "en": "en",
    "de": "de",
    "zh": "zh-Hans",
    "fr": "fr",
    "it": "it",
    "ru": "ru",
    "es": "es",
    "pt-BR": "pt-BR"
}

KOTLIN_LANG_CODES = {
    "en": "",  # default, values/
    "de": "de",
    "zh": "zh-rCN",
    "fr": "fr",
    "it": "it",
    "ru": "ru",
    "es": "es",
    "pt-BR": "pt-rBR"
}

def flatten_json(obj: Dict, prefix: str = "") -> Dict[str, str]:
    """Flatten nested JSON to dot-notation keys."""
    result = {}
    for key, value in obj.items():
        full_key = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            result.update(flatten_json(value, full_key))
        elif isinstance(value, str):
            result[full_key] = value
    return result

# Key mappings from JSON paths to Swift keys
# JSON key -> Swift key (for backwards compatibility with existing Swift code)
SWIFT_KEY_MAPPINGS = {
    # Welcome (from app section)
    "app.welcome.title": "welcome.title",
    "app.welcome.subtitle": "welcome.subtitle",
    "app.welcome.openFile": "welcome.openFile",
    "app.welcome.newFromTemplate": "welcome.newFromTemplate",
    "app.welcome.dropFile": "welcome.dropFile",
    # Settings (from app section)
    "app.settings.language": "settings.language",
    "app.settings.language_system": "settings.language.system",
    "app.settings.appearance": "settings.appearance",
    "app.settings.darkTheme": "settings.darkTheme",
    "app.settings.defaultDiagram": "settings.defaultDiagram",
    "app.settings.mountingHeight": "settings.mountingHeight",
    "app.settings.mountingHeight_description": "settings.mountingHeight.description",
    "app.settings.export": "settings.export",
    "app.settings.about": "settings.about",
    # Toolbar (from app section)
    "app.toolbar.open": "toolbar.open",
    "app.toolbar.export": "toolbar.export",
    "app.toolbar.exportSVG": "toolbar.exportSVG",
    "app.toolbar.exportIES": "toolbar.exportIES",
    "app.toolbar.exportLDT": "toolbar.exportLDT",
    "app.toolbar.dark": "toolbar.dark",
    # Nav
    "app.nav.title": "nav.title",
    # Error
    "app.error.ok": "error.ok",
    # Fullscreen
    "app.fullscreen.darkTheme": "fullscreen.darkTheme",
    "app.fullscreen.done": "fullscreen.done",
    # Templates (from app section)
    "app.template.downlight": "template.downlight",
    "app.template.downlight_desc": "template.downlight.desc",
    "app.template.projector": "template.projector",
    "app.template.projector_desc": "template.projector.desc",
    "app.template.linear": "template.linear",
    "app.template.linear_desc": "template.linear.desc",
    "app.template.fluorescent": "template.fluorescent",
    "app.template.fluorescent_desc": "template.fluorescent.desc",
    "app.template.roadLuminaire": "template.roadLuminaire",
    "app.template.roadLuminaire_desc": "template.roadLuminaire.desc",
    "app.template.floorUplight": "template.floorUplight",
    "app.template.floorUplight_desc": "template.floorUplight.desc",
    "app.template.wikiBatwing": "template.wikiBatwing",
    "app.template.wikiBatwing_desc": "template.wikiBatwing.desc",
    "app.template.wikiSpotlight": "template.wikiSpotlight",
    "app.template.wikiSpotlight_desc": "template.wikiSpotlight.desc",
    "app.template.wikiFlood": "template.wikiFlood",
    "app.template.wikiFlood_desc": "template.wikiFlood.desc",
    "app.template.atlaGrowLight": "template.atlaGrowLight",
    "app.template.atlaGrowLight_desc": "template.atlaGrowLight.desc",
    "app.template.atlaGrowLightRB": "template.atlaGrowLightRB",
    "app.template.atlaGrowLightRB_desc": "template.atlaGrowLightRB.desc",
    "app.template.atlaFluorescent": "template.atlaFluorescent",
    "app.template.atlaFluorescent_desc": "template.atlaFluorescent.desc",
    "app.template.atlaHalogen": "template.atlaHalogen",
    "app.template.atlaHalogen_desc": "template.atlaHalogen.desc",
    "app.template.atlaIncandescent": "template.atlaIncandescent",
    "app.template.atlaIncandescent_desc": "template.atlaIncandescent.desc",
    "app.template.atlaHeatLamp": "template.atlaHeatLamp",
    "app.template.atlaHeatLamp_desc": "template.atlaHeatLamp.desc",
    "app.template.atlaUvBlacklight": "template.atlaUvBlacklight",
    "app.template.atlaUvBlacklight_desc": "template.atlaUvBlacklight.desc",
    # Tabs (from app section - Swift native)
    "app.tab.general": "tab.general",
    "app.tab.dimensions": "tab.dimensions",
    "app.tab.lampSets": "tab.lampSets",
    "app.tab.optical": "tab.optical",
    "app.tab.intensity": "tab.intensity",
    "app.tab.diagram": "tab.diagram",
    # Validation (from app section - Swift native)
    "app.validation.title": "validation.title",
    # Diagram picker names (from app section - Swift native)
    "app.diagram.polar": "diagram.polar",
    "app.diagram.cartesian": "diagram.cartesian",
    "app.diagram.butterfly": "diagram.butterfly",
    "app.diagram.3d": "diagram.3d",
    "app.diagram.room": "diagram.room",
    "app.diagram.heatmap": "diagram.heatmap",
    "app.diagram.cone": "diagram.cone",
    "app.diagram.beam": "diagram.beam",
    "app.diagram.spectral": "diagram.spectral",
    "app.diagram.ppfd": "diagram.ppfd",
    "app.diagram.bug": "diagram.bug",
    "app.diagram.lcs": "diagram.lcs",
    # Tabs (from ui section - WASM)
    "ui.tabs.general": "ui.tabs.general",
    "ui.tabs.dimensions": "ui.tabs.dimensions",
    "ui.tabs.lamp_sets": "ui.tabs.lamp_sets",
    "ui.tabs.direct_ratios": "ui.tabs.direct_ratios",
    "ui.tabs.intensity": "ui.tabs.intensity",
    "ui.tabs.diagram_2d": "ui.tabs.diagram_2d",
    "ui.tabs.validation": "ui.tabs.validation",
    # Diagrams - short names for picker/menu (WASM)
    "ui.tabs.polar": "ui.tabs.polar",
    "ui.tabs.cartesian": "ui.tabs.cartesian",
    "ui.tabs.heatmap": "ui.tabs.heatmap",
    "ui.tabs.spectral": "ui.tabs.spectral",
    "ui.tabs.greenhouse": "ui.tabs.greenhouse",
    "ui.tabs.bug_rating": "ui.tabs.bug_rating",
    "ui.tabs.lcs": "ui.tabs.lcs",
    "ui.tabs.cone": "cone",
    "ui.diagram.title_3d": "butterfly",
    # Diagram labels
    "diagram.title.polar": "diagram.polar",
    "diagram.title.cartesian": "diagram.cartesian",
    "diagram.title.cone": "diagram.cone",
    "diagram.title.heatmap": "diagram.heatmap",
    "ui.tabs.spectral": "diagram.spectral",
    "ui.tabs.greenhouse": "diagram.ppfd",
    "ui.tabs.bug_rating": "diagram.bug",
    "ui.tabs.lcs": "diagram.lcs",
    "ui.diagram.title_3d": "diagram.3d",
    "diagram.angle.beam": "diagram.beam",
    "ui.tabs.diagram_3d": "diagram.butterfly",
    # Luminaire info
    "luminaire.info.manufacturer": "general.manufacturer",
    "luminaire.info.luminaire_name": "general.luminaireName",
    "luminaire.info.luminaire_number": "general.luminaireNumber",
    "luminaire.info.file_name": "general.fileName",
    "luminaire.info.identification": "general.identification",
    # Dimensions
    "luminaire.physical.length": "dimensions.length",
    "luminaire.physical.width": "dimensions.width",
    "luminaire.physical.height": "dimensions.height",
    "luminaire.physical.dimensions_mm": "dimensions.luminaire",
    "luminaire.physical.luminous_area_mm": "dimensions.luminousArea",
    # Optical
    "luminaire.optical.downward_flux_fraction": "optical.downwardFlux",
    "luminaire.optical.light_output_ratio": "optical.lightOutput",
    "luminaire.optical.conversion_factor": "optical.conversionFactor",
    "luminaire.optical.tilt_angle": "optical.tiltAngle",
    "luminaire.photometric.total_flux": "optical.totalFlux",
    "luminaire.photometric.max_intensity": "optical.maxIntensity",
    "luminaire.photometric.lor": "optical.lor",
    # Lamp sets
    "luminaire.lamp_set.title": "lampSets.title",
    "luminaire.lamp_set.num_lamps": "lampSets.numLamps",
    "luminaire.lamp_set.luminous_flux": "lampSets.luminousFlux",
    "luminaire.lamp_set.wattage": "lampSets.wattage",
    "luminaire.lamp_set.lamp_type": "lampSets.type",
    "luminaire.lamp_set.color_appearance": "lampSets.colorTemp",
    "luminaire.lamp_set.color_rendering": "lampSets.criGroup",
    "luminaire.lamp_set.remove": "lampSets.remove",
    # Intensity
    "ui.intensity.title": "intensity.title",
    "ui.data_table.copy_to_clipboard": "intensity.copyCSV",
}

def apply_key_mappings(translations: Dict[str, str]) -> Dict[str, str]:
    """Add Swift-compatible keys based on mappings."""
    result = dict(translations)
    for json_key, swift_key in SWIFT_KEY_MAPPINGS.items():
        if json_key in translations and swift_key not in result:
            result[swift_key] = translations[json_key]
    return result

def load_all_locales() -> Dict[str, Dict[str, str]]:
    """Load all locale JSON files and return flattened dictionaries."""
    locales = {}
    for json_file in LOCALES_DIR.glob("*.json"):
        lang_code = json_file.stem  # e.g., "en", "de", "zh"
        with open(json_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
        # Skip meta section, flatten the rest
        flattened = {}
        for section, content in data.items():
            if section != "meta" and isinstance(content, dict):
                flattened.update(flatten_json(content, section))
        # Apply Swift key mappings for backwards compatibility
        flattened = apply_key_mappings(flattened)
        locales[lang_code] = flattened
    return locales

def generate_xcstrings(locales: Dict[str, Dict[str, str]]) -> Dict:
    """Generate Xcode .xcstrings format."""
    # Get all unique keys from English (source)
    if "en" not in locales:
        raise ValueError("English locale (en.json) is required as source")

    all_keys = set(locales["en"].keys())

    # Build xcstrings structure
    xcstrings = {
        "sourceLanguage": "en",
        "version": "1.0",
        "strings": {}
    }

    for key in sorted(all_keys):
        string_entry = {
            "extractionState": "manual",
            "localizations": {}
        }

        for lang_code, translations in locales.items():
            swift_code = SWIFT_LANG_CODES.get(lang_code, lang_code)
            if key in translations:
                string_entry["localizations"][swift_code] = {
                    "stringUnit": {
                        "state": "translated",
                        "value": translations[key]
                    }
                }

        xcstrings["strings"][key] = string_entry

    return xcstrings

def generate_swift(locales: Dict[str, Dict[str, str]]):
    """Generate Swift Localizable.xcstrings file."""
    print(f"Generating Swift localization: {SWIFT_OUTPUT}")

    xcstrings = generate_xcstrings(locales)

    # Ensure output directory exists
    SWIFT_OUTPUT.parent.mkdir(parents=True, exist_ok=True)

    with open(SWIFT_OUTPUT, 'w', encoding='utf-8') as f:
        json.dump(xcstrings, f, ensure_ascii=False, indent=2)

    key_count = len(xcstrings["strings"])
    lang_count = len(locales)
    print(f"  Generated {key_count} keys for {lang_count} languages")

    # Also generate Localization.swift for backwards compatibility
    generate_swift_localization_file(locales)

def generate_swift_localization_file(locales: Dict[str, Dict[str, str]]):
    """Generate Localization.swift with hardcoded dictionaries (backwards compatible)."""
    swift_file = SWIFT_OUTPUT.parent / "Localization.swift"
    print(f"  Generating {swift_file.name}")

    lines = [
        "// Auto-generated from locales/*.json - DO NOT EDIT MANUALLY",
        f"// Generated: {datetime.now().isoformat()}",
        "// Run: python scripts/generate-localizations.py --swift",
        "",
        "import Foundation",
        "",
        "/// Localization helper for the app",
        "struct L10n {",
        "    private static let translations: [String: [String: String]] = ["
    ]

    # Generate each language dictionary
    for lang_code in sorted(locales.keys()):
        swift_code = SWIFT_LANG_CODES.get(lang_code, lang_code)
        # Use simple code for dictionary key
        dict_key = lang_code
        translations = locales[lang_code]

        lines.append(f'        "{dict_key}": [')
        for key in sorted(translations.keys()):
            value = translations[key].replace('\\', '\\\\').replace('"', '\\"')
            lines.append(f'            "{key}": "{value}",')
        lines.append("        ],")

    lines.append("    ]")
    lines.append("")

    # Add helper methods
    lines.extend([
        "    /// Get localized string for a specific language",
        "    static func string(_ key: String, language: String) -> String {",
        '        return translations[language]?[key] ?? translations["en"]?[key] ?? key',
        "    }",
        "",
        "    /// Get current language from user preferences or system locale",
        "    static var currentLanguage: String {",
        '        // Check if user has set a specific language in preferences',
        '        let userDefault = UserDefaults.standard.string(forKey: "appLanguage") ?? "system"',
        '        if userDefault != "system" {',
        "            return userDefault",
        "        }",
        "        // Fall back to system language",
        '        let preferred = Locale.preferredLanguages.first ?? "en"',
        '        if preferred.hasPrefix("de") { return "de" }',
        '        if preferred.hasPrefix("zh") { return "zh" }',
        '        if preferred.hasPrefix("fr") { return "fr" }',
        '        if preferred.hasPrefix("it") { return "it" }',
        '        if preferred.hasPrefix("ru") { return "ru" }',
        '        if preferred.hasPrefix("es") { return "es" }',
        '        if preferred.hasPrefix("pt") { return "pt-BR" }',
        '        return "en"',
        "    }",
        "",
        "    /// Get localized string using current language",
        "    static func string(_ key: String) -> String {",
        "        return string(key, language: currentLanguage)",
        "    }",
        "}",
    ])

    with open(swift_file, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))

def escape_xml(text: str) -> str:
    """Escape special XML characters."""
    return (text
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("'", "\\'")
        .replace('"', '\\"'))

def generate_kotlin(locales: Dict[str, Dict[str, str]]):
    """Generate Android strings.xml files."""
    print(f"Generating Kotlin/Android localizations: {KOTLIN_OUTPUT_DIR}")

    if not KOTLIN_OUTPUT_DIR.exists():
        print(f"  Warning: Android output directory not found, skipping")
        return

    for lang_code, translations in locales.items():
        kotlin_code = KOTLIN_LANG_CODES.get(lang_code, lang_code)

        if kotlin_code:
            values_dir = KOTLIN_OUTPUT_DIR / f"values-{kotlin_code}"
        else:
            values_dir = KOTLIN_OUTPUT_DIR / "values"

        values_dir.mkdir(parents=True, exist_ok=True)
        output_file = values_dir / "strings.xml"

        # Generate XML
        lines = ['<?xml version="1.0" encoding="utf-8"?>']
        lines.append(f'<!-- Auto-generated from locales/{lang_code}.json - DO NOT EDIT -->')
        lines.append('<resources>')

        for key in sorted(translations.keys()):
            # Convert dot notation to underscore for Android
            android_key = key.replace(".", "_").replace("-", "_")
            value = escape_xml(translations[key])
            lines.append(f'    <string name="{android_key}">{value}</string>')

        lines.append('</resources>')

        with open(output_file, 'w', encoding='utf-8') as f:
            f.write('\n'.join(lines))

        print(f"  Generated {output_file.name} ({len(translations)} keys)")

def generate_harmonyos(locales: Dict[str, Dict[str, str]]):
    """Generate HarmonyOS string.json files."""
    print(f"Generating HarmonyOS localizations: {HARMONYOS_OUTPUT_DIR}")

    if not HARMONYOS_OUTPUT_DIR.exists():
        print(f"  Warning: HarmonyOS output directory not found, skipping")
        return

    # HarmonyOS uses different folder structure
    lang_folders = {
        "en": "base",  # default
        "de": "de_DE",
        "zh": "zh_CN",
        "fr": "fr_FR",
        "it": "it_IT",
        "ru": "ru_RU",
        "es": "es_ES",
        "pt-BR": "pt_BR"
    }

    for lang_code, translations in locales.items():
        folder_name = lang_folders.get(lang_code, lang_code)
        output_dir = HARMONYOS_OUTPUT_DIR / folder_name / "element"
        output_dir.mkdir(parents=True, exist_ok=True)
        output_file = output_dir / "string.json"

        # Generate HarmonyOS format
        harmony_strings = {
            "string": []
        }

        for key in sorted(translations.keys()):
            harmony_strings["string"].append({
                "name": key.replace(".", "_").replace("-", "_"),
                "value": translations[key]
            })

        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(harmony_strings, f, ensure_ascii=False, indent=2)

        print(f"  Generated {folder_name}/element/string.json ({len(translations)} keys)")

def main():
    args = sys.argv[1:]

    # Determine which platforms to generate
    generate_all = len(args) == 0
    do_swift = generate_all or "--swift" in args
    do_kotlin = generate_all or "--kotlin" in args
    do_harmonyos = generate_all or "--harmonyos" in args

    print(f"Loading locales from: {LOCALES_DIR}")
    locales = load_all_locales()
    print(f"Loaded {len(locales)} languages: {', '.join(sorted(locales.keys()))}")
    print()

    if do_swift:
        generate_swift(locales)
        print()

    if do_kotlin:
        generate_kotlin(locales)
        print()

    if do_harmonyos:
        generate_harmonyos(locales)
        print()

    print("Done!")

if __name__ == "__main__":
    main()
