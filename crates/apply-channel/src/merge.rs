//! Stage 3, step 1: merge a resolved draft into a fresh copy of a verified
//! backup. The original backup is never opened for writing and never has a
//! byte rewritten: the copy is a hardlink tree in which only the planned
//! target plists are fresh files (the original inodes stay bound to the
//! source), and the SQLite manifest inside the copy is a real file copy —
//! real backups run SQLite in WAL mode (`Manifest.db-wal/-shm` exist), so
//! hard-linking those would let the copy's writes leak into the source.
//! No device is contacted by this module.

use plist::Value;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::Path;

use crate::manifest::{debug, file_metadata, open_patchable, Manifest};
use crate::plan::{resolve_draft, PlannedFile};
use crate::{device_directory, verify_backup, Error};

#[derive(Debug, Serialize)]
pub struct MergeReport {
    pub files: Vec<MergedFile>,
}

#[derive(Debug, Serialize)]
pub struct MergedFile {
    pub domain: String,
    pub relative_path: String,
    pub file_id: String,
    pub old_size: u64,
    pub new_size: u64,
    pub new_sha1: String,
    pub keys_patched: Vec<String>,
}

/// Create `applied_dir` as a hardlink copy of `backup_dir` with the draft
/// merged in. Draft resolution (digest checks, case-variant rules) is the
/// exact stage-2 logic; the source backup is never mutated, a refusal happens
/// before anything is created, and a failure after that removes the partial
/// copy again.
pub fn merge_into_copy(
    source_dir: &Path,
    draft_path: &Path,
    applied_dir: &Path,
) -> Result<MergeReport, Error> {
    if applied_dir.exists() {
        return Err(Error::MergeOutputExists);
    }
    if nested_output(source_dir, applied_dir) {
        return Err(Error::MergeOutputInsideSource);
    }
    // Gates first, so the common refusals never leave a copy behind at all.
    verify_backup(source_dir)?;
    let (source_device, source_udid) = device_directory(source_dir)?;
    let planned = resolve_draft(&source_device, draft_path)?;
    debug("planned files", &planned.len().to_string());

    // Bucket paths that must not be hardlinked: they get fresh content, so
    // their source entries keep the original inode untouched. Paths are
    // relative to the backup root, matching how `copy_tree` walks.
    let mut skipped = Vec::with_capacity(planned.len());
    for file in &planned {
        skipped.push(format!(
            "{source_udid}/{}/{}",
            &file.file_id[..2],
            file.file_id
        ));
    }
    copy_tree(source_dir, applied_dir, &skipped)?;
    let applied_device = applied_dir.join(&source_udid);
    match apply_planned(applied_dir, &applied_device, &planned) {
        Ok(report) => Ok(report),
        Err(error) => {
            // The copy only exists for this call; a failed merge must not
            // leave a half-patched directory that could be mistaken later
            // for a complete, verified apply.
            let _ = std::fs::remove_dir_all(applied_dir);
            Err(error)
        }
    }
}

