//! OxyTech `.OXL` / `.OXC` file parser (read-only).
//!
//! OxyTech LITESTAR exports luminaire data as XML in two flavors:
//! - **`.OXL`** — full package with photometry (`<LuminaireList>` populated).
//! - **`.OXC`** — commercial-only package, no photometry (`<LuminaireList>` empty).
//!
//! Both share the same root element `<LitePack>` and the same XML schema;
//! `.OXC` is just an `.OXL` with no `<Luminaire>` entries. Spec reference:
//! `docs/OXL-UK.pdf` (LITESTAR 4D v.3.00, March 2015).
//!
//! Unlike the single-luminaire formats we already parse (LDT/IES/TM-33-23),
//! OXL is **multi-luminaire** — a single file can carry several products.
//! [`parse`] therefore returns an [`OxlPackage`] containing a `Vec` of
//! [`LuminaireOpticalData`].
//!
//! # Scope
//!
//! Read-only. Photometry, lamp lists, header metadata, and basic geometry
//! (bounding box, luminous opening) are mapped into atla types. The
//! following subtrees are intentionally **skipped** by this parser:
//!
//! - `<MeshList>` / `<MaterialList>` — the 3D model side of the file.
//! - `<Hierarchy>` — multi-part luminaire transform tree.
//! - `<TechSheet>` — commercial / installation metadata (datasheets,
//!   images encoded as base64, IK / IP ratings, etc.).
//! - `<Classifications>`, `<UGR>`, `<Reliefs>` — pre-computed metrics
//!   we recompute from the intensity grid anyway.
//!
//! # Example
//!
//! ```rust,ignore
//! use eulumdat::atla::oxl;
//!
//! let xml = std::fs::read_to_string("luminaires.oxl")?;
//! let pkg = oxl::parse(&xml)?;
//! println!("{} luminaires from {}", pkg.luminaires.len(),
//!          pkg.header.manufacturer.as_deref().unwrap_or("?"));
//! for lum in &pkg.luminaires {
//!     println!("  - {}", lum.header.description.as_deref().unwrap_or("?"));
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::atla::error::{AtlaError, Result};
use crate::atla::types::*;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

// ─────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────

/// A parsed OXL/OXC package.
///
/// `header` carries catalog-level metadata (LitePack version, creator,
/// top-level product identity). `luminaires` holds the per-product
/// `LuminaireOpticalData` documents — empty for an `.OXC` file.
#[derive(Debug, Clone, Default)]
pub struct OxlPackage {
    pub header: OxlHeader,
    pub luminaires: Vec<LuminaireOpticalData>,
}

/// LitePack header metadata (multiple-luminaire-package level).
#[derive(Debug, Clone, Default)]
pub struct OxlHeader {
    /// `<LitepackVersion>` — e.g. "0.0003".
    pub litepack_version: String,
    /// `<CreatorInfo>` — usually the LITESTAR module that emitted the file.
    pub creator_info: Option<String>,
    /// Top-level `<Manufacturer>/<ManufacturerName>`.
    pub manufacturer: Option<String>,
    /// `<ProductFamily>` at package level.
    pub product_family: Option<String>,
    /// `<ProductCode>` at package level.
    pub product_code: Option<String>,
    /// `<ProductName>` at package level.
    pub product_name: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────

/// Parse an OXL/OXC XML string into an `OxlPackage`.
///
/// Handles UTF-8 BOMs transparently (quick-xml strips them on entry).
pub fn parse(xml: &str) -> Result<OxlPackage> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut pkg = OxlPackage::default();
    // Path stack of element names so we can disambiguate identical leaf
    // tags that appear under different parents (e.g. `<ProductName>` in
    // `<Header>` vs `<Luminaire>/<ProductIdentity>`).
    let mut path: Vec<String> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                path.push(name.clone());

