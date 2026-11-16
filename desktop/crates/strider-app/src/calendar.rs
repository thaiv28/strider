//! The Calendar area (port of `calendar-view.tsx`): the month grid with Trips and feed events.
//! It reads the feed cache, so it works offline and says when a feed is out of date.
use super::*;
use chrono::Datelike;
use offline::{CacheOnly, format::age};
use strider_services::{
    SystemClock,
    calendar::{CalCell, CalEvent, CalTrip, CalendarMonth, Zone, load_month},
};

pub(crate) struct CalendarState {
    pub year: i32,
    /// 0-indexed.
    pub month: u32,
    pub today: String,
    pub data: Option<CalendarMonth>,
    pub error: Option<String>,
}
impl CalendarState {
    pub fn new() -> Self {
        let today = chrono::Local::now().date_naive();
        Self {
            year: today.year(),
            month: today.month0(),
            today: today.format("%Y-%m-%d").to_string(),
            data: None,
            error: None,
        }
    }
}
/// A Trip bar across one week row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Seg {
    pub id: String,
    pub name: String,
    pub start_col: usize,
    pub end_col: usize,
    pub continues_left: bool,
    pub continues_right: bool,
    pub lane: usize,
}
/// Lay each Trip that touches a week out as a bar, stacking overlapping Trips into lanes.
pub(crate) fn week_segments(cells: &[CalCell], trips: &[CalTrip]) -> (Vec<Seg>, usize) {
    let (Some(first), Some(last)) = (cells.first(), cells.last()) else {
        return (vec![], 0);
    };
    let mut segs: Vec<Seg> = trips
        .iter()
        .filter(|t| t.end_day >= first.iso && t.start_day <= last.iso)
        .map(|t| {
            let s = cells.iter().position(|c| c.iso == t.start_day);
            let e = cells.iter().position(|c| c.iso == t.end_day);
            Seg {
                id: t.id.clone(),
                name: t.name.clone(),
                start_col: s.unwrap_or(0),
                end_col: e.unwrap_or(cells.len() - 1),
                continues_left: s.is_none(),
                continues_right: e.is_none(),
                lane: 0,
            }
        })
        .collect();
    segs.sort_by(|a, b| (a.start_col, &a.id).cmp(&(b.start_col, &b.id)));
    let mut lane_end: Vec<usize> = vec![];
    for s in &mut segs {
        match lane_end.iter().position(|end| *end < s.start_col) {
            Some(i) => {
                s.lane = i;
                lane_end[i] = s.end_col;
            }
            None => {
                s.lane = lane_end.len();
                lane_end.push(s.end_col);
            }
        }
    }
    (segs, lane_end.len())
}
type Tone = Hsla;
fn feed_color(css: &str, fallback: Tone) -> Tone {
    Rgba::try_from(css).map(Hsla::from).unwrap_or(fallback)
}
const LANE: f32 = 20.;
impl AppView {
    /// Read the visible month from the feed cache and refresh stale feeds in the background.
    pub(super) fn load_calendar(&mut self, _cx: &mut Context<Self>) {
        let (y, m) = (self.cal.year, self.cal.month);
        match load_month(&self.store, &CacheOnly, &SystemClock, y, m, Zone::LOCAL) {
            Ok(month) => {
                let wants = month.status.iter().any(|s| s.stale || s.error.is_some());
                self.cal.data = Some(month);
                self.cal.error = None;
                if wants {
                    self.refresh(format!("calendar|{y}|{m}"), move |s, h, c| {
                        let _ = load_month(s, h, c, y, m, Zone::LOCAL);
                    });
                }
            }
            Err(e) => self.cal.error = Some(e.to_string()),
        }
    }
    pub(super) fn shift_month(&mut self, delta: i32, cx: &mut Context<Self>) {
        let m = self.cal.month as i32 + delta;
        self.cal.year += m.div_euclid(12);
        self.cal.month = m.rem_euclid(12) as u32;
        self.load_calendar(cx);
        cx.notify();
    }
    pub(super) fn jump_today(&mut self, cx: &mut Context<Self>) {
        let today = chrono::Local::now().date_naive();
        self.cal.year = today.year();
        self.cal.month = today.month0();
        self.cal.today = today.format("%Y-%m-%d").to_string();
        self.load_calendar(cx);
        cx.notify();
    }
    pub(super) fn calendar_header_actions(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        div()
            .flex()
            .gap_2()
            .child(
                strider_ui::button("prev-month", "Previous", ButtonVariant::HeaderOutline, t)
                    .on_click(cx.listener(|this, _, _, cx| this.shift_month(-1, cx))),
            )
            .child(
                strider_ui::button("next-month", "Next", ButtonVariant::HeaderOutline, t)
                    .on_click(cx.listener(|this, _, _, cx| this.shift_month(1, cx))),
            )
            .child(
                strider_ui::button("today", "Today", ButtonVariant::Header, t)
                    .on_click(cx.listener(|this, _, _, cx| this.jump_today(cx))),
            )
            .into_any_element()
    }
    pub(super) fn calendar_label(&self) -> String {
        self.cal
            .data
            .as_ref()
            .map(|d| d.data.label.clone())
            .unwrap_or_default()
    }
    fn event_line(&self, e: &CalEvent, conflict: bool, t: Theme) -> Div {
        let color = feed_color(&e.color, t.accent);
        div()
            .debug_selector(|| "cal-event".to_owned())
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(12.))
            .text_color(color)
            .overflow_hidden()
            .child(div().size(px(6.)).flex_shrink_0().rounded_full().bg(color))
            .when_some(e.time.clone(), |d, time| {
                d.child(
                    div()
                        .readout()
                        .flex_shrink_0()
                        .text_color(t.muted)
                        .child(time),
                )
            })
            .child(div().truncate().child(format!(
                "{}{}",
                e.title,
                if conflict { " · conflict" } else { "" }
            )))
    }
    fn week_row(&self, w: usize, cells: &[CalCell], trips: &[CalTrip], cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let (segs, lanes) = week_segments(cells, trips);
        let mut row = div().flex();
        for c in cells {
            let today = c.iso == self.cal.today;
            row = row.child(
                div()
                    .id(SharedString::from(format!("day-{}", c.iso)))
                    .flex_1()
                    .min_w_0()
                    .min_h(px(104.))
                    .p_1p5()
                    .border_r_1()
                    .border_color(t.divider)
                    .when(!c.in_month, |d| d.bg(t.panel2).text_color(t.muted))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .readout()
                                    .text_size(px(12.))
                                    .size(px(20.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(today, |d| {
                                        d.rounded_full()
                                            .bg(t.accent)
                                            .text_color(t.accent_ink)
                                            .font_weight(FontWeight::BOLD)
                                    })
                                    .when(!today, |d| d.text_color(t.muted))
                                    .child(c.day.to_string()),
                            )
                            .when(c.conflict, |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(t.signal)
                                        .child("Conflict"),
                                )
                            }),
                    )
                    .child(div().h(px(lanes as f32 * LANE)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(
                                c.events
                                    .iter()
                                    .take(3)
                                    .map(|e| self.event_line(e, c.conflict, t)),
                            )
                            .when(c.events.len() > 3, |d| {
                                d.child(
                                    div().meta(t).child(format!("+{} more", c.events.len() - 3)),
                                )
                            }),
                    ),
            );
        }
        let bars = div()
            .absolute()
            .left_0()
            .right_0()
            .top(px(26.))
            .children(segs.iter().map(|s| {
                let id = s.id.clone();
                div()
                    .id(SharedString::from(format!("trip-bar-{w}-{}", s.id)))
                    .debug_selector(|| "trip-bar".to_owned())
                    .absolute()
                    .left(relative(s.start_col as f32 / 7.))
                    .w(relative((s.end_col - s.start_col + 1) as f32 / 7.))
                    .top(px(s.lane as f32 * LANE))
                    .h(px(18.))
                    .px_1p5()
                    .bg(t.accent)
                    .text_color(t.accent_ink)
                    .text_size(px(11.))
                    .overflow_hidden()
                    .cursor_pointer()
                    .when(!s.continues_left, |d| d.rounded_l(px(4.)).ml_0p5())
                    .when(!s.continues_right, |d| d.rounded_r(px(4.)).mr_0p5())
                    .child(format!(
                        "{}{}",
                        if s.continues_left { "‹ " } else { "" },
                        s.name
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| this.open_trip(id.clone(), cx)))
            }));
        div()
            .relative()
            .border_b_1()
            .border_color(t.divider)
            .child(row)
            .child(bars)
    }
    pub(super) fn render_calendar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        if self.cal.data.is_none() && self.cal.error.is_none() {
            self.load_calendar(cx);
        }
        let mut body = div().flex().flex_col().gap_3();
        let Some(month) = &self.cal.data else {
            let e = self.cal.error.clone().unwrap_or_default();
            return strider_ui::page_body(vec![div().meta(t).child(e).into_any_element()], t)
                .into_any_element();
        };
        let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let mut grid = strider_ui::card(t).child(
            div()
                .flex()
                .bg(t.panel2)
                .border_b_1()
                .border_color(t.divider)
                .children(
                    weekdays.map(|d| div().flex_1().py_1p5().eyebrow(t).text_center().child(d)),
                ),
        );
        for (w, cells) in month.data.cells.chunks(7).enumerate() {
            grid = grid.child(self.week_row(w, cells, &month.data.trips, cx));
        }
        body = body.child(grid);
        let mut legend = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_4()
            .gap_y_2()
            .meta(t)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().size(px(10.)).rounded(px(2.)).bg(t.accent))
                    .child("Trip"),
            );
        for f in &month.data.feeds {
            legend = legend.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .size(px(10.))
                            .rounded_full()
                            .bg(feed_color(&f.color, t.accent)),
                    )
                    .child(f.label.clone()),
            );
        }
        body = body.child(legend);
        if month.data.feeds.is_empty() {
            body = body.child(div().meta(t).child(
                "No calendars connected yet. Add a secret iCal address in Settings. Trips still show here.",
            ));
        }
        // A feed that could not be refreshed keeps its last events; say how old they are.
        let now = self.now();
        for s in &month.status {
            let label = month
                .data
                .feeds
                .iter()
                .find(|f| f.id == s.feed_id)
                .map(|f| f.label.clone())
                .unwrap_or_default();
            let note = match (s.fetched_at, s.stale, &s.error) {
                (Some(at), true, _) => Some(format!("{label}: offline, updated {}", age(now, at))),
                (None, _, Some(e)) => Some(format!("{label}: {e}")),
                _ => None,
            };
            if let Some(n) = note {
                body = body.child(div().meta(t).child(n));
            }
        }
        strider_ui::page_body(
            [div()
                .id("calendar-scroll")
                .overflow_y_scroll()
                .flex_1()
                .min_h_0()
                .child(body)
                .into_any_element()],
            t,
        )
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::{CalCell, CalTrip, week_segments};
    fn cell(iso: &str) -> CalCell {
        CalCell {
            iso: iso.into(),
            day: 1,
            in_month: true,
            events: vec![],
            trip_ids: vec![],
            conflict: false,
        }
    }
    fn trip(id: &str, a: &str, b: &str) -> CalTrip {
        CalTrip {
            id: id.into(),
            name: id.into(),
            start_day: a.into(),
            end_day: b.into(),
        }
    }
    #[test]
    fn overlapping_trips_stack_and_clip_to_the_week() {
        let week: Vec<_> = (1..=7).map(|d| cell(&format!("2026-03-0{d}"))).collect();
        let trips = [
            trip("a", "2026-02-27", "2026-03-03"),
            trip("b", "2026-03-02", "2026-03-04"),
            trip("c", "2026-03-05", "2026-03-20"),
            trip("far", "2026-04-01", "2026-04-02"),
        ];
        let (segs, lanes) = week_segments(&week, &trips);
        assert_eq!(segs.len(), 3);
        assert_eq!(lanes, 2);
        assert!(segs[0].continues_left && !segs[0].continues_right);
        assert_eq!(
            (segs[0].start_col, segs[0].end_col, segs[0].lane),
            (0, 2, 0)
        );
        assert_eq!((segs[1].start_col, segs[1].lane), (1, 1));
        assert!(segs[2].continues_right);
        assert_eq!(segs[2].lane, 0, "a finished lane is reused");
    }

    const ICS: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//t//EN\r\nBEGIN:VEVENT\r\nUID:rec@test\r\nDTSTAMP:20260101T000000Z\r\nDTSTART;VALUE=DATE:20260303\r\nDTEND;VALUE=DATE:20260304\r\nRRULE:FREQ=WEEKLY;COUNT=4\r\nSUMMARY:Piano\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    #[gpui::test]
    fn the_calendar_shows_a_trip_and_a_recurring_event_from_the_cache(
        cx: &mut gpui::TestAppContext,
    ) {
        use crate::app::test_support::{headless, with_view};
        use rusqlite::types::Value;
        use strider_store::{CacheKind, Command, Row};
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        with_view(&view, &mut cx, |g, _, cx| {
            g.command(
                Command::CreateTrip {
                    name: "March trip".into(),
                    trail: None,
                    loadout: None,
                },
                cx,
            );
            let trip = g.trips.list.upcoming.first().expect("trip").sync_id.clone();
            g.command(
                Command::UpdateTrip {
                    trip,
                    patch: Row::from([
                        ("start_date".into(), Value::Text("2026-03-10".into())),
                        ("nights".into(), Value::Integer(2)),
                    ]),
                },
                cx,
            );
            g.command(
                Command::AddCalendarFeed {
                    label: "Family".into(),
                    color: "#1f7a70".into(),
                    url: "https://example.test/family.ics".into(),
                },
                cx,
            );
            let feed = g.store.calendar_feeds().expect("feeds")[0].id.clone();
            // The feed was fetched a moment ago; nothing here touches the network.
            g.store
                .cache_put(
                    CacheKind::Calendar,
                    &feed,
                    g.now(),
                    &serde_json::to_string(ICS).expect("json"),
                )
                .expect("cache");
            (g.cal.year, g.cal.month, g.cal.data) = (2026, 2, None);
            g.area = "Calendar";
            cx.notify();
        });
        cx.run_until_parked();
        view.read_with(&cx, |g, _| {
            assert!(g.cal.error.is_none());
            let month = &g.cal.data.as_ref().expect("month").data;
            assert_eq!(month.label, "March 2026");
            assert_eq!(month.trips.len(), 1);
            assert_eq!(month.trips[0].name, "March trip");
            let with_events: Vec<_> = month
                .cells
                .iter()
                .filter(|c| !c.events.is_empty())
                .map(|c| c.iso.as_str())
                .collect();
            assert_eq!(
                with_events,
                ["2026-03-03", "2026-03-10", "2026-03-17", "2026-03-24"],
                "the weekly rule expands to four Tuesdays"
            );
            let conflict: Vec<_> = month.cells.iter().filter(|c| c.conflict).collect();
            assert_eq!(conflict.len(), 1, "the 10th has both a trip and an event");
        });
        assert!(
            cx.debug_bounds("trip-bar").is_some(),
            "the trip bar is drawn"
        );
        assert!(
            cx.debug_bounds("cal-event").is_some(),
            "feed events are drawn"
        );
        // Month navigation reads the cache again and Today returns to the current month.
        with_view(&view, &mut cx, |g, _, cx| {
            g.shift_month(1, cx);
            assert_eq!((g.cal.year, g.cal.month), (2026, 3));
            g.shift_month(-4, cx);
            assert_eq!((g.cal.year, g.cal.month), (2025, 11));
            g.jump_today(cx);
            assert_eq!(g.cal.today[..4].parse::<i32>().ok(), Some(g.cal.year));
        });
        cx.run_until_parked();
    }
}
