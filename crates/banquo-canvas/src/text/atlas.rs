//! Loading a baked glyph atlas (format 2) and its font.
//!
//! `atlas.json` (format 2) carries per size the hinted `advance`, `ascent`,
//! `descent`, `height`, `line_height`, the page file name, `glyph_count` and
//! `glyphs`: glyph ID -> `[px, py, w, h, ox, oy]` (place on the page, bitmap
//! size, offset of the bitmap's top-left from the pen origin with y = 0 at the
//! top of the line box). Pages carry the 8-bit coverage in the red channel.
//! The top level records the measured `overlap_rule`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Deserialize;

use super::compose::AtlasRule;
use super::TextError;

/// The atlas format this loader reads.
pub const ATLAS_FORMAT: u32 = 2;
const ENTRY_FIELDS: [&str; 6] = ["px", "py", "w", "h", "ox", "oy"];

#[derive(Deserialize)]
struct FormatProbe {
    format: u32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AtlasDoc {
    format: u32,
    font: String,
    font_sha256: String,
    pua_base: u32,
    glyph_count: u32,
    source: BTreeMap<String, String>,
    tile_chars: Vec<String>,
    overlap_rule: String,
    entry_fields: Vec<String>,
    oy_origin: String,
    page_channel: String,
    sizes: BTreeMap<String, SizeDoc>,
}

#[derive(Deserialize)]
struct SizeDoc {
    advance: i32,
    ascent: i32,
    descent: i32,
    height: i32,
    line_height: i32,
    page: String,
    glyph_count: usize,
    glyphs: BTreeMap<String, [i32; 6]>,
}

/// One baked glyph bitmap.
#[derive(Debug, Clone)]
pub(crate) struct Glyph {
    pub w: i32,
    pub h: i32,
    pub ox: i32,
    pub oy: i32,
    /// `w * h` coverage bytes, row-major.
    pub alpha: Vec<u8>,
}

/// One baked size.
#[derive(Debug, Clone)]
pub(crate) struct SizeAtlas {
    pub advance: i32,
    pub ascent: i32,
    pub descent: i32,
    pub height: i32,
    pub line_height: i32,
    pub glyphs: HashMap<u32, Glyph>,
}

/// A font with its baked glyph atlas.
pub struct FontFace {
    pub(crate) ttf: Vec<u8>,
    #[cfg(feature = "shaping")]
    pub(crate) hb_font: harfrust::Font,
    pub(crate) sizes: BTreeMap<u32, SizeAtlas>,
    pub(crate) rule: AtlasRule,
    font_sha256: String,
}

impl std::fmt::Debug for FontFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFace")
            .field("sizes", &self.sizes.keys().collect::<Vec<_>>())
            .field("overlap_rule", &self.rule)
            .finish_non_exhaustive()
    }
}

impl FontFace {
    /// A face from the font file bytes, `atlas.json` and the PNG pages it
    /// names (`pages` maps each page file name to its bytes).
    ///
    /// # Errors
    ///
    /// [`TextError::UnsupportedFormat`] for any `format` but 2; other
    /// [`TextError`]s for a malformed atlas, a missing or bad page, a glyph
    /// outside its page or a font harfrust cannot read.
    pub fn from_baked(
        ttf: Vec<u8>,
        atlas_json: &[u8],
        pages: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, TextError> {
        let probe: FormatProbe = serde_json::from_slice(atlas_json)
            .map_err(|e| TextError::Atlas(format!("atlas.json: {e}")))?;
        if probe.format != ATLAS_FORMAT {
            return Err(TextError::UnsupportedFormat(probe.format));
        }
        let doc: AtlasDoc = serde_json::from_slice(atlas_json)
            .map_err(|e| TextError::Atlas(format!("atlas.json: {e}")))?;
        if doc.entry_fields != ENTRY_FIELDS {
            return Err(TextError::Atlas(format!(
                "entry_fields {:?}, expected {ENTRY_FIELDS:?}",
                doc.entry_fields
            )));
        }
        if doc.page_channel != "red" || doc.oy_origin != "top of line box" {
            return Err(TextError::Atlas(format!(
                "page_channel {:?} / oy_origin {:?}",
                doc.page_channel, doc.oy_origin
            )));
        }
        let rule = AtlasRule::parse(&doc.overlap_rule)
            .ok_or_else(|| TextError::UnknownOverlapRule(doc.overlap_rule.clone()))?;
        let mut sizes = BTreeMap::new();
        for (key, entry) in doc.sizes {
            let size: u32 = key
                .parse()
                .map_err(|_| TextError::Atlas(format!("size key {key:?}")))?;
            sizes.insert(size, load_size(size, entry, pages)?);
        }
        #[cfg(feature = "shaping")]
        let hb_font = harfrust::Font::new(ttf.clone(), 0).ok_or(TextError::BadFont)?;
        #[cfg(not(feature = "shaping"))]
        if super::shape::table(&ttf, b"cmap").is_none() {
            return Err(TextError::BadFont);
        }
        Ok(Self {
            ttf,
            #[cfg(feature = "shaping")]
            hb_font,
            sizes,
            rule,
            font_sha256: doc.font_sha256,
        })
    }

    /// [`FontFace::from_baked`] with `atlas.json` and its pages read from `dir`.
    ///
    /// # Errors
    ///
    /// [`TextError::Io`] when a file cannot be read, else as `from_baked`.
    pub fn from_dir(ttf: Vec<u8>, dir: &Path) -> Result<Self, TextError> {
        let read =
            |p: &Path| std::fs::read(p).map_err(|e| TextError::Io(format!("{}: {e}", p.display())));
        let json = read(&dir.join("atlas.json"))?;
        #[derive(Deserialize)]
        struct Pages {
            sizes: BTreeMap<String, PageOnly>,
        }
        #[derive(Deserialize)]
        struct PageOnly {
            page: String,
        }
        let mut pages = BTreeMap::new();
        // A format-1 atlas may not have this shape; let from_baked report it.
        if let Ok(names) = serde_json::from_slice::<Pages>(&json) {
            for p in names.sizes.into_values() {
                if let std::collections::btree_map::Entry::Vacant(slot) = pages.entry(p.page) {
                    let bytes = read(&dir.join(slot.key()))?;
                    slot.insert(bytes);
                }
            }
        }
        Self::from_baked(ttf, &json, &pages)
    }

    /// The baked sizes, ascending.
    pub fn sizes(&self) -> Vec<u32> {
        self.sizes.keys().copied().collect()
    }

    /// The overlap rule recorded in `atlas.json`.
    pub fn overlap_rule(&self) -> &'static str {
        self.rule.as_str()
    }

