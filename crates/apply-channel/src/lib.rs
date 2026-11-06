//! Read-only groundwork for the backup/restore apply channel (see
//! `docs/APPLY_CHANNEL.md`). Nothing here writes to a device: the only
//! device-spawning command supported is a consent-gated `idevicebackup2
//! backup`, and the remaining commands read an existing local backup
//! directory. Restore is deliberately absent until the merge stage is
//! verified end to end.
//!
//! Failure mode is "fail closed": every step stops on the first ambiguity,
//! reports a stable snake_case code, and prints only whitelisted fields
//! (ProductType, ProductVersion, SnapshotState) — the UDID is always
//! masked and serials/IMEI/phone numbers are never read at all.

use customization::version;
use device_manager::{valid_udid, Runner};
use plist::Value;
use serde::Serialize;
use std::{cmp::Ordering, path::Path, path::PathBuf};

pub const BACKUP_TOOL: &str = "idevicebackup2";
pub const BACKUP_TIMEOUT_SECS: u64 = 3 * 60 * 60;

/// The exact device this channel is allowed to touch: iPhone 7, iOS 15.8.8,
/// GSM (9,1) or global (9,3). Any other model or build refuses with
/// `target_mismatch`.
pub const PRIMARY_TARGET_PRODUCTS: [&str; 2] = ["iPhone9,1", "iPhone9,3"];
pub const PRIMARY_TARGET_IOS: &str = "15.8.8";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    UsbFailed,
    LockdownUnreadable,
    DeviceNotFound,
    TargetMismatch,
    BackupToolMissing,
    BackupRequiresConsent,
    BackupFailed,
    BackupUnfinished,
    BackupLayoutUnsupported,
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Error::UsbFailed => "usb_failed",
            Error::LockdownUnreadable => "lockdown_unreadable",
            Error::DeviceNotFound => "device_not_found",
            Error::TargetMismatch => "target_mismatch",
            Error::BackupToolMissing => "backup_tool_missing",
            Error::BackupRequiresConsent => "backup_requires_consent",
            Error::BackupFailed => "backup_failed",
            Error::BackupUnfinished => "backup_unfinished",
            Error::BackupLayoutUnsupported => "backup_layout_unsupported",
        }
    }
}

/// Exact-target gate: the model must be one of the target products and the
/// iOS version must compare `Equal` (strict, zero-padded tuple). Anything
/// else — including a newer patch release — refuses.
pub fn target_matches(product: &str, ios: &str) -> bool {
    PRIMARY_TARGET_PRODUCTS
        .iter()
        .any(|target| target.eq_ignore_ascii_case(product.trim()))
        && version::compare(ios.trim(), PRIMARY_TARGET_IOS) == Some(Ordering::Equal)
}

/// UDIDs are secrets; keep only a recognizable frame plus a hyphen skeleton.
pub fn mask_udid(udid: &str) -> String {
    if !valid_udid(udid) {
        return "?".to_string();
    }
    let chars: Vec<char> = udid.chars().collect();
    let keep = chars.len().max(8);
    let mut out = chars[..4].iter().collect::<String>();
    out.push_str(&"*".repeat(keep - 8));
    out.extend(chars[chars.len() - 4..].iter());
    out
}

/// The fixed command line handed to `idevicebackup2` — deliberately a strict
/// whitelist so no caller can smuggle extra options through.
pub fn backup_args(udid: &str, target_dir: &Path) -> Vec<String> {
    vec![
        "-u".into(),
        udid.into(),
        "backup".into(),
        target_dir.display().to_string(),
    ]
}

#[derive(Debug, Serialize)]
pub struct DetectedDevice {
    pub udid_masked: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ios: Option<String>,
    pub matches_target: bool,
}

#[derive(Debug, Serialize)]
pub struct Detection {
    pub failures: Vec<String>,
    pub devices: Vec<DetectedDevice>,
}

/// Enumerate USB devices and evaluate the exact-target gate for each. This is
/// informational only; the hard gate runs again right before backup/verify.
pub fn detection(runner: &impl Runner) -> Result<Detection, Error> {
    let scan = device_manager::scan(runner);
    if scan.devices.is_empty() {
        // No device seen at all: transport/tool level failure.
        return Err(scan
            .failure
            .map(|_| Error::UsbFailed)
            .unwrap_or(Error::DeviceNotFound));
    }
    let mut devices = Vec::new();
    let mut failures = Vec::new();
    for device in &scan.devices {
        let masked = mask_udid(&device.id);
        match &device.info {
            Some(info) => devices.push(DetectedDevice {
                matches_target: target_matches(&info.product, &info.ios),
                product: Some(info.product.clone()),
                ios: Some(info.ios.clone()),
                udid_masked: masked,
            }),
            None => {
                devices.push(DetectedDevice {
                    matches_target: false,
                    product: None,
                    ios: None,
                    udid_masked: masked,
                });
                failures.push("lockdown_unreadable".to_string());
            }
        }
    }
    Ok(Detection { failures, devices })
}

