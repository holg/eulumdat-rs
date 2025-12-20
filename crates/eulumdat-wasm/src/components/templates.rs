//! Built-in templates for photometric files

/// Template format
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemplateFormat {
    Ldt,
    AtlaXml,
    AtlaJson,
}

/// Template definition with name, format, and content
pub struct Template {
    pub name: &'static str,
    pub description: &'static str,
    pub format: TemplateFormat,
    pub content: &'static str,
}

// === LDT Templates ===

/// Projector/spotlight template
pub const PROJECTOR: Template = Template {
    name: "Projector",
    description: "CDM-TD 70W spotlight with asymmetric beam",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/projector.ldt"),
};

/// Fluorescent luminaire template
pub const FLUORESCENT: Template = Template {
    name: "Fluorescent Luminaire",
    description: "T16 G5 54W linear luminaire with bilateral symmetry",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/fluorescent_luminaire.ldt"),
};

/// Road luminaire template
pub const ROAD_LUMINAIRE: Template = Template {
    name: "Road Luminaire",
    description: "SON-TPP 250W street light with C90-C270 symmetry",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/road_luminaire.ldt"),
};

/// Floor uplight template
pub const FLOOR_UPLIGHT: Template = Template {
    name: "Floor Uplight",
    description: "HIT-DE 250W floor-standing uplight",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/floor_uplight.ldt"),
};

/// Downlight (vertical symmetry) template
pub const DOWNLIGHT: Template = Template {
    name: "Downlight",
    description: "Simple downlight with vertical axis symmetry",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/1-1-0.ldt"),
};

/// Linear luminaire template
pub const LINEAR: Template = Template {
    name: "Linear Luminaire",
    description: "Linear luminaire with C0-C180 symmetry",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/0-2-0.ldt"),
};

// === Wikipedia Beam Angle Demo Templates ===

/// Batwing distribution - shows IES vs CIE beam angle difference
pub const WIKI_BATWING: Template = Template {
    name: "Wiki: Batwing (IES vs CIE)",
    description: "Batwing distribution demonstrating IES vs CIE beam angle difference",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/wiki-batwing.ldt"),
};

/// Narrow spotlight - standard center-peak distribution
pub const WIKI_SPOTLIGHT: Template = Template {
    name: "Wiki: Spotlight (30°)",
    description: "Narrow 30° beam spotlight with center-peak distribution",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/wiki-spotlight.ldt"),
};

/// Wide flood - cosine distribution
pub const WIKI_FLOOD: Template = Template {
    name: "Wiki: Flood (120°)",
    description: "Wide flood with ~60° beam and ~120° field angle",
    format: TemplateFormat::Ldt,
    content: include_str!("../../templates/wiki-flood.ldt"),
};

// === ATLA Templates ===

/// ATLA XML fluorescent template
pub const ATLA_FLUORESCENT_XML: Template = Template {
    name: "_atla Fluorescent (XML)",
    description: "ATLA S001/TM-33 format - T16 G5 fluorescent with full metadata",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_fluorescent.xml"),
};

/// ATLA JSON fluorescent template
pub const ATLA_FLUORESCENT_JSON: Template = Template {
    name: "_atla Fluorescent (JSON)",
    description: "ATLA S001-A JSON format - T16 G5 fluorescent with full metadata",
    format: TemplateFormat::AtlaJson,
    content: include_str!("../../templates/_atla_fluorescent.json"),
};

/// ATLA Full Spectrum Grow Light with spectral data
pub const ATLA_GROW_LIGHT_FS: Template = Template {
    name: "_atla Grow Light (Full Spectrum)",
    description: "600W horticultural LED with PAR-optimized spectrum + spectral data",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_grow_light.xml"),
};

/// ATLA Red/Blue Grow Light with spectral data
pub const ATLA_GROW_LIGHT_RB: Template = Template {
    name: "_atla Grow Light (Red/Blue)",
    description: "200W red/blue LED grow light with spectral data - high PAR efficiency",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_grow_light_rb.xml"),
};

