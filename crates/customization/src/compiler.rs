//! Offline draft compilation: the restricted host-side port of the pinned
//! upstream `TweakCompiler` + `TweakDomainMap` (see
//! `apps/ios/Sources/Core/LegacyTweakPort.swift` for the adapter it mirrors).
//!
//! Invariants carried over from the port, not inherited silently:
//! * only **explicitly selected** specs are compiled — never all reversible
//!   defaults of the catalog;
//! * every plist is a NEW dictionary proposal; nothing is merged with a device
//!   backup here and nothing in this crate reaches a restore channel;
//! * an unmapped location, an empty key, a failed serialization or a zero-file
//!   draft fails the whole draft instead of producing a partial success.

use crate::{catalog, device_status, status, version, Device, Kind, Spec, Status, TweakValue};
use plist::{Dictionary as PlistDict, Value as PlistValue};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Absolute device path → backup domain, ported from the pinned upstream
/// `TweakDomainMap` (`src/restore/path_mapping.py`). One table, one order.
///
/// Order is load-bearing: `/var/Managed Preferences/` must be reached before a
/// looser `/var/` prefix could shadow it.
const RULES: [(&str, &str, bool); 8] = [
    (
        "/var/Managed Preferences/",
        "ManagedPreferencesDomain",
        false,
    ),
    ("/var/root/", "RootDomain", false),
    ("/var/preferences/", "SystemPreferencesDomain", false),
    ("/var/MobileDevice/", "MobileDeviceDomain", false),
    ("/var/mobile/", "HomeDomain", false),
    ("/var/db/", "DatabaseDomain", false),
    (
        "/var/containers/Shared/SystemGroup/",
        "SysSharedContainerDomain-",
        true,
    ),
    (
        "/var/containers/Data/SystemGroup/",
        "SysContainerDomain-",
        true,
    ),
];

/// The `.GlobalPreferences` path that gets the HomeDomain mirror below.
const GLOBAL_PREFERENCES: &str = "/var/Managed Preferences/mobile/.GlobalPreferences.plist";
const GLOBAL_PREFERENCES_HOME: &str = "/var/mobile/Library/Preferences/.GlobalPreferences.plist";

/// A draft payload rendered as XML plist text plus its restore coordinates.
/// `xml` is display data only: this crate has no transport to a device.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlistFile {
    pub domain: String,
    pub relative_path: String,
    pub xml: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DraftPreview {
    pub files: Vec<PlistFile>,
    pub selected_count: usize,
    /// Deliberately constant: XML generation is not device application. The
    /// only way a draft reaches an iPhone in this project is a separately
    /// validated backup/restore channel that this crate does not contain.
    pub applied_to_device: bool,
}

impl DraftPreview {
    fn new(files: Vec<PlistFile>, selected_count: usize) -> Self {
        DraftPreview {
            files,
            selected_count,
            applied_to_device: false,
        }
    }
}

/// One contradiction or bad entry anywhere in the draft fails the whole draft.
#[derive(Debug, Clone, PartialEq)]
pub enum DraftError {
    EmptySelection,
    TooManySelections,
    UnknownTweak,
    BlockedTweak,
    InvalidValue,
    Conflict(String),
    SerializationFailed,
}

impl DraftError {
    /// Stable snake_case IPC code; wording stays client-side.
    pub fn code(&self) -> &'static str {
        match self {
            DraftError::EmptySelection => "tweaks_empty_selection",
            DraftError::TooManySelections => "tweaks_too_many_selections",
            DraftError::UnknownTweak => "tweaks_unknown_tweak",
            DraftError::BlockedTweak => "tweaks_blocked_tweak",
            DraftError::InvalidValue => "tweaks_invalid_value",
            DraftError::Conflict(_) => "tweaks_conflict",
            DraftError::SerializationFailed => "tweaks_compile_failed",
        }
    }
}

impl std::fmt::Display for DraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftError::EmptySelection => write!(formatter, "Сначала добавьте твик в черновик."),
            DraftError::TooManySelections => write!(
                formatter,
                "Не более {} настроек в одном черновике.",
                crate::PREVIEW_LIMIT
            ),
            DraftError::UnknownTweak => write!(formatter, "Черновик содержит неизвестный твик."),
            DraftError::BlockedTweak => write!(
                formatter,
                "Твик не допускается к черновику для этого устройства/iOS."
            ),
            DraftError::InvalidValue => {
                write!(formatter, "Неверный тип, длина или диапазон значения.")
            }
            DraftError::Conflict(reason) => write!(formatter, "Конфликт черновика: {reason}"),
            DraftError::SerializationFailed => {
                write!(formatter, "Компилятор не создал корректный XML plist.")
            }
        }
    }
}

