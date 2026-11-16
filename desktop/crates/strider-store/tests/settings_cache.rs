use strider_store::*;

fn outbox(s: &Store) -> i64 {
    s.connection()
        .query_row("SELECT count(*) FROM outbox", [], |r| r.get(0))
        .unwrap()
}
fn trip(s: &mut Store, name: &str) -> String {
    s.execute(Command::CreateTrip {
        name: name.into(),
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

#[test]
fn cache_and_region_tables_exist_but_are_not_synced() {
    let s = Store::in_memory().unwrap();
    for t in [
        "weather_cache",
        "calendar_cache",
        "overpass_cache",
        "map_region",
    ] {
        assert!(!TABLES.contains(&t), "{t} must not be a synced table");
        let n: i64 = s
            .connection()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                [t],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "{t}");
    }
    s.cache_put(CacheKind::Weather, "k", 5, "{}").unwrap();
    s.save_map_region(&MapRegionRecord {
        trip_sync_id: "t".into(),
        south: 1.,
        west: 2.,
        north: 3.,
        east: 4.,
        min_zoom: 8,
        max_zoom: 15,
        tiles_total: 10,
        tiles_done: 3,
        bytes: 99,
        state: "downloading".into(),
        updated_at: 7,
    })
    .unwrap();
    assert_eq!(outbox(&s), 0);
}
#[test]
fn cache_round_trips_and_replaces() {
    let s = Store::in_memory().unwrap();
    assert_eq!(s.cache_get(CacheKind::Overpass, "a").unwrap(), None);
    s.cache_put(CacheKind::Overpass, "a", 10, "one").unwrap();
    s.cache_put(CacheKind::Overpass, "a", 20, "two").unwrap();
    s.cache_put(CacheKind::Calendar, "a", 30, "other table")
        .unwrap();
    assert_eq!(
        s.cache_get(CacheKind::Overpass, "a").unwrap(),
        Some(CacheEntry {
            fetched_at: 20,
            body: "two".into()
        })
    );
    s.cache_delete(CacheKind::Overpass, "a").unwrap();
    assert_eq!(s.cache_get(CacheKind::Overpass, "a").unwrap(), None);
    assert!(s.cache_get(CacheKind::Calendar, "a").unwrap().is_some());
}
#[test]
fn map_region_state_round_trips_and_rejects_unknown_states() {
    let s = Store::in_memory().unwrap();
    let mut r = MapRegionRecord {
        trip_sync_id: "t".into(),
        south: 39.5,
        west: -105.5,
        north: 40.5,
        east: -104.5,
        min_zoom: 8,
        max_zoom: 15,
        tiles_total: 100,
        tiles_done: 40,
        bytes: 1_000,
        state: "downloading".into(),
        updated_at: 1,
    };
    s.save_map_region(&r).unwrap();
    r.state = "complete".into();
    r.tiles_done = 100;
    s.save_map_region(&r).unwrap();
    assert_eq!(s.map_regions().unwrap(), vec![r.clone()]);
    assert_eq!(s.map_region("t").unwrap(), Some(r.clone()));
    r.state = "bogus".into();
    assert!(s.save_map_region(&r).is_err());
    s.delete_map_region("t").unwrap();
    assert!(s.map_region("t").unwrap().is_none());
}
#[test]
fn energy_and_report_settings_use_the_web_clamps_and_sync() {
    let mut s = Store::in_memory().unwrap();
    assert_eq!(s.energy_settings().unwrap(), EnergySettings::default());
    s.execute(Command::UpdateEnergyParams {
        input: EnergyInput {
            bmr: -5.4,
            cal_per_mile: 210.6,
            ft_per_mile: 0.2,
            sex: Some(" ".into()),
            weight_lb: Some("165.5".parse().unwrap()),
            height_in: None,
            age_years: Some(41),
        },
    })
    .unwrap();
    let e = s.energy_settings().unwrap();
    assert_eq!((e.bmr, e.cal_per_mile, e.ft_per_mile), (0, 211, 1));
    assert_eq!(e.sex, None);
    assert_eq!(e.weight_lb.map(|d| d.to_string()), Some("165.5".into()));
    assert_eq!(e.age_years, Some(41));
    // A second save updates the singleton row.
    s.execute(Command::UpdateEnergyParams {
        input: EnergyInput {
            bmr: 1800.,
            cal_per_mile: 200.,
            ft_per_mile: 625.,
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(s.energy_settings().unwrap().bmr, 1800);
    let rows: i64 = s
        .connection()
        .query_row("SELECT count(*) FROM energy_params", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
    s.execute(Command::UpdateReportSettings {
        template: "T".into(),
        packing_default: "P".into(),
    })
    .unwrap();
    s.execute(Command::UpdateReportSettings {
        template: "T2".into(),
        packing_default: "P".into(),
    })
    .unwrap();
    assert_eq!(
        s.report_settings().unwrap(),
        ReportSettings {
            template: Some("T2".into()),
            packing_default: Some("P".into())
        }
    );
    assert!(outbox(&s) > 0, "settings are synced rows");
    // Undo restores the previous report settings.
    assert!(s.undo().unwrap());
    assert_eq!(s.report_settings().unwrap().template.as_deref(), Some("T"));
}
#[test]
fn calendar_feeds_add_update_reorder_delete() {
    let mut s = Store::in_memory().unwrap();
    let add = |s: &mut Store, label: &str, url: &str| {
        s.execute(Command::AddCalendarFeed {
            label: label.into(),
            color: "#123456".into(),
            url: url.into(),
        })
    };
    assert!(add(&mut s, "x", "  ").is_err());
    add(&mut s, "  ", " https://a.example/a.ics ").unwrap();
    add(&mut s, "Work", "https://b.example/b.ics").unwrap();
    add(&mut s, "Home", "https://c.example/c.ics").unwrap();
    let feeds = s.calendar_feeds().unwrap();
    assert_eq!(
        feeds.iter().map(|f| f.label.as_str()).collect::<Vec<_>>(),
        ["Calendar", "Work", "Home"]
    );
    assert_eq!(feeds[0].url, "https://a.example/a.ics");
    let (a, b, c) = (
        feeds[0].id.clone(),
        feeds[1].id.clone(),
        feeds[2].id.clone(),
    );
    s.cache_put(CacheKind::Calendar, &b, 1, "BEGIN:VCALENDAR")
        .unwrap();
    s.execute(Command::UpdateCalendarFeed {
        id: b.clone(),
        label: "Job".into(),
        color: "#654321".into(),
        url: "https://b.example/new.ics".into(),
    })
    .unwrap();
    assert!(
        s.cache_get(CacheKind::Calendar, &b).unwrap().is_none(),
        "a changed address drops its cache"
    );
    s.execute(Command::ReorderCalendarFeeds {
        ordered_ids: vec![c.clone(), a.clone(), b.clone()],
    })
    .unwrap();
    let order: Vec<_> = s
        .calendar_feeds()
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(order, [c.clone(), a.clone(), b.clone()]);
    assert!(
        s.execute(Command::ReorderCalendarFeeds {
            ordered_ids: vec![a.clone()]
        })
        .is_err()
    );
    s.cache_put(CacheKind::Calendar, &a, 1, "x").unwrap();
    s.execute(Command::DeleteCalendarFeed { id: a.clone() })
        .unwrap();
    assert!(s.cache_get(CacheKind::Calendar, &a).unwrap().is_none());
    assert_eq!(s.calendar_feeds().unwrap().len(), 2);
    assert!(s.execute(Command::DeleteCalendarFeed { id: a }).is_err());
}
#[test]
fn shared_trips_group_by_permission_and_calendar_trips_are_owned_only() {
    let mut s = Store::in_memory().unwrap();
    let own = trip(&mut s, "Own");
    let edit = trip(&mut s, "Edit me");
    let view = trip(&mut s, "View me");
    for (id, access) in [(&edit, "edit"), (&view, "view")] {
        s.connection()
            .execute("UPDATE trip SET access=? WHERE sync_id=?", [access, id])
            .unwrap();
    }
    for id in [&own, &edit, &view] {
        s.connection()
            .execute(
                "UPDATE trip SET start_date='2025-09-10', nights=2 WHERE sync_id=?",
                [id],
            )
            .unwrap();
    }
    let shared = s.shared_trips().unwrap();
    assert_eq!(
        shared
            .edit
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["Edit me"]
    );
    assert_eq!(
        shared
            .view
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["View me"]
    );
    let cal = s.calendar_trips().unwrap();
    assert_eq!(
        cal,
        vec![CalendarTrip {
            sync_id: own,
            name: "Own".into(),
            start_date: "2025-09-10".into(),
            nights: 2
        }]
    );
}
#[test]
fn trip_track_reads_the_gpx_route() {
    let mut s = Store::in_memory().unwrap();
    let t = trip(&mut s, "Route");
    assert!(s.trip_track(&t).unwrap().is_empty());
    s.connection()
        .execute(
            "INSERT INTO trip_gpx(sync_id, trip_id, track) VALUES('g', ?, '[[40.0,-105.0],[40.5,-105.5]]')",
            [&t],
        )
        .unwrap();
    assert_eq!(
        s.trip_track(&t).unwrap(),
        vec![[40.0, -105.0], [40.5, -105.5]]
    );
}
