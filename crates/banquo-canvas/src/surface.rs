//! RGBA8 pixel surfaces with pygame `Surface` semantics.

use crate::{IRect, Rgba8};

/// A row-major RGBA8 pixel buffer (`width * height * 4` bytes).
///
/// An opaque surface (`alpha = false`) starts black with `a = 255` and stores
/// `a = 255` on every write; an alpha surface starts fully transparent (all
/// zero bytes). Out-of-bounds reads return `None` and out-of-bounds writes do
/// nothing, like a write outside pygame's clip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    width: u32,
    height: u32,
    alpha: bool,
    data: Vec<u8>,
}

impl Surface {
    /// A new `width` x `height` surface.
    pub fn new(width: u32, height: u32, alpha: bool) -> Self {
        let len = width as usize * height as usize * 4;
        let mut surface = Self {
            width,
            height,
            alpha,
            data: vec![0; len],
        };
        if !alpha {
            surface.fill(Rgba8::rgb(0, 0, 0));
        }
        surface
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn has_alpha(&self) -> bool {
        self.alpha
    }

    /// The pixels as RGBA8 bytes, row-major.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// The colour as this surface stores it (opaque surfaces force `a = 255`).
    fn stored(&self, c: Rgba8) -> [u8; 4] {
        let a = if self.alpha { c.a } else { 255 };
        [c.r, c.g, c.b, a]
    }

    /// Sets every pixel to `c`.
    pub fn fill(&mut self, c: Rgba8) {
        let px = self.stored(c);
        // `data.len()` is always a multiple of 4, so there is no remainder.
        for chunk in self.data.as_chunks_mut::<4>().0 {
            *chunk = px;
        }
    }

    fn offset(&self, x: i32, y: i32) -> Option<usize> {
        let x = u32::try_from(x).ok()?;
        let y = u32::try_from(y).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }

    /// The pixel at (`x`, `y`), or `None` outside the surface.
    pub fn get(&self, x: i32, y: i32) -> Option<Rgba8> {
        let i = self.offset(x, y)?;
        let p = &self.data[i..i + 4];
        Some(Rgba8::new(p[0], p[1], p[2], p[3]))
    }

    /// Sets the pixel at (`x`, `y`); does nothing outside the surface.
    pub fn set(&mut self, x: i32, y: i32, c: Rgba8) {
        if let Some(i) = self.offset(x, y) {
            let px = self.stored(c);
            self.data[i..i + 4].copy_from_slice(&px);
        }
    }

    /// The surface's bounds as a rect at the origin.
    pub fn rect(&self) -> IRect {
        IRect::new(
            0,
            0,
            i32::try_from(self.width).unwrap_or(i32::MAX),
            i32::try_from(self.height).unwrap_or(i32::MAX),
        )
    }

    /// The part of `r` that lies on this surface (pygame `Rect.clip` against
    /// the surface bounds).
    pub fn clip_rect(&self, r: IRect) -> IRect {
        r.clip(self.rect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(s: &Surface) -> Vec<Rgba8> {
        let mut out = Vec::new();
        for y in 0..s.height() as i32 {
            for x in 0..s.width() as i32 {
                out.push(s.get(x, y).unwrap());
            }
        }
        out
    }

    #[test]
    fn new_alpha_surface_is_zeroed() {
        let s = Surface::new(4, 3, true);
        assert!(s.has_alpha());
        assert_eq!((s.width(), s.height()), (4, 3));
        assert_eq!(s.as_bytes().len(), 48);
        assert!(s.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn new_opaque_surface_is_opaque_black() {
        let s = Surface::new(4, 3, false);
        assert!(!s.has_alpha());
        assert_eq!(s.as_bytes().len(), 48);
        let px = pixels(&s);
        assert_eq!(px.len(), 12);
        assert!(px.iter().all(|&p| p == Rgba8::new(0, 0, 0, 255)));
    }

    #[test]
    fn fill_respects_alpha_mode() {
        let c = Rgba8::new(10, 20, 30, 40);
        let mut a = Surface::new(4, 3, true);
        a.fill(c);
        assert!(pixels(&a).iter().all(|&p| p == Rgba8::new(10, 20, 30, 40)));
        let mut o = Surface::new(4, 3, false);
        o.fill(c);
        assert!(pixels(&o).iter().all(|&p| p == Rgba8::new(10, 20, 30, 255)));
    }

    #[test]
    fn set_changes_one_pixel_only() {
        let c = Rgba8::new(1, 2, 3, 4);
        for alpha in [true, false] {
            let mut s = Surface::new(4, 3, alpha);
            let before = pixels(&s);
            s.set(3, 2, c);
            let after = pixels(&s);
            let expected = if alpha { c } else { Rgba8::rgb(1, 2, 3) };
            assert_eq!(after[2 * 4 + 3], expected);
            let changed = before.iter().zip(&after).filter(|(b, a)| b != a).count();
            assert_eq!(changed, 1);
        }
    }

    #[test]
    fn out_of_bounds_is_safe() {
        let mut s = Surface::new(4, 3, true);
        assert_eq!(s.get(4, 0), None);
        assert_eq!(s.get(0, 3), None);
        assert_eq!(s.get(-1, 0), None);
        let before = s.as_bytes().to_vec();
        s.set(-1, 0, Rgba8::new(9, 9, 9, 9));
        s.set(4, 0, Rgba8::new(9, 9, 9, 9));
        s.set(0, -1, Rgba8::new(9, 9, 9, 9));
        assert_eq!(s.as_bytes(), &before[..]);
    }

    #[test]
    fn clip_rect_uses_pygame_clip() {
        let s = Surface::new(20, 20, true);
        assert_eq!(
            s.clip_rect(IRect::new(-5, -3, 10, 10)),
            IRect::new(0, 0, 5, 7)
        );
        assert_eq!(
            s.clip_rect(IRect::new(30, 30, 5, 5)),
            IRect::new(30, 30, 0, 0)
        );
    }
}
