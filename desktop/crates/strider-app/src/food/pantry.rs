use super::*;
impl AppView {
    pub(crate) fn render_pantry(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let mut list = strider_ui::card(t).child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .px_4()
                .py_2()
                .bg(t.panel2)
                .child(div().flex_1().meta(t).child("Ingredient"))
                .children(
                    ["kcal/100g", "cal/oz", "g/tbsp"]
                        .into_iter()
                        .map(|s| div().w(px(80.)).text_right().meta(t).child(s)),
                )
                .child(div().w(px(76.))),
        );
        for (i, r) in self.food.ingredients.iter().enumerate() {
            let edit = r.id.clone();
            let delete = r.id.clone();
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .border_b_1()
                    .border_color(t.divider)
                    .child(
                        strider_ui::button(
                            ("ingredient", i),
                            r.input.name.clone(),
                            ButtonVariant::Ghost,
                            t,
                        )
                        .flex_1()
                        .justify_start()
                        .when_some(r.input.category.clone(), |d, c| {
                            d.child(div().meta(t).child(c))
                        })
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.open_ingredient(Some(edit.clone()), w, cx)
                        })),
                    )
                    .children(
                        [
                            r.input.kcal_per_100g.map(|d| format!("{:.0}", d.as_f64())),
                            strider_core::food::kcal_per_oz(
                                r.input.kcal_per_100g.map(|d| d.as_f64()),
                            )
                            .map(|v| format!("{v:.0}")),
                            r.input
                                .density_g_ml
                                .map(|d| format!("{:.1}", d.as_f64() * 14.7868)),
                        ]
                        .into_iter()
                        .map(|v| {
                            div()
                                .w(px(80.))
                                .readout()
                                .text_right()
                                .meta(t)
                                .child(v.unwrap_or_else(|| "—".into()))
                        }),
                    )
                    .child(
                        strider_ui::button(
                            ("delete-ingredient", i),
                            "Delete",
                            ButtonVariant::Danger,
                            t,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.command(Command::DeleteIngredient { id: delete.clone() }, cx)
                        })),
                    ),
            );
        }
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.render_reference_search(false, cx));
        if self.food.ingredients.is_empty() {
            body = body.child(div().meta(t).child("No saved ingredients."));
        } else {
            body = body.child(list);
        }
        body.into_any_element()
    }
}
