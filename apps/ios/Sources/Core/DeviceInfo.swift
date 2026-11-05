import Foundation
#if canImport(UIKit)
import UIKit
import Darwin
#endif

public struct DeviceInfo: Equatable, Sendable {
    public let model: String
    public let systemVersion: String
    public let architecture: String
    public let isSimulator: Bool

    public init(model: String, systemVersion: String, architecture: String, isSimulator: Bool = false) {
        self.model = model
        self.systemVersion = systemVersion
        self.architecture = architecture
        self.isSimulator = isSimulator
    }
}

public protocol DeviceInfoProviding {
    func read() -> DeviceInfo
}

/// UIDevice and uname only. No serial number, UDID, IDFA or private APIs.
public struct LocalDeviceInfo: DeviceInfoProviding {
    public init() {}

    public func read() -> DeviceInfo {
        #if canImport(UIKit)
        var system = utsname()
        let result = uname(&system)
        let capacity = MemoryLayout.size(ofValue: system.machine)
        let model = result == 0 ? withUnsafePointer(to: &system.machine) {
            $0.withMemoryRebound(to: CChar.self, capacity: capacity) {
                String(cString: $0)
            }
        } : "unknown"
        #if arch(arm64)
        let architecture = "arm64"
        #else
        let architecture = "other"
        #endif
        #if targetEnvironment(simulator)
        let simulator = true
        #else
        let simulator = false
        #endif
        return DeviceInfo(model: model, systemVersion: UIDevice.current.systemVersion,
                          architecture: architecture, isSimulator: simulator)
        #else
        return DeviceInfo(model: "non-iOS host", systemVersion: "unknown", architecture: "unknown")
        #endif
    }
}
