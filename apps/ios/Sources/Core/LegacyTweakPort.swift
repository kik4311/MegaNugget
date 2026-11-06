import Foundation

public enum LegacyTweakStatus: String, Sendable {
    case draftCandidate, requiresNewerIOS, upstreamUnavailable, hardwareMismatch
    case outsideLegacyScope, invalidDevice, mechanismNotPorted

    public var explanation: String {
        switch self {
        case .draftCandidate: return "Кандидат для черновика · работа на устройстве не проверена"
        case .requiresNewerIOS: return "Upstream требует более новую iOS"
        case .upstreamUnavailable: return "Отключено или ограничено в upstream"
        case .hardwareMismatch: return "Ограничение модели или версии устройства"
        case .outsideLegacyScope: return "Вне диапазона iOS 15–17"
        case .invalidDevice: return "Недостаточно корректных данных устройства"
        case .mechanismNotPorted: return "Специальный механизм не перенесён"
        }
    }
}

public enum LegacyTweakValue: Equatable, Sendable {
    case bool(Bool), integer(Int), number(Double), text(String)

    public var display: String {
        switch self {
        case .bool(let value): return value ? "true" : "false"
        case .integer(let value): return String(value)
        case .number(let value): return String(value)
        case .text(let value): return value
        }
    }

    fileprivate init(_ value: TweakValue) {
        switch value {
        case .bool(let value): self = .bool(value)
        case .int(let value): self = .integer(value)
        case .double(let value): self = .number(value)
        case .string(let value): self = .text(value)
        }
    }

    fileprivate var upstream: TweakValue {
        switch self {
        case .bool(let value): return .bool(value)
        case .integer(let value): return .int(value)
        case .number(let value): return .double(value)
        case .text(let value): return .string(value)
        }
    }
}

public struct LegacyTweakSummary: Identifiable, Sendable {
    public let id: String
    public let title: String
    public let section: String
    public let detail: String
    public let warning: String?
    public let minimumIOS: String?
    public let maximumIOS: String?
    public let status: LegacyTweakStatus
    public let defaultValue: LegacyTweakValue
    public let destination: String
    public let key: String
    public let isWholeDictionary: Bool
}

public struct LegacyPlistPreview: Identifiable, Sendable {
    public var id: String { domain + ":" + relativePath }
    public let domain: String
    public let relativePath: String
    public let xml: String
}

public struct LegacyDraftPreview: Sendable {
    public let files: [LegacyPlistPreview]
    public let selectedCount: Int
    /// Deliberately constant: XML generation is not device application.
    public let appliedToDevice = false
}

public enum LegacyDraftError: Error, LocalizedError {
    case emptySelection, tooManySelections, unknownTweak, blockedTweak, invalidValue
    case conflict(String), serializationFailed

    public var errorDescription: String? {
        switch self {
        case .emptySelection: return "Сначала добавьте твик в черновик."
        case .tooManySelections: return "Не более 64 настроек в одном черновике."
        case .unknownTweak: return "Черновик содержит неизвестный твик."
        case .blockedTweak: return "Твик не допускается к черновику для этого устройства/iOS."
        case .invalidValue: return "Неверный тип, длина или диапазон значения."
        case .conflict(let reason): return "Конфликт черновика: \(reason)"
        case .serializationFailed: return "Компилятор не создал корректный XML plist."
        }
    }
}

/// A Foundation-only adapter around the licensed upstream catalog/compiler.
/// No pairing records, networking, restore, file writes or apply API are exposed.
public struct LegacyTweakPort {
    public init() {}
    public static let upstreamRevision = "e4be86f050f4980ccc58de3b74f9de58b070b705"
    public var registryCount: Int { TweakCatalog.all.count }

    public func catalog(for device: DeviceInfo) -> [LegacyTweakSummary] {
        TweakCatalog.allWithDaemons.map { spec in
            LegacyTweakSummary(id: spec.id, title: spec.title, section: spec.section.rawValue,
                detail: spec.detail ?? "Определение импортировано из GoldenNugget Mobile.",
                warning: spec.warning, minimumIOS: spec.minVersion, maximumIOS: spec.maxVersion,
                status: status(spec, device: device), defaultValue: LegacyTweakValue(spec.value),
                destination: spec.location.rawValue, key: spec.key, isWholeDictionary: spec.writesWholeDict)
        }
    }

