use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, Int32Builder, ListBuilder, RecordBatch,
    StringBuilder, StructBuilder, UInt32Builder,
};
use arrow::datatypes::{DataType, Field, Schema};
use eulumdat::Eulumdat;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

use crate::schema::build_schema;

/// Streaming Parquet writer for a collection of Eulumdat records.
///
/// Usage:
/// ```no_run
/// # use eulumdat::Eulumdat;
/// # use eulumdat_parquet::EulumdatParquetWriter;
/// # fn main() -> anyhow::Result<()> {
/// let mut w = EulumdatParquetWriter::create("catalog.parquet")?;
/// for path in std::fs::read_dir("catalog")? {
///     let path = path?.path();
///     if path.extension().and_then(|s| s.to_str()) != Some("ldt") { continue; }
///     let ldt = Eulumdat::from_file(&path)?;
///     w.append(path.to_string_lossy().as_ref(), &ldt)?;
/// }
/// w.finish()?;
/// # Ok(()) }
/// ```
pub struct EulumdatParquetWriter {
    writer: ArrowWriter<File>,
    schema: Arc<Schema>,
    builders: RowBuilders,
    batch_size: usize,
}

impl EulumdatParquetWriter {
    /// Create a new Parquet file at `path` with the default schema.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file =
            File::create(path.as_ref()).with_context(|| format!("creating {:?}", path.as_ref()))?;
        let schema = build_schema();
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::default()))
            .build();
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))
            .context("creating Parquet ArrowWriter")?;
        Ok(Self {
            writer,
            schema: schema.clone(),
            builders: RowBuilders::new(&schema),
            batch_size: 256,
        })
    }

    /// Append one Eulumdat record as a row.
    ///
    /// `file_path` is stored in the `file_path` column (for traceability);
    /// pass an empty string if not applicable.
    pub fn append(&mut self, file_path: &str, ldt: &Eulumdat) -> Result<()> {
        self.builders.push(file_path, ldt);
        if self.builders.rows_buffered() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    /// Flush buffered rows as a RecordBatch.
    fn flush(&mut self) -> Result<()> {
        if self.builders.rows_buffered() == 0 {
            return Ok(());
        }
        let batch = self.builders.finish_batch(&self.schema)?;
        self.writer.write(&batch).context("writing RecordBatch")?;
        Ok(())
    }

    /// Flush remaining rows and close the file.
    pub fn finish(mut self) -> Result<()> {
        self.flush()?;
        self.writer.close().context("closing Parquet writer")?;
        Ok(())
    }
}

/// Array builders for one RecordBatch worth of rows.
///
/// The field order here must match `build_schema` exactly.
struct RowBuilders {
    rows: usize,

    // Identity
    file_path: StringBuilder,
    identification: StringBuilder,
    luminaire_name: StringBuilder,
    luminaire_number: StringBuilder,
    file_name: StringBuilder,
    date_user: StringBuilder,
    measurement_report_number: StringBuilder,

    // Classification
    type_indicator: StringBuilder,
    symmetry: StringBuilder,

    // Grid
    num_c_planes: UInt32Builder,
    c_plane_distance: Float64Builder,
    num_g_planes: UInt32Builder,
    g_plane_distance: Float64Builder,

    // Dimensions
    length: Float64Builder,
    width: Float64Builder,
    height: Float64Builder,
    luminous_area_length: Float64Builder,
    luminous_area_width: Float64Builder,
    height_c0: Float64Builder,
    height_c90: Float64Builder,
    height_c180: Float64Builder,
    height_c270: Float64Builder,

    // Optical
    downward_flux_fraction: Float64Builder,
    light_output_ratio: Float64Builder,
    conversion_factor: Float64Builder,
    tilt_angle: Float64Builder,

    // direct_ratios (list<double>)
    direct_ratios: ListBuilder<Float64Builder>,

    // lamp_sets (list<struct>)
    lamp_sets: ListBuilder<StructBuilder>,

    // ── Summary ────────────────────────────────────────────────────────────
    #[cfg(feature = "summary")]
    summary: SummaryBuilders,

    // ── Raw photometry ─────────────────────────────────────────────────────
    #[cfg(feature = "raw-photometry")]
    c_angles: ListBuilder<Float64Builder>,
    #[cfg(feature = "raw-photometry")]
    g_angles: ListBuilder<Float64Builder>,
    #[cfg(feature = "raw-photometry")]
    intensities: ListBuilder<ListBuilder<Float64Builder>>,
}

