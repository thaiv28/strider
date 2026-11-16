use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::VecDeque;
use strider_store::{Command, Store, blob_hash};
use strider_sync::{Result, SyncError, Transport, engine::SyncEngine};
type Upload = (String, String, String, Vec<u8>);
#[derive(Default)]
struct Fake {
    pages: RefCell<VecDeque<Value>>,
    uploads: RefCell<Vec<Upload>>,
    downloads: RefCell<Vec<String>>,
    bytes: Vec<u8>,
    get_error: RefCell<Option<SyncError>>,
    fail_upload: RefCell<Option<usize>>,
    requests: RefCell<Vec<Value>>,
}
impl Transport for Fake {
    fn origin(&self) -> String {
        "http://fake.test".into()
    }
    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        match (method, path) {
            ("GET", "/api/v1/me") => Ok(json!({"email":"test@example.test"})),
            ("POST", "/api/v1/sync/push") => {
                self.requests.borrow_mut().push(body.unwrap().clone());
                Ok(json!({"adopt":[],"replaced":[]}))
            }
            ("GET", p) if p.starts_with("/api/v1/sync/pull?") => Ok(self
                .pages
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| page(vec![]))),
            _ => panic!("Unexpected {method} {path}"),
        }
    }
    fn upload_permit(&self, trip: &str, name: &str, mime: &str, bytes: &[u8]) -> Result<String> {
        let index = self.uploads.borrow().len();
        if *self.fail_upload.borrow() == Some(index) {
            self.fail_upload.borrow_mut().take();
            return Err(SyncError::Offline);
        }
        self.uploads
            .borrow_mut()
            .push((trip.into(), name.into(), mime.into(), bytes.to_vec()));
        Ok(blob_hash(bytes))
    }
    fn download_permit(&self, trip: &str) -> Result<Vec<u8>> {
        self.downloads.borrow_mut().push(trip.into());
        if let Some(e) = self.get_error.borrow_mut().take() {
            Err(e)
        } else {
            Ok(self.bytes.clone())
        }
    }
}
fn page(rows: Vec<Value>) -> Value {
    json!({"rows":rows,"tombstones":[],"revoked_trips":[],"cursor":"10","more":false})
}
fn trip(s: &mut Store) -> String {
    s.apply(Command::CreateTrip {
        name: "Permit trip".into(),
        trail: None,
        loadout: None,
    })
    .unwrap();
    s.connection()
        .query_row(
            "SELECT sync_id FROM trip ORDER BY rowid DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap()
}
fn attach(s: &mut Store, t: &str, bytes: &[u8]) {
    s.apply(Command::AttachPermit {
        trip: t.into(),
        filename: "permit.pdf".into(),
        mime_type: "application/pdf".into(),
        bytes: bytes.to_vec(),
    })
    .unwrap();
}
fn permit(t: &str, bytes: &[u8]) -> Value {
    json!({"table":"trip_permit","sync_id":"remote-permit","trip_id":t,"filename":"remote.pdf","mime_type":"application/pdf","size_bytes":bytes.len(),"sha256":blob_hash(bytes),"server_seq":"9"})
}
#[test]
fn upload_follows_parent_push_and_pulled_identity_is_adopted() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    let bytes = b"%PDF local";
    attach(&mut s, &t, bytes);
    let fake = Fake::default();
    fake.pages
        .borrow_mut()
        .push_back(page(vec![permit(&t, b"old file")]));
    let mut e = SyncEngine::new(s, fake);
    let report = e.sync_once().unwrap();
    assert_eq!(report.pushed, 1);
    assert_eq!(e.client.uploads.borrow().len(), 1);
    let u = &e.client.uploads.borrow()[0];
    assert_eq!(
        (&u.0, &u.1, &u.2, u.3.as_slice()),
        (
            &t,
            &"permit.pdf".into(),
            &"application/pdf".into(),
            bytes.as_slice()
        )
    );
    assert_eq!(e.store.permit_blobs().unwrap()[0].state, "uploaded");
    assert_eq!(e.store.permit_blobs().unwrap()[0].sync_id, "remote-permit");
    assert_eq!(e.store.permit_blobs().unwrap()[0].sha256, blob_hash(bytes));
    assert_eq!(e.store.pending_count().unwrap(), 0);
    let _ = std::fs::remove_dir_all(e.store.data_dir());
}
#[test]
fn download_verifies_hash_persists_and_does_not_repeat() {
    let root = std::env::temp_dir().join(format!("strider-blob-test-{}", rand::random::<u64>()));
    let path = root.join("workspace.db");
    let bytes = b"%PDF remote".to_vec();
    let fake = Fake {
        bytes: bytes.clone(),
        ..Default::default()
    };
    fake.pages.borrow_mut().push_back(page(vec![
        json!({"table":"trip","sync_id":"t","name":"Shared","access":"view","server_seq":"1"}),
        permit("t", &bytes),
    ]));
    let mut e = SyncEngine::new(Store::open(&path).unwrap(), fake);
    e.sync_once().unwrap();
    assert_eq!(e.client.downloads.borrow().len(), 1);
    assert_eq!(
        e.store.blob_bytes(&blob_hash(&bytes)).unwrap(),
        Some(bytes.clone())
    );
    assert_eq!(e.store.pending_count().unwrap(), 0);
    e.sync_once().unwrap();
    assert_eq!(e.client.downloads.borrow().len(), 1);
    drop(e);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.permit_blobs().unwrap()[0].state, "uploaded");
    assert_eq!(s.blob_bytes(&blob_hash(&bytes)).unwrap(), Some(bytes));
    drop(s);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn corrupt_and_unavailable_downloads_are_missing_and_retryable() {
    for error in [None, Some(SyncError::Server(404)), Some(SyncError::Offline)] {
        let fake = Fake {
            bytes: b"corrupt".to_vec(),
            get_error: RefCell::new(error),
            ..Default::default()
        };
        fake.pages.borrow_mut().push_back(page(vec![
            json!({"table":"trip","sync_id":"t","name":"Trip","server_seq":"1"}),
            permit("t", b"expected"),
        ]));
        let mut e = SyncEngine::new(Store::in_memory().unwrap(), fake);
        let _ = e.sync_once();
        assert_eq!(e.store.permit_blobs().unwrap()[0].state, "missing");
        assert_eq!(e.store.blob_bytes(&blob_hash(b"expected")).unwrap(), None);
        e.client.bytes = b"expected".to_vec();
        e.sync_once().unwrap();
        assert_eq!(e.store.permit_blobs().unwrap()[0].state, "uploaded");
        let _ = std::fs::remove_dir_all(e.store.data_dir());
    }
}
#[test]
fn identical_content_on_two_trips_retries_each_upload() {
    let mut s = Store::in_memory().unwrap();
    let a = trip(&mut s);
    let b = trip(&mut s);
    attach(&mut s, &a, b"same");
    attach(&mut s, &b, b"same");
    let fake = Fake {
        fail_upload: RefCell::new(Some(1)),
        ..Default::default()
    };
    let mut e = SyncEngine::new(s, fake);
    assert_eq!(e.sync_once().unwrap_err(), SyncError::Offline);
    assert_eq!(
        e.store
            .permit_blobs()
            .unwrap()
            .iter()
            .filter(|p| p.state == "local_only")
            .count(),
        1
    );
    e.sync_once().unwrap();
    assert_eq!(e.client.uploads.borrow().len(), 2);
    assert!(
        e.store
            .permit_blobs()
            .unwrap()
            .iter()
            .all(|p| p.state == "uploaded")
    );
    let _ = std::fs::remove_dir_all(e.store.data_dir());
}
#[test]
fn deletion_before_server_permit_identity_is_known_is_not_lost() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    attach(&mut s, &t, b"file");
    let mut e = SyncEngine::new(s, Fake::default());
    e.sync_once().unwrap();
    e.store
        .apply(Command::DeletePermit { trip: t.clone() })
        .unwrap();
    e.client
        .pages
        .borrow_mut()
        .push_back(page(vec![permit(&t, b"file")]));
    e.sync_once().unwrap();
    assert!(e.store.permit_blobs().unwrap().is_empty());
    assert!(e.store.pending_count().unwrap() > 0);
    e.sync_once().unwrap();
    let requests = e.client.requests.borrow();
    assert!(
        requests
            .iter()
            .flat_map(|r| r["ops"].as_array().unwrap())
            .any(|op| op["table"] == "trip_permit"
                && op["sync_id"] == "remote-permit"
                && op["op"] == "delete")
    );
    let _ = std::fs::remove_dir_all(e.store.data_dir());
}

