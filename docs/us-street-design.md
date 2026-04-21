# US Street Lighting Standards — Implementation Guide for eulumdat-rs

## 1. Overview & Vision

This document defines the implementation of US/North American street lighting standards
into the eulumdat-rs ecosystem. The goal is a **compliance engine** that works at two levels:

1. **File-level classification** — computed from IES/LDT photometric data alone, no geometry needed
2. **Design-level compliance** — computed in the Designer/Interior, requiring layout + geometry

The engine should be modular enough to serve:
- eulumdat.icu / iesna.eu (browser via WASM)
- CLI batch processing (e.g., validate 500 IES files against RP-8 criteria)
- Library consumers (AEC integration, future clients)
- Future EU/DIN EN 13201 parity (same trait/interface, different tables)

---

## 2. Standards Inventory

### 2.1 Luminaire Classification (file-level, no geometry)

| Standard | What it classifies | Input needed | Output |
|---|---|---|---|
| ANSI/IES RP-8 §5 | IES Road Type (I–V, VS) | Half-max candela trace on isocandela plot | `Type II` |
| ANSI/IES RP-8 §5 | Longitudinal Classification (Short/Medium/Long/Very Long) | Max candela point vs MH TRL | `Medium` |
| IES TM-15-20 | BUG Rating (Backlight/Uplight/Glare) | Zonal lumen sums in 10 solid angles | `B2-U0-G1` |
| IES TM-15-20 | LCS Zones (FL, FM, FH, FVH, BL, BM, BH, BVH, UL, UH) | Zonal lumen sums | Lumens per zone |
| Legacy (deprecated) | Cutoff Classification | Max cd at 80° and 90° vertical | `Full Cutoff` |
| ANSI C78.377-2024 | CCT Bin | CCT value from file header | `Nominal 3000K` |
| CIE 13.3 | CIE Flux Codes | Intensity distribution | `42 72 95 100 68` |
| — | DLOR / ULOR | Flux below/above 90° | `DLOR: 95.2%` |
| — | LER (Luminaire Efficacy Rating) | Total lumens / total watts | `136 lm/W` |
| — | S/MH Ratio | From intensity distribution | `2.22` |

### 2.2 Design-Level Compliance (requires layout geometry)

| Standard | Scope | Key criteria |
|---|---|---|
| ANSI/IES RP-8-25 | Roadway & parking | Illuminance/luminance targets, uniformity, veiling luminance |
| IES RP-20 | Parking facilities | Illuminance by activity level |
| IES DG-21 | Residential streets | Simplified criteria |
| IDA/IES MLO | Light pollution / dark sky | BUG → Lighting Zone compliance matrix |
| AASHTO | Highway lighting | Referenced by state DOTs, largely follows RP-8 |

### 2.3 Future: EU/DIN Parity

| Standard | Scope | Notes |
|---|---|---|
| DIN EN 13201-1 | Lighting class selection | Wizard with more parameters than RP-8 |
| DIN EN 13201-2 | Performance requirements | M/C/P classes |
| DIN EN 13201-3 | Calculation methods | Luminance requires R-tables |
| DIN EN 13201-5 | Energy performance | PDI, annual energy |
| DIN 67523 | Pedestrian crossings | Min 30 lux vertical at 1m |
| CIE 115 | International roadway lighting | Framework standard |

---

## 3. File-Level Classifications

### 3.1 IES Road Type Classification

**Standard:** ANSI/IES RP-8, Section 5 (Luminaire Classification)

**Algorithm:**

The lateral type (I–V, VS) is determined by where the half-maximum candela trace
falls relative to Longitudinal Roadway Lines (LRL), expressed as multiples of
Mounting Height (MH):

```
Type I:   Half-max trace falls between 1.0 MH LRL (house side) and 1.0 MH LRL (street side)
Type II:  Half-max trace on street side is beyond 1.0 MH but not beyond 1.75 MH LRL
Type III: Half-max trace on street side is beyond 1.75 MH but not beyond 2.75 MH LRL
Type IV:  Half-max trace on street side is beyond 2.75 MH LRL
Type V:   Circular/symmetric distribution (essentially equal in all directions)
Type VS:  Zonal lumens in each of 8 octants are within ±10% of the average
```

**Longitudinal throw** is determined by where the maximum candela point falls relative
to Transverse Roadway Lines (TRL):

```
Very Short (VS): 0 to 1.0 MH TRL
Short (S):       1.0 to 2.25 MH TRL
Medium (M):      2.25 to 3.75 MH TRL
Long (L):        3.75 to 6.0 MH TRL
Very Long (VL):  Beyond 6.0 MH TRL
```