#[cfg(feature = "summary")]
struct SummaryBuilders {
    total_lamp_flux: Float64Builder,
    calculated_flux: Float64Builder,
    lor: Float64Builder,
    dlor: Float64Builder,
    ulor: Float64Builder,
    lamp_efficacy: Float64Builder,
    luminaire_efficacy: Float64Builder,
    total_wattage: Float64Builder,
    beam_angle: Float64Builder,
    field_angle: Float64Builder,
    beam_angle_cie: Float64Builder,
    field_angle_cie: Float64Builder,
    upward_beam_angle: Float64Builder,
    upward_field_angle: Float64Builder,
    max_intensity: Float64Builder,
    min_intensity: Float64Builder,
    avg_intensity: Float64Builder,
    spacing_c0: Float64Builder,
    spacing_c90: Float64Builder,
    is_batwing: BooleanBuilder,
    primary_direction: StringBuilder,
    distribution_type: StringBuilder,
}

#[cfg(feature = "summary")]
impl SummaryBuilders {
    fn new() -> Self {
        Self {
            total_lamp_flux: Float64Builder::new(),
            calculated_flux: Float64Builder::new(),
            lor: Float64Builder::new(),
            dlor: Float64Builder::new(),
            ulor: Float64Builder::new(),
            lamp_efficacy: Float64Builder::new(),
            luminaire_efficacy: Float64Builder::new(),
            total_wattage: Float64Builder::new(),
            beam_angle: Float64Builder::new(),
            field_angle: Float64Builder::new(),
            beam_angle_cie: Float64Builder::new(),
            field_angle_cie: Float64Builder::new(),
            upward_beam_angle: Float64Builder::new(),
            upward_field_angle: Float64Builder::new(),
            max_intensity: Float64Builder::new(),
            min_intensity: Float64Builder::new(),
            avg_intensity: Float64Builder::new(),
            spacing_c0: Float64Builder::new(),
            spacing_c90: Float64Builder::new(),
            is_batwing: BooleanBuilder::new(),
            primary_direction: StringBuilder::new(),
            distribution_type: StringBuilder::new(),
        }
    }

    fn push(&mut self, ldt: &Eulumdat) {
        let s = eulumdat::PhotometricSummary::from_eulumdat(ldt);
        self.total_lamp_flux.append_value(s.total_lamp_flux);
        self.calculated_flux.append_value(s.calculated_flux);
        self.lor.append_value(s.lor);
        self.dlor.append_value(s.dlor);
        self.ulor.append_value(s.ulor);
        self.lamp_efficacy.append_value(s.lamp_efficacy);
        self.luminaire_efficacy.append_value(s.luminaire_efficacy);
        self.total_wattage.append_value(s.total_wattage);
        self.beam_angle.append_value(s.beam_angle);
        self.field_angle.append_value(s.field_angle);
        self.beam_angle_cie.append_value(s.beam_angle_cie);
        self.field_angle_cie.append_value(s.field_angle_cie);
        self.upward_beam_angle.append_value(s.upward_beam_angle);
        self.upward_field_angle.append_value(s.upward_field_angle);
        self.max_intensity.append_value(s.max_intensity);
        self.min_intensity.append_value(s.min_intensity);
        self.avg_intensity.append_value(s.avg_intensity);
        self.spacing_c0.append_value(s.spacing_c0);
        self.spacing_c90.append_value(s.spacing_c90);
        self.is_batwing.append_value(s.is_batwing);
        self.primary_direction
            .append_value(format!("{:?}", s.primary_direction));
        self.distribution_type
            .append_value(format!("{:?}", s.distribution_type));
    }

    fn finish(&mut self) -> Vec<ArrayRef> {
        vec![
            Arc::new(self.total_lamp_flux.finish()) as ArrayRef,
            Arc::new(self.calculated_flux.finish()),
            Arc::new(self.lor.finish()),
            Arc::new(self.dlor.finish()),
            Arc::new(self.ulor.finish()),
            Arc::new(self.lamp_efficacy.finish()),
            Arc::new(self.luminaire_efficacy.finish()),
            Arc::new(self.total_wattage.finish()),
            Arc::new(self.beam_angle.finish()),
            Arc::new(self.field_angle.finish()),
            Arc::new(self.beam_angle_cie.finish()),
            Arc::new(self.field_angle_cie.finish()),
            Arc::new(self.upward_beam_angle.finish()),
            Arc::new(self.upward_field_angle.finish()),
            Arc::new(self.max_intensity.finish()),
            Arc::new(self.min_intensity.finish()),
            Arc::new(self.avg_intensity.finish()),
            Arc::new(self.spacing_c0.finish()),
            Arc::new(self.spacing_c90.finish()),
            Arc::new(self.is_batwing.finish()),
            Arc::new(self.primary_direction.finish()),
            Arc::new(self.distribution_type.finish()),
        ]
    }
}

