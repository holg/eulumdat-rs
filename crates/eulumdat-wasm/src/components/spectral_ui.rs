//! Shared building blocks for the two spectral tabs — **Spectrum Lab**
//! (luminaire SPD analysis) and **Daylight & Sky** (location/sun/moon/sky).
//!
//! Both tabs run the same Monte Carlo tracer and present the same family of
//! metric cards, so the trace functions, the [`SpectralResult`] they produce,
//! the vendor-SPD presets, and the card/result rendering all live here.

use eulumdat::Eulumdat;
use eulumdat_goniosim::nalgebra::{Point3, Rotation3};
use eulumdat_goniosim::*;
use eulumdat_spectrum::mesopic::mesopic_luminance;
use leptos::prelude::*;

// ── Vendor SPD presets ───────────────────────────────────────────────────

/// Real measured vendor SPDs, embedded at compile time. Curated to tell the
/// spectral story: a warm/cool high-CRI LED pair (Luxeon 90-CRI), the Yuji WB
/// Day-vs-Nite pair (purpose-built for S/P demonstration), and a Signify lab
/// spectrum. Each is parsed on demand via `eulumdat::parse_spd`.
pub(crate) const PRESETS: &[(&str, &str)] = &[
    (
        "Yuji WB Nite 2200K (amber, low S/P)",
        include_str!("../../../../docs/SPDs/Youji-Nite/Yuji_WB_Nite_2200K.csv"),
    ),
    (
        "Yuji WB Nite 2700K",
        include_str!("../../../../docs/SPDs/Youji-Nite/Yuji_WB_Nite_2700K.csv"),
    ),
    (
        "Luxeon 90-CRI 3000K (warm LED)",
        include_str!("../../../../docs/SPDs/Luxeon_SPD_fixed/Luxeon_90_3000K.spd"),
    ),
    (
        "Luxeon 90-CRI 4000K (neutral LED)",
        include_str!("../../../../docs/SPDs/Luxeon_SPD_fixed/Luxeon_90_4000K.spd"),
    ),
    (
        "Yuji WB Day 5000K (daylight)",
        include_str!("../../../../docs/SPDs/Youji-Nite/Yuji_WB_Day_5000K.csv"),
    ),
    (
        "Luxeon 90-CRI 6500K (cool LED, high S/P)",
        include_str!("../../../../docs/SPDs/Luxeon_SPD_fixed/Luxeon_90_6500K.spd"),
    ),
    (
        "Signify tunable (retail 'bread')",
        include_str!("../../../../docs/SPDs/Signify/Signify bread .csv"),
    ),
];

/// A source-spectrum preset — an index into [`PRESETS`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpdPreset(pub usize);

impl SpdPreset {
    pub(crate) fn label(&self) -> &'static str {
        PRESETS.get(self.0).map(|p| p.0).unwrap_or("SPD")
    }

    /// Parse the embedded vendor SPD into a `SourceSpectrum`. Falls back to a
    /// 3000 K synthetic spectrum if the file fails to parse.
    pub(crate) fn spectrum(&self) -> SourceSpectrum {
        match PRESETS.get(self.0) {
            Some((_, content)) => match eulumdat::parse_spd(content) {
                Ok(loaded) => SourceSpectrum::from_eulumdat(&loaded.spd),
                Err(_) => SourceSpectrum::from_cct(3000.0),
            },
            None => SourceSpectrum::from_cct(3000.0),
        }
    }

    pub(crate) fn index(&self) -> usize {
        self.0
    }
    pub(crate) fn from_index(i: usize) -> Self {
        SpdPreset(i.min(PRESETS.len().saturating_sub(1)))
    }
}

// ── The metric bundle a trace produces ───────────────────────────────────

/// The metrics computed from one spectral trace.
#[derive(Clone, Default)]
pub(crate) struct SpectralResult {
    pub cct_k: f64,
    pub duv: f64,
    pub sp_ratio: f64,
    pub melanopic_der: f64,
    pub mesopic_road: f64,
    pub ulor: f64,
    pub blue_pct: f64,
    pub upward_cct: f64,
    pub meets_darksky: bool,
    pub photons: u64,
    pub detected_pct: f64,
    /// True if this result came from a daylight/nightlight sky trace.
    pub sky_mode: bool,
    /// Open-plane illuminance from the sky (lux), sky mode only.
    pub sky_lux: f64,
    /// Sun altitude that drove the sky (degrees), sky mode only.
    pub sun_altitude: f64,
    /// Whether it was daytime, sky mode only.
    pub is_day: bool,
    /// Ground-illuminance regime key (day / twilight / moonlit / night).
    pub regime: &'static str,
}

