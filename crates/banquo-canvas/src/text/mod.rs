//! Baked-font text: shaping, measuring and rendering like pygame 2.6.1's
//! `Font.size` and `Font.render(text, True, colour)` (SDL_ttf 2.20.1).
//!
//! A [`FontFace`] is a font file plus its baked glyph atlas (format 2, see
//! [`FontFace::from_baked`]). Text is shaped by harfrust with SDL_ttf 2.20.1's
//! settings (see [`shape()`]: direction LTR, script `Zzzz`, no language, no
//! user features), laid out on a pen that moves by the size's hinted advance
//! after each glyph with a shaped advance, and composed from the atlas
//! bitmaps.
//!
//! # Overlap rule
//!
//! Where two glyph bitmaps overlap (for example the right side bearing of
//! `&` running into the next cell), their coverages combine by the rule the
//! atlas records in `overlap_rule`. SDL_ttf 2.20.1's blended renderer ORs the
//! 8-bit coverage into the destination (`BG_Blended_Opaque`:
//! `*dst |= *src << 24`), so the measured rule for the bundled JetBrains Mono
//! atlas is **`or`**: rendering with it gives 0 differing pixels on every
//! rendered corpus entry, while `max` and `over` each differ on some overlap
//! strings. All three rules are implemented ([`OverlapRule`]); an atlas that
//! records `no-overlap-observed` renders with `or`.
//!
//! # Zero width
//!
//! A non-empty string whose run has no width at all (U+00AD SOFT HYPHEN
//! alone: HarfBuzz gives it no advance and it has no ink) is refused by
//! SDL_ttf with "Text has zero width", so pygame raises. [`measure`] still
//! returns pygame's `Font.size` result `(0, height)`; [`try_render`] returns
//! [`TextError::ZeroWidth`]; [`render`] returns a `0 x height` surface, the
//! surface pygame returns for an empty string. A zero-advance glyph inside a
//! string (`"a\u{ad}b"`) takes no room, as in pygame.
//!
//! Colour: every pixel's RGB is the colour and A is the coverage; the
//! colour's own alpha is ignored, as pygame renders text opaque.

mod atlas;
pub mod classify;
mod compose;
mod shape;

pub use atlas::{FontFace, ATLAS_FORMAT};
pub use compose::OverlapRule;
/// harfrust's direction and script types, as [`DIRECTION`] and [`SCRIPT`] use them.
#[cfg(feature = "shaping")]
pub use harfrust::{Direction as ShapeDirection, Script as ShapeScript};
pub use shape::shape;
#[cfg(feature = "shaping")]
pub use shape::{shaping_buffer, DIRECTION, SCRIPT, USER_FEATURES};

use crate::{Rgba8, Surface};

/// Errors from loading an atlas or rendering text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextError {
    /// `atlas.json` has a `format` this loader does not read (only 2).
    UnsupportedFormat(u32),
    /// `atlas.json` is malformed.
    Atlas(String),
    /// `overlap_rule` is not `max`, `over`, `or` or `no-overlap-observed`.
    UnknownOverlapRule(String),
    /// A page named by `atlas.json` was not supplied.
    MissingPage(String),
    /// A page could not be decoded.
    Page(String),
    /// A glyph's rectangle lies outside its page.
    GlyphOutsidePage { size: u32, gid: u32 },
    /// The font file could not be read.
    BadFont,
    /// A file could not be read.
    Io(String),
    /// The size is not baked.
    UnknownSize(u32),
    /// A glyph of the run is not baked at this size.
    MissingGlyph { size: u32, gid: u32 },
    /// A non-empty string with no width (SDL_ttf: "Text has zero width").
    ZeroWidth,
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::UnsupportedFormat(v) => write!(f, "unsupported atlas format {v}"),
            TextError::Atlas(m) => write!(f, "bad atlas: {m}"),
            TextError::UnknownOverlapRule(r) => write!(f, "unknown overlap_rule {r:?}"),
            TextError::MissingPage(p) => write!(f, "missing atlas page {p}"),
            TextError::Page(m) => write!(f, "bad atlas page: {m}"),
            TextError::GlyphOutsidePage { size, gid } => {
                write!(f, "glyph {gid} at size {size} lies outside its page")
            }
            TextError::BadFont => write!(f, "the font could not be read"),
            TextError::Io(m) => write!(f, "{m}"),
            TextError::UnknownSize(s) => write!(f, "size {s} is not baked"),
            TextError::MissingGlyph { size, gid } => {
                write!(f, "glyph {gid} is not baked at size {size}")
            }
            TextError::ZeroWidth => write!(f, "text has zero width"),
        }
    }
}

