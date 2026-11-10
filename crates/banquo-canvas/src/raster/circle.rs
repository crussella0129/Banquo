//! Circles and circle quadrants (pygame `draw.circle` and the rounded-rect
//! corners), midpoint-circle based like pygame 2.6.1.

use super::hline;
use crate::{Rgba8, Surface};

/// Which quadrants of a corner circle to draw.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Quadrants {
    pub top_right: bool,
    pub top_left: bool,
    pub bottom_left: bool,
    pub bottom_right: bool,
}

/// The midpoint-circle stepper shared by every circle routine.
struct Stepper {
    f: i32,
    ddf_x: i32,
    ddf_y: i32,
    x: i32,
    y: i32,
}

impl Stepper {
    fn new(radius: i32) -> Self {
        Self {
            f: 1 - radius,
            ddf_x: 0,
            ddf_y: -2 * radius,
            x: 0,
            y: radius,
        }
    }

    /// One step; `y` drops when `f` is non-negative before the step.
    fn step(&mut self) {
        if self.f >= 0 {
            self.y -= 1;
            self.ddf_y += 2;
            self.f += self.ddf_y;
        }
        self.x += 1;
        self.ddf_x += 2;
        self.f += self.ddf_x + 1;
    }
}

/// The rows of the filled circle of `radius` as half-widths: entry `dy` is
/// the half-width `a` of rows `y0 - 1 - dy` and `y0 + dy`, which span
/// `x0 - a ..= x0 + a - 1` (the widest run the filled routine draws there).
fn filled_half_widths(radius: i32) -> Vec<i32> {
    let mut half = vec![0; radius.max(0) as usize];
    let mut widen = |b: i32, a: i32| {
        // Rows y0 + b - 1 and y0 - b, i.e. dy = b - 1 (b = 0 names the same
        // two rows as b = 1).
        if let Some(h) = usize::try_from((b - 1).max(0))
            .ok()
            .and_then(|i| half.get_mut(i))
        {
            *h = (*h).max(a);
        }
    };
    let mut st = Stepper::new(radius);
    while st.x < st.y {
        st.step();
        if st.f >= 0 {
            widen(st.y, st.x);
        }
        widen(st.x, st.y);
    }
    half
}

impl Surface {
    /// A circle centred on (`x0`, `y0`) like pygame's
    /// `draw.circle(surface, color, center, radius, width)`: `width == 0` (or
    /// `width >= radius`) fills it, otherwise `width` is the ring thickness.
    /// A radius below 1 or a negative width draws nothing. Overwrites,
    /// clipped to the surface.
    ///
    /// Filled circles, 1 px circles and `width >= radius` match pygame
    /// pixel for pixel. A ring with `1 < width < radius` is drawn as the
    /// filled circle minus the filled `radius - width` circle, which differs
    /// from pygame's thick-ring rule by a few pixels near the diagonals.
    pub fn draw_circle(&mut self, x0: i32, y0: i32, radius: i32, width: i32, c: Rgba8) {
        if radius < 1 || width < 0 {
            return;
        }
        let width = width.min(radius);
        if width == 0 || width == radius {
            self.circle_filled(x0, y0, radius, c);
        } else if width == 1 {
            self.circle_thin(x0, y0, radius, c);
        } else {
            self.circle_thick(x0, y0, radius, width, c);
        }
    }

    fn circle_filled(&mut self, x0: i32, y0: i32, radius: i32, c: Rgba8) {
        let mut st = Stepper::new(radius);
        while st.x < st.y {
            st.step();
            let (x, y) = (st.x, st.y);
            // Draw the cap rows only when y is about to drop, to avoid
            // overdraw.
            if st.f >= 0 {
                hline(self, x0 - x, x0 + x - 1, y0 + y - 1, c);
                hline(self, x0 - x, x0 + x - 1, y0 - y, c);
            }
            hline(self, x0 - y, x0 + y - 1, y0 + x - 1, c);
            hline(self, x0 - y, x0 + y - 1, y0 - x, c);
        }
    }

    fn circle_thin(&mut self, x0: i32, y0: i32, radius: i32, c: Rgba8) {
        let mut st = Stepper::new(radius);
        while st.x < st.y {
            st.step();
            let (x, y) = (st.x, st.y);
            self.set(x0 + x - 1, y0 + y - 1, c);
            self.set(x0 - x, y0 + y - 1, c);
            self.set(x0 + x - 1, y0 - y, c);
            self.set(x0 - x, y0 - y, c);
            self.set(x0 + y - 1, y0 + x - 1, c);
            self.set(x0 + y - 1, y0 - x, c);
            self.set(x0 - y, y0 + x - 1, c);
            self.set(x0 - y, y0 - x, c);
        }
    }