**Not applicable when:**
- Maximum candela vertical angle ≥ 90°
- Maximum candela horizontal angle falls on house side (exception: Type V/VS)
- Asymmetric distributions where type differs between 0–90° and 270–360° quadrants

**Rust interface:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum IesLateralType {
    TypeI,
    TypeII,
    TypeIII,
    TypeIV,
    TypeV,
    TypeVS,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IesLongitudinalThrow {
    VeryShort,
    Short,
    Medium,
    Long,
    VeryLong,
    NotApplicable,
}

#[derive(Debug, Clone)]
pub struct IesRoadClassification {
    pub lateral_type: IesLateralType,
    pub longitudinal_throw: IesLongitudinalThrow,
}

impl IesRoadClassification {
    /// Compute from intensity distribution data.
    /// Requires: C-plane intensity table, mounting height assumption (normalized to 1.0)
    pub fn from_intensity_distribution(photometry: &Photometry) -> Self { ... }

    /// Display string, e.g. "Type II Medium"
    pub fn display(&self) -> String { ... }
}
```

### 3.2 BUG Rating (TM-15-20)

**Standard:** IES TM-15-20, Luminaire Classification System for Outdoor Luminaires

The BUG rating classifies a luminaire's light output into three components,
each rated on a scale from 0 (least) to a maximum:

- **B (Backlight):** B0–B5 — light behind the luminaire (towards house side)
- **U (Uplight):** U0–U5 — light above horizontal (90°–180°)
- **G (Glare):** G0–G5 — light at high angles in forward direction

**Zone definitions (10 solid angles):**

```
Forward Light:
  FL  (Forward Low):       0°–30° vertical, front half
  FM  (Forward Mid):       30°–60° vertical, front half
  FH  (Forward High):      60°–80° vertical, front half
  FVH (Forward Very High): 80°–90° vertical, front half

Backlight:
  BL  (Back Low):          0°–30° vertical, back half
  BM  (Back Mid):          30°–60° vertical, back half
  BH  (Back High):         60°–80° vertical, back half
  BVH (Back Very High):    80°–90° vertical, back half

Uplight:
  UL  (Uplight Low):       90°–100° vertical
  UH  (Uplight High):      100°–180° vertical
```

**Rating thresholds (lumens in each zone):**

```
Backlight (BH + BVH lumens):
  B0: 0        B1: ≤ 500    B2: ≤ 1000   B3: ≤ 2500   B4: ≤ 5000   B5: > 5000

Uplight (UL + UH lumens):
  U0: 0        U1: ≤ 50     U2: ≤ 500    U3: ≤ 1000   U4: ≤ 2500   U5: > 2500

Glare (FH + FVH lumens):
  G0: ≤ 350    G1: ≤ 600    G2: ≤ 1100   G3: ≤ 2300   G4: ≤ 4600   G5: > 4600
```

Note: Exact thresholds should be verified against the published TM-15-20 tables.
Some thresholds depend on total luminaire lumens. Above is a simplified reference.

**Rust interface:**

```rust
#[derive(Debug, Clone)]
pub struct BugRating {
    pub backlight: u8,   // 0–5
    pub uplight: u8,     // 0–5
    pub glare: u8,       // 0–5
}

#[derive(Debug, Clone)]
pub struct LcsZones {
    pub fl: f64,   // Forward Low lumens
    pub fm: f64,   // Forward Mid lumens
    pub fh: f64,   // Forward High lumens
    pub fvh: f64,  // Forward Very High lumens
    pub bl: f64,   // Back Low lumens
    pub bm: f64,   // Back Mid lumens
    pub bh: f64,   // Back High lumens
    pub bvh: f64,  // Back Very High lumens
    pub ul: f64,   // Uplight Low lumens
    pub uh: f64,   // Uplight High lumens
}

impl LcsZones {
    pub fn from_photometry(photometry: &Photometry) -> Self { ... }
    pub fn bug_rating(&self) -> BugRating { ... }
    pub fn total_forward(&self) -> f64 { self.fl + self.fm + self.fh + self.fvh }
    pub fn total_back(&self) -> f64 { self.bl + self.bm + self.bh + self.bvh }
    pub fn total_uplight(&self) -> f64 { self.ul + self.uh }
}

impl fmt::Display for BugRating {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B{}-U{}-G{}", self.backlight, self.uplight, self.glare)
    }
}
```

### 3.3 Legacy Cutoff Classification (Deprecated)

Still referenced by many US municipalities. Based on maximum candela values
at specific vertical angles:

```
Full Cutoff:  0% at or above 90°, max 10% at or above 80°
Cutoff:       max 2.5% at or above 90°, max 10% at or above 80°
Semi-Cutoff:  max 5% at or above 90°, max 20% at or above 80°
Non-Cutoff:   exceeds Semi-Cutoff limits
```

Percentages are of rated lamp lumens.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum CutoffClassification {
    FullCutoff,
    Cutoff,
    SemiCutoff,
    NonCutoff,
    NotApplicable,  // absolute photometry
}

impl CutoffClassification {
    pub fn from_photometry(photometry: &Photometry) -> Self { ... }
}
```

