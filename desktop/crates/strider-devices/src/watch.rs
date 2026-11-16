//! The watched folder: new or changed GPX and FIT files, once they have finished downloading.
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime},
};

/// How long a file must be quiet before it is offered.
pub const DEBOUNCE: Duration = Duration::from_secs(1);
/// How many recent files are kept.
pub const RECENT: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

/// A browser's unfinished download: `.crdownload`, `.download`, `.part` (also as a suffix after
/// the real extension, as in `route.gpx.crdownload`).
pub fn is_partial(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".crdownload", ".download", ".part"]
        .iter()
        .any(|s| lower.ends_with(s))
}
/// A `.gpx` or `.fit` file that is not hidden and not a partial download.
pub fn is_candidate_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.starts_with('.')
        && !is_partial(name)
        && (lower.ends_with(".gpx") || lower.ends_with(".fit"))
}

fn candidate(path: &Path) -> Option<Candidate> {
    let name = path.file_name()?.to_str()?.to_owned();
    if !is_candidate_name(&name) {
        return None;
    }
    let meta = fs::metadata(path).ok().filter(|m| m.is_file())?;
    Some(Candidate {
        name,
        path: path.to_owned(),
        size: meta.len(),
        modified: meta.modified().ok()?,
    })
}

struct Pending {
    due: Instant,
    size: u64,
}

pub struct FolderWatcher {
    folder: PathBuf,
    debounce: Duration,
    rx: Receiver<notify::Result<notify::Event>>,
    _watcher: RecommendedWatcher,
    pending: HashMap<PathBuf, Pending>,
    recent: Vec<Candidate>,
}
impl FolderWatcher {
    pub fn new(folder: &Path) -> notify::Result<Self> {
        Self::with_debounce(folder, DEBOUNCE)
    }
    /// Watch `folder` (not its subfolders). Files already there are listed at once.
    pub fn with_debounce(folder: &Path, debounce: Duration) -> notify::Result<Self> {
        let folder = fs::canonicalize(folder).map_err(notify::Error::io)?;
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |e| {
            let _ = tx.send(e);
        })?;
        watcher.watch(&folder, RecursiveMode::NonRecursive)?;
        let mut this = Self {
            folder,
            debounce,
            rx,
            _watcher: watcher,
            pending: HashMap::new(),
            recent: vec![],
        };
        this.scan();
        Ok(this)
    }
    pub fn folder(&self) -> &Path {
        &self.folder
    }
    /// The most recent candidates, newest first (at most [`RECENT`]).
    pub fn recent(&self) -> &[Candidate] {
        &self.recent
    }
    fn scan(&mut self) {
        let Ok(entries) = fs::read_dir(&self.folder) else {
            return;
        };
        for e in entries.flatten() {
            if let Some(c) = candidate(&e.path()) {
                self.accept(c);
            }
        }
    }
    fn accept(&mut self, c: Candidate) {
        self.recent.retain(|r| r.path != c.path);
        self.recent.push(c);
        self.recent.sort_by(|a, b| {
            b.modified
                .cmp(&a.modified)
                .then_with(|| a.name.cmp(&b.name))
        });
        self.recent.truncate(RECENT);
    }
    /// Take in file system events and settle files that have been quiet for the debounce
    /// interval. Returns whether the list changed. Never blocks.
    pub fn poll(&mut self) -> bool {
        self.poll_at(Instant::now())
    }
    fn poll_at(&mut self, now: Instant) -> bool {
        let mut changed = false;
        while let Ok(event) = self.rx.try_recv() {
            let Ok(event) = event else { continue };
            if matches!(event.kind, EventKind::Access(_)) {
                continue;
            }
            for path in event.paths {
                let in_folder = path.parent() == Some(self.folder.as_path());
                let named = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(is_candidate_name);
                if !in_folder || !named {
                    continue;
                }
                let size = fs::metadata(&path).map_or(0, |m| m.len());
                self.pending.insert(
                    path,
                    Pending {
                        due: now + self.debounce,
                        size,
                    },
                );
            }
        }
        let due: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, p)| p.due <= now)
            .map(|(path, _)| path.clone())
            .collect();
        for path in due {
            let Some(pending) = self.pending.remove(&path) else {
                continue;
            };
            match candidate(&path) {
                None => {
                    // Deleted, renamed away or no longer a plain file.
                    let before = self.recent.len();
                    self.recent.retain(|r| r.path != path);
                    changed |= self.recent.len() != before;
                }
                Some(c) if c.size != pending.size => {
                    // Still growing: look again after another quiet interval.
                    self.pending.insert(
                        path,
                        Pending {
                            due: now + self.debounce,
                            size: c.size,
                        },
                    );
                }
                Some(c) => {
                    if self.recent.iter().all(|r| r != &c) {
                        changed = true;
                    }
                    self.accept(c);
                }
            }
        }
        changed
    }
}

