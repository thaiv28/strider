use super::*;
pub(crate) struct Manager {
    pub(crate) categories: bool,
    pub(crate) names: Vec<(String, Entity<InputState>)>,
    pub(crate) new_name: Entity<InputState>,
}
impl AppView {
    pub(crate) fn open_manager(
        &mut self,
        categories: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reload(cx);
        let names: Vec<_> = if categories {
            self.categories
                .iter()
                .map(|c| (c.id.clone(), c.name.clone()))
                .collect()
        } else {
            self.loadouts
                .loadouts
                .iter()
                .map(|l| (l.id.clone(), l.name.clone()))
                .collect()
        };
        self.manager = Some(Manager {
            categories,
            names: names
                .into_iter()
                .map(|(id, name)| (id, input(name, false, w, cx)))
                .collect(),
            new_name: input("", false, w, cx),
        });
        cx.notify();
    }
    fn reorder_manager(&mut self, index: usize, delta: isize, cx: &mut Context<Self>) {
        let Some(m) = &self.manager else { return };
        let mut ids: Vec<_> = if m.categories {
            self.categories.iter().map(|c| c.id.clone()).collect()
        } else {
            self.loadouts
                .loadouts
                .iter()
                .map(|l| l.id.clone())
                .collect()
        };
        let next = index as isize + delta;
        if next < 0 || next >= ids.len() as isize {
            return;
        }
        ids.swap(index, next as usize);
        let command = if m.categories {
            Command::ReorderCategories { ordered_ids: ids }
        } else {
            Command::ReorderLoadouts { ordered_ids: ids }
        };
        self.command(command, cx);
        if let Some(m) = &mut self.manager {
            m.names.sort_by_key(|(id, _)| {
                if m.categories {
                    self.categories.iter().position(|c| &c.id == id)
                } else {
                    self.loadouts.loadouts.iter().position(|l| &l.id == id)
                }
            });
        }
    }
    pub(crate) fn render_manager(&self, cx: &Context<Self>) -> AnyElement {
        let m = self.manager.as_ref().expect("manager");
        let categories = m.categories;
        let t = Theme::get(cx);
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_color(t.signal).child(e))
            });
        for (i, (id, name)) in m.names.iter().enumerate() {
            let rename = id.clone();
            let delete = id.clone();
            let default = id.clone();
            let state = name.clone();
            let mut row = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .child(strider_ui::text_input(name, "Name", t)),
                )
                .child(
                    strider_ui::button(("rename", i), "Rename", ButtonVariant::Ghost, t).on_click(
                        cx.listener(move |this, _, _, cx| {
                            let name = value(&state, cx);
                            this.command(
                                if categories {
                                    Command::RenameCategory {
                                        id: rename.clone(),
                                        name,
                                    }
                                } else {
                                    Command::RenameLoadout {
                                        id: rename.clone(),
                                        name,
                                    }
                                },
                                cx,
                            );
                        }),
                    ),
                )
                .child(
                    strider_ui::button(("up", i), "↑", ButtonVariant::Ghost, t)
                        .aria_label("Move up")
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.reorder_manager(i, -1, cx)),
                        ),
                )
                .child(
                    strider_ui::button(("down", i), "↓", ButtonVariant::Ghost, t)
                        .aria_label("Move down")
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.reorder_manager(i, 1, cx)),
                        ),
                );
            if !categories {
                let is_default = self
                    .loadouts
                    .loadouts
                    .iter()
                    .any(|l| l.id == *id && l.is_default);
                row = row.child(
                    strider_ui::button(
                        ("default", i),
                        if is_default { "Default" } else { "Set default" },
                        ButtonVariant::Ghost,
                        t,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.command(
                            Command::SetDefaultLoadout {
                                id: default.clone(),
                            },
                            cx,
                        )
                    })),
                );
            }
            row = row.child(
                strider_ui::button(("delete", i), "Delete", ButtonVariant::Danger, t).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.command(
                            if categories {
                                Command::DeleteCategory { id: delete.clone() }
                            } else {
                                Command::DeleteLoadout { id: delete.clone() }
                            },
                            cx,
                        );
                        if this.error.is_none()
                            && let Some(m) = &mut this.manager
                        {
                            m.names.retain(|(id, _)| id != &delete);
                        }
                    }),
                ),
            );
            body = body.child(row);
        }
        body = body.child(
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .child(strider_ui::text_input(&m.new_name, "New name", t)),
                )
                .child(
                    strider_ui::button(
                        "create-managed",
                        if categories {
                            "+ Add category"
                        } else {
                            "+ Add loadout"
                        },
                        ButtonVariant::Outline,
                        t,
                    )
                    .on_click(cx.listener(move |this, _, w, cx| {
                        let Some(m) = &this.manager else { return };
                        let name = value(&m.new_name, cx);
                        if name.trim().is_empty() {
                            this.error = Some("Name is required".into());
                            cx.notify();
                            return;
                        }
                        this.command(
                            if categories {
                                Command::CreateCategory { name }
                            } else {
                                Command::CreateLoadout { name }
                            },
                            cx,
                        );
                        if this.error.is_none() {
                            this.open_manager(categories, w, cx);
                        }
                    })),
                ),
        );
        self.modal(
            if categories {
                "Manage categories"
            } else {
                "Manage loadouts"
            },
            body.into_any_element(),
            cx,
        )
    }
}
