//! Offline tweak catalog, gate evaluation and plist draft compilation for
//! MegaNugget Desktop.
//!
//! The registry snapshot embedded here is exported verbatim from the pinned
//! GoldenNugget Mobile revision declared in `LICENSES/GoldenNuggetMobile-import.json`
//! (regenerate with `scripts/export_desktop_catalog.py`). The gate and compiler
//! logic is a host-side port of the iOS adapter
//! (`apps/ios/Sources/Core/LegacyTweakPort.swift`), with the same deliberate
//! restriction: drafts are generated **locally** and nothing in this crate ever
//! contacts a device, a backup or a restore channel.
//!
//! A `Status::DraftCandidate` verdict means an offline XML draft may be compiled
//! for this device/iOS combination. It is NOT evidence that the key works on
//! that version and it is not an apply pipeline.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub mod compiler;

pub use compiler::{DraftError, DraftPreview, PlistFile};

/// GoldenNugget Mobile revision the snapshot was exported from.
pub const UPSTREAM_REVISION: &str = "e4be86f050f4980ccc58de3b74f9de58b070b705";
/// The registry row count of the pinned snapshot; the export refuses other counts.
pub const REGISTRY_COUNT: usize = 258;
/// Maximum entries in one offline draft (mirrors the Swift adapter).
pub const PREVIEW_LIMIT: usize = 64;
/// Maximum UTF-8 bytes for a text draft value.
pub const MAX_TEXT_BYTES: usize = 1024;

const CATALOG_JSON: &str = include_str!("../catalog.json");

/// Factories whose plist write semantics are ported end to end. Everything
/// else stays `Status::MechanismNotPorted`, exactly like the Swift adapter:
/// unknown factories are neither guessed into a draft nor silently accepted.
const PORTED_FACTORIES: [&str; 3] = [
    "BasicPlistTweak",
    "AdvancedPlistTweak",
    "ReversibleBooleanPlistTweak",
];

/// One plist scalar a draft key may carry, tagged like the exported registry.
/// The *kind* decides the wire encoding: an `Integer` must reach the plist as an
/// `<integer>`, not as a `Double` that happens to have no fraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum TweakValue {
    Bool(bool),
    #[serde(rename = "int")]
    Integer(i64),
    Double(f64),
    #[serde(rename = "string")]
    Text(String),
}

impl TweakValue {
    /// Editor-side rendering; mirrors the Swift `display`, including the
    /// trailing `.0` a whole fraction keeps (f64 Debug formatting).
    pub fn display(&self) -> String {
        match self {
            TweakValue::Bool(value) => value.to_string(),
            TweakValue::Integer(value) => value.to_string(),
            TweakValue::Double(value) => format!("{value:?}"),
            TweakValue::Text(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Toggle,
    Text,
    Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Section {
    LiquidGlass,
    Springboard,
    InternalOptions,
    Daemons,
}

/// One registry row, field for field with the pinned Swift `TweakSpec`.
/// `multi_values` is non-nil exactly when `factory` builds the upstream dict
/// writer; `disabled`, `key == ""` or an `unavailable_reason` block a draft.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Spec {
    pub id: String,
    pub section: Section,
    pub title: String,
    /// Absolute device path; the backup domain is derived in `compiler`.
    pub path: String,
    pub key: String,
    pub value: TweakValue,
    pub kind: Kind,
    pub min_value: f64,
    pub max_value: f64,
    pub step: f64,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub iphone_only: bool,
    pub ipad_only: bool,
    pub disabled: bool,
    pub detail: Option<String>,
    pub multi_values: Option<std::collections::BTreeMap<String, TweakValue>>,
    pub reversible_boolean: bool,
    pub requires_dynamic_island: bool,
    pub factory: String,
    pub off_value: Option<TweakValue>,
    pub integer_boolean: bool,
    pub requires_camera_control: bool,
    pub requires_action_button: bool,
    pub requires_home_button: bool,
    pub requires_always_on: bool,
    pub warning: Option<String>,
    pub requires: Vec<String>,
    pub exclusive_with: Option<String>,
    pub unavailable_reason: Option<String>,
    pub supported_models: Vec<String>,
}

impl Spec {
    /// `writesWholeDict`: the spec replaces a location's whole dictionary
    /// instead of writing a single key/value pair.
    pub fn writes_whole_dict(&self) -> bool {
        self.multi_values.is_some()
    }

    /// Modified version of upstream `isCompatible`: fails closed for unknown or
    /// malformed device versions and bounds, never "succeeds" by accident.
    /// The adapter only drafts for iPhones, so `is_iphone` is fixed `true`.
    pub fn is_compatible(&self, device: &Device) -> bool {
        if version::compare(&device.system_version, &device.system_version).is_none() {
            return false;
        }
        if self.disabled || self.unavailable_reason.is_some() {
            return false;
        }
        if let Some(minimum) = &self.min_version {
            if !matches!(
                version::compare(&device.system_version, minimum),
                Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater)
            ) {
                return false;
            }
        }
        if let Some(maximum) = &self.max_version {
            if !matches!(
                version::compare(&device.system_version, maximum),
                Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Less)
            ) {
                return false;
            }
        }
        // Opaque NSKeyedArchives are not part of the legacy port.
        if self.factory == "SiriAITweak" {
            return false;
        }
        if self.ipad_only {
            return false;
        }
        // `iphone_only` specs stay reachable because drafting assumes an iPhone.
        if !self.supported_models.is_empty() && !self.supported_models.contains(&device.model) {
            return false;
        }
        if self.requires_dynamic_island && !has_dynamic_island(&device.model) {
            return false;
        }
        if self.requires_camera_control && !has_camera_control(&device.model) {
            return false;
        }
        if self.requires_action_button && !has_action_button(&device.model) {
            return false;
        }
        if self.requires_home_button && !has_home_button(&device.model) {
            return false;
        }
        if self.requires_always_on && !has_always_on(&device.model) {
            return false;
        }
        true
    }
}

/// The bundled, hash-referenced snapshot of the upstream registry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub repository: String,
    pub revision: String,
    pub swift_sha256: String,
    pub specs: Vec<Spec>,
}

