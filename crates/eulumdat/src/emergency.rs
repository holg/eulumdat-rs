//! EN 1838 emergency lighting: escape routes, open (anti-panic) areas,
//! high-risk task areas, points of emphasis, disability glare and colour
//! rendering.
//!
//! EN 1838 counts direct light only (no inter-reflections), from the
//! luminaires' emergency flux at the end of the rated duration, times a
//! maintenance factor. The limit values are data
//! (`data/en1838_en17037_limits.json`, [`Limits::en1838`]); a national
//! profile can pass its own table. The illuminance kernel is the one of the
//! area / interior designers (same rotation convention: `rotation` = C0
//! direction, degrees counter-clockwise from +x).
//!
//! Plan coordinates are metres on the floor (x, y), heights above the floor.
//!
//! Profiles: [`Limits::en1838`] (Europe) and [`Limits::gb55037`] (China:
//! GB 55037-2022 §10.1.10, a minimum floor illuminance per kind of place,
//! [`PlaceCategory`], without the EN extras: no uniformity ratio, no
//! vertical illuminance at equipment, no glare or Ra limit). Routes and
//! areas carry their place category; EN 1838 ignores it.

use crate::area::{compute_illuminance_at_point, AreaPolygon, LuminairePlace};
use crate::{Eulumdat, PhotometricCalculations};

// ─── limit values ────────────────────────────────────────────────────

/// Escape routes. A value of 0 (minimums, average) or ∞ (ratio, width)
/// means the profile has no such rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteLimits {
    /// Minimum on the centre line (lx).
    pub centre_min_lx: f64,
    /// Minimum over the band (`band_share` of the width, centred) (lx).
    pub band_min_lx: f64,
    /// Maximum Emax / Emin: on the centre line, or over the band when
    /// `ratio_over_band`.
    pub max_ratio: f64,
    /// Minimum average over the band (lx).
    #[cfg_attr(feature = "serde", serde(default))]
    pub avg_min_lx: f64,
    /// Share of the width the band covers (EN 1838: the central half).
    #[cfg_attr(feature = "serde", serde(default = "half"))]
    pub band_share: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ratio_over_band: bool,
    /// Routes wider than this are to be split into strips or treated as
    /// open areas (EN 1838: 2 m); reported.
    #[cfg_attr(feature = "serde", serde(default = "unlimited"))]
    pub max_width_m: f64,
    /// The strip left out along each side is at most this wide (EN
    /// 1838:2024: min(W / 4, 0.5 m), so a route wider than 2 m is lit over
    /// W − 1 m); ∞ = always `(1 − band_share) / 2` of the width.
    #[cfg_attr(feature = "serde", serde(default = "unlimited"))]
    pub edge_max_m: f64,
    /// At least this many luminaires light the route (luminaires within
    /// half the width + 0.5 m of the centre line), on routes longer than
    /// `count_above_m`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_luminaires: usize,
    #[cfg_attr(feature = "serde", serde(default))]
    pub count_above_m: f64,
    /// Largest distance between neighbouring luminaires along the route,
    /// and from each end to the nearest one (m; France: 15 m).
    #[cfg_attr(feature = "serde", serde(default = "unlimited"))]
    pub max_spacing_m: f64,
}

#[cfg(feature = "serde")]
fn half() -> f64 {
    0.5
}

#[cfg(feature = "serde")]
fn unlimited() -> f64 {
    f64::INFINITY
}

/// Open (anti-panic) areas, or rooms for profiles that light whole floors.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaLimits {
    /// Minimum on the floor of the core area (lx).
    pub min_lx: f64,
    /// Border left out along the walls (m).
    pub border_m: f64,
    /// Maximum Emax / Emin.
    pub max_ratio: f64,
    /// Areas larger than this need anti-panic lighting (m²).
    pub applies_above_m2: f64,
    /// Minimum average (lx; 0 = none).
    #[cfg_attr(feature = "serde", serde(default))]
    pub avg_min_lx: f64,
    /// The minimum holds on every horizontal plane from the floor up to
    /// this height (Spain: 1 m; checked on the floor and at this height).
    #[cfg_attr(feature = "serde", serde(default))]
    pub planes_up_to_m: f64,
    /// At least this many luminaires in the area.
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_luminaires: usize,
    /// Luminaire flux per floor area (lm/m², France: 5).
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_flux_per_m2: f64,
    /// Distance to the nearest neighbouring luminaire at most this many
    /// mounting heights (France: 4).
    #[cfg_attr(feature = "serde", serde(default = "unlimited"))]
    pub max_spacing_per_height: f64,
}

/// Values allowed at the end of the rated duration, when a profile
/// states them apart from the initial ones (US: NFPA 101 lets the
/// illuminance fall to 60 % after 1.5 h). Checked with the output at the
/// end of the duration ([`Settings::end_output_share`]).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EndOfDuration {
    pub after_h: f64,
    /// Minimum average and minimum at any point (lx; 0 = none).
    pub avg_min_lx: f64,
    pub min_lx: f64,
}

/// High-risk task areas.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HighRiskLimits {
    /// Percentage of the normal maintained illuminance of the task.
    pub pct_of_normal: f64,
    /// Absolute minimum (lx).
    pub abs_min_lx: f64,
    /// Maximum Emax / Emin.
    pub max_ratio: f64,
}

impl RouteLimits {
    /// No route rule (start for a profile: `..RouteLimits::none()`); the
    /// band is the central half.
    pub fn none() -> Self {
        RouteLimits {
            centre_min_lx: 0.0,
            band_min_lx: 0.0,
            max_ratio: f64::INFINITY,
            avg_min_lx: 0.0,
            band_share: 0.5,
            ratio_over_band: false,
            max_width_m: f64::INFINITY,
            edge_max_m: f64::INFINITY,
            min_luminaires: 0,
            count_above_m: 0.0,
            max_spacing_m: f64::INFINITY,
        }
    }

    /// Half the width of the checked band of a route `width` wide.
    pub fn band_half_width(&self, width: f64) -> f64 {
        let edge = (width * (1.0 - self.band_share) / 2.0).min(self.edge_max_m);
        (width / 2.0 - edge).max(0.0)
    }
}

impl AreaLimits {
    /// No area rule (start for a profile: `..AreaLimits::none()`): the
    /// whole floor, no border.
    pub fn none() -> Self {
        AreaLimits {
            min_lx: 0.0,
            border_m: 0.0,
            max_ratio: f64::INFINITY,
            applies_above_m2: 0.0,
            avg_min_lx: 0.0,
            planes_up_to_m: 0.0,
            min_luminaires: 0,
            min_flux_per_m2: 0.0,
            max_spacing_per_height: f64::INFINITY,
        }
    }
}

impl HighRiskLimits {
    /// No high-risk rule.
    pub fn none() -> Self {
        HighRiskLimits {
            pct_of_normal: 0.0,
            abs_min_lx: 0.0,
            max_ratio: f64::INFINITY,
        }
    }
}

/// Luminous intensity limit between 60° and 90° from the downward vertical
/// for mounting heights below `below_height_m`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlareStep {
    pub below_height_m: f64,
    /// Escape routes and open areas (cd).
    pub escape_cd: f64,
    /// High-risk task areas (cd).
    pub high_risk_cd: f64,
}

impl GlareStep {
    /// No glare limit.
    pub fn none() -> Vec<Self> {
        vec![GlareStep {
            below_height_m: f64::INFINITY,
            escape_cd: f64::INFINITY,
            high_risk_cd: f64::INFINITY,
        }]
    }
}

/// Response times and durations (reported, not computed).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timing {
    /// 50 % of the illuminance within (s).
    pub half_s: f64,
    /// 100 % within (s).
    pub full_s: f64,
    /// High-risk areas: 100 % within (s).
    pub high_risk_full_s: f64,
    /// Rated duration of the emergency supply, by kind of building.
    #[cfg_attr(feature = "serde", serde(default))]
    pub durations: Vec<DurationRule>,
}

/// A rated duration (h) and where it applies (English, for reports).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DurationRule {
    pub hours: f64,
    pub applies_to: String,
}

impl DurationRule {
    fn new(hours: f64, applies_to: &str) -> Self {
        DurationRule {
            hours,
            applies_to: applies_to.into(),
        }
    }
}

/// What a place is, for profiles whose minimum depends on it (China:
/// GB 55037-2022 §10.1.10). EN 1838 does not use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlaceCategory {
    /// Evacuation corridor (疏散走道).
    #[default]
    #[cfg_attr(
        feature = "serde",
        serde(rename = "corridor", alias = "escape_corridor")
    )]
    EscapeCorridor,
    /// Evacuation stairwell and its front room, refuge corridor and its
    /// front room, refuge floor / room, fire-fighting-only passage
    /// (疏散楼梯间、前室或合用前室、避难走道、避难层、避难间、消防专用通道).
    #[cfg_attr(
        feature = "serde",
        serde(rename = "stairs", alias = "stairs_or_refuge")
    )]
    StairsOrRefuge,
    /// Crowded place (人员密集的场所).
    #[cfg_attr(feature = "serde", serde(rename = "crowded", alias = "crowded_place"))]
    CrowdedPlace,
    /// Any other place (其他场所).
    #[cfg_attr(feature = "serde", serde(rename = "other", alias = "other_place"))]
    OtherPlace,
}

impl PlaceCategory {
    pub const ALL: [PlaceCategory; 4] = [
        PlaceCategory::EscapeCorridor,
        PlaceCategory::StairsOrRefuge,
        PlaceCategory::CrowdedPlace,
        PlaceCategory::OtherPlace,
    ];

    /// Short key (links, JSON).
    pub fn key(self) -> &'static str {
        match self {
            PlaceCategory::EscapeCorridor => "corridor",
            PlaceCategory::StairsOrRefuge => "stairs",
            PlaceCategory::CrowdedPlace => "crowded",
            PlaceCategory::OtherPlace => "other",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == k)
    }

    /// English name for reports.
    pub fn words(self) -> &'static str {
        match self {
            PlaceCategory::EscapeCorridor => "evacuation corridor",
            PlaceCategory::StairsOrRefuge => "stairs / front room / refuge",
            PlaceCategory::CrowdedPlace => "crowded place",
            PlaceCategory::OtherPlace => "other place",
        }
    }
}

/// Minimum floor illuminance per place (profiles of that shape). The
/// route / area band and border come from the profile's
/// [`RouteLimits`] / [`AreaLimits`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceLimits {
    pub stairs_or_refuge_lx: f64,
    pub corridor_lx: f64,
    pub crowded_lx: f64,
    pub other_lx: f64,
    /// The kinds of place a route / a room can be in this profile (the
    /// choices a planner gets), first = default.
    pub route_places: Vec<PlaceCategory>,
    pub area_places: Vec<PlaceCategory>,
}

impl PlaceLimits {
    /// Minimum floor illuminance (lx) for a place.
    pub fn required(&self, place: PlaceCategory) -> f64 {
        match place {
            PlaceCategory::StairsOrRefuge => self.stairs_or_refuge_lx,
            PlaceCategory::EscapeCorridor => self.corridor_lx,
            PlaceCategory::CrowdedPlace => self.crowded_lx,
            PlaceCategory::OtherPlace => self.other_lx,
        }
    }
}

/// One set of limit values (a profile): what a standard checks and how.
/// A minimum or average of 0, or a ratio / glare limit of ∞, means the
/// standard has no such rule; [`Limits::all`] lists the profiles.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Limits {
    /// Profile id ("en1838", "gb55037", …) and the name reports show.
    pub id: String,
    pub name: String,
    /// Where the profile applies, and the clauses its checks follow
    /// (English, for reports).
    #[cfg_attr(feature = "serde", serde(default))]
    pub region: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub clauses: String,
    /// Profiles with a minimum per kind of place (China …); `None` = one
    /// set of route and area values.
    pub places: Option<PlaceLimits>,
    pub escape_route: RouteLimits,
    /// Open (anti-panic) areas; for profiles that light whole floors, the
    /// room's floor.
    pub anti_panic: AreaLimits,
    /// High-risk task areas (a minimum of 0 = no such rule).
    pub high_risk: HighRiskLimits,
    /// Vertical illuminance at safety equipment and externally lit signs
    /// (lx; 0 = no such rule); horizontal at the equipment's height when
    /// `emphasis_horizontal` (Spain).
    pub emphasis_min_vertical_lx: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub emphasis_horizontal: bool,
    /// The kinds of point that need it (EN 1838:2024 added door releases,
    /// escape plans and evacuation aids).
    #[cfg_attr(feature = "serde", serde(default = "emphasis_2013"))]
    pub emphasis_kinds: Vec<EmphasisKind>,
    /// Horizontal illuminance on the floor in front of each exit (lx;
    /// Russia: 5), measured 0.5 m in front of it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub exit_floor_min_lx: f64,
    /// Emergency flux of every luminaire at least (lm; France: 45).
    #[cfg_attr(feature = "serde", serde(default))]
    pub min_luminaire_lm: f64,
    /// Illuminance is shown for information only (France: the rules are
    /// lumens and spacings).
    #[cfg_attr(feature = "serde", serde(default))]
    pub lux_informative: bool,
    /// Ascending by height (∞ cd = no limit).
    pub glare: Vec<GlareStep>,
    /// Minimum colour rendering index Ra of the emergency light (0 = none).
    pub min_ra: f64,
    pub timing: Timing,
    /// Lower values allowed at the end of the rated duration (US).
    #[cfg_attr(feature = "serde", serde(default))]
    pub end_of_duration: Option<EndOfDuration>,
    /// How the values are measured and where they come from, what is not
    /// calculated (English lines for reports).
    #[cfg_attr(feature = "serde", serde(default))]
    pub notes: Vec<String>,
}

/// No limit (ratios, glare).
const NONE: f64 = f64::INFINITY;

/// Points of emphasis with a vertical illuminance rule in EN 1838:2013.
fn emphasis_2013() -> Vec<EmphasisKind> {
    use EmphasisKind as E;
    vec![
        E::FireEquipment,
        E::FirstAid,
        E::CallPoint,
        E::ExternallyLitSign,
    ]
}

/// … and in EN 1838:2024.
fn emphasis_2024() -> Vec<EmphasisKind> {
    use EmphasisKind as E;
    let mut v = emphasis_2013();
    v.extend([E::DoorRelease, E::EscapePlan, E::EvacuationAid]);
    v
}