impl std::error::Error for DraftError {}

/// `split_path_into_domain`: the first matching prefix rule wins; container
/// rules fold the first path segment into the domain name. Returns `None` when
/// no rule matches — draft compilation refuses instead of emitting a row no
/// device manifest ever produced.
pub fn split_domain(path: &str) -> Option<(String, String)> {
    for (prefix, domain, is_container) in RULES {
        let Some(rest) = path.strip_prefix(prefix) else {
            continue;
        };
        if !is_container {
            return Some((domain.to_string(), rest.to_string()));
        }
        // Python's `rest.partition("/")`: no separator means the whole rest is
        // the group name and the relative path is empty.
        return match rest.split_once('/') {
            Some((group, remainder)) => Some((format!("{domain}{group}"), remainder.to_string())),
            None => Some((format!("{domain}{rest}"), String::new())),
        };
    }
    None
}

/// cfprefs mirrors managed preferences into the app's HomeDomain plist. Used by
/// upstream for iOS 27+ device writes; legacy draft targets never hit it, the
/// rule stays ported so future callers cannot drift.
pub fn managed_preference_mirror(path: &str) -> Option<String> {
    const PREFIX: &str = "/var/Managed Preferences/";
    let name = path.strip_prefix(PREFIX)?;
    let name = name.strip_prefix("mobile/").unwrap_or(name);
    if !name.ends_with(".plist") {
        return None;
    }
    Some(format!("Library/Preferences/{name}"))
}

fn plist_value(value: &TweakValue) -> PlistValue {
    match value {
        TweakValue::Bool(inner) => PlistValue::Boolean(*inner),
        TweakValue::Integer(inner) => PlistValue::from(*inner),
        TweakValue::Double(inner) => PlistValue::Real(*inner),
        TweakValue::Text(inner) => PlistValue::String(inner.clone()),
    }
}

/// `plistlib.dumps(plist)` XML output — one plist writer for all draft files.
fn serialise(dict: &PlistDict) -> Result<String, DraftError> {
    let mut bytes = Vec::new();
    plist::to_writer_xml(&mut bytes, dict).map_err(|_| DraftError::SerializationFailed)?;
    String::from_utf8(bytes).map_err(|_| DraftError::SerializationFailed)
}

/// One drafted entry as the upstream selection would carry it.
#[derive(Debug)]
struct Drafted<'a> {
    spec: &'a Spec,
    /// `selection.isOn(spec)`.
    on: bool,
    /// The stored value the selection carries, if any.
    stored: Option<TweakValue>,
}

