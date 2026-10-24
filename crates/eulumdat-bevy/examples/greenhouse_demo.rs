//! Greenhouse Demo — "Vom Schweinestall zum Gewächshaus"
//!
//! An old Münsterland pig barn (brick socle, stepped west gable, gabled
//! roof) rebuilt as a modern glasshouse: everything above the 90 cm brick
//! socle is glazing, four long growing benches fill the nave, and rows of
//! horticultural LED fixtures hang from the roof structure.
//!
//! The point of the demo is the **spectral** stack in 3D:
//!
//! - **Daylight** — real solar position for the site (Lüdinghausen by
//!   default), clear-sky (Perez) or overcast (CIE) availability, daylight
//!   CCT, all from `eulumdat-daylight`; the sky itself is Bevy's physically
//!   based `Atmosphere`.
//! - **Glazing as a spectral material** — float glass, twin-wall
//!   polycarbonate, diffusing film or a green shade net, each with a τ(λ)
//!   curve. The same curve drives the visual tint *and* the photon transport.
//! - **Grow lights with measured SPDs** — four TM-33 / ATLA templates
//!   (full-spectrum white, red/blue, high-blue seedling, 730 nm far-red).
//!   The Bevy light colour is computed from the SPD's chromaticity (CIE XYZ →
//!   linear sRGB), not from a nominal CCT — red/blue spectra are nowhere near
//!   the Planckian locus.
//! - **Canopy PPFD** — an analytic point-by-point calculation over the bench
//!   plane (LDT sampling + inverse square + daylight through the envelope),
//!   converted lux → µmol·m⁻²·s⁻¹ *per source spectrum* (`eulumdat-spectrum`),
//!   rendered as a heat-map overlay with live statistics (avg/min/max,
//!   uniformity, DLI, mixed-light CCT, electric share).
//! - **Spectral Monte Carlo** (press `M`) — the CPU tracer from
//!   `eulumdat-goniosim` re-traces the canopy with one wavelength per photon:
//!   sun disc + sky dome (D-series spectrum) + every LED fixture, through the
//!   spectral glazing, onto a spectral plane detector. Per-cell X/Y/Z/PAR
//!   channels give lux, PPFD and CCT of the mixed light — the reference the
//!   analytic estimate is compared against, plus a daylight factor.
//!
//! Controls:
//!   WASD / Q / E        — Move camera (Shift = faster)
//!   Right-drag / Arrows — Look around
//!   R                   — Reset camera
//!   [ / ]               — Hour −/+ 30 min          , / .  — Month −/+
//!   K                   — Clear ↔ overcast sky      G      — Cycle glazing
//!   1 – 4               — Grow-light preset          0      — Grow lights off
//!   - / =               — Dim −/+ 10 %
//!   T                   — Toggle PPFD heat-map      L      — Toggle luminaire models
//!   P                   — Toggle photometric solids
//!   M                   — Run the spectral Monte Carlo trace
//!
//! Run:
//!   cargo run --bin greenhouse-demo -p eulumdat-bevy --features post-process,bevy-ui --release
//!
//! WASM: built by `scripts/build-wasm-split.sh` as its own lazy bundle and
//! opened from the editor with `?wasm=greenhouse_demo` (WebGPU only — the
//! atmosphere and bloom passes need compute + storage textures).

use bevy::asset::RenderAssetUsages;
use bevy::camera::{Exposure, Hdr};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputDispatchPlugin,
};
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{
    Atmosphere, AtmosphereEnvironmentMapLight, GlobalAmbientLight, NotShadowCaster,
    NotShadowReceiver,
};
use bevy::pbr::AtmosphereSettings;
use bevy::picking::hover::Hovered;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::NoIndirectDrawing;
use bevy::ui_widgets::{
    observe, slider_self_update, Activate, Button as UiButton, Slider, SliderRange, SliderThumb,
    SliderValue, TrackClick, UiWidgetsPlugins, ValueChange,
};

use eulumdat::atla;
use eulumdat::Eulumdat;
use eulumdat_bevy::photometric::{
    heatmap_color, kelvin_to_color, PhotometricLight, PhotometricPlugin,
};
use eulumdat_bevy::EulumdatLightBundle;
use eulumdat_daylight::coords::altaz_to_world;
use eulumdat_daylight::{
    daylight_cct, location_by_name, CieSky, DaylightAvailability, LocalDateTime, NamedLocation,
    PerezSky, SkyParams, SkyRadiance, SolarPosition,
};
use eulumdat_goniosim::nalgebra::{Point3, Rotation3, Unit, Vector3};
use eulumdat_goniosim::{
    sun_source, trace_illuminance, MaterialParams, PlaneDetector, Primitive, Scene, SkyDomeSource,
    Source, SourceSpectrum, SpectralOverride, TracerConfig,
};
use eulumdat_spectrum::colorimetry::{analyze, from_chromaticity};
use eulumdat_spectrum::constants::KM_PHOTOPIC;
use eulumdat_spectrum::daylight::d_illuminant;
use eulumdat_spectrum::lut::photopic;
use eulumdat_spectrum::metrics::{ppfd_integral, SpectralMetrics};
use eulumdat_spectrum::Spd;

// ---------------------------------------------------------------------------
// Embedded luminaires (TM-33 / ATLA templates with SPD + intensity data)
// ---------------------------------------------------------------------------

const LAMP_FULL_SPECTRUM: &str =
    include_str!("../../eulumdat-wasm/templates/tm-33-23_horticultural_led.xml");
const LAMP_RED_BLUE: &str = include_str!("../../eulumdat-wasm/templates/_atla_grow_light_rb.xml");
const LAMP_SEEDLING: &str =
    include_str!("../../eulumdat-wasm/templates/tm-33-23_seedling_propagation.xml");
const LAMP_FAR_RED: &str =
    include_str!("../../eulumdat-wasm/templates/tm-33-23_far_red_supplemental.xml");

// ---------------------------------------------------------------------------
// Building geometry (metres, Bevy world frame: +X East, +Y up, +Z South)
// ---------------------------------------------------------------------------

/// Ridge runs east–west along X.
const BARN_LENGTH: f32 = 24.0;
const BARN_WIDTH: f32 = 10.0;
/// Old brick socle kept from the pig barn.
const SOCLE_H: f32 = 0.9;
const EAVE_H: f32 = 2.6;
const RIDGE_H: f32 = 5.4;
const WALL_T: f32 = 0.3;

const BENCH_H: f32 = 0.8;
const BENCH_W: f32 = 1.6;
const BENCH_LEN: f32 = 22.0;
const BENCH_Z: [f32; 4] = [-3.6, -1.2, 1.2, 3.6];
/// Canopy / calculation plane height (bench top + ~20 cm of foliage).
const CANOPY_H: f32 = 1.0;
/// Fixture centre height above the floor.
const FIXTURE_H: f32 = 2.45;
const FIXTURE_SPACING: f32 = 2.4;
const FIXTURES_PER_ROW: usize = 9;

/// Analytic grid over the interior (0.25 m cells).
const GRID_NX: usize = 96;
const GRID_NZ: usize = 40;
/// Monte Carlo detector grid (0.5 m cells — needs photons per cell).
const MC_NX: usize = 48;
const MC_NZ: usize = 20;

/// Mullions, purlins and gutters block roughly this much of the glazing.
const STRUCTURE_FACTOR: f64 = 0.90;
/// Average sky-view factor of a canopy point inside the envelope.
const SKY_VIEW_FACTOR: f64 = 0.82;

/// Roof height at a given Z (gable, ridge at z = 0).
fn roof_y(z: f32) -> f32 {
    RIDGE_H - (RIDGE_H - EAVE_H) * (z.abs() / (BARN_WIDTH / 2.0))
}

fn fixture_positions() -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(FIXTURES_PER_ROW * BENCH_Z.len());
    let x0 = -(FIXTURES_PER_ROW as f32 - 1.0) / 2.0 * FIXTURE_SPACING;
    for &z in &BENCH_Z {
        for i in 0..FIXTURES_PER_ROW {
            out.push((x0 + i as f32 * FIXTURE_SPACING, z));
        }
    }
    out
}

fn over_bench(z: f32) -> bool {
    BENCH_Z.iter().any(|&bz| (z - bz).abs() <= BENCH_W / 2.0)
}

// ---------------------------------------------------------------------------
// Glazing presets — spectral materials
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Glazing {
    FloatGlass,
    Polycarbonate,
    DiffuseFilm,
    ShadeNet,
}

impl Glazing {
    const ALL: [Glazing; 4] = [
        Glazing::FloatGlass,
        Glazing::Polycarbonate,
        Glazing::DiffuseFilm,
        Glazing::ShadeNet,
    ];

    fn label(self) -> &'static str {
        match self {
            Glazing::FloatGlass => "Float glass 4 mm",
            Glazing::Polycarbonate => "Polycarbonate twin-wall",
            Glazing::DiffuseFilm => "Diffusing film",
            Glazing::ShadeNet => "Green shade net 40 %",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Glazing::FloatGlass => "Glass",
            Glazing::Polycarbonate => "PC",
            Glazing::DiffuseFilm => "Film",
            Glazing::ShadeNet => "Net",
        }
    }

    fn next(self) -> Glazing {
        let i = Glazing::ALL.iter().position(|g| *g == self).unwrap_or(0);
        Glazing::ALL[(i + 1) % Glazing::ALL.len()]
    }

    /// Spectral transmittance τ(λ), 300–800 nm on a 10 nm grid.
    fn tau_curve(self) -> Spd {
        let values: Vec<f64> = (0..=50)
            .map(|i| {
                let wl = 300.0 + 10.0 * i as f64;
                match self {
                    // Soda-lime float glass: UV edge at ~330 nm, then flat.
                    Glazing::FloatGlass => {
                        if wl < 320.0 {
                            0.0
                        } else if wl < 380.0 {
                            0.90 * (wl - 320.0) / 60.0
                        } else {
                            0.90
                        }
                    }
                    // Twin-wall PC: sharper UV cut, slight yellow cast.
                    Glazing::Polycarbonate => {
                        if wl < 385.0 {
                            0.0
                        } else if wl < 450.0 {
                            0.50 + 0.30 * (wl - 385.0) / 65.0
                        } else {
                            0.82 - 0.02 * ((wl - 450.0) / 350.0)
                        }
                    }
                    // PE diffusing film: broadband, mild UV cut.
                    Glazing::DiffuseFilm => {
                        if wl < 340.0 {
                            0.0
                        } else if wl < 400.0 {
                            0.85 * (wl - 340.0) / 60.0
                        } else {
                            0.85
                        }
                    }
                    // Green HDPE net: 40 % shade overall, but green-selective —
                    // the red/blue PAR bands are cut harder than the lux.
                    Glazing::ShadeNet => {
                        let g = (-((wl - 545.0) / 55.0).powi(2)).exp();
                        0.42 + 0.30 * g
                    }
                }
            })
            .collect();
        Spd::uniform(300.0, 10.0, &values)
    }

    /// Haze / diffusion in percent (datasheet style).
    fn haze_pct(self) -> f64 {
        match self {
            Glazing::FloatGlass => 0.0,
            Glazing::Polycarbonate => 30.0,
            Glazing::DiffuseFilm => 80.0,
            Glazing::ShadeNet => 15.0,
        }
    }

    fn thickness_mm(self) -> f64 {
        match self {
            Glazing::FloatGlass => 4.0,
            Glazing::Polycarbonate => 10.0,
            Glazing::DiffuseFilm => 0.2,
            Glazing::ShadeNet => 1.0,
        }
    }

    /// (tint, alpha, roughness) for the Bevy material.
    fn visual(self) -> (Color, f32, f32) {
        match self {
            Glazing::FloatGlass => (Color::srgb(0.85, 0.92, 0.95), 0.14, 0.05),
            Glazing::Polycarbonate => (Color::srgb(0.92, 0.90, 0.78), 0.30, 0.35),
            Glazing::DiffuseFilm => (Color::srgb(0.95, 0.95, 0.95), 0.45, 0.80),
            Glazing::ShadeNet => (Color::srgb(0.45, 0.70, 0.40), 0.55, 0.90),
        }
    }
}

// ---------------------------------------------------------------------------
// Grow-light presets
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lamp {
    FullSpectrum,
    RedBlue,
    Seedling,
    FarRed,
}

impl Lamp {
    const ALL: [Lamp; 4] = [
        Lamp::FullSpectrum,
        Lamp::RedBlue,
        Lamp::Seedling,
        Lamp::FarRed,
    ];

    fn index(self) -> usize {
        Lamp::ALL.iter().position(|l| *l == self).unwrap_or(0)
    }

