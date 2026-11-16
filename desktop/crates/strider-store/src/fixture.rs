use super::*;
#[derive(Debug, Clone, Copy)]
pub enum FixtureSize {
    Small,
    Development,
    Items(usize),
}
impl FixtureSize {
    fn items(self) -> usize {
        match self {
            Self::Small => 24,
            Self::Development => 2000,
            Self::Items(n) => n,
        }
    }
}
struct Ids(u128);
impl Ids {
    fn next(&mut self) -> String {
        self.0 += 1;
        Uuid::from_u128((1_700_000_000_000u128 << 80) | (7u128 << 76) | (2u128 << 62) | self.0)
            .to_string()
    }
}
const STAMP: &str = "2026-01-01T00:00:00.000Z";
const CATEGORIES: [&str; 12] = [
    "Shelter",
    "Sleep",
    "Packs",
    "Kitchen",
    "Water",
    "Clothing",
    "Worn",
    "Electronics",
    "Safety",
    "Repair",
    "Hygiene",
    "Food",
];
pub fn seed(store: &mut Store, items: usize) -> Result<()> {
    seed_fixture(store, FixtureSize::Items(items))
}
pub fn seed_fixture(store: &mut Store, size: FixtureSize) -> Result<()> {
    if store
        .connection
        .query_row("SELECT count(*) FROM gear_item", [], |r| r.get::<_, i64>(0))?
        != 0
    {
        return Ok(());
    }
    let tx = store.connection.transaction()?;
    let mut ids = Ids(0);
    let mut cats = vec![];
    for (i, name) in CATEGORIES.iter().enumerate() {
        let key = ids.next();
        write(
            &tx,
            "gear_category",
            &key,
            fields([
                ("name", text(*name)),
                ("sort_order", Value::Integer(i as i64)),
            ]),
            true,
        )?;
        cats.push(key);
    }
    let mut loadouts = vec![];
    for (i, name) in ["Weekend", "Alpine", "Summer", "Winter"].iter().enumerate() {
        let key = ids.next();
        write(
            &tx,
            "loadout",
            &key,
            fields([
                ("name", text(*name)),
                ("is_default", Value::Integer((i == 0).into())),
                ("sort_order", Value::Integer(i as i64)),
                ("created_at", text(STAMP)),
            ]),
            true,
        )?;
        loadouts.push(key);
    }
    let mut current = vec![];
    let mut wishlist = vec![];
    for i in 0..size.items() {
        let key = ids.next();
        let cat = i % 12;
        let kit = i % 40 == 0;
        let status = if i < 20 {
            "wishlist"
        } else if i % 17 == 0 {
            "retired"
        } else {
            "current"
        };
        let name = format!(
            "{} {} {:04}",
            if i % 2 == 0 { "Ultralight" } else { "Trail" },
            [
                "tarp",
                "quilt",
                "backpack",
                "stove",
                "water filter",
                "rain jacket",
                "trail runners",
                "headlamp",
                "first aid",
                "multitool",
                "toothbrush",
                "trail mix"
            ][cat],
            i
        );
        let weight = if kit { 90 } else { 5 + (i * 173 % 1496) as i32 };
        let class = if cat == 6 {
            "worn"
        } else if cat == 11 {
            "consumable"
        } else {
            "base"
        };
        write(
            &tx,
            "gear_item",
            &key,
            fields([
                ("name", text(name)),
                ("category_id", text(&cats[cat])),
                ("weight_g", Value::Integer(weight.into())),
                ("quantity", Value::Integer(if i % 31 == 0 { 2 } else { 1 })),
                ("sort_order", Value::Integer((i / 12) as i64)),
                ("default_weight_class", text(class)),
                ("status", text(status)),
                ("is_kit", Value::Integer(kit.into())),
                ("created_at", text(STAMP)),
                ("updated_at", text(STAMP)),
            ]),
            true,
        )?;
        if kit {
            for j in 0..3 {
                write(
                    &tx,
                    "gear_component",
                    &ids.next(),
                    fields([
                        ("gear_item_id", text(&key)),
                        ("name", text(format!("Component {}", j + 1))),
                        ("weight_g", Value::Integer(20 + j * 10)),
                        ("quantity", Value::Integer(1)),
                    ]),
                    true,
                )?;
            }
        }
        if status == "current" {
            current.push(key.clone());
            for (j, l) in loadouts.iter().enumerate() {
                if (i + j) % 3 != 0 {
                    write(
                        &tx,
                        "loadout_item",
                        &ids.next(),
                        fields([
                            ("loadout_id", text(l)),
                            ("gear_item_id", text(&key)),
                            (
                                "weight_class",
                                text(if class == "worn" { "worn" } else { "base" }),
                            ),
                        ]),
                        true,
                    )?;
                }
            }
        }
        if status == "wishlist" {
            wishlist.push(key);
        }
    }
    for (i, w) in wishlist.iter().enumerate() {
        if !current.is_empty() {
            write(
                &tx,
                "wishlist_replacement",
                &ids.next(),
                fields([
                    ("wishlist_item_id", text(w)),
                    ("replaces_item_id", text(&current[i % current.len()])),
                ]),
                true,
            )?;
        }
        write(
            &tx,
            "wishlist_loadout",
            &ids.next(),
            fields([
                ("wishlist_item_id", text(w)),
                ("loadout_id", text(&loadouts[i % 4])),
                ("included", Value::Integer((i % 2 == 0).into())),
            ]),
            true,
        )?;
    }
    let mut ingredients = vec![];
    for i in 0..80 {
        let key = ids.next();
        write(
            &tx,
            "ingredient",
            &key,
            fields([
                ("name", text(format!("Ingredient {i:02}"))),
                ("kcal_per_100g", text((100 + i * 7).to_string())),
                (
                    "density_g_ml",
                    if i % 3 == 0 {
                        Value::Null
                    } else {
                        text("0.45")
                    },
                ),
                ("source", text("manual")),
                ("default_serving_g", Value::Integer(50)),
            ]),
            true,
        )?;
        ingredients.push(key);
    }
    for i in 0..30 {
        let key = ids.next();
        write(
            &tx,
            "meal",
            &key,
            fields([
                ("name", text(format!("Meal {i:02}"))),
                ("base_servings", Value::Integer(2)),
                ("created_at", text(STAMP)),
            ]),
            true,
        )?;
        for j in 0..3 {
            let ing = &ingredients[(i * 3 + j) % 80];
            let r = row(&tx, "ingredient", ing)?;
            write(
                &tx,
                "meal_ingredient",
                &ids.next(),
                fields([
                    ("meal_id", text(&key)),
                    ("ingredient_id", text(ing)),
                    ("snapshot_name", text(s(&r, "name"))),
                    ("snapshot_kcal_per_100g", r["kcal_per_100g"].clone()),
                    ("snapshot_density_g_ml", r["density_g_ml"].clone()),
                    (
                        "amount_g",
                        Value::Integer(if j == 1 && r["density_g_ml"] != Value::Null {
                            106
                        } else {
                            50
                        }),
                    ),
                    (
                        "entered_amount",
                        if j == 1 { text("1") } else { Value::Null },
                    ),
                    (
                        "entered_unit",
                        if j == 1 { text("cup") } else { Value::Null },
                    ),
                    ("sort_order", Value::Integer(j as i64)),
                ]),
                true,
            )?;
        }
    }
    tx.commit()?;
    store.undo.clear();
    store.redo.clear();
    Ok(())
}
