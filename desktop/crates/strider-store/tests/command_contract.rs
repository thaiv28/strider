//! Public-API command contracts. Compare every persisted synced row, not merely
//! the rows a query happens to render, through apply/undo/redo.
use rusqlite::{Connection, types::Value};
use std::collections::{BTreeMap, BTreeSet};
use strider_core::{Cents, Grams, units::Dec};
use strider_store::{
    Command, ComponentInput, GearInput, IngredientInput, MealPatch, Store, TABLES,
};

type Row = BTreeMap<String, Value>;
type Snapshot = BTreeMap<String, BTreeMap<String, Row>>;
fn snapshot(db: &Connection) -> Snapshot {
    TABLES
        .iter()
        .map(|table| {
            let mut statement = db
                .prepare(&format!("SELECT * FROM {table} ORDER BY sync_id"))
                .unwrap();
            let names: Vec<_> = statement.column_names().map(|s| s.to_string()).collect();
            let rows = statement
                .query_map([], |row| {
                    let mut values = Row::new();
                    for (i, name) in names.iter().enumerate() {
                        values.insert(name.clone(), row.get(i)?);
                    }
                    let Value::Text(id) = &values["sync_id"] else {
                        panic!("sync_id must be text")
                    };
                    Ok((id.clone(), values))
                })
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            (table.to_string(), rows)
        })
        .collect()
}
fn query_output(store: &Store) -> String {
    format!(
        "{:?}",
        (
            store.gear_view().unwrap(),
            store.loadout_data().unwrap(),
            store.ingredients().unwrap(),
            store.meals().unwrap(),
            store.totals().unwrap()
        )
    )
}
fn cursor(store: &Store) -> i64 {
    store
        .connection()
        .query_row("SELECT coalesce(max(seq),0) FROM outbox", [], |r| r.get(0))
        .unwrap()
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    table: String,
    id: String,
    op: String,
    columns: BTreeSet<String>,
}
fn expected_entries(
    before: &Snapshot,
    after: &Snapshot,
    derived: &[(&str, &str)],
) -> BTreeSet<Entry> {
    let mut entries = BTreeSet::new();
    for table in TABLES {
        let a = &before[*table];
        let b = &after[*table];
        let ids: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for id in ids {
            if a.get(id) == b.get(id) {
                continue;
            }
            let (op, columns) = if let Some(row) = b.get(id) {
                let columns = row
                    .iter()
                    .filter(|(column, value)| {
                        a.get(id).is_none_or(|old| old.get(*column) != Some(*value))
                            && (a.get(id).is_none()
                                || !derived.contains(&(*table, column.as_str())))
                    })
                    .map(|(column, _)| column.clone())
                    .collect::<BTreeSet<_>>();
                if columns.is_empty() {
                    continue;
                }
                ("upsert", columns)
            } else {
                ("delete", BTreeSet::new())
            };
            entries.insert(Entry {
                table: table.to_string(),
                id: id.clone(),
                op: op.into(),
                columns,
            });
        }
    }
    entries
}
fn assert_outbox(store: &Store, since: i64, expected: BTreeSet<Entry>) -> Option<String> {
    let mut statement = store
        .connection()
        .prepare("SELECT batch_id,tbl,sync_id,op,columns FROM outbox WHERE seq>? ORDER BY seq")
        .unwrap();
    let rows = statement
        .query_map([since], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let batches: BTreeSet<_> = rows.iter().map(|r| r.0.clone()).collect();
    assert_eq!(
        batches.len(),
        usize::from(!expected.is_empty()),
        "one batch per command"
    );
    let mut actual = BTreeSet::new();
    for (_, table, id, op, json) in &rows {
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "columns must be a JSON array"
        );
        let columns: BTreeSet<_> = json[1..json.len() - 1]
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let s = s.trim();
                assert!(s.starts_with('"') && s.ends_with('"'));
                let value = &s[1..s.len() - 1];
                assert!(
                    value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                );
                value.to_string()
            })
            .collect();
        assert!(
            actual.insert(Entry {
                table: table.clone(),
                id: id.clone(),
                op: op.clone(),
                columns
            }),
            "duplicate outbox row"
        );
    }
    assert_eq!(actual, expected);
    assert_eq!(rows.len(), actual.len());
    batches.into_iter().next()
}
fn check(store: &mut Store, cmd: Command, direct_tables: &[&str], derived: &[(&str, &str)]) {
    let before = snapshot(store.connection());
    let before_query = query_output(store);
    let seq = cursor(store);
    let changes = store.apply(cmd).unwrap();
    let after = snapshot(store.connection());
    let after_query = query_output(store);
    let expected = expected_entries(&before, &after, derived);
    assert_eq!(
        expected
            .iter()
            .map(|e| e.table.as_str())
            .collect::<BTreeSet<_>>(),
        direct_tables.iter().copied().collect()
    );
    let batch = assert_outbox(store, seq, expected);
    let expected_changed: BTreeSet<_> = TABLES
        .iter()
        .filter(|t| before[**t] != after[**t])
        .map(|t| t.to_string())
        .collect();
    assert_eq!(changes.tables, expected_changed);
    let seq = cursor(store);
    assert!(store.undo().unwrap());
    assert_eq!(
        snapshot(store.connection()),
        before,
        "undo must restore all rows including cascades and snapshots"
    );
    assert_eq!(query_output(store), before_query);
    let undo_batch = assert_outbox(store, seq, expected_entries(&after, &before, &[]));
    if batch.is_some() && undo_batch.is_some() {
        assert_ne!(batch, undo_batch);
    }
    let seq = cursor(store);
    assert!(store.redo().unwrap());
    assert_eq!(
        snapshot(store.connection()),
        after,
        "redo must preserve exact identities, values and timestamps"
    );
    assert_eq!(query_output(store), after_query);
    let redo_batch = assert_outbox(store, seq, expected_entries(&before, &after, &[]));
    if redo_batch.is_some() && undo_batch.is_some() {
        assert_ne!(redo_batch, undo_batch);
    }
}
fn id(store: &Store, table: &str, name: &str) -> String {
    store
        .connection()
        .query_row(
            &format!("SELECT sync_id FROM {table} WHERE name=?"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
}
fn integer(store: &Store, table: &str, id: &str, column: &str) -> i64 {
    store
        .connection()
        .query_row(
            &format!("SELECT {column} FROM {table} WHERE sync_id=?"),
            [id],
            |r| r.get(0),
        )
        .unwrap()
}
fn text(store: &Store, table: &str, id: &str, column: &str) -> Option<String> {
    store
        .connection()
        .query_row(
            &format!("SELECT {column} FROM {table} WHERE sync_id=?"),
            [id],
            |r| r.get(0),
        )
        .unwrap()
}
fn gear(name: &str, category: Option<String>) -> GearInput {
    GearInput {
        name: name.into(),
        category_id: category,
        weight_g: Some(Grams(100)),
        quantity: 1,
        class: "base".into(),
        status: "current".into(),
        ..Default::default()
    }
}
fn d(s: &str) -> Dec {
    s.parse().unwrap()
}
fn ingredient(name: &str) -> IngredientInput {
    IngredientInput {
        name: name.into(),
        kcal_per_100g: Some(d("350")),
        density_g_ml: Some(d("0.5")),
        ..Default::default()
    }
}

#[test]
fn every_gear_category_loadout_and_wishlist_command_roundtrips() {
    let mut store = Store::in_memory().unwrap();
    check(
        &mut store,
        Command::CreateCategory {
            name: "  Shelter  ".into(),
        },
        &["gear_category"],
        &[],
    );
    let shelter = id(&store, "gear_category", "Shelter");
    check(
        &mut store,
        Command::CreateCategory {
            name: "Clothing".into(),
        },
        &["gear_category"],
        &[],
    );
    let clothing = id(&store, "gear_category", "Clothing");
    check(
        &mut store,
        Command::RenameCategory {
            id: shelter.clone(),
            name: "  Sleep  ".into(),
        },
        &["gear_category"],
        &[],
    );
    assert_eq!(
        text(&store, "gear_category", &shelter, "name").as_deref(),
        Some("Sleep")
    );
    check(
        &mut store,
        Command::ReorderCategories {
            ordered_ids: vec![clothing.clone(), shelter.clone()],
        },
        &["gear_category"],
        &[],
    );
    assert_eq!(integer(&store, "gear_category", &clothing, "sort_order"), 0);
    let mut tent = gear("  Tent  ", Some(shelter.clone()));
    tent.weight_oz = Some(1.75);
    tent.price_cents = Some(Cents(-5));
    tent.quantity = 0;
    tent.notes = Some("  notes  ".into());
    check(
        &mut store,
        Command::CreateGear { input: tent },
        &["gear_item"],
        &[],
    );
    let tent = id(&store, "gear_item", "Tent");
    assert_eq!(integer(&store, "gear_item", &tent, "weight_g"), 50);
    assert_eq!(integer(&store, "gear_item", &tent, "price_cents"), 0);
    assert_eq!(integer(&store, "gear_item", &tent, "quantity"), 1);
    assert_eq!(
        text(&store, "gear_item", &tent, "notes").as_deref(),
        Some("notes")
    );
    check(
        &mut store,
        Command::CreateGear {
            input: gear("Jacket", Some(clothing.clone())),
        },
        &["gear_item"],
        &[],
    );
    let jacket = id(&store, "gear_item", "Jacket");
    let mut kit = gear("Sleep kit", Some(shelter.clone()));
    kit.is_kit = true;
    kit.components = vec![
        ComponentInput {
            name: "  Bag  ".into(),
            weight_g: 25,
            quantity: 2,
            notes: None,
        },
        ComponentInput {
            name: "Pad".into(),
            weight_g: -1,
            quantity: 0,
            notes: None,
        },
        ComponentInput {
            name: " ".into(),
            weight_g: 1000,
            quantity: 5,
            notes: None,
        },
    ];
    check(
        &mut store,
        Command::CreateGear { input: kit.clone() },
        &["gear_item", "gear_component"],
        &[],
    );
    let kit_id = id(&store, "gear_item", "Sleep kit");
    assert_eq!(integer(&store, "gear_item", &kit_id, "weight_g"), 50);
    assert_eq!(
        store
            .gear_view()
            .unwrap()
            .groups
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.id == kit_id)
            .unwrap()
            .components
            .len(),
        2
    );
    kit.name = "Sleep kit upgraded".into();
    kit.components[0].weight_g = 30;
    check(
        &mut store,
        Command::UpdateGear {
            id: kit_id.clone(),
            input: kit,
        },
        &["gear_item", "gear_component"],
        &[],
    );
    assert_eq!(integer(&store, "gear_item", &kit_id, "weight_g"), 60);
    check(
        &mut store,
        Command::MoveGear {
            item: tent.clone(),
            to_category: Some(clothing.clone()),
            ordered_ids: vec![tent.clone(), jacket.clone()],
        },
        &["gear_item"],
        &[],
    );
    assert_eq!(integer(&store, "gear_item", &tent, "sort_order"), 0);
    assert_eq!(integer(&store, "gear_item", &jacket, "sort_order"), 1);
    check(
        &mut store,
        Command::CreateLoadout { name: "  ".into() },
        &["loadout"],
        &[],
    );
    let loadout = id(&store, "loadout", "New Loadout");
    assert_eq!(integer(&store, "loadout", &loadout, "sort_order"), -1);
    check(
        &mut store,
        Command::CreateLoadout {
            name: "Summer".into(),
        },
        &["loadout"],
        &[],
    );
    let summer = id(&store, "loadout", "Summer");
    assert_eq!(integer(&store, "loadout", &summer, "sort_order"), -2);
    check(
        &mut store,
        Command::RenameLoadout {
            id: loadout.clone(),
            name: "  Winter  ".into(),
        },
        &["loadout"],
        &[],
    );
    check(
        &mut store,
        Command::ReorderLoadouts {
            ordered_ids: vec![loadout.clone(), summer.clone()],
        },
        &["loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetDefaultLoadout {
            id: loadout.clone(),
        },
        &["loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetDefaultLoadout { id: summer.clone() },
        &["loadout"],
        &[],
    );
    assert_eq!(integer(&store, "loadout", &loadout, "is_default"), 0);
    check(
        &mut store,
        Command::SetLoadoutMember {
            loadout: summer.clone(),
            item: jacket.clone(),
            present: true,
            class: "worn".into(),
        },
        &["loadout_item"],
        &[],
    );
    assert_eq!(
        store.loadout_data().unwrap().membership[&summer][&jacket],
        "worn"
    );
    check(
        &mut store,
        Command::SetLoadoutMember {
            loadout: summer.clone(),
            item: jacket.clone(),
            present: true,
            class: "base".into(),
        },
        &["loadout_item"],
        &[],
    );
    check(
        &mut store,
        Command::SetLoadoutMember {
            loadout: summer.clone(),
            item: jacket.clone(),
            present: false,
            class: "base".into(),
        },
        &["loadout_item"],
        &[],
    );
    check(
        &mut store,
        Command::SetAllLoadoutMembers {
            loadout: summer.clone(),
            present: true,
        },
        &["loadout_item"],
        &[],
    );
    assert_eq!(store.loadout_data().unwrap().membership[&summer].len(), 3);
    check(
        &mut store,
        Command::SetAllLoadoutMembers {
            loadout: summer.clone(),
            present: false,
        },
        &["loadout_item"],
        &[],
    );
    let mut wish = gear("New tent", Some(shelter.clone()));
    wish.status = "wishlist".into();
    check(
        &mut store,
        Command::CreateGear { input: wish },
        &["gear_item"],
        &[],
    );
    let wish = id(&store, "gear_item", "New tent");
    check(
        &mut store,
        Command::SetWishlistReplacements {
            item: wish.clone(),
            replaces: vec![tent.clone(), tent.clone(), wish.clone()],
        },
        &["wishlist_replacement"],
        &[],
    );
    assert_eq!(
        store.gear_view().unwrap().wishlist[0].replaces,
        vec![tent.clone()]
    );
    check(
        &mut store,
        Command::SetWishlistLoadout {
            item: wish.clone(),
            loadout: summer.clone(),
            included: Some(true),
        },
        &["wishlist_loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetWishlistLoadout {
            item: wish.clone(),
            loadout: summer.clone(),
            included: Some(false),
        },
        &["wishlist_loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetWishlistLoadout {
            item: wish.clone(),
            loadout: summer.clone(),
            included: None,
        },
        &["wishlist_loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetWishlistLoadout {
            item: wish,
            loadout: summer.clone(),
            included: Some(true),
        },
        &["wishlist_loadout"],
        &[],
    );
    check(
        &mut store,
        Command::SetAllLoadoutMembers {
            loadout: summer.clone(),
            present: true,
        },
        &["loadout_item"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteLoadout { id: summer },
        &["loadout", "loadout_item", "wishlist_loadout"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteCategory { id: clothing },
        &["gear_category", "gear_item"],
        &[],
    );
    assert_eq!(text(&store, "gear_item", &tent, "category_id"), None);
    assert!(
        store
            .gear_view()
            .unwrap()
            .groups
            .iter()
            .any(|g| g.name == "Other")
    );
    check(
        &mut store,
        Command::DeleteGear { id: tent },
        &["gear_item", "wishlist_replacement"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteGear { id: kit_id },
        &["gear_item", "gear_component"],
        &[],
    );
}

#[test]
fn every_food_command_roundtrips_and_patch_fields_are_preserved() {
    let mut store = Store::in_memory().unwrap();
    check(
        &mut store,
        Command::CreateIngredient {
            input: ingredient("  Rice  "),
        },
        &["ingredient"],
        &[],
    );
    let rice = id(&store, "ingredient", "Rice");
    assert_eq!(
        text(&store, "ingredient", &rice, "source").as_deref(),
        Some("manual")
    );
    let mut rice_input = ingredient("  Rice flour  ");
    rice_input.kcal_per_100g = Some(d("350.5"));
    rice_input.default_serving_g = Some(20);
    rice_input.notes = Some("label notes".into());
    check(
        &mut store,
        Command::UpdateIngredient {
            id: rice.clone(),
            input: rice_input,
        },
        &["ingredient"],
        &[],
    );
    assert_eq!(
        text(&store, "ingredient", &rice, "kcal_per_100g").as_deref(),
        Some("351")
    );
    check(
        &mut store,
        Command::SetIngredientMeasure {
            id: rice.clone(),
            unit: "cup".into(),
            grams: d("118.294"),
        },
        &["ingredient"],
        &[],
    );
    assert_eq!(
        text(&store, "ingredient", &rice, "density_g_ml").as_deref(),
        Some("0.5")
    );
    assert_eq!(
        text(&store, "ingredient", &rice, "density_source").as_deref(),
        Some("manual")
    );
    check(
        &mut store,
        Command::LinkIngredientReference {
            id: rice.clone(),
            density: d("0.65"),
            reference_name: Some("USDA rice flour".into()),
        },
        &["ingredient"],
        &[],
    );
    assert_eq!(
        text(&store, "ingredient", &rice, "density_source").as_deref(),
        Some("usda")
    );
    check(
        &mut store,
        Command::CreateIngredient {
            input: ingredient("Beans"),
        },
        &["ingredient"],
        &[],
    );
    let beans = id(&store, "ingredient", "Beans");
    check(
        &mut store,
        Command::CreateMeal { name: "  ".into() },
        &["meal"],
        &[],
    );
    let meal = id(&store, "meal", "New meal");
    check(
        &mut store,
        Command::UpdateMeal {
            id: meal.clone(),
            patch: MealPatch {
                name: Some("  Dinner  ".into()),
                base_servings: Some(0),
                meal_type: Some(Some("dinner".into())),
                is_hot: Some(false),
                water_ml: Some(Some(400)),
                notes: Some(Some("Keep this".into())),
            },
        },
        &["meal"],
        &[],
    );
    assert_eq!(integer(&store, "meal", &meal, "base_servings"), 1);
    check(
        &mut store,
        Command::UpdateMeal {
            id: meal.clone(),
            patch: MealPatch {
                name: Some(" ".into()),
                ..Default::default()
            },
        },
        &["meal"],
        &[],
    );
    assert_eq!(
        text(&store, "meal", &meal, "name").as_deref(),
        Some("Untitled")
    );
    assert_eq!(
        text(&store, "meal", &meal, "notes").as_deref(),
        Some("Keep this")
    );
    assert_eq!(integer(&store, "meal", &meal, "water_ml"), 400);
    check(
        &mut store,
        Command::UpdateMeal {
            id: meal.clone(),
            patch: MealPatch {
                meal_type: Some(None),
                water_ml: Some(None),
                notes: Some(None),
                ..Default::default()
            },
        },
        &["meal"],
        &[],
    );
    check(
        &mut store,
        Command::AddMealIngredient {
            meal: meal.clone(),
            ingredient: rice.clone(),
            amount_g: 100,
        },
        &["meal_ingredient"],
        &[],
    );
    let line = store.meals().unwrap()[0].items[0].id.clone();
    assert_eq!(integer(&store, "meal_ingredient", &line, "amount_g"), 100);
    assert_eq!(
        text(&store, "meal_ingredient", &line, "snapshot_name").as_deref(),
        Some("Rice flour")
    );
    check(
        &mut store,
        Command::AddMealIngredient {
            meal: meal.clone(),
            ingredient: beans.clone(),
            amount_g: -4,
        },
        &["meal_ingredient"],
        &[],
    );
    let second = store.meals().unwrap()[0].items[1].id.clone();
    assert_eq!(integer(&store, "meal_ingredient", &second, "amount_g"), 0);
    check(
        &mut store,
        Command::ReorderMealIngredients {
            meal: meal.clone(),
            ordered_ids: vec![second.clone(), line.clone()],
        },
        &["meal_ingredient"],
        &[],
    );
    assert_eq!(store.meals().unwrap()[0].items[0].id, second);
    check(
        &mut store,
        Command::UpdateMealIngredientAmount {
            id: line.clone(),
            amount_g: 60,
            entered: Some((d("0.5"), "cup".into())),
        },
        &["meal_ingredient"],
        &[],
    );
    assert_eq!(
        text(&store, "meal_ingredient", &line, "entered_amount").as_deref(),
        Some("0.5")
    );
    assert_eq!(
        text(&store, "meal_ingredient", &line, "entered_unit").as_deref(),
        Some("cup")
    );
    assert_eq!(store.meals().unwrap()[0].total_g, 60);
    assert_eq!(store.meals().unwrap()[0].total_kcal, 211);
    check(
        &mut store,
        Command::UpdateMealIngredientAmount {
            id: line.clone(),
            amount_g: -2,
            entered: Some((d("2"), "invalid".into())),
        },
        &["meal_ingredient"],
        &[],
    );
    assert_eq!(integer(&store, "meal_ingredient", &line, "amount_g"), 0);
    assert_eq!(text(&store, "meal_ingredient", &line, "entered_unit"), None);
    check(
        &mut store,
        Command::RemoveMealIngredient { id: second },
        &["meal_ingredient"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteMeal { id: meal },
        &["meal", "meal_ingredient"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteIngredient { id: beans },
        &["ingredient"],
        &[],
    );
    check(
        &mut store,
        Command::DeleteIngredient { id: rice },
        &["ingredient"],
        &[],
    );
}

const DERIVED_FOOD: &[(&str, &str)] = &[
    ("meal_ingredient", "snapshot_name"),
    ("meal_ingredient", "snapshot_kcal_per_100g"),
    ("meal_ingredient", "snapshot_density_g_ml"),
    ("meal_ingredient", "amount_g"),
    ("trip_meal", "snapshot_name"),
    ("trip_meal", "snapshot_kcal"),
    ("trip_meal", "snapshot_weight_g"),
    ("trip", "food_g_per_day"),
];
#[test]
fn ingredient_recipe_trip_propagation_is_atomic_and_derived_columns_stay_out_of_outbox() {
    let mut store = Store::in_memory().unwrap();
    let ingredient_input = ingredient("Rice");
    store
        .apply(Command::CreateIngredient {
            input: ingredient_input.clone(),
        })
        .unwrap();
    let ingredient_id = id(&store, "ingredient", "Rice");
    store
        .apply(Command::CreateMeal {
            name: "Dinner".into(),
        })
        .unwrap();
    let meal = id(&store, "meal", "Dinner");
    store
        .apply(Command::AddMealIngredient {
            meal: meal.clone(),
            ingredient: ingredient_id.clone(),
            amount_g: 10,
        })
        .unwrap();
    let line = store.meals().unwrap()[0].items[0].id.clone();
    store.connection().execute_batch("INSERT INTO trip(sync_id,name,nights,food_g_per_day,status) VALUES('trip','Completed',1,777,'completed');").unwrap();
    store
        .connection()
        .execute(
            "UPDATE meal_ingredient SET entered_amount='1.5',entered_unit='ml' WHERE sync_id=?",
            [&line],
        )
        .unwrap();
    store.connection().execute("INSERT INTO trip_meal(sync_id,trip_id,day_number,meal_id,servings,snapshot_name,snapshot_kcal,snapshot_weight_g) VALUES('recipe-trip','trip',1,?,'0.5','Old Dinner',99,999)", [&meal]).unwrap();
    store.connection().execute("INSERT INTO trip_meal(sync_id,trip_id,day_number,ingredient_id,snapshot_name,snapshot_kcal,snapshot_weight_g) VALUES('ingredient-trip','trip',2,?,'Old Rice',88,100)", [&ingredient_id]).unwrap();
    let mut changed = ingredient_input.clone();
    changed.name = "Renamed rice".into();
    changed.density_g_ml = Some(Dec::ONE);
    changed.kcal_per_100g = Some(d("100"));
    check(
        &mut store,
        Command::UpdateIngredient {
            id: ingredient_id.clone(),
            input: changed.clone(),
        },
        &["ingredient"],
        DERIVED_FOOD,
    );
    assert_eq!(
        integer(&store, "meal_ingredient", &line, "amount_g"),
        2,
        "1.5 ml times 1 g/ml rounds half away"
    );
    assert_eq!(
        integer(&store, "trip_meal", "recipe-trip", "snapshot_weight_g"),
        1
    );
    assert_eq!(
        integer(&store, "trip_meal", "recipe-trip", "snapshot_kcal"),
        1
    );
    assert_eq!(
        integer(&store, "trip_meal", "ingredient-trip", "snapshot_kcal"),
        100
    );
    assert_eq!(
        integer(&store, "trip", "trip", "food_g_per_day"),
        51,
        "101 grams / 2 days rounds half away"
    );
    changed.kcal_per_100g = None;
    changed.density_g_ml = None;
    check(
        &mut store,
        Command::UpdateIngredient {
            id: ingredient_id.clone(),
            input: changed,
        },
        &["ingredient"],
        DERIVED_FOOD,
    );
    assert_eq!(
        text(&store, "meal_ingredient", &line, "snapshot_density_g_ml"),
        None
    );
    assert_eq!(
        text(&store, "meal_ingredient", &line, "snapshot_kcal_per_100g").as_deref(),
        Some("100")
    );
    assert_eq!(integer(&store, "meal_ingredient", &line, "amount_g"), 2);
    assert_eq!(
        integer(&store, "trip_meal", "ingredient-trip", "snapshot_kcal"),
        100
    );
    check(
        &mut store,
        Command::LinkIngredientReference {
            id: ingredient_id.clone(),
            density: d("2"),
            reference_name: Some("Reference".into()),
        },
        &["ingredient"],
        DERIVED_FOOD,
    );
    assert_eq!(integer(&store, "meal_ingredient", &line, "amount_g"), 3);
    check(
        &mut store,
        Command::SetIngredientMeasure {
            id: ingredient_id,
            unit: "ml".into(),
            grams: d("3"),
        },
        &["ingredient"],
        DERIVED_FOOD,
    );
    assert_eq!(integer(&store, "meal_ingredient", &line, "amount_g"), 5);
    check(
        &mut store,
        Command::UpdateMeal {
            id: meal.clone(),
            patch: MealPatch {
                name: Some("New dinner".into()),
                base_servings: Some(2),
                ..Default::default()
            },
        },
        &["meal"],
        DERIVED_FOOD,
    );
    assert_eq!(
        text(&store, "trip_meal", "recipe-trip", "snapshot_name").as_deref(),
        Some("New dinner")
    );
    assert_eq!(
        integer(&store, "trip_meal", "recipe-trip", "snapshot_weight_g"),
        1
    );
    // A direct line edit must be synced; only trip snapshots and food totals are derived here.
    let trip_only = &DERIVED_FOOD[4..];
    check(
        &mut store,
        Command::UpdateMealIngredientAmount {
            id: line.clone(),
            amount_g: 10,
            entered: None,
        },
        &["meal_ingredient"],
        trip_only,
    );
    assert_eq!(
        integer(&store, "trip_meal", "recipe-trip", "snapshot_weight_g"),
        3
    );
    check(
        &mut store,
        Command::RemoveMealIngredient { id: line },
        &["meal_ingredient"],
        trip_only,
    );
    assert_eq!(
        integer(&store, "trip_meal", "recipe-trip", "snapshot_weight_g"),
        3,
        "last line deletion preserves old SQL inner-join totals"
    );
}

#[test]
fn gear_propagates_to_completed_trip_preserves_unknown_weight_and_undo_restores_snapshots() {
    let mut store = Store::in_memory().unwrap();
    store
        .apply(Command::CreateGear {
            input: gear("Tent", None),
        })
        .unwrap();
    let item = id(&store, "gear_item", "Tent");
    store
        .connection()
        .execute_batch(
            "INSERT INTO trip(sync_id,name,status) VALUES('trip','Completed','completed');",
        )
        .unwrap();
    store.connection().execute("INSERT INTO trip_gear(sync_id,trip_id,gear_item_id,snapshot_name,snapshot_weight_g,quantity,weight_class,packed,snapshot_category) VALUES('tg','trip',?,'Old',20,3,'worn',1,'Original')", [&item]).unwrap();
    let derived = &[
        ("trip_gear", "snapshot_name"),
        ("trip_gear", "snapshot_weight_g"),
    ];
    let mut updated = gear("New tent", None);
    updated.weight_g = Some(Grams(200));
    check(
        &mut store,
        Command::UpdateGear {
            id: item.clone(),
            input: updated.clone(),
        },
        &["gear_item"],
        derived,
    );
    assert_eq!(integer(&store, "trip_gear", "tg", "snapshot_weight_g"), 200);
    assert_eq!(integer(&store, "trip_gear", "tg", "quantity"), 3);
    assert_eq!(integer(&store, "trip_gear", "tg", "packed"), 1);
    assert_eq!(
        text(&store, "trip_gear", "tg", "snapshot_category").as_deref(),
        Some("Original")
    );
    updated.name = "Weight unknown".into();
    updated.weight_g = None;
    check(
        &mut store,
        Command::UpdateGear {
            id: item,
            input: updated,
        },
        &["gear_item"],
        derived,
    );
    assert_eq!(integer(&store, "trip_gear", "tg", "snapshot_weight_g"), 200);
    assert_eq!(
        text(&store, "trip_gear", "tg", "snapshot_name").as_deref(),
        Some("Weight unknown")
    );
}

#[test]
fn rejected_commands_are_atomic_and_new_command_clears_redo() {
    let mut store = Store::in_memory().unwrap();
    store
        .apply(Command::CreateCategory { name: "One".into() })
        .unwrap();
    let category = id(&store, "gear_category", "One");
    let invalid_commands = [
        Command::CreateGear {
            input: gear("  ", None),
        },
        Command::CreateGear {
            input: gear("Bad category", Some("missing".into())),
        },
        Command::CreateIngredient {
            input: ingredient(" "),
        },
        Command::ReorderCategories {
            ordered_ids: vec![category.clone(), category.clone()],
        },
        Command::SetDefaultLoadout {
            id: "missing".into(),
        },
        Command::DeleteIngredient {
            id: "missing".into(),
        },
    ];
    for command in invalid_commands {
        let before = snapshot(store.connection());
        let seq = cursor(&store);
        assert!(store.apply(command).is_err());
        assert_eq!(snapshot(store.connection()), before);
        assert_eq!(cursor(&store), seq);
    }
    store
        .apply(Command::CreateIngredient {
            input: ingredient("Rice"),
        })
        .unwrap();
    let rice = id(&store, "ingredient", "Rice");
    let before = snapshot(store.connection());
    let seq = cursor(&store);
    assert!(
        store
            .apply(Command::SetIngredientMeasure {
                id: rice,
                unit: "g".into(),
                grams: Dec::ONE
            })
            .is_err()
    );
    assert_eq!(snapshot(store.connection()), before);
    assert_eq!(cursor(&store), seq);
    assert!(store.undo().unwrap());
    store
        .apply(Command::RenameCategory {
            id: category,
            name: "Renamed".into(),
        })
        .unwrap();
    assert!(!store.redo().unwrap());
}

#[test]
fn trip_reference_blocks_gear_delete_without_touching_rows_or_history() {
    let mut store = Store::in_memory().unwrap();
    store
        .apply(Command::CreateGear {
            input: gear("Tent", None),
        })
        .unwrap();
    let item = id(&store, "gear_item", "Tent");
    store
        .connection()
        .execute_batch("INSERT INTO trip(sync_id,name) VALUES('trip','Weekend');")
        .unwrap();
    store.connection().execute(
        "INSERT INTO trip_gear(sync_id,trip_id,gear_item_id,snapshot_name,snapshot_weight_g) VALUES('tg','trip',?,'Tent',100)",
        [&item],
    ).unwrap();
    let before = snapshot(store.connection());
    let seq = cursor(&store);
    let error = store
        .apply(Command::DeleteGear { id: item.clone() })
        .unwrap_err();
    assert!(error.to_string().contains("gear used by a trip"));
    assert_eq!(snapshot(store.connection()), before);
    assert_eq!(cursor(&store), seq);
    // A failed deletion must leave the previous successful command undoable.
    store
        .connection()
        .execute("DELETE FROM trip_gear WHERE sync_id='tg'", [])
        .unwrap();
    assert!(store.undo().unwrap());
    assert!(store.gear_view().unwrap().groups.is_empty());
}