    fn short(self) -> &'static str {
        match self {
            Lamp::FullSpectrum => "Full",
            Lamp::RedBlue => "R/B",
            Lamp::Seedling => "Blue",
            Lamp::FarRed => "FarRed",
        }
    }
}

/// A grow-light preset: LDT geometry/intensities plus its measured SPD and
/// the spectral conversion factors derived from it.
#[derive(Clone)]
struct LampData {
    name: String,
    ldt: Eulumdat,
    spd: Spd,
    /// Lamp lumens (what the cd/klm intensities are normalised to).
    lamp_flux_lm: f64,
    /// Luminaire flux actually emitted (lamp flux × LOR).
    luminaire_flux_lm: f64,
    watts: f64,
    /// µmol·m⁻²·s⁻¹ per lux for this spectrum.
    ppfd_per_lux: f64,
    /// CIE XYZ per lux (Y = 1) for mixing colours at the canopy.
    xyz_per_lux: [f64; 3],
    /// Luminous efficacy of radiation, lm/W.
    ler: f64,
    cct_k: f64,
    /// Rendered light colour from the SPD chromaticity.
    color: Color,
}

impl LampData {
    fn load(xml: &str) -> Self {
        let doc = atla::parse(xml).expect("embedded grow-light template must parse");
        let ldt = doc.to_eulumdat();
        let emitter = doc.emitters.first();
        let spd = emitter
            .and_then(|e| e.spectral_distribution.as_ref())
            .map(|s| resample_1nm(&Spd::new(&s.wavelengths, &s.values)))
            .unwrap_or_else(|| eulumdat_spectrum::synth::synthesize(4000.0));
        let mut lamp_flux_lm = ldt.total_luminous_flux();
        if lamp_flux_lm <= 0.0 {
            lamp_flux_lm = emitter.and_then(|e| e.rated_lumens).unwrap_or(10_000.0);
        }
        // What the cd/klm table actually integrates to (lm per klm of lamp
        // flux). TM-33 conversions default LOR to 100 % when it is not
        // derivable, so integrate the distribution ourselves — this is the
        // flux the Monte Carlo source must be normalised to.
        let lor = intensity_integral_per_klm(&ldt) / 1000.0;
        let watts = emitter.and_then(|e| e.input_watts).unwrap_or(0.0);
        let name = emitter
            .and_then(|e| e.description.clone())
            .unwrap_or_else(|| ldt.luminaire_name.clone());
        let spec = spectral_factors(&spd);
        Self {
            name,
            ldt,
            spd: spd.clone(),
            lamp_flux_lm,
            luminaire_flux_lm: lamp_flux_lm * lor,
            watts,
            ppfd_per_lux: spec.ppfd_per_lux,
            xyz_per_lux: spec.xyz_per_lux,
            ler: spec.ler,
            cct_k: spec.cct_k,
            color: spec.color,
        }
    }
}

/// ∫ I(C,γ) dΩ over the sphere for a cd/klm table — lumens per klm.
fn intensity_integral_per_klm(ldt: &Eulumdat) -> f64 {
    let dc = 5.0f64;
    let dg = 1.0f64;
    let mut sum = 0.0;
    let mut c = 0.0;
    while c < 360.0 {
        let mut g = 0.0;
        while g < 180.0 {
            let gm = g + dg / 2.0;
            sum += ldt.sample(c + dc / 2.0, gm) * gm.to_radians().sin();
            g += dg;
        }
        c += dc;
    }
    sum * dc.to_radians() * dg.to_radians()
}

/// Everything the canopy calculation needs to know about a spectrum.
#[derive(Clone, Copy, Debug)]
struct SpectralFactors {
    ppfd_per_lux: f64,
    xyz_per_lux: [f64; 3],
    ler: f64,
    cct_k: f64,
    color: Color,
}

/// Resample an SPD onto a 1 nm grid so band-edge integrals (PAR cut-off at
/// 700 nm, V(λ) tails) match what the per-photon Monte Carlo sees, instead of
/// depending on the vendor file's 5–10 nm sample spacing.
fn resample_1nm(spd: &Spd) -> Spd {
    let w = spd.wavelengths();
    if w.len() < 2 {
        return spd.clone();
    }
    let start = w[0].floor();
    let end = w[w.len() - 1].ceil();
    let n = ((end - start) as usize) + 1;
    let values: Vec<f64> = (0..n).map(|i| spd.value_at(start + i as f64)).collect();
    Spd::uniform(start, 1.0, &values)
}

fn spectral_factors(spd: &Spd) -> SpectralFactors {
    let fine = resample_1nm(spd);
    let spd = &fine;
    let photopic_integral = spd.integral_weighted(photopic).max(1e-12);
    let ppfd_per_lux = ppfd_integral(spd) / (KM_PHOTOPIC * photopic_integral);
    let c = analyze(spd);
    let y = c.y.max(1e-6);
    let xyz_per_lux = [c.x / y, 1.0, (1.0 - c.x - c.y) / y];
    let metrics = SpectralMetrics::from_spd(spd);
    SpectralFactors {
        ppfd_per_lux,
        xyz_per_lux,
        ler: metrics.ler_lm_per_w.max(1e-6),
        cct_k: c.cct_k,
        color: xyz_to_color(xyz_per_lux),
    }
}

/// CIE XYZ (relative) → linear sRGB, clamped and normalised so the brightest
/// channel is 1.0. Used for light colours: photometric intensity is carried
/// by the lumen value, so only the chromaticity matters here.
fn xyz_to_color(xyz: [f64; 3]) -> Color {
    let [x, y, z] = xyz;
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;
    let (r, g, b) = (r.max(0.0), g.max(0.0), b.max(0.0));
    let m = r.max(g).max(b).max(1e-6);
    Color::linear_rgb((r / m) as f32, (g / m) as f32, (b / m) as f32)
}

// ---------------------------------------------------------------------------
// Sky / site presets
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SkyKind {
    Clear,
    Overcast,
}

const SITE_NAMES: [&str; 3] = ["Lüdinghausen", "Wuhan", "Sydney"];

fn site(idx: usize) -> NamedLocation {
    location_by_name(SITE_NAMES[idx.min(SITE_NAMES.len() - 1)])
        .or_else(|| location_by_name("Berlin"))
        .expect("daylight location table has the demo sites")
}

// ---------------------------------------------------------------------------
// Canopy grid + statistics
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct CanopyGrid {
    nx: usize,
    nz: usize,
    /// µmol·m⁻²·s⁻¹ per cell (row-major, `[iz * nx + ix]`, iz = 0 at z = −W/2).
    ppfd: Vec<f64>,
    /// lux per cell.
    lux: Vec<f64>,
    /// Electric-light share of the PPFD per cell (0..1).
    electric_ppfd: Vec<f64>,
    /// CIE XYZ per cell (absolute, Y = lux).
    xyz: Vec<[f64; 3]>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CanopyStats {
    avg_ppfd: f64,
    min_ppfd: f64,
    max_ppfd: f64,
    avg_lux: f64,
    /// min / avg over the bench cells.
    uniformity: f64,
    /// Electric share of the average PPFD, 0..1.
    electric_share: f64,
    /// CCT of the mixed light averaged over the benches.
    mix_cct: f64,
}

impl CanopyGrid {
    fn new(nx: usize, nz: usize) -> Self {
        Self {
            nx,
            nz,
            ppfd: vec![0.0; nx * nz],
            lux: vec![0.0; nx * nz],
            electric_ppfd: vec![0.0; nx * nz],
            xyz: vec![[0.0; 3]; nx * nz],
        }
    }

    fn cell_center(&self, ix: usize, iz: usize) -> (f32, f32) {
        let dx = BARN_LENGTH / self.nx as f32;
        let dz = BARN_WIDTH / self.nz as f32;
        (
            -BARN_LENGTH / 2.0 + (ix as f32 + 0.5) * dx,
            -BARN_WIDTH / 2.0 + (iz as f32 + 0.5) * dz,
        )
    }

    /// Statistics over the cells that lie above a growing bench (the canopy).
    fn stats(&self) -> CanopyStats {
        let mut n = 0usize;
        let mut sum = 0.0;
        let mut sum_lux = 0.0;
        let mut sum_el = 0.0;
        let mut min = f64::INFINITY;
        let mut max = 0.0f64;
        let mut xyz = [0.0f64; 3];
        for iz in 0..self.nz {
            for ix in 0..self.nx {
                let (x, z) = self.cell_center(ix, iz);
                if x.abs() > BENCH_LEN / 2.0 || !over_bench(z) {
                    continue;
                }
                let i = iz * self.nx + ix;
                let p = self.ppfd[i];
                n += 1;
                sum += p;
                sum_lux += self.lux[i];
                sum_el += self.electric_ppfd[i];
                min = min.min(p);
                max = max.max(p);
                for k in 0..3 {
                    xyz[k] += self.xyz[i][k];
                }
            }
        }
        if n == 0 {
            return CanopyStats::default();
        }
        let avg = sum / n as f64;
        let s = xyz[0] + xyz[1] + xyz[2];
        let mix_cct = if s > 1e-9 && avg > 0.05 {
            from_chromaticity(xyz[0] / s, xyz[1] / s).cct_k
        } else {
            0.0
        };
        CanopyStats {
            avg_ppfd: avg,
            min_ppfd: if min.is_finite() { min } else { 0.0 },
            max_ppfd: max,
            avg_lux: sum_lux / n as f64,
            uniformity: if avg > 0.0 { min / avg } else { 0.0 },
            electric_share: if sum > 0.0 { sum_el / sum } else { 0.0 },
            mix_cct,
        }
    }
}

// ---------------------------------------------------------------------------
// Daylight state derived from site + date + hour + sky
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct DaylightState {
    sun: SolarPosition,
    avail: DaylightAvailability,
    cct_k: f64,
    /// Up-going unit vector toward the sun, Bevy world frame.
    sun_dir: Vec3,
    /// Photopic transmittance of the current glazing for this daylight spectrum.
    tau_v: f64,
    /// Spectral factors of the daylight *after* the glazing.
    inside: SpectralFactors,
}

fn compute_daylight(state: &GreenhouseState) -> DaylightState {
    let loc = site(state.site_idx);
    let ldt = LocalDateTime::new(2026, state.month, 15, state.hour as f64);
    let sun = ldt.solar_position(&loc);
    let avail = match state.sky {
        SkyKind::Clear => DaylightAvailability::clear_sky(&sun, state.turbidity as f64),
        SkyKind::Overcast => DaylightAvailability::overcast(&sun),
    };
    let cct_k = match state.sky {
        SkyKind::Clear => daylight_cct(sun.altitude_deg(), state.turbidity as f64),
        SkyKind::Overcast => 6500.0,
    };
    let d = altaz_to_world(sun.altitude_deg(), sun.azimuth_deg());
    let sun_dir = Vec3::new(d.x as f32, d.y as f32, d.z as f32).normalize_or_zero();

    // Daylight spectrum through the glazing: D(λ)·τ(λ).
    let day = d_illuminant(cct_k);
    let tau = state.glazing.tau_curve();
    let wl: Vec<f64> = day.wavelengths().to_vec();
    let transmitted: Vec<f64> = wl
        .iter()
        .zip(day.values())
        .map(|(w, v)| v * tau.value_at(*w))
        .collect();
    let inside_spd = Spd::new(&wl, &transmitted);
    let tau_v = inside_spd.integral_weighted(photopic) / day.integral_weighted(photopic).max(1e-12);
    let inside = spectral_factors(&inside_spd);
    DaylightState {
        sun,
        avail,
        cct_k,
        sun_dir,
        tau_v,
        inside,
    }
}

// ---------------------------------------------------------------------------
// Analytic canopy calculation
// ---------------------------------------------------------------------------

/// Which envelope surface a ray from `origin` in direction `dir` leaves through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Exit {
    Glazing,
    Opaque,
}

fn envelope_exit(origin: Vec3, dir: Vec3) -> Exit {
    if dir.y <= 0.0 {
        return Exit::Opaque;
    }
    let step = 0.1;
    let mut p = origin;
    let mut last = origin;
    for _ in 0..400 {
        p += dir * step;
        let inside =
            p.x.abs() <= BARN_LENGTH / 2.0 && p.z.abs() <= BARN_WIDTH / 2.0 && p.y <= roof_y(p.z);
        if !inside {
            // Classify by which bound was crossed first.
            if p.x < -BARN_LENGTH / 2.0 {
                return Exit::Opaque; // west gable: brick
            }
            if p.x > BARN_LENGTH / 2.0 {
                return if last.y > SOCLE_H {
                    Exit::Glazing
                } else {
                    Exit::Opaque
                };
            }
            if p.z.abs() > BARN_WIDTH / 2.0 {
                return if last.y > SOCLE_H {
                    Exit::Glazing
                } else {
                    Exit::Opaque
                };
            }
            return Exit::Glazing; // roof
        }
        last = p;
    }
    Exit::Opaque
}