impl Catalog {
    pub fn spec(&self, id: &str) -> Option<&Spec> {
        self.specs.iter().find(|spec| spec.id == id)
    }
}

/// Lazily parsed and validated snapshot; malformed bundles abort loudly.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let catalog: Catalog =
            serde_json::from_str(CATALOG_JSON).expect("bundled catalog.json is valid");
        assert_eq!(
            catalog.revision, UPSTREAM_REVISION,
            "catalog provenance changed"
        );
        assert_eq!(
            catalog.specs.len(),
            REGISTRY_COUNT,
            "registry snapshot changed size"
        );
        let mut seen = std::collections::HashSet::new();
        if !catalog.specs.iter().all(|spec| seen.insert(&spec.id)) {
            panic!("duplicate registry id");
        }
        catalog
    })
}

/// A device description is only used for gate evaluation; nothing accounts for
/// pairing or connection state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    pub model: String,
    pub system_version: String,
    pub architecture: String,
    pub is_simulator: bool,
}

impl Device {
    pub fn new(
        model: impl Into<String>,
        system_version: impl Into<String>,
        architecture: impl Into<String>,
    ) -> Self {
        Device {
            model: model.into(),
            system_version: system_version.into(),
            architecture: architecture.into(),
            is_simulator: false,
        }
    }
}

/// Strict version components: 1–3 dot-separated ASCII-digit parts, no spaces,
/// suffixes, signs or empty parts. A malformed version blocks evaluation
/// instead of silently passing gates.
pub mod version {
    use std::cmp::Ordering;

    fn components(version: &str) -> Option<Vec<i64>> {
        let parts: Vec<&str> = version.split('.').collect();
        if !(1..=3).contains(&parts.len()) {
            return None;
        }
        let mut out = Vec::with_capacity(parts.len());
        for part in parts {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            out.push(part.parse::<i64>().ok()?);
        }
        Some(out)
    }

    /// PEP-440-style zero-padded tuple compare, `None` when either side has no
    /// strict components.
    pub fn compare(lhs: &str, rhs: &str) -> Option<Ordering> {
        let mut left = components(lhs)?;
        let mut right = components(rhs)?;
        let count = left.len().max(right.len());
        left.resize(count, 0);
        right.resize(count, 0);
        Some(left[..].cmp(&right[..]))
    }
}