/// Serialize only explicit choices; never emit all reversible defaults. Every
/// plist below is a NEW dictionary proposal, not a merge with a device
/// backup, and nothing here is delivered to a device.
pub fn preview(
    choices: &BTreeMap<String, TweakValue>,
    device: &Device,
) -> Result<DraftPreview, DraftError> {
    if choices.is_empty() {
        return Err(DraftError::EmptySelection);
    }
    if choices.len() > crate::PREVIEW_LIMIT {
        return Err(DraftError::TooManySelections);
    }
    let all = &catalog().specs;
    for id in choices.keys() {
        if !all.iter().any(|spec| spec.id == *id) {
            return Err(DraftError::UnknownTweak);
        }
    }

    let mut drafted: Vec<Drafted> = Vec::new();
    let mut writes: BTreeMap<String, TweakValue> = BTreeMap::new();
    let mut staged: BTreeSet<String> = BTreeSet::new();

    for spec in all {
        let Some(value) = choices.get(&spec.id) else {
            continue;
        };
        if status(spec, device) != Status::DraftCandidate {
            return Err(DraftError::BlockedTweak);
        }
        if device_status(device).is_some() {
            return Err(DraftError::BlockedTweak);
        }
        // A dict-shaped entry may only ride its upstream dictionary.
        if spec.writes_whole_dict() && *value != spec.value {
            return Err(DraftError::InvalidValue);
        }
        // Exclusive pairs, even when the metadata would still round-trip.
        if let (TweakValue::Bool(true), Some(peer)) = (value, &spec.exclusive_with) {
            if matches!(choices.get(peer.as_str()), Some(TweakValue::Bool(true))) {
                return Err(DraftError::Conflict(format!(
                    "{} и {} взаимоисключающие.",
                    spec.id, peer
                )));
            }
        }
        match (&spec.kind, value) {
            (Kind::Toggle, TweakValue::Bool(enabled)) => {
                // `TweakSelection.setOn` for off-path switches, `setValue`
                // otherwise — an explicit false stays an explicit key write.
                if let Some(off) = &spec.off_value {
                    if *enabled {
                        writes.insert(spec.id.clone(), spec.value.clone());
                        staged.insert(spec.id.clone());
                    } else {
                        writes.insert(spec.id.clone(), off.clone());
                        staged.remove(&spec.id);
                    }
                } else if spec.reversible_boolean {
                    // No stored value; the state alone decides what is written.
                    if *enabled {
                        staged.insert(spec.id.clone());
                    } else {
                        staged.remove(&spec.id);
                    }
                } else {
                    writes.insert(spec.id.clone(), value.clone());
                    staged.insert(spec.id.clone());
                }
            }
            (Kind::Text, TweakValue::Text(text)) => {
                if text.len() > crate::MAX_TEXT_BYTES
                    || text.contains('\0')
                    || spec.writes_whole_dict()
                {
                    return Err(DraftError::InvalidValue);
                }
                writes.insert(spec.id.clone(), value.clone());
                staged.insert(spec.id.clone());
            }
            (Kind::Number, TweakValue::Integer(number)) => {
                // The registry's own type decides `1` vs `1.0`; no coercion.
                if !matches!(spec.value, TweakValue::Integer(_))
                    || (*number as f64) < spec.min_value
                    || (*number as f64) > spec.max_value
                {
                    return Err(DraftError::InvalidValue);
                }
                writes.insert(spec.id.clone(), value.clone());
                staged.insert(spec.id.clone());
            }
            (Kind::Number, TweakValue::Double(number)) => {
                if !matches!(spec.value, TweakValue::Double(_))
                    || !number.is_finite()
                    || *number < spec.min_value
                    || *number > spec.max_value
                {
                    return Err(DraftError::InvalidValue);
                }
                writes.insert(spec.id.clone(), value.clone());
                staged.insert(spec.id.clone());
            }
            _ => return Err(DraftError::InvalidValue),
        }
        drafted.push(Drafted {
            spec,
            on: staged.contains(&spec.id),
            stored: writes.get(&spec.id).cloned(),
        });
    }

    // `TweakSelection.validationIssue`, restricted to the drafted registry rows.
    let title = |id: &str| {
        catalog()
            .spec(id)
            .map(|spec| spec.title.clone())
            .unwrap_or_else(|| id.to_string())
    };
    for entry in &drafted {
        let spec = entry.spec;
        if let Some(reason) = &spec.unavailable_reason {
            return Err(DraftError::Conflict(format!("{}: {}", spec.title, reason)));
        }
        if let Some(peer) = &spec.exclusive_with {
            if staged.contains(peer) {
                return Err(DraftError::Conflict(format!(
                    "{} cannot be combined with {}.",
                    spec.title,
                    title(peer)
                )));
            }
        }
        let chosen = entry.stored.as_ref().unwrap_or(&spec.value);
        if !matches!(spec.kind, Kind::Number) || chosen != &spec.value {
            for required in &spec.requires {
                if !staged.contains(required) {
                    return Err(DraftError::Conflict(format!(
                        "Enable {} before applying {}.",
                        title(required),
                        spec.title
                    )));
                }
            }
        }
        if spec.factory == "OptionalBundlePreferenceTweak" {
            let text = chosen.display();
            if text.chars().count() > 255 || !valid_bundle_identifier(&text) {
                return Err(DraftError::Conflict(format!(
                    "Enter a valid app bundle identifier for {}, or clear the field.",
                    spec.title
                )));
            }
        }
    }

    if let (Some(TweakValue::Bool(true)), Some(TweakValue::Bool(true))) =
        (choices.get("RTL"), choices.get("LTR"))
    {
        return Err(DraftError::Conflict(
            "RTL и LTR нельзя включать одновременно.".into(),
        ));
    }

    let files = compile(&drafted, &writes, device)?;
    Ok(DraftPreview::new(files, drafted.len()))
}

