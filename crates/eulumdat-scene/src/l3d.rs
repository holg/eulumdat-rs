//! L3D / GLDF readers for the luminaire body (feature `l3d`).
//!
//! `mesh_source_from_l3d` turns an L3D archive into a [`MeshSource`];
//! `gldf_extract` pulls the photometry files, the sensor files and the first
//! L3D archive out of a GLDF container. It reads only the `<File>` list of
//! `product.xml` (content types), not the variants (variant selection is the
//! editor's job).

use crate::mesh::{EmittingFace, MeshPart, MeshSource};
use glam::Vec3;
use std::io::Read;

/// Geometry and emitting faces of an L3D archive.
pub fn mesh_source_from_l3d(bytes: &[u8]) -> Option<MeshSource> {
    let l3d = l3d_rs::from_buffer(bytes);
    if l3d.model.parts.is_empty() {
        return None;
    }
    let mut src = MeshSource::default();
    for part in &l3d.model.parts {
        let want = part.path.to_ascii_lowercase();
        let asset = l3d.file.assets.iter().find(|a| {
            let n = a.name.to_ascii_lowercase();
            n == want || n.ends_with(&format!("/{want}")) || want.ends_with(&n)
        });
        let Some(asset) = asset else {
            continue;
        };
        src.parts.push(MeshPart {
            obj: String::from_utf8_lossy(&asset.content).into_owned(),
            mat: part.mat,
        });
    }
    if let Ok(lum) = l3d_rs::Luminaire::from_xml(&l3d.file.structure) {
        collect_faces(&lum.structure.geometry, Vec3::ZERO, &mut src.faces);
    }
    (!src.parts.is_empty()).then_some(src)
}

fn collect_faces(geo: &l3d_rs::Geometry, parent: Vec3, out: &mut Vec<EmittingFace>) {
    let base = parent + Vec3::new(geo.position.x, geo.position.y, geo.position.z);
    if let Some(leos) = &geo.light_emitting_objects {
        for leo in &leos.light_emitting_object {
            let centre = base + Vec3::new(leo.position.x, leo.position.y, leo.position.z);
            if let Some(r) = &leo.rectangle {
                out.push(EmittingFace {
                    centre,
                    half: (r.size_x as f32 * 0.5, r.size_y as f32 * 0.5),
                    round: false,
                });
            } else if let Some(c) = &leo.circle {
                let r = c.diameter as f32 * 0.5;
                out.push(EmittingFace {
                    centre,
                    half: (r, r),
                    round: true,
                });
            }
        }
    }
    if let Some(joints) = &geo.joints {
        for j in &joints.joint {
            let jp = base + Vec3::new(j.position.x, j.position.y, j.position.z);
            for child in &j.geometries.geometry {
                collect_faces(child, jp, out);
            }
        }
    }
}

/// One photometry file of a GLDF container.
#[derive(Clone, Debug, Default)]
pub struct GldfPhotometry {
    /// Zip path (`ldc/x.ldt`).
    pub name: String,
    pub text: String,
}

