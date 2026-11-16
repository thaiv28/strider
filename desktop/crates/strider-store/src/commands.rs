use super::*;
#[derive(Debug, Clone, Default)]
pub struct GearInput {
    pub name: String,
    pub category_id: Option<String>,
    pub weight_g: Option<Grams>,
    pub weight_oz: Option<f64>,
    pub price_cents: Option<Cents>,
    pub quantity: i32,
    pub class: String,
    pub status: String,
    pub is_kit: bool,
    pub components: Vec<ComponentInput>,
    pub product_url: Option<String>,
    pub notes: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct ComponentInput {
    pub name: String,
    pub weight_g: i32,
    pub quantity: i32,
    pub notes: Option<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngredientInput {
    pub name: String,
    pub kcal_per_100g: Option<Dec>,
    pub density_g_ml: Option<Dec>,
    pub density_source: Option<String>,
    pub reference_name: Option<String>,
    pub default_serving_g: Option<i32>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub notes: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct MealPatch {
    pub name: Option<String>,
    pub base_servings: Option<i32>,
    pub meal_type: Option<Option<String>>,
    pub is_hot: Option<bool>,
    pub water_ml: Option<Option<i32>>,
    pub notes: Option<Option<String>>,
}
#[derive(Debug, Clone)]
pub enum Command {
    CreateTrip {
        name: String,
        trail: Option<String>,
        loadout: Option<String>,
    },
    UpdateTrip {
        trip: String,
        patch: Row,
    },
    DeleteTrip {
        trip: String,
    },
    ImportTrip {
        json: String,
    },
    SeedTripFromLoadout {
        trip: String,
        loadout: String,
    },
    AddTripGear {
        trip: String,
        gear: String,
    },
    RemoveTripGear {
        trip: String,
        id: String,
    },
    RestoreTripGear {
        trip: String,
        snapshot: Row,
    },
    SetTripGearClass {
        trip: String,
        id: String,
        class: String,
    },
    SetTripGearQty {
        trip: String,
        id: String,
        qty: f64,
    },
    SetTripGearPacked {
        trip: String,
        id: String,
        packed: bool,
    },
    ClearTripGear {
        trip: String,
    },
    MoveTripGear {
        trip: String,
        id: String,
        category: String,
        ordered_ids: Vec<String>,
    },
    SaveTripPackingList {
        trip: String,
        packing_list: String,
    },
    UpsertTripDay {
        trip: String,
        day: i32,
        distance_mi: Option<Option<String>>,
        elevation_gain_ft: Option<Option<i32>>,
    },
    AddTripDay {
        trip: String,
    },
    DeleteTripDay {
        trip: String,
        id: String,
    },
    AddCampsite {
        trip: String,
        site: crate::trips::Site,
    },
    MoveCampsite {
        trip: String,
        id: String,
        site: crate::trips::Site,
    },
    RemoveCampsite {
        trip: String,
        id: String,
    },
    ClearCampsites {
        trip: String,
    },
    AttachGpx {
        trip: String,
        filename: String,
        bytes: Vec<u8>,
    },
    ReverseGpx {
        trip: String,
    },
    RemoveGpx {
        trip: String,
    },
    RestoreGpx {
        trip: String,
        snapshot: Row,
    },
    AttachPermit {
        trip: String,
        filename: String,
        mime_type: String,
        bytes: Vec<u8>,
    },
    DeletePermit {
        trip: String,
    },
    AddTripMeal {
        trip: String,
        day: i32,
        entry: crate::trips::AddTripFood,
    },
    SetTripMealQty {
        trip: String,
        id: String,
        qty: f64,
    },
    RemoveTripMeal {
        trip: String,
        id: String,
    },
    CopyTripMealToAll {
        trip: String,
        id: String,
    },
    CreateGear {
        input: GearInput,
    },
    UpdateGear {
        id: String,
        input: GearInput,
    },
    DeleteGear {
        id: String,
    },
    MoveGear {
        item: String,
        to_category: Option<String>,
        ordered_ids: Vec<String>,
    },
    SetWishlistReplacements {
        item: String,
        replaces: Vec<String>,
    },
    SetWishlistLoadout {
        item: String,
        loadout: String,
        included: Option<bool>,
    },
    CreateLoadout {
        name: String,
    },
    RenameLoadout {
        id: String,
        name: String,
    },
    ReorderLoadouts {
        ordered_ids: Vec<String>,
    },
    DeleteLoadout {
        id: String,
    },
    SetDefaultLoadout {
        id: String,
    },
    SetLoadoutMember {
        loadout: String,
        item: String,
        present: bool,
        class: String,
    },
    SetAllLoadoutMembers {
        loadout: String,
        present: bool,
    },
    CreateCategory {
        name: String,
    },
    RenameCategory {
        id: String,
        name: String,
    },
    ReorderCategories {
        ordered_ids: Vec<String>,
    },
    DeleteCategory {
        id: String,
    },
    CreateIngredient {
        input: IngredientInput,
    },
    UpdateIngredient {
        id: String,
        input: IngredientInput,
    },
    DeleteIngredient {
        id: String,
    },
    SetIngredientMeasure {
        id: String,
        unit: String,
        grams: Dec,
    },
    LinkIngredientReference {
        id: String,
        density: Dec,
        reference_name: Option<String>,
    },
    CreateMeal {
        name: String,
    },
    UpdateMeal {
        id: String,
        patch: MealPatch,
    },
    DeleteMeal {
        id: String,
    },
    AddMealIngredient {
        meal: String,
        ingredient: String,
        amount_g: i32,
    },
    UpdateMealIngredientAmount {
        id: String,
        amount_g: i32,
        entered: Option<(Dec, String)>,
    },
    RemoveMealIngredient {
        id: String,
    },
    ReorderMealIngredients {
        meal: String,
        ordered_ids: Vec<String>,
    },
    UpdateItem {
        id: String,
        name: String,
        weight: Option<Grams>,
        quantity: i32,
    },
    MoveItem {
        id: String,
        to_category: Option<String>,
        before: Option<String>,
    },
    DeleteItem {
        id: String,
    },
}
#[derive(Default)]
pub(crate) struct Affected {
    pub gear: Vec<String>,
    pub ingredients: Vec<String>,
    pub meals: Vec<String>,
    pub derive_days: BTreeSet<String>,
    pub campsite_days: BTreeSet<String>,
    pub food_trips: BTreeSet<String>,
    pub trip_command: bool,
}
fn clean(v: Option<String>) -> Value {
    nullable(v.and_then(|v| {
        let v = v.trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    }))
}
fn fallback(name: String, default: &str) -> String {
    let v = name.trim();
    if v.is_empty() {
        default.into()
    } else {
        v.into()
    }
}
fn gear_values(input: &GearInput) -> Result<Row> {
    if input.name.trim().is_empty() {
        return invalid("Name is required");
    }
    let weight = if input.is_kit {
        let total: i64 = input
            .components
            .iter()
            .filter(|c| !c.name.trim().is_empty())
            .map(|c| i64::from(c.weight_g.max(0)) * i64::from(c.quantity.max(1)))
            .sum();
        Some(i32::try_from(total).map_err(|_| Error::Invalid("Kit weight overflow".into()))?)
    } else if let Some(oz) = input.weight_oz.filter(|o| o.is_finite()) {
        {
            let grams = (oz * strider_core::format::OZ_TO_G + 0.5).floor();
            if grams < f64::from(i32::MIN) || grams > f64::from(i32::MAX) {
                return invalid("Weight exceeds the supported gram range");
            }
            Some(strider_core::format::oz_to_g(oz).0)
        }
    } else {
        input.weight_g.map(|g| g.0)
    };
    Ok(fields([
        ("name", text(input.name.trim())),
        ("category_id", nullable(input.category_id.clone())),
        ("weight_g", number(weight)),
        ("price_cents", number(input.price_cents.map(|c| c.0.max(0)))),
        ("quantity", Value::Integer(input.quantity.max(1).into())),
        (
            "default_weight_class",
            text(if input.class.is_empty() {
                "base"
            } else {
                &input.class
            }),
        ),
        (
            "status",
            text(if input.status.is_empty() {
                "current"
            } else {
                &input.status
            }),
        ),
        ("is_kit", Value::Integer(input.is_kit.into())),
        ("product_url", clean(input.product_url.clone())),
        ("notes", clean(input.notes.clone())),
    ]))
}
fn ingredient_values(v: IngredientInput, create: bool) -> Row {
    fields([
        ("name", text(v.name.trim())),
        (
            "kcal_per_100g",
            v.kcal_per_100g
                .map(|d| text(js_round(d).to_string()))
                .unwrap_or(Value::Null),
        ),
        ("density_g_ml", decimal(v.density_g_ml)),
        ("density_source", nullable(v.density_source)),
        ("reference_name", nullable(v.reference_name)),
        ("default_serving_g", number(v.default_serving_g)),
        ("category", nullable(v.category)),
        (
            "source",
            nullable(if create {
                Some(v.source.unwrap_or_else(|| "manual".into()))
            } else {
                v.source
            }),
        ),
        ("source_id", nullable(v.source_id)),
        ("notes", nullable(v.notes)),
    ])
}
fn js_round(d: Dec) -> i64 {
    {
        let scale = 1_000_000_000;
        let whole = d.scaled().div_euclid(scale);
        let fraction = d.scaled().rem_euclid(scale);
        (whole + i128::from(fraction >= scale / 2)) as i64
    }
}
fn components(db: &Connection, key: &str, input: &GearInput) -> Result<()> {
    db.execute("DELETE FROM gear_component WHERE gear_item_id=?", [key])?;
    if input.is_kit {
        for c in &input.components {
            if !c.name.trim().is_empty() {
                write(
                    db,
                    "gear_component",
                    &id(),
                    fields([
                        ("gear_item_id", text(key)),
                        ("name", text(c.name.trim())),
                        ("weight_g", Value::Integer(c.weight_g.max(0).into())),
                        ("quantity", Value::Integer(c.quantity.max(1).into())),
                        ("notes", clean(c.notes.clone())),
                    ]),
                    true,
                )?;
            }
        }
    }
    Ok(())
}
fn ids_where(
    db: &Connection,
    table: &str,
    column: &str,
    value: Option<&str>,
) -> Result<Vec<String>> {
    Ok(rows(db, table)?
        .into_iter()
        .filter(|r| opt(r, column).as_deref() == value)
        .map(|r| s(&r, "sync_id").to_string())
        .collect())
}
fn order(db: &Connection, table: &str, ids: Vec<String>, expected: Vec<String>) -> Result<()> {
    let a: BTreeSet<_> = ids.iter().collect();
    let b: BTreeSet<_> = expected.iter().collect();
    if a != b || a.len() != ids.len() {
        return invalid("Invalid order");
    };
    for (i, key) in ids.iter().enumerate() {
        write(
            db,
            table,
            key,
            fields([("sort_order", Value::Integer(i as i64))]),
            false,
        )?;
    }
    Ok(())
}
fn set_member(
    db: &Connection,
    loadout: &str,
    item: &str,
    present: bool,
    class: &str,
    preserve: bool,
) -> Result<()> {
    row(db, "loadout", loadout)?;
    row(db, "gear_item", item)?;
    let existing = rows(db, "loadout_item")?
        .into_iter()
        .find(|r| s(r, "loadout_id") == loadout && s(r, "gear_item_id") == item);
    match (existing, present) {
        (Some(r), false) => delete(db, "loadout_item", s(&r, "sync_id")),
        (Some(r), true) if !preserve => write(
            db,
            "loadout_item",
            s(&r, "sync_id"),
            fields([("weight_class", text(class))]),
            false,
        ),
        (None, true) => write(
            db,
            "loadout_item",
            &id(),
            fields([
                ("loadout_id", text(loadout)),
                ("gear_item_id", text(item)),
                ("weight_class", text(class)),
            ]),
            true,
        ),
        _ => Ok(()),
    }
}
pub(crate) fn run(db: &Connection, cmd: Command, data_dir: &Path) -> Result<Affected> {
    let mut a = Affected::default();
    match cmd {
        Command::CreateGear { input } => {
            let key = id();
            write(db, "gear_item", &key, gear_values(&input)?, true)?;
            components(db, &key, &input)?;
        }
        Command::UpdateGear { id: key, input } => {
            let mut v = gear_values(&input)?;
            v.insert("updated_at".into(), text(now(db)?));
            write(db, "gear_item", &key, v, false)?;
            components(db, &key, &input)?;
            a.gear.push(key);
        }
        Command::DeleteGear { id: key } | Command::DeleteItem { id: key } => {
            row(db, "gear_item", &key)?;
            let used: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM trip_gear WHERE gear_item_id=?)",
                [&key],
                |r| r.get(0),
            )?;
            if used {
                return invalid("Cannot delete gear used by a trip; remove it from the trip first");
            }
            delete(db, "gear_item", &key)?;
        }
        Command::UpdateItem {
            id: key,
            name,
            weight,
            quantity,
        } => {
            if name.trim().is_empty() || quantity < 1 || weight.is_some_and(|g| g.0 < 0) {
                return invalid("Name, nonnegative weight and positive quantity required");
            };
            if weight.is_some_and(|g| g.0.checked_mul(quantity).is_none()) {
                return invalid("Weight overflow");
            };
            write(
                db,
                "gear_item",
                &key,
                fields([
                    ("name", text(name.trim())),
                    ("weight_g", number(weight.map(|g| g.0))),
                    ("quantity", Value::Integer(quantity.into())),
                ]),
                false,
            )?;
            a.gear.push(key);
        }
        Command::MoveItem {
            id: key,
            to_category,
            before,
        } => {
            let mut target = rows(db, "gear_item")?
                .into_iter()
                .filter(|r| opt(r, "category_id") == to_category && s(r, "sync_id") != key)
                .collect::<Vec<_>>();
            target.sort_by_key(|r| (n(r, "sort_order"), s(r, "sync_id").to_string()));
            let mut ids: Vec<_> = target.iter().map(|r| s(r, "sync_id").to_string()).collect();
            let pos = match before {
                Some(b) => ids
                    .iter()
                    .position(|i| i == &b)
                    .ok_or_else(|| Error::Invalid("Insertion item not in destination".into()))?,
                None => ids.len(),
            };
            ids.insert(pos, key.clone());
            move_gear(db, key, to_category, ids)?;
        }
        Command::MoveGear {
            item,
            to_category,
            ordered_ids,
        } => move_gear(db, item, to_category, ordered_ids)?,
        Command::SetWishlistReplacements { item, replaces } => {
            row(db, "gear_item", &item)?;
            let ids: BTreeSet<_> = replaces.into_iter().filter(|r| r != &item).collect();
            for key in &ids {
                row(db, "gear_item", key)?;
            }
            db.execute(
                "DELETE FROM wishlist_replacement WHERE wishlist_item_id=?",
                [&item],
            )?;
            for key in ids {
                write(
                    db,
                    "wishlist_replacement",
                    &id(),
                    fields([
                        ("wishlist_item_id", text(&item)),
                        ("replaces_item_id", text(key)),
                    ]),
                    true,
                )?;
            }
        }
        Command::SetWishlistLoadout {
            item,
            loadout,
            included,
        } => {
            row(db, "gear_item", &item)?;
            row(db, "loadout", &loadout)?;
            let existing = rows(db, "wishlist_loadout")?
                .into_iter()
                .find(|r| s(r, "wishlist_item_id") == item && s(r, "loadout_id") == loadout);
            match (existing, included) {
                (Some(r), None) => delete(db, "wishlist_loadout", s(&r, "sync_id"))?,
                (Some(r), Some(b)) => write(
                    db,
                    "wishlist_loadout",
                    s(&r, "sync_id"),
                    fields([("included", Value::Integer(b.into()))]),
                    false,
                )?,
                (None, Some(b)) => write(
                    db,
                    "wishlist_loadout",
                    &id(),
                    fields([
                        ("wishlist_item_id", text(item)),
                        ("loadout_id", text(loadout)),
                        ("included", Value::Integer(b.into())),
                    ]),
                    true,
                )?,
                _ => {}
            }
        }
        Command::CreateLoadout { name } => {
            let min: i64 = db.query_row(
                "SELECT coalesce(min(sort_order),0)-1 FROM loadout",
                [],
                |r| r.get(0),
            )?;
            write(
                db,
                "loadout",
                &id(),
                fields([
                    ("name", text(fallback(name, "New Loadout"))),
                    ("sort_order", Value::Integer(min)),
                ]),
                true,
            )?;
        }
        Command::RenameLoadout { id: key, name } => write(
            db,
            "loadout",
            &key,
            fields([("name", text(fallback(name, "Loadout")))]),
            false,
        )?,
        Command::DeleteLoadout { id: key } => delete(db, "loadout", &key)?,
        Command::ReorderLoadouts { ordered_ids } => order(
            db,
            "loadout",
            ordered_ids,
            rows(db, "loadout")?
                .iter()
                .map(|r| s(r, "sync_id").to_string())
                .collect(),
        )?,
        Command::SetDefaultLoadout { id: key } => {
            row(db, "loadout", &key)?;
            db.execute("UPDATE loadout SET is_default=0", [])?;
            db.execute("UPDATE loadout SET is_default=1 WHERE sync_id=?", [key])?;
        }
        Command::SetLoadoutMember {
            loadout,
            item,
            present,
            class,
        } => set_member(db, &loadout, &item, present, &class, false)?,
        Command::SetAllLoadoutMembers { loadout, present } => {
            row(db, "loadout", &loadout)?;
            if !present {
                db.execute("DELETE FROM loadout_item WHERE loadout_id=?", [loadout])?;
            } else {
                for r in rows(db, "gear_item")? {
                    if s(&r, "status") == "current" {
                        set_member(
                            db,
                            &loadout,
                            s(&r, "sync_id"),
                            true,
                            if s(&r, "default_weight_class") == "worn" {
                                "worn"
                            } else {
                                "base"
                            },
                            true,
                        )?;
                    }
                }
            }
        }
        Command::CreateCategory { name } => {
            let max: i64 = db.query_row(
                "SELECT coalesce(max(sort_order),-1)+1 FROM gear_category",
                [],
                |r| r.get(0),
            )?;
            write(
                db,
                "gear_category",
                &id(),
                fields([
                    ("name", text(fallback(name, "Category"))),
                    ("sort_order", Value::Integer(max)),
                ]),
                true,
            )?;
        }
        Command::RenameCategory { id: key, name } => write(
            db,
            "gear_category",
            &key,
            fields([("name", text(fallback(name, "Category")))]),
            false,
        )?,
        Command::ReorderCategories { ordered_ids } => order(
            db,
            "gear_category",
            ordered_ids,
            rows(db, "gear_category")?
                .iter()
                .map(|r| s(r, "sync_id").to_string())
                .collect(),
        )?,
        Command::DeleteCategory { id: key } => {
            row(db, "gear_category", &key)?;
            db.execute(
                "UPDATE gear_item SET category_id=NULL WHERE category_id=?",
                [&key],
            )?;
            db.execute(
                "UPDATE gear_category SET parent_id=NULL WHERE parent_id=?",
                [&key],
            )?;
            delete(db, "gear_category", &key)?;
        }
        Command::CreateIngredient { input } => {
            if input.name.trim().is_empty() {
                return invalid("Name is required");
            };
            write(
                db,
                "ingredient",
                &id(),
                ingredient_values(input, true),
                true,
            )?;
        }
        Command::UpdateIngredient { id: key, input } => {
            write(
                db,
                "ingredient",
                &key,
                ingredient_values(input, false),
                false,
            )?;
            a.ingredients.push(key);
        }
        Command::DeleteIngredient { id: key } => delete(db, "ingredient", &key)?,
        Command::SetIngredientMeasure {
            id: key,
            unit,
            grams,
        } => {
            let ml = strider_core::food::VOLUME_ML
                .iter()
                .find(|(u, _)| *u == unit)
                .map(|(_, ml)| *ml)
                .ok_or_else(|| Error::Invalid("A volume unit is required".into()))?;
            if grams <= Dec::from(0) {
                return invalid("Positive weight required");
            };
            let d: Dec = strider_core::format::fixed(grams.as_f64() / ml, 3)
                .parse()
                .map_err(|_| {
                    Error::Invalid("Density exceeds the supported decimal range".into())
                })?;
            write(
                db,
                "ingredient",
                &key,
                fields([
                    ("density_g_ml", text(d.to_string())),
                    ("density_source", text("manual")),
                    ("reference_name", Value::Null),
                ]),
                false,
            )?;
            a.ingredients.push(key);
        }
        Command::LinkIngredientReference {
            id: key,
            density,
            reference_name,
        } => {
            write(
                db,
                "ingredient",
                &key,
                fields([
                    ("density_g_ml", text(density.to_string())),
                    ("density_source", text("usda")),
                    ("reference_name", nullable(reference_name)),
                ]),
                false,
            )?;
            a.ingredients.push(key);
        }
        Command::CreateMeal { name } => write(
            db,
            "meal",
            &id(),
            fields([("name", text(fallback(name, "New meal")))]),
            true,
        )?,
        Command::UpdateMeal { id: key, patch } => {
            let mut v = Row::new();
            if let Some(x) = patch.name {
                v.insert("name".into(), text(fallback(x, "Untitled")));
            }
            if let Some(x) = patch.base_servings {
                v.insert("base_servings".into(), Value::Integer(x.max(1).into()));
            }
            if let Some(x) = patch.meal_type {
                v.insert("meal_type".into(), nullable(x.filter(|s| !s.is_empty())));
            }
            if let Some(x) = patch.is_hot {
                v.insert("is_hot".into(), Value::Integer(x.into()));
            }
            if let Some(x) = patch.water_ml {
                v.insert("water_ml".into(), number(x));
            }
            if let Some(x) = patch.notes {
                v.insert("notes".into(), nullable(x));
            }
            row(db, "meal", &key)?;
            if !v.is_empty() {
                write(db, "meal", &key, v, false)?;
                a.meals.push(key);
            }
        }
        Command::DeleteMeal { id: key } => delete(db, "meal", &key)?,
        Command::AddMealIngredient {
            meal,
            ingredient,
            amount_g,
        } => {
            row(db, "meal", &meal)?;
            let ing = row(db, "ingredient", &ingredient)?;
            let max: i64 = db.query_row(
                "SELECT coalesce(max(sort_order),-1)+1 FROM meal_ingredient WHERE meal_id=?",
                [&meal],
                |r| r.get(0),
            )?;
            write(
                db,
                "meal_ingredient",
                &id(),
                fields([
                    ("meal_id", text(&meal)),
                    ("ingredient_id", text(ingredient)),
                    ("snapshot_name", text(s(&ing, "name"))),
                    (
                        "snapshot_kcal_per_100g",
                        dec(&ing, "kcal_per_100g")?
                            .map(|d| text(js_round(d).to_string()))
                            .unwrap_or(Value::Null),
                    ),
                    ("snapshot_density_g_ml", ing["density_g_ml"].clone()),
                    ("amount_g", Value::Integer(amount_g.max(0).into())),
                    ("sort_order", Value::Integer(max)),
                ]),
                true,
            )?;
            a.meals.push(meal);
        }
        Command::UpdateMealIngredientAmount {
            id: key,
            amount_g,
            entered,
        } => {
            let line = row(db, "meal_ingredient", &key)?;
            let entered = entered.filter(|(d, u)| {
                *d >= Dec::from(0)
                    && (u == "g"
                        || u == "oz"
                        || strider_core::food::VOLUME_ML.iter().any(|(k, _)| k == u))
            });
            write(
                db,
                "meal_ingredient",
                &key,
                fields([
                    ("amount_g", Value::Integer(amount_g.max(0).into())),
                    ("entered_amount", decimal(entered.as_ref().map(|(d, _)| *d))),
                    ("entered_unit", nullable(entered.map(|(_, u)| u))),
                ]),
                false,
            )?;
            a.meals.push(s(&line, "meal_id").into());
        }
        Command::RemoveMealIngredient { id: key } => {
            let line = row(db, "meal_ingredient", &key)?;
            delete(db, "meal_ingredient", &key)?;
            a.meals.push(s(&line, "meal_id").into());
        }
        Command::ReorderMealIngredients { meal, ordered_ids } => {
            row(db, "meal", &meal)?;
            order(
                db,
                "meal_ingredient",
                ordered_ids,
                ids_where(db, "meal_ingredient", "meal_id", Some(&meal))?,
            )?;
        }
        cmd => crate::trips::run(db, data_dir, cmd, &mut a)?,
    }
    Ok(a)
}
fn move_gear(
    db: &Connection,
    item: String,
    to: Option<String>,
    ordered: Vec<String>,
) -> Result<()> {
    row(db, "gear_item", &item)?;
    if let Some(c) = &to {
        row(db, "gear_category", c)?;
    }
    for key in &ordered {
        row(db, "gear_item", key)?;
    }
    write(
        db,
        "gear_item",
        &item,
        fields([("category_id", nullable(to.clone()))]),
        false,
    )?;
    let mut all = rows(db, "gear_item")?
        .into_iter()
        .filter(|r| opt(r, "category_id") == to)
        .collect::<Vec<_>>();
    all.sort_by_key(|r| (n(r, "sort_order"), s(r, "sync_id").to_string()));
    let seen: BTreeSet<_> = ordered.iter().cloned().collect();
    let mut final_order = ordered;
    final_order.extend(
        all.iter()
            .map(|r| s(r, "sync_id").to_string())
            .filter(|k| !seen.contains(k)),
    );
    for (i, key) in final_order.iter().enumerate() {
        write(
            db,
            "gear_item",
            key,
            fields([("sort_order", Value::Integer(i as i64))]),
            false,
        )?;
    }
    Ok(())
}
fn now(db: &Connection) -> Result<String> {
    Ok(
        db.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
            r.get(0)
        })?,
    )
}
