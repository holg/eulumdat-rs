//! `eulumdat emergency`: spacing tables and plan checks against one of the
//! emergency-lighting profiles of `eulumdat::emergency::Limits::all`
//! (`--standard en1838 | gb55037 | …`, `--standard list` prints them).

use anyhow::{bail, Context, Result};
use eulumdat::emergency::{
    self, EmergencyLuminaire, EmergencyPlan, EmergencyReport, EmphasisPoint, EscapeRoute,
    HighRiskArea, Limits, OpenArea, Settings, SpacingRow,
};
use eulumdat::gldf_product::{EmergencyBehaviour, FluxSource, GldfProduct, LuminaireConfig};
use eulumdat::{Eulumdat, IesParser};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct Args {
    pub file: Option<PathBuf>,
    pub flux: Option<f64>,
    pub heights: Vec<f64>,
    pub width: f64,
    pub mf: f64,
    pub variant: Option<String>,
    pub duration: Option<u32>,
    pub plan: Option<PathBuf>,
    pub standard: String,
    pub end_output: f64,
    pub json: bool,
}

pub fn run(a: Args) -> Result<()> {
    if a.standard.eq_ignore_ascii_case("list") {
        for l in Limits::all() {
            println!("{:<12} {} — {}", l.id, l.name, l.region);
            println!("{:<12} {}", "", l.clauses);
        }
        return Ok(());
    }
    let Some(limits) = Limits::by_id(&a.standard) else {
        let ids: Vec<String> = Limits::all().into_iter().map(|l| l.id).collect();
        bail!(
            "unknown standard {:?}: one of {} (or `--standard list`)",
            a.standard,
            ids.join(", ")
        );
    };
    let settings = Settings {
        maintenance_factor: a.mf,
        end_output_share: (a.end_output / 100.0).clamp(0.01, 1.0),
        ..Default::default()
    };
    if let Some(plan) = &a.plan {
        return run_plan(plan, &limits, &settings, a.json);
    }
    let Some(file) = &a.file else {
        bail!("give a luminaire file (.ldt / .ies / .gldf) or --plan plan.json");
    };
    if ext(file) == "gldf" {
        return run_gldf(file, &a, &limits, &settings);
    }
    let ldt = load_luminaire(file)?;
    let flux = a.flux.unwrap_or_else(|| emergency::luminaire_flux_lm(&ldt));
    let rows = emergency::spacing_table(&ldt, flux, &a.heights, a.width, &limits, &settings);
    if a.json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }
    println!(
        "Emergency lighting ({}): {}, {} lm in emergency mode{}, maintenance factor {}",
        limits.name,
        display_name(&ldt, file),
        fmt(flux, 0),
        if a.flux.is_none() {
            " (the file's flux)"
        } else {
            ""
        },
        a.mf
    );
    print_table(&rows, &limits);
    Ok(())
}

