import XCTest
@testable import MegaNuggetCore

final class LegacyTweakPortTests: XCTestCase {
    private let port = LegacyTweakPort()
    private let iphone7 = DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64")

    private func entry(_ id: String, device: DeviceInfo? = nil) throws -> LegacyTweakSummary {
        try XCTUnwrap(port.catalog(for: device ?? iphone7).first { $0.id == id })
    }

    private func dictionary(_ file: LegacyPlistPreview) throws -> [String: Any] {
        let data = try XCTUnwrap(file.xml.data(using: .utf8))
        return try XCTUnwrap(PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any])
    }

    func testImportedCatalogAndNoApplyClaims() throws {
        XCTAssertEqual(port.registryCount, 258)
        let all = port.catalog(for: iphone7)
        XCTAssertGreaterThan(all.count, 258) // Daemon groups retained for provenance, not enabled.
        XCTAssertEqual(Set(all.map(\.id)).count, all.count)
        XCTAssertEqual(try entry("SBHideLowPowerAlerts").status, .draftCandidate)
        XCTAssertEqual(try entry("SBAlwaysShowSystemApertureInSnapshots").status, .requiresNewerIOS)
        XCTAssertTrue(all.contains { $0.id.hasPrefix("Daemon.") && $0.status != .draftCandidate })
    }

    func testNoNewerTweaksBecomeCompatibleByLoweringTarget() throws {
        let newer = port.catalog(for: iphone7).filter { ($0.minimumIOS?.hasPrefix("26") ?? false) || ($0.minimumIOS?.hasPrefix("27") ?? false) }
        XCTAssertFalse(newer.isEmpty)
        XCTAssertTrue(newer.allSatisfy { $0.status != .draftCandidate })
        XCTAssertThrowsError(try port.preview([newer[0].id: newer[0].defaultValue], for: iphone7))
    }

    func testStrictVersionsAndUnknownDevicesFailClosed() {
        for version in ["", "15..8", "15.8.8.1", " 15.8.8", "15.8.8 ", "15.8b", "+15.8", "99999999999999999999999999999"] {
            XCTAssertNil(TweakVersion.compare(version, "15.8.8"))
            let device = DeviceInfo(model: "iPhone9,1", systemVersion: version, architecture: "arm64")
            XCTAssertFalse(port.catalog(for: device).contains { $0.status == .draftCandidate })
        }
        let unknown = DeviceInfo(model: "iPhone99,1", systemVersion: "17.4", architecture: "arm64")
        XCTAssertFalse(port.catalog(for: unknown).contains { $0.status == .draftCandidate })
        let simulator = DeviceInfo(model: "iPhone9,1", systemVersion: "15.8.8", architecture: "arm64", isSimulator: true)
        XCTAssertFalse(port.catalog(for: simulator).contains { $0.status == .draftCandidate })
    }

    func testOSAndHardwareAreSeparateGates() throws {
        let impossible = DeviceInfo(model: "iPhone9,1", systemVersion: "17.4", architecture: "arm64")
        XCTAssertEqual(try entry("SBHideLowPowerAlerts", device: impossible).status, .hardwareMismatch)
        let iphoneX = DeviceInfo(model: "iPhone10,6", systemVersion: "16.7.1", architecture: "arm64")
        XCTAssertEqual(try entry("SBHideLowPowerAlerts", device: iphoneX).status, .draftCandidate)
        let modern = DeviceInfo(model: "iPhone15,2", systemVersion: "17.4", architecture: "arm64")
        XCTAssertEqual(try entry("HideDICompletely", device: modern).status, .draftCandidate)
        let noIsland = DeviceInfo(model: "iPhone13,2", systemVersion: "17.4", architecture: "arm64")
        XCTAssertEqual(try entry("HideDICompletely", device: noIsland).status, .hardwareMismatch)
        let outside = DeviceInfo(model: "iPhone15,2", systemVersion: "18.0", architecture: "arm64")
        XCTAssertEqual(try entry("SBHideLowPowerAlerts", device: outside).status, .outsideLegacyScope)
    }

    func testMergeOnlyExplicitChoicesAndNeverApply() throws {
        let preview = try port.preview(["SBHideLowPowerAlerts": .bool(true), "SBDontDimOrLockOnAC": .bool(false)], for: iphone7)
        XCTAssertFalse(preview.appliedToDevice)
        XCTAssertEqual(preview.selectedCount, 2)
        XCTAssertEqual(preview.files.count, 1)
        let file = try XCTUnwrap(preview.files.first)
        XCTAssertEqual(file.domain, "ManagedPreferencesDomain")
        XCTAssertEqual(file.relativePath, "mobile/com.apple.springboard.plist")
        let values = try dictionary(file)
        XCTAssertEqual(values.count, 2)
        XCTAssertEqual(values["SBHideLowPowerAlerts"] as? Bool, true)
        XCTAssertEqual(values["SBDontDimOrLockOnAC"] as? Bool, false)
    }

    func testPreserveTypesAndEscapeText() throws {
        let number = try port.preview(["SBMinimumLockscreenIdleTime": .integer(42)], for: iphone7)
        XCTAssertTrue(try XCTUnwrap(number.files.first).xml.contains("<integer>42</integer>"))
        let text = "Mega <Nugget> & iPhone"
        let preview = try port.preview(["LockScreenFootnote": .text(text)], for: iphone7)
        XCTAssertEqual(try dictionary(XCTUnwrap(preview.files.first))["LockScreenFootnote"] as? String, text)
    }

    func testInvalidValuesAndConflictsAreRejected() {
        let cases: [[String: LegacyTweakValue]] = [
            ["SBMinimumLockscreenIdleTime": .integer(601)],
            ["SBMinimumLockscreenIdleTime": .number(.infinity)],
            ["SBMinimumLockscreenIdleTime": .number(1.5)],
            ["SBHideLowPowerAlerts": .text("true")],
            ["LockScreenFootnote": .text(String(repeating: "x", count: 1025))],
            ["LockScreenFootnote": .text("bad\0text")],
            ["RTL": .bool(true), "LTR": .bool(true)],
            ["not_a_tweak": .bool(true)]
        ]
        for choices in cases { XCTAssertThrowsError(try port.preview(choices, for: iphone7)) }
        XCTAssertThrowsError(try port.preview([:], for: iphone7))
        XCTAssertThrowsError(try port.preview(Dictionary(uniqueKeysWithValues: (0..<65).map { ("id\($0)", .bool(true)) }), for: iphone7))
    }

    func testReversibleTweakDoesNotWriteUnselectedDefaults() throws {
        let device = DeviceInfo(model: "iPhone15,2", systemVersion: "17.4", architecture: "arm64")
        let preview = try port.preview(["HideDICompletely": .bool(false)], for: device)
        XCTAssertEqual(preview.files.count, 1)
        let values = try dictionary(XCTUnwrap(preview.files.first))
        XCTAssertEqual(Set(values.keys), ["SBSuppressDynamicIslandCompletely"])
        XCTAssertEqual(values["SBSuppressDynamicIslandCompletely"] as? Bool, false)
    }
}