    private func deviceStatus(_ device: DeviceInfo) -> LegacyTweakStatus? {
        guard let majorText = device.systemVersion.split(separator: ".").first,
              let major = Int(majorText),
              TweakVersion.compare(device.systemVersion, device.systemVersion) != nil,
              ["arm64", "arm64e"].contains(device.architecture), !device.isSimulator else { return .invalidDevice }
        guard (15...17).contains(major) else { return .outsideLegacyScope }
        // These are OS/hardware existence bounds, NOT proof that a tweak works.
        let ios15Only = ["iPhone8,1", "iPhone8,2", "iPhone8,4", "iPhone9,1", "iPhone9,2", "iPhone9,3", "iPhone9,4"]
        let through16 = ["iPhone10,1", "iPhone10,2", "iPhone10,3", "iPhone10,4", "iPhone10,5", "iPhone10,6"]
        let from15 = ["iPhone11,2", "iPhone11,4", "iPhone11,6", "iPhone11,8", "iPhone12,1", "iPhone12,3", "iPhone12,5", "iPhone12,8", "iPhone13,1", "iPhone13,2", "iPhone13,3", "iPhone13,4", "iPhone14,2", "iPhone14,3", "iPhone14,4", "iPhone14,5", "iPhone14,6"]
        let from16 = ["iPhone14,7", "iPhone14,8", "iPhone15,2", "iPhone15,3"]
        let from17 = ["iPhone15,4", "iPhone15,5", "iPhone16,1", "iPhone16,2"]
        if ios15Only.contains(device.model) { return major == 15 ? nil : .hardwareMismatch }
        if through16.contains(device.model) { return major <= 16 ? nil : .hardwareMismatch }
        if from15.contains(device.model) { return nil }
        if from16.contains(device.model) { return major >= 16 ? nil : .hardwareMismatch }
        if from17.contains(device.model) { return major == 17 ? nil : .hardwareMismatch }
        return .invalidDevice
    }

    private func status(_ spec: TweakSpec, device: DeviceInfo) -> LegacyTweakStatus {
        if let blocked = deviceStatus(device) { return blocked }
        if spec.disabled || spec.unavailableReason != nil { return .upstreamUnavailable }
        if let minimum = spec.minVersion {
            guard let comparison = TweakVersion.compare(device.systemVersion, minimum) else { return .upstreamUnavailable }
            if comparison < 0 { return .requiresNewerIOS }
        }
        guard spec.isCompatible(deviceVersion: device.systemVersion, isIPhone: true, deviceModel: device.model) else { return .hardwareMismatch }
        guard spec.section != .daemons,
              ["BasicPlistTweak", "AdvancedPlistTweak", "ReversibleBooleanPlistTweak"].contains(spec.factory) else { return .mechanismNotPorted }
        return .draftCandidate
    }

    /// Serialize only explicit choices; never emit all reversible defaults.
    /// Each plist is a NEW dictionary proposal, not a merge with a device backup.
    public func preview(_ choices: [String: LegacyTweakValue], for device: DeviceInfo) throws -> LegacyDraftPreview {
        guard !choices.isEmpty else { throw LegacyDraftError.emptySelection }
        guard choices.count <= 64 else { throw LegacyDraftError.tooManySelections }
        for id in choices.keys where TweakCatalog.byID[id] == nil { throw LegacyDraftError.unknownTweak }
        let specs = TweakCatalog.allWithDaemons.filter { choices[$0.id] != nil }
        var selection = TweakSelection()
        for spec in specs {
            guard status(spec, device: device) == .draftCandidate else { throw LegacyDraftError.blockedTweak }
            guard let value = choices[spec.id] else { throw LegacyDraftError.unknownTweak }
            if spec.writesWholeDict && value != LegacyTweakValue(spec.value) { throw LegacyDraftError.invalidValue }
            if case .bool(true) = value, let peer = spec.exclusiveWith,
               case .bool(true)? = choices[peer] {
                throw LegacyDraftError.conflict("\(spec.id) и \(peer) взаимоисключающие.")
            }
            switch (spec.kind, value) {
            case (.toggle, .bool(let enabled)):
                if spec.offValue != nil || spec.reversibleBoolean { selection.setOn(enabled, for: spec) }
                else { selection.setValue(value.upstream, for: spec) }
            case (.text, .text(let text)):
                guard text.utf8.count <= 1024, !text.contains("\0"), !spec.writesWholeDict else { throw LegacyDraftError.invalidValue }
                selection.setValue(value.upstream, for: spec)
            case (.number, .integer(let number)):
                guard case .int = spec.value, Double(number) >= spec.minValue,
                      Double(number) <= spec.maxValue else { throw LegacyDraftError.invalidValue }
                selection.setValue(value.upstream, for: spec)
            case (.number, .number(let number)):
                guard case .double = spec.value, number.isFinite, number >= spec.minValue,
                      number <= spec.maxValue else { throw LegacyDraftError.invalidValue }
                selection.setValue(value.upstream, for: spec)
            default: throw LegacyDraftError.invalidValue
            }
        }
        if let issue = selection.validationIssue() { throw LegacyDraftError.conflict(issue) }
        // Also reject contradictory layout directions, even if metadata changes.
        if case .bool(true)? = choices["RTL"], case .bool(true)? = choices["LTR"] {
            throw LegacyDraftError.conflict("RTL и LTR нельзя включать одновременно.")
        }
        let result = TweakCompiler.compile(selection: selection, deviceVersion: device.systemVersion,
            isIPhone: true, deviceModel: device.model, specs: specs)
        guard result.skipped.isEmpty, !result.payloads.isEmpty else { throw LegacyDraftError.serializationFailed }
        let files = try result.payloads.map { payload -> LegacyPlistPreview in
            let data = try payload.bytes()
            guard !data.isEmpty, payload.source == nil,
                  (try PropertyListSerialization.propertyList(from: data, options: [], format: nil)) is [String: Any],
                  let xml = String(data: data, encoding: .utf8) else { throw LegacyDraftError.serializationFailed }
            return LegacyPlistPreview(domain: payload.domain, relativePath: payload.relativePath, xml: xml)
        }
        return LegacyDraftPreview(files: files, selectedCount: specs.count)
    }
}
