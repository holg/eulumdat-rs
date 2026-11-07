//! GLDF `product.xml`: emitters (normal light, emergency behaviour,
//! sensors) and variants, resolved into luminaire configurations.
//!
//! A GLDF variant puts emitters into the light-emitting objects of its L3D
//! (`EmitterReference` + `EmitterObjectExternalName`). One placed variant
//! can then give the normal light (EN 12464-1), the emergency light
//! (EN 1838) and the presence detection of the same luminaire. Only the
//! elements needed for that are read (no gldf-rs dependency); the zip is
//! the caller's business.
//!
//! Emergency flux of an emitter, first that exists: the product's or
//! variant's `DurationTimeAndFlux` entry for the chosen duration →
//! `EmergencyRatedLuminousFlux` (emitter, light source, control gear) →
//! for emergency-only emitters their `RatedLuminousFlux` →
//! `EmergencyBallastLumenFactor` × rated flux → unknown (user input).

use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<Node>,
}

impl Node {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.as_str())
    }
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
    fn text_of(&self, name: &str) -> Option<String> {
        self.child(name)
            .map(|c| c.text.trim().to_string())
            .filter(|t| !t.is_empty())
    }
    fn num_of(&self, name: &str) -> Option<f64> {
        self.text_of(name)
            .and_then(|t| t.replace(',', ".").parse().ok())
    }
    /// All descendants with this name (depth first).
    fn find_all<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        for c in &self.children {
            if c.name == name {
                out.push(c);
            }
            c.find_all(name, out);
        }
    }
    fn first(&self, name: &str) -> Option<&Node> {
        let mut v = Vec::new();
        self.find_all(name, &mut v);
        v.into_iter().next()
    }
    /// Text of a `<Locale>` list: English, else the first.
    fn locale_text(&self) -> Option<String> {
        let locales: Vec<&Node> = self.children_named("Locale").collect();
        locales
            .iter()
            .find(|l| l.attr("language").is_some_and(|x| x.starts_with("en")))
            .or(locales.first())
            .map(|l| l.text.trim().to_string())
            .filter(|t| !t.is_empty())
    }
}

fn parse_tree(xml: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Node> = vec![Node::default()];
    let open = |e: &quick_xml::events::BytesStart| -> Node {
        let mut n = Node {
            name: String::from_utf8_lossy(e.local_name().as_ref()).to_string(),
            ..Default::default()
        };
        for a in e.attributes().flatten() {
            n.attrs.push((
                String::from_utf8_lossy(a.key.local_name().as_ref()).to_string(),
                a.unescape_value()
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ));
        }
        n
    };
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => stack.push(open(&e)),
            Ok(Event::Empty(e)) => {
                let n = open(&e);
                if let Some(top) = stack.last_mut() {
                    top.children.push(n);
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text
                        .push_str(&t.unescape().map(|s| s.to_string()).unwrap_or_default());
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let n = stack.pop().expect("stack");
                    if let Some(top) = stack.last_mut() {
                        top.children.push(n);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("product.xml: {e}")),
            _ => {}
        }
    }
    let root = stack.into_iter().next().unwrap_or_default();
    root.children
        .into_iter()
        .next()
        .ok_or_else(|| "product.xml: empty".to_string())
}

/// What an emitter does in emergency mode (GLDF `emergencyBehaviour`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EmergencyBehaviour {
    /// Normal light only.
    #[default]
    None,
    /// Normal light, and emergency light at a lower flux.
    Combined,
    /// Emergency light only.
    EmergencyOnly,
}

impl EmergencyBehaviour {
    fn parse(s: Option<&str>) -> Self {
        match s.map(|s| s.trim().to_ascii_lowercase()) {
            Some(s) if s == "combined" => Self::Combined,
            Some(s) if s == "emergencyonly" || s == "emergency_only" => Self::EmergencyOnly,
            _ => Self::None,
        }
    }
}

/// A `<File>` of the container.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfFile {
    pub id: String,
    pub content_type: String,
    /// File name (inside the folder of its content type) or URL.
    pub name: String,
}