impl RowBuilders {
    fn new(_schema: &Schema) -> Self {
        Self {
            rows: 0,
            file_path: StringBuilder::new(),
            identification: StringBuilder::new(),
            luminaire_name: StringBuilder::new(),
            luminaire_number: StringBuilder::new(),
            file_name: StringBuilder::new(),
            date_user: StringBuilder::new(),
            measurement_report_number: StringBuilder::new(),
            type_indicator: StringBuilder::new(),
            symmetry: StringBuilder::new(),
            num_c_planes: UInt32Builder::new(),
            c_plane_distance: Float64Builder::new(),
            num_g_planes: UInt32Builder::new(),
            g_plane_distance: Float64Builder::new(),
            length: Float64Builder::new(),
            width: Float64Builder::new(),
            height: Float64Builder::new(),
            luminous_area_length: Float64Builder::new(),
            luminous_area_width: Float64Builder::new(),
            height_c0: Float64Builder::new(),
            height_c90: Float64Builder::new(),
            height_c180: Float64Builder::new(),
            height_c270: Float64Builder::new(),
            downward_flux_fraction: Float64Builder::new(),
            light_output_ratio: Float64Builder::new(),
            conversion_factor: Float64Builder::new(),
            tilt_angle: Float64Builder::new(),
            direct_ratios: ListBuilder::new(Float64Builder::new()),
            lamp_sets: lamp_sets_builder(),
            #[cfg(feature = "summary")]
            summary: SummaryBuilders::new(),
            #[cfg(feature = "raw-photometry")]
            c_angles: ListBuilder::new(Float64Builder::new()),
            #[cfg(feature = "raw-photometry")]
            g_angles: ListBuilder::new(Float64Builder::new()),
            #[cfg(feature = "raw-photometry")]
            intensities: ListBuilder::new(ListBuilder::new(Float64Builder::new())),
        }
    }

    fn rows_buffered(&self) -> usize {
        self.rows
    }

    fn push(&mut self, file_path: &str, ldt: &Eulumdat) {
        self.file_path.append_value(file_path);
        self.identification.append_value(&ldt.identification);
        self.luminaire_name.append_value(&ldt.luminaire_name);
        self.luminaire_number.append_value(&ldt.luminaire_number);
        self.file_name.append_value(&ldt.file_name);
        self.date_user.append_value(&ldt.date_user);
        self.measurement_report_number
            .append_value(&ldt.measurement_report_number);

        self.type_indicator
            .append_value(format!("{:?}", ldt.type_indicator));
        self.symmetry.append_value(format!("{:?}", ldt.symmetry));

        self.num_c_planes.append_value(ldt.num_c_planes as u32);
        self.c_plane_distance.append_value(ldt.c_plane_distance);
        self.num_g_planes.append_value(ldt.num_g_planes as u32);
        self.g_plane_distance.append_value(ldt.g_plane_distance);

        self.length.append_value(ldt.length);
        self.width.append_value(ldt.width);
        self.height.append_value(ldt.height);
        self.luminous_area_length
            .append_value(ldt.luminous_area_length);
        self.luminous_area_width
            .append_value(ldt.luminous_area_width);
        self.height_c0.append_value(ldt.height_c0);
        self.height_c90.append_value(ldt.height_c90);
        self.height_c180.append_value(ldt.height_c180);
        self.height_c270.append_value(ldt.height_c270);

        self.downward_flux_fraction
            .append_value(ldt.downward_flux_fraction);
        self.light_output_ratio.append_value(ldt.light_output_ratio);
        self.conversion_factor.append_value(ldt.conversion_factor);
        self.tilt_angle.append_value(ldt.tilt_angle);

        // direct_ratios: fixed 10-element list
        for v in &ldt.direct_ratios {
            self.direct_ratios.values().append_value(*v);
        }
        self.direct_ratios.append(true);

        // lamp_sets: list<struct>
        let struct_builder = self.lamp_sets.values();
        for lamp in &ldt.lamp_sets {
            struct_builder
                .field_builder::<Int32Builder>(0)
                .unwrap()
                .append_value(lamp.num_lamps);
            struct_builder
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_value(&lamp.lamp_type);
            struct_builder
                .field_builder::<Float64Builder>(2)
                .unwrap()
                .append_value(lamp.total_luminous_flux);
            struct_builder
                .field_builder::<StringBuilder>(3)
                .unwrap()
                .append_value(&lamp.color_appearance);
            struct_builder
                .field_builder::<StringBuilder>(4)
                .unwrap()
                .append_value(&lamp.color_rendering_group);
            struct_builder
                .field_builder::<Float64Builder>(5)
                .unwrap()
                .append_value(lamp.wattage_with_ballast);
            struct_builder.append(true);
        }
        self.lamp_sets.append(true);

        #[cfg(feature = "summary")]
        self.summary.push(ldt);

        #[cfg(feature = "raw-photometry")]
        {
            for v in &ldt.c_angles {
                self.c_angles.values().append_value(*v);
            }
            self.c_angles.append(true);

            for v in &ldt.g_angles {
                self.g_angles.values().append_value(*v);
            }
            self.g_angles.append(true);

            for c_plane in &ldt.intensities {
                let inner = self.intensities.values();
                for v in c_plane {
                    inner.values().append_value(*v);
                }
                inner.append(true);
            }
            self.intensities.append(true);
        }

        self.rows += 1;
    }

