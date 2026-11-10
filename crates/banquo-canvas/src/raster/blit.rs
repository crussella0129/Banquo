//! Blitting surfaces and coverage masks.

use super::{blend, blend_alpha};
use crate::{Rgba8, Surface};

impl Surface {
    /// Draws `src` with its top-left corner at (`x`, `y`), clipped, like
    /// pygame's `dest.blit(src, (x, y))`.
    ///
    /// An opaque `src` is copied. An alpha `src` blends each channel with
    /// [`blend`]; onto an opaque destination the alpha stays 255, onto an
    /// alpha destination it becomes [`blend_alpha`], and a fully transparent
    /// destination pixel takes the source pixel as is.
    pub fn blit(&mut self, src: &Surface, x: i32, y: i32) {
        let alpha = src.has_alpha();
        self.blit_with(src, x, y, |p| (p, alpha));
    }

    /// Draws colour `c` through the alpha channel of `mask` (a coverage
    /// plane) with its top-left corner at (`x`, `y`), clipped. Each pixel is
    /// blended as an alpha source of colour `c.rgb` and alpha
    /// `mask.a * c.a / 255` (rounded to nearest), exactly like blitting an
    /// SRCALPHA sprite of that colour and alpha.
    pub fn blit_mask(&mut self, mask: &Surface, x: i32, y: i32, c: Rgba8) {
        self.blit_with(mask, x, y, |m| {
            let a = (u32::from(m.a) * u32::from(c.a) + 127) / 255;
            (Rgba8::new(c.r, c.g, c.b, a as u8), true)
        });
    }

    fn blit_with(&mut self, src: &Surface, x: i32, y: i32, pixel: impl Fn(Rgba8) -> (Rgba8, bool)) {
        let (sw, sh) = (src.width() as i64, src.height() as i64);
        let (dw, dh) = (i64::from(self.width()), i64::from(self.height()));
        let (x, y) = (i64::from(x), i64::from(y));
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + sw).min(dw);
        let y1 = (y + sh).min(dh);
        let dst_alpha = self.has_alpha();
        for dy in y0..y1 {
            for dx in x0..x1 {
                // Inside both surfaces, so every coordinate fits in i32.
                let (sx, sy) = ((dx - x) as i32, (dy - y) as i32);
                let (dx, dy) = (dx as i32, dy as i32);
                let Some(sp) = src.get(sx, sy) else { continue };
                let (s, blended) = pixel(sp);
                if !blended {
                    self.set(dx, dy, Rgba8::new(s.r, s.g, s.b, 255));
                    continue;
                }
                let Some(d) = self.get(dx, dy) else { continue };
                let out = if dst_alpha && d.a == 0 {
                    s
                } else {
                    Rgba8::new(
                        blend(s.r, d.r, s.a),
                        blend(s.g, d.g, s.a),
                        blend(s.b, d.b, s.a),
                        if dst_alpha {
                            blend_alpha(s.a, d.a)
                        } else {
                            255
                        },
                    )
                };
                self.set(dx, dy, out);
            }
        }
    }
}
