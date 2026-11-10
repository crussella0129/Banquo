//! pygame's per-channel alpha blend.

/// One channel of a source with alpha `a` blended onto a destination channel,
/// exactly as pygame 2.6.1 blits an SRCALPHA surface:
/// `((((s - d) * a) + s) >> 8) + d` in `i32` with an arithmetic shift, so
/// `blend(0, 255, 128) == 127`, alpha 0 keeps `d` and alpha 255 gives `s`.
#[inline]
pub fn blend(s: u8, d: u8, a: u8) -> u8 {
    let (s, d, a) = (i32::from(s), i32::from(d), i32::from(a));
    // The result is always in 0..=255 for 8-bit inputs.
    (((((s - d) * a) + s) >> 8) + d) as u8
}

/// The destination alpha after blending a source alpha `sa` onto a
/// destination alpha `da` (pygame's SRCALPHA-onto-SRCALPHA rule):
/// `sa + da - sa * da / 255`.
#[inline]
pub fn blend_alpha(sa: u8, da: u8) -> u8 {
    let (sa, da) = (i32::from(sa), i32::from(da));
    (sa + da - (sa * da) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_edges() {
        assert_eq!(blend(0, 255, 128), 127);
        assert_eq!(blend(255, 0, 128), 128);
        for s in 0..=255u8 {
            for d in 0..=255u8 {
                assert_eq!(blend(s, d, 0), d);
                assert_eq!(blend(s, d, 255), s);
            }
        }
    }

    #[test]
    fn blend_alpha_edges() {
        assert_eq!(blend_alpha(0, 0), 0);
        assert_eq!(blend_alpha(255, 0), 255);
        assert_eq!(blend_alpha(0, 200), 200);
        assert_eq!(blend_alpha(128, 128), 192);
    }
}