/// Light source data that matters here.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfLightSource {
    pub id: String,
    pub ra: Option<f64>,
    pub cct: Option<f64>,
    pub rated_lm: Option<f64>,
    pub emergency_rated_lm: Option<f64>,
    pub emergency_ballast_lumen_factor: Option<f64>,
}

/// Control gear data that matters here.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfControlGear {
    pub id: String,
    pub emergency_rated_lm: Option<f64>,
    pub emergency_ballast_lumen_factor: Option<f64>,
}

/// An emitter: light (with its emergency behaviour) or a sensor.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GldfEmitterKind {
    Light {
        behaviour: EmergencyBehaviour,
        photometry: Option<String>,
        light_source: Option<String>,
        control_gear: Option<String>,
        rated_lm: Option<f64>,
        emergency_rated_lm: Option<f64>,
        emergency_ballast_lumen_factor: Option<f64>,
    },
    Sensor {
        sensor: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfEmitter {
    pub id: String,
    pub kind: GldfEmitterKind,
}

/// A variant: which emitter sits in which object of the geometry.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfVariant {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// e.g. "Ceiling/Recessed".
    pub mounting: Option<String>,
    /// (emitter id, light-emitting / sensor object name)
    pub slots: Vec<(String, Option<String>)>,
    /// `DurationTimeAndFlux` of the variant: (hours, lm).
    pub emergency_durations: Vec<(u32, f64)>,
    pub dedicated_emergency_type: Option<String>,
}

/// The parts of a GLDF `product.xml` used for luminaire configurations.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GldfProduct {
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub files: Vec<GldfFile>,
    /// (photometry id, file id)
    pub photometries: Vec<(String, String)>,
    /// (sensor id, file id)
    pub sensors: Vec<(String, String)>,
    pub light_sources: Vec<GldfLightSource>,
    pub control_gears: Vec<GldfControlGear>,
    pub emitters: Vec<GldfEmitter>,
    pub variants: Vec<GldfVariant>,
    /// Product-level `DurationTimeAndFlux`: (hours, lm).
    pub emergency_durations: Vec<(u32, f64)>,
    pub dedicated_emergency_type: Option<String>,
}

