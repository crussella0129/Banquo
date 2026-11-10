//! Shape rasterising onto a [`crate::Surface`] (lines, rects, circles, blits),
//! pixel-exact against pygame 2.6.1.
//!
//! The `draw_*` methods and [`Surface::fill_rect`] have pygame's overwrite
//! semantics: they store the colour as given (an opaque surface forces
//! `a = 255`) and never blend. [`Surface::blit`] and [`Surface::blit_mask`]
//! blend with [`blend`]. Every primitive clips to the surface bounds, so any
//! offset, including one entirely off the surface, is safe.
//!
//! [`Surface::fill_rect`]: crate::Surface::fill_rect
//! [`Surface::blit`]: crate::Surface::blit
//! [`Surface::blit_mask`]: crate::Surface::blit_mask

mod blend;
mod blit;
mod circle;
mod line;
mod rect;

pub use blend::{blend, blend_alpha};

use crate::{Rgba8, Surface};

/// A horizontal run from `x1` to `x2` inclusive (either order), clipped.
fn hline(s: &mut Surface, x1: i32, x2: i32, y: i32, c: Rgba8) {
    if y < 0 || y >= s.height() as i32 {
        return;
    }
    let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let lo = lo.max(0);
    let hi = hi.min(s.width() as i32 - 1);
    for x in lo..=hi {
        s.set(x, y, c);
    }
}

/// A vertical run from `y1` to `y2` inclusive (either order), clipped.
fn vline(s: &mut Surface, x: i32, y1: i32, y2: i32, c: Rgba8) {
    if x < 0 || x >= s.width() as i32 {
        return;
    }
    let (lo, hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let lo = lo.max(0);
    let hi = hi.min(s.height() as i32 - 1);
    for y in lo..=hi {
        s.set(x, y, c);
    }
}
