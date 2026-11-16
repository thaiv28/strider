//! Headless tests of the menu bar, key map, command palette, multiple windows and the print sheet.
use super::test_support::{headless, open_routed_trip, with_view};
use super::{AppView, ExportGpx, ImportGpx, menus, palette, print, windows};
use gpui::{
    Action, AppContext, Entity, MenuItem, TestAppContext, VisualTestContext, WindowOptions,
};
use gpui_component::Root;
use rusqlite::types::Value;
use std::collections::BTreeSet;
use strider_store::{Command, Store};

fn action_names(items: &[MenuItem], out: &mut BTreeSet<String>) {
    for item in items {
        match item {
            MenuItem::Action { action, .. } => {
                out.insert(action.name().to_owned());
            }
            MenuItem::Submenu(menu) => action_names(&menu.items, out),
            _ => {}
        }
    }
}
#[test]
fn menu_bar_has_the_standard_menus_and_every_palette_command() {
    let bar = menus::app_menus();
    let titles: Vec<_> = bar.iter().map(|m| m.name.to_string()).collect();
    assert_eq!(
        titles,
        ["Strider", "File", "Edit", "View", "Sync", "Window", "Help"]
    );
    let mut reachable = BTreeSet::new();
    for menu in &bar {
        action_names(&menu.items, &mut reachable);
    }
    for (label, action) in menus::commands() {
        assert!(
            reachable.contains(action.name()),
            "{label} is not in the menu bar"
        );
    }
    // The standard items, by menu.
    let names_in = |title: &str| {
        let menu = bar.iter().find(|m| m.name == title).expect("menu");
        let mut names = BTreeSet::new();
        action_names(&menu.items, &mut names);
        names
    };
    for (menu, expected) in [
        ("Strider", vec!["About", "OpenSettings", "Hide", "Quit"]),
        (
            "File",
            vec![
                "NewWindow",
                "AddGear",
                "NewTrip",
                "ImportGpx",
                "ExportGpx",
                "PrintTripSheet",
                "CloseWindow",
            ],
        ),
        (
            "Edit",
            vec!["Undo", "Redo", "Cut", "Copy", "Paste", "SelectAll", "Find"],
        ),
        (
            "View",
            vec![
                "ShowGear",
                "ShowFood",
                "ShowTrips",
                "ShowCalendar",
                "ShowBasecamp",
                "Light",
                "Dark",
                "System",
                "ToggleFullScreen",
            ],
        ),
        (
            "Sync",
            vec!["SyncNow", "ShowSyncStatus", "SignIn", "SignOut"],
        ),
        ("Window", vec!["Minimize", "Zoom"]),
        ("Help", vec!["OpenWebsite"]),
    ] {
        let have = names_in(menu);
        for name in expected {
            assert!(
                have.iter().any(|h| h.ends_with(&format!("::{name}"))),
                "{menu} lacks {name}: {have:?}"
            );
        }
    }
    assert_eq!(menus::WEBSITE, "https://strider.thaiv.dev");
}
#[gpui::test]
fn the_keyboard_map_navigates_and_opens_settings(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    for (keys, area) in [
        ("cmd-2", "Food"),
        ("cmd-3", "Trips"),
        ("cmd-4", "Calendar"),
        ("cmd-5", "Basecamp"),
        ("cmd-1", "Gear"),
        ("cmd-,", "Settings"),
    ] {
        cx.simulate_keystrokes(keys);
        assert_eq!(view.read_with(&cx, |g, _| g.area), area, "{keys}");
    }
    // The key context follows the area, so cmd-n is New Gear in Gear and New Trip in Trips.
    cx.simulate_keystrokes("cmd-1 cmd-n");
    assert!(view.read_with(&cx, |g, _| g.editor.is_some()), "New Gear");
    cx.simulate_keystrokes("escape cmd-3 cmd-n");
    assert!(
        view.read_with(&cx, |g, _| g.trips.editor.is_some()),
        "New Trip"
    );
    cx.simulate_keystrokes("escape");
    assert!(view.read_with(&cx, |g, _| !g.overlay_open()));
}
fn available(cx: &mut VisualTestContext, action: &dyn Action) -> bool {
    cx.run_until_parked();
    cx.update(|window, cx| window.is_action_available(action, cx))
}
#[gpui::test]
fn menu_items_disable_when_they_do_not_apply(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    assert!(!available(&mut cx, &ExportGpx), "no Trip open");
    assert!(!available(&mut cx, &ImportGpx));
    assert!(!available(&mut cx, &menus::PrintTripSheet));
    assert!(!available(&mut cx, &menus::NewTrip), "Gear area");
    assert!(available(&mut cx, &super::AddGear));
    assert!(available(&mut cx, &menus::SignIn), "signed out");
    assert!(!available(&mut cx, &menus::SignOut));
    assert!(!available(&mut cx, &menus::SyncNow));
    open_routed_trip(&view, "Routed", &mut cx);
    assert!(available(&mut cx, &ExportGpx), "a Trip with a route");
    assert!(available(&mut cx, &ImportGpx));
    assert!(available(&mut cx, &menus::PrintTripSheet));
    assert!(available(&mut cx, &menus::NewTrip));
    assert!(!available(&mut cx, &super::AddGear), "Trips area");
    // A Trip without a route cannot export, but can still be printed.
    let bare = with_view(&view, &mut cx, |g, _, cx| {
        g.command(
            Command::CreateTrip {
                name: "Bare".into(),
                trail: None,
                loadout: None,
            },
            cx,
        );
        let id = g
            .trips
            .list
            .upcoming
            .iter()
            .chain(&g.trips.list.in_progress)
            .chain(&g.trips.list.past)
            .find(|t| t.name == "Bare")
            .expect("created")
            .sync_id
            .clone();
        g.open_trip(id.clone(), cx);
        id
    });
    assert!(!bare.is_empty());
    assert!(!available(&mut cx, &ExportGpx), "no route");
    assert!(available(&mut cx, &menus::PrintTripSheet));
}
#[gpui::test]
fn palette_lists_commands_and_go_to_entries_and_runs_them(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let trip = open_routed_trip(&view, "Granite Basin Loop", &mut cx);
    with_view(&view, &mut cx, |g, w, cx| {
        g.request_navigation(super::Navigation::Area("Gear"), w, cx)
    });
    cx.simulate_keystrokes("cmd-k");
    assert!(
        view.read_with(&cx, |g, _| g.palette.is_some()),
        "cmd-k opens"
    );
    let labels = view.read_with(&cx, |g, _| {
        g.palette
            .as_ref()
            .expect("palette")
            .entries
            .iter()
            .map(|e| e.label.clone())
            .collect::<Vec<_>>()
    });
    assert!(labels.iter().any(|l| l == "Go to Trips"));
    assert!(labels.iter().any(|l| l == "Go to Trip: Granite Basin Loop"));
    assert!(labels.iter().any(|l| l.starts_with("Go to Gear: ")));
    assert!(labels.iter().any(|l| l.starts_with("Go to Meal: ")));
    assert!(
        !labels.iter().any(|l| l == "Export GPX"),
        "commands that do not apply are not offered"
    );
    cx.simulate_keystrokes("down");
    assert_eq!(
        view.read_with(&cx, |g, _| g.palette.as_ref().map(|p| p.selected)),
        Some(1)
    );
    cx.simulate_keystrokes("up");
    assert_eq!(
        view.read_with(&cx, |g, _| g.palette.as_ref().map(|p| p.selected)),
        Some(0)
    );
    // Typing narrows by subsequence; Return runs the top result.
    let set_query = |view: &Entity<AppView>, cx: &mut VisualTestContext, text: &str| {
        cx.update(|w, cx| {
            view.update(cx, |g, cx| {
                let input = g.palette.as_ref().expect("palette").input.clone();
                input.update(cx, |s, cx| s.set_value(text.to_owned(), w, cx));
            })
        })
    };
    set_query(&view, &mut cx, "granbas");
    let top = view.read_with(&cx, |g, cx| {
        let first = g.palette_results(cx)[0];
        g.palette.as_ref().expect("palette").entries[first]
            .label
            .clone()
    });
    assert_eq!(top, "Go to Trip: Granite Basin Loop");
    cx.simulate_keystrokes("enter");
    assert!(
        view.read_with(&cx, |g, _| g.palette.is_none()),
        "Return closes"
    );
    assert_eq!(view.read_with(&cx, |g, _| g.area), "Trips");
    assert_eq!(
        view.read_with(&cx, |g, _| g.trips.selected.clone()),
        Some(trip)
    );
    // A command runs as its action; Esc closes without running anything.
    cx.simulate_keystrokes("cmd-k");
    set_query(&view, &mut cx, "gotocal");
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(&cx, |g, _| g.area), "Calendar");
    cx.simulate_keystrokes("cmd-k");
    set_query(&view, &mut cx, "gotofood");
    cx.simulate_keystrokes("escape");
    assert!(
        view.read_with(&cx, |g, _| g.palette.is_none()),
        "Esc closes"
    );
    assert_eq!(
        view.read_with(&cx, |g, _| g.area),
        "Calendar",
        "nothing ran"
    );
    // Gear and meals navigate to their areas.
    cx.simulate_keystrokes("cmd-k");
    set_query(&view, &mut cx, "gotogear:");
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(&cx, |g, _| g.area), "Gear");
    assert!(
        view.read_with(&cx, |g, _| g.selected.is_some()),
        "item selected"
    );
    cx.simulate_keystrokes("cmd-k");
    set_query(&view, &mut cx, "gotomeal:");
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(&cx, |g, _| g.area), "Food");
    assert!(
        view.read_with(&cx, |g, _| g.food.editor.is_some()),
        "meal opened"
    );
    let _ = palette::rank;
}
fn two_windows(
    cx: &mut TestAppContext,
) -> (
    Entity<AppView>,
    VisualTestContext,
    Entity<AppView>,
    VisualTestContext,
) {
    cx.update(|cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        menus(cx);
    });
    let mut store = Store::in_memory().expect("memory");
    strider_store::seed(&mut store, 24).expect("fixture");
    let shared = windows::SharedStore::new(store);
    let open = |cx: &mut TestAppContext| {
        let mut view = None;
        let shared = shared.clone();
        let window = cx.update(|cx| {
            cx.open_window(WindowOptions::default(), |w, cx| {
                let root = cx.new(|cx| AppView::build(shared, w, cx));
                view = Some(root.clone());
                cx.new(|cx| Root::new(root, w, cx))
            })
            .expect("window")
        });
        (
            view.expect("view"),
            VisualTestContext::from_window(window.into(), cx),
        )
    };
    let (a, ca) = open(cx);
    let (b, cb) = open(cx);
    (a, ca, b, cb)
}
#[gpui::test]
fn windows_share_one_store_and_one_undo_history(cx: &mut TestAppContext) {
    let (a, mut ca, b, mut cb) = two_windows(cx);
    ca.run_until_parked();
    cb.run_until_parked();
    let trips = |v: &Entity<AppView>, cx: &VisualTestContext| {
        v.read_with(cx, |g, _| {
            g.trips.list.upcoming.len() + g.trips.list.in_progress.len() + g.trips.list.past.len()
        })
    };
    let before = trips(&b, &cb);
    with_view(&a, &mut ca, |g, _, cx| {
        g.command(
            Command::CreateTrip {
                name: "From window A".into(),
                trail: None,
                loadout: None,
            },
            cx,
        )
    });
    assert_eq!(trips(&b, &cb), before + 1, "B reloads on A's ChangeSet");
    // B edits, A sees it; and undo from either window walks the one history.
    with_view(&b, &mut cb, |g, _, cx| {
        g.command(
            Command::CreateTrip {
                name: "From window B".into(),
                trail: None,
                loadout: None,
            },
            cx,
        )
    });
    assert_eq!(trips(&a, &ca), before + 2);
    cb.simulate_keystrokes("cmd-z");
    assert_eq!(
        trips(&a, &ca),
        before + 1,
        "undo in B reverts B's edit in A"
    );
    ca.simulate_keystrokes("cmd-z");
    assert_eq!(trips(&b, &cb), before, "undo in A reverts A's edit in B");
    ca.simulate_keystrokes("cmd-shift-z");
    assert_eq!(trips(&b, &cb), before + 1, "redo is global too");
    // Window areas are independent, and titles follow the area or the open Trip.
    with_view(&a, &mut ca, |g, _, cx| {
        g.area = "Food";
        cx.notify();
    });
    ca.run_until_parked();
    cb.run_until_parked();
    assert_eq!(a.read_with(&ca, |g, _| g.window_title.clone()), "Food");
    assert_eq!(b.read_with(&cb, |g, _| g.window_title.clone()), "Gear");
    let id = open_routed_trip(&b, "Titled Trip", &mut cb);
    cb.run_until_parked();
    assert_eq!(
        b.read_with(&cb, |g, _| g.window_title.clone()),
        "Titled Trip"
    );
    assert!(!id.is_empty());
    // Only the first window owns sign-in and sync.
    assert!(a.read_with(&ca, |g, cx| g.primary(cx).is_none()));
    assert!(b.read_with(&cb, |g, cx| g.primary(cx).is_some()));
}
#[gpui::test]
fn closing_the_first_window_passes_sync_to_the_next(cx: &mut TestAppContext) {
    let (a, mut ca, b, cb) = two_windows(cx);
    ca.run_until_parked();
    cb.run_until_parked();
    assert!(b.read_with(&cb, |g, cx| g.primary(cx).is_some()));
    ca.update(|window, _| window.remove_window());
    ca.run_until_parked();
    drop(a);
    cb.run_until_parked();
    assert!(
        b.read_with(&cb, |g, cx| g.primary(cx).is_none()),
        "the survivor is first in the registry"
    );
}
#[gpui::test]
fn the_trip_row_menu_opens_and_closes(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let at = gpui::point(gpui::px(120.), gpui::px(200.));
    with_view(&view, &mut cx, |g, _, cx| {
        g.area = "Trips";
        g.trip_menu = Some(("any".into(), at));
        cx.notify();
    });
    cx.run_until_parked();
    assert!(view.read_with(&cx, |g, _| g.overlay_open()));
    assert!(with_view(&view, &mut cx, |g, _, cx| g
        .render_trip_menu(cx)
        .is_some()));
    cx.simulate_keystrokes("escape");
    assert!(view.read_with(&cx, |g, _| g.trip_menu.is_none()));
}
#[gpui::test]
fn open_window_opens_another_view_over_the_shared_store(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_routed_trip(&view, "Own Window Trip", &mut cx);
    let store = view.read_with(&cx, |g, _| g.store.clone());
    cx.update(|_, cx| {
        cx.default_global::<windows::Registry>().workspace = Some(windows::Workspace {
            store,
            path: std::path::PathBuf::new(),
            server: String::new(),
            live: false,
        });
        windows::open_window(
            cx,
            windows::WindowSpec {
                trip: Some(id.clone()),
                ..Default::default()
            },
        )
        .expect("second window");
    });
    cx.run_until_parked();
    assert_eq!(cx.update(|_, cx| cx.windows().len()), 2);
    let second = cx.update(|_, cx| {
        cx.default_global::<windows::Registry>()
            .members
            .last()
            .and_then(|m| m.view.upgrade())
            .expect("second view")
    });
    assert_eq!(second.read_with(&cx, |g, _| g.area), "Trips");
    assert_eq!(
        second.read_with(&cx, |g, _| g.trips.selected.clone()),
        Some(id)
    );
    // It shares the store: an edit in the first window shows in the second.
    with_view(&view, &mut cx, |g, _, cx| {
        g.command(
            Command::CreateTrip {
                name: "Seen everywhere".into(),
                trail: None,
                loadout: None,
            },
            cx,
        )
    });
    assert!(second.read_with(&cx, |g, _| {
        g.trips
            .list
            .upcoming
            .iter()
            .chain(&g.trips.list.in_progress)
            .chain(&g.trips.list.past)
            .any(|t| t.name == "Seen everywhere")
    }));
}
#[gpui::test]
fn the_frame_and_area_are_saved_and_restored(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    with_view(&view, &mut cx, |g, _, cx| {
        g.area = "Trips";
        cx.notify();
    });
    cx.run_until_parked();
    let store = view.read_with(&cx, |g, _| g.store.clone());
    assert_eq!(
        store.borrow().state("window_area").expect("read"),
        Some("Trips".into()),
        "the area is saved when it changes"
    );
    let saved = store
        .borrow()
        .state("window_frame")
        .expect("read")
        .expect("frame saved");
    assert!(windows::SavedFrame::decode(&saved).is_some(), "{saved}");
    // A later launch reads both back (the frame must be on a connected display).
    store
        .borrow()
        .set_state("window_frame", "10,20,900,700,w")
        .expect("set");
    store
        .borrow()
        .set_state("window_area", "Food")
        .expect("set");
    let workspace = windows::Workspace {
        store: store.clone(),
        path: std::path::PathBuf::new(),
        server: String::new(),
        live: false,
    };
    assert_eq!(workspace.saved_area(), Some("Food"));
    assert!(workspace.saved_frame_unchecked().is_some());
    store
        .borrow()
        .set_state("window_area", "Nowhere")
        .expect("set");
    assert_eq!(workspace.saved_area(), None, "unknown areas are ignored");
    store
        .borrow()
        .set_state("window_frame", "garbage")
        .expect("set");
    assert!(workspace.saved_frame_unchecked().is_none());
}
#[gpui::test]
fn the_print_sheet_follows_the_trip_and_renders_a_pdf(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_routed_trip(&view, "Sheet Trip", &mut cx);
    with_view(&view, &mut cx, |g, _, cx| {
        g.command(
            Command::UpdateTrip {
                trip: id.clone(),
                patch: strider_store::Row::from([
                    ("start_date".into(), Value::Text("2026-06-03".into())),
                    ("nights".into(), Value::Integer(3)),
                    ("trailhead".into(), Value::Text("Mattole".into())),
                    (
                        "permit_notes".into(),
                        Value::Text("Bear canister required.".into()),
                    ),
                ]),
            },
            cx,
        )
    });
    let sheet = view.read_with(&cx, |g, _| {
        print::build_sheet(g.trips.detail.as_ref().expect("detail"), print::palette())
    });
    assert_eq!(sheet.name, "Sheet Trip");
    assert_eq!(sheet.when, "Jun 3\u{2013}6, 2026 \u{b7} 3 nights");
    assert!(sheet.summary.contains("from Mattole"), "{}", sheet.summary);
    assert_eq!(
        sheet.permit_notes.as_deref(),
        Some("Bear canister required.")
    );
    assert_eq!(sheet.totals.day, "Total");
    assert_eq!(sheet.track.len(), 4, "the route is drawn");
    let pdf = strider_print::render_pdf_scaled(&sheet, None, 1).expect("pdf");
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(print::pdf_filename("Sheet Trip"), "Sheet-Trip.pdf");
    assert_eq!(print::pdf_filename("  "), "route.pdf");
    assert_eq!(print::pdf_filename("a/b c.gpx"), "a-b-c.gpx.pdf");
}
