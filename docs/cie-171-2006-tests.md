# CIE 171:2006 Test Cases for eulumdat-goniosim

Reference: CIE 171:2006 "Test Cases to Assess the Accuracy of Lighting Computer Programs"

## Overview

CIE 171:2006 defines 14 analytical test cases in three domains:

| Domain | Test Cases | Relevant to goniosim |
|--------|-----------|---------------------|
| Direct artificial lighting | 5.1, 5.2, 5.3, 5.4 | Yes |
| Daylighting | 5.9-5.14 | No (sky models) |
| Diffuse reflections | 5.5, 5.6, 5.7, 5.8 | Yes |

We implement all 8 test cases relevant to Monte Carlo photon tracing through
luminaire geometry (TC 5.1-5.8). The 6 daylighting tests (TC 5.9-5.14)
are not applicable to goniophotometer simulation.

## Implemented Test Cases

### TC 5.1 — Point Source Direct Illumination (On-Axis)

**What it validates**: Inverse-square law for a point source directly above
a measurement plane.

**Setup**:
- Isotropic point source, Phi = 10,000 lm, at height h = 3 m above floor
- Measurement points on floor at radial distances r = 0, 1, 2, 3, 4, 5 m
  from the nadir point

**Analytical formula**:
```
E(r) = I * cos(theta) / d^2
     = (Phi / 4pi) * h / (h^2 + r^2)^(3/2)
```

**Expected values** (for Phi = 10,000 lm, h = 3 m):

| r [m] | theta [deg] | d [m] | E [lux] |
|-------|-------------|-------|---------|
| 0.0 | 0.0 | 3.000 | 88.42 |
| 1.0 | 18.4 | 3.162 | 75.45 |
| 2.0 | 33.7 | 3.606 | 52.15 |
| 3.0 | 45.0 | 4.243 | 31.83 |
| 4.0 | 53.1 | 5.000 | 19.10 |
| 5.0 | 59.0 | 5.831 | 11.68 |

**Tolerance**: < 2% at 1M photons.

### TC 5.2 — Point Source Direct Illumination (Off-Axis)

**What it validates**: Cosine correction for oblique incidence from point
sources at arbitrary positions.

**Setup**:
- Isotropic point source at position (1, 2, 3) m
- Measurement points on the floor (z=0) plane at various (x, y) positions
- Floor normal = +Z

**Analytical formula** (same as 5.1 but generalized):
```
E = (Phi / 4pi) * cos(theta) / d^2
```
where d = distance from source to measurement point,
theta = angle between floor normal and source direction.

**Tolerance**: < 2% at 1M photons.

### TC 5.3 — Area Source Direct Illumination

**What it validates**: Configuration factor (form factor) integration for
a finite rectangular diffuse emitter.

**Setup**:
- Rectangular diffuse emitter, 2 m x 1 m, luminance L = 1000 cd/m^2
- Emitter centered at (0, 0, 3) m, facing downward (-Z)
- Measurement points on floor at various positions

**Analytical formula**: Uses the closed-form configuration factor for
a parallel rectangle to a differential area element:
```
E = L * F_12
```
where F_12 is the configuration factor involving arctangent terms
(Howell catalog, case C-11).

**Tolerance**: < 5% at 1M photons (area sources converge slower).

### TC 5.5 — Directional Transmittance of Clear Glass

**What it validates**: Fresnel equations for a dielectric slab (clear glass,
6 mm, IOR = 1.52).

**Setup**:
- Collimated beam hitting a glass slab at varying incidence angles
- Measure fraction of photons transmitted

**Analytical formula** (Fresnel for unpolarized light):
```
R_s = ((n1*cos_i - n2*cos_t) / (n1*cos_i + n2*cos_t))^2
R_p = ((n2*cos_i - n1*cos_t) / (n2*cos_i + n1*cos_t))^2
R = (R_s + R_p) / 2
```
where cos_t = sqrt(1 - (n1/n2 * sin_i)^2), applied at both surfaces.

Total transmittance for a slab (two surfaces):
```
T_slab = (1 - R_entry) * (1 - R_exit)
```

**Expected values** (IOR = 1.52, 6mm clear glass):

| Incidence [deg] | T_slab (analytical) |
|-----------------|---------------------|
| 0 | 0.9174 |
| 15 | 0.9170 |
| 30 | 0.9143 |
| 45 | 0.9039 |
| 60 | 0.8596 |
| 75 | 0.6898 |
| 80 | 0.5333 |

**Tolerance**: < 1% absolute error at 100k photons per angle.

### TC 5.4 — Luminous Flux Conservation (Unglazed Opening)

**What it validates**: All photon energy is accounted for — detected +
absorbed + terminated = emitted.

**Setup**:
- Multiple scene configurations: free space, Lambertian, LED+housing,
  LED+housing+cover
- 100,000 photons per configuration

**Validation**: `photons_detected + photons_absorbed + photons_max_bounces +
photons_russian_roulette = photons_traced` for every configuration.

