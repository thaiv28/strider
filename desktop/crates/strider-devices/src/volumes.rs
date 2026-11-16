//! Mounted GPS volumes and sending a GPX file to one.
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

/// Where macOS mounts external volumes.
pub const DEFAULT_ROOT: &str = "/Volumes";

/// A mounted volume that has a `Garmin/GPX` folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    /// The volume's name (its folder name under the root).
    pub name: String,
    pub path: PathBuf,
    /// The volume's `Garmin/GPX` folder, with the case the device uses.
    pub gpx_dir: PathBuf,
}

/// The child of `parent` named `name`, ignoring case.
fn child_ci(parent: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(parent)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
}

/// Volumes under `root` that contain `Garmin/GPX` (any case), sorted by name.
pub fn list_volumes(root: &Path) -> Vec<Volume> {
    let Ok(entries) = fs::read_dir(root) else {
        return vec![];
    };
    let mut out: Vec<Volume> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if !path.is_dir() {
                return None;
            }
            let gpx_dir = child_ci(&child_ci(&path, "Garmin")?, "GPX")?;
            Some(Volume {
                name: e.file_name().to_string_lossy().into_owned(),
                path,
                gpx_dir,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn split_name(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(i) if i > 0 => filename.split_at(i),
        _ => (filename, ""),
    }
}

/// Write `bytes` as `filename` into the volume's `Garmin/GPX` folder.
///
/// The data goes to a hidden temporary file that is renamed into place, so the device never sees
/// a half-written GPX. A file of the same name with different content is never replaced: the new
/// file becomes `name (2).gpx`, `name (3).gpx`, and so on. A file that already holds exactly these
/// bytes is reused. Returns the final path.
pub fn send_gpx(volume: &Volume, filename: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && !n.starts_with('.'))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name."))?;
    if !volume.gpx_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is no longer available.", volume.name),
        ));
    }
    let (stem, ext) = split_name(name);
    let mut target = volume.gpx_dir.join(name);
    let mut n = 1;
    while target.exists() {
        if fs::read(&target).is_ok_and(|existing| existing == bytes) {
            return Ok(target);
        }
        n += 1;
        target = volume.gpx_dir.join(format!("{stem} ({n}){ext}"));
    }
    let tmp = volume
        .gpx_dir
        .join(format!(".{stem}.{}.part", std::process::id()));
    let write = || -> io::Result<()> {
        use io::Write;
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, &target)
    };
    match write() {
        Ok(()) => Ok(target),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VolumeEvent {
    Appeared(Volume),
    Disappeared(String),
}

/// Reports GPS volumes appearing and disappearing under a root.
pub struct VolumeMonitor {
    root: PathBuf,
    known: Vec<Volume>,
    rx: Receiver<()>,
    /// Keeps the watch alive. `None` when the root cannot be watched (it does not exist).
    _watcher: Option<RecommendedWatcher>,
}
impl VolumeMonitor {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |_: notify::Result<notify::Event>| {
            let _ = tx.send(());
        })
        .ok()
        .and_then(|mut w| {
            w.watch(&root, RecursiveMode::NonRecursive).ok()?;
            Some(w)
        });
        let known = list_volumes(&root);
        Self {
            root,
            known,
            rx,
            _watcher: watcher,
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// The volumes found at the last poll.
    pub fn volumes(&self) -> &[Volume] {
        &self.known
    }
    /// Whether the root is being watched; without it only `rescan` finds changes.
    pub fn watching(&self) -> bool {
        self._watcher.is_some()
    }
    /// Rescan if the watcher saw anything since the last call. Never blocks.
    pub fn poll(&mut self) -> Vec<VolumeEvent> {
        let mut dirty = false;
        while self.rx.try_recv().is_ok() {
            dirty = true;
        }
        if dirty { self.rescan() } else { vec![] }
    }
    /// Rescan now and report what changed.
    pub fn rescan(&mut self) -> Vec<VolumeEvent> {
        let now = list_volumes(&self.root);
        let mut events: Vec<VolumeEvent> = self
            .known
            .iter()
            .filter(|v| !now.iter().any(|n| n.path == v.path))
            .map(|v| VolumeEvent::Disappeared(v.name.clone()))
            .collect();
        events.extend(
            now.iter()
                .filter(|n| !self.known.iter().any(|v| v.path == n.path))
                .map(|v| VolumeEvent::Appeared(v.clone())),
        );
        self.known = now;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_dir;
    use std::time::{Duration, Instant};

    fn gps(root: &Path, name: &str, gpx: &str) -> PathBuf {
        let dir = root.join(name).join(gpx);
        fs::create_dir_all(&dir).expect("gps dirs");
        dir
    }

    #[test]
    fn lists_only_volumes_with_a_garmin_gpx_folder_ignoring_case() {
        let root = temp_dir("vol-list");
        gps(&root, "GARMIN", "Garmin/GPX");
        gps(&root, "lower", "garmin/gpx");
        gps(&root, "Camera", "DCIM/100");
        gps(&root, "NoGpx", "Garmin/Courses");
        fs::write(root.join("file.txt"), "x").expect("file");
        let names: Vec<_> = list_volumes(&root).into_iter().map(|v| v.name).collect();
        assert_eq!(names, ["GARMIN", "lower"]);
        assert!(list_volumes(&root.join("missing")).is_empty());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn send_writes_atomically_and_never_overwrites_a_different_file() {
        let root = temp_dir("vol-send");
        let dir = gps(&root, "GARMIN", "Garmin/GPX");
        let volume = list_volumes(&root).remove(0);
        let first = send_gpx(&volume, "Loop.gpx", b"one").expect("send");
        assert_eq!(first, dir.join("Loop.gpx"));
        assert_eq!(fs::read(&first).expect("read"), b"one");
        // The same bytes again reuse the file.
        assert_eq!(send_gpx(&volume, "Loop.gpx", b"one").expect("same"), first);
        // Different bytes get a numbered name; the original is untouched.
        let second = send_gpx(&volume, "Loop.gpx", b"two").expect("second");
        assert_eq!(second, dir.join("Loop (2).gpx"));
        let third = send_gpx(&volume, "Loop.gpx", b"three").expect("third");
        assert_eq!(third, dir.join("Loop (3).gpx"));
        assert_eq!(fs::read(&first).expect("read"), b"one");
        assert_eq!(fs::read(&second).expect("read"), b"two");
        // No temporary file is left behind.
        let mut names: Vec<_> = fs::read_dir(&dir)
            .expect("dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["Loop (2).gpx", "Loop (3).gpx", "Loop.gpx"]);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn send_rejects_bad_names_and_vanished_volumes() {
        let root = temp_dir("vol-bad");
        gps(&root, "GARMIN", "Garmin/GPX");
        let volume = list_volumes(&root).remove(0);
        assert!(send_gpx(&volume, "", b"x").is_err());
        assert!(send_gpx(&volume, ".hidden.gpx", b"x").is_err());
        // A path in the name is reduced to its file name; it cannot escape the folder.
        let sent = send_gpx(&volume, "../../evil.gpx", b"x").expect("send");
        assert_eq!(sent.parent(), Some(volume.gpx_dir.as_path()));
        fs::remove_dir_all(&volume.path).expect("unmount");
        let err = send_gpx(&volume, "a.gpx", b"x").expect_err("gone");
        assert!(err.to_string().contains("no longer available"));
        fs::remove_dir_all(root).ok();
    }

    fn wait_for(monitor: &mut VolumeMonitor, want: impl Fn(&VolumeEvent) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if monitor.poll().iter().any(&want) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn monitor_reports_volumes_appearing_and_disappearing() {
        let root = temp_dir("vol-watch");
        let mut monitor = VolumeMonitor::new(&root);
        assert!(monitor.watching());
        assert!(monitor.volumes().is_empty());
        gps(&root, "GARMIN", "Garmin/GPX");
        assert!(wait_for(&mut monitor, |e| matches!(
            e,
            VolumeEvent::Appeared(v) if v.name == "GARMIN"
        )));
        assert_eq!(monitor.volumes().len(), 1);
        fs::remove_dir_all(root.join("GARMIN")).expect("eject");
        assert!(wait_for(
            &mut monitor,
            |e| matches!(e, VolumeEvent::Disappeared(n) if n == "GARMIN")
        ));
        assert!(monitor.volumes().is_empty());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_missing_root_is_not_watched_but_scans_empty() {
        let mut monitor = VolumeMonitor::new(temp_dir("vol-none").join("missing"));
        assert!(!monitor.watching());
        assert!(monitor.poll().is_empty() && monitor.rescan().is_empty());
    }
}
