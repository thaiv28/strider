use super::*;
pub(super) mod ingredient_dialog;
pub(super) mod meal_editor;
mod meals;
mod pantry;
mod reference_search;
use strider_store::{IngredientRow, MealRow};
pub(crate) struct FoodState {
    pub(crate) pantry: bool,
    pub(crate) new_meal: Option<String>,
    pub(crate) ingredients: Vec<IngredientRow>,
    pub(crate) meals: Vec<MealRow>,
    pub(crate) editor: Option<MealEditor>,
    pub(crate) ingredient_editor: Option<IngredientEditor>,
    pub(crate) search: Entity<InputState>,
}
impl FoodState {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<AppView>) -> Self {
        Self {
            pantry: false,
            new_meal: None,
            ingredients: vec![],
            meals: vec![],
            editor: None,
            ingredient_editor: None,
            search: input("", false, w, cx),
        }
    }
}
impl AppView {
    pub(crate) fn render_food(&mut self, w: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let body = if self.food.pantry {
            self.render_pantry(cx)
        } else {
            self.render_meals(w, cx)
        };
        strider_ui::page_body(
            vec![
                div()
                    .id("food-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body)
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
}

impl AppView {
    pub(crate) fn request_navigation(
        &mut self,
        next: Navigation,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.meal_dirty(cx) {
            self.pending_navigation = Some(next);
            cx.notify();
        } else {
            self.navigate(next, w, cx);
        }
    }
    fn navigate(&mut self, next: Navigation, w: &mut Window, cx: &mut Context<Self>) {
        self.pending_navigation = None;
        if let Some(id) = self.food.new_meal.take()
            && self.food.meals.iter().any(|m| {
                m.id == id
                    && m.items.is_empty()
                    && m.name == "New meal"
                    && m.meal_type.is_none()
                    && !m.is_hot
                    && m.notes.is_none()
            })
        {
            self.command(Command::DeleteMeal { id }, cx);
            if self.error.is_some() {
                return;
            }
        }
        self.food.editor = None;
        match next {
            Navigation::Area(area) => {
                self.area = area;
                self.trips.selected = None;
                // Calendar and Settings read their data fresh on every visit.
                self.cal.data = None;
                self.settings = None;
            }
            Navigation::Pantry(pantry) => self.food.pantry = pantry,
            Navigation::Meal(Some(id)) => self.open_meal(id, w, cx),
            Navigation::Meal(None) => {}
            Navigation::CreateMeal => self.create_meal(w, cx),
        }
        self.focus.focus(w, cx);
        cx.notify();
    }
    pub(crate) fn render_navigation_guard(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child("Save changes to this meal before leaving?")
            .when_some(self.error.clone(), |d, e| {
                d.child(div().text_color(t.signal).child(e))
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        strider_ui::button(
                            "save-and-leave",
                            "Save and leave",
                            ButtonVariant::Primary,
                            t,
                        )
                        .on_click(cx.listener(|this, _, w, cx| {
                            this.save_meal(cx);
                            if this.error.is_none() {
                                this.food.new_meal = None;
                                if let Some(next) = this.pending_navigation.clone() {
                                    this.navigate(next, w, cx);
                                }
                            }
                        })),
                    )
                    .child(
                        strider_ui::button(
                            "discard-meal-edits",
                            "Discard",
                            ButtonVariant::Danger,
                            t,
                        )
                        .on_click(cx.listener(|this, _, w, cx| {
                            if let Some(next) = this.pending_navigation.clone() {
                                this.navigate(next, w, cx);
                            }
                        })),
                    )
                    .child(
                        strider_ui::button("keep-editing", "Keep editing", ButtonVariant::Ghost, t)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pending_navigation = None;
                                cx.notify();
                            })),
                    ),
            );
        self.modal("Unsaved changes", body.into_any_element(), cx)
    }
}
