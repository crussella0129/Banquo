//! 1 px and thick lines (pygame `draw.line` and its thick-line helper).

use super::{hline, vline};
use crate::{Rgba8, Surface};

/// pygame-style out-codes against the bounds `0..=w` x `0..=h` (the far
/// edges are inclusive, as pygame's line clip treats them).
fn outcode(x: i32, y: i32, w: i32, h: i32) -> u8 {
    let mut code = 0;
    if x < 0 {
        code |= 1;
    } else if x > w {
        code |= 2;
    }
    if y < 0 {
        code |= 8;
    } else if y > h {
        code |= 4;
    }
    code
}

/// The line clipped to the surface, or `None` when it lies wholly outside.
///
/// Axis-aligned lines keep their ends (their runs are clipped per pixel).
/// A diagonal end outside the bounds moves to the crossing with the left or
/// right edge, then the top or bottom edge, each computed from the first
/// end in `f64` and rounded half away from zero. This matches pygame 2.6.1
/// on the clip goldens; it is an empirical rule, not a transcription of
/// pygame's code, and a few lines with both ends far off the surface can
/// still differ by a pixel row.
fn clip_line(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> Option<[i32; 4]> {
    if outcode(x1, y1, w, h) & outcode(x2, y2, w, h) != 0 {
        return None;
    }
    if x1 == x2 || y1 == y2 {
        return Some([x1, y1, x2, y2]);
    }
    let m = f64::from(y2 - y1) / f64::from(x2 - x1);
    let along_x = |edge: i32| y1 + (f64::from(edge - x1) * m).round() as i32;
    let along_y = |edge: i32| x1 + (f64::from(edge - y1) / m).round() as i32;
    let fix = |mut x: i32, mut y: i32| {
        if x < 0 {
            y = along_x(0);
            x = 0;
        } else if x > w {
            y = along_x(w);
            x = w;
        }
        if y < 0 {
            x = along_y(0);
            y = 0;
        } else if y > h {
            x = along_y(h);
            y = h;
        }
        (x, y)
    };
    let (ax, ay) = fix(x1, y1);
    let (bx, by) = fix(x2, y2);
    Some([ax, ay, bx, by])
}

impl Surface {
    /// A 1 px line from (`x1`, `y1`) to (`x2`, `y2`), both ends included,
    /// like pygame's `draw.line(surface, color, start, end, 1)`: straight runs
    /// for horizontal and vertical lines, Bresenham otherwise. A diagonal
    /// with an end off the surface is clipped first (see `clip_line`).
    /// Overwrites, clipped to the surface.
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, c: Rgba8) {
        let (w, h) = (self.width() as i32, self.height() as i32);
        let Some([x1, y1, x2, y2]) = clip_line(x1, y1, x2, y2, w, h) else {
            return;
        };
        if y1 == y2 {
            hline(self, x1, x2, y1, c);
            return;
        }
        if x1 == x2 {
            vline(self, x1, y1, y2, c);
            return;
        }
        let (mut x, mut y) = (x1, y1);
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2;
        while x != x2 || y != y2 {
            self.set(x, y, c);
            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
        self.set(x2, y2, c);
    }

    /// A line `width` px thick, grown across its major axis (pygame's
    /// thick-line rule: `width / 2` on one side, `width / 2 - 1` on the other
    /// when `width` is even). Used for the straight edges of rounded rects.
    pub(crate) fn draw_line_width(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        width: i32,
        c: Rgba8,
    ) {
        if width < 1 {
            return;
        }
        if width == 1 {
            self.draw_line(x1, y1, x2, y2, c);
            return;
        }
        // pygame drops a thick line whose centre line lies wholly outside,
        // even when its width would reach the surface.
        let (w, h) = (self.width() as i32, self.height() as i32);
        let Some([x1, y1, x2, y2]) = clip_line(x1, y1, x2, y2, w, h) else {
            return;
        };
        let extra = 1 - (width % 2);
        let half = width / 2;
        let grow_x = (x1 - x2).abs() <= (y1 - y2).abs();
        let (mut x, mut y) = (x1, y1);
        if x1 == x2 && y1 == y2 {
            hline(self, x - half + extra, x + half, y, c);
            return;
        }
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x2 > x1 { 1 } else { -1 };
        let sy = if y2 > y1 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2;
        if grow_x {
            while y != y2 + sy {
                hline(self, x - half + extra, x + half, y, c);
                let e2 = err;
                if e2 > -dx {
                    err -= dy;
                    x += sx;
                }
                if e2 < dy {
                    err += dx;
                    y += sy;
                }
            }
        } else {
            while x != x2 + sx {
                vline(self, x, y - half + extra, y + half, c);
                let e2 = err;
                if e2 > -dx {
                    err -= dy;
                    x += sx;
                }
                if e2 < dy {
                    err += dx;
                    y += sy;
                }
            }
        }
    }
}