impl Limits {
    /// EN 1838 (the values of `data/en1838_en17037_limits.json`; a test
    /// keeps both in step).
    pub fn en1838() -> Self {
        let step = |h: f64, e: f64, r: f64| GlareStep {
            below_height_m: h,
            escape_cd: e,
            high_risk_cd: r,
        };
        Limits {
            id: "en1838".into(),
            name: "EN 1838:2013".into(),
            region: "Europe: EU / EFTA (DIN EN 1838, BS EN 1838, NF EN 1838, UNE-EN 1838 …); withdrawn on 18 June 2027, replaced by EN 1838:2024".into(),
            clauses: "EN 1838:2013 escape routes, open (anti-panic) areas, high-risk task areas, points of emphasis, glare and colour rendering".into(),
            places: None,
            escape_route: RouteLimits {
                centre_min_lx: 1.0,
                band_min_lx: 0.5,
                max_ratio: 40.0,
                avg_min_lx: 0.0,
                band_share: 0.5,
                ratio_over_band: false,
                max_width_m: 2.0,
                ..RouteLimits::none()
            },
            anti_panic: AreaLimits {
                min_lx: 0.5,
                border_m: 0.5,
                max_ratio: 40.0,
                applies_above_m2: 60.0,
                ..AreaLimits::none()
            },
            high_risk: HighRiskLimits {
                pct_of_normal: 10.0,
                abs_min_lx: 15.0,
                max_ratio: 10.0,
            },
            emphasis_min_vertical_lx: 5.0,
            emphasis_horizontal: false,
            emphasis_kinds: emphasis_2013(),
            exit_floor_min_lx: 0.0,
            min_luminaire_lm: 0.0,
            lux_informative: false,
            glare: vec![
                step(2.5, 500.0, 1000.0),
                step(3.0, 900.0, 1800.0),
                step(3.5, 1600.0, 3200.0),
                step(4.0, 2500.0, 5000.0),
                step(4.5, 3500.0, 7000.0),
                step(NONE, 5000.0, 10000.0),
            ],
            min_ra: 40.0,
            timing: Timing {
                half_s: 5.0,
                full_s: 60.0,
                high_risk_full_s: 0.5,
                durations: vec![DurationRule::new(
                    1.0,
                    "escape routes and open areas (minimum; national rules ask more for some buildings)",
                )],
            },
            end_of_duration: None,
            notes: vec![
                "Escape routes up to 2 m wide: centre line and the central band of half the width; wider routes are split into strips or treated as open areas.".into(),
                "Open areas larger than 60 m² (or smaller ones on an escape route), without a 0.5 m border along the walls.".into(),
                "Direct light only, at the end of the rated duration.".into(),
            ],
        }
    }

    /// China: GB 55037-2022 §10.1.10 (the values of
    /// `data/cn_gb55037_limits.json`; a test keeps both in step). Minimum
    /// horizontal illuminance on the floor by place: 10 lx stairs, front
    /// rooms and refuge spaces, 3 lx evacuation corridors and crowded
    /// places, 1 lx elsewhere; no uniformity ratio, no vertical illuminance
    /// at equipment, no glare or Ra limit. Where GB 51309-2018 gives lower
    /// floor values (corridors 1 lx, stairs 5 lx) the general code
    /// governs; its durations and switch-over times are reported.
    pub fn gb55037() -> Self {
        use PlaceCategory as P;
        Limits {
            id: "gb55037".into(),
            name: "GB 55037-2022".into(),
            region: "China (mainland)".into(),
            clauses: "GB 55037-2022 §10.1.10 (minimum floor illuminance by place); durations and switch-over times GB 51309-2018".into(),
            places: Some(PlaceLimits {
                stairs_or_refuge_lx: 10.0,
                corridor_lx: 3.0,
                crowded_lx: 3.0,
                other_lx: 1.0,
                route_places: vec![P::EscapeCorridor, P::StairsOrRefuge],
                area_places: vec![P::CrowdedPlace, P::OtherPlace, P::StairsOrRefuge],
            }),
            escape_route: RouteLimits {
                band_min_lx: 3.0,
                ..RouteLimits::none()
            },
            anti_panic: AreaLimits {
                min_lx: 3.0,
                ..AreaLimits::none()
            },
            high_risk: HighRiskLimits::none(),
            emphasis_min_vertical_lx: 0.0,
            emphasis_horizontal: false,
            emphasis_kinds: Vec::new(),
            exit_floor_min_lx: 0.0,
            min_luminaire_lm: 0.0,
            lux_informative: false,
            glare: GlareStep::none(),
            min_ra: 0.0,
            timing: Timing {
                half_s: 5.0,
                full_s: 5.0,
                high_risk_full_s: 0.25,
                durations: vec![
                    DurationRule::new(1.5, "buildings over 100 m high"),
                    DurationRule::new(
                        1.0,
                        "hospitals, elderly care, public buildings over 100 000 m², underground buildings over 20 000 m²",
                    ),
                    DurationRule::new(0.5, "other buildings"),
                ],
            },
            end_of_duration: None,
            notes: vec![
                "Minimum horizontal illuminance on the floor by place (not an average), no uniformity ratio.".into(),
                "Routes over the central half of their width, rooms over the whole floor.".into(),
                "Where GB 51309-2018 gives lower floor values (corridors 1 lx, stairs 5 lx) the general code governs.".into(),
            ],
        }
    }

    /// EN 1838:2024 (published December 2024, read with EN 50172:2024;
    /// EN 1838:2013 may be used until 18 June 2027): the escape route is
    /// lit to 1 lx over its width less a strip of min(W / 4, 0.5 m) along
    /// each side, at least two luminaires per route and open area, more
    /// points of emphasis. Open areas, high-risk areas, glare and Ra as in
    /// 2013 (public summaries name no change to the glare table).
    pub fn en1838_2024() -> Self {
        let mut l = Self::en1838();
        l.id = "en1838-2024".into();
        l.name = "EN 1838:2024".into();
        l.region = "Europe: EU / EFTA (DIN EN 1838:2025, BS EN 1838:2024, ÖNORM EN 1838:2025, UNI EN 1838:2025, NF EN 1838, UNE-EN 1838:2025 …)".into();
        l.clauses = "EN 1838:2024 §5.1 escape routes (§5.1.1 edge strips, §5.1.8 two luminaires), §5.2 open areas, §5.3 high-risk task areas, §4.2 points of emphasis; durations and activation times EN 50172:2024 (EN 1838 Annex A)".into();
        l.escape_route = RouteLimits {
            band_min_lx: 1.0,
            max_ratio: 40.0,
            band_share: 0.5,
            ratio_over_band: true,
            edge_max_m: 0.5,
            min_luminaires: 2,
            ..RouteLimits::none()
        };
        l.anti_panic.min_luminaires = 2;
        l.emphasis_kinds = emphasis_2024();
        l.timing.durations = vec![
            DurationRule::new(1.0, "minimum"),
            DurationRule::new(
                3.0,
                "recommended (EN 50172:2024 Table A.1) where evacuation is not immediate and for shops, exhibitions, hotels, care homes, car parks, stations, high-rise and hospitals",
            ),
        ];
        l.notes = vec![
            "Escape routes: at least 1 lx over the width less a strip of min(W / 4, 0.5 m) along each side (up to 2 m wide: the central half; wider: all but 0.5 m on each side); Emax / Emin ≤ 40, checked over that band.".into(),
            "At least two luminaires per escape route and per open area (§5.1.8).".into(),
            "Open areas larger than 60 m²: 0.5 lx without a 0.5 m border; 5 lx vertical at fire equipment, first aid, call points, manual door releases, escape plans and evacuation aids within 2 m.".into(),
            "Not checked here: toilets over 8 m² (open-area rule), accessible WCs and baby-changing (1 lx, 5 lx at the alarm), indoor pools (5 lx), plant rooms (0.5 lx, 5 lx at switchboards), every step of a stair lit directly.".into(),
            "Direct light only (the first reflection of indirect luminaires counts).".into(),
        ];
        l
    }

    /// United Kingdom: BS 5266-1:2025 (in force 31 October 2025) with BS EN
    /// 1838:2024: the EN 1838:2024 values, UK durations.
    pub fn bs5266() -> Self {
        let mut l = Self::en1838_2024();
        l.id = "bs5266".into();
        l.name = "BS 5266-1:2025".into();
        l.region = "United Kingdom".into();
        l.clauses = "BS 5266-1:2025 code of practice (in force 31 October 2025) with BS EN 1838:2024 and BS EN 50172:2024".into();
        l.timing.durations = vec![
            DurationRule::new(
                3.0,
                "sleeping accommodation, entertainment venues, phased evacuation, premises that may be re-occupied before the batteries have recharged",
            ),
            DurationRule::new(
                1.0,
                "only where the building is evacuated at once and not re-occupied until recharged",
            ),
        ];
        l.notes.insert(
            2,
            "Open areas of 60 m² or less may need anti-panic lighting by risk assessment; local area and standby lighting are in the scope (not checked).".into(),
        );
        l
    }

    /// Russia: SP 52.13330.2016 (amendments 1 and 2) §7.6, Tables 7.28 /
    /// 7.29, design rules SP 439.1325800.2018: the EN 1838:2013 shape, plus
    /// 5 lx on the floor in front of every exit; switch-over 5 s / 10 s.
    pub fn sp52() -> Self {
        let mut l = Self::en1838();
        l.id = "sp52".into();
        l.name = "SP 52.13330.2016".into();
        l.region = "Russia".into();
        l.clauses = "SP 52.13330.2016 (amendments 1 and 2) §7.6, Tables 7.28 (values) and 7.29 (glare); SP 439.1325800.2018 (design); fire safety SP 1.13130.2020 §4.3.12".into();
        l.exit_floor_min_lx = 5.0;
        l.emphasis_kinds.push(EmphasisKind::EscapePlan);
        l.timing = Timing {
            half_s: 5.0,
            full_s: 10.0,
            high_risk_full_s: 0.5,
            durations: vec![DurationRule::new(1.0, "minimum")],
        };
        l.notes = vec![
            "Escape routes up to 2 m wide: 1 lx on the floor along the axis, 0.5 lx over a band of at least half the width, Emin / Emax ≥ 1 : 40; wider routes as 2 m strips or as anti-panic areas.".into(),
            "Anti-panic lighting where an area exceeds 60 m² and is for 30 people or more, or wherever people with limited mobility or preschool children stay: 0.5 lx without a 0.5 m border.".into(),
            "5 lx on the floor in front of every exit (checked 0.5 m in front of it) and at first aid, fire equipment, call points and evacuation plans (vertical at the centre, as SP 439 asks; SP 52 says horizontal).".into(),
            "Not distinguished here: routes and stairs where people with limited mobility or preschool children are permanently present need 5 lx.".into(),
            "Direct light assumed (the documents do not say).".into(),
        ];
        l
    }

    /// Spain: CTE DB SUA 4 §2.3 (escape routes, safety equipment) with REBT
    /// ITC-BT-28 §3.1 (anti-panic, high-risk): the 2013-style route rule,
    /// 5 lx horizontal at safety equipment, anti-panic 0.5 lx in the whole
    /// space up to 1 m, no glare limit.
    pub fn cte_sua4() -> Self {
        let mut l = Self::en1838();
        l.id = "cte-sua4".into();
        l.name = "CTE DB SUA 4 / REBT ITC-BT-28".into();
        l.region = "Spain".into();
        l.clauses = "CTE DB SUA 4 §2.3 (escape routes, safety equipment, Ra; consolidated 2022); REBT ITC-BT-28 §3.1 (anti-panic, high-risk); industrial buildings RSCIEI Annex III §16".into();
        l.anti_panic = AreaLimits {
            min_lx: 0.5,
            max_ratio: 40.0,
            planes_up_to_m: 1.0,
            ..AreaLimits::none()
        };
        l.emphasis_horizontal = true;
        l.glare = GlareStep::none();
        l.notes = vec![
            "Escape routes up to 2 m wide: 1 lx on the axis, 0.5 lx over the central band of half the width, Emax / Emin ≤ 40 on the axis; wider routes as bands of 2 m.".into(),
            "Anti-panic (REBT): 0.5 lx in the whole space from the floor up to 1 m (checked on the floor and at 1 m), no size threshold, no border, Emax / Emin < 40.".into(),
            "Safety equipment, manual fire-protection equipment and distribution boards: 5 lx horizontal (here at the equipment's height).".into(),
            "Zero reflectance (direct light only); a maintenance factor is required, its value is the planner's.".into(),
        ];
        l
    }

    /// France: buildings open to the public (arrêté du 25 juin 1980, EC 7
    /// to EC 15; workplaces: arrêté du 14 décembre 2011): no lux values,
    /// rules on luminaire flux and spacing. Illuminance is reported only.
    pub fn fr_erp() -> Self {
        Limits {
            id: "fr-erp".into(),
            name: "ERP EC 7–EC 15 (arrêté du 25 juin 1980)".into(),
            region: "France (buildings open to the public; workplaces: arrêté du 14 décembre 2011)".into(),
            clauses: "Arrêté du 25 juin 1980, EC 8 / EC 9 (evacuation lighting) and EC 10 (ambient lighting); workplaces arrêté du 14 décembre 2011, articles 5 and 6".into(),
            places: None,
            escape_route: RouteLimits {
                min_luminaires: 2,
                count_above_m: 15.0,
                max_spacing_m: 15.0,
                ..RouteLimits::none()
            },
            anti_panic: AreaLimits {
                min_luminaires: 2,
                min_flux_per_m2: 5.0,
                max_spacing_per_height: 4.0,
                ..AreaLimits::none()
            },
            high_risk: HighRiskLimits::none(),
            emphasis_min_vertical_lx: 0.0,
            emphasis_horizontal: false,
            emphasis_kinds: Vec::new(),
            exit_floor_min_lx: 0.0,
            min_luminaire_lm: 45.0,
            lux_informative: true,
            glare: GlareStep::none(),
            min_ra: 0.0,
            timing: Timing {
                half_s: 0.0,
                full_s: 0.0,
                high_risk_full_s: 0.0,
                durations: vec![DurationRule::new(
                    1.0,
                    "(sleeping premises without a replacement source: extra 5 h units)",
                )],
            },
            end_of_duration: None,
            notes: vec![
                "No lux values. Evacuation lighting: luminaires at most 15 m apart along circulation routes (and from each end), at least 45 lm rated flux each, two or more on routes longer than 15 m; one at exits, obstacles and changes of direction (not checked).".into(),
                "Ambient (anti-panic) lighting in rooms for 100 people or more (50 in a basement): at least 5 lm of rated flux per m² of floor, uniformly spread, neighbours at most 4 mounting heights apart, two or more per room.".into(),
                "The flux given for the emergency luminaire is taken as its rated flux; illuminance is shown for information only.".into(),
            ],
        }
    }

