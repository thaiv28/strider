//! "Print Trip Sheet…": build the PDF from the open Trip and open it in the default viewer.
use super::*;
use std::path::Path;
use strider_core::{fixed, fmt_date_range, gpx::gpx_filename, units::Grams};
use strider_print::{
    DayRow, FoodDay, GearLine, GearSection, Marker, Palette, PermitDocument, TripSheet,
};
use strider_store::TripDetail;

fn channels(c: Hsla) -> [u8; 3] {
    let c: Rgba = c.into();
    [c.r, c.g, c.b].map(|v| (v * 255.).round().clamp(0., 255.) as u8)
}
/// Paper is white, so the sheet uses the light Topo tokens whatever the app shows.
pub(crate) fn palette() -> Palette {
    let t = Theme::light();
    Palette {
        ink: channels(t.ink),
        muted: channels(t.muted),
        line: channels(t.line),
        accent: channels(t.accent),
        route: channels(t.route),
        map: channels(t.map),
    }
}
/// `<gpx_filename stem>.pdf`, so the sheet sits beside the exported route.
pub(crate) fn pdf_filename(trip_name: &str) -> String {
    let gpx = gpx_filename(trip_name);
    format!("{}.pdf", gpx.strip_suffix(".gpx").unwrap_or(&gpx))
}
fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}
const NONE: &str = "\u{2014}";
fn miles(v: Option<f64>) -> String {
    v.map_or_else(|| NONE.into(), |v| format!("{} mi", fixed(v, 1)))
}
fn feet(v: Option<i64>) -> String {
    v.map_or_else(|| NONE.into(), |v| format!("{} ft", thousands(v)))
}
fn coordinates(lat: Option<f64>, lon: Option<f64>) -> String {
    match (lat, lon) {
        (Some(lat), Some(lon)) => format!("{}, {}", fixed(lat, 4), fixed(lon, 4)),
        _ => NONE.into(),
    }
}
fn weight(g: i64) -> String {
    fmt_weight(Some(Grams(g as i32)))
}
fn capitalized(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
/// The sheet's content, in the web print page's order, then Gear, Food and Campsites.
pub(crate) fn build_sheet(d: &TripDetail, palette: Palette) -> TripSheet {
    let f = &d.fields;
    let nights: Option<i32> = trips::text(f, "nights").parse().ok();
    let start = trips::text(f, "start_date");
    let dates = fmt_date_range((!start.is_empty()).then_some(start.as_str()), nights)
        .unwrap_or_else(|| "date TBD".into());
    let when = match nights {
        Some(n) => format!("{dates} \u{b7} {n} night{}", if n == 1 { "" } else { "s" }),
        None => dates,
    };
    let place = [d.region.clone(), d.area_type.clone()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    let trailhead = trips::text(f, "trailhead");
    let summary = format!(
        "{} \u{b7} {} gain{}",
        miles(d.route.distance_mi),
        feet(d.route.elevation_ft),
        if trailhead.is_empty() {
            String::new()
        } else {
            format!(" \u{b7} from {trailhead}")
        }
    );
    let camp_of = |day: usize| {
        d.segments
            .iter()
            .find(|s| s.day == day)
            .map_or_else(|| NONE.into(), |s| coordinates(s.lat, s.lon))
    };
    let days: Vec<DayRow> = if d.days.is_empty() {
        d.segments
            .iter()
            .map(|s| DayRow {
                day: format!("Day {}", s.day),
                distance: miles(Some(s.distance_mi)),
                gain: feet(Some(s.elevation_gain_ft)),
                camp: camp_of(s.day),
            })
            .collect()
    } else {
        d.days
            .iter()
            .map(|r| {
                let n = trips::num(r, "day_number") as usize;
                DayRow {
                    day: format!("Day {n}"),
                    distance: miles(trips::text(r, "distance_mi").parse().ok()),
                    gain: feet(trips::text(r, "elevation_gain_ft").parse().ok()),
                    camp: camp_of(n),
                }
            })
            .collect()
    };
    let permit_notes = trips::text(f, "permit_notes");
    let gear = d
        .groups
        .iter()
        .map(|(category, rows)| GearSection {
            category: category.clone(),
            lines: rows
                .iter()
                .map(|r| GearLine {
                    name: trips::text(r, "snapshot_name"),
                    quantity: format!("\u{d7}{}", trips::text(r, "quantity")),
                    weight: weight(trips::num(r, "snapshot_weight_g") as i64),
                    class: trips::text(r, "weight_class"),
                })
                .collect(),
        })
        .collect();
    let w = &d.weights;
    let mut gear_totals = vec![
        ("Base".to_owned(), weight(w.base)),
        ("Worn".to_owned(), weight(w.worn)),
        ("Food".to_owned(), weight(w.food)),
        ("Water".to_owned(), weight(w.water)),
        ("Fuel".to_owned(), weight(w.fuel)),
    ];
    if w.consumable > 0 {
        gear_totals.push(("Consumables".to_owned(), weight(w.consumable)));
    }
    gear_totals.push(("Pack".to_owned(), weight(w.pack)));
    gear_totals.push(("Skin-out".to_owned(), weight(w.skin_out)));
    let mut food: Vec<FoodDay> = d
        .food
        .days
        .iter()
        .filter(|day| !day.entries.is_empty())
        .map(|day| FoodDay {
            heading: format!(
                "Day {} \u{b7} {} / {} kcal",
                day.day_number, day.planned_kcal, day.target_kcal
            ),
            lines: day
                .entries
                .iter()
                .map(|e| {
                    let kind = e
                        .meal_type
                        .as_deref()
                        .filter(|k| !k.is_empty())
                        .map(|k| format!("{}: ", capitalized(k)))
                        .unwrap_or_default();
                    format!(
                        "{kind}{} \u{b7} {} kcal \u{b7} {}",
                        e.name,
                        e.kcal,
                        weight(e.weight_g)
                    )
                })
                .collect(),
        })
        .collect();
    if !food.is_empty() {
        food.push(FoodDay {
            heading: format!(
                "Trip food \u{b7} {} kcal \u{b7} {}",
                d.food.total_kcal,
                weight(d.food.total_g)
            ),
            lines: vec![],
        });
    }
    let sites: Vec<_> = trips::camps(d).into_iter().map(|(_, c)| c).collect();
    let campsites = d
        .campsites
        .iter()
        .zip(&sites)
        .map(|(r, c)| {
            let elevation = c
                .ele_ft
                .map(|e| format!(" \u{b7} {} ft", thousands(e as i64)));
            format!(
                "Night {} \u{b7} {} \u{b7} {}{}",
                c.night,
                miles(Some(trips::num(r, "distance_mi"))),
                coordinates(c.lat, c.lon),
                elevation.unwrap_or_default()
            )
        })
        .collect();
    TripSheet {
        name: trips::text(f, "name"),
        when,
        place: (!place.is_empty()).then_some(place),
        summary,
        days,
        totals: DayRow {
            day: "Total".into(),
            distance: miles(d.route.distance_mi),
            gain: feet(d.route.elevation_ft),
            camp: String::new(),
        },
        permit_notes: (!permit_notes.trim().is_empty()).then_some(permit_notes),
        permit_document: d.permit.as_ref().map(|p| PermitDocument {
            filename: p.filename.clone(),
            kind: if p.mime_type.starts_with("image/") {
                "image".into()
            } else {
                "PDF".into()
            },
        }),
        gear_totals,
        gear,
        food,
        campsites,
        track: d.points.iter().map(|p| [p.lat, p.lon]).collect(),
        markers: sites
            .iter()
            .filter_map(|c| {
                Some(Marker {
                    night: u32::try_from(c.night).ok()?,
                    lat: c.lat?,
                    lon: c.lon?,
                })
            })
            .collect(),
        palette,
    }
}
/// Open a file in its default app. Skipped under test, which must not launch anything.
fn open_file(path: &Path) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
impl AppView {
    /// Import GPX needs an open Trip the user may edit.
    pub(crate) fn can_import_gpx(&self) -> bool {
        !self.overlay_open()
            && self
                .trips
                .detail
                .as_ref()
                .is_some_and(|d| d.access != "view")
    }
    /// Export GPX needs an open Trip that has a route.
    pub(crate) fn can_export_gpx(&self) -> bool {
        !self.overlay_open()
            && self
                .trips
                .detail
                .as_ref()
                .is_some_and(|d| !d.points.is_empty())
    }
    /// Whether the open Trip can be printed.
    pub(crate) fn can_print(&self) -> bool {
        self.trips.detail.is_some()
    }
    pub(crate) fn print_trip_sheet(&mut self, cx: &mut Context<Self>) {
        let Some(d) = &self.trips.detail else { return };
        let sheet = build_sheet(d, palette());
        let filename = pdf_filename(&sheet.name);
        let tiles =
            Some(self.offline.backend.tiles_root.clone()).filter(|p| !p.as_os_str().is_empty());
        let receiver = cx.prompt_for_new_path(self.store.borrow().data_dir(), Some(&filename));
        cx.spawn(async move |this, cx| {
            let failure = match receiver.await {
                Ok(Ok(Some(path))) => {
                    // Decoding tiles and drawing the map is the slow part: keep it off the UI thread.
                    let task = cx.background_executor().spawn(async move {
                        let bytes = strider_print::render_pdf(&sheet, tiles.as_deref())
                            .map_err(|e| e.to_string())?;
                        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                        open_file(&path)
                    });
                    task.await.err()
                }
                Ok(Ok(None)) => None,
                _ => Some("Could not open save panel.".into()),
            };
            if let Some(message) = failure {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(message);
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
