//! Offline map download for a Trip's route, Overpass overlays, and the map cache controls.
use super::*;
use format::{age, megabytes};
use strider_map::region::{Estimate, from_record};
use strider_store::TripDetail;
use trips::text;

fn record(region: &Region, p: Progress, state: &str, now: i64) -> MapRegionRecord {
    MapRegionRecord {
        trip_sync_id: region.trip_sync_id.clone(),
        south: region.bbox.south,
        west: region.bbox.west,
        north: region.bbox.north,
        east: region.bbox.east,
        min_zoom: region.min_zoom,
        max_zoom: region.max_zoom,
        tiles_total: p.total,
        tiles_done: p.done,
        bytes: p.bytes,
        state: state.into(),
        updated_at: now,
    }
}
#[cfg(test)]
pub(super) fn record_for_tests(region: &Region, state: &str) -> MapRegionRecord {
    record(region, Progress::default(), state, 0)
}
impl AppView {
    /// The selected Trip's region, when its tiles are fully downloaded.
    pub(in crate::app) fn has_offline_map(&self) -> bool {
        self.offline
            .region
            .as_ref()
            .is_some_and(|r| r.state == "complete")
    }
    /// The Trip has a route but no map to show without a connection.
    pub(in crate::app) fn map_message(&self, d: &TripDetail) -> Option<&'static str> {
        (self.offline() && !d.points.is_empty() && !self.has_offline_map())
            .then_some("Map not downloaded. Connect, or download this map before you go.")
    }
    pub(in crate::app) fn selected_region(&self) -> Option<Region> {
        let d = self.trips.detail.as_ref()?;
        let track: Vec<[f64; 2]> = d.points.iter().map(|p| [p.lat, p.lon]).collect();
        Region::for_route(&text(&d.fields, "sync_id"), &track)
    }
    pub(in crate::app) fn load_regions(&mut self) {
        self.offline.regions = self.store.map_regions().unwrap_or_default();
        self.offline.region = self
            .trips
            .selected
            .as_ref()
            .and_then(|id| self.offline.regions.iter().find(|r| &r.trip_sync_id == id))
            .cloned();
    }
    /// Campsite POIs from the Overpass cache, refreshed in the background when stale.
    pub(in crate::app) fn load_pois(&mut self) {
        self.offline.pois.clear();
        let Some(d) = &self.trips.detail else { return };
        let track: Vec<[f64; 2]> = d.points.iter().map(|p| [p.lat, p.lon]).collect();
        let Some(bbox) = overpass::track_bbox(&track, 0.01) else {
            return;
        };
        let read = overpass::fetch_campsite_pois(&self.store, &CacheOnly, &SystemClock, &bbox);
        let stale = match read {
            Ok(c) => {
                self.offline.pois = c.value;
                c.stale
            }
            Err(_) => true,
        };
        if stale {
            self.refresh(
                format!("pois|{}", overpass::cache_key(&bbox)),
                move |s, h, c| {
                    let _ = overpass::fetch_campsite_pois(s, h, c, &bbox);
                },
            );
        }
    }
    pub(in crate::app) fn push_pois(&mut self, cx: &mut Context<Self>) {
        let pois = self
            .offline
            .pois
            .iter()
            .map(|p| (p.lat, p.lon, p.name.clone()))
            .collect();
        self.trips.map.update(cx, |m, cx| {
            m.pois = pois;
            cx.notify();
        });
    }
    pub(in crate::app) fn open_map_sheet(&mut self, cx: &mut Context<Self>) {
        self.offline.message = None;
        self.offline.sheet = true;
        cx.notify();
    }
    pub(in crate::app) fn start_download(&mut self, cx: &mut Context<Self>) {
        if self.offline.download.is_some() {
            return;
        }
        let Some(region) = self.selected_region() else {
            return;
        };
        let estimate = match region.check() {
            Ok(e) => e,
            Err(e) => {
                self.offline.message = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let now = self.now();
        let begin = Progress {
            total: estimate.tiles,
            ..Progress::default()
        };
        if let Err(e) = self
            .store
            .save_map_region(&record(&region, begin, "downloading", now))
        {
            self.offline.message = Some(e.to_string());
            cx.notify();
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(begin));
        self.offline.download = Some(Download {
            trip: region.trip_sync_id.clone(),
            cancel: cancel.clone(),
            progress: progress.clone(),
        });
        self.offline.message = None;
        let (root, fetcher, tx) = (
            self.offline.backend.tiles_root.clone(),
            self.offline.backend.fetcher.clone(),
            self.offline.jobs_tx.clone(),
        );
        std::thread::spawn(move || {
            let outcome = region::download(&root, &region, &*fetcher, &cancel, &|p| {
                if let Ok(mut last) = progress.lock() {
                    *last = p;
                }
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(Job::Region { region, outcome });
        });
        self.load_regions();
        cx.notify();
    }
    pub(in crate::app) fn cancel_download(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = &self.offline.download {
            d.cancel.store(true, Ordering::Relaxed);
            cx.notify();
        }
    }
    pub(super) fn finish_download(
        &mut self,
        region: Region,
        outcome: Result<Outcome, String>,
        cx: &mut Context<Self>,
    ) {
        let last = self
            .offline
            .download
            .take()
            .and_then(|d| d.progress.lock().ok().map(|p| *p))
            .unwrap_or_default();
        let now = self.now();
        let state = match &outcome {
            Ok(Outcome::Complete) => "complete",
            Ok(Outcome::Cancelled) => "cancelled",
            _ => "failed",
        };
        if let Err(e) = self
            .store
            .save_map_region(&record(&region, last, state, now))
        {
            self.offline.message = Some(e.to_string());
        }
        self.offline.message = match outcome {
            Ok(Outcome::Complete | Outcome::Cancelled) => self.offline.message.take(),
            Ok(Outcome::Incomplete { failed }) => Some(format!(
                "{failed} tiles could not be downloaded. Download again to retry."
            )),
            Err(e) => Some(e),
        };
        self.load_regions();
        cx.notify();
    }
    pub(in crate::app) fn delete_region(&mut self, trip: &str, cx: &mut Context<Self>) {
        let Some(rec) = self
            .offline
            .regions
            .iter()
            .find(|r| r.trip_sync_id == trip)
            .cloned()
        else {
            return;
        };
        if self
            .offline
            .download
            .as_ref()
            .is_some_and(|d| d.trip == trip)
        {
            return;
        }
        if let Err(e) = region::delete_region(
            &self.store,
            &self.offline.backend.tiles_root,
            &from_record(&rec),
        ) {
            self.offline.message = Some(e.to_string());
        }
        self.load_regions();
        self.settings_cache_size();
        cx.notify();
    }
    pub(in crate::app) fn clear_map_cache(&mut self, cx: &mut Context<Self>) {
        match strider_map::clear_unpinned(&self.offline.backend.tiles_root) {
            Ok(_) => self.offline.message = None,
            Err(e) => self.offline.message = Some(e.to_string()),
        }
        self.settings_cache_size();
        cx.notify();
    }
    pub(in crate::app) fn render_map_sheet(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let mut body = div().flex().flex_col().gap_3();
        let region = self.selected_region();
        let trip = self.trips.selected.clone().unwrap_or_default();
        if let Some(dl) = &self.offline.download
            && dl.trip == trip
        {
            let p = dl.progress.lock().map(|p| *p).unwrap_or_default();
            body = body
                .child(format!("Downloading · {} of {} tiles", p.done, p.total))
                .child(div().meta(t).child(megabytes(p.bytes)))
                .child(
                    strider_ui::button("cancel-download", "Cancel", ButtonVariant::Outline, t)
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_download(cx))),
                );
        } else if self.has_offline_map() {
            let bytes = self.offline.region.as_ref().map_or(0, |r| r.bytes);
            body = body
                .child(format!("Downloaded · {}", megabytes(bytes)))
                .child(
                    strider_ui::button("delete-region", "Delete", ButtonVariant::Danger, t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let trip = this.trips.selected.clone().unwrap_or_default();
                            this.delete_region(&trip, cx)
                        })),
                );
        } else if let Some(region) = region {
            let Estimate { tiles, bytes } = region.estimate();
            body = body
                .child(format!("About {tiles} map tiles, {}.", megabytes(bytes)))
                .child(div().meta(t).child(
                    "Topo tiles for the route and its surroundings, kept until you delete them.",
                ))
                .child(
                    strider_ui::button("start-download", "Download", ButtonVariant::Primary, t)
                        .on_click(cx.listener(|this, _, _, cx| this.start_download(cx))),
                );
        } else {
            body = body.child(div().meta(t).child("Attach a route to download its map."));
        }
        if let Some(m) = &self.offline.message {
            body = body.child(div().text_color(t.signal).child(m.clone()));
        }
        self.modal("Download map for offline", body.into_any_element(), cx)
    }
    /// One line per stored region for Settings: name, size, state.
    pub(in crate::app) fn region_summary(&self, r: &MapRegionRecord) -> String {
        let name = self
            .trips
            .list
            .in_progress
            .iter()
            .chain(&self.trips.list.upcoming)
            .chain(&self.trips.list.past)
            .find(|t| t.sync_id == r.trip_sync_id)
            .map_or_else(|| "Trip".to_owned(), |t| t.name.clone());
        let status = match r.state.as_str() {
            "complete" => megabytes(r.bytes),
            "downloading" => format!("downloading, {} of {} tiles", r.tiles_done, r.tiles_total),
            s => format!("{s}, {} of {} tiles", r.tiles_done, r.tiles_total),
        };
        format!("{name} · {status} · {}", age(self.now(), r.updated_at))
    }
}
