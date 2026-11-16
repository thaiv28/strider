//! Signed self-updates: the 30 s / 24 h schedule, the "Check for Updates…" command and the
//! update Sheet. The mechanics (manifest, signature, download, bundle checks) live in
//! `strider-update`; this file only drives them from the window that owns the account.
use super::*;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};
use strider_update::{Available, Config, HttpTransport, UpdateError};

const FIRST_CHECK: Duration = Duration::from_secs(30);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// Progress messages are sent at most once per this many bytes.
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Available),
    Downloading {
        done: u64,
        total: Option<u64>,
    },
    Ready(PathBuf),
    Failed(String),
}
enum Event {
    Checked(Result<Option<Available>, UpdateError>),
    Progress(u64, Option<u64>),
    Staged(Result<PathBuf, UpdateError>),
}
#[derive(Default)]
pub(super) struct UpdateState {
    config: Option<Config>,
    pub sheet: bool,
    phase: Phase,
    manual: bool,
    next_check: Option<Instant>,
    events: Option<Receiver<Event>>,
    available: Option<Available>,
}
impl UpdateState {
    /// Read the bundle's configuration; with none, updates stay off.
    pub(super) fn init(&mut self) {
        self.config = Config::from_bundle();
        self.next_check = self.config.as_ref().map(|_| Instant::now() + FIRST_CHECK);
    }
    pub(super) fn enabled(&self) -> bool {
        self.config.is_some()
    }
}
/// The macOS version, for the manifest's `minimum_os`. Unknown is treated as new enough.
fn os_version() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "99".into())
}