                match name.as_str() {
                    // Each `<Luminaire>` starts a fresh sub-document.
                    "Luminaire"
                        if at(&path, &["LitePack", "Data", "LuminaireList", "Luminaire"]) =>
                    {
                        let lum = parse_luminaire(&mut reader)?;
                        pkg.luminaires.push(lum);
                        // parse_luminaire consumed up to and including the
                        // matching </Luminaire>, so we drop it from our
                        // local path stack.
                        path.pop();
                    }
                    // Skip subtrees we don't model.
                    "MeshList" | "MaterialList" | "Hierarchy" | "TechSheet" => {
                        skip_subtree(&mut reader, &name)?;
                        path.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = t.unescape().map(|s| s.into_owned()).unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                // Header-level metadata. Match exact paths so we don't
                // accidentally absorb a nested manufacturer name from a
                // luminaire entry.
                let here = path.last().map(String::as_str).unwrap_or("");
                let parent = path.iter().rev().nth(1).map(String::as_str).unwrap_or("");
                if path.starts_with(&["LitePack".to_string(), "Header".to_string()]) {
                    match (parent, here) {
                        (_, "LitepackVersion") => pkg.header.litepack_version = text,
                        (_, "CreatorInfo") => pkg.header.creator_info = Some(text),
                        ("Manufacturer", "ManufacturerName") => {
                            pkg.header.manufacturer = Some(text);
                        }
                        (_, "ProductFamily") => pkg.header.product_family = Some(text),
                        (_, "ProductCode") => pkg.header.product_code = Some(text),
                        (_, "ProductName") => pkg.header.product_name = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => {
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AtlaError::XmlParse(format!(
                    "OXL parse error at byte {}: {e}",
                    reader.buffer_position()
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(pkg)
}

/// Parse an OXL/OXC file from disk.
///
/// Reads the file as UTF-8 and dispatches to [`parse`]. The file
/// extension is not checked — both `.oxl` and `.oxc` (and any other
/// extension) are accepted.
pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<OxlPackage> {
    let content = std::fs::read_to_string(path.as_ref())?;
    parse(&content)
}

// ─────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────

/// Returns `true` when the current element-name stack ends with `expected`
/// (treated as the path of nested elements down to the current one).
fn at(path: &[String], expected: &[&str]) -> bool {
    if path.len() < expected.len() {
        return false;
    }
    let tail = &path[path.len() - expected.len()..];
    tail.iter().zip(expected.iter()).all(|(a, b)| a == b)
}

/// Read events until the matching `</tag>` end is consumed.
///
/// Handles arbitrary nesting; we only care that whatever subtree we
/// don't model gets walked past so the outer parser stays in step with
/// the document.
fn skip_subtree(reader: &mut Reader<&[u8]>, tag: &str) -> Result<()> {
    let mut depth: i32 = 1;
    let mut buf = Vec::new();
    while depth > 0 {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let name = String::from_utf8_lossy(qname.as_ref());
                if name == tag {
                    depth += 1;
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let name = String::from_utf8_lossy(qname.as_ref());
                if name == tag {
                    depth -= 1;
                }
                // Any other end tag just balances out an earlier Start
                // (whose increment we tracked above), so we don't react.
            }
            Ok(Event::Eof) => {
                return Err(AtlaError::XmlParse(format!(
                    "unexpected EOF while skipping <{tag}>"
                )));
            }
            Err(e) => {
                return Err(AtlaError::XmlParse(format!(
                    "skip_subtree({tag}) error at byte {}: {e}",
                    reader.buffer_position()
                )));
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Parse a single `<Luminaire>` element (caller has already consumed the
/// opening tag). Returns when the matching `</Luminaire>` is consumed.
fn parse_luminaire(reader: &mut Reader<&[u8]>) -> Result<LuminaireOpticalData> {
    let mut doc = LuminaireOpticalData::new();
    doc.schema_version = SchemaVersion::AtlaS001;
    doc.version = "1.0".into();

    // Every OXL luminaire has at most one emitter (one lamp set, one
    // measurement). We assemble it incrementally.
    let mut emitter = Emitter {
        quantity: 1,
        ..Default::default()
    };
    let mut luminaire = Luminaire::default();
    let mut opening = LuminousOpening::default();
    let mut have_opening = false;

    let mut path: Vec<String> = vec!["Luminaire".into()];
    let mut depth = 1;
    let mut buf = Vec::new();

    while depth > 0 {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                depth += 1;
                path.push(name.clone());

                match name.as_str() {
                    // Photometry needs a structured walk because it has
                    // its own nested measurement matrix.
                    "Photometry" => {
                        let dist = parse_photometry(reader)?;
                        if let Some(dist) = dist {
                            emitter.intensity_distribution = Some(dist);
                        }
                        depth -= 1;
                        path.pop();
                    }
                    // Walk past subtrees we explicitly don't import.
                    "Classifications" | "UGR" | "Reliefs" | "Geometry" | "MeshList"
                    | "MaterialList" | "Constraints" | "Flags" => {
                        skip_subtree(reader, &name)?;
                        depth -= 1;
                        path.pop();
                    }
                    "Lamp" => {
                        // <Lamp quantity="N"> — pull quantity from attr.
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"quantity" {
                                let q = String::from_utf8_lossy(&attr.value);
                                if let Ok(n) = q.parse::<u32>() {
                                    emitter.quantity = n.max(1);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                depth -= 1;
                path.pop();

                if depth == 0 && name == "Luminaire" {
                    break;
                }
                if name == "LuminousArea" && have_opening {
                    luminaire.luminous_openings.push(opening.clone());
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = t.unescape().map(|s| s.into_owned()).unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                let here = path.last().map(String::as_str).unwrap_or("");
                let parent = path.iter().rev().nth(1).map(String::as_str).unwrap_or("");
                let grand = path.iter().rev().nth(2).map(String::as_str).unwrap_or("");

                // Luminaire / ProductIdentity
                if grand == "Luminaire" && parent == "ProductIdentity" {
                    match here {
                        "ProductCode" => doc.header.catalog_number = Some(text),
                        "ProductName" => doc.header.description = Some(text),
                        _ => {}
                    }
                } else if grand == "Manufacturer"
                    && here == "ManufacturerName"
                    && path.contains(&"ProductIdentity".to_string())
                    && doc.header.manufacturer.is_none()
                {
                    doc.header.manufacturer = Some(text);
                }
                // Luminaire / Shape, BBoxDims
                else if parent == "Luminaire" && here == "Shape" {
                    // Shape applies to the luminaire bounding-box; we
                    // only model the luminous opening's shape, so this
                    // is informational and dropped for now.
                } else if grand == "Luminaire" && parent == "BBoxDims" {
                    let dims = luminaire.dimensions.get_or_insert_with(Dimensions::default);
                    set_bbox_dim(dims, here, &text);
                }
                // Luminaire / LuminousArea
                else if grand == "LuminousArea" && parent == "BBoxDims" {
                    set_opening_dim(&mut opening, here, &text);
                    have_opening = true;
                } else if parent == "LuminousArea" && here == "Shape" {
                    opening.shape = parse_shape(&text);
                    have_opening = true;
                }
                // Luminaire / LampList / Lamp
                else if path.iter().any(|s| s == "Lamp") {
                    match here {
                        "Flux" => emitter.rated_lumens = parse_f64_opt(&text),
                        "Power" => emitter.input_watts = parse_f64_opt(&text),
                        "ColorTemperature" => emitter.cct = parse_f64_opt(&text),
                        "ColorRenderingIndexRa" => {
                            let ra = parse_f64_opt(&text);
                            if ra.is_some() {
                                let cr = emitter
                                    .color_rendering
                                    .get_or_insert_with(ColorRendering::default);
                                cr.ra = ra;
                            }
                        }
                        "ProductCode" if parent == "ProductIdentity" => {
                            // Lamp's own ProductCode — atla doesn't have a
                            // dedicated lamp-catalog field; surface as
                            // emitter description if not already set.
                            if emitter.description.is_none() {
                                emitter.description = Some(text);
                            }
                        }
                        "ProductName" if parent == "ProductIdentity" => {
                            if emitter.description.is_none() {
                                emitter.description = Some(text);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => {
                return Err(AtlaError::XmlParse(
                    "unexpected EOF inside <Luminaire>".into(),
                ));
            }
            Err(e) => {
                return Err(AtlaError::XmlParse(format!(
                    "Luminaire parse error at byte {}: {e}",
                    reader.buffer_position()
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    if luminaire.dimensions.is_some() || !luminaire.luminous_openings.is_empty() {
        doc.luminaire = Some(luminaire);
    }
    if emitter.intensity_distribution.is_some()
        || emitter.rated_lumens.is_some()
        || emitter.input_watts.is_some()
    {
        doc.emitters.push(emitter);
    }

    Ok(doc)
}

/// Parse a `<Photometry>` block. Returns the assembled
/// [`IntensityDistribution`] when the matrix is present, or `None` when
/// the OXL is `.OXC`-shaped (no photometry — although LITESTAR usually
/// omits the entire `<Photometry>` element in that case).
fn parse_photometry(reader: &mut Reader<&[u8]>) -> Result<Option<IntensityDistribution>> {
    let mut symmetry_str: Option<String> = None;
    let mut flux_used: Option<f64> = None;
    let mut have_matrix = false;
    // Per-plane data: (c_angle, intensities-by-gamma)
    let mut planes: Vec<(f64, Vec<f64>)> = Vec::new();
    let mut gv_angles: Vec<f64> = Vec::new();

    let mut depth = 1;
    let mut path: Vec<String> = vec!["Photometry".into()];
    let mut buf = Vec::new();

    // While walking <Plane> children, accumulate the current C-angle and
    // its <Vals> string until we hit </Plane>.
    let mut current_c: Option<f64> = None;
    let mut current_vals: Option<String> = None;

    while depth > 0 {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                depth += 1;
                path.push(name.clone());

                if name == "MeasurementMatrix" {
                    have_matrix = true;
                }
                // We don't actively skip MeasurementConditions — its
                // sub-elements just won't match anything below.
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                depth -= 1;
                path.pop();
                if name == "Plane" {
                    if let (Some(c), Some(v)) = (current_c.take(), current_vals.take()) {
                        let row: Vec<f64> = v
                            .split_ascii_whitespace()
                            .filter_map(|s| s.parse::<f64>().ok())
                            .collect();
                        planes.push((c, row));
                    }
                }
                if depth == 0 && name == "Photometry" {
                    break;
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = t.unescape().map(|s| s.into_owned()).unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                let here = path.last().map(String::as_str).unwrap_or("");

                if path.iter().any(|s| s == "MeasurementMatrix") {
                    match here {
                        "GVAngles" => {
                            gv_angles = text
                                .split_ascii_whitespace()
                                .filter_map(|s| s.parse::<f64>().ok())
                                .collect();
                        }
                        "CHPlane" => {
                            current_c = text.parse::<f64>().ok();
                        }
                        "Vals" => {
                            current_vals = Some(text);
                        }
                        _ => {}
                    }
                } else {
                    match here {
                        "SymmetryType" => symmetry_str = Some(text),
                        "FluxUsed" => flux_used = parse_f64_opt(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => {
                return Err(AtlaError::XmlParse(
                    "unexpected EOF inside <Photometry>".into(),
                ));
            }
            Err(e) => {
                return Err(AtlaError::XmlParse(format!(
                    "Photometry parse error at byte {}: {e}",
                    reader.buffer_position()
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    if !have_matrix || planes.is_empty() || gv_angles.is_empty() {
        return Ok(None);
    }

    // Some exporters emit C-angles in lexicographic order ("100" before
    // "20"). Sort numerically so the resulting grid is consistent.
    planes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Convert absolute cd → cd/klm using FluxUsed when available. If
    // FluxUsed is missing or zero we leave the raw cd values and tag
    // the units accordingly so downstream callers can interpret them.
    let (intensities, units) = if let Some(flux) = flux_used.filter(|f| *f > 0.0) {
        let scale = 1000.0 / flux;
        let scaled: Vec<Vec<f64>> = planes
            .iter()
            .map(|(_, row)| row.iter().map(|v| v * scale).collect())
            .collect();
        (scaled, IntensityUnits::CandelaPerKilolumen)
    } else {
        (
            planes.iter().map(|(_, row)| row.clone()).collect(),
            IntensityUnits::Candela,
        )
    };

    let horizontal_angles: Vec<f64> = planes.iter().map(|(c, _)| *c).collect();

    let mut dist = IntensityDistribution {
        photometry_type: PhotometryType::default(),
        metric: IntensityMetric::default(),
        units,
        horizontal_angles,
        vertical_angles: gv_angles,
        intensities,
        ..Default::default()
    };
    if let Some(s) = symmetry_str {
        dist.symmetry = Some(map_oxl_symmetry(&s));
    }

    Ok(Some(dist))
}

// ─────────────────────────────────────────────────────────────────────────
// Small mappers
// ─────────────────────────────────────────────────────────────────────────

fn parse_f64_opt(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn parse_shape(s: &str) -> LuminousOpeningShape {
    match s.trim().to_ascii_lowercase().as_str() {
        "rectangular" | "rectangle" | "square" => LuminousOpeningShape::Rectangular,
        "circular" | "circle" | "round" => LuminousOpeningShape::Circular,
        "elliptical" | "ellipse" | "oval" => LuminousOpeningShape::Elliptical,
        "point" => LuminousOpeningShape::Point,
        _ => LuminousOpeningShape::Rectangular,
    }
}

/// OXL `<BBoxDims>` values are in **meters**; atla `Dimensions` are in
/// **millimeters**. Convert on the way in.
fn set_bbox_dim(dims: &mut Dimensions, key: &str, text: &str) {
    let Some(v) = parse_f64_opt(text) else { return };
    let mm = v * 1000.0;
    match key {
        "WidthC0C180" => dims.width = mm,
        "LengthC90C270" => dims.length = mm,
        "Height" => dims.height = mm,
        _ => {}
    }
}

fn set_opening_dim(opening: &mut LuminousOpening, key: &str, text: &str) {
    let Some(v) = parse_f64_opt(text) else { return };
    let mm = v * 1000.0;
    match key {
        "WidthC0C180" => opening.dimensions.width = Some(mm),
        "LengthC90C270" => opening.dimensions.length = mm,
        "Height" => { /* atla OpeningDimensions has no height */ }
        _ => {}
    }
}

/// Map OXL `SymmetryType` strings (per LITESTAR 4D conventions) onto
/// atla [`SymmetryType`]. Unknown strings fall back to `None` so the
/// distribution is treated as fully sampled — safer than guessing.
fn map_oxl_symmetry(s: &str) -> SymmetryType {
    let s = s.trim().to_ascii_lowercase();
    match s.as_str() {
        "asymmetricc" | "asymmetricg" | "asymmetriccg" | "none" => SymmetryType::None,
        "symc0c180" | "c0c180" | "bic0c180" => SymmetryType::Bi0,
        "symc90c270" | "c90c270" | "bic90c270" => SymmetryType::Bi90,
        "symboth" | "quadcg" | "quad" | "bothplanes" => SymmetryType::Quad,
        "symrotational" | "vertical" | "rotational" | "full" => SymmetryType::Full,
        _ => SymmetryType::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_litepack_header() {
        let xml = r#"<?xml version="1.0"?>
<LitePack>
  <Header>
    <LitepackVersion>0.0003</LitepackVersion>
    <CreatorInfo>FotomDati</CreatorInfo>
    <ProductIdentity>
      <Manufacturer>
        <ManufacturerName>OxyTech</ManufacturerName>
      </Manufacturer>
      <ProductFamily>OXL</ProductFamily>
      <ProductCode>X1</ProductCode>
      <ProductName>Test</ProductName>
    </ProductIdentity>
  </Header>
  <Data><LuminaireList/></Data>
</LitePack>"#;
        let pkg = parse(xml).expect("parse");
        assert_eq!(pkg.header.litepack_version, "0.0003");
        assert_eq!(pkg.header.creator_info.as_deref(), Some("FotomDati"));
        assert_eq!(pkg.header.manufacturer.as_deref(), Some("OxyTech"));
        assert_eq!(pkg.header.product_code.as_deref(), Some("X1"));
        assert_eq!(pkg.header.product_name.as_deref(), Some("Test"));
        assert!(pkg.luminaires.is_empty());
    }

    #[test]
    fn parses_minimal_luminaire_with_photometry() {
        let xml = r#"<?xml version="1.0"?>
<LitePack>
  <Header><LitepackVersion>0.0003</LitepackVersion></Header>
  <Data>
    <LuminaireList>
      <Luminaire id="100">
        <ProductIdentity>
          <ProductCode>A12</ProductCode>
          <ProductName>Test Lum</ProductName>
        </ProductIdentity>
        <BBoxDims>
          <WidthC0C180>0.300</WidthC0C180>
          <LengthC90C270>0.500</LengthC90C270>
          <Height>0.100</Height>
        </BBoxDims>
        <LampList>
          <Lamp quantity="2">
            <Flux>2000.0</Flux>
            <Power>20.0</Power>
            <ColorTemperature>4000.0</ColorTemperature>
            <ColorRenderingIndexRa>80.0</ColorRenderingIndexRa>
          </Lamp>
        </LampList>
        <Photometry>
          <SymmetryType>asymmetricCG</SymmetryType>
          <FluxUsed>4000.0</FluxUsed>
          <MeasurementMatrix>
            <NumCH>2</NumCH>
            <NumGV>3</NumGV>
            <GVAngles>0 45 90</GVAngles>
            <Planes>
              <Plane><CHPlane>0</CHPlane><Vals>100 50 0</Vals></Plane>
              <Plane><CHPlane>180</CHPlane><Vals>100 50 0</Vals></Plane>
            </Planes>
          </MeasurementMatrix>
        </Photometry>
      </Luminaire>
    </LuminaireList>
  </Data>
</LitePack>"#;
        let pkg = parse(xml).expect("parse");
        assert_eq!(pkg.luminaires.len(), 1);
        let lum = &pkg.luminaires[0];
        assert_eq!(lum.header.catalog_number.as_deref(), Some("A12"));
        assert_eq!(lum.header.description.as_deref(), Some("Test Lum"));
        let dims = lum
            .luminaire
            .as_ref()
            .and_then(|l| l.dimensions.as_ref())
            .unwrap();
        assert!((dims.width - 300.0).abs() < 1e-6);
        assert!((dims.length - 500.0).abs() < 1e-6);

        let em = &lum.emitters[0];
        assert_eq!(em.quantity, 2);
        assert_eq!(em.rated_lumens, Some(2000.0));
        assert_eq!(em.input_watts, Some(20.0));
        assert_eq!(em.cct, Some(4000.0));
        assert_eq!(em.color_rendering.as_ref().unwrap().ra, Some(80.0));

        let dist = em.intensity_distribution.as_ref().unwrap();
        assert_eq!(dist.horizontal_angles, vec![0.0, 180.0]);
        assert_eq!(dist.vertical_angles, vec![0.0, 45.0, 90.0]);
        assert_eq!(dist.units, IntensityUnits::CandelaPerKilolumen);
        // 100 cd at FluxUsed=4000 → 100 × 1000 / 4000 = 25 cd/klm
        assert!((dist.intensities[0][0] - 25.0).abs() < 1e-6);
        assert_eq!(dist.symmetry, Some(SymmetryType::None));
    }

    #[test]
    fn lex_sorted_c_angles_are_sorted_numerically() {
        // Some OXL exporters write CHPlane in string order: 0, 100, 110,
        // 20, 200, … The parser must reorder them numerically so the
        // intensity grid stays aligned with the angles.
        let xml = r#"<?xml version="1.0"?>
<LitePack>
  <Data><LuminaireList><Luminaire id="1">
    <Photometry>
      <FluxUsed>1000.0</FluxUsed>
      <MeasurementMatrix>
        <NumCH>3</NumCH><NumGV>1</NumGV>
        <GVAngles>0</GVAngles>
        <Planes>
          <Plane><CHPlane>0</CHPlane><Vals>10</Vals></Plane>
          <Plane><CHPlane>100</CHPlane><Vals>30</Vals></Plane>
          <Plane><CHPlane>20</CHPlane><Vals>20</Vals></Plane>
        </Planes>
      </MeasurementMatrix>
    </Photometry>
  </Luminaire></LuminaireList></Data>
</LitePack>"#;
        let pkg = parse(xml).unwrap();
        let dist = pkg.luminaires[0].emitters[0]
            .intensity_distribution
            .as_ref()
            .unwrap();
        assert_eq!(dist.horizontal_angles, vec![0.0, 20.0, 100.0]);
        // intensities must follow the new ordering.
        assert!((dist.intensities[0][0] - 10.0).abs() < 1e-6);
        assert!((dist.intensities[1][0] - 20.0).abs() < 1e-6);
        assert!((dist.intensities[2][0] - 30.0).abs() < 1e-6);
    }

    #[test]
    fn empty_luminaire_list_yields_oxc_shaped_package() {
        let xml = r#"<?xml version="1.0"?>
<LitePack>
  <Header><LitepackVersion>0.0003</LitepackVersion></Header>
  <Data><LuminaireList/></Data>
</LitePack>"#;
        let pkg = parse(xml).unwrap();
        assert!(pkg.luminaires.is_empty());
        assert_eq!(pkg.header.litepack_version, "0.0003");
    }

    #[test]
    fn missing_flux_used_keeps_absolute_candela() {
        let xml = r#"<?xml version="1.0"?>
<LitePack>
  <Data><LuminaireList><Luminaire id="1">
    <Photometry>
      <MeasurementMatrix>
        <NumCH>1</NumCH><NumGV>1</NumGV>
        <GVAngles>0</GVAngles>
        <Planes>
          <Plane><CHPlane>0</CHPlane><Vals>123.45</Vals></Plane>
        </Planes>
      </MeasurementMatrix>
    </Photometry>
  </Luminaire></LuminaireList></Data>
</LitePack>"#;
        let pkg = parse(xml).unwrap();
        let dist = pkg.luminaires[0].emitters[0]
            .intensity_distribution
            .as_ref()
            .unwrap();
        assert_eq!(dist.units, IntensityUnits::Candela);
        assert!((dist.intensities[0][0] - 123.45).abs() < 1e-6);
    }
}
