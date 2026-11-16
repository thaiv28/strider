use super::*;
impl AppView {
    pub(crate) fn loadout_select(
        &self,
        id: &'static str,
        wishlist: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let mut options = vec![(String::new(), "All gear".into())];
        options.extend(
            self.loadouts
                .loadouts
                .iter()
                .map(|l| (l.id.clone(), l.name.clone())),
        );
        let label = self
            .loadouts
            .loadouts
            .iter()
            .find(|l| {
                Some(&l.id)
                    == if wishlist {
                        self.wishlist_loadout.as_ref()
                    } else {
                        self.loadout.as_ref()
                    }
            })
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "All gear".into());
        self.choice(id, label, options, cx, move |this, id, _, cx| {
            if wishlist {
                this.wishlist_loadout = (!id.is_empty()).then_some(id);
            } else {
                this.loadout = (!id.is_empty()).then_some(id);
            }
            this.reload(cx);
        })
    }
    pub(crate) fn render_base_bar(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let mut content = strider_ui::card(t)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.loadout_select("base-loadout", false, cx))
            .child(
                div()
                    .section_heading()
                    .readout()
                    .child(fmt_weight(Some(self.totals.base))),
            )
            .child(div().meta(t).child("Base weight by category"));
        let mut bar = div()
            .flex()
            .w_full()
            .h(px(32.))
            .rounded_full()
            .overflow_hidden();
        for (i, s) in self.totals.by_category.iter().enumerate() {
            let detail = format!(
                "{}: {}\n{}",
                s.name,
                fmt_weight(Some(s.g)),
                s.items
                    .iter()
                    .map(|i| format!(
                        "{}{} · {}",
                        i.name,
                        if i.quantity > 1 {
                            format!(" ×{}", i.quantity)
                        } else {
                            String::new()
                        },
                        fmt_oz(Some(i.g))
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let color = strider_ui::category_color(&s.name).unwrap_or(t.muted);

            bar = bar.child(
                div()
                    .w(relative(s.g.0 as f32 / self.totals.base.0.max(1) as f32))
                    .h_full()
                    .child(strider_ui::focus_info(
                        ("base-segment", i),
                        s.name.clone(),
                        detail.clone(),
                        Some(color),
                        t,
                    )),
            );
            content = content.child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().size(px(10.)).bg(color))
                    .child(div().flex_1().meta(t).child(s.name.clone()))
                    .child(div().readout().meta(t).child(fmt_oz(Some(s.g))))
                    .child(strider_ui::info_badge(("base-info", i), detail, t)),
            );
        }
        content.child(bar).into_any_element()
    }
}
