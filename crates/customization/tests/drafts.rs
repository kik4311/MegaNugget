//! Integration tests for the offline draft compiler, mirroring the Swift
//! `LegacyTweakPortTests` so host and iOS adapters stay behaviorally identical.

use customization::{
    catalog, compiler, summarize, version, Device, DraftError, Status, Summary, TweakValue,
    REGISTRY_COUNT,
};
use std::collections::BTreeMap;

fn iphone7() -> Device {
    Device::new("iPhone9,1", "15.8.8", "arm64")
}

fn entries(device: &Device) -> Vec<Summary> {
    summarize(device)
}

fn entry<'a>(id: &str, all: &'a [Summary]) -> &'a Summary {
    all.iter()
        .find(|entry| entry.id == id)
        .unwrap_or_else(|| panic!("{id} missing from snapshot"))
}

fn draft(choices: &[(&str, TweakValue)]) -> BTreeMap<String, TweakValue> {
    choices
        .iter()
        .map(|(id, value)| ((*id).to_string(), value.clone()))
        .collect()
}

fn dictionary(file: &compiler::PlistFile) -> plist::Dictionary {
    match plist::from_bytes::<plist::Value>(file.xml.as_bytes()).expect("XML plist parses") {
        plist::Value::Dictionary(dict) => dict,
        other => panic!("expected a dictionary, got {other:?}"),
    }
}

#[test]
fn imported_catalog_and_no_apply_claims() {
    let all = entries(&iphone7());
    assert_eq!(all.len(), REGISTRY_COUNT);
    let unique = all
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<std::collections::BTreeSet<&str>>();
    assert_eq!(unique.len(), all.len(), "no duplicate ids");
    assert_eq!(
        entry("SBHideLowPowerAlerts", &all).status,
        Status::DraftCandidate
    );
    assert_eq!(
        entry("SBAlwaysShowSystemApertureInSnapshots", &all).status,
        Status::RequiresNewerIos
    );
    // Daemon-group and ScreenTime rows are not part of the host snapshot.
    assert!(catalog().spec("Daemon.AirDrop").is_none());
    assert!(catalog().spec("Daemons").is_none());
}

#[test]
fn ios26_27_minimums_never_draft_on_legacy() {
    let all = entries(&iphone7());
    let newer: Vec<&Summary> = all
        .iter()
        .filter(|entry| {
            let min = entry.minimum_ios.as_deref().unwrap_or("");
            (min.starts_with("26") || min.starts_with("27"))
                && entry.status == Status::DraftCandidate
        })
        .collect();
    assert!(newer.is_empty(), "registry bounds must block new-iOS rows");

    let mut choices = BTreeMap::new();
    choices.insert("SiriAI".to_string(), TweakValue::Bool(true));
    assert_eq!(
        compiler::preview(&choices, &iphone7()).err(),
        Some(DraftError::BlockedTweak)
    );
}

#[test]
fn strict_versions_and_unknown_devices_fail_closed() {
    for malformed in [
        "",
        "15..8",
        "15.8.8.1",
        " 15.8.8",
        "15.8.8 ",
        "15.8b",
        "+15.8",
        "99999999999999999999999999999",
    ] {
        assert_eq!(version::compare(malformed, "15.8.8"), None, "{malformed}");
        let device = Device::new("iPhone9,1", malformed, "arm64");
        assert!(
            entries(&device)
                .iter()
                .all(|entry| entry.status != Status::DraftCandidate),
            "malformed device version must not unlock the catalog"
        );
        let mut choices = BTreeMap::new();
        choices.insert("SBHideLowPowerAlerts".to_string(), TweakValue::Bool(true));
        assert_eq!(
            compiler::preview(&choices, &device).err(),
            Some(DraftError::BlockedTweak),
            "{malformed}"
        );
    }
    let unknown = Device::new("iPhone99,1", "17.4", "arm64");
    assert!(entries(&unknown)
        .iter()
        .all(|entry| entry.status != Status::DraftCandidate));
    let mut simulator = Device::new("iPhone9,1", "15.8.8", "arm64");
    simulator.is_simulator = true;
    assert!(entries(&simulator)
        .iter()
        .all(|entry| entry.status != Status::DraftCandidate));
    let wrong_arch = Device::new("iPhone9,1", "15.8.8", "x86_64");
    assert!(entries(&wrong_arch)
        .iter()
        .all(|entry| entry.status == Status::InvalidDevice));
}

#[test]
fn os_and_hardware_are_separate_gates() {
    let impossible = entries(&Device::new("iPhone9,1", "17.4", "arm64"));
    assert_eq!(
        entry("SBHideLowPowerAlerts", &impossible).status,
        Status::HardwareMismatch
    );
    let iphone_x = entries(&Device::new("iPhone10,6", "16.7.1", "arm64"));
    assert_eq!(
        entry("SBHideLowPowerAlerts", &iphone_x).status,
        Status::DraftCandidate
    );

    let modern = entries(&Device::new("iPhone15,2", "17.4", "arm64"));
    assert_eq!(
        entry("HideDICompletely", &modern).status,
        Status::DraftCandidate
    );
    let no_island = entries(&Device::new("iPhone13,2", "17.4", "arm64"));
    assert_eq!(
        entry("HideDICompletely", &no_island).status,
        Status::HardwareMismatch
    );

    let outside = entries(&Device::new("iPhone15,2", "18.0", "arm64"));
    assert_eq!(
        entry("SBHideLowPowerAlerts", &outside).status,
        Status::OutsideLegacyScope
    );
}

