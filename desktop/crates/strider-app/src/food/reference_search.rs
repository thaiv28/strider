use super::*;
use strider_store::{IngredientInput, reference::ReferenceFood};
impl AppView {
    pub(crate) fn add_reference(
        &mut self,
        food: ReferenceFood,
        to_meal: bool,
        cx: &mut Context<Self>,
    ) {
        let before: HashSet<_> = self.food.ingredients.iter().map(|i| i.id.clone()).collect();
        let dec = |n: f64| strider_core::format::fixed(n, 6).parse().ok();
        self.command(
            Command::CreateIngredient {
                input: IngredientInput {
                    name: food.name,
                    kcal_per_100g: food.kcal_per_100g.and_then(dec),
                    density_g_ml: food.density_g_ml.and_then(dec),
                    density_source: food.density_g_ml.map(|_| "usda".into()),
                    reference_name: Some(food.reference_name),
                    source: Some("usda".into()),
                    source_id: Some(food.id.to_string()),
                    ..Default::default()
                },
            },
            cx,
        );
        if self.error.is_none()
            && to_meal
            && let Some(ingredient) = self
                .food
                .ingredients
                .iter()
                .find(|i| !before.contains(&i.id))
                .map(|i| i.id.clone())
        {
            self.add_pantry_line(ingredient, cx);
        }
    }
    pub(crate) fn render_reference_search(&self, to_meal: bool, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let query = value(&self.food.search, cx);
        let mut body = div().flex().flex_col().gap_2().child(strider_ui::field(
            if to_meal {
                "Add ingredient"
            } else {
                "Food search"
            },
            strider_ui::text_input(&self.food.search, "Search foods", t),
            t,
        ));
        if to_meal && !query.trim().is_empty() {
            for (i, ingredient) in self
                .food
                .ingredients
                .iter()
                .filter(|i| i.input.name.to_lowercase().contains(&query.to_lowercase()))
                .take(6)
                .enumerate()
            {
                let id = ingredient.id.clone();
                body = body.child(
                    strider_ui::button(
                        ("saved-food", i),
                        ingredient.input.name.clone(),
                        ButtonVariant::Ghost,
                        t,
                    )
                    .justify_start()
                    .child(strider_ui::badge("saved", "neutral", t))
                    .on_click(cx.listener(move |this, _, w, cx| {
                        this.add_pantry_line(id.clone(), cx);
                        this.food.search.update(cx, |s, cx| s.set_value("", w, cx));
                    })),
                );
            }
        }
        for (i, hit) in strider_store::reference::search_reference_foods(&query, 12, 0.5)
            .into_iter()
            .enumerate()
        {
            let label = format!(
                "{} · {} kcal/100g",
                hit.name,
                hit.kcal_per_100g
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".into())
            );
            body = body.child(
                strider_ui::button(("reference", i), label, ButtonVariant::Ghost, t)
                    .justify_start()
                    .on_click(cx.listener(move |this, _, w, cx| {
                        this.add_reference(hit.clone(), to_meal, cx);
                        this.food.search.update(cx, |s, cx| s.set_value("", w, cx));
                    })),
            );
        }
        body.child(
            strider_ui::button(
                "packaged-search",
                "Search packaged products",
                ButtonVariant::Ghost,
                t,
            )
            .tab_index(-1)
            .opacity(0.5)
            .aria_label("Packaged search unavailable offline"),
        )
        .child(div().meta(t).child("Packaged search needs a connection."))
        .into_any_element()
    }
}