#[test]
fn manual_day_patch_waits_for_campsite_day_identity_then_pushes() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    s.apply(Command::AttachGpx {
        trip: t.clone(),
        filename: "route.gpx".into(),
        bytes: b"<trkpt lat=\"37\" lon=\"-119\"></trkpt><trkpt lat=\"37.02\" lon=\"-119\"></trkpt>"
            .to_vec(),
    })
    .unwrap();
    s.apply(Command::AddCampsite {
        trip: t.clone(),
        site: strider_store::Site {
            distance_mi: 0.7,
            lat: None,
            lon: None,
            ele_ft: None,
        },
    })
    .unwrap();
    s.apply(Command::UpsertTripDay {
        trip: t.clone(),
        day: 1,
        distance_mi: Some(Some("9".into())),
        elevation_gain_ft: None,
    })
    .unwrap();
    let fake = Fake::default();
    fake.pages.borrow_mut().push_back(page(vec![json!({"table":"trip_day","sync_id":"remote-day-1","trip_id":t,"day_number":1,"distance_mi":"0.7","server_seq":"5"}),json!({"table":"trip_day","sync_id":"remote-day-2","trip_id":t,"day_number":2,"distance_mi":"0.68","server_seq":"6"})]));
    let mut e = SyncEngine::new(s, fake);
    e.sync_once().unwrap();
    let reqs = e.client.requests.borrow();
    let day_ops = reqs
        .iter()
        .flat_map(|r| r["ops"].as_array().unwrap())
        .filter(|r| r["table"] == "trip_day")
        .collect::<Vec<_>>();
    assert_eq!(day_ops.len(), 1);
    assert_eq!(day_ops[0]["sync_id"], "remote-day-1");
    assert_eq!(day_ops[0]["set"]["distance_mi"], "9");
    assert_eq!(e.store.pending_count().unwrap(), 0);
}