fn compute_analytic(state: &GreenhouseState, day: &DaylightState) -> CanopyGrid {
    let mut grid = CanopyGrid::new(GRID_NX, GRID_NZ);
    let lamp = &state.lamps[state.lamp.index()];
    let fixtures = fixture_positions();
    let lamps_on = state.lamps_on && state.dimming > 0.001;
    let dim = state.dimming as f64;
    let h = (FIXTURE_H - CANOPY_H) as f64;
    // cd per cd/klm at the current dimming.
    let cd_scale = lamp.lamp_flux_lm / 1000.0 * dim;

    let daytime = day.sun.altitude_deg() > 0.0;
    let sin_alt = day.sun.altitude_rad.sin().max(0.0);
    let sky_lux_inside = day.avail.dhi_lux * day.tau_v * 0.92 * SKY_VIEW_FACTOR * STRUCTURE_FACTOR;
    let sun_lux_inside = day.avail.dni_lux * sin_alt * day.tau_v * STRUCTURE_FACTOR;

    for iz in 0..GRID_NZ {
        for ix in 0..GRID_NX {
            let (x, z) = grid.cell_center(ix, iz);
            let i = iz * GRID_NX + ix;

            // --- Electric ---
            let mut e_led = 0.0;
            if lamps_on {
                for &(fx, fz) in &fixtures {
                    let dx = (x - fx) as f64;
                    let dz = (z - fz) as f64;
                    let d2 = dx * dx + dz * dz + h * h;
                    let d = d2.sqrt();
                    let cos_t = h / d;
                    let gamma = cos_t.clamp(-1.0, 1.0).acos().to_degrees();
                    // C0 = +X (across the fixture width), C90 = +Z (along its
                    // length) — the same frame the Monte Carlo source uses
                    // (`Rotation3::identity()` → C0 = sim +X = world +X).
                    let mut c = dz.atan2(dx).to_degrees();
                    if c < 0.0 {
                        c += 360.0;
                    }
                    let intensity = lamp.ldt.sample(c, gamma) * cd_scale;
                    e_led += intensity * cos_t / d2;
                }
            }
            let ppfd_led = e_led * lamp.ppfd_per_lux;

            // --- Daylight ---
            let mut e_day = 0.0;
            if daytime {
                e_day += sky_lux_inside;
                if sun_lux_inside > 0.0
                    && envelope_exit(Vec3::new(x, CANOPY_H, z), day.sun_dir) == Exit::Glazing
                {
                    e_day += sun_lux_inside;
                }
            }
            let ppfd_day = e_day * day.inside.ppfd_per_lux;

            grid.lux[i] = e_led + e_day;
            grid.ppfd[i] = ppfd_led + ppfd_day;
            grid.electric_ppfd[i] = ppfd_led;
            for k in 0..3 {
                grid.xyz[i][k] = e_led * lamp.xyz_per_lux[k] + e_day * day.inside.xyz_per_lux[k];
            }
        }
    }
    grid
}

// ---------------------------------------------------------------------------
// Spectral Monte Carlo (eulumdat-goniosim)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct McResult {
    grid: CanopyGrid,
    photons: u64,
    elapsed_ms: f64,
    /// Interior daylight-only average lux over the benches / exterior GHI.
    daylight_factor_pct: f64,
}

/// Bevy world (+Y up, +Z South) → tracer frame (+Z up, C0 = +X).
fn w2s(x: f32, y: f32, z: f32) -> Point3<f64> {
    Point3::new(x as f64, -(z as f64), y as f64)
}