**Tolerance**: Exact equality (integer photon count).

### TC 5.6 — Diffuse Reflection from a Single Surface

**What it validates**: Single-bounce Lambertian reflection in a closed room.

**Setup**:
- Isotropic point source at center of a 4m cube
- Floor: Lambertian reflector (ρ = 0.5)
- Ceiling + 4 walls: perfect absorbers (ρ = 0)
- max_bounces = 2 (direct + one reflection)

**Validation**:
- Closed room: no photons escape (detected < 1%)
- Energy conservation: all photons absorbed within 2 bounces
- ~1/6 of photons hit the floor; of those, 50% reflect and hit another
  (absorbing) surface

**Tolerance**: < 1% escaped photons (closed box).

### TC 5.7 — Diffuse Reflections with Internal Obstruction

**What it validates**: Multi-bounce diffuse inter-reflections in the
presence of an internal opaque partition.

**Setup**:
- 4m integrating cube with ρ = 0.5 walls (same as TC 5.8)
- Absorbing partition (ρ = 0) at x = 0.5m, 2m wide × 3m tall
- Compare with unobstructed integrating cube

**Validation**:
- Both cases: closed room, no photons escape
- Partition absorbs photons that would otherwise bounce off walls
- Energy conservation verified in both configurations

**Note**: CIE 171:2006 Table 19 reference values contain errata.
We validate against self-consistent physical behavior rather than
the incorrect published values.

**Tolerance**: < 1% escaped photons, energy conservation < 1%.

### TC 5.8 — Diffuse Inter-Reflections (Integrating Sphere)

**What it validates**: Multi-bounce diffuse inter-reflections. The most
important test for global illumination accuracy.

**Setup**:
- Cubic room, 4 m x 4 m x 4 m (S_T = 96 m^2)
- All 6 surfaces are uniform Lambertian diffusers
- Isotropic point source at center, Phi = 10,000 lm
- Uniform reflectance rho on all surfaces
- Test at rho = 0%, 20%, 50%, 80%, 90%, 95%

**Analytical formula** (integrating sphere approximation):
```
E_direct = Phi / S_T = 10000 / 96 = 104.17 lux

E_indirect = E_direct * rho / (1 - rho)

E_total = E_direct / (1 - rho)
        = Phi / (S_T * (1 - rho))
```

**Expected values**:

| rho | E_total [lux] | E_indirect / E_direct |
|-----|---------------|-----------------------|
| 0.00 | 104.17 | 0.00 |
| 0.20 | 130.21 | 0.25 |
| 0.50 | 208.33 | 1.00 |
| 0.80 | 520.83 | 4.00 |
| 0.90 | 1041.67 | 9.00 |
| 0.95 | 2083.33 | 19.00 |

Note: The cube is not a perfect integrating sphere, so there is a small
geometric correction factor. The analytical formula assumes uniform
irradiance, which is exact for a sphere but approximate for a cube.
Difference is ~2-4% depending on measurement position (corners vs centers).

**Tolerance**: < 5% for rho <= 0.8, < 10% for rho = 0.9 and 0.95
(high reflectance requires many bounces to converge).

## Not Applicable

### TC 5.9-5.14 — Daylighting Tests
Not relevant for goniophotometer simulation (sky models, window
components, external masks).

## Notes

### TC 5.4 — Luminous Flux Conservation
Covered by the `cie_energy_conservation` test which verifies
detected + absorbed = emitted for every scene type.

### TC 5.7 — Errata in CIE 171:2006
CIE 171:2006 Table 19 contains incorrect reference values for TC 5.7.
Our implementation validates the physical behavior (energy conservation,
shadow casting by obstruction) using self-consistent reference values
rather than the erroneous published values.

## Known Errata in CIE 171:2006

1. **Table 19 (TC 5.7)**: Values computed with wrong geometry. Do not use.
2. **TC 5.13 and 5.14**: Questioned for invalid assumptions.
3. No official CIE errata published as of 2025.

Source: Ian Ashdown / Lighting Analysts errata analysis.

## Convergence Behavior

Monte Carlo convergence rate is 1/sqrt(N). Expected RMS error scaling:

| Photons | Expected RMS error |
|---------|-------------------|
| 10,000 | ~10% |
| 100,000 | ~3% |
| 1,000,000 | ~1% |
| 10,000,000 | ~0.3% |

For volume-scattering scenes (opal PMMA), convergence is slower due to
higher variance per photon path.

## References

- CIE 171:2006 "Test Cases to Assess the Accuracy of Lighting Computer Programs"
- Maamari, Fontoynont & Adra (2006), "Application of CIE Test Cases"
- Mangkuto (2016), "Validation of DIALux", LEUKOS 12(3):139-150
- Geisler-Moroder & Dur (2008), "Radiance Validation Against CIE 171:2006"
- NVIDIA iray Validation Report (2016)
