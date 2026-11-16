use gpui::{prelude::*, *};
use gpui_component::{
    Root,
    input::{Input, InputEvent, InputState},
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use strider_core::{fmt_oz, fmt_weight, g_to_oz, oz_to_g};
use strider_store::{
    Command, ComponentInput, GearFilter, GearInput, GearRow, Group, StatusFilter, Store, Totals,
    seed,
};
use strider_ui::{Appearance, ButtonVariant, Icon, Theme, TopoStyle};
actions!(
    strider,
    [
        SelectNext,
        SelectPrevious,
        EditSelected,
        DeleteSelected,
        Undo,
        Redo,
        Find,
        Escape,
        Quit,
        Light,
        Dark,
        System
    ]
);
#[derive(Clone)]
enum Entry {
    Category(Group),
    Item {
        row: Box<GearRow>,
        category: Option<String>,
        last: bool,
    },
    Component {
        name: String,
        weight: strider_core::Grams,
        quantity: i32,
        last: bool,
    },
    Gap,
}
#[derive(Clone)]
struct DragRow {
    id: String,
    row: GearRow,
}
impl Render for DragRow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx);
        strider_ui::card(t)
            .body()
            .min_w(px(560.))
            .min_h(px(44.))
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .text_color(t.ink)
            .child(strider_ui::icon(Icon::Grip, t.muted))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                        "{}{}",
                        self.row.name,
                        if self.row.quantity > 1 {
                            format!(" ×{}", self.row.quantity)
                        } else {
                            String::new()
                        }
                    )))
                    .when(self.row.is_kit, |d| {
                        d.child(
                            div()
                                .meta(t)
                                .child(format!("Kit · {} items", self.row.components.len())),
                        )
                    })
                    .when(self.row.status == "retired", |d| {
                        d.child(div().meta(t).child("Retired"))
                    }),
            )
            .when(self.row.class == "worn", |d| {
                d.child(
                    div()
                        .meta(t)
                        .font_weight(FontWeight::BOLD)
                        .text_color(t.signal)
                        .child("Worn"),
                )
            })
            .child(
                div()
                    .readout()
                    .child(fmt_oz(self.row.weight.map(|w| w * self.row.quantity))),
            )
    }
}
struct Editor {
    id: String,
    title: String,
    name: Entity<InputState>,
    weight: Entity<InputState>,
    quantity: Entity<InputState>,
    weight_error: bool,
    quantity_error: bool,
}
struct FrameLog {
    enabled: bool,
    previous: Option<Instant>,
    since: Instant,
    samples: Vec<f64>,
}
impl FrameLog {
    fn new() -> Self {
        Self {
            enabled: std::env::var("STRIDER_FRAME_LOG").is_ok_and(|v| v == "1"),
            previous: None,
            since: Instant::now(),
            samples: Vec::new(),
        }
    }
    fn record(&mut self) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        if let Some(previous) = self.previous.replace(now) {
            self.samples
                .push(now.duration_since(previous).as_secs_f64() * 1000.);
        }
        if now.duration_since(self.since) >= Duration::from_secs(1) && !self.samples.is_empty() {
            self.samples.sort_by(f64::total_cmp);
            let p95 = self.samples
                [((self.samples.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
            eprintln!(
                "Strider root render intervals: p95={p95:.2}ms max={:.2}ms n={}",
                self.samples[self.samples.len() - 1],
                self.samples.len()
            );
            self.samples.clear();
            self.since = now;
        }
    }
}
struct GearView {
    store: Store,
    groups: Vec<Group>,
    totals: Totals,
    filter: GearFilter,
    entries: Vec<Entry>,
    list: ListState,
    selected: Option<String>,
    expanded: HashSet<String>,
    filter_input: Entity<InputState>,
    focus: FocusHandle,
    editor: Option<Editor>,
    area: &'static str,
    wishlist: bool,
    error: Option<String>,
    appearance: Appearance,
    _subscriptions: Vec<Subscription>,
    frames: FrameLog,
}
impl GearView {
    fn new(store: Store, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter gear"));
        let subscription = cx.subscribe_in(&filter_input, window, |this, input, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.filter.query = input.read(cx).value().to_string();
                this.reload(cx);
            }
        });
        let appearance_sub = cx.observe_window_appearance(window, |this, window, cx| {
            if this.appearance == Appearance::System {
                strider_ui::set_appearance(Appearance::System, window, cx);
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut this = Self {
            store,
            groups: Vec::new(),
            totals: Totals::default(),
            filter: GearFilter::default(),
            entries: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(500.)),
            selected: None,
            expanded: HashSet::new(),
            filter_input,
            focus,
            editor: None,
            area: "Gear",
            wishlist: false,
            error: None,
            appearance: Appearance::System,
            _subscriptions: vec![subscription, appearance_sub],
            frames: FrameLog::new(),
        };
        this.reload(cx);
        this
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        match (self.store.gear_groups(&self.filter), self.store.totals()) {
            (Ok(groups), Ok(totals)) => {
                self.groups = groups;
                self.totals = totals;
                self.error = None;
                self.flatten();
            }
            (Err(e), _) | (_, Err(e)) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn flatten(&mut self) {
        let scroll = self.list.logical_scroll_top();
        self.entries.clear();
        for group in &self.groups {
            self.entries.push(Entry::Category(Group {
                rows: Vec::new(),
                ..group.clone()
            }));
            for (i, row) in group.rows.iter().enumerate() {
                let expanded = self.expanded.contains(&row.id);
                let last = i + 1 == group.rows.len();
                self.entries.push(Entry::Item {
                    row: Box::new(row.clone()),
                    category: group.id.clone(),
                    last: last && (!expanded || row.components.is_empty()),
                });
                if expanded {
                    for (j, component) in row.components.iter().enumerate() {
                        self.entries.push(Entry::Component {
                            name: component.name.clone(),
                            weight: component.weight,
                            quantity: component.quantity,
                            last: last && j + 1 == row.components.len(),
                        });
                    }
                }
            }
            self.entries.push(Entry::Gap);
        }
        self.list.reset(self.entries.len());
        self.list.scroll_to(scroll);
        if self.selected.as_ref().is_some_and(|id| {
            !self
                .groups
                .iter()
                .any(|g| g.rows.iter().any(|r| &r.id == id))
        }) {
            self.selected = None;
        }
    }
    fn command(&mut self, command: Command, cx: &mut Context<Self>) {
        match self.store.apply(command) {
            Ok(_) => self.reload(cx),
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn select(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.area != "Gear" || self.wishlist {
            return;
        }
        let rows: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if let Entry::Item { row, .. } = e {
                    Some((i, row.id.clone()))
                } else {
                    None
                }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| rows.iter().position(|(_, r)| r == id));
        let index = match current {
            Some(i) => (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize,
            None => {
                if delta > 0 {
                    0
                } else {
                    rows.len() - 1
                }
            }
        };
        self.selected = Some(rows[index].1.clone());
        self.list.scroll_to_reveal_item(rows[index].0);
        cx.notify();
    }
    fn delete(&mut self, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.area != "Gear" || self.wishlist {
            return;
        }
        if let Some(id) = self.selected.clone() {
            self.command(Command::DeleteGear { id }, cx);
        }
    }
    fn undo(&mut self, redo: bool, cx: &mut Context<Self>) {
        if self.editor.is_some() {
            return;
        }
        let result = if redo {
            self.store.redo()
        } else {
            self.store.undo()
        };
        match result {
            Ok(_) => self.reload(cx),
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn edit(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.area != "Gear" || self.wishlist {
            return;
        }
        let Some(row) = self
            .groups
            .iter()
            .flat_map(|g| &g.rows)
            .find(|r| r.id == id)
            .cloned()
        else {
            return;
        };
        let name = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |s, cx| s.set_value(row.name.clone(), window, cx));
        let weight = cx.new(|cx| InputState::new(window, cx));
        weight.update(cx, |s, cx| {
            s.set_value(
                row.weight
                    .map(|w| g_to_oz(w).to_string())
                    .unwrap_or_default(),
                window,
                cx,
            )
        });
        let quantity = cx.new(|cx| InputState::new(window, cx));
        quantity.update(cx, |s, cx| {
            s.set_value(row.quantity.to_string(), window, cx)
        });
        name.update(cx, |s, cx| s.focus(window, cx));
        self.editor = Some(Editor {
            id,
            title: row.name,
            name,
            weight,
            quantity,
            weight_error: false,
            quantity_error: false,
        });
        cx.notify();
    }
    fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor = None;
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        let weight_text = editor.weight.read(cx).value().to_string();
        let weight = if weight_text.trim().is_empty() {
            Ok(None)
        } else {
            weight_text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| {
                    n.is_finite() && *n >= 0. && *n * strider_core::OZ_TO_G <= i32::MAX as f64
                })
                .map(|n| Some(oz_to_g(n)))
                .ok_or(())
        };
        let quantity = editor
            .quantity
            .read(cx)
            .value()
            .trim()
            .parse::<i32>()
            .ok()
            .filter(|n| *n > 0);
        editor.weight_error = weight.is_err();
        editor.quantity_error = quantity.is_none();
        if let (Ok(weight), Some(quantity)) = (weight, quantity) {
            let Some(row) = self
                .groups
                .iter()
                .flat_map(|group| &group.rows)
                .find(|row| row.id == editor.id)
            else {
                self.error = Some("Gear item no longer exists".into());
                cx.notify();
                return;
            };
            let command = Command::UpdateGear {
                id: row.id.clone(),
                input: GearInput {
                    name: editor.name.read(cx).value().to_string(),
                    category_id: row.category_id.clone(),
                    weight_g: weight,
                    weight_oz: None,
                    price_cents: row.price_cents,
                    quantity,
                    class: row.class.clone(),
                    status: row.status.clone(),
                    is_kit: row.is_kit,
                    components: row
                        .components
                        .iter()
                        .map(|component| ComponentInput {
                            name: component.name.clone(),
                            weight_g: component.weight.0,
                            quantity: component.quantity,
                            notes: component.notes.clone(),
                        })
                        .collect(),
                    product_url: row.product_url.clone(),
                    notes: row.notes.clone(),
                },
            };
            self.command(command, cx);
            if self.error.is_none() {
                self.close_editor(window, cx);
            }
        } else {
            cx.notify();
        }
    }
    fn appearance(&mut self, appearance: Appearance, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance = appearance;
        strider_ui::set_appearance(appearance, window, cx);
        cx.notify();
    }
    fn drop_row(
        &mut self,
        drag: &DragRow,
        category: Option<String>,
        before: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.filter.query.is_empty() && before.as_ref() != Some(&drag.id) {
            let mut ordered_ids: Vec<_> = self
                .groups
                .iter()
                .filter(|group| group.id == category)
                .flat_map(|group| &group.rows)
                .filter(|row| row.id != drag.id)
                .map(|row| row.id.clone())
                .collect();
            let insertion = before
                .as_ref()
                .and_then(|id| ordered_ids.iter().position(|candidate| candidate == id))
                .unwrap_or(ordered_ids.len());
            ordered_ids.insert(insertion, drag.id.clone());
            self.command(
                Command::MoveGear {
                    item: drag.id.clone(),
                    to_category: category,
                    ordered_ids,
                },
                cx,
            );
        }
    }
    fn render_entry(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        match self.entries[index].clone() {
            Entry::Gap => div().w_full().h(px(12.)).into_any_element(),
            Entry::Category(group) => {
                let category = group.id.clone();
                div()
                    .id(("category", index))
                    .w_full()
                    .relative()
                    .h(px(44.))
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded_t(px(14.))
                    .bg(t.panel)
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_t(px(14.))
                            .bg(t.panel2.opacity(0.6)),
                    )
                    .child(
                        div()
                            .relative()
                            .font_weight(FontWeight::BOLD)
                            .child(group.name),
                    )
                    .child(
                        div()
                            .relative()
                            .readout()
                            .text_size(px(12.))
                            .flex()
                            .child(
                                div()
                                    .text_color(t.cbase)
                                    .child(format!("{} base", fmt_oz(Some(group.base)))),
                            )
                            .when(group.consumable.0 != 0, |d| {
                                d.child(
                                    div().text_color(t.ccons).child(format!(
                                        " · {} cons",
                                        fmt_oz(Some(group.consumable))
                                    )),
                                )
                            })
                            .when(group.worn.0 != 0, |d| {
                                d.child(
                                    div()
                                        .text_color(t.cworn)
                                        .child(format!(" · {} worn", fmt_oz(Some(group.worn)))),
                                )
                            }),
                    )
                    .border_b_1()
                    .border_color(t.line)
                    .drag_over::<DragRow>(move |s, _, _, _| s.border_b_2().border_color(t.accent))
                    .on_drop(cx.listener(move |this, drag: &DragRow, _, cx| {
                        this.drop_row(drag, category.clone(), None, cx)
                    }))
                    .into_any_element()
            }
            Entry::Component {
                name,
                weight,
                quantity,
                last,
            } => div()
                .w_full()
                .relative()
                .h(px(44.))
                .px_4()
                .pl(px(104.))
                .flex()
                .items_center()
                .justify_between()
                .bg(t.panel)
                .when(last, |d| d.rounded_b(px(14.)))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(t.panel2.opacity(0.25))
                        .when(last, |d| d.rounded_b(px(14.))),
                )
                .child(div().relative().meta(t).child(format!(
                    "{name}{}",
                    if quantity > 1 {
                        format!(" ×{quantity}")
                    } else {
                        String::new()
                    }
                )))
                .child(
                    div()
                        .relative()
                        .readout()
                        .text_color(t.muted)
                        .child(fmt_oz(Some(weight * quantity))),
                )
                .into_any_element(),
            Entry::Item {
                row,
                category,
                last,
            } => {
                let row = *row;
                let id = row.id.clone();
                let click_id = id.clone();
                let edit_id = id.clone();
                let expand_id = id.clone();
                let before = id.clone();
                let expanded = self.expanded.contains(&id);
                let selected = self.selected.as_ref() == Some(&id);
                let drag = DragRow {
                    id: id.clone(),
                    row: row.clone(),
                };
                let grip = strider_ui::icon_button(("grip", index), Icon::Grip, t)
                    .cursor_move()
                    .when(self.filter.query.is_empty(), |d| {
                        d.on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
                    });
                let mut item = div()
                    .id(("item", index))
                    .w_full()
                    .min_h(px(44.))
                    .flex()
                    .items_center()
                    .px_4()
                    .bg(if selected { t.panel2 } else { t.panel })
                    .when(last, |d| d.rounded_b(px(14.)))
                    .when(!last, |d| d.border_b_1().border_color(t.line))
                    .hover(move |s| s.bg(t.panel2.opacity(0.4)))
                    .child(grip);
                if row.is_kit {
                    item = item.child(
                        strider_ui::icon_button(("expand", index), Icon::ChevronDown, t)
                            .when(expanded, |d| d.bg(t.panel2))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if !this.expanded.remove(&expand_id) {
                                    this.expanded.insert(expand_id.clone());
                                }
                                this.flatten();
                                cx.notify();
                            })),
                    );
                }
                let mut description = div().flex_1().min_w_0().flex().flex_col().py_1().child(
                    div().font_weight(FontWeight::SEMIBOLD).child(format!(
                        "{}{}",
                        row.name,
                        if row.quantity > 1 {
                            format!(" ×{}", row.quantity)
                        } else {
                            String::new()
                        }
                    )),
                );
                let mut meta = Vec::new();
                if row.is_kit {
                    meta.push(format!("Kit · {} items", row.components.len()));
                }
                if row.status == "retired" {
                    meta.push("Retired".into());
                }
                if !meta.is_empty() {
                    description = description.child(div().meta(t).child(meta.join(" · ")));
                }
                item.child(description)
                    .when(row.class == "worn", |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(t.signal)
                                .mr_4()
                                .child("Worn"),
                        )
                    })
                    .child(
                        div()
                            .readout()
                            .w(px(104.))
                            .text_right()
                            .child(fmt_oz(row.weight.map(|w| w * row.quantity))),
                    )
                    .child(
                        strider_ui::icon_button(("more", index), Icon::MoreHorizontal, t).on_click(
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.selected = Some(edit_id.clone());
                                this.edit(edit_id.clone(), window, cx);
                            }),
                        ),
                    )
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        this.selected = Some(click_id.clone());
                        this.focus.focus(window, cx);
                        if event.click_count() == 2 {
                            this.edit(click_id.clone(), window, cx);
                        }
                        cx.notify();
                    }))
                    .drag_over::<DragRow>(move |s, _, _, _| s.border_t_2().border_color(t.accent))
                    .on_drop(cx.listener(move |this, drag: &DragRow, _, cx| {
                        this.drop_row(drag, category.clone(), Some(before.clone()), cx)
                    }))
                    .into_any_element()
            }
        }
    }
}
impl Render for GearView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames.record();
        let t = Theme::get(cx);
        let shell = div()
            .relative()
            .h(px(64.))
            .flex_shrink_0()
            .bg(t.header)
            .child(strider_ui::contour_band(t))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .items_center()
                    .pl(px(84.))
                    .gap_1p5()
                    .child(
                        div()
                            .flex_shrink_0()
                            .section_heading()
                            .text_color(t.header_ink)
                            .child("Strider"),
                    )
                    .children(
                        ["Gear", "Food", "Trips", "Calendar", "Settings"]
                            .into_iter()
                            .map(|area| {
                                div()
                                    .id(area)
                                    .min_h(px(44.))
                                    .px_2()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .text_size(px(15.))
                                    .rounded(px(12.))
                                    .text_color(if area == self.area {
                                        t.header_ink
                                    } else {
                                        t.header_muted
                                    })
                                    .when(area == self.area, |d| {
                                        d.bg(t.header_active).font_weight(FontWeight::BOLD)
                                    })
                                    .cursor_pointer()
                                    .child(area)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.area = area;
                                        this.focus.focus(window, cx);
                                        cx.notify();
                                    }))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            }),
                    ),
            );
        let summary = div()
            .flex()
            .gap_6()
            .children(
                [
                    ("Base weight", fmt_weight(Some(self.totals.base))),
                    ("Worn", fmt_weight(Some(self.totals.worn))),
                    ("Consumable", fmt_weight(Some(self.totals.consumable))),
                    ("Items", self.totals.items.to_string()),
                ]
                .into_iter()
                .map(|(label, value)| {
                    div()
                        .flex()
                        .flex_col()
                        .child(div().eyebrow(t).text_color(t.header_muted).child(label))
                        .child(div().readout().text_color(t.header_ink).child(value))
                }),
            )
            .into_any_element();
        let tabs = div()
            .flex()
            .gap_2()
            .children([("Inventory", false), ("Wishlist", true)].into_iter().map(
                |(label, wishlist)| {
                    strider_ui::page_tab(label, label, self.wishlist == wishlist, true, t).on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.wishlist = wishlist;
                            cx.notify();
                        }),
                    )
                },
            ))
            .into_any_element();
        let header = strider_ui::page_header(
            self.area,
            div().into_any_element(),
            if self.area == "Gear" {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(summary)
                    .child(tabs)
                    .into_any_element()
            } else {
                div().into_any_element()
            },
            t,
        );
        let body = if self.area != "Gear" || self.wishlist {
            strider_ui::page_body(
                vec![
                    div()
                        .meta(t)
                        .child(if self.area == "Gear" {
                            "Wishlist is not part of the spike."
                        } else {
                            "Not part of the spike."
                        })
                        .into_any_element(),
                ],
                t,
            )
            .into_any_element()
        } else {
            let filters = div()
                .flex()
                .items_center()
                .gap_2()
                .mb_3()
                .child(
                    div().flex_1().child(
                        Input::new(&self.filter_input)
                            .aria_label("Filter gear")
                            .bg(t.panel)
                            .rounded(px(10.))
                            .h(px(44.))
                            .prefix(strider_ui::icon(Icon::Search, t.muted)),
                    ),
                )
                .children(
                    [
                        ("Current", StatusFilter::Current),
                        ("Retired", StatusFilter::Retired),
                        ("All", StatusFilter::All),
                    ]
                    .into_iter()
                    .map(|(label, status)| {
                        strider_ui::chip(label, label, self.filter.status == status, t).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.filter.status = status;
                                this.reload(cx);
                            }),
                        )
                    }),
                )
                .into_any_element();
            let view = cx.entity();
            let list = list(self.list.clone(), move |index, window, cx| {
                view.update(cx, |this, cx| this.render_entry(index, window, cx))
            })
            .w_full()
            .min_h_0()
            .flex_1();
            strider_ui::page_body(vec![filters, list.into_any_element()], t)
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .into_any_element()
        };
        let overlay =
            self.editor.as_ref().map(|editor| {
                let content = div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when_some(self.error.clone(), |d, error| {
                        d.child(div().meta(t).child(error))
                    })
                    .child(strider_ui::field(
                        "Name",
                        Input::new(&editor.name)
                            .aria_label("Name")
                            .bg(t.panel)
                            .rounded(px(10.))
                            .h(px(44.)),
                        t,
                    ))
                    .child(strider_ui::field(
                        "Weight (oz)",
                        div()
                            .child(
                                Input::new(&editor.weight)
                                    .aria_label("Weight in ounces")
                                    .bg(t.panel)
                                    .rounded(px(10.))
                                    .h(px(44.)),
                            )
                            .when(editor.weight_error, |d| {
                                d.child(div().meta(t).child("Enter a number."))
                            }),
                        t,
                    ))
                    .child(strider_ui::field(
                        "Quantity",
                        div()
                            .child(
                                Input::new(&editor.quantity)
                                    .aria_label("Quantity")
                                    .bg(t.panel)
                                    .rounded(px(10.))
                                    .h(px(44.)),
                            )
                            .when(editor.quantity_error, |d| {
                                d.child(div().meta(t).child("Enter a number."))
                            }),
                        t,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                strider_ui::button("save", "Save", ButtonVariant::Primary, t)
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.save(window, cx)),
                                    ),
                            )
                            .child(
                                strider_ui::button("cancel", "Cancel", ButtonVariant::Ghost, t)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_editor(window, cx)
                                    })),
                            ),
                    );
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div().id("scrim").absolute().inset_0().bg(t.scrim).on_click(
                            cx.listener(|this, _, window, cx| this.close_editor(window, cx)),
                        ),
                    )
                    .child(
                        strider_ui::sheet(
                            editor.title.clone(),
                            content.into_any_element(),
                            strider_ui::icon_button("close", Icon::X, t).on_click(
                                cx.listener(|this, _, window, cx| this.close_editor(window, cx)),
                            ),
                            t,
                        )
                        .id("edit-sheet")
                        .occlude()
                        .max_h(relative(0.9))
                        .overflow_y_scroll(),
                    )
                    .into_any_element()
            });
        div()
            .size_full()
            .relative()
            .body()
            .bg(t.canvas)
            .text_color(t.ink)
            .track_focus(&self.focus)
            .key_context("Gear")
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_editor(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.select(1, cx)))
            .on_action(cx.listener(|this, _: &SelectPrevious, _, cx| this.select(-1, cx)))
            .on_action(cx.listener(|this, _: &DeleteSelected, _, cx| this.delete(cx)))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(false, cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.undo(true, cx)))
            .on_action(cx.listener(|this, _: &EditSelected, window, cx| {
                if let Some(id) = this.selected.clone() {
                    this.edit(id, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Find, window, cx| {
                if this.editor.is_none() {
                    this.filter_input.update(cx, |s, cx| s.focus(window, cx));
                }
            }))
            .on_action(cx.listener(|this, _: &Escape, window, cx| this.close_editor(window, cx)))
            .on_action(cx.listener(|this, _: &Light, window, cx| {
                this.appearance(Appearance::Light, window, cx)
            }))
            .on_action(cx.listener(|this, _: &Dark, window, cx| {
                this.appearance(Appearance::Dark, window, cx)
            }))
            .on_action(cx.listener(|this, _: &System, window, cx| {
                this.appearance(Appearance::System, window, cx)
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(shell)
                    .child(header)
                    .child(body)
                    .when_some(self.error.clone(), |d, error| {
                        d.child(div().meta(t).child(error))
                    }),
            )
            .children(overlay)
    }
}
fn menus(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some("Gear")),
        KeyBinding::new("up", SelectPrevious, Some("Gear")),
        KeyBinding::new("enter", EditSelected, Some("Gear")),
        KeyBinding::new("backspace", DeleteSelected, Some("Gear")),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("cmd-z", Undo, Some("Gear")),
        KeyBinding::new("cmd-shift-z", Redo, Some("Gear")),
        KeyBinding::new("cmd-q", Quit, None),
    ]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus([
        Menu::new("Strider").items([MenuItem::action("Quit Strider", Quit)]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", gpui_component::input::Cut),
            MenuItem::action("Copy", gpui_component::input::Copy),
            MenuItem::action("Paste", gpui_component::input::Paste),
            MenuItem::action("Select all", gpui_component::input::SelectAll),
            MenuItem::separator(),
            MenuItem::action("Find gear", Find),
            MenuItem::action("Select next", SelectNext),
            MenuItem::action("Select previous", SelectPrevious),
            MenuItem::action("Edit item", EditSelected),
            MenuItem::action("Delete item", DeleteSelected),
        ]),
        Menu::new("View").items([MenuItem::submenu(Menu::new("Appearance").items([
            MenuItem::action("Light", Light),
            MenuItem::action("Dark", Dark),
            MenuItem::action("System", System),
        ]))]),
    ]);
}
#[derive(Debug, Default, PartialEq, Eq)]
struct WorkspaceOptions {
    profile: String,
    reset: bool,
}
impl WorkspaceOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            profile: "dev".into(),
            reset: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--reset" => options.reset = true,
                "--profile" => {
                    options.profile = args
                        .next()
                        .filter(|value| !value.starts_with('-'))
                        .ok_or("--profile requires a name")?;
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(options)
    }
}
fn open_store() -> Result<Store, Box<dyn std::error::Error>> {
    let options = WorkspaceOptions::parse(std::env::args().skip(1))?;
    let path = Store::path_for_profile(&options.profile)?;
    if options.reset && path.exists() {
        std::fs::remove_file(&path)?;
    }
    let needs_seed = !path.exists();
    let mut store = Store::open(&path)?;
    if needs_seed {
        seed(&mut store, 2000)?;
    }
    Ok(store)
}
fn main() {
    let store = match open_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Could not open the Strider workspace: {error}");
            return;
        }
    };
    gpui_platform::application().run(move |cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        menus(cx);
        let bounds = Bounds::centered(None, size(px(1180.), px(820.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(640.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Strider".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.), px(22.))),
                }),
                app_owns_titlebar_drag: true,
                ..Default::default()
            },
            move |window, cx| {
                strider_ui::set_appearance(Appearance::System, window, cx);
                let gear = cx.new(|cx| GearView::new(store, window, cx));
                cx.new(|cx| Root::new(gear, window, cx))
            },
        )
        .expect("open the Strider window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use super::{DeleteSelected, EditSelected, GearView, SelectNext, Undo, menus};
    use gpui::{AppContext, TestAppContext, VisualTestContext, WindowOptions};
    use gpui_component::Root;
    use strider_store::{GearFilter, StatusFilter, Store, seed};
    #[test]
    fn workspace_options() {
        let parse = |args: &[&str]| {
            super::WorkspaceOptions::parse(args.iter().map(|arg| (*arg).to_owned()))
        };
        assert_eq!(parse(&[]).expect("defaults").profile, "dev");
        let custom = parse(&["--profile", "test", "--reset"]).expect("custom profile");
        assert_eq!(custom.profile, "test");
        assert!(custom.reset);
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--profile", "--reset"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
    }
    #[gpui::test]
    fn gear_actions_delete_and_restore_fixture(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            strider_ui::initialize(cx);
            menus(cx);
        });
        let mut store = Store::in_memory().expect("in-memory store");
        seed(&mut store, 2000).expect("fixture");
        let before = store
            .gear_groups(&GearFilter {
                status: StatusFilter::All,
                query: String::new(),
            })
            .expect("groups");
        let mut view = None;
        let window = cx.update(|cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let gear = cx.new(|cx| GearView::new(store, window, cx));
                view = Some(gear.clone());
                cx.new(|cx| Root::new(gear, window, cx))
            })
            .expect("headless window")
        });
        let gear = view.expect("gear entity");
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        cx.dispatch_action(SelectNext);
        cx.dispatch_action(SelectNext);
        let selected = gear
            .read_with(&cx, |g, _| g.selected.clone())
            .expect("second row selected");
        assert_eq!(
            selected,
            before[0]
                .rows
                .iter()
                .filter(|r| r.status == "current")
                .nth(1)
                .expect("second current item")
                .id
        );
        cx.dispatch_action(EditSelected);
        assert!(gear.read_with(&cx, |g, _| g.editor.is_some()));
        cx.update(|window, cx| {
            gear.update(cx, |g, cx| {
                let weight = g.editor.as_ref().expect("editor").weight.clone();
                weight.update(cx, |s, cx| s.set_value("not a number", window, cx));
                g.save(window, cx);
                assert!(
                    g.editor
                        .as_ref()
                        .expect("invalid save stays open")
                        .weight_error
                );
            })
        });
        cx.simulate_keystrokes("escape");
        assert!(gear.read_with(&cx, |g, _| g.editor.is_none()));
        let old_count = gear.read_with(&cx, |g, _| g.totals.items);
        cx.dispatch_action(DeleteSelected);
        assert_eq!(gear.read_with(&cx, |g, _| g.totals.items), old_count - 1);
        assert!(
            !gear
                .read_with(&cx, |g, _| g
                    .store
                    .gear_groups(&GearFilter {
                        status: StatusFilter::All,
                        query: String::new()
                    })
                    .expect("groups"))
                .iter()
                .flat_map(|g| &g.rows)
                .any(|r| r.id == selected)
        );
        cx.dispatch_action(Undo);
        assert_eq!(
            gear.read_with(&cx, |g, _| g
                .store
                .gear_groups(&GearFilter {
                    status: StatusFilter::All,
                    query: String::new()
                })
                .expect("groups")),
            before
        );
    }
}
