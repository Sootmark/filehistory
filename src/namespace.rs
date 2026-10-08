//! The `namespace` table: every file and folder of the protected
//! libraries File History has seen, one row per version.

use common::time::Ts;
use ese::Database;

use crate::record::records;
use crate::strings::{Located, Strings};

/// `FILE_ATTRIBUTE_DIRECTORY`.
const DIRECTORY: u32 = 0x10;

/// A file or folder as File History saw it (a `namespace` row).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    /// `id`, the row's key.
    pub id: i64,
    /// `parentId`: the string id of its folder's path (not a `namespace`
    /// id).
    pub parent_id: Option<i64>,
    /// `childId`: the string id of its name.
    pub child_id: Option<i64>,
    /// Its folder's path, as the catalog writes it: `?UP\Pictures` (`?UP`
    /// the user's profile folder, `?PP` the public one, `C:\Users`), empty
    /// for a drive.
    pub folder: Option<String>,
    /// Its name (`desktop.ini`, `Camera roll`, `C:`).
    pub name: Option<String>,
    /// Its path: the folder and the name (`?PP\Pictures\desktop.ini`).
    pub path: Option<String>,
    /// `status`, as stored.
    pub status: Option<i64>,
    /// `fileAttrib`: its Windows file attributes (`0x10` a folder, `0x20`
    /// archive).
    pub attributes: Option<u32>,
    /// `fileCreated`: when the file was created (UTC); `None` when unset.
    pub created: Option<Ts>,
    /// `fileModified`: when its content was last modified (UTC); `None`
    /// when unset.
    pub modified: Option<Ts>,
    /// `usn`: its NTFS change journal number when seen.
    pub usn: Option<i64>,
    /// `tCreated`: the backup set that first saw this version (a
    /// [`BackupSet`](crate::BackupSet) id).
    pub created_in: Option<i64>,
    /// `tVisible`: the last backup set this version is part of; `2147483647`
    /// while it still is.
    pub visible_until: Option<i64>,
    /// `fileRecordId`: its copy in the `file` table (a
    /// [`FileRecord`](crate::FileRecord) id); `0` for none, as for folders.
    pub file_record_id: Option<i64>,
}

impl Entry {
    /// Whether it is a folder (its attributes say so).
    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.attributes
            .is_some_and(|attributes| attributes & DIRECTORY != 0)
    }
}

/// Every `namespace` row, in id order.
pub(crate) fn read(db: &Database, strings: &Strings, problems: &mut Vec<String>) -> Vec<Entry> {
    let mut entries = Vec::new();
    for record in records(db, "namespace", problems) {
        let Some(id) = record.id(problems) else {
            continue;
        };
        let parent_id = record.integer("parentId");
        let child_id = record.integer("childId");
        let Located { folder, name, path } =
            strings.locate(parent_id, child_id, &format!("namespace {id}"), problems);
        entries.push(Entry {
            id,
            parent_id,
            child_id,
            folder,
            name,
            path,
            status: record.integer("status"),
            attributes: record
                .integer("fileAttrib")
                .and_then(|a| u32::try_from(a).ok()),
            created: record.filetime("fileCreated"),
            modified: record.filetime("fileModified"),
            usn: record.integer("usn"),
            created_in: record.integer("tCreated"),
            visible_until: record.integer("tVisible"),
            file_record_id: record.integer("fileRecordId"),
        });
    }
    entries
}
