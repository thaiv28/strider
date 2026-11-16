//! Settings reads and the calendar-trip projection (parity with `app/settings` and `app/calendar`).
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct EnergySettings {
    pub bmr: i64,
    pub cal_per_mile: i64,
    pub ft_per_mile: i64,
    pub sex: Option<String>,
    pub weight_lb: Option<Dec>,
    pub height_in: Option<Dec>,
    pub age_years: Option<i32>,
}
impl Default for EnergySettings {
    fn default() -> Self {
        Self {
            bmr: 1735,
            cal_per_mile: 200,
            ft_per_mile: 625,
            sex: None,
            weight_lb: None,
            height_in: None,
            age_years: None,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReportSettings {
    pub template: Option<String>,
    pub packing_default: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarFeedRow {
    pub id: String,
    pub label: String,
    pub color: String,
    /// The user's private feed address. Never log it.
    pub url: String,
    pub sort_order: i64,
}
/// A Trip with a start date, as the Calendar places it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarTrip {
    pub sync_id: String,
    pub name: String,
    pub start_date: String,
    pub nights: i64,
}
/// Trips shared with this user, grouped as the Logbook "Shared with me" section.
#[derive(Clone, Debug, Default)]
pub struct SharedTrips {
    pub edit: Vec<TripSummary>,
    pub view: Vec<TripSummary>,
}
impl Store {
    pub fn energy_settings(&self) -> Result<EnergySettings> {
        let Some(r) = rows(&self.connection, "energy_params")?.into_iter().next() else {
            return Ok(EnergySettings::default());
        };
        Ok(EnergySettings {
            bmr: n(&r, "bmr"),
            cal_per_mile: n(&r, "cal_per_energy_mile"),
            ft_per_mile: n(&r, "ft_per_energy_mile"),
            sex: opt(&r, "sex"),
            weight_lb: dec(&r, "weight_lb")?,
            height_in: dec(&r, "height_in")?,
            age_years: match r.get("age_years") {
                Some(Value::Integer(v)) => i32::try_from(*v).ok(),
                _ => None,
            },
        })
    }
    pub fn report_settings(&self) -> Result<ReportSettings> {
        Ok(rows(&self.connection, "report_settings")?
            .into_iter()
            .next()
            .map(|r| ReportSettings {
                template: opt(&r, "template"),
                packing_default: opt(&r, "packing_default"),
            })
            .unwrap_or_default())
    }
    pub fn calendar_feeds(&self) -> Result<Vec<CalendarFeedRow>> {
        let mut feeds: Vec<_> = rows(&self.connection, "calendar_feed")?
            .into_iter()
            .map(|r| CalendarFeedRow {
                id: s(&r, "sync_id").into(),
                label: s(&r, "label").into(),
                color: s(&r, "color").into(),
                url: s(&r, "url").into(),
                sort_order: n(&r, "sort_order"),
            })
            .collect();
        feeds.sort_by(|a, b| (a.sort_order, &a.id).cmp(&(b.sort_order, &b.id)));
        Ok(feeds)
    }
    /// The user's own dated Trips (shared Trips are not placed on the Calendar).
    pub fn calendar_trips(&self) -> Result<Vec<CalendarTrip>> {
        Ok(rows(&self.connection, "trip")?
            .into_iter()
            .filter(|r| s(r, "access") == "owner")
            .filter_map(|r| {
                Some(CalendarTrip {
                    sync_id: s(&r, "sync_id").into(),
                    name: s(&r, "name").into(),
                    start_date: opt(&r, "start_date")?,
                    nights: n(&r, "nights").max(0),
                })
            })
            .collect())
    }
    /// Trips owned by someone else, split by the permission granted to this user.
    pub fn shared_trips(&self) -> Result<SharedTrips> {
        let list = self.trips_list()?;
        let mut out = SharedTrips::default();
        for t in list
            .upcoming
            .into_iter()
            .chain(list.in_progress)
            .chain(list.past)
        {
            match t.access.as_str() {
                "edit" => out.edit.push(t),
                "view" => out.view.push(t),
                _ => {}
            }
        }
        Ok(out)
    }
}