    fn finish_batch(&mut self, schema: &Arc<Schema>) -> Result<RecordBatch> {
        let mut arrays: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
        arrays.push(Arc::new(self.file_path.finish()));
        arrays.push(Arc::new(self.identification.finish()));
        arrays.push(Arc::new(self.luminaire_name.finish()));
        arrays.push(Arc::new(self.luminaire_number.finish()));
        arrays.push(Arc::new(self.file_name.finish()));
        arrays.push(Arc::new(self.date_user.finish()));
        arrays.push(Arc::new(self.measurement_report_number.finish()));
        arrays.push(Arc::new(self.type_indicator.finish()));
        arrays.push(Arc::new(self.symmetry.finish()));
        arrays.push(Arc::new(self.num_c_planes.finish()));
        arrays.push(Arc::new(self.c_plane_distance.finish()));
        arrays.push(Arc::new(self.num_g_planes.finish()));
        arrays.push(Arc::new(self.g_plane_distance.finish()));
        arrays.push(Arc::new(self.length.finish()));
        arrays.push(Arc::new(self.width.finish()));
        arrays.push(Arc::new(self.height.finish()));
        arrays.push(Arc::new(self.luminous_area_length.finish()));
        arrays.push(Arc::new(self.luminous_area_width.finish()));
        arrays.push(Arc::new(self.height_c0.finish()));
        arrays.push(Arc::new(self.height_c90.finish()));
        arrays.push(Arc::new(self.height_c180.finish()));
        arrays.push(Arc::new(self.height_c270.finish()));
        arrays.push(Arc::new(self.downward_flux_fraction.finish()));
        arrays.push(Arc::new(self.light_output_ratio.finish()));
        arrays.push(Arc::new(self.conversion_factor.finish()));
        arrays.push(Arc::new(self.tilt_angle.finish()));
        arrays.push(Arc::new(self.direct_ratios.finish()));
        arrays.push(Arc::new(self.lamp_sets.finish()));

        #[cfg(feature = "summary")]
        arrays.extend(self.summary.finish());

        #[cfg(feature = "raw-photometry")]
        {
            arrays.push(Arc::new(self.c_angles.finish()));
            arrays.push(Arc::new(self.g_angles.finish()));
            arrays.push(Arc::new(self.intensities.finish()));
        }

        self.rows = 0;
        RecordBatch::try_new(schema.clone(), arrays).context("building RecordBatch")
    }
}

fn lamp_sets_builder() -> ListBuilder<StructBuilder> {
    let fields = vec![
        Field::new("num_lamps", DataType::Int32, false),
        Field::new("lamp_type", DataType::Utf8, false),
        Field::new("total_luminous_flux", DataType::Float64, false),
        Field::new("color_appearance", DataType::Utf8, false),
        Field::new("color_rendering_group", DataType::Utf8, false),
        Field::new("wattage_with_ballast", DataType::Float64, false),
    ];
    let sb = StructBuilder::new(
        fields.clone(),
        vec![
            Box::new(Int32Builder::new()),
            Box::new(StringBuilder::new()),
            Box::new(Float64Builder::new()),
            Box::new(StringBuilder::new()),
            Box::new(StringBuilder::new()),
            Box::new(Float64Builder::new()),
        ],
    );
    ListBuilder::new(sb).with_field(Arc::new(Field::new(
        "item",
        DataType::Struct(fields.into()),
        true,
    )))
}
