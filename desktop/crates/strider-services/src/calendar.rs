//! Calendar feeds: ICS fetching with an offline cache (`calendar_cache`), VEVENT parsing and
//! expansion into inclusive `[start_day, end_day]` events (port of `lib/calendar.ts`
//! `expandEvents`), and the month grid (port of `getCalendarData`).
//!
//! Feed addresses are the user's private URLs: they are never logged and never appear in `Debug`.
use crate::{Cached, Clock, HttpClient, Result, ServiceError, read_through, ttl};
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use rrule::{RRule, Tz, Unvalidated};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};
use strider_store::{CacheKind, CalendarTrip, Store};

/// The zone used for "local" days and for floating (zone-less) times. Use `Tz::LOCAL` for the
/// system zone, or `Tz::Tz(chrono_tz::..)` for a named one.
pub type Zone = Tz;

#[derive(Clone, PartialEq, Eq)]
pub struct Feed {
    pub id: String,
    pub label: String,
    pub color: String,
    /// Private. Never log.
    pub url: String,
}
impl std::fmt::Debug for Feed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Feed")
            .field("id", &self.id)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}
impl From<&strider_store::CalendarFeedRow> for Feed {
    fn from(r: &strider_store::CalendarFeedRow) -> Self {
        Self {
            id: r.id.clone(),
            label: r.label.clone(),
            color: r.color.clone(),
            url: r.url.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalEvent {
    pub feed_id: String,
    pub label: String,
    pub color: String,
    pub title: String,
    /// `YYYY-MM-DD`, inclusive.
    pub start_day: String,
    /// `YYYY-MM-DD`, inclusive.
    pub end_day: String,
    pub all_day: bool,
    /// `9 am` / `6:30 pm`; `None` for all-day events.
    pub time: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// Fetching

/// Raw ICS text for a feed, fresh for five minutes, refetched otherwise, and the last stored copy
/// (marked stale) when the feed cannot be reached. `webcal://` addresses are fetched over https.
pub fn fetch_feed_ics(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    feed: &Feed,
) -> Result<Cached<String>> {
    let now = clock.now();
    let url = match feed.url.trim().strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => feed.url.trim().to_string(),
    };
    let got = read_through(
        store,
        CacheKind::Calendar,
        &feed.id,
        ttl::CALENDAR,
        now,
        || {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(ServiceError::Protocol(
                    "Unsupported calendar address".into(),
                ));
            }
            let r = http.get(&url)?;
            if !r.ok() {
                return Err(r.error());
            }
            if !r.body.contains("BEGIN:VCALENDAR") {
                return Err(ServiceError::Protocol("Not a calendar feed".into()));
            }
            Ok(Some(r.body))
        },
    )?;
    got.ok_or_else(|| ServiceError::Protocol("Empty calendar feed".into()))
}

// ---------------------------------------------------------------------------------------------
// Parsing

#[derive(Clone, Copy, Debug)]
enum Stamp {
    Day(NaiveDate),
    Utc(DateTime<Utc>),
    Zoned(NaiveDateTime, chrono_tz::Tz),
    Floating(NaiveDateTime),
}
#[derive(Default, Debug)]
struct RawEvent {
    uid: String,
    summary: String,
    start: Option<Stamp>,
    end: Option<Stamp>,
    duration: Option<Duration>,
    rrule: Option<String>,
    exdates: Vec<Stamp>,
    recurrence_id: Option<Stamp>,
}

fn unfold(ics: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for line in ics.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if let Some(rest) = line.strip_prefix([' ', '\t'])
            && let Some(last) = out.last_mut()
        {
            last.push_str(rest);
        } else {
            out.push(line.to_string());
        }
    }
    out
}
/// `NAME;PARAM=V:value` into (NAME, params, value); the first colon outside quotes splits.
type Params = Vec<(String, String)>;
fn content_line(line: &str) -> Option<(String, Params, &str)> {
    let mut quoted = false;
    let colon = line.char_indices().find_map(|(i, c)| {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => return Some(i),
            _ => {}
        }
        None
    })?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|p| {
            let (k, v) = p.split_once('=')?;
            Some((
                k.trim().to_ascii_uppercase(),
                v.trim().trim_matches('"').to_string(),
            ))
        })
        .collect();
    Some((name, params, value))
}
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}
fn zone_named(name: &str) -> Option<chrono_tz::Tz> {
    if let Ok(tz) = name.parse::<chrono_tz::Tz>() {
        return Some(tz);
    }
    // Common Windows zone names written by Outlook.
    let iana = match name {
        "UTC" | "Coordinated Universal Time" => "UTC",
        "Eastern Standard Time" => "America/New_York",
        "Central Standard Time" => "America/Chicago",
        "Mountain Standard Time" => "America/Denver",
        "US Mountain Standard Time" => "America/Phoenix",
        "Pacific Standard Time" => "America/Los_Angeles",
        "Alaskan Standard Time" => "America/Anchorage",
        "Hawaiian Standard Time" => "Pacific/Honolulu",
        "GMT Standard Time" => "Europe/London",
        "W. Europe Standard Time" => "Europe/Berlin",
        _ => return None,
    };
    iana.parse().ok()
}
fn stamp(value: &str, params: &[(String, String)]) -> Option<Stamp> {
    let value = value.trim();
    let param = |k: &str| params.iter().find(|p| p.0 == k).map(|p| p.1.as_str());
    if param("VALUE") == Some("DATE") || (value.len() == 8 && !value.contains('T')) {
        return NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .map(Stamp::Day);
    }
    let (core, zulu) = match value.strip_suffix(['Z', 'z']) {
        Some(c) => (c, true),
        None => (value, false),
    };
    let naive = NaiveDateTime::parse_from_str(core, "%Y%m%dT%H%M%S")
        .or_else(|_| NaiveDateTime::parse_from_str(core, "%Y%m%dT%H%M"))
        .ok()?;
    Some(if zulu {
        Stamp::Utc(Utc.from_utc_datetime(&naive))
    } else if let Some(tz) = param("TZID").and_then(zone_named) {
        Stamp::Zoned(naive, tz)
    } else {
        Stamp::Floating(naive)
    })
}
/// `P1DT2H30M`, `PT45M`, `P2W`.
fn duration(value: &str) -> Option<Duration> {
    let v = value.trim();
    let (neg, v) = match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v.strip_prefix('+').unwrap_or(v)),
    };
    let v = v.strip_prefix('P')?;
    let (mut secs, mut num, mut time) = (0_i64, String::new(), false);
    for c in v.chars() {
        match c {
            'T' => time = true,
            '0'..='9' => num.push(c),
            _ => {
                let n: i64 = num.parse().ok()?;
                num.clear();
                secs += n * match (c, time) {
                    ('W', _) => 604_800,
                    ('D', _) => 86_400,
                    ('H', true) => 3_600,
                    ('M', true) => 60,
                    ('S', true) => 1,
                    _ => return None,
                };
            }
        }
    }
    Some(Duration::seconds(if neg { -secs } else { secs }))
}