fn aabox(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> Primitive {
    let a = w2s(x0, y0, z0);
    let b = w2s(x1, y1, z1);
    Primitive::AaBox {
        min: Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
        max: Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
    }
}

/// The greenhouse envelope + furniture as tracer geometry (no sources).
fn build_mc_geometry(glazing: Glazing) -> Scene {
    let mut scene = Scene::new();

    let concrete = scene.add_material(MaterialParams {
        name: "concrete floor".into(),
        reflectance_pct: 25.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    });
    let brick = scene.add_material(MaterialParams {
        name: "brick".into(),
        reflectance_pct: 32.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    });
    let bench = scene.add_material(MaterialParams {
        name: "bench + foliage".into(),
        reflectance_pct: 22.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    });
    // goniosim `Sheet`s are single surfaces (thickness is not traced), so a
    // pane is modelled as ONE interface: IOR 1.0 (no bending — a real pane
    // has two parallel faces and only offsets the ray), Fresnel folded into
    // the measured τ, and τ passed squared because the clear transmitter
    // applies √τ per surface. Haze is visual-only here.
    let tau = glazing.tau_curve();
    let tau_sq = Spd::new(
        tau.wavelengths(),
        &tau.values().iter().map(|t| t * t).collect::<Vec<_>>(),
    );
    let tau_v = tau.integral_weighted(photopic)
        / Spd::uniform(300.0, 10.0, &vec![1.0; 51]).integral_weighted(photopic);
    let glass = scene.add_material_spectral(
        MaterialParams {
            name: glazing.label().into(),
            reflectance_pct: 0.0,
            ior: 1.0,
            transmittance_pct: (tau_v * tau_v * 100.0).clamp(1.0, 99.0),
            thickness_mm: glazing.thickness_mm(),
            diffusion_pct: 0.0,
        },
        SpectralOverride {
            dispersion: None,
            transmittance_curve: Some(tau_sq),
        },
    );

    // Ground (inside floor and the field outside share one infinite plane).
    scene.add_object(
        Primitive::Plane {
            point: Point3::new(0.0, 0.0, 0.0),
            normal: Vector3::z_axis(),
        },
        concrete,
        "ground",
    );
    // Debug knobs for the --selftest harness (native only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::var("GH_MC_OPEN_FIELD").is_ok() {
            return scene;
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let skip_glazing = std::env::var("GH_MC_NO_GLAZING").is_ok();
    #[cfg(target_arch = "wasm32")]
    let skip_glazing = false;

    let hl = BARN_LENGTH / 2.0;
    let hw = BARN_WIDTH / 2.0;
    // Brick socle on all four sides, full-height brick gable in the west.
    scene.add_object(
        aabox(-hl - WALL_T, 0.0, -hw - WALL_T, hl + WALL_T, SOCLE_H, -hw),
        brick,
        "socle N",
    );
    scene.add_object(
        aabox(-hl - WALL_T, 0.0, hw, hl + WALL_T, SOCLE_H, hw + WALL_T),
        brick,
        "socle S",
    );
    scene.add_object(
        aabox(hl, 0.0, -hw, hl + WALL_T, SOCLE_H, hw),
        brick,
        "socle E",
    );
    scene.add_object(
        aabox(-hl - WALL_T, 0.0, -hw, -hl, RIDGE_H, hw),
        brick,
        "west gable",
    );

    #[cfg(not(target_arch = "wasm32"))]
    let skip_benches = std::env::var("GH_MC_NO_BENCHES").is_ok();
    #[cfg(target_arch = "wasm32")]
    let skip_benches = false;
    // Benches (solid boxes; the canopy detector sits above them).
    for &bz in BENCH_Z.iter().filter(|_| !skip_benches) {
        scene.add_object(
            aabox(
                -BENCH_LEN / 2.0,
                0.0,
                bz - BENCH_W / 2.0,
                BENCH_LEN / 2.0,
                BENCH_H,
                bz + BENCH_W / 2.0,
            ),
            bench,
            "bench",
        );
    }

    if skip_glazing {
        return scene;
    }
    // Glazing: side walls above the socle.
    let wall_mid = (SOCLE_H + EAVE_H) / 2.0;
    let wall_half = (EAVE_H - SOCLE_H) / 2.0;
    for (z, label) in [(-hw, "glass N"), (hw, "glass S")] {
        scene.add_object(
            Primitive::Sheet {
                center: w2s(0.0, wall_mid, z),
                normal: Unit::new_normalize(Vector3::new(0.0, -(z as f64).signum(), 0.0)),
                u_axis: Vector3::x_axis(),
                half_width: hl as f64,
                half_height: wall_half as f64,
                thickness: glazing.thickness_mm() / 1000.0,
            },
            glass,
            label,
        );
    }
    // East gable (rectangle up to the ridge — the corners above the roof
    // line are covered by the roof sheets anyway).
    scene.add_object(
        Primitive::Sheet {
            center: w2s(hl, (SOCLE_H + RIDGE_H) / 2.0, 0.0),
            normal: Vector3::x_axis(),
            u_axis: Vector3::y_axis(),
            half_width: hw as f64,
            half_height: ((RIDGE_H - SOCLE_H) / 2.0) as f64,
            thickness: glazing.thickness_mm() / 1000.0,
        },
        glass,
        "glass E gable",
    );
    // Roof slopes: from the ridge (z=0, y=RIDGE_H) to each eave.
    let rise = (RIDGE_H - EAVE_H) as f64;
    let run = hw as f64;
    let slope_len = (rise * rise + run * run).sqrt();
    for (sign, label) in [(-1.0f64, "roof N"), (1.0, "roof S")] {
        // World-frame down-slope vector (0, -rise, sign·run); normal (0, run, sign·rise).
        let n_world = Vector3::new(0.0, run, sign * rise);
        // world → sim: (x, y, z) → (x, -z, y)
        let n_sim = Vector3::new(n_world.x, -n_world.z, n_world.y);
        scene.add_object(
            Primitive::Sheet {
                center: w2s(0.0, (RIDGE_H + EAVE_H) / 2.0, sign as f32 * hw / 2.0),
                normal: Unit::new_normalize(n_sim),
                u_axis: Vector3::x_axis(),
                half_width: (hl + 0.2) as f64,
                half_height: slope_len / 2.0,
                thickness: glazing.thickness_mm() / 1000.0,
            },
            glass,
            label,
        );
    }
    scene
}

fn canopy_detector() -> PlaneDetector {
    PlaneDetector::horizontal(
        Point3::new(0.0, 0.0, CANOPY_H as f64),
        (BARN_LENGTH / 2.0) as f64,
        (BARN_WIDTH / 2.0) as f64,
        MC_NX,
        MC_NZ,
        true,
    )
    .one_sided()
}

/// Trace one source group (all sources share `spectrum`) and accumulate the
/// per-cell lux / PPFD / XYZ into `grid` (Bevy-frame grid indexing).
#[allow(clippy::too_many_arguments)]
fn trace_group(
    glazing: Glazing,
    sources: Vec<Source>,
    spectrum: &SourceSpectrum,
    photons: u64,
    seed: u64,
    electric: bool,
    grid: &mut CanopyGrid,
    lux_only: Option<&mut Vec<f64>>,
) {
    if sources.is_empty() || photons == 0 {
        return;
    }
    let mut scene = build_mc_geometry(glazing);
    for s in sources {
        scene.add_source_with_spectrum(s, spectrum.clone());
    }
    let cfg = TracerConfig {
        num_photons: photons,
        max_bounces: 24,
        seed,
        ..Default::default()
    };
    let result = trace_illuminance(&scene, &cfg, vec![canopy_detector()]);
    let plane = &result.planes[0];
    let ler = SpectralMetrics::from_spd(spectrum.spd())
        .ler_lm_per_w
        .max(1e-6);
    // Radiant watts represented by one unit of recorded photon energy.
    let w_per_e = if result.total_emitted_energy > 0.0 {
        result.total_flux_lm / result.total_emitted_energy / ler
    } else {
        0.0
    };
    let (nu, nv) = plane.dims();
    let cell_area = (BARN_LENGTH as f64 / nu as f64) * (BARN_WIDTH as f64 / nv as f64);
    let Some(channels) = plane.channels() else {
        return;
    };
    let mut lux_only = lux_only;
    for iu in 0..nu {
        for iv in 0..nv {
            let ch = &channels[iu * nv + iv];
            // Detector v axis is sim +Y = world −Z: flip rows.
            let iz = nv - 1 - iv;
            let i = iz * nu + iu;
            let lux = KM_PHOTOPIC * ch.y * w_per_e / cell_area;
            let ppfd = ch.par * w_per_e / cell_area;
            grid.lux[i] += lux;
            grid.ppfd[i] += ppfd;
            if electric {
                grid.electric_ppfd[i] += ppfd;
            }
            grid.xyz[i][0] += ch.x * w_per_e / cell_area;
            grid.xyz[i][1] += ch.y * w_per_e / cell_area;
            grid.xyz[i][2] += ch.z * w_per_e / cell_area;
            if let Some(v) = lux_only.as_deref_mut() {
                v[i] += lux;
            }
        }
    }
}

fn run_monte_carlo(state: &GreenhouseState, day: &DaylightState) -> McResult {
    #[cfg(not(target_arch = "wasm32"))]
    let t0 = std::time::Instant::now();

    let mut grid = CanopyGrid::new(MC_NX, MC_NZ);
    let mut day_lux = vec![0.0; MC_NX * MC_NZ];
    let photons_per_group = state.mc_photons as u64;
    let mut total_photons = 0u64;

    // --- Daylight groups ---
    if day.sun.altitude_deg() > 0.0 {
        let emit_h = RIDGE_H + 3.0;
        // Emission square just large enough to cover the footprint: the sun
        // beam is parallel, so its horizontal footprint is the square itself
        // (shifted), and the sky square only needs a small margin.
        let half_extent = 13.5f64;
        let day_spec = SourceSpectrum::from_cct(day.cct_k);

        if state.sky == SkyKind::Clear && day.avail.dni_lux > 0.0 {
            // Centre the emission square on the beam that hits the building.
            let d = day.sun_dir;
            let t = emit_h / d.y.max(1e-3);
            let origin = w2s(d.x * t, emit_h, d.z * t);
            let flux = day.avail.dni_lux * (2.0 * half_extent).powi(2);
            let src = sun_source(&day.sun, origin, half_extent, flux);
            trace_group(
                state.glazing,
                vec![src],
                &day_spec,
                photons_per_group,
                7,
                false,
                &mut grid,
                Some(&mut day_lux),
            );
            total_photons += photons_per_group;
        }
        if day.avail.dhi_lux > 0.0 {
            let radiance = match state.sky {
                SkyKind::Clear => SkyRadiance::perez_from_dhi(
                    PerezSky::new(day.sun, SkyParams::from_turbidity(state.turbidity as f64)),
                    day.avail.dhi_lux,
                ),
                SkyKind::Overcast => {
                    SkyRadiance::cie_from_dhi(CieSky::Overcast, day.sun, day.avail.dhi_lux)
                }
            };
            let dome = SkyDomeSource::with_extent(
                &radiance,
                Point3::new(0.0, 0.0, emit_h as f64),
                1.0,
                half_extent,
                16,
                32,
            );
            let flux = dome.collector_flux(day.avail.dhi_lux);
            let dome = dome.with_flux(flux).with_spectrum(day_spec.clone());
            trace_group(
                state.glazing,
                vec![Source::SkyDome(dome)],
                &day_spec,
                photons_per_group,
                11,
                false,
                &mut grid,
                Some(&mut day_lux),
            );
            total_photons += photons_per_group;
        }
    }

    // --- Electric group ---
    if state.lamps_on && state.dimming > 0.001 {
        let lamp = &state.lamps[state.lamp.index()];
        let flux = lamp.luminaire_flux_lm * state.dimming as f64;
        let template = Source::from_lvk(
            Point3::new(0.0, 0.0, 0.0),
            Rotation3::identity(),
            lamp.ldt.clone(),
            flux,
        );
        let sources: Vec<Source> = fixture_positions()
            .into_iter()
            .map(|(fx, fz)| {
                let mut s = template.clone();
                if let Source::FromLvk { position, .. } = &mut s {
                    *position = w2s(fx, FIXTURE_H, fz);
                }
                s
            })
            .collect();
        let spec = SourceSpectrum::new(lamp.spd.clone());
        trace_group(
            state.glazing,
            sources,
            &spec,
            photons_per_group,
            23,
            true,
            &mut grid,
            None,
        );
        total_photons += photons_per_group;
    }

    // Daylight factor: bench-average interior daylight / exterior GHI.
    let mut n = 0usize;
    let mut sum = 0.0;
    for iz in 0..MC_NZ {
        for ix in 0..MC_NX {
            let (x, z) = grid.cell_center(ix, iz);
            if x.abs() <= BENCH_LEN / 2.0 && over_bench(z) {
                sum += day_lux[iz * MC_NX + ix];
                n += 1;
            }
        }
    }
    let daylight_factor_pct = if n > 0 && day.avail.ghi_lux > 1.0 {
        100.0 * (sum / n as f64) / day.avail.ghi_lux
    } else {
        0.0
    };

    #[cfg(not(target_arch = "wasm32"))]
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let elapsed_ms = 0.0;

    McResult {
        grid,
        photons: total_photons,
        elapsed_ms,
        daylight_factor_pct,
    }
}

// ---------------------------------------------------------------------------
// Resources & components
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct GreenhouseState {
    site_idx: usize,
    month: u32,
    hour: f32,
    sky: SkyKind,
    turbidity: f32,
    glazing: Glazing,
    lamp: Lamp,
    lamps_on: bool,
    dimming: f32,
    show_heatmap: bool,
    show_models: bool,
    show_solid: bool,
    exposure_bias: f32,
    mc_photons: u32,
    lamps: Vec<LampData>,

    // Derived
    day: DaylightState,
    analytic: CanopyGrid,
    stats: CanopyStats,
    mc: Option<McResult>,
    /// Show the Monte Carlo grid (when available) instead of the analytic one.
    show_mc: bool,

    // Dirty flags
    inputs_dirty: bool,
    lights_dirty: bool,
    glazing_dirty: bool,
    heatmap_dirty: bool,
    run_mc: bool,
}

impl GreenhouseState {
    fn new(lamps: Vec<LampData>) -> Self {
        let mut s = Self {
            site_idx: 0,
            month: 6,
            hour: 14.0,
            sky: SkyKind::Clear,
            turbidity: 3.0,
            glazing: Glazing::FloatGlass,
            lamp: Lamp::FullSpectrum,
            lamps_on: true,
            dimming: 1.0,
            show_heatmap: true,
            show_models: true,
            show_solid: false,
            exposure_bias: 0.0,
            mc_photons: 60_000,
            lamps,
            day: DaylightState {
                sun: SolarPosition {
                    altitude_rad: 0.0,
                    azimuth_rad: 0.0,
                    zenith_rad: std::f64::consts::FRAC_PI_2,
                },
                avail: DaylightAvailability::dark(),
                cct_k: 6500.0,
                sun_dir: Vec3::Y,
                tau_v: 0.9,
                inside: spectral_factors(&d_illuminant(6500.0)),
            },
            analytic: CanopyGrid::new(GRID_NX, GRID_NZ),
            stats: CanopyStats::default(),
            mc: None,
            show_mc: false,
            inputs_dirty: true,
            lights_dirty: true,
            glazing_dirty: true,
            heatmap_dirty: true,
            run_mc: false,
        };
        s.recompute();
        s
    }

    fn lamp_data(&self) -> &LampData {
        &self.lamps[self.lamp.index()]
    }

    fn recompute(&mut self) {
        self.day = compute_daylight(self);
        self.analytic = compute_analytic(self, &self.day);
        self.stats = self.analytic.stats();
        self.mc = None;
        self.show_mc = false;
        self.inputs_dirty = false;
        self.heatmap_dirty = true;
    }

    fn touch(&mut self) {
        self.inputs_dirty = true;
    }

    /// Auto exposure from the canopy illuminance (incident-light metering:
    /// E = 2.5 · 2^EV at ISO 100), plus the user bias.
    fn ev100(&self) -> f32 {
        let e = self.stats.avg_lux.max(40.0) as f32;
        ((e / 2.5).log2() - 0.6 + self.exposure_bias).clamp(6.0, 16.0)
    }

    /// The grid currently shown on the heat-map.
    fn display_grid(&self) -> &CanopyGrid {
        match (&self.mc, self.show_mc) {
            (Some(mc), true) => &mc.grid,
            _ => &self.analytic,
        }
    }

    fn display_stats(&self) -> CanopyStats {
        match (&self.mc, self.show_mc) {
            (Some(mc), true) => mc.grid.stats(),
            _ => self.stats,
        }
    }

    fn heatmap_scale(&self) -> f64 {
        let s = self.display_stats();
        let m = s.max_ppfd.max(self.stats.max_ppfd);
        if m < 50.0 {
            50.0
        } else {
            (m * 1.05 / 50.0).ceil() * 50.0
        }
    }
}

#[derive(Component)]
struct GrowLight;

#[derive(Component)]
struct GlazingSurface;

#[derive(Component)]
struct SunLight;

#[derive(Component)]
struct HeatmapQuad;

#[derive(Resource, Default)]
struct HeatmapAssets {
    material: Option<Handle<StandardMaterial>>,
    image: Option<Handle<Image>>,
}

#[derive(Component)]
struct FlyCamera {
    speed: f32,
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

impl FlyCamera {
    fn from_look_direction(from: Vec3, to: Vec3) -> Self {
        let dir = (to - from).normalize();
        let yaw = dir.z.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
        let pitch = (-dir.y).asin();
        Self {
            speed: 4.0,
            sensitivity: 0.003,
            pitch,
            yaw,
        }
    }
}

const CAM_START: Vec3 = Vec3::new(-10.6, 1.75, 0.0);
const CAM_LOOK: Vec3 = Vec3::new(4.0, 1.5, 0.6);

struct Rng(u64);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (u32::MAX as f32)
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

#[derive(Resource)]
#[allow(dead_code)] // screenshot fields are only read by the native debug system
struct DebugFlags {
    no_atmosphere: bool,
    /// Native debug: save a screenshot to this path after `screenshot_frame`
    /// frames, then quit shortly after. `--screenshot out.png [--frames N]`.
    screenshot: Option<String>,
    screenshot_frame: u32,
    /// Native debug: start with the camera outside (`--outside`).
    outside: bool,
}

#[derive(Resource, Default)]
#[allow(dead_code)]
struct FrameCounter(u32);

// ---------------------------------------------------------------------------
// UI components
// ---------------------------------------------------------------------------

mod ui_colors {
    use bevy::prelude::Color;
    pub const PANEL_BG: Color = Color::srgba(0.04, 0.06, 0.05, 0.86);
    pub const SECTION_BG: Color = Color::srgba(0.08, 0.11, 0.09, 0.6);
    pub const SLIDER_TRACK_COLOR: Color = Color::srgb(0.16, 0.18, 0.16);
    pub const SLIDER_THUMB_COLOR: Color = Color::srgb(0.55, 0.85, 0.40);
    pub const LABEL_COLOR: Color = Color::srgb(0.68, 0.72, 0.66);
    pub const VALUE_COLOR: Color = Color::srgb(0.93, 0.95, 0.90);
    pub const ACCENT: Color = Color::srgb(0.62, 0.90, 0.45);
    pub const BTN_NORMAL: Color = Color::srgb(0.13, 0.15, 0.13);
    pub const BTN_ACTIVE: Color = Color::srgb(0.24, 0.50, 0.28);
    pub const BTN_ACTION: Color = Color::srgb(0.20, 0.32, 0.55);
}
use ui_colors::*;

#[derive(Component)]
struct DashboardPanel;

#[derive(Component)]
struct DashboardSliderThumb;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Site(usize),
    Sky(SkyKind),
    Glazing(Glazing),
    Lamp(Lamp),
    LampsOff,
    RunMc,
    ShowMc(bool),
    ToggleHeatmap,
    ToggleModels,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum SliderBinding {
    Hour,
    Month,
    Turbidity,
    Dimming,
    Exposure,
    Photons,
}

impl SliderBinding {
    fn label(self, v: f32) -> String {
        match self {
            SliderBinding::Hour => {
                let h = v.floor();
                let m = ((v - h) * 60.0).round();
                format!("Local time: {:02}:{:02}", h as i32, m as i32)
            }
            SliderBinding::Month => {
                const NAMES: [&str; 12] = [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ];
                let i = (v.round() as usize).clamp(1, 12) - 1;
                format!("Date: 15 {}", NAMES[i])
            }
            SliderBinding::Turbidity => format!("Turbidity (Linke): {v:.1}"),
            SliderBinding::Dimming => format!("Grow-light dimming: {:.0} %", v * 100.0),
            SliderBinding::Exposure => format!("Exposure bias: {v:+.1} EV"),
            SliderBinding::Photons => format!("MC photons / source group: {:.0} k", v / 1000.0),
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum UiText {
    Site,
    Sun,
    Daylight,
    Glazing,
    Lamp,
    LampSpectral,
    PpfdAvg,
    PpfdRange,
    Uniformity,
    Dli,
    MixCct,
    Electric,
    McStatus,
    McCompare,
    Legend,
    Exposure,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let debug_flags = {
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--selftest") {
            // Headless check of the science path: analytic canopy grid +
            // spectral Monte Carlo for a few presets, no window needed.
            selftest();
            return;
        }
        let value_of = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        DebugFlags {
            no_atmosphere: args.iter().any(|a| a == "--no-atmo"),
            screenshot: value_of("--screenshot"),
            screenshot_frame: value_of("--frames")
                .and_then(|v| v.parse().ok())
                .unwrap_or(90),
            outside: args.iter().any(|a| a == "--outside"),
        }
    };
    #[cfg(target_arch = "wasm32")]
    let debug_flags = DebugFlags {
        no_atmosphere: false,
        screenshot: None,
        screenshot_frame: 0,
        outside: false,
    };

    let lamps = vec![
        LampData::load(LAMP_FULL_SPECTRUM),
        LampData::load(LAMP_RED_BLUE),
        LampData::load(LAMP_SEEDLING),
        LampData::load(LAMP_FAR_RED),
    ];

    #[allow(unused_mut)]
    let mut window = Window {
        title: "Greenhouse Demo — Vom Schweinestall zum Gewächshaus".into(),
        resolution: (1280u32, 720u32).into(),
        present_mode: bevy::window::PresentMode::Fifo,
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#greenhouse-canvas".into());
        window.fit_canvas_to_parent = true;
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }))
    .add_plugins(PhotometricPlugin::<Eulumdat>::default());
    // The `bevy-ui` feature pulls the widget/focus plugins into DefaultPlugins
    // on some feature combinations and not others — add only what's missing.
    if !app.is_plugin_added::<bevy::ui_widgets::popover::PopoverPlugin>() {
        app.add_plugins(UiWidgetsPlugins);
    }
    if !app.is_plugin_added::<InputDispatchPlugin>() {
        app.add_plugins(InputDispatchPlugin);
    }
    if !app.is_plugin_added::<TabNavigationPlugin>() {
        app.add_plugins(TabNavigationPlugin);
    }
    app.insert_resource(GreenhouseState::new(lamps))
        .insert_resource(debug_flags)
        .init_resource::<HeatmapAssets>()
        .init_resource::<PendingCameraReset>()
        .init_resource::<FrameCounter>()
        .add_systems(Startup, (setup_scene, setup_lamps, setup_ui))
        .add_systems(
            Update,
            (
                keyboard_shortcuts,
                recompute_state,
                run_monte_carlo_if_requested,
                apply_daylight,
                apply_glazing,
                apply_lamps,
                apply_heatmap,
                apply_exposure,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                update_ui_texts,
                update_choice_visuals,
                update_slider_visuals,
            ),
        )
        .add_systems(
            Update,
            (
                fly_camera_look,
                fly_camera_move,
                fly_camera_zoom,
                fly_camera_reset,
            )
                .chain(),
        );

    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, debug_screenshot);

    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn selftest() {
    let lamps = vec![
        LampData::load(LAMP_FULL_SPECTRUM),
        LampData::load(LAMP_RED_BLUE),
        LampData::load(LAMP_SEEDLING),
        LampData::load(LAMP_FAR_RED),
    ];
    for l in &lamps {
        println!(
            "lamp {:<40} {:>7.0} lm  {:>5.0} W  ∫I dΩ {:>5.0} lm/klm  LER {:>5.1} lm/W  {:.4} µmol/s·lm  CCT {:.0} K  color {:?}",
            l.name,
            l.lamp_flux_lm,
            l.watts,
            intensity_integral_per_klm(&l.ldt),
            l.ler,
            l.ppfd_per_lux,
            l.cct_k,
            l.color.to_linear()
        );
    }
    let mut state = GreenhouseState::new(lamps);
    state.mc_photons = 20_000;
    let cases: [(&str, u32, f32, SkyKind, Glazing, Option<Lamp>); 5] = [
        (
            "June 14:00 clear, glass, full-spectrum",
            6,
            14.0,
            SkyKind::Clear,
            Glazing::FloatGlass,
            Some(Lamp::FullSpectrum),
        ),
        (
            "June 14:00 clear, shade net, lamps off",
            6,
            14.0,
            SkyKind::Clear,
            Glazing::ShadeNet,
            None,
        ),
        (
            "Dec 10:00 overcast, PC, red/blue",
            12,
            10.0,
            SkyKind::Overcast,
            Glazing::Polycarbonate,
            Some(Lamp::RedBlue),
        ),
        (
            "Night, film, far-red",
            6,
            23.0,
            SkyKind::Clear,
            Glazing::DiffuseFilm,
            Some(Lamp::FarRed),
        ),
        (
            "Night, glass, seedling",
            6,
            23.0,
            SkyKind::Clear,
            Glazing::FloatGlass,
            Some(Lamp::Seedling),
        ),
    ];
    for (label, month, hour, sky, glazing, lamp) in cases {
        state.month = month;
        state.hour = hour;
        state.sky = sky;
        state.glazing = glazing;
        state.lamps_on = lamp.is_some();
        if let Some(l) = lamp {
            state.lamp = l;
        }
        state.recompute();
        let d = &state.day;
        let s = state.stats;
        println!("\n== {label} ==");
        println!(
            "sun alt {:.1}° az {:.0}°  DNI {:.0} DHI {:.0} lx  CCT {:.0} K  τv {:.2}  inside {:.4} µmol/s·lm",
            d.sun.altitude_deg(),
            d.sun.azimuth_deg(),
            d.avail.dni_lux,
            d.avail.dhi_lux,
            d.cct_k,
            d.tau_v,
            d.inside.ppfd_per_lux
        );
        println!(
            "analytic: PPFD avg {:.0} min {:.0} max {:.0}  lux {:.0}  U {:.2}  electric {:.0}%  mix CCT {:.0} K  EV {:.1}",
            s.avg_ppfd, s.min_ppfd, s.max_ppfd, s.avg_lux, s.uniformity, s.electric_share * 100.0, s.mix_cct, state.ev100()
        );
        let day = state.day;
        let mc = run_monte_carlo(&state, &day);
        let m = mc.grid.stats();
        println!(
            "MC ({} photons, {:.0} ms): PPFD avg {:.0} min {:.0} max {:.0}  lux {:.0}  U {:.2}  electric {:.0}%  mix CCT {:.0} K  DF {:.1}%",
            mc.photons, mc.elapsed_ms, m.avg_ppfd, m.min_ppfd, m.max_ppfd, m.avg_lux, m.uniformity, m.electric_share * 100.0, m.mix_cct, mc.daylight_factor_pct
        );
    }
}

/// `--screenshot path [--frames N]`: capture the primary window after N
/// frames (lets the atmosphere LUTs and shadows settle), then exit.
#[cfg(not(target_arch = "wasm32"))]
fn debug_screenshot(
    mut commands: Commands,
    flags: Res<DebugFlags>,
    mut frames: ResMut<FrameCounter>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let Some(path) = flags.screenshot.clone() else {
        return;
    };
    frames.0 += 1;
    if frames.0 == flags.screenshot_frame {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if frames.0 == flags.screenshot_frame + 40 {
        std::process::exit(0);
    }
}

// ---------------------------------------------------------------------------
// Scene setup
// ---------------------------------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    state: Res<GreenhouseState>,
    flags: Res<DebugFlags>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut heatmap: ResMut<HeatmapAssets>,
    mut cluster_settings: ResMut<bevy::light::cluster::GlobalClusterSettings>,
) {
    if let Some(ref mut gpu) = cluster_settings.gpu_clustering {
        gpu.initial_z_slice_list_capacity = 4096;
        gpu.initial_index_list_capacity = 524288;
    }

    commands.insert_resource(ClearColor(Color::srgb(0.55, 0.70, 0.90)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.75, 0.82, 0.95),
        brightness: 200.0,
        affects_lightmapped_meshes: true,
    });

    // Sun
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 0.0,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.05,
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::default().looking_to(-Vec3::Y, Vec3::Z),
        SunLight,
    ));

    // Sky
    if !flags.no_atmosphere {
        let earth = media.add(ScatteringMedium::earth(256, 256));
        commands.spawn(Atmosphere::earth(earth));
    }

    // Camera
    let (cam_start, cam_look) = if flags.outside {
        (Vec3::new(-24.0, 6.0, 20.0), Vec3::new(0.0, 2.5, 0.0))
    } else {
        (CAM_START, CAM_LOOK)
    };
    let cam_transform = Transform::from_translation(cam_start).looking_at(cam_look, Vec3::Y);
    let mut cam = commands.spawn((
        Camera3d::default(),
        Hdr,
        Msaa::Off,
        Tonemapping::AgX,
        Exposure {
            ev100: state.ev100(),
        },
        Bloom {
            intensity: 0.06,
            ..default()
        },
        cam_transform,
        FlyCamera::from_look_direction(cam_start, cam_look),
        NoIndirectDrawing,
        bevy::light::cluster::ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: default(),
            dynamic_resizing: true,
        },
    ));
    if !flags.no_atmosphere {
        cam.insert((
            AtmosphereSettings::default(),
            AtmosphereEnvironmentMapLight::default(),
        ));
    }

    build_greenhouse(&mut commands, &state, &mut meshes, &mut materials);
    build_heatmap_quad(
        &mut commands,
        &state,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut heatmap,
    );
}

