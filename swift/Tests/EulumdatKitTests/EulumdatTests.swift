import XCTest
@testable import EulumdatKit

final class EulumdatTests: XCTestCase {

    let testLDT = """
    RUCO Licht GmbH
    1
    1
    1
    0
    19
    5
    0
    Test
    Supersky
    Test
    2021
    1500
    0
    129
    1486
    0
    0
    0
    0
    0
    100
    100.0
    1.0
    0.0
    1
    1
    LED 830
    19800
    3000
    80
    195
    0.358
    0.468
    0.545
    0.619
    0.675
    0.733
    0.776
    0.802
    0.847
    0.874
    0
    0
    5
    10
    15
    20
    25
    30
    35
    40
    45
    50
    55
    60
    65
    70
    75
    80
    85
    90
    386.8
    384.3
    377.1
    365.4
    349.7
    330.3
    307.8
    283.0
    256.5
    228.7
    200.5
    172.3
    144.3
    116.2
    88.6
    62.3
    38.4
    17.8
    0
    """

    func testParseLDT() throws {
        let ldt = try parseLdt(content: testLDT)

        XCTAssertEqual(ldt.luminaireName, "Test")
        XCTAssertEqual(ldt.symmetry, .verticalAxis)
        XCTAssertEqual(ldt.typeIndicator, .pointSourceSymmetric)
        XCTAssertGreaterThan(ldt.maxIntensity, 0)
    }

    func testBugRating() throws {
        let ldt = try parseLdt(content: testLDT)
        let rating = calculateBugRating(ldt: ldt)

        XCTAssertLessThanOrEqual(rating.b, 5)
        XCTAssertLessThanOrEqual(rating.u, 5)
        XCTAssertLessThanOrEqual(rating.g, 5)
    }

    func testPolarSVG() throws {
        let ldt = try parseLdt(content: testLDT)
        let svg = generatePolarSvg(ldt: ldt, width: 500, height: 500, theme: .light)

        XCTAssertTrue(svg.contains("<svg"))
        XCTAssertTrue(svg.contains("</svg>"))
    }

    func testButterflySVG() throws {
        let ldt = try parseLdt(content: testLDT)
        let svg = generateButterflySvg(ldt: ldt, width: 500, height: 400, tiltDegrees: 60, theme: .dark)

        XCTAssertTrue(svg.contains("<svg"))
    }

    func testCartesianSVG() throws {
        let ldt = try parseLdt(content: testLDT)
        let svg = generateCartesianSvg(ldt: ldt, width: 600, height: 400, maxCurves: 8, theme: .light)

        XCTAssertTrue(svg.contains("<svg"))
    }

    func testHeatmapSVG() throws {
        let ldt = try parseLdt(content: testLDT)
        let svg = generateHeatmapSvg(ldt: ldt, width: 700, height: 500, theme: .light)

        XCTAssertTrue(svg.contains("<svg"))
    }

    func testBugSVG() throws {
        let ldt = try parseLdt(content: testLDT)
        let svg = generateBugSvg(ldt: ldt, width: 400, height: 350, theme: .light)

        XCTAssertTrue(svg.contains("<svg"))
    }
}
