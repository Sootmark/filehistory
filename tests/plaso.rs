//! plaso's `Catalog1.edb` (Apache-2.0, `tests/fixtures/plaso/`, stored
//! gzip-compressed): every one of the 2,713 events plaso's
//! `esedb/file_history` plugin makes from its 1,373 `namespace` rows, read
//! the same (`tests/oracle/plaso.tsv.gz`, written from plaso's output):
//! identifier, parent identifier, attributes, USN, and each time to the
//! 100 ns with its description (a creation and a modification event per
//! row, one "Not a time" event for the 33 rows with neither).
//!
//! One difference, plaso's: its "original filename" is the string whose id
//! is the row's own id, not the string of the row's name (`childId`) under
//! its folder (`parentId`). The two id spaces only meet by chance, so
//! plaso names entry 1, `?PP\Pictures\WP_20130728_004.jpg`, after string 1,
//! `?UP\AppData\Local\Microsoft\Windows\FileHistory`, and leaves the 379
//! entries whose id no string has unnamed. This crate's
//! [`Entry::path`](filehistory::Entry) is the folder and the name; the test
//! checks plaso's value against [`Catalog::strings`](filehistory::Catalog)
//! by the row's id, to show where it comes from.

use std::collections::BTreeSet;
use std::io::Read;

use common::time::Ts;
use filehistory::{Catalog, Entry};

/// FILETIME ticks at the Unix epoch.
const FILETIME_UNIX_EPOCH: i64 = 116_444_736_000_000_000;

fn gunzip(path: &str) -> Vec<u8> {
    let compressed = std::fs::read(format!("{}/tests/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut data = Vec::new();
    common::gzip::Decoder::new(compressed.as_slice())
        .read_to_end(&mut data)
        .unwrap();
    data
}

fn catalog() -> Catalog {
    let data = gunzip("fixtures/plaso/Catalog1.edb.gz");
    assert!(filehistory::detect(&data));
    filehistory::read(&data).unwrap()
}

/// A time as the FILETIME plaso writes.
fn filetime(time: Ts) -> String {
    time.ticks()
        .map(|ticks| (ticks + FILETIME_UNIX_EPOCH).to_string())
        .unwrap_or_default()
}

fn text<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// plaso's events for an entry, as `tests/oracle/plaso_tsv.py` writes them.
fn events(catalog: &Catalog, entry: &Entry) -> Vec<String> {
    let fields = [
        entry.id.to_string(),
        text(entry.parent_id),
        text(catalog.strings.get(entry.id)),
        text(entry.attributes),
        text(entry.usn),
    ]
    .join("\t");
    let times: Vec<(String, &str)> = [
        (entry.created, "Creation Time"),
        (entry.modified, "Content Modification Time"),
    ]
    .into_iter()
    .filter_map(|(time, description)| Some((filetime(time?), description)))
    .collect();
    if times.is_empty() {
        return vec![format!("{fields}\t\tNot a time")];
    }
    times
        .into_iter()
        .map(|(time, description)| format!("{fields}\t{time}\t{description}"))
        .collect()
}

#[test]
fn every_event_as_plaso_reads_it() {
    let catalog = catalog();
    assert_eq!(catalog.problems, Vec::<String>::new());
    assert_eq!(catalog.entries.len(), 1373);
    let mut got: Vec<String> = catalog
        .entries
        .iter()
        .flat_map(|entry| events(&catalog, entry))
        .collect();
    got.sort();
    let oracle = String::from_utf8(gunzip("oracle/plaso.tsv.gz")).unwrap();
    let expected: Vec<&str> = oracle.lines().collect();
    assert_eq!(got.len(), 2713);
    assert_eq!(got, expected);
}

#[test]
fn paths_are_the_folder_and_the_name() {
    let catalog = catalog();
    let entry = &catalog.entries[0];
    assert_eq!(entry.id, 1);
    assert_eq!(
        entry.path.as_deref(),
        Some(r"?PP\Pictures\WP_20130728_004.jpg")
    );
    // plaso's name for it: string 1.
    assert_eq!(
        catalog.strings.get(1),
        Some(r"?UP\AppData\Local\Microsoft\Windows\FileHistory")
    );
    let folder = catalog.entries.iter().find(|e| e.id == 356).unwrap();
    assert!(folder.is_folder());
    assert_eq!(
        folder.path.as_deref(),
        Some(r"?UP\Pictures\From Donald's Windows Phone\Camera roll")
    );
    assert_eq!(folder.created_in, Some(67));
    assert_eq!(folder.visible_until, Some(87));
    // A drive sits under the empty root.
    let drive = catalog.entries.iter().find(|e| e.id == 170).unwrap();
    assert_eq!(
        (drive.folder.as_deref(), drive.path.as_deref()),
        (Some(""), Some("C:"))
    );
    assert_eq!((drive.created, drive.modified), (None, None));
    let unnamed = catalog
        .entries
        .iter()
        .filter(|e| catalog.strings.get(e.id).is_none())
        .count();
    assert_eq!((catalog.strings.len(), unnamed), (994, 379));
    assert!(catalog.entries.iter().all(|e| e.path.is_some()));
}

#[test]
fn copies_on_the_target() {
    let catalog = catalog();
    assert_eq!(catalog.files.len(), 912);
    let entry = &catalog.entries[0];
    let copy = catalog.file(entry.file_record_id.unwrap()).unwrap();
    assert_eq!(
        copy.path.as_deref(),
        Some(r"C:\Users\Public\Pictures\WP_20130728_004.jpg")
    );
    assert_eq!(
        (copy.size, copy.state, copy.status, copy.captured_in),
        (Some(2_650_218), Some(4), Some(8), Some(1))
    );
    let with_copies = catalog
        .entries
        .iter()
        .filter(|e| {
            e.file_record_id
                .is_some_and(|id| catalog.file(id).is_some())
        })
        .count();
    assert_eq!(with_copies, 912);
}

#[test]
fn backup_sets_libraries_and_state() {
    let catalog = catalog();
    assert_eq!(catalog.backup_sets.len(), 150);
    let first = catalog.backup_set(1).unwrap();
    assert_eq!(
        first.time.and_then(|t| t.to_iso8601()).as_deref(),
        Some("2013-10-01T18:48:18.0890000Z")
    );
    let iso = |key: &str| {
        catalog
            .setting(key)
            .and_then(filehistory::Setting::time)
            .and_then(|t| t.to_iso8601())
    };
    assert_eq!(
        iso("FirstBackupTime").as_deref(),
        Some("2013-10-01T18:51:43.4120593Z")
    );
    assert!(iso("LastBackupTime").is_some());
    let last_set = catalog
        .setting("LastBSet")
        .and_then(filehistory::Setting::number);
    assert_eq!(last_set, Some(151));
    assert_eq!(catalog.settings.len(), 20);

    assert_eq!(catalog.library_folders.len(), 14);
    let excluded: BTreeSet<&str> = catalog
        .library_folders
        .iter()
        .filter(|f| f.library.as_deref() == Some("?Exclude"))
        .filter_map(|f| f.folder.as_deref())
        .collect();
    assert!(excluded.contains(r"?UP\AppData\Local\Microsoft\Windows\FileHistory"));
}