    /// A ring: the pixels of the filled `radius` circle that are not in the
    /// filled `radius - thickness` circle, row by row.
    fn circle_thick(&mut self, x0: i32, y0: i32, radius: i32, thickness: i32, c: Rgba8) {
        let outer = filled_half_widths(radius);
        let inner = filled_half_widths(radius - thickness);
        for (dy, &half) in outer.iter().enumerate() {
            let ihalf = inner.get(dy).copied().unwrap_or(0);
            let dy = dy as i32;
            for y in [y0 - 1 - dy, y0 + dy] {
                if ihalf == 0 {
                    hline(self, x0 - half, x0 + half - 1, y, c);
                } else {
                    hline(self, x0 - half, x0 - ihalf - 1, y, c);
                    hline(self, x0 + ihalf, x0 + half - 1, y, c);
                }
            }
        }
    }

    /// The selected quadrants of a circle, as the rounded-rect corners draw
    /// them; `thickness == 0` fills.
    pub(crate) fn circle_quadrant(
        &mut self,
        x0: i32,
        y0: i32,
        radius: i32,
        thickness: i32,
        q: Quadrants,
        c: Rgba8,
    ) {
        if radius == 1 {
            if q.top_right {
                self.set(x0, y0 - 1, c);
            }
            if q.top_left {
                self.set(x0 - 1, y0 - 1, c);
            }
            if q.bottom_left {
                self.set(x0 - 1, y0, c);
            }
            if q.bottom_right {
                self.set(x0, y0, c);
            }
            return;
        }
        let mut outer = Stepper::new(radius);
        if thickness != 0 {
            let mut inner = Stepper::new(radius - thickness);
            let mut thickness = thickness;
            while outer.x < outer.y {
                outer.step();
                inner.step();
                let (x, y) = (outer.x, outer.y);
                if thickness > 1 {
                    thickness = y - inner.y;
                }
                for i in 0..thickness {
                    let y1 = y - i;
                    if q.top_right {
                        if (y0 - y1) < (y0 - x) {
                            self.set(x0 + x - 1, y0 - y1, c);
                        }
                        self.set(x0 + y1 - 1, y0 - x, c);
                    }
                    if q.top_left {
                        if (y0 - y1) < (y0 - x) {
                            self.set(x0 - x, y0 - y1, c);
                        }
                        self.set(x0 - y1, y0 - x, c);
                    }
                    if q.bottom_left {
                        if (y0 + y1 - 1) > (y0 + x - 1) {
                            self.set(x0 - x, y0 + y1 - 1, c);
                        }
                        self.set(x0 - y1, y0 + x - 1, c);
                    }
                    if q.bottom_right {
                        if (y0 + y1 - 1) > (y0 + x - 1) {
                            self.set(x0 + x - 1, y0 + y1 - 1, c);
                        }
                        self.set(x0 + y1 - 1, y0 + x - 1, c);
                    }
                }
            }
        } else {
            while outer.x < outer.y {
                outer.step();
                let (x, y) = (outer.x, outer.y);
                if q.top_right {
                    for y1 in y0 - x..=y0 {
                        self.set(x0 + y - 1, y1, c);
                    }
                    for y1 in y0 - y..=y0 {
                        self.set(x0 + x - 1, y1, c);
                    }
                }
                if q.top_left {
                    for y1 in y0 - x..=y0 {
                        self.set(x0 - y, y1, c);
                    }
                    for y1 in y0 - y..=y0 {
                        self.set(x0 - x, y1, c);
                    }
                }
                if q.bottom_left {
                    for y1 in y0..y0 + x {
                        self.set(x0 - y, y1, c);
                    }
                    for y1 in y0..y0 + y {
                        self.set(x0 - x, y1, c);
                    }
                }
                if q.bottom_right {
                    for y1 in y0..y0 + x {
                        self.set(x0 + y - 1, y1, c);
                    }
                    for y1 in y0..y0 + y {
                        self.set(x0 + x - 1, y1, c);
                    }
                }
            }
        }
    }
}
