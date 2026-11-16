//! The Trip share menu (port of `share-menu.tsx`): view-only and edit links, owner only, online only.
use super::*;
use offline::Job;
use strider_services::share::{LinkAction, LinkKind, ShareLink, change_share_link};

#[derive(Default)]
pub(crate) struct ShareState {
    pub open: bool,
    pub busy: bool,
    pub view: Option<ShareLink>,
    pub edit: Option<ShareLink>,
    /// A revoke waits for a second press, as on the web.
    pub confirm: Option<LinkKind>,
    pub message: Option<String>,
}
const NEEDS_CONNECTION: &str = "Sharing needs a connection.";
fn noun(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::View => "View-only link",
        LinkKind::Edit => "Edit link",
    }
}
impl AppView {
    pub(super) fn is_owner(&self) -> bool {
        self.trips
            .detail
            .as_ref()
            .is_some_and(|d| d.access == "owner")
    }
    /// Sharing needs the server: signed in, and the last sync reached it.
    pub(super) fn can_share(&self) -> bool {
        self.is_owner() && self.account.client.is_some() && !self.offline()
    }
    pub(super) fn open_share(&mut self, cx: &mut Context<Self>) {
        if !self.is_owner() {
            return;
        }
        self.share.message = None;
        self.share.confirm = None;
        self.share.open = true;
        cx.notify();
    }
    fn change_link(&mut self, kind: LinkKind, action: LinkAction, cx: &mut Context<Self>) {
        if !self.can_share() || self.share.busy {
            return;
        }
        let (Some(client), Some(trip)) = (self.account.client.clone(), self.trips.selected.clone())
        else {
            return;
        };
        self.share.busy = true;
        let tx = self.offline.jobs_tx.clone();
        std::thread::spawn(move || {
            let result = change_share_link(&client, &trip, kind, action).map_err(|e| e.to_string());
            let _ = tx.send(Job::Share {
                kind,
                action,
                result,
            });
        });
        cx.notify();
    }
    pub(super) fn copy_link(&mut self, kind: LinkKind, cx: &mut Context<Self>) {
        self.share.confirm = None;
        self.change_link(kind, LinkAction::Create, cx);
    }
    pub(super) fn revoke_link(&mut self, kind: LinkKind, cx: &mut Context<Self>) {
        if self.share.confirm != Some(kind) {
            self.share.confirm = Some(kind);
            self.share.message = Some("Select revoke again to confirm.".into());
            cx.notify();
            return;
        }
        self.share.confirm = None;
        self.change_link(kind, LinkAction::Revoke, cx);
    }
    pub(super) fn finish_share(
        &mut self,
        kind: LinkKind,
        action: LinkAction,
        result: Result<Option<ShareLink>, String>,
        cx: &mut Context<Self>,
    ) {
        self.share.busy = false;
        match result {
            Ok(link) => {
                self.share.message = Some(match (action, &link) {
                    (LinkAction::Revoke, _) => match kind {
                        LinkKind::View => "View-only link revoked.".into(),
                        LinkKind::Edit => "Edit access revoked.".into(),
                    },
                    (_, Some(l)) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(l.url.clone()));
                        match kind {
                            LinkKind::View => "View-only link copied.".into(),
                            LinkKind::Edit => {
                                "Edit link copied. Anyone signed in with this link can edit the trip."
                                    .into()
                            }
                        }
                    }
                    _ => "The link could not be read.".into(),
                });
                match kind {
                    LinkKind::View => self.share.view = link,
                    LinkKind::Edit => self.share.edit = link,
                }
            }
            Err(e) => self.share.message = Some(format!("Could not change the link. {e}")),
        }
        cx.notify();
    }
    fn link_block(&self, kind: LinkKind, hint: &'static str, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let enabled = self.can_share() && !self.share.busy;
        let id = match kind {
            LinkKind::View => "view",
            LinkKind::Edit => "edit",
        };
        let copy = strider_ui::button(
            format!("copy-{id}-link"),
            "Copy link",
            ButtonVariant::Outline,
            t,
        )
        .when(!enabled, |b| b.opacity(0.5))
        .when(enabled, |b| {
            b.on_click(cx.listener(move |this, _, _, cx| this.copy_link(kind, cx)))
        });
        let revoke = strider_ui::button(
            format!("revoke-{id}-link"),
            if self.share.confirm == Some(kind) {
                "Confirm revoke"
            } else {
                "Revoke"
            },
            ButtonVariant::Danger,
            t,
        )
        .when(!enabled, |b| b.opacity(0.5))
        .when(enabled, |b| {
            b.on_click(cx.listener(move |this, _, _, cx| this.revoke_link(kind, cx)))
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(px(12.))
            .bg(t.panel2)
            .child(div().font_weight(FontWeight::SEMIBOLD).child(noun(kind)))
            .child(div().meta(t).child(hint))
            .child(div().flex().gap_2().child(copy).child(revoke))
    }
    pub(super) fn render_share_sheet(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let mut body = div().flex().flex_col().gap_3();
        if !self.can_share() {
            body = body.child(div().meta(t).child(NEEDS_CONNECTION));
        }
        body = body
            .child(self.link_block(LinkKind::View, "Live trip details, no private notes", cx))
            .child(self.link_block(
                LinkKind::Edit,
                "Sign-in required · private notes excluded",
                cx,
            ));
        if let Some(m) = &self.share.message {
            body = body.child(div().meta(t).child(m.clone()));
        }
        self.modal("Share trip", body.into_any_element(), cx)
    }
}