#[derive(Debug, Serialize, PartialEq)]
pub struct BackupSnapshot {
    pub udid_masked: String,
    pub product: String,
    pub ios: String,
    pub snapshot_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_full_backup: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Inspection {
    #[serde(flatten)]
    pub snapshot: BackupSnapshot,
    pub layout: &'static str,
    pub bucket_dirs: usize,
    pub domain_files: usize,
    pub domain_dirs: usize,
    pub entries: usize,
}

/// Run one consent-gated full (`backup` mode, first run implies full) backup
/// into `target_dir`. With `consent == false` this returns
/// `backup_requires_consent` without spawning any process.
pub fn backup(
    runner: &impl Runner,
    target_dir: &Path,
    consent: bool,
) -> Result<BackupSnapshot, Error> {
    if !consent {
        return Err(Error::BackupRequiresConsent);
    }
    let device = single_target_device(runner)?;
    std::fs::create_dir_all(target_dir).map_err(|_| Error::BackupFailed)?;
    let args = backup_args(&device, target_dir);
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    device_manager::checked(runner, BACKUP_TOOL, &args_ref, BACKUP_TIMEOUT_SECS).map_err(
        |failure| match failure {
            device_manager::Failure::ToolMissing => Error::BackupToolMissing,
            _ => Error::BackupFailed,
        },
    )?;
    verify_backup(target_dir)
}

/// Consent gate + exact-target gate enforced in one place, shared by `backup`.
/// Returns the raw UDID (never serialized) on success.
fn single_target_device(runner: &impl Runner) -> Result<String, Error> {
    let scan = device_manager::scan(runner);
    if scan.devices.is_empty() {
        return Err(scan
            .failure
            .map(|_| Error::UsbFailed)
            .unwrap_or(Error::DeviceNotFound));
    }
    // Ambiguity is a refusal, never a guess: backup targets exactly one device.
    if scan.devices.len() > 1 {
        return Err(Error::DeviceNotFound);
    }
    let device = &scan.devices[0];
    match &device.info {
        Some(info) if target_matches(&info.product, &info.ios) => Ok(device.id.clone()),
        Some(_) => Err(Error::TargetMismatch),
        None => Err(Error::LockdownUnreadable),
    }
}

/// Locate the single UDID-named subdirectory of a backup directory.
fn device_directory(dir: &Path) -> Result<(PathBuf, String), Error> {
    let entries = std::fs::read_dir(dir).map_err(|_| Error::BackupLayoutUnsupported)?;
    let mut found: Option<(PathBuf, String)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) && valid_udid(&name) {
            if found.is_some() {
                return Err(Error::BackupLayoutUnsupported);
            }
            found = Some((entry.path(), name));
        }
    }
    found.ok_or(Error::BackupLayoutUnsupported)
}

fn plist_dictionary(path: &Path) -> Result<plist::Dictionary, Error> {
    let bytes = std::fs::read(path).map_err(|_| Error::BackupLayoutUnsupported)?;
    plist::from_bytes::<Value>(&bytes)
        .map_err(|_| Error::BackupLayoutUnsupported)?
        .into_dictionary()
        .ok_or(Error::BackupLayoutUnsupported)
}

fn string<'a>(dict: &'a plist::Dictionary, key: &str) -> Result<&'a str, Error> {
    dict.get(key)
        .and_then(Value::as_string)
        .ok_or(Error::BackupLayoutUnsupported)
}

/// Read Status.plist — the whole-git trust anchor of a backup.
fn status(dir: &Path) -> Result<(String, Option<bool>), Error> {
    let dict = plist_dictionary(&dir.join("Status.plist"))?;
    let state = string(&dict, "SnapshotState")?.to_string();
    let full = dict.get("IsFullBackup").and_then(Value::as_boolean);
    Ok((state, full))
}

/// Read Info.plist, extracting only whitelisted fields. Serial number,
/// IMEI, phone number and device name present in that file are parsed and
/// immediately dropped, never stored or printed.
///
/// Apple/libimobiledevice Info.plist spells these keys with spaces
/// ("Product Type"); older tooling uses camelCase — both accepted.
fn identity(dir: &Path) -> Result<(String, String), Error> {
    let dict = plist_dictionary(&dir.join("Info.plist"))?;
    let product = first_string(&dict, ["ProductType", "Product Type"])?.to_string();
    let ios = first_string(&dict, ["ProductVersion", "Product Version"])?.to_string();
    Ok((product, ios))
}

