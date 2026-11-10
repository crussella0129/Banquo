//! Laying out a shaped run and composing its glyph coverage, as SDL_ttf
//! 2.20.1 does for `TTF_RenderUTF8_Blended`.

use super::atlas::{Glyph, SizeAtlas};
use super::shape::ShapedGlyph;

/// How two overlapping glyph coverages combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlapRule {
    /// The larger of the two 8-bit coverages.
    Max,
    /// `src + dst * (255 - src) / 255`, rounded half up, in glyph order.
    Over,
    /// Bitwise OR of the 8-bit coverages: SDL_ttf 2.20.1's `BG_Blended_Opaque`
    /// does `*dst |= *src << 24` (`SDL_ttf.c`, tag `release-2.20.1`).
    Or,
}

impl OverlapRule {
    /// All three rules.
    pub const ALL: [OverlapRule; 3] = [OverlapRule::Max, OverlapRule::Over, OverlapRule::Or];

    /// The rule's name in `atlas.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            OverlapRule::Max => "max",
            OverlapRule::Over => "over",
            OverlapRule::Or => "or",
        }
    }

    /// `dst` and `src` combined (both non-zero; zero takes the other).
    #[inline]
    pub fn combine(self, dst: u8, src: u8) -> u8 {
        if dst == 0 {
            return src;
        }
        if src == 0 {
            return dst;
        }
        match self {
            OverlapRule::Max => dst.max(src),
            OverlapRule::Over => {
                let (d, s) = (u32::from(dst), u32::from(src));
                // s + d * (255 - s) / 255 never exceeds 255.
                (s + (d * (255 - s) + 127) / 255) as u8
            }
            OverlapRule::Or => dst | src,
        }
    }
}

/// The `overlap_rule` an atlas records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtlasRule {
    Rule(OverlapRule),
    /// No corpus entry told the rules apart; rendering uses [`OverlapRule::Or`],
    /// SDL_ttf 2.20.1's own operation.
    NoOverlapObserved,
}

impl AtlasRule {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "max" => AtlasRule::Rule(OverlapRule::Max),
            "over" => AtlasRule::Rule(OverlapRule::Over),
            "or" => AtlasRule::Rule(OverlapRule::Or),
            "no-overlap-observed" => AtlasRule::NoOverlapObserved,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AtlasRule::Rule(r) => r.as_str(),
            AtlasRule::NoOverlapObserved => "no-overlap-observed",
        }
    }

    pub fn render_rule(self) -> OverlapRule {
        match self {
            AtlasRule::Rule(r) => r,
            AtlasRule::NoOverlapObserved => OverlapRule::Or,
        }
    }
}

/// A run placed on its surface.
pub(crate) struct Layout<'a> {
    pub width: i32,
    pub height: i32,
    /// Each baked glyph with its bitmap's top-left on the surface.
    pub placed: Vec<(i32, i32, &'a Glyph)>,
    /// Glyph IDs of the run not baked at this size.
    pub missing: Vec<u32>,
}

/// SDL_ttf 2.20.1's `TTF_Size_Internal` box for a shaped run.
///
/// The pen starts at 0 and moves by the size's hinted advance after every
/// glyph the shaper gave a non-zero advance (a monospace font: every hinted
/// advance is the same; a glyph with no shaped advance, such as U+00AD, does
/// not move it). A glyph's bitmap goes at `(pen + ox, oy)`. The box is
/// `minx = min(0, pen + ox)`, `maxx = max(pen + ox + w, final pen)`,
/// `miny = min(0, oy)`, `maxy = max(height, oy + h)`.
pub(crate) fn layout<'a>(size: &'a SizeAtlas, run: &[ShapedGlyph]) -> Layout<'a> {
    let (mut pen, mut minx, mut maxx, mut miny, mut maxy) = (0, 0, 0, 0, size.height);
    let mut placed = Vec::with_capacity(run.len());
    let mut missing = Vec::new();
    for g in run {
        match size.glyphs.get(&g.gid) {
            Some(glyph) => {
                let x = pen + glyph.ox;
                minx = minx.min(x);
                maxx = maxx.max(x + glyph.w);
                miny = miny.min(glyph.oy);
                maxy = maxy.max(glyph.oy + glyph.h);
                placed.push((x, glyph.oy, glyph));
            }
            None => missing.push(g.gid),
        }
        if g.x_advance != 0 {
            pen += size.advance;
        }
    }
    maxx = maxx.max(pen);
    for p in &mut placed {
        p.0 -= minx;
        p.1 -= miny;
    }
    Layout {
        width: maxx - minx,
        height: maxy - miny,
        placed,
        missing,
    }
}

/// The run's coverage plane (`width * height` bytes), glyphs drawn in order.
pub(crate) fn coverage(layout: &Layout<'_>, rule: OverlapRule) -> Vec<u8> {
    let w = layout.width.max(0) as usize;
    let mut out = vec![0u8; w * layout.height.max(0) as usize];
    for &(x, y, glyph) in &layout.placed {
        let gw = glyph.w as usize;
        for row in 0..glyph.h as usize {
            let src = &glyph.alpha[row * gw..(row + 1) * gw];
            let start = (y as usize + row) * w + x as usize;
            for (d, &s) in out[start..start + gw].iter_mut().zip(src) {
                *d = rule.combine(*d, s);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_differ_only_on_overlap() {
        for rule in OverlapRule::ALL {
            assert_eq!(rule.combine(0, 77), 77);
            assert_eq!(rule.combine(77, 0), 77);
        }
        assert_eq!(OverlapRule::Max.combine(0x50, 0x30), 0x50);
        assert_eq!(OverlapRule::Or.combine(0x50, 0x30), 0x70);
        assert_eq!(OverlapRule::Over.combine(255, 255), 255);
        assert_eq!(OverlapRule::Over.combine(128, 128), 192);
    }

    #[test]
    fn atlas_rules_round_trip() {
        for s in ["max", "over", "or", "no-overlap-observed"] {
            assert_eq!(AtlasRule::parse(s).unwrap().as_str(), s);
        }
        assert_eq!(AtlasRule::parse("sum"), None);
        assert_eq!(AtlasRule::NoOverlapObserved.render_rule(), OverlapRule::Or);
    }
}
