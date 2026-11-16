//! Maintainer-run against a real local server. Tokens A/B are two devices of
//! the same user; TOKEN_OTHER is a separate user for the isolation test.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use strider_store::{
    AddTripFood, Command, GearInput, IngredientInput, MealPatch, Site, Store, TABLES,
};
use strider_sync::{Client, engine::SyncEngine};
fn client(key: &str) -> Client {
    Client::new(
        &std::env::var("STRIDER_TEST_SERVER").expect("STRIDER_TEST_SERVER"),
        Some(std::env::var(key).expect(key)),
    )
    .unwrap()
}
fn batch_id() -> String {
    let bytes: [u8; 16] = rand::random();
    let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
struct Cleanup {
    client: Client,
    rows: Vec<(String, String)>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        for chunk in self.rows.iter().rev().collect::<Vec<_>>().chunks(500) {
            let ops = chunk
                .iter()
                .map(|(table, id)| json!({"op":"delete","table":table,"sync_id":id}))
                .collect::<Vec<_>>();
            let result = self.client.request(
                "POST",
                "/api/v1/sync/push",
                Some(&json!({"batch_id":batch_id(),"ops":ops})),
            );
            if result.is_err() {
                eprintln!(
                    "Integration cleanup failed; remove rows named STRIDER-DESKTOP-TEST from the local test account."
                );
            }
        }
    }
}
fn ids(s: &Store, t: &str) -> Vec<String> {
    assert!(TABLES.contains(&t));
    s.connection()
        .prepare(&format!("SELECT sync_id FROM {t} ORDER BY sync_id"))
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap()
}
fn rows(s: &Store) -> BTreeMap<(String, String), Value> {
    TABLES
        .iter()
        .flat_map(|t| {
            ids(s, t).into_iter().map(move |id| {
                let row = s.row_values(t, &id, &[]).unwrap().unwrap();
                ((t.to_string(), id), Value::Object(row))
            })
        })
        .collect()
}
fn settle(a: &mut SyncEngine, b: &mut SyncEngine) {
    a.sync_once().unwrap();
    b.sync_once().unwrap();
    a.sync_once().unwrap();
    b.sync_once().unwrap();
    assert_eq!(a.store.pending_count().unwrap(), 0);
    assert_eq!(b.store.pending_count().unwrap(), 0);
}
fn create(
    engine: &mut SyncEngine,
    kind: u64,
    name: String,
    cleanup: &mut Cleanup,
) -> (String, String) {
    let (table, command) = match kind % 3 {
        0 => (
            "gear_item",
            Command::CreateGear {
                input: GearInput {
                    name,
                    quantity: 1,
                    class: "base".into(),
                    status: "current".into(),
                    ..Default::default()
                },
            },
        ),
        1 => (
            "ingredient",
            Command::CreateIngredient {
                input: IngredientInput {
                    name,
                    ..Default::default()
                },
            },
        ),
        _ => ("meal", Command::CreateMeal { name }),
    };
    let before = ids(&engine.store, table);
    engine.store.apply(command).unwrap();
    let id = ids(&engine.store, table)
        .into_iter()
        .find(|id| !before.contains(id))
        .unwrap();
    let row = (table.to_string(), id);
    cleanup.rows.push(row.clone());
    row
}
#[test]
#[ignore = "requires local server and two same-user device tokens"]
fn seeded_two_device_convergence_200_steps() {
    let ca = client("STRIDER_TEST_TOKEN_A");
    let cb = client("STRIDER_TEST_TOKEN_B");
    let ma = ca.request("GET", "/api/v1/me", None).unwrap();
    let mb = cb.request("GET", "/api/v1/me", None).unwrap();
    assert_eq!(
        ma["email"], mb["email"],
        "A/B must be two devices for the same user"
    );
    let mut cleanup = Cleanup {
        client: ca.clone(),
        rows: vec![],
    };
    let mut a = SyncEngine::new(Store::in_memory().unwrap(), ca.clone());
    let mut b = SyncEngine::new(Store::in_memory().unwrap(), cb);
    settle(&mut a, &mut b);
    let nonce = batch_id();
    let mut seed = 0x51d_eed_u64;
    let mut managed = vec![];
    for step in 0..200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let engine = if seed & 1 == 0 { &mut a } else { &mut b };
        match (seed >> 32) % 5 {
            0 | 1 => managed.push(create(
                engine,
                seed >> 4,
                format!("STRIDER-DESKTOP-TEST-{nonce}-{step}"),
                &mut cleanup,
            )),
            2 | 3 if !managed.is_empty() => {
                let (table, id) = &managed[(seed as usize >> 8) % managed.len()];
                if engine.store.row_values(table, id, &[]).unwrap().is_some() {
                    let command = if (seed >> 32) % 5 == 3 {
                        match table.as_str() {
                            "meal" => Command::DeleteMeal { id: id.clone() },
                            "ingredient" => Command::DeleteIngredient { id: id.clone() },
                            _ => Command::DeleteGear { id: id.clone() },
                        }
                    } else {
                        match table.as_str() {
                            "meal" => Command::UpdateMeal {
                                id: id.clone(),
                                patch: MealPatch {
                                    notes: Some(Some(format!("Step {step}"))),
                                    ..Default::default()
                                },
                            },
                            "ingredient" => Command::UpdateIngredient {
                                id: id.clone(),
                                input: IngredientInput {
                                    name: format!("STRIDER-DESKTOP-TEST-{nonce}-{step}"),
                                    notes: Some(format!("Step {step}")),
                                    ..Default::default()
                                },
                            },
                            _ => Command::UpdateGear {
                                id: id.clone(),
                                input: GearInput {
                                    name: format!("STRIDER-DESKTOP-TEST-{nonce}-{step}"),
                                    quantity: 1,
                                    class: "base".into(),
                                    status: "current".into(),
                                    notes: Some(format!("Step {step}")),
                                    ..Default::default()
                                },
                            },
                        }
                    };
                    engine.store.apply(command).unwrap();
                }
            }
            _ => {
                engine.sync_once().unwrap();
            }
        }
    }
    settle(&mut a, &mut b);
    // A fresh full pull is the server's canonical snapshot, covering every table.
    let mut server = SyncEngine::new(Store::in_memory().unwrap(), ca);
    server.sync_once().unwrap();
    assert_eq!(rows(&a.store), rows(&b.store));
    assert_eq!(rows(&a.store), rows(&server.store));
}
#[test]
#[ignore = "requires local server and two same-user device tokens"]
fn replaced_long_text_is_saved() {
    let ca = client("STRIDER_TEST_TOKEN_A");
    let mut cleanup = Cleanup {
        client: ca.clone(),
        rows: vec![],
    };
    let mut a = SyncEngine::new(Store::in_memory().unwrap(), ca);
    let mut b = SyncEngine::new(Store::in_memory().unwrap(), client("STRIDER_TEST_TOKEN_B"));
    settle(&mut a, &mut b);
    let (_, id) = create(
        &mut a,
        2,
        format!("STRIDER-DESKTOP-TEST-{}", batch_id()),
        &mut cleanup,
    );
    a.store
        .apply(Command::UpdateMeal {
            id: id.clone(),
            patch: MealPatch {
                notes: Some(Some("Base".into())),
                ..Default::default()
            },
        })
        .unwrap();
    settle(&mut a, &mut b);
    a.store
        .apply(Command::UpdateMeal {
            id: id.clone(),
            patch: MealPatch {
                notes: Some(Some("A text".into())),
                ..Default::default()
            },
        })
        .unwrap();
    a.sync_once().unwrap();
    b.store
        .apply(Command::UpdateMeal {
            id,
            patch: MealPatch {
                notes: Some(Some("B text".into())),
                ..Default::default()
            },
        })
        .unwrap();
    b.sync_once().unwrap();
    assert!(
        b.store
            .sync_details()
            .unwrap()
            .iter()
            .any(|(_, value)| value == "A text")
    );
    settle(&mut a, &mut b);
    assert_eq!(rows(&a.store), rows(&b.store));
}
#[test]
#[ignore = "requires STRIDER_TEST_TOKEN_OTHER belonging to a different user"]
fn other_user_never_receives_private_rows() {
    let ca = client("STRIDER_TEST_TOKEN_A");
    let other = client("STRIDER_TEST_TOKEN_OTHER");
    assert_ne!(
        ca.request("GET", "/api/v1/me", None).unwrap()["email"],
        other.request("GET", "/api/v1/me", None).unwrap()["email"]
    );
    let mut cleanup = Cleanup {
        client: ca.clone(),
        rows: vec![],
    };
    let mut a = SyncEngine::new(Store::in_memory().unwrap(), ca);
    let (table, id) = create(
        &mut a,
        0,
        format!("STRIDER-DESKTOP-TEST-{}", batch_id()),
        &mut cleanup,
    );
    a.sync_once().unwrap();
    let mut b = SyncEngine::new(Store::in_memory().unwrap(), other);
    b.sync_once().unwrap();
    assert!(b.store.row_values(&table, &id, &[]).unwrap().is_none());
}

