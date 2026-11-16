use rusqlite::{params, types::Value};
use strider_store::*;
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn fields<const N: usize>(xs: [(&str, Value); N]) -> Row {
    xs.into_iter().map(|(k, v)| (k.into(), v)).collect()
}
fn id(s: &Store, t: &str) -> String {
    s.connection()
        .query_row(
            &format!("SELECT sync_id FROM {t} ORDER BY rowid DESC LIMIT 1"),
            [],
            |r| r.get(0),
        )
        .unwrap()
}
fn trip(s: &mut Store) -> String {
    s.apply(Command::CreateTrip {
        name: "Trip".into(),
        trail: None,
        loadout: None,
    })
    .unwrap();
    id(s, "trip")
}
fn capture(s: &Store) -> Vec<(String, String)> {
    TABLES
        .iter()
        .flat_map(|t| {
            let mut q = s
                .connection()
                .prepare(&format!("SELECT sync_id FROM {t} ORDER BY sync_id"))
                .unwrap();
            let ids = q
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            ids.into_iter()
                .map(|id| {
                    (
                        t.to_string(),
                        serde_json::to_string(&s.row_values(t, &id, &[]).unwrap()).unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
fn roundtrip(s: &mut Store, cmd: Command) {
    let before = capture(s);
    let changes = s.apply(cmd).unwrap();
    let after = capture(s);
    assert!(!changes.tables.is_empty() || before == after);
    assert!(s.undo().unwrap());
    assert_eq!(capture(s), before);
    assert!(s.redo().unwrap());
    assert_eq!(capture(s), after)
}
const GPX:&[u8]=b"<gpx><trkpt lat=\"37\" lon=\"-119\"><ele>100</ele></trkpt><trkpt lat=\"37.01\" lon=\"-119\"><ele>120</ele></trkpt><trkpt lat=\"37.02\" lon=\"-119\"><ele>110</ele></trkpt></gpx>";
#[test]
fn creation_loadout_import_and_trip_fields_roundtrip() {
    let mut s = Store::in_memory().unwrap();
    s.connection().execute_batch("INSERT INTO trail(sync_id,name,typical_distance_mi,typical_elevation_ft,region) VALUES('trail','Trail','20',500,'Sierra');INSERT INTO loadout(sync_id,name) VALUES('load','Load');INSERT INTO gear_item(sync_id,name,weight_g,quantity) VALUES('gear','Tent',400,2);INSERT INTO loadout_item(sync_id,loadout_id,gear_item_id) VALUES('member','load','gear');").unwrap();
    roundtrip(
        &mut s,
        Command::CreateTrip {
            name: " ".into(),
            trail: Some("trail".into()),
            loadout: Some("load".into()),
        },
    );
    let t = id(&s, "trip");
    let d = s.trip_detail(&t).unwrap().unwrap();
    assert_eq!(d.route.distance_mi, Some(20.));
    assert_eq!(d.weights.base, 800);
    assert_eq!(d.region.as_deref(), Some("Sierra"));
    assert_eq!(d.fields["name"], text("New Trip"));
    roundtrip(
        &mut s,
        Command::UpdateTrip {
            trip: t.clone(),
            patch: fields([
                ("name", text("Updated")),
                ("lat", text("bad")),
                ("trip_report", text("Private")),
            ]),
        },
    );
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().lat, None);
    roundtrip(
        &mut s,
        Command::SaveTripPackingList {
            trip: t.clone(),
            packing_list: "Check stove".into(),
        },
    );
    roundtrip(&mut s, Command::ClearTripGear { trip: t.clone() });
    roundtrip(
        &mut s,
        Command::SeedTripFromLoadout {
            trip: t.clone(),
            loadout: "load".into(),
        },
    );
    roundtrip(&mut s,Command::ImportTrip{json:r#"Prose ```json {"trip":{"name":"Imported","elevationGainFt":200},"route":{"track":[[37,-119],[37.02,-119]]},"campsites":[{"lat":37.01,"lon":-119}]} ```"#.into()});
    let imported = id(&s, "trip");
    assert_eq!(s.trip_detail(&imported).unwrap().unwrap().days.len(), 2);
    roundtrip(&mut s, Command::DeleteTrip { trip: imported });
    for (input, message) in [
        (
            "bad",
            "Couldn't find valid JSON. Paste the JSON code block the agent produced.",
        ),
        ("{}", "The file needs a trip.name field."),
    ] {
        let before = capture(&s);
        assert_eq!(
            s.apply(Command::ImportTrip { json: input.into() })
                .unwrap_err()
                .to_string(),
            message
        );
        assert_eq!(capture(&s), before);
    }
}
#[test]
fn gear_commands_snapshot_restore_and_access() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    s.connection()
        .execute_batch("INSERT INTO gear_item(sync_id,name,weight_g) VALUES('g','Pack',500)")
        .unwrap();
    roundtrip(
        &mut s,
        Command::AddTripGear {
            trip: t.clone(),
            gear: "g".into(),
        },
    );
    let g = id(&s, "trip_gear");
    for cmd in [
        Command::SetTripGearClass {
            trip: t.clone(),
            id: g.clone(),
            class: "worn".into(),
        },
        Command::SetTripGearQty {
            trip: t.clone(),
            id: g.clone(),
            qty: 2.5,
        },
        Command::SetTripGearPacked {
            trip: t.clone(),
            id: g.clone(),
            packed: true,
        },
        Command::MoveTripGear {
            trip: t.clone(),
            id: g.clone(),
            category: "Sleep".into(),
            ordered_ids: vec![g.clone()],
        },
    ] {
        roundtrip(&mut s, cmd)
    }
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().weights.worn, 1500);
    roundtrip(
        &mut s,
        Command::RemoveTripGear {
            trip: t.clone(),
            id: g,
        },
    );
    roundtrip(
        &mut s,
        Command::RestoreTripGear {
            trip: t.clone(),
            snapshot: fields([
                ("gear_item_id", text("unavailable")),
                ("snapshot_name", text("Owner's tent")),
                ("snapshot_weight_g", Value::Integer(700)),
                ("snapshot_category", Value::Null),
                ("quantity", Value::Integer(1)),
                ("weight_class", text("base")),
                ("packed", Value::Integer(0)),
            ]),
        },
    );
    let other = trip(&mut s);
    let before = capture(&s);
    assert!(
        s.apply(Command::RemoveTripGear {
            trip: other,
            id: id(&s, "trip_gear")
        })
        .is_err()
    );
    assert_eq!(before, capture(&s));
    s.connection()
        .execute(
            "UPDATE trip SET access='edit',trip_report='secret' WHERE sync_id=?",
            [&t],
        )
        .unwrap();
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().fields["trip_report"],
        Value::Null
    );
    assert_eq!(
        s.apply(Command::UpdateTrip {
            trip: t.clone(),
            patch: fields([("trip_report", text("x"))])
        })
        .unwrap_err()
        .to_string(),
        "Private notes are owner-only"
    );
    assert!(s.apply(Command::DeleteTrip { trip: t.clone() }).is_err());
    s.connection()
        .execute("UPDATE trip SET access='view' WHERE sync_id=?", [&t])
        .unwrap();
    assert!(s.apply(Command::ClearTripGear { trip: t }).is_err());
}
#[test]
fn days_campsites_gpx_derived_columns_and_reversal() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    roundtrip(
        &mut s,
        Command::UpsertTripDay {
            trip: t.clone(),
            day: 1,
            distance_mi: Some(Some("2.25".into())),
            elevation_gain_ft: Some(Some(200)),
        },
    );
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().route.distance_mi,
        Some(2.25)
    );
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .filter(|e| e.table == "trip")
            .all(|e| !e.columns.contains(&"distance_mi".into()) || e.base_seq.is_none())
    );
    roundtrip(&mut s, Command::AddTripDay { trip: t.clone() });
    let day = id(&s, "trip_day");
    roundtrip(
        &mut s,
        Command::DeleteTripDay {
            trip: t.clone(),
            id: day,
        },
    );
    roundtrip(
        &mut s,
        Command::AttachGpx {
            trip: t.clone(),
            filename: "route.gpx".into(),
            bytes: GPX.to_vec(),
        },
    );
    let original = s.trip_detail(&t).unwrap().unwrap().points;
    roundtrip(
        &mut s,
        Command::AddCampsite {
            trip: t.clone(),
            site: Site {
                distance_mi: 0.69,
                lat: Some(37.01),
                lon: Some(-119.),
                ele_ft: Some(394),
            },
        },
    );
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().segments.len(), 2);
    let c = id(&s, "trip_campsite");
    roundtrip(
        &mut s,
        Command::MoveCampsite {
            trip: t.clone(),
            id: c.clone(),
            site: Site {
                distance_mi: 0.7,
                lat: Some(37.01),
                lon: Some(-119.),
                ele_ft: None,
            },
        },
    );
    roundtrip(&mut s, Command::ReverseGpx { trip: t.clone() });
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().points[0].lat, 37.02);
    s.apply(Command::ReverseGpx { trip: t.clone() }).unwrap();
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().points, original);
    roundtrip(
        &mut s,
        Command::RemoveCampsite {
            trip: t.clone(),
            id: c,
        },
    );
    roundtrip(&mut s, Command::ClearCampsites { trip: t.clone() });
    let g = s.trip_detail(&t).unwrap().unwrap().gpx.unwrap();
    let snapshot = g
        .into_iter()
        .filter(|(k, _)| !["sync_id", "trip_id", "uploaded_at"].contains(&k.as_str()))
        .collect();
    roundtrip(&mut s, Command::RemoveGpx { trip: t.clone() });
    roundtrip(
        &mut s,
        Command::RestoreGpx {
            trip: t.clone(),
            snapshot,
        },
    );
    let before = capture(&s);
    assert_eq!(
        s.apply(Command::AttachGpx {
            trip: t,
            filename: "x".into(),
            bytes: vec![]
        })
        .unwrap_err()
        .to_string(),
        "No file selected."
    );
    assert_eq!(before, capture(&s));
}
#[test]
fn food_snapshots_scaling_shopping_water_and_library_propagation() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    s.connection().execute_batch("INSERT INTO ingredient(sync_id,name,kcal_per_100g) VALUES('i','Rice','350');INSERT INTO meal(sync_id,name,base_servings,water_ml) VALUES('m','Dinner',2,1000);INSERT INTO meal_ingredient(sync_id,meal_id,ingredient_id,snapshot_name,snapshot_kcal_per_100g,amount_g) VALUES('l','m','i','Rice','350',200)").unwrap();
    roundtrip(
        &mut s,
        Command::AddTripMeal {
            trip: t.clone(),
            day: 1,
            entry: AddTripFood::Meal {
                meal: "m".into(),
                servings: 1.,
            },
        },
    );
    let meal = id(&s, "trip_meal");
    let d = s.trip_detail(&t).unwrap().unwrap();
    assert_eq!(
        (
            d.food.total_g,
            d.food.total_kcal,
            d.cook_water_ml_per_person
        ),
        (100, 350, 500)
    );
    assert_eq!(d.shopping[0].grams, 100);
    roundtrip(
        &mut s,
        Command::SetTripMealQty {
            trip: t.clone(),
            id: meal.clone(),
            qty: 2.,
        },
    );
    roundtrip(&mut s, Command::AddTripDay { trip: t.clone() });
    roundtrip(&mut s, Command::AddTripDay { trip: t.clone() });
    roundtrip(
        &mut s,
        Command::CopyTripMealToAll {
            trip: t.clone(),
            id: meal.clone(),
        },
    );
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().food.total_g, 400);
    roundtrip(
        &mut s,
        Command::AddTripMeal {
            trip: t.clone(),
            day: 1,
            entry: AddTripFood::Ingredient {
                ingredient: "i".into(),
                grams: 50.,
            },
        },
    );
    let ing = s
        .connection()
        .query_row(
            "SELECT sync_id FROM trip_meal WHERE ingredient_id='i'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap();
    roundtrip(
        &mut s,
        Command::SetTripMealQty {
            trip: t.clone(),
            id: ing.clone(),
            qty: 75.,
        },
    );
    roundtrip(
        &mut s,
        Command::AddTripMeal {
            trip: t.clone(),
            day: 3,
            entry: AddTripFood::Hit {
                ingredient: Box::new(IngredientInput {
                    name: "Oats".into(),
                    kcal_per_100g: Some(400.into()),
                    source: Some("manual".into()),
                    source_id: Some("oats".into()),
                    ..Default::default()
                }),
                grams: 100.,
            },
        },
    );
    let d = s.trip_detail(&t).unwrap().unwrap();
    assert_eq!(d.food.days.len(), 3);
    assert_eq!(d.food.total_g, 575);
    roundtrip(
        &mut s,
        Command::RemoveTripMeal {
            trip: t.clone(),
            id: ing,
        },
    );
    roundtrip(&mut s, Command::RemoveTripMeal { trip: t, id: meal });
}
#[test]
fn permit_blob_validation_undo_and_sqlite_metadata_only() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    let bytes = b"%PDF permit".to_vec();
    let hash = blob_hash(&bytes);
    roundtrip(
        &mut s,
        Command::AttachPermit {
            trip: t.clone(),
            filename: "permit.pdf".into(),
            mime_type: "application/pdf".into(),
            bytes: bytes.clone(),
        },
    );
    assert_eq!(s.blob_bytes(&hash).unwrap(), Some(bytes));
    assert!(s.data_dir().join("blobs").join(&hash).is_file());
    let p = s.trip_detail(&t).unwrap().unwrap().permit.unwrap();
    assert_eq!(p.state, "local_only");
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .all(|e| e.table != "trip_permit")
    );
    roundtrip(&mut s, Command::DeletePermit { trip: t.clone() });
    assert!(s.trip_detail(&t).unwrap().unwrap().permit.is_none());
    s.undo().unwrap();
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().permit.unwrap().sha256,
        hash
    );
    for (bytes, mime, msg) in [
        (vec![], "application/pdf", "No file."),
        (vec![1], "text/plain", "Use a PDF or image (PNG/JPG/WEBP)."),
        (
            vec![0; PERMIT_MAX_BYTES + 1],
            "application/pdf",
            "File exceeds 15 MB.",
        ),
    ] {
        let before = capture(&s);
        assert_eq!(
            s.apply(Command::AttachPermit {
                trip: t.clone(),
                filename: "x".into(),
                mime_type: mime.into(),
                bytes
            })
            .unwrap_err()
            .to_string(),
            msg
        );
        assert_eq!(before, capture(&s));
    }
    assert!(s.receive_blob(&hash, b"corrupt").is_err());
    s.connection()
        .execute("UPDATE trip SET access='view' WHERE sync_id=?", params![t])
        .unwrap();
    assert!(s.apply(Command::DeletePermit { trip: t }).is_err());
    let _ = std::fs::remove_dir_all(s.data_dir());
}
#[test]
fn grouping_precedence_snapshot_only_and_read_only_queries() {
    let mut s = Store::in_memory().unwrap();
    for (name, date, nights, status) in [
        ("Upcoming", Some("2026-12-01"), 1, "planned"),
        ("Active", Some("2026-11-16"), 1, "planned"),
        ("Past", Some("2026-10-01"), 1, "planned"),
        ("Completed", None, 0, "completed"),
        ("Idea", None, 0, "idea"),
    ] {
        let t = trip(&mut s);
        s.connection()
            .execute(
                "UPDATE trip SET name=?,start_date=?,nights=?,status=? WHERE sync_id=?",
                params![name, date, nights, status, t],
            )
            .unwrap();
    }
    let before = capture(&s);
    let pending = s.pending_count().unwrap();
    let list = s.trips_list_on("2026-11-16").unwrap();
    assert_eq!(
        (list.upcoming.len(), list.in_progress.len(), list.past.len()),
        (2, 1, 2)
    );
    assert_eq!(before, capture(&s));
    assert_eq!(pending, s.pending_count().unwrap());
    let t = id(&s, "trip");
    s.connection()
        .execute("UPDATE trip SET access='edit' WHERE sync_id=?", [&t])
        .unwrap();
    s.connection().execute("INSERT INTO trip_gear(sync_id,trip_id,snapshot_name,snapshot_weight_g) VALUES('snap',?,'Shared pack',600)",[&t]).unwrap();
    s.connection().execute("INSERT INTO trip_meal(sync_id,trip_id,day_number,snapshot_name,snapshot_weight_g,snapshot_kcal) VALUES('food',?,1,'Shared dinner',150,500)",[&t]).unwrap();
    let d = s.trip_detail(&t).unwrap().unwrap();
    assert_eq!(
        (d.weights.base, d.food.total_kcal, d.shopping[0].grams),
        (600, 500, 150)
    );
    assert_eq!(d.cook_water_ml_per_person, 0);
}