impl SpectralResult {
    /// Record the sky's target illuminance if the trace produced no reading.
    fn with_target(mut self, target_lux: f64) -> Self {
        if self.sky_lux <= 0.0 {
            self.sky_lux = target_lux;
        }
        self
    }
}

// ── Trace functions ──────────────────────────────────────────────────────

/// Trace a luminaire (editor LDT or a bare isotropic emitter) carrying the
/// chosen vendor spectrum, and derive the full spectral metric set.
pub(crate) fn run_luminaire_trace(
    p: &SpdPreset,
    src: &Option<Eulumdat>,
    n: u64,
    road_l: f64,
) -> SpectralResult {
    let mut scene = Scene::new();
    match src {
        Some(ldt) if !ldt.intensities.is_empty() => {
            let flux = ldt.total_luminous_flux().max(1000.0);
            scene.add_source_with_spectrum(
                Source::from_lvk(Point3::origin(), Rotation3::identity(), ldt.clone(), flux),
                p.spectrum(),
            );
        }
        _ => {
            scene.add_source_with_spectrum(
                Source::Isotropic {
                    position: Point3::origin(),
                    flux_lm: 10_000.0,
                },
                p.spectrum(),
            );
        }
    }
    let cfg = TracerConfig {
        num_photons: n,
        max_bounces: 4,
        detector_c_resolution: 15.0,
        detector_g_resolution: 5.0,
        seed: 42,
        ..TracerConfig::default()
    };
    let res = Tracer::trace(&scene, &cfg);
    let channels = res.channels.expect("spectral scene → channels");
    let col = channels.integrated_colorimetry();
    let sp = channels.integrated_sp_ratio();
    let dark = DarkSkyReport::from_channels(&channels);
    let mel_der = eulumdat_spectrum::melanopic_der(p.spectrum().spd());

    SpectralResult {
        cct_k: col.map(|c| c.cct_k).unwrap_or(0.0),
        duv: col.map(|c| c.duv).unwrap_or(0.0),
        sp_ratio: sp,
        melanopic_der: mel_der,
        mesopic_road: mesopic_luminance(road_l, sp),
        ulor: dark.ulor * 100.0,
        blue_pct: dark.blue_fraction_up * 100.0,
        upward_cct: dark.upward_cct_k.unwrap_or(0.0),
        meets_darksky: dark.meets_cct_limit(3000.0),
        photons: res.stats.photons_traced,
        detected_pct: if res.stats.photons_traced > 0 {
            res.stats.photons_detected as f64 / res.stats.photons_traced as f64 * 100.0
        } else {
            0.0
        },
        sky_mode: false,
        ..Default::default()
    }
}

/// Trace the sky at the current sun + moon: a daylight Perez dome when the sun
/// is up, a night sky (starlight + moon) when it is down. Reads the open-plane
/// illuminance and derives colour metrics from the matching sky spectrum.
pub(crate) fn run_sky_trace(
    sun: &eulumdat_daylight::SolarPosition,
    moon: &eulumdat_daylight::MoonPosition,
    n: u64,
    road_l: f64,
) -> SpectralResult {
    use eulumdat_daylight::availability::DaylightAvailability;
    use eulumdat_daylight::sky::{PerezSky, SkyParams, SkyRadiance};

    let is_day = sun.is_daytime();
    let emit_half = 2.0;

    let (dome, target_lux, sky_spd, regime) = if is_day {
        let avail = DaylightAvailability::clear_sky(sun, 2.5);
        let cct = eulumdat_daylight::daylight_cct(sun.altitude_deg(), 2.5);
        let spd = eulumdat_spectrum::synth::synthesize(cct);
        let perez = PerezSky::new(*sun, SkyParams::from_turbidity(2.5));
        let target = avail.ghi_lux.max(1.0);
        let radiance = SkyRadiance::perez_from_dhi(perez, target);
        let dome =
            SkyDomeSource::with_extent(&radiance, Point3::new(0.0, 0.0, 0.02), 1.0, emit_half, 20, 40);
        let flux = dome.collector_flux(target);
        let dome = dome.with_flux(flux);
        let regime = if sun.altitude_deg() > 6.0 { "day" } else { "twilight" };
        (dome, target, spd, regime)
    } else {
        // Night: starlight + airglow PLUS the moon (which dominates when up).
        let twilight = sun.altitude_deg() > -6.0;
        let star_floor = if twilight { 5.0 } else { 0.0015 };

        let moon_face_lux = eulumdat_goniosim::moon_illuminance(moon.illuminated_fraction);
        let moon_lux = if moon.is_up() {
            moon_face_lux * moon.altitude_rad.sin().max(0.0)
        } else {
            0.0
        };
        let night_lux = star_floor + moon_lux;

        let cct = if moon_lux > star_floor {
            let base = eulumdat_goniosim::MOON_CCT_K;
            (base - 900.0 * (1.0 - moon.altitude_rad.sin().max(0.0))).clamp(3000.0, 4300.0)
        } else {
            6000.0
        };
        let spd = eulumdat_spectrum::synth::synthesize(cct);

        let src =
            night_sky_source(Point3::new(0.0, 0.0, 0.02), emit_half, night_lux.max(1e-4), 12, 24);
        let dome = match src {
            Source::SkyDome(d) => d,
            _ => unreachable!("night_sky_source returns a SkyDome"),
        };
        let regime = if twilight {
            "twilight"
        } else if moon_lux > 10.0 * star_floor {
            "moonlit"
        } else {
            "night"
        };
        (dome, night_lux, spd, regime)
    };

    let mut scene = Scene::new();
    scene.add_source_with_spectrum(Source::SkyDome(dome), SourceSpectrum::new(sky_spd.clone()));

    let plane = PlaneDetector::horizontal(Point3::origin(), 0.5, 0.5, 1, 1, false);
    let cfg = TracerConfig {
        num_photons: n,
        max_bounces: 4,
        seed: 42,
        ..TracerConfig::default()
    };
    let result = trace_illuminance(&scene, &cfg, vec![plane]);
    let sky_lux = result.average_lux(0);

    let col = eulumdat_spectrum::analyze(&sky_spd);
    let sp = eulumdat_spectrum::sp_ratio(&sky_spd);
    let mel_der = eulumdat_spectrum::melanopic_der(&sky_spd);

    SpectralResult {
        cct_k: col.cct_k,
        duv: col.duv,
        sp_ratio: sp,
        melanopic_der: mel_der,
        mesopic_road: mesopic_luminance(road_l, sp),
        photons: result.total_emitted_energy as u64,
        detected_pct: 100.0,
        sky_mode: true,
        sky_lux,
        sun_altitude: sun.altitude_deg(),
        is_day,
        regime,
        meets_darksky: true,
        upward_cct: col.cct_k,
        ..Default::default()
    }
    .with_target(target_lux)
}

