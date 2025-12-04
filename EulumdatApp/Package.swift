// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "EulumdatApp",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .executable(name: "EulumdatApp", targets: ["EulumdatApp"]),
    ],
    dependencies: [
        .package(path: "../swift"),
    ],
    targets: [
        .executableTarget(
            name: "EulumdatApp",
            dependencies: [
                .product(name: "EulumdatKit", package: "swift"),
            ],
            path: "EulumdatApp"
        ),
    ]
)
