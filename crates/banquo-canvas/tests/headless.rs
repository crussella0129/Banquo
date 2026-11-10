//! CPU reference presentation: identity at 1x, whole-number blocks at 2x,
//! background outside the viewport.

use banquo_canvas::headless::{render_headless, viewport, HeadlessParams, ScaleMode};
use banquo_canvas::{IRect, Rgba8, Surface};

/// Deterministic xorshift64* so the test needs no RNG crate.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn random_opaque(w: u32, h: u32, seed: u64) -> Surface {
    let mut rng = Rng(seed);
    let mut s = Surface::new(w, h, false);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let v = rng.next();
            s.set(x, y, Rgba8::rgb(v as u8, (v >> 8) as u8, (v >> 16) as u8));
        }
    }
    s
}

const BG: Rgba8 = Rgba8::rgb(12, 34, 56);

fn params(scale: ScaleMode) -> HeadlessParams {
    HeadlessParams {
        scale,
        background: BG,
    }
}

#[test]
fn identity_at_1x() {
    let frame = random_opaque(900, 700, 0x5EED_0001);
    for mode in [ScaleMode::Integer, ScaleMode::Fit] {
        let out = render_headless(&frame, (900, 700), &params(mode));
        assert_eq!(out.as_bytes(), frame.as_bytes(), "{mode:?}");
    }
}

#[test]
fn every_pixel_fills_a_2x2_block_at_2x() {
    let frame = random_opaque(900, 700, 0x5EED_0002);
    let out = render_headless(&frame, (1800, 1400), &params(ScaleMode::Integer));
    assert_eq!((out.width(), out.height()), (1800, 1400));
    let mut wrong = 0usize;
    for y in 0..700 {
        for x in 0..900 {
            let c = frame.get(x, y);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                if out.get(2 * x + dx, 2 * y + dy) != c {
                    wrong += 1;
                }
            }
        }
    }
    assert_eq!(wrong, 0);
}

#[test]
fn outside_the_viewport_is_background() {
    let frame = random_opaque(900, 700, 0x5EED_0003);
    for (mode, target) in [
        (ScaleMode::Integer, (1920, 1080)),
        (ScaleMode::Integer, (1366, 768)),
        (ScaleMode::Fit, (1366, 768)),
        (ScaleMode::Fit, (1280, 1000)),
    ] {
        let out = render_headless(&frame, target, &params(mode));
        let vp = viewport((900, 700), target, mode);
        let mut outside = 0usize;
        let mut wrong = 0usize;
        for y in 0..target.1 as i32 {
            for x in 0..target.0 as i32 {
                if !vp.collide_point(x, y) {
                    outside += 1;
                    if out.get(x, y) != Some(BG) {
                        wrong += 1;
                    }
                }
            }
        }
        assert!(outside > 0, "{mode:?} {target:?} has bars");
        assert_eq!(wrong, 0, "{mode:?} {target:?}");
    }
}

#[test]
fn viewport_rules() {
    assert_eq!(
        viewport((900, 700), (1920, 1080), ScaleMode::Integer),
        IRect::new(510, 190, 900, 700)
    );
    assert_eq!(
        viewport((900, 700), (1800, 1400), ScaleMode::Integer),
        IRect::new(0, 0, 1800, 1400)
    );
    assert_eq!(
        viewport((900, 700), (1366, 768), ScaleMode::Fit),
        IRect::new(189, 0, 987, 768)
    );
    // A smaller target crops at scale 1.
    let frame = random_opaque(30, 20, 0x5EED_0004);
    let out = render_headless(&frame, (10, 10), &params(ScaleMode::Integer));
    assert_eq!(
        viewport((30, 20), (10, 10), ScaleMode::Integer),
        IRect::new(-10, -5, 30, 20)
    );
    assert_eq!(out.get(0, 0), frame.get(10, 5));
    assert_eq!(out.get(9, 9), frame.get(19, 14));
}