// ── Shared result rendering ──────────────────────────────────────────────

/// Render the metric cards + optional dark-sky report for a result. Shared by
/// both tabs. `show_dark_sky` gates the dark-sky report (luminaire-only);
/// `show_sky_card` prepends the sky-illuminance card (sky-only).
pub(crate) fn render_result(
    r: &SpectralResult,
    sl: &eulumdat_i18n::SpectrumLabLocale,
    road: f64,
) -> impl IntoView {
    let stats = sl
        .stats
        .replace("{photons}", &format_photons(r.photons))
        .replace("{pct}", &format!("{:.0}", r.detected_pct));

    let sky_card = if r.sky_mode {
        let regime = match r.regime {
            "day" => sl.regime_day.clone(),
            "twilight" => sl.regime_twilight.clone(),
            "moonlit" => sl.regime_moonlit.clone(),
            _ => sl.regime_night.clone(),
        };
        Some(metric_card(
            sl.sky_illuminance.clone(),
            format!("{} lx", format_lux(r.sky_lux)),
            format!("{regime} · {}: {:.1}°", sl.sun_altitude, r.sun_altitude),
            "#f0883e",
        ))
    } else {
        None
    };

    let r = r.clone();
    let sl = sl.clone();
    view! {
        <div>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; margin-bottom: 16px;">
                {sky_card}
                {metric_card(sl.cct.clone(), format!("{:.0} K", r.cct_k), format!("Duv {:+.4}", r.duv), "#58a6ff")}
                {metric_card(sl.sp_ratio.clone(), format!("{:.2}", r.sp_ratio), sp_hint(&sl, r.sp_ratio), "#a371f7")}
                {metric_card(sl.melanopic_der.clone(), format!("{:.2}", r.melanopic_der), mel_hint(&sl, r.melanopic_der), "#3fb950")}
                {metric_card(sl.mesopic_road.clone(), format!("{:.2} cd/m²", r.mesopic_road), mesopic_hint(&sl, r.mesopic_road, road), "#f0883e")}
            </div>

            {(!r.sky_mode).then(|| {
                let (meets, ulor, upward_cct, blue) = (r.meets_darksky, r.ulor, r.upward_cct, r.blue_pct);
                let sl2 = sl.clone();
                view! {
                <div style=format!(
                    "padding: 16px; border-radius: 8px; border: 1px solid {}; background: {};",
                    if meets { "rgba(63,185,80,0.4)" } else { "rgba(248,81,73,0.4)" },
                    if meets { "rgba(63,185,80,0.08)" } else { "rgba(248,81,73,0.08)" },
                )>
                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px;">
                        <span style="font-size: 1.1rem;">{if meets { "✅" } else { "⚠️" }}</span>
                        <strong style="font-size: 1rem;">{sl2.dark_sky.clone()}</strong>
                        <span style=format!(
                            "margin-left: auto; font-size: 0.8rem; padding: 2px 10px; border-radius: 12px; background: {}; color: white;",
                            if meets { "#238636" } else { "#da3633" },
                        )>
                            {if meets { sl2.passes.clone() } else { sl2.fails.clone() }}
                        </span>
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 10px; font-size: 0.85rem;">
                        {report_row(sl2.ulor.clone(), format!("{ulor:.1}%"))}
                        {report_row(sl2.upward_cct.clone(), format!("{upward_cct:.0} K"))}
                        {report_row(sl2.blue_content.clone(), format!("{blue:.1}%"))}
                    </div>
                </div>
            }})}

            <div style="margin-top: 12px; font-size: 0.8rem; color: var(--text-muted, #8b949e); text-align: right;">
                {stats}
            </div>
        </div>
    }
}