    /// A profile without EN 1838's extras (no high-risk, equipment, glare
    /// or Ra rule): the start for the profiles below.
    fn plain(id: &str, name: &str, region: &str, clauses: &str) -> Self {
        Limits {
            id: id.into(),
            name: name.into(),
            region: region.into(),
            clauses: clauses.into(),
            places: None,
            escape_route: RouteLimits::none(),
            anti_panic: AreaLimits::none(),
            high_risk: HighRiskLimits::none(),
            emphasis_min_vertical_lx: 0.0,
            emphasis_horizontal: false,
            emphasis_kinds: Vec::new(),
            exit_floor_min_lx: 0.0,
            min_luminaire_lm: 0.0,
            lux_informative: false,
            glare: GlareStep::none(),
            min_ra: 0.0,
            timing: Timing {
                half_s: 0.0,
                full_s: 0.0,
                high_risk_full_s: 0.0,
                durations: Vec::new(),
            },
            end_of_duration: None,
            notes: Vec::new(),
        }
    }

    /// USA: NFPA 101 Life Safety Code §7.9.2 (= IBC §1008.3): along the
    /// path of egress at floor level an average of 1 fc (10.8 lx) and 0.1
    /// fc (1.1 lx) at any point, max / min ≤ 40; after 1.5 h at least 0.6
    /// fc (6.5 lx) average and 0.06 fc (0.65 lx) at any point. Checked over
    /// the drawn route width (the codes do not say line or width).
    pub fn nfpa101() -> Self {
        let mut l = Self::plain(
            "nfpa101",
            "NFPA 101 / IBC",
            "United States (NFPA 101 Life Safety Code; International Building Code)",
            "NFPA 101 §7.9.2.1 (1.5 h), §7.9.2.1.1 (initial values), §7.9.2.1.2 (decline), §7.9.2.1.3 (40 : 1); IBC §1008.3.4 / §1008.3.5; externally illuminated exit signs IBC §1013.6.2",
        );
        l.escape_route = RouteLimits {
            band_min_lx: 1.1,
            avg_min_lx: 10.8,
            max_ratio: 40.0,
            band_share: 1.0,
            ratio_over_band: true,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 1.1,
            avg_min_lx: 10.8,
            max_ratio: 40.0,
            ..AreaLimits::none()
        };
        // externally illuminated exit signs: 5 fc on the face
        l.emphasis_min_vertical_lx = 54.0;
        l.emphasis_kinds = vec![EmphasisKind::ExternallyLitSign];
        l.end_of_duration = Some(EndOfDuration {
            after_h: 1.5,
            avg_min_lx: 6.5,
            min_lx: 0.65,
        });
        l.timing = Timing {
            half_s: 10.0,
            full_s: 10.0,
            high_risk_full_s: 0.0,
            durations: vec![DurationRule::new(1.5, "all occupancies")],
        };
        l.notes = vec![
            "Along the path of egress at floor level: average at least 1 fc (10.8 lx), at any point at least 0.1 fc (1.1 lx), max / min ≤ 40; the codes do not say whether line or width: checked over the drawn route width (and the whole floor of a room).".into(),
            "After 1.5 h the levels may fall to an average of 0.6 fc (6.5 lx) and 0.06 fc (0.65 lx) at any point (60 %, the decline a UL 924 unit may show).".into(),
            "Externally illuminated exit signs: 5 fc (54 lx) on the face. IBC Group I-2: a single lamp failure must not leave less than 0.2 fc (2.2 lx) (not checked).".into(),
            "Reflected light counts in the field; direct light only here, which is on the safe side.".into(),
        ];
        l
    }

    /// Canada: National Building Code of Canada 2020 §3.2.7.3 / §3.2.7.4:
    /// an average of 10 lx and at least 1 lx at floor or tread level over
    /// exits, public corridors, principal routes and the listed rooms; 30
    /// min, 1 h or 2 h by building.
    pub fn nbcc() -> Self {
        let mut l = Self::plain(
            "nbcc",
            "NBC Canada 2020",
            "Canada (National Building Code; provinces adopt it with changes, e.g. Ontario OBC)",
            "NBC 2020 §3.2.7.3 (1) average 10 lx, (3) minimum 1 lx; §3.2.7.4 (1) durations; Part 9 §9.9.12.3; units CSA C22.2 No. 141",
        );
        l.escape_route = RouteLimits {
            band_min_lx: 1.0,
            avg_min_lx: 10.0,
            band_share: 1.0,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 1.0,
            avg_min_lx: 10.0,
            ..AreaLimits::none()
        };
        l.timing.durations = vec![
            DurationRule::new(2.0, "high buildings (§3.2.6)"),
            DurationRule::new(
                1.0,
                "Group B (care, treatment, detention), buildings of §3.2.2.51 / §3.2.2.60",
            ),
            DurationRule::new(0.5, "other buildings"),
        ];
        l.notes = vec![
            "Average at least 10 lx and minimum 1 lx at floor or tread level over exits, principal routes in open floor areas, public corridors, assembly areas, day care, public washrooms and the other listed spaces; no uniformity ratio.".into(),
            "Checked over the drawn route width and the whole floor of a room.".into(),
            "The units transfer automatically (no time stated); health care also CSA Z32.".into(),
        ];
        l
    }

    /// Australia: AS/NZS 2293.1:2018 + Amd 1:2021 (from public summaries;
    /// the 2026 edition is published, not yet checked): at least 0.2 lx and
    /// an average of 0.5 lx on the floor of escape routes, 1 lx on stairs.
    pub fn as2293() -> Self {
        use PlaceCategory as P;
        let mut l = Self::plain(
            "as2293",
            "AS/NZS 2293.1:2018",
            "Australia (referenced by the NCC)",
            "AS/NZS 2293.1:2018 + Amd 1:2021 (escape routes, stairs: Appendices E / F spacing tables); luminaire classification AS/NZS 2293.3",
        );
        l.places = Some(PlaceLimits {
            stairs_or_refuge_lx: 1.0,
            corridor_lx: 0.2,
            crowded_lx: 0.2,
            other_lx: 0.2,
            route_places: vec![P::EscapeCorridor, P::StairsOrRefuge],
            area_places: vec![P::OtherPlace],
        });
        l.escape_route = RouteLimits {
            band_min_lx: 0.2,
            avg_min_lx: 0.5,
            band_share: 0.5,
            edge_max_m: 0.5,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 0.2,
            avg_min_lx: 0.5,
            border_m: 0.5,
            ..AreaLimits::none()
        };
        l.timing.durations = vec![DurationRule::new(1.5, "in service (initial test 2 h)")];
        l.notes = vec![
            "Escape routes: at least 0.2 lx and an average of 0.5 lx on the floor; stairs 1 lx. Measured here over the route less a strip of min(W / 4, 0.5 m) along each wall, rooms without a 0.5 m border.".into(),
            "Values from public summaries of the 2018 edition; AS/NZS 2293.1:2026 is published and not yet checked; the 40 : 1 ratio some summaries give is not confirmed (not checked).".into(),
            "Direct light.".into(),
        ];
        l
    }

    /// New Zealand: NZBC F6 Acceptable Solution F6/AS1 (Amd 4, cites AS
    /// 2293.1-2005): direct illuminance 1 lx in exitways and at changes of
    /// level on escape routes, 0.2 lx elsewhere.
    pub fn nz_f6() -> Self {
        use PlaceCategory as P;
        let mut l = Self::plain(
            "nz-f6",
            "NZBC F6/AS1",
            "New Zealand (Building Code clause F6, Acceptable Solution F6/AS1)",
            "F6/AS1 (3rd edition, Amd 4) §1.3.1 direct illuminance, §1.3.2 identified routes; cites AS 2293.1-2005",
        );
        l.places = Some(PlaceLimits {
            stairs_or_refuge_lx: 1.0,
            corridor_lx: 1.0,
            crowded_lx: 0.2,
            other_lx: 0.2,
            route_places: vec![P::EscapeCorridor, P::StairsOrRefuge, P::OtherPlace],
            area_places: vec![P::OtherPlace, P::EscapeCorridor],
        });
        l.escape_route = RouteLimits {
            band_min_lx: 1.0,
            band_share: 0.5,
            edge_max_m: 0.5,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 0.2,
            border_m: 0.5,
            ..AreaLimits::none()
        };
        l.timing.durations = vec![DurationRule::new(1.5, "(commissioning test 2 h)")];
        l.notes = vec![
            "Direct illuminance on the floor: 1 lx in exitways and at changes of level on escape routes, 0.2 lx everywhere else; or 1 lx along identified routes (§1.3.2).".into(),
            "Points within 0.5 m of the walls are left out (consultants' practice); routes less a strip of min(W / 4, 0.5 m) along each side.".into(),
        ];
        l
    }

    /// Japan: Building Standard Law Enforcement Order Art. 126-4 / 126-5
    /// with MOC Notification No. 1830 (1970, LEDs since June 2017): direct
    /// light, at least 1 lx on the floor everywhere in the room and its
    /// escape corridors and stairs, 2 lx for fluorescent and LED sources;
    /// 30 min.
    pub fn jp_bsl() -> Self {
        let mut l = Self::plain(
            "jp-bsl",
            "Building Standard Law (Japan)",
            "Japan (emergency lighting devices of the Building Standard Law)",
            "Enforcement Order Art. 126-4 (where), Art. 126-5 (direct light, 1 lx on the floor); MOC Notification No. 1830 (1970, amended 2 June 2017 for LEDs)",
        );
        l.escape_route = RouteLimits {
            band_min_lx: 2.0,
            band_share: 1.0,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 2.0,
            ..AreaLimits::none()
        };
        l.timing.durations = vec![DurationRule::new(0.5, "on batteries, without recharging")];
        l.notes = vec![
            "Direct light only: at least 1 lx on the floor at every point of the room and of its escape corridors and stairs, corners included; 2 lx (at normal temperature) for fluorescent and LED sources, used here.".into(),
            "Underground passages of underground shopping streets 10 lx; guide lights (Fire Service Act): 1 lx on the centre line of stair treads (not checked).".into(),
            "Automatic changeover, lit at once; luminaires rated for 70 °C for 30 min.".into(),
        ];
        l
    }

    /// India: National Building Code 2016 Part 4 §3.4.7: at least 10 lx on
    /// the centre line of escape routes and 5 lx over half the width (routes
    /// up to 2 m); 1.5 h, on within 5 s.
    pub fn nbc_in() -> Self {
        let mut l = Self::plain(
            "nbc-in",
            "NBC India 2016",
            "India (National Building Code 2016, Part 4 Fire and Life Safety)",
            "NBC 2016 Part 4 §3.4.7 escape lighting and exit signage",
        );
        l.escape_route = RouteLimits {
            centre_min_lx: 10.0,
            band_min_lx: 5.0,
            band_share: 0.5,
            max_width_m: 2.0,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 5.0,
            ..AreaLimits::none()
        };
        l.timing = Timing {
            half_s: 5.0,
            full_s: 5.0,
            high_risk_full_s: 0.0,
            durations: vec![DurationRule::new(1.5, "even for the smallest premises")],
        };
        l.notes = vec![
            "Escape routes: at least 10 lx on the centre line and 5 lx over 50 % of the width (routes up to 2 m), no uniformity ratio; rooms checked here at 5 lx over the whole floor.".into(),
            "Luminaires within 2 m of exits, changes of direction and level, stairs, call points and fire equipment, at least 2 m above the floor (not checked).".into(),
            "A revision is under way (a BIS draft follows EN 1838); check the edition in force.".into(),
        ];
        l
    }

    /// Brazil: ABNT NBR 10898 (2013; reissued 2023), as the state fire
    /// brigades apply it: at least 3 lx on flat areas, 5 lx where the level
    /// changes (stairs, ramps, obstacles), max / min ≤ 20 along the route.
    pub fn nbr10898() -> Self {
        use PlaceCategory as P;
        let mut l = Self::plain(
            "nbr10898",
            "ABNT NBR 10898",
            "Brazil (enforced through the state fire brigades' technical rules, e.g. GO NT 18/2022, RS RTCBMRS 13/2025)",
            "ABNT NBR 10898:2013 (reissued 02/2023) emergency lighting systems",
        );
        l.places = Some(PlaceLimits {
            stairs_or_refuge_lx: 5.0,
            corridor_lx: 3.0,
            crowded_lx: 3.0,
            other_lx: 3.0,
            route_places: vec![P::EscapeCorridor, P::StairsOrRefuge],
            area_places: vec![P::CrowdedPlace, P::StairsOrRefuge],
        });
        l.escape_route = RouteLimits {
            band_min_lx: 3.0,
            max_ratio: 20.0,
            band_share: 0.5,
            edge_max_m: 0.5,
            ratio_over_band: true,
            ..RouteLimits::none()
        };
        l.anti_panic = AreaLimits {
            min_lx: 3.0,
            ..AreaLimits::none()
        };
        l.timing.durations = vec![DurationRule::new(
            1.0,
            "minimum, losing at most 10 % of the initial level",
        )];
        l.notes = vec![
            "Horizontal on the floor: at least 3 lx on flat areas (corridors, halls, doors, refuge areas), 5 lx where the level changes (stairs, ramps, obstacles); max / min ≤ 20 along the escape route.".into(),
            "Routes measured here over the width less a strip of min(W / 4, 0.5 m) along each side, rooms over the whole floor.".into(),
            "State rules: points at most 15 m apart and 7.5 m from walls unless a calculation shows compliance (not checked). Direct light only.".into(),
        ];
        l
    }

    /// Every profile, in the order a picker shows them (Europe, the
    /// Americas, Asia and the Pacific).
    pub fn all() -> Vec<Self> {
        vec![
            Self::en1838(),
            Self::en1838_2024(),
            Self::bs5266(),
            Self::cte_sua4(),
            Self::fr_erp(),
            Self::sp52(),
            Self::nfpa101(),
            Self::nbcc(),
            Self::nbr10898(),
            Self::gb55037(),
            Self::jp_bsl(),
            Self::nbc_in(),
            Self::as2293(),
            Self::nz_f6(),
        ]
    }

