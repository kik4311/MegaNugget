import XCTest
@testable import MegaNuggetCore

final class CustomizationTests: XCTestCase {
    private let catalog = CustomizationCatalog()
    private let target = DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64")

    func testLocalAppearanceWorksOutsideResearchTarget() {
        let other = DeviceInfo(model: "unknown", systemVersion: "unknown", architecture: "unknown")
        XCTAssertEqual(catalog.availability(of: .appAppearance, for: other), .availableLocally)
        XCTAssertEqual(AppAppearance.allCases.map(\.rawValue), ["system", "dark", "light"])
        XCTAssertNil(AppAppearance(rawValue: "invalid"))
    }

    func testSystemFeaturesNeverClaimToWork() {
        for feature in CustomizationFeature.allCases where feature != .appAppearance {
            XCTAssertEqual(catalog.availability(of: feature, for: target), .backendNotPorted)
        }
    }

    func testExactInitialTargetOnly() {
        XCTAssertTrue(catalog.isInitialTarget(target))
        XCTAssertTrue(catalog.isInitialTarget(DeviceInfo(model: "iPhone9,3", systemVersion: "15.8.8", architecture: "arm64")))
        let others = [
            DeviceInfo(model: "iPhone9,2", systemVersion: "15.8.8", architecture: "arm64"),
            DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.7", architecture: "arm64"),
            DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8 ", architecture: "arm64"),
            DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "other"),
            DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64", isSimulator: true)
        ]
        for device in others {
            XCTAssertFalse(catalog.isInitialTarget(device))
            XCTAssertEqual(catalog.availability(of: .statusBar, for: device), .outsideInitialTarget)
        }
    }
}