// ── Card + hint helpers ──────────────────────────────────────────────────

pub(crate) fn metric_card(
    label: String,
    value: String,
    hint: String,
    accent: &str,
) -> impl IntoView {
    let border = format!("border-left: 3px solid {accent};");
    view! {
        <div style=format!("padding: 14px; background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; {border}")>
            <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">{label}</div>
            <div style="font-size: 1.5rem; font-weight: 600; line-height: 1;">{value}</div>
            <div style="font-size: 0.78rem; color: var(--text-muted, #8b949e); margin-top: 4px;">{hint}</div>
        </div>
    }
}

pub(crate) fn report_row(label: String, value: String) -> impl IntoView {
    view! {
        <div>
            <div style="color: var(--text-muted, #8b949e); font-size: 0.72rem; text-transform: uppercase;">{label}</div>
            <div style="font-weight: 600;">{value}</div>
        </div>
    }
}

pub(crate) fn sp_hint(sl: &eulumdat_i18n::SpectrumLabLocale, sp: f64) -> String {
    if sp > 1.8 {
        sl.sp_cool.clone()
    } else if sp > 1.3 {
        sl.sp_neutral.clone()
    } else {
        sl.sp_warm.clone()
    }
}

pub(crate) fn mel_hint(sl: &eulumdat_i18n::SpectrumLabLocale, der: f64) -> String {
    if der > 0.9 {
        sl.mel_high.clone()
    } else if der > 0.5 {
        sl.mel_moderate.clone()
    } else {
        sl.mel_low.clone()
    }
}

pub(crate) fn mesopic_hint(
    sl: &eulumdat_i18n::SpectrumLabLocale,
    mesopic: f64,
    photopic: f64,
) -> String {
    let pct = if photopic > 0.0 {
        (mesopic / photopic - 1.0) * 100.0
    } else {
        0.0
    };
    sl.mesopic_vs
        .replace("{pct}", &format!("{pct:+.0}"))
        .replace("{photopic}", &format!("{photopic:.1}"))
}

/// Localised name for a moon phase.
pub(crate) fn moon_phase_label(
    sl: &eulumdat_i18n::SpectrumLabLocale,
    phase: eulumdat_daylight::MoonPhase,
) -> String {
    use eulumdat_daylight::MoonPhase::*;
    match phase {
        New => sl.moon_new.clone(),
        WaxingCrescent => sl.moon_waxing_crescent.clone(),
        FirstQuarter => sl.moon_first_quarter.clone(),
        WaxingGibbous => sl.moon_waxing_gibbous.clone(),
        Full => sl.moon_full.clone(),
        WaningGibbous => sl.moon_waning_gibbous.clone(),
        LastQuarter => sl.moon_last_quarter.clone(),
        WaningCrescent => sl.moon_waning_crescent.clone(),
    }
}

/// Format decimal local hours as "HH:MM".
pub(crate) fn fmt_hour(h: f64) -> String {
    let total_min = (h * 60.0).round() as i32;
    let hh = (total_min / 60).rem_euclid(24);
    let mm = (total_min % 60).rem_euclid(60);
    format!("{hh:02}:{mm:02}")
}

pub(crate) fn format_photons(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}

/// Format an illuminance value across the ~7-decade starlight→noon span.
pub(crate) fn format_lux(lux: f64) -> String {
    if lux >= 10_000.0 {
        format!("{:.0}", (lux / 100.0).round() * 100.0)
    } else if lux >= 100.0 {
        format!("{lux:.0}")
    } else if lux >= 1.0 {
        format!("{lux:.1}")
    } else {
        format!("{lux:.4}")
    }
}

/// Yield control to the browser so the UI can repaint before a blocking trace.
pub(crate) async fn yield_to_browser() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
