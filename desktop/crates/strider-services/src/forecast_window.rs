//! Port of `lib/forecast-window.ts`: which NOAA WPC precipitation period best fits a trip.
use chrono::NaiveDate;
use strider_core::format::fmt_date_range;

/// (first day, last day, WMS layer id), counted in days from the forecast's start.
const PERIODS: [(i64, i64, &str); 9] = [
    (0, 0, "25"),
    (1, 1, "24"),
    (2, 2, "23"),
    (3, 4, "22"),
    (5, 6, "21"),
    (0, 1, "18"),
    (0, 2, "17"),
    (0, 4, "16"),
    (0, 6, "15"),
];
const NEXT_FIVE_DAYS: &str = "16";
const LAST_FORECAST_DAY: i64 = 6;
const UNITS: &str = "Inches of rain and melted snow";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForecastWindow {
    pub layer: String,
    pub title: String,
    pub note: String,
}

fn date(iso: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()
}
fn period(start: &str, first: i64, last: i64) -> String {
    let from = date(start)
        .and_then(|d| d.checked_add_signed(chrono::Duration::days(first)))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| start.to_string());
    fmt_date_range(Some(&from), Some((last - first) as i32)).unwrap_or(from)
}

/// `forecast_start` is the UTC date (YYYY-MM-DD) NOAA's current forecast begins on.
pub fn forecast_window(
    start_date: Option<&str>,
    nights: Option<i64>,
    forecast_start: &str,
) -> ForecastWindow {
    let next_five = |note: String| ForecastWindow {
        layer: NEXT_FIVE_DAYS.into(),
        title: "Forecast, next 5 days".into(),
        note,
    };
    let (Some(start), Some(fstart)) = (start_date.and_then(date), date(forecast_start)) else {
        return next_five(UNITS.into());
    };
    let first = (start - fstart).num_days();
    let last = first + nights.unwrap_or(0);
    if last < 0 {
        return next_five(UNITS.into());
    }
    if first > LAST_FORECAST_DAY {
        return next_five(format!(
            "Trip forecast opens 7 days before you leave. {UNITS}."
        ));
    }
    let want_first = first.max(0);
    let want_last = last.min(LAST_FORECAST_DAY);
    // Narrowest period that covers the trip; ties keep the first listed (a stable sort).
    let Some(best) = PERIODS
        .iter()
        .filter(|p| p.0 <= want_first && p.1 >= want_last)
        .min_by_key(|p| p.1 - p.0)
    else {
        return next_five(UNITS.into());
    };
    let mut notes = vec![UNITS.to_string()];
    if best.0 < want_first || best.1 > want_last {
        notes.push("Closest period NOAA publishes".into());
    }
    if last > LAST_FORECAST_DAY {
        notes.push(format!(
            "Forecast ends {}",
            period(forecast_start, LAST_FORECAST_DAY, LAST_FORECAST_DAY)
        ));
    }
    ForecastWindow {
        layer: best.2.into(),
        title: format!("Forecast, {}", period(forecast_start, best.0, best.1)),
        note: notes.join(". "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "2025-07-01";

    #[test]
    fn no_start_date_shows_next_five_days() {
        let w = forecast_window(None, Some(2), F);
        assert_eq!(
            (w.layer.as_str(), w.title.as_str()),
            ("16", "Forecast, next 5 days")
        );
        assert_eq!(w.note, UNITS);
    }
    #[test]
    fn past_trip_shows_next_five_days() {
        let w = forecast_window(Some("2025-06-20"), Some(2), F);
        assert_eq!(w.layer, "16");
    }
    #[test]
    fn far_future_trip_explains_when_forecast_opens() {
        let w = forecast_window(Some("2025-07-20"), Some(2), F);
        assert_eq!(w.layer, "16");
        assert!(
            w.note
                .starts_with("Trip forecast opens 7 days before you leave.")
        );
    }
    #[test]
    fn single_day_picks_its_own_layer() {
        let w = forecast_window(Some("2025-07-03"), Some(0), F);
        assert_eq!(w.layer, "23");
        assert_eq!(w.title, "Forecast, Jul 3, 2025");
        assert_eq!(w.note, UNITS);
    }
    #[test]
    fn multi_day_trip_prefers_the_narrowest_covering_period() {
        let w = forecast_window(Some("2025-07-01"), Some(1), F);
        assert_eq!(
            (w.layer.as_str(), w.title.as_str()),
            ("18", "Forecast, Jul 1–2, 2025")
        );
        let w = forecast_window(Some("2025-07-04"), Some(1), F);
        assert_eq!(w.layer, "22");
    }
    #[test]
    fn inexact_fit_notes_the_closest_period() {
        // Days 2..=3 are covered only by the 0-4 day period.
        let w = forecast_window(Some("2025-07-03"), Some(1), F);
        assert_eq!(w.layer, "16");
        assert_eq!(w.note, format!("{UNITS}. Closest period NOAA publishes"));
    }
    #[test]
    fn trip_running_past_the_forecast_notes_where_it_ends() {
        let w = forecast_window(Some("2025-07-06"), Some(5), F);
        assert_eq!(w.layer, "21");
        assert_eq!(w.note, format!("{UNITS}. Forecast ends Jul 7, 2025"));
    }
    #[test]
    fn trip_in_progress_clamps_to_today() {
        let w = forecast_window(Some("2025-06-29"), Some(3), F);
        // first=-2, last=1: wants days 0..=1.
        assert_eq!(w.layer, "18");
        assert_eq!(w.note, UNITS);
    }
}