impl AppView {
    /// Whether updates are configured (asked of the window that owns the account).
    pub(super) fn updates_enabled(&self, cx: &App) -> bool {
        match self.primary(cx) {
            Some((primary, _)) => primary.read(cx).account.update.enabled(),
            None => self.account.update.enabled(),
        }
    }
    pub(super) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.forward_to_primary(cx, |p, cx| p.check_for_updates(cx)) {
            return;
        }
        if self.account.update.enabled() {
            let busy = matches!(
                self.account.update.phase,
                Phase::Checking | Phase::Downloading { .. }
            );
            if !busy
                && !matches!(
                    self.account.update.phase,
                    Phase::Available(_) | Phase::Ready(_)
                )
            {
                self.start_check(true);
            }
            self.account.update.sheet = true;
        }
        cx.notify();
    }
    fn start_check(&mut self, manual: bool) {
        let Some(config) = self.account.update.config.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.account.update.events = Some(rx);
        self.account.update.phase = Phase::Checking;
        self.account.update.manual = manual;
        self.account.update.next_check = Some(Instant::now() + INTERVAL);
        std::thread::spawn(move || {
            let result = strider_update::check(
                &HttpTransport::default(),
                &config,
                env!("CARGO_PKG_VERSION"),
                &os_version(),
            );
            let _ = tx.send(Event::Checked(result));
        });
    }
    fn start_install(&mut self, cx: &mut Context<Self>) {
        let Some(available) = self.account.update.available.clone() else {
            return;
        };
        let Some(running) = strider_update::running_bundle() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.account.update.events = Some(rx);
        self.account.update.phase = Phase::Downloading {
            done: 0,
            total: Some(available.manifest.size),
        };
        std::thread::spawn(move || {
            let temp = std::env::temp_dir().join(format!("strider-update-{}", std::process::id()));
            let mut last = 0u64;
            let progress_tx: Sender<Event> = tx.clone();
            let result = strider_update::stage(
                &HttpTransport::default(),
                &available,
                &temp,
                &running,
                &mut |done, total| {
                    if done.saturating_sub(last) >= PROGRESS_STEP || Some(done) == total {
                        last = done;
                        let _ = progress_tx.send(Event::Progress(done, total));
                    }
                },
            );
            let _ = tx.send(Event::Staged(result));
        });
        cx.notify();
    }
    /// Hand the swap to a helper process and quit; it relaunches the new bundle.
    fn relaunch_into(&mut self, new_app: PathBuf, cx: &mut Context<Self>) {
        let Some(running) = strider_update::running_bundle() else {
            return;
        };
        match strider_update::install_on_quit(&new_app, &running) {
            Ok(()) => cx.quit(),
            Err(e) => self.account.update.phase = Phase::Failed(e.to_string()),
        }
        cx.notify();
    }
    /// Apply finished work and start the scheduled check. Called from the 200 ms poll loop, and
    /// only acts in the window that owns the account.
    pub(super) fn poll_updates(&mut self, cx: &mut Context<Self>) {
        if !self.account.update.enabled() || self.primary(cx).is_some() {
            return;
        }
        let events: Vec<Event> = self
            .account
            .update
            .events
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        let mut notify = !events.is_empty();
        for event in events {
            match event {
                Event::Checked(Ok(Some(found))) => {
                    self.account.update.available = Some(found.clone());
                    if self.account.update.manual {
                        self.account.update.sheet = true;
                    } else {
                        self.toast(format!(
                            "Strider {} is available. Choose Strider \u{25b8} Check for Updates\u{2026} to install it.",
                            found.manifest.version
                        ));
                    }
                    self.account.update.phase = Phase::Available(found);
                }
                Event::Checked(Ok(None)) => {
                    self.account.update.phase = if self.account.update.manual {
                        Phase::UpToDate
                    } else {
                        Phase::Idle
                    };
                }
                Event::Checked(Err(e)) => {
                    // A background check that cannot reach the host stays quiet.
                    self.account.update.phase = if self.account.update.manual {
                        Phase::Failed(e.to_string())
                    } else {
                        Phase::Idle
                    };
                }
                Event::Progress(done, total) => {
                    self.account.update.phase = Phase::Downloading { done, total };
                }
                Event::Staged(Ok(app)) => {
                    if self.account.update.sheet {
                        self.relaunch_into(app, cx);
                    } else {
                        self.account.update.phase = Phase::Ready(app);
                        self.toast(
                            "Update downloaded. Choose Check for Updates\u{2026} to relaunch.",
                        );
                    }
                }
                Event::Staged(Err(e)) => self.account.update.phase = Phase::Failed(e.to_string()),
            }
        }
        let due = self
            .account
            .update
            .next_check
            .is_some_and(|at| Instant::now() >= at);
        if due && matches!(self.account.update.phase, Phase::Idle | Phase::UpToDate) {
            self.start_check(false);
        }
        if due {
            notify = true;
        }
        if notify {
            cx.notify();
        }
    }
    pub(super) fn render_update_sheet(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let close = |label: &'static str| {
            strider_ui::button("update-later", label, ButtonVariant::Ghost, t)
                .on_click(cx.listener(|this, _, w, cx| this.close_overlays(w, cx)))
        };
        let mut content = div().flex().flex_col().gap_3();
        match &self.account.update.phase {
            Phase::Idle | Phase::Checking => {
                content = content.child("Checking for updates\u{2026}");
            }
            Phase::UpToDate => {
                content = content
                    .child(format!(
                        "Strider {} is up to date.",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .child(div().flex().child(close("Done")));
            }
            Phase::Available(a) => {
                content = content
                    .child(format!("Strider {} is available.", a.manifest.version))
                    .when(!a.manifest.notes.is_empty(), |d| {
                        d.child(div().meta(t).child(a.manifest.notes.clone()))
                    })
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                strider_ui::button(
                                    "update-install",
                                    "Install and Relaunch",
                                    ButtonVariant::Primary,
                                    t,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.start_install(cx))),
                            )
                            .child(close("Later")),
                    );
            }
            Phase::Downloading { done, total } => {
                let text = match total {
                    Some(total) if *total > 0 => {
                        format!("Downloading\u{2026} {}%", (done * 100 / total).min(100))
                    }
                    _ => "Downloading\u{2026}".into(),
                };
                content = content.child(text).child(close("Hide"));
            }
            Phase::Ready(app) => {
                let app = app.clone();
                content = content
                    .child("The update is downloaded and verified.")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                strider_ui::button(
                                    "update-relaunch",
                                    "Install and Relaunch",
                                    ButtonVariant::Primary,
                                    t,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.relaunch_into(app.clone(), cx),
                                )),
                            )
                            .child(close("Later")),
                    );
            }
            Phase::Failed(message) => {
                content = content
                    .child(message.clone())
                    .child(div().flex().child(close("Close")));
            }
        }
        self.modal("Software Update", content.into_any_element(), cx)
    }
}
