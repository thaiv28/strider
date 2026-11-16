//! Local fixture store. One connection; commands are transactional and undo is in memory.
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fmt,
    path::{Path, PathBuf},
};
use strider_core::Grams;
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    Sql(rusqlite::Error),
    Io(std::io::Error),
    Invalid(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::Invalid(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    Current,
    Retired,
    All,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GearFilter {
    pub status: StatusFilter,
    pub query: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub weight: Grams,
    pub quantity: i32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearRow {
    pub id: String,
    pub name: String,
    pub weight: Option<Grams>,
    pub quantity: i32,
    pub class: String,
    pub status: String,
    pub is_kit: bool,
    pub components: Vec<Component>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: Option<String>,
    pub name: String,
    pub rows: Vec<GearRow>,
    pub base: Grams,
    pub consumable: Grams,
    pub worn: Grams,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Totals {
    pub base: Grams,
    pub worn: Grams,
    pub consumable: Grams,
    pub items: usize,
}
#[derive(Debug, Clone)]
pub enum Command {
    UpdateItem {
        id: String,
        name: String,
        weight: Option<Grams>,
        quantity: i32,
    },
    MoveItem {
        id: String,
        to_category: Option<String>,
        before: Option<String>,
    },
    DeleteItem {
        id: String,
    },
}
#[derive(Debug, Clone)]
struct StoredItem {
    row: GearRow,
    category: Option<String>,
    order: i32,
}
type Snapshot = Vec<StoredItem>;
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
    let home = std::env::var_os("HOME").ok_or_else(|| Error::Invalid("HOME is not set".into()))?;
    Ok(PathBuf::from(home).join("Library/Application Support/Strider/spike/workspace.db"))
}
impl Store {
    pub fn default_path() -> Result<PathBuf> {
        default_path()
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }
    fn from_connection(connection: Connection) -> Result<Self> {
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        let version: i32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err(Error::Invalid(format!(
                "Unsupported schema version {version}"
            )));
        }
        if version == 0 {
            connection.execute_batch("BEGIN;
            CREATE TABLE gear_category(id TEXT PRIMARY KEY,name TEXT NOT NULL,sort_order INTEGER NOT NULL);
            CREATE TABLE gear_item(id TEXT PRIMARY KEY,category_id TEXT REFERENCES gear_category(id),name TEXT NOT NULL,weight_g INTEGER,quantity INTEGER NOT NULL DEFAULT 1,sort_order INTEGER NOT NULL,default_weight_class TEXT NOT NULL CHECK(default_weight_class IN ('base','worn','consumable')),status TEXT NOT NULL CHECK(status IN ('current','retired','wishlist')),is_kit INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE gear_component(id TEXT PRIMARY KEY,gear_item_id TEXT NOT NULL REFERENCES gear_item(id) ON DELETE CASCADE,name TEXT NOT NULL,weight_g INTEGER NOT NULL,quantity INTEGER NOT NULL DEFAULT 1);
            PRAGMA user_version=1; COMMIT;")?;
        }
        Ok(Self {
            connection,
            undo: vec![],
            redo: vec![],
        })
    }
    pub fn gear_groups(&self, filter: &GearFilter) -> Result<Vec<Group>> {
        let mut categories: Vec<(Option<String>, String)> = self
            .connection
            .prepare("SELECT id,name FROM gear_category ORDER BY sort_order,id")?
            .query_map([], |r| Ok((Some(r.get(0)?), r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        categories.push((None, "Other".into()));
        let rows = snapshot(&self.connection)?;
        let query = filter.query.to_lowercase();
        let mut groups = Vec::new();
        for (id, name) in categories {
            let mut group = Group {
                id: id.clone(),
                name,
                rows: vec![],
                base: Grams(0),
                consumable: Grams(0),
                worn: Grams(0),
            };
            let category_matches = group.name.to_lowercase().contains(&query);
            for item in &rows {
                let row = &item.row;
                let status_matches = match filter.status {
                    StatusFilter::Current => row.status == "current",
                    StatusFilter::Retired => row.status == "retired",
                    StatusFilter::All => row.status != "wishlist",
                };
                if item.category != id
                    || !status_matches
                    || (!category_matches && !row.name.to_lowercase().contains(&query))
                {
                    continue;
                }
                let weight = weighted(row)?;
                match row.class.as_str() {
                    "worn" => group.worn = checked_total(group.worn, weight)?,
                    "consumable" => group.consumable = checked_total(group.consumable, weight)?,
                    _ => group.base = checked_total(group.base, weight)?,
                }
                group.rows.push(row.clone());
            }
            if !group.rows.is_empty() {
                groups.push(group);
            }
        }
        Ok(groups)
    }
    pub fn totals(&self) -> Result<Totals> {
        let mut totals = Totals::default();
        for group in self.gear_groups(&GearFilter::default())? {
            totals.base = checked_total(totals.base, group.base)?;
            totals.worn = checked_total(totals.worn, group.worn)?;
            totals.consumable = checked_total(totals.consumable, group.consumable)?;
            totals.items += group.rows.len();
        }
        Ok(totals)
    }
    pub fn execute(&mut self, command: Command) -> Result<()> {
        let before = snapshot(&self.connection)?;
        let tx = self.connection.transaction()?;
        match command {
            Command::UpdateItem {
                id,
                name,
                weight,
                quantity,
            } => {
                if name.trim().is_empty() || quantity < 1 || weight.is_some_and(|g| g.0 < 0) {
                    return Err(Error::Invalid(
                        "Name, nonnegative weight, and positive quantity are required".into(),
                    ));
                }
                if tx.execute(
                    "UPDATE gear_item SET name=?1,weight_g=?2,quantity=?3 WHERE id=?4",
                    params![name, weight.map(|w| w.0), quantity, id],
                )? == 0
                {
                    return Err(Error::Invalid("Item not found".into()));
                }
            }
            Command::DeleteItem { id } => {
                if tx.execute("DELETE FROM gear_item WHERE id=?1", [id])? == 0 {
                    return Err(Error::Invalid("Item not found".into()));
                }
            }
            Command::MoveItem {
                id,
                to_category,
                before,
            } => {
                let source: Option<Option<String>> = tx
                    .query_row(
                        "SELECT category_id FROM gear_item WHERE id=?1",
                        [&id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let source = source.ok_or_else(|| Error::Invalid("Item not found".into()))?;
                let mut ids = category_ids(&tx, &to_category)?;
                ids.retain(|v| v != &id);
                let position = match before {
                    Some(ref before_id) if before_id == &id => {
                        return Err(Error::Invalid("Cannot insert an item before itself".into()));
                    }
                    Some(before_id) => {
                        ids.iter().position(|v| v == &before_id).ok_or_else(|| {
                            Error::Invalid("Insertion item is not in destination".into())
                        })?
                    }
                    None => ids.len(),
                };
                ids.insert(position, id.clone());
                tx.execute(
                    "UPDATE gear_item SET category_id=?1 WHERE id=?2",
                    params![to_category, id],
                )?;
                normalize(&tx, &ids)?;
                if source != to_category {
                    normalize(&tx, &category_ids(&tx, &source)?)?;
                }
            }
        }
        let after = snapshot(&tx)?;
        // Reject edits before commit if a row or an All-status subtotal cannot fit
        // the canonical i32 unit. This also keeps every undo state renderable.
        let mut class_totals = [Grams(0); 3];
        for item in &after {
            let class = match item.row.class.as_str() {
                "worn" => 1,
                "consumable" => 2,
                _ => 0,
            };
            class_totals[class] = checked_total(class_totals[class], weighted(&item.row)?)?;
        }
        tx.commit()?;
        self.undo.push(Change { before, after });
        self.redo.clear();
        Ok(())
    }
    pub fn undo(&mut self) -> Result<bool> {
        let Some(change) = self.undo.last() else {
            return Ok(false);
        };
        restore(&mut self.connection, &change.before)?;
        if let Some(change) = self.undo.pop() {
            self.redo.push(change);
        }
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool> {
        let Some(change) = self.redo.last() else {
            return Ok(false);
        };
        restore(&mut self.connection, &change.after)?;
        if let Some(change) = self.redo.pop() {
            self.undo.push(change);
        }
        Ok(true)
    }
}
fn weighted(row: &GearRow) -> Result<Grams> {
    row.weight
        .unwrap_or_default()
        .0
        .checked_mul(row.quantity)
        .map(Grams)
        .ok_or_else(|| Error::Invalid("Weight × quantity exceeds the supported gram range".into()))
}
fn checked_total(a: Grams, b: Grams) -> Result<Grams> {
    a.0.checked_add(b.0)
        .map(Grams)
        .ok_or_else(|| Error::Invalid("Total weight exceeds the supported gram range".into()))
}
fn category_ids(connection: &Connection, category: &Option<String>) -> Result<Vec<String>> {
    Ok(connection
        .prepare("SELECT id FROM gear_item WHERE category_id IS ?1 ORDER BY sort_order,id")?
        .query_map([category], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?)
}
fn normalize(connection: &Connection, ids: &[String]) -> Result<()> {
    for (order, id) in ids.iter().enumerate() {
        connection.execute(
            "UPDATE gear_item SET sort_order=?1 WHERE id=?2",
            params![order as i32, id],
        )?;
    }
    Ok(())
}
fn snapshot(connection: &Connection) -> Result<Snapshot> {
    let mut items: Snapshot = connection.prepare("SELECT id,category_id,name,weight_g,quantity,sort_order,default_weight_class,status,is_kit FROM gear_item ORDER BY sort_order,id")?.query_map([],|r|Ok(StoredItem {
        category:r.get(1)?,order:r.get(5)?,row:GearRow{id:r.get(0)?,name:r.get(2)?,weight:r.get::<_,Option<i32>>(3)?.map(Grams),quantity:r.get(4)?,class:r.get(6)?,status:r.get(7)?,is_kit:r.get(8)?,components:vec![]}
    }))?.collect::<std::result::Result<_,_>>()?;
    let mut components: std::collections::HashMap<String, Vec<Component>> =
        std::collections::HashMap::new();
    let mut stmt = connection.prepare(
        "SELECT gear_item_id,id,name,weight_g,quantity FROM gear_component ORDER BY rowid",
    )?;
    let component_rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            Component {
                id: r.get(1)?,
                name: r.get(2)?,
                weight: Grams(r.get(3)?),
                quantity: r.get(4)?,
            },
        ))
    })?;
    for row in component_rows {
        let (id, component) = row?;
        components.entry(id).or_default().push(component);
    }
    for item in &mut items {
        item.row.components = components.remove(&item.row.id).unwrap_or_default();
    }
    Ok(items)
}
fn insert_item(connection: &Connection, item: &StoredItem) -> Result<()> {
    let r = &item.row;
    connection.execute(
        "INSERT INTO gear_item VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            r.id,
            item.category,
            r.name,
            r.weight.map(|w| w.0),
            r.quantity,
            item.order,
            r.class,
            r.status,
            r.is_kit
        ],
    )?;
    for c in &r.components {
        connection.execute(
            "INSERT INTO gear_component VALUES(?1,?2,?3,?4,?5)",
            params![c.id, r.id, c.name, c.weight.0, c.quantity],
        )?;
    }
    Ok(())
}
fn restore(connection: &mut Connection, items: &Snapshot) -> Result<()> {
    let tx = connection.transaction()?;
    tx.execute("DELETE FROM gear_item", [])?;
    for item in items {
        insert_item(&tx, item)?;
    }
    tx.commit()?;
    Ok(())
}
const CATEGORIES: [&str; 12] = [
    "Shelter",
    "Sleep",
    "Storage",
    "Kitchen",
    "Water",
    "Clothing",
    "Worn",
    "Electronics",
    "Safety",
    "Tools & Repair",
    "Health & Hygiene",
    "Consumables",
];
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }
}
// A fixed fixture timestamp plus monotonic payload keeps every identity stable
// across resets while preserving the UUIDv7 version and RFC variant bits.
struct FixtureIds(u64);
impl FixtureIds {
    fn next(&mut self) -> String {
        self.0 += 1;
        Uuid::from_u128(
            (1_700_000_000_000u128 << 80) | (7u128 << 76) | (2u128 << 62) | u128::from(self.0),
        )
        .to_string()
    }
}
/// Seed an empty store with deterministic UUIDv7 identities and fixture values.
pub fn seed(store: &mut Store, items: usize) -> Result<()> {
    let count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM gear_item", [], |r| r.get(0))?;
    if count != 0 {
        return Ok(());
    }
    let tx = store.connection.transaction()?;
    tx.execute("DELETE FROM gear_category", [])?;
    let mut ids = FixtureIds(0);
    let mut category_ids = Vec::new();
    for (index, name) in CATEGORIES.iter().enumerate() {
        let id = ids.next();
        tx.execute(
            "INSERT INTO gear_category VALUES(?1,?2,?3)",
            params![id, name, index as i32],
        )?;
        category_ids.push(id);
    }
    let names: [&[&str]; 12] = [
        &["tarp", "tent", "groundsheet", "tent stakes", "guyline"],
        &[
            "quilt 20°F",
            "sleeping bag",
            "sleeping pad",
            "pillow",
            "pad straps",
        ],
        &[
            "backpack",
            "dry bag",
            "stuff sack",
            "pack liner",
            "hipbelt pocket",
        ],
        &["stove", "titanium pot", "spoon", "fuel stand", "windshield"],
        &[
            "water bottle",
            "water filter",
            "hydration bladder",
            "water bag",
            "purification drops",
        ],
        &[
            "rain jacket",
            "fleece",
            "down jacket",
            "hiking socks",
            "thermal leggings",
        ],
        &[
            "trail runners",
            "sun hat",
            "hiking shirt",
            "shorts",
            "trekking poles",
        ],
        &[
            "headlamp",
            "power bank",
            "charging cable",
            "satellite messenger",
            "camera",
        ],
        &[
            "first aid pouch",
            "emergency blanket",
            "whistle",
            "compass",
            "rescue beacon",
        ],
        &[
            "repair tape",
            "multitool",
            "sewing kit",
            "cord",
            "patch kit",
        ],
        &[
            "toothbrush",
            "soap",
            "trowel",
            "hand sanitizer",
            "sunscreen",
        ],
        &["trail mix", "oatmeal", "coffee", "electrolytes", "dinner"],
    ];
    let adjectives = [
        "Ultralight",
        "Compact",
        "Alpine",
        "Lightweight",
        "Trail",
        "Summit",
        "Classic",
        "Packable",
        "Minimal",
        "Durable",
    ];
    let mut rng = Lcg(2718281828);
    for i in 0..items {
        let category = i % CATEGORIES.len();
        let name = format!(
            "{} {}",
            adjectives[rng.next() as usize % adjectives.len()],
            names[category][rng.next() as usize % names[category].len()]
        );
        let status_roll = rng.next() % 100;
        let status = if status_roll < 5 {
            "retired"
        } else if status_roll < 8 {
            "wishlist"
        } else {
            "current"
        };
        let quantity = if rng.next() % 100 < 5 {
            2 + (rng.next() % 3) as i32
        } else {
            1
        };
        let is_kit = rng.next() % 100 < 2;
        let mut components = Vec::new();
        if is_kit {
            for j in 0..3 + rng.next() % 6 {
                components.push(Component {
                    id: ids.next(),
                    name: format!(
                        "{} component {}",
                        names[category][j as usize % names[category].len()],
                        j + 1
                    ),
                    weight: Grams(5 + (rng.next() % 86) as i32),
                    quantity: 1 + (rng.next() % 2) as i32,
                });
            }
        }
        let weight = if is_kit {
            components.iter().map(|c| c.weight * c.quantity).sum()
        } else {
            Grams(5 + (rng.next() % 1496) as i32)
        };
        insert_item(
            &tx,
            &StoredItem {
                category: Some(category_ids[category].clone()),
                order: (i / 12) as i32,
                row: GearRow {
                    id: ids.next(),
                    name,
                    weight: Some(weight),
                    quantity,
                    class: match category {
                        6 => "worn",
                        11 => "consumable",
                        _ => "base",
                    }
                    .into(),
                    status: status.into(),
                    is_kit,
                    components,
                },
            },
        )?;
    }
    tx.commit()?;
    store.undo.clear();
    store.redo.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Store {
        let mut s = Store::in_memory().unwrap();
        seed(&mut s, 2000).unwrap();
        s
    }
    fn all() -> GearFilter {
        GearFilter {
            status: StatusFilter::All,
            query: String::new(),
        }
    }
    #[test]
    fn fixture_identities_and_values_are_reproducible() {
        let first = fixture();
        let second = fixture();
        assert_eq!(
            first.gear_groups(&all()).unwrap(),
            second.gear_groups(&all()).unwrap()
        );
        let a = snapshot(&first.connection).unwrap();
        let b = snapshot(&second.connection).unwrap();
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.row, b.row);
            assert_eq!(a.category, b.category);
            assert_eq!(a.order, b.order);
            for id in std::iter::once(&a.row.id).chain(a.row.components.iter().map(|c| &c.id)) {
                let uuid = Uuid::parse_str(id).unwrap();
                assert_eq!(uuid.get_version_num(), 7);
                assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
            }
        }
    }
    #[test]
    fn fixture_counts_weights_and_status() {
        let store = fixture();
        let rows = snapshot(&store.connection).unwrap();
        assert_eq!(rows.len(), 2000);
        assert_eq!(
            store
                .connection
                .query_row("SELECT count(*) FROM gear_category", [], |r| r
                    .get::<_, i32>(0))
                .unwrap(),
            12
        );
        let kits: Vec<_> = rows.iter().filter(|i| i.row.is_kit).collect();
        assert!((20..70).contains(&kits.len()));
        for kit in kits {
            assert!((3..=8).contains(&kit.row.components.len()));
            assert_eq!(
                kit.row.weight,
                Some(
                    kit.row
                        .components
                        .iter()
                        .map(|c| c.weight * c.quantity)
                        .sum()
                )
            );
        }
        assert!(rows.iter().filter(|i| i.row.status == "retired").count() > 60);
        assert!(rows.iter().filter(|i| i.row.status == "wishlist").count() > 30);
        for r in rows {
            assert!((5..=1500).contains(&r.row.weight.unwrap().0));
        }
    }
    #[test]
    fn filters_and_subtotals() {
        let store = fixture();
        let current = store.gear_groups(&GearFilter::default()).unwrap();
        let retired = store
            .gear_groups(&GearFilter {
                status: StatusFilter::Retired,
                query: String::new(),
            })
            .unwrap();
        let all = store.gear_groups(&all()).unwrap();
        assert_eq!(
            all.iter().map(|g| g.rows.len()).sum::<usize>(),
            current.iter().map(|g| g.rows.len()).sum::<usize>()
                + retired.iter().map(|g| g.rows.len()).sum::<usize>()
        );
        assert!(
            current
                .iter()
                .flat_map(|g| &g.rows)
                .all(|r| r.status == "current")
        );
        assert!(
            retired
                .iter()
                .flat_map(|g| &g.rows)
                .all(|r| r.status == "retired")
        );
        let shelter = store
            .gear_groups(&GearFilter {
                query: "SHELTER".into(),
                ..super::tests::all()
            })
            .unwrap();
        assert_eq!(shelter.len(), 1);
        assert_eq!(shelter[0].name, "Shelter");
        let tarp = store
            .gear_groups(&GearFilter {
                query: "TARP".into(),
                ..super::tests::all()
            })
            .unwrap();
        assert!(!tarp.is_empty());
        assert!(
            tarp.iter()
                .flat_map(|g| &g.rows)
                .all(|r| r.name.to_lowercase().contains("tarp"))
        );
        for g in &all {
            for (class, total) in [
                ("base", g.base),
                ("worn", g.worn),
                ("consumable", g.consumable),
            ] {
                assert_eq!(
                    total,
                    g.rows
                        .iter()
                        .filter(|r| r.class == class)
                        .map(|r| r.weight.unwrap_or_default() * r.quantity)
                        .sum()
                );
            }
        }
        let totals = store.totals().unwrap();
        assert_eq!(
            totals.items,
            current.iter().map(|g| g.rows.len()).sum::<usize>()
        );
        assert_eq!(totals.base, current.iter().map(|g| g.base).sum());
        assert_eq!(totals.worn, current.iter().map(|g| g.worn).sum());
        assert_eq!(
            totals.consumable,
            current.iter().map(|g| g.consumable).sum()
        );
    }
    #[test]
    fn every_command_undo_redo_and_redo_clear() {
        let mut store = fixture();
        let initial = store.gear_groups(&all()).unwrap();
        let id = initial[0].rows[0].id.clone();
        let kit_id = initial
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.is_kit)
            .unwrap()
            .id
            .clone();
        for command in [
            Command::UpdateItem {
                id: id.clone(),
                name: "Edited tarp".into(),
                weight: None,
                quantity: 3,
            },
            Command::MoveItem {
                id: id.clone(),
                to_category: initial[1].id.clone(),
                before: Some(initial[1].rows[0].id.clone()),
            },
            Command::DeleteItem { id: kit_id },
        ] {
            let raw_before = snapshot(&store.connection).unwrap();
            store.execute(command).unwrap();
            let after = store.gear_groups(&all()).unwrap();
            assert_ne!(after, initial);
            assert!(store.undo().unwrap());
            assert_eq!(store.gear_groups(&all()).unwrap(), initial);
            let raw_after = snapshot(&store.connection).unwrap();
            for (a, b) in raw_before.iter().zip(raw_after.iter()) {
                assert_eq!(a.row, b.row);
                assert_eq!(a.category, b.category);
                assert_eq!(a.order, b.order);
            }
            assert!(store.redo().unwrap());
            assert_eq!(store.gear_groups(&all()).unwrap(), after);
            assert!(store.undo().unwrap());
        }
        store.execute(Command::DeleteItem { id }).unwrap();
        assert!(!store.redo().unwrap());
    }
    fn edge_fixture() -> Store {
        let store = Store::in_memory().unwrap();
        store
            .connection
            .execute_batch(
                "INSERT INTO gear_category VALUES('s','Shelter',1),('w','Worn',0),('e','Empty',2);
            INSERT INTO gear_item VALUES
            ('tent','s','Tarp 100%_Lite',10,3,5,'base','current',0),
            ('coat','w','Coat',20,2,0,'worn','current',0),
            ('retired','s','Retired tent',100,2,1,'base','retired',0),
            ('wish','s','Wishlist tent',999,1,0,'base','wishlist',0),
            ('unknown',NULL,'Mystery',NULL,4,8,'consumable','current',0);",
            )
            .unwrap();
        store
    }
    #[test]
    fn missing_weights_other_order_and_literal_query() {
        let store = edge_fixture();
        let groups = store.gear_groups(&all()).unwrap();
        assert_eq!(
            groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
            ["Worn", "Shelter", "Other"]
        );
        assert_eq!(
            groups[1]
                .rows
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            ["retired", "tent"]
        );
        assert_eq!(groups[1].base, Grams(230));
        assert_eq!(groups[2].id, None);
        assert_eq!(groups[2].rows[0].weight, None);
        assert_eq!(groups[2].consumable, Grams(0));
        assert_eq!(
            store.totals().unwrap(),
            Totals {
                base: Grams(30),
                worn: Grams(40),
                consumable: Grams(0),
                items: 3
            }
        );
        let filtered = store
            .gear_groups(&GearFilter {
                query: "%_lItE".into(),
                ..all()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rows.len(), 1);
        assert_eq!(filtered[0].base, Grams(30));
        let other = store
            .gear_groups(&GearFilter {
                query: "oThEr".into(),
                ..all()
            })
            .unwrap();
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].name, "Other");
        assert!(
            store
                .gear_groups(&GearFilter {
                    query: "does not exist".into(),
                    ..all()
                })
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn invalid_commands_are_atomic_and_preserve_redo() {
        let mut store = edge_fixture();
        store
            .execute(Command::DeleteItem { id: "tent".into() })
            .unwrap();
        assert!(store.undo().unwrap());
        let initial = store.gear_groups(&all()).unwrap();
        for command in [
            Command::UpdateItem {
                id: "tent".into(),
                name: " ".into(),
                weight: Some(Grams(10)),
                quantity: 1,
            },
            Command::UpdateItem {
                id: "tent".into(),
                name: "Tarp".into(),
                weight: Some(Grams(-1)),
                quantity: 1,
            },
            Command::UpdateItem {
                id: "tent".into(),
                name: "Tarp".into(),
                weight: Some(Grams(10)),
                quantity: 0,
            },
            Command::UpdateItem {
                id: "tent".into(),
                name: "Tarp".into(),
                weight: Some(Grams(i32::MAX)),
                quantity: 2,
            },
            Command::UpdateItem {
                id: "tent".into(),
                name: "Tarp".into(),
                weight: Some(Grams(i32::MAX)),
                quantity: 1,
            },
            Command::DeleteItem {
                id: "missing".into(),
            },
            Command::MoveItem {
                id: "tent".into(),
                to_category: Some("missing".into()),
                before: None,
            },
            Command::MoveItem {
                id: "tent".into(),
                to_category: Some("w".into()),
                before: Some("retired".into()),
            },
        ] {
            assert!(store.execute(command).is_err());
            assert_eq!(store.gear_groups(&all()).unwrap(), initial);
            assert!(!store.undo().unwrap());
        }
        assert!(store.redo().unwrap());
        assert_eq!(store.totals().unwrap().items, 2);
    }
    #[test]
    fn move_into_and_out_of_other_and_empty_categories() {
        let mut store = edge_fixture();
        let initial = store.gear_groups(&all()).unwrap();
        store
            .execute(Command::MoveItem {
                id: "tent".into(),
                to_category: None,
                before: Some("unknown".into()),
            })
            .unwrap();
        assert_eq!(
            category_ids(&store.connection, &None).unwrap(),
            ["tent", "unknown"]
        );
        assert!(store.undo().unwrap());
        assert_eq!(store.gear_groups(&all()).unwrap(), initial);
        store
            .execute(Command::MoveItem {
                id: "unknown".into(),
                to_category: Some("e".into()),
                before: None,
            })
            .unwrap();
        assert_eq!(
            store.gear_groups(&all()).unwrap().last().unwrap().name,
            "Empty"
        );
        assert!(store.undo().unwrap());
        assert_eq!(store.gear_groups(&all()).unwrap(), initial);
    }
    #[test]
    fn moves_normalize_all_rows() {
        let mut store = fixture();
        let cats: Vec<String> = store
            .connection
            .prepare("SELECT id FROM gear_category ORDER BY sort_order")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let a = Some(cats[0].clone());
        let b = Some(cats[1].clone());
        let start = category_ids(&store.connection, &a).unwrap();
        let mut expected = start.clone();
        let id = expected.remove(0);
        expected.insert(2, id.clone());
        store
            .execute(Command::MoveItem {
                id: id.clone(),
                to_category: a.clone(),
                before: Some(start[3].clone()),
            })
            .unwrap();
        assert_eq!(category_ids(&store.connection, &a).unwrap(), expected);
        let mut destination = category_ids(&store.connection, &b).unwrap();
        destination.push(id.clone());
        store
            .execute(Command::MoveItem {
                id: id.clone(),
                to_category: b.clone(),
                before: None,
            })
            .unwrap();
        expected.retain(|v| v != &id);
        assert_eq!(category_ids(&store.connection, &a).unwrap(), expected);
        assert_eq!(category_ids(&store.connection, &b).unwrap(), destination);
        for cat in [a, b] {
            let orders: Vec<i32> = store
                .connection
                .prepare(
                    "SELECT sort_order FROM gear_item WHERE category_id IS ?1 ORDER BY sort_order",
                )
                .unwrap()
                .query_map([cat], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            assert_eq!(orders, (0..orders.len() as i32).collect::<Vec<_>>());
        }
    }
}