/// Convenience for callers that only have a path string to validate.
pub fn check_folder(folder: &Path) -> io::Result<()> {
    if folder.is_dir() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "That folder is not available.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_dir;

    const SHORT: Duration = Duration::from_millis(300);

    fn names(w: &FolderWatcher) -> Vec<String> {
        w.recent().iter().map(|c| c.name.clone()).collect()
    }
    /// Poll for up to `secs` seconds until `done` holds.
    fn until(w: &mut FolderWatcher, secs: u64, done: impl Fn(&FolderWatcher) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            w.poll();
            if done(w) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        false
    }

    #[test]
    fn name_rules() {
        assert!(is_candidate_name("a.gpx") && is_candidate_name("B.FIT"));
        assert!(!is_candidate_name("a.gpx.crdownload"));
        assert!(!is_candidate_name("a.gpx.download"));
        assert!(!is_candidate_name("a.fit.part"));
        assert!(!is_candidate_name(".a.gpx"));
        assert!(!is_candidate_name("a.txt"));
        assert_eq!(DEBOUNCE, Duration::from_secs(1));
    }

    #[test]
    fn existing_files_are_listed_newest_first_and_capped() {
        let dir = temp_dir("watch-scan");
        for i in 0..25 {
            fs::write(dir.join(format!("r{i:02}.gpx")), "x").expect("write");
        }
        fs::write(dir.join("skip.txt"), "x").expect("write");
        fs::write(dir.join("r.gpx.crdownload"), "x").expect("write");
        let w = FolderWatcher::with_debounce(&dir, SHORT).expect("watch");
        assert_eq!(w.recent().len(), RECENT);
        assert!(
            w.recent()
                .windows(2)
                .all(|p| p[0].modified >= p[1].modified)
        );
        assert!(w.recent().iter().all(|c| c.size == 1));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_new_file_appears_only_after_the_debounce() {
        let dir = temp_dir("watch-new");
        let mut w = FolderWatcher::with_debounce(&dir, SHORT).expect("watch");
        assert!(w.recent().is_empty());
        let started = Instant::now();
        fs::write(dir.join("ridge.gpx"), "<gpx/>").expect("write");
        // Let the event arrive, then confirm the file is held back until the interval passes.
        let mut early = false;
        while started.elapsed() < Duration::from_millis(200) {
            w.poll();
            early |= !w.recent().is_empty();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!early, "listed before the debounce interval");
        assert!(until(&mut w, 10, |w| names(w) == ["ridge.gpx"]));
        assert!(started.elapsed() >= SHORT);
        assert_eq!(w.recent()[0].size, 6);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn partial_downloads_and_other_files_are_ignored_until_renamed() {
        let dir = temp_dir("watch-partial");
        let mut w = FolderWatcher::with_debounce(&dir, SHORT).expect("watch");
        fs::write(dir.join("hike.gpx.crdownload"), "partial").expect("write");
        fs::write(dir.join("hike.gpx.download"), "partial").expect("write");
        fs::write(dir.join("hike.part"), "partial").expect("write");
        fs::write(dir.join("notes.txt"), "text").expect("write");
        assert!(!until(&mut w, 2, |w| !w.recent().is_empty()));
        // The browser finishes: the file is renamed to its real name.
        fs::rename(dir.join("hike.gpx.crdownload"), dir.join("hike.gpx")).expect("rename");
        assert!(until(&mut w, 10, |w| names(w) == ["hike.gpx"]));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_file_whose_size_is_still_changing_waits() {
        let dir = temp_dir("watch-growing");
        let mut w = FolderWatcher::with_debounce(&dir, SHORT).expect("watch");
        let path = dir.join("big.fit");
        fs::write(&path, "a").expect("write");
        let now = Instant::now();
        // Drain the creation event, then grow the file before the interval passes.
        let deadline = now + Duration::from_secs(5);
        while w.pending.is_empty() && Instant::now() < deadline {
            w.poll_at(Instant::now());
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!w.pending.is_empty(), "no event for the new file");
        fs::write(&path, "abcdef").expect("grow");
        // The recorded size (1) no longer matches at the due time, so nothing is listed yet.
        let due = Instant::now() + SHORT + Duration::from_millis(50);
        let changed = w.poll_at(due);
        assert!(!changed && w.recent().is_empty());
        assert!(!w.pending.is_empty(), "still waiting");
        // Once it stops changing, it is offered.
        assert!(until(&mut w, 10, |w| names(w) == ["big.fit"]));
        assert_eq!(w.recent()[0].size, 6);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_deleted_file_leaves_the_list_and_subfolders_are_not_watched() {
        let dir = temp_dir("watch-delete");
        fs::create_dir_all(dir.join("sub")).expect("sub");
        fs::write(dir.join("old.gpx"), "x").expect("write");
        let mut w = FolderWatcher::with_debounce(&dir, SHORT).expect("watch");
        assert_eq!(names(&w), ["old.gpx"]);
        fs::write(dir.join("sub/deep.gpx"), "x").expect("write");
        fs::remove_file(dir.join("old.gpx")).expect("remove");
        assert!(until(&mut w, 10, |w| w.recent().is_empty()));
        assert!(!until(&mut w, 1, |w| !w.recent().is_empty()));
        assert!(check_folder(&dir).is_ok() && check_folder(&dir.join("nope")).is_err());
        fs::remove_dir_all(dir).ok();
    }
}
