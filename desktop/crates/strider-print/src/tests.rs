//! Parses the PDF the sheet produces: structure, extractable text, determinism and the map raster.
use super::*;
use std::{collections::BTreeMap, io::Read, path::PathBuf};

fn palette() -> Palette {
    Palette {
        ink: [20, 30, 25],
        muted: [110, 120, 115],
        line: [226, 230, 223],
        accent: [23, 56, 45],
        route: [217, 98, 43],
        map: [231, 236, 224],
    }
}
fn day(day: &str, distance: &str, gain: &str, camp: &str) -> DayRow {
    DayRow {
        day: day.into(),
        distance: distance.into(),
        gain: gain.into(),
        camp: camp.into(),
    }
}
fn sample() -> TripSheet {
    TripSheet {
        name: "Lost Coast Loop M\u{f6}nch".into(),
        when: "Jun 3\u{2013}6, 2026 \u{b7} 3 nights".into(),
        place: Some("California \u{b7} Coast".into()),
        summary: "42.5 mi \u{b7} 9,800 ft gain \u{b7} from Mattole trailhead".into(),
        days: vec![
            day("Day 1", "12.0 mi", "2,100 ft", "40.2500, -124.0500"),
            day("Day 2", "15.5 mi", "3,400 ft", "40.3000, -124.1000"),
            day("Day 3", "15.0 mi", "4,300 ft", "\u{2014}"),
        ],
        totals: day("Total", "42.5 mi", "9,800 ft", ""),
        permit_notes: Some("Bear canister required. Permit pickup at the ranger station.".into()),
        permit_document: Some(PermitDocument {
            filename: "permit (2026).pdf".into(),
            kind: "PDF".into(),
        }),
        gear_totals: vec![
            ("Base".into(), "14 lb 2 oz".into()),
            ("Worn".into(), "3 lb".into()),
        ],
        gear: vec![GearSection {
            category: "Shelter".into(),
            lines: vec![GearLine {
                name: "Tent".into(),
                quantity: "1".into(),
                weight: "2 lb 4 oz".into(),
                class: "base".into(),
            }],
        }],
        food: vec![FoodDay {
            heading: "Day 1 \u{b7} 2,400 / 2,600 kcal".into(),
            lines: vec!["Oatmeal \u{b7} 600 kcal \u{b7} 4.2 oz".into()],
        }],
        campsites: vec!["Night 1 \u{b7} 12.0 mi \u{b7} 40.2500, -124.0500".into()],
        track: vec![
            [39.95, -105.30],
            [39.96, -105.29],
            [39.97, -105.28],
            [39.98, -105.27],
        ],
        markers: vec![
            Marker {
                night: 1,
                lat: 39.96,
                lon: -105.29,
            },
            Marker {
                night: 2,
                lat: 39.97,
                lon: -105.28,
            },
        ],
        palette: palette(),
    }
}
fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "strider-print-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