### 3.4 ANSI C78.377-2024 CCT Bin Mapping

Simple lookup: given a CCT value, find the nearest nominal bin.

```rust
pub struct AnsiCctBin {
    pub nominal_cct: u16,
    pub label: &'static str,
}

const ANSI_CCT_BINS: &[(u16, u16, u16)] = &[
    // (nominal, lower_bound, upper_bound) — approximate boundaries
    (1800, 1650, 1900),
    (2000, 1900, 2100),
    (2200, 2100, 2350),
    (2500, 2350, 2600),
    (2700, 2600, 2850),
    (3000, 2850, 3250),
    (3500, 3250, 3750),
    (4000, 3750, 4250),
    (4500, 4250, 4750),
    (5000, 4750, 5350),
    (5700, 5350, 6100),
    (6500, 6100, 7000),
];

pub fn cct_to_ansi_bin(cct: u16) -> Option<u16> { ... }
```

Note: Actual bin boundaries are defined by chromaticity quadrangle corners on the
CIE diagram, not simple CCT thresholds. The above is a practical approximation
since IES/LDT files only carry a CCT number, not chromaticity coordinates.

### 3.5 CIE Flux Codes

Five values representing the percentage of total downward flux in cumulative
angular zones:

```
N1: 0°–30°   (direct downward)
N2: 0°–40°
N3: 0°–60°
N4: 0°–90°   (entire lower hemisphere)
N5: 0°–180°  (total sphere, usually = 100% for DLOR reference)
```

Displayed as: `42 72 95 100 68`

```rust
pub struct CieFluxCodes {
    pub n1: f64,  // % flux 0–30°
    pub n2: f64,  // % flux 0–40°
    pub n3: f64,  // % flux 0–60°
    pub n4: f64,  // % flux 0–90° (DLOR)
    pub n5: f64,  // % flux 0–180° (reference)
}

impl CieFluxCodes {
    pub fn from_photometry(photometry: &Photometry) -> Self { ... }
}
```

---

## 4. Design-Level Compliance (RP-8 Wizard)

### 4.1 RP-8 Selection Parameters

The user selects road parameters via dropdowns. Three inputs determine the criteria:

**Road Functional Classification:**

| Class | Description | Examples |
|---|---|---|
| Major | High-speed, high-volume arterials | Freeways, expressways, major arterials |
| Collector | Moderate traffic distributors | Distributor roads, minor arterials |
| Local | Low-speed residential | Neighborhood streets, cul-de-sacs |

**Pedestrian Conflict Area Classification:**

| Level | Pedestrians/hr | Typical context |
|---|---|---|
| High | > 100 | Urban commercial, dense mixed-use |
| Medium | 11–100 | Urban mixed, suburban commercial |
| Low | < 11 | Suburban residential, rural |

**Pavement Classification (for luminance method):**

| Class | Description | Q₀ range |
|---|---|---|
| R1 | Diffuse (concrete, light asphalt) | 0.10 |
| R2 | Mostly diffuse | 0.07 |
| R3 | Slightly specular | 0.07 |
| R4 | Mostly specular (wet-look dark asphalt) | 0.08 |

### 4.2 RP-8 Illuminance Criteria Tables

**Illuminance Method (horizontal illuminance on roadway):**

| Road Class | Ped. Conflict | Avg Maintained (lux) | Uniformity Avg/Min |
|---|---|---|---|
| Major | High | 17.0 | 3:1 |
| Major | Medium | 13.0 | 3:1 |
| Major | Low | 9.0 | 3:1 |
| Collector | High | 12.0 | 4:1 |
| Collector | Medium | 9.0 | 4:1 |
| Collector | Low | 6.0 | 4:1 |
| Local | High | 9.0 | 6:1 |
| Local | Medium | 7.0 | 6:1 |
| Local | Low | 4.0 | 6:1 |

