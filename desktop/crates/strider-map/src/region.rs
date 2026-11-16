//! Offline map regions: a Trip's padded route bounding box, pre-downloaded USGS Topo tiles pinned
//! against LRU eviction. OpenStreetMap tiles are never bulk downloaded.
use crate::{Fetch, MAX_LAT, Source, TILE, Tile, TileFetcher, pin_path, project, tile_path};
use std::{
    fmt, fs, io,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use strider_store::{MapRegionRecord, Store};

pub const MIN_ZOOM: u8 = 8;
pub const MAX_ZOOM: u8 = 15;
/// Refuse a download above this many tiles.
pub const MAX_TILES: u64 = 20_000;
/// Average USGS Topo tile size used for the pre-download estimate.
pub const ESTIMATED_TILE_BYTES: u64 = 24 * 1024;
pub const CONCURRENCY: usize = 4;
const PAD: f64 = 0.10;
const MIN_PAD_DEG: f64 = 0.005;

static PIN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bbox {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub trip_sync_id: String,
    pub bbox: Bbox,
    pub min_zoom: u8,
    pub max_zoom: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Estimate {
    pub tiles: u64,
    pub bytes: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    pub failed: u64,
    pub bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Cancelled,
    /// Some tiles could not be fetched; running the download again resumes.
    Incomplete {
        failed: u64,
    },
}
#[derive(Debug)]
pub enum RegionError {
    TooLarge { tiles: u64 },
    Io(io::Error),
    Store(strider_store::Error),
}
impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { tiles } => write!(
                f,
                "This area needs about {tiles} map tiles, above the {MAX_TILES} tile limit. Choose a shorter route."
            ),
            Self::Io(e) => e.fmt(f),
            Self::Store(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for RegionError {}
impl From<io::Error> for RegionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<strider_store::Error> for RegionError {
    fn from(e: strider_store::Error) -> Self {
        Self::Store(e)
    }
}

impl Region {
    /// The route's bounding box padded 10% on each side, at the default zooms.
    /// `track` is `[lat, lon]` pairs; returns `None` without a finite point.
    pub fn for_route(trip_sync_id: &str, track: &[[f64; 2]]) -> Option<Self> {
        let mut pts = track
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite());
        let first = pts.next()?;
        let (mut s, mut w, mut n, mut e) = (first[0], first[1], first[0], first[1]);
        for p in pts {
            s = s.min(p[0]);
            n = n.max(p[0]);
            w = w.min(p[1]);
            e = e.max(p[1]);
        }
        let dlat = ((n - s) * PAD).max(MIN_PAD_DEG);
        let dlon = ((e - w) * PAD).max(MIN_PAD_DEG);
        Some(Self {
            trip_sync_id: trip_sync_id.into(),
            bbox: Bbox {
                south: (s - dlat).max(-MAX_LAT),
                west: (w - dlon).max(-180.),
                north: (n + dlat).min(MAX_LAT),
                east: (e + dlon).min(180.),
            },
            min_zoom: MIN_ZOOM,
            max_zoom: MAX_ZOOM,
        })
    }
    /// Inclusive tile ranges `(x, y)` covering the bounding box at zoom `z`.
    pub fn tile_range(&self, z: u8) -> ((u32, u32), (u32, u32)) {
        let edge = (1_u64 << z) - 1;
        let index = |v: f64| ((v / TILE).floor().max(0.) as u64).min(edge) as u32;
        let nw = project(self.bbox.north, self.bbox.west, f64::from(z));
        let se = project(self.bbox.south, self.bbox.east, f64::from(z));
        ((index(nw.x), index(se.x)), (index(nw.y), index(se.y)))
    }
    fn zooms(&self) -> impl Iterator<Item = u8> {
        self.min_zoom.min(self.max_zoom)..=self.max_zoom.min(Source::Topo.max_zoom())
    }
    pub fn tile_count(&self) -> u64 {
        self.zooms()
            .map(|z| {
                let ((x0, x1), (y0, y1)) = self.tile_range(z);
                u64::from(x1 - x0 + 1) * u64::from(y1 - y0 + 1)
            })
            .sum()
    }
    pub fn tiles(&self) -> Vec<Tile> {
        let mut out = Vec::with_capacity(self.tile_count().min(MAX_TILES * 2) as usize);
        for z in self.zooms() {
            let ((x0, x1), (y0, y1)) = self.tile_range(z);
            for x in x0..=x1 {
                for y in y0..=y1 {
                    out.push(Tile { z, x, y });
                }
            }
        }
        out
    }
    pub fn estimate(&self) -> Estimate {
        let tiles = self.tile_count();
        Estimate {
            tiles,
            bytes: tiles * ESTIMATED_TILE_BYTES,
        }
    }
    /// The estimate, or a refusal above [`MAX_TILES`].
    pub fn check(&self) -> Result<Estimate, RegionError> {
        let e = self.estimate();
        if e.tiles > MAX_TILES {
            return Err(RegionError::TooLarge { tiles: e.tiles });
        }
        Ok(e)
    }
    fn record(&self, p: Progress, state: &str) -> MapRegionRecord {
        MapRegionRecord {
            trip_sync_id: self.trip_sync_id.clone(),
            south: self.bbox.south,
            west: self.bbox.west,
            north: self.bbox.north,
            east: self.bbox.east,
            min_zoom: self.min_zoom,
            max_zoom: self.max_zoom,
            tiles_total: p.total,
            tiles_done: p.done,
            bytes: p.bytes,
            state: state.into(),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        }
    }
}

fn add_pin(path: &Path, id: &str) -> io::Result<()> {
    let _guard = PIN_LOCK.lock().map_err(|_| io::Error::other("Pin lock"))?;
    let pin = pin_path(path);
    let mut ids = fs::read_to_string(&pin).unwrap_or_default();
    if ids.lines().any(|l| l == id) {
        return Ok(());
    }
    ids.push_str(id);
    ids.push('\n');
    fs::write(pin, ids)
}
fn remove_pin(path: &Path, id: &str) -> io::Result<()> {
    let _guard = PIN_LOCK.lock().map_err(|_| io::Error::other("Pin lock"))?;
    let pin = pin_path(path);
    let Ok(ids) = fs::read_to_string(&pin) else {
        return Ok(());
    };
    let rest: Vec<_> = ids.lines().filter(|l| *l != id && !l.is_empty()).collect();
    if rest.is_empty() {
        fs::remove_file(pin)
    } else {
        fs::write(pin, rest.join("\n") + "\n")
    }
}
/// Whether any region pins the tile.
pub fn is_pinned(tiles_root: &Path, tile: Tile) -> bool {
    pin_path(&tile_path(tiles_root, Source::Topo, tile)).exists()
}

fn fetch_one(root: &Path, id: &str, tile: Tile, fetcher: &dyn TileFetcher) -> io::Result<u64> {
    let path = tile_path(root, Source::Topo, tile);
    if let Ok(meta) = fs::metadata(&path)
        && meta.len() > 0
    {
        // Resume: already downloaded or cached, so only pin it.
        add_pin(&path, id)?;
        return Ok(meta.len());
    }
    let Fetch { bytes, .. } = match fetcher.fetch(Source::Topo, tile) {
        Ok(f) => f,
        Err(_) => fetcher.fetch(Source::Topo, tile)?,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &bytes)?;
    fs::rename(tmp, &path)?;
    fs::write(path.with_extension("used"), [])?;
    add_pin(&path, id)?;
    Ok(bytes.len() as u64)
}

/// Download the region's Topo tiles with at most [`CONCURRENCY`] requests in flight. Tiles already
/// on disk are pinned without refetching, so an interrupted or incomplete download resumes.
/// `progress` is called after every tile, from worker threads. Setting `cancel` stops promptly;
/// tiles already fetched stay pinned.
pub fn download(
    tiles_root: &Path,
    region: &Region,
    fetcher: &dyn TileFetcher,
    cancel: &AtomicBool,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Outcome, RegionError> {
    region.check()?;
    let tiles = region.tiles();
    let total = tiles.len() as u64;
    let (next, done, failed, bytes) = (
        AtomicUsize::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    );
    thread::scope(|scope| {
        for _ in 0..CONCURRENCY {
            scope.spawn(|| {
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(tile) = tiles.get(i) else { return };
                    match fetch_one(tiles_root, &region.trip_sync_id, *tile, fetcher) {
                        Ok(n) => {
                            done.fetch_add(1, Ordering::Relaxed);
                            bytes.fetch_add(n, Ordering::Relaxed);
                        }
                        Err(_) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    progress(Progress {
                        done: done.load(Ordering::Relaxed),
                        total,
                        failed: failed.load(Ordering::Relaxed),
                        bytes: bytes.load(Ordering::Relaxed),
                    });
                }
            });
        }
    });
    let failed = failed.load(Ordering::Relaxed);
    Ok(
        if cancel.load(Ordering::Relaxed) && done.load(Ordering::Relaxed) + failed < total {
            Outcome::Cancelled
        } else if failed > 0 {
            Outcome::Incomplete { failed }
        } else {
            Outcome::Complete
        },
    )
}

/// [`download`] with the region's state persisted in the store (`map_region`): saved as
/// `downloading` first, then `complete`, `cancelled` or `failed` with final counts.
pub fn download_and_record(
    store: &Store,
    tiles_root: &Path,
    region: &Region,
    fetcher: &dyn TileFetcher,
    cancel: &AtomicBool,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<Outcome, RegionError> {
    let estimate = region.check()?;
    let last = Mutex::new(Progress {
        total: estimate.tiles,
        ..Progress::default()
    });
    store.save_map_region(&region.record(
        Progress {
            total: estimate.tiles,
            ..Progress::default()
        },
        "downloading",
    ))?;
    let outcome = download(tiles_root, region, fetcher, cancel, &|p| {
        if let Ok(mut l) = last.lock() {
            *l = p;
        }
        progress(p);
    });
    let last = last.lock().map(|l| *l).unwrap_or_default();
    let state = match &outcome {
        Ok(Outcome::Complete) => "complete",
        Ok(Outcome::Cancelled) => "cancelled",
        _ => "failed",
    };
    store.save_map_region(&region.record(last, state))?;
    outcome
}

/// Release the region's pins (tiles stay cached but become evictable) and drop its stored state.
/// Returns the number of tiles unpinned.
pub fn delete_region(
    store: &Store,
    tiles_root: &Path,
    region: &Region,
) -> Result<u64, RegionError> {
    let mut released = 0;
    for tile in region.tiles() {
        let path = tile_path(tiles_root, Source::Topo, tile);
        if pin_path(&path).exists() {
            remove_pin(&path, &region.trip_sync_id)?;
            released += 1;
        }
    }
    store.delete_map_region(&region.trip_sync_id)?;
    Ok(released)
}

/// Rebuild a region from its stored record.
pub fn from_record(r: &MapRegionRecord) -> Region {
    Region {
        trip_sync_id: r.trip_sync_id.clone(),
        bbox: Bbox {
            south: r.south,
            west: r.west,
            north: r.north,
            east: r.east,
        },
        min_zoom: r.min_zoom,
        max_zoom: r.max_zoom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evict;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Fake {
        calls: AtomicUsize,
        in_flight: AtomicUsize,
        peak: AtomicUsize,
        /// Fail tiles whose x is odd while set.
        fail_odd: AtomicBool,
        cancel_after: Option<(usize, Arc<AtomicBool>)>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                fail_odd: AtomicBool::new(false),
                cancel_after: None,
            }
        }
    }
    impl TileFetcher for Fake {
        fn fetch(&self, source: Source, t: Tile) -> io::Result<Fetch> {
            assert_eq!(source, Source::Topo, "only Topo is bulk downloaded");
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(2));
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if let Some((limit, flag)) = &self.cancel_after
                && n >= *limit
            {
                flag.store(true, Ordering::SeqCst);
            }
            if self.fail_odd.load(Ordering::SeqCst) && t.x % 2 == 1 {
                return Err(io::Error::other("offline"));
            }
            Ok(Fetch {
                bytes: vec![1; 100],
                max_age: Duration::from_secs(60),
                store: true,
            })
        }
    }
    use std::time::Duration;

    fn root(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("strider-region-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).expect("create temp dir");
        p
    }
    fn small(id: &str) -> Region {
        Region {
            trip_sync_id: id.into(),
            bbox: Bbox {
                south: 39.00,
                west: -105.05,
                north: 39.04,
                east: -105.00,
            },
            min_zoom: 12,
            max_zoom: 13,
        }
    }

    #[test]
    fn route_region_pads_ten_percent_with_default_zooms() {
        let r = Region::for_route("t", &[[40., -120.], [41., -119.], [f64::NAN, 0.]]).unwrap();
        assert!((r.bbox.south - 39.9).abs() < 1e-9 && (r.bbox.north - 41.1).abs() < 1e-9);
        assert!((r.bbox.west - -120.1).abs() < 1e-9 && (r.bbox.east - -118.9).abs() < 1e-9);
        assert_eq!((r.min_zoom, r.max_zoom), (8, 15));
        assert!(Region::for_route("t", &[]).is_none());
        let point = Region::for_route("t", &[[40., -120.]]).unwrap();
        assert!(point.bbox.north > point.bbox.south && point.tile_count() >= 8);
    }
    #[test]
    fn estimate_counts_tiles_per_zoom_and_refuses_large_areas() {
        let r = small("t");
        let ((x0, x1), (y0, y1)) = r.tile_range(12);
        assert_eq!(
            r.tiles().iter().filter(|t| t.z == 12).count() as u32,
            (x1 - x0 + 1) * (y1 - y0 + 1)
        );
        assert_eq!(r.tiles().len() as u64, r.tile_count());
        assert_eq!(r.estimate().bytes, r.tile_count() * ESTIMATED_TILE_BYTES);
        assert!(r.check().is_ok());
        let big = Region::for_route("t", &[[37., -109.], [41., -102.]]).unwrap();
        match big.check() {
            Err(RegionError::TooLarge { tiles }) => {
                assert!(tiles > MAX_TILES);
                assert!(
                    RegionError::TooLarge { tiles }
                        .to_string()
                        .contains("limit")
                );
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        let cancel = AtomicBool::new(false);
        let dir = root("large");
        let fake = Fake::new();
        assert!(matches!(
            download(&dir, &big, &fake, &cancel, &|_| {}),
            Err(RegionError::TooLarge { .. })
        ));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn download_pins_tiles_with_bounded_concurrency_and_progress() {
        let dir = root("download");
        let r = small("trip-a");
        let fake = Fake::new();
        let seen = Mutex::new(vec![]);
        let outcome = download(&dir, &r, &fake, &AtomicBool::new(false), &|p| {
            seen.lock().unwrap().push(p.done + p.failed)
        })
        .unwrap();
        assert_eq!(outcome, Outcome::Complete);
        let total = r.tile_count() as usize;
        assert_eq!(fake.calls.load(Ordering::SeqCst), total);
        assert!(fake.peak.load(Ordering::SeqCst) <= CONCURRENCY);
        assert_eq!(seen.lock().unwrap().len(), total);
        assert!(r.tiles().iter().all(|t| is_pinned(&dir, *t)));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn download_resumes_after_failures_without_refetching() {
        let dir = root("resume");
        let r = small("trip-a");
        let fake = Fake::new();
        fake.fail_odd.store(true, Ordering::SeqCst);
        let first = download(&dir, &r, &fake, &AtomicBool::new(false), &|_| {}).unwrap();
        let Outcome::Incomplete { failed } = first else {
            panic!("expected incomplete, got {first:?}");
        };
        assert!(failed > 0);
        let before = fake.calls.load(Ordering::SeqCst);
        fake.fail_odd.store(false, Ordering::SeqCst);
        let second = download(&dir, &r, &fake, &AtomicBool::new(false), &|_| {}).unwrap();
        assert_eq!(second, Outcome::Complete);
        let refetched = fake.calls.load(Ordering::SeqCst) - before;
        assert_eq!(refetched as u64, failed);
        assert!(r.tiles().iter().all(|t| is_pinned(&dir, *t)));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn cancel_stops_early_and_keeps_fetched_tiles_pinned() {
        let dir = root("cancel");
        let r = small("trip-a");
        let cancel = Arc::new(AtomicBool::new(false));
        let mut fake = Fake::new();
        fake.cancel_after = Some((3, cancel.clone()));
        let outcome = download(&dir, &r, &fake, &cancel, &|_| {}).unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
        let calls = fake.calls.load(Ordering::SeqCst);
        assert!(calls < r.tile_count() as usize);
        let pinned = r.tiles().iter().filter(|t| is_pinned(&dir, **t)).count();
        assert_eq!(pinned, calls);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn pinned_tiles_survive_eviction_until_the_region_is_deleted() {
        let dir = root("evict");
        let store = Store::in_memory().unwrap();
        let a = small("trip-a");
        let b = small("trip-b");
        let fake = Fake::new();
        let cancel = AtomicBool::new(false);
        download_and_record(&store, &dir, &a, &fake, &cancel, &|_| {}).unwrap();
        // The second region overlaps entirely: its tiles are pinned by both, with no refetch.
        let calls = fake.calls.load(Ordering::SeqCst);
        download_and_record(&store, &dir, &b, &fake, &cancel, &|_| {}).unwrap();
        assert_eq!(fake.calls.load(Ordering::SeqCst), calls);
        let unpinned = Tile { z: 12, x: 0, y: 0 };
        let loose = tile_path(&dir, Source::Topo, unpinned);
        fs::create_dir_all(loose.parent().unwrap()).unwrap();
        fs::write(&loose, [0; 10]).unwrap();
        evict(&dir, 0).unwrap();
        assert!(!loose.exists(), "unpinned tiles are evicted");
        assert!(
            a.tiles()
                .iter()
                .all(|t| tile_path(&dir, Source::Topo, *t).exists())
        );
        delete_region(&store, &dir, &a).unwrap();
        evict(&dir, 0).unwrap();
        assert!(
            a.tiles()
                .iter()
                .all(|t| tile_path(&dir, Source::Topo, *t).exists()),
            "still pinned by trip-b"
        );
        assert!(store.map_region("trip-a").unwrap().is_none());
        delete_region(&store, &dir, &b).unwrap();
        evict(&dir, 0).unwrap();
        assert!(
            a.tiles()
                .iter()
                .all(|t| !tile_path(&dir, Source::Topo, *t).exists())
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn region_state_is_persisted() {
        let dir = root("state");
        let store = Store::in_memory().unwrap();
        let r = small("trip-a");
        let cancel = Arc::new(AtomicBool::new(false));
        let mut fake = Fake::new();
        fake.cancel_after = Some((2, cancel.clone()));
        download_and_record(&store, &dir, &r, &fake, &cancel, &|_| {}).unwrap();
        let saved = store.map_region("trip-a").unwrap().unwrap();
        assert_eq!(saved.state, "cancelled");
        assert_eq!(saved.tiles_total, r.tile_count());
        assert!(saved.tiles_done >= 2 && saved.tiles_done < saved.tiles_total);
        assert_eq!(from_record(&saved), r);
        let fake = Fake::new();
        download_and_record(&store, &dir, &r, &fake, &AtomicBool::new(false), &|_| {}).unwrap();
        let saved = store.map_region("trip-a").unwrap().unwrap();
        assert_eq!(
            (saved.state.as_str(), saved.tiles_done),
            ("complete", r.tile_count())
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