// A small reader for the PDFs this crate writes: classic xref table, flate streams, Identity-H text.
struct Parsed {
    objects: BTreeMap<u32, (String, Vec<u8>)>,
}
fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}
fn inflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .expect("zlib stream");
    out
}
impl Parsed {
    fn parse(bytes: &[u8]) -> Self {
        assert!(bytes.starts_with(b"%PDF-"), "header");
        let start = find(bytes, b"startxref", 0).expect("startxref");
        let tail = String::from_utf8_lossy(&bytes[start + 9..]).into_owned();
        let xref_at: usize = tail
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .expect("xref offset");
        assert!(
            bytes[xref_at..].starts_with(b"xref"),
            "xref offset points at the table"
        );
        let table = String::from_utf8_lossy(&bytes[xref_at..]).into_owned();
        let mut lines = table.lines();
        assert_eq!(lines.next(), Some("xref"));
        let header: Vec<usize> = lines
            .next()
            .expect("subsection")
            .split_whitespace()
            .map(|n| n.parse().expect("count"))
            .collect();
        assert_eq!(header[0], 0);
        let mut objects = BTreeMap::new();
        for number in 0..header[1] {
            let entry: Vec<&str> = lines.next().expect("entry").split_whitespace().collect();
            if entry[2] != "n" {
                continue;
            }
            let offset: usize = entry[0].parse().expect("offset");
            let marker = format!("{number} 0 obj");
            assert!(
                bytes[offset..].starts_with(marker.as_bytes()),
                "object {number} sits at its xref offset"
            );
            let end = find(bytes, b"endobj", offset).expect("endobj");
            let body = &bytes[offset + marker.len()..end];
            let (dict, data) = match find(body, b"stream", 0) {
                Some(s) => {
                    let dict = String::from_utf8_lossy(&body[..s]).into_owned();
                    let mut from = s + 6;
                    if body[from] == b'\r' {
                        from += 1;
                    }
                    assert_eq!(body[from], b'\n');
                    from += 1;
                    let length: usize = dict
                        .split("/Length")
                        .nth(1)
                        .and_then(|r| r.split_whitespace().next())
                        .and_then(|n| n.parse().ok())
                        .expect("stream length");
                    let raw = &body[from..from + length];
                    let data = if dict.contains("/FlateDecode") {
                        inflate(raw)
                    } else {
                        raw.to_vec()
                    };
                    (dict, data)
                }
                None => (String::from_utf8_lossy(body).into_owned(), vec![]),
            };
            objects.insert(number as u32, (dict, data));
        }
        Self { objects }
    }
    fn pages(&self) -> Vec<u32> {
        self.objects
            .iter()
            .filter(|(_, (d, _))| d.contains("/Type /Page") && !d.contains("/Type /Pages"))
            .map(|(n, _)| *n)
            .collect()
    }
    fn reference(dict: &str, key: &str) -> Option<u32> {
        let rest = dict.split(key).nth(1)?;
        rest.split_whitespace().next()?.parse().ok()
    }
    /// Glyph id to character, from a font's ToUnicode stream.
    fn unicode_map(&self, font: u32) -> BTreeMap<u16, char> {
        let dict = &self.objects[&font].0;
        let cmap = Self::reference(dict, "/ToUnicode").expect("ToUnicode");
        let text = String::from_utf8_lossy(&self.objects[&cmap].1).into_owned();
        let mut map = BTreeMap::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 && parts.iter().all(|p| p.starts_with('<') && p.ends_with('>')) {
                let glyph =
                    u16::from_str_radix(&parts[0][1..parts[0].len() - 1], 16).expect("glyph");
                let unit =
                    u32::from_str_radix(&parts[1][1..parts[1].len() - 1], 16).expect("unicode");
                map.insert(glyph, char::from_u32(unit).expect("char"));
            }
        }
        map
    }
    /// Every shown string on a page, in order.
    fn page_text(&self, page: u32) -> Vec<String> {
        let dict = &self.objects[&page].0;
        let content = Self::reference(dict, "/Contents").expect("contents");
        // Font resources: "/F1 10 0 R".
        let tokens: Vec<&str> = dict.split_whitespace().collect();
        let mut fonts = BTreeMap::new();
        for w in tokens.windows(4) {
            if w[0].starts_with("/F")
                && w[2] == "0"
                && w[3] == "R"
                && let Ok(n) = w[1].parse::<u32>()
            {
                fonts.insert(w[0][1..].to_owned(), self.unicode_map(n));
            }
        }
        let data = &self.objects[&content].1;
        let mut out = Vec::new();
        let mut current = BTreeMap::new();
        let mut i = 0;
        let mut pending: Option<Vec<u8>> = None;
        while i < data.len() {
            match data[i] {
                b'(' => {
                    let mut s = Vec::new();
                    i += 1;
                    let mut depth = 1;
                    while depth > 0 {
                        match data[i] {
                            b'\\' => {
                                i += 1;
                                match data[i] {
                                    b'n' => s.push(b'\n'),
                                    b'r' => s.push(b'\r'),
                                    b't' => s.push(b'\t'),
                                    b'b' => s.push(8),
                                    b'f' => s.push(12),
                                    b'0'..=b'7' => {
                                        let mut v = 0u32;
                                        let mut n = 0;
                                        while n < 3 && (b'0'..=b'7').contains(&data[i]) {
                                            v = v * 8 + u32::from(data[i] - b'0');
                                            i += 1;
                                            n += 1;
                                        }
                                        i -= 1;
                                        s.push(v as u8);
                                    }
                                    c => s.push(c),
                                }
                            }
                            b'(' => {
                                depth += 1;
                                s.push(b'(');
                            }
                            b')' => {
                                depth -= 1;
                                if depth > 0 {
                                    s.push(b')');
                                }
                            }
                            c => s.push(c),
                        }
                        i += 1;
                    }
                    pending = Some(s);
                }
                b'<' => {
                    let end = find(data, b">", i).expect("hex end");
                    let hex = std::str::from_utf8(&data[i + 1..end]).expect("hex");
                    pending = Some(
                        (0..hex.len())
                            .step_by(2)
                            .map(|k| u8::from_str_radix(&hex[k..k + 2], 16).expect("byte"))
                            .collect(),
                    );
                    i = end + 1;
                }
                b'/' => {
                    let end = data[i..]
                        .iter()
                        .position(|b| b.is_ascii_whitespace())
                        .map_or(data.len(), |p| p + i);
                    let name = String::from_utf8_lossy(&data[i + 1..end]).into_owned();
                    if let Some(map) = fonts.get(&name) {
                        current = map.clone();
                    }
                    i = end;
                }
                b'T' if data.get(i + 1) == Some(&b'j') => {
                    let s = pending.take().expect("string before Tj");
                    assert!(s.len().is_multiple_of(2), "two bytes per glyph");
                    out.push(
                        s.chunks(2)
                            .map(|c| {
                                *current
                                    .get(&u16::from_be_bytes([c[0], c[1]]))
                                    .expect("glyph in ToUnicode")
                            })
                            .collect(),
                    );
                    i += 2;
                }
                _ => i += 1,
            }
        }
        out
    }
    fn text(&self) -> Vec<String> {
        self.pages()
            .into_iter()
            .flat_map(|p| self.page_text(p))
            .collect()
    }
}

