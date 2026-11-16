use crate::{Client, Result, SyncError, Transport};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};
use strider_store::{LONG_TEXT, OutboxBatch, Store, writable_columns};
#[derive(Clone, Debug, Default)]
pub struct SyncReport {
    pub pushed: usize,
    pub pulled: usize,
    pub rejected: usize,
    pub tables: BTreeSet<String>,
}
pub enum SyncEvent {
    Started,
    Finished(SyncReport),
    Offline,
    NeedsSignIn,
    UpgradeRequired,
    Error(String),
}
pub struct SyncEngine<T: Transport = Client> {
    pub store: Store,
    pub client: T,
}
impl<T: Transport> SyncEngine<T> {
    pub fn new(store: Store, client: T) -> Self {
        Self { store, client }
    }
    fn sync_blobs(&self) -> Result<BTreeSet<String>> {
        let mut changed = BTreeSet::new();
        for permit in self.store.permit_blobs()? {
            if permit.state == "local_only" {
                if self.store.server_seq("trip", &permit.trip_id)?.is_none() {
                    continue;
                }
                let Some(bytes) = self.store.blob_bytes(&permit.sha256)? else {
                    self.store.set_blob_state(&permit.sha256, "missing")?;
                    continue;
                };
                let hash = self.client.upload_permit(
                    &permit.trip_id,
                    &permit.filename,
                    &permit.mime_type,
                    &bytes,
                )?;
                if hash != permit.sha256 {
                    return Err(SyncError::Protocol("Permit hash mismatch".into()));
                }
                self.store
                    .acknowledge_permit_upload(&permit.trip_id, &hash)?;
                changed.insert("trip_permit".into());
            } else if permit.state == "missing" && !permit.sha256.is_empty() {
                match self.client.download_permit(&permit.trip_id) {
                    Ok(bytes)
                        if strider_store::blob_hash(&bytes) == permit.sha256
                            && bytes.len() == permit.size_bytes =>
                    {
                        self.store.receive_blob(&permit.sha256, &bytes)?;
                        changed.insert("trip_permit".into());
                    }
                    Ok(_) | Err(SyncError::Server(404)) => {
                        self.store.set_blob_state(&permit.sha256, "missing")?;
                    }
                    Err(e) => {
                        self.store.set_blob_state(&permit.sha256, "missing")?;
                        return Err(e);
                    }
                }
            }
        }
        Ok(changed)
    }
    pub fn operations(&self, batch: &OutboxBatch) -> Result<Value> {
        let snapshot = self
            .store
            .connection()
            .unchecked_transaction()
            .map_err(strider_store::Error::from)?;
        let mut ops = vec![];
        let mut entries: Vec<strider_store::OutboxEntry> = vec![];
        for entry in &batch.entries {
            if let Some(old) = entries
                .iter_mut()
                .find(|e| e.table == entry.table && e.sync_id == entry.sync_id)
            {
                for c in &entry.columns {
                    if !old.columns.contains(c) {
                        old.columns.push(c.clone());
                    }
                }
            } else {
                entries.push(entry.clone());
            }
        }
        for entry in &entries {
            let row = self.store.row_values(&entry.table, &entry.sync_id, &[])?;
            if let Some(row) = row {
                let existing = entry.base_seq.is_some()
                    || self
                        .store
                        .server_seq(&entry.table, &entry.sync_id)?
                        .is_some();
                let mut set = serde_json::Map::new();
                for (c, v) in row.iter() {
                    if !entry.columns.contains(c)
                        || !writable_columns(&entry.table)
                            .split_whitespace()
                            .any(|s| s == c)
                    {
                        continue;
                    }
                    if existing && (c.starts_with("snapshot_") || c == "food_g_per_day") {
                        continue;
                    }
                    if existing
                        && entry.table == "meal_ingredient"
                        && c == "amount_g"
                        && row
                            .get("entered_unit")
                            .and_then(Value::as_str)
                            .is_some_and(|u| {
                                matches!(u, "tsp" | "tbsp" | "cup" | "floz" | "ml" | "l")
                            })
                    {
                        continue;
                    }
                    set.insert(c.clone(), v.clone());
                }
                let base = self.store.base_values(&entry.table, &entry.sync_id)?;
                let base_text = LONG_TEXT
                    .iter()
                    .filter(|c| set.contains_key(**c))
                    .map(|c| (c.to_string(), base.get(*c).cloned().unwrap_or(Value::Null)))
                    .collect::<serde_json::Map<_, _>>();
                let mut op = json!({"op":"upsert","table":entry.table,"sync_id":entry.sync_id,"set":set,"base_text":base_text});
                if let Some(seq) = &entry.base_seq {
                    op["base_seq"] = seq.clone().into();
                }
                ops.push(op);
            } else {
                ops.push(json!({"op":"delete","table":entry.table,"sync_id":entry.sync_id}));
            }
        }
        snapshot.commit().map_err(strider_store::Error::from)?;
        Ok(json!({"batch_id":batch.batch_id,"ops":ops}))
    }
    pub fn sync_once(&mut self) -> Result<SyncReport> {
        let me = self.client.request("GET", "/api/v1/me", None)?;
        let email = me["email"]
            .as_str()
            .ok_or_else(|| SyncError::Protocol("Missing account identity".into()))?;
        let identity = format!("{}:{email}", self.client.origin());
        if self
            .store
            .state("account_identity")?
            .is_some_and(|old| old != identity)
        {
            return Err(SyncError::Protocol(
                "Workspace belongs to another account or server. Use a different --profile.".into(),
            ));
        }
        self.store.set_state("account_identity", &identity)?;
        self.store.set_state("account_email", email)?;
        let mut report = SyncReport::default();
        let mut deferred_days = false;
        loop {
            // Persist exact payloads: retry safely after a lost response, including
            // when the UI edited the row again while the request was in flight.
            let request = if let Some(saved) = self.store.state("inflight")? {
                serde_json::from_str(&saved)
                    .map_err(|_| SyncError::Protocol("Invalid saved request".into()))?
            } else {
                let batches = self.store.outbox_batches(500)?;
                let Some(batch) = batches.first() else {
                    break;
                };
                if self.store.has_unresolved_day_edits(batch)? {
                    deferred_days = true;
                    break;
                }
                let request = self.operations(batch)?;
                self.store.set_state("inflight", &request.to_string())?;
                request
            };
            let batch = request["batch_id"]
                .as_str()
                .ok_or_else(|| SyncError::Protocol("Missing batch id".into()))?;
            match self
                .client
                .request("POST", "/api/v1/sync/push", Some(&request))
            {
                Ok(result) => {
                    let adopt = result["adopt"]
                        .as_array()
                        .ok_or_else(|| SyncError::Protocol("Missing adopt results".into()))?;
                    let replaced = result["replaced"]
                        .as_array()
                        .ok_or_else(|| SyncError::Protocol("Missing conflict results".into()))?;
                    self.store.ack_batch(batch, adopt, replaced)?;
                    if let Some(ops) = request["ops"].as_array() {
                        report.tables.extend(
                            ops.iter()
                                .filter_map(|op| op["table"].as_str().map(str::to_owned)),
                        );
                    }
                    report.pushed += request["ops"].as_array().map_or(0, Vec::len);
                }
                Err(SyncError::Rejected { index, reason }) => {
                    self.store
                        .reject_batch(batch, &format!("Operation {index}: {reason}"))?;
                    report.rejected += 1;
                    report
                        .tables
                        .extend(strider_store::TABLES.iter().map(|t| t.to_string()));
                }
                Err(e) => return Err(e),
            }
        }
        loop {
            let cursor = self.store.cursor()?;
            let page =
                self.client
                    .request("GET", &format!("/api/v1/sync/pull?since={cursor}"), None)?;
            let more = page["more"]
                .as_bool()
                .ok_or_else(|| SyncError::Protocol("Missing pagination flag".into()))?;
            if more && page["cursor"].as_str() == Some(&cursor) {
                return Err(SyncError::Protocol("Pull cursor did not advance".into()));
            }
            let changes = self.store.apply_pull_page(&page)?;
            report.tables.extend(changes.tables);
            report.pulled += page["rows"].as_array().map_or(0, Vec::len);
            if !more {
                break;
            }
        }
        if self.store.state("pull_buffer")?.is_some() {
            return Err(SyncError::Protocol("Pulled rows reference unavailable parents; waiting for server visibility or a later page.".into()));
        }
        self.store.finish_day_mapping()?;
        if deferred_days {
            let next = self.sync_once()?;
            report.pushed += next.pushed;
            report.pulled += next.pulled;
            report.rejected += next.rejected;
            report.tables.extend(next.tables);
            return Ok(report);
        }
        report.tables.extend(self.sync_blobs()?);
        self.store.set_state(
            "last_sync",
            &std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string(),
        )?;
        Ok(report)
    }
}
enum Signal {
    Nudge,
    Now,
    Stop,
}
pub struct SyncHandle {
    sender: Sender<Signal>,
    thread: Option<thread::JoinHandle<()>>,
}
impl SyncHandle {
    pub fn nudge(&self) {
        let _ = self.sender.send(Signal::Nudge);
    }
    pub fn now(&self) {
        let _ = self.sender.send(Signal::Now);
    }
    /// Stops after the current sync pass; drop never blocks the UI thread.
    pub fn stop(&self) {
        let _ = self.sender.send(Signal::Stop);
    }
    pub fn join(mut self) {
        self.stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
impl Drop for SyncHandle {
    fn drop(&mut self) {
        self.stop();
    }
}
pub fn spawn(
    store_path: impl AsRef<Path>,
    client: Client,
    events: Sender<SyncEvent>,
) -> SyncHandle {
    let path: PathBuf = store_path.as_ref().into();
    let (sender, receiver) = mpsc::channel();
    let thread = thread::spawn(move || {
        let store = match Store::open_secondary(path) {
            Ok(s) => s,
            Err(e) => {
                let _ = events.send(SyncEvent::Error(e.to_string()));
                return;
            }
        };
        let mut engine = SyncEngine::new(store, client);
        let mut deadline = Instant::now();
        let mut periodic = deadline;
        let mut backoff = Duration::from_secs(2);
        let mut retry = None;
        loop {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Signal::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Signal::Now) => deadline = Instant::now(),
                Ok(Signal::Nudge) => {
                    deadline = (Instant::now() + Duration::from_millis(500))
                        .min(periodic)
                        .max(retry.unwrap_or_else(Instant::now))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let _ = events.send(SyncEvent::Started);
                    let event = match engine.sync_once() {
                        Ok(report) => {
                            backoff = Duration::from_secs(2);
                            retry = None;
                            periodic = Instant::now() + Duration::from_secs(60);
                            deadline = periodic;
                            SyncEvent::Finished(report)
                        }
                        Err(e) => {
                            deadline = Instant::now() + backoff;
                            retry = Some(deadline);
                            backoff = (backoff * 2).min(Duration::from_secs(300));
                            match e {
                                SyncError::Offline | SyncError::Server(_) => SyncEvent::Offline,
                                SyncError::Unauthorized => {
                                    let _ = events.send(SyncEvent::NeedsSignIn);
                                    break;
                                }
                                SyncError::UpgradeRequired => {
                                    let _ = events.send(SyncEvent::UpgradeRequired);
                                    break;
                                }
                                e => SyncEvent::Error(e.to_string()),
                            }
                        }
                    };
                    if events.send(event).is_err() {
                        break;
                    }
                }
            }
        }
    });
    SyncHandle {
        sender,
        thread: Some(thread),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use strider_store::{Command, GearInput};
    #[test]
    fn outbox_ops_filter_metadata_convert_booleans_and_use_current_rows() {
        let mut store = Store::in_memory().unwrap();
        store
            .apply(Command::CreateGear {
                input: GearInput {
                    name: "Tent".into(),
                    quantity: 1,
                    class: "base".into(),
                    status: "current".into(),
                    notes: Some("text".into()),
                    ..Default::default()
                },
            })
            .unwrap();
        let batch = store.outbox_batches(500).unwrap().remove(0);
        let engine = SyncEngine::new(store, Client::new("http://localhost:3100", None).unwrap());
        let request = engine.operations(&batch).unwrap();
        let op = &request["ops"][0];
        assert_eq!(op["set"]["is_kit"], false);
        assert!(op["set"].get("sync_id").is_none());
        assert!(op["set"].get("updated_at").is_none());
        assert!(op["base_text"]["notes"].is_null());
        let mut store = engine.store;
        store
            .apply(Command::DeleteGear {
                id: batch.entries[0].sync_id.clone(),
            })
            .unwrap();
        let engine = SyncEngine::new(store, Client::new("http://localhost:3100", None).unwrap());
        assert_eq!(engine.operations(&batch).unwrap()["ops"][0]["op"], "delete");
    }
    #[test]
    fn long_text_uses_last_pulled_base() {
        let mut store = Store::in_memory().unwrap();
        store.apply_pull_page(&json!({"rows":[{"table":"meal","sync_id":"m","name":"Meal","notes":"base","server_seq":"5"}],"tombstones":[],"revoked_trips":[],"cursor":"5","more":false})).unwrap();
        store
            .apply(Command::UpdateMeal {
                id: "m".into(),
                patch: strider_store::MealPatch {
                    notes: Some(Some("local".into())),
                    ..Default::default()
                },
            })
            .unwrap();
        let batch = store.outbox_batches(500).unwrap().remove(0);
        let engine = SyncEngine::new(store, Client::new("http://localhost:3100", None).unwrap());
        let request = engine.operations(&batch).unwrap();
        assert_eq!(request["ops"][0]["base_text"]["notes"], "base");
        assert_eq!(request["ops"][0]["base_seq"], "5");
    }
    #[test]
    fn volume_edits_leave_server_derived_grams_out_of_patch() {
        for unit in ["ml", "l", "floz", "tsp", "tbsp", "cup"] {
            let mut store = Store::in_memory().unwrap();
            store.apply_pull_page(&json!({"rows":[
                {"table":"meal","sync_id":"m","name":"Meal","server_seq":"1"},
                {"table":"ingredient","sync_id":"i","name":"Ingredient","density_g_ml":"1","server_seq":"2"},
                {"table":"meal_ingredient","sync_id":"line","meal_id":"m","ingredient_id":"i","snapshot_name":"Ingredient","amount_g":1,"entered_amount":"1","entered_unit":unit,"server_seq":"3"}
            ],"tombstones":[],"revoked_trips":[],"cursor":"3","more":false})).unwrap();
            store
                .apply(Command::UpdateMealIngredientAmount {
                    id: "line".into(),
                    amount_g: 2,
                    entered: Some((2.into(), unit.into())),
                })
                .unwrap();
            let batch = store.outbox_batches(500).unwrap().remove(0);
            let engine =
                SyncEngine::new(store, Client::new("http://localhost:3100", None).unwrap());
            let request = engine.operations(&batch).unwrap();
            let op = request["ops"]
                .as_array()
                .unwrap()
                .iter()
                .find(|op| op["table"] == "meal_ingredient")
                .unwrap();
            assert!(op["set"].get("amount_g").is_none(), "{unit}");
            assert_eq!(op["set"]["entered_amount"], "2");
        }
    }
}
