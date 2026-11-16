use super::*;
impl AppView {
    pub(crate) fn render_wishlist(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let rows = self.core_rows();
        let membership = self.wishlist_loadout.as_ref().map(|id| self.membership(id));
        let overrides: BTreeMap<_, _> = self
            .all_gear
            .iter()
            .filter_map(|r| {
                self.wishlist_loadout
                    .as_ref()
                    .and_then(|id| {
                        self.loadouts
                            .wishlist_overrides
                            .get(&r.id)
                            .and_then(|m| m.get(id))
                    })
                    .map(|v| (r.id.clone(), *v))
            })
            .collect();
        let projection =
            strider_core::gear::wishlist_projection(&rows, membership.as_ref(), &overrides);
        let summary = strider_ui::card(t)
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().meta(t).child("If added to"))
            .child(self.loadout_select("wishlist-loadout", true, cx))
            .child(div().section_heading().readout().child(format!(
                "{:.1} → {:.1} lb",
                projection.baseline_g as f64 / strider_core::OZ_TO_G / 16.,
                projection.projected_base_g as f64 / strider_core::OZ_TO_G / 16.
            )))
            .child(div().meta(t).child(format!(
                "{} items · {}",
                projection.count,
                if projection.price_known {
                    format!("${:.2}", projection.total_cents as f64 / 100.)
                } else {
                    "Price incomplete".into()
                }
            )));
        let saving =
            (projection.baseline_g - projection.projected_base_g) as f64 / strider_core::OZ_TO_G;
        let summary = summary.when(saving > 0., |d| {
            d.child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(t.signal)
                    .child(format!("Saves {saving:.1} oz")),
            )
            .when(projection.price_known, |d| {
                d.child(div().meta(t).child(format!(
                    "About ${:.2} per ounce saved",
                    projection.total_cents as f64 / 100. / saving
                )))
            })
        });
        let mut list = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(strider_ui::text_input(
                &self.wishlist_query,
                "Search wishlist",
                t,
            ));
        let mut count = 0;
        let mut group: Option<(Option<String>, Div)> = None;
        for (i, r) in self
            .all_gear
            .iter()
            .filter(|r| {
                r.status == "wishlist"
                    && r.name
                        .to_lowercase()
                        .contains(&value(&self.wishlist_query, cx).to_lowercase())
            })
            .enumerate()
        {
            if group
                .as_ref()
                .is_none_or(|(category, _)| category != &r.category_id)
            {
                if let Some((_, card)) = group.take() {
                    list = list.child(card);
                }
                let name = r.slot_name.clone().unwrap_or_else(|| "Other".into());
                group = Some((
                    r.category_id.clone(),
                    strider_ui::card(t).child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_4()
                            .py_2()
                            .bg(t.panel2)
                            .child(
                                div()
                                    .size(px(10.))
                                    .bg(strider_ui::category_color(&name).unwrap_or(t.muted)),
                            )
                            .child(div().font_weight(FontWeight::BOLD).child(name)),
                    ),
                ));
            }
            count += 1;
            let active = self.wishlist_loadout.as_ref().is_none_or(|id| {
                self.loadouts
                    .wishlist_overrides
                    .get(&r.id)
                    .and_then(|m| m.get(id))
                    .copied()
                    .unwrap_or_else(|| {
                        self.loadouts
                            .membership
                            .get(id)
                            .is_some_and(|m| r.replaces.iter().any(|id| m.contains_key(id)))
                    })
            });
            let id = r.id.clone();
            let name_edit = r.id.clone();
            let swing = i64::from(r.weight.unwrap_or_default().0) * i64::from(r.quantity)
                - r.replaces
                    .iter()
                    .filter_map(|id| {
                        self.all_gear
                            .iter()
                            .find(|r| &r.id == id && r.status == "current")
                    })
                    .map(|r| i64::from(r.weight.unwrap_or_default().0) * i64::from(r.quantity))
                    .sum::<i64>();
            let edit = r.id.clone();
            let mut row = div()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(t.line)
                .flex()
                .gap_2()
                .items_center()
                .when(!active, |d| d.opacity(0.55));
            if let Some(loadout) = self.wishlist_loadout.clone() {
                row = row.child(
                    strider_ui::checkbox(
                        ("wish-include", i),
                        format!("Try {} in this loadout", r.name),
                        active,
                        t,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.command(
                            Command::SetWishlistLoadout {
                                item: id.clone(),
                                loadout: loadout.clone(),
                                included: Some(!active),
                            },
                            cx,
                        )
                    })),
                );
            }
            row = row
                .child(
                    div()
                        .id(("wish-name", i))
                        .cursor_pointer()
                        .on_click(
                            cx.listener(move |this, _, w, cx| this.edit(name_edit.clone(), w, cx)),
                        )
                        .flex_1()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(r.name.clone()),
                        )
                        .child(div().meta(t).child(if r.replaces.is_empty() {
                            "New item".into()
                        } else {
                            format!(
                                "Replaces {}",
                                r.replaces
                                    .iter()
                                    .filter_map(|id| self
                                        .all_gear
                                        .iter()
                                        .find(|r| &r.id == id)
                                        .map(|r| r.name.clone()))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })),
                )
                .child(div().readout().child(format!(
                        "{} · {}",
                        if r.weight.is_none() {
                            "—".into()
                        } else {
                            format!(
                                "{}{:.1} oz",
                                if swing < 0 { "−" } else { "+" },
                                swing.unsigned_abs() as f64 / strider_core::OZ_TO_G
                            )
                        },
                        r.price_cents
                            .map(|p| format!(
                                "${:.2}",
                                f64::from(p.0) * f64::from(r.quantity) / 100.
                            ))
                            .unwrap_or_else(|| "—".into())
                    )))
                .child(self.choice(
                    ("wish-more", i),
                    "…",
                    vec![
                        ("edit".into(), "Edit".into()),
                        ("replacements".into(), "Edit replacements".into()),
                        ("auto".into(), "Use automatic selection".into()),
                        ("delete".into(), "Delete".into()),
                    ],
                    cx,
                    move |this, v, w, cx| match v.as_str() {
                        "edit" => this.edit(edit.clone(), w, cx),
                        "replacements" => {
                            this.replacements = Some(edit.clone());
                            cx.notify();
                        }
                        "delete" => this.command(Command::DeleteGear { id: edit.clone() }, cx),
                        "auto" => {
                            if let Some(loadout) = this.wishlist_loadout.clone() {
                                this.command(
                                    Command::SetWishlistLoadout {
                                        item: edit.clone(),
                                        loadout,
                                        included: None,
                                    },
                                    cx,
                                );
                            }
                        }
                        _ => {}
                    },
                ));
            if let Some((_, card)) = group.take() {
                group = Some((r.category_id.clone(), card.child(row)));
            }
        }
        if let Some((_, card)) = group.take() {
            list = list.child(card);
        }
        if count == 0 {
            list = list.child(div().meta(t).child(
                "Nothing on the wishlist yet — add an item and set its status to “wishlist”.",
            ));
        }
        let layout = div().flex().gap_5().items_start();
        let layout = if window.viewport_size().width >= px(1100.) {
            layout
                .child(div().flex_1().min_w_0().child(list))
                .child(div().w(px(340.)).flex_shrink_0().child(summary))
        } else {
            layout.flex_col().child(summary).child(list)
        };
        strider_ui::page_body(
            vec![
                div()
                    .id("wishlist-list")
                    .flex_1()
                    .min_h_0()
                    .child(layout)
                    .overflow_y_scroll()
                    .into_any_element(),
            ],
            t,
        )
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .into_any_element()
    }
    pub(crate) fn render_replacements(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let Some(wish) = self
            .replacements
            .as_ref()
            .and_then(|id| self.all_gear.iter().find(|r| &r.id == id))
        else {
            return self.modal(
                "Replacements",
                div().child("Item no longer exists").into_any_element(),
                cx,
            );
        };
        let mut body = div().flex().flex_col().gap_2();
        for (i, r) in self
            .all_gear
            .iter()
            .filter(|r| r.status == "current")
            .enumerate()
        {
            let item = wish.id.clone();
            let target = r.id.clone();
            let selected = wish.replaces.contains(&r.id);
            body = body.child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        strider_ui::checkbox(
                            ("replacement", i),
                            format!("Replace {}", r.name),
                            selected,
                            t,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let mut replaces = this
                                .all_gear
                                .iter()
                                .find(|r| r.id == item)
                                .map(|r| r.replaces.clone())
                                .unwrap_or_default();
                            if selected {
                                replaces.retain(|id| id != &target);
                            } else {
                                replaces.push(target.clone());
                            }
                            this.command(
                                Command::SetWishlistReplacements {
                                    item: item.clone(),
                                    replaces,
                                },
                                cx,
                            );
                        })),
                    )
                    .child(r.name.clone()),
            );
        }
        self.modal(
            format!("Replacements for {}", wish.name),
            body.into_any_element(),
            cx,
        )
    }
}