    /// The font SHA-256 recorded in `atlas.json`.
    pub fn atlas_font_sha256(&self) -> &str {
        &self.font_sha256
    }

    /// The font file bytes.
    pub fn ttf(&self) -> &[u8] {
        &self.ttf
    }

    /// Whether glyph `gid` is baked at `size_px`.
    pub fn has_glyph(&self, size_px: u32, gid: u32) -> bool {
        self.sizes
            .get(&size_px)
            .is_some_and(|s| s.glyphs.contains_key(&gid))
    }

    /// The number of glyphs baked at `size_px` (0 for an unbaked size).
    pub fn glyph_count(&self, size_px: u32) -> usize {
        self.sizes.get(&size_px).map_or(0, |s| s.glyphs.len())
    }

    /// pygame `Font.get_linesize()` at `size_px`.
    pub fn line_height(&self, size_px: u32) -> Option<i32> {
        self.sizes.get(&size_px).map(|s| s.line_height)
    }

    /// pygame `Font.get_height()` at `size_px` (the height of a rendered line).
    pub fn height(&self, size_px: u32) -> Option<i32> {
        self.sizes.get(&size_px).map(|s| s.height)
    }

    /// pygame `Font.get_ascent()` at `size_px`.
    pub fn ascent(&self, size_px: u32) -> Option<i32> {
        self.sizes.get(&size_px).map(|s| s.ascent)
    }

    /// pygame `Font.get_descent()` at `size_px` (negative).
    pub fn descent(&self, size_px: u32) -> Option<i32> {
        self.sizes.get(&size_px).map(|s| s.descent)
    }

    /// The hinted horizontal advance at `size_px` (every glyph of a monospace
    /// font).
    pub fn advance(&self, size_px: u32) -> Option<i32> {
        self.sizes.get(&size_px).map(|s| s.advance)
    }
}

fn load_size(
    size: u32,
    entry: SizeDoc,
    pages: &BTreeMap<String, Vec<u8>>,
) -> Result<SizeAtlas, TextError> {
    if entry.glyph_count != entry.glyphs.len() {
        return Err(TextError::Atlas(format!(
            "size {size}: glyph_count {} but {} glyphs",
            entry.glyph_count,
            entry.glyphs.len()
        )));
    }
    let png = pages
        .get(&entry.page)
        .ok_or_else(|| TextError::MissingPage(entry.page.clone()))?;
    let (pw, ph, red) =
        decode_red(png).map_err(|e| TextError::Page(format!("{}: {e}", entry.page)))?;
    let mut glyphs = HashMap::with_capacity(entry.glyphs.len());
    for (key, [px, py, w, h, ox, oy]) in entry.glyphs {
        let gid: u32 = key
            .parse()
            .map_err(|_| TextError::Atlas(format!("size {size}: glyph key {key:?}")))?;
        let fits = px >= 0
            && py >= 0
            && w >= 0
            && h >= 0
            && (w == 0 || h == 0 || (px + w <= pw && py + h <= ph));
        if !fits {
            return Err(TextError::GlyphOutsidePage { size, gid });
        }
        let mut alpha = Vec::with_capacity((w * h) as usize);
        if w > 0 && h > 0 {
            for row in py..py + h {
                let start = (row * pw + px) as usize;
                alpha.extend_from_slice(&red[start..start + w as usize]);
            }
        }
        glyphs.insert(
            gid,
            Glyph {
                w,
                h,
                ox,
                oy,
                alpha,
            },
        );
    }
    Ok(SizeAtlas {
        advance: entry.advance,
        ascent: entry.ascent,
        descent: entry.descent,
        height: entry.height,
        line_height: entry.line_height,
        glyphs,
    })
}

/// A PNG page's first (red, or gray) channel as (width, height, bytes).
fn decode_red(bytes: &[u8]) -> Result<(i32, i32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png size")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(format!("bit depth {:?}", info.bit_depth));
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => return Err(format!("colour type {other:?}")),
    };
    let (w, h) = (info.width as usize, info.height as usize);
    let red = buf[..w * h * channels]
        .iter()
        .step_by(channels)
        .copied()
        .collect();
    let w = i32::try_from(w).map_err(|_| "page too wide")?;
    let h = i32::try_from(h).map_err(|_| "page too tall")?;
    Ok((w, h, red))
}