Note: Values from RP-8-00. Latest RP-8-25 should be verified — structure is the same,
some values may have been adjusted.

**Luminance Method (cd/m² on roadway surface):**

| Road Class | Ped. Conflict | Avg Luminance (cd/m²) | U₀ (overall) | Uₗ (longitudinal) | TI max (%) |
|---|---|---|---|---|---|
| Major | High | 1.2 | 0.4 | 0.7 | 10 |
| Major | Medium | 0.9 | 0.4 | 0.7 | 10 |
| Major | Low | 0.6 | 0.4 | 0.7 | 10 |
| Collector | High | 0.8 | 0.4 | 0.5 | 10 |
| Collector | Medium | 0.6 | 0.4 | 0.5 | 10 |
| Collector | Low | 0.4 | 0.4 | 0.5 | 10 |
| Local | High | 0.6 | 0.4 | 0.5 | 10 |
| Local | Medium | 0.5 | 0.4 | 0.5 | 10 |
| Local | Low | 0.3 | 0.3 | — | 10 |

### 4.3 RP-8 Compliance Check

```rust
#[derive(Debug, Clone)]
pub struct Rp8Selection {
    pub road_class: RoadClass,         // Major, Collector, Local
    pub pedestrian_conflict: PedLevel, // High, Medium, Low
    pub method: CalcMethod,            // Illuminance or Luminance
    pub pavement: Option<PavementClass>, // R1–R4, only for luminance method
}

#[derive(Debug, Clone)]
pub struct Rp8Criteria {
    // Illuminance method
    pub avg_illuminance_lux: f64,
    pub uniformity_avg_min: f64,
    // Luminance method
    pub avg_luminance_cdm2: Option<f64>,
    pub overall_uniformity: Option<f64>,
    pub longitudinal_uniformity: Option<f64>,
    pub max_ti_percent: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Rp8ComplianceResult {
    pub criteria: Rp8Criteria,
    pub measured: Rp8Criteria,
    pub compliant: bool,
    pub failures: Vec<Rp8Violation>,
}

#[derive(Debug, Clone)]
pub struct Rp8Violation {
    pub parameter: String,        // e.g. "Average Illuminance"
    pub required: f64,
    pub achieved: f64,
    pub unit: String,
}

impl Rp8Selection {
    pub fn criteria(&self) -> Rp8Criteria {
        // Table lookup based on road_class × pedestrian_conflict
        ...
    }
}

pub fn check_rp8_compliance(
    selection: &Rp8Selection,
    design_result: &DesignerResult,
) -> Rp8ComplianceResult { ... }
```

### 4.4 Designer UI Integration

In the Designer tab, add a collapsible **"Compliance"** panel:

```
┌─ RP-8 Compliance ─────────────────────────────────────┐
│                                                        │
│  Road Class:    [Major ▾]                              │
│  Pedestrians:   [Medium ▾]                             │
│  Method:        [Illuminance ▾]                        │
│  Pavement:      [R2 ▾]  (luminance method only)        │
│                                                        │
│  ── Results ──────────────────────────────────────────  │
│                                                        │
│  Parameter          Required    Achieved    Status      │
│  ─────────────────  ─────────   ─────────   ─────────  │
│  Avg Illuminance    13.0 lux    15.7 lux    ✅         │
│  Uniformity (A/M)   ≤ 3.0       2.4         ✅         │
│  BUG Rating         —           B2-U0-G1    ℹ️          │
│  IES Type           —           Type II M   ℹ️          │
│                                                        │
│  Overall: ✅ COMPLIANT with RP-8 Major/Medium          │
│                                                        │
│  [Export Compliance Report PDF]                         │
└────────────────────────────────────────────────────────┘
```

---

## 5. IDA/IES Model Lighting Ordinance (MLO)

### 5.1 Lighting Zones

| Zone | Description | Typical area |
|---|---|---|
| LZ0 | No ambient light | Wilderness, dark sky preserves |
| LZ1 | Low ambient light | Rural, low-density residential |
| LZ2 | Moderate ambient light | Light commercial, suburban residential |
| LZ3 | Moderately high ambient light | Urban commercial, industrial |
| LZ4 | High ambient light | Dense urban, entertainment districts |

### 5.2 Maximum BUG Ratings per Lighting Zone

