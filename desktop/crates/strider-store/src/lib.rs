//! Local workspace. Each command, derived propagation, and outbox batch is atomic.
use rusqlite::{Connection, OptionalExtension, params, types::Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
};
use strider_core::units::Dec;
use strider_core::{Cents, Grams};
use uuid::Uuid;
mod commands;
mod fixture;
mod propagation;
mod queries;
pub use commands::*;
pub use fixture::*;
pub use queries::*;
#[derive(Debug)]
pub enum StoreError {
    Sql(rusqlite::Error),
    Io(std::io::Error),
    Invalid(String),
}
pub type Error = StoreError;
pub type Result<T> = std::result::Result<T, StoreError>;
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::Invalid(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub const TABLES: &[&str] = &[
    "energy_params",
    "gear_category",
    "gear_item",
    "gear_component",
    "loadout",
    "wishlist_replacement",
    "wishlist_loadout",
    "loadout_item",
    "trail",
    "trip",
    "report_settings",
    "calendar_feed",
    "trip_permit",
    "trip_day",
    "trip_gear",
    "ingredient",
    "meal",
    "meal_ingredient",
    "trip_meal",
    "trip_consumable",
    "trip_gpx",
    "trip_campsite",
];
const MIGRATIONS: &[&str] = &[include_str!("schema.sql")];
pub type Row = BTreeMap<String, Value>;
type Snapshot = BTreeMap<String, BTreeMap<String, Row>>;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeSet {
    pub tables: BTreeSet<String>,
}
struct Change {
    before: Snapshot,
    after: Snapshot,
}
pub struct Store {
    connection: Connection,
    undo: Vec<Change>,
    redo: Vec<Change>,
}
pub fn default_path() -> Result<PathBuf> {
    Store::path_for_profile("dev")
}
impl Store {
    pub fn default_path() -> Result<PathBuf> {
        default_path()
    }
    pub fn path_for_profile(profile: &str) -> Result<PathBuf> {
        if profile.is_empty()
            || profile == "."
            || profile == ".."
            || !profile
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return invalid("Profile must be a single name containing letters, digits, '-' or '_'");
        }
        let home =
            std::env::var_os("HOME").ok_or_else(|| Error::Invalid("HOME is not set".into()))?;
        Ok(PathBuf::from(home)
            .join("Library/Application Support/Strider")
            .join(profile)
            .join("workspace.db"))
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(p) = path.as_ref().parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(p)?;
        }
        Self::from_connection(Connection::open(path)?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }
    fn from_connection(mut connection: Connection) -> Result<Self> {
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        let version =
            connection.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
        if version > MIGRATIONS.len() {
            return invalid(format!("Unsupported schema version {version}"));
        }
        // The disposable milestone-0 store also called itself version 1. Refuse it explicitly.
        if version == 1 {
            let has_sync: bool = connection.query_row(
                "SELECT count(*) FROM pragma_table_info('gear_item') WHERE name='sync_id'",
                [],
                |r| r.get::<_, i64>(0),
            )? > 0;
            if !has_sync {
                return invalid(
                    "Milestone-0 spike database requires --reset; it is not a synced workspace",
                );
            }
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = connection.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(Self {
            connection,
            undo: vec![],
            redo: vec![],
        })
    }
    /// Connection access for diagnostics, import preparation, and integration tests.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
    pub fn apply(&mut self, command: Command) -> Result<ChangeSet> {
        let tx = self.connection.transaction()?;
        let before = snapshot(&tx)?;
        let affected = commands::run(&tx, command)?;
        let direct = snapshot(&tx)?;
        propagation::run(&tx, affected)?;
        let after = snapshot(&tx)?;
        let changed = changes(&before, &after);
        append_outbox(&tx, &before, &direct)?;
        tx.commit()?;
        self.undo.push(Change { before, after });
        self.redo.clear();
        Ok(changed)
    }
    pub fn execute(&mut self, command: Command) -> Result<()> {
        self.apply(command).map(|_| ())
    }
    pub fn undo(&mut self) -> Result<bool> {
        let Some(c) = self.undo.last() else {
            return Ok(false);
        };
        restore(&mut self.connection, &c.before)?;
        if let Some(c) = self.undo.pop() {
            self.redo.push(c);
        }
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool> {
        let Some(c) = self.redo.last() else {
            return Ok(false);
        };
        restore(&mut self.connection, &c.after)?;
        if let Some(c) = self.redo.pop() {
            self.undo.push(c);
        }
        Ok(true)
    }
}
fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(message.into()))
}
fn id() -> String {
    Uuid::now_v7().to_string()
}
fn text(v: impl Into<String>) -> Value {
    Value::Text(v.into())
}
fn nullable(v: Option<String>) -> Value {
    v.map(text).unwrap_or(Value::Null)
}
fn number(v: Option<i32>) -> Value {
    v.map(|v| Value::Integer(v.into())).unwrap_or(Value::Null)
}
fn decimal(v: Option<Dec>) -> Value {
    v.map(|d| text(d.to_string())).unwrap_or(Value::Null)
}
fn s<'a>(r: &'a Row, key: &str) -> &'a str {
    match r.get(key) {
        Some(Value::Text(v)) => v,
        _ => "",
    }
}
fn opt(r: &Row, key: &str) -> Option<String> {
    match r.get(key) {
        Some(Value::Text(v)) => Some(v.clone()),
        _ => None,
    }
}
fn n(r: &Row, key: &str) -> i64 {
    match r.get(key) {
        Some(Value::Integer(v)) => *v,
        _ => 0,
    }
}
fn dec(r: &Row, key: &str) -> Result<Option<Dec>> {
    opt(r, key)
        .map(|v| {
            v.parse()
                .map_err(|_| Error::Invalid(format!("Invalid numeric in {key}")))
        })
        .transpose()
}
fn row(db: &Connection, table: &str, key: &str) -> Result<Row> {
    let mut stmt = db.prepare(&format!("SELECT * FROM {table} WHERE sync_id=?1"))?;
    let names: Vec<_> = stmt.column_names().map(|s| s.to_string()).collect();
    let result = stmt
        .query_row([key], |r| {
            let mut v = Row::new();
            for (i, k) in names.iter().enumerate() {
                v.insert(k.clone(), r.get(i)?);
            }
            Ok(v)
        })
        .optional()?;
    result.ok_or_else(|| Error::Invalid(format!("{table} row not found")))
}
fn rows(db: &Connection, table: &str) -> Result<Vec<Row>> {
    let mut stmt = db.prepare(&format!("SELECT * FROM {table} ORDER BY sync_id"))?;
    let names: Vec<_> = stmt.column_names().map(|s| s.to_string()).collect();
    let out = stmt
        .query_map([], |r| {
            let mut v = Row::new();
            for (i, k) in names.iter().enumerate() {
                v.insert(k.clone(), r.get(i)?);
            }
            Ok(v)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(out)
}
fn write(db: &Connection, table: &str, key: &str, values: Row, insert: bool) -> Result<()> {
    for (column, value) in &values {
        if let Value::Integer(n) = value
            && i32::try_from(*n).is_err()
        {
            return invalid(format!("{table}.{column} exceeds the server integer range"));
        }
    }
    let mut values = values;
    if insert {
        values.insert("sync_id".into(), text(key));
    }
    if values.is_empty() {
        return Ok(());
    }
    let keys: Vec<_> = values.keys().cloned().collect();
    let mut vals: Vec<Value> = values.into_values().collect();
    if insert {
        let qs = vec!["?"; keys.len()].join(",");
        db.execute(
            &format!("INSERT INTO {table} ({}) VALUES ({qs})", keys.join(",")),
            rusqlite::params_from_iter(vals),
        )?;
    } else {
        let setters = keys
            .iter()
            .map(|k| format!("{k}=?"))
            .collect::<Vec<_>>()
            .join(",");
        vals.push(text(key));
        if db.execute(
            &format!("UPDATE {table} SET {setters} WHERE sync_id=?"),
            rusqlite::params_from_iter(vals),
        )? == 0
        {
            return invalid(format!("{table} row not found"));
        }
    }
    Ok(())
}
fn fields<const N: usize>(v: [(&str, Value); N]) -> Row {
    v.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn delete(db: &Connection, table: &str, key: &str) -> Result<()> {
    row(db, table, key)?;
    db.execute(&format!("DELETE FROM {table} WHERE sync_id=?"), [key])?;
    Ok(())
}
fn snapshot(db: &Connection) -> Result<Snapshot> {
    TABLES
        .iter()
        .map(|t| {
            Ok((
                t.to_string(),
                rows(db, t)?
                    .into_iter()
                    .map(|r| (s(&r, "sync_id").to_string(), r))
                    .collect(),
            ))
        })
        .collect()
}
fn changes(a: &Snapshot, b: &Snapshot) -> ChangeSet {
    ChangeSet {
        tables: TABLES
            .iter()
            .filter(|t| a.get(**t) != b.get(**t))
            .map(|t| t.to_string())
            .collect(),
    }
}
fn append_outbox(db: &Connection, a: &Snapshot, b: &Snapshot) -> Result<()> {
    let batch = id();
    for table in TABLES {
        let empty = BTreeMap::new();
        let old = a.get(*table).unwrap_or(&empty);
        let new = b.get(*table).unwrap_or(&empty);
        let keys: BTreeSet<_> = old.keys().chain(new.keys()).collect();
        for key in keys {
            if old.get(key) == new.get(key) {
                continue;
            }
            let (op, cols) = match new.get(key) {
                None => ("delete", vec![]),
                Some(r) => (
                    "upsert",
                    r.keys()
                        .filter(|k| {
                            old.get(key)
                                .is_none_or(|before| before.get(*k) != r.get(*k))
                        })
                        .cloned()
                        .collect::<Vec<_>>(),
                ),
            };
            let json = format!(
                "[{}]",
                cols.iter()
                    .map(|c| format!("\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            db.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,base_seq,created_at) VALUES(?1,?2,?3,?4,?5,(SELECT server_seq FROM row_meta WHERE tbl=?2 AND sync_id=?3),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![batch,table,key,op,json])?;
        }
    }
    Ok(())
}
fn restore(db: &mut Connection, target: &Snapshot) -> Result<()> {
    let before = snapshot(db)?;
    let tx = db.transaction()?;
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;")?;
    // Only changed rows are touched. Deferred checks preserve non-cascading references.
    for table in TABLES.iter().rev() {
        for (key, r) in &before[*table] {
            if target[*table].get(key) != Some(r) {
                tx.execute(&format!("DELETE FROM {table} WHERE sync_id=?"), [key])?;
            }
        }
    }
    for table in TABLES {
        for (key, r) in &target[*table] {
            if before[*table].get(key) != Some(r) {
                let mut r = r.clone();
                r.remove("sync_id");
                write(&tx, table, key, r, true)?;
            }
        }
    }
    // Cascades may have removed unchanged child rows; restore those as well.
    for table in TABLES {
        for (key, r) in &target[*table] {
            if row(&tx, table, key).is_err() {
                let mut r = r.clone();
                r.remove("sync_id");
                write(&tx, table, key, r, true)?;
            }
        }
    }
    append_outbox(&tx, &before, target)?;
    tx.commit()?;
    Ok(())
}
#[cfg(test)]
mod tests;