/// Hardware gates generated from the pinned snapshot (upstream
/// `device_capabilities.py`). Matching hardware is necessary, not sufficient:
/// it never verifies a write mechanism.
pub fn has_dynamic_island(model: &str) -> bool {
    matches!(
        model.trim(),
        "iPhone15,2"
            | "iPhone15,3"
            | "iPhone15,4"
            | "iPhone15,5"
            | "iPhone16,1"
            | "iPhone16,2"
            | "iPhone17,1"
            | "iPhone17,2"
            | "iPhone17,3"
            | "iPhone17,4"
            | "iPhone18,1"
            | "iPhone18,2"
            | "iPhone18,3"
            | "iPhone18,4"
    )
}

pub fn has_camera_control(model: &str) -> bool {
    matches!(
        model.trim(),
        "iPhone17,1"
            | "iPhone17,2"
            | "iPhone17,3"
            | "iPhone17,4"
            | "iPhone18,1"
            | "iPhone18,2"
            | "iPhone18,3"
            | "iPhone18,4"
            | "iPhone19,2"
            | "iPhone19,3"
            | "iPhone19,7"
    )
}

pub fn has_action_button(model: &str) -> bool {
    matches!(
        model.trim(),
        "iPhone16,1"
            | "iPhone16,2"
            | "iPhone17,1"
            | "iPhone17,2"
            | "iPhone17,3"
            | "iPhone17,4"
            | "iPhone17,5"
            | "iPhone18,1"
            | "iPhone18,2"
            | "iPhone18,3"
            | "iPhone18,4"
            | "iPhone18,5"
            | "iPhone19,2"
            | "iPhone19,3"
            | "iPhone19,7"
    )
}

pub fn has_home_button(model: &str) -> bool {
    matches!(
        model.trim(),
        "iPad11,1"
            | "iPad11,2"
            | "iPad11,3"
            | "iPad11,4"
            | "iPad11,6"
            | "iPad11,7"
            | "iPad12,1"
            | "iPad12,2"
            | "iPad5,1"
            | "iPad5,2"
            | "iPad5,3"
            | "iPad5,4"
            | "iPad6,11"
            | "iPad6,12"
            | "iPad6,3"
            | "iPad6,4"
            | "iPad6,7"
            | "iPad6,8"
            | "iPad7,1"
            | "iPad7,11"
            | "iPad7,12"
            | "iPad7,2"
            | "iPad7,3"
            | "iPad7,4"
            | "iPad7,5"
            | "iPad7,6"
            | "iPhone10,1"
            | "iPhone10,2"
            | "iPhone10,4"
            | "iPhone10,5"
            | "iPhone12,8"
            | "iPhone14,6"
            | "iPhone8,1"
            | "iPhone8,2"
            | "iPhone8,4"
            | "iPhone9,1"
            | "iPhone9,2"
            | "iPhone9,3"
            | "iPhone9,4"
    )
}

pub fn has_always_on(model: &str) -> bool {
    matches!(
        model.trim(),
        "iPhone15,2"
            | "iPhone15,3"
            | "iPhone16,1"
            | "iPhone16,2"
            | "iPhone17,1"
            | "iPhone17,2"
            | "iPhone18,1"
            | "iPhone18,2"
            | "iPhone18,3"
            | "iPhone18,4"
            | "iPhone19,2"
            | "iPhone19,3"
            | "iPhone19,7"
    )
}

/// What blocks a snapshot entry for a given device. Serialized to the UI as
/// snake_case; wording stays client-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// An offline XML draft may be compiled. Not an apply pipeline.
    DraftCandidate,
    RequiresNewerIos,
    UpstreamUnavailable,
    HardwareMismatch,
    OutsideLegacyScope,
    InvalidDevice,
    MechanismNotPorted,
}

