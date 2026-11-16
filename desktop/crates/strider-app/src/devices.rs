//! Devices: GPS volumes, the watched folder and the Attach GPX sheet.
use super::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strider_devices::{
    prefs::DevicePrefs,
    volumes::{self, DEFAULT_ROOT, Volume, VolumeMonitor},
    watch::{self, Candidate, FolderWatcher},
};

/// GPS volumes and the watched folder for one window.
pub(crate) struct DevicesState {
    monitor: VolumeMonitor,
    /// Where the per-device preferences live.
    prefs_dir: PathBuf,
    debounce: Duration,
    watcher: Option<FolderWatcher>,
    /// The watched folder, whether or not it could be opened.
    pub folder: Option<PathBuf>,
    /// The Attach GPX sheet is open.
    pub sheet: bool,
    /// Why the watched folder is not being watched.
    pub message: Option<String>,
}
impl DevicesState {
    /// Volumes are looked for under `root`; preferences are read from `prefs_dir`.
    pub fn new(root: PathBuf, prefs_dir: PathBuf) -> Self {
        let mut this = Self {
            monitor: VolumeMonitor::new(root),
            folder: DevicePrefs::load(&prefs_dir).watched_folder,
            prefs_dir,
            debounce: watch::DEBOUNCE,
            watcher: None,
            sheet: false,
            message: None,
        };
        this.watch();
        this
    }
    /// The root volumes are looked for under: `/Volumes`, or nowhere under test.
    pub fn default_root() -> PathBuf {
        if cfg!(test) {
            std::env::temp_dir().join("strider-no-volumes")
        } else {
            PathBuf::from(DEFAULT_ROOT)
        }
    }
    pub fn volumes(&self) -> &[Volume] {
        self.monitor.volumes()
    }
    /// The watched folder's recent GPX and FIT files, newest first.
    pub fn recent(&self) -> &[Candidate] {
        self.watcher.as_ref().map_or(&[], |w| w.recent())
    }
    fn watch(&mut self) {
        self.watcher = None;
        self.message = None;
        let Some(folder) = &self.folder else { return };
        match FolderWatcher::with_debounce(folder, self.debounce) {
            Ok(w) => self.watcher = Some(w),
            Err(_) => {
                self.message = Some(format!("{} is not available.", folder.display()));
            }
        }
    }
}

