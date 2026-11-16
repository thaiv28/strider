//! Embedded TrueType fonts addressed by glyph id, with a ToUnicode map so text stays extractable.
use crate::PrintError;
use std::collections::BTreeMap;
use ttf_parser::Face;

/// The app's bundled faces (the same files the UI registers).
pub(crate) const DISPLAY: &[u8] =
    include_bytes!("../../../assets/fonts/BricolageGrotesque-Bold.ttf");
pub(crate) const BODY: &[u8] = include_bytes!("../../../assets/fonts/InstrumentSans-Regular.ttf");
pub(crate) const STRONG: &[u8] =
    include_bytes!("../../../assets/fonts/InstrumentSans-SemiBold.ttf");

pub(crate) struct Font {
    pub resource: &'static str,
    pub base_font: &'static str,
    pub data: &'static [u8],
    face: Face<'static>,
    /// Glyphs used so far and the character that first selected each.
    pub used: BTreeMap<u16, char>,
}
impl Font {
    pub fn new(
        resource: &'static str,
        base_font: &'static str,
        data: &'static [u8],
    ) -> Result<Self, PrintError> {
        let face = Face::parse(data, 0).map_err(|e| PrintError::Font(e.to_string()))?;
        Ok(Self {
            resource,
            base_font,
            data,
            face,
            used: BTreeMap::new(),
        })
    }
    pub fn units(&self) -> f32 {
        f32::from(self.face.units_per_em())
    }
    pub fn ascent(&self) -> f32 {
        f32::from(self.face.ascender()) * 1000. / self.units()
    }
    pub fn descent(&self) -> f32 {
        f32::from(self.face.descender()) * 1000. / self.units()
    }
    pub fn cap_height(&self) -> f32 {
        self.face
            .capital_height()
            .map_or(700., |h| f32::from(h) * 1000. / self.units())
    }
    pub fn bbox(&self) -> [f32; 4] {
        let b = self.face.global_bounding_box();
        let k = 1000. / self.units();
        [
            f32::from(b.x_min) * k,
            f32::from(b.y_min) * k,
            f32::from(b.x_max) * k,
            f32::from(b.y_max) * k,
        ]
    }
    /// Advance of a glyph in 1/1000 em.
    pub fn glyph_width(&self, glyph: u16) -> f32 {
        f32::from(
            self.face
                .glyph_hor_advance(ttf_parser::GlyphId(glyph))
                .unwrap_or(0),
        ) * 1000.
            / self.units()
    }
    /// The glyph used for a character: itself, a plain look-alike, then '?'.
    pub fn glyph(&self, c: char) -> u16 {
        let c = match c {
            '\u{2014}' | '\u{2013}' | '\u{2212}' => '-',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            '\u{00a0}' | '\t' => ' ',
            c => c,
        };
        let id = |c: char| self.face.glyph_index(c).map(|g| g.0);
        id(c).or_else(|| id('?')).unwrap_or(0)
    }
    /// A character with its own glyph, or the one that stands in for it.
    fn shown(&self, c: char) -> char {
        if self.face.glyph_index(c).is_some() {
            c
        } else {
            match c {
                '\u{2014}' | '\u{2013}' | '\u{2212}' => '-',
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{201c}' | '\u{201d}' => '"',
                '\u{00a0}' | '\t' => ' ',
                _ => '?',
            }
        }
    }
    pub fn width(&self, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|c| self.glyph_width(self.glyph(self.shown(c))))
            .sum::<f32>()
            * size
            / 1000.
    }
    /// Two big-endian bytes per glyph; records each glyph for the width and ToUnicode tables.
    pub fn encode(&mut self, text: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(text.len() * 2);
        for c in text.chars() {
            let shown = self.shown(c);
            let glyph = self.glyph(shown);
            // Look-alikes can share a glyph; the plain ASCII character is the safest text.
            let entry = self.used.entry(glyph).or_insert(shown);
            if !entry.is_ascii() && shown.is_ascii() {
                *entry = shown;
            }
            out.extend_from_slice(&glyph.to_be_bytes());
        }
        out
    }
}