/// OS/hardware existence bounds for the legacy adapter, ported verbatim from
/// the Swift `deviceStatus`. Device-model gates exist separately from the
/// catalog's own bounds.
const OS15_ONLY_MODELS: [&str; 7] = [
    "iPhone8,1",
    "iPhone8,2",
    "iPhone8,4",
    "iPhone9,1",
    "iPhone9,2",
    "iPhone9,3",
    "iPhone9,4",
];
const THROUGH_OS16_MODELS: [&str; 6] = [
    "iPhone10,1",
    "iPhone10,2",
    "iPhone10,3",
    "iPhone10,4",
    "iPhone10,5",
    "iPhone10,6",
];
const FROM_OS15_MODELS: [&str; 17] = [
    "iPhone11,2",
    "iPhone11,4",
    "iPhone11,6",
    "iPhone11,8",
    "iPhone12,1",
    "iPhone12,3",
    "iPhone12,5",
    "iPhone12,8",
    "iPhone13,1",
    "iPhone13,2",
    "iPhone13,3",
    "iPhone13,4",
    "iPhone14,2",
    "iPhone14,3",
    "iPhone14,4",
    "iPhone14,5",
    "iPhone14,6",
];
const FROM_OS16_MODELS: [&str; 4] = ["iPhone14,7", "iPhone14,8", "iPhone15,2", "iPhone15,3"];
const FROM_OS17_MODELS: [&str; 4] = ["iPhone15,4", "iPhone15,5", "iPhone16,1", "iPhone16,2"];

/// `Some(reason)` means the device description itself is not draftable:
/// invalid data, out-of-scope major version or a model/OS impossibility.
///
/// Ported stepwise from the Swift `deviceStatus` so each gate fails exactly
/// like the reference: malformed input is `InvalidDevice`, a major outside
/// 15–17 is `OutsideLegacyScope`, and an impossible model/OS pairing is
/// `HardwareMismatch`. Unknown models are `InvalidDevice`.
pub fn device_status(device: &Device) -> Option<Status> {
    let major = device
        .system_version
        .split('.')
        .next()
        .and_then(|part| part.parse::<i64>().ok());
    if major.is_none()
        || version::compare(&device.system_version, &device.system_version).is_none()
        || !matches!(&device.architecture[..], "arm64" | "arm64e")
        || device.is_simulator
    {
        return Some(Status::InvalidDevice);
    }
    let major = major.expect("checked above");
    if !(15..=17).contains(&major) {
        return Some(Status::OutsideLegacyScope);
    }
    // These are OS/hardware existence bounds, NOT proof that a tweak works.
    if OS15_ONLY_MODELS.contains(&&*device.model) {
        return if major == 15 {
            None
        } else {
            Some(Status::HardwareMismatch)
        };
    }
    if THROUGH_OS16_MODELS.contains(&&*device.model) {
        return if matches!(major, 15 | 16) {
            None
        } else {
            Some(Status::HardwareMismatch)
        };
    }
    if FROM_OS15_MODELS.contains(&&*device.model) {
        return None;
    }
    if FROM_OS16_MODELS.contains(&&*device.model) {
        return if matches!(major, 16 | 17) {
            None
        } else {
            Some(Status::HardwareMismatch)
        };
    }
    if FROM_OS17_MODELS.contains(&&*device.model) {
        return if major == 17 {
            None
        } else {
            Some(Status::HardwareMismatch)
        };
    }
    Some(Status::InvalidDevice)
}

/// Gate outcome for one registry entry on one device description.
pub fn status(spec: &Spec, device: &Device) -> Status {
    if let Some(blocked) = device_status(device) {
        return blocked;
    }
    if spec.disabled || spec.unavailable_reason.is_some() {
        return Status::UpstreamUnavailable;
    }
    if let Some(minimum) = &spec.min_version {
        match version::compare(&device.system_version, minimum) {
            None => return Status::UpstreamUnavailable,
            Some(std::cmp::Ordering::Less) => return Status::RequiresNewerIos,
            _ => {}
        }
    }
    if !spec.is_compatible(device) {
        return Status::HardwareMismatch;
    }
    if spec.section != Section::Daemons && PORTED_FACTORIES.contains(&spec.factory.as_str()) {
        return Status::DraftCandidate;
    }
    Status::MechanismNotPorted
}

/// Serializable summary for the Desktop UI, in snapshot order.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub section: Section,
    pub detail: Option<String>,
    pub warning: Option<String>,
    pub minimum_ios: Option<String>,
    pub maximum_ios: Option<String>,
    pub status: Status,
    pub default_value: TweakValue,
    pub destination: String,
    pub key: String,
    pub kind: Kind,
    pub writes_whole_dict: bool,
}

/// Summaries in registry order; daemons are not part of the host snapshot.
pub fn summarize(device: &Device) -> Vec<Summary> {
    catalog()
        .specs
        .iter()
        .map(|spec| Summary {
            id: spec.id.clone(),
            title: spec.title.clone(),
            section: spec.section,
            detail: spec.detail.clone(),
            warning: spec.warning.clone(),
            minimum_ios: spec.min_version.clone(),
            maximum_ios: spec.max_version.clone(),
            status: status(spec, device),
            default_value: spec.value.clone(),
            destination: spec.path.clone(),
            key: spec.key.clone(),
            kind: spec.kind,
            writes_whole_dict: spec.writes_whole_dict(),
        })
        .collect()
}