fn build_greenhouse(
    commands: &mut Commands,
    state: &GreenhouseState,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let hl = BARN_LENGTH / 2.0;
    let hw = BARN_WIDTH / 2.0;

    // --- Materials ---
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.30, 0.12),
        perceptual_roughness: 0.95,
        ..default()
    });
    let gravel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.46, 0.42),
        perceptual_roughness: 0.95,
        ..default()
    });
    let concrete = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.50, 0.48),
        perceptual_roughness: 0.85,
        ..default()
    });
    let brick = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.24, 0.17),
        perceptual_roughness: 0.92,
        ..default()
    });
    let brick_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.18, 0.13),
        perceptual_roughness: 0.92,
        ..default()
    });
    let aluminium = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.23, 0.25),
        metallic: 0.7,
        perceptual_roughness: 0.45,
        ..default()
    });
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.56, 0.58),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });
    let bench_top = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.32),
        perceptual_roughness: 0.8,
        ..default()
    });
    let door = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.20, 0.10),
        perceptual_roughness: 0.8,
        ..default()
    });
    let pot = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.20, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });
    let foliage: Vec<Handle<StandardMaterial>> =
        [(0.16, 0.42, 0.14), (0.22, 0.50, 0.16), (0.12, 0.36, 0.14)]
            .iter()
            .map(|&(r, g, b)| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(r, g, b),
                    perceptual_roughness: 0.85,
                    ..default()
                })
            })
            .collect();
    let (tint, alpha, rough) = state.glazing.visual();
    let glass = materials.add(StandardMaterial {
        base_color: tint.with_alpha(alpha),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: rough,
        reflectance: 0.35,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    // --- Meshes ---
    let unit_cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let pot_mesh = meshes.add(Cuboid::new(0.13, 0.12, 0.13));
    let leaf_mesh = meshes.add(
        Sphere::new(1.0)
            .mesh()
            .ico(2)
            .unwrap_or_else(|_| Sphere::new(1.0).mesh().uv(12, 8)),
    );

    let spawn_box = |commands: &mut Commands,
                     mat: &Handle<StandardMaterial>,
                     center: Vec3,
                     size: Vec3,
                     rotation: Quat,
                     shadow: bool| {
        let mut e = commands.spawn((
            Mesh3d(unit_cube.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(center)
                .with_rotation(rotation)
                .with_scale(size),
        ));
        if !shadow {
            e.insert(NotShadowCaster);
        }
    };

    // Ground + slab + path
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(300.0, 300.0))),
        MeshMaterial3d(grass.clone()),
        Transform::from_xyz(0.0, -0.03, 0.0),
    ));
    spawn_box(
        commands,
        &gravel,
        Vec3::new(-hl - 4.0, -0.02, 0.0),
        Vec3::new(8.0, 0.04, 4.0),
        Quat::IDENTITY,
        false,
    );
    spawn_box(
        commands,
        &concrete,
        Vec3::new(0.0, -0.05, 0.0),
        Vec3::new(BARN_LENGTH + 2.0 * WALL_T, 0.1, BARN_WIDTH + 2.0 * WALL_T),
        Quat::IDENTITY,
        false,
    );

    // --- Brick socle (the old Schweinestall footprint) ---
    for (z, mat) in [(-hw - WALL_T / 2.0, &brick), (hw + WALL_T / 2.0, &brick)] {
        spawn_box(
            commands,
            mat,
            Vec3::new(0.0, SOCLE_H / 2.0, z),
            Vec3::new(BARN_LENGTH + 2.0 * WALL_T, SOCLE_H, WALL_T),
            Quat::IDENTITY,
            true,
        );
    }
    spawn_box(
        commands,
        &brick,
        Vec3::new(hl + WALL_T / 2.0, SOCLE_H / 2.0, 0.0),
        Vec3::new(WALL_T, SOCLE_H, BARN_WIDTH),
        Quat::IDENTITY,
        true,
    );
    // West gable: full brick wall to the eaves, then a stepped gable (Treppengiebel).
    spawn_box(
        commands,
        &brick,
        Vec3::new(-hl - WALL_T / 2.0, EAVE_H / 2.0, 0.0),
        Vec3::new(WALL_T, EAVE_H, BARN_WIDTH + 2.0 * WALL_T),
        Quat::IDENTITY,
        true,
    );
    let steps = 7;
    for i in 0..steps {
        let y0 = EAVE_H + (RIDGE_H - EAVE_H) * i as f32 / steps as f32;
        let y1 = EAVE_H + (RIDGE_H - EAVE_H) * (i + 1) as f32 / steps as f32;
        let half_w = hw * (1.0 - i as f32 / steps as f32) + WALL_T;
        spawn_box(
            commands,
            if i % 2 == 0 { &brick } else { &brick_dark },
            Vec3::new(-hl - WALL_T / 2.0, (y0 + y1) / 2.0, 0.0),
            Vec3::new(WALL_T, y1 - y0, 2.0 * half_w),
            Quat::IDENTITY,
            true,
        );
    }
    // Old barn door in the west gable.
    spawn_box(
        commands,
        &door,
        Vec3::new(-hl - WALL_T - 0.02, 1.2, 0.0),
        Vec3::new(0.06, 2.4, 2.4),
        Quat::IDENTITY,
        false,
    );

    // --- Glazing ---
    let wall_mid = (SOCLE_H + EAVE_H) / 2.0;
    for z in [-hw, hw] {
        commands.spawn((
            Mesh3d(unit_cube.clone()),
            MeshMaterial3d(glass.clone()),
            Transform::from_xyz(0.0, wall_mid, z).with_scale(Vec3::new(
                BARN_LENGTH,
                EAVE_H - SOCLE_H,
                0.02,
            )),
            NotShadowCaster,
            GlazingSurface,
        ));
    }
    // East gable: rectangle + triangle.
    commands.spawn((
        Mesh3d(unit_cube.clone()),
        MeshMaterial3d(glass.clone()),
        Transform::from_xyz(hl, wall_mid, 0.0).with_scale(Vec3::new(
            0.02,
            EAVE_H - SOCLE_H,
            BARN_WIDTH,
        )),
        NotShadowCaster,
        GlazingSurface,
    ));
    let gable_tri = meshes.add(
        Triangle3d::new(
            Vec3::new(hl, EAVE_H, -hw),
            Vec3::new(hl, RIDGE_H, 0.0),
            Vec3::new(hl, EAVE_H, hw),
        )
        .mesh(),
    );
    commands.spawn((
        Mesh3d(gable_tri),
        MeshMaterial3d(glass.clone()),
        Transform::IDENTITY,
        NotShadowCaster,
        GlazingSurface,
    ));
    // Roof slopes.
    let rise = RIDGE_H - EAVE_H;
    let slope_len = (rise * rise + hw * hw).sqrt();
    let slope_angle = (rise / hw).atan();
    for sign in [-1.0f32, 1.0] {
        let rot = Quat::from_rotation_x(sign * slope_angle);
        let center = Vec3::new(0.0, (RIDGE_H + EAVE_H) / 2.0, sign * hw / 2.0);
        commands.spawn((
            Mesh3d(unit_cube.clone()),
            MeshMaterial3d(glass.clone()),
            Transform::from_translation(center)
                .with_rotation(rot)
                .with_scale(Vec3::new(BARN_LENGTH + 0.4, 0.02, slope_len)),
            NotShadowCaster,
            GlazingSurface,
        ));
        // Rafters / glazing bars along the slope.
        let n_bars = (BARN_LENGTH / 1.2) as i32;
        for i in 0..=n_bars {
            let x = -hl + i as f32 * 1.2;
            spawn_box(
                commands,
                &aluminium,
                center + Vec3::new(x, 0.03, 0.0),
                Vec3::new(0.06, 0.08, slope_len),
                rot,
                true,
            );
        }
        // Purlins.
        for f in [0.33f32, 0.66] {
            let along = (f - 0.5) * slope_len;
            let local = rot * Vec3::new(0.0, 0.03, along);
            spawn_box(
                commands,
                &aluminium,
                center + local,
                Vec3::new(BARN_LENGTH + 0.4, 0.08, 0.06),
                rot,
                true,
            );
        }
        // Gutter at the eave.
        spawn_box(
            commands,
            &aluminium,
            Vec3::new(0.0, EAVE_H - 0.05, sign * (hw + 0.1)),
            Vec3::new(BARN_LENGTH + 0.6, 0.12, 0.14),
            Quat::IDENTITY,
            true,
        );
    }
    // Ridge beam and wall mullions.
    spawn_box(
        commands,
        &aluminium,
        Vec3::new(0.0, RIDGE_H + 0.03, 0.0),
        Vec3::new(BARN_LENGTH + 0.4, 0.12, 0.16),
        Quat::IDENTITY,
        true,
    );
    let n_mullions = (BARN_LENGTH / 1.2) as i32;
    for i in 0..=n_mullions {
        let x = -hl + i as f32 * 1.2;
        for z in [-hw, hw] {
            spawn_box(
                commands,
                &aluminium,
                Vec3::new(x, wall_mid, z),
                Vec3::new(0.06, EAVE_H - SOCLE_H, 0.08),
                Quat::IDENTITY,
                true,
            );
        }
    }
    for z in (-4..=4).map(|i| i as f32 * 1.25) {
        spawn_box(
            commands,
            &aluminium,
            Vec3::new(hl, (SOCLE_H + roof_y(z)) / 2.0, z),
            Vec3::new(0.08, roof_y(z) - SOCLE_H, 0.06),
            Quat::IDENTITY,
            true,
        );
    }
    // Eave-height horizontal rail on the long walls.
    for z in [-hw, hw] {
        spawn_box(
            commands,
            &aluminium,
            Vec3::new(0.0, SOCLE_H + 0.03, z),
            Vec3::new(BARN_LENGTH + 0.4, 0.08, 0.10),
            Quat::IDENTITY,
            true,
        );
    }

    // --- Benches, pots, plants ---
    let mut rng = Rng(20260914);
    let leaf_mesh_h = leaf_mesh.clone();
    for &bz in &BENCH_Z {
        spawn_box(
            commands,
            &bench_top,
            Vec3::new(0.0, BENCH_H - 0.025, bz),
            Vec3::new(BENCH_LEN, 0.05, BENCH_W),
            Quat::IDENTITY,
            true,
        );
        let n_legs = (BENCH_LEN / 2.2) as i32;
        for i in 0..=n_legs {
            let x = -BENCH_LEN / 2.0 + 0.2 + i as f32 * (BENCH_LEN - 0.4) / n_legs as f32;
            for dz in [-BENCH_W / 2.0 + 0.1, BENCH_W / 2.0 - 0.1] {
                spawn_box(
                    commands,
                    &steel,
                    Vec3::new(x, (BENCH_H - 0.05) / 2.0, bz + dz),
                    Vec3::new(0.05, BENCH_H - 0.05, 0.05),
                    Quat::IDENTITY,
                    false,
                );
            }
        }
        let n_pots = (BENCH_LEN / 0.36) as i32;
        for i in 0..n_pots {
            let x = -BENCH_LEN / 2.0 + 0.18 + i as f32 * 0.36;
            for k in 0..4 {
                let z = bz + (k as f32 - 1.5) * 0.36;
                if rng.next_f32() < 0.08 {
                    continue;
                }
                commands.spawn((
                    Mesh3d(pot_mesh.clone()),
                    MeshMaterial3d(pot.clone()),
                    Transform::from_xyz(x, BENCH_H + 0.06, z),
                    NotShadowCaster,
                ));
                let r = rng.range(0.08, 0.15);
                let mat = &foliage[(rng.next_f32() * 3.0) as usize % 3];
                commands.spawn((
                    Mesh3d(leaf_mesh_h.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(x, BENCH_H + 0.08 + r * 0.8, z).with_scale(Vec3::new(
                        r,
                        r * 0.9,
                        r,
                    )),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
            }
        }
    }

    // --- Fixture suspension (chains to the roof) ---
    for (fx, fz) in fixture_positions() {
        let top = roof_y(fz) - 0.1;
        let bottom = FIXTURE_H + 0.05;
        // The luminaire model's length (1.2 m) runs along Z.
        for dz in [-0.5f32, 0.5] {
            spawn_box(
                commands,
                &steel,
                Vec3::new(fx, (top + bottom) / 2.0, fz + dz),
                Vec3::new(0.015, top - bottom, 0.015),
                Quat::IDENTITY,
                false,
            );
        }
    }
}

fn build_heatmap_quad(
    commands: &mut Commands,
    state: &GreenhouseState,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    heatmap: &mut HeatmapAssets,
) {
    let image = images.add(heatmap_image(state));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(image.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    heatmap.image = Some(image);
    heatmap.material = Some(material.clone());
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(BARN_LENGTH, BARN_WIDTH))),
        MeshMaterial3d(material),
        // Floats just above the foliage so it stays readable from eye level.
        Transform::from_xyz(0.0, CANOPY_H + 0.12, 0.0),
        NotShadowCaster,
        NotShadowReceiver,
        HeatmapQuad,
        if state.show_heatmap {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    ));
}

