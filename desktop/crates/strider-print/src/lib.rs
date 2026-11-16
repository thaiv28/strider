//! PDF trip sheet: a Letter-size document with real, extractable text in the app's bundled fonts,
//! and a route map rasterized from cached tiles. Content follows the web print page
//! (`app/trips/[id]/print/page.tsx`), followed by the Gear, Food and Campsites sections.
mod canvas;
mod font;
mod map;
#[cfg(test)]
mod tests;

use canvas::{Canvas, PAGE_H, PAGE_W};
use flate2::{Compression, write::ZlibEncoder};
use font::Font;
pub use map::{MapImage, render_map};
use pdf_writer::{
    Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr,
    types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap},
};
use std::{fmt, io::Write, path::Path};

pub type Color = [u8; 3];
/// Colors come from the Topo tokens (the app passes the light set, as paper is white).
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub ink: Color,
    pub muted: Color,
    pub line: Color,
    pub accent: Color,
    pub route: Color,
    pub map: Color,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DayRow {
    pub day: String,
    pub distance: String,
    pub gain: String,
    pub camp: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GearLine {
    pub name: String,
    pub quantity: String,
    pub weight: String,
    pub class: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GearSection {
    pub category: String,
    pub lines: Vec<GearLine>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FoodDay {
    pub heading: String,
    pub lines: Vec<String>,
}
/// A numbered campsite on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub night: u32,
    pub lat: f64,
    pub lon: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PermitDocument {
    pub filename: String,
    /// "PDF" or "image".
    pub kind: String,
}
/// Everything the sheet prints, already formatted for display.
#[derive(Clone, Debug, PartialEq)]
pub struct TripSheet {
    pub name: String,
    /// Dates and nights, or "date TBD".
    pub when: String,
    pub place: Option<String>,
    /// Distance, gain and trailhead.
    pub summary: String,
    pub days: Vec<DayRow>,
    pub totals: DayRow,
    pub permit_notes: Option<String>,
    pub permit_document: Option<PermitDocument>,
    /// Weight by class, as (label, value).
    pub gear_totals: Vec<(String, String)>,
    pub gear: Vec<GearSection>,
    pub food: Vec<FoodDay>,
    pub campsites: Vec<String>,
    /// `[lat, lon]` pairs.
    pub track: Vec<[f64; 2]>,
    pub markers: Vec<Marker>,
    pub palette: Palette,
}
#[derive(Debug)]
pub enum PrintError {
    Font(String),
    Map,
}
impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Font(e) => write!(f, "Could not read a bundled font: {e}"),
            Self::Map => f.write_str("Could not draw the route map."),
        }
    }
}
impl std::error::Error for PrintError {}