fn file_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.)
    } else {
        format!("{} KB", bytes.div_ceil(1024).max(1))
    }
}
fn unix(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

impl AppView {
    /// Take in volume and folder changes. Called from the 200 ms poll loop; returns whether the
    /// screen needs to redraw.
    pub(super) fn poll_devices(&mut self) -> bool {
        let volumes = !self.devices.monitor.poll().is_empty();
        let files = self.devices.watcher.as_mut().is_some_and(|w| w.poll());
        volumes || files
    }
    /// Remember (or forget) the watched folder and start watching it.
    pub(super) fn set_watched_folder(&mut self, folder: Option<PathBuf>, cx: &mut Context<Self>) {
        self.devices.folder = folder.clone();
        self.devices.watch();
        let prefs = DevicePrefs {
            watched_folder: folder,
        };
        if let Err(e) = prefs.save(&self.devices.prefs_dir) {
            self.error = Some(format!("Couldn't save the watched folder: {e}"));
        }
        cx.notify();
    }
    pub(super) fn choose_watched_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose the folder to watch for GPX and FIT files".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(folder) = paths.into_iter().next() {
                        this.set_watched_folder(Some(folder), cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.error = Some("Could not open folder picker.".into());
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub(super) fn open_attach_sheet(&mut self, cx: &mut Context<Self>) {
        self.devices.sheet = true;
        cx.notify();
    }
    /// Write the open Trip's GPX into the `Garmin/GPX` folder of the named volume.
    pub(super) fn send_trip_gpx(&mut self, volume: &str, cx: &mut Context<Self>) {
        let Some(target) = self
            .devices
            .volumes()
            .iter()
            .find(|v| v.name == volume)
            .cloned()
        else {
            self.error = Some(format!("{volume} is no longer available."));
            cx.notify();
            return;
        };
        let Some((filename, xml)) = self.trip_gpx() else {
            return;
        };
        let task = cx
            .background_executor()
            .spawn(async move { volumes::send_gpx(&target, &filename, xml.as_bytes()) });
        let name = volume.to_owned();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.error = None;
                        this.toast(format!("Sent to {name}"));
                    }
                    Err(e) => this.error = Some(format!("Couldn't send to {name}: {e}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
    /// The Attach GPX sheet: recent files from the watched folder, then a file picker.
    pub(super) fn render_attach_sheet(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let now = self.now();
        let mut body = div().flex().flex_col().gap_3();
        let recent = self.devices.recent();
        if recent.is_empty() {
            body = body.child(div().meta(t).child(match &self.devices.folder {
                Some(_) => "No recent GPX or FIT files in the watched folder.",
                None => "Choose a watched folder in Settings to see new downloads here.",
            }));
        }
        for (i, c) in recent.iter().enumerate() {
            let path = c.path.clone();
            body = body.child(
                strider_ui::button(
                    ("recent-file", i),
                    format!(
                        "{} · {} · {}",
                        c.name,
                        file_size(c.size),
                        super::offline::format::age(now, unix(c.modified))
                    ),
                    ButtonVariant::Ghost,
                    t,
                )
                .on_click(cx.listener(move |this, _, w, cx| {
                    this.close_overlays(w, cx);
                    this.trip_file(path.clone(), cx);
                })),
            );
        }
        body = body.child(
            strider_ui::button("choose-gpx-file", "Choose file…", ButtonVariant::Outline, t)
                .on_click(cx.listener(|this, _, w, cx| {
                    this.close_overlays(w, cx);
                    this.pick_trip_file(cx);
                })),
        );
        self.modal("Attach GPX", body.into_any_element(), cx)
    }
    /// The Settings rows for the watched folder and the detected GPS volumes.
    pub(super) fn devices_rows(&self, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let folder = self.devices.folder.as_deref().map(Path::display);
        let mut rows = div().flex().flex_col().gap_2().child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child("Watched folder")
                .child(div().flex_1().meta(t).child(match folder {
                    Some(f) => f.to_string(),
                    None => "None".into(),
                }))
                .child(
                    strider_ui::button("choose-watched", "Choose…", ButtonVariant::Outline, t)
                        .on_click(cx.listener(|this, _, _, cx| this.choose_watched_folder(cx))),
                )
                .when(self.devices.folder.is_some(), |d| {
                    d.child(
                        strider_ui::button("clear-watched", "Clear", ButtonVariant::Outline, t)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.set_watched_folder(None, cx)),
                            ),
                    )
                }),
        );
        if let Some(m) = &self.devices.message {
            rows = rows.child(div().text_color(t.signal).child(m.clone()));
        }
        let volumes = self.devices.volumes();
        rows.child(div().meta(t).child(if volumes.is_empty() {
            "No GPS detected. Plug in a Garmin in mass-storage mode.".to_owned()
        } else {
            format!(
                "GPS detected: {}",
                volumes
                    .iter()
                    .map(|v| v.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::DevicesState;
    use crate::app::test_support::{headless, open_routed_trip, temp_dir, with_view};
    use gpui::TestAppContext;
    use std::fs;
    use std::time::Duration;
    use strider_devices::prefs::DevicePrefs;
    use strider_devices::testing::encode_fit;

    #[gpui::test]
    fn send_to_a_fake_volume_writes_the_gpx_and_toasts(cx: &mut TestAppContext) {
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        let root = temp_dir("send-root");
        let gpx_dir = root.join("GARMIN/Garmin/GPX");
        fs::create_dir_all(&gpx_dir).expect("fake volume");
        open_routed_trip(&view, "Send trip", &mut cx);
        with_view(&view, &mut cx, |g, _, _| {
            g.devices = DevicesState::new(root.clone(), root.join("prefs"));
            assert_eq!(g.devices.volumes().len(), 1);
            assert_eq!(g.devices.volumes()[0].name, "GARMIN");
        });
        with_view(&view, &mut cx, |g, _, cx| g.send_trip_gpx("GARMIN", cx));
        cx.run_until_parked();
        view.read_with(&cx, |g, _| {
            assert!(g.error.is_none(), "{:?}", g.error);
            assert_eq!(
                g.toast.as_ref().map(|t| t.0.as_str()),
                Some("Sent to GARMIN")
            );
        });
        let sent = fs::read_to_string(gpx_dir.join("Send-trip.gpx")).expect("sent file");
        assert!(sent.contains("<trkpt lat=\"39.950000\""));
        // A vanished volume is an inline error, not a panic.
        with_view(&view, &mut cx, |g, _, cx| g.send_trip_gpx("GONE", cx));
        view.read_with(&cx, |g, _| {
            assert_eq!(g.error.as_deref(), Some("GONE is no longer available."));
        });
        fs::remove_dir_all(root).ok();
    }

    #[gpui::test]
    fn a_new_file_in_the_watched_folder_shows_in_the_sheet(cx: &mut TestAppContext) {
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        let watched = temp_dir("watched");
        let prefs = temp_dir("watched-prefs");
        with_view(&view, &mut cx, |g, _, cx| {
            g.devices = DevicesState::new(DevicesState::default_root(), prefs.clone());
            g.devices.debounce = Duration::from_millis(150);
            g.set_watched_folder(Some(watched.clone()), cx);
            g.open_attach_sheet(cx);
            assert!(g.devices.recent().is_empty());
        });
        fs::write(watched.join("ridge.gpx.crdownload"), "partial").expect("partial");
        fs::write(watched.join("ridge.gpx"), "<gpx/>").expect("file");
        let mut seen = false;
        for _ in 0..200 {
            seen = with_view(&view, &mut cx, |g, _, _| {
                g.poll_devices();
                g.devices.recent().len() == 1
            });
            if seen {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(seen, "the new file never reached the sheet");
        cx.run_until_parked();
        view.read_with(&cx, |g, _| {
            assert!(g.devices.sheet);
            assert_eq!(g.devices.recent()[0].name, "ridge.gpx");
        });
        // The folder is a per-device preference that survives a restart.
        assert_eq!(
            DevicePrefs::load(&prefs).watched_folder.as_deref(),
            Some(watched.as_path())
        );
        with_view(&view, &mut cx, |g, _, cx| {
            g.set_watched_folder(None, cx);
            assert!(g.devices.recent().is_empty() && g.devices.folder.is_none());
        });
        assert_eq!(DevicePrefs::load(&prefs).watched_folder, None);
        fs::remove_dir_all(watched).ok();
        fs::remove_dir_all(prefs).ok();
    }

    #[gpui::test]
    fn attaching_a_fit_file_sets_distance_and_gain(cx: &mut TestAppContext) {
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        let id = open_routed_trip(&view, "Fit trip", &mut cx);
        let points: Vec<_> = (0..40)
            .map(|i| {
                let t = f64::from(i);
                (39.95 + t * 0.001, -105.3 + t * 0.001, 1800.0 + t * 12.0)
            })
            .collect();
        let dir = temp_dir("fit");
        let path = dir.join("ride.fit");
        fs::write(&path, encode_fit(&points)).expect("fit file");
        let expected = strider_devices::fit::parse_fit(&encode_fit(&points)).expect("stats");
        with_view(&view, &mut cx, |g, _, cx| {
            g.open_trip(id.clone(), cx);
            g.trip_file(path.clone(), cx);
        });
        cx.run_until_parked();
        view.read_with(&cx, |g, _| {
            assert!(g.error.is_none(), "{:?}", g.error);
            let d = g.trips.detail.as_ref().expect("detail");
            assert_eq!(d.route.distance_mi, Some(expected.distance_mi));
            assert_eq!(d.route.elevation_ft, Some(expected.elevation_gain_ft as _));
            assert!(d.route.elevation_ft.unwrap_or(0) > 1500);
        });
        // A damaged FIT file is an inline error and leaves the route alone.
        let bad = dir.join("bad.fit");
        fs::write(&bad, b"definitely not fit").expect("bad file");
        with_view(&view, &mut cx, |g, _, cx| g.trip_file(bad.clone(), cx));
        cx.run_until_parked();
        view.read_with(&cx, |g, _| {
            let e = g.error.as_deref().unwrap_or("");
            assert!(e.contains("Couldn't read that FIT file"), "{e}");
        });
        fs::remove_dir_all(dir).ok();
    }
}
