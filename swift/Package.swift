// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "EulumdatKit",
    platforms: [
        .iOS(.v13),
        .macOS(.v13),
        .watchOS(.v6),
    ],
    products: [
        .library(
            name: "EulumdatKit",
            targets: ["EulumdatKit"]
        ),
    ],
    targets: [
        // Binary FFI target
        .binaryTarget(
            name: "eulumdat_ffiFFI",
            path: "eulumdat_ffiFFI.xcframework"
        ),

        // Swift bindings target
        .target(
            name: "EulumdatKit",
            dependencies: ["eulumdat_ffiFFI"],
            path: "Sources/Eulumdat"
        ),

        // Tests
        .testTarget(
            name: "EulumdatKitTests",
            dependencies: ["EulumdatKit"],
            path: "Tests/EulumdatKitTests"
        ),
    ]
)
