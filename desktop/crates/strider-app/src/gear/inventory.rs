use super::*;
use strider_core::gear::{self, Membership, Status, WeightClass};
impl AppView {
    pub(crate) fn core_rows(&self) -> Vec<gear::GearRow> {
        self.all_gear
            .iter()
            .map(|r| gear::GearRow {
                id: r.id.clone(),
                name: r.name.clone(),
                category: r.category_id.clone(),
                weight_g: r.weight.map(|g| i64::from(g.0)),
                quantity: i64::from(r.quantity),
                price_cents: r.price_cents.map(|p| i64::from(p.0)),
                default_weight_class: class(&r.class),
                status: match r.status.as_str() {
                    "retired" => Status::Retired,
                    "wishlist" => Status::Wishlist,
                    _ => Status::Current,
                },
                replaces: r.replaces.clone(),
            })
            .collect()
    }
    pub(crate) fn membership(&self, id: &str) -> Membership {
        self.loadouts
            .membership
            .get(id)
            .into_iter()
            .flat_map(|m| m.iter())
            .map(|(id, c)| (id.clone(), class(c)))
            .collect()
    }
    pub(crate) fn member_class(&self, item: &str) -> Option<String> {
        self.loadout
            .as_ref()
            .and_then(|id| self.loadouts.membership.get(id))
            .and_then(|m| m.get(item))
            .cloned()
    }
    pub(crate) fn refresh_subtotals(&mut self) -> strider_store::Result<()> {
        let membership = self.loadout.as_ref().map(|id| self.membership(id));
        let sums = gear::category_subtotals(&self.core_rows(), membership.as_ref());
        self.totals = Totals::default();
        for (category, s) in &sums {
            let grams = |g: i64| {
                i32::try_from(g)
                    .map(strider_core::Grams)
                    .map_err(|_| strider_store::Error::Invalid("Weight subtotal overflow".into()))
            };
            self.totals.base.0 = grams(i64::from(self.totals.base.0) + s.base_g)?.0;
            self.totals.worn.0 = grams(i64::from(self.totals.worn.0) + s.worn_g)?.0;
            self.totals.consumable.0 =
                grams(i64::from(self.totals.consumable.0) + s.consumable_g)?.0;
            if s.base_g > 0 {
                self.totals.by_category.push(strider_store::CategorySlice {
                    name: self
                        .categories
                        .iter()
                        .find(|c| Some(&c.id) == category.as_ref())
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "Other".into()),
                    g: grams(s.base_g)?,
                    items: s
                        .base_items
                        .iter()
                        .map(|i| {
                            Ok(strider_store::SliceItem {
                                name: i.name.clone(),
                                g: grams(i.grams)?,
                                quantity: i.quantity as i32,
                            })
                        })
                        .collect::<strider_store::Result<Vec<_>>>()?,
                });
            }
            for group in &mut self.groups {
                if &group.id == category {
                    group.base = grams(s.base_g)?;
                    group.worn = grams(s.worn_g)?;
                    group.consumable = grams(s.consumable_g)?;
                }
            }
        }
        self.totals.items = self
            .all_gear
            .iter()
            .filter(|r| r.status == "current")
            .count();
        Ok(())
    }
    pub(crate) fn toggle_member(
        &mut self,
        item: String,
        present: bool,
        class: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(loadout) = self.loadout.clone() {
            self.command(
                Command::SetLoadoutMember {
                    loadout,
                    item,
                    present,
                    class,
                },
                cx,
            );
        }
    }
    pub(crate) fn render_inventory(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        let toolbar = div()
            .flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().child(strider_ui::text_input(
                &self.filter_input,
                "Search gear",
                t,
            )))
            .child(
                strider_ui::button("more-options", "More", ButtonVariant::Outline, t).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.more = !this.more;
                        cx.notify();
                    }),
                ),
            );
        let mut left = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .child(toolbar);
        if self.more {
            let options = vec![
                ("current".into(), "Current".into()),
                ("retired".into(), "Retired".into()),
                ("all".into(), "All".into()),
            ];
            let label = match self.filter.status {
                StatusFilter::Current => "Current",
                StatusFilter::Retired => "Retired",
                StatusFilter::All => "All",
            };
            let mut row = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .items_center()
                .child(div().meta(t).child("Show"))
                .child(
                    self.choice("status-filter", label, options, cx, |this, value, _, cx| {
                        this.filter.status = match value.as_str() {
                            "retired" => StatusFilter::Retired,
                            "all" => StatusFilter::All,
                            _ => StatusFilter::Current,
                        };
                        this.reload(cx);
                    }),
                );
            if let Some(loadout) = self.loadout.clone() {
                for (label, present) in [("Select all", true), ("Deselect all", false)] {
                    let loadout = loadout.clone();
                    row = row.child(
                        strider_ui::button(label, label, ButtonVariant::Ghost, t).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.command(
                                    Command::SetAllLoadoutMembers {
                                        loadout: loadout.clone(),
                                        present,
                                    },
                                    cx,
                                )
                            }),
                        ),
                    );
                }
            }
            row = row
                .child(
                    strider_ui::button(
                        "manage-loadouts",
                        "Manage loadouts",
                        ButtonVariant::Ghost,
                        t,
                    )
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_manager(false, window, cx)),
                    ),
                )
                .child(
                    strider_ui::button(
                        "manage-categories",
                        "Manage categories",
                        ButtonVariant::Ghost,
                        t,
                    )
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_manager(true, window, cx)),
                    ),
                );
            left = left.child(row);
        }
        let view = cx.entity();
        left = left.child(
            list(self.list.clone(), move |index, window, cx| {
                view.update(cx, |this, cx| this.render_entry(index, window, cx))
            })
            .w_full()
            .min_h_0()
            .flex_1(),
        );
        let left = left.debug_selector(|| "inventory-column".to_owned());
        let aside = self.render_base_bar(cx);
        let wide = window.viewport_size().width >= px(1100.);
        let layout = div().flex().gap_5().flex_1().min_h_0();
        let layout = if wide {
            layout
                .child(left)
                .child(div().w(px(340.)).flex_shrink_0().child(aside))
        } else {
            // A tall legend must not consume the retained list's entire viewport.
            // Scroll the stacked page, giving the virtual list its own finite viewport.
            layout
                .flex_col()
                .flex_none()
                .child(div().flex_none().child(aside))
                .child(left.h(px(500.)).flex_none())
        };
        strider_ui::page_body(vec![layout.into_any_element()], t)
            .id("inventory-body")
            .debug_selector(|| "inventory-body".to_owned())
            .when(!wide, |d| d.overflow_y_scroll())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .into_any_element()
    }
}
fn class(c: &str) -> WeightClass {
    match c {
        "worn" => WeightClass::Worn,
        "consumable" => WeightClass::Consumable,
        _ => WeightClass::Base,
    }
}
