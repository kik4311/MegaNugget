//! Read-only access to a backup's Manifest.db (SQLite manifest written by
//! `idevicebackup2`). The connection is opened with `OPEN_READ_ONLY` so a
//! planning session can never mutate the backup it inspects. The per-file
//! metadata blob inside SQLite is a small NSKeyedArchiver plist whose
//! important parts for this channel are the SHA-1 `Digest` and `Size`.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

use crate::Error;

pub struct Manifest {
    conn: Connection,
}

/// Opt-in low-volume diagnostics; prints only error strings, never data.
pub(crate) fn debug(stage: &str, detail: &str) {
    if std::env::var_os("MG_DEBUG").is_some() {
        eprintln!("mg-debug: {stage}: {detail}");
    }
}

pub struct Entry {
    pub file_id: String,
    pub domain: String,
    pub relative_path: String,
    pub flags: i64,
    pub blob: Vec<u8>,
}

pub struct FileMetadata {
    pub digest: Vec<u8>,
    pub size: u64,
    pub relative_path: Option<String>,
}

impl Manifest {
    pub fn open(path: &Path) -> Result<Manifest, Error> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .inspect_err(|error| debug("manifest open", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)?;
        let tables: Result<Vec<String>, _> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .and_then(|mut stmt| stmt.query_map([], |row| row.get(0))?.collect());
        let mut tables = tables.map_err(|_| Error::BackupLayoutUnsupported)?;
        tables.retain(|name| name == "Files");
        if tables.is_empty() {
            return Err(Error::BackupLayoutUnsupported);
        }
        Ok(Manifest { conn })
    }

    /// Exact (case-sensitive) lookup of one manifest entry by domain + path.
    pub fn find(&self, domain: &str, relative_path: &str) -> Result<Option<Entry>, Error> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT fileID, domain, relativePath, flags, file FROM Files
                 WHERE domain = ?1 AND relativePath = ?2",
            )
            .inspect_err(|error| debug("find", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)?;
        let rows = stmt
            .query_map([domain, relative_path], map_entry)
            .inspect_err(|error| debug("find query", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)?;
        let mut entries: Vec<Entry> = Vec::new();
        for entry in rows {
            entries.push(entry.map_err(|_| Error::BackupLayoutUnsupported)?);
        }
        match entries.len() {
            0 => Ok(None),
            1 => Ok(entries.pop()),
            _ => Err(Error::PlanAmbiguousTarget),
        }
    }

    /// Case-insensitive siblings of `relative_path` inside `domain` (self
    /// included). More than one means the device path being targeted is
    /// identified by two manifest entries that differ only in case — refuse.
    pub fn case_variants(&self, domain: &str, relative_path: &str) -> Result<usize, Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM Files WHERE domain = ?1 AND relativePath = ?2 COLLATE NOCASE",
                [domain, relative_path],
                |row| row.get::<_, i64>(0),
            )
            .inspect_err(|error| debug("case variants", &error.to_string()))
            .map_err(|_| Error::BackupLayoutUnsupported)
            .map(|count| count as usize)
    }

    pub fn total_entries(&self) -> Result<usize, Error> {
        self.conn
            .query_row("SELECT COUNT(*) FROM Files", [], |row| row.get::<_, i64>(0))
            .map_err(|_| Error::BackupLayoutUnsupported)
            .map(|count| count as usize)
    }
}

/// Open a *copy's* Manifest.db with write access so the merge can re-stamp
/// Digest/Size rows. Only ever call this on a directory this process just
/// created (see `merge_into_copy`): opening the manifest of an original
/// backup read-write is exactly the accident this channel refuses to allow.
pub(crate) fn open_patchable(path: &Path) -> Result<Connection, Error> {
    if !path.is_file() {
        return Err(Error::BackupLayoutUnsupported);
    }
    let conn = Connection::open(path)
        .inspect_err(|error| debug("manifest rw open", &error.to_string()))
        .map_err(|_| Error::BackupLayoutUnsupported)?;
    let has_files: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='Files'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .inspect_err(|error| debug("manifest rw schema", &error.to_string()))
        .map_err(|_| Error::BackupLayoutUnsupported)?;
    if !has_files {
        return Err(Error::BackupLayoutUnsupported);
    }
    Ok(conn)
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        file_id: row.get(0)?,
        domain: row.get(1)?,
        relative_path: row.get(2)?,
        flags: row.get(3)?,
        blob: row.get(4)?,
    })
}

/// Parse the per-file NSKeyedArchiver blob and return Digest/Size/RelativePath.
/// Anything else is parsed and dropped.
pub fn file_metadata(blob: &[u8]) -> Result<FileMetadata, Error> {
    let root: plist::Value = plist::from_bytes(blob)
        .inspect_err(|error| debug("blob parse", &error.to_string()))
        .map_err(|_| Error::BackupLayoutUnsupported)?;
    let root = root
        .into_dictionary()
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob", "root not a dictionary"))?;
    let objects = root
        .get("$objects")
        .and_then(plist::Value::as_array)
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob", "no $objects"))?;
    let index = root
        .get("$top")
        .and_then(plist::Value::as_dictionary)
        .and_then(|top| top.get("root"))
        .and_then(plist::Value::as_uid)
        .map(|uid| uid.get() as usize)
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob", "no $top.root uid"))?;
    let meta = objects
        .get(index)
        .and_then(plist::Value::as_dictionary)
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob", &format!("objects[{index}] not a dict")))?;
    let digest = match meta.get("Digest") {
        Some(plist::Value::Data(data)) => data.to_vec(),
        // Apple's NSKeyedArchiver refs the bytes object by UID:
        Some(plist::Value::Uid(uid)) => objects
            .get(uid.get() as usize)
            .and_then(plist::Value::as_data)
            .ok_or(Error::BackupLayoutUnsupported)
            .inspect_err(|_| debug("blob", "Digest uid target not data"))?
            .to_vec(),
        other => {
            debug("blob", &format!("Digest unexpected: {other:?}"));
            return Err(Error::BackupLayoutUnsupported);
        }
    };
    let size = meta
        .get("Size")
        .and_then(plist::Value::as_signed_integer)
        .ok_or(Error::BackupLayoutUnsupported)
        .inspect_err(|_| debug("blob", "no Size integer"))?;
    if size < 0 {
        debug("blob", "negative Size");
        return Err(Error::BackupLayoutUnsupported);
    }
    let relative_path = meta.get("RelativePath").and_then(|value| match value {
        plist::Value::Uid(uid) => objects
            .get(uid.get() as usize)
            .and_then(plist::Value::as_string)
            .map(str::to_owned),
        plist::Value::String(text) => Some(text.clone()),
        _ => None,
    });
    Ok(FileMetadata {
        digest,
        size: size as u64,
        relative_path,
    })
}
