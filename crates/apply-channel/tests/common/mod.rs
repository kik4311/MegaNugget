//! Shared fixtures emulating a real idevicebackup2 backup layout (UDID dir,
//! Manifest.db + hash buckets, WAL-mode SQLite manifest). Used by the plan,
//! merge and effect integration tests; no device involved.

use plist::{Dictionary, Value};
use sha1::{Digest, Sha1};
use std::path::PathBuf;

pub const UDID: &str = "0123456789012345678901234567890123456789";
pub const DOMAIN: &str = "HomeDomain";
pub const LOWER_PATH: &str = "Library/Preferences/com.apple.springboard.plist";
#[allow(dead_code)]
pub const UPPER_PATH: &str = "Library/Preferences/com.apple.SpringBoard.plist";
pub const LOWER_ID: &str = "662bc19b13aecef58a7e855d0316e4cf61e2642b";
#[allow(dead_code)]
pub const UPPER_ID: &str = "22e3b6f23409a50b22b33df1881b50265ea10da3";
pub const KEY: &str = "SBHideLowPowerAlerts";

/// A plist that already carries one known key plus padding; also used raw as
/// a springboard stub.
pub fn sample_plist(extra: &[(&str, Value)]) -> Vec<u8> {
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

/// NSKeyedArchiver-shaped metadata blob as written by the device tooling:
/// `Digest` and `RelativePath` are UID references into `$objects`, exactly
/// like the real Manifest.db of idevicebackup2 writes them.
pub fn keyed_blob(relative_path: &str, content: &[u8], digest: &[u8]) -> Value {
    let int = |value: i64| Value::Integer(plist::Integer::from(value));
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

pub struct Fixture {
    pub dir: tempfile::TempDir,
    /// Sibling temporary root for merge outputs — `applied` directories must
    /// live outside the source backup (see `merge_into_copy`).
    #[allow(dead_code)] // not every test binary merges
    pub applied: tempfile::TempDir,
}

impl Fixture {
    /// rows: (fileID, relativePath, content, digest_ok). Content is stored in
    /// its hash bucket and registered in Manifest.db.
    pub fn new(rows: Vec<(&'static str, &'static str, Vec<u8>, bool)>) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let inner = dir.path().join(UDID);
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            inner.join("Status.plist"),
            "<plist version=\"1.0\"><dict><key>SnapshotState</key><string>finished</string></dict></plist>",
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
        write_info_plist(&inner);
        Fixture {
            dir,
            applied: tempfile::tempdir().expect("applied tempdir"),
        }
    }

    pub fn backing(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    /// A fresh non-existent output path OUTSIDE the source backup tree.
    #[allow(dead_code)]
    pub fn applied_target(&self, name: &str) -> PathBuf {
        self.applied.path().join(name)
    }

    pub fn draft(&self, body: &str) -> PathBuf {
        let path = self.dir.path().join("draft.json");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[allow(dead_code)]
    pub fn out(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn write_info_plist(device_dir: &std::path::Path) {
    std::fs::write(
        device_dir.join("Info.plist"),
        format!(
            "<plist version=\"1.0\"><dict><key>Product Type</key><string>iPhone9,3</string><key>Product Version</key><string>{}</string></dict></plist>",
            apply_channel::PRIMARY_TARGET_IOS
        ),
    )
    .unwrap();
}

pub fn toggle_item(relative_path: &str, value: bool) -> String {
    format!(
        "{{\"id\":\"t1\",\"kind\":\"toggle\",\"key\":\"{KEY}\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{relative_path}\",\"value\":{value}}}"
    )
}

pub fn wrap(item: &str) -> String {
    format!("{{\"items\":[{item}]}}")
}

#[allow(dead_code)]
pub fn toggle_item_case_accepted(relative_path: &str, value: bool) -> String {
    format!(
        "{{\"id\":\"t\",\"kind\":\"toggle\",\"key\":\"{KEY}\",\"domain\":\"{DOMAIN}\",\"relative_path\":\"{relative_path}\",\"value\":{value},\"accept_case_variants\":true}}"
    )
}
