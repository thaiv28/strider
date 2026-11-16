//! The command palette (⌘K): every command, plus "Go to" entries for each Trip, gear item and meal.
use super::*;
use strider_store::StatusFilter;

pub(crate) enum Target {
    Action(Box<dyn Action>),
    Trip(String),
    Gear(String),
    Meal(String),
}
pub(crate) struct PaletteItem {
    pub label: String,
    /// Shown dimmed beside the label.
    pub kind: &'static str,
    pub target: Target,
}
pub(crate) struct PaletteState {
    pub input: Entity<InputState>,
    pub entries: Vec<PaletteItem>,
    pub selected: usize,
}
/// Results shown at once; typing narrows them.
const SHOWN: usize = 12;

/// Subsequence match, case-insensitive. `None` when the query's characters do not appear in
/// order. Higher is better: consecutive runs and word starts score, gaps and long labels cost.
pub(crate) fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    let c: Vec<char> = candidate.chars().flat_map(char::to_lowercase).collect();
    if q.is_empty() {
        return Some(0);
    }
    let mut score = 0;
    let mut at = 0;
    let mut previous: Option<usize> = None;
    for (n, wanted) in q.iter().enumerate() {
        let found = (at..c.len()).find(|i| c[*i] == *wanted)?;
        score += 10;
        if previous.is_some_and(|p| p + 1 == found) {
            score += 15;
        } else if let Some(p) = previous {
            score -= (found - p - 1).min(8) as i32;
        }
        if found == 0 || !c[found - 1].is_alphanumeric() {
            score += 20;
        }
        if n == 0 && found == 0 {
            score += 25;
        }
        previous = Some(found);
        at = found + 1;
    }
    Some(score - (c.len() as i32) / 4)
}
/// Indices of the entries that match, best first; ties keep their original order.
pub(crate) fn rank(entries: &[PaletteItem], query: &str) -> Vec<usize> {
    let mut scored: Vec<(i32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| fuzzy_score(query, &e.label).map(|s| (s, i)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, i)| i).collect()
}
impl AppView {
    pub(crate) fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() {
            self.close_overlays(window, cx);
            return;
        }
        if self.overlay_open() {
            return;
        }
        let mut entries: Vec<PaletteItem> = menus::commands()
            .into_iter()
            .filter(|(_, action)| window.is_action_available(action.as_ref(), cx))
            .map(|(label, action)| PaletteItem {
                label: label.to_owned(),
                kind: "Command",
                target: Target::Action(action),
            })
            .collect();
        entries.extend(self.all_trips().into_iter().map(|t| PaletteItem {
            label: format!("Go to Trip: {}", t.name),
            kind: "Trip",
            target: Target::Trip(t.sync_id),
        }));
        entries.extend(self.all_gear.iter().map(|g| PaletteItem {
            label: format!("Go to Gear: {}", g.name),
            kind: "Gear",
            target: Target::Gear(g.id.clone()),
        }));
        entries.extend(self.food.meals.iter().map(|m| PaletteItem {
            label: format!("Go to Meal: {}", m.name),
            kind: "Meal",
            target: Target::Meal(m.id.clone()),
        }));
        let input = input("", false, window, cx);
        input.update(cx, |s, cx| s.focus(window, cx));
        self.palette = Some(PaletteState {
            input,
            entries,
            selected: 0,
        });
        cx.notify();
    }
    /// Up, Down and Return while the palette is open. The text field would otherwise consume them,
    /// so this runs from a keystroke interceptor ahead of action dispatch. Returns whether it handled the key.
    pub(crate) fn palette_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.palette.is_none() {
            return false;
        }
        match key {
            "down" => self.palette_move(1, cx),
            "up" => self.palette_move(-1, cx),
            "enter" => {
                let row = self.palette.as_ref().map_or(0, |p| p.selected);
                self.run_palette(row, window, cx);
            }
            _ => return false,
        }
        true
    }
    fn palette_query(&self, cx: &App) -> String {
        self.palette
            .as_ref()
            .map(|p| value(&p.input, cx))
            .unwrap_or_default()
    }
    /// Entry indices currently listed, best first.
    pub(crate) fn palette_results(&self, cx: &App) -> Vec<usize> {
        let Some(p) = &self.palette else {
            return vec![];
        };
        let mut found = rank(&p.entries, &self.palette_query(cx));
        found.truncate(SHOWN);
        found
    }
    pub(crate) fn palette_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.palette_results(cx).len();
        if let Some(p) = &mut self.palette
            && count > 0
        {
            p.selected = (p.selected as isize + delta).clamp(0, count as isize - 1) as usize;
        }
        cx.notify();
    }
    /// Run the entry at `row` of the current results (Return, or a click).
    pub(crate) fn run_palette(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.palette_results(cx).get(row).copied() else {
            return;
        };
        let Some(mut palette) = self.palette.take() else {
            return;
        };
        let target = palette.entries.swap_remove(index).target;
        self.close_overlays(window, cx);
        match target {
            Target::Action(action) => window.dispatch_action(action, cx),
            Target::Trip(id) => {
                self.request_navigation(Navigation::Area("Trips"), window, cx);
                if self.pending_navigation.is_none() {
                    self.open_trip(id, cx);
                }
            }
            Target::Gear(id) => self.go_to_gear(id, window, cx),
            Target::Meal(id) => {
                self.request_navigation(Navigation::Area("Food"), window, cx);
                if self.pending_navigation.is_none() {
                    self.food.pantry = false;
                    self.request_navigation(Navigation::Meal(Some(id)), window, cx);
                }
            }
        }
    }
    fn go_to_gear(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(status) = self
            .all_gear
            .iter()
            .find(|g| g.id == id)
            .map(|g| g.status.clone())
        else {
            return;
        };
        self.request_navigation(Navigation::Area("Gear"), window, cx);
        if self.pending_navigation.is_some() {
            return;
        }
        self.wishlist = status == "wishlist";
        self.filter.status = if status == "retired" {
            StatusFilter::Retired
        } else {
            StatusFilter::Current
        };
        self.filter.query.clear();
        let search = self.filter_input.clone();
        search.update(cx, |s, cx| s.set_value("", window, cx));
        self.reload(cx);
        self.selected = Some(id.clone());
        if let Some(index) = self
            .entries
            .iter()
            .position(|e| matches!(e, gear::list::Entry::Item { row, .. } if row.id == id))
        {
            self.list.scroll_to_reveal_item(index);
        }
        cx.notify();
    }
    pub(crate) fn render_palette(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let Some(p) = &self.palette else {
            return div().into_any_element();
        };
        let results = self.palette_results(cx);
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(strider_ui::text_input(
                &p.input,
                "Search commands, trips, gear and meals",
                t,
            ))
            .child(
                div()
                    .id("palette-results")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(results.iter().enumerate().map(|(row, index)| {
                        let entry = &p.entries[*index];
                        strider_ui::button(
                            ("palette-row", row),
                            entry.label.clone(),
                            if row == p.selected {
                                ButtonVariant::Outline
                            } else {
                                ButtonVariant::Ghost
                            },
                            t,
                        )
                        .justify_between()
                        .gap_3()
                        .child(div().meta(t).child(entry.kind))
                        .on_click(cx.listener(move |this, _, w, cx| this.run_palette(row, w, cx)))
                    }))
                    .when(results.is_empty(), |d| {
                        d.child(div().meta(t).child("Nothing matches."))
                    }),
            );
        self.modal("Command palette", body.into_any_element(), cx)
    }
}
#[cfg(test)]
mod tests {
    use super::{PaletteItem, Target, fuzzy_score, rank};
    fn entry(label: &str) -> PaletteItem {
        PaletteItem {
            label: label.into(),
            kind: "Command",
            target: Target::Trip(String::new()),
        }
    }
    #[test]
    fn subsequences_match_in_order_and_ignore_case() {
        assert!(fuzzy_score("gtt", "Go to Trips").is_some());
        assert!(fuzzy_score("GEAR", "Go to Gear").is_some());
        assert_eq!(fuzzy_score("zz", "Go to Gear"), None);
        assert_eq!(fuzzy_score("rg", "Go to Gear"), None, "order matters");
        assert_eq!(fuzzy_score("", "anything"), Some(0));
        assert_eq!(fuzzy_score("   ", "anything"), Some(0));
    }
    #[test]
    fn prefixes_runs_and_word_starts_rank_higher() {
        let prefix = fuzzy_score("tri", "Trips").expect("match");
        let scattered = fuzzy_score("tri", "Set a rough itinerary").expect("match");
        assert!(prefix > scattered);
        let word_starts = fuzzy_score("nw", "New Window").expect("match");
        let buried = fuzzy_score("nw", "Show Sync Status Now").expect("match");
        assert!(word_starts > buried);
        let tight = fuzzy_score("sync", "Sync Now").expect("match");
        let loose = fuzzy_score("sync", "Sunny Navy Cat").expect("match");
        assert!(tight > loose);
    }
    #[test]
    fn ranking_is_best_first_and_stable() {
        let entries = [
            entry("Sign Out"),
            entry("Sync Now"),
            entry("Show Sync Status"),
            entry("Quit Strider"),
        ];
        let ranked = rank(&entries, "sync");
        assert_eq!(ranked, vec![1, 2]);
        assert_eq!(rank(&entries, "").len(), 4, "empty query lists everything");
        assert_eq!(rank(&entries, ""), vec![0, 1, 2, 3], "in original order");
        assert!(rank(&entries, "qqq").is_empty());
    }
}