    /// Where a profile sits in a picker: "europe", "americas" or "asia".
    pub fn group(&self) -> &'static str {
        match self.id.as_str() {
            "nfpa101" | "nbcc" | "nbr10898" => "americas",
            "gb55037" | "jp-bsl" | "nbc-in" | "as2293" | "nz-f6" => "asia",
            _ => "europe",
        }
    }

    /// The profile with this id, or one of its short names
    /// (`cn`, `eu` …).
    pub fn by_id(id: &str) -> Option<Self> {
        let id = id.trim().to_lowercase();
        let id = match id.as_str() {
            "en" | "eu" | "en1838" | "en-1838" | "en1838-2013" | "en1838_2013" => "en1838",
            "en2024" | "en1838_2024" | "en1838:2024" => "en1838-2024",
            "uk" | "bs" | "bs5266-1" | "bs5266_1" => "bs5266",
            "es" | "spain" | "cte" | "cte_sua4" | "sua4" | "rebt" => "cte-sua4",
            "fr" | "france" | "erp" | "fr_erp" => "fr-erp",
            "ru" | "russia" | "sp52.13330" | "sp52_13330" => "sp52",
            "us" | "usa" | "nfpa" | "nfpa-101" | "ibc" => "nfpa101",
            "ca" | "canada" | "nbc-canada" => "nbcc",
            "au" | "australia" | "as-2293" | "as2293.1" | "asnzs2293" => "as2293",
            "nz" | "new-zealand" | "f6" | "nzbc" | "nz_f6" => "nz-f6",
            "jp" | "japan" | "bsl" | "jp_bsl" => "jp-bsl",
            "in" | "india" | "nbc_in" => "nbc-in",
            "br" | "brazil" | "brasil" | "nbr" | "nbr-10898" => "nbr10898",
            "cn" | "china" | "gb55037" | "gb-55037" => "gb55037",
            other => other,
        };
        Self::all().into_iter().find(|l| l.id == id)
    }

    /// Route limits for a place: the profile's route values, or with
    /// places, the place's floor minimum over the band.
    pub fn route_limits(&self, place: PlaceCategory) -> RouteLimits {
        let mut r = self.escape_route.clone();
        if let Some(p) = &self.places {
            r.band_min_lx = p.required(place);
        }
        r
    }

    /// Area limits for a place: the profile's area values, or with places,
    /// the place's floor minimum.
    pub fn area_limits(&self, place: PlaceCategory) -> AreaLimits {
        let mut a = self.anti_panic.clone();
        if let Some(p) = &self.places {
            a.min_lx = p.required(place);
        }
        a
    }

    /// Half the width of the checked band of a route.
    fn band_half_width(&self, width: f64) -> f64 {
        self.escape_route.band_half_width(width)
    }

    /// The profile checks vertical illuminance at points of emphasis.
    pub fn checks_emphasis(&self) -> bool {
        self.emphasis_min_vertical_lx > 0.0 && !self.emphasis_kinds.is_empty()
    }

    /// The profile asks for illuminance at this kind of point.
    pub fn checks_kind(&self, kind: EmphasisKind) -> bool {
        self.emphasis_min_vertical_lx > 0.0 && self.emphasis_kinds.contains(&kind)
    }
    /// The profile limits disability glare.
    pub fn checks_glare(&self) -> bool {
        self.glare.iter().any(|g| g.escape_cd.is_finite())
    }
    /// The profile asks for a minimum Ra.
    pub fn checks_ra(&self) -> bool {
        self.min_ra > 0.0
    }
    /// The profile has a rule for high-risk task areas.
    pub fn checks_high_risk(&self) -> bool {
        self.high_risk.abs_min_lx > 0.0 || self.high_risk.pct_of_normal > 0.0
    }

    /// Glare limit (cd) for a mounting height.
    pub fn glare_limit(&self, height: f64, high_risk: bool) -> f64 {
        let step = self
            .glare
            .iter()
            .find(|s| height < s.below_height_m)
            .or(self.glare.last());
        step.map_or(f64::INFINITY, |s| {
            if high_risk {
                s.high_risk_cd
            } else {
                s.escape_cd
            }
        })
    }

    /// What the profile states but the check does not compute (English):
    /// response or switch-over times, rated durations.
    pub fn not_calculated(&self) -> String {
        let t = &self.timing;
        let mut out = if t.full_s <= 0.0 {
            "switch-over time not stated".to_string()
        } else if t.half_s < t.full_s {
            format!(
                "response times 50 % within {} s, 100 % within {} s",
                t.half_s, t.full_s
            )
        } else {
            format!("switch-over within {} s", t.full_s)
        };
        if self.checks_high_risk() || (t.high_risk_full_s > 0.0 && t.high_risk_full_s < t.full_s) {
            out.push_str(&format!(", high-risk places {} s", t.high_risk_full_s));
        }
        if !t.durations.is_empty() {
            let d: Vec<String> = t
                .durations
                .iter()
                .map(|d| format!("{} h {}", d.hours, d.applies_to))
                .collect();
            out.push_str(&format!("; rated duration: {}", d.join(", ")));
        }
        out
    }

    /// Required minimum in a high-risk area with the given normal
    /// maintained illuminance.
    pub fn high_risk_min_lx(&self, normal_maintained_lx: f64) -> f64 {
        (normal_maintained_lx * self.high_risk.pct_of_normal / 100.0).max(self.high_risk.abs_min_lx)
    }

    /// Limits from the JSON layout of `data/en1838_en17037_limits.json`
    /// (section `EN_1838`); missing values keep the EN 1838 defaults.
    #[cfg(feature = "json")]
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let en = v.get("EN_1838").ok_or("no EN_1838 section")?;
        let mut l = Limits::en1838();
        let num = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_f64());
        if let Some(r) = en.get("escape_route") {
            l.escape_route.centre_min_lx =
                num(r, "min_illuminance_center_lx").unwrap_or(l.escape_route.centre_min_lx);
            l.escape_route.band_min_lx =
                num(r, "min_illuminance_band_lx").unwrap_or(l.escape_route.band_min_lx);
            l.escape_route.max_ratio =
                num(r, "max_uniformity_ratio").unwrap_or(l.escape_route.max_ratio);
            l.timing.half_s = num(r, "response_time_50pct_sec").unwrap_or(l.timing.half_s);
            l.timing.full_s = num(r, "response_time_100pct_sec").unwrap_or(l.timing.full_s);
            l.min_ra = num(r, "min_color_rendering_Ra").unwrap_or(l.min_ra);
        }
        if let Some(a) = en.get("anti_panic") {
            l.anti_panic.min_lx = num(a, "min_illuminance_lx").unwrap_or(l.anti_panic.min_lx);
            l.anti_panic.border_m = num(a, "edge_exclusion_m").unwrap_or(l.anti_panic.border_m);
            l.anti_panic.max_ratio =
                num(a, "max_uniformity_ratio").unwrap_or(l.anti_panic.max_ratio);
        }
        if let Some(h) = en.get("high_risk_task") {
            l.high_risk.pct_of_normal =
                num(h, "min_illuminance_pct_of_normal").unwrap_or(l.high_risk.pct_of_normal);
            l.high_risk.abs_min_lx =
                num(h, "absolute_min_illuminance_lx").unwrap_or(l.high_risk.abs_min_lx);
            l.high_risk.max_ratio = num(h, "max_uniformity_ratio").unwrap_or(l.high_risk.max_ratio);
            l.timing.high_risk_full_s =
                num(h, "response_time_100pct_sec").unwrap_or(l.timing.high_risk_full_s);
        }
        if let Some(p) = en.get("points_of_emphasis") {
            l.emphasis_min_vertical_lx =
                num(p, "min_vertical_illuminance_lx").unwrap_or(l.emphasis_min_vertical_lx);
        }
        if let Some(steps) = en.get("glare_limits_cd").and_then(|g| g.as_array()) {
            let mut glare: Vec<GlareStep> = steps
                .iter()
                .filter_map(|s| {
                    let h = num(s, "max_height_m")?;
                    Some(GlareStep {
                        // a last step with a huge height stands for "and above"
                        below_height_m: if h >= 100.0 { f64::INFINITY } else { h },
                        escape_cd: num(s, "escape_route_cd")?,
                        high_risk_cd: num(s, "high_risk_cd")?,
                    })
                })
                .collect();
            glare.sort_by(|a, b| a.below_height_m.total_cmp(&b.below_height_m));
            if !glare.is_empty() {
                l.glare = glare;
            }
        }
        Ok(l)
    }
}

/// The limit table shipped with the crate (`EN_1838` and `EN_17037`).
pub const LIMITS_JSON: &str = include_str!("../data/en1838_en17037_limits.json");

/// The Chinese profile's values and sources (GB 55037-2022 §10.1.10,
/// GB 51309-2018 durations); [`Limits::gb55037`] keeps them.
pub const LIMITS_CN_JSON: &str = include_str!("../data/cn_gb55037_limits.json");

/// The other profiles' values, sources and open points; a test keeps
/// [`Limits::all`] in step.
pub const PROFILES_JSON: &str = include_str!("../data/emergency_profiles.json");

// ─── plan ────────────────────────────────────────────────────────────

/// A luminaire in emergency mode.
#[derive(Debug, Clone)]
pub struct EmergencyLuminaire<'a> {
    pub ldt: &'a Eulumdat,
    /// Luminous flux of the luminaire in emergency mode at the end of the
    /// rated duration (lm, luminaire output).
    pub flux_lm: f64,
    pub x: f64,
    pub y: f64,
    /// Mounting height above the floor (m).
    pub height: f64,
    /// C0 direction, degrees counter-clockwise from +x.
    pub rotation: f64,
    /// Colour rendering index of the emergency light, when known.
    pub ra: Option<f64>,
}

/// Flux of a luminaire per 1000 lm of lamp flux, from its intensity table.
fn table_flux_per_klm(ldt: &Eulumdat) -> f64 {
    let cf = if ldt.conversion_factor > 0.0 {
        ldt.conversion_factor
    } else {
        1.0
    };
    PhotometricCalculations::calculated_luminous_flux(ldt) / cf
}

/// Luminous flux of the luminaire described by an LDT (lm): the flux of
/// its intensity table for its lamp flux (absolute photometry: the lamp
/// flux is the luminaire flux).
pub fn luminaire_flux_lm(ldt: &Eulumdat) -> f64 {
    table_flux_per_klm(ldt) * ldt.luminous_flux() / 1000.0
}

/// A copy of a luminaire whose lamp flux is scaled so that the luminaire
/// gives `flux_lm` (e.g. its emergency mode from a GLDF emitter).
pub fn at_flux(ldt: &Eulumdat, flux_lm: f64) -> Eulumdat {
    let mut out = ldt.clone();
    let now = luminaire_flux_lm(ldt);
    if now > 0.0 && flux_lm > 0.0 {
        let k = flux_lm / now;
        for ls in &mut out.lamp_sets {
            ls.total_luminous_flux *= k;
        }
    }
    out
}

impl EmergencyLuminaire<'_> {
    /// Factor from the LDT's cd/klm to candela at the emergency flux.
    pub fn candela_scale(&self) -> f64 {
        let per_klm = table_flux_per_klm(self.ldt);
        if per_klm > 0.0 {
            self.flux_lm / per_klm
        } else {
            self.flux_lm / 1000.0
        }
    }

    fn place(&self) -> LuminairePlace {
        let mut p = LuminairePlace::simple(0, self.x, self.y, self.height);
        p.rotation = self.rotation;
        p
    }

    /// Luminous intensity (cd) towards plan direction `azimuth` (degrees
    /// counter-clockwise from +x) at `gamma` from the downward vertical.
    pub fn intensity_cd(&self, azimuth: f64, gamma: f64) -> f64 {
        let c = (azimuth - self.rotation).rem_euclid(360.0);
        self.ldt.sample(c, gamma) * self.candela_scale()
    }
}

/// An escape route: centre line and width.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscapeRoute {
    pub name: String,
    /// Centre line (m), at least two points.
    pub path: Vec<(f64, f64)>,
    /// Width (m).
    pub width: f64,
    /// What the route is (profiles with a minimum per place).
    #[cfg_attr(feature = "serde", serde(default))]
    pub place: PlaceCategory,
}

/// An open (anti-panic) area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenArea {
    pub name: String,
    pub polygon: Vec<(f64, f64)>,
    /// What the area is (profiles with a minimum per place).
    #[cfg_attr(feature = "serde", serde(default = "crowded"))]
    pub place: PlaceCategory,
}

#[cfg(feature = "serde")]
fn crowded() -> PlaceCategory {
    PlaceCategory::CrowdedPlace
}

/// A high-risk task area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HighRiskArea {
    pub name: String,
    pub polygon: Vec<(f64, f64)>,
    /// Height of the task plane above the floor (m).
    pub task_height: f64,
    /// Normal maintained illuminance of the task (lx).
    pub normal_maintained_lx: f64,
}

/// What sits at a point of emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EmphasisKind {
    /// Fire alarm call point (vertical illuminance required).
    CallPoint,
    /// First-aid equipment (vertical illuminance required).
    FirstAid,
    /// Fire-fighting equipment (vertical illuminance required).
    FireEquipment,
    /// Externally lit escape sign (vertical illuminance required).
    ExternallyLitSign,
    /// Exit door (a luminaire nearby; reported).
    Exit,
    /// Stairs (reported).
    Stairs,
    /// Change of floor level (reported).
    LevelChange,
    /// Change of direction / intersection of routes (reported).
    Intersection,
    /// Manual release of an electronically locked door (EN 1838:2024:
    /// vertical illuminance required).
    DoorRelease,
    /// Escape and rescue plan (EN 1838:2024; vertical illuminance
    /// required).
    EscapePlan,
    /// Evacuation equipment for disabled people, e.g. an evacuation chair
    /// (EN 1838:2024; vertical illuminance required).
    EvacuationAid,
}