/// ATLA Halogen lamp with IR spectral data
pub const ATLA_HALOGEN_LAMP: Template = Template {
    name: "_atla Halogen Lamp (IR)",
    description: "500W halogen flood - extended spectrum to 1000nm with ~50% near-IR",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_halogen_lamp.xml"),
};

/// ATLA Incandescent bulb with IR spectral data
pub const ATLA_INCANDESCENT: Template = Template {
    name: "_atla Incandescent (IR)",
    description: "100W A19 incandescent - Planckian spectrum to 1100nm",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_incandescent.xml"),
};

/// ATLA Heat lamp with high IR content
pub const ATLA_HEAT_LAMP: Template = Template {
    name: "_atla Heat Lamp (High IR)",
    description: "250W infrared heat lamp - 92% near-IR, triggers thermal warning",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_heat_lamp.xml"),
};

/// ATLA UV blacklight with UV-A spectral data
pub const ATLA_UV_BLACKLIGHT: Template = Template {
    name: "_atla UV Blacklight (UV-A)",
    description: "18W UV-A blacklight - spectrum from 315nm, triggers UV warning",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/_atla_uv_blacklight.xml"),
};

// === TM-33-23 Templates (Horticultural) ===

/// TM-33-23 Minimal valid document
pub const TM33_MINIMAL: Template = Template {
    name: "tm-33-23 Minimal",
    description: "Minimal valid TM-33-23 document with all required fields",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_minimal.xml"),
};

/// TM-33-23 with CustomData
pub const TM33_CUSTOM_DATA: Template = Template {
    name: "tm-33-23 Custom Data",
    description: "TM-33-23 with multiple CustomData blocks and extended fields",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_with_custom_data.xml"),
};

/// TM-33-23 Full spectrum horticultural LED
pub const TM33_HORT_LED: Template = Template {
    name: "tm-33-23 Horticultural LED",
    description: "600W full spectrum LED panel with PPFD metrics and spectral data",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_horticultural_led.xml"),
};

/// TM-33-23 Far-red supplemental LED
pub const TM33_FAR_RED: Template = Template {
    name: "tm-33-23 Far-Red (730nm)",
    description: "120W far-red supplemental for flowering enhancement",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_far_red_supplemental.xml"),
};

/// TM-33-23 UV supplemental LED
pub const TM33_UV: Template = Template {
    name: "tm-33-23 UV-A/B Supplemental",
    description: "60W UV-A/UV-B for secondary metabolite enhancement",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_uv_supplemental.xml"),
};

/// TM-33-23 Seedling propagation LED
pub const TM33_SEEDLING: Template = Template {
    name: "tm-33-23 Seedling/Clone",
    description: "200W high-blue LED for seedling and clone propagation",
    format: TemplateFormat::AtlaXml,
    content: include_str!("../../templates/tm-33-23_seedling_propagation.xml"),
};

/// All available templates
pub const ALL_TEMPLATES: &[&Template] = &[
    // Wikipedia beam angle demos (put first for visibility)
    &WIKI_BATWING,
    &WIKI_SPOTLIGHT,
    &WIKI_FLOOD,
    // Standard LDT templates
    &DOWNLIGHT,
    &PROJECTOR,
    &LINEAR,
    &FLUORESCENT,
    &ROAD_LUMINAIRE,
    &FLOOR_UPLIGHT,
    // ATLA S001 format templates
    &ATLA_FLUORESCENT_XML,
    &ATLA_FLUORESCENT_JSON,
    &ATLA_GROW_LIGHT_FS,
    &ATLA_GROW_LIGHT_RB,
    &ATLA_HALOGEN_LAMP,
    &ATLA_INCANDESCENT,
    &ATLA_HEAT_LAMP,
    &ATLA_UV_BLACKLIGHT,
    // TM-33-23 format templates (horticultural)
    &TM33_MINIMAL,
    &TM33_CUSTOM_DATA,
    &TM33_HORT_LED,
    &TM33_FAR_RED,
    &TM33_UV,
    &TM33_SEEDLING,
];
