//! Shared helpers for the headless tests of the offline and sharing screens.
use super::{AppView, menus};
use gpui::{AppContext, Entity, TestAppContext, VisualTestContext, WindowOptions};
use gpui_component::Root;
use std::path::PathBuf;
use strider_store::{Command, Store};

pub(crate) fn headless(cx: &mut TestAppContext) -> (Entity<AppView>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        menus(cx);
    });
    let mut store = Store::in_memory().expect("memory");
    strider_store::seed(&mut store, 24).expect("fixture");
    let mut view = None;
    let window = cx.update(|cx| {
        cx.open_window(WindowOptions::default(), |w, cx| {
            let root = cx.new(|cx| AppView::new(store, w, cx));
            view = Some(root.clone());
            cx.new(|cx| Root::new(root, w, cx))
        })
        .expect("headless window")
    });
    (
        view.expect("view"),
        VisualTestContext::from_window(window.into(), cx),
    )
}
/// A fresh directory under the system temp dir, removed by the caller.
pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("strider-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}
/// A short route near Boulder, long enough to produce a few tiles per zoom level.
pub(crate) const GPX: &str = r#"<?xml version="1.0"?><gpx version="1.1" creator="t"><trk><name>T</name><trkseg>
<trkpt lat="39.9500" lon="-105.3000"><ele>1800</ele></trkpt>
<trkpt lat="39.9600" lon="-105.2900"><ele>1900</ele></trkpt>
<trkpt lat="39.9700" lon="-105.2800"><ele>2000</ele></trkpt>
<trkpt lat="39.9800" lon="-105.2700"><ele>2100</ele></trkpt>
</trkseg></trk></gpx>"#;
/// Create a Trip with the sample route, open it, and return its id.
pub(crate) fn open_routed_trip(
    view: &Entity<AppView>,
    name: &str,
    cx: &mut VisualTestContext,
) -> String {
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.area = "Trips";
            g.command(
                Command::CreateTrip {
                    name: name.into(),
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
                .find(|t| t.name == name)
                .expect("created")
                .sync_id
                .clone();
            g.command(
                Command::AttachGpx {
                    trip: id.clone(),
                    filename: "r.gpx".into(),
                    bytes: GPX.as_bytes().to_vec(),
                },
                cx,
            );
            g.open_trip(id.clone(), cx);
            id
        })
    })
}
/// Run `f` on the view with a window.
pub(crate) fn with_view<R>(
    view: &Entity<AppView>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut AppView, &mut gpui::Window, &mut gpui::Context<AppView>) -> R,
) -> R {
    cx.update(|w, cx| view.update(cx, |g, cx| f(g, w, cx)))
}