/// Headline numbers for the UI, derived from the same gates as `summarize`;
/// no separate code path that could disagree with the row list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Counts {
    pub total: usize,
    pub candidates: usize,
    pub requires_newer_ios: usize,
    pub upstream_unavailable: usize,
    pub hardware_mismatch: usize,
    pub outside_legacy_scope: usize,
    pub invalid_device: usize,
    pub mechanism_not_ported: usize,
}

pub fn counts(device: &Device) -> Counts {
    let mut counts = Counts {
        total: 0,
        candidates: 0,
        requires_newer_ios: 0,
        upstream_unavailable: 0,
        hardware_mismatch: 0,
        outside_legacy_scope: 0,
        invalid_device: 0,
        mechanism_not_ported: 0,
    };
    for summary in summarize(device) {
        counts.total += 1;
        match summary.status {
            Status::DraftCandidate => counts.candidates += 1,
            Status::RequiresNewerIos => counts.requires_newer_ios += 1,
            Status::UpstreamUnavailable => counts.upstream_unavailable += 1,
            Status::HardwareMismatch => counts.hardware_mismatch += 1,
            Status::OutsideLegacyScope => counts.outside_legacy_scope += 1,
            Status::InvalidDevice => counts.invalid_device += 1,
            Status::MechanismNotPorted => counts.mechanism_not_ported += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    fn device(model: &str, version: &str) -> Device {
        Device::new(model, version, "arm64")
    }

    #[test]
    fn strict_versions_fail_closed() {
        for malformed in [
            "",
            "15..8",
            "15.8.8.1",
            " 15.8.8",
            "15.8.8 ",
            "15.8b",
            "+15.8",
            ".15",
            "99999999999999999999999999999",
        ] {
            assert_eq!(version::compare(malformed, "15.8.8"), None, "{malformed}");
        }
        assert_eq!(version::compare("15.8", "15.8.0"), Some(Ordering::Equal));
        assert_eq!(version::compare("15.8.8", "16"), Some(Ordering::Less));
        assert_eq!(version::compare("16.0", "15.8.8"), Some(Ordering::Greater));
    }

    #[test]
    fn device_gates_match_swift_adapter() {
        assert_eq!(device_status(&device("iPhone9,1", "15.8.8")), None);
        assert_eq!(
            device_status(&device("iPhone9,1", "17.4")),
            Some(Status::HardwareMismatch)
        );
        assert_eq!(device_status(&device("iPhone10,6", "16.7.1")), None);
        assert_eq!(device_status(&device("iPhone15,2", "17.4")), None);
        assert_eq!(
            device_status(&device("iPhone15,2", "18.0")),
            Some(Status::OutsideLegacyScope)
        );
        assert_eq!(
            device_status(&device("iPhone99,1", "17.4")),
            Some(Status::InvalidDevice)
        );
        for malformed in ["", "15..8", "15.8b", "+15.8"] {
            assert_eq!(
                device_status(&device("iPhone9,1", malformed)),
                Some(Status::InvalidDevice)
            );
        }
        let mut simulator = device("iPhone9,1", "15.8.8");
        simulator.is_simulator = true;
        assert_eq!(device_status(&simulator), Some(Status::InvalidDevice));
    }

    #[test]
    fn catalog_snapshot_is_complete() {
        let all = catalog();
        assert_eq!(all.specs.len(), 258);
        assert_eq!(
            all.repository,
            "https://github.com/GoldenNugget-Team/GoldenNugget-mobile"
        );
        let target = device("iPhone9,1", "15.8.8");
        assert_eq!(summarize(&target).len(), 258);
        let target_counts = counts(&target);
        assert_eq!(target_counts.total, 258);
        assert!(
            target_counts.candidates > 0,
            "target device has draft candidates"
        );
        assert_eq!(target_counts.invalid_device, 0);
        assert_eq!(counts(&device("iPhone99,1", "17.4")).invalid_device, 258);
        assert!(all.spec("SBHideLowPowerAlerts").is_some());
    }
}
