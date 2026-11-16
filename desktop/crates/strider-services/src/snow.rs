//! Port of `lib/snow.ts`: modeled snow depth from Open-Meteo (current, observed, forecast, and
//! typical for a calendar date), cached in the store (`weather_cache`) per source value.
use crate::weather::{add_days, today_utc};
use crate::{Cached, Clock, HttpClient, Result, coord_key, ttl};
use chrono::{DateTime, Datelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use strider_core::format::js_round;
use strider_store::{CacheKind, Store};

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
const FORECAST_DAYS: i64 = 16;
/// ERA5 lands in the archive about five days late; the forecast API covers the gap.
const ARCHIVE_LAG_DAYS: i64 = 6;
const TYPICAL_YEARS: i32 = 10;
/// Open-Meteo rejects large bursts; cap in-flight requests.
const MAX_IN_FLIGHT: usize = 6;
const CHUNK: usize = 50;
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

#[derive(Clone, Debug, PartialEq)]
pub struct SnowQuery {
    pub lat: f64,
    pub lon: f64,
    pub date: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnowKind {
    /// Reanalysis or model value for a past date.
    Observed,
    /// Within the ~16 day forecast horizon.
    Forecast,
    /// Mean for the calendar date across recent years.
    Typical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TripDateSnow {
    pub kind: SnowKind,
    pub inches: i64,
    pub years: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnowResult {
    pub now_in: Option<i64>,
    pub trip: Option<TripDateSnow>,
}

fn to_in(m: f64) -> f64 {
    m * 39.3701
}
fn round(inches: f64) -> i64 {
    (js_round(inches) as i64).max(0)
}
fn mean(a: &[f64]) -> f64 {
    a.iter().sum::<f64>() / a.len() as f64
}

/// One request with the web's single retry of a throttled or failed reply.
fn get_json(http: &dyn HttpClient, url: &str, delay: std::time::Duration) -> Option<Value> {
    for attempt in 0..2 {
        match http.get(url) {
            Ok(r) if r.ok() => return serde_json::from_str(&r.body).ok(),
            Ok(r) if r.status != 429 && r.status < 500 => return None,
            _ => {}
        }
        if attempt == 0 {
            std::thread::sleep(delay);
        }
    }
    None
}

struct Lookup {
    key: String,
    url: String,
    ttl: i64,
    parse: fn(&Value) -> Option<f64>,
}
/// A cached scalar: the value (inches, or `None` when the source had none) with provenance.
type Got = Option<Cached<f64>>;

/// Resolve many scalars: fresh cache hits first, the misses fetched concurrently, and any fetch
/// failure answered from the last stored copy (marked stale).
fn resolve(
    store: &Store,
    http: &dyn HttpClient,
    now: i64,
    lookups: &[Lookup],
    delay: std::time::Duration,
) -> Result<Vec<Got>> {
    let mut out: Vec<Got> = vec![None; lookups.len()];
    let mut old: Vec<Option<(f64, i64)>> = vec![None; lookups.len()];
    let mut missing = vec![];
    for (i, l) in lookups.iter().enumerate() {
        let hit = store
            .cache_get(CacheKind::Weather, &l.key)?
            .and_then(|e| e.body.parse::<f64>().ok().map(|v| (v, e.fetched_at)));
        match hit {
            Some((value, fetched_at)) if (0..l.ttl).contains(&(now - fetched_at)) => {
                out[i] = Some(Cached {
                    value,
                    fetched_at,
                    stale: false,
                })
            }
            other => {
                old[i] = other;
                missing.push(i);
            }
        }
    }
    let next = AtomicUsize::new(0);
    let fetched = Mutex::new(Vec::<(usize, Option<f64>)>::new());
    std::thread::scope(|scope| {
        for _ in 0..MAX_IN_FLIGHT.min(missing.len()) {
            scope.spawn(|| {
                loop {
                    let n = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&i) = missing.get(n) else { return };
                    let l = &lookups[i];
                    let v = get_json(http, &l.url, delay).and_then(|j| (l.parse)(&j));
                    if let Ok(mut f) = fetched.lock() {
                        f.push((i, v));
                    }
                }
            });
        }
    });
    for (i, v) in fetched.into_inner().unwrap_or_default() {
        match (v, old[i]) {
            (Some(value), _) => {
                store.cache_put(CacheKind::Weather, &lookups[i].key, now, &value.to_string())?;
                out[i] = Some(Cached {
                    value,
                    fetched_at: now,
                    stale: false,
                });
            }
            (None, Some((value, fetched_at))) => {
                out[i] = Some(Cached {
                    value,
                    fetched_at,
                    stale: true,
                })
            }
            (None, None) => {}
        }
    }
    Ok(out)
}

fn archive_lookup(lat: f64, lon: f64, date: &str) -> Lookup {
    Lookup {
        key: format!("snow|archive|{}|{date}", coord_key(lat, lon)),
        url: format!(
            "{ARCHIVE_URL}?latitude={lat}&longitude={lon}&start_date={date}&end_date={date}&daily=snow_depth_mean&timezone=auto"
        ),
        ttl: ttl::HISTORICAL,
        parse: |j| {
            j.pointer("/daily/snow_depth_mean/0")
                .and_then(Value::as_f64)
                .map(to_in)
        },
    }
}
fn forecast_lookup(lat: f64, lon: f64, date: &str) -> Lookup {
    Lookup {
        key: format!("snow|forecast|{}|{date}", coord_key(lat, lon)),
        url: format!(
            "{FORECAST_URL}?latitude={lat}&longitude={lon}&start_date={date}&end_date={date}&hourly=snow_depth&timezone=auto"
        ),
        ttl: ttl::FORECAST,
        parse: |j| {
            let vals: Vec<f64> = j
                .pointer("/hourly/snow_depth")?
                .as_array()?
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            (!vals.is_empty()).then(|| to_in(mean(&vals)))
        },
    }
}
fn leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn merge(a: &mut (i64, bool), c: &Cached<f64>) {
    a.0 = a.0.min(c.fetched_at);
    a.1 |= c.stale;
}

/// Current depth per coordinate (one request per chunk of 50) and the depth on each query's date.
pub fn fetch_snow_depths(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    queries: &[SnowQuery],
) -> Result<Vec<Cached<SnowResult>>> {
    fetch_snow_depths_with(store, http, clock, queries, RETRY_DELAY)
}

/// [`fetch_snow_depths`] with the pause before the single retry of a failed request. Passing
/// `Duration::ZERO` makes a cache-only read (with a client that always fails) return at once.
pub fn fetch_snow_depths_with(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    queries: &[SnowQuery],
    delay: std::time::Duration,
) -> Result<Vec<Cached<SnowResult>>> {
    let now = clock.now();
    let today = today_utc(now);
    let now_year = DateTime::from_timestamp(now, 0)
        .map(|d| d.year())
        .unwrap_or(1970);
    let valid = |q: &SnowQuery| q.lat.abs() <= 90. && q.lon.abs() <= 180.;

    // Current depth, per unique coordinate.
    let mut coords: Vec<(f64, f64)> = vec![];
    for q in queries.iter().filter(|q| valid(q)) {
        if !coords
            .iter()
            .any(|c| coord_key(c.0, c.1) == coord_key(q.lat, q.lon))
        {
            coords.push((q.lat, q.lon));
        }
    }
    let mut current: Vec<Got> = vec![];
    for chunk in coords.chunks(CHUNK) {
        let mut got: Vec<Got> = vec![None; chunk.len()];
        let mut miss = vec![];
        for (k, c) in chunk.iter().enumerate() {
            let key = format!("snow|now|{}", coord_key(c.0, c.1));
            let hit = store.cache_get(CacheKind::Weather, &key)?;
            let hit = hit.and_then(|e| e.body.parse::<f64>().ok().map(|v| (v, e.fetched_at)));
            match hit {
                Some((value, fetched_at))
                    if (0..ttl::SNOW_CURRENT).contains(&(now - fetched_at)) =>
                {
                    got[k] = Some(Cached {
                        value,
                        fetched_at,
                        stale: false,
                    });
                }
                other => miss.push((k, key, other)),
            }
        }
        if !miss.is_empty() {
            let lats: Vec<String> = miss.iter().map(|m| chunk[m.0].0.to_string()).collect();
            let lons: Vec<String> = miss.iter().map(|m| chunk[m.0].1.to_string()).collect();
            let url = format!(
                "{FORECAST_URL}?latitude={}&longitude={}&current=snow_depth&timezone=auto",
                lats.join(","),
                lons.join(",")
            );
            let json = get_json(http, &url, delay);
            let list: Vec<Value> = match json {
                Some(Value::Array(a)) => a,
                Some(v) => vec![v],
                None => vec![],
            };
            for (n, (k, key, old)) in miss.into_iter().enumerate() {
                let meters = list
                    .get(n)
                    .and_then(|v| v.pointer("/current/snow_depth"))
                    .and_then(Value::as_f64);
                got[k] = match (meters, old) {
                    (Some(m), _) => {
                        let inches = to_in(m);
                        store.cache_put(CacheKind::Weather, &key, now, &inches.to_string())?;
                        Some(Cached {
                            value: inches,
                            fetched_at: now,
                            stale: false,
                        })
                    }
                    (None, Some((value, fetched_at))) => Some(Cached {
                        value,
                        fetched_at,
                        stale: true,
                    }),
                    (None, None) => None,
                };
            }
        }
        current.extend(got);
    }

    // Depth on each query's date.
    let mut out = vec![];
    for q in queries {
        if !valid(q) {
            out.push(Cached {
                value: SnowResult {
                    now_in: None,
                    trip: None,
                },
                fetched_at: now,
                stale: false,
            });
            continue;
        }
        let mut meta = (now, false);
        let now_in = coords
            .iter()
            .position(|c| coord_key(c.0, c.1) == coord_key(q.lat, q.lon))
            .and_then(|i| current[i].as_ref())
            .map(|c| {
                merge(&mut meta, c);
                round(c.value)
            });
        let trip = match q.date.as_deref() {
            None => None,
            Some(date)
                if date
                    > add_days(&today, FORECAST_DAYS - 1)
                        .unwrap_or_default()
                        .as_str() =>
            {
                // Typical: the same calendar date across recent years (Feb 29 falls back to Feb 28).
                let md = date.get(5..).unwrap_or("");
                let dates: Vec<String> = (1..=TYPICAL_YEARS)
                    .map(|i| now_year - i)
                    .map(|y| {
                        if md == "02-29" && !leap(y) {
                            format!("{y}-02-28")
                        } else {
                            format!("{y}-{md}")
                        }
                    })
                    .collect();
                let lookups: Vec<_> = dates
                    .iter()
                    .map(|d| archive_lookup(q.lat, q.lon, d))
                    .collect();
                let vals: Vec<Cached<f64>> = resolve(store, http, now, &lookups, delay)?
                    .into_iter()
                    .flatten()
                    .collect();
                if vals.is_empty() {
                    None
                } else {
                    vals.iter().for_each(|c| merge(&mut meta, c));
                    let all: Vec<f64> = vals.iter().map(|c| c.value).collect();
                    Some(TripDateSnow {
                        kind: SnowKind::Typical,
                        inches: round(mean(&all)),
                        years: Some(vals.len() as u32),
                    })
                }
            }
            Some(date)
                if date
                    < add_days(&today, -ARCHIVE_LAG_DAYS)
                        .unwrap_or_default()
                        .as_str() =>
            {
                let l = [archive_lookup(q.lat, q.lon, date)];
                resolve(store, http, now, &l, delay)?.remove(0).map(|c| {
                    merge(&mut meta, &c);
                    TripDateSnow {
                        kind: SnowKind::Observed,
                        inches: round(c.value),
                        years: None,
                    }
                })
            }
            Some(date) => {
                let l = [forecast_lookup(q.lat, q.lon, date)];
                resolve(store, http, now, &l, delay)?.remove(0).map(|c| {
                    merge(&mut meta, &c);
                    TripDateSnow {
                        kind: if date < today.as_str() {
                            SnowKind::Observed
                        } else {
                            SnowKind::Forecast
                        },
                        inches: round(c.value),
                        years: None,
                    }
                })
            }
        };
        out.push(Cached {
            value: SnowResult { now_in, trip },
            fetched_at: meta.0,
            stale: meta.1,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::{FakeHttp, FixedClock};
    use std::time::Duration;

    // 2025-07-01T12:00:00Z
    const NOON: i64 = 1_751_371_200;
    const CURRENT: &str = include_str!("../../../fixtures/http/open-meteo-snow-current.json");
    const ARCHIVE: &str = include_str!("../../../fixtures/http/open-meteo-snow-archive.json");
    const HOURLY: &str = include_str!("../../../fixtures/http/open-meteo-snow-hourly.json");
    const NO_DELAY: Duration = Duration::ZERO;

    fn q(date: Option<&str>) -> SnowQuery {
        SnowQuery {
            lat: 39.1,
            lon: -105.2,
            date: date.map(String::from),
        }
    }
    fn run(
        store: &Store,
        http: &FakeHttp,
        c: &FixedClock,
        qs: &[SnowQuery],
    ) -> Vec<Cached<SnowResult>> {
        fetch_snow_depths_with(store, http, c, qs, NO_DELAY).unwrap()
    }

    #[test]
    fn current_depth_converts_metres_to_rounded_inches() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("current=snow_depth", 200, CURRENT);
        let c = FixedClock::at(NOON);
        let r = run(&store, &http, &c, &[q(None)]);
        // 0.5 m = 19.7 in.
        assert_eq!(
            r[0].value,
            SnowResult {
                now_in: Some(20),
                trip: None
            }
        );
        assert!(!r[0].stale);
    }
    #[test]
    fn forecast_day_averages_hourly_depth() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("current=snow_depth", 200, CURRENT)
            .route("hourly=snow_depth", 200, HOURLY);
        let c = FixedClock::at(NOON);
        let r = run(&store, &http, &c, &[q(Some("2025-07-05"))]);
        // Hourly 0.1 and 0.3 m average 0.2 m = 7.87 in.
        assert_eq!(
            r[0].value.trip,
            Some(TripDateSnow {
                kind: SnowKind::Forecast,
                inches: 8,
                years: None
            })
        );
        // Dates inside the archive lag are observed values from the forecast API.
        let r = run(&store, &http, &c, &[q(Some("2025-06-28"))]);
        assert_eq!(r[0].value.trip.map(|t| t.kind), Some(SnowKind::Observed));
    }
    #[test]
    fn old_dates_use_the_archive() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("current=snow_depth", 200, CURRENT)
            .route("snow_depth_mean", 200, ARCHIVE);
        let c = FixedClock::at(NOON);
        let r = run(&store, &http, &c, &[q(Some("2025-03-01"))]);
        // 1.0 m = 39.37 in.
        assert_eq!(
            r[0].value.trip,
            Some(TripDateSnow {
                kind: SnowKind::Observed,
                inches: 39,
                years: None
            })
        );
    }
    #[test]
    fn far_future_dates_are_typical_across_ten_years_and_leap_day_falls_back() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("current=snow_depth", 200, CURRENT)
            .route("snow_depth_mean", 200, ARCHIVE);
        let c = FixedClock::at(NOON);
        let r = run(&store, &http, &c, &[q(Some("2026-02-29"))]);
        assert_eq!(
            r[0].value.trip,
            Some(TripDateSnow {
                kind: SnowKind::Typical,
                inches: 39,
                years: Some(10)
            })
        );
        let log = http.log.lock().unwrap().join("\n");
        assert!(log.contains("start_date=2024-02-29"));
        assert!(log.contains("start_date=2023-02-28") && !log.contains("2023-02-29"));
        assert_eq!(log.matches("snow_depth_mean").count(), 10);
    }
    #[test]
    fn values_are_cached_and_served_stale_offline() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("current=snow_depth", 200, CURRENT)
            .route("snow_depth_mean", 200, ARCHIVE);
        let c = FixedClock::at(NOON);
        run(&store, &http, &c, &[q(Some("2025-03-01"))]);
        let calls = http.calls();
        c.advance(60);
        let r = run(&store, &http, &c, &[q(Some("2025-03-01"))]);
        assert_eq!(http.calls(), calls, "fresh values are not refetched");
        assert!(!r[0].stale);
        // The current depth expires after 30 minutes; offline, the old copy is stale.
        c.advance(3_600);
        http.clear();
        let r = run(&store, &http, &c, &[q(Some("2025-03-01"))]);
        assert!(r[0].stale);
        assert_eq!(r[0].fetched_at, NOON);
        assert_eq!(r[0].value.now_in, Some(20));
        assert_eq!(r[0].value.trip.map(|t| t.inches), Some(39));
    }
    #[test]
    fn offline_without_a_cache_yields_no_values_and_invalid_coordinates_are_skipped() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        let c = FixedClock::at(NOON);
        let bad = SnowQuery {
            lat: 95.,
            lon: 0.,
            date: Some("2025-03-01".into()),
        };
        let r = run(&store, &http, &c, &[q(Some("2025-03-01")), bad]);
        assert_eq!(
            r[0].value,
            SnowResult {
                now_in: None,
                trip: None
            }
        );
        assert_eq!(
            r[1].value,
            SnowResult {
                now_in: None,
                trip: None
            }
        );
    }
    #[test]
    fn nearby_queries_share_one_current_request_in_one_chunk() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route(
            "current=snow_depth",
            200,
            r#"[{"current":{"snow_depth":0.5}},{"current":{"snow_depth":0.0}}]"#,
        );
        let c = FixedClock::at(NOON);
        let far = SnowQuery {
            lat: 45.,
            lon: -110.,
            date: None,
        };
        let r = run(&store, &http, &c, &[q(None), far, q(None)]);
        assert_eq!(http.calls(), 1);
        assert_eq!(
            r.iter().map(|x| x.value.now_in).collect::<Vec<_>>(),
            vec![Some(20), Some(0), Some(20)]
        );
    }
}
