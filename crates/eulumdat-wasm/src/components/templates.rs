//! Built-in LDT templates from QLumEditWasm

/// Template definition with name and content
pub struct Template {
    pub name: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

/// Projector/spotlight template
pub const PROJECTOR: Template = Template {
    name: "Projector",
    description: "CDM-TD 70W spotlight with asymmetric beam",
    content: include_str!("../../templates/projector.ldt"),
};

/// Fluorescent luminaire template
pub const FLUORESCENT: Template = Template {
    name: "Fluorescent Luminaire",
    description: "T16 G5 54W linear luminaire with bilateral symmetry",
    content: include_str!("../../templates/fluorescent_luminaire.ldt"),
};

/// Road luminaire template
pub const ROAD_LUMINAIRE: Template = Template {
    name: "Road Luminaire",
    description: "SON-TPP 250W street light with C90-C270 symmetry",
    content: include_str!("../../templates/road_luminaire.ldt"),
};

/// Floor uplight template
pub const FLOOR_UPLIGHT: Template = Template {
    name: "Floor Uplight",
    description: "HIT-DE 250W floor-standing uplight",
    content: include_str!("../../templates/floor_uplight.ldt"),
};

/// Downlight (vertical symmetry) template
pub const DOWNLIGHT: Template = Template {
    name: "Downlight",
    description: "Simple downlight with vertical axis symmetry",
    content: include_str!("../../templates/1-1-0.ldt"),
};

/// Linear luminaire template
pub const LINEAR: Template = Template {
    name: "Linear Luminaire",
    description: "Linear luminaire with C0-C180 symmetry",
    content: include_str!("../../templates/0-2-0.ldt"),
};

/// All available templates
pub const ALL_TEMPLATES: &[&Template] = &[
    &DOWNLIGHT,
    &PROJECTOR,
    &LINEAR,
    &FLUORESCENT,
    &ROAD_LUMINAIRE,
    &FLOOR_UPLIGHT,
];
