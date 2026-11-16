use super::*;
use gpui_component::input::TextareaState;
use strider_store::IngredientInput;
pub(crate) struct IngredientEditor {
    pub(crate) id: Option<String>,
    pub(crate) name: Entity<InputState>,
    pub(crate) kcal: Entity<InputState>,
    pub(crate) density: Entity<InputState>,
    pub(crate) density_source: Entity<InputState>,
    pub(crate) serving: Entity<InputState>,
    pub(crate) category: Entity<InputState>,
    pub(crate) notes: Entity<TextareaState>,
    pub(crate) measure: Entity<InputState>,
    pub(crate) measure_unit: String,
}
impl AppView {
    pub(crate) fn open_ingredient(
        &mut self,
        id: Option<String>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reload(cx);
        let row = id
            .as_ref()
            .and_then(|id| self.food.ingredients.iter().find(|r| &r.id == id));
        if id.is_some() && row.is_none() {
            self.error = Some("Ingredient no longer exists".into());
            cx.notify();
            return;
        }
        let data = row.map(|r| r.input.clone()).unwrap_or_default();
        self.food.ingredient_editor = Some(IngredientEditor {
            id,
            name: input(data.name, false, w, cx),
            kcal: input(
                data.kcal_per_100g
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            density: input(
                data.density_g_ml.map(|d| d.to_string()).unwrap_or_default(),
                false,
                w,
                cx,
            ),
            density_source: input(data.density_source.unwrap_or_default(), false, w, cx),
            serving: input(
                data.default_serving_g
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            category: input(data.category.unwrap_or_default(), false, w, cx),
            notes: textarea_input(data.notes.unwrap_or_default(), w, cx),
            measure: input("", false, w, cx),
            measure_unit: "tbsp".into(),
        });
        cx.notify();
    }
    pub(crate) fn save_ingredient(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = &self.food.ingredient_editor else {
            return;
        };
        let result = (|| -> Result<Command, String> {
            let density = decimal_input(&e.density, cx)?;
            let existing =
                e.id.as_ref()
                    .and_then(|id| self.food.ingredients.iter().find(|r| &r.id == id))
                    .map(|r| r.input.clone())
                    .unwrap_or_default();
            let name = value(&e.name, cx);
            if name.trim().is_empty() {
                return Err("Name is required".into());
            }
            let input = IngredientInput {
                name,
                kcal_per_100g: decimal_input(&e.kcal, cx)?,
                density_g_ml: density,
                density_source: optional(&e.density_source, cx),
                default_serving_g: number(&e.serving, cx)?.map(|n| n as i32),
                category: optional(&e.category, cx),
                notes: Some(e.notes.read(cx).value().to_string()),
                ..existing
            };
            Ok(if let Some(id) = e.id.clone() {
                Command::UpdateIngredient { id, input }
            } else {
                Command::CreateIngredient { input }
            })
        })();
        match result {
            Ok(command) => {
                self.command(command, cx);
                if self.error.is_none() {
                    self.food.ingredient_editor = None;
                    self.focus.focus(w, cx);
                }
            }
            Err(e) => {
                self.error = Some(e);
                cx.notify();
            }
        }
    }
    fn measure_ingredient(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = &self.food.ingredient_editor else {
            return;
        };
        let Some(id) = e.id.clone() else { return };
        match decimal_input(&e.measure, cx).and_then(|v| v.ok_or_else(|| "Enter a number.".into()))
        {
            Ok(grams) => {
                self.command(
                    Command::SetIngredientMeasure {
                        id: id.clone(),
                        unit: e.measure_unit.clone(),
                        grams,
                    },
                    cx,
                );
                if self.error.is_none() {
                    self.refresh_ingredient_density(&id, w, cx);
                }
            }
            Err(e) => {
                self.error = Some(e);
                cx.notify();
            }
        }
    }
    fn refresh_ingredient_density(&mut self, id: &str, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(data) = self
            .food
            .ingredients
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.input.clone())
            && let Some(e) = &self.food.ingredient_editor
        {
            e.density.update(cx, |s, cx| {
                s.set_value(
                    data.density_g_ml.map(|d| d.to_string()).unwrap_or_default(),
                    w,
                    cx,
                )
            });
            e.density_source.update(cx, |s, cx| {
                s.set_value(data.density_source.unwrap_or_default(), w, cx)
            });
        }
    }
    pub(crate) fn render_ingredient_dialog(
        &self,
        _w: &mut Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        let e = self
            .food
            .ingredient_editor
            .as_ref()
            .expect("ingredient editor");
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_color(t.signal).child(e))
            })
            .child(strider_ui::field(
                "Name",
                strider_ui::text_input(&e.name, "Ingredient name", t),
                t,
            ))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(strider_ui::field(
                        "kcal / 100g",
                        strider_ui::text_input(&e.kcal, "kcal per 100 grams", t),
                        t,
                    ))
                    .child(strider_ui::field(
                        "Serving (g)",
                        strider_ui::text_input(&e.serving, "Default serving grams", t),
                        t,
                    )),
            )
            .child(strider_ui::field(
                "Density (g/mL)",
                strider_ui::text_input(&e.density, "Density grams per mL", t),
                t,
            ))
            .child(strider_ui::field(
                "Density source",
                strider_ui::text_input(&e.density_source, "Density source", t),
                t,
            ))
            .child(strider_ui::field(
                "Category",
                strider_ui::text_input(&e.category, "Ingredient category", t),
                t,
            ))
            .child(strider_ui::field(
                "Notes",
                strider_ui::textarea(&e.notes, "Ingredient notes", t),
                t,
            ));
        if let Some(id) = e.id.clone() {
            body = body.child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child("1")
                    .child(
                        self.choice(
                            "measure-unit",
                            e.measure_unit.clone(),
                            ["tsp", "tbsp", "cup", "floz", "ml", "l"]
                                .into_iter()
                                .map(|s| (s.into(), if s == "floz" { "fl oz" } else { s }.into()))
                                .collect(),
                            cx,
                            |this, v, _, cx| {
                                if let Some(e) = &mut this.food.ingredient_editor {
                                    e.measure_unit = v;
                                }
                                cx.notify();
                            },
                        ),
                    )
                    .child("=")
                    .child(div().flex_1().child(strider_ui::text_input(
                        &e.measure,
                        "Measure weight grams",
                        t,
                    )))
                    .child("g")
                    .child(
                        strider_ui::button("set-measure", "Set", ButtonVariant::Outline, t)
                            .on_click(cx.listener(|this, _, w, cx| this.measure_ingredient(w, cx))),
                    ),
            );
            let name = value(&e.name, cx);
            for (i, hit) in strider_store::reference::match_reference_foods(&name, 4)
                .into_iter()
                .enumerate()
            {
                let id = id.clone();
                body = body.child(
                    strider_ui::button(
                        ("link-reference", i),
                        format!("Link: {}", hit.reference_name),
                        ButtonVariant::Ghost,
                        t,
                    )
                    .justify_start()
                    .on_click(cx.listener(move |this, _, w, cx| {
                        let density = hit
                            .density_g_ml
                            .and_then(|n| strider_core::format::fixed(n, 6).parse().ok());
                        if let Some(density) = density {
                            this.command(
                                Command::LinkIngredientReference {
                                    id: id.clone(),
                                    density,
                                    reference_name: Some(hit.reference_name.clone()),
                                },
                                cx,
                            );
                            if this.error.is_none() {
                                this.refresh_ingredient_density(&id, w, cx);
                            }
                        }
                    })),
                );
            }
            if let Some(reference) = self
                .food
                .ingredients
                .iter()
                .find(|r| r.id == id)
                .and_then(|r| r.input.reference_name.clone())
            {
                body = body.child(div().meta(t).child(format!("USDA: {reference}")));
            }
        }
        body = body.child(
            div()
                .flex()
                .gap_2()
                .child(
                    strider_ui::button("save-ingredient", "Save", ButtonVariant::Primary, t)
                        .on_click(cx.listener(|this, _, w, cx| this.save_ingredient(w, cx))),
                )
                .child(
                    strider_ui::button("cancel-ingredient", "Cancel", ButtonVariant::Ghost, t)
                        .on_click(cx.listener(|this, _, w, cx| this.close_overlays(w, cx))),
                ),
        );
        self.modal(
            if e.id.is_some() {
                "Edit ingredient"
            } else {
                "Add ingredient"
            },
            body.into_any_element(),
            cx,
        )
    }
}
