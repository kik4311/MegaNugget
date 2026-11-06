//! Stage-3 merge tests: a resolved draft is merged into a fresh hardlink copy
//! of a real-shaped fixture backup. Every test must keep the source backup
//! byte-identical and leave no half-built copy behind on refusal.

mod common;

use apply_channel::merge::merge_into_copy;
use apply_channel::plan::plan_backup;
use common::*;
use plist::Value;
use sha1::{Digest, Sha1};

/// Hash every file of the fixture's device directory (sorted by path so the
/// comparison is stable).
fn hash_tree(dir: &std::path::Path) -> std::collections::BTreeMap<String, [u8; 20]> {
    let mut hashes = std::collections::BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("read_dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let bytes = std::fs::read(&path).expect("read file");
            let mut hasher = Sha1::new();
            hasher.update(&bytes);
            let digest: [u8; 20] = hasher.finalize().into();
            hashes.insert(path.display().to_string(), digest);
        }
    }
    hashes
}

#[test]
fn merge_writes_patched_plist_and_manifest_into_a_copy() {
    let content = sample_plist(&[(KEY, Value::Boolean(false))]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));

    let applied = fixture.applied_target("applied");
    let report = merge_into_copy(&fixture.backing(), &path, &applied).unwrap();

    assert_eq!(report.files.len(), 1);
    let merged = &report.files[0];
    assert_eq!(merged.domain, DOMAIN);
    assert_eq!(merged.relative_path, LOWER_PATH);
    assert_eq!(merged.file_id, LOWER_ID);
    assert_eq!(merged.keys_patched, vec![format!("{KEY}=true")]);

    // The new bucket file in the copy carries the key with the planned value.
    let applied_device = applied.join(UDID);
    let bucket = applied_device.join("66").join(LOWER_ID);
    let patched = std::fs::read(&bucket).unwrap();
    let value: Value = plist::from_bytes(&patched).unwrap();
    let dict = value.into_dictionary().unwrap();
    assert_eq!(dict.get(KEY), Some(&Value::Boolean(true)));

    // The copy's manifest row agrees with the new content (digest + size).
    let manifest =
        apply_channel::manifest::Manifest::open(&applied_device.join("Manifest.db")).unwrap();
    let entry = manifest
        .find(DOMAIN, LOWER_PATH)
        .unwrap()
        .expect("row present");
    let metadata = apply_channel::manifest::file_metadata(&entry.blob).expect("blob parses");
    let mut hasher = sha1::Sha1::new();
    hasher.update(&patched);
    assert_eq!(metadata.digest, hasher.finalize().to_vec());
    assert_eq!(metadata.size, patched.len() as u64);
}

#[test]
fn merge_keeps_source_backup_byte_identical() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let before = hash_tree(&fixture.dir.path().join(UDID));

    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let applied = fixture.applied_target("applied");
    merge_into_copy(&fixture.backing(), &path, &applied).unwrap();

    let after = hash_tree(&fixture.dir.path().join(UDID));
    assert_eq!(before, after, "merge must not touch the source backup");
}

#[test]
fn merge_copy_shares_inodes_except_patched_targets() {
    if !cfg!(unix) {
        return;
    }
    use std::os::unix::fs::MetadataExt;

    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let applied = fixture.applied_target("applied");
    merge_into_copy(&fixture.backing(), &path, &applied).unwrap();

    // Shared inode: hardlinked files have the same inode number in copy and
    // source, and the copy's link count grew by one.
    let shared_source = fixture.dir.path().join(UDID).join("Status.plist");
    let shared_copy = applied.join(UDID).join("Status.plist");
    assert_eq!(
        std::fs::metadata(&shared_source).unwrap().ino(),
        std::fs::metadata(&shared_copy).unwrap().ino()
    );
    assert!(std::fs::metadata(&shared_copy).unwrap().nlink() >= 2);

    // The patched target is a fresh file (new inode, single link).
    let target_source = fixture.dir.path().join(UDID).join("66").join(LOWER_ID);
    let target_copy = applied.join(UDID).join("66").join(LOWER_ID);
    assert_ne!(
        std::fs::metadata(&target_source).unwrap().ino(),
        std::fs::metadata(&target_copy).unwrap().ino()
    );
    assert_eq!(std::fs::metadata(&target_copy).unwrap().nlink(), 1);

    // The SQLite manifest family is copied, not hardlinked (WAL mode).
    let manifest_source = fixture.dir.path().join(UDID).join("Manifest.db");
    let manifest_copy = applied.join(UDID).join("Manifest.db");
    assert_ne!(
        std::fs::metadata(&manifest_source).unwrap().ino(),
        std::fs::metadata(&manifest_copy).unwrap().ino()
    );
    assert_eq!(std::fs::metadata(&manifest_source).unwrap().nlink(), 1);
}

#[test]
fn merge_refuses_existing_output_directory() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let applied = fixture.applied_target("applied");
    std::fs::create_dir_all(&applied).unwrap();
    std::fs::write(applied.join("marker"), b"keep").unwrap();

    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = merge_into_copy(&fixture.backing(), &path, &applied).unwrap_err();
    assert_eq!(error.code(), "merge_output_exists");
    assert_eq!(
        std::fs::read(applied.join("marker")).unwrap(),
        b"keep".to_vec(),
        "existing output is never touched"
    );
}

#[test]
fn merge_refuses_output_inside_the_source_backup() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let nested = fixture.backing().join("applied");
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = merge_into_copy(&fixture.backing(), &path, &nested).unwrap_err();
    assert_eq!(error.code(), "merge_output_inside_source");
    assert!(!nested.exists(), "refusal must not create anything");
}

#[test]
fn merge_refusal_on_bad_draft_creates_nothing() {
    let fixture = Fixture::new(vec![]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let applied = fixture.applied_target("applied");
    let error = merge_into_copy(&fixture.backing(), &path, &applied).unwrap_err();
    assert_eq!(error.code(), "plan_item_not_found");
    assert!(!applied.exists(), "no half-built copy may remain");
}

#[test]
fn merge_refusal_on_corrupt_bucket_creates_nothing() {
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, false)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let applied = fixture.applied_target("applied");
    let error = merge_into_copy(&fixture.backing(), &path, &applied).unwrap_err();
    assert_eq!(error.code(), "plan_digest_mismatch");
    assert!(!applied.exists());
}

#[test]
fn merged_copy_passes_plan_again() {
    // The copy must behave like a normal backup: planning the draft against
    // it resolves the same entry and sees the already-applied value.
    let content = sample_plist(&[]);
    let fixture = Fixture::new(vec![(LOWER_ID, LOWER_PATH, content, true)]);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let applied = fixture.applied_target("applied");
    merge_into_copy(&fixture.backing(), &path, &applied).unwrap();

    let again = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let report = plan_backup(&applied, &again, &fixture.out("plan-after.json")).unwrap();
    let entry = &report.files[0].entries[0];
    assert_eq!(entry.action, "replace_value");
    assert_eq!(
        entry.old.as_deref(),
        Some("true"),
        "the merged value is visible"
    );
}
