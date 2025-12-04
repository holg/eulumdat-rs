// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Eulumdat",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15),
        .tvOS(.v13),
        .watchOS(.v6)
    ],
    products: [
        .library(
            name: "Eulumdat",
            targets: ["Eulumdat"]
        ),
    ],
    targets: [
        .target(
            name: "Eulumdat",
            dependencies: ["eulumdat_ffiFFI"],
            path: "swift/Sources/Eulumdat"
        ),
        // Binary target name must match the module name in the XCFramework's modulemap
        .binaryTarget(
            name: "eulumdat_ffiFFI",
            path: "swift/eulumdat_ffiFFI.xcframework"
        ),
        .testTarget(
            name: "EulumdatTests",
            dependencies: ["Eulumdat"],
            path: "swift/Tests/EulumdatTests"
        ),
    ]
)
