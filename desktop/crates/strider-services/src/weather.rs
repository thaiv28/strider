//! Port of `lib/weather.ts`: Open-Meteo forecast, archive and climatology, cached in the store
//! (`weather_cache`) so the Trip page can show the last fetched values offline.
use crate::{Cached, Clock, HttpClient, Result, ServiceError, coord_key, read_through, ttl};
use chrono::{DateTime, Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use strider_core::format::js_round;
use strider_store::{CacheKind, Store};

pub const FORECAST_DAYS: i64 = 16;
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

pub fn add_days(iso: &str, n: i64) -> Option<String> {
    let d = NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()?;
    Some(
        d.checked_add_signed(Duration::days(n))?
            .format("%Y-%m-%d")
            .to_string(),
    )
}
/// `Fri, Jan 5`, as `prettyDate` renders in an en-US locale.
pub fn pretty_date(iso: &str) -> String {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d")
        .map(|d| d.format("%a, %b %-d").to_string())
        .unwrap_or_else(|_| iso.to_string())
}
/// `6:05 am` from an ISO local timestamp such as `2025-07-01T06:05`; an em dash when absent.
pub fn clock(iso: Option<&str>) -> String {
    let Some(iso) = iso else {
        return "—".into();
    };
    let hm = iso.get(11..16).unwrap_or("");
    let mut parts = hm.split(':').map(|p| p.parse::<u32>().ok());
    let (Some(Some(h)), Some(Some(m))) = (parts.next(), parts.next()) else {
        return "NaN:NaN am".into();
    };
    let ampm = if h < 12 { "am" } else { "pm" };
    let hr = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{hr}:{m:02} {ampm}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Description {
    pub icon: &'static str,
    pub text: &'static str,
}
/// WMO weather code to glyph and short description (Open-Meteo `weather_code`).
pub fn describe(code: Option<i64>) -> Description {
    let (icon, text) = match code {
        Some(0) => ("☀️", "Clear"),
        Some(1) => ("🌤️", "Mainly clear"),
        Some(2) => ("⛅", "Partly cloudy"),
        Some(3) => ("☁️", "Overcast"),
        Some(45 | 48) => ("🌫️", "Fog"),
        Some(51) => ("🌦️", "Light drizzle"),
        Some(53) => ("🌦️", "Drizzle"),
        Some(55) => ("🌧️", "Heavy drizzle"),
        Some(56 | 57) => ("🌧️", "Freezing drizzle"),
        Some(61) => ("🌦️", "Light rain"),
        Some(63) => ("🌧️", "Rain"),
        Some(65) => ("🌧️", "Heavy rain"),
        Some(66 | 67) => ("🌧️", "Freezing rain"),
        Some(71) => ("🌨️", "Light snow"),
        Some(73) => ("🌨️", "Snow"),
        Some(75) => ("❄️", "Heavy snow"),
        Some(77) => ("🌨️", "Snow grains"),
        Some(80) => ("🌦️", "Light showers"),
        Some(81) => ("🌧️", "Showers"),
        Some(82) => ("⛈️", "Heavy showers"),
        Some(85 | 86) => ("🌨️", "Snow showers"),
        Some(95) => ("⛈️", "Thunderstorm"),
        Some(96 | 99) => ("⛈️", "Thunderstorm, hail"),
        _ => ("•", "—"),
    };
    Description { icon, text }
}

pub fn today_utc(now: i64) -> String {
    DateTime::from_timestamp(now, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}
/// Beyond the ~16 day forecast horizon, where typical conditions replace a forecast.
pub fn is_far_future(date_iso: &str, now: i64) -> bool {
    let today = today_utc(now);
    let horizon = add_days(&today, FORECAST_DAYS).unwrap_or_default();
    date_iso >= today.as_str() && date_iso > horizon.as_str()
}

/// Open-Meteo `daily` arrays (one entry for a single-day request). Values may be null.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyWx {
    pub time: Vec<String>,
    pub temperature_2m_max: Vec<Option<f64>>,
    pub temperature_2m_min: Vec<Option<f64>>,
    pub precipitation_sum: Vec<Option<f64>>,
    pub wind_speed_10m_max: Vec<Option<f64>>,
    pub sunrise: Vec<Option<String>>,
    pub sunset: Vec<Option<String>>,
    pub weather_code: Vec<Option<i64>>,
    pub precipitation_probability_max: Option<Vec<Option<f64>>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WxKind {
    Forecast,
    Historical,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WxDay {
    pub kind: WxKind,
    pub day: DailyWx,
}
#[derive(Clone, Debug, PartialEq)]
pub enum WxResult {
    Day(Box<Cached<WxDay>>),
    /// No coordinates, out of range, no data, or offline with nothing cached.
    Unavailable(String),
}

fn located(lat: Option<f64>, lon: Option<f64>) -> Option<(f64, f64)> {
    match (lat, lon) {
        (Some(a), Some(b)) if a.is_finite() && b.is_finite() => Some((a, b)),
        _ => None,
    }
}
fn get_json(http: &dyn HttpClient, url: &str) -> Result<Option<Value>> {
    let r = http.get(url)?;
    if r.status == 429 || r.status >= 500 {
        return Err(r.error());
    }
    // Like the web, a client error with a JSON body simply carries no data.
    Ok(serde_json::from_str(&r.body).ok())
}

/// One day's forecast (today onward) or archive (past) for a coordinate.
pub fn fetch_day(
    store: &Store,
    http: &dyn HttpClient,
    clock_: &dyn Clock,
    lat: Option<f64>,
    lon: Option<f64>,
    date: &str,
    elevation_ft: Option<f64>,
) -> WxResult {
    let Some((lat, lon)) = located(lat, lon) else {
        return WxResult::Unavailable("no coordinates".into());
    };
    let now = clock_.now();
    let today = today_utc(now);
    let past = date < today.as_str();
    if !past && date > add_days(&today, FORECAST_DAYS).unwrap_or_default().as_str() {
        return WxResult::Unavailable("too far out for forecast".into());
    }
    let vars = format!(
        "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset{}",
        if past {
            ""
        } else {
            ",precipitation_probability_max"
        }
    );
    let base = if past { ARCHIVE_URL } else { FORECAST_URL };
    let elev = elevation_ft
        .filter(|e| e.is_finite())
        .map(|e| format!("&elevation={}", js_round(e / 3.28084)))
        .unwrap_or_default();
    let url = format!(
        "{base}?latitude={lat}&longitude={lon}&start_date={date}&end_date={date}&daily={vars}\
         &temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch&timezone=auto{elev}"
    );
    let kind = if past {
        WxKind::Historical
    } else {
        WxKind::Forecast
    };
    let key = format!(
        "day|{}|{date}|{}|{}",
        coord_key(lat, lon),
        if past { "historical" } else { "forecast" },
        elevation_ft
            .map(|e| js_round(e).to_string())
            .unwrap_or_default()
    );
    let result = read_through(
        store,
        CacheKind::Weather,
        &key,
        if past { ttl::HISTORICAL } else { ttl::FORECAST },
        now,
        || {
            let Some(j) = get_json(http, &url)? else {
                return Ok(None);
            };
            let day: DailyWx = j
                .get("daily")
                .and_then(|d| serde_json::from_value(d.clone()).ok())
                .unwrap_or_default();
            Ok((!day.time.is_empty()).then_some(WxDay { kind, day }))
        },
    );
    match result {
        Ok(Some(c)) => WxResult::Day(Box::new(c)),
        Ok(None) => WxResult::Unavailable("no data".into()),
        Err(_) => WxResult::Unavailable("unavailable".into()),
    }
}

/// Typical conditions for a calendar date, averaged from the ERA5 archive over recent years.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Climo {
    pub years: u32,
    pub hi_f: i64,
    pub lo_f: i64,
    pub precip_in: f64,
    pub precip_pct: i64,
    pub snow_in: f64,
    pub snow_pct: i64,
}
fn mean(a: &[f64]) -> f64 {
    a.iter().sum::<f64>() / a.len() as f64
}
fn to_fixed2(x: f64) -> f64 {
    js_round(x * 100.) / 100.
}

/// Average a +/- `window_days` window around the target month and day across the last
/// `years_back` complete years. The ten archive requests run four at a time.
#[allow(clippy::too_many_arguments)]
pub fn fetch_climatology(
    store: &Store,
    http: &dyn HttpClient,
    clock_: &dyn Clock,
    lat: Option<f64>,
    lon: Option<f64>,
    date_iso: &str,
    years_back: u32,
    window_days: i64,
) -> Result<Option<Cached<Climo>>> {
    let Some((lat, lon)) = located(lat, lon) else {
        return Ok(None);
    };
    let Some(md) = date_iso.get(5..).filter(|m| m.len() == 5) else {
        return Ok(None);
    };
    let now = clock_.now();
    let key = format!(
        "climo|{}|{md}|{years_back}|{window_days}",
        coord_key(lat, lon)
    );
    let now_year = DateTime::from_timestamp(now, 0)
        .map(|d| d.year())
        .unwrap_or(1970);
    let years: Vec<i32> = (0..years_back as i32).map(|i| now_year - 1 - i).collect();
    read_through(
        store,
        CacheKind::Weather,
        &key,
        ttl::CLIMATOLOGY,
        now,
        || {
            let next = AtomicUsize::new(0);
            let results = std::sync::Mutex::new(Vec::<Result<Option<Value>>>::new());
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(|| {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(y) = years.get(i) else { return };
                        let anchor = format!("{y}-{md}");
                        let (Some(start), Some(end)) =
                            (add_days(&anchor, -window_days), add_days(&anchor, window_days))
                        else {
                            continue;
                        };
                        let url = format!(
                            "{ARCHIVE_URL}?latitude={lat}&longitude={lon}&start_date={start}&end_date={end}\
                             &daily=temperature_2m_max,temperature_2m_min,precipitation_sum,snowfall_sum\
                             &temperature_unit=fahrenheit&precipitation_unit=inch&timezone=auto"
                        );
                        let r = get_json(http, &url);
                        if let Ok(mut l) = results.lock() {
                            l.push(r);
                        }
                    }
                });
                }
            });
            let results = results.into_inner().unwrap_or_default();
            let (mut max_t, mut min_t, mut precip, mut snow) = (vec![], vec![], vec![], vec![]);
            let mut contributing = 0u32;
            let mut failures = 0;
            for r in &results {
                let j = match r {
                    Ok(Some(j)) => j,
                    Ok(None) => continue,
                    Err(_) => {
                        failures += 1;
                        continue;
                    }
                };
                let Some(d) = j.get("daily") else { continue };
                let Some(times) = d
                    .get("time")
                    .and_then(Value::as_array)
                    .filter(|t| !t.is_empty())
                else {
                    continue;
                };
                contributing += 1;
                let col = |name: &str, out: &mut Vec<f64>| {
                    for i in 0..times.len() {
                        if let Some(v) = d.get(name).and_then(|a| a.get(i)).and_then(Value::as_f64)
                        {
                            out.push(v);
                        }
                    }
                };
                col("temperature_2m_max", &mut max_t);
                col("temperature_2m_min", &mut min_t);
                col("precipitation_sum", &mut precip);
                col("snowfall_sum", &mut snow);
            }
            if contributing == 0 && failures > 0 {
                return Err(ServiceError::Offline);
            }
            if max_t.is_empty() || precip.is_empty() {
                return Ok(None);
            }
            let pct = |n: usize, d: usize| js_round(100. * n as f64 / d as f64) as i64;
            Ok(Some(Climo {
                years: contributing,
                hi_f: js_round(mean(&max_t)) as i64,
                lo_f: if min_t.is_empty() {
                    0
                } else {
                    js_round(mean(&min_t)) as i64
                },
                precip_in: to_fixed2(mean(&precip)),
                precip_pct: pct(precip.iter().filter(|p| **p >= 0.01).count(), precip.len()),
                // snowfall_sum is cm; the average is shown in inches.
                snow_in: if snow.is_empty() {
                    0.
                } else {
                    to_fixed2(mean(&snow) / 2.54)
                },
                snow_pct: if snow.is_empty() {
                    0
                } else {
                    pct(snow.iter().filter(|s| **s > 0.).count(), snow.len())
                },
            }))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::{FakeHttp, FixedClock};

    // 2025-07-01T12:00:00Z
    const NOON: i64 = 1_751_371_200;
    const DAY: &str = include_str!("../../../fixtures/http/open-meteo-forecast-day.json");
    const ARCHIVE: &str = include_str!("../../../fixtures/http/open-meteo-archive-climo.json");

    #[test]
    fn describes_wmo_codes() {
        assert_eq!(describe(Some(0)).text, "Clear");
        assert_eq!(
            describe(Some(3)),
            Description {
                icon: "☁️",
                text: "Overcast"
            }
        );
        assert_eq!(describe(Some(48)).text, "Fog");
        assert_eq!(describe(Some(57)).text, "Freezing drizzle");
        assert_eq!(describe(Some(67)).text, "Freezing rain");
        assert_eq!(describe(Some(86)).text, "Snow showers");
        assert_eq!(describe(Some(99)).text, "Thunderstorm, hail");
        assert_eq!(
            describe(Some(4)),
            Description {
                icon: "•",
                text: "—"
            }
        );
        assert_eq!(describe(None).text, "—");
    }
    #[test]
    fn date_helpers() {
        assert_eq!(add_days("2024-02-28", 2).as_deref(), Some("2024-03-01"));
        assert_eq!(add_days("2025-01-01", -1).as_deref(), Some("2024-12-31"));
        assert_eq!(add_days("nope", 1), None);
        assert_eq!(pretty_date("2025-07-04"), "Fri, Jul 4");
        assert_eq!(clock(Some("2025-07-01T06:05")), "6:05 am");
        assert_eq!(clock(Some("2025-07-01T00:00")), "12:00 am");
        assert_eq!(clock(Some("2025-07-01T12:30")), "12:30 pm");
        assert_eq!(clock(Some("2025-07-01T20:45")), "8:45 pm");
        assert_eq!(clock(None), "—");
    }
    #[test]
    fn far_future_starts_after_the_sixteen_day_horizon() {
        assert!(!is_far_future("2025-06-30", NOON));
        assert!(!is_far_future("2025-07-17", NOON));
        assert!(is_far_future("2025-07-18", NOON));
    }
    #[test]
    fn forecast_is_cached_then_served_stale_offline() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("api.open-meteo.com/v1/forecast", 200, DAY);
        let clock_ = FixedClock::at(NOON);
        let first = fetch_day(
            &store,
            &http,
            &clock_,
            Some(39.1),
            Some(-105.2),
            "2025-07-03",
            Some(9000.),
        );
        let WxResult::Day(c) = first else {
            panic!("{first:?}")
        };
        assert_eq!(
            (c.value.kind, c.stale, c.fetched_at),
            (WxKind::Forecast, false, NOON)
        );
        assert_eq!(describe(c.value.day.weather_code[0]).text, "Light rain");
        assert!(http.log.lock().unwrap()[0].contains("&elevation=2743"));
        // Within the hour: served from the cache without a request.
        clock_.advance(600);
        let again = fetch_day(
            &store,
            &http,
            &clock_,
            Some(39.1),
            Some(-105.2),
            "2025-07-03",
            Some(9000.),
        );
        assert!(matches!(again, WxResult::Day(ref c) if !c.stale && c.fetched_at == NOON));
        assert_eq!(http.calls(), 1);
        // After the hour and offline: the last copy comes back marked stale.
        clock_.advance(7_200);
        http.clear();
        let stale = fetch_day(
            &store,
            &http,
            &clock_,
            Some(39.1),
            Some(-105.2),
            "2025-07-03",
            Some(9000.),
        );
        assert!(matches!(stale, WxResult::Day(ref c) if c.stale && c.fetched_at == NOON));
    }
    #[test]
    fn unavailable_reasons_match_the_web() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        let c = FixedClock::at(NOON);
        let r = |lat, lon, d: &str| fetch_day(&store, &http, &c, lat, lon, d, None);
        assert_eq!(
            r(None, Some(1.), "2025-07-03"),
            WxResult::Unavailable("no coordinates".into())
        );
        assert_eq!(
            r(Some(f64::NAN), Some(1.), "2025-07-03"),
            WxResult::Unavailable("no coordinates".into())
        );
        assert_eq!(
            r(Some(1.), Some(1.), "2025-08-01"),
            WxResult::Unavailable("too far out for forecast".into())
        );
        // Offline with nothing cached.
        assert_eq!(
            r(Some(1.), Some(1.), "2025-07-03"),
            WxResult::Unavailable("unavailable".into())
        );
        // A reply without data.
        http.route(
            "api.open-meteo.com",
            200,
            r#"{"error":true,"reason":"bad"}"#,
        );
        assert_eq!(
            r(Some(1.), Some(1.), "2025-07-03"),
            WxResult::Unavailable("no data".into())
        );
        assert_eq!(http.calls(), 2);
    }
    #[test]
    fn past_dates_use_the_archive_without_probability() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("archive-api.open-meteo.com/v1/archive", 200, DAY);
        let c = FixedClock::at(NOON);
        let r = fetch_day(
            &store,
            &http,
            &c,
            Some(39.1),
            Some(-105.2),
            "2025-06-20",
            None,
        );
        assert!(matches!(r, WxResult::Day(ref c) if c.value.kind == WxKind::Historical));
        let url = http.log.lock().unwrap()[0].clone();
        assert!(url.contains("archive-api") && !url.contains("precipitation_probability_max"));
        // Historical values stay fresh for a week.
        c.advance(6 * 86_400);
        fetch_day(
            &store,
            &http,
            &c,
            Some(39.1),
            Some(-105.2),
            "2025-06-20",
            None,
        );
        assert_eq!(http.calls(), 1);
    }
    #[test]
    fn climatology_averages_years_and_caches() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("archive-api.open-meteo.com", 200, ARCHIVE);
        let c = FixedClock::at(NOON);
        let got = fetch_climatology(
            &store,
            &http,
            &c,
            Some(39.1),
            Some(-105.2),
            "2026-01-10",
            10,
            3,
        )
        .unwrap()
        .unwrap();
        assert_eq!(http.calls(), 10);
        let v = got.value;
        // Every year returns the same three days: highs 40/42/38, lows 20/22/18, precip 0/0.1/0.3,
        // snow 0/5.08/2.54 cm.
        assert_eq!((v.years, v.hi_f, v.lo_f), (10, 40, 20));
        assert_eq!((v.precip_in, v.precip_pct), (0.13, 67));
        assert_eq!((v.snow_in, v.snow_pct), (1.0, 67));
        // Fresh: no further requests. Offline later: stale copy.
        fetch_climatology(
            &store,
            &http,
            &c,
            Some(39.1),
            Some(-105.2),
            "2026-01-10",
            10,
            3,
        )
        .unwrap();
        assert_eq!(http.calls(), 10);
        c.advance(40 * 86_400);
        http.clear();
        let stale = fetch_climatology(
            &store,
            &http,
            &c,
            Some(39.1),
            Some(-105.2),
            "2026-01-10",
            10,
            3,
        )
        .unwrap()
        .unwrap();
        assert!(stale.stale && stale.value.hi_f == 40);
        assert!(
            fetch_climatology(&store, &http, &c, None, Some(1.), "2026-01-10", 10, 3)
                .unwrap()
                .is_none()
        );
        let empty = Store::in_memory().unwrap();
        assert_eq!(
            fetch_climatology(&empty, &http, &c, Some(1.), Some(1.), "2026-01-10", 10, 3),
            Err(ServiceError::Offline)
        );
    }
}
