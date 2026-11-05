public enum EngineStatus: String, Equatable, Sendable {
    case notImplemented = "not_implemented"
}

/// Future native implementations must document each privilege transition and obtain
/// explicit confirmation before mutation. This engine performs no privileged actions.
public protocol JailbreakEngine {
    func availability(for device: DeviceInfo) -> EngineStatus
    func execute(for device: DeviceInfo, userConfirmed: Bool) async -> EngineStatus
}

public struct UnimplementedJailbreakEngine: JailbreakEngine {
    public init() {}
    public func availability(for device: DeviceInfo) -> EngineStatus { .notImplemented }
    public func execute(for device: DeviceInfo, userConfirmed: Bool) async -> EngineStatus { .notImplemented }
}
