use super::*;
use gpui_component::input::TextareaState;
use strider_core::{food, units::Dec};
use strider_store::{MealIngredientRow, MealPatch};
pub(crate) struct LineDraft {
    pub(crate) amount: Entity<InputState>,
    pub(crate) unit: String,
    original: String,
    original_unit: String,
}
pub(crate) struct MealEditor {
    pub(crate) id: String,
    pub(crate) name: Entity<InputState>,
    pub(crate) servings: Entity<InputState>,
    pub(crate) water: Entity<InputState>,
    pub(crate) notes: Entity<TextareaState>,
    pub(crate) lines: BTreeMap<String, LineDraft>,
    pub(crate) as_weight: bool,
    pub(crate) preview: Entity<InputState>,
    preview_servings: i32,
    originals: [String; 4],
}
impl AppView {
    pub(crate) fn open_meal(&mut self, id: String, w: &mut Window, cx: &mut Context<Self>) {
        self.reload(cx);
        let Some(m) = self.food.meals.iter().find(|m| m.id == id).cloned() else {
            return;
        };
        self.food.editor = Some(MealEditor {
            originals: [
                m.name.clone(),
                m.base_servings.to_string(),
                m.water_ml.map(|n| n.to_string()).unwrap_or_default(),
                m.notes.clone().unwrap_or_default(),
            ],
            preview: input(m.base_servings.to_string(), false, w, cx),
            preview_servings: m.base_servings,
            id,
            name: input(m.name, false, w, cx),
            servings: input(m.base_servings.to_string(), false, w, cx),
            water: input(
                m.water_ml.map(|n| n.to_string()).unwrap_or_default(),
                false,
                w,
                cx,
            ),
            notes: textarea_input(m.notes.unwrap_or_default(), w, cx),
            lines: BTreeMap::new(),
            as_weight: false,
        });
        self.sync_meal_lines(w, cx);
        cx.notify();
    }
    pub(crate) fn sync_meal_lines(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = &mut self.food.editor else {
            return;
        };
        let Some(m) = self.food.meals.iter().find(|m| m.id == e.id) else {
            self.food.editor = None;
            return;
        };
        let originals = [
            m.name.clone(),
            m.base_servings.to_string(),
            m.water_ml.map(|n| n.to_string()).unwrap_or_default(),
            m.notes.clone().unwrap_or_default(),
        ];
        for (i, state) in [&e.name, &e.servings, &e.water].into_iter().enumerate() {
            if value(state, cx) == e.originals[i] && e.originals[i] != originals[i] {
                state.update(cx, |s, cx| s.set_value(originals[i].clone(), w, cx));
            }
        }
        if e.notes.read(cx).value().as_str() == e.originals[3] && e.originals[3] != originals[3] {
            e.notes
                .update(cx, |s, cx| s.set_value(originals[3].clone(), w, cx));
        }
        e.originals = originals;
        let scale = f64::from(e.preview_servings) / f64::from(m.base_servings);
        e.lines.retain(|id, _| m.items.iter().any(|l| &l.id == id));
        for line in &m.items {
            let unit = if e.as_weight {
                "g".into()
            } else {
                line.entered_unit.clone().unwrap_or_else(|| "g".into())
            };
            let amount = if e.as_weight {
                f64::from(line.amount_g)
            } else {
                line.entered_amount
                    .map(Dec::as_f64)
                    .unwrap_or(f64::from(line.amount_g))
            } * scale;
            let text = strider_core::format::fixed(amount, 6)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned();
            let text = if text.is_empty() { "0".into() } else { text };
            if let Some(d) = e.lines.get_mut(&line.id) {
                if value(&d.amount, cx) == d.original && d.unit == d.original_unit {
                    if d.original != text {
                        d.amount
                            .update(cx, |s, cx| s.set_value(text.clone(), w, cx));
                    }
                    d.original = text;
                    d.original_unit = unit.clone();
                    d.unit = unit;
                }
            } else {
                e.lines.insert(
                    line.id.clone(),
                    LineDraft {
                        amount: input(text.clone(), false, w, cx),
                        unit: unit.clone(),
                        original: text,
                        original_unit: unit,
                    },
                );
            }
        }
    }
    pub(crate) fn meal_dirty(&self, cx: &App) -> bool {
        let Some(e) = &self.food.editor else {
            return false;
        };
        value(&e.name, cx) != e.originals[0]
            || value(&e.servings, cx) != e.originals[1]
            || value(&e.water, cx) != e.originals[2]
            || e.notes.read(cx).value().as_str() != e.originals[3]
            || e.lines
                .values()
                .any(|d| value(&d.amount, cx) != d.original || d.unit != d.original_unit)
    }
    pub(crate) fn save_meal(&mut self, cx: &mut Context<Self>) {
        self.food.new_meal = None;
        let result = (|| -> Result<Vec<Command>, String> {
            let Some(e) = &self.food.editor else {
                return Ok(vec![]);
            };
            let mut commands = vec![];
            for id in e.lines.keys() {
                if let Some(command) = self.line_command(id, cx)? {
                    commands.push(command);
                }
            }
            commands.push(Command::UpdateMeal {
                id: e.id.clone(),
                patch: MealPatch {
                    name: Some(value(&e.name, cx)),
                    base_servings: Some(integer(&e.servings, cx, true)?),
                    water_ml: Some(number(&e.water, cx)?.map(|n| n as i32)),
                    notes: Some(Some(e.notes.read(cx).value().to_string())),
                    ..Default::default()
                },
            });
            Ok(commands)
        })();
        match result {
            Ok(commands) => {
                for c in commands {
                    self.command(c, cx);
                    if self.error.is_some() {
                        return;
                    }
                }
                if let Some(e) = &mut self.food.editor {
                    for d in e.lines.values_mut() {
                        d.original = value(&d.amount, cx);
                        d.original_unit = d.unit.clone();
                    }
                    e.originals = [
                        value(&e.name, cx),
                        value(&e.servings, cx),
                        value(&e.water, cx),
                        e.notes.read(cx).value().to_string(),
                    ];
                }
            }
            Err(e) => {
                self.error = Some(e);
                cx.notify();
            }
        }
    }
    fn line_command(&self, id: &str, cx: &App) -> Result<Option<Command>, String> {
        let e = self.food.editor.as_ref().ok_or("No meal selected")?;
        let d = e.lines.get(id).ok_or("Line no longer exists")?;
        if value(&d.amount, cx) == d.original && d.unit == d.original_unit {
            return Ok(None);
        }
        let meal = self
            .food
            .meals
            .iter()
            .find(|m| m.id == e.id)
            .ok_or("Meal no longer exists")?;
        let line = meal
            .items
            .iter()
            .find(|l| l.id == id)
            .ok_or("Line no longer exists")?;
        let scale = f64::from(e.preview_servings) / f64::from(meal.base_servings);
        let typed = decimal_input(&d.amount, cx)?.ok_or("Enter a number.")?;
        let amount = if scale == 1. {
            typed
        } else {
            strider_core::format::fixed(typed.as_f64() / scale, 6)
                .parse()
                .map_err(|_| "Amount exceeds supported range")?
        };
        let grams = grams_for(amount.as_f64(), &d.unit, line.density_g_ml.map(Dec::as_f64))?;
        let entered = if e.as_weight {
            let unit = line.entered_unit.as_deref().unwrap_or("g");
            let n = display_amount(grams, unit, line.density_g_ml.map(Dec::as_f64))
                .ok_or("Set a density before entering volume.")?;
            Some((
                strider_core::format::fixed(n, 6)
                    .parse()
                    .map_err(|_| "Amount exceeds supported range")?,
                unit.to_owned(),
            ))
        } else {
            Some((amount, d.unit.clone()))
        };
        Ok(Some(Command::UpdateMealIngredientAmount {
            id: id.into(),
            amount_g: grams,
            entered,
        }))
    }
    pub(crate) fn save_line(&mut self, id: String, cx: &mut Context<Self>) {
        match self.line_command(&id, cx) {
            Ok(Some(c)) => {
                self.command(c, cx);
                if self.error.is_none()
                    && let Some(d) = self.food.editor.as_mut().and_then(|e| e.lines.get_mut(&id))
                {
                    d.original = value(&d.amount, cx);
                    d.original_unit = d.unit.clone();
                }
            }
            Ok(None) => {
                self.error = None;
                cx.notify();
            }
            Err(e) => {
                self.error = Some(e);
                cx.notify();
            }
        }
    }
    pub(crate) fn set_preview(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let result = self
            .food
            .editor
            .as_ref()
            .map(|e| integer(&e.preview, cx, true));
        match result {
            Some(Ok(n)) => {
                let ids = self
                    .food
                    .editor
                    .as_ref()
                    .map(|e| e.lines.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                for id in ids {
                    self.save_line(id, cx);
                    if self.error.is_some() {
                        return;
                    }
                }
                if let Some(e) = &mut self.food.editor {
                    e.preview_servings = n;
                    e.lines.clear();
                }
                self.sync_meal_lines(w, cx);
                cx.notify();
            }
            Some(Err(e)) => {
                self.error = Some(e);
                cx.notify();
            }
            None => {}
        }
    }
    pub(crate) fn toggle_weight_view(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        // Commit valid edits before changing representation, preserving entered units.
        let ids = self
            .food
            .editor
            .as_ref()
            .map(|e| e.lines.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for id in ids {
            self.save_line(id, cx);
            if self.error.is_some() {
                return;
            }
        }
        if let Some(e) = &mut self.food.editor {
            e.as_weight = !e.as_weight;
            e.lines.clear();
        }
        self.sync_meal_lines(w, cx);
        cx.notify();
    }
    fn reorder_line(&mut self, id: String, delta: isize, cx: &mut Context<Self>) {
        let Some(m) = self
            .food
            .editor
            .as_ref()
            .and_then(|e| self.food.meals.iter().find(|m| m.id == e.id))
        else {
            return;
        };
        let mut ordered_ids: Vec<_> = m.items.iter().map(|l| l.id.clone()).collect();
        let Some(i) = ordered_ids.iter().position(|l| l == &id) else {
            return;
        };
        let next = i as isize + delta;
        if next < 0 || next >= ordered_ids.len() as isize {
            return;
        }
        ordered_ids.swap(i, next as usize);
        self.command(
            Command::ReorderMealIngredients {
                meal: m.id.clone(),
                ordered_ids,
            },
            cx,
        );
    }
    pub(crate) fn render_meal_editor(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let e = self.food.editor.as_ref().expect("meal editor");
        let m = self
            .food
            .meals
            .iter()
            .find(|m| m.id == e.id)
            .expect("selected meal");
        let mut body = strider_ui::card(t)
            .p_5()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(div().section_heading().child(m.name.clone()))
                    .child(strider_ui::icon_button("close-meal", Icon::X, t).on_click(
                        cx.listener(|this, _, w, cx| {
                            this.request_navigation(Navigation::Meal(None), w, cx);
                        }),
                    )),
            )
            .child(strider_ui::field(
                "Meal name",
                strider_ui::text_input(&e.name, "Meal name", t),
                t,
            ));
        body = body.child(
            div()
                .flex()
                .gap_3()
                .child(strider_ui::field(
                    "Recipe yields",
                    strider_ui::text_input(&e.servings, "Base servings", t),
                    t,
                ))
                .child(strider_ui::field(
                    "Type",
                    self.choice(
                        "meal-type",
                        m.meal_type.clone().unwrap_or_else(|| "—".into()),
                        ["", "breakfast", "lunch", "dinner", "snack"]
                            .into_iter()
                            .map(|s| (s.to_owned(), if s.is_empty() { "—" } else { s }.into()))
                            .collect(),
                        cx,
                        |this, v, _, cx| {
                            if let Some(e) = &this.food.editor {
                                this.command(
                                    Command::UpdateMeal {
                                        id: e.id.clone(),
                                        patch: MealPatch {
                                            meal_type: Some((!v.is_empty()).then_some(v)),
                                            ..Default::default()
                                        },
                                    },
                                    cx,
                                );
                            }
                        },
                    ),
                    t,
                )),
        );
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    strider_ui::checkbox("hot-meal", "Hot meal", m.is_hot, t).on_click(
                        cx.listener(|this, _, _, cx| {
                            if let Some(m) = this
                                .food
                                .editor
                                .as_ref()
                                .and_then(|e| this.food.meals.iter().find(|m| m.id == e.id))
                            {
                                this.command(
                                    Command::UpdateMeal {
                                        id: m.id.clone(),
                                        patch: MealPatch {
                                            is_hot: Some(!m.is_hot),
                                            water_ml: Some(if m.is_hot {
                                                None
                                            } else {
                                                Some(m.water_ml.unwrap_or(500))
                                            }),
                                            ..Default::default()
                                        },
                                    },
                                    cx,
                                );
                            }
                        }),
                    ),
                )
                .child("Hot meal")
                .child(strider_ui::info_badge(
                    "hot-info",
                    "Hot meals need a stove and drive the trip fuel calculation.",
                    t,
                )),
        );
        if m.is_hot {
            body = body.child(strider_ui::field(
                "Cook water (mL)",
                strider_ui::text_input(&e.water, "Cook water in mL", t),
                t,
            ));
        }
        let efficiency = if m.total_g > 0 {
            m.total_kcal as f64 / m.total_g as f64 * strider_core::OZ_TO_G
        } else {
            0.
        };
        body = body
            .child(div().readout().meta(t).child(format!(
                "Recipe: {} kcal · {:.1} oz · {:.0} cal/oz",
                m.total_kcal,
                m.total_g as f64 / strider_core::OZ_TO_G,
                efficiency
            )))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        strider_ui::checkbox("view-weight", "View as weight", e.as_weight, t)
                            .on_click(cx.listener(|this, _, w, cx| this.toggle_weight_view(w, cx))),
                    )
                    .child("View as weight"),
            );
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().meta(t).child("Show amounts for"))
                .child(div().w(px(72.)).child(strider_ui::text_input(
                    &e.preview,
                    "Preview servings",
                    t,
                )))
                .child("servings")
                .child(
                    strider_ui::button("preview-servings", "Show", ButtonVariant::Ghost, t)
                        .on_click(cx.listener(|this, _, w, cx| this.set_preview(w, cx))),
                ),
        );
        for (i, line) in m.items.iter().enumerate() {
            if let Some(draft) = e.lines.get(&line.id) {
                body = body.child(self.render_line(i, line, draft, cx));
            }
        }
        let id = m.id.clone();
        body = body
            .child(strider_ui::field(
                "Notes",
                strider_ui::textarea(&e.notes, "Meal notes", t),
                t,
            ))
            .child(self.render_reference_search(true, cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        strider_ui::button("save-meal", "Save", ButtonVariant::Primary, t)
                            .on_click(cx.listener(|this, _, _, cx| this.save_meal(cx))),
                    )
                    .child(
                        strider_ui::button("delete-meal", "Delete meal", ButtonVariant::Danger, t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.command(Command::DeleteMeal { id: id.clone() }, cx);
                                if this.error.is_none() {
                                    this.food.editor = None;
                                }
                            })),
                    ),
            );
        body.into_any_element()
    }
    fn render_line(
        &self,
        index: usize,
        line: &MealIngredientRow,
        draft: &LineDraft,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        let id = line.id.clone();
        let amount_id = id.clone();
        let save = id.clone();
        let remove = id.clone();
        let up = id.clone();
        let down = id.clone();
        let ingredient = line.ingredient_id.clone();
        let options = if self.food.editor.as_ref().is_some_and(|e| e.as_weight) {
            vec![("g".into(), "g".into()), ("oz".into(), "oz".into())]
        } else {
            ["g", "oz", "tsp", "tbsp", "cup", "floz", "ml", "l"]
                .into_iter()
                .map(|s| (s.into(), if s == "floz" { "fl oz" } else { s }.into()))
                .collect()
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .border_b_1()
            .border_color(t.divider)
            .py_2()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(line.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .items_center()
                    .child(div().w(px(80.)).child(strider_ui::text_input(
                        &draft.amount,
                        "Ingredient amount",
                        t,
                    )))
                    .child(
                        self.choice(
                            ("line-unit", index),
                            if draft.unit == "floz" {
                                "fl oz"
                            } else {
                                &draft.unit
                            }
                            .to_owned(),
                            options,
                            cx,
                            move |this, v, w, cx| {
                                let line = this
                                    .food
                                    .meals
                                    .iter()
                                    .flat_map(|m| &m.items)
                                    .find(|l| l.id == amount_id)
                                    .cloned();
                                if let Some(line) = line {
                                    let scale = this
                                        .food
                                        .editor
                                        .as_ref()
                                        .and_then(|e| {
                                            this.food.meals.iter().find(|m| m.id == e.id).map(|m| {
                                                f64::from(e.preview_servings)
                                                    / f64::from(m.base_servings)
                                            })
                                        })
                                        .unwrap_or(1.);
                                    let n = display_amount(
                                        line.amount_g,
                                        &v,
                                        line.density_g_ml.map(Dec::as_f64),
                                    )
                                    .map(|n| n * scale);
                                    if let Some(d) = this
                                        .food
                                        .editor
                                        .as_mut()
                                        .and_then(|e| e.lines.get_mut(&amount_id))
                                    {
                                        let typed = value(&d.amount, cx) != d.original;
                                        d.unit = v;
                                        if let Some(n) = n.filter(|_| !typed) {
                                            d.amount.update(cx, |s, cx| {
                                                s.set_value(n.to_string(), w, cx)
                                            });
                                        }
                                    }
                                }
                                cx.notify();
                            },
                        ),
                    )
                    .child(
                        strider_ui::button(("save-line", index), "Set", ButtonVariant::Outline, t)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.save_line(save.clone(), cx)),
                            ),
                    )
                    .child(div().readout().meta(t).child(format!(
                        "{} g · {} kcal",
                        line.amount_g,
                        food::kcal_for_grams(
                            line.kcal_per_100g.map(Dec::as_f64),
                            f64::from(line.amount_g)
                        )
                    )))
                    .child(
                        strider_ui::button(("line-up", index), "↑", ButtonVariant::Ghost, t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.reorder_line(up.clone(), -1, cx)
                            })),
                    )
                    .child(
                        strider_ui::button(("line-down", index), "↓", ButtonVariant::Ghost, t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.reorder_line(down.clone(), 1, cx)
                            })),
                    )
                    .child(
                        strider_ui::icon_button(("remove-line", index), Icon::X, t)
                            .aria_label("Remove ingredient line")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.command(
                                    Command::RemoveMealIngredient { id: remove.clone() },
                                    cx,
                                )
                            })),
                    )
                    .when_some(ingredient, |d, ingredient| {
                        d.child(
                            strider_ui::button(
                                ("measure-line", index),
                                "Measure / link",
                                ButtonVariant::Ghost,
                                t,
                            )
                            .on_click(cx.listener(
                                move |this, _, w, cx| {
                                    this.open_ingredient(Some(ingredient.clone()), w, cx)
                                },
                            )),
                        )
                    }),
            )
            .into_any_element()
    }
}
fn grams_for(amount: f64, unit: &str, density: Option<f64>) -> Result<i32, String> {
    let grams = match unit {
        "g" => strider_core::format::js_round(amount),
        "oz" => strider_core::format::js_round(amount * strider_core::OZ_TO_G),
        _ => food::grams_from_volume(amount, unit, density)
            .ok_or("Set a density before entering volume.")? as f64,
    };
    if !grams.is_finite() || grams < 0. || grams > i32::MAX as f64 {
        return Err("Amount exceeds supported range".into());
    }
    Ok(grams as i32)
}
fn display_amount(grams: i32, unit: &str, density: Option<f64>) -> Option<f64> {
    match unit {
        "g" => Some(f64::from(grams)),
        "oz" => Some(g_to_oz(strider_core::Grams(grams))),
        _ => food::volume_from_grams(f64::from(grams), unit, density),
    }
}
