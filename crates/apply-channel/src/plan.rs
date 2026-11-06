//! Stage 2: build a merge plan from a draft and a verified backup. The plan
//! is a report, nothing else: no backup file is rewritten, no device is
//! contacted beyond the gates of `verify_backup`, and the only thing ever
//! written is the plan document itself (created fresh, never overwritten).

use plist::Value;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::Path;

use crate::manifest::{debug, file_metadata, Manifest};
use crate::{device_directory, Error};

#[derive(Debug, Deserialize)]
pub struct Draft {
    pub items: Vec<DraftItem>,
}

#[derive(Debug, Deserialize)]
pub struct DraftItem {
    pub id: String,
    pub kind: String,
    pub key: String,
    pub domain: String,
    pub relative_path: String,
    pub value: Value,
    /// Only set on purpose: when other manifest entries point at the same
    /// device path but differ in case, planning is forbidden by default. This
    /// explicit flag records an informed decision to proceed with the exact
    /// case match anyway (the siblings are reported, untouched, in the plan).
    #[serde(default)]
    pub accept_case_variants: bool,
}

#[derive(Debug, Serialize)]
pub struct PlanReport {
    pub inputs: usize,
    pub checksum_ok: usize,
    pub files: Vec<FilePlan>,
}

#[derive(Debug, Serialize)]
pub struct FilePlan {
    pub domain: String,
    pub relative_path: String,
    pub file_id: String,
    pub size: u64,
    pub digest_verified: bool,
    pub case_variants: usize,
    pub entries: Vec<EntryPlan>,
}

#[derive(Debug, Serialize)]
pub struct EntryPlan {
    pub id: String,
    pub kind: String,
    pub key: String,
    pub action: &'static str,
    pub old: Option<String>,
    pub new: String,
}

/// Compute the plan for `backup_dir` with the draft from `draft_path` and
/// write the report to `out_path`. Existing output files are never
/// overwritten (`plan_output_exists`).
pub fn plan_backup(
    backup_dir: &Path,
    draft_path: &Path,
    out_path: &Path,
) -> Result<PlanReport, Error> {
    if out_path.exists() {
        return Err(Error::PlanOutputExists);
    }
    let bytes = std::fs::read(draft_path)
        .inspect_err(|error| debug("draft read", &error.to_string()))
        .map_err(|_| Error::BackupLayoutUnsupported)?;
    let draft: Draft = serde_json::from_slice(&bytes)
        .inspect_err(|error| debug("draft parse", &error.to_string()))
        .map_err(|_| Error::PlanBadDraft)?;
    if draft.items.is_empty() {
        return Err(Error::PlanBadDraft);
    }
    debug("draft ok", &draft.items.len().to_string());

    let (device_dir, _udid) = device_directory(backup_dir)?;
    let manifest = Manifest::open(&device_dir.join("Manifest.db"))?;
    debug(
        "manifest ok",
        &manifest.total_entries().unwrap_or(0).to_string(),
    );
    let mut files: Vec<FilePlan> = Vec::new();
    let mut checksum_ok = 0usize;

    // Group draft items by exact (domain, relativePath) so one plan entry
    // describes one file and all keys merged into it.
    let mut groups: Vec<(String, String, Vec<&DraftItem>)> = Vec::new();
    for item in &draft.items {
        match groups
            .iter_mut()
            .find(|(domain, path, _)| domain == &item.domain && path == &item.relative_path)
        {
            Some((_, _, entries)) => entries.push(item),
            None => groups.push((item.domain.clone(), item.relative_path.clone(), vec![item])),
        }
    }

    for (domain, relative_path, items) in &groups {
        let entry = manifest
            .find(domain, relative_path)?
            .ok_or(Error::PlanItemNotFound)?;
        let variants = manifest.case_variants(domain, relative_path)?;
        debug("variants", &variants.to_string());
        if variants > 1 && !items.iter().all(|item| item.accept_case_variants) {
            return Err(Error::PlanAmbiguousTarget);
        }
        if entry.flags != 1 {
            return Err(Error::PlanItemNotFound);
        }
        let metadata = file_metadata(&entry.blob)?;
        if let Some(manifest_rel) = &metadata.relative_path {
            if manifest_rel != relative_path.as_str() {
                return Err(Error::PlanDigestMismatch);
            }
        }
        if !valid_file_id(&entry.file_id) {
            return Err(Error::BackupLayoutUnsupported);
        }
        let bucket = device_dir.join(&entry.file_id[..2]).join(&entry.file_id);
        let contents = std::fs::read(&bucket)
            .inspect_err(|error| debug("bucket read", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)?;
        if contents.len() as u64 != metadata.size {
            return Err(Error::PlanDigestMismatch);
        }
        let mut hasher = Sha1::new();
        hasher.update(&contents);
        let actual = hasher.finalize().to_vec();
        if actual != metadata.digest {
            return Err(Error::PlanDigestMismatch);
        }
        checksum_ok += 1;

        let plist: Value = plist::from_bytes(&contents)
            .inspect_err(|error| debug("target plist parse", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)?;
        let dict = plist
            .into_dictionary()
            .ok_or(Error::BackupLayoutUnsupported)?;
        let mut entries = Vec::with_capacity(items.len());
        for item in items {
            validate_kind(item)?;
            let old = dict.get(&item.key).map(describe);
            let action = if dict.contains_key(&item.key) {
                "replace_value"
            } else {
                "add_key"
            };
            entries.push(EntryPlan {
                id: item.id.clone(),
                kind: item.kind.clone(),
                key: item.key.clone(),
                action,
                old,
                new: describe(&item.value),
            });
        }
        files.push(FilePlan {
            domain: domain.clone(),
            relative_path: relative_path.clone(),
            file_id: entry.file_id.clone(),
            size: metadata.size,
            digest_verified: true,
            case_variants: variants,
            entries,
        });
    }

    let report = PlanReport {
        inputs: draft.items.len(),
        checksum_ok,
        files,
    };
    let rendered = serde_json::to_string_pretty(&report).map_err(|_| Error::PlanBadDraft)?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| Error::PlanOutputExists)?;
    }
    std::fs::write(out_path, rendered).map_err(|_| Error::PlanOutputExists)?;
    Ok(report)
}

fn validate_kind(item: &DraftItem) -> Result<(), Error> {
    let matches_kind = match item.kind.as_str() {
        "toggle" => matches!(item.value, Value::Boolean(_)),
        "text" => matches!(item.value, Value::String(_)),
        "number" => matches!(item.value, Value::Real(_) | Value::Integer(_)),
        _ => false,
    };
    if matches_kind {
        Ok(())
    } else {
        Err(Error::PlanUnsupportedKind)
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::Boolean(bool) => format!("{bool}"),
        Value::Integer(number) => format!("{}", number),
        Value::Real(number) => format!("{number}"),
        Value::String(string) => format!("{string:?}"),
        other => format!("(unreported {})", other_type(other)),
    }
}

fn other_type(value: &Value) -> &'static str {
    match value {
        Value::Data(_) => "data",
        Value::Date(_) => "date",
        Value::Uid(_) => "uid",
        Value::Array(_) => "array",
        Value::Dictionary(_) => "dictionary",
        _ => "ambiguous",
    }
}

fn valid_file_id(file_id: &str) -> bool {
    file_id.len() >= 2 && file_id.bytes().all(|byte| byte.is_ascii_hexdigit())
}
