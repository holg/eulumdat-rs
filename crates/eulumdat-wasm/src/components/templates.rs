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

/// All available templates
pub const ALL_TEMPLATES: &[&Template] = &[
    &DOWNLIGHT,
    &PROJECTOR,
    &LINEAR,
    &FLUORESCENT,
    &ROAD_LUMINAIRE,
    &FLOOR_UPLIGHT,
    &ATLA_FLUORESCENT_XML,
    &ATLA_FLUORESCENT_JSON,
    &ATLA_GROW_LIGHT_FS,
    &ATLA_GROW_LIGHT_RB,
    &ATLA_HALOGEN_LAMP,
    &ATLA_INCANDESCENT,
    &ATLA_HEAT_LAMP,
    &ATLA_UV_BLACKLIGHT,
];