fn ext(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn display_name(ldt: &Eulumdat, file: &Path) -> String {
    let n = ldt.luminaire_name.trim();
    if n.is_empty() {
        file.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        n.to_string()
    }
}

fn load_luminaire(path: &Path) -> Result<Eulumdat> {
    match ext(path).as_str() {
        "ies" => IesParser::parse_file(path).context("Failed to parse IES file"),
        _ => Eulumdat::from_file(path).context("Failed to parse LDT file"),
    }
}

/// Text of a file inside a GLDF: UTF-8, else Latin-1.
fn text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn fmt(v: f64, decimals: usize) -> String {
    format!("{v:.decimals$}")
}

fn print_table(rows: &[SpacingRow], l: &Limits) {
    use emergency::PlaceCategory as P;
    if let Some(r) = rows.first() {
        let rl = l.route_limits(P::EscapeCorridor);
        let route = if rl.avg_min_lx > 0.0 {
            format!(
                "avg {} / min {}",
                rl.avg_min_lx,
                rl.centre_min_lx.max(rl.band_min_lx)
            )
        } else {
            format!("{}", rl.centre_min_lx.max(rl.band_min_lx))
        };
        let al = l.area_limits(P::CrowdedPlace);
        let area = if al.avg_min_lx > 0.0 {
            format!("avg {} / min {}", al.avg_min_lx, al.min_lx)
        } else {
            format!("{}", al.min_lx)
        };
        let (route, area) = if l.lux_informative {
            (
                format!("route: spacing rule, ≤ {} m", rl.max_spacing_m),
                format!("area: ≤ {} × h", al.max_spacing_per_height),
            )
        } else {
            (
                format!(
                    "route {} m wide, {route} lx: C0 / C90",
                    fmt(r.route_width_m, 1)
                ),
                format!("open area {area} lx"),
            )
        };
        println!(
            "  {:>5}  {:>9}  {:>27}  {:>15}  {:>18}",
            "h (m)", "E below", route, area, "I 60–90° / limit"
        );
    }
    for r in rows {
        let spacing = |s: f64| {
            if s > 0.0 {
                format!("{} m", fmt(s, 1))
            } else {
                "–".to_string()
            }
        };
        println!(
            "  {:>5}  {:>6} lx  {:>27}  {:>15}  {:>13} cd {}",
            fmt(r.height_m, 1),
            fmt(r.e_nadir_lx, 2),
            format!(
                "{} / {}",
                spacing(r.route_spacing_c0_m),
                spacing(r.route_spacing_c90_m)
            ),
            format!("grid {}", spacing(r.open_area_grid_m)),
            format!(
                "{} / {}",
                fmt(r.glare_i_max_cd, 1),
                if r.glare_limit_cd.is_finite() {
                    fmt(r.glare_limit_cd, 0)
                } else {
                    "none".to_string()
                }
            ),
            if !r.glare_limit_cd.is_finite() {
                " "
            } else if r.glare_i_max_cd <= r.glare_limit_cd {
                "✓"
            } else {
                "✗"
            }
        );
    }
}

// ─── GLDF ────────────────────────────────────────────────────────────

fn read_zip(file: &Path) -> Result<zip::ZipArchive<std::fs::File>> {
    let f = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
    zip::ZipArchive::new(f).context("not a GLDF (zip) container")
}

fn zip_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let wanted = (0..zip.len()).find(|&i| {
        zip.by_index(i).is_ok_and(|f| {
            let n = f.name();
            n == name || n.ends_with(&format!("/{name}"))
        })
    })?;
    let mut f = zip.by_index(wanted).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn source_text(s: FluxSource) -> String {
    match s {
        FluxSource::Duration(h) => format!("DurationTimeAndFlux, {h} h"),
        FluxSource::EmergencyRated => "EmergencyRatedLuminousFlux".into(),
        FluxSource::Rated => "RatedLuminousFlux of an emergency-only emitter".into(),
        FluxSource::BallastLumenFactor => "EmergencyBallastLumenFactor × rated flux".into(),
        FluxSource::None => "not given".into(),
    }
}

fn print_config(i: usize, c: &LuminaireConfig) {
    println!(
        "  {}. {}{}",
        i + 1,
        c.name,
        c.mounting
            .as_deref()
            .map(|m| format!("  ({m})"))
            .unwrap_or_default()
    );
    for l in &c.lamps {
        let mut parts = Vec::new();
        if let Some(n) = l.normal_lm {
            parts.push(format!("normal {} lm", fmt(n, 0)));
        }
        match (l.behaviour, l.emergency_lm) {
            (EmergencyBehaviour::None, _) => parts.push("no emergency light".into()),
            (_, Some(e)) => parts.push(format!(
                "emergency {} lm ({})",
                fmt(e, 0),
                source_text(l.emergency_flux_source)
            )),
            (_, None) => parts.push("emergency flux not given (use --flux)".into()),
        }
        println!(
            "     {:<6} {:<10} {:<14} {}",
            l.slot.as_deref().unwrap_or("-"),
            l.emitter_id,
            l.photometry_file.as_deref().unwrap_or("-"),
            parts.join(", ")
        );
    }
    for s in &c.sensors {
        println!(
            "     {:<6} {:<10} sensor: {}",
            s.slot.as_deref().unwrap_or("-"),
            s.emitter_id,
            s.sensor_file.as_deref().unwrap_or("-")
        );
    }
}

fn run_gldf(file: &Path, a: &Args, limits: &Limits, settings: &Settings) -> Result<()> {
    let mut zip = read_zip(file)?;
    let xml = zip_entry(&mut zip, "product.xml").context("no product.xml in the GLDF")?;
    let product = GldfProduct::parse(&text(&xml)).map_err(anyhow::Error::msg)?;
    let configs = product.configs(a.duration);
    let chosen: Vec<usize> = match &a.variant {
        Some(v) => {
            let i = v
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1))
                .filter(|&i| i < configs.len())
                .or_else(|| {
                    configs
                        .iter()
                        .position(|c| c.variant_id == *v || c.name == *v)
                })
                .with_context(|| format!("no variant {v:?}"))?;
            vec![i]
        }
        None => (0..configs.len())
            .filter(|&i| configs[i].has_emergency())
            .collect(),
    };
    if a.json {
        let out: Vec<serde_json::Value> = chosen
            .iter()
            .map(|&i| {
                let c = &configs[i];
                let tables: Vec<serde_json::Value> = c
                    .emergency_lamps()
                    .filter_map(|l| {
                        let flux = a.flux.or(l.emergency_lm)?;
                        let ldt = l
                            .photometry_file
                            .as_deref()
                            .and_then(|f| zip_entry(&mut zip, f))
                            .and_then(|b| Eulumdat::parse(&text(&b)).ok())?;
                        let rows =
                            emergency::spacing_table(&ldt, flux, &a.heights, a.width, limits, settings);
                        Some(serde_json::json!({ "emitter": l.emitter_id, "flux_lm": flux, "rows": rows }))
                    })
                    .collect();
                serde_json::json!({ "config": c, "spacing": tables })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    println!(
        "{} {}: {} variant(s)",
        product.manufacturer.as_deref().unwrap_or(""),
        product.product_name.as_deref().unwrap_or(""),
        configs.len()
    );
    for (i, c) in configs.iter().enumerate() {
        print_config(i, c);
    }
    if chosen.is_empty() {
        println!("\nNo variant has emergency emitters.");
        return Ok(());
    }
    for &i in &chosen {
        let c = &configs[i];
        for l in c.emergency_lamps() {
            println!(
                "\nVariant {}, {} ({}):",
                i + 1,
                l.emitter_id,
                l.photometry_file.as_deref().unwrap_or("-")
            );
            let Some(flux) = a.flux.or(l.emergency_lm) else {
                println!("  emergency flux not given in the GLDF: use --flux");
                continue;
            };
            let Some(bytes) = l
                .photometry_file
                .as_deref()
                .and_then(|f| zip_entry(&mut zip, f))
            else {
                println!("  photometry file missing in the container");
                continue;
            };
            let ldt = match Eulumdat::parse(&text(&bytes)) {
                Ok(l) => l,
                Err(e) => {
                    println!("  photometry not readable: {e}");
                    continue;
                }
            };
            // the GLDF flux wins; say so when the file disagrees
            let file_flux = emergency::luminaire_flux_lm(&ldt);
            let gldf_flux = l.normal_lm.or(l.emergency_lm);
            if let Some(g) = gldf_flux.filter(|g| *g > 0.0 && file_flux > 0.0) {
                let k = g / file_flux;
                if !(0.9..=1.1).contains(&k) {
                    println!(
                        "  note: the photometry file gives {} lm, the GLDF emitter {} lm (× {}); scaled to the GLDF flux",
                        fmt(file_flux, 0),
                        fmt(g, 0),
                        fmt(k, 1)
                    );
                }
            }
            println!(
                "  {} lm in emergency mode, maintenance factor {}{}",
                fmt(flux, 0),
                settings.maintenance_factor,
                l.ra.map(|r| format!(", Ra {} (≥ {} needed)", fmt(r, 0), fmt(limits.min_ra, 0)))
                    .unwrap_or_default()
            );
            print_table(
                &emergency::spacing_table(&ldt, flux, &a.heights, a.width, limits, settings),
                limits,
            );
        }
    }
    Ok(())
}

// ─── plan ────────────────────────────────────────────────────────────

fn num(v: &serde_json::Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}

fn list<T: serde::de::DeserializeOwned>(v: &serde_json::Value, k: &str) -> Result<Vec<T>> {
    match v.get(k) {
        Some(x) => serde_json::from_value(x.clone()).with_context(|| format!("plan: \"{k}\"")),
        None => Ok(Vec::new()),
    }
}

fn run_plan(path: &Path, limits: &Limits, settings: &Settings, json: bool) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
    )
    .context("plan: not JSON")?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut settings = settings.clone();
    if let Some(mf) = num(&v, "maintenance_factor") {
        settings.maintenance_factor = mf;
    }
    let lum_entries = v
        .get("luminaires")
        .and_then(|l| l.as_array())
        .context("plan: \"luminaires\" missing")?;
    // each distinct file once
    let mut files: Vec<(String, Eulumdat)> = Vec::new();
    for l in lum_entries {
        let f = l
            .get("file")
            .and_then(|f| f.as_str())
            .context("plan: luminaire without \"file\"")?;
        if !files.iter().any(|(n, _)| n == f) {
            files.push((f.to_string(), load_luminaire(&dir.join(f))?));
        }
    }
    let mut luminaires = Vec::new();
    for l in lum_entries {
        let f = l.get("file").and_then(|f| f.as_str()).unwrap_or_default();
        let ldt = &files.iter().find(|(n, _)| n == f).expect("loaded").1;
        luminaires.push(EmergencyLuminaire {
            ldt,
            flux_lm: num(l, "flux_lm").unwrap_or_else(|| emergency::luminaire_flux_lm(ldt)),
            x: num(l, "x").context("plan: luminaire without x")?,
            y: num(l, "y").context("plan: luminaire without y")?,
            height: num(l, "height").context("plan: luminaire without height")?,
            rotation: num(l, "rotation").unwrap_or(0.0),
            ra: num(l, "ra"),
        });
    }
    let plan = EmergencyPlan {
        luminaires,
        routes: list::<EscapeRoute>(&v, "routes")?,
        open_areas: list::<OpenArea>(&v, "open_areas")?,
        high_risk: list::<HighRiskArea>(&v, "high_risk")?,
        points: list::<EmphasisPoint>(&v, "points")?,
    };
    let report = emergency::evaluate(&plan, limits, &settings);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report, limits);
    }
    if !report.pass() {
        std::process::exit(2);
    }
    Ok(())
}

