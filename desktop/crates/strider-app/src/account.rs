use super::*;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};
use strider_sync::{
    Client, SyncError, auth,
    engine::{self, SyncEvent, SyncHandle},
    keychain::{Keychain, TokenStore},
};
#[derive(Default)]
pub(super) struct AccountState {
    pub path: PathBuf,
    pub server: String,
    pub client: Option<Client>,
    email: Option<String>,
    pub handle: Option<SyncHandle>,
    events: Option<Receiver<SyncEvent>>,
    auth: Option<Receiver<AuthResult>>,
    busy: bool,
    pub offline: bool,
    message: Option<String>,
    pub sheet: bool,
}
enum AuthResult {
    SignedIn(strider_sync::Result<auth::Session>),
    SignedOut(strider_sync::Result<()>),
}
impl AccountState {
    pub(crate) fn syncing(&self) -> bool {
        self.handle.is_some()
    }
}
impl AppView {
    pub(super) fn configure_account(
        &mut self,
        path: PathBuf,
        server: String,
        cx: &mut Context<Self>,
    ) {
        self.offline.backend.db_path = path.clone();
        self.account.path = path;
        self.account.server = server;
        match Keychain.load(&self.account.server) {
            Ok(Some(token)) => match Client::new(&self.account.server, Some(token)) {
                Ok(client) => {
                    self.account.email = self.store.state("account_email").ok().flatten();
                    self.start_sync(client);
                }
                Err(e) => self.account.message = Some(e.to_string()),
            },
            Ok(None) => (),
            Err(e) => self.account.message = Some(e.to_string()),
        }
        // GPUI's timer only polls thread channels; all HTTP, Keychain writes and
        // callback listening run on OS threads and never block the render thread.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.poll_sync(cx);
                        this.poll_jobs(cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
    fn start_sync(&mut self, client: Client) {
        let (tx, rx) = mpsc::channel();
        self.account.handle = Some(engine::spawn(&self.account.path, client.clone(), tx));
        self.account.events = Some(rx);
        self.account.client = Some(client);
    }
    pub(super) fn nudge_sync(&self) {
        if let Some(h) = &self.account.handle {
            h.nudge();
        }
    }
    fn poll_sync(&mut self, cx: &mut Context<Self>) {
        let mut notify = false;
        let result = self.account.auth.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = result {
            self.account.busy = false;
            self.account.auth = None;
            notify = true;
            match result {
                AuthResult::SignedIn(Ok(session)) => {
                    self.account.email = Some(session.user.email.clone());
                    match Client::new(&self.account.server, Some(session.token)) {
                        Ok(c) => self.start_sync(c),
                        Err(e) => self.account.message = Some(e.to_string()),
                    }
                }
                AuthResult::SignedOut(Ok(())) => {
                    self.account.client = None;
                    self.account.email = None;
                    self.account.offline = false;
                }
                AuthResult::SignedOut(Err(e)) => {
                    self.account.message = Some(e.to_string());
                    if let Some(c) = self.account.client.clone() {
                        self.start_sync(c);
                    }
                }
                AuthResult::SignedIn(Err(e)) => self.account.message = Some(e.to_string()),
            }
        }
        let events = self
            .account
            .events
            .as_ref()
            .map(|r| r.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in events {
            notify = true;
            match event {
                SyncEvent::Started => (),
                SyncEvent::Finished(report) => {
                    self.account.offline = false;
                    self.account.email = self.store.state("account_email").ok().flatten();
                    self.account.message = None;
                    if !report.tables.is_empty() {
                        self.reload_changes(
                            ChangeSet {
                                tables: report.tables,
                            },
                            cx,
                        );
                    }
                }
                SyncEvent::Offline => self.account.offline = true,
                SyncEvent::NeedsSignIn => {
                    self.account.handle = None;
                    self.account.client = None;
                    self.account.email = None;
                    self.account.message = Some("Sign in to resume sync.".into());
                }
                SyncEvent::UpgradeRequired => {
                    self.account.handle = None;
                    self.account.message = Some("Update Strider to resume sync.".into());
                }
                SyncEvent::Error(e) => self.account.message = Some(e),
            }
        }
        if notify {
            cx.notify();
        }
    }
    fn sign_in(&mut self, cx: &mut Context<Self>) {
        if self.account.busy {
            return;
        }
        self.account.busy = true;
        self.account.message = None;
        let server = self.account.server.clone();
        let expected = self.store.state("account_email").ok().flatten();
        let (tx, rx) = mpsc::channel();
        self.account.auth = Some(rx);
        std::thread::spawn(move || {
            let result = auth::sign_in(&server, "Strider Desktop", |url| {
                let _ = std::process::Command::new("open").arg(url.as_str()).spawn();
            })
            .and_then(|session| {
                if expected
                    .as_ref()
                    .is_some_and(|email| *email != session.user.email)
                {
                    if let Ok(c) = Client::new(&server, Some(session.token.clone())) {
                        let _ = auth::sign_out(&c);
                    }
                    return Err(SyncError::Protocol(
                        "This workspace belongs to another account. Open a different --profile."
                            .into(),
                    ));
                }
                Keychain.store(&server, &session.token)?;
                Ok(session)
            });
            let _ = tx.send(AuthResult::SignedIn(result));
        });
        cx.notify();
    }
    fn sign_out(&mut self, cx: &mut Context<Self>) {
        if self.account.busy {
            return;
        }
        let Some(client) = self.account.client.clone() else {
            return;
        };
        self.account.busy = true;
        let handle = self.account.handle.take();
        self.account.events = None;
        let server = self.account.server.clone();
        let (tx, rx) = mpsc::channel();
        self.account.auth = Some(rx);
        std::thread::spawn(move || {
            if let Some(h) = handle {
                h.join();
            }
            let result = match auth::sign_out(&client) {
                Ok(()) | Err(SyncError::Unauthorized) => Keychain.delete(&server),
                Err(e) => Err(e),
            };
            let _ = tx.send(AuthResult::SignedOut(result));
        });
        cx.notify();
    }
    pub(super) fn sync_status(&self) -> Option<String> {
        let pending = self.store.pending_count().unwrap_or_default();
        if self.account.offline {
            Some(format!("Offline · {pending} changes waiting"))
        } else if pending > 0 {
            Some(format!("{pending} changes waiting"))
        } else {
            None
        }
    }
    pub(super) fn account_card(&self, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let mut card = strider_ui::card(t)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().card_heading().child("Account"));
        if self.account.client.is_some() {
            card = card
                .child(
                    self.account
                        .email
                        .clone()
                        .unwrap_or_else(|| "Checking account…".into()),
                )
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            strider_ui::button("sync-now", "Sync now", ButtonVariant::Outline, t)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(h) = &this.account.handle {
                                        h.now();
                                    }
                                    this.account.sheet = true;
                                    cx.notify();
                                })),
                        )
                        .child(
                            strider_ui::button("sign-out", "Sign out", ButtonVariant::Ghost, t)
                                .on_click(cx.listener(|this, _, _, cx| this.sign_out(cx))),
                        ),
                );
        } else {
            card = card
                .child(
                    div()
                        .meta(t)
                        .child("Sign in to sync your Strider workspace."),
                )
                .child(
                    strider_ui::button(
                        "sign-in",
                        if self.account.busy {
                            "Signing in…"
                        } else {
                            "Sign in"
                        },
                        ButtonVariant::Primary,
                        t,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                );
        }
        if let Some(message) = &self.account.message {
            card = card.child(div().meta(t).child(message.clone()));
        }
        card
    }
    pub(super) fn dismiss_conflict(&mut self, id: i64, cx: &mut Context<Self>) {
        if let Err(e) = self.store.dismiss_conflict(id) {
            self.error = Some(e.to_string());
        }
        cx.notify();
    }
    pub(super) fn render_sync_sheet(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let pending = self.store.pending_count().unwrap_or_default();
        let last = self
            .store
            .state("last_sync")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok());
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(if pending == 1 {
                "1 change waiting".to_owned()
            } else {
                format!("{pending} changes waiting")
            })
            .child(
                div().meta(t).child(
                    last.map(|at| format!("Last synced {}", offline::format::age(self.now(), at)))
                        .unwrap_or_else(|| "Not synced yet".into()),
                ),
            );
        if self.offline() {
            content = content.child(
                div()
                    .meta(t)
                    .child("Offline. Changes sync when you reconnect."),
            );
        }
        if let Some(message) = &self.account.message {
            content = content.child(div().meta(t).child(message.clone()));
        }
        match self.store.conflicts() {
            Ok(conflicts) => {
                for c in conflicts {
                    let copy = c.text.clone();
                    let id = c.id;
                    content = content.child(
                        div()
                            .id(("conflict", c.id as usize))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().meta(t).child(format!("Replaced text · {}", c.field)))
                            .child(c.text)
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        strider_ui::button(
                                            ("copy-conflict", id as usize),
                                            "Copy",
                                            ButtonVariant::Ghost,
                                            t,
                                        )
                                        .on_click(
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    copy.clone(),
                                                ))
                                            },
                                        ),
                                    )
                                    .child(
                                        strider_ui::button(
                                            ("dismiss-conflict", id as usize),
                                            "Dismiss",
                                            ButtonVariant::Ghost,
                                            t,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.dismiss_conflict(id, cx)
                                            }),
                                        ),
                                    ),
                            ),
                    );
                }
            }
            Err(e) => content = content.child(e.to_string()),
        }
        match self.store.rejections() {
            Ok(rejected) => {
                for r in rejected {
                    content = content.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().meta(t).child("Rejected change"))
                            .child(r),
                    );
                }
            }
            Err(e) => content = content.child(e.to_string()),
        }
        content = content.child(
            strider_ui::button("sync-now-sheet", "Sync now", ButtonVariant::Outline, t)
                .when(self.account.handle.is_none(), |b| b.opacity(0.5))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(h) = &this.account.handle {
                        h.now();
                    }
                    cx.notify();
                })),
        );
        self.modal("Sync", content.into_any_element(), cx)
    }
}
#[cfg(test)]
mod tests {
    use crate::app::test_support::{headless, with_view};
    use gpui::TestAppContext;

    #[gpui::test]
    fn a_conflict_appears_in_the_sync_sheet_and_can_be_dismissed(cx: &mut TestAppContext) {
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        let id = with_view(&view, &mut cx, |g, _, cx| {
            g.store
                .connection()
                .execute_batch(
                    "INSERT INTO conflict(tbl,sync_id,col,local_value,server_value) VALUES('trip','t','planning_notes','mine','their longer text');
                     INSERT INTO sync_log(kind,detail) VALUES('rejected','batch 7: stale');",
                )
                .expect("conflict");
            g.account.sheet = true;
            cx.notify();
            let conflicts = g.store.conflicts().expect("conflicts");
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].text, "their longer text");
            assert_eq!(g.store.rejections().expect("rejections").len(), 1);
            conflicts[0].id
        });
        cx.run_until_parked(); // renders the sheet with Copy, Dismiss and Sync now
        with_view(&view, &mut cx, |g, w, cx| {
            assert!(g.render_overlay(w, cx).is_some());
            g.dismiss_conflict(id, cx);
            assert!(g.store.conflicts().expect("conflicts").is_empty());
            assert_eq!(
                g.store.rejections().expect("rejections").len(),
                1,
                "rejections stay listed"
            );
        });
        cx.run_until_parked();
    }
}
