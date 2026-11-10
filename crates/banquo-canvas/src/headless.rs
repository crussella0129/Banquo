//! Headless rendering: draw a frame to a [`crate::Surface`] and encode it without a GPU.
//!
//! [`render_headless`] is the CPU reference for presenting a canvas into a
//! window-sized target: the same viewport arithmetic as the GPU presenter
//! (whole-number `Integer` scaling or SDL-style `Fit` letterboxing, floor
//! centred) and nearest-texel sampling at pixel centres, so a GPU readback of
//! the presenter can be compared to it byte for byte.

use crate::{IRect, Rgba8, Surface};

/// How the canvas is scaled into the target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    /// Whole-number scale `max(1, min(pw / cw, ph / ch))` (integer
    /// division), centred; a smaller target crops. Default.
    #[default]
    Integer,
    /// Aspect-preserving fractional scale: the limiting axis fills the target
    /// exactly, the other is floored; centred.
    Fit,
}

/// Parameters for [`render_headless`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessParams {
    /// Scaling rule.
    pub scale: ScaleMode,
    /// Colour of every target pixel outside the viewport.
    pub background: Rgba8,
}

impl Default for HeadlessParams {
    fn default() -> Self {
        Self {
            scale: ScaleMode::Integer,
            background: Rgba8::rgb(0, 0, 0),
        }
    }
}

/// Floor of `a / 2` for possibly negative `a`.
fn floor_half(a: i64) -> i32 {
    a.div_euclid(2) as i32
}

/// Where a `canvas`-sized frame lands on a `target`-sized surface. A zero
/// size gives an empty rect at the origin.
pub fn viewport(canvas: (u32, u32), target: (u32, u32), scale: ScaleMode) -> IRect {
    let (cw, ch) = (i64::from(canvas.0), i64::from(canvas.1));
    let (pw, ph) = (i64::from(target.0), i64::from(target.1));
    if cw == 0 || ch == 0 || pw == 0 || ph == 0 {
        return IRect::new(0, 0, 0, 0);
    }
    let (w, h) = match scale {
        ScaleMode::Integer => {
            let s = (pw / cw).min(ph / ch).max(1);
            (cw * s, ch * s)
        }
        ScaleMode::Fit => {
            if cw * ph > pw * ch {
                (pw, ch * pw / cw)
            } else {
                (cw * ph / ch, ph)
            }
        }
    };
    let clamp = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
    IRect::new(floor_half(pw - w), floor_half(ph - h), clamp(w), clamp(h))
}

/// Renders `frame` into a new `target`-sized surface: `params.background`
/// outside [`viewport`], and inside it the frame pixel under each target
/// pixel's centre (nearest texel, as a GPU `Nearest` sampler picks it). The
/// result has the frame's alpha mode.
pub fn render_headless(frame: &Surface, target: (u32, u32), params: &HeadlessParams) -> Surface {
    let (cw, ch) = (frame.width(), frame.height());
    let mut out = Surface::new(target.0, target.1, frame.has_alpha());
    out.fill(params.background);
    let vp = viewport((cw, ch), target, params.scale);
    if vp.w <= 0 || vp.h <= 0 {
        return out;
    }
    let (vw, vh) = (i64::from(vp.w), i64::from(vp.h));
    let x0 = vp.x.max(0);
    let y0 = vp.y.max(0);
    let x1 = (i64::from(vp.x) + vw).min(i64::from(target.0)) as i32;
    let y1 = (i64::from(vp.y) + vh).min(i64::from(target.1)) as i32;
    for ty in y0..y1 {
        // Source row under the pixel centre: floor((dy + 0.5) * ch / vh).
        let dy = i64::from(ty - vp.y);
        let sy = ((2 * dy + 1) * i64::from(ch) / (2 * vh)) as i32;
        for tx in x0..x1 {
            let dx = i64::from(tx - vp.x);
            let sx = ((2 * dx + 1) * i64::from(cw) / (2 * vw)) as i32;
            if let Some(c) = frame.get(sx, sy) {
                out.set(tx, ty, c);
            }
        }
    }
    out
}