/// Rasterise the displayed canopy grid into an RGBA texture.
fn heatmap_image(state: &GreenhouseState) -> Image {
    let grid = state.display_grid();
    let scale = state.heatmap_scale();
    let mut data = Vec::with_capacity(grid.nx * grid.nz * 4);
    for iz in 0..grid.nz {
        for ix in 0..grid.nx {
            let (x, z) = grid.cell_center(ix, iz);
            let v = (grid.ppfd[iz * grid.nx + ix] / scale).clamp(0.0, 1.0);
            let (r, g, b) = heatmap_color(v);
            let on_bench = x.abs() <= BENCH_LEN / 2.0 && over_bench(z);
            let a = if on_bench { 215 } else { 110 };
            data.extend_from_slice(&[(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, a]);
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: grid.nx as u32,
            height: grid.nz as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

fn setup_lamps(mut commands: Commands, state: Res<GreenhouseState>) {
    spawn_lamps(&mut commands, &state);
}

fn spawn_lamps(commands: &mut Commands, state: &GreenhouseState) {
    if !state.lamps_on || state.dimming <= 0.001 {
        return;
    }
    let lamp = state.lamp_data();
    // The photometric plugin multiplies by 50 for "visibility"; undo that so
    // Bevy's lumens are the real ones and exposure can be physical.
    let scale = state.dimming / 50.0;
    for (fx, fz) in fixture_positions() {
        commands.spawn((
            EulumdatLightBundle::new(lamp.ldt.clone())
                .with_transform(Transform::from_xyz(fx, FIXTURE_H, fz))
                .with_intensity_scale(scale)
                .with_color(lamp.color)
                .with_model(state.show_models)
                .with_solid(state.show_solid),
            GrowLight,
        ));
    }
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

fn keyboard_shortcuts(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GreenhouseState>) {
    let mut changed = false;
    if keys.just_pressed(KeyCode::BracketLeft) {
        state.hour = (state.hour - 0.5).max(0.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        state.hour = (state.hour + 0.5).min(24.0);
        changed = true;
    }
    if keys.just_pressed(KeyCode::Comma) {
        state.month = if state.month <= 1 {
            12
        } else {
            state.month - 1
        };
        changed = true;
    }
    if keys.just_pressed(KeyCode::Period) {
        state.month = if state.month >= 12 {
            1
        } else {
            state.month + 1
        };
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        state.sky = match state.sky {
            SkyKind::Clear => SkyKind::Overcast,
            SkyKind::Overcast => SkyKind::Clear,
        };
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        state.glazing = state.glazing.next();
        state.glazing_dirty = true;
        changed = true;
    }
    for (key, lamp) in [
        (KeyCode::Digit1, Lamp::FullSpectrum),
        (KeyCode::Digit2, Lamp::RedBlue),
        (KeyCode::Digit3, Lamp::Seedling),
        (KeyCode::Digit4, Lamp::FarRed),
    ] {
        if keys.just_pressed(key) {
            state.lamp = lamp;
            state.lamps_on = true;
            state.lights_dirty = true;
            changed = true;
        }
    }
    if keys.just_pressed(KeyCode::Digit0) {
        state.lamps_on = !state.lamps_on;
        state.lights_dirty = true;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        state.dimming = (state.dimming - 0.1).max(0.0);
        state.lights_dirty = true;
        changed = true;
    }
    if keys.just_pressed(KeyCode::Equal) {
        state.dimming = (state.dimming + 0.1).min(1.0);
        state.lights_dirty = true;
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        state.show_heatmap = !state.show_heatmap;
        state.heatmap_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        state.show_models = !state.show_models;
        state.lights_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        state.show_solid = !state.show_solid;
        state.lights_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        state.run_mc = true;
    }
    if changed {
        state.touch();
    }
}

fn recompute_state(mut state: ResMut<GreenhouseState>) {
    if state.inputs_dirty {
        state.recompute();
    }
}

fn run_monte_carlo_if_requested(mut state: ResMut<GreenhouseState>) {
    if !state.run_mc {
        return;
    }
    state.run_mc = false;
    let day = state.day;
    let result = run_monte_carlo(&state, &day);
    let s = result.grid.stats();
    info!(
        "Monte Carlo: {} photons in {:.0} ms — canopy PPFD avg {:.0} (analytic {:.0}) µmol/m²/s, DF {:.1} %",
        result.photons, result.elapsed_ms, s.avg_ppfd, state.stats.avg_ppfd, result.daylight_factor_pct
    );
    state.mc = Some(result);
    state.show_mc = true;
    state.heatmap_dirty = true;
}

fn apply_daylight(
    state: Res<GreenhouseState>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    if !state.is_changed() {
        return;
    }
    let day = &state.day;
    let daytime = day.sun.altitude_deg() > 0.0;
    for (mut light, mut transform) in suns.iter_mut() {
        if daytime {
            // Physical: the beam that reaches the interior is DNI × τ_v.
            light.illuminance = (day.avail.dni_lux * day.tau_v) as f32;
            light.color = kelvin_to_color(day.cct_k as f32);
            *transform = Transform::default().looking_to(-day.sun_dir, Vec3::Y);
        } else {
            light.illuminance = 0.0;
        }
    }
    // Diffuse sky inside the envelope (the atmosphere IBL is un-occluded, so
    // keep this modest and let overcast skies lean on it).
    let sky_inside = day.avail.dhi_lux * day.tau_v * SKY_VIEW_FACTOR;
    let k = match state.sky {
        SkyKind::Clear => 0.10,
        SkyKind::Overcast => 0.45,
    };
    ambient.brightness = (sky_inside * k) as f32 + 2.0;
    ambient.color = kelvin_to_color(day.cct_k as f32);
}

fn apply_glazing(
    mut state: ResMut<GreenhouseState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    glazing: Query<&MeshMaterial3d<StandardMaterial>, With<GlazingSurface>>,
) {
    if !state.glazing_dirty {
        return;
    }
    state.glazing_dirty = false;
    let (tint, alpha, rough) = state.glazing.visual();
    let mut done = std::collections::HashSet::new();
    for handle in glazing.iter() {
        if !done.insert(handle.0.id()) {
            continue;
        }
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.base_color = tint.with_alpha(alpha);
            mat.perceptual_roughness = rough;
        }
    }
}

fn apply_lamps(
    mut state: ResMut<GreenhouseState>,
    mut commands: Commands,
    lamps: Query<Entity, With<GrowLight>>,
    lights: Query<(Entity, &PhotometricLight<Eulumdat>), With<GrowLight>>,
) {
    if !state.lights_dirty {
        return;
    }
    state.lights_dirty = false;
    let lamp = state.lamp_data();
    let same_preset = lights
        .iter()
        .next()
        .map(|(_, l)| l.data.luminaire_name == lamp.ldt.luminaire_name)
        .unwrap_or(false);
    let want_lights = state.lamps_on && state.dimming > 0.001;
    if same_preset && want_lights && lights.iter().count() == fixture_positions().len() {
        // Only dimming / display flags changed: patch in place.
        let scale = state.dimming / 50.0;
        for (entity, light) in lights.iter() {
            let needs = (light.intensity_scale - scale).abs() > 1e-4
                || light.show_model != state.show_models
                || light.show_solid != state.show_solid;
            if needs {
                commands.entity(entity).insert(PhotometricLight {
                    intensity_scale: scale,
                    show_model: state.show_models,
                    show_solid: state.show_solid,
                    ..light.clone()
                });
            }
        }
        return;
    }
    for e in lamps.iter() {
        commands.entity(e).despawn();
    }
    spawn_lamps(&mut commands, &state);
}

fn apply_heatmap(
    mut state: ResMut<GreenhouseState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut heatmap: ResMut<HeatmapAssets>,
    mut quads: Query<&mut Visibility, With<HeatmapQuad>>,
) {
    if !state.heatmap_dirty {
        return;
    }
    state.heatmap_dirty = false;
    for mut vis in quads.iter_mut() {
        *vis = if state.show_heatmap {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.show_heatmap {
        return;
    }
    let new_image = images.add(heatmap_image(&state));
    if let Some(mat_handle) = heatmap.material.clone() {
        if let Some(mut mat) = materials.get_mut(&mat_handle) {
            mat.base_color_texture = Some(new_image.clone());
        }
    }
    if let Some(old) = heatmap.image.replace(new_image) {
        images.remove(&old);
    }
}

fn apply_exposure(state: Res<GreenhouseState>, mut cams: Query<&mut Exposure, With<Camera3d>>) {
    if !state.is_changed() {
        return;
    }
    let ev = state.ev100();
    for mut exp in cams.iter_mut() {
        if (exp.ev100 - ev).abs() > 0.01 {
            exp.ev100 = ev;
        }
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

fn text(parent: &mut ChildSpawnerCommands, s: &str, size: f32, color: Color) -> Entity {
    parent
        .spawn((
            Text::new(s),
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
        ))
        .id()
}

fn bound_text(parent: &mut ChildSpawnerCommands, binding: UiText, size: f32, color: Color) {
    parent.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        binding,
    ));
}

fn separator(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.28, 0.32, 0.28)),
    ));
}

fn section(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    text(parent, title, 12.0, LABEL_COLOR);
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(SECTION_BG),
        ))
        .with_children(build);
}

fn button_row(parent: &mut ChildSpawnerCommands, items: &[(Choice, &str, bool)]) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|row| {
            for &(choice, label, active) in items {
                let bg = match choice {
                    Choice::RunMc => BTN_ACTION,
                    _ if active => BTN_ACTIVE,
                    _ => BTN_NORMAL,
                };
                row.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    UiButton,
                    Hovered::default(),
                    choice,
                    BackgroundColor(bg),
                    observe(on_choice_activate),
                    children![(
                        Text::new(label),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(VALUE_COLOR),
                    )],
                ));
            }
        });
}

