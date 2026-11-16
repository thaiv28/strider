//! Page layout: wrapped text, tables, pagination and the map page.
use crate::{Color, MapImage, Palette, PrintError, TripSheet, font, font::Font};
use pdf_writer::{Content, Name, Str, types::ColorSpaceOperand};
use std::mem;

pub(crate) const PAGE_W: f32 = 612.;
pub(crate) const PAGE_H: f32 = 792.;
const MARGIN: f32 = 48.;
const WIDTH: f32 = PAGE_W - 2. * MARGIN;
pub(crate) const DISPLAY: usize = 0;
pub(crate) const BODY: usize = 1;
pub(crate) const STRONG: usize = 2;

fn channels(c: Color) -> (f32, f32, f32) {
    (
        f32::from(c[0]) / 255.,
        f32::from(c[1]) / 255.,
        f32::from(c[2]) / 255.,
    )
}
pub(crate) struct Canvas {
    fonts: Vec<Font>,
    palette: Palette,
    pages: Vec<Vec<u8>>,
    content: Option<Content>,
    y: f32,
}
impl Canvas {
    pub fn new(palette: Palette) -> Result<Self, PrintError> {
        Ok(Self {
            fonts: vec![
                Font::new("F1", "BricolageGrotesque-Bold", font::DISPLAY)?,
                Font::new("F2", "InstrumentSans-Regular", font::BODY)?,
                Font::new("F3", "InstrumentSans-SemiBold", font::STRONG)?,
            ],
            palette,
            pages: vec![],
            content: Some(Self::blank()),
            y: PAGE_H - MARGIN,
        })
    }
    /// A page that draws in the RGB color space.
    fn blank() -> Content {
        let mut c = Content::new();
        c.set_fill_color_space(ColorSpaceOperand::DeviceRgb);
        c.set_stroke_color_space(ColorSpaceOperand::DeviceRgb);
        c
    }
    pub fn finish(mut self) -> (Vec<Vec<u8>>, Vec<Font>) {
        if let Some(last) = self.content.take() {
            self.pages.push(last.finish().to_vec());
        }
        (self.pages, self.fonts)
    }
    fn new_page(&mut self) {
        if let Some(done) = self.content.replace(Self::blank()) {
            self.pages.push(done.finish().to_vec());
        }
        self.y = PAGE_H - MARGIN;
    }
    /// The page being drawn. It exists until `finish`, which consumes the canvas.
    fn c(&mut self) -> &mut Content {
        self.content.get_or_insert_with(Self::blank)
    }
    /// Start a new page unless `height` still fits.
    fn need(&mut self, height: f32) {
        if self.y - height < MARGIN {
            self.new_page();
        }
    }
    fn text(&mut self, font: usize, size: f32, color: Color, x: f32, baseline: f32, s: &str) {
        if s.is_empty() {
            return;
        }
        let bytes = self.fonts[font].encode(s);
        let resource = self.fonts[font].resource;
        let (r, g, b) = channels(color);
        let c = self.c();
        c.begin_text();
        c.set_fill_color([r, g, b]);
        c.set_font(Name(resource.as_bytes()), size);
        c.next_line(x, baseline);
        c.show(Str(&bytes));
        c.end_text();
    }
    fn width(&self, font: usize, size: f32, s: &str) -> f32 {
        self.fonts[font].width(s, size)
    }
    fn rule(&mut self, y: f32, weight: f32, color: Color) {
        let (r, g, b) = channels(color);
        let c = self.c();
        c.set_stroke_color([r, g, b]);
        c.set_line_width(weight);
        c.move_to(MARGIN, y);
        c.line_to(MARGIN + WIDTH, y);
        c.stroke();
    }
    /// Greedy word wrap; words wider than the line are broken by character.
    fn wrap(&self, font: usize, size: f32, width: f32, text: &str) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.lines() {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if self.width(font, size, &candidate) <= width {
                    line = candidate;
                    continue;
                }
                if !line.is_empty() {
                    lines.push(mem::take(&mut line));
                }
                let mut rest = word.to_owned();
                while self.width(font, size, &rest) > width {
                    let mut cut = rest.chars().count().saturating_sub(1).max(1);
                    while cut > 1
                        && self.width(font, size, &rest.chars().take(cut).collect::<String>())
                            > width
                    {
                        cut -= 1;
                    }
                    lines.push(rest.chars().take(cut).collect());
                    rest = rest.chars().skip(cut).collect();
                }
                line = rest;
            }
            lines.push(line);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }
    /// Shorten to fit, ending in "...".
    fn fit(&self, font: usize, size: f32, width: f32, s: &str) -> String {
        if self.width(font, size, s) <= width {
            return s.to_owned();
        }
        let mut out: String = s.to_owned();
        while !out.is_empty() && self.width(font, size, &format!("{out}...")) > width {
            out.pop();
        }
        format!("{}...", out.trim_end())
    }
    /// One line of text at the cursor.
    fn line(&mut self, font: usize, size: f32, color: Color, x: f32, leading: f32, s: &str) {
        self.need(leading);
        let baseline = self.y - size * 0.92;
        self.text(font, size, color, x, baseline, s);
        self.y -= leading;
    }
    fn paragraph(&mut self, font: usize, size: f32, color: Color, leading: f32, text: &str) {
        for line in self.wrap(font, size, WIDTH, text) {
            self.line(font, size, color, MARGIN, leading, &line);
        }
    }
    fn heading(&mut self, label: &str) {
        self.need(60.);
        self.y -= 8.;
        let muted = self.palette.muted;
        self.line(STRONG, 8.5, muted, MARGIN, 14., &label.to_uppercase());
    }
    pub fn header(&mut self, sheet: &TripSheet) {
        let (ink, muted) = (self.palette.ink, self.palette.muted);
        for line in self.wrap(DISPLAY, 22., WIDTH, &sheet.name) {
            self.line(DISPLAY, 22., ink, MARGIN, 26., &line);
        }
        self.y -= 2.;
        self.paragraph(BODY, 10.5, muted, 15., &sheet.when);
        if let Some(place) = &sheet.place {
            self.paragraph(BODY, 10.5, muted, 15., place);
        }
        self.paragraph(BODY, 10.5, muted, 15., &sheet.summary);
        self.y -= 10.;
    }
    fn columns(&self) -> [f32; 4] {
        [MARGIN, MARGIN + 82., MARGIN + 174., MARGIN + 266.]
    }
    fn table_row(&mut self, font: usize, row: &crate::DayRow) {
        let ink = self.palette.ink;
        self.need(18.);
        let baseline = self.y - 12.5;
        let x = self.columns();
        for (x, value) in x
            .iter()
            .zip([&row.day, &row.distance, &row.gain, &row.camp])
        {
            self.text(font, 10., ink, *x, baseline, value);
        }
        self.y -= 18.;
    }
    pub fn day_table(&mut self, sheet: &TripSheet) {
        if sheet.days.is_empty() {
            return;
        }
        let (ink, line) = (self.palette.ink, self.palette.line);
        self.table_row(
            STRONG,
            &crate::DayRow {
                day: "Day".into(),
                distance: "Distance".into(),
                gain: "Gain".into(),
                camp: "Camp (lat, lon)".into(),
            },
        );
        self.rule(self.y, 1.4, ink);
        for row in &sheet.days {
            self.table_row(BODY, row);
            self.rule(self.y, 0.5, line);
        }
        self.rule(self.y, 1.4, ink);
        self.table_row(STRONG, &sheet.totals);
        self.y -= 8.;
    }
    pub fn permit(&mut self, sheet: &TripSheet) {
        let (ink, muted, line) = (self.palette.ink, self.palette.muted, self.palette.line);
        if let Some(notes) = sheet.permit_notes.as_ref().filter(|n| !n.trim().is_empty()) {
            self.heading("Permit");
            self.paragraph(BODY, 10., ink, 14., notes);
            self.y -= 4.;
        }
        if let Some(doc) = &sheet.permit_document {
            let label = "Permit attached: ";
            let rest = format!(
                "{} - {}; kept with this trip in Strider.",
                doc.filename, doc.kind
            );
            self.need(32.);
            let top = self.y;
            let c = self.c();
            let (r, g, b) = channels(line);
            c.set_stroke_color([r, g, b]);
            c.set_line_width(0.8);
            c.rect(MARGIN, top - 26., WIDTH, 26.);
            c.stroke();
            let w = self.width(STRONG, 10., label);
            self.text(STRONG, 10., ink, MARGIN + 10., top - 17., label);
            let fitted = self.fit(BODY, 10., WIDTH - 20. - w, &rest);
            self.text(BODY, 10., muted, MARGIN + 10. + w, top - 17., &fitted);
            self.y = top - 36.;
        }
    }
    pub fn gear(&mut self, sheet: &TripSheet) {
        if sheet.gear.is_empty() {
            return;
        }
        let (ink, muted, line) = (self.palette.ink, self.palette.muted, self.palette.line);
        self.heading("Gear");
        if !sheet.gear_totals.is_empty() {
            let totals = sheet
                .gear_totals
                .iter()
                .map(|(label, value)| format!("{label} {value}"))
                .collect::<Vec<_>>()
                .join("  |  ");
            self.paragraph(BODY, 10., ink, 14., &totals);
            self.y -= 4.;
        }
        let x = [MARGIN, MARGIN + 300., MARGIN + 340., MARGIN + 430.];
        for section in &sheet.gear {
            self.need(18. * 3.);
            self.line(STRONG, 10., ink, MARGIN, 18., &section.category);
            self.rule(self.y + 3., 0.5, line);
            for l in &section.lines {
                self.need(16.);
                let baseline = self.y - 11.5;
                let name = self.fit(BODY, 9.5, 290., &l.name);
                self.text(BODY, 9.5, ink, x[0], baseline, &name);
                self.text(BODY, 9.5, muted, x[1], baseline, &l.quantity);
                self.text(BODY, 9.5, ink, x[2], baseline, &l.weight);
                self.text(BODY, 9.5, muted, x[3], baseline, &l.class);
                self.y -= 16.;
            }
            self.y -= 4.;
        }
    }
    pub fn food(&mut self, sheet: &TripSheet) {
        if sheet.food.is_empty() {
            return;
        }
        let (ink, muted) = (self.palette.ink, self.palette.muted);
        self.heading("Food");
        for day in &sheet.food {
            self.need(16. * 2.);
            self.line(STRONG, 10., ink, MARGIN, 16., &day.heading);
            for entry in &day.lines {
                for (i, text) in self
                    .wrap(BODY, 9.5, WIDTH - 14., entry)
                    .into_iter()
                    .enumerate()
                {
                    self.line(
                        BODY,
                        9.5,
                        muted,
                        MARGIN + if i == 0 { 8. } else { 18. },
                        14.,
                        &text,
                    );
                }
            }
            self.y -= 4.;
        }
    }
    pub fn campsites(&mut self, sheet: &TripSheet) {
        if sheet.campsites.is_empty() {
            return;
        }
        let ink = self.palette.ink;
        self.heading("Campsites");
        for text in &sheet.campsites {
            self.line(BODY, 10., ink, MARGIN, 15., text);
        }
    }
    /// Size of the map in points: the page below its heading and above the credit line.
    pub fn map_box(&self) -> (f32, f32) {
        (WIDTH, PAGE_H - 2. * MARGIN - 44.)
    }
    pub fn map_page(&mut self, map: &MapImage, sheet: &TripSheet, scale: u32, credit: bool) {
        self.new_page();
        let (ink, muted) = (self.palette.ink, self.palette.muted);
        self.line(STRONG, 11., ink, MARGIN, 22., "Route map");
        let (w, h) = self.map_box();
        let (left, top) = (MARGIN, self.y - 4.);
        let bottom = top - h;
        let c = self.c();
        c.save_state();
        c.transform([w, 0., 0., h, left, bottom]);
        c.x_object(Name(b"Im1"));
        c.restore_state();
        let k = scale as f64;
        for m in &sheet.markers {
            let (px, py) = map.project(m.lat, m.lon);
            let (x, y) = (left + (px / k) as f32, top - (py / k) as f32);
            if x < left || x > left + w || y > top || y < bottom {
                continue;
            }
            self.marker(x, y, m.night);
        }
        if credit {
            self.text(
                BODY,
                8.,
                muted,
                MARGIN,
                bottom - 12.,
                "Map: USGS The National Map",
            );
        }
    }
    fn marker(&mut self, x: f32, y: f32, night: u32) {
        let (accent, map) = (self.palette.accent, self.palette.map);
        let r = 8.;
        let k = 0.5523 * r;
        let c = self.c();
        let (fr, fg, fb) = channels(accent);
        let (sr, sg, sb) = channels(map);
        c.set_fill_color([fr, fg, fb]);
        c.set_stroke_color([sr, sg, sb]);
        c.set_line_width(1.5);
        c.move_to(x + r, y);
        c.cubic_to(x + r, y + k, x + k, y + r, x, y + r);
        c.cubic_to(x - k, y + r, x - r, y + k, x - r, y);
        c.cubic_to(x - r, y - k, x - k, y - r, x, y - r);
        c.cubic_to(x + k, y - r, x + r, y - k, x + r, y);
        c.close_path();
        c.fill_nonzero_and_stroke();
        let label = night.to_string();
        let w = self.width(STRONG, 8., &label);
        self.text(STRONG, 8., map, x - w / 2., y - 2.8, &label);
    }
}
