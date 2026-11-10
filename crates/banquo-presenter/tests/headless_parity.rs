//! The CPU reference (`banquo_canvas::headless`) and the presenter place the
//! canvas identically. banquo-canvas is only a dev-dependency here; the
//! presenter itself does not depend on it.

use banquo_canvas::headless;
use banquo_presenter::{viewport_for, PresentParams, ScaleMode, Viewport};

/// (canvas, surface, mode). The spec vectors plus the viewports measured with
/// SDL 2.28.4 RenderSetLogicalSize (windowed whole-number, logical-size
/// letterbox, and fullscreen).
type Vector = ((u32, u32), (u32, u32), ScaleMode);

const VECTORS: [Vector; 23] = [
    ((900, 700), (1920, 1080), ScaleMode::Integer),
    ((900, 700), (1800, 1400), ScaleMode::Integer),
    ((1244, 700), (1600, 900), ScaleMode::Integer),
    ((900, 700), (1280, 800), ScaleMode::Integer),
    ((900, 700), (1366, 768), ScaleMode::Integer),
    ((900, 700), (1600, 900), ScaleMode::Integer),
    ((900, 700), (1280, 800), ScaleMode::Fit),
    ((900, 700), (1366, 768), ScaleMode::Fit),
    ((900, 700), (1920, 1080), ScaleMode::Fit),
    ((900, 700), (2560, 1080), ScaleMode::Fit),
    ((1120, 700), (1280, 800), ScaleMode::Fit),
    ((1120, 700), (1366, 768), ScaleMode::Fit),
    ((1120, 700), (1920, 1080), ScaleMode::Fit),
    ((1120, 700), (2560, 1080), ScaleMode::Fit),
    ((1244, 700), (1280, 800), ScaleMode::Fit),
    ((1244, 700), (1366, 768), ScaleMode::Fit),
    ((1244, 700), (1920, 1080), ScaleMode::Fit),
    ((1244, 700), (2560, 1080), ScaleMode::Fit),
    ((1680, 700), (1280, 800), ScaleMode::Fit),
    ((1680, 700), (1366, 768), ScaleMode::Fit),
    ((1680, 700), (1920, 1080), ScaleMode::Fit),
    ((1680, 700), (2560, 1080), ScaleMode::Fit),
    ((900, 700), (2560, 1440), ScaleMode::Fit),
];

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

    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() % u64::from(hi - lo + 1)) as u32
    }
}

fn agree(canvas: (u32, u32), surface: (u32, u32), mode: ScaleMode) -> bool {
    let presenter = viewport_for(
        canvas,
        surface,
        &PresentParams {
            scale: mode,
            ..PresentParams::default()
        },
    );
    let cpu_mode = match mode {
        ScaleMode::Integer => headless::ScaleMode::Integer,
        ScaleMode::Fit => headless::ScaleMode::Fit,
    };
    let cpu = headless::viewport(canvas, surface, cpu_mode);
    let as_vp = Viewport {
        x: cpu.x,
        y: cpu.y,
        w: cpu.w as u32,
        h: cpu.h as u32,
    };
    if as_vp != presenter {
        eprintln!("{mode:?} {canvas:?} in {surface:?}: cpu {cpu:?}, presenter {presenter:?}");
    }
    as_vp == presenter
}

#[test]
fn headless_viewport_matches_presenter() {
    let mut mismatches = 0;
    for (c, s, m) in VECTORS {
        if !agree(c, s, m) {
            mismatches += 1;
        }
    }
    let mut rng = Rng(0x0BAD_C0DE_5EED_0325);
    for _ in 0..500 {
        let canvas = (rng.range(1, 2000), rng.range(1, 1500));
        let surface = (rng.range(1, 4000), rng.range(1, 2500));
        let mode = if rng.next() & 1 == 0 {
            ScaleMode::Integer
        } else {
            ScaleMode::Fit
        };
        if !agree(canvas, surface, mode) {
            mismatches += 1;
        }
    }
    println!(
        "headless_viewport_matches_presenter: {} vectors + 500 random, {mismatches} mismatches",
        VECTORS.len()
    );
    assert_eq!(mismatches, 0);
}
