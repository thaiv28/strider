//! Weather and snow panels for the Trip page (ports of `weather-panel.tsx` and `snow-panel.tsx`).
//! Both render only what the device cache holds, so they work offline and say how old it is.
use super::*;
use offline::{CacheOnly, format::age};
use strider_services::{
    Cached, Clock, HttpClient, SystemClock,
    snow::{self, SnowKind, SnowQuery, TripDateSnow},
    weather::{self, Climo, WxDay, WxKind, WxResult},
};
use strider_store::TripDetail;
use trips::{num, text};

/// What a panel needs from the Trip. Cloneable so a refresh thread can own it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PanelInput {
    pub start_date: Option<String>,
    pub nights: i64,
    pub days: usize,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// `(night, lat, lon)` of each campsite.
    pub camps: Vec<(i64, Option<f64>, Option<f64>)>,
    pub end: Option<(f64, f64)>,
    pub elevation_ft: Option<f64>,
}
pub(crate) enum Cell {
    Day(Box<Cached<WxDay>>),
    Typical(Box<Cached<Climo>>),
    /// A definite answer: no coordinates, nothing for that date.
    Message(&'static str),
    /// No cached value and no connection.
    Missing,
}
pub(crate) struct WeatherRow {
    pub label: String,
    pub note: Option<&'static str>,
    pub date: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub cell: Cell,
}
pub(crate) struct PanelData {
    pub input: PanelInput,
    /// Why there are no rows, when there are none.
    pub hint: Option<&'static str>,
    pub rows: Vec<WeatherRow>,
    pub snow_now: Option<i64>,
    pub snow_days: Vec<Option<TripDateSnow>>,
    /// The oldest value shown, and whether any of it is out of date because refreshing failed.
    pub updated: Option<i64>,
    pub stale: bool,
}
impl PanelData {
    pub fn has_weather(&self) -> bool {
        self.rows.iter().any(|r| !matches!(r.cell, Cell::Missing))
    }
    pub fn has_snow(&self) -> bool {
        self.snow_now.is_some() || self.snow_days.iter().any(Option::is_some)
    }
    /// Whether any row still needs the network.
    fn wants_refresh(&self) -> bool {
        self.stale
            || self.rows.iter().any(|r| matches!(r.cell, Cell::Missing))
            || (!self.has_snow() && self.input.lat.is_some() && self.input.start_date.is_some())
    }
}
fn located(lat: Option<f64>, lon: Option<f64>) -> bool {
    matches!((lat, lon), (Some(a), Some(b)) if a.is_finite() && b.is_finite())
}
impl PanelInput {
    pub fn from_trip(d: &TripDetail) -> Self {
        let start = text(&d.fields, "start_date");
        let nights = num(&d.fields, "nights") as i64;
        Self {
            start_date: (!start.is_empty()).then_some(start),
            nights,
            days: d.days.len().max(nights.max(0) as usize + 1),
            lat: d.lat,
            lon: d.lon,
            camps: d
                .campsites
                .iter()
                .map(|r| {
                    (
                        num(r, "night") as i64,
                        text(r, "lat").parse().ok(),
                        text(r, "lon").parse().ok(),
                    )
                })
                .collect(),
            end: d.points.last().map(|p| (p.lat, p.lon)),
            elevation_ft: d
                .points
                .iter()
                .map(|p| p.e)
                .fold(None, |m: Option<f64>, e| Some(m.map_or(e, |m| m.max(e)))),
        }
    }
    /// Each day and where to look: a campsite per day, then the finish; or the trip point.
    fn day_rows(&self) -> (Vec<WeatherRow>, Option<&'static str>) {
        let Some(start) = &self.start_date else {
            return (vec![], Some("Set a start date to load weather."));
        };
        let row = |label: String, note, lat, lon, date: String| WeatherRow {
            label,
            note,
            date,
            lat,
            lon,
            cell: Cell::Missing,
        };
        let mut rows = vec![];
        if !self.camps.is_empty() {
            for (i, (_, lat, lon)) in self.camps.iter().enumerate() {
                let date = weather::add_days(start, i as i64).unwrap_or_default();
                rows.push(row(format!("Day {}", i + 1), None, *lat, *lon, date));
            }
            if let Some((lat, lon)) = self.end {
                let date = weather::add_days(start, self.nights).unwrap_or_default();
                rows.push(row(
                    format!("Day {}", self.camps.len() + 1),
                    Some("finish"),
                    Some(lat),
                    Some(lon),
                    date,
                ));
            }
        } else if located(self.lat, self.lon) {
            for i in 0..self.days {
                rows.push(row(
                    if self.days > 1 {
                        format!("Day {}", i + 1)
                    } else {
                        "Forecast".into()
                    },
                    None,
                    self.lat,
                    self.lon,
                    weather::add_days(start, i as i64).unwrap_or_default(),
                ));
            }
        }
        let hint = rows
            .is_empty()
            .then_some("Add a route or coordinates to the trip to load weather.");
        (rows, hint)
    }
}
/// Read (or, with a real client, fetch and store) every value the panels show.
pub(crate) fn compute(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    input: &PanelInput,
    retry: Duration,
) -> PanelData {
    let now = clock.now();
    let (mut rows, hint) = input.day_rows();
    let (mut updated, mut stale): (Option<i64>, bool) = (None, false);
    let mut note = |c: i64, s: bool| {
        updated = Some(updated.map_or(c, |u| u.min(c)));
        stale |= s;
    };
    for r in &mut rows {
        r.cell = if !located(r.lat, r.lon) {
            Cell::Message("no coordinates")
        } else if weather::is_far_future(&r.date, now) {
            match weather::fetch_climatology(store, http, clock, r.lat, r.lon, &r.date, 10, 3) {
                Ok(Some(c)) => {
                    note(c.fetched_at, c.stale);
                    Cell::Typical(Box::new(c))
                }
                Ok(None) => Cell::Message("no climate data"),
                Err(_) => Cell::Missing,
            }
        } else {
            match weather::fetch_day(
                store,
                http,
                clock,
                r.lat,
                r.lon,
                &r.date,
                input.elevation_ft,
            ) {
                WxResult::Day(c) => {
                    note(c.fetched_at, c.stale);
                    Cell::Day(c)
                }
                WxResult::Unavailable(m) if m == "no data" => Cell::Message("no data"),
                WxResult::Unavailable(m) if m == "unavailable" => Cell::Missing,
                WxResult::Unavailable(_) => Cell::Message("not available for this date"),
            }
        };
    }
    let (mut snow_now, mut snow_days) = (None, vec![None; rows.len()]);
    if let (Some(lat), Some(lon)) = (input.lat, input.lon).into_pair()
        && input.start_date.is_some()
    {
        let mut queries = vec![SnowQuery {
            lat,
            lon,
            date: None,
        }];
        let mut at = vec![];
        for (i, r) in rows.iter().enumerate() {
            if let (Some(a), Some(b)) = (r.lat, r.lon).into_pair() {
                queries.push(SnowQuery {
                    lat: a,
                    lon: b,
                    date: Some(r.date.clone()),
                });
                at.push(i);
            }
        }
        if let Ok(got) = snow::fetch_snow_depths_with(store, http, clock, &queries, retry) {
            if let Some(first) = got.first() {
                snow_now = first.value.now_in;
                if snow_now.is_some() {
                    note(first.fetched_at, first.stale);
                }
            }
            for (k, i) in at.into_iter().enumerate() {
                if let Some(c) = got.get(k + 1)
                    && c.value.trip.is_some()
                {
                    snow_days[i] = c.value.trip;
                    note(c.fetched_at, c.stale);
                }
            }
        }
    }
    PanelData {
        input: input.clone(),
        hint,
        rows,
        snow_now,
        snow_days,
        updated,
        stale,
    }
}
trait IntoPair {
    fn into_pair(self) -> (Option<f64>, Option<f64>);
}
impl IntoPair for (Option<f64>, Option<f64>) {
    fn into_pair(self) -> (Option<f64>, Option<f64>) {
        if located(self.0, self.1) {
            self
        } else {
            (None, None)
        }
    }
}
impl AppView {
    /// Read the panels for the selected Trip from the cache; refresh them in the background.
    pub(in crate::app) fn load_panels(&mut self) {
        let Some(d) = &self.trips.detail else {
            self.panels = None;
            return;
        };
        if self.trips.section != "Weather" {
            return;
        }
        let input = PanelInput::from_trip(d);
        let data = compute(
            &self.store,
            &CacheOnly,
            &SystemClock,
            &input,
            Duration::ZERO,
        );
        let key = format!("panels|{}", text(&d.fields, "sync_id"));
        if data.wants_refresh() {
            self.refresh(key, move |s, h, c| {
                compute(s, h, c, &input, Duration::from_millis(750));
            });
        }
        self.panels = Some(data);
    }
    fn loading_panels(&self) -> bool {
        self.trips
            .selected
            .as_ref()
            .is_some_and(|id| self.refreshing(&format!("panels|{id}")))
    }
    fn updated_line(&self, data: &PanelData, t: Theme) -> Option<AnyElement> {
        let at = data.updated?;
        let mut line = format!("Updated {}", age(self.now(), at));
        if data.stale {
            line.push_str(" · Offline, showing saved data");
        }
        Some(div().meta(t).child(line).into_any_element())
    }
    pub(in crate::app) fn render_trip_weather(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let Some(data) = &self.panels else {
            return div().into_any_element();
        };
        let loading = self.loading_panels();
        let mut body = div().flex().flex_col().gap_3();
        // With no cached value and no connection there is nothing to show at all.
        let nothing = !data.has_weather() && data.hint.is_none() && !loading;
        if !nothing && !(data.rows.is_empty() && data.hint.is_none()) {
            let mut card = strider_ui::card(t)
                .p_4()
                .flex()
                .flex_col()
                .child(div().card_heading().child("Weather"));
            if let Some(hint) = data.hint {
                card = card.child(div().py_3().meta(t).child(hint));
            }
            for (i, r) in data.rows.iter().enumerate() {
                card = card.child(self.weather_row(i, r, loading, t));
            }
            if let Some(line) = self.updated_line(data, t) {
                card = card.child(div().pt_3().child(line));
            }
            body = body.child(card);
        } else if data.hint.is_none() && loading {
            body = body.child(
                strider_ui::card(t)
                    .p_4()
                    .child(div().meta(t).child("Loading…")),
            );
        }
        if data.has_snow() || (loading && data.input.lat.is_some()) {
            body = body.child(self.snow_card(data, loading, t));
        }
        body.into_any_element()
    }
    fn weather_row(&self, i: usize, r: &WeatherRow, loading: bool, t: Theme) -> AnyElement {
        let mut head = div().w(px(150.)).flex_shrink_0().child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(r.label.clone())
                .when_some(r.note, |d, n| {
                    d.child(div().meta(t).child(format!(" · {n}")))
                }),
        );
        head = head.child(div().meta(t).readout().child(weather::pretty_date(&r.date)));
        if let (Some(lat), Some(lon)) = (r.lat, r.lon) {
            head = head.child(div().meta(t).readout().child(format!("{lat:.3}, {lon:.3}")));
        }
        let metric = |label: &'static str, value: String| {
            div()
                .w(px(96.))
                .child(div().eyebrow(t).child(label))
                .child(div().readout().child(value))
        };
        let detail = match &r.cell {
            Cell::Day(c) => {
                let d = &c.value.day;
                let first = |v: &Vec<Option<f64>>| v.first().copied().flatten();
                let wx = weather::describe(d.weather_code.first().copied().flatten());
                let hi = first(&d.temperature_2m_max)
                    .map_or("—".into(), |v| format!("{}°", v.round()));
                let lo = first(&d.temperature_2m_min)
                    .map_or("—".into(), |v| format!("{}°", v.round()));
                let chance = d
                    .precipitation_probability_max
                    .as_ref()
                    .and_then(first)
                    .map(|p| format!(" · {p:.0}%"))
                    .unwrap_or_default();
                div()
                    .flex_1()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(
                        div().w(px(130.)).child(wx.text).child(
                            div()
                                .readout()
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{hi} / {lo}")),
                        ),
                    )
                    .child(metric(
                        "Precip",
                        format!(
                            "{}″{chance}",
                            first(&d.precipitation_sum).map_or("—".into(), |v| format!("{v:.2}"))
                        ),
                    ))
                    .child(metric(
                        "Wind",
                        first(&d.wind_speed_10m_max)
                            .map_or("—".into(), |v| format!("{} mph", v.round())),
                    ))
                    .child(metric(
                        "Sunrise",
                        weather::clock(d.sunrise.first().and_then(|s| s.as_deref())),
                    ))
                    .child(metric(
                        "Sunset",
                        weather::clock(d.sunset.first().and_then(|s| s.as_deref())),
                    ))
                    .child(div().meta(t).child(match c.value.kind {
                        WxKind::Forecast => "forecast",
                        WxKind::Historical => "historical",
                    }))
                    .into_any_element()
            }
            Cell::Typical(c) => {
                let c = &c.value;
                div()
                    .flex_1()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(
                        div().w(px(130.)).child("Typical").child(
                            div()
                                .readout()
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{}° / {}°", c.hi_f, c.lo_f)),
                        ),
                    )
                    .child(metric("Avg precip", format!("{:.2}″", c.precip_in)))
                    .child(metric("Wet days", format!("{}%", c.precip_pct)))
                    .when(c.snow_pct > 0, |d| {
                        d.child(metric("Snow days", format!("{}%", c.snow_pct)))
                    })
                    .child(div().meta(t).child(format!("typical · {}-yr avg", c.years)))
                    .into_any_element()
            }
            Cell::Message(m) => div().flex_1().meta(t).child(*m).into_any_element(),
            Cell::Missing => div()
                .flex_1()
                .meta(t)
                .child(if loading { "Loading…" } else { "—" })
                .into_any_element(),
        };
        div()
            .id(("weather-row", i))
            .flex()
            .gap_3()
            .py_3()
            .border_t_1()
            .border_color(t.divider)
            .child(head)
            .child(detail)
            .into_any_element()
    }
    fn snow_card(&self, data: &PanelData, loading: bool, t: Theme) -> AnyElement {
        let depth = |inches: Option<i64>| {
            div()
                .w(px(64.))
                .readout()
                .font_weight(FontWeight::BOLD)
                .text_color(if inches.is_some_and(|n| n > 0) {
                    t.ink
                } else {
                    t.muted
                })
                .child(match inches {
                    Some(n) => format!("{n} in"),
                    None if loading => "…".into(),
                    None => "—".into(),
                })
        };
        let mut card = strider_ui::card(t)
            .p_4()
            .flex()
            .flex_col()
            .child(div().card_heading().child("Snow depth"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_3()
                    .child(
                        div()
                            .flex_1()
                            .child("On the ground now")
                            .child(div().meta(t).child("At the trip start")),
                    )
                    .child(div().meta(t).child("current"))
                    .child(depth(data.snow_now)),
            );
        for (i, r) in data.rows.iter().enumerate() {
            if !located(r.lat, r.lon) {
                continue;
            }
            let day = data.snow_days.get(i).copied().flatten();
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_2()
                    .border_t_1()
                    .border_color(t.divider)
                    .child(
                        div()
                            .w(px(150.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(r.label.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .meta(t)
                            .readout()
                            .child(weather::pretty_date(&r.date)),
                    )
                    .child(div().meta(t).child(match day {
                        Some(TripDateSnow {
                            kind: SnowKind::Observed,
                            ..
                        }) => "historical".to_owned(),
                        Some(TripDateSnow {
                            kind: SnowKind::Forecast,
                            ..
                        }) => "forecast".to_owned(),
                        Some(TripDateSnow {
                            kind: SnowKind::Typical,
                            years,
                            ..
                        }) => {
                            format!("typical · {}-yr avg", years.unwrap_or(0))
                        }
                        None => String::new(),
                    }))
                    .child(depth(day.map(|d| d.inches))),
            );
        }
        card = card.child(div().pt_3().meta(t).child(
            "Modeled by Open-Meteo at each point, so treat it as a planning signal rather than a trail report.",
        ));
        if let Some(line) = self.updated_line(data, t) {
            card = card.child(div().pt_2().child(line));
        }
        card.into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::{CacheOnly, Cell, PanelInput, Store, age, compute};
    use std::time::Duration;
    use strider_services::{HttpClient, HttpResponse, ServiceError};

    struct Clock(i64);
    impl strider_services::Clock for Clock {
        fn now(&self) -> i64 {
            self.0
        }
    }
    /// Answers the forecast request once, and every other request with a 404.
    struct Forecast;
    impl HttpClient for Forecast {
        fn get(&self, url: &str) -> Result<HttpResponse, ServiceError> {
            Ok(
                if url.contains("api.open-meteo.com/v1/forecast") && url.contains("daily=") {
                    HttpResponse {
                    status: 200,
                    body: r#"{"daily":{"time":["2025-10-10"],"weather_code":[3],"temperature_2m_max":[61.4],"temperature_2m_min":[38.2],"precipitation_sum":[0.12],"wind_speed_10m_max":[9.6],"sunrise":["2025-10-10T07:05"],"sunset":["2025-10-10T18:20"],"precipitation_probability_max":[40]}}"#.into(),
                }
                } else {
                    HttpResponse {
                        status: 404,
                        body: "{}".into(),
                    }
                },
            )
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, ServiceError> {
            Err(ServiceError::Offline)
        }
    }
    #[test]
    fn panels_read_the_cache_offline_and_say_when_it_is_old() {
        let store = Store::in_memory().expect("store");
        let input = PanelInput {
            start_date: Some("2025-10-10".into()),
            nights: 0,
            days: 1,
            lat: Some(40.0),
            lon: Some(-105.0),
            camps: vec![],
            end: None,
            elevation_ft: None,
        };
        let now = 1_760_000_000; // 2025-10-09
        // Offline with nothing cached: nothing to show, and no snow either.
        let none = compute(&store, &CacheOnly, &Clock(now), &input, Duration::ZERO);
        assert!(!none.has_weather() && !none.has_snow() && none.wants_refresh());
        // A refresh fills the cache.
        let online = compute(&store, &Forecast, &Clock(now), &input, Duration::ZERO);
        assert!(online.has_weather());
        // Read again with no connection: the same day comes from the cache.
        let cached = compute(&store, &CacheOnly, &Clock(now + 60), &input, Duration::ZERO);
        let Cell::Day(day) = &cached.rows[0].cell else {
            panic!("expected the cached forecast");
        };
        assert_eq!(day.value.day.temperature_2m_max[0], Some(61.4));
        assert_eq!(cached.updated, Some(now));
        assert!(!cached.stale);
        // Two hours later the forecast has expired; it is still shown, marked as saved data.
        let old = compute(
            &store,
            &CacheOnly,
            &Clock(now + 7200),
            &input,
            Duration::ZERO,
        );
        assert!(old.has_weather() && old.stale && old.wants_refresh());
        assert_eq!(age(now + 7200, old.updated.expect("updated")), "2 h ago");
    }
    #[test]
    fn rows_follow_campsites_then_the_finish() {
        let mut input = PanelInput {
            start_date: Some("2025-10-10".into()),
            nights: 2,
            days: 3,
            lat: None,
            lon: None,
            camps: vec![(1, Some(40.0), Some(-105.0)), (2, None, None)],
            end: Some((40.1, -105.1)),
            elevation_ft: None,
        };
        let (rows, hint) = input.day_rows();
        assert!(hint.is_none());
        let labels: Vec<_> = rows
            .iter()
            .map(|r| (r.label.as_str(), r.date.as_str(), r.note))
            .collect();
        assert_eq!(
            labels,
            [
                ("Day 1", "2025-10-10", None),
                ("Day 2", "2025-10-11", None),
                ("Day 3", "2025-10-12", Some("finish")),
            ]
        );
        input.start_date = None;
        assert_eq!(
            input.day_rows().1,
            Some("Set a start date to load weather.")
        );
    }
}