impl EmphasisKind {
    /// English name for reports.
    pub fn words(self) -> &'static str {
        match self {
            Self::CallPoint => "Call point",
            Self::FirstAid => "First aid",
            Self::FireEquipment => "Fire equipment",
            Self::ExternallyLitSign => "Externally lit sign",
            Self::Exit => "Exit",
            Self::Stairs => "Stairs",
            Self::LevelChange => "Level change",
            Self::Intersection => "Intersection",
            Self::DoorRelease => "Door release",
            Self::EscapePlan => "Escape plan",
            Self::EvacuationAid => "Evacuation aid",
        }
    }

    /// Kinds with a vertical illuminance requirement.
    pub fn needs_vertical_illuminance(self) -> bool {
        matches!(
            self,
            Self::CallPoint
                | Self::FirstAid
                | Self::FireEquipment
                | Self::ExternallyLitSign
                | Self::DoorRelease
                | Self::EscapePlan
                | Self::EvacuationAid
        )
    }
}

/// A point of emphasis.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmphasisPoint {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    /// Height of the equipment / sign above the floor (m); default from the settings.
    pub z: Option<f64>,
    /// Direction the equipment faces (degrees counter-clockwise from +x);
    /// without it the best of eight directions is taken and the result says so.
    pub facing: Option<f64>,
}

/// Everything EN 1838 looks at.
#[derive(Debug, Clone, Default)]
pub struct EmergencyPlan<'a> {
    pub luminaires: Vec<EmergencyLuminaire<'a>>,
    pub routes: Vec<EscapeRoute>,
    pub open_areas: Vec<OpenArea>,
    pub high_risk: Vec<HighRiskArea>,
    pub points: Vec<EmphasisPoint>,
}

/// Calculation settings (not in the limit table; stated in reports).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Settings {
    /// Maintenance factor applied to the emergency flux.
    pub maintenance_factor: f64,
    /// Point spacing along routes (m).
    pub route_step_m: f64,
    /// Grid spacing in areas (m).
    pub grid_step_m: f64,
    /// Height of points of emphasis without their own height (m).
    pub emphasis_height_m: f64,
    /// Output at the end of the rated duration as a share of the flux the
    /// luminaires are given (profiles with [`EndOfDuration`] values; 1 =
    /// constant output).
    #[cfg_attr(feature = "serde", serde(default = "one"))]
    pub end_output_share: f64,
}

#[cfg(feature = "serde")]
fn one() -> f64 {
    1.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            maintenance_factor: 0.8,
            route_step_m: 0.1,
            grid_step_m: 0.25,
            emphasis_height_m: 1.2,
            end_output_share: 1.0,
        }
    }
}

// ─── results ─────────────────────────────────────────────────────────

/// Minimum, maximum, mean and Emax / Emin of a set of points.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stats {
    pub min_lx: f64,
    pub max_lx: f64,
    pub avg_lx: f64,
    /// Emax / Emin (infinite when Emin is 0).
    pub ratio: f64,
    /// Where the minimum is (m).
    pub at_min: (f64, f64),
    pub points: usize,
}

impl Stats {
    fn of(values: &[(f64, f64, f64)]) -> Self {
        let mut s = Stats {
            min_lx: f64::INFINITY,
            max_lx: 0.0,
            avg_lx: 0.0,
            ratio: f64::INFINITY,
            at_min: (0.0, 0.0),
            points: values.len(),
        };
        if values.is_empty() {
            s.min_lx = 0.0;
            return s;
        }
        let mut sum = 0.0;
        for &(x, y, e) in values {
            if e < s.min_lx {
                s.min_lx = e;
                s.at_min = (x, y);
            }
            s.max_lx = s.max_lx.max(e);
            sum += e;
        }
        s.avg_lx = sum / values.len() as f64;
        s.ratio = if s.min_lx > 0.0 {
            s.max_lx / s.min_lx
        } else {
            f64::INFINITY
        };
        s
    }
}

/// Result for one escape route.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteResult {
    pub name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub place: PlaceCategory,
    /// The requirements the route was checked against (lx; ratio infinite
    /// = none).
    #[cfg_attr(feature = "serde", serde(default))]
    pub required_centre_lx: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required_band_lx: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_ratio: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required_avg_lx: f64,
    pub length_m: f64,
    /// Centre line.
    pub centre: Stats,
    /// The band (the profile's share of the width: five lines across it,
    /// the centre line included).
    pub band: Stats,
    pub pass_centre: bool,
    pub pass_band: bool,
    pub pass_ratio: bool,
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_avg: bool,
    /// The values allowed at the end of the rated duration.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_end: bool,
    /// Luminaire count and spacing along the route.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_layout: bool,
    /// Wider than the profile's limit (EN 1838: 2 m): split it into strips
    /// or treat it as an open area.
    pub too_wide: bool,
    /// Centre-line samples (x, y, lx) for drawing.
    pub samples: Vec<(f64, f64, f64)>,
}

#[cfg(feature = "serde")]
fn yes() -> bool {
    true
}

impl RouteResult {
    pub fn pass(&self) -> bool {
        self.pass_centre
            && self.pass_band
            && self.pass_ratio
            && self.pass_avg
            && self.pass_end
            && self.pass_layout
    }

    /// The minimum every point of the centre line must reach (lx), for
    /// marking it on a plan.
    pub fn point_min_lx(&self) -> f64 {
        self.required_centre_lx.max(self.required_band_lx)
    }
}

/// Result for an open or high-risk area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaResult {
    pub name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub place: PlaceCategory,
    pub area_m2: f64,
    pub stats: Stats,
    pub required_lx: f64,
    pub max_ratio: f64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required_avg_lx: f64,
    pub pass_min: bool,
    pub pass_ratio: bool,
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_avg: bool,
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_end: bool,
    /// Luminaire count, flux per m² and spacing in the area.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub pass_layout: bool,
    /// Anti-panic: the area is large enough to need it (EN 1838: > 60 m²).
    pub required: bool,
    /// Grid samples (x, y, lx).
    pub samples: Vec<(f64, f64, f64)>,
}

impl AreaResult {
    pub fn pass(&self) -> bool {
        self.pass_min && self.pass_ratio && self.pass_avg && self.pass_end && self.pass_layout
    }
}

/// Result for a point of emphasis.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointResult {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    /// Vertical illuminance at the point (lx), for kinds that need it
    /// (horizontal at its height for profiles that ask for that).
    pub vertical_lx: Option<f64>,
    /// Exits: horizontal illuminance on the floor 0.5 m in front (lx).
    #[cfg_attr(feature = "serde", serde(default))]
    pub floor_lx: Option<f64>,
    /// The facing was not given: best of eight directions.
    pub assumed_facing: bool,
    /// Horizontal distance to the nearest emergency luminaire (m).
    pub nearest_luminaire_m: Option<f64>,
    /// `None` for kinds without a limit.
    pub pass: Option<bool>,
}

/// Disability glare of one luminaire.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlareResult {
    pub luminaire: usize,
    /// Highest intensity between 60° and 90° in the checked directions (cd).
    pub i_max_cd: f64,
    /// Where: plan azimuth and γ (degrees).
    pub at: (f64, f64),
    pub limit_cd: f64,
    /// Checked against the high-risk column.
    pub high_risk: bool,
    /// Checked only in the directions of travel of a route.
    pub route_directions: bool,
    pub pass: bool,
}

/// What a [`Check`] measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CheckKind {
    /// A route's minimum on the centre line.
    CentreMin,
    /// A route's minimum over its band (EN 1838: the central half; other
    /// profiles: their share of the width).
    BandMin,
    /// A route's average over its band.
    RouteAverage,
    /// A route's Emax / Emin.
    RouteRatio,
    /// A route wider than the profile allows (reported).
    RouteWidth,
    /// An area's minimum, average, Emax / Emin.
    AreaMin,
    AreaAverage,
    AreaRatio,
    /// A high-risk task area's minimum and Emax / Emin.
    HighRiskMin,
    HighRiskRatio,
    /// Average / minimum at the end of the rated duration (route or area).
    EndAverage,
    EndMin,
    /// Vertical illuminance at a point of emphasis.
    Vertical,
    /// The highest intensity between 60° and 90° (all luminaires).
    Glare,
    /// Colour rendering of the emergency light (all luminaires).
    Ra,
    /// Horizontal illuminance on the floor in front of an exit.
    ExitFloor,
    /// The lowest emergency flux of a luminaire (lm).
    LuminaireFlux,
    /// Luminaires lighting a route / an area (count).
    RouteCount,
    AreaCount,
    /// Largest gap between luminaires along a route (m).
    RouteSpacing,
    /// Luminaire flux per floor area (lm/m²).
    AreaFlux,
    /// Nearest neighbour of a luminaire in an area, in mounting heights.
    AreaSpacing,
}

/// What a [`Check`] is about (indices into the plan's lists).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Subject {
    Route(usize),
    Area(usize),
    HighRisk(usize),
    Point(usize),
    Luminaires,
}

/// One row of a report: a value against the profile's limit. The rows a
/// plan gets depend on the profile, so every renderer (web table, PDF, CLI)
/// shows exactly what the chosen standard asks for.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Check {
    pub kind: CheckKind,
    pub subject: Subject,
    /// lx, a ratio, cd, Ra or m (`RouteWidth`); `None` = unknown (Ra).
    pub value: Option<f64>,
    pub limit: f64,
    /// The value must not exceed the limit (ratios, glare, width).
    pub at_most: bool,
    /// `None`: reported only (an area below the size that needs it, an
    /// unknown Ra, a rule the profile does not have).
    pub pass: Option<bool>,
    /// Where the worst value is (m), when it is a point.
    pub at: Option<(f64, f64)>,
}

impl Check {
    fn new(kind: CheckKind, subject: Subject, value: f64, limit: f64, at_most: bool) -> Self {
        let pass = if at_most {
            value <= limit
        } else {
            value >= limit
        };
        Check {
            kind,
            subject,
            value: Some(value),
            limit,
            at_most,
            pass: Some(pass),
            at: None,
        }
    }

    fn at(mut self, p: (f64, f64)) -> Self {
        self.at = Some(p);
        self
    }

    fn reported(mut self) -> Self {
        self.pass = None;
        self
    }
}

/// The check of a plan against one profile.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmergencyReport {
    pub routes: Vec<RouteResult>,
    pub open_areas: Vec<AreaResult>,
    pub high_risk: Vec<AreaResult>,
    pub points: Vec<PointResult>,
    pub glare: Vec<GlareResult>,
    /// Luminaires whose Ra is known and below the limit.
    pub ra_below_limit: Vec<usize>,
    /// Luminaires without a known Ra.
    pub ra_unknown: Vec<usize>,
    pub settings: Settings,
    /// The profile checked against ([`Limits::name`]) and its id.
    #[cfg_attr(feature = "serde", serde(default))]
    pub profile: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub profile_id: String,
    /// The rows of the report, in the profile's order: routes, areas,
    /// high-risk areas, points, glare, Ra.
    #[cfg_attr(feature = "serde", serde(default))]
    pub checks: Vec<Check>,
}

impl EmergencyReport {
    /// Every check with a limit passes (areas only where required).
    pub fn pass(&self) -> bool {
        self.checks.iter().all(|c| c.pass != Some(false))
    }

    /// At least one check has a limit (something to pass or fail).
    pub fn has_checks(&self) -> bool {
        self.checks.iter().any(|c| c.pass.is_some())
    }

    /// English text of a check for reports and the CLI: what, value,
    /// required.
    pub fn describe(&self, c: &Check, limits: &Limits) -> [String; 3] {
        use CheckKind as K;
        let lx = |v: f64| format!("{v:.2} lx");
        let ratio = |v: f64| {
            if v.is_finite() {
                format!("{v:.1}")
            } else {
                "∞".to_string()
            }
        };
        let num = |v: f64| format!("{}", (v * 1000.0).round() / 1000.0);
        let band = band_words(limits.escape_route.band_share);
        let subject = match c.subject {
            Subject::Route(i) => {
                let r = self.routes.get(i);
                let name = r.map_or(String::new(), |r| r.name.clone());
                match (r, &limits.places) {
                    (Some(r), Some(_)) => format!("Route {name} ({})", r.place.words()),
                    _ => format!("Route {name}"),
                }
            }
            Subject::Area(i) => {
                let a = self.open_areas.get(i);
                match (a, &limits.places) {
                    (Some(a), Some(_)) => {
                        format!("Room ({}, {:.1} m²)", a.place.words(), a.area_m2)
                    }
                    (Some(a), None) if limits.anti_panic.applies_above_m2 > 0.0 => {
                        format!("Open area ({:.1} m²)", a.area_m2)
                    }
                    (Some(a), None) => format!("Room ({:.1} m²)", a.area_m2),
                    _ => "Area".to_string(),
                }
            }
            Subject::HighRisk(i) => format!(
                "High-risk area {}",
                self.high_risk.get(i).map_or("", |a| a.name.as_str())
            ),
            Subject::Point(i) => self.points.get(i).map_or(String::new(), |p| {
                format!("{} at ({:.1}, {:.1})", p.kind.words(), p.x, p.y)
            }),
            Subject::Luminaires => "Luminaires".to_string(),
        };
        let after = limits.end_of_duration.as_ref().map_or(0.0, |e| e.after_h);
        let share = self.settings.end_output_share;
        let what = match c.kind {
            K::CentreMin => format!("{subject}: centre line min"),
            K::BandMin => format!("{subject}: floor min, {band}"),
            K::RouteAverage => format!("{subject}: average, {band}"),
            K::RouteRatio if limits.escape_route.ratio_over_band => {
                format!("{subject}: Emax / Emin, {band}")
            }
            K::RouteRatio => format!("{subject}: Emax / Emin, centre line"),
            K::RouteWidth => format!("{subject}: width"),
            K::AreaMin => format!("{subject}: floor min"),
            K::AreaAverage => format!("{subject}: average"),
            K::AreaRatio | K::HighRiskRatio => format!("{subject}: Emax / Emin"),
            K::HighRiskMin => format!("{subject}: min on the task plane"),
            K::EndAverage => format!(
                "{subject}: average after {} h (output {:.0} %)",
                num(after),
                share * 100.0
            ),
            K::EndMin => format!(
                "{subject}: min after {} h (output {:.0} %)",
                num(after),
                share * 100.0
            ),
            K::Vertical if limits.emphasis_horizontal => {
                format!("{subject}: horizontal at the equipment")
            }
            K::Vertical => format!("{subject}: vertical"),
            K::Glare => "Glare, highest intensity 60°–90°".to_string(),
            K::Ra => "Colour rendering Ra".to_string(),
            K::ExitFloor => format!("{subject}: floor 0.5 m in front"),
            K::LuminaireFlux => "Emergency flux of each luminaire (lowest)".to_string(),
            K::RouteCount | K::AreaCount => format!("{subject}: luminaires"),
            K::RouteSpacing => format!("{subject}: largest gap between luminaires"),
            K::AreaFlux => format!("{subject}: luminaire flux per m²"),
            K::AreaSpacing => format!("{subject}: nearest luminaire / mounting height (largest)"),
        };
        let value = match (c.kind, c.value) {
            (_, None) => "unknown".to_string(),
            (K::RouteRatio | K::AreaRatio | K::HighRiskRatio, Some(v)) => ratio(v),
            (K::Glare, Some(v)) => format!("{v:.1} cd"),
            (K::Ra, Some(v)) => format!("{v:.0}"),
            (K::RouteWidth | K::RouteSpacing, Some(v)) => format!("{v:.1} m"),
            (K::RouteCount | K::AreaCount, Some(v)) => format!("{v:.0}"),
            (K::LuminaireFlux, Some(v)) => format!("{v:.0} lm"),
            (K::AreaFlux, Some(v)) => format!("{v:.1} lm/m²"),
            (K::AreaSpacing, Some(v)) => format!("{v:.1}"),
            (_, Some(v)) => lx(v),
        };
        let required = match c.kind {
            _ if c.pass.is_none() && c.limit == 0.0 && c.kind != K::Vertical => {
                "for information".to_string()
            }
            K::AreaMin if c.pass.is_none() => format!(
                "not required (≤ {} m²)",
                num(limits.anti_panic.applies_above_m2)
            ),
            K::Vertical if c.pass.is_none() => "no requirement".to_string(),
            K::RouteWidth => format!(
                "≤ {} m: split into strips or check as an open area",
                num(c.limit)
            ),
            K::RouteRatio | K::AreaRatio | K::HighRiskRatio => format!("≤ {}", num(c.limit)),
            K::Glare => format!("≤ {:.0} cd", c.limit),
            K::Ra | K::RouteCount | K::AreaCount => format!("≥ {}", num(c.limit)),
            K::RouteSpacing => format!("≤ {} m", num(c.limit)),
            K::LuminaireFlux => format!("≥ {} lm", num(c.limit)),
            K::AreaFlux => format!("≥ {} lm/m²", num(c.limit)),
            K::AreaSpacing => format!("≤ {}", num(c.limit)),
            _ => format!("≥ {} lx", num(c.limit)),
        };
        [what, value, required]
    }
}

