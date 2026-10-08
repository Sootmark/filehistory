//! Windows File History catalogs (Windows 8 and later), for forensics:
//! every file and folder of the libraries File History protects, with its
//! path, times, attributes and change journal number, kept even after the
//! file was deleted; the backup runs; the folders protected.
//!
//! A catalog is an ESE database, read with `sootmark-ese`: the working copy
//! `%LOCALAPPDATA%\Microsoft\Windows\FileHistory\Configuration\Catalog1.edb`
//! (and `Catalog2.edb`), copied to the backup target under
//! `FileHistory\<user>\<computer>\Configuration\`. Its tables:
//!
//! - `string`: every path and name, once, by id. Folders are written with
//!   the profile they're in (`?UP\Pictures`: the user's profile, `?PP` the
//!   public one), or as on the disk (`C:\Users\Public\Pictures`);
//! - `namespace`: one row per version of a file or folder seen: the string
//!   ids of its folder's path (`parentId`) and of its name (`childId`),
//!   its attributes, creation and modification times (FILETIMEs), USN, the
//!   backup sets it is part of (`tCreated` to `tVisible`), and its copy in
//!   `file` (`fileRecordId`);
//! - `file`: the copies on the target: folder as on the disk, name, size
//!   and the backup sets that queued, copied and updated them;
//! - `backupset`: each run of File History and its time;
//! - `library`: the folders of each library (`*` and its GUID) and those
//!   excluded (`?Exclude`);
//! - `global`: File History's state, by key (`LastBackupTime`, …).
//!
//! A folder's path is a string of its own, so an entry's path is its
//! folder's string and its name joined: no walk up a tree is needed.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let catalog = filehistory::read(&std::fs::read("Catalog1.edb")?)?;
//! for entry in &catalog.entries {
//!     println!("{:?} {:?} {:?}", entry.modified, entry.path, entry.usn);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Damage is reported in `problems`, never a panic.

mod config;
mod file;
mod namespace;
mod record;
mod strings;

use ese::Database;

pub use config::{BackupSet, LibraryFolder, Setting};
pub use file::FileRecord;
pub use namespace::Entry;
pub use strings::Strings;

/// This crate's version, for records of what parsed them.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The tables a catalog has, besides those every ESE database has.
const CATALOG_TABLES: [&str; 2] = ["namespace", "string"];

/// A File History catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    /// The files and folders seen (`namespace`), in id order.
    pub entries: Vec<Entry>,
    /// The copies on the backup target (`file`), in id order.
    pub files: Vec<FileRecord>,
    /// The runs (`backupset`), in id order.
    pub backup_sets: Vec<BackupSet>,
    /// The folders protected and excluded (`library`), in id order.
    pub library_folders: Vec<LibraryFolder>,
    /// File History's state (`global`), in id order.
    pub settings: Vec<Setting>,
    /// The paths and names the other tables point to (`string`).
    pub strings: Strings,
    /// Damage in the database, and ids the string table lacks.
    pub problems: Vec<String>,
}

impl Catalog {
    /// The copy an entry's [`Entry::file_record_id`] points to.
    #[must_use]
    pub fn file(&self, id: i64) -> Option<&FileRecord> {
        self.files.iter().find(|file| file.id == id)
    }

    /// The backup set with this id.
    #[must_use]
    pub fn backup_set(&self, id: i64) -> Option<&BackupSet> {
        self.backup_sets.iter().find(|set| set.id == id)
    }

    /// The state value of this key (`LastBackupTime`).
    #[must_use]
    pub fn setting(&self, key: &str) -> Option<&Setting> {
        self.settings.iter().find(|setting| setting.key == key)
    }
}

/// Why a file can't be read as a File History catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Whether a file is a File History catalog: an ESE database with a
/// `namespace` and a `string` table.
#[must_use]
pub fn detect(data: &[u8]) -> bool {
    Database::open(data).is_ok_and(|db| is_catalog(&db))
}

/// Read a File History catalog (`Catalog1.edb`, `Catalog2.edb`).
///
/// # Errors
/// When it isn't an ESE database, or has no `namespace` or `string` table.
pub fn read(data: &[u8]) -> Result<Catalog, Error> {
    let db = Database::open(data).map_err(|e| Error(e.to_string()))?;
    if !is_catalog(&db) {
        return Err(Error("no namespace and string tables".to_owned()));
    }
    let mut problems = db.problems.clone();
    let strings = Strings::read(&db, &mut problems);
    Ok(Catalog {
        entries: namespace::read(&db, &strings, &mut problems),
        files: file::read(&db, &strings, &mut problems),
        backup_sets: config::backup_sets(&db, &mut problems),
        library_folders: config::library_folders(&db, &strings, &mut problems),
        settings: config::settings(&db, &mut problems),
        strings,
        problems,
    })
}

fn is_catalog(db: &Database) -> bool {
    CATALOG_TABLES.iter().all(|name| db.table(name).is_some())
}

#[cfg(test)]
mod tests {
    #[test]
    fn not_a_catalog() {
        assert!(!super::detect(b""));
        assert!(super::read(&[0; 4096]).is_err());
    }
}
