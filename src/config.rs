//! The backup sets (`backupset`), the folders of each protected library
//! (`library`) and File History's own state (`global`).

use common::time::Ts;
use ese::Database;

use crate::record::{filetime, records};
use crate::strings::Strings;

/// Bytes of a FILETIME.
const FILETIME_SIZE: usize = 8;

/// One run of File History (a `backupset` row): entries and files name
/// these by id.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BackupSet {
    /// `id`, the row's key.
    pub id: i64,
    /// `timestamp`: when it ran (UTC).
    pub time: Option<Ts>,
}

/// A folder File History protects or excludes (a `library` row).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryFolder {
    /// `id`, the row's key.
    pub id: i64,
    /// `parentId`: the string id of its library.
    pub library_id: Option<i64>,
    /// `childId`: the string id of the folder.
    pub folder_id: Option<i64>,
    /// Its library: `*` and the library's GUID, or `?Exclude` for the
    /// folders left out.
    pub library: Option<String>,
    /// The folder (`?UP\Documents`, `?PP\Music`).
    pub folder: Option<String>,
    /// `tCreated`: the backup set from which it was protected.
    pub created_in: Option<i64>,
    /// `tVisible`: the last backup set it was part of; `2147483647` while
    /// it still is.
    pub visible_until: Option<i64>,
}

/// A value of File History's state (a `global` row).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Setting {
    /// `id`, the row's key.
    pub id: i64,
    /// `key`: `FirstBackupTime`, `LastBackupTime`, `LastBSet`,
    /// `TotalBytesOnDefaultTarget`, …; empty when unset.
    pub key: String,
    /// `value`, as stored (little-endian numbers, FILETIMEs, UTF-16 text);
    /// empty when unset.
    pub value: Vec<u8>,
}

impl Setting {
    /// The value as a FILETIME, when it is eight bytes, as for the keys
    /// ending in `Time` (`FirstBackupTime`, `LastBackupTime`,
    /// `LastCatalogBackupTime`), `LastFullElScan` and
    /// `EarliestConsecutiveElFailure`; `None` when zero.
    #[must_use]
    pub fn time(&self) -> Option<Ts> {
        let bytes: [u8; FILETIME_SIZE] = self.value.as_slice().try_into().ok()?;
        filetime(i64::from_le_bytes(bytes))
    }

    /// The value as a little-endian unsigned number, when it is one, two,
    /// four or eight bytes (`LastBSet`, `FullyProtectedFileRecords`,
    /// `TotalBytesOnDefaultTarget`).
    #[must_use]
    pub fn number(&self) -> Option<u64> {
        match self.value.len() {
            1 | 2 | 4 | 8 => {
                let mut bytes = [0; 8];
                bytes
                    .get_mut(..self.value.len())?
                    .copy_from_slice(&self.value);
                Some(u64::from_le_bytes(bytes))
            }
            _ => None,
        }
    }
}

/// Every `backupset` row, in id order.
pub(crate) fn backup_sets(db: &Database, problems: &mut Vec<String>) -> Vec<BackupSet> {
    records(db, "backupset", problems)
        .into_iter()
        .filter_map(|record| {
            Some(BackupSet {
                id: record.id(problems)?,
                time: record.filetime("timestamp"),
            })
        })
        .collect()
}

/// Every `library` row, in id order.
pub(crate) fn library_folders(
    db: &Database,
    strings: &Strings,
    problems: &mut Vec<String>,
) -> Vec<LibraryFolder> {
    let mut folders = Vec::new();
    for record in records(db, "library", problems) {
        let Some(id) = record.id(problems) else {
            continue;
        };
        let row = format!("library {id}");
        let library_id = record.integer("parentId");
        let folder_id = record.integer("childId");
        folders.push(LibraryFolder {
            id,
            library_id,
            folder_id,
            library: strings.resolve(library_id, &row, problems),
            folder: strings.resolve(folder_id, &row, problems),
            created_in: record.integer("tCreated"),
            visible_until: record.integer("tVisible"),
        });
    }
    folders
}

/// Every `global` row, in id order.
pub(crate) fn settings(db: &Database, problems: &mut Vec<String>) -> Vec<Setting> {
    records(db, "global", problems)
        .into_iter()
        .filter_map(|record| {
            Some(Setting {
                id: record.id(problems)?,
                key: record.text("key").unwrap_or_default().to_owned(),
                value: record.bytes("value").unwrap_or_default().to_vec(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(value: &[u8]) -> Setting {
        Setting {
            id: 1,
            key: "k".to_owned(),
            value: value.to_vec(),
        }
    }

    #[test]
    fn values_by_size() {
        let first_backup = setting(&[145, 41, 136, 69, 215, 190, 206, 1]);
        assert_eq!(
            first_backup.time().and_then(|t| t.to_iso8601()).as_deref(),
            Some("2013-10-01T18:51:43.4120593Z")
        );
        assert_eq!(setting(&[151, 0, 0, 0]).number(), Some(151));
        assert_eq!(setting(&[1]).number(), Some(1));
        assert_eq!(setting(&[0; 8]).time(), None);
        assert_eq!(setting(&[1, 2, 3]).number(), None);
        assert_eq!(setting(&[1, 2, 3, 4]).time(), None);
    }
}
