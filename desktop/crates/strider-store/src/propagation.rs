use super::*;
use strider_core::propagation as p;
fn line(r: &Row) -> Result<p::MealLine> {
    Ok(p::MealLine {
        snapshot_name: s(r, "snapshot_name").into(),
        snapshot_kcal_per_100g: dec(r, "snapshot_kcal_per_100g")?,
        snapshot_density_g_ml: dec(r, "snapshot_density_g_ml")?,
        amount_g: n(r, "amount_g"),
        entered_amount: dec(r, "entered_amount")?,
        entered_unit: opt(r, "entered_unit"),
    })
}
fn ingredient(r: &Row) -> Result<p::Ingredient> {
    Ok(p::Ingredient {
        name: s(r, "name").into(),
        kcal_per_100g: dec(r, "kcal_per_100g")?,
        density_g_ml: dec(r, "density_g_ml")?,
    })
}
pub(crate) fn run(db: &Connection, a: commands::Affected) -> Result<()> {
    for key in a.gear {
        let r = row(db, "gear_item", &key)?;
        let item = p::GearItem {
            name: s(&r, "name").into(),
            weight_g: match r["weight_g"] {
                Value::Integer(g) => Some(g),
                _ => None,
            },
        };
        for r in rows(db, "trip_gear")?
            .iter()
            .filter(|r| s(r, "gear_item_id") == key)
        {
            let mut snapshot = p::GearSnapshot {
                snapshot_name: s(r, "snapshot_name").into(),
                snapshot_weight_g: n(r, "snapshot_weight_g"),
            };
            p::trip_gear_from_item(&mut snapshot, &item);
            write(
                db,
                "trip_gear",
                s(r, "sync_id"),
                fields([
                    ("snapshot_name", text(snapshot.snapshot_name)),
                    (
                        "snapshot_weight_g",
                        Value::Integer(snapshot.snapshot_weight_g),
                    ),
                ]),
                false,
            )?;
        }
    }
    let mut meals: BTreeSet<_> = a.meals.into_iter().collect();
    for key in a.ingredients {
        let i = ingredient(&row(db, "ingredient", &key)?)?;
        for r in rows(db, "meal_ingredient")?
            .iter()
            .filter(|r| s(r, "ingredient_id") == key)
        {
            let mut l = line(r)?;
            p::meal_line_from_ingredient(&mut l, &i);
            write(
                db,
                "meal_ingredient",
                s(r, "sync_id"),
                fields([
                    ("snapshot_name", text(l.snapshot_name)),
                    ("snapshot_kcal_per_100g", decimal(l.snapshot_kcal_per_100g)),
                    ("snapshot_density_g_ml", decimal(l.snapshot_density_g_ml)),
                    ("amount_g", Value::Integer(l.amount_g)),
                ]),
                false,
            )?;
            meals.insert(s(r, "meal_id").to_string());
        }
        for r in rows(db, "trip_meal")?
            .iter()
            .filter(|r| s(r, "ingredient_id") == key && opt(r, "meal_id").is_none())
        {
            let mut entry = p::TripMealSnapshot {
                snapshot_name: s(r, "snapshot_name").into(),
                snapshot_kcal: n(r, "snapshot_kcal"),
                snapshot_weight_g: n(r, "snapshot_weight_g"),
            };
            p::standalone_trip_meal_from_ingredient(&mut entry, &i);
            write(
                db,
                "trip_meal",
                s(r, "sync_id"),
                fields([
                    ("snapshot_name", text(entry.snapshot_name)),
                    ("snapshot_kcal", Value::Integer(entry.snapshot_kcal)),
                ]),
                false,
            )?;
        }
    }
    let mut trips = BTreeSet::new();
    for key in meals {
        let m = row(db, "meal", &key)?;
        let lines = rows(db, "meal_ingredient")?
            .iter()
            .filter(|r| s(r, "meal_id") == key)
            .map(line)
            .collect::<Result<Vec<_>>>()?;
        let totals = p::meal_totals(&lines);
        for r in rows(db, "trip_meal")?
            .iter()
            .filter(|r| s(r, "meal_id") == key)
        {
            let mut values = fields([("snapshot_name", text(s(&m, "name")))]);
            if let Some(totals) = totals {
                let servings = dec(r, "servings")?.unwrap_or(Dec::from(1));
                let (kcal, g) = p::trip_meal_snapshot(totals, servings, n(&m, "base_servings"));
                values.insert("snapshot_kcal".into(), Value::Integer(kcal));
                values.insert("snapshot_weight_g".into(), Value::Integer(g));
                if kcal != n(r, "snapshot_kcal") || g != n(r, "snapshot_weight_g") {
                    trips.insert(s(r, "trip_id").to_string());
                }
            }
            write(db, "trip_meal", s(r, "sync_id"), values, false)?;
        }
    }
    for key in trips {
        let t = row(db, "trip", &key)?;
        let sum = rows(db, "trip_meal")?
            .iter()
            .filter(|r| s(r, "trip_id") == key)
            .map(|r| n(r, "snapshot_weight_g"))
            .sum();
        let nights = match t["nights"] {
            Value::Integer(v) => Some(v as i32),
            _ => None,
        };
        write(
            db,
            "trip",
            &key,
            fields([(
                "food_g_per_day",
                Value::Integer(p::trip_food_g_per_day(sum, nights)),
            )]),
            false,
        )?;
    }
    Ok(())
}
