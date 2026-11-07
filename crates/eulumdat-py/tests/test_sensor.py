"""Sensor bindings: sensor LDT and SensXML through `eulumdat.SensorFile`.

Run after `maturin develop` (or installing the built wheel):
    python -m pytest crates/eulumdat-py/tests/test_sensor.py
    python crates/eulumdat-py/tests/test_sensor.py
"""
import os
import zipfile

import eulumdat

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
FIX = os.path.join(ROOT, "crates", "eulumdat", "tests", "fixtures", "sensors")


def test_sensor_ldt():
    f = eulumdat.SensorFile.from_file(os.path.join(FIX, "demo_ceiling_360.ldt"), "sensor/sensldt")
    assert f.kind == "sensor LDT" and not f.is_sensxml
    assert f.motions == ["tangential", "radial", "presence"]
    assert f.default_height == 2.0 and f.warnings == []
    areas = {a.motion: a for a in f.areas(2.0)}
    t = areas["tangential"]
    assert t.detected and abs(t.max_range_m - 8.0) < 0.1
    assert t.quality is None and t.depth_limit_m is not None
    assert len(t.outline) > 10 and len(t.outline[0]) == 2
    # Below the end of the volume nothing is detected.
    assert not any(a.detected for a in f.areas(50.0))
    svg = f.plan_svg(2.0, size=400, theme=eulumdat.SvgTheme.Dark)
    assert svg.startswith("<svg") and "tangential walking" in svg


def test_sensxml():
    f = eulumdat.SensorFile.from_file(os.path.join(FIX, "demo_highbay_sensxml.xml"))
    assert f.is_sensxml and f.kind == "SensXML"
    assert f.measured_heights == [6.0, 10.0, 14.0]
    q = {a.motion: a.quality for a in f.areas(8.0)}
    assert q["radial"] == "interpolated" and q["presence"] == "extrapolated"
    planned = {a.motion: a for a in f.areas(10.0, planning=True)}
    assert planned["tangential"].detected


def test_sensors_inside_a_gldf():
    with zipfile.ZipFile(os.path.join(FIX, "demo_sensors.gldf")) as z:
        files = {n: z.read(n) for n in z.namelist() if n.startswith("sensor/")}
    kinds = {n: eulumdat.SensorFile.from_bytes(b).kind for n, b in files.items()}
    assert sorted(kinds.values()) == ["SensXML", "sensor LDT"]


def test_luminaire_ldt_as_sensor_is_flagged():
    path = os.path.join(ROOT, "crates", "eulumdat-wasm", "templates", "fluorescent_luminaire.ldt")
    f = eulumdat.SensorFile.from_file(path, "sensor/sensldt")
    assert f.warnings and "luminaire LDT" in f.warnings[0]


def test_garbage_raises():
    try:
        eulumdat.SensorFile.from_bytes(b"no sensor", "sensor/sensldt")
    except ValueError:
        return
    raise AssertionError("expected ValueError")


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
            print("ok", name)
