# filehistory

Windows File History catalogs (Windows 8 and later: `%LOCALAPPDATA%\Microsoft\Windows\FileHistory\Configuration\Catalog1.edb` and `Catalog2.edb`, and their copies on the backup drive), for forensics: every file and folder of the protected libraries File History saw, with its path, creation and modification times, attributes and USN, kept after the file was deleted; each backup run and its time; the folders protected and excluded. Read with [`sootmark-ese`](https://crates.io/crates/sootmark-ese), a reader of ESE databases written from the format; no other dependency but its sibling `sootmark-common` (times).

```toml
[dependencies]
sootmark-filehistory = "0.1"
```

```rust
let catalog = filehistory::read(&std::fs::read("Catalog1.edb")?)?;
for entry in &catalog.entries {
    println!("{:?} {:?} {:?}", entry.modified, entry.path, entry.usn);
}
```

## What you get

- `read(bytes)`: a `Catalog` of
  - `entries`: every `namespace` row, a version of a file or folder: its path (`?UP\Pictures\desktop.ini`, `?UP` the user's profile folder, `?PP` the public one), its folder and name with their string ids (`parentId`, `childId`), attributes, creation and modification times (UTC, to the 100 ns; `None` when zero), USN, status, the backup sets it is part of (`tCreated` to `tVisible`, `2147483647` while current) and its copy (`fileRecordId`). A folder's path is a string of its own, so a path is the folder's string and the name joined; no tree to walk;
  - `files`: every `file` row, a copy on the target: its path as on the disk (`C:\Users\Public\Pictures\desktop.ini`), size, state, status, and the backup sets that queued, copied and last updated it;
  - `backup_sets`: each run (`backupset`) and its time;
  - `library_folders`: each library's folders (`library`: `*` and the library's GUID, or `?Exclude`);
  - `settings`: File History's state (`global`: `FirstBackupTime`, `LastBackupTime`, `LastBSet`, `TotalBytesOnDefaultTarget`, …), read with `time()` or `number()`;
  - `strings`: the `string` table, by id;
  - lookups `file(id)`, `backup_set(id)`, `setting(key)`.
- `detect(bytes)`: whether a file is a catalog (an ESE database with `namespace` and `string`).
- Damage goes to `problems`, never a panic; only a file that isn't an ESE database, or lacks those two tables, is an error.

Not read here: the catalog's transaction logs, whose changes a dirty catalog may lack (`sootmark-ese` reports a dirty shutdown); the configuration files (`Config1.xml`); the copies themselves. `status`, `state` and the `global` values are kept as stored, undecoded.

## How it's checked

- plaso's `Catalog1.edb` (Apache-2.0, `tests/fixtures/plaso/`, see `NOTICE`): every one of the 2,713 events plaso's `esedb/file_history` plugin makes from its 1,373 entries (a creation and a modification event per entry, "Not a time" for the 33 with neither), field by field: identifier, parent identifier, attributes, USN, time to the 100 ns and its description (`tests/oracle/`, written from plaso's output). One difference, plaso's: its "original filename" is the string whose id is the entry's own id, which names another path or nothing; this crate joins the strings `parentId` and `childId` point to (entry 1: `?PP\Pictures\WP_20130728_004.jpg`, where plaso says `?UP\AppData\Local\Microsoft\Windows\FileHistory`). The test checks plaso's value is that string. The copies' paths show `?UP` and `?PP` are `C:\Users\<user>` and `C:\Users\Public`. `sootmark-ese` matches libesedb on every value of this database.
- Property tests: arbitrary bytes, and the catalog cut or damaged anywhere, never panic.

## Licence

MIT or Apache-2.0, at your option. The test catalog under `tests/fixtures/plaso/` keeps plaso's licence (Apache-2.0).
