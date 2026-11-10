//! Classifying a render that differs from the pygame reference, and checking
//! it against the known differences.
//!
//! The split is the one the glyph-bake verification uses: a difference is
//! `shaping` when pygame's `Font.size` width differs from the Rust width, or
//! when the default-feature and features-off glyph-ID sequences differ (the
//! font's GSUB/GPOS changed the run, so the HarfBuzz versions could disagree);
//! otherwise it is `raster` (same sequence and width, different pixels).
//!
//! A known difference (`approved_differences.json`) holds exactly `size`,
//! `index`, `class` and `sha256`, the SHA-256 of the exact Rust output (see
//! [`output_digest_input`]). It is matched by size and index; a pinned entry
//! whose hash no longer equals the Rust output fails, so any change to a
//! known difference is caught.

use serde::{Deserialize, Serialize};

use super::{shape, FontFace};
use crate::Surface;

/// The class of a differing entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffClass {
    /// Different glyph IDs or advances.
    Shaping,
    /// Same glyph sequence and width, different pixels.
    Raster,
}

impl DiffClass {
    pub fn as_str(self) -> &'static str {
        match self {
            DiffClass::Shaping => "shaping",
            DiffClass::Raster => "raster",
        }
    }
}

/// The class of a difference for `text` at `size_px`, given pygame's
/// `Font.size(text)` width and the width of the Rust render.
pub fn classify(face: &FontFace, text: &str, pygame_width: i32, rust_width: i32) -> DiffClass {
    let ids = |run: Vec<shape::ShapedGlyph>| run.into_iter().map(|g| g.gid).collect::<Vec<_>>();
    let default = ids(shape::run(face, text));
    let off = ids(shape::run_features_off(face, text));
    if pygame_width != rust_width || default != off {
        DiffClass::Shaping
    } else {
        DiffClass::Raster
    }
}

/// Pixels that differ between two surfaces laid top-left on top of each
/// other, the smaller padded with transparent black `(0, 0, 0, 0)`.
pub fn differing_pixels(a: &Surface, b: &Surface) -> usize {
    let w = a.width().max(b.width()) as i32;
    let h = a.height().max(b.height()) as i32;
    let blank = crate::Rgba8::new(0, 0, 0, 0);
    let mut n = 0;
    for y in 0..h {
        for x in 0..w {
            if a.get(x, y).unwrap_or(blank) != b.get(x, y).unwrap_or(blank) {
                n += 1;
            }
        }
    }
    n
}

/// The bytes whose SHA-256 identifies a Rust render: width and height as
/// big-endian `u32`, then the RGBA bytes. A 0-wide surface still records its
/// height.
pub fn output_digest_input(s: &Surface) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + s.as_bytes().len());
    out.extend_from_slice(&s.width().to_be_bytes());
    out.extend_from_slice(&s.height().to_be_bytes());
    out.extend_from_slice(s.as_bytes());
    out
}

/// One entry of `approved_differences.json` (the known differences).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownDifference {
    pub size: u32,
    pub index: usize,
    pub class: DiffClass,
    pub sha256: String,
}

/// The outcome of one corpus entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Equal to pygame, byte for byte.
    Exact,
    /// Differs, and matches its known-difference entry.
    Pinned,
    /// Differs, with no known-difference entry.
    Unpinned(DiffClass),
    /// A known-difference entry whose hash or class is not the Rust output's.
    PinMismatch {
        expected: KnownDifference,
        got_sha256: String,
        got_class: Option<DiffClass>,
    },
}

impl Verdict {
    /// Whether the entry passes (exact or pinned).
    pub fn passes(&self) -> bool {
        matches!(self, Verdict::Exact | Verdict::Pinned)
    }
}

/// The verdict for an entry: `exact` when the render equals pygame's,
/// `class` computes the class of a difference, `pin` is the entry's known
/// difference (if any) and `sha256` the hex SHA-256 of the Rust output.
pub fn verdict(
    exact: bool,
    class: impl FnOnce() -> DiffClass,
    pin: Option<&KnownDifference>,
    sha256: &str,
) -> Verdict {
    match (exact, pin) {
        (true, None) => Verdict::Exact,
        (true, Some(p)) => Verdict::PinMismatch {
            expected: p.clone(),
            got_sha256: sha256.to_owned(),
            got_class: None,
        },
        (false, None) => Verdict::Unpinned(class()),
        (false, Some(p)) => {
            let got = class();
            if p.sha256.eq_ignore_ascii_case(sha256) && p.class == got {
                Verdict::Pinned
            } else {
                Verdict::PinMismatch {
                    expected: p.clone(),
                    got_sha256: sha256.to_owned(),
                    got_class: Some(got),
                }
            }
        }
    }
}
