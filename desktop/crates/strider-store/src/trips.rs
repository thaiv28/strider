use super::*;
use strider_core::{
    format::{fixed, js_round},
    gpx::{self, ParsedGpx, Point},
};
#[derive(Clone, Debug)]
pub struct Site {
    pub distance_mi: f64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub ele_ft: Option<i32>,
}
#[derive(Clone, Debug)]
pub enum AddTripFood {
    Meal {
        meal: String,
        servings: f64,
    },
    Ingredient {
        ingredient: String,
        grams: f64,
    },
    Hit {
        ingredient: Box<IngredientInput>,
        grams: f64,
    },
}
pub(crate) fn editor(db: &Connection, trip: &str) -> Result<Row> {
    let t = row(db, "trip", trip)?;
    if s(&t, "access") == "view" {
        return invalid("Trip not found");
    }
    Ok(t)
}
fn child(db: &Connection, table: &str, key: &str, trip: &str) -> Result<Row> {
    editor(db, trip)?;
    let r = row(db, table, key)?;
    if s(&r, "trip_id") != trip {
        return invalid("Trip not found");
    }
    Ok(r)
}
pub(crate) fn children(db: &Connection, table: &str, trip: &str) -> Result<Vec<Row>> {
    Ok(rows(db, table)?
        .into_iter()
        .filter(|r| s(r, "trip_id") == trip)
        .collect())
}
fn clear(db: &Connection, table: &str, trip: &str) -> Result<()> {
    db.execute(&format!("DELETE FROM {table} WHERE trip_id=?"), [trip])?;
    Ok(())
}
fn finite(x: f64) -> Result<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        invalid("Enter a number.")
    }
}
fn integer(x: f64) -> Result<Value> {
    let x = js_round(finite(x)?);
    if x < i32::MIN as f64 || x > i32::MAX as f64 {
        return invalid("Value exceeds the server integer range");
    }
    Ok(Value::Integer(x as i64))
}
pub(crate) fn numeric(r: &Row, k: &str) -> Option<f64> {
    match r.get(k) {
        Some(Value::Text(v)) => v.parse().ok(),
        Some(Value::Integer(v)) => Some(*v as f64),
        Some(Value::Real(v)) => Some(*v),
        _ => None,
    }
}
fn num(x: f64) -> Result<Value> {
    Ok(text(finite(x)?.to_string()))
}
fn optnum(x: Option<f64>) -> Result<Value> {
    x.map(num).transpose().map(|v| v.unwrap_or(Value::Null))
}
fn next_order(db: &Connection, table: &str, trip: &str, day: Option<i32>) -> Result<i64> {
    Ok(children(db, table, trip)?
        .iter()
        .filter(|r| day.is_none_or(|d| n(r, "day_number") == i64::from(d)))
        .map(|r| n(r, "sort_order"))
        .max()
        .unwrap_or(-1)
        + 1)
}
fn allowed_patch(patch: &Row, allowed: &str) -> Result<()> {
    if patch
        .keys()
        .any(|k| !allowed.split_whitespace().any(|a| a == k))
    {
        return invalid("Invalid trip field");
    }
    Ok(())
}
fn add_gear(db: &Connection, trip: &str, gear: &str, class: Option<&str>) -> Result<()> {
    let g = row(db, "gear_item", gear)?;
    if children(db, "trip_gear", trip)?
        .iter()
        .any(|r| s(r, "gear_item_id") == gear)
    {
        return Ok(());
    }
    let cat = opt(&g, "category_id")
        .map(|c| row(db, "gear_category", &c))
        .transpose()?
        .and_then(|r| opt(&r, "name"));
    let class = class.unwrap_or(if s(&g, "default_weight_class") == "worn" {
        "worn"
    } else {
        "base"
    });
    write(
        db,
        "trip_gear",
        &id(),
        fields([
            ("trip_id", text(trip)),
            ("gear_item_id", text(gear)),
            ("snapshot_name", g["name"].clone()),
            ("snapshot_weight_g", Value::Integer(n(&g, "weight_g"))),
            ("snapshot_category", nullable(cat)),
            ("quantity", g["quantity"].clone()),
            ("weight_class", text(class)),
            (
                "sort_order",
                Value::Integer(next_order(db, "trip_gear", trip, None)?),
            ),
        ]),
        true,
    )
}
fn seed(db: &Connection, trip: &str, loadout: &str) -> Result<()> {
    row(db, "loadout", loadout)?;
    if !children(db, "trip_gear", trip)?.is_empty() {
        return Ok(());
    }
    let mut stmt=db.prepare("SELECT li.gear_item_id,li.weight_class FROM loadout_item li JOIN gear_item g ON g.sync_id=li.gear_item_id LEFT JOIN gear_category c ON c.sync_id=g.category_id WHERE li.loadout_id=? ORDER BY c.sort_order,g.sort_order,g.sync_id")?;
    let members = stmt
        .query_map([loadout], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (g, c) in members {
        add_gear(db, trip, &g, Some(&c))?
    }
    Ok(())
}
fn site_values(site: Site) -> Result<Row> {
    Ok(fields([
        ("distance_mi", num(site.distance_mi)?),
        ("lat", optnum(site.lat)?),
        ("lon", optnum(site.lon)?),
        ("ele_ft", number(site.ele_ft)),
    ]))
}
pub(crate) fn points(db: &Connection, trip: &str) -> Result<Vec<Point>> {
    let gpx = children(db, "trip_gpx", trip)?;
    let Some(raw) = gpx.first().and_then(|r| opt(r, "points")) else {
        return Ok(vec![]);
    };
    serde_json::from_str(&raw).map_err(|_| Error::Invalid("Invalid GPX points".into()))
}

fn json(v: impl serde::Serialize) -> Result<Value> {
    serde_json::to_string(&v)
        .map(text)
        .map_err(|e| Error::Invalid(e.to_string()))
}
fn gpx_values(g: ParsedGpx) -> Result<Row> {
    Ok(fields([
        ("start_lat", num(g.start_lat)?),
        ("start_lon", num(g.start_lon)?),
        ("distance_mi", num(g.distance_mi)?),
        ("elevation_gain_ft", integer(g.elevation_gain_ft)?),
        (
            "min_ele_ft",
            g.min_ele_ft
                .map(integer)
                .transpose()?
                .unwrap_or(Value::Null),
        ),
        (
            "max_ele_ft",
            g.max_ele_ft
                .map(integer)
                .transpose()?
                .unwrap_or(Value::Null),
        ),
        ("track", json(g.track)?),
        ("profile", json(g.profile)?),
        ("points", json(g.points)?),
    ]))
}
fn attach_gpx(db: &Connection, trip: &str, filename: String, g: ParsedGpx) -> Result<()> {
    clear(db, "trip_gpx", trip)?;
    let mut v = gpx_values(g)?;
    v.insert("filename".into(), text(filename));
    v.insert("trip_id".into(), text(trip));
    write(db, "trip_gpx", &id(), v, true)
}
pub(crate) fn run(
    db: &Connection,
    data_dir: &Path,
    cmd: Command,
    a: &mut commands::Affected,
) -> Result<()> {
    use Command::*;
    a.trip_command = true;
    match cmd {
        CreateTrip {
            name,
            trail,
            loadout,
        } => {
            let key = id();
            let mut v = fields([(
                "name",
                text(if name.trim().is_empty() {
                    "New Trip"
                } else {
                    name.trim()
                }),
            )]);
            if let Some(t) = trail {
                row(db, "trail", &t)?;
                v.insert("trail_id".into(), text(t));
            }
            write(db, "trip", &key, v, true)?;
            if let Some(l) = loadout {
                seed(db, &key, &l)?
            }
        }
        UpdateTrip { trip, mut patch } => {
            let t = editor(db, &trip)?;
            allowed_patch(
                &patch,
                "name status start_date nights party_size grocery_people grocery_removed region area_type lat lon distance_mi elevation_gain_ft food_g_per_day water_g_per_day fuel_g_per_day trailhead permit_required permit_notes driving_notes water_sources planning_notes trip_report",
            )?;
            if s(&t, "access") != "owner" && patch.contains_key("trip_report") {
                return invalid("Private notes are owner-only");
            }
            for k in ["lat", "lon"] {
                if patch.get(k).is_some_and(|v| *v != Value::Null)
                    && numeric(&patch, k).is_none_or(|n| !n.is_finite())
                {
                    patch.remove(k);
                }
            }
            patch.insert("updated_at".into(), text(timestamp(db)?));
            write(db, "trip", &trip, patch, false)?;
        }
        DeleteTrip { trip } => {
            let t = editor(db, &trip)?;
            if s(&t, "access") != "owner" {
                return invalid("Trip not found");
            }
            delete(db, "trip", &trip)?
        }
        ImportTrip { json } => import(db, &json, a)?,
        SeedTripFromLoadout { trip, loadout } => {
            editor(db, &trip)?;
            seed(db, &trip, &loadout)?
        }
        AddTripGear { trip, gear } => {
            editor(db, &trip)?;
            add_gear(db, &trip, &gear, None)?
        }
        RemoveTripGear { trip, id } => {
            child(db, "trip_gear", &id, &trip)?;
            delete(db, "trip_gear", &id)?
        }
        RestoreTripGear { trip, mut snapshot } => {
            editor(db, &trip)?;
            allowed_patch(
                &snapshot,
                "gear_item_id snapshot_name snapshot_weight_g snapshot_category weight_class quantity packed",
            )?;
            if let Some(g) = opt(&snapshot, "gear_item_id") {
                if row(db, "gear_item", &g).is_err() {
                    snapshot.insert("gear_item_id".into(), Value::Null);
                } else if children(db, "trip_gear", &trip)?
                    .iter()
                    .any(|r| s(r, "gear_item_id") == g)
                {
                    return Ok(());
                }
            }
            snapshot.insert("trip_id".into(), text(&trip));
            snapshot.insert(
                "sort_order".into(),
                Value::Integer(next_order(db, "trip_gear", &trip, None)?),
            );
            write(db, "trip_gear", &id(), snapshot, true)?
        }
        SetTripGearClass { trip, id, class } => {
            child(db, "trip_gear", &id, &trip)?;
            if !matches!(class.as_str(), "base" | "worn") {
                return invalid("Invalid weight class");
            }
            write(
                db,
                "trip_gear",
                &id,
                fields([("weight_class", text(class))]),
                false,
            )?
        }
        SetTripGearQty { trip, id, qty } => {
            child(db, "trip_gear", &id, &trip)?;
            write(
                db,
                "trip_gear",
                &id,
                fields([("quantity", integer(finite(qty)?.max(1.))?)]),
                false,
            )?
        }
        SetTripGearPacked { trip, id, packed } => {
            child(db, "trip_gear", &id, &trip)?;
            write(
                db,
                "trip_gear",
                &id,
                fields([("packed", Value::Integer(packed.into()))]),
                false,
            )?
        }
        ClearTripGear { trip } => {
            editor(db, &trip)?;
            clear(db, "trip_gear", &trip)?
        }
        MoveTripGear {
            trip,
            id,
            category,
            ordered_ids,
        } => {
            child(db, "trip_gear", &id, &trip)?;
            for k in &ordered_ids {
                child(db, "trip_gear", k, &trip)?;
            }
            write(
                db,
                "trip_gear",
                &id,
                fields([("snapshot_category", text(&category))]),
                false,
            )?;
            let mut all = children(db, "trip_gear", &trip)?;
            all.sort_by_key(|r| (n(r, "sort_order"), s(r, "sync_id").to_string()));
            let seen: BTreeSet<_> = ordered_ids.iter().cloned().collect();
            let mut order = ordered_ids;
            order.extend(
                all.iter()
                    .filter(|r| {
                        s(r, "snapshot_category") == category && !seen.contains(s(r, "sync_id"))
                    })
                    .map(|r| s(r, "sync_id").to_string()),
            );
            for (i, k) in order.iter().enumerate() {
                write(
                    db,
                    "trip_gear",
                    k,
                    fields([("sort_order", Value::Integer(i as i64))]),
                    false,
                )?
            }
        }
        SaveTripPackingList { trip, packing_list } => {
            editor(db, &trip)?;
            write(
                db,
                "trip",
                &trip,
                fields([
                    ("packing_list", text(packing_list)),
                    ("updated_at", text(timestamp(db)?)),
                ]),
                false,
            )?
        }
        UpsertTripDay {
            trip,
            day,
            distance_mi,
            elevation_gain_ft,
        } => {
            editor(db, &trip)?;
            let ex = children(db, "trip_day", &trip)?
                .into_iter()
                .find(|r| n(r, "day_number") == day as i64);
            let mut v = Row::new();
            if let Some(d) = distance_mi {
                v.insert("distance_mi".into(), nullable(d));
            }
            if let Some(g) = elevation_gain_ft {
                v.insert("elevation_gain_ft".into(), number(g));
            }
            let key = ex
                .as_ref()
                .map(|r| s(r, "sync_id").to_string())
                .unwrap_or_else(crate::id);
            if ex.is_none() {
                v.insert("trip_id".into(), text(&trip));
                v.insert("day_number".into(), Value::Integer(day.into()));
            }
            db.execute(
                "UPDATE local_derived_day SET edited=1 WHERE sync_id=?",
                [&key],
            )?;
            write(db, "trip_day", &key, v, ex.is_none())?;
            a.derive_days.insert(trip);
        }
        AddTripDay { trip } => {
            editor(db, &trip)?;
            let next = children(db, "trip_day", &trip)?
                .iter()
                .map(|r| n(r, "day_number"))
                .max()
                .unwrap_or(0)
                .max(0)
                + 1;
            write(
                db,
                "trip_day",
                &id(),
                fields([
                    ("trip_id", text(&trip)),
                    ("day_number", Value::Integer(next)),
                ]),
                true,
            )?;
            a.derive_days.insert(trip);
        }
        DeleteTripDay { trip, id } => {
            child(db, "trip_day", &id, &trip)?;
            db.execute(
                "UPDATE local_derived_day SET edited=1 WHERE sync_id=?",
                [&id],
            )?;
            delete(db, "trip_day", &id)?;
            let mut days = children(db, "trip_day", &trip)?;
            days.sort_by_key(|r| (n(r, "day_number"), s(r, "sync_id").to_string()));
            for (i, d) in days.iter().enumerate() {
                write(
                    db,
                    "trip_day",
                    s(d, "sync_id"),
                    fields([("day_number", Value::Integer(i as i64 + 1))]),
                    false,
                )?
            }
            a.derive_days.insert(trip);
        }
        AddCampsite { trip, site } => {
            editor(db, &trip)?;
            let mut v = site_values(site)?;
            v.insert("trip_id".into(), text(&trip));
            write(db, "trip_campsite", &id(), v, true)?;
            a.campsite_days.insert(trip);
        }
        MoveCampsite { trip, id, site } => {
            child(db, "trip_campsite", &id, &trip)?;
            write(db, "trip_campsite", &id, site_values(site)?, false)?;
            a.campsite_days.insert(trip);
        }
        RemoveCampsite { trip, id } => {
            child(db, "trip_campsite", &id, &trip)?;
            delete(db, "trip_campsite", &id)?;
            a.campsite_days.insert(trip);
        }
        ClearCampsites { trip } => {
            editor(db, &trip)?;
            clear(db, "trip_campsite", &trip)?;
            a.campsite_days.insert(trip);
        }
        AttachGpx {
            trip,
            filename,
            bytes,
        } => {
            editor(db, &trip)?;
            if bytes.is_empty() {
                return invalid("No file selected.");
            }
            let g = gpx::parse_gpx(&String::from_utf8_lossy(&bytes)).ok_or_else(|| {
                Error::Invalid("Couldn't read track points from that GPX.".into())
            })?;
            attach_gpx(db, &trip, filename, g)?
        }
        ReverseGpx { trip } => {
            editor(db, &trip)?;
            let mut pts = points(db, &trip)?;
            if pts.len() < 2 {
                return Ok(());
            }
            let total = pts[pts.len() - 1].d;
            let flip = |d: f64| fixed((total - d).max(0.), 3).parse::<f64>().unwrap_or(0.);
            let g = children(db, "trip_gpx", &trip)?.remove(0);
            let gain = numeric(&g, "elevation_gain_ft")
                .map(|v| (v + pts[0].e - pts[pts.len() - 1].e).max(0.));
            pts.reverse();
            for p in &mut pts {
                p.d = flip(p.d)
            }
            write(
                db,
                "trip_gpx",
                s(&g, "sync_id"),
                fields([
                    ("start_lat", num(pts[0].lat)?),
                    ("start_lon", num(pts[0].lon)?),
                    (
                        "elevation_gain_ft",
                        gain.map(integer).transpose()?.unwrap_or(Value::Null),
                    ),
                    (
                        "track",
                        json(pts.iter().map(|p| [p.lat, p.lon]).collect::<Vec<_>>())?,
                    ),
                    (
                        "profile",
                        json(
                            pts.iter()
                                .map(|p| gpx::ProfilePoint { d: p.d, e: p.e })
                                .collect::<Vec<_>>(),
                        )?,
                    ),
                    ("points", json(pts)?),
                ]),
                false,
            )?;
            for c in children(db, "trip_campsite", &trip)? {
                write(
                    db,
                    "trip_campsite",
                    s(&c, "sync_id"),
                    fields([(
                        "distance_mi",
                        num(flip(numeric(&c, "distance_mi").unwrap_or(0.)))?,
                    )]),
                    false,
                )?
            }
            a.campsite_days.insert(trip);
        }
        RemoveGpx { trip } => {
            editor(db, &trip)?;
            clear(db, "trip_gpx", &trip)?
        }
        RestoreGpx { trip, mut snapshot } => {
            editor(db, &trip)?;
            allowed_patch(
                &snapshot,
                "filename start_lat start_lon distance_mi elevation_gain_ft min_ele_ft max_ele_ft track profile points",
            )?;
            clear(db, "trip_gpx", &trip)?;
            snapshot.insert("trip_id".into(), text(trip));
            write(db, "trip_gpx", &id(), snapshot, true)?
        }
        AttachPermit {
            trip,
            filename,
            mime_type,
            bytes,
        } => {
            editor(db, &trip)?;
            if bytes.is_empty() {
                return invalid("No file.");
            }
            if ![
                "application/pdf",
                "image/png",
                "image/jpeg",
                "image/webp",
                "image/heic",
            ]
            .contains(&mime_type.as_str())
            {
                return invalid("Use a PDF or image (PNG/JPG/WEBP).");
            }
            if bytes.len() > PERMIT_MAX_BYTES {
                return invalid("File exceeds 15 MB.");
            }
            if filename.trim().is_empty()
                || filename.chars().count() > 255
                || filename.contains(['\r', '\n'])
            {
                return invalid("invalid_file");
            }
            db.execute("DELETE FROM permit_delete WHERE trip_id=?", [&trip])?;
            let hash = blob_hash(&bytes);
            db.execute("INSERT INTO permit_upload(trip_id,sha256) VALUES(?,?) ON CONFLICT(trip_id) DO UPDATE SET sha256=excluded.sha256",params![trip,hash])?;
            let path = blobs::save_blob(data_dir, &hash, &bytes)?;
            db.execute("INSERT INTO blob_ref(sha256,state,path) VALUES(?,'local_only',?) ON CONFLICT(sha256) DO UPDATE SET state='local_only',path=excluded.path",params![hash,path.to_string_lossy()])?;
            let ex = children(db, "trip_permit", &trip)?.into_iter().next();
            let key = ex
                .as_ref()
                .map(|r| s(r, "sync_id").to_string())
                .unwrap_or_else(crate::id);
            write(
                db,
                "trip_permit",
                &key,
                fields([
                    ("trip_id", text(trip)),
                    ("filename", text(filename)),
                    ("mime_type", text(mime_type)),
                    ("size_bytes", Value::Integer(bytes.len() as i64)),
                    ("sha256", text(hash)),
                    ("uploaded_at", text(timestamp(db)?)),
                ]),
                ex.is_none(),
            )?;
        }
        DeletePermit { trip } => {
            editor(db, &trip)?;
            db.execute(
                "INSERT OR IGNORE INTO permit_delete(trip_id) VALUES(?)",
                [&trip],
            )?;
            clear(db, "trip_permit", &trip)?;
            db.execute("DELETE FROM permit_upload WHERE trip_id=?", [trip])?;
        }
        AddTripMeal { trip, day, entry } => {
            editor(db, &trip)?;
            add_food(db, &trip, day, entry)?;
            a.food_trips.insert(trip);
        }
        SetTripMealQty { trip, id, qty } => {
            let r = child(db, "trip_meal", &id, &trip)?;
            let qty = finite(qty)?;
            let mut v = Row::new();
            let ratio = if opt(&r, "meal_id").is_some() {
                let old = numeric(&r, "servings").filter(|v| *v != 0.).unwrap_or(1.);
                v.insert("servings".into(), num(qty.max(1.))?);
                (if qty == 0. { 1. } else { qty }) / old
            } else {
                qty.max(0.)
                    / (if n(&r, "snapshot_weight_g") == 0 {
                        1
                    } else {
                        n(&r, "snapshot_weight_g")
                    } as f64)
            };
            v.insert(
                "snapshot_weight_g".into(),
                integer(if opt(&r, "meal_id").is_some() {
                    n(&r, "snapshot_weight_g") as f64 * ratio
                } else {
                    qty.max(0.)
                })?,
            );
            v.insert(
                "snapshot_kcal".into(),
                integer(n(&r, "snapshot_kcal") as f64 * ratio)?,
            );
            write(db, "trip_meal", &id, v, false)?;
            a.food_trips.insert(trip);
        }
        RemoveTripMeal { trip, id } => {
            child(db, "trip_meal", &id, &trip)?;
            delete(db, "trip_meal", &id)?;
            a.food_trips.insert(trip);
        }
        CopyTripMealToAll { trip, id } => {
            let r = child(db, "trip_meal", &id, &trip)?;
            let t = row(db, "trip", &trip)?;
            let count = (children(db, "trip_day", &trip)?.len() as i64)
                .max(n(&t, "nights") + 1)
                .max(n(&r, "day_number"))
                .max(1);
            for k in 1..=count {
                if k == n(&r, "day_number") {
                    continue;
                }
                let mut v = r.clone();
                v.remove("sync_id");
                v.insert("day_number".into(), Value::Integer(k));
                v.insert(
                    "sort_order".into(),
                    Value::Integer(next_order(db, "trip_meal", &trip, Some(k as i32))?),
                );
                write(db, "trip_meal", &crate::id(), v, true)?
            }
            a.food_trips.insert(trip);
        }
        _ => return invalid("Not a trip command"),
    }
    Ok(())
}
fn timestamp(db: &Connection) -> Result<String> {
    Ok(
        db.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
            r.get(0)
        })?,
    )
}
fn add_food(db: &Connection, trip: &str, day: i32, entry: AddTripFood) -> Result<()> {
    let (name, kcal, weight, servings, meal_id, ingredient_id) = match entry {
        AddTripFood::Meal { meal, servings } => {
            let m = row(db, "meal", &meal)?;
            let lines = rows(db, "meal_ingredient")?
                .into_iter()
                .filter(|r| s(r, "meal_id") == meal)
                .collect::<Vec<_>>();
            let g = lines.iter().map(|r| n(r, "amount_g") as f64).sum::<f64>();
            let kcal = lines
                .iter()
                .map(|r| {
                    js_round(
                        numeric(r, "snapshot_kcal_per_100g").unwrap_or(0.) / 100.
                            * n(r, "amount_g") as f64,
                    )
                })
                .sum::<f64>();
            let scale = finite(servings)? / (n(&m, "base_servings").max(1) as f64);
            (
                s(&m, "name").to_string(),
                kcal * scale,
                g * scale,
                servings,
                Some(meal),
                None,
            )
        }
        AddTripFood::Ingredient { ingredient, grams } => {
            let i = row(db, "ingredient", &ingredient)?;
            (
                s(&i, "name").to_string(),
                strider_core::food::kcal_for_grams(numeric(&i, "kcal_per_100g"), finite(grams)?)
                    as f64,
                grams,
                1.,
                None,
                Some(ingredient),
            )
        }
        AddTripFood::Hit { ingredient, grams } => {
            let existing = rows(db, "ingredient")?.into_iter().find(|r| {
                opt(r, "source") == ingredient.source
                    && opt(r, "source_id") == ingredient.source_id
                    && ingredient.source.is_some()
                    && ingredient.source_id.is_some()
            });
            let key = if let Some(r) = existing {
                s(&r, "sync_id").to_string()
            } else {
                let key = id();
                write(
                    db,
                    "ingredient",
                    &key,
                    fields([
                        ("name", text(ingredient.name)),
                        (
                            "kcal_per_100g",
                            ingredient
                                .kcal_per_100g
                                .map(|v| text(js_round(v.as_f64()).to_string()))
                                .unwrap_or(Value::Null),
                        ),
                        ("density_g_ml", decimal(ingredient.density_g_ml)),
                        ("density_source", nullable(ingredient.density_source)),
                        ("reference_name", nullable(ingredient.reference_name)),
                        ("default_serving_g", number(ingredient.default_serving_g)),
                        ("category", nullable(ingredient.category)),
                        ("source", nullable(ingredient.source)),
                        ("source_id", nullable(ingredient.source_id)),
                        ("notes", nullable(ingredient.notes)),
                    ]),
                    true,
                )?;
                key
            };
            return add_food(
                db,
                trip,
                day,
                AddTripFood::Ingredient {
                    ingredient: key,
                    grams,
                },
            );
        }
    };
    write(
        db,
        "trip_meal",
        &id(),
        fields([
            ("trip_id", text(trip)),
            ("day_number", Value::Integer(day.into())),
            ("meal_id", nullable(meal_id)),
            ("ingredient_id", nullable(ingredient_id)),
            ("servings", num(servings)?),
            ("snapshot_name", text(name)),
            ("snapshot_kcal", integer(kcal)?),
            ("snapshot_weight_g", integer(weight)?),
            (
                "sort_order",
                Value::Integer(next_order(db, "trip_meal", trip, Some(day))?),
            ),
        ]),
        true,
    )
}
fn import(db: &Connection, input: &str, a: &mut commands::Affected) -> Result<()> {
    use serde_json::Value as J;
    let input = if let Some(start) = input.find("```") {
        let tail = &input[start + 3..];
        let tail = tail
            .strip_prefix("json")
            .or_else(|| tail.strip_prefix("JSON"))
            .unwrap_or(tail);
        tail.split("```").next().unwrap_or(tail)
    } else {
        input
    };
    let body = input
        .find('{')
        .zip(input.rfind('}'))
        .and_then(|(a, b)| input.get(a..=b))
        .unwrap_or(input)
        .trim();
    let data: J = serde_json::from_str(body).map_err(|_| {
        Error::Invalid(
            "Couldn't find valid JSON. Paste the JSON code block the agent produced.".into(),
        )
    })?;
    let t = &data["trip"];
    let name = t["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::Invalid("The file needs a trip.name field.".into()))?;
    let trip = id();
    let mut v = fields([(
        "name",
        text(name.trim().chars().take(200).collect::<String>()),
    )]);
    for (js, sql) in [
        ("startDate", "start_date"),
        ("region", "region"),
        ("areaType", "area_type"),
        ("trailhead", "trailhead"),
        ("permitNotes", "permit_notes"),
        ("drivingNotes", "driving_notes"),
        ("waterSources", "water_sources"),
    ] {
        if let Some(x) = t[js].as_str() {
            v.insert(sql.into(), text(x));
        }
    }
    if let Some(x) = t["nights"].as_f64() {
        v.insert("nights".into(), integer(x)?);
    }
    if let Some(x) = t["permitRequired"].as_bool() {
        v.insert("permit_required".into(), Value::Integer(x.into()));
    }
    if let Some(x) = t["distanceMi"].as_f64() {
        v.insert("distance_mi".into(), num(x)?);
    } else if let Some(x) = t["distanceMi"].as_str() {
        v.insert("distance_mi".into(), text(x));
    }
    if let Some(x) = t["elevationGainFt"].as_f64() {
        v.insert("elevation_gain_ft".into(), integer(x)?);
    }
    write(db, "trip", &trip, v, true)?;
    let route = &data["route"];
    let mut parsed = None;
    if let Some(track) = route["track"].as_array().filter(|v| v.len() > 1) {
        let mut xml = String::from("<gpx>");
        for p in track {
            if let (Some(lat), Some(lon)) = (p[0].as_f64(), p[1].as_f64()) {
                xml.push_str(&format!("<trkpt lat=\"{lat}\" lon=\"{lon}\">"));
                if let Some(e) = p[2].as_f64() {
                    xml.push_str(&format!("<ele>{e}</ele>"));
                }
                xml.push_str("</trkpt>");
            }
        }
        xml.push_str("</gpx>");
        parsed = gpx::parse_gpx(&xml);
    }
    if let Some(g) = parsed.clone() {
        attach_gpx(
            db,
            &trip,
            route["filename"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.gpx", name.trim())),
            g,
        )?
    }
    let camps = data["campsites"]
        .as_array()
        .map(|cs| {
            cs.iter()
                .filter(|c| c["lat"].as_f64().is_some() && c["lon"].as_f64().is_some())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let total = numeric(&row(db, "trip", &trip)?, "distance_mi")
        .or_else(|| parsed.as_ref().map(|g| g.distance_mi))
        .unwrap_or(0.);
    for (i, c) in camps.iter().enumerate() {
        let (lat, lon) = (
            c["lat"].as_f64().unwrap_or(0.),
            c["lon"].as_f64().unwrap_or(0.),
        );
        let nearest = parsed.as_ref().and_then(|g| {
            g.points.iter().min_by(|a, b| {
                ((a.lat - lat).powi(2) + (a.lon - lon).powi(2))
                    .total_cmp(&((b.lat - lat).powi(2) + (b.lon - lon).powi(2)))
            })
        });
        let distance = nearest.map(|p| p.d).unwrap_or(if total != 0. {
            (i + 1) as f64 / (camps.len() + 1) as f64 * total
        } else {
            (i + 1) as f64
        });
        let ele = c["eleFt"].as_f64().or_else(|| nearest.map(|p| p.e));
        let mut v = site_values(Site {
            distance_mi: distance,
            lat: Some(lat),
            lon: Some(lon),
            ele_ft: ele.map(|x| js_round(x) as i32),
        })?;
        v.insert("trip_id".into(), text(&trip));
        write(db, "trip_campsite", &id(), v, true)?;
    }
    if !camps.is_empty() && parsed.is_some() {
        // Imported day rows are part of the atomic import; retain researched gain for elevation-less tracks.
        crate::propagation::sync_campsite_days(db, &trip)?;
        if numeric(&row(db, "trip", &trip)?, "elevation_gain_ft").unwrap_or(0.) == 0.
            && let Some(g) = t["elevationGainFt"].as_f64()
        {
            write(
                db,
                "trip",
                &trip,
                fields([("elevation_gain_ft", integer(g)?)]),
                false,
            )?;
        }
    }
    let _ = a;
    Ok(())
}
