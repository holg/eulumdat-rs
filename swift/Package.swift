// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "EulumdatKit",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(
            name: "EulumdatKit",
            targets: ["EulumdatKit"]
        ),
    ],
    targets: [
        .target(
            name: "EulumdatKit",
            dependencies: ["eulumdat_ffiFFI"],
            path: "Sources/Eulumdat"
        ),
        .binaryTarget(
            name: "eulumdat_ffiFFI",
            path: "eulumdat_ffiFFI.xcframework"
        ),
    ]
)
