use arrow::datatypes::{DataType, Field, Fields, Schema};
use std::sync::Arc;

/// Build the Arrow schema used to write eulumdat data.
///
/// Columns are grouped into: identity, classification, grid, dimensions,
/// optical scalars, direct_ratios, lamp_sets, and (behind feature flags)
/// computed summary metrics and raw photometry.
#[allow(clippy::vec_init_then_push)]
pub fn build_schema() -> Arc<Schema> {
    let mut fields: Vec<Field> = Vec::new();

    // ── Identity ───────────────────────────────────────────────────────────
    fields.push(Field::new("file_path", DataType::Utf8, true));
    fields.push(Field::new("identification", DataType::Utf8, false));
    fields.push(Field::new("luminaire_name", DataType::Utf8, false));
    fields.push(Field::new("luminaire_number", DataType::Utf8, false));
    fields.push(Field::new("file_name", DataType::Utf8, false));
    fields.push(Field::new("date_user", DataType::Utf8, false));
    fields.push(Field::new(
        "measurement_report_number",
        DataType::Utf8,
        false,
    ));

    // ── Classification ─────────────────────────────────────────────────────
    fields.push(Field::new("type_indicator", DataType::Utf8, false));
    fields.push(Field::new("symmetry", DataType::Utf8, false));

    // ── Grid ───────────────────────────────────────────────────────────────
    fields.push(Field::new("num_c_planes", DataType::UInt32, false));
    fields.push(Field::new("c_plane_distance", DataType::Float64, false));
    fields.push(Field::new("num_g_planes", DataType::UInt32, false));
    fields.push(Field::new("g_plane_distance", DataType::Float64, false));

    // ── Dimensions (mm) ────────────────────────────────────────────────────
    for name in [
        "length",
        "width",
        "height",
        "luminous_area_length",
        "luminous_area_width",
        "height_c0",
        "height_c90",
        "height_c180",
        "height_c270",
    ] {
        fields.push(Field::new(name, DataType::Float64, false));
    }

    // ── Optical scalars ────────────────────────────────────────────────────
    for name in [
        "downward_flux_fraction",
        "light_output_ratio",
        "conversion_factor",
        "tilt_angle",
    ] {
        fields.push(Field::new(name, DataType::Float64, false));
    }

    // ── direct_ratios: list<double> (always 10 elements) ───────────────────
    fields.push(Field::new(
        "direct_ratios",
        DataType::List(Arc::new(Field::new("item", DataType::Float64, true))),
        false,
    ));

    // ── lamp_sets: list<struct{...}> ───────────────────────────────────────
    fields.push(Field::new(
        "lamp_sets",
        DataType::List(Arc::new(Field::new(
            "item",
            DataType::Struct(lamp_set_fields()),
            true,
        ))),
        false,
    ));

    // ── Summary (feature-gated) ────────────────────────────────────────────
    #[cfg(feature = "summary")]
    {
        for name in [
            "total_lamp_flux",
            "calculated_flux",
            "lor",
            "dlor",
            "ulor",
            "lamp_efficacy",
            "luminaire_efficacy",
            "total_wattage",
            "beam_angle",
            "field_angle",
            "beam_angle_cie",
            "field_angle_cie",
            "upward_beam_angle",
            "upward_field_angle",
            "max_intensity",
            "min_intensity",
            "avg_intensity",
            "spacing_c0",
            "spacing_c90",
        ] {
            fields.push(Field::new(name, DataType::Float64, false));
        }
        fields.push(Field::new("is_batwing", DataType::Boolean, false));
        fields.push(Field::new("primary_direction", DataType::Utf8, false));
        fields.push(Field::new("distribution_type", DataType::Utf8, false));
    }

    // ── Raw photometry (opt-in, large) ─────────────────────────────────────
    #[cfg(feature = "raw-photometry")]
    {
        fields.push(Field::new(
            "c_angles",
            DataType::List(Arc::new(Field::new("item", DataType::Float64, true))),
            false,
        ));
        fields.push(Field::new(
            "g_angles",
            DataType::List(Arc::new(Field::new("item", DataType::Float64, true))),
            false,
        ));
        fields.push(Field::new(
            "intensities",
            DataType::List(Arc::new(Field::new(
                "item",
                DataType::List(Arc::new(Field::new("item", DataType::Float64, true))),
                true,
            ))),
            false,
        ));
    }

    Arc::new(Schema::new(fields))
}

fn lamp_set_fields() -> Fields {
    Fields::from(vec![
        Field::new("num_lamps", DataType::Int32, false),
        Field::new("lamp_type", DataType::Utf8, false),
        Field::new("total_luminous_flux", DataType::Float64, false),
        Field::new("color_appearance", DataType::Utf8, false),
        Field::new("color_rendering_group", DataType::Utf8, false),
        Field::new("wattage_with_ballast", DataType::Float64, false),
    ])
}
