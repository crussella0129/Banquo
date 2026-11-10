//! Shaping a string into glyph IDs, configured like SDL_ttf 2.20.1.
//!
//! SDL_ttf 2.20.1 (`SDL_ttf.c` at tag `release-2.20.1`) shapes every line with
//! a fresh HarfBuzz buffer: direction `HB_DIRECTION_LTR` (the global default,
//! line 89, set on the buffer at line 3210), script `HB_SCRIPT_UNKNOWN` (`Zzzz`,
//! line 90, set at line 3211), no language and no segment-property guessing,
//! then `hb_shape(font, buffer, NULL, 0)` (line 3215): no user features, so
//! only the font's default features apply. `TTF_SetFontKerning` only affects
//! the non-HarfBuzz path. [`shape`] does the same through harfrust 0.15.
//!
//! Without the `shaping` feature, text falls back to an unshaped run: one
//! glyph per character through the font's `cmap`, every glyph advancing.

use super::FontFace;

/// One shaped glyph: its ID and whether the shaper gave it an advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShapedGlyph {
    pub gid: u32,
    /// The shaper's horizontal advance in font units (0 for a glyph that
    /// takes no room, such as U+00AD SOFT HYPHEN).
    pub x_advance: i32,
}

#[cfg(feature = "shaping")]
mod hb {
    use harfrust::{Buffer, Direction, Feature, Script, ShapeOptions, ShaperFont};

    use super::{FontFace, ShapedGlyph};

    /// SDL_ttf 2.20.1's default direction (`g_hb_direction`, `SDL_ttf.c:89`).
    pub const DIRECTION: Direction = Direction::LeftToRight;
    /// SDL_ttf 2.20.1's default script (`g_hb_script = HB_SCRIPT_UNKNOWN`,
    /// `SDL_ttf.c:90`), the ISO 15924 tag `Zzzz`.
    pub const SCRIPT: Script = Script::UNKNOWN;
    /// The user features SDL_ttf 2.20.1 passes to `hb_shape` (`NULL, 0`,
    /// `SDL_ttf.c:3215`): none.
    pub const USER_FEATURES: &[Feature] = &[];

    /// The buffer [`super::shape`] shapes: `text` with direction
    /// [`DIRECTION`], script [`SCRIPT`] and no language. Segment properties
    /// are never guessed.
    pub fn shaping_buffer(text: &str) -> Buffer {
        let mut buffer = Buffer::new();
        buffer.push_str(text);
        buffer.set_direction(DIRECTION);
        buffer.set_script(Some(SCRIPT));
        buffer.set_language(None);
        buffer
    }

    pub(crate) fn run(face: &FontFace, text: &str, features: &[Feature]) -> Vec<ShapedGlyph> {
        if text.is_empty() {
            return Vec::new();
        }
        let font = ShaperFont::new(&face.hb_font);
        let mut buffer = shaping_buffer(text);
        if harfrust::shape(&font, &mut buffer, ShapeOptions::new().features(features)).is_err() {
            return Vec::new();
        }
        buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(info, pos)| ShapedGlyph {
                gid: info.glyph_id,
                x_advance: pos.x_advance,
            })
            .collect()
    }

    /// Every GSUB/GPOS feature switched off, for the shaping/raster split.
    pub(crate) fn run_features_off(face: &FontFace, text: &str) -> Vec<ShapedGlyph> {
        let off: Vec<Feature> = super::feature_tags(&face.ttf)
            .iter()
            .map(|tag| Feature::new(harfrust::Tag::new(tag), 0, ..))
            .collect();
        run(face, text, &off)
    }
}

#[cfg(feature = "shaping")]
pub use hb::{shaping_buffer, DIRECTION, SCRIPT, USER_FEATURES};

/// The glyph run SDL_ttf would lay out for `text`, in glyph order.
pub(crate) fn run(face: &FontFace, text: &str) -> Vec<ShapedGlyph> {
    #[cfg(feature = "shaping")]
    {
        hb::run(face, text, USER_FEATURES)
    }
    #[cfg(not(feature = "shaping"))]
    {
        unshaped(face, text)
    }
}

/// The glyph run with every font feature off (used only to classify a
/// difference; equal to [`run`] without the `shaping` feature).
pub(crate) fn run_features_off(face: &FontFace, text: &str) -> Vec<ShapedGlyph> {
    #[cfg(feature = "shaping")]
    {
        hb::run_features_off(face, text)
    }
    #[cfg(not(feature = "shaping"))]
    {
        unshaped(face, text)
    }
}

#[cfg(not(feature = "shaping"))]
fn unshaped(face: &FontFace, text: &str) -> Vec<ShapedGlyph> {
    text.chars()
        .map(|c| ShapedGlyph {
            gid: cmap_lookup(&face.ttf, u32::from(c)),
            x_advance: 1,
        })
        .collect()
}

/// The glyph IDs `text` shapes to, in glyph order (SDL_ttf 2.20.1 settings).
pub fn shape(face: &FontFace, text: &str) -> Vec<u32> {
    run(face, text).iter().map(|g| g.gid).collect()
}

