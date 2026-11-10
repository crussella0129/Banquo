//! Integer rectangles with pygame `Rect` semantics.

use serde::{Deserialize, Serialize};

/// An integer rectangle with pygame `Rect` semantics: the left and top edges
/// are inside, the right (`x + w`) and bottom (`y + h`) edges are outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The start of the overlap of `[a0, a1)` and `[b0, b1)`, as pygame's clip
/// computes it, or `None` when neither start lies in the other span.
fn overlap_start(a0: i64, a1: i64, b0: i64, b1: i64) -> Option<i64> {
    if a0 >= b0 && a0 < b1 {
        Some(a0)
    } else if b0 >= a0 && b0 < a1 {
        Some(b0)
    } else {
        None
    }
}

/// The end of the overlap of `[a0, a1)` and `[b0, b1)`, as pygame's clip
/// computes it, or `None` when neither end lies in the other span.
fn overlap_end(a0: i64, a1: i64, b0: i64, b1: i64) -> Option<i64> {
    if a1 > b0 && a1 <= b1 {
        Some(a1)
    } else if b1 > a0 && b1 <= a1 {
        Some(b1)
    } else {
        None
    }
}

/// The ordered (low, high) edges of one axis, so negative sizes still work.
fn span(start: i64, end: i64) -> (i64, i64) {
    (start.min(end), start.max(end))
}

impl IRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    fn left(&self) -> i64 {
        i64::from(self.x)
    }

    fn top(&self) -> i64 {
        i64::from(self.y)
    }

    fn right(&self) -> i64 {
        self.left() + i64::from(self.w)
    }

    fn bottom(&self) -> i64 {
        self.top() + i64::from(self.h)
    }

    /// The part of `self` inside `other`, like pygame's `Rect.clip`.
    ///
    /// When the rectangles do not overlap the result has `self`'s position and
    /// zero size.
    pub fn clip(&self, other: IRect) -> IRect {
        let (a, b) = (self, &other);
        let edges = (|| {
            let x0 = overlap_start(a.left(), a.right(), b.left(), b.right())?;
            let x1 = overlap_end(a.left(), a.right(), b.left(), b.right())?;
            let y0 = overlap_start(a.top(), a.bottom(), b.top(), b.bottom())?;
            let y1 = overlap_end(a.top(), a.bottom(), b.top(), b.bottom())?;
            Some((x0, y0, x1, y1))
        })();
        match edges {
            // Every edge comes from one of the two rects, so each value and
            // each size fits in i32.
            Some((x0, y0, x1, y1)) => {
                IRect::new(x0 as i32, y0 as i32, (x1 - x0) as i32, (y1 - y0) as i32)
            }
            None => IRect::new(a.x, a.y, 0, 0),
        }
    }

    /// Whether the rectangles overlap, like pygame's `Rect.colliderect`.
    /// Rectangles that only share an edge do not collide, and a rectangle with
    /// zero width or height collides with nothing.
    pub fn colliderect(&self, other: IRect) -> bool {
        if self.w == 0 || self.h == 0 || other.w == 0 || other.h == 0 {
            return false;
        }
        let (ax0, ax1) = span(self.left(), self.right());
        let (ay0, ay1) = span(self.top(), self.bottom());
        let (bx0, bx1) = span(other.left(), other.right());
        let (by0, by1) = span(other.top(), other.bottom());
        ax0 < bx1 && ay0 < by1 && ax1 > bx0 && ay1 > by0
    }

    /// A rectangle grown by `dx` and `dy` around the same centre, like pygame's
    /// `Rect.inflate` (the offset is `dx / 2` truncated towards zero).
    pub fn inflate(&self, dx: i32, dy: i32) -> IRect {
        IRect::new(
            self.x.wrapping_sub(dx / 2),
            self.y.wrapping_sub(dy / 2),
            self.w.wrapping_add(dx),
            self.h.wrapping_add(dy),
        )
    }

    /// Whether the point is inside, like pygame's `Rect.collidepoint`.
    pub fn collide_point(&self, x: i32, y: i32) -> bool {
        let (x, y) = (i64::from(x), i64::from(y));
        x >= self.left() && x < self.right() && y >= self.top() && y < self.bottom()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_matches_pygame() {
        let screen = IRect::new(0, 0, 20, 20);
        assert_eq!(
            IRect::new(-5, -3, 10, 10).clip(screen),
            IRect::new(0, 0, 5, 7)
        );
        assert_eq!(
            IRect::new(30, 30, 5, 5).clip(screen),
            IRect::new(30, 30, 0, 0)
        );
        assert_eq!(IRect::new(2, 3, 4, 5).clip(screen), IRect::new(2, 3, 4, 5));
        assert_eq!(
            screen.clip(IRect::new(15, -2, 10, 10)),
            IRect::new(15, 0, 5, 8)
        );
    }

    #[test]
    fn colliderect_excludes_shared_edges() {
        let a = IRect::new(0, 0, 10, 10);
        assert!(!a.colliderect(IRect::new(10, 0, 5, 5)));
        assert!(!a.colliderect(IRect::new(0, 10, 5, 5)));
        assert!(a.colliderect(IRect::new(9, 9, 5, 5)));
        assert!(!a.colliderect(IRect::new(2, 2, 0, 5)));
    }

    #[test]
    fn inflate_and_collide_point() {
        assert_eq!(
            IRect::new(10, 10, 4, 4).inflate(4, 2),
            IRect::new(8, 9, 8, 6)
        );
        assert_eq!(
            IRect::new(10, 10, 4, 4).inflate(-3, 3),
            IRect::new(11, 9, 1, 7)
        );
        let r = IRect::new(0, 0, 10, 10);
        assert!(r.collide_point(0, 0));
        assert!(r.collide_point(9, 9));
        assert!(!r.collide_point(10, 5));
        assert!(!r.collide_point(-1, 5));
    }
}
