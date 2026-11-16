use super::*;
use gpui_component::input::TextareaState;
pub(crate) struct ComponentDraft {
    pub(crate) name: Entity<InputState>,
    pub(crate) weight: Entity<InputState>,
    pub(crate) quantity: Entity<InputState>,
    pub(crate) notes: Option<String>,
}
pub(crate) struct Editor {
    pub(crate) id: Option<String>,
    pub(crate) name: Entity<InputState>,
    pub(crate) weight: Entity<InputState>,
    pub(crate) quantity: Entity<InputState>,
    pub(crate) price: Entity<InputState>,
    pub(crate) url: Entity<InputState>,
    pub(crate) notes: Entity<TextareaState>,
    pub(crate) category: Option<String>,
    pub(crate) class: String,
    pub(crate) status: String,
    pub(crate) is_kit: bool,
    pub(crate) components: Vec<ComponentDraft>,
    pub(crate) weight_error: bool,
    pub(crate) quantity_error: bool,
}
impl ComponentDraft {
    fn new(
        name: String,
        weight: String,
        quantity: String,
        notes: Option<String>,
        w: &mut Window,
        cx: &mut Context<AppView>,
    ) -> Self {
        Self {
            name: input(name, false, w, cx),
            weight: input(weight, false, w, cx),
            quantity: input(quantity, false, w, cx),
            notes,
        }
    }
}
impl AppView {
    pub(crate) fn add_gear(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open_gear(None, w, cx);
    }
    pub(crate) fn edit(&mut self, id: String, w: &mut Window, cx: &mut Context<Self>) {
        if self.overlay_open() || self.area != "Gear" {
            return;
        }
        self.open_gear(Some(id), w, cx);
    }
    fn open_gear(&mut self, id: Option<String>, w: &mut Window, cx: &mut Context<Self>) {
        self.reload(cx);
        let row = id
            .as_ref()
            .and_then(|id| self.all_gear.iter().find(|r| &r.id == id))
            .cloned();
        if id.is_some() && row.is_none() {
            self.error = Some("Gear item no longer exists".into());
            return;
        }
        let name = input(
            row.as_ref().map(|r| r.name.clone()).unwrap_or_default(),
            false,
            w,
            cx,
        );
        name.update(cx, |s, cx| s.focus(w, cx));
        self.editor = Some(Editor {
            id,
            name,
            weight: input(
                row.as_ref()
                    .and_then(|r| r.weight)
                    .map(|g| g_to_oz(g).to_string())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            quantity: input(
                row.as_ref().map(|r| r.quantity).unwrap_or(1).to_string(),
                false,
                w,
                cx,
            ),
            price: input(
                row.as_ref()
                    .and_then(|r| r.price_cents)
                    .map(|c| format!("{:.2}", f64::from(c.0) / 100.))
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            url: input(
                row.as_ref()
                    .and_then(|r| r.product_url.clone())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            notes: textarea_input(
                row.as_ref()
                    .and_then(|r| r.notes.clone())
                    .unwrap_or_default(),
                w,
                cx,
            ),
            category: row.as_ref().and_then(|r| r.category_id.clone()),
            class: row
                .as_ref()
                .map(|r| r.class.clone())
                .unwrap_or_else(|| "base".into()),
            status: row
                .as_ref()
                .map(|r| r.status.clone())
                .unwrap_or_else(|| if self.wishlist { "wishlist" } else { "current" }.into()),
            is_kit: row.as_ref().is_some_and(|r| r.is_kit),
            components: row
                .as_ref()
                .into_iter()
                .flat_map(|r| r.components.iter())
                .map(|c| {
                    ComponentDraft::new(
                        c.name.clone(),
                        g_to_oz(c.weight).to_string(),
                        c.quantity.to_string(),
                        c.notes.clone(),
                        w,
                        cx,
                    )
                })
                .collect(),
            weight_error: false,
            quantity_error: false,
        });
        cx.notify();
    }
    pub(crate) fn close_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.editor = None;
        self.focus.focus(w, cx);
        cx.notify();
    }
    pub(crate) fn save(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = &self.editor else { return };
        let result = (|| -> Result<GearInput, String> {
            let weight = if e.is_kit {
                None
            } else {
                number(&e.weight, cx)?
                    .map(|n| {
                        if n * strider_core::OZ_TO_G > i32::MAX as f64 {
                            Err("Weight exceeds the supported gram range".to_owned())
                        } else {
                            Ok(oz_to_g(n))
                        }
                    })
                    .transpose()?
            };
            let quantity = integer(&e.quantity, cx, true)?;
            let price = number(&e.price, cx)?
                .map(|n| {
                    if n * 100. > i32::MAX as f64 {
                        Err("Price exceeds the supported range".to_owned())
                    } else {
                        Ok(strider_core::Cents((n * 100. + 0.5).floor() as i32))
                    }
                })
                .transpose()?;
            let mut components = vec![];
            for c in &e.components {
                if value(&c.name, cx).trim().is_empty() {
                    continue;
                }
                let oz = number(&c.weight, cx)?.unwrap_or(0.);
                if oz * strider_core::OZ_TO_G > i32::MAX as f64 {
                    return Err("Weight exceeds the supported gram range".into());
                }
                components.push(ComponentInput {
                    name: value(&c.name, cx),
                    weight_g: oz_to_g(oz).0,
                    quantity: integer(&c.quantity, cx, true)?,
                    notes: c.notes.clone(),
                });
            }
            Ok(GearInput {
                name: value(&e.name, cx),
                weight_g: weight,
                weight_oz: None,
                quantity,
                price_cents: price,
                category_id: e.category.clone(),
                class: e.class.clone(),
                status: e.status.clone(),
                is_kit: e.is_kit,
                components,
                product_url: optional(&e.url, cx),
                notes: Some(e.notes.read(cx).value().to_string()),
            })
        })();
        match result {
            Ok(input) => {
                let command = if let Some(id) = e.id.clone() {
                    Command::UpdateGear { id, input }
                } else {
                    Command::CreateGear { input }
                };
                self.command(command, cx);
                if self.error.is_none() {
                    self.close_editor(w, cx);
                }
            }
            Err(error) => {
                if let Some(e) = &mut self.editor {
                    e.weight_error = true;
                    e.quantity_error = true;
                }
                self.error = Some(error);
                cx.notify();
            }
        }
    }
    pub(crate) fn render_gear_dialog(&self, cx: &Context<Self>) -> AnyElement {
        let e = self.editor.as_ref().expect("open editor");
        let t = Theme::get(cx);
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_color(t.signal).child(e))
            })
            .child(strider_ui::field(
                "Name",
                strider_ui::text_input(&e.name, "Name", t),
                t,
            ));
        let mut cats = vec![(String::new(), "No category".into())];
        cats.extend(
            self.categories
                .iter()
                .map(|c| (c.id.clone(), c.name.clone())),
        );
        body = body.child(strider_ui::field(
            "Category",
            self.choice(
                "gear-category",
                self.categories
                    .iter()
                    .find(|c| Some(&c.id) == e.category.as_ref())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "No category".into()),
                cats,
                cx,
                |this, v, _, cx| {
                    if let Some(e) = &mut this.editor {
                        e.category = (!v.is_empty()).then_some(v);
                    }
                    cx.notify();
                },
            ),
            t,
        ));
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    strider_ui::checkbox("is-kit", "This is a kit", e.is_kit, t).on_click(
                        cx.listener(|this, _, _, cx| {
                            if let Some(e) = &mut this.editor {
                                e.is_kit = !e.is_kit;
                            }
                            cx.notify();
                        }),
                    ),
                )
                .child("This is a kit"),
        );
        if e.is_kit {
            let total: i64 = e
                .components
                .iter()
                .map(|c| {
                    let oz = number(&c.weight, cx).ok().flatten().unwrap_or(0.);
                    i64::from(oz_to_g(oz).0)
                        * i64::from(integer(&c.quantity, cx, true).unwrap_or(1))
                })
                .sum();
            body = body.child(div().readout().child(format!(
                "Total: {:.2} oz",
                total as f64 / strider_core::OZ_TO_G
            )));
            for (i, c) in e.components.iter().enumerate() {
                body = body.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().flex_1().child(strider_ui::text_input(
                            &c.name,
                            "Component name",
                            t,
                        )))
                        .child(div().w(px(88.)).child(strider_ui::text_input(
                            &c.weight,
                            "Component weight in ounces",
                            t,
                        )))
                        .child(div().w(px(64.)).child(strider_ui::text_input(
                            &c.quantity,
                            "Component quantity",
                            t,
                        )))
                        .child(
                            strider_ui::icon_button(("remove-component", i), Icon::X, t).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(e) = &mut this.editor {
                                        e.components.remove(i);
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                );
            }
            body = body.child(
                strider_ui::button("add-component", "+ Add item", ButtonVariant::Ghost, t)
                    .on_click(cx.listener(|this, _, w, cx| {
                        let c = ComponentDraft::new(
                            String::new(),
                            String::new(),
                            "1".into(),
                            None,
                            w,
                            cx,
                        );
                        if let Some(e) = &mut this.editor {
                            e.components.push(c);
                        }
                        cx.notify();
                    })),
            );
        } else {
            body = body.child(strider_ui::field(
                "Weight (oz)",
                strider_ui::text_input(&e.weight, "Weight in ounces", t),
                t,
            ));
        }
        body = body.child(
            div()
                .flex()
                .gap_3()
                .child(strider_ui::field(
                    "Quantity",
                    strider_ui::text_input(&e.quantity, "Quantity", t),
                    t,
                ))
                .child(strider_ui::field(
                    "Price (USD)",
                    strider_ui::text_input(&e.price, "Price in USD", t),
                    t,
                )),
        );
        body = body.child(strider_ui::field(
            "Default class",
            strider_ui::segmented_control(
                ["base", "worn", "consumable"].into_iter().map(|class| {
                    strider_ui::segment(class, class, e.class == class, t)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(e) = &mut this.editor {
                                e.class = class.into();
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                }),
                t,
            ),
            t,
        ));
        body = body
            .child(strider_ui::field(
                "Status",
                self.choice(
                    "gear-status",
                    e.status.clone(),
                    ["current", "retired", "wishlist"]
                        .into_iter()
                        .map(|s| (s.into(), s.into()))
                        .collect(),
                    cx,
                    |this, v, _, cx| {
                        if let Some(e) = &mut this.editor {
                            e.status = v;
                        }
                        cx.notify();
                    },
                ),
                t,
            ))
            .child(strider_ui::field(
                "Product URL",
                strider_ui::text_input(&e.url, "Product URL", t),
                t,
            ))
            .child(strider_ui::field(
                "Notes",
                strider_ui::textarea(&e.notes, "Notes", t),
                t,
            ));
        let mut footer = div()
            .flex()
            .gap_2()
            .child(
                strider_ui::button("save", "Save", ButtonVariant::Primary, t)
                    .on_click(cx.listener(|this, _, w, cx| this.save(w, cx))),
            )
            .child(
                strider_ui::button("cancel", "Cancel", ButtonVariant::Ghost, t)
                    .on_click(cx.listener(|this, _, w, cx| this.close_editor(w, cx))),
            );
        if let Some(id) = e.id.clone() {
            footer = footer.child(
                strider_ui::button("delete-gear", "Delete", ButtonVariant::Danger, t).on_click(
                    cx.listener(move |this, _, w, cx| {
                        this.command(Command::DeleteGear { id: id.clone() }, cx);
                        if this.error.is_none() {
                            this.close_editor(w, cx);
                        }
                    }),
                ),
            );
        }
        self.modal(
            if e.id.is_some() {
                "Edit gear"
            } else {
                "Add gear"
            },
            body.child(footer).into_any_element(),
            cx,
        )
    }
}
