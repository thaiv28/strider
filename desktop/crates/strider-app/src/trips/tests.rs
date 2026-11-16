use super::{AddTripFood, EditKind, RouteClick};
use crate::app::AppView;
use gpui::{AppContext, Entity, TestAppContext, VisualTestContext, WindowOptions};
use gpui_component::Root;
use strider_store::{Command, Store};

fn headless(cx: &mut TestAppContext) -> (Entity<AppView>, VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        strider_ui::initialize(cx);
        crate::app::menus(cx);
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
/// A longer parseGpx case from the shared golden fixture, with its expected stats.
fn golden_gpx() -> (Vec<u8>, f64, i64) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/domain/gpx.json")).expect("json");
    let case = &cases.as_array().expect("array")[11];
    (
        case["args"][0].as_str().expect("xml").as_bytes().to_vec(),
        case["out"]["distanceMi"].as_f64().expect("distance"),
        case["out"]["elevationGainFt"].as_i64().expect("gain"),
    )
}
fn open_new_trip(view: &Entity<AppView>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.area = "Trips";
            let loadout = g.loadouts.loadouts.first().map(|l| l.id.clone());
            g.command(
                Command::CreateTrip {
                    name: "Headless trip".into(),
                    trail: None,
                    loadout,
                },
                cx,
            );
            let id = g
                .all_trips()
                .iter()
                .find(|t| t.name == "Headless trip")
                .expect("created")
                .sync_id
                .clone();
            g.open_trip(id.clone(), cx);
            id
        })
    })
}
#[gpui::test]
fn trip_from_loadout_gpx_campsite_meal_undo_redo(cx: &mut TestAppContext) {
    trip_flow(cx);
}
fn trip_flow(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_new_trip(&view, &mut cx);
    cx.update(|_, cx| {
        view.update(cx, |g, _| {
            let d = g.trips.detail.as_ref().expect("detail");
            assert!(!d.gear.is_empty(), "loadout seeds trip gear");
            assert!(d.weights.base > 0, "gear weights shown");
        })
    });
    let (bytes, miles, gain) = golden_gpx();
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.command(
                Command::AttachGpx {
                    trip: id.clone(),
                    filename: "r.gpx".into(),
                    bytes,
                },
                cx,
            );
            let d = g.trips.detail.as_ref().expect("detail");
            assert_eq!(d.route.distance_mi, Some(miles));
            assert_eq!(d.route.elevation_ft, Some(gain as _));
        })
    });
    // A click on the profile emits a distance; the app turns it into a campsite.
    let profile = view.read_with(&cx, |g, _| g.trips.profile.clone());
    cx.update(|_, cx| profile.update(cx, |p, cx| p.click_distance(1.0, cx)));
    cx.run_until_parked();
    view.read_with(&cx, |g, _| {
        let d = g.trips.detail.as_ref().expect("detail");
        assert_eq!(d.campsites.len(), 1);
        assert_eq!(
            d.segments.len(),
            2,
            "campsite splits the route into day segments"
        );
    });
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.store.connection().execute_batch("INSERT INTO ingredient(sync_id,name,kcal_per_100g) VALUES('tti','Rice','350');INSERT INTO meal(sync_id,name,base_servings,water_ml) VALUES('ttm','Dinner',2,1000);INSERT INTO meal_ingredient(sync_id,meal_id,ingredient_id,snapshot_name,snapshot_kcal_per_100g,amount_g) VALUES('ttl','ttm','tti','Rice','350',200)").expect("sql");
            g.command(Command::AddTripMeal { trip: id.clone(), day: 1, entry: AddTripFood::Meal { meal: "ttm".into(), servings: 1. } }, cx);
            let d = g.trips.detail.as_ref().expect("detail");
            assert!(!d.shopping.is_empty() && d.food.total_g > 0);
            g.undo(false, cx);
            assert!(g.trips.detail.as_ref().expect("detail").shopping.is_empty());
            g.undo(false, cx);
            assert_eq!(g.trips.detail.as_ref().expect("detail").campsites.len(), 0);
            g.undo(true, cx);
            assert_eq!(g.trips.detail.as_ref().expect("detail").segments.len(), 2);
            g.undo(true, cx);
            assert!(!g.trips.detail.as_ref().expect("detail").shopping.is_empty());
        })
    });
    cx.run_until_parked();
}
#[gpui::test]
fn view_only_trip_has_no_edit_paths(cx: &mut TestAppContext) {
    view_only_flow(cx);
}
fn view_only_flow(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_new_trip(&view, &mut cx);
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            g.store
                .connection()
                .execute("UPDATE trip SET access='view' WHERE sync_id=?1", [&id])
                .expect("access");
            g.reload(cx);
            assert_eq!(g.trips.detail.as_ref().expect("detail").access, "view");
            g.edit_trip(EditKind::Details, w, cx);
            assert!(g.trips.editor.is_none(), "no edit dialog for viewers");
            g.route_click(
                &RouteClick {
                    distance: 0.5,
                    campsite: None,
                },
                cx,
            );
            assert!(
                g.trips
                    .detail
                    .as_ref()
                    .expect("detail")
                    .campsites
                    .is_empty()
            );
        })
    });
    cx.run_until_parked(); // renders the read-only page
}
fn set_access(view: &Entity<AppView>, id: &str, access: &str, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.store
                .connection()
                .execute(
                    "UPDATE trip SET access=?1, trip_report='private' WHERE sync_id=?2",
                    [access, id],
                )
                .expect("access");
            g.reload(cx);
        })
    });
    cx.run_until_parked();
}
fn has(cx: &mut VisualTestContext, selector: &'static str) -> bool {
    cx.debug_bounds(selector).is_some()
}
#[gpui::test]
fn shared_view_and_edit_trips_hide_what_their_permission_forbids(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_new_trip(&view, &mut cx);
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.trips.section = "Report";
            cx.notify();
        })
    });
    cx.run_until_parked();
    // The owner sees everything.
    assert!(has(&mut cx, "share-trip"), "owner share");
    assert!(has(&mut cx, "trip-details"), "owner edit");
    assert!(has(&mut cx, "note-trip_report"), "owner notes");
    view.read_with(&cx, |g, _| assert!(g.is_owner()));

    // Edit access: full editing, but no sharing, private report notes, or deleting.
    set_access(&view, &id, "edit", &mut cx);
    assert!(
        has(&mut cx, "trip-details"),
        "collaborators can edit the trip"
    );
    assert!(!has(&mut cx, "share-trip"), "sharing is the owner's");
    assert!(
        !has(&mut cx, "note-trip_report"),
        "private notes are the owner's"
    );
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            assert!(!g.is_owner() && !g.can_share());
            g.open_share(cx);
            assert!(
                !g.share.open,
                "the share menu does not open for a collaborator"
            );
            g.edit_trip(EditKind::Details, w, cx);
            assert!(g.trips.editor.is_some(), "edit access edits");
            g.trips.editor = None;
            g.command(Command::AddTripDay { trip: id.clone() }, cx);
            assert!(g.error.is_none(), "edit access writes");
            assert_eq!(g.trips.detail.as_ref().expect("detail").access, "edit");
        })
    });
    // Shared trips are in the Logbook's shared section, never in Mine, and never deletable.
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.trips.selected = None;
            g.area = "Basecamp";
            g.reload(cx);
        })
    });
    cx.run_until_parked();
    assert!(
        !has(&mut cx, "trip-delete"),
        "a collaborator cannot delete the trip"
    );
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.area = "Trips";
            g.trips.filter = "all";
            cx.notify();
        })
    });
    cx.run_until_parked();
    assert!(!has(&mut cx, "shared-card") && !has(&mut cx, "trip-delete"));
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.trips.filter = "shared";
            cx.notify();
        })
    });
    cx.run_until_parked();
    assert!(has(&mut cx, "shared-card"));

    // View access: read-only.
    set_access(&view, &id, "view", &mut cx);
    cx.update(|w, cx| {
        view.update(cx, |g, cx| {
            g.open_trip(id.clone(), cx);
            g.trips.section = "Report";
            g.edit_trip(EditKind::Details, w, cx);
            assert!(g.trips.editor.is_none());
            let days = g.trips.detail.as_ref().expect("detail").days.len();
            g.command(Command::AddTripDay { trip: id.clone() }, cx);
            // The store refuses the write for a viewer, and nothing changed.
            assert!(g.error.is_some());
            assert_eq!(g.trips.detail.as_ref().expect("detail").days.len(), days);
        })
    });
    cx.run_until_parked();
    assert!(
        !has(&mut cx, "trip-details")
            && !has(&mut cx, "share-trip")
            && !has(&mut cx, "note-trip_report")
    );
}
#[gpui::test]
fn a_revoked_shared_trip_disappears_with_a_toast(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let id = open_new_trip(&view, &mut cx);
    set_access(&view, &id, "view", &mut cx);
    cx.update(|_, cx| {
        view.update(cx, |g, cx| {
            g.open_trip(id.clone(), cx);
            assert!(g.trips.detail.is_some() && g.toast.is_none());
            // The owner revokes access: the next pull deletes the trip row.
            g.store
                .connection()
                .execute("DELETE FROM trip WHERE sync_id=?1", [&id])
                .expect("revoked");
            g.reload(cx);
            assert!(g.trips.selected.is_none() && g.trips.detail.is_none());
            let (message, _) = g.toast.clone().expect("toast");
            assert!(message.contains("no longer shared"), "{message}");
        })
    });
    cx.run_until_parked();
}