/// Patch every planned file inside the copy: fresh plist bytes, fresh bucket
/// content, manifest row re-stamped, then the whole copy round-trips the same
/// verify gates a real backup must pass before `restore` may touch it.
fn apply_planned(
    applied_dir: &Path,
    applied_device: &Path,
    planned: &[PlannedFile],
) -> Result<MergeReport, Error> {
    let mut report = MergeReport { files: Vec::new() };
    let patchable = open_patchable(&applied_device.join("Manifest.db"))?;
    for file in planned {
        let encoded = encode_plist_with(&file.plist, &file.insertions)
            .inspect_err(|error| debug("plist encode", &error.to_string()))
            .map_err(|_| Error::BackupFailed)?;
        let sha = {
            let mut hasher = Sha1::new();
            hasher.update(&encoded);
            hasher.finalize().to_vec()
        };
        let bucket = applied_device.join(&file.file_id[..2]).join(&file.file_id);
        std::fs::write(&bucket, &encoded)
            .inspect_err(|error| debug("bucket write", &error.to_string()))
            .map_err(|_| Error::BackupFailed)?;

        // Same NSKeyedArchiver blob as the source with fresh Digest/Size so
        // restore and later verifications stay mutually consistent.
        let patched = repatch_blob(&file.source_blob, &sha, encoded.len() as u64)?;
        let stored = encode_plist_with(&patched, &[])
            .inspect_err(|error| debug("blob re-encode", &error.to_string()))
            .map_err(|_| Error::BackupFailed)?;
        apply_manifest_patch(&patchable, &file.file_id, &stored)?;

        // The manifest must agree with the new bucket file before the merge
        // is accepted.
        let manifest_copy = Manifest::open(&applied_device.join("Manifest.db"))?;
        let entry = manifest_copy
            .find(&file.domain, &file.relative_path)?
            .ok_or(Error::BackupLayoutUnsupported)?;
        let metadata = file_metadata(&entry.blob)?;
        if metadata.digest != sha || metadata.size != encoded.len() as u64 {
            debug("manifest patch", "row update did not stick");
            return Err(Error::BackupFailed);
        }
        report.files.push(MergedFile {
            domain: file.domain.clone(),
            relative_path: file.relative_path.clone(),
            file_id: file.file_id.clone(),
            old_size: file.size,
            new_size: encoded.len() as u64,
            new_sha1: hex(&sha),
            keys_patched: file.keys_patched.clone(),
        });
    }
    // Fold any WAL fragments (real backups carry Manifest.db-wal/-shm) into
    // the single copied database so restore reads one consistent file.
    let _ = patchable.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    drop(patchable);
    verify_backup(applied_dir)?;
    Ok(report)
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Re-encode a plist dictionary with extra insertions applied.
fn encode_plist_with(
    dictionary: &plist::Dictionary,
    insertions: &[(String, Value)],
) -> Result<Vec<u8>, plist::Error> {
    let mut copy = dictionary.clone();
    for (key, value) in insertions {
        copy.insert(key.clone(), value.clone());
    }
    let mut encoded = Vec::new();
    plist::to_writer_binary(&mut encoded, &Value::Dictionary(copy))?;
    Ok(encoded)
}

/// Load the source metadata blob and swap Digest/Size in place, keeping the
/// full keyed-archive shape (RelativePath, Mode, ProtectionClass, … intact).
/// The swapped Digest becomes inline data; `file_metadata` reads both the
/// inline and the UID-referenced form, so nothing downstream can tell the
/// difference.
fn repatch_blob(source_blob: &[u8], digest: &[u8], size: u64) -> Result<plist::Dictionary, Error> {
    let root: Value = plist::from_bytes(source_blob).map_err(|_| Error::BackupLayoutUnsupported)?;
    let mut root = root
        .into_dictionary()
        .ok_or(Error::BackupLayoutUnsupported)?;
    let index = root
        .get("$top")
        .and_then(Value::as_dictionary)
        .and_then(|top| top.get("root"))
        .and_then(Value::as_uid)
        .map(|uid| uid.get() as usize)
        .ok_or(Error::BackupLayoutUnsupported)?;
    let meta = root
        .get_mut("$objects")
        .and_then(Value::as_array_mut)
        .and_then(|objects| objects.get_mut(index))
        .and_then(Value::as_dictionary_mut)
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob repatch", "metadata object missing"))?;
    if meta.get("Digest").is_none() || meta.get("Size").is_none() {
        debug("blob repatch", "missing Digest/Size markers");
        return Err(Error::BackupLayoutUnsupported);
    }
    meta.insert("Digest".into(), Value::Data(digest.to_vec()));
    meta.insert("Size".into(), Value::Integer((size as i64).into()));
    Ok(root)
}

fn apply_manifest_patch(
    patchable: &rusqlite::Connection,
    file_id: &str,
    blob: &[u8],
) -> Result<(), Error> {
    let changed = patchable
        .execute(
            "UPDATE Files SET file = ?1 WHERE fileID = ?2",
            rusqlite::params![blob, file_id],
        )
        .inspect_err(|error| debug("manifest patch", &error.to_string()))
        .map_err(|_| Error::BackupFailed)?;
    if changed == 0 {
        debug("manifest patch", "no row replaced");
        return Err(Error::BackupLayoutUnsupported);
    }
    Ok(())
}

/// Copy the whole backup tree into the copy without deleting anything from
/// the source. Regular files are hardlinked (shared inodes, zero-copy);
/// planned target paths are skipped — the merge writes fresh files there, so
/// the source inode is never touched. `Manifest.db*` (SQLite WAL family) is
/// copied for real for the same reason. Symlinks and other specials refuse.
fn copy_tree(source: &Path, destination: &Path, skip: &[String]) -> Result<(), Error> {
    std::fs::create_dir_all(destination).map_err(|_| Error::BackupFailed)?;
    let mut stack = vec![(
        source.to_path_buf(),
        destination.to_path_buf(),
        String::new(),
    )];
    while let Some((from, to, rel)) = stack.pop() {
        let entries = std::fs::read_dir(&from)
            .inspect_err(|error| debug("copy read", &error.to_string()))
            .map_err(|_| Error::BackupFailed)?;
        for entry in entries {
            let entry = entry.map_err(|_| Error::BackupFailed)?;
            let file_type = entry
                .file_type()
                .map_err(|_| Error::BackupLayoutUnsupported)?;
            let name = entry.file_name();
            let child_rel = if rel.is_empty() {
                name.to_string_lossy().into_owned()
            } else {
                format!("{rel}/{}", name.to_string_lossy())
            };
            let source_path = from.join(&name);
            let target = to.join(&name);
            if file_type.is_dir() {
                std::fs::create_dir_all(&target).map_err(|_| Error::BackupFailed)?;
                stack.push((source_path, target, child_rel));
                continue;
            }
            if !file_type.is_file() {
                debug("copy", "non-regular entry refused");
                return Err(Error::BackupLayoutUnsupported);
            }
            if skip.iter().any(|planned| planned == &child_rel) {
                continue;
            }
            if is_manifest_db_family(&name.to_string_lossy()) {
                std::fs::copy(&source_path, &target)
                    .inspect_err(|error| debug("manifest copy", &error.to_string()))
                    .map_err(|_| Error::BackupFailed)?;
                continue;
            }
            if std::fs::hard_link(&source_path, &target).is_err() {
                // Fallback for exotic filesystems (EXDEV, quota): plain copy.
                std::fs::copy(&source_path, &target)
                    .inspect_err(|error| debug("copy fallback", &error.to_string()))
                    .map_err(|_| Error::BackupFailed)?;
            }
        }
    }
    Ok(())
}

fn is_manifest_db_family(name: &str) -> bool {
    name == "Manifest.db" || name.starts_with("Manifest.db-")
}

/// The copy must not live inside the source backup (the tree walker would
/// recurse into its own output) nor wrap it (the applied directory would
/// swallow the original). `applied_dir` usually does not exist yet, so the
/// parent is resolved instead when possible.
fn nested_output(source_dir: &Path, applied_dir: &Path) -> bool {
    let source_key = std::fs::canonicalize(source_dir).unwrap_or_else(|_| source_dir.to_path_buf());
    let mut applied_key = applied_dir.to_path_buf();
    if let (Some(parent), Some(name)) = (applied_dir.parent(), applied_dir.file_name()) {
        if let Ok(resolved_parent) = std::fs::canonicalize(parent) {
            applied_key = resolved_parent.join(name);
        }
    }
    let nested = applied_key.starts_with(&source_key) || source_key.starts_with(&applied_key);
    if nested {
        debug(
            "merge",
            "applied directory must be outside the source backup",
        );
    }
    nested
}
