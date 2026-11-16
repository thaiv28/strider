//! Native raster map and elevation surface. UI events carry route miles.
use crate::*;
use std::{
    collections::HashMap,
    sync::mpsc,
    time::{Duration, Instant},
};
use strider_core::gpx::{ExportCampsite, Point as RoutePoint, group_by_location, is_loop_track};
use strider_map::{
    Pixel, Source, Tile, TileCache, Viewport, fit_bounds, parent_fallback, project, unproject,
};
#[derive(Clone, Debug)]
pub struct RouteClick {
    pub distance: f64,
    pub campsite: Option<String>,
}
pub struct RouteMap {
    pub points: Vec<RoutePoint>,
    pub camps: Vec<(String, ExportCampsite)>,
    pub editable: bool,
    /// Campsite points of interest (`lat`, `lon`, name) drawn under the trip's own markers.
    pub pois: Vec<(f64, f64, Option<String>)>,
    pub viewport: Viewport,
    pub source: Source,
    cache: Option<Arc<TileCache>>,
    bounds: Bounds<Pixels>,
    focus: FocusHandle,
    drag: Option<(Point<Pixels>, Point<Pixels>, Option<String>)>,
    failed: HashMap<(Source, Tile), Instant>,
    waiting: std::collections::HashSet<(Source, Tile)>,
    tx: mpsc::Sender<(Source, Tile, bool)>,
    _poll: Task<()>,
}
impl EventEmitter<RouteClick> for RouteMap {}
impl RouteMap {
    pub fn new(cache: Option<Arc<TileCache>>, cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<(Source, Tile, bool)>();
        let poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let results: Vec<_> = rx.try_iter().collect();
                if !results.is_empty()
                    && this
                        .update(cx, |this, cx| {
                            for (source, tile, ok) in results {
                                this.waiting.remove(&(source, tile));
                                if !ok {
                                    this.failed.insert((source, tile), Instant::now());
                                }
                            }
                            cx.notify();
                        })
                        .is_err()
                {
                    break;
                }
                if this.upgrade().is_none() {
                    break;
                }
            }
        });
        Self {
            points: vec![],
            camps: vec![],
            editable: true,
            pois: vec![],
            viewport: Viewport::default(),
            source: Source::Topo,
            cache,
            bounds: Bounds::default(),
            focus: cx.focus_handle(),
            drag: None,
            failed: HashMap::new(),
            waiting: Default::default(),
            tx,
            _poll: poll,
        }
    }
    pub fn set_route(
        &mut self,
        points: Vec<RoutePoint>,
        camps: Vec<(String, ExportCampsite)>,
        editable: bool,
        cx: &mut Context<Self>,
    ) {
        if self.points != points {
            let track: Vec<_> = points.iter().map(|p| [p.lat, p.lon]).collect();
            self.viewport = fit_bounds(&track, self.width(), self.height(), 36., 16.);
            self.points = points;
        }
        self.camps = camps;
        self.editable = editable;
        cx.notify();
    }
    fn width(&self) -> f64 {
        f64::from(f32::from(self.bounds.size.width)).max(500.)
    }
    fn height(&self) -> f64 {
        f64::from(f32::from(self.bounds.size.height)).max(300.)
    }
    fn local(&self, p: Point<Pixels>) -> (f64, f64) {
        (
            f64::from(f32::from(p.x - self.bounds.origin.x)),
            f64::from(f32::from(p.y - self.bounds.origin.y)),
        )
    }
    fn screen(&self, p: &RoutePoint) -> (f64, f64) {
        let world = project(p.lat, p.lon, self.viewport.zoom);
        let o = self.viewport.origin(self.width(), self.height());
        (world.x - o.x, world.y - o.y)
    }
    fn nearest(&self, pos: Point<Pixels>, threshold: f64) -> Option<&RoutePoint> {
        let (x, y) = self.local(pos);
        self.points
            .iter()
            .min_by(|a, b| {
                let (ax, ay) = self.screen(a);
                let (bx, by) = self.screen(b);
                ((ax - x).hypot(ay - y)).total_cmp(&(bx - x).hypot(by - y))
            })
            .filter(|p| {
                let (a, b) = self.screen(p);
                (a - x).hypot(b - y) <= threshold
            })
    }
    fn zoom(&mut self, delta: f64, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let (x, y) = self.local(pos);
        self.viewport
            .zoom_at(delta, Pixel { x, y }, self.width(), self.height());
        cx.notify();
    }
    pub fn zoom_by(&mut self, delta: f64, cx: &mut Context<Self>) {
        self.viewport.zoom = (self.viewport.zoom + delta).clamp(0., 22.);
        cx.notify();
    }
    fn pan(&mut self, dx: f64, dy: f64) {
        let p = project(
            self.viewport.center.0,
            self.viewport.center.1,
            self.viewport.zoom,
        );
        self.viewport.center = unproject(
            Pixel {
                x: p.x - dx,
                y: p.y - dy,
            },
            self.viewport.zoom,
        );
    }
}
impl Render for RouteMap {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx);
        let w = self.width();
        let h = self.height();
        let origin = self.viewport.origin(w, h);
        let zoom = self.viewport.zoom;
        let mut tiles = div().absolute().inset_0();
        if let Some(cache) = &self.cache {
            for tile in self.viewport.tiles(w, h, self.source) {
                let key = (self.source, tile);
                let bytes = cache.get(self.source, tile);
                if bytes.is_none()
                    && !self.waiting.contains(&key)
                    && self
                        .failed
                        .get(&key)
                        .is_none_or(|at| at.elapsed() > Duration::from_secs(30))
                {
                    self.waiting.insert(key);
                    let tx = self.tx.clone();
                    cache.request(self.source, tile, move |r| {
                        let _ = tx.send((key.0, key.1, r.is_ok()));
                    });
                }
                let shown = bytes.map(|b| (tile, b)).or_else(|| {
                    parent_fallback(tile, |p| cache.get(self.source, p).is_some())
                        .and_then(|(p, _)| cache.get(self.source, p).map(|b| (p, b)))
                });
                if let Some((p, bytes)) = shown {
                    let scale = 2_f64.powf(zoom - f64::from(tile.z));
                    let side = 256. * scale;
                    let parent_scale = 2_f64.powf(f64::from(tile.z - p.z));
                    let offset_x = f64::from(tile.x - (p.x << (tile.z - p.z))) * side;
                    let offset_y = f64::from(tile.y - (p.y << (tile.z - p.z))) * side;
                    let format = if bytes.starts_with(b"\x89PNG") {
                        ImageFormat::Png
                    } else {
                        ImageFormat::Jpeg
                    };
                    tiles = tiles.child(
                        div()
                            .absolute()
                            .left(px((f64::from(tile.x) * side - origin.x) as f32))
                            .top(px((f64::from(tile.y) * side - origin.y) as f32))
                            .size(px(side as f32))
                            .overflow_hidden()
                            .child(
                                img(Arc::new(Image::from_bytes(format, bytes.to_vec())))
                                    .absolute()
                                    .left(px(-offset_x as f32))
                                    .top(px(-offset_y as f32))
                                    .size(px((side * parent_scale) as f32)),
                            ),
                    );
                }
            }
        }
        let points = self.points.clone();
        let viewport = self.viewport;
        let weak = cx.entity().downgrade();
        let shape = canvas(
            move |bounds, _, cx| {
                let _ = weak.update(cx, |this, cx| {
                    if this.bounds != bounds {
                        this.bounds = bounds;
                        cx.notify();
                    }
                });
            },
            move |bounds, _, window, _| paint_route(bounds, &points, viewport, t.route, window),
        )
        .absolute()
        .inset_0()
        .size_full();
        let mut markers = div().absolute().inset_0();
        for (lat, lon, _) in &self.pois {
            let p = project(*lat, *lon, zoom);
            markers = markers.child(
                div()
                    .absolute()
                    .left(px((p.x - origin.x) as f32 - 5.))
                    .top(px((p.y - origin.y) as f32 - 5.))
                    .size(px(10.))
                    .rounded_full()
                    .bg(t.muted)
                    .border_1()
                    .border_color(t.panel),
            );
        }
        let camps: Vec<_> = self.camps.iter().map(|(_, c)| c.clone()).collect();
        for group in group_by_location(&camps) {
            let first = group[0];
            if let (Some(lat), Some(lon)) = (first.lat, first.lon) {
                let p = project(lat, lon, zoom);
                let label = group
                    .iter()
                    .map(|c| c.night.to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                markers = markers.child(marker(p.x - origin.x, p.y - origin.y, label, t));
            }
        }
        if let (Some(start), Some(end)) = (self.points.first(), self.points.last()) {
            let track: Vec<_> = self.points.iter().map(|p| [p.lat, p.lon]).collect();
            let (x, y) = self.screen(start);
            markers = markers.child(marker(
                x,
                y,
                if is_loop_track(&track) { "S/E" } else { "S" }.into(),
                t,
            ));
            if !is_loop_track(&track) {
                let (x, y) = self.screen(end);
                markers = markers.child(marker(x, y, "E".into(), t));
            }
        }
        div()
            .id("route-map")
            .relative()
            .w_full()
            .h(px(380.))
            .bg(t.map)
            .overflow_hidden()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.modifiers.platform {
                    match e.keystroke.key.as_str() {
                        "+" | "=" => this.zoom_by(1., cx),
                        "-" => this.zoom_by(-1., cx),
                        _ => {}
                    }
                }
            }))
            .on_pinch(cx.listener(|this, e: &PinchEvent, _, cx| {
                this.zoom(f64::from(e.delta) * 2., e.position, cx);
                cx.stop_propagation();
            }))
            .on_scroll(cx.listener(|this, e: &ScrollEvent, _, cx| {
                let delta = e.delta.pixel_delta(px(20.));
                this.pan(f64::from(f32::from(delta.x)), f64::from(f32::from(delta.y)));
                cx.notify();
                cx.stop_propagation();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, w, cx| {
                    this.focus.focus(w, cx);
                    if e.click_count == 2 {
                        this.zoom(1., e.position, cx);
                        return;
                    }
                    let site = if this.editable {
                        this.camps.iter().find_map(|(id, c)| {
                            let p = project(c.lat?, c.lon?, this.viewport.zoom);
                            let o = this.viewport.origin(this.width(), this.height());
                            let (x, y) = this.local(e.position);
                            ((p.x - o.x - x).hypot(p.y - o.y - y) < 16.).then(|| id.clone())
                        })
                    } else {
                        None
                    };
                    this.drag = Some((e.position, e.position, site));
                }),
            )
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                if e.dragging()
                    && let Some((start, last, site)) = this.drag.clone()
                {
                    if site.is_none() {
                        this.pan(
                            f64::from(f32::from(e.position.x - last.x)),
                            f64::from(f32::from(e.position.y - last.y)),
                        );
                    }
                    this.drag = Some((start, e.position, site));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| {
                    if let Some((start, _, site)) = this.drag.take() {
                        let moved = f32::from(e.position.x - start.x)
                            .hypot(f32::from(e.position.y - start.y));
                        if this.editable
                            && (site.is_some() || moved < 5.)
                            && let Some(p) = this.nearest(
                                e.position,
                                if site.is_some() { f64::INFINITY } else { 24. },
                            )
                        {
                            cx.emit(RouteClick {
                                distance: p.d,
                                campsite: site,
                            });
                        }
                    }
                }),
            )
            .child(tiles)
            .child(shape)
            .child(markers)
            .child(
                div().absolute().top_3().left_3().flex().gap_1().children(
                    [(Source::Topo, "Topo"), (Source::Osm, "OSM")]
                        .into_iter()
                        .map(|(source, label)| {
                            chip(label, label, self.source == source, t).on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.source = source;
                                    cx.stop_propagation();
                                    cx.notify();
                                },
                            ))
                        }),
                ),
            )
            .child(
                div()
                    .absolute()
                    .bottom_1()
                    .right_2()
                    .bg(t.panel)
                    .meta(t)
                    .child(self.source.attribution()),
            )
    }
}
fn marker(x: f64, y: f64, label: String, t: Theme) -> Div {
    div()
        .absolute()
        .left(px(x as f32 - 12.))
        .top(px(y as f32 - 12.))
        .min_w(px(24.))
        .h(px(24.))
        .px_1()
        .rounded_full()
        .bg(t.accent)
        .text_color(t.accent_ink)
        .text_size(px(12.))
        .flex()
        .items_center()
        .justify_center()
        .child(label)
}
fn paint_route(
    bounds: Bounds<Pixels>,
    points: &[RoutePoint],
    v: Viewport,
    color: Hsla,
    window: &mut Window,
) {
    let origin = v.origin(
        f64::from(f32::from(bounds.size.width)),
        f64::from(f32::from(bounds.size.height)),
    );
    let mut path = PathBuilder::stroke(px(2.));
    for (i, p) in points.iter().enumerate() {
        let p = project(p.lat, p.lon, v.zoom);
        let p = point(
            bounds.left() + px((p.x - origin.x) as f32),
            bounds.top() + px((p.y - origin.y) as f32),
        );
        if i == 0 {
            path.move_to(p)
        } else {
            path.line_to(p)
        }
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}
pub fn route_thumbnail(track: Vec<[f64; 2]>, t: Theme) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |b, _, w, _| {
            let v = fit_bounds(
                &track,
                f64::from(f32::from(b.size.width)),
                f64::from(f32::from(b.size.height)),
                6.,
                16.,
            );
            let pts: Vec<_> = track
                .iter()
                .map(|p| RoutePoint {
                    lat: p[0],
                    lon: p[1],
                    d: 0.,
                    e: 0.,
                })
                .collect();
            paint_route(b, &pts, v, t.route, w);
        },
    )
    .w(px(88.))
    .h(px(44.))
}
pub struct ElevationProfile {
    pub points: Vec<RoutePoint>,
    pub camps: Vec<(String, f64)>,
    pub editable: bool,
    bounds: Bounds<Pixels>,
    hover: Option<RoutePoint>,
    drag: Option<String>,
}
impl EventEmitter<RouteClick> for ElevationProfile {}
impl Default for ElevationProfile {
    fn default() -> Self {
        Self {
            points: vec![],
            camps: vec![],
            editable: true,
            bounds: Bounds::default(),
            hover: None,
            drag: None,
        }
    }
}
impl ElevationProfile {
    pub fn click_distance(&self, distance: f64, cx: &mut Context<Self>) {
        if self.editable {
            cx.emit(RouteClick {
                distance,
                campsite: None,
            });
        }
    }
    fn nearest(&self, x: Pixels) -> Option<RoutePoint> {
        let max = self.points.last()?.d;
        let width = f64::from(f32::from(self.bounds.size.width)).max(1.);
        let d = (f64::from(f32::from(x - self.bounds.left())) / width).clamp(0., 1.) * max;
        self.points
            .iter()
            .min_by(|a, b| (a.d - d).abs().total_cmp(&(b.d - d).abs()))
            .cloned()
    }
}
impl Render for ElevationProfile {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx);
        let points = self.points.clone();
        let camps = self.camps.clone();
        let hover = self.hover.clone();
        let weak = cx.entity().downgrade();
        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .bg(t.panel)
            .child(
                div().meta(t).readout().child(
                    self.hover
                        .as_ref()
                        .map(|p| format!("Elevation · {:.1} mi · {:.0} ft", p.d, p.e))
                        .unwrap_or_else(|| "Elevation · Click to add a campsite".into()),
                ),
            )
            .child(
                div()
                    .id("elevation-profile")
                    .relative()
                    .h(px(92.))
                    .w_full()
                    .cursor_crosshair()
                    .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                        this.hover = this.nearest(e.position.x);
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, _| {
                            if this.editable
                                && let Some(p) = this.nearest(e.position.x)
                            {
                                let max = this.points.last().map_or(1., |p| p.d).max(1e-9);
                                let width = f64::from(f32::from(this.bounds.size.width));
                                this.drag = this
                                    .camps
                                    .iter()
                                    .find(|(_, d)| (d - p.d).abs() / max * width < 14.)
                                    .map(|(id, _)| id.clone());
                            }
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseUpEvent, _, cx| {
                            if this.editable
                                && let Some(p) = this.nearest(e.position.x)
                            {
                                cx.emit(RouteClick {
                                    distance: p.d,
                                    campsite: this.drag.take(),
                                });
                            }
                        }),
                    )
                    .on_hover(cx.listener(|this, hover, _, cx| {
                        if !hover {
                            this.hover = None;
                            this.drag = None;
                            cx.notify();
                        }
                    }))
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                let _ = weak.update(cx, |this, _| this.bounds = bounds);
                            },
                            move |b, _, w, _| {
                                let Some(last) = points.last() else { return };
                                let max = last.d.max(1e-9);
                                let low = points.iter().map(|p| p.e).fold(f64::INFINITY, f64::min);
                                let high =
                                    points.iter().map(|p| p.e).fold(f64::NEG_INFINITY, f64::max);
                                let span = (high - low).max(1.);
                                let at = |d: f64, e: f64| {
                                    point(
                                        b.left() + b.size.width * (d / max) as f32,
                                        b.bottom()
                                            - px(2.)
                                            - (b.size.height - px(4.)) * ((e - low) / span) as f32,
                                    )
                                };
                                let mut area = PathBuilder::fill();
                                area.move_to(point(b.left(), b.bottom()));
                                for p in &points {
                                    area.line_to(at(p.d, p.e));
                                }
                                area.line_to(point(b.right(), b.bottom()));
                                area.close();
                                if let Ok(path) = area.build() {
                                    w.paint_path(path, t.elevation);
                                }
                                let mut line = PathBuilder::stroke(px(2.));
                                for (i, p) in points.iter().enumerate() {
                                    if i == 0 {
                                        line.move_to(at(p.d, p.e));
                                    } else {
                                        line.line_to(at(p.d, p.e));
                                    }
                                }
                                if let Ok(path) = line.build() {
                                    w.paint_path(path, t.route);
                                }
                                for d in camps
                                    .iter()
                                    .map(|(_, d)| *d)
                                    .chain(hover.iter().map(|p| p.d))
                                {
                                    let x = b.left() + b.size.width * (d / max) as f32;
                                    let mut path = PathBuilder::stroke(px(1.));
                                    path.move_to(point(x, b.top()));
                                    path.line_to(point(x, b.bottom()));
                                    if let Ok(path) = path.build() {
                                        w.paint_path(path, t.ink.opacity(0.3));
                                    }
                                }
                            },
                        )
                        .size_full(),
                    ),
            )
    }
}