impl std::error::Error for TextError {}

/// pygame `Font.size(text)` at `size_px`: `(width, height)`.
///
/// An empty string gives `(0, line_height)`; an unbaked size `(0, 0)`. A
/// glyph not baked at this size still moves the pen but adds no ink box.
pub fn measure(face: &FontFace, size_px: u32, text: &str) -> (i32, i32) {
    let Some(size) = face.sizes.get(&size_px) else {
        return (0, 0);
    };
    let run = shape::run(face, text);
    let l = compose::layout(size, &run);
    (l.width, l.height)
}

/// pygame `Font.render(text, True, colour)` at `size_px` under the atlas's
/// overlap rule. See the module docs for zero-width text; an unbaked size
/// gives a `0 x 0` surface and an unbaked glyph is left out.
pub fn render(face: &FontFace, size_px: u32, text: &str, colour: Rgba8) -> Surface {
    render_with_rule(face, size_px, text, colour, face.rule.render_rule())
}

/// [`render`] with an explicit overlap rule.
pub fn render_with_rule(
    face: &FontFace,
    size_px: u32,
    text: &str,
    colour: Rgba8,
    rule: OverlapRule,
) -> Surface {
    let Some(size) = face.sizes.get(&size_px) else {
        return Surface::new(0, 0, true);
    };
    let run = shape::run(face, text);
    let l = compose::layout(size, &run);
    paint(&l, colour, rule)
}

/// [`render`], but failing where pygame fails or the atlas cannot draw the
/// run exactly.
///
/// # Errors
///
/// [`TextError::UnknownSize`], [`TextError::MissingGlyph`] for the first
/// glyph not baked at this size, [`TextError::ZeroWidth`] for a non-empty
/// string with no width (pygame raises "Text has zero width").
pub fn try_render(
    face: &FontFace,
    size_px: u32,
    text: &str,
    colour: Rgba8,
) -> Result<Surface, TextError> {
    let size = face
        .sizes
        .get(&size_px)
        .ok_or(TextError::UnknownSize(size_px))?;
    let run = shape::run(face, text);
    let l = compose::layout(size, &run);
    if let Some(&gid) = l.missing.first() {
        return Err(TextError::MissingGlyph { size: size_px, gid });
    }
    if !text.is_empty() && l.width == 0 {
        return Err(TextError::ZeroWidth);
    }
    Ok(paint(&l, colour, face.rule.render_rule()))
}

fn paint(l: &compose::Layout<'_>, colour: Rgba8, rule: OverlapRule) -> Surface {
    let (w, h) = (l.width.max(0) as u32, l.height.max(0) as u32);
    let mut s = Surface::new(w, h, true);
    if w == 0 || h == 0 {
        return s;
    }
    // SDL_ttf fills the blended surface with the colour at alpha 0.
    s.fill(Rgba8::new(colour.r, colour.g, colour.b, 0));
    let cov = compose::coverage(l, rule);
    for (i, &a) in cov.iter().enumerate() {
        if a != 0 {
            let (x, y) = ((i as u32 % w) as i32, (i as u32 / w) as i32);
            s.set(x, y, Rgba8::new(colour.r, colour.g, colour.b, a));
        }
    }
    s
}
