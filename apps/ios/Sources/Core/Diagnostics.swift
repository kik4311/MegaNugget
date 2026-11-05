import Foundation

public enum DiagnosticEvent: String, Codable, Sendable {
    case availabilityChecked = "availability_checked"
    case engineNotImplemented = "engine_not_implemented"
    case recoveryGuidanceOpened = "recovery_guidance_opened"
}

public struct DiagnosticEntry: Identifiable, Sendable {
    public let id = UUID()
    public let date: Date
    public let event: DiagnosticEvent
}

/// Closed event vocabulary: no identifiers or free-form device output are stored.
public struct Diagnostics {
    public private(set) var entries: [DiagnosticEntry] = []
    public init() {}
    public mutating func record(_ event: DiagnosticEvent) {
        if entries.count == 200 { entries.removeFirst() }
        entries.append(DiagnosticEntry(date: Date(), event: event))
    }
}