/// How a band of `share` of the route width is called in reports.
pub fn band_words(share: f64) -> String {
    if (share - 0.5).abs() < 1e-9 {
        "central half".to_string()
    } else if share >= 1.0 - 1e-9 {
        "full width".to_string()
    } else if share <= 1e-9 {
        "centre line".to_string()
    } else {
        format!("central {:.0} %", share * 100.0)
    }
}

// ─── evaluation ──────────────────────────────────────────────────────

/// A luminaire ready for many illuminance calls (placement and candela
/// scale computed once: the scale integrates the intensity table).
struct Prepared<'a> {
    ldt: &'a Eulumdat,
    place: LuminairePlace,
    scale: f64,
}

fn prepare<'a>(luminaires: &[EmergencyLuminaire<'a>]) -> Vec<Prepared<'a>> {
    let mut cache: Vec<(*const Eulumdat, f64)> = Vec::new();
    luminaires
        .iter()
        .map(|l| {
            let key = l.ldt as *const Eulumdat;
            let per_klm = match cache.iter().find(|(k, _)| *k == key) {
                Some((_, v)) => *v,
                None => {
                    let v = table_flux_per_klm(l.ldt);
                    cache.push((key, v));
                    v
                }
            };
            Prepared {
                ldt: l.ldt,
                place: l.place(),
                scale: if per_klm > 0.0 {
                    l.flux_lm / per_klm
                } else {
                    l.flux_lm / 1000.0
                },
            }
        })
        .collect()
}

fn prepared_illuminance(
    lums: &[Prepared],
    point: (f64, f64, f64),
    normal: (f64, f64, f64),
    maintenance_factor: f64,
) -> f64 {
    lums.iter()
        .map(|l| compute_illuminance_at_point(l.ldt, &l.place, point, normal, l.scale))
        .sum::<f64>()
        * maintenance_factor
}

/// Illuminance (lx) at `point` on a surface with `normal`, from all
/// luminaires, maintenance factor included.
pub fn illuminance(
    luminaires: &[EmergencyLuminaire],
    point: (f64, f64, f64),
    normal: (f64, f64, f64),
    maintenance_factor: f64,
) -> f64 {
    prepared_illuminance(&prepare(luminaires), point, normal, maintenance_factor)
}

const UP: (f64, f64, f64) = (0.0, 0.0, 1.0);

/// Points along a polyline every `step` (first and last point included),
/// shifted sideways by `offset` (left of the direction of travel).
fn walk(path: &[(f64, f64)], step: f64, offset: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for w in path.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = ((x1 - x0) / len, (y1 - y0) / len);
        let (nx, ny) = (-uy * offset, ux * offset);
        let n = (len / step.max(1e-3)).ceil().max(1.0) as usize;
        let start = if out.is_empty() { 0 } else { 1 };
        for k in start..=n {
            let t = len * k as f64 / n as f64;
            out.push((x0 + ux * t + nx, y0 + uy * t + ny));
        }
    }
    out
}

fn path_length(path: &[(f64, f64)]) -> f64 {
    path.windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .sum()
}

/// Distance from a point to a segment, and the segment's direction (deg).
fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
    (
        ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt(),
        dy.atan2(dx).to_degrees(),
    )
}

/// Distance from a point to the polygon's outline.
fn edge_distance(p: (f64, f64), poly: &[(f64, f64)]) -> f64 {
    (0..poly.len())
        .map(|i| segment_distance(p, poly[i], poly[(i + 1) % poly.len()]).0)
        .fold(f64::INFINITY, f64::min)
}

/// Grid points inside a polygon, at least `border` from its outline.
fn area_points(poly: &[(f64, f64)], step: f64, border: f64) -> Vec<(f64, f64)> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let area = AreaPolygon::new(poly.to_vec());
    let (x0, y0, x1, y1) = area.bounding_box();
    let step = step.max(0.01);
    let nx = ((x1 - x0) / step).floor() as usize;
    let ny = ((y1 - y0) / step).floor() as usize;
    let mut out = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let p = (x0 + (i as f64 + 0.5) * step, y0 + (j as f64 + 0.5) * step);
            if p.0 > x1 || p.1 > y1 {
                continue;
            }
            if area.contains(p.0, p.1) && edge_distance(p, poly) >= border {
                out.push(p);
            }
        }
    }
    out
}

/// Horizontal illuminance on the floor in emergency mode, for a false-colour
/// plan: cell centres of a `step` grid over the outline's bounding box, row
/// by row from the lowest y; cells whose centre is outside are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorGrid {
    pub x0: f64,
    pub y0: f64,
    pub step: f64,
    pub nx: usize,
    pub ny: usize,
    pub values: Vec<Option<f64>>,
}

impl FloorGrid {
    /// Value of cell (`i`, `j`).
    pub fn at(&self, i: usize, j: usize) -> Option<f64> {
        self.values.get(j * self.nx + i).copied().flatten()
    }

    /// Lowest and highest value inside the outline.
    pub fn min_max(&self) -> Option<(f64, f64)> {
        self.values
            .iter()
            .flatten()
            .fold(None, |acc, &v| match acc {
                None => Some((v, v)),
                Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
            })
    }
}

/// [`FloorGrid`] of `luminaires` over `outline` (maintenance factor
/// included, direct light as EN 1838 counts it). At most 200 × 200 cells:
/// a coarser step is taken for large rooms.
pub fn floor_grid(
    luminaires: &[EmergencyLuminaire],
    outline: &[(f64, f64)],
    step: f64,
    maintenance_factor: f64,
) -> FloorGrid {
    let empty = FloorGrid {
        x0: 0.0,
        y0: 0.0,
        step,
        nx: 0,
        ny: 0,
        values: Vec::new(),
    };
    if outline.len() < 3 {
        return empty;
    }
    let area = AreaPolygon::new(outline.to_vec());
    let (x0, y0, x1, y1) = area.bounding_box();
    let step = step.max((x1 - x0).max(y1 - y0) / 200.0).max(0.01);
    let nx = ((x1 - x0) / step).ceil().max(1.0) as usize;
    let ny = ((y1 - y0) / step).ceil().max(1.0) as usize;
    let lums = prepare(luminaires);
    let mut values = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let (x, y) = (x0 + (i as f64 + 0.5) * step, y0 + (j as f64 + 0.5) * step);
            values.push(
                area.contains(x, y)
                    .then(|| prepared_illuminance(&lums, (x, y, 0.0), UP, maintenance_factor)),
            );
        }
    }
    FloorGrid {
        x0,
        y0,
        step,
        nx,
        ny,
        values,
    }
}

fn polygon_area(poly: &[(f64, f64)]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    AreaPolygon::new(poly.to_vec()).area()
}

/// Highest intensity between 60° and 90° (1° steps) over the given plan
/// azimuths.
fn glare_max(l: &Prepared, azimuths: &[f64]) -> (f64, (f64, f64)) {
    let mut best = (0.0, (0.0, 60.0));
    for &az in azimuths {
        let c = (az - l.place.rotation).rem_euclid(360.0);
        for g in 60..=90 {
            let i = l.ldt.sample(c, g as f64) * l.scale;
            if i > best.0 {
                best = (i, (az.rem_euclid(360.0), g as f64));
            }
        }
    }
    best
}

/// The checks of one route, from its centre-line and band statistics (in
/// report order). `share` = output at the end of the rated duration.
fn route_checks(
    rl: &RouteLimits,
    end: Option<&EndOfDuration>,
    share: f64,
    c: &Stats,
    b: &Stats,
    width: f64,
    subject: Subject,
) -> Vec<Check> {
    use CheckKind as K;
    let mut out = Vec::new();
    if rl.centre_min_lx > 0.0 {
        out.push(Check::new(K::CentreMin, subject, c.min_lx, rl.centre_min_lx, false).at(c.at_min));
    }
    if rl.band_min_lx > 0.0 {
        out.push(Check::new(K::BandMin, subject, b.min_lx, rl.band_min_lx, false).at(b.at_min));
    }
    if rl.avg_min_lx > 0.0 {
        out.push(Check::new(
            K::RouteAverage,
            subject,
            b.avg_lx,
            rl.avg_min_lx,
            false,
        ));
    }
    if rl.max_ratio.is_finite() {
        let r = if rl.ratio_over_band { b.ratio } else { c.ratio };
        out.push(Check::new(K::RouteRatio, subject, r, rl.max_ratio, true));
    }
    if let Some(e) = end {
        // the minimum where the profile measures it: the band, else the
        // centre line
        let m = if rl.band_min_lx > 0.0 { b } else { c };
        if e.avg_min_lx > 0.0 {
            out.push(Check::new(
                K::EndAverage,
                subject,
                b.avg_lx * share,
                e.avg_min_lx,
                false,
            ));
        }
        if e.min_lx > 0.0 {
            out.push(
                Check::new(K::EndMin, subject, m.min_lx * share, e.min_lx, false).at(m.at_min),
            );
        }
    }
    if width > rl.max_width_m + 1e-9 {
        out.push(Check::new(K::RouteWidth, subject, width, rl.max_width_m, true).reported());
    }
    out
}

/// The checks of one area (`required`: large enough to need the rule).
fn area_checks(
    al: &AreaLimits,
    end: Option<&EndOfDuration>,
    share: f64,
    st: &Stats,
    required: bool,
    subject: Subject,
) -> Vec<Check> {
    use CheckKind as K;
    let mut out = Vec::new();
    let min = if st.points > 0 { st.min_lx } else { 0.0 };
    if al.min_lx > 0.0 {
        let c = Check::new(K::AreaMin, subject, min, al.min_lx, false).at(st.at_min);
        out.push(if required { c } else { c.reported() });
    }
    if !required {
        return out;
    }
    if al.avg_min_lx > 0.0 {
        out.push(Check::new(
            K::AreaAverage,
            subject,
            st.avg_lx,
            al.avg_min_lx,
            false,
        ));
    }
    if al.max_ratio.is_finite() {
        out.push(Check::new(
            K::AreaRatio,
            subject,
            st.ratio,
            al.max_ratio,
            true,
        ));
    }
    if let Some(e) = end {
        if e.avg_min_lx > 0.0 {
            out.push(Check::new(
                K::EndAverage,
                subject,
                st.avg_lx * share,
                e.avg_min_lx,
                false,
            ));
        }
        if e.min_lx > 0.0 {
            out.push(Check::new(K::EndMin, subject, min * share, e.min_lx, false).at(st.at_min));
        }
    }
    out
}

/// Distance of `p` from a polyline and its arc length along it (m).
fn project(path: &[(f64, f64)], p: (f64, f64)) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    let mut start = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy);
        if d < best.0 {
            best = (d, start + t * len2.sqrt());
        }
        start += len2.sqrt();
    }
    best
}

/// Luminaire count and spacing along a route (profiles with such rules).
fn route_layout_checks(
    rl: &RouteLimits,
    lums: &[EmergencyLuminaire],
    r: &EscapeRoute,
    subject: Subject,
) -> Vec<Check> {
    use CheckKind as K;
    let mut out = Vec::new();
    let len = path_length(&r.path);
    let mut arcs: Vec<f64> = lums
        .iter()
        .map(|l| project(&r.path, (l.x, l.y)))
        .filter(|(d, _)| *d <= r.width / 2.0 + 0.5)
        .map(|(_, a)| a)
        .collect();
    arcs.sort_by(f64::total_cmp);
    if rl.min_luminaires > 0 && len > rl.count_above_m {
        out.push(Check::new(
            K::RouteCount,
            subject,
            arcs.len() as f64,
            rl.min_luminaires as f64,
            false,
        ));
    }
    if rl.max_spacing_m.is_finite() {
        // the gaps between neighbours, and from each end to the nearest
        let gap = match (arcs.first(), arcs.last()) {
            (Some(&a), Some(&b)) => arcs
                .windows(2)
                .map(|w| w[1] - w[0])
                .fold(a.max(len - b), f64::max),
            _ => len,
        };
        out.push(Check::new(
            K::RouteSpacing,
            subject,
            gap,
            rl.max_spacing_m,
            true,
        ));
    }
    out
}

