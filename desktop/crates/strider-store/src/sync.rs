//! Sync writes bypass commands, propagation, undo and the outbox.
use super::*;
use serde_json::{Map, Value as Json, json};

pub const LONG_TEXT: &[&str] = &[
    "planning_notes",
    "trip_report",
    "packing_list",
    "notes",
    "template",
    "packing_default",
    "permit_notes",
    "driving_notes",
    "water_sources",
];
#[derive(Clone, Debug)]
pub struct OutboxEntry {
    pub seq: i64,
    pub table: String,
    pub sync_id: String,
    pub columns: Vec<String>,
    pub base_seq: Option<String>,
}
#[derive(Clone, Debug)]
pub struct OutboxBatch {
    pub batch_id: String,
    pub entries: Vec<OutboxEntry>,
}
fn protocol(e: impl fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}
fn table(t: &str) -> Result<()> {
    if TABLES.contains(&t) {
        Ok(())
    } else {
        invalid("Unknown sync table")
    }
}
fn columns(db: &Connection, t: &str) -> Result<BTreeSet<String>> {
    table(t)?;
    Ok(db
        .prepare(&format!("PRAGMA table_info({t})"))?
        .query_map([], |r| r.get(1))?
        .collect::<std::result::Result<_, _>>()?)
}
fn json_value(v: Value, col: &str) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Integer(n) if boolean(col) => Json::Bool(n != 0),
        Value::Integer(n) => n.into(),
        Value::Real(n) => json!(n),
        Value::Text(s) if json_column(col) => serde_json::from_str(&s).unwrap_or(Json::Null),
        Value::Text(s) => s.into(),
        Value::Blob(_) => Json::Null,
    }
}
fn boolean(c: &str) -> bool {
    matches!(
        c,
        "is_kit" | "is_default" | "included" | "packed" | "is_hot" | "permit_required"
    )
}
fn json_column(c: &str) -> bool {
    matches!(c, "grocery_removed" | "track" | "profile" | "points")
}
fn sql_value(v: &Json) -> Result<Value> {
    Ok(match v {
        Json::Null => Value::Null,
        Json::Bool(v) => Value::Integer(i64::from(*v)),
        Json::Number(v) => Value::Integer(v.as_i64().ok_or_else(|| protocol("Expected integer"))?),
        Json::String(v) => text(v),
        _ => text(serde_json::to_string(v).map_err(protocol)?),
    })
}
pub fn writable_columns(t: &str) -> &'static str {
    match t {
        "gear_category" => "name parent_id weight_class sort_order",
        "gear_item" => {
            "category_id name weight_g price_cents quantity sort_order default_weight_class status is_kit product_url notes acquired_at retired_at"
        }
        "gear_component" => "gear_item_id name weight_g quantity notes",
        "wishlist_replacement" => "wishlist_item_id replaces_item_id",
        "wishlist_loadout" => "wishlist_item_id loadout_id included",
        "loadout" => "name is_default sort_order notes",
        "loadout_item" => "loadout_id gear_item_id weight_class",
        "trail" => "name region area_type typical_distance_mi typical_elevation_ft notes",
        "trip" => {
            "trail_id name status start_date nights party_size grocery_people grocery_removed region area_type lat lon distance_mi elevation_gain_ft food_g_per_day water_g_per_day fuel_g_per_day trailhead permit_required permit_notes driving_notes water_sources planning_notes trip_report packing_list"
        }
        "trip_day" => "trip_id day_number distance_mi elevation_gain_ft",
        "trip_gear" => {
            "trip_id gear_item_id snapshot_name snapshot_weight_g snapshot_category weight_class quantity packed sort_order"
        }
        "trip_campsite" => "trip_id distance_mi lat lon ele_ft",
        "trip_gpx" => {
            "trip_id filename start_lat start_lon distance_mi elevation_gain_ft min_ele_ft max_ele_ft track profile points"
        }
        "trip_permit" => "filename",
        "trip_meal" => {
            "trip_id day_number meal_type meal_id ingredient_id servings snapshot_name snapshot_kcal snapshot_weight_g sort_order"
        }
        "trip_consumable" => "trip_id type label grams day_number",
        "ingredient" => {
            "name kcal_per_100g density_g_ml density_source reference_name default_serving_g category source source_id notes"
        }
        "meal" => "name base_servings meal_type is_hot water_ml notes",
        "meal_ingredient" => {
            "meal_id ingredient_id snapshot_name snapshot_kcal_per_100g snapshot_density_g_ml entered_amount entered_unit amount_g sort_order"
        }
        "calendar_feed" => "label color url sort_order",
        "energy_params" => {
            "bmr cal_per_energy_mile ft_per_energy_mile sex weight_lb height_in age_years"
        }
        "report_settings" => "template packing_default",
        _ => "",
    }
}
impl Store {
    pub fn open_secondary(path: impl AsRef<Path>) -> Result<Self> {
        Self::open(path)
    }
    pub fn state(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row("SELECT value FROM sync_state WHERE key=?", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn set_state(&self, key: &str, value: &str) -> Result<()> {
        self.connection.execute("INSERT INTO sync_state(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
        Ok(())
    }
    pub fn cursor(&self) -> Result<String> {
        Ok(self.state("cursor")?.unwrap_or_else(|| "0".into()))
    }
    pub fn set_cursor(&self, value: &str) -> Result<()> {
        self.set_state("cursor", value)
    }
    /// Day edits made before the first campsite push need the server's generated IDs.
    pub fn has_unresolved_day_edits(&self, batch: &OutboxBatch) -> Result<bool> {
        for e in &batch.entries {
            if e.table == "trip_day"
                && self.connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM local_derived_day WHERE sync_id=? AND edited=1)",
                    [&e.sync_id],
                    |r| r.get::<_, bool>(0),
                )?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// A complete pull maps existing days. Unmatched edits are new explicit days.
    pub fn finish_day_mapping(&self) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute("DELETE FROM outbox WHERE tbl='trip_day' AND op='delete' AND sync_id IN (SELECT sync_id FROM local_derived_day WHERE edited=1)",[])?;
        tx.execute("DELETE FROM local_derived_day WHERE edited=1", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn pending_count(&self) -> Result<usize> {
        Ok(self
            .connection
            .query_row("SELECT (SELECT count(*) FROM outbox)+(SELECT count(*) FROM permit_upload u JOIN trip_permit p ON p.trip_id=u.trip_id AND p.sha256=u.sha256)", [], |r| r.get::<_, i64>(0))?
            as usize)
    }
    /// Undo snapshots encompass the workspace. A pull invalidates all history,
    /// including on a UI connection when a secondary connection applied the pull.
    /// Restoring a snapshot over newer server rows would silently lose their data.
    pub fn invalidate_pulled_history(&mut self) -> Result<()> {
        let generation = self.state("pull_generation")?.unwrap_or_default();
        if generation != self.pull_generation {
            self.undo.clear();
            self.redo.clear();
            self.pull_generation = generation;
        }
        Ok(())
    }
    /// Whole command batches in sequence order. Oversized commands are split into
    /// persisted, independently retryable batch identities before transmission.
    pub fn outbox_batches(&mut self, limit: usize) -> Result<Vec<OutboxBatch>> {
        let limit = limit.clamp(1, 500);
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let entries = tx
            .prepare("SELECT seq,batch_id,tbl,sync_id,columns,base_seq FROM outbox ORDER BY seq")?
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    OutboxEntry {
                        seq: r.get(0)?,
                        table: r.get(2)?,
                        sync_id: r.get(3)?,
                        columns: serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or_default(),
                        base_seq: r.get::<_, Option<i64>>(5)?.map(|v| v.to_string()),
                    },
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut batches: Vec<OutboxBatch> = vec![];
        let mut original = String::new();
        for (batch, entry) in entries {
            if original != batch || batches.last().is_some_and(|b| b.entries.len() == limit) {
                let batch_id = if original == batch {
                    id()
                } else {
                    batch.clone()
                };
                batches.push(OutboxBatch {
                    batch_id,
                    entries: vec![],
                });
            }
            original = batch;
            if let Some(b) = batches.last_mut() {
                tx.execute(
                    "UPDATE outbox SET batch_id=? WHERE seq=?",
                    params![b.batch_id, entry.seq],
                )?;
                b.entries.push(entry);
            }
        }
        tx.commit()?;
        Ok(batches)
    }
    pub fn row_values(
        &self,
        t: &str,
        key: &str,
        requested: &[String],
    ) -> Result<Option<Map<String, Json>>> {
        table(t)?;
        let known = columns(&self.connection, t)?;
        if requested.iter().any(|c| !known.contains(c)) {
            return invalid("Unknown column");
        }
        let exists = self
            .connection
            .query_row(&format!("SELECT 1 FROM {t} WHERE sync_id=?"), [key], |_| {
                Ok(())
            })
            .optional()?
            .is_some();
        if !exists {
            return Ok(None);
        }
        let r = row(&self.connection, t, key)?;
        Ok(Some(
            r.into_iter()
                .filter(|(c, _)| requested.is_empty() || requested.contains(c))
                .map(|(c, v)| {
                    let v = json_value(v, &c);
                    (c, v)
                })
                .collect(),
        ))
    }
    pub fn server_seq(&self, t: &str, key: &str) -> Result<Option<i64>> {
        Ok(self
            .connection
            .query_row(
                "SELECT server_seq FROM row_meta WHERE tbl=? AND sync_id=?",
                params![t, key],
                |r| r.get(0),
            )
            .optional()?
            .flatten())
    }
    pub fn base_values(&self, t: &str, key: &str) -> Result<Map<String, Json>> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT base_values FROM row_meta WHERE tbl=? AND sync_id=?",
                params![t, key],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        value
            .map(|s| serde_json::from_str(&s).map_err(protocol))
            .unwrap_or_else(|| Ok(Map::new()))
    }
    pub fn ack_batch(&mut self, batch: &str, adopt: &[Json], replaced: &[Json]) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON")?;
        for a in adopt {
            let t = required(a, "table")?;
            table(t)?;
            let from = required(a, "from")?;
            let to = required(a, "to")?;
            for child in TABLES {
                let fks = tx
                    .prepare(&format!("PRAGMA foreign_key_list({child})"))?
                    .query_map([], |r| Ok((r.get::<_, String>(2)?, r.get::<_, String>(3)?)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                for (parent, col) in fks {
                    if parent == t {
                        tx.execute(
                            &format!("UPDATE {child} SET {col}=? WHERE {col}=?"),
                            params![to, from],
                        )?;
                    }
                }
            }
            let exists = tx
                .query_row(&format!("SELECT 1 FROM {t} WHERE sync_id=?"), [to], |_| {
                    Ok(())
                })
                .optional()?
                .is_some();
            if exists {
                tx.execute(&format!("DELETE FROM {t} WHERE sync_id=?"), [from])?;
            } else {
                tx.execute(
                    &format!("UPDATE {t} SET sync_id=? WHERE sync_id=?"),
                    params![to, from],
                )?;
            }
            tx.execute(
                "UPDATE outbox SET sync_id=? WHERE tbl=? AND sync_id=?",
                params![to, t, from],
            )?;
            if t == "trip" {
                tx.execute(
                    "UPDATE outbox SET trip_id=? WHERE trip_id=?",
                    params![to, from],
                )?;
            }
            tx.execute(
                "DELETE FROM row_meta WHERE tbl=? AND sync_id=?",
                params![t, from],
            )?;
        }
        for r in replaced {
            let t = required(r, "table")?;
            table(t)?;
            let key = required(r, "sync_id")?;
            let col = required(r, "column")?;
            if !LONG_TEXT.contains(&col) {
                return invalid("Invalid conflict column");
            }
            let local = row(&tx, t, key).ok().and_then(|r| opt(&r, col));
            tx.execute("INSERT INTO conflict(tbl,sync_id,col,local_value,server_value,created_at) VALUES(?,?,?,?,?,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![t,key,col,local,r.get("value").and_then(Json::as_str)])?;
        }
        // Acknowledged rows exist remotely even before the next pull. This keeps
        // later queued edits from resending insert-only columns as an update.
        tx.execute("INSERT OR IGNORE INTO row_meta(tbl,sync_id,server_seq) SELECT tbl,sync_id,0 FROM outbox WHERE batch_id=? AND op='upsert'",[batch])?;
        tx.execute("DELETE FROM permit_delete WHERE trip_id IN (SELECT trip_id FROM outbox WHERE batch_id=? AND tbl='trip_permit' AND op='delete')",[batch])?;
        tx.execute("DELETE FROM outbox WHERE batch_id=?", [batch])?;
        tx.execute("DELETE FROM sync_state WHERE key='inflight'", [])?;
        if !adopt.is_empty() {
            generation(&tx)?;
        }
        tx.commit()?;
        self.invalidate_pulled_history()?;
        Ok(())
    }
    pub fn reject_batch(&mut self, batch: &str, reason: &str) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let before = snapshot(&tx)?;
        let rejected=tx.prepare("SELECT tbl,sync_id FROM outbox WHERE batch_id=? AND op='upsert' AND NOT EXISTS(SELECT 1 FROM row_meta m WHERE m.tbl=outbox.tbl AND m.sync_id=outbox.sync_id AND m.server_seq IS NOT NULL)")?.query_map([batch],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        for t in TABLES.iter().rev() {
            for (tbl, key) in &rejected {
                if tbl == t {
                    let newer:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM outbox WHERE tbl=? AND sync_id=? AND batch_id<>?)",params![t,key,batch],|r|r.get(0))?;
                    if newer {
                        // A correction queued during the rejected request still
                        // deserves its own attempt. It now needs a full insert.
                        if let Some(r) = before[*t].get(key) {
                            let cols = serde_json::to_string(&r.keys().collect::<Vec<_>>())
                                .map_err(protocol)?;
                            tx.execute("UPDATE outbox SET columns=? WHERE tbl=? AND sync_id=? AND batch_id<>? AND op='upsert'",params![cols,t,key,batch])?;
                        }
                    } else {
                        tx.execute(&format!("DELETE FROM {t} WHERE sync_id=?"), [key])?;
                    }
                }
            }
        }
        let after = snapshot(&tx)?;
        for t in TABLES {
            for key in before[*t].keys() {
                if !after[*t].contains_key(key) {
                    tx.execute(
                        "DELETE FROM outbox WHERE tbl=? AND sync_id=?",
                        params![t, key],
                    )?;
                }
            }
        }
        generation(&tx)?;
        // Rewind so unchanged server rows rejected by push are actually refetched.
        tx.execute("INSERT INTO sync_state(key,value) VALUES('cursor','0') ON CONFLICT(key) DO UPDATE SET value='0'",[])?;
        tx.execute(
            "INSERT INTO sync_log(kind,detail) VALUES('rejected',?)",
            [format!("{batch}: {reason}")],
        )?;
        tx.execute("DELETE FROM outbox WHERE batch_id=?", [batch])?;
        tx.execute("DELETE FROM sync_state WHERE key='inflight'", [])?;
        tx.commit()?;
        self.invalidate_pulled_history()?;
        Ok(())
    }
    pub fn apply_pull_page(&mut self, page: &Json) -> Result<ChangeSet> {
        // Sequence pagination can deliver a child before its parent's current
        // version. Persist those pages and the cursor together, then materialize
        // when dependencies arrive. A restart cannot lose the buffered rows.
        let page = merge_buffer(self.state("pull_buffer")?, page)?;
        let cursor = required(&page, "cursor")?;
        cursor.parse::<u64>().map_err(protocol)?;
        let incoming = page
            .get("rows")
            .and_then(Json::as_array)
            .ok_or_else(|| protocol("Missing rows"))?;
        let tombstones = page
            .get("tombstones")
            .and_then(Json::as_array)
            .ok_or_else(|| protocol("Missing tombstones"))?;
        let revoked = page
            .get("revoked_trips")
            .and_then(Json::as_array)
            .ok_or_else(|| protocol("Missing revoked_trips"))?;
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch("PRAGMA defer_foreign_keys=ON")?;
        let before = snapshot(&tx)?;
        if missing_parents(&tx, incoming)? {
            tx.execute("INSERT INTO sync_state(key,value) VALUES('pull_buffer',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[page.to_string()])?;
            tx.execute("INSERT INTO sync_state(key,value) VALUES('cursor',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[cursor])?;
            tx.commit()?;
            return Ok(ChangeSet::default());
        }
        tx.execute("DELETE FROM sync_state WHERE key='pull_buffer'", [])?;
        let mut ordered = incoming.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|r| {
            TABLES
                .iter()
                .position(|t| Some(*t) == r.get("table").and_then(Json::as_str))
                .unwrap_or(usize::MAX)
        });
        for r in ordered {
            let t = required(r, "table")?;
            let key = required(r, "sync_id")?;
            // Binary PUT owns permit identities. Adopt metadata by its unique trip.
            if t == "trip_permit"
                && let Some(trip) = r.get("trip_id").and_then(Json::as_str)
            {
                tx.execute(
                    "UPDATE trip_permit SET sync_id=? WHERE trip_id=? AND sync_id<>?",
                    params![key, trip, key],
                )?;
            }
            if t == "trip_day"
                && let Some(trip) = r.get("trip_id").and_then(Json::as_str)
            {
                let day = r
                    .get("day_number")
                    .and_then(Json::as_i64)
                    .ok_or_else(|| protocol("Missing day number"))?;
                let local:Option<String>=tx.query_row("SELECT sync_id FROM local_derived_day WHERE trip_id=? AND day_number=? AND edited=1",params![trip,day],|r|r.get(0)).optional()?;
                if let Some(local) = local {
                    tx.execute(
                        "UPDATE trip_day SET sync_id=? WHERE sync_id=?",
                        params![key, local],
                    )?;
                    tx.execute(
                        "UPDATE outbox SET sync_id=? WHERE tbl='trip_day' AND sync_id=?",
                        params![key, local],
                    )?;
                    tx.execute("DELETE FROM local_derived_day WHERE sync_id=?", [local])?;
                }
                tx.execute("DELETE FROM trip_day WHERE trip_id=? AND sync_id IN (SELECT sync_id FROM local_derived_day WHERE edited=0)",[trip])?;
            }
            let known = columns(&tx, t)?;
            let mut pending = BTreeSet::new();
            let entries = tx
                .prepare("SELECT op,columns FROM outbox WHERE tbl=? AND sync_id=?")?
                .query_map(params![t, key], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let deleting_permit = if t == "trip_permit" {
                if let Some(trip) = r.get("trip_id").and_then(Json::as_str) {
                    let deleting: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM permit_delete WHERE trip_id=?)",
                        [trip],
                        |r| r.get(0),
                    )?;
                    if deleting {
                        let queued:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM outbox WHERE tbl='trip_permit' AND sync_id=? AND op='delete')",[key],|r|r.get(0))?;
                        if !queued {
                            tx.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at,trip_id) VALUES(?,'trip_permit',?,'delete','[]',strftime('%Y-%m-%dT%H:%M:%fZ','now'),?)",params![id(),key,trip])?;
                        }
                    }
                    deleting
                } else {
                    false
                }
            } else {
                false
            };
            let queued_permit = if t == "trip_permit" {
                if let Some(trip) = r.get("trip_id").and_then(Json::as_str) {
                    tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM permit_upload WHERE trip_id=?)",
                        [trip],
                        |r| r.get::<_, bool>(0),
                    )?
                } else {
                    false
                }
            } else {
                false
            };
            let deleted =
                deleting_permit || queued_permit || entries.iter().any(|(op, _)| op == "delete");
            for (_, cs) in entries {
                pending.extend(serde_json::from_str::<Vec<String>>(&cs).map_err(protocol)?);
            }
            let base: Map<String, Json> = r
                .as_object()
                .ok_or_else(|| protocol("Invalid row"))?
                .iter()
                .filter(|(c, _)| LONG_TEXT.contains(&c.as_str()))
                .map(|(c, v)| (c.clone(), v.clone()))
                .collect();
            if !deleted {
                let exists = tx
                    .query_row(&format!("SELECT 1 FROM {t} WHERE sync_id=?"), [key], |_| {
                        Ok(())
                    })
                    .optional()?
                    .is_some();
                let mut values = Row::new();
                for (c, v) in r.as_object().ok_or_else(|| protocol("Invalid row"))? {
                    if c == "sync_id" || !known.contains(c) || (exists && pending.contains(c)) {
                        continue;
                    }
                    values.insert(c.clone(), sql_value(v)?);
                }
                write(&tx, t, key, values, !exists)?;
            }
            let seq = required(r, "server_seq")?
                .parse::<i64>()
                .map_err(protocol)?;
            tx.execute("INSERT INTO row_meta(tbl,sync_id,server_seq,base_values) VALUES(?,?,?,?) ON CONFLICT(tbl,sync_id) DO UPDATE SET server_seq=excluded.server_seq,base_values=excluded.base_values",params![t,key,seq,Json::Object(base).to_string()])?;
        }
        for r in tombstones {
            let t = required(r, "table")?;
            table(t)?;
            let key = required(r, "sync_id")?;
            let seq = required(r, "server_seq")?
                .parse::<i64>()
                .map_err(protocol)?;
            let current = tx
                .query_row(
                    "SELECT server_seq FROM row_meta WHERE tbl=? AND sync_id=?",
                    params![t, key],
                    |r| r.get::<_, Option<i64>>(0),
                )
                .optional()?
                .flatten();
            if current.is_none_or(|current| current <= seq) {
                tx.execute(&format!("DELETE FROM {t} WHERE sync_id=?"), [key])?;
            }
        }
        for key in revoked {
            let key = key.as_str().ok_or_else(|| protocol("Invalid revoked id"))?;
            tx.execute("DELETE FROM trip WHERE sync_id=?", [key])?;
            tx.execute(
                "DELETE FROM outbox WHERE trip_id=? OR (tbl='trip' AND sync_id=?)",
                params![key, key],
            )?;
        }
        let after = snapshot(&tx)?;
        // Cascades remove descendants; discard queued writes for every removed row.
        for t in TABLES {
            for key in before[*t].keys() {
                if !after[*t].contains_key(key) {
                    tx.execute(
                        "DELETE FROM outbox WHERE tbl=? AND sync_id=?",
                        params![t, key],
                    )?;
                    tx.execute(
                        "DELETE FROM row_meta WHERE tbl=? AND sync_id=?",
                        params![t, key],
                    )?;
                }
            }
        }
        let changed = changes(&before, &after);
        tx.execute("INSERT INTO sync_state(key,value) VALUES('cursor',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[cursor])?;
        if !incoming.is_empty() || !tombstones.is_empty() || !revoked.is_empty() {
            generation(&tx)?;
        }
        tx.commit()?;
        self.invalidate_pulled_history()?;
        Ok(changed)
    }
    pub fn sync_details(&self) -> Result<Vec<(String, String)>> {
        let mut details=self.connection.prepare("SELECT 'rejected', at || ' · ' || detail FROM sync_log WHERE kind='rejected' ORDER BY id DESC")?.query_map([], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        details.extend(self.connection.prepare("SELECT tbl || '.' || col, COALESCE(server_value,'') FROM conflict WHERE dismissed=0 ORDER BY id DESC")?.query_map([], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?);
        Ok(details)
    }
    /// Undismissed text conflicts, newest first.
    pub fn conflicts(&self) -> Result<Vec<ConflictRow>> {
        let mut statement = self.connection.prepare("SELECT id, tbl || '.' || col, COALESCE(server_value,'') FROM conflict WHERE dismissed=0 ORDER BY id DESC")?;
        let rows = statement.query_map([], |r| {
            Ok(ConflictRow {
                id: r.get(0)?,
                field: r.get(1)?,
                text: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    /// Hide a conflict from the sync sheet. The row stays for diagnostics.
    pub fn dismiss_conflict(&self, id: i64) -> Result<()> {
        self.connection
            .execute("UPDATE conflict SET dismissed=1 WHERE id=?", [id])?;
        Ok(())
    }
    /// Pushes the server rejected, newest first.
    pub fn rejections(&self) -> Result<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT at || ' · ' || detail FROM sync_log WHERE kind='rejected' ORDER BY id DESC",
        )?;
        let rows = statement.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
/// A server-replaced long text the user may want back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRow {
    pub id: i64,
    /// `table.column`.
    pub field: String,
    /// The replaced text.
    pub text: String,
}
fn merge_buffer(buffer: Option<String>, page: &Json) -> Result<Json> {
    let mut page = page.clone();
    if let Some(buffer) = buffer {
        let buffer: Json = serde_json::from_str(&buffer).map_err(protocol)?;
        for key in ["rows", "tombstones", "revoked_trips"] {
            let mut merged = buffer[key]
                .as_array()
                .ok_or_else(|| protocol("Invalid pull buffer"))?
                .clone();
            merged.extend(
                page[key]
                    .as_array()
                    .ok_or_else(|| protocol(format!("Missing {key}")))?
                    .iter()
                    .cloned(),
            );
            if key == "rows" {
                let mut rows = BTreeMap::new();
                for r in merged {
                    let key = (
                        required(&r, "table")?.to_string(),
                        required(&r, "sync_id")?.to_string(),
                    );
                    let seq = required(&r, "server_seq")?
                        .parse::<i64>()
                        .map_err(protocol)?;
                    if rows.get(&key).is_none_or(|(old, _)| *old <= seq) {
                        rows.insert(key, (seq, r));
                    }
                }
                merged = rows.into_values().map(|(_, r)| r).collect();
            }
            page[key] = Json::Array(merged);
        }
    }
    let revoked = page["revoked_trips"]
        .as_array()
        .ok_or_else(|| protocol("Missing revoked_trips"))?
        .iter()
        .filter_map(Json::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let tombstones = page["tombstones"]
        .as_array()
        .ok_or_else(|| protocol("Missing tombstones"))?
        .clone();
    let rows = page["rows"]
        .as_array_mut()
        .ok_or_else(|| protocol("Missing rows"))?;
    rows.retain(|r| {
        let trip = if r["table"] == "trip" {
            r["sync_id"].as_str()
        } else {
            r["trip_id"].as_str()
        };
        if trip.is_some_and(|t| revoked.contains(t)) {
            return false;
        }
        !tombstones.iter().any(|d| {
            d["table"] == r["table"]
                && d["sync_id"] == r["sync_id"]
                && d["server_seq"].as_str().and_then(|s| s.parse::<i64>().ok())
                    >= r["server_seq"].as_str().and_then(|s| s.parse::<i64>().ok())
        })
    });
    Ok(page)
}
fn missing_parents(db: &Connection, incoming: &[Json]) -> Result<bool> {
    for r in incoming {
        let t = required(r, "table")?;
        table(t)?;
        let fks = db
            .prepare(&format!("PRAGMA foreign_key_list({t})"))?
            .query_map([], |r| Ok((r.get::<_, String>(2)?, r.get::<_, String>(3)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (parent, col) in fks {
            if let Some(key) = r.get(&col).and_then(Json::as_str) {
                if incoming.iter().any(|p| {
                    p["table"].as_str() == Some(&parent) && p["sync_id"].as_str() == Some(key)
                }) {
                    continue;
                }
                if db
                    .query_row(
                        &format!("SELECT 1 FROM {parent} WHERE sync_id=?"),
                        [key],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_none()
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
fn generation(db: &Connection) -> Result<()> {
    db.execute("INSERT INTO sync_state(key,value) VALUES('pull_generation',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[id()])?;
    Ok(())
}
fn required<'a>(v: &'a Json, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Json::as_str)
        .ok_or_else(|| protocol(format!("Missing {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(rows: Json) -> Json {
        json!({"rows":rows,"tombstones":[],"revoked_trips":[],"cursor":"10","more":false})
    }
    #[test]
    fn pull_orders_parents_and_preserves_pending_columns() {
        let mut s = Store::in_memory().unwrap();
        s.apply_pull_page(&page(json!([
            {"table":"gear_item","sync_id":"g","name":"Server","category_id":"c","server_seq":"2","notes":"base"},
            {"table":"gear_category","sync_id":"c","name":"Category","server_seq":"1"}
        ]))).unwrap();
        assert_eq!(s.pending_count().unwrap(), 0);
        s.connection
            .execute("UPDATE gear_item SET name='Local' WHERE sync_id='g'", [])
            .unwrap();
        s.connection.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at) VALUES('b','gear_item','g','upsert','[\"name\"]','now')",[]).unwrap();
        s.apply_pull_page(&page(json!([{"table":"gear_item","sync_id":"g","name":"Other","notes":"remote","server_seq":"3"}]))).unwrap();
        let r = s.row_values("gear_item", "g", &[]).unwrap().unwrap();
        assert_eq!(r["name"], "Local");
        assert_eq!(r["notes"], "remote");
        assert_eq!(s.base_values("gear_item", "g").unwrap()["notes"], "remote");
        assert_eq!(s.cursor().unwrap(), "10");
        assert_eq!(s.pending_count().unwrap(), 1);
        assert!(!s.undo().unwrap());
    }
    #[test]
    fn adopt_rewrites_identity_references_and_conflicts_atomically() {
        let mut s = Store::in_memory().unwrap();
        s.apply_pull_page(&page(json!([
            {"table":"gear_category","sync_id":"old","name":"Old","server_seq":"1"},
            {"table":"gear_item","sync_id":"g","name":"Gear","category_id":"old","server_seq":"2"}
        ])))
        .unwrap();
        s.ack_batch(
            "none",
            &[json!({"table":"gear_category","from":"old","to":"new"})],
            &[json!({"table":"gear_item","sync_id":"g","column":"notes","value":"replaced text"})],
        )
        .unwrap();
        assert!(s.row_values("gear_category", "old", &[]).unwrap().is_none());
        assert_eq!(
            s.row_values("gear_item", "g", &[]).unwrap().unwrap()["category_id"],
            "new"
        );
        assert_eq!(s.sync_details().unwrap()[0].1, "replaced text");
    }
    #[test]
    fn revoked_trip_removes_children_and_outbox() {
        let mut s = Store::in_memory().unwrap();
        s.apply_pull_page(&page(json!([
            {"table":"trip_day","sync_id":"day","trip_id":"trip","day_number":1,"server_seq":"2"},
            {"table":"trip","sync_id":"trip","name":"Trip","access":"view","server_seq":"1"}
        ])))
        .unwrap();
        s.connection.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at) VALUES('b','trip_day','day','upsert','[\"day_number\"]','now')",[]).unwrap();
        let mut p = page(json!([]));
        p["revoked_trips"] = json!(["trip"]);
        s.apply_pull_page(&p).unwrap();
        assert!(s.row_values("trip_day", "day", &[]).unwrap().is_none());
        assert_eq!(s.pending_count().unwrap(), 0);
    }
    #[test]
    fn pull_is_atomic_and_validates_identifiers() {
        let mut s = Store::in_memory().unwrap();
        assert!(
            s.apply_pull_page(&page(json!([
                {"table":"meal","sync_id":"m","name":"Meal","server_seq":"1"},
                {"table":"evil; DROP TABLE meal","sync_id":"x","server_seq":"2"}
            ])))
            .is_err()
        );
        assert!(s.row_values("meal", "m", &[]).unwrap().is_none());
        assert_eq!(s.cursor().unwrap(), "0");
    }
    #[test]
    fn tombstone_does_not_delete_newer_resurrection() {
        let mut s = Store::in_memory().unwrap();
        let mut p =
            page(json!([{"table":"meal","sync_id":"m","name":"Restored","server_seq":"4"}]));
        p["tombstones"] = json!([{"table":"meal","sync_id":"m","server_seq":"3"}]);
        s.apply_pull_page(&p).unwrap();
        assert!(s.row_values("meal", "m", &[]).unwrap().is_some());
    }
    #[test]
    fn secondary_pull_invalidates_ui_undo() {
        let path = std::env::temp_dir().join(format!("strider-sync-{}.db", id()));
        {
            let mut ui = Store::open(&path).unwrap();
            ui.apply(Command::CreateMeal {
                name: "Local".into(),
            })
            .unwrap();
            let mut worker = Store::open_secondary(&path).unwrap();
            worker
                .apply_pull_page(&page(
                    json!([{"table":"ingredient","sync_id":"i","name":"New","server_seq":"1"}]),
                ))
                .unwrap();
            assert!(!ui.undo().unwrap());
            assert!(ui.row_values("ingredient", "i", &[]).unwrap().is_some());
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejection_logs_and_rewinds_cursor() {
        let mut s = Store::in_memory().unwrap();
        s.apply(Command::CreateMeal {
            name: "Meal".into(),
        })
        .unwrap();
        s.set_cursor("20").unwrap();
        let batch = s.outbox_batches(500).unwrap().remove(0);
        s.reject_batch(&batch.batch_id, "Owner only").unwrap();
        assert_eq!(s.pending_count().unwrap(), 0);
        assert_eq!(s.cursor().unwrap(), "0");
        assert!(s.sync_details().unwrap()[0].1.contains("Owner only"));
    }
    #[test]
    fn pages_buffer_until_parents_arrive_across_restart() {
        let path = std::env::temp_dir().join(format!("strider-buffer-{}.db", id()));
        {
            let mut s = Store::open(&path).unwrap();
            s.apply_pull_page(&page(json!([{"table":"gear_item","sync_id":"g","category_id":"c","name":"Gear","server_seq":"1"}]))).unwrap();
            assert!(s.state("pull_buffer").unwrap().is_some());
            assert_eq!(s.cursor().unwrap(), "10");
        }
        {
            let mut s = Store::open(&path).unwrap();
            let mut p = page(
                json!([{"table":"gear_category","sync_id":"c","name":"Category","server_seq":"11"}]),
            );
            p["cursor"] = json!("11");
            s.apply_pull_page(&p).unwrap();
            assert!(s.state("pull_buffer").unwrap().is_none());
            assert_eq!(
                s.row_values("gear_item", "g", &[]).unwrap().unwrap()["category_id"],
                "c"
            );
            assert_eq!(s.pending_count().unwrap(), 0);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn revocation_discards_buffered_inaccessible_trip_rows() {
        let mut s = Store::in_memory().unwrap();
        s.apply_pull_page(&page(json!([{"table":"trip","sync_id":"t","trail_id":"private","name":"Shared","server_seq":"1"}]))).unwrap();
        assert!(s.state("pull_buffer").unwrap().is_some());
        let mut p = page(json!([]));
        p["revoked_trips"] = json!(["t"]);
        s.apply_pull_page(&p).unwrap();
        assert!(s.state("pull_buffer").unwrap().is_none());
        assert!(s.row_values("trip", "t", &[]).unwrap().is_none());
    }
    #[test]
    fn revoked_trip_drops_already_deleted_children_from_outbox() {
        let mut s = Store::in_memory().unwrap();
        s.connection.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at,trip_id) VALUES('b','trip_day','gone','delete','[]','now','trip')",[]).unwrap();
        let mut p = page(json!([]));
        p["revoked_trips"] = json!(["trip"]);
        s.apply_pull_page(&p).unwrap();
        assert_eq!(s.pending_count().unwrap(), 0);
    }
    #[test]
    fn migration_preserves_existing_workspace_and_outbox() {
        let path = std::env::temp_dir().join(format!("strider-migrate-{}.db", id()));
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(include_str!("schema.sql")).unwrap();
            c.pragma_update(None, "user_version", 1).unwrap();
            c.execute("INSERT INTO meal(sync_id,name) VALUES('m','Meal')", [])
                .unwrap();
            c.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at) VALUES('b','meal','m','upsert','[\"name\"]','now')",[]).unwrap();
        }
        {
            let s = Store::open(&path).unwrap();
            assert_eq!(s.pending_count().unwrap(), 1);
            assert_eq!(
                s.row_values("meal", "m", &[]).unwrap().unwrap()["name"],
                "Meal"
            );
            assert!(s.base_values("meal", "m").unwrap().is_empty());
            assert!(s.sync_details().unwrap().is_empty());
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejected_create_preserves_a_newer_correction() {
        let mut s = Store::in_memory().unwrap();
        s.apply(Command::CreateMeal {
            name: "Original".into(),
        })
        .unwrap();
        let first = s.outbox_batches(500).unwrap().remove(0);
        let key = first.entries[0].sync_id.clone();
        s.apply(Command::UpdateMeal {
            id: key.clone(),
            patch: MealPatch {
                notes: Some(Some("Correction".into())),
                ..Default::default()
            },
        })
        .unwrap();
        s.reject_batch(&first.batch_id, "Invalid value").unwrap();
        assert_eq!(s.pending_count().unwrap(), 1);
        assert_eq!(
            s.row_values("meal", &key, &[]).unwrap().unwrap()["notes"],
            "Correction"
        );
        assert!(
            s.outbox_batches(500).unwrap()[0].entries[0]
                .columns
                .contains(&"name".to_string())
        );
    }
    #[test]
    fn acknowledgement_retains_a_newer_batch_and_clears_inflight() {
        let mut s = Store::in_memory().unwrap();
        s.apply(Command::CreateMeal {
            name: "Original".into(),
        })
        .unwrap();
        let first = s.outbox_batches(500).unwrap().remove(0);
        let key = first.entries[0].sync_id.clone();
        s.set_state("inflight", "request").unwrap();
        s.apply(Command::UpdateMeal {
            id: key.clone(),
            patch: MealPatch {
                name: Some("Newer".into()),
                ..Default::default()
            },
        })
        .unwrap();
        s.ack_batch(&first.batch_id, &[], &[]).unwrap();
        assert_eq!(s.pending_count().unwrap(), 1);
        assert!(s.state("inflight").unwrap().is_none());
        assert_eq!(
            s.row_values("meal", &key, &[]).unwrap().unwrap()["name"],
            "Newer"
        );
        assert!(s.server_seq("meal", &key).unwrap().is_some());
    }
    #[test]
    fn large_batches_split_into_stable_bounded_ids() {
        let mut s = Store::in_memory().unwrap();
        for n in 0..503 {
            s.connection.execute("INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at) VALUES('b','meal',?,'delete','[]','now')",[n.to_string()]).unwrap();
        }
        let a = s.outbox_batches(500).unwrap();
        let b = s.outbox_batches(500).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].entries.len(), 500);
        assert_eq!(a[1].entries.len(), 3);
        assert_eq!(a[1].batch_id, b[1].batch_id);
    }
}
