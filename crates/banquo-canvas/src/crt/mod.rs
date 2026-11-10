//! CPU post-processing passes for the CRT look: scanlines, vignette and
//! flicker, at any surface size.
//!
//! [`CrtMasks::build`] pre-renders the scanline and vignette planes for one
//! size; rebuild them when the surface is resized ([`CrtMasks::resize`]).
//! [`apply`] composites one frame in place: scanlines, then vignette, then
//! flicker. [`CrtParams::legacy`] gives the legacy CRT parameters, which are
//! pixel-exact against pygame 2.6.1 at any width.

use crate::raster::blend;
use crate::{IRect, Rgba8, Surface};

/// How the edges are darkened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vignette {
    /// No vignette.
    None,
    /// `steps` concentric band frames drawn outside-in, each overwriting the
    /// last: frame `i` has `t = i / steps`, alpha `int(t * t * max_alpha)`
    /// and edges `int(center * (1.0 - t))` from the centre, computed in
    /// `f64` and truncated like Python's `int()`. Each band is `band` px
    /// wide, or [`legacy_band_width`] when `None`.
    LegacyBands {
        steps: u32,
        max_alpha: f64,
        band: Option<i32>,
    },
    /// A smooth per-pixel falloff: alpha `int(min(1, r2) * 120.0)`, where
    /// `r2` is the squared elliptical distance from the centre normalised
    /// so the corners are at 1.
    Radial,
}

/// Parameters of the CRT passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrtParams {
    /// A scanline on every row `y` with `y % scanline_every == 0`; 0 turns
    /// scanlines off.
    pub scanline_every: u32,
    /// The scanline darkening alpha.
    pub scanline_alpha: u8,
    pub vignette: Vignette,
    /// The flicker sine's angular rate (radians per second of frame time).
    pub flicker_rate: f64,
    /// The flicker sine's amplitude (0 turns flicker off).
    pub flicker_intensity: f64,
    /// The largest flicker darkening alpha.
    pub flicker_max_alpha: u8,
}

impl CrtParams {
    /// The legacy CRT parameters: scanlines every 3 rows at alpha 35, 20
    /// vignette bands up to alpha 120 with the derived band width, flicker
    /// rate 8.0, intensity 0.02, max alpha 15.
    pub const fn legacy() -> Self {
        Self {
            scanline_every: 3,
            scanline_alpha: 35,
            vignette: Vignette::LegacyBands {
                steps: 20,
                max_alpha: 120.0,
                band: None,
            },
            flicker_rate: 8.0,
            flicker_intensity: 0.02,
            flicker_max_alpha: 15,
        }
    }
}

/// The legacy band width for a `w` x `h` surface:
/// `max(2, int((w + h) / steps / 2))`, computed in `f64`.
pub fn legacy_band_width(w: u32, h: u32, steps: u32) -> i32 {
    let b = (f64::from(w) + f64::from(h)) / f64::from(steps) / 2.0;
    (b as i32).max(2)
}

/// The pre-rendered planes for one surface size. Both are alpha surfaces of
/// black pixels whose alpha is the darkening.
#[derive(Debug, Clone, PartialEq)]
pub struct CrtMasks {
    pub scanlines: Surface,
    pub vignette: Surface,
}

impl CrtMasks {
    /// Renders the scanline and vignette planes for a `w` x `h` surface.
    pub fn build(w: u32, h: u32, p: &CrtParams) -> Self {
        Self {
            scanlines: scanline_plane(w, h, p),
            vignette: vignette_plane(w, h, &p.vignette),
        }
    }

    /// The size the planes were built for.
    pub fn size(&self) -> (u32, u32) {
        (self.scanlines.width(), self.scanlines.height())
    }

    /// Rebuilds the planes for a new size; returns whether anything changed.
    pub fn resize(&mut self, w: u32, h: u32, p: &CrtParams) -> bool {
        if self.size() == (w, h) {
            return false;
        }
        *self = Self::build(w, h, p);
        true
    }
}

fn scanline_plane(w: u32, h: u32, p: &CrtParams) -> Surface {
    let mut plane = Surface::new(w, h, true);
    if p.scanline_every == 0 {
        return plane;
    }
    let c = Rgba8::new(0, 0, 0, p.scanline_alpha);
    for y in (0..h).step_by(p.scanline_every as usize) {
        plane.draw_line(0, y as i32, w as i32, y as i32, c);
    }
    plane
}

