//! The `string` table: every folder path, file name and library the other
//! tables name, stored once and pointed to by id.

use std::collections::BTreeMap;

use ese::Database;

use crate::record::records;

/// The `string` table, by id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Strings(BTreeMap<i64, String>);

impl Strings {
    /// Read the `string` table (empty when absent).
    pub(crate) fn read(db: &Database, problems: &mut Vec<String>) -> Self {
        let mut strings = BTreeMap::new();
        for record in records(db, "string", problems) {
            let Some(id) = record.id(problems) else {
                continue;
            };
            match record.text("string") {
                Some(text) => {
                    strings.insert(id, text.to_owned());
                }
                None => problems.push(format!("string {id}: no text")),
            }
        }
        Self(strings)
    }

    /// The string with this id.
    #[must_use]
    pub fn get(&self, id: i64) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }

    /// Every string with its id, by id.
    pub fn iter(&self) -> impl Iterator<Item = (i64, &str)> {
        self.0.iter().map(|(&id, text)| (id, text.as_str()))
    }

    /// How many strings there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The string a row's column points to, or a problem naming the row
    /// when the table lacks it.
    pub(crate) fn resolve(
        &self,
        id: Option<i64>,
        row: &str,
        problems: &mut Vec<String>,
    ) -> Option<String> {
        let id = id?;
        let text = self.get(id);
        if text.is_none() {
            problems.push(format!("{row}: string {id} not in the string table"));
        }
        text.map(str::to_owned)
    }

    /// The folder `parent_id` and the name `child_id` point to, joined;
    /// a problem naming `row` for each the table lacks.
    pub(crate) fn locate(
        &self,
        parent_id: Option<i64>,
        child_id: Option<i64>,
        row: &str,
        problems: &mut Vec<String>,
    ) -> Located {
        let folder = self.resolve(parent_id, row, problems);
        let name = self.resolve(child_id, row, problems);
        let path = folder
            .as_deref()
            .zip(name.as_deref())
            .map(|(folder, name)| join(folder, name));
        Located { folder, name, path }
    }
}

/// A row's folder and name, from the strings its `parentId` and `childId`
/// point to, and the path they make.
pub(crate) struct Located {
    pub(crate) folder: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) path: Option<String>,
}

/// A folder's path and a name in it, joined with `\`; the name alone under
/// the empty root (`""` and `C:` make `C:`).
#[must_use]
pub(crate) fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else if folder.ends_with('\\') {
        format!("{folder}{name}")
    } else {
        format!("{folder}\\{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_join_with_a_backslash() {
        assert_eq!(join("", "C:"), "C:");
        assert_eq!(join("C:", "Users"), r"C:\Users");
        assert_eq!(join(r"C:\", "Users"), r"C:\Users");
        assert_eq!(
            join(r"?UP\Pictures", "Camera roll"),
            r"?UP\Pictures\Camera roll"
        );
    }

    #[test]
    fn a_missing_string_is_a_problem() {
        let strings = Strings(BTreeMap::from([(1, "C:".to_owned())]));
        let mut problems = Vec::new();
        assert_eq!(
            strings.resolve(Some(1), "namespace 5", &mut problems),
            Some("C:".to_owned())
        );
        assert_eq!(strings.resolve(None, "namespace 5", &mut problems), None);
        assert_eq!(strings.resolve(Some(2), "namespace 5", &mut problems), None);
        assert_eq!(problems, ["namespace 5: string 2 not in the string table"]);
    }
}
