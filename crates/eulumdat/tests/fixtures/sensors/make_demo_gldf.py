#!/usr/bin/env python3
"""Synthetic GLDF with two sensors (Candelabra demo data, not a product).

Writes demo_sensors.gldf next to this script:
  ldc/demo_led_downlight.ldt          photometry (../colour_quality fixture)
  sensor/demo_ceiling_360.ldt         sensor/sensldt (detection volume)
  sensor/demo_highbay_sensxml.xml     sensor/sensxml (6 / 10 / 14 m)
Two <Sensor> entries, a SensorEmitter per sensor and one variant that uses
the light emitter and the ceiling sensor. Used by eulumdat-scene's GLDF
reader tests and to try the sensor pages of the GLDF tools.
"""
import os
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
CQ = os.path.join(HERE, "..", "colour_quality")

PRODUCT = """<?xml version="1.0" encoding="UTF-8"?>
<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd">
  <Header>
    <Manufacturer>Candelabra demo (synthetic, not a product)</Manufacturer>
    <FormatVersion major="1" minor="0" pre-release="3"/>
    <CreatedWithApplication>eulumdat-rs make_demo_gldf.py</CreatedWithApplication>
    <GldfCreationTimeCode>2026-11-05T12:00:00Z</GldfCreationTimeCode>
    <UniqueGldfId>0b0f6c1e-5e2b-4c43-9a6e-6f1d2d6a0001</UniqueGldfId>
    <Author>Candelabra</Author>
  </Header>
  <GeneralDefinitions>
    <Files>
      <File id="ldt-downlight" contentType="ldc/eulumdat" type="localFileName">demo_led_downlight.ldt</File>
      <File id="sens-ceiling" contentType="sensor/sensldt" type="localFileName">demo_ceiling_360.ldt</File>
      <File id="sens-highbay" contentType="sensor/sensxml" type="localFileName">demo_highbay_sensxml.xml</File>
    </Files>
    <Sensors>
      <Sensor id="sensor-ceiling">
        <SensorFileReference fileId="sens-ceiling"/>
        <DetectorCharacteristics><DetectorCharacteristic>Round</DetectorCharacteristic></DetectorCharacteristics>
        <DetectionMethods><DetectionMethod>Passive Infrared</DetectionMethod></DetectionMethods>
        <DetectorTypes><DetectorType>Presence</DetectorType></DetectorTypes>
      </Sensor>
      <Sensor id="sensor-highbay">
        <SensorFileReference fileId="sens-highbay"/>
        <DetectionMethods><DetectionMethod>Passive Infrared</DetectionMethod></DetectionMethods>
        <DetectorTypes><DetectorType>Motion</DetectorType></DetectorTypes>
      </Sensor>
    </Sensors>
    <Photometries>
      <Photometry id="ph-downlight"><PhotometryFileReference fileId="ldt-downlight"/></Photometry>
    </Photometries>
    <LightSources>
      <FixedLightSource id="ls-led">
        <Name><Locale language="en">LED 3000 K</Locale></Name>
        <RatedInputPower>18</RatedInputPower>
      </FixedLightSource>
    </LightSources>
    <Emitters>
      <Emitter id="em-light">
        <FixedLightEmitter>
          <PhotometryReference photometryId="ph-downlight"/>
          <LightSourceReference fixedLightSourceId="ls-led"/>
          <RatedLuminousFlux>2000</RatedLuminousFlux>
        </FixedLightEmitter>
      </Emitter>
      <Emitter id="em-sensor-ceiling"><SensorEmitter><SensorReference sensorId="sensor-ceiling"/></SensorEmitter></Emitter>
      <Emitter id="em-sensor-highbay"><SensorEmitter><SensorReference sensorId="sensor-highbay"/></SensorEmitter></Emitter>
    </Emitters>
  </GeneralDefinitions>
  <ProductDefinitions>
    <ProductMetaData>
      <UniqueProductId>0b0f6c1e-5e2b-4c43-9a6e-6f1d2d6a0002</UniqueProductId>
      <ProductNumber><Locale language="en">DEMO-SENS-01</Locale></ProductNumber>
      <Name><Locale language="en">Demo downlight with presence detector</Locale></Name>
      <Description><Locale language="en">Synthetic demo data: a downlight with a ceiling presence detector, plus a high-bay detector measured at 6, 10 and 14 m.</Locale></Description>
    </ProductMetaData>
    <Variants>
      <Variant id="v-with-sensor">
        <Name><Locale language="en">Downlight with presence detector</Locale></Name>
        <Mountings><Ceiling><Recessed recessedDepth="80"/></Ceiling></Mountings>
      </Variant>
    </Variants>
  </ProductDefinitions>
</Root>
"""


def main():
    out = os.path.join(HERE, "demo_sensors.gldf")
    entries = [
        ("product.xml", PRODUCT.encode("utf-8")),
        ("ldc/demo_led_downlight.ldt", open(os.path.join(CQ, "demo_led_downlight.ldt"), "rb").read()),
        ("sensor/demo_ceiling_360.ldt", open(os.path.join(HERE, "demo_ceiling_360.ldt"), "rb").read()),
        ("sensor/demo_highbay_sensxml.xml", open(os.path.join(HERE, "demo_highbay_sensxml.xml"), "rb").read()),
    ]
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(2026, 11, 5, 12, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    print("wrote", out)


if __name__ == "__main__":
    main()
