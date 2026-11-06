//! Stage-2 plan tests: resolve/diff behaviour over real-shaped fixtures
//! (see tests/common/mod.rs). Nothing here touches a device.

mod common;

use apply_channel::plan::plan_backup;
use common::*;
use plist::Value;

#[test]
fn plan_adds_missing_key_and_verifies_digest() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = plan_backup(&fixture.backing(), &path, &fixture.out("plan-1.json")).unwrap();
    assert_eq!(report.inputs, 1);
    assert_eq!(report.checksum_ok, 1);
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].file_id, LOWER_ID);
    assert!(report.files[0].digest_verified);
    let entry = &report.files[0].entries[0];
    assert_eq!(entry.action, "add_key");
    assert_eq!(entry.old, None);
    assert_eq!(entry.new, "true");
    let written = std::fs::read_to_string(fixture.out("plan-1.json")).unwrap();
    assert!(written.contains("add_key"));
}

#[test]
fn plan_replaces_existing_value() {
    let content = sample_plist(&[(KEY, Value::Boolean(false))]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = plan_backup(&fixture.backing(), &path, &fixture.out("plan-2.json")).unwrap();
    let entry = &report.files[0].entries[0];
    assert!(entry.old.as_deref() == Some("false"), "old value recorded");
    assert_eq!(entry.action, "replace_value");
    assert_eq!(entry.new, "true");
}

#[test]
fn plan_refuses_corrupt_content() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, false)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("plan-3.json")).unwrap_err();
    assert_eq!(error.code(), "plan_digest_mismatch");
}

#[test]
fn plan_refuses_truncated_bucket_file() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content.clone(), true)]);
    let bucket = fixture.dir.path().join(UDID).join("66").join(LOWER_ID);
    let mut shorter = content;
    shorter.truncate(shorter.len() - 4);
    std::fs::write(&bucket, shorter).unwrap();
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("plan-4.json")).unwrap_err();
    assert_eq!(error.code(), "plan_digest_mismatch");
}

#[test]
fn plan_refuses_case_duplicate_targets() {
    let content = sample_plist(&[]);
    let stub = sample_plist(&[]);
    let fixture = Fixture::new(vec![
        (LOWER_ID, LOWER_PATH, content, true),
        (UPPER_ID, UPPER_PATH, stub, true),
    ]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("plan-5.json")).unwrap_err();
    assert_eq!(error.code(), "plan_ambiguous_target");
}

#[test]
fn plan_proceeds_when_case_variants_explicitly_accepted() {
    let content = sample_plist(&[(KEY, Value::Boolean(false))]);
    let stub = sample_plist(&[]);
    let fixture = Fixture::new(vec![
        (LOWER_ID, LOWER_PATH, content, true),
        (UPPER_ID, UPPER_PATH, stub, true),
    ]);
    let path = fixture.draft(&wrap(&toggle_item_case_accepted(LOWER_PATH, true)));
    let report = plan_backup(&fixture.backing(), &path, &fixture.out("plan-5b.json")).unwrap();
    assert_eq!(report.files[0].case_variants, 2, "sibling reported");
    assert_eq!(report.files[0].file_id, LOWER_ID, "exact case chosen");
    assert_eq!(report.files[0].entries[0].action, "replace_value");
}

#[test]
fn plan_refuses_missing_manifest_entry() {
    let fixture = Fixture::new(vec![]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("plan-6.json")).unwrap_err();
    assert_eq!(error.code(), "plan_item_not_found");
}

#[test]
fn plan_refuses_bad_and_empty_drafts() {
    let fixture = Fixture::new(vec![]);
    let broken = fixture.draft("not json");
    let error = plan_backup(&fixture.backing(), &broken, &fixture.out("p7.json")).unwrap_err();
    assert_eq!(error.code(), "plan_bad_draft");

    let empty = fixture.draft("{\"items\":[]}");
    let error = plan_backup(&fixture.backing(), &empty, &fixture.out("p8.json")).unwrap_err();
    assert_eq!(error.code(), "plan_bad_draft");
}

#[test]
fn plan_refuses_kind_mismatch() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let item = format!(
        "{{\"id\":\"t\",\"kind\":\"toggle\",\"key\":\"K\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{LOWER_PATH}\",\"value\":7}}"
    );
    let path = fixture.draft(&wrap(&item));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("p9.json")).unwrap_err();
    assert_eq!(error.code(), "plan_unsupported_kind");
}

#[test]
fn plan_never_overwrites_existing_output() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let out = fixture.out("twice.json");
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    plan_backup(&fixture.backing(), &path, &out).unwrap();
    let error = plan_backup(&fixture.backing(), &path, &out).unwrap_err();
    assert_eq!(error.code(), "plan_output_exists");
}

#[test]
fn plan_numbers_work() {
    let content = sample_plist(&[(KEY, Value::Real(1.0))]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let item = format!(
        "{{\"id\":\"n\",\"kind\":\"number\",\"key\":\"{KEY}\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{LOWER_PATH}\",\"value\":3.5}}"
    );
    let path = fixture.draft(&wrap(&item));
    let report = plan_backup(&fixture.backing(), &path, &fixture.out("p10.json")).unwrap();
    let entry = &report.files[0].entries[0];
    assert_eq!(entry.action, "replace_value");
    assert_eq!(entry.old.as_deref(), Some("1"));
    assert_eq!(entry.new, "3.5");
}
