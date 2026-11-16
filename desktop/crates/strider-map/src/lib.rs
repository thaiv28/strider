//! UI-independent, clamped Web Mercator viewport and bounded raster tile cache.
pub mod region;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, SystemTime},
};
pub const TILE: f64 = 256.;
pub const MAX_LAT: f64 = 85.0511287798066;
pub const DEFAULT_DISK_CAP: u64 = 500 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30 * 86400);
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum Source {
    #[default]
    Topo,
    Osm,
}
impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Self::Topo => "topo",
            Self::Osm => "osm",
        }
    }
    pub fn max_zoom(self) -> u8 {
        match self {
            Self::Topo => 16,
            Self::Osm => 19,
        }
    }
    pub fn attribution(self) -> &'static str {
        match self {
            Self::Topo => "USGS The National Map",
            Self::Osm => "© OpenStreetMap contributors",
        }
    }
    pub fn url(self, t: Tile) -> String {
        match self {
            Self::Topo => format!(
                "https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/{}/{}/{}",
                t.z, t.y, t.x
            ),
            Self::Osm => format!("https://tile.openstreetmap.org/{}/{}/{}.png", t.z, t.x, t.y),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixel {
    pub x: f64,
    pub y: f64,
}
pub fn project(lat: f64, lon: f64, zoom: f64) -> Pixel {
    let size = TILE * 2_f64.powf(zoom.clamp(0., 22.));
    let r = lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    Pixel {
        x: (lon.clamp(-180., 180.) + 180.) / 360. * size,
        y: ((1. - r.tan().asinh() / std::f64::consts::PI) / 2. * size).clamp(0., size),
    }
}
pub fn unproject(p: Pixel, zoom: f64) -> (f64, f64) {
    let size = TILE * 2_f64.powf(zoom.clamp(0., 22.));
    (
        (std::f64::consts::PI * (1. - 2. * p.y.clamp(0., size) / size))
            .sinh()
            .atan()
            .to_degrees(),
        p.x.clamp(0., size) / size * 360. - 180.,
    )
}
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Tile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}
impl Tile {
    pub fn parent(self) -> Option<Self> {
        (self.z > 0).then(|| Self {
            z: self.z - 1,
            x: self.x / 2,
            y: self.y / 2,
        })
    }
    pub fn valid(self) -> bool {
        self.z <= 22 && self.x < (1 << self.z) && self.y < (1 << self.z)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub center: (f64, f64),
    pub zoom: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            center: (39., -105.),
            zoom: 5.,
        }
    }
}
impl Viewport {
    pub fn origin(self, width: f64, height: f64) -> Pixel {
        let p = project(self.center.0, self.center.1, self.zoom);
        Pixel {
            x: p.x - width / 2.,
            y: p.y - height / 2.,
        }
    }
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let p = project(self.center.0, self.center.1, self.zoom);
        self.center = unproject(
            Pixel {
                x: p.x - dx,
                y: p.y - dy,
            },
            self.zoom,
        );
    }
    pub fn zoom_at(&mut self, delta: f64, anchor: Pixel, width: f64, height: f64) {
        let old = self.origin(width, height);
        let location = unproject(
            Pixel {
                x: old.x + anchor.x,
                y: old.y + anchor.y,
            },
            self.zoom,
        );
        self.zoom = (self.zoom + delta).clamp(0., 22.);
        let p = project(location.0, location.1, self.zoom);
        self.center = unproject(
            Pixel {
                x: p.x - anchor.x + width / 2.,
                y: p.y - anchor.y + height / 2.,
            },
            self.zoom,
        );
    }
    pub fn tiles(self, width: f64, height: f64, source: Source) -> Vec<Tile> {
        if width <= 0. || height <= 0. || !width.is_finite() || !height.is_finite() {
            return vec![];
        }
        let z = (self.zoom.floor() as u8).min(source.max_zoom());
        let scale = 2_f64.powf(self.zoom - f64::from(z));
        let origin = self.origin(width, height);
        let edge = f64::from(1_u32 << z);
        let lo = |v: f64| (v / scale / TILE).floor().clamp(0., edge) as u32;
        let hi = |v: f64| (v / scale / TILE).ceil().clamp(0., edge) as u32;
        let mut out = vec![];
        for y in lo(origin.y)..hi(origin.y + height) {
            for x in lo(origin.x)..hi(origin.x + width) {
                out.push(Tile { z, x, y });
            }
        }
        out
    }
}
pub fn fit_bounds(
    track: &[[f64; 2]],
    width: f64,
    height: f64,
    padding: f64,
    max_zoom: f64,
) -> Viewport {
    let pts: Vec<_> = track
        .iter()
        .filter(|p| p.iter().all(|n| n.is_finite()))
        .map(|p| project(p[0], p[1], 0.))
        .collect();
    if pts.is_empty() {
        return Viewport::default();
    }
    let (mut min, mut max) = (pts[0], pts[0]);
    for p in pts {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    let zoom = (((width - 2. * padding).max(1.) / (max.x - min.x).max(1e-9))
        .min((height - 2. * padding).max(1.) / (max.y - min.y).max(1e-9)))
    .log2()
    .clamp(0., max_zoom.clamp(0., 22.));
    Viewport {
        center: unproject(
            Pixel {
                x: (min.x + max.x) / 2.,
                y: (min.y + max.y) / 2.,
            },
            0.,
        ),
        zoom,
    }
}
/// First available ancestor, and normalized crop within that ancestor.
pub fn parent_fallback(
    tile: Tile,
    mut available: impl FnMut(Tile) -> bool,
) -> Option<(Tile, [f64; 4])> {
    let mut parent = tile;
    while let Some(p) = parent.parent() {
        parent = p;
        if available(p) {
            let n = 1_u32 << (tile.z - p.z);
            let size = 1. / f64::from(n);
            return Some((
                p,
                [
                    f64::from(tile.x % n) * size,
                    f64::from(tile.y % n) * size,
                    size,
                    size,
                ],
            ));
        }
    }
    None
}
pub struct Fetch {
    pub bytes: Vec<u8>,
    pub max_age: Duration,
    pub store: bool,
}
pub trait TileFetcher: Send + Sync + 'static {
    fn fetch(&self, source: Source, tile: Tile) -> io::Result<Fetch>;
}
pub struct HttpFetcher;
impl TileFetcher for HttpFetcher {
    fn fetch(&self, source: Source, tile: Tile) -> io::Result<Fetch> {
        let mut response = ureq::get(source.url(tile))
            .header(
                "User-Agent",
                concat!("StriderDesktop/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .map_err(io::Error::other)?;
        let control = response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let age = control
            .split(',')
            .find_map(|s| s.trim().strip_prefix("max-age=")?.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(MAX_AGE)
            .min(MAX_AGE);
        let bytes = response
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_vec()
            .map_err(io::Error::other)?;
        if !(bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.starts_with(&[255, 216, 255])) {
            return Err(io::Error::other("Unsupported tile image"));
        }
        Ok(Fetch {
            bytes,
            max_age: age,
            store: !control.contains("no-store"),
        })
    }
}
type Key = (Source, Tile);
struct Memory {
    clock: u64,
    size: usize,
    cap: usize,
    entries: HashMap<Key, (Arc<[u8]>, u64)>,
    pending: HashSet<Key>,
}
impl Memory {
    fn put(&mut self, key: Key, bytes: Arc<[u8]>) {
        self.clock += 1;
        if let Some((old, _)) = self.entries.remove(&key) {
            self.size -= old.len();
        }
        self.size += bytes.len();
        self.entries.insert(key, (bytes, self.clock));
        while self.size > self.cap {
            let Some(key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.1)
                .map(|(k, _)| *k)
            else {
                break;
            };
            if let Some((bytes, _)) = self.entries.remove(&key) {
                self.size -= bytes.len();
            }
        }
    }
}
type Callback = Box<dyn FnOnce(io::Result<Arc<[u8]>>) + Send>;
struct Job {
    key: Key,
    callback: Callback,
}
/// Nonblocking requests. At most four worker threads; duplicate in-flight requests
/// are coalesced. Callback executes on a worker, including disk hits.
pub struct TileCache {
    root: PathBuf,
    memory: Arc<Mutex<Memory>>,
    sender: mpsc::SyncSender<Job>,
}
impl TileCache {
    pub fn new(
        root: PathBuf,
        cap: u64,
        memory_cap: usize,
        workers: usize,
        fetcher: Arc<dyn TileFetcher>,
    ) -> io::Result<Self> {
        let root = root.join("tiles");
        fs::create_dir_all(&root)?;
        evict(&root, cap)?;
        let memory = Arc::new(Mutex::new(Memory {
            clock: 0,
            size: 0,
            cap: memory_cap,
            entries: HashMap::new(),
            pending: HashSet::new(),
        }));
        let (sender, receiver) = mpsc::sync_channel::<Job>(128);
        let receiver = Arc::new(Mutex::new(receiver));
        // Serial disk writes/eviction avoid temporary-file and eviction races.
        let disk = Arc::new(Mutex::new(()));
        for i in 0..workers.clamp(1, 4) {
            let (root, memory, receiver, fetcher, disk) = (
                root.clone(),
                memory.clone(),
                receiver.clone(),
                fetcher.clone(),
                disk.clone(),
            );
            thread::Builder::new()
                .name(format!("tiles-{i}"))
                .spawn(move || {
                    loop {
                        let job = match receiver.lock() {
                            Ok(rx) => rx.recv(),
                            Err(_) => return,
                        };
                        let Ok(job) = job else { return };
                        let (source, tile) = job.key;
                        let path = tile_path(&root, source, tile);
                        let result = (|| {
                            let _guard = disk
                                .lock()
                                .map_err(|_| io::Error::other("Tile cache lock"))?;
                            if fresh(&path) {
                                let bytes = fs::read(&path)?;
                                fs::write(path.with_extension("used"), [])?;
                                return Ok(Arc::<[u8]>::from(bytes));
                            }
                            drop(_guard);
                            let fetched = fetcher.fetch(source, tile)?;
                            let bytes: Arc<[u8]> = fetched.bytes.into();
                            if fetched.store {
                                let _guard = disk
                                    .lock()
                                    .map_err(|_| io::Error::other("Tile cache lock"))?;
                                if let Some(parent) = path.parent() {
                                    fs::create_dir_all(parent)?;
                                }
                                let tmp = path.with_extension("tmp");
                                fs::write(&tmp, &bytes)?;
                                fs::rename(tmp, &path)?;
                                let expiry = SystemTime::now() + fetched.max_age.min(MAX_AGE);
                                let secs = expiry
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                fs::write(path.with_extension("expires"), secs.to_string())?;
                                fs::write(path.with_extension("used"), [])?;
                                evict(&root, cap)?;
                            }
                            Ok(bytes)
                        })();
                        if let Ok(mut m) = memory.lock() {
                            m.pending.remove(&job.key);
                            if let Ok(bytes) = &result {
                                m.put(job.key, bytes.clone());
                            }
                        }
                        (job.callback)(result);
                    }
                })?;
        }
        Ok(Self {
            root,
            memory,
            sender,
        })
    }
    pub fn get(&self, source: Source, tile: Tile) -> Option<Arc<[u8]>> {
        let mut m = self.memory.lock().ok()?;
        m.clock += 1;
        let clock = m.clock;
        let (bytes, used) = m.entries.get_mut(&(source, tile))?;
        *used = clock;
        Some(bytes.clone())
    }
    pub fn request(
        &self,
        source: Source,
        tile: Tile,
        callback: impl FnOnce(io::Result<Arc<[u8]>>) + Send + 'static,
    ) {
        if !tile.valid() || tile.z > source.max_zoom() {
            return;
        }
        let Ok(mut m) = self.memory.lock() else {
            return;
        };
        if !m.pending.insert((source, tile)) {
            return;
        }
        if self
            .sender
            .try_send(Job {
                key: (source, tile),
                callback: Box::new(callback),
            })
            .is_err()
        {
            m.pending.remove(&(source, tile));
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}
fn tile_path(root: &Path, source: Source, tile: Tile) -> PathBuf {
    root.join(source.name())
        .join(tile.z.to_string())
        .join(tile.x.to_string())
        .join(tile.y.to_string())
}
/// Marker beside a tile file naming the offline regions that pin it.
pub(crate) fn pin_path(path: &Path) -> PathBuf {
    path.with_extension("pin")
}
fn pinned(path: &Path) -> bool {
    pin_path(path).exists()
}
fn fresh(path: &Path) -> bool {
    // Pinned tiles are an offline guarantee: they never expire.
    if pinned(path) && path.exists() {
        return true;
    }
    let age = path
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok());
    let expires = fs::read_to_string(path.with_extension("expires"))
        .ok()
        .and_then(|s| s.parse::<u64>().ok());
    age.is_some_and(|a| a < MAX_AGE)
        && expires
            .is_some_and(|t| SystemTime::now() < SystemTime::UNIX_EPOCH + Duration::from_secs(t))
}
fn evict(root: &Path, cap: u64) -> io::Result<()> {
    fn walk(path: &Path, out: &mut Vec<(SystemTime, PathBuf, u64)>) -> io::Result<()> {
        for e in fs::read_dir(path)? {
            let e = e?;
            let path = e.path();
            if e.file_type()?.is_dir() {
                walk(&path, out)?;
            } else if path.extension().is_none() && !pinned(&path) {
                let meta = e.metadata()?;
                let used = path
                    .with_extension("used")
                    .metadata()
                    .and_then(|m| m.modified())
                    .or_else(|_| meta.modified())?;
                out.push((used, path, meta.len()));
            }
        }
        Ok(())
    }
    let mut files = vec![];
    walk(root, &mut files)?;
    files.sort_by_key(|f| f.0);
    let mut total: u64 = files.iter().map(|f| f.2).sum();
    for (_, path, size) in files {
        if total <= cap {
            break;
        }
        fs::remove_file(&path)?;
        for ext in ["used", "expires"] {
            let _ = fs::remove_file(path.with_extension(ext));
        }
        total -= size;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_round_trips() {
        for z in [0., 4.25, 16., 19.5] {
            for (lat, lon) in [(0., 0.), (47.5, -122.), (-80., 179.99), (MAX_LAT, 180.)] {
                let (a, b) = unproject(project(lat, lon, z), z);
                assert!((a - lat).abs() < 1e-8);
                assert!((b - lon).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn boundaries_and_antimeridian_clamp() {
        let v = Viewport {
            center: (0., 0.),
            zoom: 0.,
        };
        assert_eq!(
            v.tiles(256., 256., Source::Topo),
            vec![Tile { z: 0, x: 0, y: 0 }]
        );
        for lon in [-180., 180.] {
            let tiles = Viewport {
                center: (0., lon),
                zoom: 2.,
            }
            .tiles(256., 256., Source::Topo);
            assert!(tiles.iter().all(|t| t.valid()));
            assert!(tiles.len() <= 4);
        }
        assert_eq!(project(90., 200., 0.), project(MAX_LAT, 180., 0.));
    }
    #[test]
    fn fit() {
        let track = [[40., -120.], [41., -119.]];
        let v = fit_bounds(&track, 800., 400., 30., 16.);
        let origin = v.origin(800., 400.);
        for p in track {
            let p = project(p[0], p[1], v.zoom);
            assert!(p.x - origin.x >= 29.999 && p.x - origin.x <= 770.001);
            assert!(p.y - origin.y >= 29.999 && p.y - origin.y <= 370.001);
        }
    }
    #[test]
    fn fallback() {
        let t = Tile { z: 4, x: 11, y: 6 };
        let (p, crop) = parent_fallback(t, |p| p.z == 2).unwrap();
        assert_eq!(p, Tile { z: 2, x: 2, y: 1 });
        assert_eq!(crop, [0.75, 0.5, 0.25, 0.25]);
        assert!(parent_fallback(t, |_| false).is_none());
    }
    struct Fake;
    impl TileFetcher for Fake {
        fn fetch(&self, _: Source, t: Tile) -> io::Result<Fetch> {
            Ok(Fetch {
                bytes: vec![t.x as u8; 32],
                max_age: MAX_AGE,
                store: true,
            })
        }
    }
    #[test]
    fn cache_eviction_and_disk_hit() {
        let root = std::env::temp_dir().join(format!("strider-tiles-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let c = TileCache::new(root.clone(), 32, 32, 1, Arc::new(Fake)).unwrap();
        for x in 0..2 {
            let (tx, rx) = mpsc::channel();
            c.request(Source::Topo, Tile { z: 1, x, y: 0 }, move |r| {
                tx.send(r).unwrap()
            });
            assert_eq!(
                rx.recv_timeout(Duration::from_secs(3))
                    .unwrap()
                    .unwrap()
                    .len(),
                32
            );
        }
        assert!(c.get(Source::Topo, Tile { z: 1, x: 0, y: 0 }).is_none());
        assert!(c.get(Source::Topo, Tile { z: 1, x: 1, y: 0 }).is_some());
        assert!(!tile_path(c.root(), Source::Topo, Tile { z: 1, x: 0, y: 0 }).exists());
        drop(c);
        fs::remove_dir_all(root).unwrap();
    }
}
