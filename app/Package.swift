// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "NetProxy",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "NetProxy", targets: ["NetProxy"])],
    targets: [
        // Everything testable without a UI: wire model, control client, daemon supervision, view-models.
        .target(name: "NetProxyKit"),
        .executableTarget(name: "NetProxy", dependencies: ["NetProxyKit"]),
        .testTarget(name: "NetProxyKitTests", dependencies: ["NetProxyKit"]),
    ],
    swiftLanguageVersions: [.v5]
)