/// Luminaire count, flux per m² and spacing in an area.
fn area_layout_checks(
    al: &AreaLimits,
    lums: &[EmergencyLuminaire],
    polygon: &[(f64, f64)],
    area_m2: f64,
    subject: Subject,
) -> Vec<Check> {
    use CheckKind as K;
    let mut out = Vec::new();
    if polygon.len() < 3 {
        return out;
    }
    let poly = AreaPolygon::new(polygon.to_vec());
    let inside: Vec<&EmergencyLuminaire> =
        lums.iter().filter(|l| poly.contains(l.x, l.y)).collect();
    if al.min_luminaires > 0 {
        out.push(Check::new(
            K::AreaCount,
            subject,
            inside.len() as f64,
            al.min_luminaires as f64,
            false,
        ));
    }
    if al.min_flux_per_m2 > 0.0 && area_m2 > 0.0 {
        let flux = inside.iter().fold(0.0, |a, l| a + l.flux_lm);
        out.push(Check::new(
            K::AreaFlux,
            subject,
            flux / area_m2,
            al.min_flux_per_m2,
            false,
        ));
    }
    if al.max_spacing_per_height.is_finite() && inside.len() >= 2 {
        // each luminaire's nearest neighbour, in its mounting heights
        let worst = inside
            .iter()
            .map(|l| {
                let d = lums
                    .iter()
                    .filter(|o| !std::ptr::eq(*o, *l))
                    .map(|o| (o.x - l.x).hypot(o.y - l.y))
                    .fold(f64::INFINITY, f64::min);
                d / l.height.max(0.1)
            })
            .fold(0.0, f64::max);
        out.push(Check::new(
            K::AreaSpacing,
            subject,
            worst,
            al.max_spacing_per_height,
            true,
        ));
    }
    out
}

/// All checks of `kind` pass (true when there are none).
fn all_pass(checks: &[Check], kind: CheckKind) -> bool {
    checks
        .iter()
        .filter(|c| c.kind == kind)
        .all(|c| c.pass != Some(false))
}

/// Points with their illuminance (x, y, lx).
type Samples = Vec<(f64, f64, f64)>;

/// Centre line and band samples of a route: the band is five lines across
/// the profile's share of the width (`half` to each side), the centre line
/// included.
fn route_samples(
    prepared: &[Prepared],
    path: &[(f64, f64)],
    step: f64,
    half: f64,
    mf: f64,
) -> (Samples, Samples) {
    let e = |p: (f64, f64)| {
        (
            p.0,
            p.1,
            prepared_illuminance(prepared, (p.0, p.1, 0.0), UP, mf),
        )
    };
    let centre: Vec<(f64, f64, f64)> = walk(path, step, 0.0).into_iter().map(e).collect();
    let band: Vec<(f64, f64, f64)> = [half, -half, half / 2.0, -half / 2.0]
        .iter()
        .filter(|o| o.abs() > 1e-9)
        .flat_map(|&o| walk(path, step, o))
        .map(e)
        .chain(centre.iter().copied())
        .collect();
    (centre, band)
}

/// Check a plan against a profile ([`Limits`]): the rows of
/// [`EmergencyReport::checks`] are the ones that profile asks for.
pub fn evaluate(plan: &EmergencyPlan, limits: &Limits, settings: &Settings) -> EmergencyReport {
    use CheckKind as K;
    let lums = &plan.luminaires;
    let prepared = prepare(lums);
    let mf = settings.maintenance_factor;
    let end = limits.end_of_duration.as_ref();
    let share = settings.end_output_share;
    let mut checks: Vec<Check> = Vec::new();

    let routes = plan
        .routes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let q = limits.band_half_width(r.width);
            let (centre, band) = route_samples(&prepared, &r.path, settings.route_step_m, q, mf);
            let (c, b) = (Stats::of(&centre), Stats::of(&band));
            let rl = limits.route_limits(r.place);
            let mut rc = route_checks(&rl, end, share, &c, &b, r.width, Subject::Route(i));
            if limits.lux_informative {
                rc.push(Check::new(K::BandMin, Subject::Route(i), b.min_lx, 0.0, false).reported());
            }
            let layout = route_layout_checks(&rl, lums, r, Subject::Route(i));
            let pass_layout = layout.iter().all(|c| c.pass != Some(false));
            rc.extend(layout);
            let result = RouteResult {
                name: r.name.clone(),
                place: r.place,
                required_centre_lx: rl.centre_min_lx,
                required_band_lx: rl.band_min_lx,
                max_ratio: rl.max_ratio,
                required_avg_lx: rl.avg_min_lx,
                length_m: path_length(&r.path),
                pass_centre: all_pass(&rc, K::CentreMin),
                pass_band: all_pass(&rc, K::BandMin),
                pass_ratio: all_pass(&rc, K::RouteRatio),
                pass_avg: all_pass(&rc, K::RouteAverage),
                pass_end: all_pass(&rc, K::EndAverage) && all_pass(&rc, K::EndMin),
                pass_layout,
                too_wide: r.width > rl.max_width_m + 1e-9,
                centre: c,
                band: b,
                samples: centre,
            };
            checks.extend(rc);
            result
        })
        .collect();

    let open_areas = plan
        .open_areas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let al = limits.area_limits(a.place);
            let grid = area_points(&a.polygon, settings.grid_step_m, al.border_m);
            // the floor, and the top of the space the rule covers (Spain: 1 m)
            let planes: &[f64] = if al.planes_up_to_m > 0.0 {
                &[0.0, al.planes_up_to_m]
            } else {
                &[0.0]
            };
            let samples: Vec<(f64, f64, f64)> = planes
                .iter()
                .flat_map(|&z| grid.iter().map(move |p| (p.0, p.1, z)))
                .map(|(x, y, z)| (x, y, prepared_illuminance(&prepared, (x, y, z), UP, mf)))
                .collect();
            let stats = Stats::of(&samples);
            let area_m2 = polygon_area(&a.polygon);
            let required = area_m2 > al.applies_above_m2;
            let mut ac = area_checks(&al, end, share, &stats, required, Subject::Area(i));
            if limits.lux_informative {
                let min = if stats.points > 0 { stats.min_lx } else { 0.0 };
                ac.push(Check::new(K::AreaMin, Subject::Area(i), min, 0.0, false).reported());
            }
            let layout = if required {
                area_layout_checks(&al, lums, &a.polygon, area_m2, Subject::Area(i))
            } else {
                Vec::new()
            };
            let pass_layout = layout.iter().all(|c| c.pass != Some(false));
            ac.extend(layout);
            let result = AreaResult {
                name: a.name.clone(),
                place: a.place,
                area_m2,
                pass_min: stats.points > 0 && stats.min_lx >= al.min_lx,
                pass_ratio: stats.ratio <= al.max_ratio,
                pass_avg: stats.avg_lx >= al.avg_min_lx,
                pass_end: !required || (all_pass(&ac, K::EndAverage) && all_pass(&ac, K::EndMin)),
                pass_layout,
                required,
                required_lx: al.min_lx,
                max_ratio: al.max_ratio,
                required_avg_lx: al.avg_min_lx,
                stats,
                samples,
            };
            checks.extend(ac);
            result
        })
        .collect();

    let high_risk = plan
        .high_risk
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let samples: Vec<(f64, f64, f64)> = area_points(&a.polygon, settings.grid_step_m, 0.0)
                .into_iter()
                .map(|p| {
                    (
                        p.0,
                        p.1,
                        prepared_illuminance(&prepared, (p.0, p.1, a.task_height), UP, mf),
                    )
                })
                .collect();
            let stats = Stats::of(&samples);
            let required = limits.high_risk_min_lx(a.normal_maintained_lx);
            let applies = limits.checks_high_risk();
            let min = if stats.points > 0 { stats.min_lx } else { 0.0 };
            let s = Subject::HighRisk(i);
            if applies {
                checks.push(Check::new(K::HighRiskMin, s, min, required, false).at(stats.at_min));
                if limits.high_risk.max_ratio.is_finite() {
                    checks.push(Check::new(
                        K::HighRiskRatio,
                        s,
                        stats.ratio,
                        limits.high_risk.max_ratio,
                        true,
                    ));
                }
            }
            AreaResult {
                name: a.name.clone(),
                place: PlaceCategory::OtherPlace,
                area_m2: polygon_area(&a.polygon),
                pass_min: !applies || (stats.points > 0 && stats.min_lx >= required),
                pass_ratio: !applies || stats.ratio <= limits.high_risk.max_ratio,
                pass_avg: true,
                pass_end: true,
                pass_layout: true,
                required: applies,
                required_lx: required,
                max_ratio: limits.high_risk.max_ratio,
                required_avg_lx: 0.0,
                stats,
                samples,
            }
        })
        .collect();

    let points: Vec<PointResult> = plan
        .points
        .iter()
        .map(|p| {
            let z = p.z.unwrap_or(settings.emphasis_height_m);
            let nearest = lums
                .iter()
                .map(|l| ((l.x - p.x).powi(2) + (l.y - p.y).powi(2)).sqrt())
                .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))));
            let vertical = p.kind.needs_vertical_illuminance().then(|| {
                if limits.emphasis_horizontal {
                    return prepared_illuminance(&prepared, (p.x, p.y, z), UP, mf);
                }
                let e_dir = |deg: f64| {
                    let a = deg.to_radians();
                    prepared_illuminance(&prepared, (p.x, p.y, z), (a.cos(), a.sin(), 0.0), mf)
                };
                match p.facing {
                    Some(f) => e_dir(f),
                    None => (0..8).map(|k| e_dir(k as f64 * 45.0)).fold(0.0, f64::max),
                }
            });
            // the floor in front of an exit (into the room)
            let floor_lx =
                (p.kind == EmphasisKind::Exit && limits.exit_floor_min_lx > 0.0).then(|| {
                    let (fx, fy) = p.facing.map_or((0.0, 0.0), |f| {
                        (f.to_radians().cos() * 0.5, f.to_radians().sin() * 0.5)
                    });
                    prepared_illuminance(&prepared, (p.x + fx, p.y + fy, 0.0), UP, mf)
                });
            PointResult {
                kind: p.kind,
                x: p.x,
                y: p.y,
                floor_lx,
                pass: vertical
                    .filter(|_| limits.checks_kind(p.kind))
                    .map(|e| e >= limits.emphasis_min_vertical_lx),
                vertical_lx: vertical,
                assumed_facing: vertical.is_some() && p.facing.is_none(),
                nearest_luminaire_m: nearest,
            }
        })
        .collect();
    for (i, p) in points.iter().enumerate() {
        if let Some(e) = p.vertical_lx {
            let c = Check::new(
                K::Vertical,
                Subject::Point(i),
                e,
                limits.emphasis_min_vertical_lx,
                false,
            )
            .at((p.x, p.y));
            checks.push(if limits.checks_kind(p.kind) {
                c
            } else {
                c.reported()
            });
        }
        if let Some(e) = p.floor_lx {
            checks.push(
                Check::new(
                    K::ExitFloor,
                    Subject::Point(i),
                    e,
                    limits.exit_floor_min_lx,
                    false,
                )
                .at((p.x, p.y)),
            );
        }
    }
    if limits.min_luminaire_lm > 0.0 && !lums.is_empty() {
        let lowest = lums.iter().map(|l| l.flux_lm).fold(f64::INFINITY, f64::min);
        checks.push(Check::new(
            K::LuminaireFlux,
            Subject::Luminaires,
            lowest,
            limits.min_luminaire_lm,
            false,
        ));
    }

    let glare: Vec<GlareResult> = lums
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let high = plan.high_risk.iter().any(|a| {
                a.polygon.len() >= 3 && AreaPolygon::new(a.polygon.clone()).contains(l.x, l.y)
            });
            // a luminaire over a route: the directions of travel there
            let on_route = plan
                .routes
                .iter()
                .flat_map(|r| {
                    r.path
                        .windows(2)
                        .map(move |w| (segment_distance((l.x, l.y), w[0], w[1]), r.width))
                })
                .filter(|((d, _), w)| *d <= w / 2.0 + 0.5)
                .min_by(|a, b| a.0 .0.total_cmp(&b.0 .0))
                .map(|((_, dir), _)| dir);
            let azimuths: Vec<f64> = match (high, on_route) {
                (false, Some(dir)) => vec![dir, dir + 180.0],
                _ => (0..360).map(|a| a as f64).collect(),
            };
            let (i_max, at) = glare_max(&prepared[i], &azimuths);
            let limit = if limits.checks_glare() {
                limits.glare_limit(l.height, high)
            } else {
                f64::INFINITY
            };
            GlareResult {
                luminaire: i,
                i_max_cd: i_max,
                at,
                limit_cd: limit,
                high_risk: high,
                route_directions: !high && on_route.is_some(),
                pass: i_max <= limit,
            }
        })
        .collect();
    if limits.checks_glare() {
        if let Some(g) = glare
            .iter()
            .max_by(|a, b| (a.i_max_cd / a.limit_cd).total_cmp(&(b.i_max_cd / b.limit_cd)))
        {
            let mut c = Check::new(K::Glare, Subject::Luminaires, g.i_max_cd, g.limit_cd, true);
            c.pass = Some(glare.iter().all(|g| g.pass));
            checks.push(c);
        }
    }

    let ra_below_limit: Vec<usize> = lums
        .iter()
        .enumerate()
        .filter(|(_, l)| limits.checks_ra() && l.ra.is_some_and(|ra| ra < limits.min_ra))
        .map(|(i, _)| i)
        .collect();
    let ra_unknown: Vec<usize> = lums
        .iter()
        .enumerate()
        .filter(|(_, l)| limits.checks_ra() && l.ra.is_none())
        .map(|(i, _)| i)
        .collect();
    if limits.checks_ra() && !lums.is_empty() {
        let lowest = lums
            .iter()
            .filter_map(|l| l.ra)
            .fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.min(r))));
        let mut c = Check::new(K::Ra, Subject::Luminaires, 0.0, limits.min_ra, false);
        (c.value, c.pass) = if !ra_below_limit.is_empty() {
            (lowest, Some(false))
        } else if !ra_unknown.is_empty() {
            (None, None)
        } else {
            (lowest, Some(true))
        };
        checks.push(c);
    }

    EmergencyReport {
        routes,
        open_areas,
        high_risk,
        points,
        glare,
        ra_below_limit,
        ra_unknown,
        settings: settings.clone(),
        profile: limits.name.clone(),
        profile_id: limits.id.clone(),
        checks,
    }
}

