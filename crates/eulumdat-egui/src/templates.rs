//! Luminaire templates loaded from embedded LDT files
//!
//! Templates are copied from EulumdatApp/Resources/Templates during build.

use eulumdat::Eulumdat;

/// Template metadata and content
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    content: &'static str,
}

/// All available templates (embedded at compile time)
pub static TEMPLATES: &[Template] = &[
    Template {
        id: "downlight",
        name: "Downlight",
        description: "Simple downlight with vertical axis symmetry (Isym=1)",
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/1-1-0.ldt")),
    },
    Template {
        id: "linear",
        name: "Linear Luminaire",
        description: "Linear luminaire with C0-C180 plane symmetry (Isym=2)",
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/0-2-0.ldt")),
    },
    Template {
        id: "fluorescent",
        name: "Fluorescent",
        description: "T16 G5 54W linear fluorescent with bilateral symmetry",
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/fluorescent_luminaire.ldt"
        )),
    },
    Template {
        id: "projector",
        name: "Projector",
        description: "CDM-TD 70W spotlight with asymmetric distribution",
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/projector.ldt")),
    },
    Template {
        id: "road",
        name: "Road Luminaire",
        description: "SON-TPP 250W street light with forward throw",
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/road_luminaire.ldt")),
    },
    Template {
        id: "uplight",
        name: "Floor Uplight",
        description: "HIT-DE 250W decorative floor uplight",
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/floor_uplight.ldt")),
    },
];

impl Template {
    /// Parse the template content into an Eulumdat struct
    pub fn parse(&self) -> Result<Eulumdat, String> {
        Eulumdat::parse(self.content).map_err(|e| e.to_string())
    }
}

/// Get all templates
pub fn all_templates() -> &'static [Template] {
    TEMPLATES
}