fn parse_ics(ics: &str) -> Vec<RawEvent> {
    let mut events = vec![];
    let mut stack: Vec<String> = vec![];
    let mut current: Option<RawEvent> = None;
    for line in unfold(ics) {
        let Some((name, params, value)) = content_line(&line) else {
            continue;
        };
        match name.as_str() {
            "BEGIN" => {
                let comp = value.trim().to_ascii_uppercase();
                if comp == "VEVENT" && stack.last().map(String::as_str) == Some("VCALENDAR") {
                    current = Some(RawEvent::default());
                }
                stack.push(comp);
            }
            "END" => {
                if stack.pop().as_deref() == Some("VEVENT")
                    && let Some(e) = current.take()
                {
                    events.push(e);
                }
            }
            _ if stack.last().map(String::as_str) == Some("VEVENT") => {
                let Some(e) = current.as_mut() else { continue };
                match name.as_str() {
                    "UID" => e.uid = value.trim().to_string(),
                    "SUMMARY" => e.summary = unescape(value),
                    "DTSTART" => e.start = stamp(value, &params),
                    "DTEND" => e.end = stamp(value, &params),
                    "DURATION" => e.duration = duration(value),
                    "RRULE" => e.rrule = Some(value.trim().to_string()),
                    "EXDATE" => e
                        .exdates
                        .extend(value.split(',').filter_map(|v| stamp(v, &params))),
                    "RECURRENCE-ID" => e.recurrence_id = stamp(value, &params),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    events
}

// ---------------------------------------------------------------------------------------------
// Expansion

fn local(tz: Tz, n: NaiveDateTime) -> Option<DateTime<Tz>> {
    match tz.from_local_datetime(&n) {
        LocalResult::Single(d) => Some(d),
        LocalResult::Ambiguous(first, _) => Some(first),
        // A time skipped by a DST jump lands an hour later.
        LocalResult::None => tz.from_local_datetime(&(n + Duration::hours(1))).earliest(),
    }
}
fn midnight_utc(d: NaiveDate) -> Option<DateTime<Tz>> {
    Tz::UTC
        .from_local_datetime(&d.and_hms_opt(0, 0, 0)?)
        .single()
}
fn instant(s: Stamp, display: Tz) -> Option<DateTime<Tz>> {
    match s {
        Stamp::Day(d) => midnight_utc(d),
        Stamp::Utc(t) => Some(t.with_timezone(&Tz::UTC)),
        Stamp::Zoned(n, z) => local(Tz::Tz(z), n),
        Stamp::Floating(n) => local(display, n),
    }
}
fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}
fn time_label(t: &DateTime<Tz>) -> String {
    let (h, m) = (t.hour(), t.minute());
    let ap = if h < 12 { "am" } else { "pm" };
    let h = if h % 12 == 0 { 12 } else { h % 12 };
    if m == 0 {
        format!("{h} {ap}")
    } else {
        format!("{h}:{m:02} {ap}")
    }
}
struct Ctx<'a> {
    feed: &'a Feed,
    display: Tz,
}
impl Ctx<'_> {
    fn event(
        &self,
        title: &str,
        start_day: String,
        end_day: String,
        all_day: bool,
        time: Option<String>,
    ) -> CalEvent {
        CalEvent {
            feed_id: self.feed.id.clone(),
            label: self.feed.label.clone(),
            color: self.feed.color.clone(),
            title: if title.is_empty() {
                "(untitled)".into()
            } else {
                title.into()
            },
            end_day: end_day.max(start_day.clone()),
            start_day,
            all_day,
            time,
        }
    }
    /// An all-day span of `days` days from `start`.
    fn all_day(&self, title: &str, start: NaiveDate, days: i64) -> CalEvent {
        let last = start + Duration::days((days - 1).max(0));
        self.event(title, iso(start), iso(last), true, None)
    }
    fn timed(&self, title: &str, start: &DateTime<Tz>, end: &DateTime<Tz>) -> CalEvent {
        let (s, e) = (
            start.with_timezone(&self.display),
            end.with_timezone(&self.display),
        );
        self.event(
            title,
            iso(s.date_naive()),
            iso(e.date_naive()),
            false,
            Some(time_label(&s)),
        )
    }
    /// A single (non-recurring or overriding) VEVENT.
    fn single(&self, e: &RawEvent) -> Option<CalEvent> {
        let start = e.start?;
        if let Stamp::Day(d) = start {
            let days = match (e.end, e.duration) {
                (Some(Stamp::Day(end)), _) => (end - d).num_days(),
                (_, Some(dur)) => dur.num_days(),
                _ => 1,
            };
            return Some(self.all_day(&e.summary, d, days));
        }
        let s = instant(start, self.display)?;
        let end = e
            .end
            .and_then(|x| instant(x, self.display))
            .or_else(|| e.duration.map(|d| s + d))
            .unwrap_or(s);
        Some(self.timed(&e.summary, &s, &end))
    }
}

#[derive(Hash, PartialEq, Eq)]
enum Key {
    Day(NaiveDate),
    At(i64),
}
fn key(s: Stamp, display: Tz) -> Option<Key> {
    match s {
        Stamp::Day(d) => Some(Key::Day(d)),
        other => Some(Key::At(instant(other, display)?.timestamp())),
    }
}
fn build_rrule(rule: &str, start: DateTime<Tz>) -> Option<rrule::RRuleSet> {
    let rule = rule.strip_prefix("RRULE:").unwrap_or(rule);
    let parsed = RRule::<Unvalidated>::from_str(rule).ok()?;
    if let Ok(set) = parsed.build(start) {
        return Some(set);
    }
    // A date-only UNTIL on a date-time series (or the reverse) is a common feed quirk: end the
    // series at the close of that day instead.
    let fixed: Vec<String> = rule
        .split(';')
        .map(|p| match p.strip_prefix("UNTIL=") {
            Some(v) if v.len() == 8 => format!("UNTIL={v}T235959Z"),
            _ => p.to_string(),
        })
        .collect();
    RRule::<Unvalidated>::from_str(&fixed.join(";"))
        .ok()?
        .build(start)
        .ok()
}

/// Expand one feed's VEVENTs, including RRULE series with EXDATE and RECURRENCE-ID overrides,
/// to those overlapping `[from, to]` (inclusive days in the display zone). Recurrence is
/// evaluated in the event's own zone, so a 9 am series keeps 9 am across DST.
pub fn expand_feed(
    ics: &str,
    feed: &Feed,
    from: NaiveDate,
    to: NaiveDate,
    display: Zone,
) -> Vec<CalEvent> {
    let ctx = Ctx { feed, display };
    let raw = parse_ics(ics);
    let (from_iso, to_iso) = (iso(from), iso(to));
    let overlaps = |e: &CalEvent| e.end_day >= from_iso && e.start_day <= to_iso;

    // Occurrences replaced by a RECURRENCE-ID override, per series.
    let mut replaced: HashMap<&str, HashSet<Key>> = HashMap::new();
    for e in raw.iter().filter(|e| e.recurrence_id.is_some()) {
        if let Some(k) = e.recurrence_id.and_then(|s| key(s, display)) {
            replaced.entry(e.uid.as_str()).or_default().insert(k);
        }
    }
    let mut out: Vec<(CalEvent, i64)> = vec![];
    for e in &raw {
        let Some(start) = e.start else { continue };
        let series = e
            .recurrence_id
            .is_none()
            .then_some(())
            .and(e.rrule.as_deref());
        let Some(rule) = series else {
            if let Some(ev) = ctx.single(e).filter(overlaps) {
                out.push((ev, instant(start, display).map_or(0, |d| d.timestamp())));
            }
            continue;
        };
        let all_day = matches!(start, Stamp::Day(_));
        let Some(first) = instant(start, display) else {
            continue;
        };
        let span = if all_day {
            let days = match (e.end, e.duration) {
                (Some(Stamp::Day(end)), _) => (end - first.date_naive()).num_days(),
                (_, Some(d)) => d.num_days(),
                _ => 1,
            };
            Duration::days(days.max(1))
        } else {
            let end = e
                .end
                .and_then(|x| instant(x, display))
                .or_else(|| e.duration.map(|d| first + d));
            end.map_or(Duration::zero(), |end| (end - first).max(Duration::zero()))
        };
        let Some(mut set) = build_rrule(rule, first) else {
            // Unusable rule: show the first occurrence only.
            if let Some(ev) = ctx.single(e).filter(overlaps) {
                out.push((ev, first.timestamp()));
            }
            continue;
        };
        set = set.set_exdates(
            e.exdates
                .iter()
                .filter_map(|x| instant(*x, display))
                .collect(),
        );
        // Occurrences starting inside the window, or early enough to still reach into it.
        let (lo, hi) = if all_day {
            (
                midnight_utc(from),
                from.succ_opt().and(midnight_utc(to + Duration::days(1))),
            )
        } else {
            (
                local(display, from.and_hms_opt(0, 0, 0).unwrap_or_default()),
                local(
                    display,
                    (to + Duration::days(1))
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default(),
                ),
            )
        };
        let (Some(lo), Some(hi)) = (lo, hi) else {
            continue;
        };
        let set = set.after(lo - span).before(hi - Duration::seconds(1));
        let skip = replaced.get(e.uid.as_str());
        for occ in set.all(u16::MAX).dates {
            let k = if all_day {
                Key::Day(occ.date_naive())
            } else {
                Key::At(occ.timestamp())
            };
            if skip.is_some_and(|s| s.contains(&k)) {
                continue;
            }
            let ev = if all_day {
                ctx.all_day(&e.summary, occ.date_naive(), span.num_days())
            } else {
                ctx.timed(&e.summary, &occ, &(occ + span))
            };
            if overlaps(&ev) {
                out.push((ev, occ.timestamp()));
            }
        }
    }
    out.sort_by(|a, b| {
        (&a.0.start_day, !a.0.all_day, a.1, &a.0.title).cmp(&(
            &b.0.start_day,
            !b.0.all_day,
            b.1,
            &b.0.title,
        ))
    });
    out.into_iter().map(|(e, _)| e).collect()
}

// ---------------------------------------------------------------------------------------------
// Month grid

pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalCell {
    pub iso: String,
    pub day: u32,
    pub in_month: bool,
    pub events: Vec<CalEvent>,
    pub trip_ids: Vec<String>,
    /// A Trip day that also has an event.
    pub conflict: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedInfo {
    pub id: String,
    pub label: String,
    pub color: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalTrip {
    pub id: String,
    pub name: String,
    pub start_day: String,
    pub end_day: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarData {
    pub year: i32,
    /// 0-indexed.
    pub month: u32,
    pub label: String,
    pub cells: Vec<CalCell>,
    pub feeds: Vec<FeedInfo>,
    pub trips: Vec<CalTrip>,
}

/// First and last day of the 42-cell Sunday-start grid for a month (0-indexed).
pub fn grid_range(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)> {
    let first = NaiveDate::from_ymd_opt(year, month + 1, 1)?;
    let start = first - Duration::days(i64::from(first.weekday().num_days_from_sunday()));
    Some((start, start + Duration::days(41)))
}

/// Pure grid builder over store data and already-expanded events.
pub fn calendar_data(
    year: i32,
    month: u32,
    feeds: &[Feed],
    trips: &[CalendarTrip],
    events: &[CalEvent],
) -> Option<CalendarData> {
    let (from, to) = grid_range(year, month)?;
    let (grid_start, grid_end) = (iso(from), iso(to));
    let trips: Vec<CalTrip> = trips
        .iter()
        .filter_map(|t| {
            let start = NaiveDate::parse_from_str(&t.start_date, "%Y-%m-%d").ok()?;
            let end = start + Duration::days(t.nights.max(0));
            Some(CalTrip {
                id: t.sync_id.clone(),
                name: t.name.clone(),
                start_day: t.start_date.clone(),
                end_day: iso(end),
            })
        })
        .filter(|t| t.end_day >= grid_start && t.start_day <= grid_end)
        .collect();
    let cells = (0..42)
        .map(|i| {
            let d = from + Duration::days(i);
            let day_iso = iso(d);
            let day_events: Vec<CalEvent> = events
                .iter()
                .filter(|e| e.start_day <= day_iso && e.end_day >= day_iso)
                .cloned()
                .collect();
            let trip_ids: Vec<String> = trips
                .iter()
                .filter(|t| t.start_day <= day_iso && t.end_day >= day_iso)
                .map(|t| t.id.clone())
                .collect();
            CalCell {
                day: d.day(),
                in_month: d.month0() == month,
                conflict: !trip_ids.is_empty() && !day_events.is_empty(),
                iso: day_iso,
                events: day_events,
                trip_ids,
            }
        })
        .collect();
    Some(CalendarData {
        year,
        month,
        label: format!("{} {year}", MONTHS[month as usize % 12]),
        cells,
        feeds: feeds
            .iter()
            .map(|f| FeedInfo {
                id: f.id.clone(),
                label: f.label.clone(),
                color: f.color.clone(),
            })
            .collect(),
        trips,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedStatus {
    pub feed_id: String,
    /// When the shown events were fetched; `None` if the feed was never reachable.
    pub fetched_at: Option<i64>,
    pub stale: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarMonth {
    pub data: CalendarData,
    pub status: Vec<FeedStatus>,
}

/// Load a month: store feeds and Trips, each feed's ICS (cache-backed, so it works offline),
/// expansion, and the grid.
pub fn load_month(
    store: &Store,
    http: &dyn HttpClient,
    clock: &dyn Clock,
    year: i32,
    month: u32,
    display: Zone,
) -> Result<CalendarMonth> {
    let (from, to) =
        grid_range(year, month).ok_or_else(|| ServiceError::Protocol("Invalid month".into()))?;
    let feeds: Vec<Feed> = store.calendar_feeds()?.iter().map(Feed::from).collect();
    let (mut events, mut status) = (vec![], vec![]);
    for feed in &feeds {
        match fetch_feed_ics(store, http, clock, feed) {
            Ok(c) => {
                events.extend(expand_feed(&c.value, feed, from, to, display));
                status.push(FeedStatus {
                    feed_id: feed.id.clone(),
                    fetched_at: Some(c.fetched_at),
                    stale: c.stale,
                    error: None,
                });
            }
            Err(e) => status.push(FeedStatus {
                feed_id: feed.id.clone(),
                fetched_at: None,
                stale: false,
                error: Some(e.to_string()),
            }),
        }
    }
    let trips = store.calendar_trips()?;
    let data = calendar_data(year, month, &feeds, &trips, &events)
        .ok_or_else(|| ServiceError::Protocol("Invalid month".into()))?;
    Ok(CalendarMonth { data, status })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::{FakeHttp, FixedClock};
    use chrono_tz::America::Denver;

    const FIXTURE: &str = include_str!("../../../fixtures/http/calendar-feed.ics");

    fn feed() -> Feed {
        Feed {
            id: "f1".into(),
            label: "Work".into(),
            color: "#1f7a70".into(),
            url: "https://calendar.example/private/abc.ics".into(),
        }
    }
    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn ics(body: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{body}END:VCALENDAR\r\n")
    }
    fn expand(body: &str, from: NaiveDate, to: NaiveDate, zone: Zone) -> Vec<CalEvent> {
        expand_feed(&ics(body), &feed(), from, to, zone)
    }
    const DENVER: Zone = Tz::Tz(Denver);
    fn summary(e: &[CalEvent]) -> Vec<(String, String, String, Option<String>)> {
        e.iter()
            .map(|e| {
                (
                    e.title.clone(),
                    e.start_day.clone(),
                    e.end_day.clone(),
                    e.time.clone(),
                )
            })
            .collect()
    }
    fn row(
        t: &str,
        s: &str,
        e: &str,
        time: Option<&str>,
    ) -> (String, String, String, Option<String>) {
        (t.into(), s.into(), e.into(), time.map(String::from))
    }

    #[test]
    fn parses_folded_lines_escapes_and_skips_alarms() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Dinner\\, with\\n a very long\r\n  folded title\r\nDTSTART:20250115T020000Z\r\nBEGIN:VALARM\r\nSUMMARY:ignored\r\nDTSTART:20240101T000000Z\r\nEND:VALARM\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), Tz::UTC);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].title, "Dinner, with\n a very long folded title");
        assert_eq!(ev[0].label, "Work");
    }
    #[test]
    fn utc_times_are_shown_in_the_display_zone() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Call\r\nDTSTART:20250115T013000Z\r\nDTEND:20250115T023000Z\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), DENVER);
        assert_eq!(
            summary(&ev),
            vec![row("Call", "2025-01-14", "2025-01-14", Some("6:30 pm"))]
        );
        let utc = expand(body, d(2025, 1, 1), d(2025, 1, 31), Tz::UTC);
        assert_eq!(
            summary(&utc),
            vec![row("Call", "2025-01-15", "2025-01-15", Some("1:30 am"))]
        );
    }
    #[test]
    fn tzid_and_floating_times_keep_their_wall_clock() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Zoned\r\nDTSTART;TZID=America/New_York:20250115T090000\r\nDTEND;TZID=America/New_York:20250115T100000\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nSUMMARY:Floating\r\nDTSTART:20250116T073000\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:c\r\nSUMMARY:Outlook\r\nDTSTART;TZID=\"Mountain Standard Time\":20250117T120000\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), DENVER);
        assert_eq!(
            summary(&ev),
            vec![
                row("Zoned", "2025-01-15", "2025-01-15", Some("7 am")),
                row("Floating", "2025-01-16", "2025-01-16", Some("7:30 am")),
                row("Outlook", "2025-01-17", "2025-01-17", Some("12 pm")),
            ]
        );
    }
    #[test]
    fn all_day_end_is_exclusive_and_missing_end_is_one_day() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Trip\r\nDTSTART;VALUE=DATE:20250110\r\nDTEND;VALUE=DATE:20250113\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20250120\r\nEND:VEVENT\r\n";
        // All-day days never shift with the display zone.
        for zone in [DENVER, Tz::UTC, Tz::Tz(chrono_tz::Pacific::Auckland)] {
            let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), zone);
            assert_eq!(
                summary(&ev),
                vec![
                    row("Trip", "2025-01-10", "2025-01-12", None),
                    row("Holiday", "2025-01-20", "2025-01-20", None)
                ]
            );
            assert!(ev.iter().all(|e| e.all_day));
        }
    }
    #[test]
    fn timed_event_ending_after_midnight_spans_two_days_and_untitled_is_named() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20250115T220000Z\r\nDTEND:20250116T020000Z\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), Tz::UTC);
        assert_eq!(
            summary(&ev),
            vec![row("(untitled)", "2025-01-15", "2025-01-16", Some("10 pm"))]
        );
    }
    #[test]
    fn events_outside_the_window_are_dropped() {
        let body = "BEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Old\r\nDTSTART:20240101T120000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nSUMMARY:Edge\r\nDTSTART;VALUE=DATE:20250101\r\nDTEND;VALUE=DATE:20250103\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 2), d(2025, 1, 31), Tz::UTC);
        assert_eq!(
            summary(&ev),
            vec![row("Edge", "2025-01-01", "2025-01-02", None)]
        );
    }
    #[test]
    fn weekly_series_keeps_local_time_across_dst() {
        // Denver springs forward on 2025-03-09: the offset changes, the 9 am wall clock does not.
        let body = "BEGIN:VEVENT\r\nUID:w\r\nSUMMARY:Standup\r\nDTSTART;TZID=America/Denver:20250303T090000\r\nDTEND;TZID=America/Denver:20250303T093000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 3, 1), d(2025, 3, 31), DENVER);
        assert_eq!(
            summary(&ev),
            vec![
                row("Standup", "2025-03-03", "2025-03-03", Some("9 am")),
                row("Standup", "2025-03-10", "2025-03-10", Some("9 am")),
                row("Standup", "2025-03-17", "2025-03-17", Some("9 am")),
                row("Standup", "2025-03-24", "2025-03-24", Some("9 am")),
            ]
        );
        // The same series seen from UTC shifts by the DST hour.
        let utc = expand(body, d(2025, 3, 1), d(2025, 3, 31), Tz::UTC);
        assert_eq!(utc[0].time.as_deref(), Some("4 pm"));
        assert_eq!(utc[1].time.as_deref(), Some("3 pm"));
    }
    #[test]
    fn dst_boundary_days_land_on_the_right_local_day() {
        // 23:30 local on the fall-back night (2025-11-02) stays on that day.
        let body = "BEGIN:VEVENT\r\nUID:n\r\nSUMMARY:Late\r\nDTSTART;TZID=America/Denver:20251101T233000\r\nDTEND;TZID=America/Denver:20251102T003000\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 11, 1), d(2025, 11, 5), DENVER);
        assert_eq!(
            summary(&ev),
            vec![
                row("Late", "2025-11-01", "2025-11-02", Some("11:30 pm")),
                row("Late", "2025-11-02", "2025-11-03", Some("11:30 pm")),
                row("Late", "2025-11-03", "2025-11-04", Some("11:30 pm")),
            ]
        );
    }
    #[test]
    fn nonexistent_local_time_in_the_spring_gap_moves_forward() {
        let body = "BEGIN:VEVENT\r\nUID:g\r\nSUMMARY:Gap\r\nDTSTART;TZID=America/Denver:20250309T023000\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 3, 1), d(2025, 3, 31), DENVER);
        assert_eq!(
            summary(&ev),
            vec![row("Gap", "2025-03-09", "2025-03-09", Some("3:30 am"))]
        );
    }
    #[test]
    fn recurring_all_day_with_until_and_exdate() {
        let body = "BEGIN:VEVENT\r\nUID:s\r\nSUMMARY:Market\r\nDTSTART;VALUE=DATE:20250601\r\nRRULE:FREQ=WEEKLY;BYDAY=SA;UNTIL=20250628\r\nEXDATE;VALUE=DATE:20250614\r\nEND:VEVENT\r\n";
        for zone in [DENVER, Tz::UTC] {
            let ev = expand(body, d(2025, 6, 1), d(2025, 7, 31), zone);
            assert_eq!(
                summary(&ev),
                vec![
                    row("Market", "2025-06-07", "2025-06-07", None),
                    row("Market", "2025-06-21", "2025-06-21", None),
                    row("Market", "2025-06-28", "2025-06-28", None),
                ],
                "zone {}",
                zone.name()
            );
        }
    }
    #[test]
    fn multi_day_recurring_all_day_reaches_into_the_window_from_before_it() {
        let body = "BEGIN:VEVENT\r\nUID:m\r\nSUMMARY:Camp\r\nDTSTART;VALUE=DATE:20250530\r\nDTEND;VALUE=DATE:20250602\r\nRRULE:FREQ=MONTHLY;COUNT=3\r\nEND:VEVENT\r\n";
        // The May 30 - Jun 1 occurrence overlaps a window that starts on June 1.
        let ev = expand(body, d(2025, 6, 1), d(2025, 6, 30), Tz::UTC);
        assert_eq!(
            summary(&ev),
            vec![
                row("Camp", "2025-05-30", "2025-06-01", None),
                row("Camp", "2025-06-30", "2025-07-02", None)
            ]
        );
    }
    #[test]
    fn exdate_and_recurrence_id_override_replace_one_occurrence() {
        let body = "BEGIN:VEVENT\r\nUID:r\r\nSUMMARY:Yoga\r\nDTSTART;TZID=America/Denver:20250106T180000\r\nDTEND;TZID=America/Denver:20250106T190000\r\nRRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=4\r\nEXDATE;TZID=America/Denver:20250120T180000\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:r\r\nRECURRENCE-ID;TZID=America/Denver:20250113T180000\r\nSUMMARY:Yoga (moved)\r\nDTSTART;TZID=America/Denver:20250114T070000\r\nDTEND;TZID=America/Denver:20250114T080000\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), DENVER);
        assert_eq!(
            summary(&ev),
            vec![
                row("Yoga", "2025-01-06", "2025-01-06", Some("6 pm")),
                row("Yoga (moved)", "2025-01-14", "2025-01-14", Some("7 am")),
                row("Yoga", "2025-01-27", "2025-01-27", Some("6 pm")),
            ]
        );
    }
    #[test]
    fn invalid_rules_fall_back_to_the_first_occurrence() {
        let body = "BEGIN:VEVENT\r\nUID:x\r\nSUMMARY:Odd\r\nDTSTART:20250105T120000Z\r\nRRULE:FREQ=SOMETIMES\r\nEND:VEVENT\r\n";
        let ev = expand(body, d(2025, 1, 1), d(2025, 1, 31), Tz::UTC);
        assert_eq!(ev.len(), 1);
    }
    #[test]
    fn fixture_feed_expands_every_kind_for_the_month() {
        let ev = expand_feed(FIXTURE, &feed(), d(2025, 9, 1), d(2025, 9, 30), DENVER);
        let titles: Vec<_> = ev.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles.iter().filter(|t| **t == "Team sync").count(), 4);
        assert!(titles.contains(&"Wedding weekend") && titles.contains(&"Dentist"));
        assert!(!titles.contains(&"Cancelled sync (excluded)"));
        let sorted = ev.windows(2).all(|w| w[0].start_day <= w[1].start_day);
        assert!(sorted);
    }

    fn event(day: &str, end: &str) -> CalEvent {
        CalEvent {
            feed_id: "f1".into(),
            label: "Work".into(),
            color: "#000".into(),
            title: "E".into(),
            start_day: day.into(),
            end_day: end.into(),
            all_day: true,
            time: None,
        }
    }
    #[test]
    fn month_grid_is_six_sunday_start_weeks_with_trips_events_and_conflicts() {
        // February 2026 starts on a Sunday: the grid is Feb 1 - Mar 14.
        let trips = vec![
            CalendarTrip {
                sync_id: "t1".into(),
                name: "Loop".into(),
                start_date: "2026-02-13".into(),
                nights: 2,
            },
            CalendarTrip {
                sync_id: "t2".into(),
                name: "Far".into(),
                start_date: "2027-01-01".into(),
                nights: 1,
            },
        ];
        let events = vec![
            event("2026-02-14", "2026-02-14"),
            event("2026-02-20", "2026-02-21"),
        ];
        let g = calendar_data(2026, 1, &[feed()], &trips, &events).unwrap();
        assert_eq!((g.label.as_str(), g.cells.len()), ("February 2026", 42));
        assert_eq!(
            (g.cells[0].iso.as_str(), g.cells[0].day, g.cells[0].in_month),
            ("2026-02-01", 1, true)
        );
        assert_eq!(
            (g.cells[41].iso.as_str(), g.cells[41].in_month),
            ("2026-03-14", false)
        );
        assert_eq!(g.trips.len(), 1, "trips outside the grid are dropped");
        assert_eq!(g.trips[0].end_day, "2026-02-15");
        let cell = |iso: &str| g.cells.iter().find(|c| c.iso == iso).unwrap();
        assert!(cell("2026-02-14").conflict && cell("2026-02-14").trip_ids == vec!["t1"]);
        assert!(!cell("2026-02-13").conflict && !cell("2026-02-20").conflict);
        assert_eq!(cell("2026-02-21").events.len(), 1);
        assert_eq!(g.feeds[0].label, "Work");
        // A month whose first day is a Wednesday leads with the previous month.
        let (from, to) = grid_range(2025, 6).unwrap();
        assert_eq!((from, to), (d(2025, 6, 29), d(2025, 8, 9)));
    }

    fn fetch(store: &Store, http: &FakeHttp, c: &FixedClock) -> Result<Cached<String>> {
        fetch_feed_ics(store, http, c, &feed())
    }
    #[test]
    fn feed_ics_is_cached_for_five_minutes_and_served_stale_offline() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("calendar.example", 200, FIXTURE);
        let c = FixedClock::at(10_000);
        assert!(!fetch(&store, &http, &c).unwrap().stale);
        c.advance(120);
        fetch(&store, &http, &c).unwrap();
        assert_eq!(http.calls(), 1);
        c.advance(600);
        http.clear();
        let stale = fetch(&store, &http, &c).unwrap();
        assert!(stale.stale && stale.fetched_at == 10_000 && stale.value == FIXTURE);
    }
    #[test]
    fn feed_failures_without_a_copy_are_errors_and_html_is_not_cached() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        let c = FixedClock::at(10_000);
        assert_eq!(fetch(&store, &http, &c).unwrap_err(), ServiceError::Offline);
        http.route("calendar.example", 200, "<html>login</html>");
        assert!(matches!(
            fetch(&store, &http, &c),
            Err(ServiceError::Protocol(_))
        ));
        http.clear();
        http.route("calendar.example", 403, "");
        assert_eq!(
            fetch(&store, &http, &c).unwrap_err(),
            ServiceError::Unauthorized
        );
        assert!(
            store
                .cache_get(CacheKind::Calendar, "f1")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn webcal_addresses_are_fetched_over_https_and_other_schemes_are_refused() {
        let store = Store::in_memory().unwrap();
        let http = FakeHttp::default();
        http.route("https://calendar.example/", 200, FIXTURE);
        let c = FixedClock::at(1);
        let mut f = feed();
        f.url = "webcal://calendar.example/p.ics".into();
        fetch_feed_ics(&store, &http, &c, &f).unwrap();
        assert!(http.log.lock().unwrap()[0].starts_with("https://calendar.example/p.ics"));
        f.url = "file:///etc/passwd".into();
        f.id = "f2".into();
        assert!(matches!(
            fetch_feed_ics(&store, &http, &c, &f),
            Err(ServiceError::Protocol(_))
        ));
        assert_eq!(http.calls(), 1);
    }
    #[test]
    fn feed_debug_output_hides_the_private_url() {
        let text = format!("{:?}", feed());
        assert!(!text.contains("calendar.example") && !text.contains("abc.ics"));
    }
    #[test]
    fn load_month_combines_store_feeds_trips_and_cached_ics_offline() {
        use rusqlite::types::Value;
        use strider_store::Command;
        let mut store = Store::in_memory().unwrap();
        store
            .execute(Command::AddCalendarFeed {
                label: "Work".into(),
                color: "#1f7a70".into(),
                url: "https://calendar.example/p.ics".into(),
            })
            .unwrap();
        store
            .execute(Command::CreateTrip {
                name: "Loop".into(),
                trail: None,
                loadout: None,
            })
            .unwrap();
        let trip = store.trips_list().unwrap().upcoming[0].sync_id.clone();
        let patch = [
            ("start_date".to_string(), Value::Text("2025-09-10".into())),
            ("nights".to_string(), Value::Integer(2)),
        ]
        .into_iter()
        .collect();
        store
            .execute(Command::UpdateTrip {
                trip: trip.clone(),
                patch,
            })
            .unwrap();
        let http = FakeHttp::default();
        http.route("calendar.example", 200, FIXTURE);
        let c = FixedClock::at(2_000);
        let online = load_month(&store, &http, &c, 2025, 8, DENVER).unwrap();
        assert_eq!(online.status[0].error, None);
        assert_eq!(online.data.trips.len(), 1);
        let busy = online.data.cells.iter().filter(|x| x.conflict).count();
        assert!(online.data.cells.iter().any(|x| !x.events.is_empty()));
        // Offline later: same grid from the cached ICS, flagged stale.
        c.advance(3_600);
        http.clear();
        let offline = load_month(&store, &http, &c, 2025, 8, DENVER).unwrap();
        assert!(offline.status[0].stale && offline.status[0].fetched_at == Some(2_000));
        assert_eq!(offline.data.cells, online.data.cells);
        assert_eq!(
            offline.data.cells.iter().filter(|x| x.conflict).count(),
            busy
        );
        // Deleting the feed drops its cache and its events.
        let id = store.calendar_feeds().unwrap()[0].id.clone();
        store.execute(Command::DeleteCalendarFeed { id }).unwrap();
        assert!(
            store
                .cache_get(CacheKind::Calendar, "f1")
                .unwrap()
                .is_none()
        );
        let none = load_month(&store, &http, &c, 2025, 8, DENVER).unwrap();
        assert!(none.status.is_empty() && none.data.cells.iter().all(|x| x.events.is_empty()));
        assert_eq!(none.data.trips[0].id, trip);
    }
}