fn durations(n: &Node) -> Vec<(u32, f64)> {
    let mut out: Vec<(u32, f64)> = n
        .first("DurationTimeAndFlux")
        .map(|d| {
            d.children_named("Flux")
                .filter_map(|f| {
                    let h: u32 = f.attr("hours")?.trim().parse().ok()?;
                    let lm: f64 = f.text.trim().replace(',', ".").parse().ok()?;
                    Some((h, lm))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|(h, _)| *h);
    out
}

impl GldfProduct {
    /// Read a `product.xml`.
    pub fn parse(xml: &str) -> Result<Self, String> {
        let root = parse_tree(xml)?;
        let mut p = GldfProduct {
            manufacturer: root.child("Header").and_then(|h| h.text_of("Manufacturer")),
            ..Default::default()
        };
        let general = root.child("GeneralDefinitions");
        if let Some(g) = general {
            if let Some(files) = g.child("Files") {
                p.files = files
                    .children_named("File")
                    .map(|f| GldfFile {
                        id: f.attr("id").unwrap_or_default().to_string(),
                        content_type: f.attr("contentType").unwrap_or_default().to_string(),
                        name: f.text.trim().to_string(),
                    })
                    .collect();
            }
            if let Some(ph) = g.child("Photometries") {
                p.photometries = ph
                    .children_named("Photometry")
                    .filter_map(|x| {
                        let id = x.attr("id")?.to_string();
                        let file = x
                            .child("PhotometryFileReference")?
                            .attr("fileId")?
                            .to_string();
                        Some((id, file))
                    })
                    .collect();
            }
            if let Some(se) = g.child("Sensors") {
                p.sensors = se
                    .children_named("Sensor")
                    .filter_map(|x| {
                        let id = x.attr("id")?.to_string();
                        let file = x.child("SensorFileReference")?.attr("fileId")?.to_string();
                        Some((id, file))
                    })
                    .collect();
            }
            if let Some(ls) = g.child("LightSources") {
                p.light_sources = ls
                    .children
                    .iter()
                    .filter(|x| x.name.ends_with("LightSource"))
                    .map(|x| {
                        let colour = x.child("ColorInformation");
                        GldfLightSource {
                            id: x.attr("id").unwrap_or_default().to_string(),
                            ra: colour.and_then(|c| c.num_of("ColorRenderingIndex")),
                            cct: colour.and_then(|c| c.num_of("CorrelatedColorTemperature")),
                            rated_lm: x.num_of("RatedLuminousFlux"),
                            emergency_rated_lm: x.num_of("EmergencyRatedLuminousFlux"),
                            emergency_ballast_lumen_factor: x.num_of("EmergencyBallastLumenFactor"),
                        }
                    })
                    .collect();
            }
            if let Some(cg) = g.child("ControlGears") {
                p.control_gears = cg
                    .children_named("ControlGear")
                    .map(|x| GldfControlGear {
                        id: x.attr("id").unwrap_or_default().to_string(),
                        emergency_rated_lm: x.num_of("EmergencyRatedLuminousFlux"),
                        emergency_ballast_lumen_factor: x.num_of("EmergencyBallastLumenFactor"),
                    })
                    .collect();
            }
            if let Some(em) = g.child("Emitters") {
                p.emitters = em
                    .children_named("Emitter")
                    .filter_map(|e| {
                        let id = e.attr("id")?.to_string();
                        if let Some(s) = e.child("SensorEmitter") {
                            return Some(GldfEmitter {
                                id,
                                kind: GldfEmitterKind::Sensor {
                                    sensor: s
                                        .child("SensorReference")
                                        .and_then(|r| r.attr("sensorId"))
                                        .map(str::to_string),
                                },
                            });
                        }
                        let l = e
                            .children
                            .iter()
                            .find(|c| c.name.ends_with("LightEmitter"))?;
                        let gear = l
                            .child("ControlGearReference")
                            .and_then(|r| r.attr("controlGearId"))
                            .map(str::to_string);
                        let source = l.child("LightSourceReference").and_then(|r| {
                            r.attr("fixedLightSourceId")
                                .or(r.attr("changeableLightSourceId"))
                                .or(r.attr("lightSourceId"))
                                .map(str::to_string)
                        });
                        Some(GldfEmitter {
                            id,
                            kind: GldfEmitterKind::Light {
                                behaviour: EmergencyBehaviour::parse(l.attr("emergencyBehaviour")),
                                photometry: l
                                    .child("PhotometryReference")
                                    .and_then(|r| r.attr("photometryId"))
                                    .map(str::to_string),
                                light_source: source,
                                control_gear: gear,
                                rated_lm: l.num_of("RatedLuminousFlux"),
                                emergency_rated_lm: l.num_of("EmergencyRatedLuminousFlux"),
                                emergency_ballast_lumen_factor: l
                                    .num_of("EmergencyBallastLumenFactor"),
                            },
                        })
                    })
                    .collect();
            }
        }
        if let Some(defs) = root.child("ProductDefinitions") {
            if let Some(meta) = defs.child("ProductMetaData") {
                p.product_name = meta.child("Name").and_then(Node::locale_text);
                if let Some(attrs) = meta.child("DescriptiveAttributes") {
                    p.emergency_durations = durations(attrs);
                    p.dedicated_emergency_type = attrs
                        .first("DedicatedEmergencyLightingType")
                        .map(|n| n.text.trim().to_string());
                }
            }
            if let Some(vs) = defs.child("Variants") {
                p.variants = vs
                    .children_named("Variant")
                    .map(|v| {
                        let mut slots = Vec::new();
                        let mut refs = Vec::new();
                        if let Some(geo) = v.child("Geometry") {
                            geo.find_all("EmitterReference", &mut refs);
                        }
                        for r in refs {
                            if let Some(id) = r.attr("emitterId") {
                                slots
                                    .push((id.to_string(), r.text_of("EmitterObjectExternalName")));
                            }
                        }
                        let mounting = v.child("Mountings").and_then(|m| {
                            let first = m.children.first()?;
                            Some(match first.children.first() {
                                Some(c) => format!("{}/{}", first.name, c.name),
                                None => first.name.clone(),
                            })
                        });
                        let attrs = v.child("DescriptiveAttributes");
                        GldfVariant {
                            id: v.attr("id").unwrap_or_default().to_string(),
                            name: v.child("Name").and_then(Node::locale_text),
                            description: v.child("Description").and_then(Node::locale_text),
                            mounting,
                            slots,
                            emergency_durations: attrs.map(durations).unwrap_or_default(),
                            dedicated_emergency_type: attrs
                                .and_then(|a| a.first("DedicatedEmergencyLightingType"))
                                .map(|n| n.text.trim().to_string()),
                        }
                    })
                    .collect();
            }
        }
        Ok(p)
    }

    /// File name of a file id.
    pub fn file_name(&self, file_id: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|f| f.id == file_id)
            .map(|f| f.name.as_str())
    }

    /// File name of a photometry.
    pub fn photometry_file(&self, photometry_id: &str) -> Option<&str> {
        let file = self
            .photometries
            .iter()
            .find(|(id, _)| id == photometry_id)
            .map(|(_, f)| f.as_str())?;
        self.file_name(file)
    }

    /// File name of a sensor.
    pub fn sensor_file(&self, sensor_id: &str) -> Option<&str> {
        let file = self
            .sensors
            .iter()
            .find(|(id, _)| id == sensor_id)
            .map(|(_, f)| f.as_str())?;
        self.file_name(file)
    }

    fn emitter(&self, id: &str) -> Option<&GldfEmitter> {
        self.emitters.iter().find(|e| e.id == id)
    }

    /// Every variant as a luminaire configuration; `duration_h` picks the
    /// `DurationTimeAndFlux` entry (default: the shortest listed).
    pub fn configs(&self, duration_h: Option<u32>) -> Vec<LuminaireConfig> {
        self.variants
            .iter()
            .map(|v| self.config(v, duration_h))
            .collect()
    }

    /// One variant as a luminaire configuration.
    pub fn config(&self, v: &GldfVariant, duration_h: Option<u32>) -> LuminaireConfig {
        let table = if v.emergency_durations.is_empty() {
            &self.emergency_durations
        } else {
            &v.emergency_durations
        };
        let by_duration = pick_duration(table, duration_h);
        let mut cfg = LuminaireConfig {
            variant_id: v.id.clone(),
            name: v.name.clone().unwrap_or_else(|| v.id.clone()),
            description: v.description.clone(),
            mounting: v.mounting.clone(),
            lamps: Vec::new(),
            sensors: Vec::new(),
            dedicated_emergency_type: v
                .dedicated_emergency_type
                .clone()
                .or(self.dedicated_emergency_type.clone()),
        };
        for (emitter_id, slot) in &v.slots {
            let Some(e) = self.emitter(emitter_id) else {
                continue;
            };
            match &e.kind {
                GldfEmitterKind::Sensor { sensor } => cfg.sensors.push(ConfigSensor {
                    emitter_id: e.id.clone(),
                    slot: slot.clone(),
                    sensor_file: sensor
                        .as_deref()
                        .and_then(|s| self.sensor_file(s))
                        .map(str::to_string),
                }),
                GldfEmitterKind::Light {
                    behaviour,
                    photometry,
                    light_source,
                    control_gear,
                    rated_lm,
                    emergency_rated_lm,
                    emergency_ballast_lumen_factor,
                } => {
                    let ls = light_source
                        .as_deref()
                        .and_then(|id| self.light_sources.iter().find(|l| l.id == id));
                    let cg = control_gear
                        .as_deref()
                        .and_then(|id| self.control_gears.iter().find(|g| g.id == id));
                    let rated = rated_lm.or(ls.and_then(|l| l.rated_lm));
                    let emergency_rated = emergency_rated_lm
                        .or(ls.and_then(|l| l.emergency_rated_lm))
                        .or(cg.and_then(|g| g.emergency_rated_lm));
                    let blf = emergency_ballast_lumen_factor
                        .or(ls.and_then(|l| l.emergency_ballast_lumen_factor))
                        .or(cg.and_then(|g| g.emergency_ballast_lumen_factor));
                    let (emergency_lm, source) = match behaviour {
                        EmergencyBehaviour::None => (None, FluxSource::None),
                        b => {
                            if let Some((h, lm)) = by_duration {
                                (Some(lm), FluxSource::Duration(h))
                            } else if let Some(lm) = emergency_rated {
                                (Some(lm), FluxSource::EmergencyRated)
                            } else if *b == EmergencyBehaviour::EmergencyOnly && rated.is_some() {
                                (rated, FluxSource::Rated)
                            } else if let (Some(f), Some(r)) = (blf, rated) {
                                // given as a fraction or as a percentage
                                let f = if f > 1.0 { f / 100.0 } else { f };
                                (Some(f * r), FluxSource::BallastLumenFactor)
                            } else {
                                (None, FluxSource::None)
                            }
                        }
                    };
                    cfg.lamps.push(ConfigLamp {
                        emitter_id: e.id.clone(),
                        slot: slot.clone(),
                        photometry_file: photometry
                            .as_deref()
                            .and_then(|p| self.photometry_file(p))
                            .map(str::to_string),
                        behaviour: *behaviour,
                        normal_lm: match behaviour {
                            EmergencyBehaviour::EmergencyOnly => None,
                            _ => rated,
                        },
                        emergency_lm,
                        emergency_flux_source: source,
                        ra: ls.and_then(|l| l.ra),
                        cct: ls.and_then(|l| l.cct),
                    });
                }
            }
        }
        cfg
    }
}

/// The `DurationTimeAndFlux` entry for a duration: exact, else the next
/// longer one, else the longest; without a duration the shortest.
fn pick_duration(table: &[(u32, f64)], duration_h: Option<u32>) -> Option<(u32, f64)> {
    match duration_h {
        None => table.first().copied(),
        Some(d) => table
            .iter()
            .find(|(h, _)| *h >= d)
            .or(table.last())
            .copied(),
    }
}

/// Where an emitter's emergency flux came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FluxSource {
    /// `DurationTimeAndFlux` for this many hours.
    Duration(u32),
    /// `EmergencyRatedLuminousFlux`.
    EmergencyRated,
    /// `RatedLuminousFlux` of an emergency-only emitter.
    Rated,
    /// `EmergencyBallastLumenFactor` × rated flux.
    BallastLumenFactor,
    /// Not given (normal light only, or the user has to enter it).
    None,
}

/// A lamp of a configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfigLamp {
    pub emitter_id: String,
    /// Light-emitting object of the geometry (e.g. "LEO0").
    pub slot: Option<String>,
    /// Photometry file name (e.g. "wide.ldt").
    pub photometry_file: Option<String>,
    pub behaviour: EmergencyBehaviour,
    /// Flux in normal mode (lm); `None` for emergency-only emitters.
    pub normal_lm: Option<f64>,
    /// Flux in emergency mode (lm).
    pub emergency_lm: Option<f64>,
    pub emergency_flux_source: FluxSource,
    pub ra: Option<f64>,
    pub cct: Option<f64>,
}

/// A sensor of a configuration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfigSensor {
    pub emitter_id: String,
    pub slot: Option<String>,
    pub sensor_file: Option<String>,
}

/// A GLDF variant resolved: what the placed luminaire does in normal and
/// emergency mode, and which sensor it carries.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LuminaireConfig {
    pub variant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub mounting: Option<String>,
    pub lamps: Vec<ConfigLamp>,
    pub sensors: Vec<ConfigSensor>,
    pub dedicated_emergency_type: Option<String>,
}

impl LuminaireConfig {
    /// Lamps that light in emergency mode.
    pub fn emergency_lamps(&self) -> impl Iterator<Item = &ConfigLamp> {
        self.lamps
            .iter()
            .filter(|l| l.behaviour != EmergencyBehaviour::None)
    }

    /// Lamps that light in normal mode.
    pub fn normal_lamps(&self) -> impl Iterator<Item = &ConfigLamp> {
        self.lamps
            .iter()
            .filter(|l| l.behaviour != EmergencyBehaviour::EmergencyOnly)
    }

    pub fn has_emergency(&self) -> bool {
        self.emergency_lamps().next().is_some()
    }

    pub fn has_sensor(&self) -> bool {
        !self.sensors.is_empty()
    }
}
