//! Luminaire templates loaded from embedded LDT and ATLA XML files
//!
//! Templates are copied from EulumdatApp/Resources/Templates during build.

use atla::LuminaireOpticalData;
use eulumdat::Eulumdat;

/// Template format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFormat {
    Ldt,
    AtlaXml,
}

/// Template metadata and content
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub format: TemplateFormat,
    content: &'static str,
}

/// All available templates (embedded at compile time)
pub static TEMPLATES: &[Template] = &[
    // === Standard LDT templates ===
    Template {
        id: "downlight",
        name: "Downlight",
        description: "Simple downlight with vertical axis symmetry (Isym=1)",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/1-1-0.ldt")),
    },
    Template {
        id: "linear",
        name: "Linear Luminaire",
        description: "Linear luminaire with C0-C180 plane symmetry (Isym=2)",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/0-2-0.ldt")),
    },
    Template {
        id: "fluorescent",
        name: "Fluorescent",
        description: "T16 G5 54W linear fluorescent with bilateral symmetry",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/fluorescent_luminaire.ldt"
        )),
    },
    Template {
        id: "projector",
        name: "Projector",
        description: "CDM-TD 70W spotlight with asymmetric distribution",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/projector.ldt")),
    },
    Template {
        id: "road",
        name: "Road Luminaire",
        description: "SON-TPP 250W street light with forward throw",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/road_luminaire.ldt")),
    },
    Template {
        id: "uplight",
        name: "Floor Uplight",
        description: "HIT-DE 250W decorative floor uplight",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/floor_uplight.ldt")),
    },
    // === Wikipedia example templates ===
    Template {
        id: "wiki-batwing",
        name: "Batwing (Wiki)",
        description: "Batwing distribution for uniform illuminance",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/wiki-batwing.ldt")),
    },
    Template {
        id: "wiki-spotlight",
        name: "Spotlight (Wiki)",
        description: "Narrow beam spotlight distribution",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/wiki-spotlight.ldt")),
    },
    Template {
        id: "wiki-flood",
        name: "Floodlight (Wiki)",
        description: "Wide beam flood distribution",
        format: TemplateFormat::Ldt,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/wiki-flood.ldt")),
    },
    // === ATLA templates with spectral data ===
    Template {
        id: "atla-grow-light",
        name: "Grow Light (ATLA)",
        description: "LED grow light with red/blue PPF spectrum",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/_atla_grow_light.xml")),
    },
    Template {
        id: "atla-grow-light-rb",
        name: "Grow Light R/B (ATLA)",
        description: "Red/blue LED grow light for plants",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/_atla_grow_light_rb.xml"
        )),
    },
    Template {
        id: "atla-fluorescent",
        name: "Fluorescent (ATLA)",
        description: "Fluorescent lamp with spectral data",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/_atla_fluorescent.xml")),
    },
    Template {
        id: "atla-halogen",
        name: "Halogen (ATLA)",
        description: "Halogen lamp with continuous spectrum",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/_atla_halogen_lamp.xml"
        )),
    },
    Template {
        id: "atla-incandescent",
        name: "Incandescent (ATLA)",
        description: "Incandescent lamp with warm spectrum",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/_atla_incandescent.xml"
        )),
    },
    Template {
        id: "atla-heat-lamp",
        name: "Heat Lamp (ATLA)",
        description: "Infrared heat lamp with IR spectrum",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(env!("OUT_DIR"), "/templates/_atla_heat_lamp.xml")),
    },
    Template {
        id: "atla-uv-blacklight",
        name: "UV Blacklight (ATLA)",
        description: "UV blacklight with UV-A spectrum",
        format: TemplateFormat::AtlaXml,
        content: include_str!(concat!(
            env!("OUT_DIR"),
            "/templates/_atla_uv_blacklight.xml"
        )),
    },
];

impl Template {
    /// Parse the template content into an Eulumdat struct
    pub fn parse(&self) -> Result<Eulumdat, String> {
        match self.format {
            TemplateFormat::Ldt => Eulumdat::parse(self.content).map_err(|e| e.to_string()),
            TemplateFormat::AtlaXml => {
                let atla =
                    atla::xml::parse(self.content).map_err(|e| format!("ATLA XML error: {}", e))?;
                Ok(atla.to_eulumdat())
            }
        }
    }

    /// Parse the template content into an ATLA LuminaireOpticalData struct
    pub fn parse_atla(&self) -> Result<LuminaireOpticalData, String> {
        match self.format {
            TemplateFormat::Ldt => {
                let ldt = Eulumdat::parse(self.content).map_err(|e| e.to_string())?;
                Ok(LuminaireOpticalData::from_eulumdat(&ldt))
            }
            TemplateFormat::AtlaXml => {
                atla::xml::parse(self.content).map_err(|e| format!("ATLA XML error: {}", e))
            }
        }
    }

    /// Check if this template has spectral data
    pub fn has_spectral_data(&self) -> bool {
        self.format == TemplateFormat::AtlaXml
    }
}

/// Get all templates
pub fn all_templates() -> &'static [Template] {
    TEMPLATES
}
