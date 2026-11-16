//! One GPUI action per user command, the key map, and the macOS menu bar.
//! Menu items disable themselves: GPUI asks whether the focused window handles the action, and the
//! view only registers handlers for commands that apply (see `AppView::render`).
use super::*;

actions!(
    strider,
    [
        About,
        OpenSettings,
        NewWindow,
        NewTrip,
        PrintTripSheet,
        CloseWindow,
        ShowGear,
        ShowFood,
        ShowTrips,
        ShowCalendar,
        ShowBasecamp,
        SyncNow,
        ShowSyncStatus,
        SignIn,
        SignOut,
        OpenWebsite,
        Minimize,
        Zoom,
        ToggleFullScreen,
        Hide,
        HideOthers,
        CommandPalette
    ]
);
/// Opened by Help, "Strider on the web".
pub(crate) const WEBSITE: &str = "https://strider.thaiv.dev";
/// Key contexts for the area-specific ⌘N.
pub(crate) fn key_context(area: &str) -> String {
    format!("Gear in-{}", area.to_lowercase())
}
pub(crate) fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("down", SelectNext, Some("Gear")),
        KeyBinding::new("up", SelectPrevious, Some("Gear")),
        KeyBinding::new("enter", EditSelected, Some("Gear")),
        KeyBinding::new("backspace", DeleteSelected, Some("Gear")),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-n", AddGear, Some("in-gear")),
        KeyBinding::new("cmd-n", NewTrip, Some("in-trips")),
        KeyBinding::new("cmd-n", NewTrip, Some("in-basecamp")),
        KeyBinding::new("cmd-shift-n", NewWindow, None),
        KeyBinding::new("cmd-o", ImportGpx, None),
        KeyBinding::new("cmd-e", ExportGpx, None),
        KeyBinding::new("cmd-p", PrintTripSheet, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-1", ShowGear, None),
        KeyBinding::new("cmd-2", ShowFood, None),
        KeyBinding::new("cmd-3", ShowTrips, None),
        KeyBinding::new("cmd-4", ShowCalendar, None),
        KeyBinding::new("cmd-5", ShowBasecamp, None),
        KeyBinding::new("cmd-r", SyncNow, None),
        KeyBinding::new("cmd-k", CommandPalette, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("cmd-z", Undo, Some("Gear")),
        KeyBinding::new("cmd-shift-z", Redo, Some("Gear")),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}
/// Every command the palette offers, with its label. Each is also in the menu bar.
pub(crate) fn commands() -> Vec<(&'static str, Box<dyn Action>)> {
    vec![
        ("About Strider", About.boxed_clone()),
        ("Settings", OpenSettings.boxed_clone()),
        ("New Window", NewWindow.boxed_clone()),
        ("New Gear", AddGear.boxed_clone()),
        ("New Trip", NewTrip.boxed_clone()),
        ("Import GPX", ImportGpx.boxed_clone()),
        ("Export GPX", ExportGpx.boxed_clone()),
        ("Print Trip Sheet", PrintTripSheet.boxed_clone()),
        ("Close Window", CloseWindow.boxed_clone()),
        ("Undo", Undo.boxed_clone()),
        ("Redo", Redo.boxed_clone()),
        ("Find", Find.boxed_clone()),
        ("Go to Gear", ShowGear.boxed_clone()),
        ("Go to Food", ShowFood.boxed_clone()),
        ("Go to Trips", ShowTrips.boxed_clone()),
        ("Go to Calendar", ShowCalendar.boxed_clone()),
        ("Go to Basecamp", ShowBasecamp.boxed_clone()),
        ("Appearance: Light", Light.boxed_clone()),
        ("Appearance: Dark", Dark.boxed_clone()),
        ("Appearance: System", System.boxed_clone()),
        ("Enter Full Screen", ToggleFullScreen.boxed_clone()),
        ("Sync Now", SyncNow.boxed_clone()),
        ("Show Sync Status", ShowSyncStatus.boxed_clone()),
        ("Sign In", SignIn.boxed_clone()),
        ("Sign Out", SignOut.boxed_clone()),
        ("Minimize", Minimize.boxed_clone()),
        ("Zoom", Zoom.boxed_clone()),
        ("Strider on the Web", OpenWebsite.boxed_clone()),
        ("Hide Strider", Hide.boxed_clone()),
        ("Quit Strider", Quit.boxed_clone()),
    ]
}
pub(crate) fn app_menus() -> Vec<Menu> {
    vec![
        Menu::new("Strider").items([
            MenuItem::action("About Strider", About),
            MenuItem::separator(),
            MenuItem::action("Settings\u{2026}", OpenSettings),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Strider", Hide),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::separator(),
            MenuItem::action("Quit Strider", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Window", NewWindow),
            MenuItem::action("New Gear", AddGear),
            MenuItem::action("New Trip", NewTrip),
            MenuItem::separator(),
            MenuItem::action("Import GPX\u{2026}", ImportGpx),
            MenuItem::action("Export GPX\u{2026}", ExportGpx),
            MenuItem::separator(),
            MenuItem::action("Print Trip Sheet\u{2026}", PrintTripSheet),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", gpui_component::input::Cut),
            MenuItem::action("Copy", gpui_component::input::Copy),
            MenuItem::action("Paste", gpui_component::input::Paste),
            MenuItem::action("Select All", gpui_component::input::SelectAll),
            MenuItem::separator(),
            MenuItem::action("Find", Find),
            MenuItem::separator(),
            MenuItem::action("Select Next Item", SelectNext),
            MenuItem::action("Select Previous Item", SelectPrevious),
            MenuItem::action("Edit Item", EditSelected),
            MenuItem::action("Delete Item", DeleteSelected),
        ]),
        Menu::new("View").items([
            MenuItem::action("Command Palette\u{2026}", CommandPalette),
            MenuItem::separator(),
            MenuItem::action("Gear", ShowGear),
            MenuItem::action("Food", ShowFood),
            MenuItem::action("Trips", ShowTrips),
            MenuItem::action("Calendar", ShowCalendar),
            MenuItem::action("Basecamp", ShowBasecamp),
            MenuItem::separator(),
            MenuItem::submenu(Menu::new("Appearance").items([
                MenuItem::action("Light", Light),
                MenuItem::action("Dark", Dark),
                MenuItem::action("System", System),
            ])),
            MenuItem::separator(),
            MenuItem::action("Enter Full Screen", ToggleFullScreen),
        ]),
        Menu::new("Sync").items([
            MenuItem::action("Sync Now", SyncNow),
            MenuItem::action("Show Sync Status", ShowSyncStatus),
            MenuItem::separator(),
            MenuItem::action("Sign In\u{2026}", SignIn),
            MenuItem::action("Sign Out", SignOut),
        ]),
        // macOS lists the open windows at the end of the menu named "Window".
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Zoom", Zoom),
        ]),
        Menu::new("Help").items([MenuItem::action("Strider on the Web", OpenWebsite)]),
    ]
}
/// Key bindings, the menu bar and the app-wide handlers (those that need no window).
pub(crate) fn menus(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &OpenWebsite, cx| cx.open_url(WEBSITE));
    // Deferred: the focused window is mid-dispatch here and cannot be asked for its frame.
    cx.on_action(|_: &NewWindow, cx| {
        cx.defer(|cx| {
            let area = windows::active_area(cx);
            windows::open_window(
                cx,
                windows::WindowSpec {
                    area,
                    ..Default::default()
                },
            );
        });
    });
    cx.intercept_keystrokes(|event, window, cx| {
        let key = event.keystroke.key.as_str();
        if !matches!(key, "up" | "down" | "enter") || event.keystroke.modifiers.modified() {
            return;
        }
        let handle = window.window_handle();
        let view = cx.try_global::<windows::Registry>().and_then(|r| {
            r.members
                .iter()
                .find(|m| m.window == handle)
                .and_then(|m| m.view.upgrade())
        });
        if let Some(view) = view
            && view.update(cx, |view, cx| view.palette_key(key, window, cx))
        {
            cx.stop_propagation();
        }
    })
    .detach();
    cx.set_menus(app_menus());
}
