//! Stage-2 plan tests: fixtures emulate a real idevicebackup2 backup layout
//! (UDID dir, Manifest.db + hash buckets) and assert plan behaviour without
//! touching any device.

use apply_channel::plan::plan_backup;
use apply_channel::PRIMARY_TARGET_IOS;
use plist::{Dictionary, Value};
use sha1::{Digest, Sha1};
use std::path::PathBuf;

const UDID: &str = "0123456789012345678901234567890123456789";
const DOMAIN: &str = "HomeDomain";
const LOWER_PATH: &str = "Library/Preferences/com.apple.springboard.plist";
const UPPER_PATH: &str = "Library/Preferences/com.apple.SpringBoard.plist";
const LOWER_ID: &str = "662bc19b13aecef58a7e855d0316e4cf61e2642b";
const UPPER_ID: &str = "22e3b6f23409a50b22b33df1881b50265ea10da3";
const KEY: &str = "SBHideLowPowerAlerts";

/// A plist that already carries one known key plus padding; also used raw as
/// a springboard stub.
fn sample_plist(extra: &[(&str, Value)]) -> Vec<u8> {
    let mut dict = Dictionary::new();
    dict.insert("SBUserGuidedMode".into(), Value::Boolean(false));
    dict.insert("SBShowBatteryPercentage".into(), Value::Boolean(true));
    for (key, value) in extra {
        dict.insert((*key).into(), value.clone());
    }
    let mut out = Vec::new();
    plist::to_writer_binary(&mut out, &dict).expect("bplist encode");
    out
}

/// NSKeyedArchiver-shaped metadata blob as written by the device tooling.
fn keyed_blob(relative_path: &str, content: &[u8], digest: &[u8]) -> Value {
    use plist::Integer;
    let int = |value: i64| Value::Integer(Integer::from(value));
    let meta = {
        let mut dict = Dictionary::new();
        dict.insert("Digest".into(), Value::Uid(plist::Uid::new(3)));
        dict.insert("Size".into(), int(content.len() as i64));
        dict.insert("Flags".into(), int(0));
        dict.insert("Mode".into(), int(33152));
        dict.insert("ProtectionClass".into(), int(4));
        dict.insert("Birth".into(), int(0));
        dict.insert("InodeNumber".into(), int(1));
        dict.insert("UserID".into(), int(0));
        dict.insert("GroupID".into(), int(0));
        dict.insert("LastModified".into(), int(0));
        dict.insert("LastStatusChange".into(), int(0));
        dict.insert("RelativePath".into(), Value::Uid(plist::Uid::new(2)));
        dict
    };
    let top = {
        let mut dict = Dictionary::new();
        dict.insert("root".into(), Value::Uid(plist::Uid::new(1)));
        dict
    };
    let class = {
        let mut dict = Dictionary::new();
        dict.insert(
            "$classes".into(),
            Value::Array(vec![Value::String("MBFile".into())]),
        );
        dict.insert("$classname".into(), Value::String("MBFile".into()));
        dict
    };
    let mut root = Dictionary::new();
    root.insert("$version".into(), int(100000));
    root.insert("$archiver".into(), Value::String("NSKeyedArchiver".into()));
    root.insert("$top".into(), Value::Dictionary(top));
    root.insert(
        "$objects".into(),
        Value::Array(vec![
            Value::String("$null".into()),
            Value::Dictionary(meta),
            Value::String(relative_path.to_string()),
            Value::Data(digest.to_vec()),
            Value::Dictionary(class),
        ]),
    );
    Value::Dictionary(root)
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// rows: (fileID, relativePath, content, digest_ok). Content is stored in
    /// its hash bucket and registered in Manifest.db.
    fn new(rows: Vec<(&'static str, &'static str, Vec<u8>, bool)>) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let inner = dir.path().join(UDID);
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            inner.join("Status.plist"),
            "<plist version=\"1.0\"><dict><key>SnapshotState</key><string>finished</string></dict></plist>",
        )
        .unwrap();
        std::fs::write(
            inner.join("Info.plist"),
            format!(
                "<plist version=\"1.0\"><dict><key>Product Type</key><string>iPhone9,3</string><key>Product Version</key><string>{PRIMARY_TARGET_IOS}</string></dict></plist>"
            ),
        )
        .unwrap();
        let conn = rusqlite::Connection::open(inner.join("Manifest.db")).unwrap();
        conn.execute(
            "CREATE TABLE Files (fileID TEXT PRIMARY KEY, domain TEXT, relativePath TEXT, flags INTEGER, file BLOB)",
            [],
        )
        .unwrap();
        for (file_id, relative_path, content, digest_ok) in rows {
            let bucket = inner.join(&file_id[..2]).join(file_id);
            std::fs::create_dir_all(bucket.parent().unwrap()).unwrap();
            std::fs::write(&bucket, &content).unwrap();
            let mut hasher = Sha1::new();
            hasher.update(&content);
            let mut digest = hasher.finalize().to_vec();
            if !digest_ok {
                digest[0] ^= 0xff;
            }
            let blob = keyed_blob(relative_path, &content, &digest);
            let mut stored = Vec::new();
            plist::to_writer_binary(&mut stored, &blob).expect("blob encode");
            conn.execute(
                "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![file_id, DOMAIN, relative_path, 1, stored],
            )
            .unwrap();
        }
        Fixture { dir }
    }

    fn backing(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    fn draft(&self, body: &str) -> PathBuf {
        let path = self.dir.path().join("draft.json");
        std::fs::write(&path, body).unwrap();
        path
    }

    fn out(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn toggle_item(relative_path: &str, value: bool) -> String {
    format!(
        "{{\"id\":\"t1\",\"kind\":\"toggle\",\"key\":\"{KEY}\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{relative_path}\",\"value\":{value}}}"
    )
}

fn wrap(item: &str) -> String {
    format!("{{\"items\":[{item}]}}")
}

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
    drop(fixture.out("plan-4.json"));
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &fixture.out("plan-4b.json")).unwrap_err();
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
    let item = format!(
        "{{\"id\":\"t\",\"kind\":\"toggle\",\"key\":\"{KEY}\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{LOWER_PATH}\",\"value\":true,\"accept_case_variants\":true}}"
    );
    let path = fixture.draft(&wrap(&item));
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
    every_way(&fixture, &out);
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    let error = plan_backup(&fixture.backing(), &path, &out).unwrap_err();
    assert_eq!(error.code(), "plan_output_exists");
}

fn every_way(fixture: &Fixture, out: &std::path::Path) {
    let path = fixture.draft(&wrap(&toggle_item(LOWER_PATH, true)));
    plan_backup(&fixture.backing(), &path, out).unwrap();
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