#[test]
fn derived_trip_columns_and_generated_day_ids_stay_out_of_trip_outbox_including_undo() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    s.connection().execute("DELETE FROM outbox", []).unwrap();
    s.apply(Command::UpsertTripDay {
        trip: t.clone(),
        day: 1,
        distance_mi: Some(Some("10".into())),
        elevation_gain_ft: Some(Some(1000)),
    })
    .unwrap();
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .all(|e| e.table == "trip_day")
    );
    s.connection().execute("DELETE FROM outbox", []).unwrap();
    s.undo().unwrap();
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .all(|e| e.table == "trip_day")
    );
    s.redo().unwrap();
    s.apply(Command::AttachGpx {
        trip: t.clone(),
        filename: "route.gpx".into(),
        bytes: GPX.to_vec(),
    })
    .unwrap();
    s.connection().execute("DELETE FROM outbox", []).unwrap();
    s.apply(Command::AddCampsite {
        trip: t.clone(),
        site: Site {
            distance_mi: 0.7,
            lat: Some(37.01),
            lon: Some(-119.),
            ele_ft: None,
        },
    })
    .unwrap();
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .all(|e| e.table == "trip_campsite")
    );
    s.connection().execute("DELETE FROM outbox", []).unwrap();
    s.undo().unwrap();
    assert!(
        s.outbox_batches(500)
            .unwrap()
            .iter()
            .flat_map(|b| &b.entries)
            .all(|e| e.table == "trip_campsite")
    );
}
#[test]
fn pending_manual_day_edits_adopt_server_generated_ids() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    s.apply(Command::AttachGpx {
        trip: t.clone(),
        filename: "route.gpx".into(),
        bytes: GPX.to_vec(),
    })
    .unwrap();
    s.apply(Command::AddCampsite {
        trip: t.clone(),
        site: Site {
            distance_mi: 0.7,
            lat: None,
            lon: None,
            ele_ft: None,
        },
    })
    .unwrap();
    s.connection().execute("DELETE FROM outbox", []).unwrap();
    s.apply(Command::UpsertTripDay {
        trip: t.clone(),
        day: 1,
        distance_mi: Some(Some("9".into())),
        elevation_gain_ft: None,
    })
    .unwrap();
    let batch = s.outbox_batches(500).unwrap().remove(0);
    assert!(s.has_unresolved_day_edits(&batch).unwrap());
    s.apply_pull_page(&serde_json::json!({"rows":[{"table":"trip_day","sync_id":"server-day","trip_id":t,"day_number":1,"distance_mi":"0.7","elevation_gain_ft":100,"server_seq":"5"}],"tombstones":[],"revoked_trips":[],"cursor":"5","more":false})).unwrap();
    assert_eq!(
        s.row_values("trip_day", "server-day", &[])
            .unwrap()
            .unwrap()["distance_mi"],
        "9"
    );
    let batch = s.outbox_batches(500).unwrap().remove(0);
    assert_eq!(batch.entries[0].sync_id, "server-day");
    assert!(!s.has_unresolved_day_edits(&batch).unwrap());
    assert_eq!(s.trip_detail(&t).unwrap().unwrap().days.len(), 1);
}