fn mark(ok: bool) -> &'static str {
    if ok {
        "✓"
    } else {
        "✗"
    }
}

fn print_report(r: &EmergencyReport, l: &Limits) {
    use emergency::{CheckKind as K, Subject};
    println!(
        "Emergency lighting check: {} ({}), direct light only, maintenance factor {}",
        l.name, l.region, r.settings.maintenance_factor
    );
    println!("  {}", l.clauses);
    for n in &l.notes {
        println!("  {n}");
    }
    let mut group = "";
    // points without a rule of their own (exits …): where the light is,
    // at the end of the points' group
    let mut others_shown = false;
    let others = |group: &mut &str| {
        let mut lines = r
            .points
            .iter()
            .filter(|p| p.vertical_lx.is_none())
            .peekable();
        if lines.peek().is_some() && *group != "Points of emphasis" {
            println!("\nPoints of emphasis");
            *group = "Points of emphasis";
        }
        for p in lines {
            println!(
                "  {} at ({}, {}): nearest luminaire {}",
                p.kind.words(),
                fmt(p.x, 1),
                fmt(p.y, 1),
                p.nearest_luminaire_m
                    .map(|d| format!("{} m", fmt(d, 1)))
                    .unwrap_or_else(|| "–".into())
            );
        }
    };
    for c in &r.checks {
        let g = match c.subject {
            Subject::Route(_) => "Escape routes",
            Subject::Area(_) if l.anti_panic.applies_above_m2 > 0.0 => "Open areas (anti-panic)",
            Subject::Area(_) => "Rooms",
            Subject::HighRisk(_) => "High-risk task areas",
            Subject::Point(_) => "Points of emphasis",
            Subject::Luminaires => "Luminaires",
        };
        if g == "Luminaires" && !others_shown {
            others(&mut group);
            others_shown = true;
        }
        if g != group {
            println!("\n{g}");
            group = g;
        }
        let [what, value, required] = r.describe(c, l);
        let mark = match c.pass {
            Some(ok) => mark(ok),
            None => "",
        };
        println!("  {what}: {value} ({required}) {mark}");
        let darkest = matches!(
            c.kind,
            K::CentreMin | K::BandMin | K::AreaMin | K::HighRiskMin | K::EndMin
        );
        if let (Some(false), true, Some(at)) = (c.pass, darkest, c.at) {
            println!("    darkest point at ({}, {})", fmt(at.0, 1), fmt(at.1, 1));
        }
        if let (K::Vertical, Subject::Point(i)) = (c.kind, c.subject) {
            if r.points.get(i).is_some_and(|p| p.assumed_facing) {
                println!("    no facing given: the best of eight directions");
            }
        }
    }
    if !others_shown {
        others(&mut group);
    }
    if l.checks_glare() && !r.glare.is_empty() {
        println!("\nDisability glare (60°–90°), per luminaire");
        for g in &r.glare {
            println!(
                "  luminaire {}: {} cd (≤ {}, {}{}) {}",
                g.luminaire + 1,
                fmt(g.i_max_cd, 1),
                fmt(g.limit_cd, 0),
                if g.high_risk {
                    "high-risk column"
                } else {
                    "escape / anti-panic column"
                },
                if g.route_directions {
                    ", along the route"
                } else {
                    ", all directions"
                },
                mark(g.pass)
            );
        }
    }
    if !r.ra_below_limit.is_empty() {
        println!(
            "\nRa below {}: luminaire(s) {:?}",
            l.min_ra,
            r.ra_below_limit.iter().map(|i| i + 1).collect::<Vec<_>>()
        );
    }
    if !r.ra_unknown.is_empty() {
        println!(
            "\nRa unknown (≥ {} required): luminaire(s) {:?}",
            l.min_ra,
            r.ra_unknown.iter().map(|i| i + 1).collect::<Vec<_>>()
        );
    }
    println!(
        "\nResult: {}",
        if !r.has_checks() {
            "nothing to check"
        } else if r.pass() {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!("Not calculated: {}", l.not_calculated());
}