/// One sensor file of a GLDF container (`sensor/sensldt` or `sensor/sensxml`).
#[derive(Clone, Debug, Default)]
pub struct GldfSensorFile {
    /// Zip path (`sensor/x.ldt`).
    pub name: String,
    /// Content type from `product.xml`, or derived from the folder / extension.
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// What the Light scene needs from a GLDF container.
#[derive(Clone, Debug, Default)]
pub struct GldfExtract {
    /// Every luminaire `.ldt` / `.ies` in the container, in zip order.
    /// Sensor files are never in here, even if they are EULUMDAT.
    pub photometries: Vec<GldfPhotometry>,
    /// Sensor files (`contentType="sensor/…"` or stored under `sensor/`).
    pub sensors: Vec<GldfSensorFile>,
    /// First `.l3d` archive in the container, if any.
    pub l3d: Option<Vec<u8>>,
    /// The container's `product.xml` (emitters, variants, emergency data).
    pub product_xml: Option<String>,
}

/// `(file name, content type)` of every `<File>` in a GLDF `product.xml`.
/// A plain scan: the element is flat (`<File id contentType type>name</File>`).
pub fn gldf_file_types(product_xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = product_xml;
    while let Some(i) = rest.find("<File") {
        let after = &rest[i + 5..];
        // `<Files>` and `<FileReference…>` are other elements
        if !after.starts_with(|c: char| c.is_whitespace()) {
            rest = after;
            continue;
        }
        let Some(gt) = after.find('>') else { break };
        let attrs = &after[..gt];
        let body = &after[gt + 1..];
        let (name, next) = match body.find("</File>") {
            Some(end) if !attrs.ends_with('/') => (body[..end].trim().to_string(), &body[end..]),
            _ => (String::new(), body),
        };
        if let Some(ct) = attr_value(attrs, "contentType") {
            if !name.is_empty() {
                out.push((name, ct));
            }
        }
        rest = next;
    }
    out
}

fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut rest = attrs;
    while let Some(i) = rest.find(key) {
        let before_ok = i == 0 || rest[..i].ends_with(|c: char| c.is_whitespace());
        let after = rest[i + key.len()..].trim_start();
        if before_ok {
            if let Some(v) = after.strip_prefix('=') {
                let v = v.trim_start();
                let q = v.chars().next()?;
                if q == '"' || q == '\'' {
                    let end = v[1..].find(q)?;
                    return Some(v[1..1 + end].to_string());
                }
            }
        }
        rest = &rest[i + key.len()..];
    }
    None
}

impl GldfExtract {
    /// The photometry with the highest luminous flux (the main variant of
    /// most containers), else the first one.
    pub fn best_photometry(&self) -> Option<&GldfPhotometry> {
        let mut best: Option<(&GldfPhotometry, f64)> = None;
        for p in &self.photometries {
            let flux = crate::viewer::parse_any(&p.name, &p.text)
                .map(|l| l.luminous_flux())
                .unwrap_or(-1.0);
            if best.is_none_or(|(_, f)| flux > f) {
                best = Some((p, flux));
            }
        }
        best.map(|(p, _)| p).or(self.photometries.first())
    }
}