// --- minimal sfnt reading -------------------------------------------------

fn be16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn be32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// The bytes of table `tag` in a single-font sfnt.
pub(crate) fn table<'a>(font: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let count = usize::from(be16(font, 4)?);
    (0..count).find_map(|i| {
        let rec = 12 + i * 16;
        if font.get(rec..rec + 4)? != tag {
            return None;
        }
        let offset = be32(font, rec + 8)? as usize;
        let length = be32(font, rec + 12)? as usize;
        font.get(offset..offset.checked_add(length)?)
    })
}

/// Every feature tag in the font's GSUB and GPOS feature lists, sorted.
#[cfg_attr(not(feature = "shaping"), allow(dead_code))]
pub(crate) fn feature_tags(font: &[u8]) -> Vec<[u8; 4]> {
    let mut tags = Vec::new();
    for name in [b"GSUB", b"GPOS"] {
        let Some(t) = table(font, name) else { continue };
        let Some(list) = be16(t, 6).map(usize::from) else {
            continue;
        };
        let count = be16(t, list).map_or(0, usize::from);
        for i in 0..count {
            let at = list + 2 + i * 6;
            if let Some(tag) = t.get(at..at + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()) {
                tags.push(tag);
            }
        }
    }
    tags.sort_unstable();
    tags.dedup();
    tags
}

/// The glyph ID the font's Unicode `cmap` (format 4 or 12) gives `cp`, or 0.
#[cfg_attr(feature = "shaping", allow(dead_code))]
pub(crate) fn cmap_lookup(font: &[u8], cp: u32) -> u32 {
    let Some(cmap) = table(font, b"cmap") else {
        return 0;
    };
    let count = be16(cmap, 2).map_or(0, usize::from);
    let mut best: Option<(u8, usize)> = None;
    for i in 0..count {
        let rec = 4 + i * 8;
        let (Some(pid), Some(eid), Some(off)) =
            (be16(cmap, rec), be16(cmap, rec + 2), be32(cmap, rec + 4))
        else {
            continue;
        };
        let format = be16(cmap, off as usize).unwrap_or(0);
        let rank = match (pid, eid, format) {
            (3, 10, 12) | (0, 4, 12) | (0, 6, 12) => 2,
            (3, 1, 4) | (0, _, 4) => 1,
            _ => continue,
        };
        if best.is_none_or(|(r, _)| rank > r) {
            best = Some((rank, off as usize));
        }
    }
    let Some((_, off)) = best else { return 0 };
    let Some(sub) = cmap.get(off..) else { return 0 };
    match be16(sub, 0) {
        Some(4) => cmap4(sub, cp).unwrap_or(0),
        Some(12) => cmap12(sub, cp).unwrap_or(0),
        _ => 0,
    }
}

fn cmap4(sub: &[u8], cp: u32) -> Option<u32> {
    let cp = u16::try_from(cp).ok()?;
    let segs = usize::from(be16(sub, 6)? / 2);
    let ends = 14;
    let starts = ends + segs * 2 + 2;
    let deltas = starts + segs * 2;
    let ranges = deltas + segs * 2;
    for s in 0..segs {
        let end = be16(sub, ends + s * 2)?;
        if cp > end {
            continue;
        }
        let start = be16(sub, starts + s * 2)?;
        if cp < start {
            return Some(0);
        }
        let delta = be16(sub, deltas + s * 2)?;
        let range = be16(sub, ranges + s * 2)?;
        if range == 0 {
            return Some(u32::from(cp.wrapping_add(delta)));
        }
        let at = ranges + s * 2 + usize::from(range) + usize::from(cp - start) * 2;
        let g = be16(sub, at)?;
        return Some(if g == 0 {
            0
        } else {
            u32::from(g.wrapping_add(delta))
        });
    }
    Some(0)
}

fn cmap12(sub: &[u8], cp: u32) -> Option<u32> {
    let groups = be32(sub, 12)? as usize;
    for i in 0..groups {
        let at = 16 + i * 12;
        let (start, end, gid) = (be32(sub, at)?, be32(sub, at + 4)?, be32(sub, at + 8)?);
        if (start..=end).contains(&cp) {
            return Some(gid + (cp - start));
        }
    }
    Some(0)
}

#[cfg(all(test, feature = "shaping"))]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../tests/fixtures/fonts/JetBrainsMono-Regular.ttf");

    #[test]
    fn cmap_fallback_agrees_with_harfrust() {
        let font = harfrust::Font::new(FONT.to_vec(), 0).unwrap();
        let shaper = harfrust::ShaperFont::new(&font);
        for cp in (0x20..0x250).chain(0x370..0x460).chain([0x4e2d, 0x1f600]) {
            let want = shaper.nominal_glyph(cp).map_or(0, |g| g.to_u32());
            assert_eq!(cmap_lookup(FONT, cp), want, "U+{cp:04X}");
        }
    }

    #[test]
    fn font_has_calt_feature() {
        let tags = feature_tags(FONT);
        assert!(tags.contains(b"calt"));
        assert!(tags.windows(2).all(|w| w[0] < w[1]));
    }
}