#[test]
fn packing_defaults_and_list_summary_use_the_web_precedence() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s);
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().packing_list.as_deref(),
        Some(DEFAULT_PACKING_LIST)
    );
    s.connection().execute_batch("INSERT INTO report_settings(sync_id,packing_default) VALUES('settings','Default checklist');INSERT INTO trail(sync_id,name,typical_distance_mi,typical_elevation_ft,region) VALUES('trail','Trail','12',300,'Region')").unwrap();
    s.connection()
        .execute("UPDATE trip SET trail_id='trail' WHERE sync_id=?", [&t])
        .unwrap();
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().packing_list.as_deref(),
        Some("Default checklist")
    );
    s.apply(Command::SaveTripPackingList {
        trip: t.clone(),
        packing_list: String::new(),
    })
    .unwrap();
    assert_eq!(
        s.trip_detail(&t).unwrap().unwrap().packing_list.as_deref(),
        Some("")
    );
    s.apply(Command::AttachGpx {
        trip: t.clone(),
        filename: "route.gpx".into(),
        bytes: GPX.to_vec(),
    })
    .unwrap();
    let detail = s.trip_detail(&t).unwrap().unwrap();
    let list = s.trips_list_on("2026-11-16").unwrap();
    let summary = &list.upcoming[0];
    assert_eq!(summary.route, detail.route);
    assert_eq!(summary.base_g, detail.weights.base);
    assert_eq!(summary.skin_out_g, detail.weights.skin_out);
    assert_eq!(summary.region, detail.region);
    assert_eq!(summary.access, "owner");
    assert_eq!(summary.track.as_ref().unwrap().len(), 3);
}