fn first_string<'a>(
    dict: &'a plist::Dictionary,
    keys: impl AsRef<[&'a str]>,
) -> Result<&'a str, Error> {
    keys.as_ref()
        .iter()
        .find_map(|key| dict.get(key).and_then(Value::as_string))
        .ok_or(Error::BackupLayoutUnsupported)
}

/// Verify a local backup directory: UDID-named directory exists,
/// SnapshotState is "finished", and the backup's own identity still matches
/// the exact target. Returns masked-only data.
pub fn verify_backup(dir: &Path) -> Result<BackupSnapshot, Error> {
    let (_, udid) = device_directory(dir)?;
    let (state, full) = status(&dir.join(&udid))?;
    if state != "finished" {
        return Err(Error::BackupUnfinished);
    }
    let (product, ios) = identity(&dir.join(&udid))?;
    if !target_matches(&product, &ios) {
        return Err(Error::TargetMismatch);
    }
    Ok(BackupSnapshot {
        udid_masked: mask_udid(&udid),
        product,
        ios,
        snapshot_state: state,
        is_full_backup: full,
    })
}

/// Classify a backup directory: which manifest format is present, how many
/// entries carry a domain prefix, how large the flat space is. Reading the
/// manifest itself is deliberately postponed to the merge stage.
pub fn inspect_backup(dir: &Path) -> Result<Inspection, Error> {
    let (_, udid) = device_directory(dir)?;
    let device_dir = dir.join(&udid);
    let (state, full) = status(&device_dir)?;
    let (product, ios) = identity(&device_dir)?;
    if !target_matches(&product, &ios) {
        return Err(Error::TargetMismatch);
    }

    let mut layout = None;
    for (name, marker) in [
        ("Manifest.mbdb", "manifest_mbdb"),
        ("Manifest.db", "manifest_sqlite"),
    ] {
        if device_dir.join(name).exists() {
            layout = Some(marker);
            break;
        }
    }
    let layout = match layout {
        Some(marker) => marker,
        None => return Err(Error::BackupLayoutUnsupported),
    };
    let entries = std::fs::read_dir(&device_dir).map_err(|_| Error::BackupLayoutUnsupported)?;
    let mut bucket_dirs = 0;
    let mut domain_files = 0;
    let mut domain_dirs = 0;
    let mut total = 0;
    for entry in entries.flatten() {
        total += 1;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if name.len() == 2 && name.bytes().all(|byte| byte.is_ascii_hexdigit()) && is_dir {
            bucket_dirs += 1;
            continue;
        }
        if !name.contains("Domain-") {
            continue;
        }
        if is_dir {
            domain_dirs += 1;
        } else {
            domain_files += 1;
        }
    }

    Ok(Inspection {
        snapshot: BackupSnapshot {
            udid_masked: mask_udid(&udid),
            product,
            ios,
            snapshot_state: state,
            is_full_backup: full,
        },
        layout,
        bucket_dirs,
        domain_files,
        domain_dirs,
        entries: total,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use device_manager::{Failure, Output};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    const UDID: &str = "0123456789012345678901234567890123456789";
    const FAKE_INFO_CELLULAR: &[u8] = br#"<?xml version="1.0"?><plist version="1.0"><dict><key>ProductType</key><string>iPhone9,3</string><key>ProductVersion</key><string>15.8.8</string><key>BuildVersion</key><string>19H380</string><key>SerialNumber</key><string>F17TOPSECRET</string></dict></plist>"#;
    const FAKE_INFO_OTHER: &[u8] = br#"<?xml version="1.0"?><plist version="1.0"><dict><key>ProductType</key><string>iPhone9,2</string><key>ProductVersion</key><string>15.8.8</string><key>BuildVersion</key><string>19H380</string><key>SerialNumber</key><string>F17TOPSECRET</string></dict></plist>"#;

    #[derive(Clone)]
    struct Probe {
        calls: Arc<Mutex<Vec<String>>>,
        list: &'static str,
        info: Result<&'static [u8], Failure>,
        backup_exit: bool,
    }

    impl Probe {
        fn new(
            list: &'static str,
            info: Result<&'static [u8], Failure>,
            backup_exit: bool,
        ) -> Self {
            Probe {
                calls: Arc::new(Mutex::new(Vec::new())),
                list,
                info,
                backup_exit,
            }
        }
    }

    impl Runner for Probe {
        fn run(&self, tool: &str, args: &[&str], _: Duration) -> Result<Output, Failure> {
            self.calls.lock().unwrap().push(format!("{tool} {args:?}"));
            if tool == "idevice_id" {
                return Ok(Output {
                    success: true,
                    stdout: self.list.into(),
                });
            }
            if tool == "ideviceinfo" {
                return match self.info {
                    Ok(bytes) => Ok(Output {
                        success: true,
                        stdout: bytes.to_vec(),
                    }),
                    Err(ref failure) => Err(failure.clone()),
                };
            }
            Ok(Output {
                success: self.backup_exit,
                stdout: Vec::new(),
            })
        }
    }

    fn writable(status_state: &str) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path().join(UDID);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Status.plist"),
            format!(
                "<plist version=\"1.0\"><dict><key>SnapshotState</key><string>{status_state}</string><key>IsFullBackup</key><true/></dict></plist>"
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("Info.plist"),
            "<plist version=\"1.0\"><dict><key>Product Type</key><string>iPhone9,3</string><key>Product Version</key><string>15.8.8</string><key>Serial Number</key><string>F17TOPSECRET</string></dict></plist>",
        )
        .unwrap();
        std::fs::write(dir.join("Manifest.mbdb"), [0u8; 4]).unwrap();
        std::fs::write(dir.join("SysSharedContainerDomain-file"), b"payload").unwrap();
        temp
    }

    #[test]
    fn exact_target_gate() {
        assert!(target_matches("iPhone9,3", "15.8.8"));
        assert!(target_matches("iPhone9,1", "15.8.8"));
        assert!(!target_matches("iPhone9,2", "15.8.8"));
        assert!(!target_matches("iPhone9,3", "15.8.7"));
        assert!(!target_matches("iPhone9,3", "16.0"));
        assert!(!target_matches("iPhone9,3", "15.8"));
    }

    #[test]
    fn masking_keeps_frame_only() {
        assert_eq!(mask_udid(UDID).len(), UDID.len());
        assert_eq!(&mask_udid(UDID)[..4], "0123");
        assert!(mask_udid(UDID)
            .chars()
            .all(|c| c == '*' || c.is_ascii_digit()));
        assert_eq!(mask_udid("../../file"), "?");
        assert_eq!(mask_udid("00008020-0123456789ABCDEF").len(), 25);
    }

    #[test]
    fn backup_args_whitelist() {
        let dir = Path::new("/tmp/backups/x");
        assert_eq!(
            backup_args(UDID, dir),
            vec![
                "-u".to_string(),
                UDID.to_string(),
                "backup".to_string(),
                "/tmp/backups/x".to_string(),
            ]
        );
    }

    #[test]
    fn consent_missing_never_spawns_backup() {
        let probe = Probe::new(
            "0123456789012345678901234567890123456789\n",
            Ok(FAKE_INFO_CELLULAR),
            true,
        );
        let result = backup(&probe, Path::new("/no/entry"), false);
        assert!(matches!(result, Err(Error::BackupRequiresConsent)));
        assert!(
            probe.calls.lock().unwrap().is_empty(),
            "no tool call allowed"
        );
    }

    #[test]
    fn detect_reports_device_and_gate() {
        let probe = Probe::new(
            "0123456789012345678901234567890123456789\n",
            Ok(FAKE_INFO_CELLULAR),
            true,
        );
        let detection = detection(&probe).unwrap();
        assert_eq!(detection.devices.len(), 1);
        assert!(detection.devices[0].matches_target);
        assert_eq!(detection.devices[0].product.as_deref(), Some("iPhone9,3"));
        assert_eq!(detection.devices[0].udid_masked.len(), UDID.len());
    }

    #[test]
    fn detect_empty_and_unreadable() {
        let empty = Probe::new("", Ok(FAKE_INFO_CELLULAR), true);
        assert!(matches!(detection(&empty), Err(Error::DeviceNotFound)));

        let dead = Probe::new(
            "0123456789012345678901234567890123456789\n",
            Err(Failure::CommandFailed),
            true,
        );
        let dead = detection(&dead).unwrap();
        assert_eq!(dead.devices[0].product, None);
        assert_eq!(dead.failures.len(), 1);
    }

    #[test]
    fn backup_refuses_foreign_device() {
        let probe = Probe::new(
            "0123456789012345678901234567890123456789\n",
            Ok(FAKE_INFO_OTHER),
            true,
        );
        let result = backup(&probe, Path::new("/no/entry"), true);
        assert!(matches!(result, Err(Error::TargetMismatch)));
        assert!(
            probe
                .calls
                .lock()
                .unwrap()
                .iter()
                .all(|call| !call.starts_with("idevicebackup2")),
            "backup must not run for a foreign target"
        );
    }

    #[test]
    fn backup_tool_missing_is_explicit() {
        struct Absent {
            list: &'static str,
            info: &'static [u8],
        }
        impl Runner for Absent {
            fn run(&self, tool: &str, _: &[&str], _: Duration) -> Result<Output, Failure> {
                if tool == "idevice_id" {
                    return Ok(Output {
                        success: true,
                        stdout: self.list.into(),
                    });
                }
                if tool == "ideviceinfo" {
                    return Ok(Output {
                        success: true,
                        stdout: self.info.to_vec(),
                    });
                }
                Err(Failure::ToolMissing)
            }
        }
        let probe = Absent {
            list: "0123456789012345678901234567890123456789\n",
            info: FAKE_INFO_CELLULAR,
        };
        let temp = tempfile::tempdir().expect("tempdir");
        let result = backup(&probe, temp.path(), true);
        assert!(matches!(result, Err(Error::BackupToolMissing)));
    }

    #[test]
    fn backup_of_unfinished_snapshot_refuses() {
        let probe = Probe::new(
            "0123456789012345678901234567890123456789\n",
            Ok(FAKE_INFO_CELLULAR),
            true,
        );
        let temp = writable("unfinished");
        let result = backup(&probe, temp.path(), true);
        assert!(matches!(result, Err(Error::BackupUnfinished)));
    }

    #[test]
    fn verify_rejects_broken_layouts() {
        let finished = writable("finished");
        let ok = verify_backup(finished.path()).unwrap();
        assert_eq!(ok.snapshot_state, "finished");
        assert_eq!(ok.product, "iPhone9,3");
        assert_eq!(ok.is_full_backup, Some(true));

        let empty = tempfile::tempdir().unwrap();
        assert!(matches!(
            verify_backup(empty.path()),
            Err(Error::BackupLayoutUnsupported)
        ));
    }

    #[test]
    fn inspect_classifies_mbdb_layout() {
        let temp = writable("finished");
        let inspection = inspect_backup(temp.path()).unwrap();
        assert_eq!(inspection.layout, "manifest_mbdb");
        assert_eq!(inspection.bucket_dirs, 0);
        assert_eq!(inspection.domain_files, 1);
        assert_eq!(inspection.domain_dirs, 0);
        assert!(inspection.entries >= 2);
        assert_eq!(inspection.snapshot.udid_masked.len(), UDID.len());

        let sqlite = tempfile::tempdir().unwrap();
        let dir = sqlite.path().join(UDID);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Status.plist"),
            "<plist version=\"1.0\"><dict><key>SnapshotState</key><string>finished</string></dict></plist>",
        )
        .unwrap();
        std::fs::write(
            dir.join("Info.plist"),
            "<plist version=\"1.0\"><dict><key>ProductType</key><string>iPhone9,3</string><key>ProductVersion</key><string>15.8.8</string></dict></plist>",
        )
        .unwrap();
        std::fs::write(dir.join("Manifest.db"), b"SQLite format 3").unwrap();
        std::fs::create_dir(sqlite.path().join(UDID).join("00")).unwrap();
        std::fs::create_dir(sqlite.path().join(UDID).join("ff")).unwrap();
        let inspected = inspect_backup(sqlite.path()).unwrap();
        assert_eq!(inspected.layout, "manifest_sqlite");
        assert_eq!(inspected.bucket_dirs, 2);

        let bare = tempfile::tempdir().unwrap();
        let dir = bare.path().join(UDID);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Info.plist"),
            "<plist version=\"1.0\"><dict><key>ProductType</key><string>iPhone9,3</string><key>ProductVersion</key><string>15.8.8</string></dict></plist>",
        )
        .unwrap();
        assert!(matches!(
            inspect_backup(bare.path()),
            Err(Error::BackupLayoutUnsupported)
        ));
    }

    #[test]
    fn single_ambiguous_targets_refuse() {
        // Two devices: no unique target — refuse rather than guess.
        let both =
            "0123456789012345678901234567890123456789\n0123456789abcdef0123456789abcdef01234567\n";
        let probe = Probe::new(both, Ok(FAKE_INFO_CELLULAR), true);
        assert!(matches!(
            backup(&probe, Path::new("/no/entry"), true),
            Err(Error::DeviceNotFound)
        ));
    }
}