#[test]
#[ignore = "requires local server and two same-user device tokens"]
fn permit_round_trip_two_devices() {
    let ca = client("STRIDER_TEST_TOKEN_A");
    let cb = client("STRIDER_TEST_TOKEN_B");
    assert_eq!(
        ca.request("GET", "/api/v1/me", None).unwrap()["email"],
        cb.request("GET", "/api/v1/me", None).unwrap()["email"]
    );
    let mut cleanup = Cleanup {
        client: ca.clone(),
        rows: vec![],
    };
    let mut a = SyncEngine::new(Store::in_memory().unwrap(), ca);
    let mut b = SyncEngine::new(Store::in_memory().unwrap(), cb);
    a.store
        .apply(Command::CreateTrip {
            name: format!("STRIDER-DESKTOP-TEST-permit-{}", batch_id()),
            trail: None,
            loadout: None,
        })
        .unwrap();
    let trip = ids(&a.store, "trip").remove(0);
    cleanup.rows.push(("trip".into(), trip.clone()));
    let bytes = b"%PDF-1.4\nStrider desktop permit fixture\n%%EOF\n".to_vec();
    a.store
        .apply(Command::AttachPermit {
            trip: trip.clone(),
            filename: "permit.pdf".into(),
            mime_type: "application/pdf".into(),
            bytes: bytes.clone(),
        })
        .unwrap();
    settle(&mut a, &mut b);
    let permit = b
        .store
        .permit_blobs()
        .unwrap()
        .into_iter()
        .find(|p| p.trip_id == trip)
        .unwrap();
    assert_eq!(permit.state, "uploaded");
    assert_eq!(permit.filename, "permit.pdf");
    assert_eq!(b.store.blob_bytes(&permit.sha256).unwrap(), Some(bytes));
    a.store
        .apply(Command::DeletePermit { trip: trip.clone() })
        .unwrap();
    settle(&mut a, &mut b);
    assert!(
        !b.store
            .permit_blobs()
            .unwrap()
            .iter()
            .any(|p| p.trip_id == trip)
    );
    let _ = std::fs::remove_dir_all(a.store.data_dir());
    let _ = std::fs::remove_dir_all(b.store.data_dir());
}
#[test]
#[ignore = "requires local server and two same-user device tokens"]
fn trip_two_device_convergence() {
    let ca = client("STRIDER_TEST_TOKEN_A");
    let mut a = SyncEngine::new(Store::in_memory().unwrap(), ca.clone());
    let mut b = SyncEngine::new(Store::in_memory().unwrap(), client("STRIDER_TEST_TOKEN_B"));
    settle(&mut a, &mut b);
    let baseline: Vec<(String, String)> = TABLES
        .iter()
        .flat_map(|t| {
            ids(&a.store, t)
                .into_iter()
                .map(move |id| (t.to_string(), id))
        })
        .collect();
    let mut cleanup = Cleanup {
        client: ca.clone(),
        rows: vec![],
    };
    // Run the scenario in a closure so rows are registered for cleanup even if an assertion fails.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let nonce = batch_id();
        let name = format!("STRIDER-DESKTOP-TEST-{nonce}");
        let one = |s: &mut Store, table: &str, command: Command| {
            let before = ids(s, table);
            s.apply(command).unwrap();
            ids(s, table)
                .into_iter()
                .find(|id| !before.contains(id))
                .unwrap()
        };
        let gear = one(
            &mut a.store,
            "gear_item",
            Command::CreateGear {
                input: GearInput {
                    name: name.clone(),
                    quantity: 1,
                    class: "base".into(),
                    status: "current".into(),
                    ..Default::default()
                },
            },
        );
        let loadout = one(
            &mut a.store,
            "loadout",
            Command::CreateLoadout { name: name.clone() },
        );
        a.store
            .apply(Command::SetLoadoutMember {
                loadout: loadout.clone(),
                item: gear,
                present: true,
                class: "base".into(),
            })
            .unwrap();
        let trip = one(
            &mut a.store,
            "trip",
            Command::CreateTrip {
                name: name.clone(),
                trail: None,
                loadout: Some(loadout),
            },
        );
        a.store
            .apply(Command::AddTripDay { trip: trip.clone() })
            .unwrap();
        a.store
            .apply(Command::AddTripDay { trip: trip.clone() })
            .unwrap();
        let cases: Value =
            serde_json::from_str(include_str!("../../../fixtures/domain/gpx.json")).unwrap();
        let xml = cases[11]["args"][0].as_str().unwrap().as_bytes().to_vec();
        a.store
            .apply(Command::AttachGpx {
                trip: trip.clone(),
                filename: "route.gpx".into(),
                bytes: xml,
            })
            .unwrap();
        let points = a.store.trip_detail(&trip).unwrap().unwrap().points;
        let mid = &points[points.len() / 2];
        a.store
            .apply(Command::AddCampsite {
                trip: trip.clone(),
                site: Site {
                    distance_mi: mid.d,
                    lat: Some(mid.lat),
                    lon: Some(mid.lon),
                    ele_ft: Some(mid.e as i32),
                },
            })
            .unwrap();
        let ingredient = one(
            &mut a.store,
            "ingredient",
            Command::CreateIngredient {
                input: IngredientInput {
                    name: name.clone(),
                    ..Default::default()
                },
            },
        );
        a.store
            .apply(Command::AddTripMeal {
                trip: trip.clone(),
                day: 1,
                entry: AddTripFood::Ingredient {
                    ingredient,
                    grams: 80.,
                },
            })
            .unwrap();
        let tg = ids(&a.store, "trip_gear");
        a.store
            .apply(Command::SetTripGearPacked {
                trip: trip.clone(),
                id: tg[0].clone(),
                packed: true,
            })
            .unwrap();
        settle(&mut a, &mut b);
        assert_eq!(rows(&a.store), rows(&b.store));
        // Device B edits the same trip, then both converge with a fresh pull.
        b.store
            .apply(Command::SetTripGearPacked {
                trip: trip.clone(),
                id: tg[0].clone(),
                packed: false,
            })
            .unwrap();
        b.store
            .apply(Command::SetTripGearQty {
                trip: trip.clone(),
                id: tg[0].clone(),
                qty: 2.,
            })
            .unwrap();
        b.store
            .apply(Command::SaveTripPackingList {
                trip: trip.clone(),
                packing_list: "Stove\nWater".into(),
            })
            .unwrap();
        b.store
            .apply(Command::AddTripDay { trip: trip.clone() })
            .unwrap();
        settle(&mut a, &mut b);
        let mut server = SyncEngine::new(Store::in_memory().unwrap(), ca.clone());
        server.sync_once().unwrap();
        assert_eq!(rows(&a.store), rows(&b.store));
        assert_eq!(rows(&a.store), rows(&server.store));
        let detail = a.store.trip_detail(&trip).unwrap().unwrap();
        assert_eq!(detail.packed_count, 0);
        assert_eq!(detail.packing_list.as_deref(), Some("Stove\nWater"));
        assert_eq!(detail.campsites.len(), 1);
    }));
    for table in TABLES {
        for id in ids(&a.store, table) {
            if !baseline.contains(&(table.to_string(), id.clone())) {
                cleanup.rows.push((table.to_string(), id));
            }
        }
    }
    drop(cleanup);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
