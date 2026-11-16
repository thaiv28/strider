use super::*;
use gpui_component::{
    button::Button,
    input::{Input, InputState, Textarea, TextareaState},
    menu::{DropdownMenu, PopupMenuItem},
};

/// Topo Select: keyboard navigable dropdown backed by the native GPUI menu.
pub fn select(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    options: Vec<(String, String)>,
    theme: Theme,
    on_change: impl Fn(String, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_change = std::rc::Rc::new(on_change);
    Button::new(id)
        .label(label)
        .min_h(px(44.))
        .rounded(px(10.))
        .bg(theme.panel)
        .text_color(theme.ink)
        .border_1()
        .border_color(theme.field_border)
        .dropdown_menu(move |mut menu, _, _| {
            for (value, label) in &options {
                let value = value.clone();
                let handler = on_change.clone();
                menu = menu.item(
                    PopupMenuItem::new(label.clone())
                        .on_click(move |_, window, cx| handler(value.clone(), window, cx)),
                );
            }
            menu.scrollable(true)
        })
}
pub fn text_input(
    state: &Entity<InputState>,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Input {
    Input::new(state)
        .aria_label(label)
        .bg(theme.panel)
        .rounded(px(10.))
        .h(px(44.))
}
/// Topo multiline field.
pub fn textarea(
    state: &Entity<TextareaState>,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Textarea {
    Textarea::new(state)
        .aria_label(label)
        .bg(theme.panel)
        .rounded(px(10.))
        .h(px(88.))
}
pub fn checkbox(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::CheckBox)
        .focusable()
        .focus_visible(move |s| s.border_2().border_color(theme.accent))
        .aria_label(label.into())
        .min_h(px(44.))
        .min_w(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(
            div()
                .size(px(20.))
                .rounded(px(4.))
                .border_1()
                .border_color(if checked {
                    theme.accent
                } else {
                    theme.field_border
                })
                .bg(if checked { theme.accent } else { theme.panel })
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            if checked {
                                let mut p = PathBuilder::stroke(px(2.));
                                p.move_to(bounds.origin + point(px(4.), px(10.)));
                                p.line_to(bounds.origin + point(px(8.), px(14.)));
                                p.line_to(bounds.origin + point(px(16.), px(5.)));
                                if let Ok(p) = p.build() {
                                    window.paint_path(p, theme.accent_ink);
                                }
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
pub fn badge(label: impl Into<SharedString>, tone: &str, theme: Theme) -> Div {
    let color = match tone {
        "base" | "current" => theme.cbase,
        "worn" => theme.cworn,
        "consumable" => theme.ccons,
        "wishlist" => theme.accent,
        _ => theme.muted,
    };
    let neutral = matches!(tone, "neutral" | "retired");
    div()
        .rounded_full()
        .border_1()
        .border_color(if neutral {
            theme.line
        } else {
            color.opacity(if tone == "worn" { 0.4 } else { 0.3 })
        })
        .bg(if neutral {
            theme.panel2
        } else {
            color.opacity(if tone == "worn" { 0.15 } else { 0.1 })
        })
        .text_color(color)
        .px_2()
        .py_0p5()
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.into())
}
pub fn segmented_control(children: impl IntoIterator<Item = AnyElement>, theme: Theme) -> Div {
    div()
        .flex()
        .gap_0p5()
        .p(px(3.))
        .rounded(px(12.))
        .bg(theme.canvas)
        .children(children)
}
pub fn segment(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    button(id, label, ButtonVariant::Ghost, theme)
        .flex_1()
        .rounded(px(9.))
        .when(selected, |d| d.bg(theme.panel).text_color(theme.ink))
}
pub fn worn_toggle(children: impl IntoIterator<Item = AnyElement>, theme: Theme) -> Div {
    div()
        .flex()
        .rounded_full()
        .border_1()
        .border_color(theme.line)
        .overflow_hidden()
        .children(children)
}
pub fn info_badge(
    id: impl Into<ElementId>,
    detail: impl Into<SharedString>,
    theme: Theme,
) -> impl IntoElement {
    focus_info(id, "i", detail, None, theme)
}
/// A hover/focus popover used by InfoBadge and BaseBar segments.
pub fn focus_info(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    color: Option<Hsla>,
    theme: Theme,
) -> impl IntoElement {
    InfoPopup {
        id: id.into(),
        label: label.into(),
        detail: detail.into(),
        color,
        theme,
    }
}
#[derive(IntoElement)]
struct InfoPopup {
    id: ElementId,
    label: SharedString,
    detail: SharedString,
    color: Option<Hsla>,
    theme: Theme,
}
struct InfoState {
    focus: FocusHandle,
    label: SharedString,
    detail: SharedString,
    color: Option<Hsla>,
    theme: Theme,
    _subscriptions: Vec<Subscription>,
}
impl RenderOnce for InfoPopup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id, cx, |window, cx| {
            let focus = cx.focus_handle();
            let enter = cx.on_focus(&focus, window, |_, _, cx| cx.notify());
            let leave = cx.on_blur(&focus, window, |_, _, cx| cx.notify());
            InfoState {
                focus,
                label: self.label.clone(),
                detail: self.detail.clone(),
                color: self.color,
                theme: self.theme,
                _subscriptions: vec![enter, leave],
            }
        });
        state.update(cx, |s, _| {
            s.label = self.label;
            s.detail = self.detail;
            s.color = self.color;
            s.theme = self.theme;
        });
        state
    }
}
impl Render for InfoState {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        let detail = self.detail.clone();
        let focused = self.focus.is_focused(window);
        div()
            .id("info-trigger")
            .track_focus(&self.focus)
            .role(Role::Button)
            .aria_label(self.detail.clone())
            .relative()
            .h(px(if self.color.is_some() { 32. } else { 16. }))
            .min_w(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when_some(self.color, |d, c| {
                d.w_full().child(div().w_full().h(px(12.)).bg(c))
            })
            .when(self.color.is_none(), |d| {
                d.w(px(16.)).child(
                    div()
                        .rounded_full()
                        .size(px(16.))
                        .border_1()
                        .border_color(t.muted.opacity(0.5))
                        .text_color(t.muted)
                        .text_size(px(10.))
                        .text_center()
                        .child(self.label.clone()),
                )
            })
            .focus_visible(move |s| s.border_2().border_color(t.accent))
            .hoverable_tooltip(move |_, cx| {
                cx.new(|_| InfoTip {
                    text: detail.clone(),
                })
                .into()
            })
            .when(focused, |d| {
                d.child(deferred(
                    anchored()
                        .offset(point(px(0.), px(44.)))
                        .child(tip(self.detail.clone(), t)),
                ))
            })
    }
}
fn tip(text: SharedString, t: Theme) -> Div {
    div()
        .bg(t.panel)
        .border_1()
        .border_color(t.line)
        .rounded(px(8.))
        .p_3()
        .w(px(256.))
        .meta(t)
        .child(text)
}
struct InfoTip {
    text: SharedString,
}
impl Render for InfoTip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        tip(self.text.clone(), Theme::get(cx))
    }
}
