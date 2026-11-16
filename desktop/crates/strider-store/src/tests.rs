use super::*;
fn fixture() -> Store {
    let mut s = Store::in_memory().unwrap();
    seed_fixture(&mut s, FixtureSize::Small).unwrap();
    s
}
fn roundtrip(s: &mut Store, cmd: Command) {
    let before = snapshot(&s.connection).unwrap();
    let start: i64 = s
        .connection
        .query_row("SELECT coalesce(max(seq),0) FROM outbox", [], |r| r.get(0))
        .unwrap();
    let changes = s.apply(cmd).unwrap();
    let after = snapshot(&s.connection).unwrap();
    assert_eq!(changes, super::changes(&before, &after));
    assert_ne!(before, after);
    let batches: i64 = s
        .connection
        .query_row(
            "SELECT count(DISTINCT batch_id) FROM outbox WHERE seq>?",
            [start],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(batches, 1);
    let invalid:i64=s.connection.query_row("SELECT count(*) FROM outbox WHERE seq>? AND (NOT json_valid(columns) OR op NOT IN ('upsert','delete'))",[start],|r|r.get(0)).unwrap();
    assert_eq!(invalid, 0);
    for table in TABLES {
        let old = &before[*table];
        let new = &after[*table];
        for (key, row) in new {
            if !old.contains_key(key) {
                let cols:i64=s.connection.query_row("SELECT json_array_length(columns) FROM outbox WHERE seq>? AND tbl=? AND sync_id=?",params![start,table,key],|r|r.get(0)).unwrap();
                assert_eq!(cols, row.len() as i64);
            }
        }
    }
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s.connection).unwrap(), before);
    assert!(s.redo().unwrap());
    assert_eq!(snapshot(&s.connection).unwrap(), after);
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s.connection).unwrap(), before);
}
#[test]
fn fixture_deterministic() {
    let a = fixture();
    let b = fixture();
    assert_eq!(
        snapshot(&a.connection).unwrap(),
        snapshot(&b.connection).unwrap()
    );
    assert_eq!(a.ingredients().unwrap().len(), 80);
    assert_eq!(a.meals().unwrap().len(), 30);
    assert_eq!(a.loadout_data().unwrap().loadouts.len(), 4);
    assert_eq!(
        a.connection
            .query_row("SELECT count(*) FROM outbox", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    for table in TABLES {
        for r in rows(&a.connection, table).unwrap() {
            assert_eq!(
                Uuid::parse_str(s(&r, "sync_id")).unwrap().get_version_num(),
                7
            );
        }
    }
}
#[test]
fn every_command_rows_outbox_undo_redo() {
    let mut s = fixture();
    let gear = rows(&s.connection, "gear_item").unwrap();
    let item = gear.iter().find(|r| s::current(r)).unwrap();
    let item_id = super::s(item, "sync_id").to_string();
    let wish = gear
        .iter()
        .find(|r| super::s(r, "status") == "wishlist")
        .unwrap();
    let wish_id = super::s(wish, "sync_id").to_string();
    let categories = rows(&s.connection, "gear_category").unwrap();
    let cat_id = super::s(&categories[0], "sync_id").to_string();
    let loadouts = rows(&s.connection, "loadout").unwrap();
    let l_id = super::s(&loadouts[0], "sync_id").to_string();
    let ing = s.ingredients().unwrap()[0].clone();
    let meal = s.meals().unwrap()[0].clone();
    let line = meal.items[0].id.clone();
    let gear_input = GearInput {
        name: "  New kit  ".into(),
        category_id: Some(cat_id.clone()),
        weight_g: Some(Grams(999)),
        quantity: 2,
        is_kit: true,
        components: vec![ComponentInput {
            name: " Component ".into(),
            weight_g: 10,
            quantity: 3,
            notes: Some(" note ".into()),
        }],
        ..Default::default()
    };
    let commands = vec![
        Command::CreateGear {
            input: gear_input.clone(),
        },
        Command::UpdateGear {
            id: item_id.clone(),
            input: gear_input,
        },
        Command::DeleteGear {
            id: item_id.clone(),
        },
        Command::MoveGear {
            item: item_id.clone(),
            to_category: None,
            ordered_ids: vec![item_id.clone()],
        },
        Command::SetWishlistReplacements {
            item: wish_id.clone(),
            replaces: vec![item_id.clone()],
        },
        Command::SetWishlistLoadout {
            item: wish_id.clone(),
            loadout: l_id.clone(),
            included: Some(false),
        },
        Command::CreateLoadout { name: " ".into() },
        Command::RenameLoadout {
            id: l_id.clone(),
            name: " ".into(),
        },
        Command::ReorderLoadouts {
            ordered_ids: loadouts
                .iter()
                .rev()
                .map(|r| super::s(r, "sync_id").into())
                .collect(),
        },
        Command::DeleteLoadout { id: l_id.clone() },
        Command::SetDefaultLoadout {
            id: super::s(&loadouts[1], "sync_id").into(),
        },
        Command::SetLoadoutMember {
            loadout: l_id.clone(),
            item: item_id.clone(),
            present: true,
            class: "worn".into(),
        },
        Command::SetAllLoadoutMembers {
            loadout: l_id.clone(),
            present: false,
        },
        Command::CreateCategory {
            name: " Gear ".into(),
        },
        Command::RenameCategory {
            id: cat_id.clone(),
            name: " Other name ".into(),
        },
        Command::ReorderCategories {
            ordered_ids: categories
                .iter()
                .rev()
                .map(|r| super::s(r, "sync_id").into())
                .collect(),
        },
        Command::DeleteCategory { id: cat_id },
        Command::CreateIngredient {
            input: IngredientInput {
                name: " Oats ".into(),
                kcal_per_100g: Some("50.5".parse().unwrap()),
                ..Default::default()
            },
        },
        Command::UpdateIngredient {
            id: ing.id.clone(),
            input: IngredientInput {
                name: " New oats ".into(),
                density_g_ml: Some("0.5".parse().unwrap()),
                ..Default::default()
            },
        },
        Command::SetIngredientMeasure {
            id: ing.id.clone(),
            unit: "cup".into(),
            grams: Dec::from(120),
        },
        Command::LinkIngredientReference {
            id: ing.id.clone(),
            density: "0.8".parse().unwrap(),
            reference_name: Some("Oats".into()),
        },
        Command::CreateMeal { name: " ".into() },
        Command::UpdateMeal {
            id: meal.id.clone(),
            patch: MealPatch {
                name: Some(" ".into()),
                base_servings: Some(0),
                meal_type: Some(Some("breakfast".into())),
                is_hot: Some(false),
                water_ml: Some(Some(200)),
                notes: Some(Some("hello".into())),
            },
        },
        Command::DeleteMeal {
            id: meal.id.clone(),
        },
        Command::AddMealIngredient {
            meal: meal.id.clone(),
            ingredient: ing.id.clone(),
            amount_g: -2,
        },
        Command::UpdateMealIngredientAmount {
            id: line.clone(),
            amount_g: 101,
            entered: Some((Dec::from(1), "cup".into())),
        },
        Command::RemoveMealIngredient { id: line },
        Command::ReorderMealIngredients {
            meal: meal.id.clone(),
            ordered_ids: meal.items.iter().rev().map(|l| l.id.clone()).collect(),
        },
        Command::UpdateItem {
            id: item_id.clone(),
            name: "Legacy".into(),
            weight: Some(Grams(20)),
            quantity: 2,
        },
        Command::MoveItem {
            id: item_id.clone(),
            to_category: None,
            before: None,
        },
        Command::DeleteItem { id: item_id },
    ];
    for cmd in commands {
        roundtrip(&mut s, cmd);
    }
    // Non-cascading references deliberately reject deletes as PostgreSQL does.
    assert!(s.apply(Command::DeleteIngredient { id: ing.id }).is_err());
    s.apply(Command::CreateIngredient {
        input: IngredientInput {
            name: "Unlinked".into(),
            ..Default::default()
        },
    })
    .unwrap();
    let id = s
        .ingredients()
        .unwrap()
        .into_iter()
        .find(|i| i.input.name == "Unlinked")
        .unwrap()
        .id;
    roundtrip(&mut s, Command::DeleteIngredient { id });
}
mod s {
    use super::*;
    pub fn current(r: &Row) -> bool {
        super::super::s(r, "status") == "current"
    }
}
#[test]
fn propagation_numeric_nulls_derived_outbox_and_undo() {
    let mut s = Store::in_memory().unwrap();
    s.connection.execute_batch("INSERT INTO gear_item(sync_id,name,weight_g) VALUES('g','Gear',10);INSERT INTO trip(sync_id,name,nights) VALUES('t','Trip',1);INSERT INTO trip_gear(sync_id,trip_id,gear_item_id,snapshot_name,snapshot_weight_g) VALUES('tg','t','g','Old',99);INSERT INTO ingredient(sync_id,name,kcal_per_100g,density_g_ml) VALUES('i','Ingredient','50','1');INSERT INTO meal(sync_id,name,base_servings) VALUES('m','Meal',2);INSERT INTO meal_ingredient(sync_id,meal_id,ingredient_id,snapshot_name,snapshot_kcal_per_100g,amount_g,entered_amount,entered_unit) VALUES('mi','m','i','Old','99',1,'0.11','tsp');INSERT INTO trip_meal(sync_id,trip_id,day_number,meal_id,servings,snapshot_name,snapshot_kcal,snapshot_weight_g) VALUES('tm','t',1,'m','1','Old',9,9);INSERT INTO trip_meal(sync_id,trip_id,day_number,ingredient_id,snapshot_name,snapshot_kcal,snapshot_weight_g) VALUES('ti','t',1,'i','Old ingredient',999,1);").unwrap();
    let before = snapshot(&s.connection).unwrap();
    s.apply(Command::UpdateIngredient {
        id: "i".into(),
        input: IngredientInput {
            name: "New".into(),
            kcal_per_100g: Some(Dec::from(50)),
            density_g_ml: Some(Dec::from(1)),
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(
        n(
            &row(&s.connection, "meal_ingredient", "mi").unwrap(),
            "amount_g"
        ),
        1
    );
    assert_eq!(
        n(
            &row(&s.connection, "trip_meal", "tm").unwrap(),
            "snapshot_kcal"
        ),
        1
    );
    assert_eq!(
        n(
            &row(&s.connection, "trip_meal", "tm").unwrap(),
            "snapshot_weight_g"
        ),
        1
    );
    assert_eq!(
        n(
            &row(&s.connection, "trip_meal", "ti").unwrap(),
            "snapshot_kcal"
        ),
        1
    );
    assert_eq!(
        n(&row(&s.connection, "trip", "t").unwrap(), "food_g_per_day"),
        1
    );
    let out: Vec<_> = rows_outbox(&s);
    assert!(out.iter().all(|(t, _)| t == "ingredient"));
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s.connection).unwrap(), before);
    assert!(s.redo().unwrap());
    let before = snapshot(&s.connection).unwrap();
    s.apply(Command::UpdateGear {
        id: "g".into(),
        input: GearInput {
            name: "New gear".into(),
            quantity: 1,
            ..Default::default()
        },
    })
    .unwrap();
    let tg = row(&s.connection, "trip_gear", "tg").unwrap();
    assert_eq!(super::s(&tg, "snapshot_name"), "New gear");
    assert_eq!(n(&tg, "snapshot_weight_g"), 99);
    assert!(s.undo().unwrap());
    assert_eq!(snapshot(&s.connection).unwrap(), before);
}
fn rows_outbox(s: &Store) -> Vec<(String, String)> {
    s.connection
        .prepare("SELECT tbl,columns FROM outbox")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap()
}
#[test]
fn last_line_preserves_imported_trip_totals() {
    let mut s = Store::in_memory().unwrap();
    s.connection.execute_batch("INSERT INTO trip(sync_id,name,food_g_per_day) VALUES('t','Trip',99);INSERT INTO meal(sync_id,name)VALUES('m','Meal');INSERT INTO meal_ingredient(sync_id,meal_id,snapshot_name,amount_g) VALUES('l','m','Line',1);INSERT INTO trip_meal(sync_id,trip_id,day_number,meal_id,snapshot_name,snapshot_kcal,snapshot_weight_g) VALUES('tm','t',1,'m','Meal',50,100);").unwrap();
    roundtrip(&mut s, Command::RemoveMealIngredient { id: "l".into() });
    s.apply(Command::RemoveMealIngredient { id: "l".into() })
        .unwrap();
    assert_eq!(
        n(
            &row(&s.connection, "trip_meal", "tm").unwrap(),
            "snapshot_weight_g"
        ),
        100
    );
    assert_eq!(
        n(&row(&s.connection, "trip", "t").unwrap(), "food_g_per_day"),
        99
    );
}
#[test]
fn invalid_is_atomic_preserves_redo_and_constraints() {
    let mut s = fixture();
    let initial = snapshot(&s.connection).unwrap();
    s.apply(Command::CreateMeal {
        name: "Test".into(),
    })
    .unwrap();
    s.undo().unwrap();
    let count = rows_outbox(&s).len();
    for cmd in [
        Command::CreateGear {
            input: GearInput::default(),
        },
        Command::ReorderCategories {
            ordered_ids: vec![],
        },
        Command::SetIngredientMeasure {
            id: "missing".into(),
            unit: "xyz".into(),
            grams: Dec::from(0),
        },
        Command::DeleteMeal {
            id: "missing".into(),
        },
    ] {
        assert!(s.apply(cmd).is_err());
        assert_eq!(snapshot(&s.connection).unwrap(), initial);
        assert_eq!(rows_outbox(&s).len(), count);
    }
    assert!(s.redo().unwrap());
    s.apply(Command::CreateMeal {
        name: "Next".into(),
    })
    .unwrap();
    assert!(!s.redo().unwrap());
}
#[test]
fn filter_and_default_loadout() {
    let mut s = fixture();
    let all = s
        .gear_groups(&GearFilter {
            status: StatusFilter::All,
            query: String::new(),
        })
        .unwrap();
    assert!(
        all.iter()
            .flat_map(|g| &g.rows)
            .all(|r| r.status != "wishlist")
    );
    let filtered = s
        .gear_groups(&GearFilter {
            status: StatusFilter::All,
            query: "WATER".into(),
        })
        .unwrap();
    assert!(filtered.iter().all(|g| {
        g.name == "Water"
            || g.rows
                .iter()
                .all(|r| r.name.to_lowercase().contains("water"))
    }));
    let l = s.loadout_data().unwrap().loadouts[0].id.clone();
    s.apply(Command::SetAllLoadoutMembers {
        loadout: l,
        present: false,
    })
    .unwrap();
    assert_eq!(s.totals().unwrap().base, Grams(0));
}
#[test]
fn density_measure_uses_javascript_to_fixed_binary_rounding() {
    let mut store = Store::in_memory().unwrap();
    store
        .apply(Command::CreateIngredient {
            input: IngredientInput {
                name: "Measured".into(),
                ..Default::default()
            },
        })
        .unwrap();
    let id = store.ingredients().unwrap()[0].id.clone();
    store
        .apply(Command::SetIngredientMeasure {
            id: id.clone(),
            unit: "ml".into(),
            grams: "1.0005".parse().unwrap(),
        })
        .unwrap();
    assert_eq!(
        store.ingredients().unwrap()[0].input.density_g_ml,
        Some(Dec::from(1))
    );
    store
        .apply(Command::UpdateIngredient {
            id,
            input: IngredientInput {
                name: "Negative rounding".into(),
                kcal_per_100g: Some("-1.5".parse().unwrap()),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(
        store.ingredients().unwrap()[0].input.kcal_per_100g,
        Some(Dec::from(-1))
    );
}

#[test]
fn default_loadout_breakdown_merges_same_named_categories() {
    let mut store = Store::in_memory().unwrap();
    store.connection.execute_batch(
        "INSERT INTO gear_category(sync_id,name,sort_order) VALUES('a','Shelter',0),('b','Shelter',1);
         INSERT INTO gear_item(sync_id,name,category_id,weight_g) VALUES('one','Tent','a',100),('two','Tarp','b',200);
         INSERT INTO loadout(sync_id,name,is_default) VALUES('l','Weekend',1);
         INSERT INTO loadout_item(sync_id,loadout_id,gear_item_id) VALUES('li1','l','one'),('li2','l','two');"
    ).unwrap();
    let totals = store.totals().unwrap();
    assert_eq!(totals.base, Grams(300));
    assert_eq!(totals.by_category.len(), 1);
    assert_eq!(totals.by_category[0].g, Grams(300));
    assert_eq!(totals.by_category[0].items.len(), 2);
    store
        .apply(Command::DeleteLoadout { id: "l".into() })
        .unwrap();
    assert_eq!(store.totals().unwrap().by_category.len(), 2);
}
