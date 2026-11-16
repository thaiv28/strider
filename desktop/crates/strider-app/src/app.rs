#[path = "account.rs"]
mod account;
#[path = "calendar.rs"]
mod calendar;
#[path = "devices.rs"]
mod devices;
#[path = "food/mod.rs"]
mod food;
#[path = "gear/mod.rs"]
mod gear;
#[path = "menus.rs"]
mod menus;
#[path = "offline/mod.rs"]
mod offline;
#[path = "palette.rs"]
mod palette;
#[path = "panels.rs"]
mod panels;
#[path = "print.rs"]
mod print;
#[path = "settings.rs"]
mod settings;
#[path = "share.rs"]
mod share;
#[path = "shell.rs"]
mod shell;
#[path = "trips/mod.rs"]
mod trips;
#[path = "updates.rs"]
mod updates;
#[path = "windows.rs"]
mod windows;
use food::{FoodState, ingredient_dialog::IngredientEditor, meal_editor::MealEditor};
use gear::dialog::Editor;
use gear::list::Entry;
use gear::loadouts::Manager;
use gpui::{prelude::*, *};
use gpui_component::input::{InputEvent, InputState};
use menus::*;
use std::collections::BTreeMap;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use strider_core::{fmt_oz, fmt_weight, g_to_oz, oz_to_g};
use strider_store::{CategoryOption, ChangeSet, LoadoutData};
use strider_store::{
    Command, ComponentInput, GearFilter, GearInput, GearRow, Group, StatusFilter, Store, Totals,
};
use strider_ui::{Appearance, ButtonVariant, Icon, Theme, TopoStyle};
use windows::SharedStore;
actions!(
    strider,
    [
        SelectNext,
        SelectPrevious,
        EditSelected,
        DeleteSelected,
        Undo,
        Redo,
        Find,
        Escape,
        Quit,
        Light,
        Dark,
        System,
        AddGear,
        ImportGpx,
        ExportGpx
    ]
);
struct FrameLog {
    enabled: bool,
    previous: Option<Instant>,
    since: Instant,
    samples: Vec<f64>,
}
impl FrameLog {
    fn new() -> Self {
        Self {
            enabled: std::env::var("STRIDER_FRAME_LOG").is_ok_and(|v| v == "1"),
            previous: None,
            since: Instant::now(),
            samples: Vec::new(),
        }
    }
    fn record(&mut self) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        if let Some(previous) = self.previous.replace(now) {
            self.samples
                .push(now.duration_since(previous).as_secs_f64() * 1000.);
        }
        if now.duration_since(self.since) >= Duration::from_secs(1) && !self.samples.is_empty() {
            self.samples.sort_by(f64::total_cmp);
            let p95 = self.samples
                [((self.samples.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
            eprintln!(
                "Strider root render intervals: p95={p95:.2}ms max={:.2}ms n={}",
                self.samples[self.samples.len() - 1],
                self.samples.len()
            );
            self.samples.clear();
            self.since = now;
        }
    }
}
#[derive(Clone)]
pub(crate) enum Navigation {
    Area(&'static str),
    Pantry(bool),
    Meal(Option<String>),
    CreateMeal,
}
pub(crate) struct AppView {
    entity_id: EntityId,
    store: SharedStore,
    account: account::AccountState,
    groups: Vec<Group>,
    totals: Totals,
    filter: GearFilter,
    entries: Vec<Entry>,
    list: ListState,
    selected: Option<String>,
    expanded: HashSet<String>,
    filter_input: Entity<InputState>,
    focus: FocusHandle,
    editor: Option<Editor>,
    area: &'static str,
    loadouts: LoadoutData,
    loadout: Option<String>,
    categories: Vec<CategoryOption>,
    all_gear: Vec<GearRow>,
    more: bool,
    replacements: Option<String>,
    manager: Option<Manager>,
    food: FoodState,
    trips: trips::TripsState,
    offline: offline::OfflineState,
    devices: devices::DevicesState,
    panels: Option<panels::PanelData>,
    share: share::ShareState,
    settings: Option<settings::SettingsState>,
    cal: calendar::CalendarState,
    /// A short notice that clears itself.
    toast: Option<(String, Instant)>,
    pending_navigation: Option<Navigation>,
    wishlist: bool,
    wishlist_query: Entity<InputState>,
    wishlist_loadout: Option<String>,
    error: Option<String>,
    appearance: Appearance,
    palette: Option<palette::PaletteState>,
    /// A right-click menu on a Trip row: its id and where to draw the menu.
    trip_menu: Option<(String, Point<Pixels>)>,
    window_title: String,
    seen_frame: Option<String>,
    frame_pending: bool,
    _subscriptions: Vec<Subscription>,
    frames: FrameLog,
}
impl AppView {
    #[cfg(test)]
    pub(crate) fn new(store: Store, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::build(SharedStore::new(store), window, cx)
    }
    /// A window over a workspace that other windows may share.
    pub(crate) fn build(store: SharedStore, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search gear"));
        let subscription = cx.subscribe_in(&filter_input, window, |this, input, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.filter.query = input.read(cx).value().to_string();
                this.reload(cx);
            }
        });
        let appearance_sub = cx.observe_window_appearance(window, |_, window, cx| {
            if *cx.global::<Appearance>() == Appearance::System {
                strider_ui::set_appearance(Appearance::System, window, cx);
                cx.notify();
            }
        });
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let data_dir = store.borrow().data_dir().to_path_buf();
        let mut this = Self {
            entity_id: cx.entity_id(),
            store,
            account: account::AccountState::default(),
            groups: Vec::new(),
            totals: Totals::default(),
            filter: GearFilter::default(),
            entries: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(500.)),
            selected: None,
            expanded: HashSet::new(),
            filter_input,
            focus,
            editor: None,
            area: "Gear",
            loadouts: LoadoutData::default(),
            loadout: None,
            categories: vec![],
            all_gear: vec![],
            more: false,
            replacements: None,
            manager: None,
            food: FoodState::new(window, cx),
            trips: trips::TripsState::new(cx),
            offline: offline::OfflineState::new(),
            devices: devices::DevicesState::new(devices::DevicesState::default_root(), data_dir),
            panels: None,
            share: share::ShareState::default(),
            settings: None,
            cal: calendar::CalendarState::new(),
            toast: None,
            pending_navigation: None,
            wishlist: false,
            wishlist_query: input("", false, window, cx),
            wishlist_loadout: None,
            error: None,
            appearance: Appearance::System,
            palette: None,
            trip_menu: None,
            window_title: String::new(),
            seen_frame: None,
            frame_pending: false,
            _subscriptions: vec![subscription, appearance_sub],
            frames: FrameLog::new(),
        };
        let search = this.food.search.clone();
        this._subscriptions
            .push(cx.subscribe_in(&search, window, |_, _, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }));
        this.subscribe_routes(cx);
        this.register(window, cx);
        this.reload(cx);
        this.loadout = this
            .loadouts
            .loadouts
            .iter()
            .find(|l| l.is_default)
            .or_else(|| this.loadouts.loadouts.first())
            .map(|l| l.id.clone());
        this.wishlist_loadout = this.loadout.clone();
        this.reload(cx);
        this
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        let result = (|| -> strider_store::Result<()> {
            let view = self.store.borrow().gear_view()?;
            self.all_gear = view.groups.into_iter().flat_map(|g| g.rows).collect();
            self.categories = view.category_options;
            self.loadouts = self.store.borrow().loadout_data()?;
            if self
                .loadout
                .as_ref()
                .is_some_and(|id| !self.loadouts.loadouts.iter().any(|l| &l.id == id))
            {
                self.loadout = self
                    .loadouts
                    .loadouts
                    .iter()
                    .find(|l| l.is_default)
                    .or_else(|| self.loadouts.loadouts.first())
                    .map(|l| l.id.clone());
            }
            if self
                .wishlist_loadout
                .as_ref()
                .is_some_and(|id| !self.loadouts.loadouts.iter().any(|l| &l.id == id))
            {
                self.wishlist_loadout = self
                    .loadouts
                    .loadouts
                    .iter()
                    .find(|l| l.is_default)
                    .or_else(|| self.loadouts.loadouts.first())
                    .map(|l| l.id.clone());
            }
            self.groups = self.store.borrow().gear_groups(&self.filter)?;
            self.refresh_subtotals()?;
            self.food.ingredients = self.store.borrow().ingredients()?;
            self.food.meals = self.store.borrow().meals()?;
            self.reload_trips(cx)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.error = None;
                self.flatten();
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn reload_changes(&mut self, changes: ChangeSet, cx: &mut Context<Self>) {
        // Every query model is derived from SQLite; propagated tables are included.
        if !changes.tables.is_empty() {
            self.reload(cx);
            self.reload_other_windows(cx);
        } else {
            self.error = None;
            cx.notify();
        }
    }
    fn command(&mut self, command: Command, cx: &mut Context<Self>) {
        let applied = self.store.borrow_mut().apply(command);
        match applied {
            Ok(changes) => {
                self.nudge_sync(cx);
                self.reload_changes(changes, cx);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn select(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.overlay_open() || self.area != "Gear" || self.wishlist {
            return;
        }
        let rows: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if let Entry::Item { row, .. } = e {
                    Some((i, row.id.clone()))
                } else {
                    None
                }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| rows.iter().position(|(_, r)| r == id));
        let index = match current {
            Some(i) => (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize,
            None => {
                if delta > 0 {
                    0
                } else {
                    rows.len() - 1
                }
            }
        };
        self.selected = Some(rows[index].1.clone());
        self.list.scroll_to_reveal_item(rows[index].0);
        cx.notify();
    }
    fn delete(&mut self, cx: &mut Context<Self>) {
        if self.overlay_open() || self.area != "Gear" || self.wishlist {
            return;
        }
        if let Some(id) = self.selected.clone() {
            self.command(Command::DeleteGear { id }, cx);
        }
    }
    fn undo(&mut self, redo: bool, cx: &mut Context<Self>) {
        if self.overlay_open() {
            return;
        }
        self.food.editor = None;
        let result = if redo {
            self.store.borrow_mut().redo()
        } else {
            self.store.borrow_mut().undo()
        };
        match result {
            Ok(_) => {
                self.nudge_sync(cx);
                self.reload(cx);
                self.reload_other_windows(cx);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn appearance(&mut self, appearance: Appearance, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance = appearance;
        strider_ui::set_appearance(appearance, window, cx);
        cx.notify();
    }
}
impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames.record();
        self.sync_chrome(_window, cx);
        let t = Theme::get(cx);
        let shell = self.render_shell(cx);
        let header = self.render_header(cx);
        let body = match self.area {
            "Gear" => {
                if self.wishlist {
                    self.render_wishlist(_window, cx)
                } else {
                    self.render_inventory(_window, cx)
                }
            }
            "Food" => self.render_food(_window, cx),
            "Settings" => self.render_settings(_window, cx),
            "Calendar" => self.render_calendar(cx),
            "Trips" | "Basecamp" => self.render_trips(_window, cx),
            _ => strider_ui::page_body(
                vec![
                    div()
                        .meta(t)
                        .child("Not part of this milestone.")
                        .into_any_element(),
                ],
                t,
            )
            .into_any_element(),
        };
        let overlay = self.render_overlay(_window, cx);
        div()
            .size_full()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                if this.area == "Trips" && this.trips.selected.is_some() {
                    for path in paths.paths() {
                        this.trip_file(path.clone(), cx);
                    }
                }
            }))
            .relative()
            .body()
            .bg(t.canvas)
            .text_color(t.ink)
            .track_focus(&self.focus)
            .key_context(KeyContext::parse(&menus::key_context(self.area)).unwrap_or_default())
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_overlays(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.select(1, cx)))
            .on_action(cx.listener(|this, _: &SelectPrevious, _, cx| this.select(-1, cx)))
            .on_action(cx.listener(|this, _: &DeleteSelected, _, cx| this.delete(cx)))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.undo(false, cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.undo(true, cx)))
            .on_action(cx.listener(|this, _: &EditSelected, window, cx| {
                if let Some(id) = this.selected.clone() {
                    this.edit(id, window, cx);
                }
            }))
            .when(self.area == "Gear" && !self.overlay_open(), |d| {
                d.on_action(cx.listener(|this, _: &AddGear, window, cx| this.add_gear(window, cx)))
            })
            .when(
                matches!(self.area, "Trips" | "Basecamp") && !self.overlay_open(),
                |d| {
                    d.on_action(
                        cx.listener(|this, _: &NewTrip, window, cx| {
                            this.new_trip_dialog(window, cx)
                        }),
                    )
                },
            )
            .when(self.can_import_gpx(), |d| {
                d.on_action(cx.listener(|this, _: &ImportGpx, _, cx| this.pick_trip_file(cx)))
            })
            .when(self.can_export_gpx(), |d| {
                d.on_action(cx.listener(|this, _: &ExportGpx, _, cx| this.export_trip_gpx(cx)))
            })
            .when(self.can_print() && !self.overlay_open(), |d| {
                d.on_action(
                    cx.listener(|this, _: &PrintTripSheet, _, cx| this.print_trip_sheet(cx)),
                )
            })
            .when(!self.signed_in(cx), |d| {
                d.on_action(cx.listener(|this, _: &SignIn, _, cx| this.sign_in(cx)))
            })
            .when(self.signed_in(cx), |d| {
                d.on_action(cx.listener(|this, _: &SignOut, _, cx| this.sign_out(cx)))
                    .on_action(cx.listener(|this, _: &SyncNow, _, cx| this.sync_now(cx)))
            })
            .on_action(cx.listener(|this, _: &ShowSyncStatus, _, cx| this.show_sync_status(cx)))
            .when(self.updates_enabled(cx), |d| {
                d.on_action(
                    cx.listener(|this, _: &CheckForUpdates, _, cx| this.check_for_updates(cx)),
                )
            })
            .on_action(
                cx.listener(|this, _: &CommandPalette, window, cx| this.toggle_palette(window, cx)),
            )
            .on_action(cx.listener(|this, _: &About, _, cx| {
                this.toast(format!(
                    "Strider Desktop {} \u{b7} {}",
                    env!("CARGO_PKG_VERSION"),
                    WEBSITE
                ));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.request_navigation(Navigation::Area("Settings"), window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowGear, window, cx| {
                this.request_navigation(Navigation::Area("Gear"), window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowFood, window, cx| {
                this.request_navigation(Navigation::Area("Food"), window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowTrips, window, cx| {
                this.request_navigation(Navigation::Area("Trips"), window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowCalendar, window, cx| {
                this.request_navigation(Navigation::Area("Calendar"), window, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowBasecamp, window, cx| {
                this.request_navigation(Navigation::Area("Basecamp"), window, cx)
            }))
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
            .on_action(cx.listener(|_, _: &Zoom, window, _| window.zoom_window()))
            .on_action(cx.listener(|_, _: &ToggleFullScreen, window, _| window.toggle_fullscreen()))
            .on_action(cx.listener(|this, _: &Find, window, cx| {
                if !this.overlay_open() {
                    let state = if this.area == "Food" {
                        this.food.search.clone()
                    } else if this.wishlist {
                        this.wishlist_query.clone()
                    } else {
                        this.filter_input.clone()
                    };
                    state.update(cx, |s, cx| s.focus(window, cx));
                }
            }))
            .on_action(cx.listener(|this, _: &Escape, window, cx| this.close_overlays(window, cx)))
            .on_action(cx.listener(|this, _: &Light, window, cx| {
                this.appearance(Appearance::Light, window, cx)
            }))
            .on_action(cx.listener(|this, _: &Dark, window, cx| {
                this.appearance(Appearance::Dark, window, cx)
            }))
            .on_action(cx.listener(|this, _: &System, window, cx| {
                this.appearance(Appearance::System, window, cx)
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(shell)
                    .child(header)
                    .child(body)
                    .when_some(self.error.clone(), |d, error| {
                        d.child(div().meta(t).child(error))
                    }),
            )
            .children(overlay)
            .children(self.render_trip_menu(cx))
            .when_some(self.toast.clone(), |d, (message, _)| {
                d.child(
                    div()
                        .absolute()
                        .bottom_6()
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            strider_ui::card(t)
                                .id("toast")
                                .px_4()
                                .py_3()
                                .shadow_md()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toast = None;
                                    cx.notify();
                                }))
                                .child(message),
                        ),
                )
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{AppView, DeleteSelected, EditSelected, SelectNext, Undo, menus};
    use gpui::{AppContext, TestAppContext, VisualTestContext, WindowOptions};
    use gpui_component::Root;
    use strider_store::{GearFilter, StatusFilter, Store, seed};
    #[test]
    fn workspace_options() {
        let parse = |args: &[&str]| {
            crate::WorkspaceOptions::parse(args.iter().map(|arg| (*arg).to_owned()))
        };
        assert_eq!(parse(&[]).expect("defaults").profile, "dev");
        let custom = parse(&["--profile", "test", "--reset"]).expect("custom profile");
        assert_eq!(custom.profile, "test");
        assert!(custom.reset);
        assert_eq!(
            parse(&[]).expect("defaults").server,
            "https://strider.thaiv.dev"
        );
        assert_eq!(
            parse(&["--server", "http://localhost:3100"])
                .expect("server")
                .server,
            "http://localhost:3100"
        );
        assert!(parse(&["--server"]).is_err());
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--profile", "--reset"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
    }
    #[gpui::test]
    fn gear_actions_delete_and_restore_fixture(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            strider_ui::initialize(cx);
            menus(cx);
        });
        let mut store = Store::in_memory().expect("in-memory store");
        seed(&mut store, 2000).expect("fixture");
        let before = store
            .gear_groups(&GearFilter {
                status: StatusFilter::All,
                query: String::new(),
            })
            .expect("groups");
        let mut view = None;
        let window = cx.update(|cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let gear = cx.new(|cx| AppView::new(store, window, cx));
                view = Some(gear.clone());
                cx.new(|cx| Root::new(gear, window, cx))
            })
            .expect("headless window")
        });
        let gear = view.expect("gear entity");
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        cx.dispatch_action(SelectNext);
        cx.dispatch_action(SelectNext);
        let selected = gear
            .read_with(&cx, |g, _| g.selected.clone())
            .expect("second row selected");
        assert_eq!(
            selected,
            before[0]
                .rows
                .iter()
                .filter(|r| r.status == "current")
                .nth(1)
                .expect("second current item")
                .id
        );
        cx.dispatch_action(EditSelected);
        assert!(gear.read_with(&cx, |g, _| g.editor.is_some()));
        cx.update(|window, cx| {
            gear.update(cx, |g, cx| {
                let weight = g.editor.as_ref().expect("editor").weight.clone();
                weight.update(cx, |s, cx| s.set_value("not a number", window, cx));
                g.save(window, cx);
                assert!(
                    g.editor
                        .as_ref()
                        .expect("invalid save stays open")
                        .weight_error
                );
            })
        });
        cx.simulate_keystrokes("escape");
        assert!(gear.read_with(&cx, |g, _| g.editor.is_none()));
        let old_count = gear.read_with(&cx, |g, _| g.totals.items);
        cx.dispatch_action(DeleteSelected);
        assert_eq!(gear.read_with(&cx, |g, _| g.totals.items), old_count - 1);
        assert!(
            !gear
                .read_with(&cx, |g, _| g
                    .store
                    .borrow()
                    .gear_groups(&GearFilter {
                        status: StatusFilter::All,
                        query: String::new()
                    })
                    .expect("groups"))
                .iter()
                .flat_map(|g| &g.rows)
                .any(|r| r.id == selected)
        );
        cx.dispatch_action(Undo);
        assert_eq!(
            gear.read_with(&cx, |g, _| g
                .store
                .borrow()
                .gear_groups(&GearFilter {
                    status: StatusFilter::All,
                    query: String::new()
                })
                .expect("groups")),
            before
        );
    }
}
impl AppView {
    fn choice(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        options: Vec<(String, String)>,
        cx: &Context<Self>,
        handler: impl Fn(&mut Self, String, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let view = cx.entity().downgrade();
        strider_ui::select(
            id,
            label,
            options,
            Theme::get(cx),
            move |value, window, cx| {
                let _ = view.update(cx, |this, cx| handler(this, value, window, cx));
            },
        )
        .into_any_element()
    }
    fn overlay_open(&self) -> bool {
        self.palette.is_some()
            || self.trip_menu.is_some()
            || self.trips.editor.is_some()
            || self.offline.sheet
            || self.devices.sheet
            || self.share.open
            || self.account.sheet
            || self.account.update.sheet
            || self.pending_navigation.is_some()
            || self.editor.is_some()
            || self.replacements.is_some()
            || self.manager.is_some()
            || self.food.ingredient_editor.is_some()
    }
    fn close_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = None;
        self.trip_menu = None;
        self.trips.editor = None;
        self.offline.sheet = false;
        self.devices.sheet = false;
        self.share.open = false;
        self.account.sheet = false;
        self.account.update.sheet = false;
        self.pending_navigation = None;
        self.editor = None;
        self.replacements = None;
        self.manager = None;
        self.food.ingredient_editor = None;
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn modal(
        &self,
        title: impl Into<SharedString>,
        content: AnyElement,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .child(
                div()
                    .id("scrim")
                    .absolute()
                    .inset_0()
                    .bg(t.scrim)
                    .on_click(cx.listener(|this, _, w, cx| this.close_overlays(w, cx))),
            )
            .child(
                strider_ui::sheet(
                    title,
                    content,
                    strider_ui::icon_button("close", Icon::X, t)
                        .on_click(cx.listener(|this, _, w, cx| this.close_overlays(w, cx))),
                    t,
                )
                .id("edit-sheet")
                .occlude()
                .max_h(relative(0.9))
                .overflow_y_scroll(),
            )
            .into_any_element()
    }
    fn render_overlay(&self, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        if self.palette.is_some() {
            Some(self.render_palette(cx))
        } else if self.trips.editor.is_some() {
            Some(self.render_trip_editor(cx))
        } else if self.offline.sheet {
            Some(self.render_map_sheet(cx))
        } else if self.devices.sheet {
            Some(self.render_attach_sheet(cx))
        } else if self.share.open {
            Some(self.render_share_sheet(cx))
        } else if self.account.sheet {
            Some(self.render_sync_sheet(cx))
        } else if self.account.update.sheet {
            Some(self.render_update_sheet(cx))
        } else if self.pending_navigation.is_some() {
            Some(self.render_navigation_guard(cx))
        } else if self.replacements.is_some() {
            Some(self.render_replacements(cx))
        } else if self.editor.is_some() {
            Some(self.render_gear_dialog(cx))
        } else if self.manager.is_some() {
            Some(self.render_manager(cx))
        } else if self.food.ingredient_editor.is_some() {
            Some(self.render_ingredient_dialog(window, cx))
        } else {
            None
        }
    }
}
fn input(
    value: impl Into<SharedString>,
    _multiline: bool,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> Entity<InputState> {
    let state = cx.new(|cx| {
        let mut state = InputState::new(window, cx);
        state.set_value(value.into(), window, cx);
        state
    });
    cx.subscribe_in(&state, window, |_, _, event, _, cx| {
        if matches!(event, InputEvent::Change) {
            cx.notify();
        }
    })
    .detach();
    state
}
fn value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).value().to_string()
}
fn optional(input: &Entity<InputState>, cx: &App) -> Option<String> {
    let s = value(input, cx);
    (!s.trim().is_empty()).then_some(s)
}
fn number(input: &Entity<InputState>, cx: &App) -> Result<Option<f64>, String> {
    let text = value(input, cx);
    if text.trim().is_empty() {
        return Ok(None);
    }
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0. && *v <= i32::MAX as f64)
        .map(Some)
        .ok_or_else(|| "Enter a number.".into())
}
fn integer(input: &Entity<InputState>, cx: &App, positive: bool) -> Result<i32, String> {
    value(input, cx)
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|v| if positive { *v > 0 } else { *v >= 0 })
        .ok_or_else(|| "Enter a number.".into())
}
fn decimal_input(
    input: &Entity<InputState>,
    cx: &App,
) -> Result<Option<strider_core::units::Dec>, String> {
    optional(input, cx)
        .map(|s| {
            s.trim()
                .parse::<strider_core::units::Dec>()
                .map_err(|_| "Enter a number.".into())
                .and_then(|d| {
                    if d >= 0.into() {
                        Ok(d)
                    } else {
                        Err("Enter a number.".into())
                    }
                })
        })
        .transpose()
}

fn textarea_input(
    value: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> Entity<gpui_component::input::TextareaState> {
    let state = cx.new(|cx| {
        let mut state = gpui_component::input::TextareaState::new(window, cx);
        state.set_value(value.into(), window, cx);
        state
    });
    cx.subscribe_in(&state, window, |_, _, event, _, cx| {
        if matches!(event, InputEvent::Change) {
            cx.notify();
        }
    })
    .detach();
    state
}

pub(crate) fn launch(store: Store, path: std::path::PathBuf, server: String) {
    gpui_platform::application().run(move |cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        menus(cx);
        cx.default_global::<windows::Registry>().workspace = Some(windows::Workspace {
            store: SharedStore::new(store),
            path,
            server,
            live: true,
        });
        windows::open_window(cx, windows::WindowSpec::default());
        // Closing the last window ends the app, as in a document-less tool.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}

#[cfg(test)]
#[path = "chrome_tests.rs"]
mod chrome_tests;
#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;
