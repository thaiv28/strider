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
mod blobs;
mod commands;
mod fixture;
mod propagation;
mod queries;
mod sync;
mod trip_queries;
mod trips;
pub use blobs::*;
pub use sync::*;
pub use trip_queries::*;
pub use trips::{AddTripFood, Site};
pub mod reference;
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
const MIGRATIONS: &[&str] = &[
    include_str!("schema.sql"),
    include_str!("sync.sql"),
    include_str!("trips.sql"),
];
pub type Row = BTreeMap<String, Value>;
type Snapshot = BTreeMap<String, BTreeMap<String, Row>>;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeSet {
    pub tables: BTreeSet<String>,
}
struct Change {
    before: Snapshot,
    after: Snapshot,
    direct: Snapshot,
    trip_command: bool,
}
pub struct Store {
    connection: Connection,
    undo: Vec<Change>,
    redo: Vec<Change>,
    pull_generation: String,
    data_dir: PathBuf,
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
        {
            let data_dir = path
                .as_ref()
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();
            let mut store = Self::from_connection(Connection::open(path)?)?;
            store.data_dir = data_dir;
            Ok(store)
        }
    }
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }
    fn from_connection(mut connection: Connection) -> Result<Self> {
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
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
            pull_generation: String::new(),
            data_dir: std::env::temp_dir().join(format!("strider-{}", id())),
        })
    }
    /// Connection access for diagnostics, import preparation, and integration tests.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
    pub fn apply(&mut self, command: Command) -> Result<ChangeSet> {
        self.invalidate_pulled_history()?;
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let before = snapshot(&tx)?;
        let affected = commands::run(&tx, command, &self.data_dir)?;
        let direct = snapshot(&tx)?;
        let trip_command = affected.trip_command;
        propagation::run(&tx, affected)?;
        let after = snapshot(&tx)?;
        let changed = changes(&before, &after);
        append_outbox(&tx, &before, &direct)?;
        tx.commit()?;
        self.undo.push(Change {
            before,
            after,
            direct,
            trip_command,
        });
        self.redo.clear();
        Ok(changed)
    }
    pub fn execute(&mut self, command: Command) -> Result<()> {
        self.apply(command).map(|_| ())
    }
    pub fn undo(&mut self) -> Result<bool> {
        self.restore_history(false)
    }
    pub fn redo(&mut self) -> Result<bool> {
        self.restore_history(true)
    }
    fn restore_history(&mut self, redo: bool) -> Result<bool> {
        // Lock before checking the durable generation. A secondary pull cannot
        // commit between this check and the snapshot restoration.
        self.connection.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| {
            self.invalidate_pulled_history()?;
            let stack = if redo { &self.redo } else { &self.undo };
            let Some(change) = stack.last() else {
                return Ok(false);
            };
            restore(
                &self.connection,
                if redo { &change.after } else { &change.before },
                &change.before,
                &change.direct,
                change.trip_command,
            )?;
            Ok(true)
        })();
        match result {
            Ok(changed) => {
                if let Err(error) = self.connection.execute_batch("COMMIT") {
                    let _ = self.connection.execute_batch("ROLLBACK");
                    return Err(error.into());
                }
                if changed {
                    if redo {
                        if let Some(c) = self.redo.pop() {
                            self.undo.push(c);
                        }
                    } else if let Some(c) = self.undo.pop() {
                        self.redo.push(c);
                    }
                }
                Ok(changed)
            }
            Err(error) => {
                let _ = self.connection.execute_batch("ROLLBACK");
                Err(error)
            }
        }
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
            if *table == "trip_permit" && (new.contains_key(key) || !db.query_row("SELECT EXISTS(SELECT 1 FROM row_meta WHERE tbl='trip_permit' AND sync_id=?)",[key],|r|r.get::<_,bool>(0))?) { continue; }
            if *table == "trip_day"
                && db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_derived_day WHERE sync_id=? AND edited=0)",
                    [key],
                    |r| r.get::<_, bool>(0),
                )?
            {
                continue;
            }
            let (op, mut cols) = match new.get(key) {
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
            if *table == "trip_day"
                && let Some(r) = new.get(key)
            {
                let local: bool = db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_derived_day WHERE sync_id=? AND edited=1)",
                    [key],
                    |r| r.get(0),
                )?;
                if local {
                    cols = r.keys().cloned().collect();
                }
            }
            let json = format!(
                "[{}]",
                cols.iter()
                    .map(|c| format!("\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let trip_id = if *table == "trip" {
                Some(key.to_string())
            } else {
                new.get(key)
                    .or_else(|| old.get(key))
                    .and_then(|r| opt(r, "trip_id"))
            };
            db.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,base_seq,created_at,trip_id) VALUES(?1,?2,?3,?4,?5,(SELECT server_seq FROM row_meta WHERE tbl=?2 AND sync_id=?3),strftime('%Y-%m-%dT%H:%M:%fZ','now'),?6)",params![batch,table,key,op,json,trip_id])?;
        }
    }
    Ok(())
}
fn restore(
    db: &Connection,
    target: &Snapshot,
    original: &Snapshot,
    direct: &Snapshot,
    trip_command: bool,
) -> Result<()> {
    let before = snapshot(db)?;
    let tx = db;
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
                write(tx, table, key, r, true)?;
            }
        }
    }
    // Cascades may have removed unchanged child rows; restore those as well.
    for table in TABLES {
        for (key, r) in &target[*table] {
            if row(tx, table, key).is_err() {
                let mut r = r.clone();
                r.remove("sync_id");
                write(tx, table, key, r, true)?;
            }
        }
    }
    // Restoring permit metadata schedules the retained file for binary transfer.
    for (key, r) in &before["trip_permit"] {
        if !target["trip_permit"].contains_key(key) && target["trip"].contains_key(s(r, "trip_id"))
        {
            tx.execute(
                "INSERT OR IGNORE INTO permit_delete(trip_id) VALUES(?)",
                [s(r, "trip_id")],
            )?;
        }
    }
    for (key, r) in &target["trip_permit"] {
        if before["trip_permit"].get(key) != Some(r) {
            tx.execute(
                "DELETE FROM permit_delete WHERE trip_id=?",
                [s(r, "trip_id")],
            )?;
            tx.execute("INSERT INTO permit_upload(trip_id,sha256) VALUES(?,?) ON CONFLICT(trip_id) DO UPDATE SET sha256=excluded.sha256",params![s(r,"trip_id"),s(r,"sha256")])?;
        }
    }
    tx.execute(
        "DELETE FROM permit_upload WHERE trip_id NOT IN (SELECT trip_id FROM trip_permit)",
        [],
    )?;
    let mut wire_target = target.clone();
    for table in ["trip", "trip_day"].into_iter().filter(|_| trip_command) {
        let keys: BTreeSet<_> = before[table].keys().chain(target[table].keys()).collect();
        for key in keys {
            let old = original[table].get(key);
            let new = direct[table].get(key);
            if old == new {
                if let Some(row) = before[table].get(key) {
                    wire_target
                        .get_mut(table)
                        .expect("snapshot table")
                        .insert(key.clone(), row.clone());
                } else {
                    wire_target
                        .get_mut(table)
                        .expect("snapshot table")
                        .remove(key);
                }
            } else if let (Some(old), Some(new), Some(current), Some(restored)) =
                (old, new, before[table].get(key), target[table].get(key))
            {
                let mut out = current.clone();
                for (column, value) in restored {
                    if old.get(column) != new.get(column) {
                        out.insert(column.clone(), value.clone());
                    }
                }
                wire_target
                    .get_mut(table)
                    .expect("snapshot table")
                    .insert(key.clone(), out);
            }
        }
    }
    append_outbox(tx, &before, &wire_target)?;
    Ok(())
}
#[cfg(test)]
mod tests;
