use crate::app::AppView;
use crate::app::test_support::{headless, open_routed_trip, temp_dir, with_view};
use gpui::{Entity, TestAppContext};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use strider_map::{Fetch, Source, Tile, TileFetcher, region::Region};

/// A tile source that waits at a gate, so a test can look at a download in flight.
struct Gated {
    open: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}
impl TileFetcher for Gated {
    fn fetch(&self, _: Source, _: Tile) -> io::Result<Fetch> {
        let start = Instant::now();
        while !self.open.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(2));
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Fetch {
            bytes: vec![7; 1000],
            max_age: Duration::from_secs(60),
            store: true,
        })
    }
}
fn wait_for_download(view: &Entity<AppView>, cx: &mut gpui::VisualTestContext) {
    for _ in 0..1000 {
        if with_view(view, cx, |g, _, cx| {
            g.poll_jobs(cx);
            g.offline.download.is_none()
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the download did not finish");
}

#[gpui::test]
fn trip_opened_offline_without_a_region_shows_the_map_message(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let trip = open_routed_trip(&view, "Offline trip", &mut cx);
    with_view(&view, &mut cx, |g, _, cx| {
        let d = g.trips.detail.clone().expect("detail");
        assert!(!d.points.is_empty(), "the sample route parses");
        assert_eq!(g.map_message(&d), None, "online: tiles load normally");
        g.account.offline = true;
        assert_eq!(
            g.map_message(&d),
            Some("Map not downloaded. Connect, or download this map before you go.")
        );
        let region = Region::for_route(
            &trip,
            &d.points.iter().map(|p| [p.lat, p.lon]).collect::<Vec<_>>(),
        )
        .expect("region");
        g.store
            .save_map_region(&super::map::record_for_tests(&region, "complete"))
            .expect("save");
        g.load_regions();
        assert!(g.has_offline_map());
        assert_eq!(g.map_message(&d), None, "a downloaded map needs no message");
        g.store.delete_map_region(&trip).expect("delete");
        g.load_regions();
        assert!(g.map_message(&d).is_some());
        cx.notify();
    });
    cx.run_until_parked(); // renders the route over the map token with the message
}

#[gpui::test]
fn region_download_reports_progress_completes_and_deletes(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    let trip = open_routed_trip(&view, "Download trip", &mut cx);
    let root = temp_dir("tiles");
    let (gate, calls) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicUsize::new(0)),
    );
    let tiles = with_view(&view, &mut cx, |g, _, cx| {
        g.offline.backend.tiles_root = root.clone();
        g.offline.backend.fetcher = Arc::new(Gated {
            open: gate.clone(),
            calls: calls.clone(),
        });
        let estimate = g.selected_region().expect("region").estimate();
        assert!(estimate.tiles > 4, "enough tiles to see progress");
        g.open_map_sheet(cx);
        g.start_download(cx);
        let d = g.offline.download.as_ref().expect("download started");
        let p = *d.progress.lock().expect("progress");
        assert_eq!((p.done, p.total), (0, estimate.tiles));
        assert_eq!(
            g.offline.region.as_ref().map(|r| r.state.as_str()),
            Some("downloading")
        );
        estimate.tiles
    });
    cx.run_until_parked(); // renders the sheet with the in-flight download
    gate.store(true, Ordering::SeqCst);
    wait_for_download(&view, &mut cx);
    with_view(&view, &mut cx, |g, _, cx| {
        let r = g.offline.region.clone().expect("stored region");
        assert_eq!(r.state, "complete");
        assert_eq!((r.tiles_done, r.tiles_total), (tiles, tiles));
        assert_eq!(r.bytes, tiles * 1000);
        assert!(g.has_offline_map());
        assert_eq!(calls.load(Ordering::SeqCst) as u64, tiles);
        assert!(strider_map::disk_usage(&root) >= tiles * 1000);
        cx.notify();
    });
    cx.run_until_parked(); // renders "Downloaded"
    with_view(&view, &mut cx, |g, _, cx| {
        g.delete_region(&trip, cx);
        assert!(!g.has_offline_map());
        assert!(g.store.map_region(&trip).expect("read").is_none());
        // The tiles stay cached but are no longer pinned, so clearing the cache frees them.
        assert!(strider_map::disk_usage(&root) > 0);
        g.clear_map_cache(cx);
        assert_eq!(strider_map::disk_usage(&root), 0);
    });
    let _ = std::fs::remove_dir_all(root);
}

#[gpui::test]
fn cancelling_a_download_keeps_what_was_fetched(cx: &mut TestAppContext) {
    let (view, mut cx) = headless(cx);
    cx.run_until_parked();
    open_routed_trip(&view, "Cancelled trip", &mut cx);
    let root = temp_dir("cancel");
    let (gate, calls) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicUsize::new(0)),
    );
    with_view(&view, &mut cx, |g, _, cx| {
        g.offline.backend.tiles_root = root.clone();
        g.offline.backend.fetcher = Arc::new(Gated {
            open: gate.clone(),
            calls: calls.clone(),
        });
        g.start_download(cx);
        g.cancel_download(cx);
    });
    gate.store(true, Ordering::SeqCst);
    wait_for_download(&view, &mut cx);
    with_view(&view, &mut cx, |g, _, _| {
        let r = g.offline.region.clone().expect("region");
        assert_eq!(r.state, "cancelled");
        assert!(r.tiles_done < r.tiles_total);
        assert!(!g.has_offline_map());
    });
    let _ = std::fs::remove_dir_all(root);
}
