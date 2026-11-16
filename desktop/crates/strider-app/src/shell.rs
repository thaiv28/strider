use super::*;
impl AppView {
    pub(crate) fn render_shell(&self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx);
        div()
            .relative()
            .h(px(64.))
            .flex_shrink_0()
            .bg(t.header)
            .child(strider_ui::contour_band(t))
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .items_center()
                    .pl(px(84.))
                    .gap_1p5()
                    .child(
                        div()
                            .flex_shrink_0()
                            .section_heading()
                            .id("Basecamp")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.request_navigation(Navigation::Area("Basecamp"), w, cx);
                            }))
                            .text_color(t.header_ink)
                            .child("Strider"),
                    )
                    .children(
                        ["Gear", "Food", "Trips", "Calendar", "Settings"]
                            .into_iter()
                            .map(|area| {
                                div()
                                    .id(area)
                                    .min_h(px(44.))
                                    .px_2()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .text_size(px(15.))
                                    .rounded(px(12.))
                                    .text_color(if area == self.area {
                                        t.header_ink
                                    } else {
                                        t.header_muted
                                    })
                                    .when(area == self.area, |d| {
                                        d.bg(t.header_active).font_weight(FontWeight::BOLD)
                                    })
                                    .cursor_pointer()
                                    .child(area)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.request_navigation(Navigation::Area(area), window, cx);
                                    }))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            }),
                    ),
            )
    }
}
impl AppView {
    pub(crate) fn render_header(&self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx);
        let actions = match self.area {
            "Gear" => strider_ui::button("add-gear", "Add gear", ButtonVariant::Header, t)
                .on_click(cx.listener(|this, _, w, cx| this.add_gear(w, cx)))
                .into_any_element(),
            "Trips" | "Basecamp" => self.trip_header_actions(cx),
            "Calendar" => self.calendar_header_actions(cx),
            "Food" => strider_ui::button(
                "add-food",
                if self.food.pantry {
                    "Add ingredient"
                } else {
                    "Add meal"
                },
                ButtonVariant::Header,
                t,
            )
            .on_click(cx.listener(|this, _, w, cx| {
                if this.food.pantry {
                    this.open_ingredient(None, w, cx);
                } else {
                    this.request_navigation(Navigation::CreateMeal, w, cx);
                }
            }))
            .into_any_element(),
            _ => div().into_any_element(),
        };
        let tabs = if self.area == "Gear" {
            strider_ui::page_tabs(
                [
                    ("Inventory".to_owned(), false),
                    (
                        format!(
                            "Wishlist ({})",
                            self.all_gear
                                .iter()
                                .filter(|r| r.status == "wishlist")
                                .count()
                        ),
                        true,
                    ),
                ]
                .into_iter()
                .map(|(label, wishlist)| {
                    strider_ui::page_tab(
                        if wishlist {
                            "wishlist-tab"
                        } else {
                            "inventory-tab"
                        },
                        label,
                        self.wishlist == wishlist,
                        true,
                        t,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.wishlist = wishlist;
                        cx.notify();
                    }))
                    .into_any_element()
                }),
            )
            .into_any_element()
        } else if self.area == "Food" {
            strider_ui::page_tabs([("Meals", false), ("Ingredients", true)].into_iter().map(
                |(label, pantry)| {
                    strider_ui::page_tab(label, label, self.food.pantry == pantry, true, t)
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.request_navigation(Navigation::Pantry(pantry), w, cx);
                        }))
                        .into_any_element()
                },
            ))
            .into_any_element()
        } else {
            div().into_any_element()
        };
        let tabs = div()
            .child(tabs)
            .when_some(self.sync_status(cx), |d, status| {
                d.child(
                    strider_ui::button("sync-status", status, ButtonVariant::Header, t).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.account.sheet = true;
                            cx.notify();
                        }),
                    ),
                )
            });
        let (title, summary) = if let Some(d) = &self.trips.detail {
            (
                trips::text(&d.fields, "name"),
                div()
                    .readout()
                    .child(format!(
                        "{} · {} nights · {:.1} mi · {} ft gain · Pack {} · Skin-out {}",
                        trips::text(&d.fields, "start_date"),
                        trips::text(&d.fields, "nights"),
                        d.route.distance_mi.unwrap_or(0.),
                        d.route.elevation_ft.unwrap_or(0),
                        fmt_weight(Some(strider_core::units::Grams(d.weights.pack as i32))),
                        fmt_weight(Some(strider_core::units::Grams(d.weights.skin_out as i32)))
                    ))
                    .into_any_element(),
            )
        } else if self.area == "Calendar" {
            (
                self.area.to_owned(),
                div()
                    .text_color(t.header_muted)
                    .child(self.calendar_label())
                    .into_any_element(),
            )
        } else {
            (self.area.to_owned(), tabs.into_any_element())
        };
        strider_ui::page_header(title, actions, summary, t)
    }
}
