use super::*;
#[derive(Clone)]
pub(crate) enum Entry {
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
pub(crate) struct DragRow {
    pub(crate) id: String,
    pub(crate) row: GearRow,
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
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(format!(
                                "{}{}",
                                self.row.name,
                                if self.row.quantity > 1 {
                                    format!(" ×{}", self.row.quantity)
                                } else {
                                    String::new()
                                }
                            )),
                    )
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

impl AppView {
    pub(crate) fn flatten(&mut self) {
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
    pub(crate) fn drop_row(
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
    pub(crate) fn render_entry(
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
                let member = self.member_class(&id);
                let loadout_mode = self.loadout.is_some() && row.status == "current";
                let worn = if self.loadout.is_some() {
                    member.as_deref() == Some("worn")
                } else {
                    row.class == "worn"
                };
                let membership_id = id.clone();
                let default_class = if row.class == "worn" { "worn" } else { "base" }.to_owned();
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
                    .child(grip)
                    .when(loadout_mode, |d| {
                        d.child(
                            strider_ui::checkbox(
                                ("member", index),
                                format!("Include {}", row.name),
                                member.is_some(),
                                t,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.toggle_member(
                                        membership_id.clone(),
                                        member.is_none(),
                                        default_class.clone(),
                                        cx,
                                    );
                                },
                            )),
                        )
                    })
                    .when(loadout_mode && self.member_class(&id).is_none(), |d| {
                        d.opacity(0.45)
                    });
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
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(format!(
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
                    .when(worn, |d| d.child(strider_ui::badge("Worn", "worn", t)))
                    .when(loadout_mode && self.member_class(&id).is_some(), |d| {
                        let item = id.clone();
                        d.child(strider_ui::worn_toggle(
                            [("base", "pack"), ("worn", "worn")].into_iter().map(
                                |(class, label)| {
                                    let item = item.clone();
                                    strider_ui::chip(
                                        SharedString::from(format!("worn-{index}-{class}")),
                                        label,
                                        if worn {
                                            class == "worn"
                                        } else {
                                            class == "base"
                                        },
                                        t,
                                    )
                                    .rounded_full()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.toggle_member(item.clone(), true, class.into(), cx);
                                    }))
                                    .into_any_element()
                                },
                            ),
                            t,
                        ))
                    })
                    .child(
                        div()
                            .readout()
                            .w(px(104.))
                            .text_right()
                            .child(fmt_oz(row.weight.map(|w| w * row.quantity))),
                    )
                    .child(self.choice(
                        ("more", index),
                        "…",
                        {
                            let mut actions = vec![("edit".into(), "Edit".into())];
                            if loadout_mode {
                                actions.push((
                                    "worn".into(),
                                    if worn {
                                        "Mark as packed"
                                    } else {
                                        "Mark as worn"
                                    }
                                    .into(),
                                ));
                            }
                            actions.push(("delete".into(), "Delete".into()));
                            actions
                        },
                        cx,
                        move |this, action, w, cx| match action.as_str() {
                            "edit" => this.edit(edit_id.clone(), w, cx),
                            "delete" => this.command(
                                Command::DeleteGear {
                                    id: edit_id.clone(),
                                },
                                cx,
                            ),
                            "worn" => this.toggle_member(
                                edit_id.clone(),
                                true,
                                if worn { "base" } else { "worn" }.into(),
                                cx,
                            ),
                            _ => {}
                        },
                    ))
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
