# eulumdat-parquet

Apache Parquet export for [eulumdat](https://github.com/holg/eulumdat-rs) photometric files.

Produces a single Parquet file with **one row per luminaire** — ideal for analyzing a manufacturer catalog in DuckDB, Polars, or pandas.

## Usage

```rust
use eulumdat::Eulumdat;
use eulumdat_parquet::EulumdatParquetWriter;

let mut writer = EulumdatParquetWriter::create("catalog.parquet")?;
for path in glob::glob("catalog/*.ldt")? {
    let path = path?;
    let ldt = Eulumdat::from_file(&path)?;
    writer.append(path.to_string_lossy().as_ref(), &ldt)?;
}
writer.finish()?;
```

Then in DuckDB:

```sql
SELECT luminaire_name, beam_angle, luminaire_efficacy
FROM 'catalog.parquet'
WHERE beam_angle < 30 AND luminaire_efficacy > 100
ORDER BY luminaire_efficacy DESC;
```

## Schema

Wide row-per-file layout:

- **Identity**: `file_path`, `identification`, `luminaire_name`, `luminaire_number`, `date_user`, `measurement_report_number`
- **Classification**: `type_indicator`, `symmetry` (both as strings)
- **Grid**: `num_c_planes`, `c_plane_distance`, `num_g_planes`, `g_plane_distance`
- **Dimensions (mm)**: `length`, `width`, `height`, `luminous_area_length`, `luminous_area_width`, `height_c0/c90/c180/c270`
- **Optical**: `downward_flux_fraction`, `light_output_ratio`, `conversion_factor`, `tilt_angle`
- **`direct_ratios`**: `list<double>` (fixed length 10)
- **`lamp_sets`**: `list<struct{num_lamps, lamp_type, total_luminous_flux, color_appearance, color_rendering_group, wattage_with_ballast}>`
- **Summary** (with default `summary` feature): beam/field angles (IES + CIE), efficacy, flux, batwing flag, zonal lumens, etc.
- **Raw photometry** (with opt-in `raw-photometry` feature): `c_angles list<double>`, `g_angles list<double>`, `intensities list<list<double>>`

## Features

- `summary` (default): include computed `PhotometricSummary` metrics
- `raw-photometry` (opt-in): include full C/G angle grids and intensity matrix

## License

AGPL-3.0-or-later
