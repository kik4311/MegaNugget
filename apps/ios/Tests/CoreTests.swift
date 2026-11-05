import XCTest
@testable import MegaNuggetCore

final class CoreTests: XCTestCase {
    func testExactTargets() {
        for model in ["iPhone9,1", "iPhone9,3"] {
            XCTAssertEqual(CompatibilityManager().evaluate(DeviceInfo(model: model, systemVersion: "15.8.8", architecture: "arm64")), .researchTarget)
        }
        for version in ["15.8.7", "16.0", "17.0", "15.8.8 "] {
            XCTAssertEqual(CompatibilityManager().evaluate(DeviceInfo(model: "iPhone9,1", systemVersion: version, architecture: "arm64")), .unverified)
        }
        XCTAssertEqual(CompatibilityManager().evaluate(DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64", isSimulator: true)), .unverified)
    }

    func testNoVerifiedMethods() async {
        let device = DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64")
        let engine = UnimplementedJailbreakEngine()
        XCTAssertTrue(SecurityResearch.verifiedMethods.isEmpty)
        XCTAssertEqual(engine.availability(for: device), .notImplemented)
        let unconfirmed = await engine.execute(for: device, userConfirmed: false)
        let confirmed = await engine.execute(for: device, userConfirmed: true)
        XCTAssertEqual(unconfirmed, .notImplemented)
        XCTAssertEqual(confirmed, .notImplemented)
    }

    func testBoundedDiagnostics() {
        var diagnostics = Diagnostics()
        for _ in 0..<210 { diagnostics.record(.engineNotImplemented) }
        XCTAssertEqual(diagnostics.entries.count, 200)
    }
}