fn labeled_slider(
    parent: &mut ChildSpawnerCommands,
    binding: SliderBinding,
    initial: f32,
    min: f32,
    max: f32,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(binding.label(initial)),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(VALUE_COLOR),
            ));
            col.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Stretch,
                    height: Val::Px(14.0),
                    width: Val::Px(290.0),
                    ..default()
                },
                Hovered::default(),
                Slider {
                    track_click: TrackClick::Snap,
                    ..Default::default()
                },
                SliderValue(initial),
                SliderRange::new(min, max),
                binding,
                TabIndex(0),
                observe(slider_self_update),
                observe(on_slider_change),
                Children::spawn((
                    Spawn((
                        Node {
                            height: Val::Px(6.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                    )),
                    Spawn((
                        Node {
                            display: Display::Flex,
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(14.0),
                            top: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            ..default()
                        },
                        children![(
                            DashboardSliderThumb,
                            SliderThumb,
                            Node {
                                display: Display::Flex,
                                width: Val::Px(14.0),
                                height: Val::Px(14.0),
                                position_type: PositionType::Absolute,
                                left: percent(0),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(SLIDER_THUMB_COLOR),
                        )],
                    )),
                )),
            ));
        });
}

fn setup_ui(mut commands: Commands, state: Res<GreenhouseState>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(322.0),
                max_height: Val::Percent(96.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Interaction::default(),
            DashboardPanel,
            TabGroup::default(),
        ))
        .with_children(|panel| {
            text(panel, "SCHWEINESTALL → GEWÄCHSHAUS", 11.0, ACCENT);
            text(panel, "Greenhouse Spectral Lab", 16.0, VALUE_COLOR);
            text(
                panel,
                "[ ] hour   , . month   K sky   G glazing   1-4/0 lamps   T heatmap   M trace",
                9.5,
                Color::srgb(0.5, 0.55, 0.5),
            );
            separator(panel);

            section(panel, "Site & daylight", |s| {
                button_row(
                    s,
                    &[
                        (Choice::Site(0), "Lüdinghausen", state.site_idx == 0),
                        (Choice::Site(1), "Wuhan", state.site_idx == 1),
                        (Choice::Site(2), "Sydney", state.site_idx == 2),
                    ],
                );
                labeled_slider(s, SliderBinding::Month, state.month as f32, 1.0, 12.0);
                labeled_slider(s, SliderBinding::Hour, state.hour, 0.0, 24.0);
                button_row(
                    s,
                    &[
                        (
                            Choice::Sky(SkyKind::Clear),
                            "Clear sky",
                            state.sky == SkyKind::Clear,
                        ),
                        (
                            Choice::Sky(SkyKind::Overcast),
                            "Overcast",
                            state.sky == SkyKind::Overcast,
                        ),
                    ],
                );
                labeled_slider(s, SliderBinding::Turbidity, state.turbidity, 1.5, 8.0);
                bound_text(s, UiText::Sun, 11.0, VALUE_COLOR);
                bound_text(s, UiText::Daylight, 11.0, VALUE_COLOR);
            });

            section(panel, "Glazing (spectral τ(λ))", |s| {
                let items: Vec<(Choice, &str, bool)> = Glazing::ALL
                    .iter()
                    .map(|g| (Choice::Glazing(*g), g.short(), *g == state.glazing))
                    .collect();
                button_row(s, &items);
                bound_text(s, UiText::Glazing, 11.0, VALUE_COLOR);
            });

            section(panel, "Grow lights (TM-33 SPD)", |s| {
                let mut items: Vec<(Choice, &str, bool)> = Lamp::ALL
                    .iter()
                    .map(|l| {
                        (
                            Choice::Lamp(*l),
                            l.short(),
                            state.lamps_on && *l == state.lamp,
                        )
                    })
                    .collect();
                items.push((Choice::LampsOff, "Off", !state.lamps_on));
                button_row(s, &items);
                labeled_slider(s, SliderBinding::Dimming, state.dimming, 0.0, 1.0);
                bound_text(s, UiText::Lamp, 11.0, VALUE_COLOR);
                bound_text(s, UiText::LampSpectral, 11.0, VALUE_COLOR);
            });

            section(panel, "Canopy (bench plane, 1.0 m)", |s| {
                bound_text(s, UiText::PpfdAvg, 14.0, ACCENT);
                bound_text(s, UiText::PpfdRange, 11.0, VALUE_COLOR);
                bound_text(s, UiText::Uniformity, 11.0, VALUE_COLOR);
                bound_text(s, UiText::Dli, 11.0, VALUE_COLOR);
                bound_text(s, UiText::MixCct, 11.0, VALUE_COLOR);
                bound_text(s, UiText::Electric, 11.0, VALUE_COLOR);
                bound_text(s, UiText::Legend, 10.0, LABEL_COLOR);
                button_row(
                    s,
                    &[
                        (Choice::ToggleHeatmap, "Heat-map", state.show_heatmap),
                        (Choice::ToggleModels, "Models", state.show_models),
                    ],
                );
            });

            section(panel, "Spectral Monte Carlo (goniosim)", |s| {
                labeled_slider(
                    s,
                    SliderBinding::Photons,
                    state.mc_photons as f32,
                    10_000.0,
                    300_000.0,
                );
                button_row(
                    s,
                    &[
                        (Choice::RunMc, "Trace  (M)", false),
                        (Choice::ShowMc(false), "Analytic", !state.show_mc),
                        (Choice::ShowMc(true), "MC", state.show_mc),
                    ],
                );
                bound_text(s, UiText::McStatus, 11.0, VALUE_COLOR);
                bound_text(s, UiText::McCompare, 11.0, VALUE_COLOR);
            });

            section(panel, "Display", |s| {
                labeled_slider(s, SliderBinding::Exposure, state.exposure_bias, -3.0, 3.0);
                bound_text(s, UiText::Exposure, 10.0, LABEL_COLOR);
                bound_text(s, UiText::Site, 10.0, LABEL_COLOR);
            });
        });
}

