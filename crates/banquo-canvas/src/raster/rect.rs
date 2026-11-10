//! Filled, outlined and rounded rectangles (pygame `Surface.fill(color, rect)`
//! and `draw.rect(surface, color, rect, width, border_radius)`).

use super::circle::Quadrants;
use super::hline;
use crate::{IRect, Rgba8, Surface};

impl Surface {
    /// Overwrites every pixel of `r` that lies on the surface with `c`, like
    /// pygame's `Surface.fill(color, rect)` and `draw.rect(..., width=0)`.
    /// A rect with no positive width or height fills nothing.
    pub fn fill_rect(&mut self, r: IRect, c: Rgba8) {
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        let r = self.clip_rect(r);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                self.set(x, y, c);
            }
        }
    }

    /// pygame's `draw.rect(surface, color, rect, width, border_radius)`:
    /// `width == 0` fills, `width > 0` draws an outline of that thickness
    /// inside `r`, and `border_radius > 0` rounds every corner (radii too
    /// large for the rect shrink in proportion, as pygame does in `f32`).
    /// A negative width or a rect with no positive size draws nothing.
    /// Overwrites, clipped to the surface.
    pub fn draw_rect(&mut self, r: IRect, c: Rgba8, width: i32, border_radius: i32) {
        if width < 0 || r.w <= 0 || r.h <= 0 {
            return;
        }
        let mut width = width;
        if width > r.w / 2 || width > r.h / 2 {
            width = (r.w / 2).max(r.h / 2);
        }
        if border_radius <= 0 {
            if width == 0 {
                self.fill_rect(r, c);
            } else {
                self.fill_rect(IRect::new(r.x, r.y, r.w, width), c);
                self.fill_rect(IRect::new(r.x, r.y + r.h - width, r.w, width), c);
                self.fill_rect(IRect::new(r.x, r.y, width, r.h), c);
                self.fill_rect(IRect::new(r.x + r.w - width, r.y, width, r.h), c);
            }
            return;
        }
        self.round_rect(
            r.x,
            r.y,
            r.x + r.w - 1,
            r.y + r.h - 1,
            border_radius,
            width,
            c,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn round_rect(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        radius: i32,
        width: i32,
        c: Rgba8,
    ) {
        let (mut tl, mut tr, mut bl, mut br) = (radius, radius, radius, radius);
        let rw = x2 - x1 + 1;
        let rh = y2 - y1 + 1;
        if tl + tr > rw || bl + br > rw || tl + bl > rh || tr + br > rh {
            // pygame computes the shrink factor in single precision.
            let q_top = rw as f32 / (tl + tr) as f32;
            let q_left = rh as f32 / (tl + bl) as f32;
            let q_bottom = rw as f32 / (bl + br) as f32;
            let q_right = rh as f32 / (tr + br) as f32;
            let f = q_top.min(q_left).min(q_bottom).min(q_right);
            tl = (tl as f32 * f) as i32;
            tr = (tr as f32 * f) as i32;
            bl = (bl as f32 * f) as i32;
            br = (br as f32 * f) as i32;
        }
        let q_tr = Quadrants {
            top_right: true,
            ..Quadrants::default()
        };
        let q_tl = Quadrants {
            top_left: true,
            ..Quadrants::default()
        };
        let q_bl = Quadrants {
            bottom_left: true,
            ..Quadrants::default()
        };
        let q_br = Quadrants {
            bottom_right: true,
            ..Quadrants::default()
        };
        if width == 0 {
            let xs = [x1, x1 + tl, x2 - tr, x2, x2, x2 - br, x1 + bl, x1];
            let ys = [y1 + tl, y1, y1, y1 + tr, y2 - br, y2, y2, y2 - bl];
            self.fill_poly(&xs, &ys, c);
            self.circle_quadrant(x2 - tr + 1, y1 + tr, tr, 0, q_tr, c);
            self.circle_quadrant(x1 + tl, y1 + tl, tl, 0, q_tl, c);
            self.circle_quadrant(x1 + bl, y2 - bl + 1, bl, 0, q_bl, c);
            self.circle_quadrant(x2 - br + 1, y2 - br + 1, br, 0, q_br, c);
            return;
        }
        let near = width / 2 - 1 + width % 2;
        let far = width / 2;
        if x2 - tr == x1 + tl {
            for i in 0..width {
                self.set(x1 + tl, y1 + i, c);
            }
        } else {
            self.draw_line_width(x1 + tl, y1 + near, x2 - tr, y1 + near, width, c);
        }
        if y2 - bl == y1 + tl {
            for i in 0..width {
                self.set(x1 + i, y1 + tl, c);
            }
        } else {
            self.draw_line_width(x1 + near, y1 + tl, x1 + near, y2 - bl, width, c);
        }
        if x2 - br == x1 + bl {
            for i in 0..width {
                self.set(x1 + bl, y2 - i, c);
            }
        } else {
            self.draw_line_width(x1 + bl, y2 - far, x2 - br, y2 - far, width, c);
        }
        if y2 - tr == y1 + br {
            for i in 0..width {
                self.set(x2 - i, y1 + br, c);
            }
        } else {
            self.draw_line_width(x2 - far, y1 + tr, x2 - far, y2 - br, width, c);
        }
        self.circle_quadrant(x2 - tr + 1, y1 + tr, tr, width, q_tr, c);
        self.circle_quadrant(x1 + tl, y1 + tl, tl, width, q_tl, c);
        self.circle_quadrant(x1 + bl, y2 - bl + 1, bl, width, q_bl, c);
        self.circle_quadrant(x2 - br + 1, y2 - br + 1, br, width, q_br, c);
    }

    /// pygame's scanline polygon fill: pairs of edge crossings per row, the
    /// bottom row closed, then horizontal edges strictly between the top and
    /// bottom rows.
    fn fill_poly(&mut self, xs: &[i32], ys: &[i32], c: Rgba8) {
        let n = xs.len();
        let miny = *ys.iter().min().unwrap_or(&0);
        let maxy = *ys.iter().max().unwrap_or(&0);
        if miny == maxy {
            let minx = *xs.iter().min().unwrap_or(&0);
            let maxx = *xs.iter().max().unwrap_or(&0);
            hline(self, minx, maxx, miny, c);
            return;
        }
        let mut cross = Vec::with_capacity(n);
        for y in miny..=maxy {
            cross.clear();
            for i in 0..n {
                let prev = if i == 0 { n - 1 } else { i - 1 };
                let (ya, yb) = (ys[prev], ys[i]);
                let (x1, y1, x2, y2) = if ya < yb {
                    (xs[prev], ya, xs[i], yb)
                } else if ya > yb {
                    (xs[i], yb, xs[prev], ya)
                } else {
                    continue;
                };
                if (y >= y1 && y < y2) || (y == maxy && y2 == maxy) {
                    // C integer division truncates towards zero, as Rust's.
                    cross.push((y - y1) * (x2 - x1) / (y2 - y1) + x1);
                }
            }
            cross.sort_unstable();
            for &[lo, hi] in cross.as_chunks::<2>().0 {
                hline(self, lo, hi, y, c);
            }
        }
        for i in 0..n {
            let prev = if i == 0 { n - 1 } else { i - 1 };
            let y = ys[i];
            if miny < y && ys[prev] == y && y < maxy {
                hline(self, xs[i], xs[prev], y, c);
            }
        }
    }
}