#[test]
fn only_explicit_choices_merge_and_nothing_applies() {
    let preview = compiler::preview(
        &draft(&[
            ("SBHideLowPowerAlerts", TweakValue::Bool(true)),
            ("SBDontDimOrLockOnAC", TweakValue::Bool(false)),
        ]),
        &iphone7(),
    )
    .expect("both are draft candidates");
    assert_eq!(preview.selected_count, 2);
    assert_eq!(preview.files.len(), 1);
    let file = &preview.files[0];
    assert_eq!(file.domain, "ManagedPreferencesDomain");
    assert_eq!(file.relative_path, "mobile/com.apple.springboard.plist");
    let dict = dictionary(file);
    assert_eq!(dict.keys().count(), 2, "only the two chosen keys");
    assert!(!preview.applied_to_device, "drafts are never applied");
    assert_eq!(
        dict.get("SBHideLowPowerAlerts"),
        Some(&plist::Value::from(true))
    );
    assert_eq!(
        dict.get("SBDontDimOrLockOnAC"),
        Some(&plist::Value::from(false))
    );
}

#[test]
fn plist_types_and_text_escaping_are_preserved() {
    let preview = compiler::preview(
        &draft(&[("SBMinimumLockscreenIdleTime", TweakValue::Integer(42))]),
        &iphone7(),
    )
    .expect("int within bounds");
    assert!(
        preview.files[0].xml.contains("<integer>42</integer>"),
        "int stays an integer, not a double: {}",
        preview.files[0].xml
    );
    let dict = dictionary(&preview.files[0]);
    assert_eq!(
        dict.get("SBMinimumLockscreenIdleTime"),
        Some(&plist::Value::from(42i64))
    );

    let text = "Mega <Nugget> & iPhone";
    let preview = compiler::preview(
        &draft(&[("LockScreenFootnote", TweakValue::Text(text.to_string()))]),
        &iphone7(),
    )
    .expect("text accepted");
    let dict = dictionary(&preview.files[0]);
    assert_eq!(
        dict.get("LockScreenFootnote"),
        Some(&plist::Value::from(text)),
        "exact round trip through escaped XML"
    );
    // The footnote location lives in a configuration-profile system group.
    assert_eq!(
        preview.files[0].domain,
        "SysSharedContainerDomain-systemgroup.com.apple.configurationprofiles"
    );
}

#[test]
fn invalid_values_are_rejected_with_stable_codes() {
    let cases: Vec<(BTreeMap<String, TweakValue>, &str)> = vec![
        (
            draft(&[("SBMinimumLockscreenIdleTime", TweakValue::Integer(601))]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[(
                "SBMinimumLockscreenIdleTime",
                TweakValue::Double(f64::INFINITY),
            )]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[("SBMinimumLockscreenIdleTime", TweakValue::Double(1.5))]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[("SBHideLowPowerAlerts", TweakValue::Text("true".into()))]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[("LockScreenFootnote", TweakValue::Text("x".repeat(1025)))]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[("LockScreenFootnote", TweakValue::Text("bad\0text".into()))]),
            "tweaks_invalid_value",
        ),
        (
            draft(&[("not_a_tweak", TweakValue::Bool(true))]),
            "tweaks_unknown_tweak",
        ),
    ];
    for (choices, expected) in cases {
        assert_eq!(
            compiler::preview(&choices, &iphone7())
                .err()
                .unwrap()
                .code(),
            expected,
            "{:?}",
            choices.keys().next()
        );
    }
    assert_eq!(
        compiler::preview(&BTreeMap::new(), &iphone7())
            .err()
            .unwrap()
            .code(),
        "tweaks_empty_selection"
    );
    let overflow: BTreeMap<String, TweakValue> = (0..65)
        .map(|index| (format!("id{index}"), TweakValue::Bool(true)))
        .collect();
    assert_eq!(
        compiler::preview(&overflow, &iphone7())
            .err()
            .unwrap()
            .code(),
        "tweaks_too_many_selections"
    );
}

#[test]
fn layout_direction_exclusivity_is_enforced() {
    let preview = compiler::preview(&draft(&[("RTL", TweakValue::Bool(true))]), &iphone7());
    assert!(preview.is_ok(), "RTL alone is a candidate: {:?}", preview);
    let preview = compiler::preview(
        &draft(&[
            ("RTL", TweakValue::Bool(true)),
            ("LTR", TweakValue::Bool(true)),
        ]),
        &iphone7(),
    );
    assert_eq!(preview.err().unwrap().code(), "tweaks_conflict");
}

#[test]
fn global_preferences_draft_gains_home_domain_mirror() {
    let preview = compiler::preview(&draft(&[("LTR", TweakValue::Bool(true))]), &iphone7())
        .expect("GP key compiles");
    let domains: Vec<(&str, &str)> = preview
        .files
        .iter()
        .map(|file| (file.domain.as_str(), file.relative_path.as_str()))
        .collect();
    assert!(
        domains.contains(&(
            "ManagedPreferencesDomain",
            "mobile/.GlobalPreferences.plist"
        )),
        "primary managed write: {domains:?}"
    );
    assert!(
        domains.contains(&("HomeDomain", "Library/Preferences/.GlobalPreferences.plist")),
        "GP mirror is precise, never empty: {domains:?}"
    );
}
