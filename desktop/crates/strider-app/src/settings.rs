//! The Settings area (ports of `app/settings` and its components): Account, appearance, energy
//! model, trip report template, calendar feeds, map cache, and a pointer to Devices on the web.
use super::*;
use gpui_component::input::TextareaState;
use offline::format::megabytes;
use strider_core::energy::mifflin_bmr;
use strider_core::units::Dec;
use strider_store::{CalendarFeedRow, EnergyInput, EnergySettings};

pub(crate) const DEFAULT_TEMPLATE: &str = "# {{name}}\n\n**When:** {{start_date}} – {{end_date}} · {{nights}} nights\n**Where:** {{region}} · {{area_type}}\n**Distance:** {{distance}} mi · {{gain}} ft gain\n**Trailhead:** {{trailhead}}\n_Prepared {{generated_on}}_\n\n## Map\n{{route_map}}\n\n## Itinerary\n{{itinerary}}\n\n## Campsites\n{{route}}\n\n## Getting there\n{{driving}}\n\n## Permits\n{{permits}}\n\n## Water\n{{water_sources}}\n\n## Weather\n{{weather}}\n\n## Food\n{{food_plan}}\n\n## What to pack\n{{packing_list}}\n\n## Notes\n{{planning_notes}}\n\n## Grocery list\n{{shopping_list}}\n";
const NEW_FEED_COLOR: &str = "#1f7a70";

pub(crate) struct EnergyForm {
    bmr: Entity<InputState>,
    cal: Entity<InputState>,
    ft: Entity<InputState>,
    weight: Entity<InputState>,
    feet: Entity<InputState>,
    inches: Entity<InputState>,
    age: Entity<InputState>,
    sex: String,
}
pub(crate) struct FeedEdit {
    id: String,
    label: Entity<InputState>,
    color: Entity<InputState>,
    url: Entity<InputState>,
}
pub(crate) struct SettingsState {
    energy: EnergyForm,
    template: Entity<TextareaState>,
    packing: Entity<TextareaState>,
    feeds: Vec<FeedEdit>,
    new_label: Entity<InputState>,
    new_color: Entity<InputState>,
    new_url: Entity<InputState>,
    confirm_remove: Option<String>,
    cache_bytes: u64,
    /// One status line per card: energy, report, feeds, cache.
    msg: [Option<String>; 4],
}
const ENERGY: usize = 0;
const REPORT: usize = 1;
const FEEDS: usize = 2;
const CACHE: usize = 3;

