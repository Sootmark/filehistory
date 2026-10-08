//! A table's rows with their columns found by name, and the damage met
//! reading them.

use common::time::Ts;
use ese::{Database, Table, Value};

/// A row of a catalog table.
pub(crate) struct Record<'d> {
    table: &'d Table,
    /// The leaf page holding it, to say where damage is.
    page: u32,
    values: Vec<Value>,
}

impl Record<'_> {
    /// The value of column `name`, when the table has it.
    fn value(&self, name: &str) -> Option<&Value> {
        self.values.get(self.table.column_index(name)?)
    }

    /// Column `name` as an integer.
    pub(crate) fn integer(&self, name: &str) -> Option<i64> {
        self.value(name).and_then(Value::as_i64)
    }

    /// Column `name` as text.
    pub(crate) fn text(&self, name: &str) -> Option<&str> {
        self.value(name).and_then(Value::as_text)
    }

    /// Column `name` as bytes.
    pub(crate) fn bytes(&self, name: &str) -> Option<&[u8]> {
        self.value(name).and_then(Value::as_bytes)
    }

    /// Column `name` as a FILETIME; `None` when unset (NULL or zero).
    pub(crate) fn filetime(&self, name: &str) -> Option<Ts> {
        filetime(self.integer(name)?)
    }

    /// Its key (`id`), or a problem saying where the row is.
    pub(crate) fn id(&self, problems: &mut Vec<String>) -> Option<i64> {
        let id = self.integer("id");
        if id.is_none() {
            problems.push(format!(
                "{}: a row on page {} has no id, skipped",
                self.table.name, self.page
            ));
        }
        id
    }
}

/// A FILETIME stored as a `LongLong`; `None` when zero (never set). A
/// negative value is read as its unsigned bits (`-1` is the sentinel).
pub(crate) fn filetime(value: i64) -> Option<Ts> {
    (value != 0).then(|| Ts::from_filetime(value as u64))
}

/// Every row of table `name`, in key order; none when the table is
/// absent. Damage met reading them goes to `problems`, named by table.
pub(crate) fn records<'d>(
    db: &'d Database,
    name: &str,
    problems: &mut Vec<String>,
) -> Vec<Record<'d>> {
    let (Some(table), Ok(mut rows)) = (db.table(name), db.rows(name)) else {
        return Vec::new();
    };
    let records = rows
        .by_ref()
        .map(|row| Record {
            table,
            page: row.page,
            values: row.values,
        })
        .collect();
    problems.extend(rows.problems().iter().map(|p| format!("{name}: {p}")));
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::time::Semantic;

    #[test]
    fn zero_is_never_set() {
        assert_eq!(filetime(0), None);
        let time = filetime(130_226_257_456_957_745).and_then(|t| t.to_iso8601());
        assert_eq!(time.as_deref(), Some("2013-09-02T20:02:25.6957745Z"));
        assert_eq!(filetime(-1).map(|t| t.semantic()), Some(Semantic::Sentinel));
    }
}
