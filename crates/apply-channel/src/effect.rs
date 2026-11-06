//! Stage 3, step 3: prove what the restore actually changed by reading the
//! same draft keys out of the pre-apply and the post-restore backup. Both are
//! re-verified and re-resolved with the exact stage-2 logic; nothing here
//! writes anything anywhere.

use serde::Serialize;
use std::path::Path;

use crate::manifest::debug;
use crate::plan::{describe, resolve_draft};
use crate::{device_directory, verify_backup, Error};

#[derive(Debug, Serialize)]
pub struct EffectReport {
    pub files: Vec<EffectFile>,
}

#[derive(Debug, Serialize)]
pub struct EffectFile {
    pub domain: String,
    pub relative_path: String,
    pub keys: Vec<EffectKey>,
}

#[derive(Debug, Serialize)]
pub struct EffectKey {
    pub id: String,
    pub key: String,
    /// "missing" — the key is absent after the restore;
    /// "changed" — present but not the planned value;
    /// "matched" — present with the planned value.
    pub status: &'static str,
    pub before: Option<String>,
    pub planned: String,
    pub after: Option<String>,
}

/// Compare the draft's planned values against what the post-restore backup
/// actually contains. The typical call is
/// `effect(draft, original_backup_dir, fresh_backup_after_restore)`.
pub fn effect(draft_path: &Path, pre_dir: &Path, post_dir: &Path) -> Result<EffectReport, Error> {
    verify_backup(pre_dir)?;
    verify_backup(post_dir)?;
    let (pre_device, _) = device_directory(pre_dir)?;
    let (post_device, _) = device_directory(post_dir)?;
    let before = resolve_draft(&pre_device, draft_path)?;
    let after = resolve_draft(&post_device, draft_path)?;
    if before.len() != after.len() {
        debug("effect", "resolution drifted between pre and post");
        return Err(Error::BackupLayoutUnsupported);
    }
    let mut files = Vec::with_capacity(after.len());
    for (pre_file, post_file) in before.iter().zip(after.iter()) {
        if pre_file.domain != post_file.domain || pre_file.relative_path != post_file.relative_path
        {
            debug("effect", "targets drifted between pre and post");
            return Err(Error::BackupLayoutUnsupported);
        }
        let mut keys = Vec::with_capacity(post_file.insertions.len());
        for (index, (key, planned)) in post_file.insertions.iter().enumerate() {
            let planned_text = describe(planned);
            let actual = post_file.plist.get(key);
            let status = match actual {
                None => "missing",
                Some(value) if *value == *planned => "matched",
                Some(_) => "changed",
            };
            keys.push(EffectKey {
                id: post_file.entries[index].id.clone(),
                key: key.clone(),
                status,
                before: pre_file.plist.get(key).map(describe),
                planned: planned_text,
                after: actual.map(describe),
            });
        }
        files.push(EffectFile {
            domain: post_file.domain.clone(),
            relative_path: post_file.relative_path.clone(),
            keys,
        });
    }
    Ok(EffectReport { files })
}
