use super::*;
use crate::trips::{children, numeric, points};
use strider_core::{energy::EnergyParams, gear::WeightClass, route::*, trip::*};
/// Web report packing fallback, used only when neither Trip nor settings overrides it.
pub const DEFAULT_PACKING_LIST: &str = "**Shelter & sleep**\n- Tent or share (let me know)\n- Sleeping bag rated to 20°F\n- Sleeping pad\n\n**Cooking & water**\n- Stove + fuel\n- Pot, spork, mug\n- Water filter or treatment\n- 2 L water capacity\n\n**Wear & carry**\n- 40–60 L pack\n- Rain shell + insulating layer\n- Headlamp + spare batteries\n- First-aid & blister kit\n";
#[derive(Clone, Debug)]
pub struct TripDetail {
    pub fields: Row,
    pub trail: Option<Row>,
    pub access: String,
    pub route: RouteStats,
    pub region: Option<String>,
    pub area_type: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub days: Vec<Row>,
    pub segments: Vec<DaySegment>,
    pub campsites: Vec<Row>,
    pub gpx: Option<Row>,
    pub points: Vec<strider_core::gpx::Point>,
    pub gear: Vec<Row>,
    pub groups: Vec<(String, Vec<Row>)>,
    pub packed_count: usize,
    pub weights: TripWeights,
    pub food: TripFood,
    pub shopping: Vec<ShoppingItem>,
    pub cook_water_ml_per_person: i64,
    pub permit: Option<PermitBlob>,
    pub packing_list: Option<String>,
    pub consumables: Vec<Row>,
    /// The owner's live view-only and edit link tokens (None for collaborators or when no link exists).
    pub share_token: Option<String>,
    pub edit_token: Option<String>,
}
#[derive(Clone, Debug)]
pub struct TripSummary {
    pub sync_id: String,
    pub name: String,
    pub status: String,
    pub start_date: Option<String>,
    pub region: Option<String>,
    pub nights: Option<i32>,
    pub route: RouteStats,
    pub base_g: i64,
    pub skin_out_g: i64,
    pub track: Option<Vec<[f64; 2]>>,
    pub access: String,
}
#[derive(Clone, Debug, Default)]
pub struct TripsList {
    pub upcoming: Vec<TripSummary>,
    pub in_progress: Vec<TripSummary>,
    pub past: Vec<TripSummary>,
}
fn stats(r: &Row, trail: bool) -> RouteStats {
    RouteStats {
        distance_mi: numeric(
            r,
            if trail {
                "typical_distance_mi"
            } else {
                "distance_mi"
            },
        ),
        elevation_ft: numeric(
            r,
            if trail {
                "typical_elevation_ft"
            } else {
                "elevation_gain_ft"
            },
        )
        .map(|n| n as i64),
    }
}
fn optional_i32(r: &Row, k: &str) -> Option<i32> {
    numeric(r, k).map(|n| n as i32)
}
fn weights_for(t: &Row, gear: &[Row]) -> TripWeights {
    trip_weights(
        &gear
            .iter()
            .map(|g| TripGear {
                weight_g: n(g, "snapshot_weight_g"),
                quantity: n(g, "quantity"),
                weight_class: if s(g, "weight_class") == "worn" {
                    WeightClass::Worn
                } else {
                    WeightClass::Base
                },
                packed: n(g, "packed") != 0,
            })
            .collect::<Vec<_>>(),
        optional_i32(t, "nights"),
        n(t, "food_g_per_day"),
        n(t, "water_g_per_day"),
        n(t, "fuel_g_per_day"),
    )
}
impl Store {
    pub fn trips_list(&self) -> Result<TripsList> {
        let today: String =
            self.connection
                .query_row("SELECT date('now','localtime')", [], |r| r.get(0))?;
        self.trips_list_on(&today)
    }
    /// Explicit local civil date for deterministic grouping and tests.
    pub fn trips_list_on(&self, today: &str) -> Result<TripsList> {
        let mut trips = rows(&self.connection, "trip")?;
        trips.sort_by(|a, b| {
            opt(b, "start_date")
                .cmp(&opt(a, "start_date"))
                .then_with(|| s(a, "sync_id").cmp(s(b, "sync_id")))
        });
        // List summaries do not load food, permit bytes, or full GPX profiles.
        let trails = rows(&self.connection, "trail")?
            .into_iter()
            .map(|r| (s(&r, "sync_id").to_owned(), r))
            .collect::<BTreeMap<_, _>>();
        let gpx = rows(&self.connection, "trip_gpx")?
            .into_iter()
            .map(|r| (s(&r, "trip_id").to_owned(), r))
            .collect::<BTreeMap<_, _>>();
        let mut gear = BTreeMap::<String, Vec<Row>>::new();
        for g in rows(&self.connection, "trip_gear")? {
            gear.entry(s(&g, "trip_id").into()).or_default().push(g);
        }
        let mut out = TripsList::default();
        for t in trips {
            let key = s(&t, "sync_id");
            let trail = opt(&t, "trail_id").and_then(|id| trails.get(&id));
            let g = gpx.get(key);
            let weights = weights_for(&t, gear.get(key).map(Vec::as_slice).unwrap_or(&[]));
            let track = g
                .and_then(|g| opt(g, "track"))
                .map(|raw| {
                    serde_json::from_str(&raw)
                        .map_err(|_| Error::Invalid("Invalid GPX track".into()))
                })
                .transpose()?;
            let detail = TripSummary {
                sync_id: key.into(),
                name: s(&t, "name").into(),
                status: s(&t, "status").into(),
                start_date: opt(&t, "start_date"),
                region: opt(&t, "region").or_else(|| trail.and_then(|r| opt(r, "region"))),
                nights: optional_i32(&t, "nights"),
                route: effective_route_stats(
                    stats(&t, false),
                    g.map(|r| stats(r, false)),
                    trail.map(|r| stats(r, true)),
                ),
                base_g: weights.base,
                skin_out_g: weights.skin_out,
                track,
                access: s(&t, "access").into(),
            };
            let start = opt(&t, "start_date");
            let end: Option<String> = if let Some(d) = &start {
                self.connection.query_row(
                    "SELECT date(?,printf('%+d days',?))",
                    params![d, n(&t, "nights")],
                    |r| r.get(0),
                )?
            } else {
                None
            };
            if s(&t, "status") == "completed" || end.as_deref().is_some_and(|d| d < today) {
                out.past.push(detail)
            } else if start.as_deref().is_some_and(|d| d <= today) {
                out.in_progress.push(detail)
            } else {
                out.upcoming.push(detail)
            }
        }
        Ok(out)
    }
    pub fn trip_detail(&self, key: &str) -> Result<Option<TripDetail>> {
        let db = &self.connection;
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM trip WHERE sync_id=?)",
            [key],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(None);
        }
        let mut t = row(db, "trip", key)?;
        let access = s(&t, "access").to_string();
        if access != "owner" {
            t.insert("trip_report".into(), Value::Null);
        }
        let token = |v: Option<Value>| match v {
            Some(Value::Text(s)) if access == "owner" && !s.is_empty() => Some(s),
            _ => None,
        };
        let share_token = token(t.remove("share_token"));
        let edit_token = token(t.remove("edit_token"));
        let trail = opt(&t, "trail_id")
            .map(|id| row(db, "trail", &id))
            .transpose()?;
        let gpx = children(db, "trip_gpx", key)?.into_iter().next();
        let pts = points(db, key)?;
        let route = effective_route_stats(
            stats(&t, false),
            gpx.as_ref().map(|r| stats(r, false)),
            trail.as_ref().map(|r| stats(r, true)),
        );
        let mut days = children(db, "trip_day", key)?;
        days.sort_by_key(|r| n(r, "day_number"));
        let mut camps = children(db, "trip_campsite", key)?;
        camps.sort_by(|a, b| {
            numeric(a, "distance_mi")
                .unwrap_or(0.)
                .total_cmp(&numeric(b, "distance_mi").unwrap_or(0.))
                .then_with(|| s(a, "sync_id").cmp(s(b, "sync_id")))
        });
        let segments = compute_day_segments(
            &pts.iter()
                .map(|p| RoutePt {
                    d: p.d,
                    lat: p.lat,
                    lon: p.lon,
                    e: p.e,
                })
                .collect::<Vec<_>>(),
            &camps
                .iter()
                .map(|c| Campsite {
                    distance_mi: numeric(c, "distance_mi").unwrap_or(0.),
                    lat: numeric(c, "lat"),
                    lon: numeric(c, "lon"),
                })
                .collect::<Vec<_>>(),
        );
        for (i, c) in camps.iter_mut().enumerate() {
            c.insert("night".into(), Value::Integer(i as i64 + 1));
        }
        let mut gear = children(db, "trip_gear", key)?;
        gear.sort_by(|a, b| {
            n(a, "sort_order")
                .cmp(&n(b, "sort_order"))
                .then_with(|| s(a, "snapshot_name").cmp(s(b, "snapshot_name")))
                .then_with(|| s(a, "sync_id").cmp(s(b, "sync_id")))
        });
        let weights = weights_for(&t, &gear);
        let mut groups: Vec<(String, Vec<Row>)> = vec![];
        for g in &gear {
            let cat = opt(g, "snapshot_category").unwrap_or_else(|| "Uncategorized".into());
            if let Some((_, rs)) = groups.iter_mut().find(|(c, _)| *c == cat) {
                rs.push(g.clone())
            } else {
                groups.push((cat, vec![g.clone()]))
            }
        }
        let mut cats = rows(db, "gear_category")?;
        cats.sort_by_key(|c| n(c, "sort_order"));
        groups.sort_by_key(|(c, _)| cats.iter().position(|r| s(r, "name") == c).unwrap_or(99));
        let ep = rows(db, "energy_params")?.into_iter().next();
        let params = ep
            .map(|r| EnergyParams {
                bmr: n(&r, "bmr") as f64,
                cal_per_mile: n(&r, "cal_per_energy_mile") as f64,
                ft_per_mile: n(&r, "ft_per_energy_mile") as f64,
            })
            .unwrap_or_default();
        let entries = children(db, "trip_meal", key)?;
        let food = trip_food(
            &params,
            optional_i32(&t, "nights"),
            &days
                .iter()
                .map(|d| DayInput {
                    day_number: n(d, "day_number") as i32,
                    distance_mi: numeric(d, "distance_mi"),
                    gain_ft: numeric(d, "elevation_gain_ft"),
                })
                .collect::<Vec<_>>(),
            &entries
                .iter()
                .map(|e| TripFoodEntry {
                    id: s(e, "sync_id").into(),
                    day_number: n(e, "day_number") as i32,
                    meal_type: opt(e, "meal_type"),
                    name: s(e, "snapshot_name").into(),
                    meal_id: opt(e, "meal_id"),
                    servings: numeric(e, "servings").unwrap_or(1.),
                    weight_g: n(e, "snapshot_weight_g"),
                    kcal: n(e, "snapshot_kcal"),
                })
                .collect::<Vec<_>>(),
        );
        let meals = rows(db, "meal")?;
        let find_meal = |e: &Row| {
            meals
                .iter()
                .find(|m| opt(e, "meal_id").as_deref() == Some(s(m, "sync_id")))
        };
        let shopping = shopping_list(
            &entries
                .iter()
                .map(|e| ShoppingEntry {
                    meal_id: opt(e, "meal_id"),
                    ingredient_id: opt(e, "ingredient_id"),
                    servings: numeric(e, "servings").unwrap_or(1.),
                    snapshot_name: s(e, "snapshot_name").into(),
                    snapshot_weight_g: n(e, "snapshot_weight_g"),
                    base_servings: find_meal(e).and_then(|m| optional_i32(m, "base_servings")),
                })
                .collect::<Vec<_>>(),
            &rows(db, "meal_ingredient")?
                .iter()
                .map(|r| ShoppingLine {
                    meal_id: s(r, "meal_id").into(),
                    ingredient_id: opt(r, "ingredient_id"),
                    snapshot_name: s(r, "snapshot_name").into(),
                    amount_g: n(r, "amount_g"),
                })
                .collect::<Vec<_>>(),
        );
        let water = cook_water_per_person(
            &entries
                .iter()
                .map(|e| MealWater {
                    is_hot: find_meal(e).is_some_and(|m| n(m, "is_hot") != 0),
                    water_ml: find_meal(e).and_then(|m| numeric(m, "water_ml").map(|n| n as i64)),
                    base_servings: find_meal(e).and_then(|m| optional_i32(m, "base_servings")),
                    servings: numeric(e, "servings").unwrap_or(1.),
                })
                .collect::<Vec<_>>(),
        );
        let packing_list = opt(&t, "packing_list")
            .or(rows(db, "report_settings")?
                .first()
                .and_then(|r| opt(r, "packing_default")))
            .or_else(|| Some(DEFAULT_PACKING_LIST.into()));
        Ok(Some(TripDetail {
            share_token,
            edit_token,
            region: opt(&t, "region").or_else(|| trail.as_ref().and_then(|r| opt(r, "region"))),
            area_type: opt(&t, "area_type")
                .or_else(|| trail.as_ref().and_then(|r| opt(r, "area_type"))),
            lat: numeric(&t, "lat").or_else(|| gpx.as_ref().and_then(|g| numeric(g, "start_lat"))),
            lon: numeric(&t, "lon").or_else(|| gpx.as_ref().and_then(|g| numeric(g, "start_lon"))),
            fields: t,
            trail,
            access,
            route,
            days,
            segments,
            campsites: camps,
            gpx,
            points: pts,
            packed_count: gear.iter().filter(|g| n(g, "packed") != 0).count(),
            gear,
            groups,
            weights,
            food,
            shopping,
            cook_water_ml_per_person: water,
            permit: self.permit_blobs()?.into_iter().find(|p| p.trip_id == key),
            packing_list,
            consumables: children(db, "trip_consumable", key)?,
        }))
    }
}
