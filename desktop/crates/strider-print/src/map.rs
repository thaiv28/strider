//! The route map: cached topo tiles (never fetched), then the route, rasterized with tiny-skia.
use crate::{Color, Palette};
use std::path::Path;
use strider_map::{
    Source, TILE, Tile, Viewport, fit_bounds, is_cached, parent_fallback, project, read_cached,
};
use tiny_skia::{
    FilterQuality, IntRect, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke,
    Transform,
};

pub struct MapImage {
    pub width: u32,
    pub height: u32,
    /// Row-major 8-bit RGB.
    pub rgb: Vec<u8>,
    viewport: Viewport,
    /// Tiles (or tile crops) painted; zero means only the map color shows.
    pub tiles_drawn: usize,
}
impl MapImage {
    /// Pixel position (from the top left) of a coordinate.
    pub fn project(&self, lat: f64, lon: f64) -> (f64, f64) {
        let origin = self
            .viewport
            .origin(f64::from(self.width), f64::from(self.height));
        let p = project(lat, lon, self.viewport.zoom);
        (p.x - origin.x, p.y - origin.y)
    }
}
fn paint(color: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(color[0], color[1], color[2], 255);
    p.anti_alias = true;
    p
}
fn decode(bytes: &[u8]) -> Option<Pixmap> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut data = img.into_raw();
    for px in data.as_chunks_mut::<4>().0 {
        let a = u16::from(px[3]);
        for c in &mut px[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h)?)
}
/// Paint one tile at its world position, using the nearest cached ancestor when it is missing.
fn draw_tile(pixmap: &mut Pixmap, root: &Path, tile: Tile, origin: (f64, f64), zoom: f64) -> bool {
    let (source_tile, crop) = if is_cached(root, Source::Topo, tile) {
        (tile, [0., 0., 1., 1.])
    } else {
        // Overview tiles are blurrier but keep the page useful.
        match parent_fallback(tile, |t| is_cached(root, Source::Topo, t)) {
            Some(found) if tile.z - found.0.z <= 4 => found,
            _ => return false,
        }
    };
    let Some(bytes) = read_cached(root, Source::Topo, source_tile) else {
        return false;
    };
    let Some(full) = decode(&bytes) else {
        return false;
    };
    let side = f64::from(full.width());
    let rect = IntRect::from_xywh(
        (crop[0] * side).round() as i32,
        (crop[1] * side).round() as i32,
        ((crop[2] * side).round() as u32).max(1),
        ((crop[3] * side).round() as u32).max(1),
    );
    let part = match rect {
        Some(r) if crop != [0., 0., 1., 1.] => match full.clone_rect(r) {
            Some(p) => p,
            None => return false,
        },
        _ => full,
    };
    let scale = 2_f64.powf(zoom - f64::from(tile.z));
    let size = TILE * scale;
    let dest_x = f64::from(tile.x) * size - origin.0;
    let dest_y = f64::from(tile.y) * size - origin.1;
    let k = (size / f64::from(part.width())) as f32;
    pixmap.draw_pixmap(
        0,
        0,
        part.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        Transform::from_row(k, 0., 0., k, dest_x as f32, dest_y as f32),
        None,
    );
    true
}
/// Rasterize the map. `tiles` is a tile root (as `strider_map::TileCache::root`); without it,
/// or where tiles are missing, the `map` color shows through.
pub fn render_map(
    track: &[[f64; 2]],
    palette: &Palette,
    tiles: Option<&Path>,
    width: u32,
    height: u32,
) -> Option<MapImage> {
    let valid: Vec<[f64; 2]> = track
        .iter()
        .copied()
        .filter(|p| p.iter().all(|n| n.is_finite()))
        .collect();
    if valid.len() < 2 || width == 0 || height == 0 {
        return None;
    }
    let mut pixmap = Pixmap::new(width, height)?;
    pixmap.fill(tiny_skia::Color::from_rgba8(
        palette.map[0],
        palette.map[1],
        palette.map[2],
        255,
    ));
    let (w, h) = (f64::from(width), f64::from(height));
    let viewport = fit_bounds(
        &valid,
        w,
        h,
        w.min(h) * 0.06,
        f64::from(Source::Topo.max_zoom()),
    );
    let mut tiles_drawn = 0;
    if let Some(root) = tiles {
        let origin = viewport.origin(w, h);
        for tile in viewport.tiles(w, h, Source::Topo) {
            if draw_tile(&mut pixmap, root, tile, (origin.x, origin.y), viewport.zoom) {
                tiles_drawn += 1;
            }
        }
    }
    let image = MapImage {
        width,
        height,
        rgb: vec![],
        viewport,
        tiles_drawn,
    };
    let mut builder = PathBuilder::new();
    for (i, p) in valid.iter().enumerate() {
        let (x, y) = image.project(p[0], p[1]);
        if i == 0 {
            builder.move_to(x as f32, y as f32);
        } else {
            builder.line_to(x as f32, y as f32);
        }
    }
    if let Some(path) = builder.finish() {
        let unit = (width as f32 / 516.).max(1.);
        for (color, line) in [(palette.map, 3.5 * unit), (palette.route, 2.2 * unit)] {
            let stroke = Stroke {
                width: line,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
        }
    }
    let rgb = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    Some(MapImage { rgb, ..image })
}
