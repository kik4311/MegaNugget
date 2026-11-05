public enum CompatibilityStatus: String, Sendable {
    case researchTarget = "research_target"
    case unverified
}

public struct CompatibilityManager {
    public init() {}
    public func evaluate(_ device: DeviceInfo) -> CompatibilityStatus {
        let target = ["iPhone9,1", "iPhone9,3"].contains(device.model)
            && device.systemVersion == "15.8.8" && device.architecture == "arm64"
            && !device.isSimulator
        return target ? .researchTarget : .unverified
    }
}
