use super::*;
impl AppView {
    pub(crate) fn create_meal(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let before: HashSet<_> = self.food.meals.iter().map(|m| m.id.clone()).collect();
        self.command(
            Command::CreateMeal {
                name: "New meal".into(),
            },
            cx,
        );
        if self.error.is_none()
            && let Some(id) = self
                .food
                .meals
                .iter()
                .find(|m| !before.contains(&m.id))
                .map(|m| m.id.clone())
        {
            self.open_meal(id.clone(), w, cx);
            self.food.new_meal = Some(id);
        }
    }
    pub(crate) fn add_pantry_line(&mut self, ingredient: String, cx: &mut Context<Self>) {
        let Some(meal) = self.food.editor.as_ref().map(|e| e.id.clone()) else {
            return;
        };
        let amount_g = self
            .food
            .ingredients
            .iter()
            .find(|i| i.id == ingredient)
            .and_then(|i| i.input.default_serving_g)
            .unwrap_or(50);
        self.command(
            Command::AddMealIngredient {
                meal,
                ingredient,
                amount_g,
            },
            cx,
        );
    }
    pub(crate) fn render_meals(&mut self, w: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        self.sync_meal_lines(w, cx);
        let wide = w.viewport_size().width >= px(1100.);
        let mut list = div()
            .gap_3()
            .flex_1()
            .min_w_0()
            .when(self.food.editor.is_none(), |d| d.grid().grid_cols(2))
            .when(self.food.editor.is_some(), |d| d.flex().flex_col());
        for (i, m) in self.food.meals.iter().enumerate() {
            let id = m.id.clone();
            let selected = self.food.editor.as_ref().is_some_and(|e| e.id == m.id);
            let efficiency = if m.total_g > 0 {
                m.total_kcal as f64 / m.total_g as f64 * strider_core::OZ_TO_G
            } else {
                0.
            };
            list = list.child(
                strider_ui::card(t)
                    .p_4()
                    .id(("meal", i))
                    .role(Role::Button)
                    .focusable()
                    .cursor_pointer()
                    .when(selected, |d| d.border_1().border_color(t.accent))
                    .child(div().font_weight(FontWeight::BOLD).child(m.name.clone()))
                    .child(div().meta(t).readout().child(format!(
                        "{} kcal/serving · {:.1} oz · {:.0} cal/oz",
                        strider_core::format::js_round(
                            m.total_kcal as f64 / f64::from(m.base_servings)
                        ) as i64,
                        m.total_g as f64 / f64::from(m.base_servings) / strider_core::OZ_TO_G,
                        efficiency
                    )))
                    .child(div().meta(t).child(format!(
                            "{} ingredients · yields {}{}",
                            m.items.len(),
                            m.base_servings,
                            m.meal_type
                                .as_ref()
                                .map(|s| format!(" · {s}"))
                                .unwrap_or_default()
                        )))
                    .on_click(cx.listener(move |this, _, w, cx| {
                        this.request_navigation(Navigation::Meal(Some(id.clone())), w, cx)
                    })),
            );
        }
        if self.food.meals.is_empty() {
            list = list.child(div().meta(t).child("No meals yet."));
        }
        if self.food.editor.is_some() {
            let editor = self.render_meal_editor(cx);
            if wide {
                div()
                    .flex()
                    .items_start()
                    .gap_4()
                    .child(list)
                    .child(div().w(px(460.)).flex_shrink_0().child(editor))
                    .into_any_element()
            } else {
                editor
            }
        } else {
            list.into_any_element()
        }
    }
}
