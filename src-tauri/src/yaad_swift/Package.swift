// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "yaad_swift",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "yaad_swift",
            type: .static,
            targets: ["yaad_swift"]),
    ],
    dependencies: [
        .package(url: "https://github.com/Brendonovich/swift-rs", from: "1.0.7")
    ],
    targets: [
        .target(
            name: "yaad_swift",
            dependencies: [
                .product(name: "SwiftRs", package: "swift-rs")
            ])
    ]
)