fn on_choice_activate(
    activate: On<Activate>,
    choices: Query<&Choice>,
    mut state: ResMut<GreenhouseState>,
) {
    let Ok(choice) = choices.get(activate.entity) else {
        return;
    };
    match *choice {
        Choice::Site(i) => {
            state.site_idx = i;
            state.touch();
        }
        Choice::Sky(k) => {
            state.sky = k;
            state.touch();
        }
        Choice::Glazing(g) => {
            state.glazing = g;
            state.glazing_dirty = true;
            state.touch();
        }
        Choice::Lamp(l) => {
            state.lamp = l;
            state.lamps_on = true;
            state.lights_dirty = true;
            state.touch();
        }
        Choice::LampsOff => {
            state.lamps_on = false;
            state.lights_dirty = true;
            state.touch();
        }
        Choice::RunMc => state.run_mc = true,
        Choice::ShowMc(show) => {
            if state.mc.is_some() || !show {
                state.show_mc = show;
                state.heatmap_dirty = true;
            }
        }
        Choice::ToggleHeatmap => {
            state.show_heatmap = !state.show_heatmap;
            state.heatmap_dirty = true;
        }
        Choice::ToggleModels => {
            state.show_models = !state.show_models;
            state.lights_dirty = true;
        }
    }
}

fn on_slider_change(
    value_change: On<ValueChange<f32>>,
    bindings: Query<&SliderBinding>,
    mut state: ResMut<GreenhouseState>,
) {
    let Ok(binding) = bindings.get(value_change.source) else {
        return;
    };
    let v = value_change.value;
    match binding {
        SliderBinding::Hour => {
            state.hour = (v * 4.0).round() / 4.0;
            state.touch();
        }
        SliderBinding::Month => {
            state.month = (v.round() as u32).clamp(1, 12);
            state.touch();
        }
        SliderBinding::Turbidity => {
            state.turbidity = v;
            state.touch();
        }
        SliderBinding::Dimming => {
            state.dimming = (v * 20.0).round() / 20.0;
            state.lights_dirty = true;
            state.touch();
        }
        SliderBinding::Exposure => state.exposure_bias = v,
        SliderBinding::Photons => state.mc_photons = ((v / 10_000.0).round() * 10_000.0) as u32,
    }
}

fn update_ui_texts(state: Res<GreenhouseState>, mut texts: Query<(&UiText, &mut Text)>) {
    if !state.is_changed() {
        return;
    }
    let day = &state.day;
    let lamp = state.lamp_data();
    let stats = state.display_stats();
    let loc = site(state.site_idx);
    let mode = if state.show_mc && state.mc.is_some() {
        "Monte Carlo"
    } else {
        "analytic"
    };
    for (binding, mut t) in texts.iter_mut() {
        **t = match binding {
            UiText::Site => format!(
                "{}  {:.2}°N {:.2}°E  UTC{:+.1}",
                loc.name,
                loc.latitude_deg,
                loc.longitude_deg,
                loc.effective_offset(state.month, 15)
            ),
            UiText::Sun => {
                if day.sun.altitude_deg() > 0.0 {
                    format!(
                        "Sun: alt {:.1}°  az {:.0}°   daylight CCT {:.0} K",
                        day.sun.altitude_deg(),
                        day.sun.azimuth_deg(),
                        day.cct_k
                    )
                } else {
                    format!("Sun below horizon ({:.1}°) — night", day.sun.altitude_deg())
                }
            }
            UiText::Daylight => format!(
                "Outside: DNI {:.0}  DHI {:.0}  GHI {:.0} klx",
                day.avail.dni_lux / 1000.0,
                day.avail.dhi_lux / 1000.0,
                day.avail.ghi_lux / 1000.0
            ),
            UiText::Glazing => format!(
                "{}: τv {:.0} %  haze {:.0} %  →  {:.3} µmol/s per lm inside",
                state.glazing.label(),
                day.tau_v * 100.0,
                state.glazing.haze_pct(),
                day.inside.ppfd_per_lux
            ),
            UiText::Lamp => {
                if state.lamps_on {
                    format!(
                        "{} — {:.0} lm, {:.0} W × {} fixtures",
                        lamp.name,
                        lamp.luminaire_flux_lm * state.dimming as f64,
                        lamp.watts * state.dimming as f64,
                        fixture_positions().len()
                    )
                } else {
                    "Grow lights off".to_string()
                }
            }
            UiText::LampSpectral => {
                let cct = if lamp.cct_k.is_finite() && lamp.cct_k > 1000.0 && lamp.cct_k < 30000.0 {
                    format!("{:.0} K", lamp.cct_k)
                } else {
                    "off-locus".to_string()
                };
                let ppf_per_w = if lamp.watts > 0.0 {
                    lamp.luminaire_flux_lm * lamp.ppfd_per_lux / lamp.watts
                } else {
                    0.0
                };
                format!(
                    "SPD: {cct}  LER {:.0} lm/W  {:.3} µmol/s·lm  {:.2} µmol/J",
                    lamp.ler, lamp.ppfd_per_lux, ppf_per_w
                )
            }
            UiText::PpfdAvg => format!("PPFD avg {:.0} µmol·m⁻²·s⁻¹  ({mode})", stats.avg_ppfd),
            UiText::PpfdRange => format!(
                "min {:.0}  max {:.0}   avg {:.0} lux",
                stats.min_ppfd, stats.max_ppfd, stats.avg_lux
            ),
            UiText::Uniformity => format!("Uniformity min/avg {:.2}", stats.uniformity),
            UiText::Dli => format!(
                "DLI at this level for 16 h: {:.1} mol·m⁻²·d⁻¹",
                stats.avg_ppfd * 16.0 * 3600.0 / 1e6
            ),
            UiText::MixCct => {
                if stats.mix_cct > 0.0 && stats.mix_cct.is_finite() {
                    format!("Mixed light at canopy: {:.0} K", stats.mix_cct)
                } else {
                    "Mixed light at canopy: —".to_string()
                }
            }
            UiText::Electric => format!(
                "Electric share of PPFD: {:.0} %",
                stats.electric_share * 100.0
            ),
            UiText::Legend => format!(
                "Heat-map scale 0 … {:.0} µmol·m⁻²·s⁻¹ (blue → red)",
                state.heatmap_scale()
            ),
            UiText::McStatus => match &state.mc {
                Some(mc) if mc.elapsed_ms > 0.0 => {
                    format!("{} photons traced in {:.0} ms", mc.photons, mc.elapsed_ms)
                }
                Some(mc) => format!("{} photons traced", mc.photons),
                None => "No trace yet — press M or Trace".to_string(),
            },
            UiText::McCompare => match &state.mc {
                Some(mc) => {
                    let s = mc.grid.stats();
                    format!(
                        "MC avg {:.0} vs analytic {:.0} µmol  ·  daylight factor {:.1} %",
                        s.avg_ppfd, state.stats.avg_ppfd, mc.daylight_factor_pct
                    )
                }
                None => String::new(),
            },
            UiText::Exposure => format!("Auto exposure EV100 {:.1}", state.ev100()),
        };
    }
}

fn update_choice_visuals(
    state: Res<GreenhouseState>,
    mut buttons: Query<(&Choice, &mut BackgroundColor)>,
) {
    if !state.is_changed() {
        return;
    }
    for (choice, mut bg) in buttons.iter_mut() {
        let active = match *choice {
            Choice::Site(i) => state.site_idx == i,
            Choice::Sky(k) => state.sky == k,
            Choice::Glazing(g) => state.glazing == g,
            Choice::Lamp(l) => state.lamps_on && state.lamp == l,
            Choice::LampsOff => !state.lamps_on,
            Choice::RunMc => {
                bg.0 = BTN_ACTION;
                continue;
            }
            Choice::ShowMc(show) => (state.show_mc && state.mc.is_some()) == show,
            Choice::ToggleHeatmap => state.show_heatmap,
            Choice::ToggleModels => state.show_models,
        };
        bg.0 = if active { BTN_ACTIVE } else { BTN_NORMAL };
    }
}

fn update_slider_visuals(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange, &SliderBinding),
        (Changed<SliderValue>, With<Slider>),
    >,
    children_q: Query<&Children>,
    mut thumbs: Query<&mut Node, With<DashboardSliderThumb>>,
    parents: Query<&ChildOf>,
    mut texts: Query<&mut Text>,
    children_list: Query<&Children>,
) {
    for (slider_ent, value, range, binding) in sliders.iter() {
        let position = range.thumb_position(value.0) * 100.0;
        for child in children_q.iter_descendants(slider_ent) {
            if let Ok(mut thumb_node) = thumbs.get_mut(child) {
                thumb_node.left = percent(position);
            }
        }
        if let Ok(child_of) = parents.get(slider_ent) {
            let parent = child_of.parent();
            if let Ok(siblings) = children_list.get(parent) {
                for sibling in siblings.iter() {
                    if sibling != slider_ent {
                        if let Ok(mut text) = texts.get_mut(sibling) {
                            **text = binding.label(value.0);
                        }
                    }
                }
            }
        }
    }
}

fn ui_wants_pointer(panel: &Query<&Interaction, With<DashboardPanel>>) -> bool {
    panel.iter().any(|i| *i != Interaction::None)
}

// ---------------------------------------------------------------------------
// Fly camera
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct PendingCameraReset(bool);

fn fly_camera_look(
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
    accumulated: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    panel: Query<&Interaction, With<DashboardPanel>>,
) {
    let mut key_delta = Vec2::ZERO;
    let rotate_speed = 1.5;
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::ArrowLeft) {
        key_delta.x -= rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        key_delta.x += rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        key_delta.y -= rotate_speed * dt / 0.003;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        key_delta.y += rotate_speed * dt / 0.003;
    }
    let mouse_delta = if mouse_button.pressed(MouseButton::Right) && !ui_wants_pointer(&panel) {
        accumulated.delta
    } else {
        Vec2::ZERO
    };
    let delta = mouse_delta + key_delta;
    if delta == Vec2::ZERO {
        return;
    }
    for (mut transform, mut cam) in query.iter_mut() {
        cam.yaw -= delta.x * cam.sensitivity;
        cam.pitch -= delta.y * cam.sensitivity;
        cam.pitch = cam.pitch.clamp(-1.5, 1.5);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    }
}

fn fly_camera_move(
    mut query: Query<(&mut Transform, &FlyCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut transform, cam) in query.iter_mut() {
        let mut dir = Vec3::ZERO;
        let fwd = transform.forward();
        let fwd_flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let right = transform.right();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        if keys.pressed(KeyCode::KeyW) {
            dir += fwd_flat;
        }
        if keys.pressed(KeyCode::KeyS) {
            dir -= fwd_flat;
        }
        if keys.pressed(KeyCode::KeyA) {
            dir -= right_flat;
        }
        if keys.pressed(KeyCode::KeyD) {
            dir += right_flat;
        }
        if keys.pressed(KeyCode::KeyQ) {
            dir += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyE) {
            dir -= Vec3::Y;
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            cam.speed * 3.0
        } else {
            cam.speed
        };
        if dir != Vec3::ZERO {
            transform.translation += dir.normalize() * speed * time.delta_secs();
            transform.translation.y = transform.translation.y.max(0.3);
        }
    }
}

fn fly_camera_zoom(
    mut query: Query<(&mut Transform, &FlyCamera)>,
    accumulated: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    panel: Query<&Interaction, With<DashboardPanel>>,
) {
    if ui_wants_pointer(&panel) {
        return;
    }
    let dy = accumulated.delta.y;
    if dy.abs() > 0.0 {
        for (mut transform, cam) in query.iter_mut() {
            let fwd = transform.forward();
            transform.translation += fwd * dy * cam.speed * 0.5;
        }
    }
}

fn fly_camera_reset(
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut pending: ResMut<PendingCameraReset>,
) {
    if !(keys.just_pressed(KeyCode::KeyR) || pending.0) {
        return;
    }
    pending.0 = false;
    for (mut transform, mut cam) in query.iter_mut() {
        *transform = Transform::from_translation(CAM_START).looking_at(CAM_LOOK, Vec3::Y);
        let new_cam = FlyCamera::from_look_direction(CAM_START, CAM_LOOK);
        cam.yaw = new_cam.yaw;
        cam.pitch = new_cam.pitch;
    }
}