/// Upstream `OptionalBundlePreferenceTweak` validation: empty, or a lowercase
/// ASCII bundle id of at least two dot-separated parts.
fn valid_bundle_identifier(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

/// Insertion-ordered per-location dictionary, like the reference's Python dict:
/// files reach the backup in first-write order and a later spec still wins a
/// shared key.
fn location<'t>(path: &str, tables: &'t mut Vec<(String, PlistDict)>) -> &'t mut PlistDict {
    if let Some(index) = tables.iter().position(|(existing, _)| existing == path) {
        return &mut tables[index].1;
    }
    tables.push((path.to_string(), PlistDict::new()));
    &mut tables.last_mut().expect("just pushed").1
}

/// The compile half of upstream `_apply_tweak_pass`, restricted to drafted
/// specs: an empty per-location table, merge every selected key in, serialise
/// each location once, map to a backup domain.
fn compile(
    drafted: &[Drafted],
    writes: &BTreeMap<String, TweakValue>,
    device: &Device,
) -> Result<Vec<PlistFile>, DraftError> {
    let mut skipped: Vec<(&str, &str)> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut tables: Vec<(String, PlistDict)> = Vec::new();

    for entry in drafted {
        let spec = entry.spec;
        // Only catalog rows reach here: launchd groups, nullify and
        // NSKeyedArchive factories never become draft candidates.
        if spec.writes_whole_dict() {
            let dict = location(&spec.path, &mut tables);
            // Merged, not assigned: several dict writers share one file.
            for (key, value) in spec.multi_values.as_ref().expect("dict shape") {
                dict.insert(key.clone(), plist_value(value));
            }
            continue;
        }
        if spec.key.is_empty() {
            skipped.push((&spec.path, "registry entry has no plist key"));
            continue;
        }
        let dict = location(&spec.path, &mut tables);
        let chosen = writes.get(&spec.id).unwrap_or(&spec.value);
        if let Some(off) = &spec.off_value {
            dict.insert(
                spec.key.clone(),
                plist_value(if entry.on { &spec.value } else { off }),
            );
        } else if spec.reversible_boolean {
            if spec.integer_boolean {
                dict.insert(
                    spec.key.clone(),
                    PlistValue::from(if entry.on { 1 } else { 0 }),
                );
            } else {
                dict.insert(spec.key.clone(), PlistValue::Boolean(entry.on));
            }
        } else if (spec.factory == "ResettableNumericPreferenceTweak" && chosen.display() == "-1.0")
            || (spec.factory == "OptionalBundlePreferenceTweak" && chosen.display().is_empty())
        {
            // Both factories mean "remove the key". Unreachable for the drafted
            // spec set (these factories never become draft candidates); ported
            // so a future factory change cannot drift silently.
            dict.remove(&spec.key);
        } else {
            dict.insert(spec.key.clone(), plist_value(chosen));
        }
    }
    for (path, _) in &tables {
        order.push(path.clone());
    }

    let mut files = Vec::new();
    for device_path in &order {
        let dict = &tables
            .iter()
            .find(|(path, _)| path == device_path)
            .expect("ordered above")
            .1;
        let Some((domain, relative)) = split_domain(device_path) else {
            skipped.push((device_path, "no backup-domain prefix matches this location"));
            continue;
        };
        files.push(PlistFile {
            domain,
            relative_path: relative,
            xml: serialise(dict)?,
        });
        // Upstream mirrors managed preferences into HomeDomain for iOS 27+ so a
        // Phase-3 restore keeps values visible. Draft targets are 15–17, so
        // this arm stays intentionally unreachable for draftable devices.
        if version::compare(&device.system_version, "27")
            .is_some_and(|ordering| ordering != Ordering::Less)
            && device_path != GLOBAL_PREFERENCES
        {
            if let Some(mirror) = managed_preference_mirror(device_path) {
                files.push(PlistFile {
                    domain: "HomeDomain".into(),
                    relative_path: mirror,
                    xml: serialise(dict)?,
                });
            }
        }
    }

    // The GP dictionary mirrors into HomeDomain whenever something was written,
    // so tweaks that read it through the NSGlobalDomain search chain keep
    // working. Deliberate divergence from upstream, carried over from the Swift
    // port: an EMPTY dict never overwrites a live file.
    if let Some((_, dict)) = tables.iter().find(|(path, _)| path == GLOBAL_PREFERENCES) {
        if !dict.is_empty() {
            let (domain, relative) =
                split_domain(GLOBAL_PREFERENCES_HOME).ok_or(DraftError::SerializationFailed)?;
            files.push(PlistFile {
                domain,
                relative_path: relative,
                xml: serialise(dict)?,
            });
        }
    }

    if !skipped.is_empty() || files.is_empty() {
        return Err(DraftError::SerializationFailed);
    }
    Ok(files)
}
