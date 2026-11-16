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
    for trip in &a.campsite_days {
        sync_campsite_days(db, trip)?;
    }
    for trip in &a.derive_days {
        derive_days(db, trip)?;
    }
    for trip in &a.food_trips {
        sync_food(db, trip)?;
    }
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
pub(crate) fn derive_days(db: &Connection, trip: &str) -> Result<()> {
    let days = crate::trips::children(db, "trip_day", trip)?;
    let distances = days
        .iter()
        .filter_map(|r| crate::trips::numeric(r, "distance_mi"))
        .collect::<Vec<_>>();
    let gains = days
        .iter()
        .filter_map(|r| crate::trips::numeric(r, "elevation_gain_ft"))
        .collect::<Vec<_>>();
    let dist = if distances.is_empty() {
        Value::Null
    } else {
        text(
            strider_core::format::fixed(distances.iter().sum(), 2)
                .parse::<f64>()
                .unwrap_or(0.)
                .to_string(),
        )
    };
    write(
        db,
        "trip",
        trip,
        fields([
            ("distance_mi", dist),
            (
                "elevation_gain_ft",
                if gains.is_empty() {
                    Value::Null
                } else {
                    Value::Integer(gains.iter().sum::<f64>() as i64)
                },
            ),
            (
                "nights",
                if days.is_empty() {
                    Value::Null
                } else {
                    Value::Integer(days.len() as i64 - 1)
                },
            ),
        ]),
        false,
    )
}
pub(crate) fn sync_campsite_days(db: &Connection, trip: &str) -> Result<()> {
    use strider_core::route::*;
    let pts = crate::trips::points(db, trip)?
        .into_iter()
        .map(|p| RoutePt {
            d: p.d,
            e: p.e,
            lat: p.lat,
            lon: p.lon,
        })
        .collect::<Vec<_>>();
    let mut camps = crate::trips::children(db, "trip_campsite", trip)?;
    camps.sort_by(|a, b| {
        crate::trips::numeric(a, "distance_mi")
            .unwrap_or(0.)
            .total_cmp(&crate::trips::numeric(b, "distance_mi").unwrap_or(0.))
            .then_with(|| s(a, "sync_id").cmp(s(b, "sync_id")))
    });
    let camps = camps
        .iter()
        .map(|c| Campsite {
            distance_mi: crate::trips::numeric(c, "distance_mi").unwrap_or(0.),
            lat: crate::trips::numeric(c, "lat"),
            lon: crate::trips::numeric(c, "lon"),
        })
        .collect::<Vec<_>>();
    db.execute("DELETE FROM trip_day WHERE trip_id=?", [trip])?;
    for seg in compute_day_segments(&pts, &camps) {
        let key = id();
        write(
            db,
            "trip_day",
            &key,
            fields([
                ("trip_id", text(trip)),
                ("day_number", Value::Integer(seg.day as i64)),
                ("distance_mi", text(seg.distance_mi.to_string())),
                ("elevation_gain_ft", Value::Integer(seg.elevation_gain_ft)),
            ]),
            true,
        )?;
        db.execute(
            "INSERT INTO local_derived_day(sync_id,trip_id,day_number) VALUES(?,?,?)",
            params![key, trip, seg.day as i64],
        )?;
    }
    derive_days(db, trip)
}
pub(crate) fn sync_food(db: &Connection, trip: &str) -> Result<()> {
    let t = row(db, "trip", trip)?;
    let total = crate::trips::children(db, "trip_meal", trip)?
        .iter()
        .map(|r| n(r, "snapshot_weight_g"))
        .sum::<i64>();
    write(
        db,
        "trip",
        trip,
        fields([(
            "food_g_per_day",
            Value::Integer(strider_core::format::js_round(
                total as f64 / (n(&t, "nights") + 1).max(1) as f64,
            ) as i64),
        )]),
        false,
    )
}
