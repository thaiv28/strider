//! Server schema contract: public SQLite introspection against the server's columns.
use rusqlite::Connection;
use std::{collections::BTreeMap, path::PathBuf};
use strider_store::Store;
use uuid::Uuid;

struct WorkspaceFile(PathBuf);
impl WorkspaceFile {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("strider-schema-{}.db", Uuid::now_v7())))
    }
}
impl Drop for WorkspaceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(self.0.with_extension("db-wal"));
        let _ = std::fs::remove_file(self.0.with_extension("db-shm"));
    }
}

#[test]
fn synced_columns_match_server_with_local_identity_and_permit_metadata() {
    let file = WorkspaceFile::new();
    drop(Store::open(&file.0).expect("fresh workspace"));
    let db = Connection::open(&file.0).unwrap();
    let expected: &[(&str, &[(&str, &str)])] = &[
        (
            "gear_category",
            &[
                ("sync_id", "TEXT"),
                ("name", "TEXT"),
                ("parent_id", "TEXT"),
                ("sort_order", "INTEGER"),
                ("weight_class", "TEXT"),
            ],
        ),
        (
            "gear_item",
            &[
                ("sync_id", "TEXT"),
                ("category_id", "TEXT"),
                ("name", "TEXT"),
                ("weight_g", "INTEGER"),
                ("price_cents", "INTEGER"),
                ("quantity", "INTEGER"),
                ("sort_order", "INTEGER"),
                ("is_kit", "INTEGER"),
                ("product_url", "TEXT"),
                ("notes", "TEXT"),
                ("acquired_at", "TEXT"),
                ("retired_at", "TEXT"),
                ("created_at", "TEXT"),
                ("updated_at", "TEXT"),
                ("default_weight_class", "TEXT"),
                ("status", "TEXT"),
            ],
        ),
        (
            "gear_component",
            &[
                ("sync_id", "TEXT"),
                ("gear_item_id", "TEXT"),
                ("name", "TEXT"),
                ("weight_g", "INTEGER"),
                ("quantity", "INTEGER"),
                ("notes", "TEXT"),
            ],
        ),
        (
            "wishlist_replacement",
            &[
                ("sync_id", "TEXT"),
                ("wishlist_item_id", "TEXT"),
                ("replaces_item_id", "TEXT"),
            ],
        ),
        (
            "wishlist_loadout",
            &[
                ("sync_id", "TEXT"),
                ("wishlist_item_id", "TEXT"),
                ("loadout_id", "TEXT"),
                ("included", "INTEGER"),
            ],
        ),
        (
            "loadout",
            &[
                ("sync_id", "TEXT"),
                ("name", "TEXT"),
                ("is_default", "INTEGER"),
                ("sort_order", "INTEGER"),
                ("notes", "TEXT"),
                ("created_at", "TEXT"),
            ],
        ),
        (
            "loadout_item",
            &[
                ("sync_id", "TEXT"),
                ("loadout_id", "TEXT"),
                ("gear_item_id", "TEXT"),
                ("weight_class", "TEXT"),
            ],
        ),
        (
            "trail",
            &[
                ("sync_id", "TEXT"),
                ("name", "TEXT"),
                ("region", "TEXT"),
                ("area_type", "TEXT"),
                ("typical_distance_mi", "TEXT"),
                ("typical_elevation_ft", "INTEGER"),
                ("notes", "TEXT"),
            ],
        ),
        (
            "trip",
            &[
                ("sync_id", "TEXT"),
                ("trail_id", "TEXT"),
                ("name", "TEXT"),
                ("start_date", "TEXT"),
                ("nights", "INTEGER"),
                ("party_size", "INTEGER"),
                ("grocery_people", "INTEGER"),
                ("grocery_removed", "TEXT"),
                ("region", "TEXT"),
                ("area_type", "TEXT"),
                ("lat", "TEXT"),
                ("lon", "TEXT"),
                ("distance_mi", "TEXT"),
                ("elevation_gain_ft", "INTEGER"),
                ("food_g_per_day", "INTEGER"),
                ("water_g_per_day", "INTEGER"),
                ("fuel_g_per_day", "INTEGER"),
                ("trailhead", "TEXT"),
                ("permit_required", "INTEGER"),
                ("permit_notes", "TEXT"),
                ("driving_notes", "TEXT"),
                ("water_sources", "TEXT"),
                ("planning_notes", "TEXT"),
                ("trip_report", "TEXT"),
                ("share_token", "TEXT"),
                ("edit_token", "TEXT"),
                ("packing_list", "TEXT"),
                ("created_at", "TEXT"),
                ("updated_at", "TEXT"),
                ("status", "TEXT"),
                ("access", "TEXT"),
            ],
        ),
        (
            "trip_day",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("day_number", "INTEGER"),
                ("distance_mi", "TEXT"),
                ("elevation_gain_ft", "INTEGER"),
            ],
        ),
        (
            "trip_gear",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("gear_item_id", "TEXT"),
                ("snapshot_name", "TEXT"),
                ("snapshot_weight_g", "INTEGER"),
                ("snapshot_category", "TEXT"),
                ("quantity", "INTEGER"),
                ("packed", "INTEGER"),
                ("sort_order", "INTEGER"),
                ("weight_class", "TEXT"),
            ],
        ),
        (
            "trip_campsite",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("distance_mi", "TEXT"),
                ("lat", "TEXT"),
                ("lon", "TEXT"),
                ("ele_ft", "INTEGER"),
            ],
        ),
        (
            "trip_gpx",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("filename", "TEXT"),
                ("start_lat", "TEXT"),
                ("start_lon", "TEXT"),
                ("distance_mi", "TEXT"),
                ("elevation_gain_ft", "INTEGER"),
                ("min_ele_ft", "INTEGER"),
                ("max_ele_ft", "INTEGER"),
                ("track", "TEXT"),
                ("profile", "TEXT"),
                ("points", "TEXT"),
                ("uploaded_at", "TEXT"),
            ],
        ),
        (
            "trip_permit",
            &[
                ("sync_id", "TEXT"),
                ("sha256", "TEXT"),
                ("trip_id", "TEXT"),
                ("filename", "TEXT"),
                ("mime_type", "TEXT"),
                ("size_bytes", "INTEGER"),
                ("uploaded_at", "TEXT"),
            ],
        ),
        (
            "trip_meal",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("day_number", "INTEGER"),
                ("meal_id", "TEXT"),
                ("ingredient_id", "TEXT"),
                ("servings", "TEXT"),
                ("snapshot_name", "TEXT"),
                ("snapshot_kcal", "INTEGER"),
                ("snapshot_weight_g", "INTEGER"),
                ("sort_order", "INTEGER"),
                ("meal_type", "TEXT"),
            ],
        ),
        (
            "trip_consumable",
            &[
                ("sync_id", "TEXT"),
                ("trip_id", "TEXT"),
                ("label", "TEXT"),
                ("grams", "INTEGER"),
                ("day_number", "INTEGER"),
                ("type", "TEXT"),
            ],
        ),
        (
            "ingredient",
            &[
                ("sync_id", "TEXT"),
                ("name", "TEXT"),
                ("kcal_per_100g", "TEXT"),
                ("density_g_ml", "TEXT"),
                ("density_source", "TEXT"),
                ("reference_name", "TEXT"),
                ("default_serving_g", "INTEGER"),
                ("category", "TEXT"),
                ("source", "TEXT"),
                ("source_id", "TEXT"),
                ("notes", "TEXT"),
            ],
        ),
        (
            "meal",
            &[
                ("sync_id", "TEXT"),
                ("name", "TEXT"),
                ("base_servings", "INTEGER"),
                ("is_hot", "INTEGER"),
                ("water_ml", "INTEGER"),
                ("notes", "TEXT"),
                ("created_at", "TEXT"),
                ("meal_type", "TEXT"),
            ],
        ),
        (
            "meal_ingredient",
            &[
                ("sync_id", "TEXT"),
                ("meal_id", "TEXT"),
                ("ingredient_id", "TEXT"),
                ("snapshot_name", "TEXT"),
                ("snapshot_kcal_per_100g", "TEXT"),
                ("snapshot_density_g_ml", "TEXT"),
                ("entered_amount", "TEXT"),
                ("entered_unit", "TEXT"),
                ("amount_g", "INTEGER"),
                ("sort_order", "INTEGER"),
            ],
        ),
        (
            "calendar_feed",
            &[
                ("sync_id", "TEXT"),
                ("label", "TEXT"),
                ("color", "TEXT"),
                ("url", "TEXT"),
                ("sort_order", "INTEGER"),
            ],
        ),
        (
            "energy_params",
            &[
                ("sync_id", "TEXT"),
                ("bmr", "INTEGER"),
                ("cal_per_energy_mile", "INTEGER"),
                ("ft_per_energy_mile", "INTEGER"),
                ("sex", "TEXT"),
                ("weight_lb", "TEXT"),
                ("height_in", "TEXT"),
                ("age_years", "INTEGER"),
            ],
        ),
        (
            "report_settings",
            &[
                ("sync_id", "TEXT"),
                ("template", "TEXT"),
                ("packing_default", "TEXT"),
            ],
        ),
    ];
    for (table, columns) in expected {
        let actual: BTreeMap<String, String> = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap()
            .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let expected: BTreeMap<String, String> = columns
            .iter()
            .map(|(name, kind)| (name.to_string(), kind.to_string()))
            .collect();
        assert_eq!(actual, expected, "{table} columns and affinities");
        let pk: String = db
            .query_row(
                &format!("SELECT name FROM pragma_table_info('{table}') WHERE pk = 1"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pk, "sync_id", "{table} identity");
        let foreign_keys: Vec<(String, String)> = db
            .prepare(&format!("PRAGMA foreign_key_list({table})"))
            .unwrap()
            .query_map([], |row| Ok((row.get(3)?, row.get(4)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for (column, target) in foreign_keys {
            assert_eq!(actual[&column], "TEXT", "{table}.{column} uses sync id");
            assert_eq!(target, "sync_id");
        }
    }
    for table in ["outbox", "row_meta", "sync_state", "conflict", "blob_ref"] {
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |r| r.get::<_, i32>(0)
            )
            .unwrap(),
            1
        );
    }
    assert!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap()
            > 0
    );
    assert_eq!(
        db.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
}

#[test]
fn future_workspace_version_is_refused_without_modifying_it() {
    let file = WorkspaceFile::new();
    drop(Store::open(&file.0).unwrap());
    let db = Connection::open(&file.0).unwrap();
    db.pragma_update(None, "user_version", 9999).unwrap();
    drop(db);
    assert!(Store::open(&file.0).is_err());
    let db = Connection::open(&file.0).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        9999
    );
}

