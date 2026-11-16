//! Several windows over one workspace: the shared store handle, the window registry,
//! and the persisted frame and area.
use super::*;
use gpui_component::Root;
use std::{
    cell::{Ref, RefCell, RefMut},
    path::PathBuf,
    rc::Rc,
};

/// The one SQLite workspace, shared by every window on the main thread. Borrows are short and
/// never held across a call back into a view.
#[derive(Clone)]
pub(crate) struct SharedStore(Rc<RefCell<Store>>);
impl SharedStore {
    pub fn new(store: Store) -> Self {
        Self(Rc::new(RefCell::new(store)))
    }
    pub fn borrow(&self) -> Ref<'_, Store> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, Store> {
        self.0.borrow_mut()
    }
}
#[derive(Clone)]
pub(crate) struct Workspace {
    pub store: SharedStore,
    pub path: PathBuf,
    pub server: String,
    /// False in headless tests: no Keychain, tile cache or background sync is touched.
    pub live: bool,
}
pub(crate) struct Member {
    pub view: WeakEntity<AppView>,
    pub window: AnyWindowHandle,
}
/// Every open window, oldest first. The first owns sign-in and background sync.
#[derive(Default)]
pub(crate) struct Registry {
    pub members: Vec<Member>,
    pub workspace: Option<Workspace>,
}
impl Global for Registry {}

#[derive(Default)]
pub(crate) struct WindowSpec {
    pub area: Option<&'static str>,
    pub trip: Option<String>,
    pub bounds: Option<WindowBounds>,
}
/// The areas a window can open on, as saved.
const SAVED_AREAS: [&str; 6] = ["Gear", "Food", "Trips", "Calendar", "Basecamp", "Settings"];
pub(crate) fn area_named(name: &str) -> Option<&'static str> {
    SAVED_AREAS.into_iter().find(|a| *a == name)
}
const MIN_SIZE: (f32, f32) = (640., 480.);

