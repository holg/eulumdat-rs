//! BIM parameters FFI types and functions

use crate::types::{to_core_eulumdat, Eulumdat};

/// A single BIM parameter row
#[derive(Debug, Clone, uniffi::Record)]
pub struct BimParameterRow {
    pub group: String,
    pub key: String,
    pub value: String,
    pub unit: String,
}

/// BIM data extracted from a luminaire
#[derive(Debug, Clone, uniffi::Record)]
pub struct BimData {
    pub populated_count: u32,
    pub summary: String,
    pub rows: Vec<BimParameterRow>,
    pub csv: String,
    pub text_report: String,
}

/// Extract BIM parameters from a luminaire
#[uniffi::export]
pub fn get_bim_parameters(ldt: &Eulumdat) -> BimData {
    let core_ldt = to_core_eulumdat(ldt);
    let doc = atla::LuminaireOpticalData::from_eulumdat(&core_ldt);
    let bim = atla::bim::BimParameters::from_atla(&doc);

    let rows: Vec<BimParameterRow> = bim
        .to_table_rows()
        .into_iter()
        .map(|(group, key, value, unit)| BimParameterRow {
            group: group.to_string(),
            key: key.to_string(),
            value,
            unit: unit.to_string(),
        })
        .collect();

    let populated_count = rows.len() as u32;
    let summary = format!("{} parameters populated", populated_count);

    BimData {
        populated_count,
        summary,
        rows,
        csv: bim.to_csv(),
        text_report: bim.to_text_report(),
    }
}

/// Check if luminaire has any BIM data
#[uniffi::export]
pub fn has_bim_data(ldt: &Eulumdat) -> bool {
    let core_ldt = to_core_eulumdat(ldt);
    let doc = atla::LuminaireOpticalData::from_eulumdat(&core_ldt);
    let bim = atla::bim::BimParameters::from_atla(&doc);
    bim.populated_count() > 0
}
