public struct ResearchMethod: Equatable, Sendable {
    public let identifier: String
    public let verified: Bool
}

/// Intentionally empty. Entries require reproducible hardware evidence and license review.
public enum SecurityResearch {
    public static let verifiedMethods: [ResearchMethod] = []
}