#[test]
fn pdf_is_well_formed_with_a_map_page() {
    let bytes = render_pdf_scaled(&sample(), None, 1).expect("pdf");
    let pdf = Parsed::parse(&bytes);
    let pages = pdf.pages();
    assert_eq!(pages.len(), 2, "text page and map page");
    let root = pdf
        .objects
        .values()
        .find(|(d, _)| d.contains("/Type /Pages"))
        .expect("page tree");
    assert!(root.0.contains("/Count 2"));
    assert!(root.0.contains("/Kids"));
    for page in pages {
        assert!(
            pdf.objects[&page].0.contains("/MediaBox [0 0 612 792]"),
            "Letter"
        );
    }
    assert!(
        pdf.objects
            .values()
            .any(|(d, _)| d.contains("/Subtype /Image")),
        "map image"
    );
    assert!(pdf.objects.values().any(|(d, _)| d.contains("/FontFile2")));
}
#[test]
fn text_is_extractable_in_order() {
    let bytes = render_pdf_scaled(&sample(), None, 1).expect("pdf");
    let text = Parsed::parse(&bytes).text();
    let joined = text.join("\n");
    for expected in [
        "Lost Coast Loop M\u{f6}nch",
        // The bundled font draws dashes with one glyph, so extraction yields the plain hyphen.
        "Jun 3-6, 2026 \u{b7} 3 nights",
        "Day 1",
        "12.0 mi",
        "2,100 ft",
        "40.2500, -124.0500",
        "Day 3",
        "Total",
        "42.5 mi",
        "9,800 ft",
        "PERMIT",
        "Bear canister required. Permit pickup at the ranger station.",
        "Permit attached: ",
        "permit (2026).pdf - PDF; kept with this trip in Strider.",
        "Base 14 lb 2 oz | Worn 3 lb",
        "Tent",
        "Oatmeal \u{b7} 600 kcal \u{b7} 4.2 oz",
        "Night 1 \u{b7} 12.0 mi \u{b7} 40.2500, -124.0500",
        "Route map",
    ] {
        assert!(
            text.iter().any(|t| t == expected),
            "missing {expected:?} in {joined}"
        );
    }
    let position = |s: &str| text.iter().position(|t| t == s).expect(s);
    assert!(position("Lost Coast Loop M\u{f6}nch") < position("Day 1"));
    assert!(position("Total") < position("PERMIT"));
    assert!(position("PERMIT") < position("GEAR"));
    assert!(position("GEAR") < position("FOOD"));
    assert!(position("FOOD") < position("CAMPSITES"));
    assert!(position("CAMPSITES") < position("Route map"));
}
#[test]
fn output_is_deterministic() {
    let sheet = sample();
    assert_eq!(
        render_pdf_scaled(&sheet, None, 1).expect("first"),
        render_pdf_scaled(&sheet, None, 1).expect("second")
    );
}
#[test]
fn long_lists_flow_onto_more_pages() {
    let mut sheet = sample();
    sheet.gear[0].lines = (0..120)
        .map(|i| GearLine {
            name: format!(
                "Item number {i} with a rather long descriptive name that must be shortened to fit"
            ),
            quantity: "1".into(),
            weight: "1 oz".into(),
            class: "base".into(),
        })
        .collect();
    sheet.track.clear();
    let pdf = Parsed::parse(&render_pdf_scaled(&sheet, None, 1).expect("pdf"));
    assert!(pdf.pages().len() >= 3, "120 rows need several pages");
    let text = pdf.text();
    assert!(text.iter().any(|t| t.starts_with("Item number 119")));
    assert!(
        !text.iter().any(|t| t == "Route map"),
        "no track, no map page"
    );
}
#[test]
fn characters_without_glyphs_do_not_break_text() {
    let mut sheet = sample();
    sheet.name = "Trip \u{4e2d} east".into();
    let text = Parsed::parse(&render_pdf_scaled(&sheet, None, 1).expect("pdf")).text();
    assert!(text.iter().any(|t| t == "Trip ? east"));
}
#[test]
fn map_without_tiles_is_the_map_color_with_a_route() {
    let sheet = sample();
    let map = render_map(&sheet.track, &sheet.palette, None, 300, 400).expect("map");
    assert_eq!(map.tiles_drawn, 0);
    assert_eq!(&map.rgb[..3], &sheet.palette.map, "corner is the map token");
    assert!(
        map.rgb.as_chunks::<3>().0.contains(&sheet.palette.route),
        "route is drawn"
    );
    let (x, y) = map.project(39.96, -105.29);
    assert!((0.0..300.0).contains(&x) && (0.0..400.0).contains(&y));
    assert!(render_map(&[[39.9, -105.3]], &sheet.palette, None, 10, 10).is_none());
}
#[test]
fn map_paints_cached_tiles_and_never_fetches() {
    let sheet = sample();
    let root = temp_dir("tiles");
    let (w, h) = (300_u32, 400_u32);
    let viewport = strider_map::fit_bounds(
        &sheet.track,
        f64::from(w),
        f64::from(h),
        f64::from(w.min(h)) * 0.06,
        16.,
    );
    let wanted = viewport.tiles(f64::from(w), f64::from(h), strider_map::Source::Topo);
    assert!(!wanted.is_empty());
    let mut png = Vec::new();
    let tile = image::RgbImage::from_pixel(256, 256, image::Rgb([10, 200, 30]));
    image::DynamicImage::ImageRgb8(tile)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("png");
    // Leave one tile missing: it is skipped, not requested.
    for t in wanted.iter().skip(1) {
        let path = root
            .join("topo")
            .join(t.z.to_string())
            .join(t.x.to_string())
            .join(t.y.to_string());
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, &png).expect("tile");
    }
    let map = render_map(&sheet.track, &sheet.palette, Some(&root), w, h).expect("map");
    assert_eq!(map.tiles_drawn, wanted.len() - 1);
    assert!(
        map.rgb.as_chunks::<3>().0.contains(&[10, 200, 30]),
        "tile pixels reach the image"
    );
    // The whole sheet renders with the tile root too, and still parses.
    let pdf = render_pdf_scaled(&sheet, Some(&root), 1).expect("pdf");
    assert_eq!(Parsed::parse(&pdf).pages().len(), 2);
    std::fs::remove_dir_all(root).expect("cleanup");
}
#[test]
fn numbered_campsites_are_real_text_on_the_map_page() {
    let pdf = Parsed::parse(&render_pdf_scaled(&sample(), None, 1).expect("pdf"));
    let page = *pdf.pages().last().expect("map page");
    let text = pdf.page_text(page);
    assert!(text.contains(&"1".to_owned()) && text.contains(&"2".to_owned()));
}