/// A window frame as stored in the local settings: `x,y,width,height,mode`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SavedFrame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// `w` windowed, `m` maximized, `f` full screen.
    pub mode: char,
}
impl SavedFrame {
    pub fn from_bounds(bounds: WindowBounds) -> Self {
        let (mode, b) = match bounds {
            WindowBounds::Windowed(b) => ('w', b),
            WindowBounds::Maximized(b) => ('m', b),
            WindowBounds::Fullscreen(b) => ('f', b),
        };
        Self {
            x: f32::from(b.origin.x),
            y: f32::from(b.origin.y),
            w: f32::from(b.size.width),
            h: f32::from(b.size.height),
            mode,
        }
    }
    pub fn encode(&self) -> String {
        format!("{},{},{},{},{}", self.x, self.y, self.w, self.h, self.mode)
    }
    /// Rejects malformed, non-finite or implausibly small or large frames.
    pub fn decode(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.split(',').collect();
        let [x, y, w, h, mode] = parts.as_slice() else {
            return None;
        };
        let frame = Self {
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            w: w.parse().ok()?,
            h: h.parse().ok()?,
            mode: match *mode {
                "w" | "m" | "f" => mode.chars().next()?,
                _ => return None,
            },
        };
        let sane = [frame.x, frame.y, frame.w, frame.h]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= 30_000.);
        (sane && frame.w >= MIN_SIZE.0 && frame.h >= MIN_SIZE.1).then_some(frame)
    }
    pub fn bounds(&self) -> WindowBounds {
        let b = Bounds {
            origin: point(px(self.x), px(self.y)),
            size: size(px(self.w), px(self.h)),
        };
        match self.mode {
            'm' => WindowBounds::Maximized(b),
            'f' => WindowBounds::Fullscreen(b),
            _ => WindowBounds::Windowed(b),
        }
    }
    /// Whether any part of the frame is on a connected display (no displays: assume yes).
    pub fn on_screen(&self, cx: &App) -> bool {
        let displays = cx.displays();
        if displays.is_empty() {
            return true;
        }
        let me = self.bounds().get_bounds();
        displays.iter().any(|d| d.bounds().intersects(&me))
    }
}
const FRAME_KEY: &str = "window_frame";
const AREA_KEY: &str = "window_area";
impl Workspace {
    /// The saved frame, whether or not its display is still connected.
    pub fn saved_frame_unchecked(&self) -> Option<SavedFrame> {
        let text = self.store.borrow().state(FRAME_KEY).ok().flatten()?;
        SavedFrame::decode(&text)
    }
    pub fn saved_frame(&self, cx: &App) -> Option<SavedFrame> {
        self.saved_frame_unchecked().filter(|f| f.on_screen(cx))
    }
    pub fn saved_area(&self) -> Option<&'static str> {
        let name = self.store.borrow().state(AREA_KEY).ok().flatten()?;
        area_named(&name)
    }
}
/// The area of the focused window, for a new window to start on.
pub(crate) fn active_area(cx: &mut App) -> Option<&'static str> {
    let active = cx.active_window()?;
    let views: Vec<_> = cx
        .try_global::<Registry>()?
        .members
        .iter()
        .filter(|m| m.window == active)
        .filter_map(|m| m.view.upgrade())
        .collect();
    views.first().map(|v| v.read(cx).area)
}
/// Open another window over the shared store. Returns `None` before a workspace exists.
pub(crate) fn open_window(cx: &mut App, spec: WindowSpec) -> Option<WindowHandle<Root>> {
    let workspace = cx.try_global::<Registry>()?.workspace.clone()?;
    let first = cx
        .try_global::<Registry>()
        .is_none_or(|r| r.members.iter().all(|m| m.view.upgrade().is_none()));
    let bounds = spec.bounds.unwrap_or_else(|| {
        let restored = first.then(|| workspace.saved_frame(cx)).flatten();
        restored.map_or_else(
            || {
                // Offset from the focused window so a new one is visible as such.
                let base = cx.active_window().and_then(|w| {
                    w.update(cx, |_, window, _| window.window_bounds().get_bounds())
                        .ok()
                });
                match base {
                    Some(b) => WindowBounds::Windowed(Bounds {
                        origin: point(b.origin.x + px(28.), b.origin.y + px(28.)),
                        size: b.size,
                    }),
                    None => WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1180.), px(820.)),
                        cx,
                    )),
                }
            },
            |f| f.bounds(),
        )
    });
    let area = spec
        .area
        .or_else(|| first.then(|| workspace.saved_area()).flatten());
    let trip = spec.trip;
    cx.open_window(
        WindowOptions {
            window_bounds: Some(bounds),
            window_min_size: Some(size(px(MIN_SIZE.0), px(MIN_SIZE.1))),
            titlebar: Some(TitlebarOptions {
                title: Some("Strider".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.), px(22.))),
            }),
            app_owns_titlebar_drag: true,
            ..Default::default()
        },
        move |window, cx| {
            if first {
                strider_ui::set_appearance(Appearance::System, window, cx);
            }
            let view = cx.new(|cx| {
                let mut view = AppView::build(workspace.store.clone(), window, cx);
                if workspace.live {
                    view.configure_tiles(cx);
                    if first {
                        view.configure_account(
                            workspace.path.clone(),
                            workspace.server.clone(),
                            cx,
                        );
                    } else {
                        view.offline.backend.db_path = workspace.path.clone();
                        view.start_polling(cx);
                    }
                }
                if let Some(area) = area {
                    view.area = area;
                    view.reload(cx);
                }
                if let Some(id) = trip {
                    view.open_trip(id, cx);
                }
                view
            });
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .ok()
}
impl AppView {
    /// Register this view so other windows can reach it, and hand sync to a surviving window
    /// when this one closes.
    pub(crate) fn register(&mut self, window: &Window, cx: &mut Context<Self>) {
        let me = cx.weak_entity();
        let id = cx.entity_id();
        cx.default_global::<Registry>().members.push(Member {
            view: me,
            window: window.window_handle(),
        });
        let release = cx.on_release(move |this, cx| {
            let was_first;
            {
                let registry = cx.default_global::<Registry>();
                was_first = registry
                    .members
                    .first()
                    .is_some_and(|m| m.view.entity_id() == id);
                registry.members.retain(|m| m.view.entity_id() != id);
            }
            if !was_first || (!this.account.syncing() && this.account.client.is_none()) {
                return;
            }
            let next = cx
                .default_global::<Registry>()
                .members
                .iter()
                .find_map(|m| m.view.upgrade());
            if let Some(next) = next {
                let account = std::mem::take(&mut this.account);
                next.update(cx, |next, cx| next.adopt_account(account, cx));
            }
        });
        self._subscriptions.push(release);
    }
    /// The window that owns sync, when it is not this one.
    pub(crate) fn primary(&self, cx: &App) -> Option<(Entity<AppView>, AnyWindowHandle)> {
        let first = cx.try_global::<Registry>()?.members.first()?;
        if first.view.entity_id() == self.entity_id {
            return None;
        }
        Some((first.view.upgrade()?, first.window))
    }
    /// The right-click menu on a Trip row: one item, opening the Trip in its own window.
    pub(crate) fn render_trip_menu(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let (id, at) = self.trip_menu.clone()?;
        let t = Theme::get(cx);
        Some(
            div()
                .absolute()
                .inset_0()
                .child(
                    div()
                        .id("trip-menu-scrim")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_click(cx.listener(|this, _, w, cx| this.close_overlays(w, cx)))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, w, cx| this.close_overlays(w, cx)),
                        ),
                )
                .child(
                    strider_ui::card(t)
                        .absolute()
                        .left(at.x)
                        .top(at.y)
                        .shadow_md()
                        .p_1()
                        .child(
                            strider_ui::button(
                                "open-in-new-window",
                                "Open in New Window",
                                ButtonVariant::Ghost,
                                t,
                            )
                            .on_click(cx.listener(
                                move |this, _, w, cx| {
                                    this.close_overlays(w, cx);
                                    let spec = WindowSpec {
                                        trip: Some(id.clone()),
                                        ..Default::default()
                                    };
                                    cx.defer(move |cx| {
                                        open_window(cx, spec);
                                    });
                                },
                            )),
                        ),
                )
                .into_any_element(),
        )
    }
    /// Tell every other window to reload from the shared store.
    pub(crate) fn reload_other_windows(&self, cx: &mut Context<Self>) {
        let me = self.entity_id;
        let others: Vec<Entity<AppView>> = cx
            .try_global::<Registry>()
            .map(|r| {
                r.members
                    .iter()
                    .filter(|m| m.view.entity_id() != me)
                    .filter_map(|m| m.view.upgrade())
                    .collect()
            })
            .unwrap_or_default();
        for view in others {
            view.update(cx, |view, cx| view.reload(cx));
        }
    }
    /// Window title: the open Trip's name, otherwise the area.
    pub(crate) fn window_title_text(&self) -> String {
        match &self.trips.detail {
            Some(d) => {
                let name = trips::text(&d.fields, "name");
                if name.is_empty() {
                    self.area.to_owned()
                } else {
                    name
                }
            }
            None => self.area.to_owned(),
        }
    }
    /// Keep the window title current and save the frame and area (debounced) when they change.
    pub(crate) fn sync_chrome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.window_title_text();
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title = title;
        }
        let seen = format!(
            "{}|{}",
            SavedFrame::from_bounds(window.window_bounds()).encode(),
            self.area
        );
        if self.seen_frame.as_deref() == Some(seen.as_str()) {
            return;
        }
        self.seen_frame = Some(seen);
        if self.frame_pending {
            return;
        }
        self.frame_pending = true;
        #[cfg(not(test))]
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(400))
                .await;
            let _ = this.update_in(cx, |this, window, _| this.persist_frame(window));
        })
        .detach();
        #[cfg(test)]
        {
            let _ = cx;
            self.persist_frame(window);
        }
    }
    pub(crate) fn persist_frame(&mut self, window: &Window) {
        self.frame_pending = false;
        let frame = SavedFrame::from_bounds(window.window_bounds()).encode();
        let store = self.store.borrow();
        // Local, unsynced settings: a failed write only means the next launch uses defaults.
        let _ = store.set_state(FRAME_KEY, &frame);
        let _ = store.set_state(AREA_KEY, self.area);
    }
}
#[cfg(test)]
mod tests {
    use super::{SavedFrame, area_named};
    use gpui::WindowBounds;
    #[test]
    fn frames_round_trip_and_reject_nonsense() {
        let frame = SavedFrame {
            x: -20.5,
            y: 40.,
            w: 1180.,
            h: 820.,
            mode: 'w',
        };
        assert_eq!(SavedFrame::decode(&frame.encode()), Some(frame));
        for bad in [
            "",
            "1,2,3",
            "0,0,100,100,w",
            "0,0,1000,800,x",
            "0,0,NaN,800,w",
            "0,0,inf,800,w",
            "99999,0,1000,800,w",
            "a,b,c,d,w",
        ] {
            assert_eq!(SavedFrame::decode(bad), None, "{bad}");
        }
        assert_eq!(area_named("Trips"), Some("Trips"));
        assert_eq!(area_named("Nowhere"), None);
        assert!(matches!(
            SavedFrame { mode: 'f', ..frame }.bounds(),
            WindowBounds::Fullscreen(_)
        ));
    }
}