#[test]
fn profile_names_stay_within_application_support() {
    assert_eq!(
        Store::default_path().unwrap(),
        Store::path_for_profile("dev").unwrap()
    );
    assert!(
        Store::path_for_profile("test-1")
            .unwrap()
            .ends_with("Strider/test-1/workspace.db")
    );
    for name in ["", ".", "..", "../escape", "/tmp/escape", "a/b"] {
        assert!(Store::path_for_profile(name).is_err(), "{name:?}");
    }
}

#[test]
fn foreign_keys_preserve_server_delete_rules() {
    let file = WorkspaceFile::new();
    drop(Store::open(&file.0).unwrap());
    let db = Connection::open(&file.0).unwrap();
    let expected = [
        ("gear_category", "parent_id", "gear_category", "NO ACTION"),
        ("gear_item", "category_id", "gear_category", "NO ACTION"),
        ("gear_component", "gear_item_id", "gear_item", "CASCADE"),
        (
            "wishlist_replacement",
            "wishlist_item_id",
            "gear_item",
            "CASCADE",
        ),
        (
            "wishlist_replacement",
            "replaces_item_id",
            "gear_item",
            "CASCADE",
        ),
        (
            "wishlist_loadout",
            "wishlist_item_id",
            "gear_item",
            "CASCADE",
        ),
        ("wishlist_loadout", "loadout_id", "loadout", "CASCADE"),
        ("loadout_item", "loadout_id", "loadout", "CASCADE"),
        ("loadout_item", "gear_item_id", "gear_item", "CASCADE"),
        ("trip", "trail_id", "trail", "NO ACTION"),
        ("trip_day", "trip_id", "trip", "CASCADE"),
        ("trip_gear", "trip_id", "trip", "CASCADE"),
        ("trip_gear", "gear_item_id", "gear_item", "NO ACTION"),
        ("trip_campsite", "trip_id", "trip", "CASCADE"),
        ("trip_gpx", "trip_id", "trip", "CASCADE"),
        ("trip_permit", "trip_id", "trip", "CASCADE"),
        ("trip_meal", "trip_id", "trip", "CASCADE"),
        ("trip_meal", "meal_id", "meal", "NO ACTION"),
        ("trip_meal", "ingredient_id", "ingredient", "NO ACTION"),
        ("trip_consumable", "trip_id", "trip", "CASCADE"),
        ("meal_ingredient", "meal_id", "meal", "CASCADE"),
        (
            "meal_ingredient",
            "ingredient_id",
            "ingredient",
            "NO ACTION",
        ),
    ];
    for (table, column, target, delete_rule) in expected {
        let actual: (String, String, String) = db.query_row(
            &format!("SELECT [table], [to], on_delete FROM pragma_foreign_key_list('{table}') WHERE [from] = ?"),
            [column], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).expect("every server foreign key exists");
        assert_eq!(
            actual,
            (target.into(), "sync_id".into(), delete_rule.into()),
            "{table}.{column}"
        );
    }
}