fn opt_text(v: Option<Dec>) -> String {
    v.map(|d| d.to_string()).unwrap_or_default()
}
fn parse_color(s: &str) -> Option<String> {
    let s = s.trim();
    Rgba::try_from(s).ok().map(|_| s.to_owned())
}
impl AppView {
    fn build_feeds(&self, w: &mut Window, cx: &mut Context<Self>) -> Vec<FeedEdit> {
        self.store
            .calendar_feeds()
            .unwrap_or_default()
            .into_iter()
            .map(|f: CalendarFeedRow| FeedEdit {
                id: f.id,
                label: input(f.label, false, w, cx),
                color: input(f.color, false, w, cx),
                url: input(f.url, false, w, cx),
            })
            .collect()
    }
    fn build_settings(&mut self, w: &mut Window, cx: &mut Context<Self>) -> SettingsState {
        let e: EnergySettings = self.store.energy_settings().unwrap_or_default();
        let height = e.height_in.and_then(|h| h.to_string().parse::<f64>().ok());
        let report = self.store.report_settings().unwrap_or_default();
        let energy = EnergyForm {
            bmr: input(e.bmr.to_string(), false, w, cx),
            cal: input(e.cal_per_mile.to_string(), false, w, cx),
            ft: input(e.ft_per_mile.to_string(), false, w, cx),
            weight: input(opt_text(e.weight_lb), false, w, cx),
            feet: input(
                height
                    .map(|h| (h / 12.).floor().to_string())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            inches: input(
                height
                    .map(|h| (h % 12.).round().to_string())
                    .unwrap_or_default(),
                false,
                w,
                cx,
            ),
            age: input(
                e.age_years.map(|a| a.to_string()).unwrap_or_default(),
                false,
                w,
                cx,
            ),
            sex: e.sex.unwrap_or_default(),
        };
        let template = textarea_input(
            report.template.unwrap_or_else(|| DEFAULT_TEMPLATE.into()),
            w,
            cx,
        );
        let packing = textarea_input(
            report
                .packing_default
                .unwrap_or_else(|| strider_store::DEFAULT_PACKING_LIST.into()),
            w,
            cx,
        );
        SettingsState {
            energy,
            template,
            packing,
            feeds: self.build_feeds(w, cx),
            new_label: input("", false, w, cx),
            new_color: input(NEW_FEED_COLOR, false, w, cx),
            new_url: input("", false, w, cx),
            confirm_remove: None,
            cache_bytes: strider_map::disk_usage(&self.offline.backend.tiles_root),
            msg: [None, None, None, None],
        }
    }
    pub(super) fn settings_cache_size(&mut self) {
        let bytes = strider_map::disk_usage(&self.offline.backend.tiles_root);
        if let Some(s) = &mut self.settings {
            s.cache_bytes = bytes;
        }
    }
    fn say(&mut self, card: usize, text: impl Into<Option<String>>) {
        if let Some(s) = &mut self.settings {
            s.msg[card] = text.into();
        }
    }
    pub(super) fn save_energy(&mut self, cx: &mut Context<Self>) {
        let Some(s) = &self.settings else { return };
        let f = &s.energy;
        let parsed = (|| -> Result<EnergyInput, String> {
            let num = |i: &Entity<InputState>| number(i, cx);
            let height = match (num(&f.feet)?, num(&f.inches)?) {
                (None, None) => None,
                (ft, inch) => Some(
                    format!("{}", ft.unwrap_or(0.) * 12. + inch.unwrap_or(0.))
                        .parse::<Dec>()
                        .map_err(|_| "Enter a number.".to_string())?,
                ),
            };
            let age = match optional(&f.age, cx) {
                None => None,
                Some(a) => Some(
                    a.trim()
                        .parse::<i32>()
                        .ok()
                        .filter(|a| *a >= 0)
                        .ok_or("Enter a number.")?,
                ),
            };
            Ok(EnergyInput {
                bmr: num(&f.bmr)?.unwrap_or(0.),
                cal_per_mile: num(&f.cal)?.unwrap_or(0.),
                ft_per_mile: num(&f.ft)?.unwrap_or(1.),
                sex: (!f.sex.is_empty()).then(|| f.sex.clone()),
                weight_lb: decimal_input(&f.weight, cx)?,
                height_in: height,
                age_years: age,
            })
        })();
        match parsed {
            Ok(input) => {
                self.command(Command::UpdateEnergyParams { input }, cx);
                let saved = self.error.is_none().then(|| "Saved.".to_string());
                self.say(ENERGY, saved);
            }
            Err(e) => self.say(ENERGY, e),
        }
        cx.notify();
    }
    fn use_calculated_bmr(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(calc) = self.calculated_bmr(cx) else {
            return;
        };
        if let Some(s) = &self.settings {
            s.energy
                .bmr
                .update(cx, |i, cx| i.set_value(calc.to_string(), w, cx));
        }
    }
    fn calculated_bmr(&self, cx: &App) -> Option<i64> {
        let f = &self.settings.as_ref()?.energy;
        let height = match (number(&f.feet, cx).ok()?, number(&f.inches, cx).ok()?) {
            (None, None) => None,
            (ft, inch) => Some(ft.unwrap_or(0.) * 12. + inch.unwrap_or(0.)),
        };
        mifflin_bmr(
            (!f.sex.is_empty()).then_some(f.sex.as_str()),
            number(&f.weight, cx).ok()?,
            height,
            number(&f.age, cx).ok()?,
        )
    }
    pub(super) fn save_report(&mut self, cx: &mut Context<Self>) {
        let Some(s) = &self.settings else { return };
        let template = s.template.read(cx).value().to_string();
        let packing_default = s.packing.read(cx).value().to_string();
        self.command(
            Command::UpdateReportSettings {
                template,
                packing_default,
            },
            cx,
        );
        let saved = self.error.is_none().then(|| "Saved.".to_string());
        self.say(REPORT, saved);
        cx.notify();
    }
    pub(super) fn add_feed(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = &self.settings else { return };
        let (label, url) = (value(&s.new_label, cx), value(&s.new_url, cx));
        let Some(color) = parse_color(&value(&s.new_color, cx)) else {
            return self.say(FEEDS, "Enter a color like #1f7a70.".to_string());
        };
        if url.trim().is_empty() {
            return self.say(FEEDS, "Enter the calendar address.".to_string());
        }
        self.command(Command::AddCalendarFeed { label, color, url }, cx);
        if self.error.is_none() {
            let feeds = self.build_feeds(w, cx);
            let (label, url) = (input("", false, w, cx), input("", false, w, cx));
            if let Some(s) = &mut self.settings {
                s.feeds = feeds;
                s.new_label = label;
                s.new_url = url;
            }
            self.say(FEEDS, None);
            self.cal.data = None;
        }
        cx.notify();
    }
    pub(super) fn save_feed(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(f) = self.settings.as_ref().and_then(|s| s.feeds.get(i)) else {
            return;
        };
        let (id, label, url) = (f.id.clone(), value(&f.label, cx), value(&f.url, cx));
        let Some(color) = parse_color(&value(&f.color, cx)) else {
            return self.say(FEEDS, "Enter a color like #1f7a70.".to_string());
        };
        self.command(
            Command::UpdateCalendarFeed {
                id,
                label,
                color,
                url,
            },
            cx,
        );
        let saved = self.error.is_none().then(|| "Saved.".to_string());
        self.say(FEEDS, saved);
        self.cal.data = None;
        cx.notify();
    }
    pub(super) fn remove_feed(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self
            .settings
            .as_ref()
            .and_then(|s| s.feeds.get(i))
            .map(|f| f.id.clone())
        else {
            return;
        };
        if self
            .settings
            .as_ref()
            .and_then(|s| s.confirm_remove.as_ref())
            != Some(&id)
        {
            if let Some(s) = &mut self.settings {
                s.confirm_remove = Some(id);
            }
            return cx.notify();
        }
        self.command(Command::DeleteCalendarFeed { id }, cx);
        self.rebuild_feeds(w, cx);
    }
    pub(super) fn move_feed(
        &mut self,
        i: usize,
        delta: isize,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = &self.settings else { return };
        let mut ids: Vec<String> = s.feeds.iter().map(|f| f.id.clone()).collect();
        let j = i as isize + delta;
        if j < 0 || j as usize >= ids.len() {
            return;
        }
        ids.swap(i, j as usize);
        self.command(Command::ReorderCalendarFeeds { ordered_ids: ids }, cx);
        self.rebuild_feeds(w, cx);
    }
    fn rebuild_feeds(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let feeds = self.build_feeds(w, cx);
        if let Some(s) = &mut self.settings {
            s.feeds = feeds;
            s.confirm_remove = None;
        }
        self.cal.data = None;
        cx.notify();
    }
    fn settings_card(&self, title: &'static str, t: Theme) -> Div {
        strider_ui::card(t)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().card_heading().child(title))
    }
    fn labelled(&self, state: &Entity<InputState>, label: &'static str, t: Theme) -> Div {
        div().flex_1().min_w(px(120.)).child(strider_ui::field(
            label,
            strider_ui::text_input(state, label, t),
            t,
        ))
    }
    fn status(&self, card: usize, t: Theme) -> Option<Div> {
        self.settings
            .as_ref()
            .and_then(|s| s.msg[card].clone())
            .map(|m| div().meta(t).child(m))
    }
    fn appearance_card(&self, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let seg = |id: &'static str, label: &'static str, a: Appearance| {
            strider_ui::segment(id, label, self.appearance == a, t)
                .on_click(cx.listener(move |this, _, w, cx| this.appearance(a, w, cx)))
                .into_any_element()
        };
        self.settings_card("Appearance", t)
            .child(strider_ui::segmented_control(
                [
                    seg("appearance-light", "Light", Appearance::Light),
                    seg("appearance-dark", "Dark", Appearance::Dark),
                    seg("appearance-system", "System", Appearance::System),
                ],
                t,
            ))
    }
    fn energy_card(&self, s: &SettingsState, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let f = &s.energy;
        let calc = self.calculated_bmr(cx);
        let mut card = self
            .settings_card("Energy model", t)
            .child(
                div().meta(t).child(
                    "Each day's calorie target is your resting burn plus the effort of moving.",
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .child(self.labelled(&f.bmr, "BMR (resting kcal/day)", t))
                    .child(self.labelled(&f.cal, "kcal per energy-mile", t))
                    .child(self.labelled(&f.ft, "ft of gain = 1 mile", t)),
            )
            .child(
                div()
                    .card_heading()
                    .text_size(px(15.))
                    .child("BMR calculator (Mifflin–St Jeor)"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .child(
                        div().flex_1().min_w(px(120.)).child(strider_ui::field(
                            "Sex",
                            self.choice(
                                "energy-sex",
                                if f.sex.is_empty() {
                                    "—".to_owned()
                                } else {
                                    f.sex.clone()
                                },
                                [("", "—"), ("male", "male"), ("female", "female")]
                                    .map(|(a, b)| (a.to_owned(), b.to_owned()))
                                    .to_vec(),
                                cx,
                                |this, v, _, cx| {
                                    if let Some(s) = &mut this.settings {
                                        s.energy.sex = v;
                                    }
                                    cx.notify();
                                },
                            ),
                            t,
                        )),
                    )
                    .child(self.labelled(&f.weight, "Weight (lb)", t))
                    .child(self.labelled(&f.feet, "Height (ft)", t))
                    .child(self.labelled(&f.inches, "Height (in)", t))
                    .child(self.labelled(&f.age, "Age", t)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().meta(t).child(format!(
                        "Calculated BMR: {}",
                        calc.map_or("—".into(), |c| format!("{c} kcal"))
                    )))
                    .child(
                        strider_ui::button("use-bmr", "Use as BMR", ButtonVariant::Outline, t)
                            .when(calc.is_none(), |b| b.opacity(0.5))
                            .when(calc.is_some(), |b| {
                                b.on_click(
                                    cx.listener(|this, _, w, cx| this.use_calculated_bmr(w, cx)),
                                )
                            }),
                    ),
            );
        if let Some(m) = self.status(ENERGY, t) {
            card = card.child(m);
        }
        card.child(
            strider_ui::button("save-energy", "Save", ButtonVariant::Primary, t)
                .on_click(cx.listener(|this, _, _, cx| this.save_energy(cx))),
        )
    }
    fn report_card(&self, s: &SettingsState, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let mut card = self
            .settings_card("Trip report template", t)
            .child(div().meta(t).child(
                "The Markdown document behind each trip's shareable report. Use {{tokens}} to drop in trip data; empty ones remove their line.",
            ))
            .child(strider_ui::field(
                "Template",
                strider_ui::textarea(&s.template, "Template", t).h(px(220.)),
                t,
            ))
            .child(strider_ui::field(
                "Default packing list",
                strider_ui::textarea(&s.packing, "Default packing list", t).h(px(160.)),
                t,
            ));
        if let Some(m) = self.status(REPORT, t) {
            card = card.child(m);
        }
        card.child(
            strider_ui::button("save-report", "Save", ButtonVariant::Primary, t)
                .on_click(cx.listener(|this, _, _, cx| this.save_report(cx))),
        )
    }
    fn feeds_card(&self, s: &SettingsState, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let mut card = self.settings_card("Calendars", t).child(div().meta(t).child(
            "Read-only feeds shown on the planning calendar. Copy the secret address in iCal format from your calendar's settings, and treat it as private.",
        ));
        if s.feeds.is_empty() {
            card = card.child(div().meta(t).child("No calendars yet."));
        }
        let last = s.feeds.len().saturating_sub(1);
        for (i, f) in s.feeds.iter().enumerate() {
            let swatch = Rgba::try_from(value(&f.color, cx).trim())
                .map(Hsla::from)
                .unwrap_or(t.muted);
            let confirm = s.confirm_remove.as_deref() == Some(f.id.as_str());
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .pt_3()
                    .border_t_1()
                    .border_color(t.divider)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_end()
                            .gap_3()
                            .child(div().size(px(12.)).mb_4().rounded_full().bg(swatch))
                            .child(self.labelled(&f.label, "Label", t))
                            .child(self.labelled(&f.color, "Color", t)),
                    )
                    .child(self.labelled(&f.url, "URL", t))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                strider_ui::button(
                                    ("save-feed", i),
                                    "Save",
                                    ButtonVariant::Outline,
                                    t,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| this.save_feed(i, cx))),
                            )
                            .child(
                                strider_ui::button(
                                    ("feed-up", i),
                                    "Move up",
                                    ButtonVariant::Ghost,
                                    t,
                                )
                                .when(i == 0, |b| b.opacity(0.5))
                                .on_click(
                                    cx.listener(move |this, _, w, cx| this.move_feed(i, -1, w, cx)),
                                ),
                            )
                            .child(
                                strider_ui::button(
                                    ("feed-down", i),
                                    "Move down",
                                    ButtonVariant::Ghost,
                                    t,
                                )
                                .when(i == last, |b| b.opacity(0.5))
                                .on_click(
                                    cx.listener(move |this, _, w, cx| this.move_feed(i, 1, w, cx)),
                                ),
                            )
                            .child(
                                strider_ui::button(
                                    ("remove-feed", i),
                                    if confirm { "Confirm remove" } else { "Remove" },
                                    ButtonVariant::Danger,
                                    t,
                                )
                                .on_click(
                                    cx.listener(move |this, _, w, cx| this.remove_feed(i, w, cx)),
                                ),
                            ),
                    ),
            );
        }
        card = card.child(
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .pt_3()
                .border_t_1()
                .border_color(t.divider)
                .child(self.labelled(&s.new_label, "Label", t))
                .child(self.labelled(&s.new_color, "Color", t))
                .child(self.labelled(&s.new_url, "URL", t)),
        );
        if let Some(m) = self.status(FEEDS, t) {
            card = card.child(m);
        }
        card.child(
            strider_ui::button("add-feed", "Add", ButtonVariant::Primary, t)
                .on_click(cx.listener(|this, _, w, cx| this.add_feed(w, cx))),
        )
    }
    fn map_cache_card(&self, s: &SettingsState, cx: &Context<Self>) -> Div {
        let t = Theme::get(cx);
        let mut card = self.settings_card("Map cache", t).child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().readout().child(megabytes(s.cache_bytes)))
                .child(
                    strider_ui::button("clear-cache", "Clear cache", ButtonVariant::Outline, t)
                        .on_click(cx.listener(|this, _, _, cx| this.clear_map_cache(cx))),
                ),
        );
        card = card.child(div().meta(t).child(
            "Clearing removes tiles you have only viewed. Downloaded maps stay until you delete them.",
        ));
        if self.offline.regions.is_empty() {
            card = card.child(div().meta(t).child("No downloaded maps."));
        }
        for (i, r) in self.offline.regions.iter().enumerate() {
            let trip = r.trip_sync_id.clone();
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().child(self.region_summary(r)))
                    .child(
                        strider_ui::button(("delete-map", i), "Delete", ButtonVariant::Danger, t)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.delete_region(&trip, cx)),
                            ),
                    ),
            );
        }
        if let Some(m) = self.status(CACHE, t).or_else(|| {
            self.offline
                .message
                .clone()
                .map(|m| div().text_color(t.signal).child(m))
        }) {
            card = card.child(m);
        }
        card
    }
    pub(super) fn render_settings(&mut self, w: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        if self.settings.is_none() {
            self.load_regions();
            self.settings = Some(self.build_settings(w, cx));
        }
        let Some(s) = self.settings.as_ref() else {
            return div().into_any_element();
        };
        let devices = self
            .settings_card("Devices", t)
            .child(div().meta(t).child(format!(
                "Manage signed-in devices on the web at {}/settings.",
                self.account.server.trim_end_matches('/')
            )));
        let left = div()
            .flex_1()
            .min_w(px(420.))
            .flex()
            .flex_col()
            .gap_3()
            .child(self.account_card(cx))
            .child(self.appearance_card(cx))
            .child(self.energy_card(s, cx))
            .child(devices);
        let right = div()
            .flex_1()
            .min_w(px(420.))
            .flex()
            .flex_col()
            .gap_3()
            .child(self.feeds_card(s, cx))
            .child(self.map_cache_card(s, cx))
            .child(self.report_card(s, cx));
        strider_ui::page_body(
            [div()
                .id("settings-scroll")
                .overflow_y_scroll()
                .flex_1()
                .min_h_0()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap_4()
                        .child(left)
                        .child(right),
                )
                .into_any_element()],
            t,
        )
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::ENERGY;
    use crate::app::AppView;
    use crate::app::test_support::{headless, with_view};
    use gpui::{Context, Entity, TestAppContext, Window};
    use gpui_component::input::InputState;

    fn set(state: &Entity<InputState>, text: &str, w: &mut Window, cx: &mut Context<AppView>) {
        state.update(cx, |s, cx| s.set_value(text.to_owned(), w, cx));
    }
    #[gpui::test]
    fn settings_edits_persist(cx: &mut TestAppContext) {
        let (view, mut cx) = headless(cx);
        cx.run_until_parked();
        with_view(&view, &mut cx, |g, _, cx| {
            g.area = "Settings";
            cx.notify();
        });
        cx.run_until_parked(); // renders every card
        with_view(&view, &mut cx, |g, w, cx| {
            let s = g
                .settings
                .as_ref()
                .expect("forms are built on first render");
            let e = &s.energy;
            for (state, text) in [
                (&e.bmr, "1800"),
                (&e.cal, "210"),
                (&e.ft, "600"),
                (&e.weight, "150"),
                (&e.feet, "5"),
                (&e.inches, "9"),
                (&e.age, "34"),
            ] {
                set(state, text, w, cx);
            }
            g.settings.as_mut().expect("forms").energy.sex = "female".into();
            g.save_energy(cx);
            let saved = g.store.energy_settings().expect("energy");
            assert_eq!(
                (saved.bmr, saved.cal_per_mile, saved.ft_per_mile),
                (1800, 210, 600)
            );
            assert_eq!(saved.sex.as_deref(), Some("female"));
            assert_eq!(
                saved.weight_lb.map(|d| d.to_string()).as_deref(),
                Some("150")
            );
            assert_eq!(
                saved.height_in.map(|d| d.to_string()).as_deref(),
                Some("69")
            );
            assert_eq!(saved.age_years, Some(34));
            // Mifflin-St Jeor for a 150 lb, 69 in, 34 year old woman.
            assert_eq!(g.calculated_bmr(cx), Some(1445));
            // An invalid number is reported and saves nothing.
            let bmr = g.settings.as_ref().expect("forms").energy.bmr.clone();
            set(&bmr, "lots", w, cx);
            g.save_energy(cx);
            assert_eq!(g.store.energy_settings().expect("energy").bmr, 1800);
            assert_eq!(
                g.settings.as_ref().expect("forms").msg[ENERGY].as_deref(),
                Some("Enter a number.")
            );

            // Report template.
            let s = g.settings.as_ref().expect("forms");
            s.template
                .update(cx, |t, cx| t.set_value("# {{name}}\n".to_owned(), w, cx));
            s.packing
                .update(cx, |t, cx| t.set_value("- Map\n".to_owned(), w, cx));
            g.save_report(cx);
            let report = g.store.report_settings().expect("report");
            assert_eq!(report.template.as_deref(), Some("# {{name}}\n"));
            assert_eq!(report.packing_default.as_deref(), Some("- Map\n"));

            // Calendar feeds: add two, edit, reorder and remove.
            for (label, url) in [
                ("Work", "https://example.test/work.ics"),
                ("Home", "https://example.test/home.ics"),
            ] {
                let s = g.settings.as_ref().expect("forms");
                let (l, u) = (s.new_label.clone(), s.new_url.clone());
                set(&l, label, w, cx);
                set(&u, url, w, cx);
                g.add_feed(w, cx);
            }
            let labels = |g: &AppView| {
                g.store
                    .calendar_feeds()
                    .expect("feeds")
                    .into_iter()
                    .map(|f| f.label)
                    .collect::<Vec<_>>()
            };
            assert_eq!(labels(g), ["Work", "Home"]);
            assert_eq!(g.settings.as_ref().expect("forms").feeds.len(), 2);
            let first = g.settings.as_ref().expect("forms").feeds[0].label.clone();
            set(&first, "Office", w, cx);
            let color = g.settings.as_ref().expect("forms").feeds[0].color.clone();
            set(&color, "nope", w, cx);
            g.save_feed(0, cx);
            assert_eq!(labels(g), ["Work", "Home"], "a bad color saves nothing");
            set(&color, "#336699", w, cx);
            g.save_feed(0, cx);
            let feeds = g.store.calendar_feeds().expect("feeds");
            assert_eq!(
                (feeds[0].label.as_str(), feeds[0].color.as_str()),
                ("Office", "#336699")
            );
            g.move_feed(0, 1, w, cx);
            assert_eq!(labels(g), ["Home", "Office"]);
            g.remove_feed(0, w, cx);
            assert_eq!(labels(g).len(), 2, "removal asks for a second press");
            g.remove_feed(0, w, cx);
            assert_eq!(labels(g), ["Office"]);
        });
        cx.run_until_parked();
    }
}