/// Photometry files, sensor files and the first L3D archive of a GLDF
/// container. Files are classified by the `contentType` in `product.xml`
/// (`sensor/…` → sensor), and anything stored under `sensor/` counts as a
/// sensor even without it, so a sensor LDT is never taken for the luminaire.
/// An unmarked `.ldt` with the sensor LDT fingerprint
/// ([`eulumdat::sensor::is_sensor_ldt`]: ranges in metres, a mounting code
/// in the parameter block) is a sensor too. LDTs may be UTF-8 or Latin-1.
/// `None` when the container holds neither photometry nor sensor files (a
/// detector product may come without any luminaire photometry).
pub fn gldf_extract(bytes: &[u8]) -> Option<GldfExtract> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut types: Vec<(String, String)> = Vec::new();
    let mut product_xml = None;
    if let Ok(mut f) = zip.by_name("product.xml") {
        let mut raw = Vec::new();
        if f.read_to_end(&mut raw).is_ok() {
            let text = String::from_utf8_lossy(&raw).to_string();
            types = gldf_file_types(&text);
            product_xml = Some(text);
        }
    }
    let type_of = |zip_name: &str| -> Option<&str> {
        let base = zip_name.rsplit('/').next().unwrap_or(zip_name);
        types
            .iter()
            .find(|(n, _)| n == zip_name || n == base)
            .map(|(_, t)| t.as_str())
    };
    let mut out = GldfExtract {
        product_xml,
        ..Default::default()
    };
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else {
            continue;
        };
        if !f.is_file() {
            continue;
        }
        let name = f.name().to_string();
        let lower = name.to_ascii_lowercase();
        let ct = type_of(&name).map(str::to_string);
        let is_sensor = ct.as_deref().is_some_and(|t| t.starts_with("sensor/"))
            || lower.starts_with("sensor/")
            || lower.contains("/sensor/");
        if is_sensor {
            let mut b = Vec::new();
            if f.read_to_end(&mut b).is_ok() {
                let content_type = ct.unwrap_or_else(|| {
                    if lower.ends_with(".xml") {
                        "sensor/sensxml".to_string()
                    } else {
                        "sensor/sensldt".to_string()
                    }
                });
                out.sensors.push(GldfSensorFile {
                    name,
                    content_type,
                    bytes: b,
                });
            }
        } else if lower.ends_with(".ldt") || lower.ends_with(".ies") {
            let mut b = Vec::new();
            if f.read_to_end(&mut b).is_err() {
                continue;
            }
            let text = match String::from_utf8(b) {
                Ok(s) => s,
                // EULUMDAT files are often Latin-1.
                Err(e) => e.into_bytes().iter().map(|&c| c as char).collect(),
            };
            let sensor_ldt = lower.ends_with(".ldt")
                && eulumdat::Eulumdat::parse(&text)
                    .is_ok_and(|l| eulumdat::sensor::is_sensor_ldt(&l));
            if sensor_ldt {
                out.sensors.push(GldfSensorFile {
                    name,
                    content_type: "sensor/sensldt".to_string(),
                    bytes: text.chars().map(|c| c as u32 as u8).collect(),
                });
            } else {
                out.photometries.push(GldfPhotometry { name, text });
            }
        } else if lower.ends_with(".l3d") && out.l3d.is_none() {
            let mut b = Vec::new();
            if f.read_to_end(&mut b).is_ok() {
                out.l3d = Some(b);
            }
        }
    }
    (!out.photometries.is_empty() || !out.sensors.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const LDT_TEMPLATE: &str = include_str!("../../eulumdat/tests/fixtures/sensor.ldt");

    fn gldf(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let opt = zip::write::SimpleFileOptions::default();
            for (n, b) in files {
                z.start_file(*n, opt).unwrap();
                z.write_all(b).unwrap();
            }
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn reads_file_types_from_product_xml() {
        let xml = r#"<Root><GeneralDefinitions><Files>
            <File id="ldt" contentType="ldc/eulumdat" type="localFileName">wide.ldt</File>
            <File id="sensor" contentType='sensor/sensldt' type="localFileName">sensor.ldt</File>
            <File id="img" contentType="image/png" type="url">http://x/y.png</File>
          </Files></GeneralDefinitions>
          <Photometry><PhotometryFileReference fileId="ldt" /></Photometry></Root>"#;
        let t = gldf_file_types(xml);
        assert_eq!(
            t,
            vec![
                ("wide.ldt".to_string(), "ldc/eulumdat".to_string()),
                ("sensor.ldt".to_string(), "sensor/sensldt".to_string()),
                ("http://x/y.png".to_string(), "image/png".to_string()),
            ]
        );
    }

    #[test]
    fn sensor_ldt_is_never_the_luminaire() {
        // the sensor file has the higher flux, so the old flux rule picked it
        let ldt = LDT_TEMPLATE.replace("\r\n", "\n");
        let lum = ldt.replacen("\n35\n", "\n20\n", 1);
        let sen = ldt.replacen("\n35\n", "\n900\n", 1);
        let flux = |t: &str| {
            crate::viewer::parse_any("x.ldt", t)
                .unwrap()
                .luminous_flux()
        };
        assert!(
            flux(&sen) > flux(&lum),
            "test setup: sensor must have more flux"
        );
        let product = r#"<Root><Files>
            <File id="l" contentType="ldc/eulumdat" type="localFileName">wide.ldt</File>
            <File id="s" contentType="sensor/sensldt" type="localFileName">sensor.ldt</File>
            </Files></Root>"#;
        let bytes = gldf(&[
            ("product.xml", product.as_bytes()),
            ("ldc/wide.ldt", lum.as_bytes()),
            ("sensor/sensor.ldt", sen.as_bytes()),
        ]);
        let x = gldf_extract(&bytes).unwrap();
        assert_eq!(x.photometries.len(), 1);
        assert_eq!(x.best_photometry().unwrap().name, "ldc/wide.ldt");
        assert_eq!(x.sensors.len(), 1);
        assert_eq!(x.sensors[0].content_type, "sensor/sensldt");
    }

    #[test]
    fn sensor_folder_counts_without_product_xml() {
        let bytes = gldf(&[
            ("ldc/a.ldt", LDT_TEMPLATE.as_bytes()),
            ("sensor/s.xml", b"<SensXML/>"),
        ]);
        let x = gldf_extract(&bytes).unwrap();
        assert_eq!(x.photometries.len(), 1);
        assert_eq!(x.sensors[0].content_type, "sensor/sensxml");
    }

    /// A sensor LDT filed as photometry (no sensor content type, not in
    /// `sensor/`) is still recognised by its fingerprint.
    #[test]
    fn unmarked_sensor_ldt_is_recognised() {
        let sensor = include_str!("../../eulumdat/tests/fixtures/sensors/demo_ceiling_360.ldt");
        let product = r#"<Root><Files>
            <File id="l" contentType="ldc/eulumdat" type="localFileName">lum.ldt</File>
            <File id="s" contentType="ldc/eulumdat" type="localFileName">pir.ldt</File>
            </Files></Root>"#;
        let bytes = gldf(&[
            ("product.xml", product.as_bytes()),
            ("ldc/lum.ldt", LDT_TEMPLATE.as_bytes()),
            ("ldc/pir.ldt", sensor.as_bytes()),
        ]);
        let x = gldf_extract(&bytes).unwrap();
        assert_eq!(x.photometries.len(), 1);
        assert_eq!(x.photometries[0].name, "ldc/lum.ldt");
        assert_eq!(x.sensors.len(), 1);
        assert_eq!(x.sensors[0].name, "ldc/pir.ldt");
        assert_eq!(x.sensors[0].content_type, "sensor/sensldt");
    }

    /// Latin-1 LDTs (common for EULUMDAT) are read, not dropped.
    #[test]
    fn latin1_ldt_is_read() {
        // "ä" as the single Latin-1 byte 0xE4 makes the file invalid UTF-8.
        let mut bytes = LDT_TEMPLATE.as_bytes().to_vec();
        let eol = bytes.iter().position(|&c| c == b'\n').unwrap();
        bytes.insert(eol, 0xE4);
        assert!(std::str::from_utf8(&bytes).is_err());
        let x = gldf_extract(&gldf(&[("ldc/a.ldt", &bytes)])).unwrap();
        assert_eq!(x.photometries.len(), 1);
        assert!(x.photometries[0].text.contains('\u{e4}'));
    }

    #[test]
    fn sensor_only_container() {
        let bytes = gldf(&[("sensor/s.ldt", LDT_TEMPLATE.as_bytes())]);
        let x = gldf_extract(&bytes).unwrap();
        assert!(x.photometries.is_empty() && x.best_photometry().is_none());
        assert_eq!(x.sensors.len(), 1);
        // Nothing usable at all is still `None`.
        assert!(gldf_extract(&gldf(&[("image/a.png", b"png")])).is_none());
    }

    /// The synthetic demo GLDF (make_demo_gldf.py): one photometry, two
    /// sensors of both kinds, and both sensor files evaluate.
    #[test]
    fn demo_gldf_with_two_sensors() {
        let bytes = include_bytes!("../../eulumdat/tests/fixtures/sensors/demo_sensors.gldf");
        let x = gldf_extract(bytes).unwrap();
        assert_eq!(x.photometries.len(), 1);
        assert_eq!(x.sensors.len(), 2);
        for s in &x.sensors {
            let f = eulumdat::sensor::SensorFile::from_bytes(&s.bytes, &s.content_type)
                .unwrap_or_else(|e| panic!("{}: {e}", s.name));
            let h = f.default_height();
            assert!(f.areas(h, false).iter().any(|a| a.detected()), "{}", s.name);
            assert!(f.warnings().is_empty(), "{}: {:?}", s.name, f.warnings());
        }
    }
}