#[test]
fn singleton_and_enum_checks_reject_invalid_rows() {
    let file = WorkspaceFile::new();
    drop(Store::open(&file.0).unwrap());
    let db = Connection::open(&file.0).unwrap();
    for table in ["energy_params", "report_settings"] {
        db.execute(&format!("INSERT INTO {table}(sync_id) VALUES ('one')"), [])
            .unwrap();
        assert!(
            db.execute(&format!("INSERT INTO {table}(sync_id) VALUES ('two')"), [])
                .is_err()
        );
    }
    for sql in [
        "INSERT INTO gear_item(sync_id,name,status) VALUES ('bad','Bad','broken')",
        "INSERT INTO gear_item(sync_id,name,default_weight_class) VALUES ('bad','Bad','packed')",
        "INSERT INTO gear_item(sync_id,name,is_kit) VALUES ('bad','Bad',2)",
        "INSERT INTO trip(sync_id,name,status) VALUES ('bad','Bad','future')",
        "INSERT INTO trip(sync_id,name,access) VALUES ('bad','Bad','admin')",
        "INSERT INTO meal(sync_id,name,meal_type) VALUES ('bad','Bad','brunch')",
        "INSERT INTO meal(sync_id,name,is_hot) VALUES ('bad','Bad',2)",
        "INSERT INTO loadout(sync_id,name,is_default) VALUES ('bad','Bad',2)",
        "INSERT INTO blob_ref(sha256,state) VALUES ('bad','wanted')",
        "INSERT INTO outbox(batch_id,tbl,sync_id,op,columns,created_at) VALUES ('b','gear_item','x','insert','[]','2026-01-01T00:00:00Z')",
    ] {
        assert!(db.execute(sql, []).is_err(), "{sql}");
    }
}