| Zone | B max | U max | G max |
|---|---|---|---|
| LZ0 | B1 | U0 | G0 |
| LZ1 | B2 | U1 | G1 |
| LZ2 | B3 | U2 | G2 |
| LZ3 | B4 | U3 | G3 |
| LZ4 | B5 | U4 | G4 |

Note: Simplified table. Actual MLO has additional nuances for hardscape vs softscape
adjacency and luminaire mounting height categories. Verify against published MLO tables.

```rust
pub fn check_mlo_compliance(
    bug: &BugRating,
    zone: LightingZone,
) -> MloComplianceResult { ... }
```

### 5.3 Integration

Add to the Info/Analysis tab (file-level, no Designer needed):

```
┌─ Dark Sky Compliance ─────────────────────────────────┐
│                                                        │
│  BUG Rating: B2-U0-G1                                  │
│                                                        │
│  Lighting Zone:  [LZ2 ▾]                               │
│  Max allowed:    B3-U2-G2                               │
│  Status:         ✅ Compliant                           │
│                                                        │
│  [LZ0]: ❌  [LZ1]: ❌  [LZ2]: ✅  [LZ3]: ✅  [LZ4]: ✅  │
└────────────────────────────────────────────────────────┘
```

---

## 6. Parking Facility Lighting (IES RP-20)

### 6.1 Criteria Table

| Facility type | Activity | Avg Maintained (lux) | Uniformity (Avg/Min) |
|---|---|---|---|
| Open parking | High | 10.0 | 4:1 |
| Open parking | Medium | 5.0 | 4:1 |
| Open parking | Low | 2.0 | 4:1 |
| Covered parking | Basic | 50.0 | 10:1 |
| Covered parking | Enhanced | 100.0 | 10:1 |
| Ramp entrance (day) | — | 500.0 | — |
| Ramp entrance (night) | — | 50.0 | — |

Note: Verify values against current RP-20 edition.

### 6.2 Integration

Same compliance panel pattern as RP-8, different dropdown options:

```
Facility:  [Open Parking ▾]  [Covered ▾]
Activity:  [High ▾]  [Medium ▾]  [Low ▾]
```

---

## 7. Unified Compliance Trait

To support both US and EU standards from the same interface:

```rust
pub trait LightingStandard {
    type Selection;
    type Criteria;
    type Result;

    fn name(&self) -> &str;
    fn region(&self) -> Region;  // US, EU, International
    fn criteria(&self, selection: &Self::Selection) -> Self::Criteria;
    fn check(
        &self,
        selection: &Self::Selection,
        design: &DesignerResult,
    ) -> Self::Result;
}

#[derive(Debug, Clone)]
pub enum Region {
    US,     // RP-8, TM-15, MLO, RP-20
    EU,     // EN 13201, DIN 67523
    CIE,    // CIE 115 (international framework)
    Custom, // Municipality-specific overrides
}

// Implementations:
pub struct Rp8Standard;    // US roadway
pub struct En13201;        // EU roadway (future)
pub struct MloStandard;    // Dark sky (file-level)
pub struct Rp20Standard;   // US parking
```

This allows the Designer UI to swap standards based on region without
changing the calculation or compliance-check pipeline.

---

## 8. CLI Batch Processing

For batch validation of photometric files:

```bash
# Classify all IES files in a directory
eulumdat classify --standard rp8 --format json ./ies_files/

# Output:
# { "file": "AEC_MAXWELL-8.ies", "type": "Type II", "throw": "Medium", "bug": "B2-U0-G1" }

# Check MLO compliance for a lighting zone
eulumdat compliance --standard mlo --zone LZ2 ./ies_files/

# Batch validate with RP-8 criteria
eulumdat compliance --standard rp8 --road major --ped medium ./design.json
```

---

## 9. Point-by-Point Display Mode

For US submittal requirements, add a **"Submittal Grid"** display mode
to the Designer heatmap:

### 9.1 Requirements

US municipalities require illuminance values printed at each grid point
on the plan view. This is the standard format for project submittals.

### 9.2 Display Modes

```rust
pub enum HeatmapDisplay {
    /// Smooth gradient heatmap — for presentations
    Gradient,
    /// Tiled cells with color fill — existing mode
    Tiles,
    /// Tiled cells with numeric values printed — US submittal mode
    PointByPoint {
        show_values: bool,
        decimal_places: u8,  // typically 1 for lux, 0 for footcandles
        font_size: FontSize, // Auto, Small, Medium — based on grid density
    },
}
```

### 9.3 Grid Spacing

Standard US practice:
- Exterior (roadway/parking): 5×5 ft or 10×10 ft point spacing
- Interior: varies by room size