// ─── spacing table ───────────────────────────────────────────────────

/// What one emergency luminaire achieves at one mounting height: the
/// spacing tables manufacturers publish, from the limit values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpacingRow {
    pub height_m: f64,
    /// Illuminance below the luminaire, alone (lx).
    pub e_nadir_lx: f64,
    /// Largest spacing in a row along an escape route of `route_width_m`,
    /// row along C0 / C90 (m; 0 when even close spacing fails).
    pub route_spacing_c0_m: f64,
    pub route_spacing_c90_m: f64,
    pub route_width_m: f64,
    /// Largest square grid for an open area (m).
    pub open_area_grid_m: f64,
    /// Highest intensity between 60° and 90°, all directions (cd).
    pub glare_i_max_cd: f64,
    pub glare_limit_cd: f64,
}

/// Spacing table of a luminaire at its emergency flux.
pub fn spacing_table(
    ldt: &Eulumdat,
    flux_lm: f64,
    heights: &[f64],
    route_width_m: f64,
    limits: &Limits,
    settings: &Settings,
) -> Vec<SpacingRow> {
    heights
        .iter()
        .map(|&h| {
            let lum = |x: f64, y: f64| EmergencyLuminaire {
                ldt,
                flux_lm,
                x,
                y,
                height: h,
                rotation: 0.0,
                ra: None,
            };
            let mf = settings.maintenance_factor;
            let end = limits.end_of_duration.as_ref();
            let share = settings.end_output_share;
            // the usual places: an evacuation corridor and a crowded area
            let rl = limits.route_limits(PlaceCategory::EscapeCorridor);
            let al = limits.area_limits(PlaceCategory::CrowdedPlace);
            // a row of luminaires along x (C0) or y (C90), spacing s
            let route_ok = |s: f64, along_y: bool| -> bool {
                let row = prepare(
                    &(-6..=6)
                        .map(|k| {
                            let t = k as f64 * s;
                            if along_y {
                                lum(0.0, t)
                            } else {
                                lum(t, 0.0)
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                let q = limits.band_half_width(route_width_m);
                let (mut centre, mut band) = (Vec::new(), Vec::new());
                for k in 0..=40 {
                    let t = s * k as f64 / 40.0;
                    for off in [0.0, q, -q, q / 2.0, -q / 2.0] {
                        let p = if along_y {
                            (off, t, 0.0)
                        } else {
                            (t, off, 0.0)
                        };
                        let e = (p.0, p.1, prepared_illuminance(&row, p, UP, mf));
                        if off == 0.0 {
                            centre.push(e);
                        }
                        band.push(e);
                    }
                }
                let (c, b) = (Stats::of(&centre), Stats::of(&band));
                route_checks(&rl, end, share, &c, &b, 0.0, Subject::Route(0))
                    .iter()
                    .all(|c| c.pass != Some(false))
            };
            let open_ok = |s: f64| -> bool {
                let grid = prepare(
                    &(-4..=4)
                        .flat_map(|i| (-4..=4).map(move |j| (i, j)))
                        .map(|(i, j)| lum(i as f64 * s, j as f64 * s))
                        .collect::<Vec<_>>(),
                );
                let mut pts = Vec::new();
                for i in 0..=8 {
                    for j in 0..=8 {
                        let p = (s * i as f64 / 8.0, s * j as f64 / 8.0, 0.0);
                        pts.push((p.0, p.1, prepared_illuminance(&grid, p, UP, mf)));
                    }
                }
                area_checks(&al, end, share, &Stats::of(&pts), true, Subject::Area(0))
                    .iter()
                    .all(|c| c.pass != Some(false))
            };
            // in 0.1 m steps from 0.5 m; the limits get harder with the
            // spacing, so the search stops 1 m after the first failure
            let largest = |ok: &dyn Fn(f64) -> bool| -> f64 {
                let mut best = 0.0;
                let mut first_fail: Option<f64> = None;
                let mut k = 5;
                while k <= 400 {
                    let s = k as f64 / 10.0;
                    if ok(s) {
                        best = s;
                    } else if first_fail.is_none() {
                        first_fail = Some(s);
                    }
                    if first_fail.is_some_and(|f| s >= f + 1.0) {
                        break;
                    }
                    k += 1;
                }
                best
            };
            let one = prepare(&[lum(0.0, 0.0)]);
            let (i_max, _) = glare_max(&one[0], &(0..360).map(|a| a as f64).collect::<Vec<_>>());
            // profiles without lux rules (France): their spacing rules alone
            let route_lux = rl.centre_min_lx > 0.0 || rl.band_min_lx > 0.0 || rl.avg_min_lx > 0.0;
            let area_lux = al.min_lx > 0.0 || al.avg_min_lx > 0.0;
            let route_max = rl.max_spacing_m.min(40.0);
            let grid_max = (al.max_spacing_per_height * h).min(40.0);
            let route = |along_y: bool| {
                if route_lux {
                    largest(&|s| route_ok(s, along_y)).min(route_max)
                } else {
                    route_max
                }
            };
            SpacingRow {
                height_m: h,
                e_nadir_lx: prepared_illuminance(&one, (0.0, 0.0, 0.0), UP, mf),
                route_spacing_c0_m: route(false),
                route_spacing_c90_m: route(true),
                route_width_m,
                open_area_grid_m: if area_lux {
                    largest(&open_ok).min(grid_max)
                } else {
                    grid_max
                },
                glare_i_max_cd: i_max,
                glare_limit_cd: if limits.checks_glare() {
                    limits.glare_limit(h, false)
                } else {
                    f64::INFINITY
                },
            }
        })
        .collect()
}

// ─── automatic layout ────────────────────────────────────────────────

/// What [`auto_layout`] places luminaires for.
#[derive(Debug, Clone)]
pub struct AutoInput<'a> {
    pub ldt: &'a Eulumdat,
    /// Emergency flux (lm).
    pub flux_lm: f64,
    /// Mounting height (m).
    pub height: f64,
    pub ra: Option<f64>,
    /// The luminaire's spacing at this height ([`spacing_table`]).
    pub spacing: &'a SpacingRow,
    pub routes: &'a [EscapeRoute],
    /// The room's outline (luminaires are only added inside it).
    pub room: Option<&'a [(f64, f64)]>,
    /// The open (anti-panic) area, if the room is one.
    pub open_area: Option<&'a [(f64, f64)]>,
    /// What the open area is (profiles with a minimum per place).
    pub open_area_place: PlaceCategory,
    pub points: &'a [EmphasisPoint],
}

impl<'a> AutoInput<'a> {
    /// The plan for luminaires at `positions` (x, y, C0 direction).
    pub fn plan(&self, positions: &[(f64, f64, f64)]) -> EmergencyPlan<'a> {
        EmergencyPlan {
            luminaires: positions
                .iter()
                .map(|&(x, y, r)| EmergencyLuminaire {
                    ldt: self.ldt,
                    flux_lm: self.flux_lm,
                    x,
                    y,
                    height: self.height,
                    rotation: r,
                    ra: self.ra,
                })
                .collect(),
            routes: self.routes.to_vec(),
            open_areas: self
                .open_area
                .filter(|p| p.len() >= 3)
                .map(|p| {
                    vec![OpenArea {
                        name: "room".into(),
                        polygon: p.to_vec(),
                        place: self.open_area_place,
                    }]
                })
                .unwrap_or_default(),
            high_risk: Vec::new(),
            points: self.points.to_vec(),
        }
    }
}

/// Point and direction (deg) at arc length `t` of a polyline.
fn along(p: &[(f64, f64)], mut t: f64) -> (f64, f64, f64) {
    let n = p.len().saturating_sub(1);
    for i in 0..n {
        let (a, b) = (p[i], p[i + 1]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if t <= len + 1e-9 || i + 1 == n {
            let k = if len > 0.0 {
                (t / len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dir = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
            return (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k, dir);
        }
        t -= len;
    }
    let last = p.last().copied().unwrap_or((0.0, 0.0));
    (last.0, last.1, 0.0)
}

/// Route luminaires (both ends and evenly between, C0 along the route) and
/// an open-area grid of `grid_m`, away from the route luminaires.
fn layout_with_grid(
    input: &AutoInput,
    route_f: &[f64],
    grid_m: Option<f64>,
) -> Vec<(f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    let spacing = input
        .spacing
        .route_spacing_c0_m
        .max(input.spacing.route_spacing_c90_m);
    for (ri, r) in input.routes.iter().enumerate() {
        let s = (spacing * route_f.get(ri).copied().unwrap_or(1.0)).max(0.5);
        let len = path_length(&r.path);
        if len <= 0.0 {
            continue;
        }
        let n = (len / s).ceil().max(1.0) as usize;
        for k in 0..=n {
            let p = along(&r.path, len * k as f64 / n as f64);
            if out.iter().all(|q| (q.0 - p.0).hypot(q.1 - p.1) >= 0.5) {
                out.push(p);
            }
        }
    }
    if let (Some(poly), Some(g)) = (input.open_area.filter(|p| p.len() >= 3), grid_m) {
        let area = AreaPolygon::new(poly.to_vec());
        let (x0, y0, x1, y1) = area.bounding_box();
        let g = g.max(0.5);
        let nx = ((x1 - x0) / g).ceil().max(1.0) as usize;
        let ny = ((y1 - y0) / g).ceil().max(1.0) as usize;
        let on_routes = out.len();
        for j in 0..ny {
            for i in 0..nx {
                let x = x0 + (i as f64 + 0.5) * (x1 - x0) / nx as f64;
                let y = y0 + (j as f64 + 0.5) * (y1 - y0) / ny as f64;
                if area.contains(x, y)
                    && out[..on_routes]
                        .iter()
                        .all(|q| (q.0 - x).hypot(q.1 - y) >= g * 0.5)
                {
                    out.push((x, y, 0.0));
                }
            }
        }
    }
    out
}

/// Automatic layout: luminaires at both ends of every route and evenly
/// between (C0 along the route); when the open area is larger than the
/// anti-panic limit, a grid over it, tightened (× 1.0 … 0.5 of the
/// spacing) until the area passes (the spacing holds for an endless grid,
/// along walls a luminaire has fewer neighbours); then, for points of
/// emphasis still short of their vertical illuminance, a luminaire in front
/// of each (C0 towards it, inside the room) at the distance (0.5–4 m) that
/// lights it best: a wide optic close by, a directional one from where its
/// beam meets the wall.
pub fn auto_layout(
    input: &AutoInput,
    limits: &Limits,
    settings: &Settings,
) -> Vec<(f64, f64, f64)> {
    let area_required = input.open_area.filter(|p| p.len() >= 3).is_some_and(|p| {
        polygon_area(p) > limits.area_limits(input.open_area_place).applies_above_m2
    });
    // tighten a failing route's row (× 0.85, down to 0.3: stairs need
    // 10 lx in China, the spacing table is for corridors) and the open
    // area's grid (1.0 … 0.5 of the spacing), until both pass
    let mut route_f = vec![1.0; input.routes.len()];
    let mut area_f: f64 = 1.0;
    let mut pos = Vec::new();
    for _ in 0..20 {
        let grid = area_required.then_some(input.spacing.open_area_grid_m * area_f);
        pos = layout_with_grid(input, &route_f, grid);
        let rep = evaluate(&input.plan(&pos), limits, settings);
        // the area first (its grid also lights the routes), then the routes
        if area_required && !rep.open_areas.iter().all(AreaResult::pass) && area_f > 0.35 {
            area_f -= 0.1;
            continue;
        }
        let mut changed = false;
        for (f, r) in route_f.iter_mut().zip(&rep.routes) {
            if !r.pass() && *f > 0.3 {
                *f *= 0.85;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let rep = evaluate(&input.plan(&pos), limits, settings);
    for (p, r) in input.points.iter().zip(&rep.points) {
        if r.pass != Some(false) {
            continue;
        }
        let face = p.facing.unwrap_or(90.0);
        let (fx, fy) = (face.to_radians().cos(), face.to_radians().sin());
        let z = p.z.unwrap_or(settings.emphasis_height_m);
        let vertical = |cand: (f64, f64, f64)| -> f64 {
            let mut with = pos.clone();
            with.push(cand);
            illuminance(
                &input.plan(&with).luminaires,
                (p.x, p.y, z),
                (fx, fy, 0.0),
                settings.maintenance_factor,
            )
        };
        let room = input
            .room
            .filter(|r| r.len() >= 3)
            .map(|r| AreaPolygon::new(r.to_vec()));
        let best = [0.5, 0.7, 0.9, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0, 3.5, 4.0]
            .into_iter()
            .map(|d| (p.x + fx * d, p.y + fy * d, face + 180.0))
            .filter(|c| room.as_ref().is_none_or(|r| r.contains(c.0, c.1)))
            .max_by(|a, b| vertical(*a).total_cmp(&vertical(*b)));
        if let Some(c) = best {
            pos.push(c);
        }
    }
    // exits whose floor is too dark in front (Russia: 5 lx): a luminaire
    // over the measured spot, or just behind it, whichever lights it best
    if limits.exit_floor_min_lx > 0.0 {
        let rep = evaluate(&input.plan(&pos), limits, settings);
        let room = input
            .room
            .filter(|r| r.len() >= 3)
            .map(|r| AreaPolygon::new(r.to_vec()));
        for (p, r) in input.points.iter().zip(&rep.points) {
            if r.floor_lx.is_none_or(|e| e >= limits.exit_floor_min_lx) {
                continue;
            }
            let face = p.facing.unwrap_or(90.0);
            let (fx, fy) = (face.to_radians().cos(), face.to_radians().sin());
            let spot = (p.x + fx * 0.5, p.y + fy * 0.5, 0.0);
            let floor = |cand: (f64, f64, f64)| -> f64 {
                let mut with = pos.clone();
                with.push(cand);
                illuminance(
                    &input.plan(&with).luminaires,
                    spot,
                    UP,
                    settings.maintenance_factor,
                )
            };
            let best = [0.3, 0.5, 0.7, 1.0, 1.5]
                .into_iter()
                .map(|d| (p.x + fx * d, p.y + fy * d, face))
                .filter(|c| room.as_ref().is_none_or(|r| r.contains(c.0, c.1)))
                .max_by(|a, b| floor(*a).total_cmp(&floor(*b)));
            if let Some(c) = best {
                pos.push(c);
            }
        }
    }
    pos
}
