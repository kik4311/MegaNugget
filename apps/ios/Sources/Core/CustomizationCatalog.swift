import Foundation

/// Local appearance is independent of system modification and needs no pairing record.
public enum AppAppearance: String, CaseIterable, Identifiable, Sendable {
    case system, dark, light
    public var id: String { rawValue }
}

public enum CustomizationFeature: String, CaseIterable, Identifiable, Sendable {
    case appAppearance, statusBar, springBoard, wallpapers, daemons
    public var id: String { rawValue }
}

public enum CustomizationAvailability: Equatable, Sendable {
    case availableLocally
    case backendNotPorted
    case outsideInitialTarget
}

/// A research target is NOT proof that a system-write mechanism works.
/// This policy deliberately has no available system-write cases.
public struct CustomizationCatalog {
    public init() {}

    public func isInitialTarget(_ device: DeviceInfo) -> Bool {
        ["iPhone9,1", "iPhone9,3"].contains(device.model)
            && device.systemVersion == "15.8.8"
            && device.architecture == "arm64"
            && !device.isSimulator
    }

    public func availability(of feature: CustomizationFeature, for device: DeviceInfo) -> CustomizationAvailability {
        if feature == .appAppearance { return .availableLocally }
        return isInitialTarget(device) ? .backendNotPorted : .outsideInitialTarget
    }
}