const CATALOG: Ref = Ref::new(1);
const PAGES: Ref = Ref::new(2);
const INFO: Ref = Ref::new(3);
const IMAGE: Ref = Ref::new(4);
const FONT_BASE: i32 = 10;
const PAGE_BASE: i32 = 100;
/// Raster pixels per PDF point for the map (2 is about 144 dpi).
const MAP_SCALE: u32 = 2;

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    // Writing into a Vec cannot fail.
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}
/// The sheet as PDF bytes. `tiles` is a tile root; tiles are read, never fetched.
pub fn render_pdf(sheet: &TripSheet, tiles: Option<&Path>) -> Result<Vec<u8>, PrintError> {
    render_pdf_scaled(sheet, tiles, MAP_SCALE)
}
/// As [`render_pdf`], with an explicit number of map pixels per point (smaller is faster).
pub fn render_pdf_scaled(
    sheet: &TripSheet,
    tiles: Option<&Path>,
    map_scale: u32,
) -> Result<Vec<u8>, PrintError> {
    let mut canvas = Canvas::new(sheet.palette.clone())?;
    canvas.header(sheet);
    canvas.day_table(sheet);
    canvas.permit(sheet);
    canvas.gear(sheet);
    canvas.food(sheet);
    canvas.campsites(sheet);
    let map_scale = map_scale.max(1);
    let mut image = None;
    if sheet.track.len() >= 2 {
        let (w, h) = canvas.map_box();
        image = render_map(
            &sheet.track,
            &sheet.palette,
            tiles,
            (w * map_scale as f32) as u32,
            (h * map_scale as f32) as u32,
        );
        if let Some(map) = &image {
            canvas.map_page(
                map,
                sheet,
                map_scale,
                tiles.is_some() && map.tiles_drawn > 0,
            );
        }
    }
    let (pages, fonts) = canvas.finish();
    let mut pdf = Pdf::new();
    pdf.catalog(CATALOG).pages(PAGES);
    pdf.document_info(INFO).title(TextStr(&sheet.name));
    let ids: Vec<(Ref, Ref)> = (0..pages.len() as i32)
        .map(|i| (Ref::new(PAGE_BASE + 2 * i), Ref::new(PAGE_BASE + 2 * i + 1)))
        .collect();
    pdf.pages(PAGES)
        .kids(ids.iter().map(|p| p.0))
        .count(ids.len() as i32);
    let font_ref = |i: usize| Ref::new(FONT_BASE + 5 * i as i32);
    for (content, (page_id, content_id)) in pages.iter().zip(&ids) {
        let mut page = pdf.page(*page_id);
        page.media_box(Rect::new(0., 0., PAGE_W, PAGE_H));
        page.parent(PAGES);
        page.contents(*content_id);
        let mut resources = page.resources();
        {
            let mut table = resources.fonts();
            for (i, font) in fonts.iter().enumerate() {
                table.pair(Name(font.resource.as_bytes()), font_ref(i));
            }
        }
        if image.is_some() {
            resources.x_objects().pair(Name(b"Im1"), IMAGE);
        }
        resources.finish();
        page.finish();
        pdf.stream(*content_id, &zlib(content))
            .filter(Filter::FlateDecode);
    }
    for (i, font) in fonts.iter().enumerate() {
        write_font(&mut pdf, font, FONT_BASE + 5 * i as i32);
    }
    if let Some(map) = &image {
        let pixels = zlib(&map.rgb);
        let mut x = pdf.image_xobject(IMAGE, &pixels);
        x.filter(Filter::FlateDecode);
        x.width(map.width as i32);
        x.height(map.height as i32);
        x.pair(Name(b"ColorSpace"), Name(b"DeviceRGB"));
        x.bits_per_component(8);
        x.finish();
    }
    Ok(pdf.finish())
}
fn write_font(pdf: &mut Pdf, font: &Font, base: i32) {
    let (type0, cid, descriptor, file, unicode) = (
        Ref::new(base),
        Ref::new(base + 1),
        Ref::new(base + 2),
        Ref::new(base + 3),
        Ref::new(base + 4),
    );
    let name = Name(font.base_font.as_bytes());
    pdf.type0_font(type0)
        .base_font(name)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid)
        .to_unicode(unicode);
    let mut cid_font = pdf.cid_font(cid);
    cid_font.subtype(CidFontType::Type2);
    cid_font.base_font(name);
    cid_font.system_info(SystemInfo {
        registry: Str(b"Adobe"),
        ordering: Str(b"Identity"),
        supplement: 0,
    });
    cid_font.font_descriptor(descriptor);
    cid_font.cid_to_gid_map_predefined(Name(b"Identity"));
    {
        let mut widths = cid_font.widths();
        for glyph in font.used.keys() {
            widths.consecutive(*glyph, [font.glyph_width(*glyph)]);
        }
    }
    cid_font.finish();
    let b = font.bbox();
    pdf.font_descriptor(descriptor)
        .name(name)
        .flags(FontFlags::NON_SYMBOLIC)
        .bbox(Rect::new(b[0], b[1], b[2], b[3]))
        .italic_angle(0.)
        .ascent(font.ascent())
        .descent(font.descent())
        .cap_height(font.cap_height())
        .stem_v(80.)
        .font_file2(file);
    pdf.stream(file, &zlib(font.data))
        .filter(Filter::FlateDecode)
        .pair(Name(b"Length1"), font.data.len() as i32);
    let mut cmap = UnicodeCmap::new(
        Name(b"Custom"),
        SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"UCS"),
            supplement: 0,
        },
    );
    for (glyph, c) in &font.used {
        cmap.pair(*glyph, *c);
    }
    pdf.cmap(unicode, &cmap.finish());
}
