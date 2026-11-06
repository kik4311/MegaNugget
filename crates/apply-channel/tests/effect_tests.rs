//! Stage-3 effect tests: the per-key comparison between a pre-apply and a
//! post-restore backup. The "post" side is any fixture backup whose content
//! stands in for a fresh backup made after a restore.

mod common;

use apply_channel::effect::effect;
use common::*;
use plist::Value;

#[test]
fn effect_reports_key_as_matched_after_merge() {
    let content = sample_plist(&[(KEY, Value::Boolean(false))]);
    let pre = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);

    // The post fixture already carries the planned value — exactly what a
    // fresh backup after a successful restore looks like.
    let post_content = sample_plist(&[(KEY, Value::Boolean(true))]);
    let post = Fixture::new(vec![(LOWER_ID, LOWER_PATH, post_content, true)]);

    let draft = pre.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = effect(&draft, &pre.backing(), &post.backing()).unwrap();

    assert_eq!(report.files.len(), 1);
    let key = &report.files[0].keys[0];
    assert_eq!(key.id, "t1");
    assert_eq!(key.key, KEY);
    assert_eq!(key.status, "matched");
    assert_eq!(key.before.as_deref(), Some("false"));
    assert_eq!(key.planned, "true");
    assert_eq!(key.after.as_deref(), Some("true"));
}

#[test]
fn effect_reports_added_key_as_matched() {
    // Key absent before (add_key case), present after.
    let content = sample_plist(&[]);
    let pre = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let post_content = sample_plist(&[(KEY, Value::Boolean(true))]);
    let post = Fixture::new(vec![(LOWER_ID, LOWER_PATH, post_content, true)]);

    let draft = pre.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = effect(&draft, &pre.backing(), &post.backing()).unwrap();
    let key = &report.files[0].keys[0];
    assert_eq!(key.status, "matched");
    assert_eq!(key.before, None);
    assert_eq!(key.after.as_deref(), Some("true"));
}

#[test]
fn effect_reports_missing_key_after_restore() {
    let content = sample_plist(&[]);
    let pre = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let post = Fixture::new(vec![(LOWER_ID, LOWER_PATH, sample_plist(&[]), true)]);

    let draft = pre.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = effect(&draft, &pre.backing(), &post.backing()).unwrap();
    let key = &report.files[0].keys[0];
    assert_eq!(key.status, "missing");
    assert_eq!(key.after, None);
}

#[test]
fn effect_reports_changed_value_after_restore() {
    let content = sample_plist(&[]);
    let pre = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let post = Fixture::new(vec![(
        LOWER_ID,
        LOWER_PATH,
        sample_plist(&[(KEY, Value::Boolean(false))]),
        true,
    )]);

    let draft = pre.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = effect(&draft, &pre.backing(), &post.backing()).unwrap();
    let key = &report.files[0].keys[0];
    assert_eq!(key.status, "changed");
    assert_eq!(key.after.as_deref(), Some("false"));
}

#[test]
fn effect_refuses_unverified_backups() {
    let fixture = Fixture::new(vec![]);
    let draft = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let empty = tempfile::tempdir().unwrap();
    let error = effect(&draft, empty.path(), &fixture.backing()).unwrap_err();
    assert_eq!(error.code(), "backup_layout_unsupported");

    let error = effect(&draft, &fixture.backing(), empty.path()).unwrap_err();
    assert_eq!(error.code(), "backup_layout_unsupported");
}

#[test]
fn effect_refuses_drifted_targets() {
    // Pre and post disagree about where the draft target lives: refuse.
    let content = sample_plist(&[]);
    let pre = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let post = Fixture::new(vec![(UPPER_ID, UPPER_PATH, sample_plist(&[]), true)]);
    let draft = pre.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = effect(&draft, &pre.backing(), &post.backing()).unwrap_err();
    assert_eq!(error.code(), "plan_item_not_found");
}