fn vignette_plane(w: u32, h: u32, v: &Vignette) -> Surface {
    let mut plane = Surface::new(w, h, true);
    match *v {
        Vignette::None => {}
        Vignette::LegacyBands {
            steps,
            max_alpha,
            band,
        } => {
            let cx = (w / 2) as i32;
            let cy = (h / 2) as i32;
            let b = band.unwrap_or_else(|| legacy_band_width(w, h, steps));
            for i in 0..steps {
                let t = f64::from(i) / f64::from(steps);
                let c = Rgba8::new(0, 0, 0, (t * t * max_alpha) as u8);
                let mx = (f64::from(cx) * (1.0 - t)) as i32;
                let my = (f64::from(cy) * (1.0 - t)) as i32;
                let (left, top) = (cx - mx, cy - my);
                let (rw, rh) = (mx * 2, my * 2);
                plane.fill_rect(IRect::new(left, top, rw, b), c);
                plane.fill_rect(IRect::new(left, top + rh - b, rw, b), c);
                plane.fill_rect(IRect::new(left, top, b, rh), c);
                plane.fill_rect(IRect::new(left + rw - b, top, b, rh), c);
            }
        }
        Vignette::Radial => {
            let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
            for y in 0..h {
                for x in 0..w {
                    let nx = (f64::from(x) + 0.5 - cx) / cx;
                    let ny = (f64::from(y) + 0.5 - cy) / cy;
                    let r2 = ((nx * nx + ny * ny) / 2.0).min(1.0);
                    plane.set(x as i32, y as i32, Rgba8::new(0, 0, 0, (r2 * 120.0) as u8));
                }
            }
        }
    }
    plane
}

/// The flicker clock.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CrtState {
    /// Accumulated frame time in seconds.
    pub frame_time: f64,
}

/// The flicker darkening alpha at `frame_time`:
/// `min(int(|sin(frame_time * rate) * intensity| * 255), max_alpha)` when the
/// sine term is negative, else 0.
pub fn flicker_alpha(p: &CrtParams, frame_time: f64) -> u8 {
    let v = (frame_time * p.flicker_rate).sin() * p.flicker_intensity;
    if v < 0.0 {
        let a = (v.abs() * 255.0) as i64;
        a.min(i64::from(p.flicker_max_alpha)) as u8
    } else {
        0
    }
}

/// One CRT frame on `s`, in place: blits the scanline plane, then the
/// vignette plane, then advances `st.frame_time` by `dt` and darkens the
/// whole surface with black at [`flicker_alpha`]. Returns that alpha.
pub fn apply(s: &mut Surface, m: &CrtMasks, p: &CrtParams, st: &mut CrtState, dt: f64) -> u8 {
    s.blit(&m.scanlines, 0, 0);
    if !matches!(p.vignette, Vignette::None) {
        s.blit(&m.vignette, 0, 0);
    }
    st.frame_time += dt;
    let a = flicker_alpha(p, st.frame_time);
    if a > 0 {
        darken(s, a);
    }
    a
}

/// Blends black at alpha `a` over every pixel.
fn darken(s: &mut Surface, a: u8) {
    let dark = Rgba8::new(0, 0, 0, a);
    for y in 0..s.height() as i32 {
        for x in 0..s.width() as i32 {
            if let Some(d) = s.get(x, y) {
                let out = if s.has_alpha() && d.a == 0 {
                    dark
                } else {
                    Rgba8::new(
                        blend(0, d.r, a),
                        blend(0, d.g, a),
                        blend(0, d.b, a),
                        if s.has_alpha() {
                            crate::raster::blend_alpha(a, d.a)
                        } else {
                            255
                        },
                    )
                };
                s.set(x, y, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_widths() {
        let w: Vec<i32> = [800, 1020, 1144, 1580]
            .iter()
            .map(|&w| legacy_band_width(w, 600, 20))
            .collect();
        assert_eq!(w, [35, 40, 43, 54]);
        assert_eq!(legacy_band_width(10, 10, 20), 2);
    }

    #[test]
    fn resize_rebuilds_only_on_change() {
        let p = CrtParams::legacy();
        let mut m = CrtMasks::build(80, 60, &p);
        assert!(!m.resize(80, 60, &p));
        assert!(m.resize(90, 60, &p));
        assert_eq!(m.size(), (90, 60));
        assert_eq!(m, CrtMasks::build(90, 60, &p));
    }

    #[test]
    fn radial_is_dark_at_corners_clear_at_centre() {
        let p = CrtParams {
            vignette: Vignette::Radial,
            ..CrtParams::legacy()
        };
        let m = CrtMasks::build(40, 30, &p);
        assert_eq!(m.vignette.get(20, 15).unwrap().a, 0);
        assert!(m.vignette.get(0, 0).unwrap().a >= 110);
        assert_eq!(m.vignette.get(0, 0), m.vignette.get(39, 29));
    }

    #[test]
    fn none_vignette_is_clear() {
        let p = CrtParams {
            vignette: Vignette::None,
            ..CrtParams::legacy()
        };
        let m = CrtMasks::build(20, 10, &p);
        assert!(m.vignette.as_bytes().iter().all(|&b| b == 0));
    }
}
