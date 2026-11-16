//! Offline support: background jobs, cached service reads, and the offline map download.
//!
//! Every service read the UI renders comes from the device cache. A background refresh may fill
//! the cache from the network, after which the cache is read again, so the UI never waits on a
//! request and behaves the same with or without a connection.
use super::*;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};
use strider_map::{
    HttpFetcher, TileFetcher,
    region::{self, Outcome, Progress, Region},
};
use strider_services::{
    Clock, HttpClient, HttpResponse, ServiceError, SystemClock,
    overpass::{self, CampPoi},
    share::{LinkAction, LinkKind, ShareLink},
};
use strider_store::MapRegionRecord;

pub(super) mod format;
pub(super) mod map;

/// A finished piece of background work, delivered to the render thread by the poll loop.
pub(crate) enum Job {
    Region {
        region: Region,
        outcome: Result<Outcome, String>,
    },
    /// A cache refresh ended (successfully or not); read the cache again.
    Refreshed(String),
    Share {
        kind: LinkKind,
        action: LinkAction,
        result: Result<Option<ShareLink>, String>,
    },
}
/// Where the app reaches the outside world. Tests replace these with fakes.
pub(crate) struct Backend {
    pub http: Arc<dyn HttpClient>,
    pub fetcher: Arc<dyn TileFetcher>,
    /// `<cache>/tiles`, shared with the map's tile cache.
    pub tiles_root: PathBuf,
    /// The workspace database; background refreshes open their own connection to it. Empty (as in
    /// tests with an in-memory store) disables background refreshes.
    pub db_path: PathBuf,
}
/// Answers every request as if the device were offline, so a service call reads only its cache.
pub(crate) struct CacheOnly;
impl HttpClient for CacheOnly {
    fn get(&self, _: &str) -> strider_services::Result<HttpResponse> {
        Err(ServiceError::Offline)
    }
    fn post_form(&self, _: &str, _: &str) -> strider_services::Result<HttpResponse> {
        Err(ServiceError::Offline)
    }
}
pub(crate) struct Download {
    pub trip: String,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<Mutex<Progress>>,
}
pub(crate) struct OfflineState {
    pub jobs_tx: Sender<Job>,
    pub jobs_rx: Receiver<Job>,
    pub backend: Backend,
    /// The map download sheet is open.
    pub sheet: bool,
    /// The selected Trip's stored region.
    pub region: Option<MapRegionRecord>,
    /// Every stored region, for Settings.
    pub regions: Vec<MapRegionRecord>,
    pub download: Option<Download>,
    /// Campsite points of interest from the Overpass cache for the selected Trip's route.
    pub pois: Vec<CampPoi>,
    pub message: Option<String>,
    in_flight: HashSet<String>,
    last_try: HashMap<String, Instant>,
}
impl OfflineState {
    pub fn new() -> Self {
        let (jobs_tx, jobs_rx) = mpsc::channel();
        Self {
            jobs_tx,
            jobs_rx,
            backend: Backend {
                http: Arc::new(strider_services::UreqClient::new()),
                fetcher: Arc::new(HttpFetcher::default()),
                tiles_root: PathBuf::new(),
                db_path: PathBuf::new(),
            },
            sheet: false,
            region: None,
            regions: vec![],
            download: None,
            pois: vec![],
            message: None,
            in_flight: HashSet::new(),
            last_try: HashMap::new(),
        }
    }
}
const RETRY_AFTER: Duration = Duration::from_secs(300);
impl AppView {
    /// Whether the device last failed to reach the server.
    pub(super) fn offline(&self) -> bool {
        self.account.offline
    }
    pub(super) fn now(&self) -> i64 {
        SystemClock.now()
    }
    /// Fill a cache from the network on a worker thread, at most once every five minutes per key.
    /// `work` receives a connection of its own to the workspace database.
    pub(super) fn refresh(
        &mut self,
        key: String,
        work: impl FnOnce(&Store, &dyn HttpClient, &dyn Clock) + Send + 'static,
    ) {
        let o = &mut self.offline;
        if o.backend.db_path.as_os_str().is_empty()
            || o.in_flight.contains(&key)
            || o.last_try
                .get(&key)
                .is_some_and(|t| t.elapsed() < RETRY_AFTER)
        {
            return;
        }
        o.in_flight.insert(key.clone());
        o.last_try.insert(key.clone(), Instant::now());
        let (path, http, tx) = (
            o.backend.db_path.clone(),
            o.backend.http.clone(),
            o.jobs_tx.clone(),
        );
        std::thread::spawn(move || {
            if let Ok(store) = Store::open_secondary(&path) {
                work(&store, &*http, &SystemClock);
            }
            let _ = tx.send(Job::Refreshed(key));
        });
    }
    pub(super) fn refreshing(&self, key: &str) -> bool {
        self.offline.in_flight.contains(key)
    }
    /// Apply finished background work. Called from the 200 ms poll loop.
    pub(super) fn poll_jobs(&mut self, cx: &mut Context<Self>) {
        let jobs: Vec<Job> = self.offline.jobs_rx.try_iter().collect();
        let mut notify = self.offline.download.is_some() || !jobs.is_empty();
        for job in jobs {
            match job {
                Job::Region { region, outcome } => self.finish_download(region, outcome, cx),
                Job::Refreshed(key) => {
                    self.offline.in_flight.remove(&key);
                    self.reload_cached(cx);
                }
                Job::Share {
                    kind,
                    action,
                    result,
                } => self.finish_share(kind, action, result, cx),
            }
        }
        notify |= self.poll_devices();
        if let Some((_, at)) = &self.toast
            && at.elapsed() > Duration::from_secs(6)
        {
            self.toast = None;
            notify = true;
        }
        if notify {
            cx.notify();
        }
    }
    /// Re-read every cache-backed view after a refresh.
    fn reload_cached(&mut self, cx: &mut Context<Self>) {
        self.load_pois();
        self.load_panels();
        self.load_calendar(cx);
    }
    pub(super) fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }
}
#[cfg(test)]
mod tests;
