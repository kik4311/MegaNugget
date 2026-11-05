// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MegaNuggetCore",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [.library(name: "MegaNuggetCore", targets: ["MegaNuggetCore"])],
    targets: [
        .target(name: "MegaNuggetCore", path: "Sources/Core"),
        .testTarget(name: "MegaNuggetCoreTests", dependencies: ["MegaNuggetCore"], path: "Tests")
    ]
)
