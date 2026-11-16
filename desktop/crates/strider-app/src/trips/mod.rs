use super::*;
use rusqlite::types::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use strider_core::gpx::{ExportCampsite, ExportPoint, build_gpx, gpx_filename};
use strider_store::{AddTripFood, Row, Site, TripDetail, TripSummary, TripsList};
use strider_ui::{ElevationProfile, RouteClick, RouteMap};
mod files;
mod page;
fn eid(prefix: &str, id: &str) -> ElementId {
    SharedString::from(format!("{prefix}-{id}")).into()
}
#[derive(Clone)]
pub(super) enum EditKind {
    New,
    Details,
    Note(String),
    Packing,
    Day(i32),
    Gear(String),
    Meal(String),
}
pub(super) struct TripEditor {
    kind: EditKind,
    fields: Vec<(&'static str, Entity<InputState>)>,
    text: Option<Entity<gpui_component::input::TextareaState>>,
    loadout: Option<String>,
}
pub(super) struct TripsState {
    pub selected: Option<String>,
    pub detail: Option<TripDetail>,
    pub list: TripsList,
    pub editor: Option<TripEditor>,
    pub map: Entity<RouteMap>,
    pub profile: Entity<ElevationProfile>,
    pub section: &'static str,
    pub route_menu: bool,
    pub minimized: bool,
    /// Logbook filter: all, planned, idea, completed or shared.
    pub filter: &'static str,
    /// Shared trips seen at the last load, id to name, to notice a revoked one.
    pub shared_known: std::collections::HashMap<String, String>,
}
impl TripsState {
    pub fn new(cx: &mut Context<AppView>) -> Self {
        Self {
            selected: None,
            detail: None,
            list: TripsList::default(),
            editor: None,
            map: cx.new(|cx| RouteMap::new(None, cx)),
            profile: cx.new(|_| ElevationProfile::default()),
            section: "Logistics",
            route_menu: false,
            minimized: false,
            filter: "all",
            shared_known: Default::default(),
        }
    }
}
pub(in crate::app) fn text(r: &Row, k: &str) -> String {
    match r.get(k) {
        Some(Value::Text(s)) => s.clone(),
        Some(Value::Integer(n)) => n.to_string(),
        Some(Value::Real(n)) => n.to_string(),
        _ => String::new(),
    }
}
pub(in crate::app) fn num(r: &Row, k: &str) -> f64 {
    text(r, k).parse().unwrap_or(0.)
}
pub(in crate::app) fn camps(d: &TripDetail) -> Vec<(String, ExportCampsite)> {
    d.campsites
        .iter()
        .map(|r| {
            (
                text(r, "sync_id"),
                ExportCampsite {
                    night: num(r, "night") as i64,
                    lat: text(r, "lat").parse().ok(),
                    lon: text(r, "lon").parse().ok(),
                    ele_ft: text(r, "ele_ft").parse().ok(),
                },
            )
        })
        .collect()
}
impl AppView {
    pub(super) fn subscribe_routes(&mut self, cx: &mut Context<Self>) {
        self._subscriptions.push(
            cx.subscribe(&self.trips.map, |this, _, event: &RouteClick, cx| {
                this.route_click(event, cx)
            }),
        );
        self._subscriptions.push(
            cx.subscribe(&self.trips.profile, |this, _, event: &RouteClick, cx| {
                this.route_click(event, cx)
            }),
        );
    }
    pub(super) fn reload_trips(&mut self, cx: &mut Context<Self>) -> strider_store::Result<()> {
        self.trips.list = self.store.borrow().trips_list()?;
        self.notice_revoked_trips();
        self.trips.detail = self
            .trips
            .selected
            .as_ref()
            .map(|id| self.store.borrow().trip_detail(id))
            .transpose()?
            .flatten();
        if self.trips.selected.is_some() && self.trips.detail.is_none() {
            self.trips.selected = None;
        }
        if let Some(d) = &self.trips.detail {
            let points = d.points.clone();
            let camps = camps(d);
            let editable = d.access != "view";
            self.trips.map.update(cx, |map, cx| {
                map.set_route(points.clone(), camps, editable, cx)
            });
            self.trips.profile.update(cx, |p, cx| {
                p.points = points;
                p.editable = editable;
                p.camps = d
                    .campsites
                    .iter()
                    .map(|r| (text(r, "sync_id"), num(r, "distance_mi")))
                    .collect();
                cx.notify();
            });
        }
        self.load_regions();
        self.load_pois();
        self.push_pois(cx);
        self.load_panels();
        Ok(())
    }
    /// A shared trip that disappears after a sync was revoked by its owner: say so.
    fn notice_revoked_trips(&mut self) {
        let now: std::collections::HashMap<String, String> = self
            .all_trips()
            .into_iter()
            .filter(|t| t.access != "owner")
            .map(|t| (t.sync_id, t.name))
            .collect();
        let gone: Vec<&String> = self
            .trips
            .shared_known
            .iter()
            .filter(|(id, _)| !now.contains_key(*id))
            .map(|(_, name)| name)
            .collect();
        match gone.as_slice() {
            [] => {}
            [name] => {
                let message = format!("{name} is no longer shared with you.");
                self.toast(message);
            }
            many => {
                let message = format!("{} trips are no longer shared with you.", many.len());
                self.toast(message);
            }
        }
        self.trips.shared_known = now;
    }
    pub(super) fn open_trip(&mut self, id: String, cx: &mut Context<Self>) {
        self.area = "Trips";
        self.trips.selected = Some(id);
        self.trips.section = "Logistics";
        self.trips.route_menu = false;
        self.panels = None;
        self.reload(cx);
    }
    pub(super) fn route_click(&mut self, event: &RouteClick, cx: &mut Context<Self>) {
        let Some(d) = &self.trips.detail else { return };
        if d.access == "view" || !event.distance.is_finite() {
            return;
        }
        let Some(p) = d.points.iter().min_by(|a, b| {
            (a.d - event.distance)
                .abs()
                .total_cmp(&(b.d - event.distance).abs())
        }) else {
            return;
        };
        let trip = text(&d.fields, "sync_id");
        let site = Site {
            distance_mi: p.d,
            lat: Some(p.lat),
            lon: Some(p.lon),
            ele_ft: Some(p.e as i32),
        };
        self.command(
            if let Some(id) = &event.campsite {
                Command::MoveCampsite {
                    trip,
                    id: id.clone(),
                    site,
                }
            } else {
                Command::AddCampsite { trip, site }
            },
            cx,
        );
    }
    pub(super) fn new_trip_dialog(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.trips.editor = Some(TripEditor {
            kind: EditKind::New,
            fields: vec![("Name", input("", false, w, cx))],
            text: None,
            loadout: self.loadout.clone(),
        });
        cx.notify();
    }
    fn edit_trip(&mut self, kind: EditKind, w: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = &self.trips.detail else { return };
        if d.access == "view" {
            return;
        }
        let mut values: Vec<(&'static str, String)> = vec![];
        let mut multiline = None;
        match &kind {
            EditKind::Details => {
                for (label, key) in [
                    ("Name", "name"),
                    ("Start date (YYYY-MM-DD)", "start_date"),
                    ("Nights", "nights"),
                    ("Party size", "party_size"),
                    ("Region", "region"),
                    ("Trailhead", "trailhead"),
                    ("Distance mi", "distance_mi"),
                    ("Gain ft", "elevation_gain_ft"),
                    ("Food g/day", "food_g_per_day"),
                    ("Water g", "water_g_per_day"),
                    ("Fuel g", "fuel_g_per_day"),
                ] {
                    values.push((label, text(&d.fields, key)));
                }
            }
            EditKind::Note(key) => multiline = Some(text(&d.fields, key)),
            EditKind::Packing => multiline = d.packing_list.clone(),
            EditKind::Day(day) => {
                let row = d
                    .days
                    .iter()
                    .find(|r| num(r, "day_number") == f64::from(*day));
                values = vec![
                    (
                        "Distance mi",
                        row.map(|r| text(r, "distance_mi")).unwrap_or_default(),
                    ),
                    (
                        "Gain ft",
                        row.map(|r| text(r, "elevation_gain_ft"))
                            .unwrap_or_default(),
                    ),
                ];
            }
            EditKind::Gear(id) => {
                if let Some(r) = d.gear.iter().find(|r| text(r, "sync_id") == *id) {
                    values.push(("Quantity", text(r, "quantity")));
                }
            }
            EditKind::Meal(id) => {
                if let Some(e) = d
                    .food
                    .days
                    .iter()
                    .flat_map(|d| &d.entries)
                    .find(|e| e.id == *id)
                {
                    values.push((
                        "Quantity",
                        if e.meal_id.is_some() {
                            e.servings.to_string()
                        } else {
                            e.weight_g.to_string()
                        },
                    ));
                }
            }
            EditKind::New => return,
        }
        self.trips.editor = Some(TripEditor {
            kind,
            fields: values
                .into_iter()
                .map(|(l, v)| (l, input(v, false, w, cx)))
                .collect(),
            text: multiline.map(|s| textarea_input(s, w, cx)),
            loadout: None,
        });
        cx.notify();
    }
    fn save_trip_editor(&mut self, cx: &mut Context<Self>) {
        let Some(e) = &self.trips.editor else { return };
        let kind = e.kind.clone();
        let values: Vec<_> = e.fields.iter().map(|(_, s)| value(s, cx)).collect();
        let multi = e
            .text
            .as_ref()
            .map(|s| s.read(cx).value().to_string())
            .unwrap_or_default();
        let trip = self.trips.selected.clone().unwrap_or_default();
        let parse = |s: &str| -> Result<Option<f64>, String> {
            if s.trim().is_empty() {
                return Ok(None);
            }
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0. && *n <= i32::MAX as f64)
                .map(Some)
                .ok_or_else(|| "Enter a number.".into())
        };
        let result = (|| -> Result<Command, String> {
            Ok(match kind.clone() {
                EditKind::New => {
                    if values[0].trim().is_empty() {
                        return Err("Enter a trip name.".into());
                    }
                    Command::CreateTrip {
                        name: values[0].clone(),
                        trail: None,
                        loadout: e.loadout.clone(),
                    }
                }
                EditKind::Details => {
                    let mut patch = Row::new();
                    for (i, key) in [
                        "name",
                        "start_date",
                        "nights",
                        "party_size",
                        "region",
                        "trailhead",
                        "distance_mi",
                        "elevation_gain_ft",
                        "food_g_per_day",
                        "water_g_per_day",
                        "fuel_g_per_day",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let s = values[i].trim();
                        let v = if s.is_empty() {
                            Value::Null
                        } else if [
                            "nights",
                            "party_size",
                            "elevation_gain_ft",
                            "food_g_per_day",
                            "water_g_per_day",
                            "fuel_g_per_day",
                        ]
                        .contains(key)
                        {
                            let n = parse(s)?.ok_or("Enter a number.")?;
                            if n.fract() != 0. {
                                return Err("Enter a whole number.".into());
                            }
                            Value::Integer(n as i64)
                        } else {
                            if *key == "distance_mi" {
                                parse(s)?;
                            }
                            Value::Text(s.into())
                        };
                        patch.insert((*key).into(), v);
                    }
                    if values[0].trim().is_empty() {
                        return Err("Enter a trip name.".into());
                    }
                    if !values[1].is_empty()
                        && strider_core::format::fmt_date_range(Some(&values[1]), Some(0)).is_none()
                    {
                        return Err("Enter a date as YYYY-MM-DD.".into());
                    }
                    Command::UpdateTrip { trip, patch }
                }
                EditKind::Note(ref key) => Command::UpdateTrip {
                    trip,
                    patch: Row::from([(key.clone(), Value::Text(multi))]),
                },
                EditKind::Packing => Command::SaveTripPackingList {
                    trip,
                    packing_list: multi,
                },
                EditKind::Day(day) => Command::UpsertTripDay {
                    trip,
                    day,
                    distance_mi: Some(parse(&values[0])?.map(|n| n.to_string())),
                    elevation_gain_ft: Some(parse(&values[1])?.map(|n| n as i32)),
                },
                EditKind::Gear(id) => Command::SetTripGearQty {
                    trip,
                    id,
                    qty: parse(&values[0])?.ok_or("Enter a quantity.")?,
                },
                EditKind::Meal(id) => Command::SetTripMealQty {
                    trip,
                    id,
                    qty: parse(&values[0])?.ok_or("Enter a quantity.")?,
                },
            })
        })();
        match result {
            Ok(command) => {
                let before: HashSet<_> =
                    self.all_trips().iter().map(|t| t.sync_id.clone()).collect();
                self.command(command, cx);
                if self.error.is_none() {
                    self.trips.editor = None;
                    if matches!(kind, EditKind::New)
                        && let Some(id) = self
                            .all_trips()
                            .iter()
                            .find(|t| !before.contains(&t.sync_id))
                            .map(|t| t.sync_id.clone())
                    {
                        self.open_trip(id, cx);
                    }
                }
            }
            Err(e) => {
                self.error = Some(e);
                cx.notify();
            }
        }
    }
    pub(super) fn render_trip_editor(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let e = self.trips.editor.as_ref().expect("editor");
        let mut body = div()
            .id("trip-editor-fields")
            .max_h(px(480.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3();
        for (label, s) in &e.fields {
            body = body.child(strider_ui::field(
                *label,
                strider_ui::text_input(s, *label, t),
                t,
            ));
        }
        if let Some(s) = &e.text {
            body = body.child(strider_ui::textarea(s, "Notes", t));
        }
        if matches!(e.kind, EditKind::New) {
            let mut options = vec![(String::new(), "No loadout".into())];
            options.extend(
                self.loadouts
                    .loadouts
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone())),
            );
            body = body.child(self.choice(
                "new-trip-loadout",
                "Loadout",
                options,
                cx,
                |this, v, _, cx| {
                    if let Some(e) = &mut this.trips.editor {
                        e.loadout = (!v.is_empty()).then_some(v);
                    }
                    cx.notify();
                },
            ));
        }
        body = body
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_color(t.signal).child(e))
            })
            .child(
                strider_ui::button("save-trip", "Save", ButtonVariant::Primary, t)
                    .on_click(cx.listener(|this, _, _, cx| this.save_trip_editor(cx))),
            );
        self.modal(
            if matches!(e.kind, EditKind::New) {
                "New trip"
            } else {
                "Edit trip"
            },
            body.into_any_element(),
            cx,
        )
    }
    pub(in crate::app) fn all_trips(&self) -> Vec<TripSummary> {
        self.trips
            .list
            .in_progress
            .iter()
            .chain(&self.trips.list.upcoming)
            .chain(&self.trips.list.past)
            .cloned()
            .collect()
    }
    pub(super) fn trip_header_actions(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        if let Some(d) = &self.trips.detail {
            return div()
                .flex()
                .gap_2()
                .child(
                    strider_ui::button("all-trips", "All trips", ButtonVariant::HeaderOutline, t)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.trips.selected = None;
                            this.trips.detail = None;
                            cx.notify();
                        })),
                )
                .when(d.access != "view", |v| {
                    v.child(
                        strider_ui::button("trip-details", "Edit trip", ButtonVariant::Header, t)
                            .debug_selector(|| "trip-details".to_owned())
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.edit_trip(EditKind::Details, w, cx)
                            })),
                    )
                })
                .when(d.access == "owner", |v| {
                    v.child(
                        strider_ui::button("share-trip", "Share", ButtonVariant::Header, t)
                            .debug_selector(|| "share-trip".to_owned())
                            .on_click(cx.listener(|this, _, _, cx| this.open_share(cx))),
                    )
                })
                .into_any_element();
        }
        div()
            .flex()
            .gap_2()
            .child(
                strider_ui::button(
                    "import-trip",
                    "Import trip",
                    ButtonVariant::HeaderOutline,
                    t,
                )
                .on_click(cx.listener(|this, _, _, cx| this.pick_trip_file(cx))),
            )
            .child(
                strider_ui::button("new-trip", "New trip", ButtonVariant::Header, t)
                    .on_click(cx.listener(|this, _, w, cx| this.new_trip_dialog(w, cx))),
            )
            .into_any_element()
    }
}
#[cfg(test)]
mod tests;