The grid spacing maps to the existing `Resolution` parameter in the Designer.

---

## 10. C0/C90 Rotation

### 10.1 The Problem

IES files (US) define C0 along the luminaire width.
LDT/EULUMDAT files (EU) define C0 along the luminaire length.

When converting LDT→IES or when a US user views an EU-origin file,
the orientation is 90° off. Most software silently gets this wrong.

### 10.2 Implementation

```rust
pub enum CPlaneConvention {
    IES,      // C0 = across luminaire (US)
    Eulumdat, // C0 = along luminaire (EU)
}

pub fn rotate_c_planes(
    photometry: &mut Photometry,
    degrees: f64,  // typically 90.0 or -90.0
) { ... }

// Auto-detection hint: if the file is IES format but the header
// contains EU manufacturer info, suggest rotation to the user.
// Never auto-rotate silently — always show the user what happened.
```

### 10.3 UI

Add a rotation control in the viewer:

```
C-Plane Rotation: [0° ▾]  [90°] [180°] [270°]
Convention: [IES (C0=width) ▾] [EULUMDAT (C0=length)]
```

When rotation is applied, all diagrams, classifications, and calculations
update in real-time. The export/download should include the rotation
in the modified file header as a comment.

---

## 11. Implementation Priority

### Phase 1 — Quick wins (file-level, no geometry)

1. ANSI C78.377 CCT bin lookup
2. CIE Flux Codes
3. Legacy Cutoff Classification
4. DLOR/ULOR (if not already exposed)
5. C0/C90 rotation parameter

### Phase 2 — High-value classifications

6. BUG Rating (TM-15)
7. LCS Zones (TM-15)
8. MLO compliance check (BUG → lighting zone matrix)

### Phase 3 — Designer compliance

9. RP-8 wizard + compliance check (illuminance method)
10. Point-by-point submittal grid display
11. RP-20 parking criteria

### Phase 4 — Advanced / luminance

12. RP-8 luminance method (requires R-table integration)
13. Veiling luminance / TI calculation
14. EN 13201 parity (EU market)

### Phase 5 — Ecosystem

15. CLI batch classification/compliance
16. Unified `LightingStandard` trait for pluggable standards
17. PDF compliance report export
18. Municipality-specific overrides (custom criteria tables)

---

## 12. Data Sources & Verification

**Critical:** All threshold values, zone boundaries, and criteria tables in this document
are reference approximations. Before final implementation, verify against:

- ANSI/IES RP-8-25 (latest edition, $60 from IES store, 573 pages)
- IES TM-15-20 (BUG/LCS definitions, $30)
- ANSI C78.377-2024 (CCT bins, from NEMA)
- IDA/IES MLO (Model Lighting Ordinance, free from IDA)
- IES RP-20 (parking, from IES store)

Andy Koperski has offered to provide info on US/Canadian classifications.
Take him up on that — it's free domain expertise and deepens his engagement.

---

## 13. Competitive Landscape

| Tool | BUG | IES Type | RP-8 Compliance | Point-by-Point | Client-side |
|---|---|---|---|---|---|
| Acuity Visual 3D | ✅ | ✅ | ✅ | ✅ | ❌ (server) |
| AGi32 | ✅ | ✅ | ✅ | ✅ | ❌ (desktop) |
| DIALux | ✅ (EU focus) | ✅ | ❌ (EN 13201) | ✅ | ❌ (desktop) |
| Relux | ✅ (EU focus) | ✅ | ❌ (EN 13201) | ✅ | ❌ (desktop, free) |
| VISO Photometric Editor | Partial | Partial | ❌ | ❌ | ❌ |
| OxyTech LITESTAR | ✅ (EU focus) | ❌ | ❌ (EN 13201) | ✅ | ❌ (desktop) |
| **eulumdat.icu** | 🔜 | ✅ | 🔜 | 🔜 | **✅ (WASM)** |

The unique differentiator: **no other tool does all of this client-side in the browser.**
Every competitor is either server-side, desktop-only, or limited in scope.

---

## 14. Notes

- All classification algorithms operate on the same underlying intensity distribution
  data already parsed by eulumdat-rs. No new file format support needed.
- The `LightingStandard` trait enables future expansion to any regional standard
  without refactoring the core.
- Andy Koperski's feature requests (IES type, BUG, point-by-point, C0/C90)
  are all covered by Phases 1–3.
- The Chinese market (long-term strategic target) uses CIE-based standards
  similar to EU, so EN 13201 parity serves both markets.
