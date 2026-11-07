#!/usr/bin/env python3
"""Demo white LED for the colour-quality card (synthetic, our numbers).

Writes, next to this script:
  demo_led_3000k.spd       relative spectrum, 380-780 nm at 1 nm: blue pump
                           (444 nm) + broad phosphor (583 nm) + a red
                           component (622 nm). A typical "3000 K, Ra 80" LED:
                           CCT ~3020 K, Duv ~0, Ra ~82, R9 ~10, TM-30 Rf ~83,
                           Rg ~98, reds (hue bin 1) ~11 % duller. The numbers
                           are pinned in tests/colour_quality_demo.rs.
  demo_led_downlight.ldt   a plain rotationally symmetric downlight
                           (2000 lm, cos^4.8 beam = 60 deg) as the body.

The editor template is the ATLA document of both:
  eulumdat convert demo_led_downlight.ldt demo_led_3000k.xml --spd demo_led_3000k.spd
then `eulumdat-templates add` (see crates/eulumdat-wasm-templates).
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))

# Spectrum model: amplitude, centre (nm), sigma (nm) per component.
COMPONENTS = [
    (0.445, 444.0, 14.3),  # blue pump
    (1.000, 583.0, 57.5),  # broad phosphor
    (0.440, 622.0, 20.0),  # red component
]


def spectrum():
    vals = []
    for wl in range(380, 781):
        v = sum(a * math.exp(-0.5 * ((wl - c) / s) ** 2) for a, c, s in COMPONENTS)
        vals.append((wl, v))
    peak = max(v for _, v in vals)
    return [(wl, v / peak) for wl, v in vals]


def write_spd(path):
    with open(path, "w", newline="\n") as f:
        f.write("# Demo LED 3000 K - synthetic spectrum (Candelabra demo, not a product)\n")
        f.write("# Wavelength[nm]\tRelative_Power\n")
        for wl, v in spectrum():
            f.write(f"{wl}\t{v:.6f}\n")


def write_ldt(path):
    k = 4.8  # cos^k: half intensity at 30 deg
    i0 = 1000.0 * (k + 1.0) / (2.0 * math.pi)  # cd/klm for 1000 lm
    gammas = [5 * i for i in range(37)]  # 0..180
    values = [i0 * math.cos(math.radians(g)) ** k if g < 90 else 0.0 for g in gammas]
    lines = [
        "Candelabra demo (synthetic, not a product)",
        "1",  # Ityp: point source, rotationally symmetric
        "1",  # Isym
        "1",  # Mc
        "0",  # Dc
        str(len(gammas)),
        "5",  # Dg
        "synthetic",
        "Demo LED downlight 3000 K, Ra 80 (synthetic spectrum)",
        "DEMO-LED-3000",
        "demo_led_downlight.ldt",
        "2026-11-05 Candelabra",
        "200", "0", "100",  # diameter, width, height (mm)
        "150", "0",  # luminous area diameter, width
        "0", "0", "0", "0",  # luminous area heights C0/C90/C180/C270
        "100",  # downward flux fraction
        "100",  # LOR
        "1",  # conversion factor
        "0",  # tilt
        "1",  # lamp sets
        "1", "LED", "2000", "3000K", "1B", "18",
    ]
    lines += ["0"] * 10  # direct ratios (recomputed by the tools)
    lines += ["0"]  # C angle
    lines += [f"{g}" for g in gammas]
    lines += [f"{v:.1f}" for v in values]
    with open(path, "w", newline="") as f:
        f.write("\r\n".join(lines) + "\r\n")


if __name__ == "__main__":
    write_spd(os.path.join(HERE, "demo_led_3000k.spd"))
    write_ldt(os.path.join(HERE, "demo_led_downlight.ldt"))
    print("wrote demo_led_3000k.spd and demo_led_downlight.ldt")
