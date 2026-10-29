//! L3D / GLDF readers for the luminaire body (feature `l3d`).
//!
//! `mesh_source_from_l3d` turns an L3D archive into a [`MeshSource`];
//! `gldf_extract` pulls the first photometry file and the first L3D archive
//! out of a GLDF container without parsing `product.xml` (variant selection
//! is the editor's job).

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

/// What the Light scene needs from a GLDF container.
#[derive(Clone, Debug, Default)]
pub struct GldfExtract {
    /// Every `.ldt` / `.ies` in the container, in zip order.
    pub photometries: Vec<GldfPhotometry>,
    /// First `.l3d` archive in the container, if any.
    pub l3d: Option<Vec<u8>>,
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

/// Photometry files and the first L3D archive of a GLDF container.
pub fn gldf_extract(bytes: &[u8]) -> Option<GldfExtract> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut out = GldfExtract::default();
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else {
            continue;
        };
        if !f.is_file() {
            continue;
        }
        let name = f.name().to_string();
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".ldt") || lower.ends_with(".ies") {
            let mut text = String::new();
            if f.read_to_string(&mut text).is_ok() {
                out.photometries.push(GldfPhotometry { name, text });
            }
        } else if lower.ends_with(".l3d") && out.l3d.is_none() {
            let mut b = Vec::new();
            if f.read_to_end(&mut b).is_ok() {
                out.l3d = Some(b);
            }
        }
    }
    (!out.photometries.is_empty()).then_some(out)
}
