"""Synthetic demo sensors (our own numbers, no vendor data).

Writes, next to this script:
  demo_ceiling_360.ldt   ceiling presence detector, sensor LDT (detection volume in m)
  demo_corridor.ldt      corridor detector, sensor LDT (a 24 m x 2.5 m slab)
  demo_ceiling_sensxml.xml   SensXML in the observed 4.5 layout (Latin-1)

Run: python3 make_demo.py
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def ldt(name, number, isym, c_angles, g_angles, rows, params, dims=(110, 110, 50)):
    lines = [
        "Candelabra demo (synthetic sensor, not a product)",
        "1",
        str(isym),
        str(len(c_angles)),
        f"{c_angles[1] - c_angles[0]:g}" if len(c_angles) > 1 else "0",
        str(len(g_angles)),
        f"{g_angles[1] - g_angles[0]:g}",
        "synthetic",
        name,
        number,
        f"{number}.ldt",
        "2026-11-04 Candelabra",
        str(dims[0]), "0", str(dims[2]),
        str(dims[0]), "0", "0", "0", "0", "0",
        "100", "100", "1", "0",
        "1",
        # lamp block: meaningless for a sensor
        "1", "sensor", "0", "-", "-", "0.4",
    ]
    lines += [f"{p:g}" for p in params]  # parameter block, lines 33-42
    lines += [f"{c:g}" for c in c_angles]
    lines += [f"{g:g}" for g in g_angles]
    for row in rows:
        lines += [f"{v:.4f}" for v in row]
    return "\r\n".join(lines) + "\r\n"


def ceiling_360():
    # sphere sector: slant range 8.25 m up to 80 deg, tapering to 0 at 85 deg
    g = [5.0 * i for i in range(19)]
    row = []
    for gamma in g:
        if gamma <= 80:
            row.append(8.25)
        elif gamma <= 85:
            row.append(8.25 * (85 - gamma) / 5)
        else:
            row.append(0.0)
    c = [15.0 * i for i in range(24)]
    # fields: 1 radial 0.5, 2 tangential 1, 3 presence 0.35, 6 presence detector, 8 ceiling
    params = [0.5, 1, 0.35, 0, 1, 1, 0, 2, 0, 0]
    return ldt("Demo ceiling presence detector 360", "DEMO-PD360", 1, c, g, [row], params)


def corridor():
    a, b, h = 12.0, 1.25, 5.0  # half length along C90, half width along C0, depth
    # 1 deg C-planes keep the straight edges; the box is symmetric about both
    # planes, so Isym 4 stores only C0..C90 (91 planes)
    c = [1.0 * i for i in range(360)]
    g = [2.5 * i for i in range(37)]
    rows = []
    for cc in c[:91]:
        row = []
        for gamma in g:
            cr, gr = math.radians(cc), math.radians(gamma)
            dx, dy, dz = math.sin(gr) * math.cos(cr), math.sin(gr) * math.sin(cr), math.cos(gr)
            t = float("inf")
            if abs(dx) > 1e-12:
                t = min(t, b / abs(dx))
            if abs(dy) > 1e-12:
                t = min(t, a / abs(dy))
            if dz > 1e-12:
                t = min(t, h / dz)
            row.append(t if math.isfinite(t) else 0.0)
        rows.append(row)
    # radial = tangential (field 1 = 1), no presence (field 3 = 0), ceiling
    params = [1, 1, 0, 0, 1, 0, 0, 2, 0, 0]
    return ldt("Demo corridor detector 24 m", "DEMO-CORR24", 4, c, g, rows, params)


def sensxml():
    out = [
        '<?xml version="1.0" encoding="iso-8859-1" standalone="yes"?>',
        '<SensXML xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="./sensXML.xsd">',
        "<Version>4.5</Version>",
        "<Header>",
        "<Manufacturer>Candelabra demo</Manufacturer>",
        "<ProductName>Synthetic ceiling presence detector</ProductName>",
        "<ProductGroup>Präsenzmelder (Demo)</ProductGroup>",
        "<ProductNumber>DEMO-SX360</ProductNumber>",
        "<ProductType>Presence</ProductType>",
        "<ProductTechnology>PIR</ProductTechnology>",
        "<Description>Synthetic data for testing, not a product</Description>",
        "<Laboratory>none (synthetic)</Laboratory>",
        "<ReportNumber>0</ReportNumber>",
        "<ReportDate>04.11.2026</ReportDate>",
        "</Header>",
        "<Sensor>",
        "<TotalSystemPower>0.4</TotalSystemPower>",
        "<MountingType>2</MountingType>",
        "<Quantity>3</Quantity>",
        "<Dimensions><Length>0.11</Length><Width>0</Width><Height>0.05</Height></Dimensions>",
        "<SensorCenter><LengthOffset>0</LengthOffset><WidthOffset>0</WidthOffset><HeightOffset>0</HeightOffset></SensorCenter>",
        "<Measurement><Type>synthetic</Type><Version>1</Version></Measurement>",
        "</Sensor>",
    ]

    def polar(n, h, r0, wobble, plan):
        lines = [f'<PolarData n="{n}">', "<NumberValues>36</NumberValues>",
                 "<AngularIncrement>10</AngularIncrement>", "<StartAngle>0</StartAngle>",
                 "<EndAngle>350</EndAngle>", f"<DistanceFromCenter>{h:g}</DistanceFromCenter>"]
        for a in range(0, 360, 10):
            r = r0 * (1 + wobble * math.cos(math.radians(4 * a)))
            lines.append(f'<RangeData angle="{a:7.3f}">{r:.2f}</RangeData>')
        lines += ["<PlanningData>", "<Shape>2</Shape>", "<ScaleShape>0</ScaleShape>",
                  "<DetectionRate>85</DetectionRate>", f"<Radius>{plan:g}</Radius>",
                  "<StartAngle>0</StartAngle>", "<EndAngle>360</EndAngle>", "</PlanningData>",
                  "</PolarData>"]
        return lines

    def source(n, dtype, polars):
        lines = [f'<DetectionSource n="{n}">', f"<DetectionType>{dtype}</DetectionType>",
                 f"<NumberPolarData>{len(polars)}</NumberPolarData>", "<NumberGridData>0</NumberGridData>",
                 "<ActiveGrid>0</ActiveGrid>", "<ShowRawData>1</ShowRawData>",
                 "<SamplingDirection>0</SamplingDirection>", "<RotationAngle>0</RotationAngle>"]
        for i, p in enumerate(polars, 1):
            lines += polar(i, *p)
        return lines + ["</DetectionSource>"]

    out += source(1, 1, [(2.5, 7.0, 0.04, 6.5), (3.0, 7.5, 0.04, 7.0), (4.0, 8.0, 0.05, 7.5)])
    out += source(2, 2, [(2.5, 3.0, 0.03, 2.75), (3.0, 3.5, 0.03, 3.25), (4.0, 4.0, 0.03, 3.75)])
    # presence: 0.5 m grid, disc of 2.5 m (2.5 m height) and 3.0 m (3.0 m height)
    out += ['<DetectionSource n="3">', "<DetectionType>3</DetectionType>",
            "<NumberPolarData>0</NumberPolarData>", "<NumberGridData>2</NumberGridData>",
            "<ActiveGrid>1</ActiveGrid>", "<ShowRawData>1</ShowRawData>",
            "<SamplingDirection>0</SamplingDirection>", "<RotationAngle>0</RotationAngle>"]
    for n, (h, rad, plan) in enumerate([(2.5, 2.5, 2.25), (3.0, 3.0, 2.75)], 1):
        out += [f'<GridData n="{n}">', "<NumCellsX>15</NumCellsX>", "<NumCellsY>15</NumCellsY>",
                "<DimCellX>0.5</DimCellX>", "<DimCellY>0.5</DimCellY>",
                f"<DistanceFromCenter>{h:g}</DistanceFromCenter>"]
        for j in range(15):
            for i in range(15):
                x, y = -3.5 + 0.5 * i, -3.5 + 0.5 * j
                on = 1 if math.hypot(x, y) <= rad - 0.25 else 0
                out.append(f'<CellData x="{x:6.3f}" y="{y:6.3f}">{on}</CellData>')
        out += ["<PlanningData>", "<Shape>2</Shape>", f"<Radius>{plan:g}</Radius>",
                "<StartAngle>0</StartAngle>", "<EndAngle>360</EndAngle>", "</PlanningData>",
                "</GridData>"]
    out += ["</DetectionSource>", "</SensXML>"]
    return "\n".join(out) + "\n"



def sensxml_doc(product, number, group, mounting, sources, power=0.5):
    """Generic SensXML 4.5 document in the observed layout.

    sources: list of dicts with `dtype` (1 tangential, 2 radial, 3 presence),
    `polars` = [(height, [(angle, radius)], plan)], `grids` = [(height, cell,
    n, on(x, y) -> bool, plan)]. plan: dict with shape and radius / length /
    width, optional start / end angle.
    """
    out = [
        '<?xml version="1.0" encoding="iso-8859-1" standalone="yes"?>',
        '<SensXML xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="./sensXML.xsd">',
        "<Version>4.5</Version>",
        "<Header>",
        "<Manufacturer>Candelabra demo</Manufacturer>",
        f"<ProductName>{product}</ProductName>",
        f"<ProductGroup>{group}</ProductGroup>",
        f"<ProductNumber>{number}</ProductNumber>",
        "<ProductTechnology>PIR</ProductTechnology>",
        "<Description>Synthetic data for testing, not a product</Description>",
        "<Laboratory>none (synthetic)</Laboratory>",
        "<ReportDate>05.11.2026</ReportDate>",
        "</Header>",
        "<Sensor>",
        f"<TotalSystemPower>{power:g}</TotalSystemPower>",
        f"<MountingType>{mounting}</MountingType>",
        f"<Quantity>{len(sources)}</Quantity>",
        "<Measurement><Type>synthetic</Type><Version>1</Version></Measurement>",
        "</Sensor>",
    ]

    def planning(plan):
        lines = ["<PlanningData>", f"<Shape>{plan['shape']}</Shape>", "<ScaleShape>0</ScaleShape>",
                 "<DetectionRate>85</DetectionRate>"]
        for key, tag in (("radius", "Radius"), ("length", "Length"), ("width", "Width")):
            if key in plan:
                lines.append(f"<{tag}>{plan[key]:g}</{tag}>")
        lines += [f"<StartAngle>{plan.get('start', 0):g}</StartAngle>",
                  f"<EndAngle>{plan.get('end', 360):g}</EndAngle>", "</PlanningData>"]
        return lines

    for n, src in enumerate(sources, 1):
        polars, grids = src.get("polars", []), src.get("grids", [])
        out += [f'<DetectionSource n="{n}">', f"<DetectionType>{src['dtype']}</DetectionType>",
                f"<NumberPolarData>{len(polars)}</NumberPolarData>",
                f"<NumberGridData>{len(grids)}</NumberGridData>",
                f"<ActiveGrid>{1 if grids else 0}</ActiveGrid>", "<ShowRawData>1</ShowRawData>",
                "<SamplingDirection>0</SamplingDirection>", "<RotationAngle>0</RotationAngle>"]
        for i, (h, ranges, plan) in enumerate(polars, 1):
            step = ranges[1][0] - ranges[0][0]
            out += [f'<PolarData n="{i}">', f"<NumberValues>{len(ranges)}</NumberValues>",
                    f"<AngularIncrement>{step:g}</AngularIncrement>",
                    f"<StartAngle>{ranges[0][0]:g}</StartAngle>",
                    f"<EndAngle>{ranges[-1][0]:g}</EndAngle>",
                    f"<DistanceFromCenter>{h:g}</DistanceFromCenter>"]
            out += [f'<RangeData angle="{a:7.3f}">{r:.2f}</RangeData>' for a, r in ranges]
            out += planning(plan) + ["</PolarData>"]
        for i, (h, cell, n_cells, on, plan) in enumerate(grids, 1):
            out += [f'<GridData n="{i}">', f"<NumCellsX>{n_cells}</NumCellsX>",
                    f"<NumCellsY>{n_cells}</NumCellsY>", f"<DimCellX>{cell:g}</DimCellX>",
                    f"<DimCellY>{cell:g}</DimCellY>", f"<DistanceFromCenter>{h:g}</DistanceFromCenter>"]
            half = (n_cells - 1) / 2
            for j in range(n_cells):
                for k in range(n_cells):
                    x, y = (k - half) * cell, (j - half) * cell
                    out.append(f'<CellData x="{x:6.3f}" y="{y:6.3f}">{1 if on(x, y) else 0}</CellData>')
            out += planning(plan) + ["</GridData>"]
        out.append("</DetectionSource>")
    out.append("</SensXML>")
    return "\n".join(out) + "\n"


def wall_180_sensxml():
    """Wall motion detector: sees only in front of the wall (C0 = straight out)."""
    def front(r0, side):
        rows = []
        for a in range(0, 360, 10):
            d = math.radians(a)
            in_front = a <= 90 or a >= 270
            rows.append((a, r0 * (side + (1 - side) * math.cos(d)) if in_front else 0.0))
        return rows
    sources = [
        {"dtype": 1, "polars": [
            (2.0, front(10.0, 0.7), {"shape": 2, "radius": 8.5, "start": 270, "end": 90}),
            (2.5, front(12.0, 0.7), {"shape": 2, "radius": 10.0, "start": 270, "end": 90})]},
        {"dtype": 2, "polars": [
            (2.0, front(5.0, 0.8), {"shape": 2, "radius": 4.25, "start": 270, "end": 90}),
            (2.5, front(6.0, 0.8), {"shape": 2, "radius": 5.0, "start": 270, "end": 90})]},
    ]
    return sensxml_doc("Synthetic wall motion detector 180°", "DEMO-SX180", "Bewegungsmelder (Demo)", 1,
                       sources, power=0.3)


def highbay_sensxml():
    """High-bay presence detector, 6 / 10 / 14 m: shows interpolation between heights."""
    def ring(r0):
        return [(a, r0 * (1 + 0.05 * math.cos(math.radians(6 * a)))) for a in range(0, 360, 10)]
    sources = [
        {"dtype": 1, "polars": [(6.0, ring(9.0), {"shape": 2, "radius": 8.0}),
                                (10.0, ring(13.0), {"shape": 2, "radius": 12.0}),
                                (14.0, ring(15.0), {"shape": 2, "radius": 14.0})]},
        {"dtype": 2, "polars": [(6.0, ring(4.0), {"shape": 2, "radius": 3.5}),
                                (10.0, ring(6.0), {"shape": 2, "radius": 5.5}),
                                (14.0, ring(7.0), {"shape": 2, "radius": 6.5})]},
        {"dtype": 3, "grids": [(6.0, 1.0, 9, lambda x, y: math.hypot(x, y) <= 3.0,
                                {"shape": 2, "radius": 2.5})]},
    ]
    return sensxml_doc("Synthetic high-bay presence detector", "DEMO-SXHB", "Präsenzmelder Hallen (Demo)", 2,
                       sources, power=0.6)


def corridor_sensxml():
    """Corridor detector along C0, rectangular planning shapes."""
    def rect(half_l, half_w):
        rows = []
        for a in range(0, 360, 5):
            d = math.radians(a)
            c, s_ = abs(math.cos(d)), abs(math.sin(d))
            rows.append((a, min(half_l / c if c > 1e-9 else 1e9, half_w / s_ if s_ > 1e-9 else 1e9)))
        return rows
    sources = [
        {"dtype": 1, "polars": [
            (2.5, rect(12.0, 2.0), {"shape": 4, "length": 21.6, "width": 3.6}),
            (3.0, rect(14.0, 2.25), {"shape": 4, "length": 25.2, "width": 4.0}),
            (4.0, rect(15.0, 2.5), {"shape": 4, "length": 27.0, "width": 4.5})]},
        {"dtype": 2, "polars": [
            (2.5, rect(8.0, 1.5), {"shape": 4, "length": 14.4, "width": 2.7}),
            (3.0, rect(9.0, 1.75), {"shape": 4, "length": 16.2, "width": 3.2}),
            (4.0, rect(10.0, 2.0), {"shape": 4, "length": 18.0, "width": 3.6})]},
    ]
    return sensxml_doc("Synthetic corridor detector", "DEMO-SXCOR", "Korridormelder (Demo)", 2, sources,
                       power=0.4)


if __name__ == "__main__":
    with open(os.path.join(HERE, "demo_ceiling_360.ldt"), "w", newline="") as f:
        f.write(ceiling_360())
    with open(os.path.join(HERE, "demo_corridor.ldt"), "w", newline="") as f:
        f.write(corridor())
    with open(os.path.join(HERE, "demo_ceiling_sensxml.xml"), "wb") as f:
        f.write(sensxml().encode("latin-1"))
    for name, fn in (("demo_wall_180_sensxml.xml", wall_180_sensxml),
                     ("demo_highbay_sensxml.xml", highbay_sensxml),
                     ("demo_corridor_sensxml.xml", corridor_sensxml)):
        with open(os.path.join(HERE, name), "wb") as f:
            f.write(fn().encode("latin-1"))
    print("wrote demo_ceiling_360.ldt, demo_corridor.ldt and four SensXML demos")
