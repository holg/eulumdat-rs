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
            dependencies: ["EulumdatFFI"],
            path: "swift/Sources/Eulumdat"
        ),
        .binaryTarget(
            name: "EulumdatFFI",
            path: "swift/EulumdatFFI.xcframework"
        ),
        .testTarget(
            name: "EulumdatTests",
            dependencies: ["Eulumdat"],
            path: "swift/Tests/EulumdatTests"
        ),
    ]
)
