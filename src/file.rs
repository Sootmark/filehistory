//! The `file` table: the files File History copied to its target, under
//! the folders they were copied from.

use ese::Database;

use crate::record::records;
use crate::strings::{Located, Strings};

/// A file copied to the backup target (a `file` row).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileRecord {
    /// `id`, the row's key: what [`Entry::file_record_id`](crate::Entry)
    /// points to.
    pub id: i64,
    /// `parentId`: the string id of the folder it was copied from.
    pub parent_id: Option<i64>,
    /// `childId`: the string id of its name.
    pub child_id: Option<i64>,
    /// The folder it was copied from, as on the disk
    /// (`C:\Users\Public\Pictures`).
    pub folder: Option<String>,
    /// Its name.
    pub name: Option<String>,
    /// Its path: the folder and the name.
    pub path: Option<String>,
    /// `state`, as stored.
    pub state: Option<i64>,
    /// `status`, as stored.
    pub status: Option<i64>,
    /// `fileSize`: its size in bytes.
    pub size: Option<i64>,
    /// `tQueued`: the backup set that queued it for copying (a
    /// [`BackupSet`](crate::BackupSet) id).
    pub queued_in: Option<i64>,
    /// `tCaptured`: the backup set that copied it.
    pub captured_in: Option<i64>,
    /// `tUpdated`: the backup set that last updated it.
    pub updated_in: Option<i64>,
}

/// Every `file` row, in id order.
pub(crate) fn read(
    db: &Database,
    strings: &Strings,
    problems: &mut Vec<String>,
) -> Vec<FileRecord> {
    let mut files = Vec::new();
    for record in records(db, "file", problems) {
        let Some(id) = record.id(problems) else {
            continue;
        };
        let parent_id = record.integer("parentId");
        let child_id = record.integer("childId");
        let Located { folder, name, path } =
            strings.locate(parent_id, child_id, &format!("file {id}"), problems);
        files.push(FileRecord {
            id,
            parent_id,
            child_id,
            folder,
            name,
            path,
            state: record.integer("state"),
            status: record.integer("status"),
            size: record.integer("fileSize"),
            queued_in: record.integer("tQueued"),
            captured_in: record.integer("tCaptured"),
            updated_in: record.integer("tUpdated"),
        });
    }
    files
}
