//! CRT planes and frames against the pygame 2.6.1 goldens in `golden/crt`
//! (legacy CRT parameters at terminal widths 800, 1020, 1144 and 1580).

use std::fs;
use std::path::{Path, PathBuf};

use banquo_canvas::crt::{self, CrtMasks, CrtParams, CrtState, Vignette};
use banquo_canvas::{Rgba8, Surface};

const WIDTHS: [u32; 4] = [800, 1020, 1144, 1580];
const HEIGHT: u32 = 600;

fn golden(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/crt")
        .join(rel)
}

fn read_png(path: &Path) -> (u32, u32, usize, Vec<u8>) {
    let file = fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "{}", path.display());
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::Rgb => 3,
        other => panic!("{}: unexpected colour type {other:?}", path.display()),
    };
    buf.truncate(info.width as usize * info.height as usize * channels);
    (info.width, info.height, channels, buf)
}

/// A gray alpha-plane PNG as an alpha surface of black pixels (0, 0, 0, v).
fn load_plane(path: &Path) -> Surface {
    let (w, h, ch, data) = read_png(path);
    assert_eq!(ch, 1, "{}", path.display());
    let mut s = Surface::new(w, h, true);
    for (i, &v) in data.iter().enumerate() {
        let (x, y) = ((i as u32 % w) as i32, (i as u32 / w) as i32);
        s.set(x, y, Rgba8::new(0, 0, 0, v));
    }
    s
}

fn load_rgb(path: &Path) -> Surface {
    let (w, h, ch, data) = read_png(path);
    assert_eq!(ch, 3, "{}", path.display());
    let mut s = Surface::new(w, h, false);
    for (i, &[r, g, b]) in data.as_chunks::<3>().0.iter().enumerate() {
        let (x, y) = ((i as u32 % w) as i32, (i as u32 / w) as i32);
        s.set(x, y, Rgba8::rgb(r, g, b));
    }
    s
}

fn differing_pixels(got: &Surface, want: &Surface) -> usize {
    assert_eq!((got.width(), got.height()), (want.width(), want.height()));
    got.as_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(want.as_bytes().as_chunks::<4>().0)
        .filter(|(a, b)| a != b)
        .count()
}

struct Flicker {
    dts: Vec<f64>,
    alphas: Vec<u8>,
}

fn flicker_golden() -> Flicker {
    let doc: serde_json::Value =
        serde_json::from_slice(&fs::read(golden("flicker.json")).unwrap()).unwrap();
    // dt is stored as integer microseconds and divided exactly as the
    // generator does, so no float text is parsed.
    let dts = doc["dt_micros"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as f64 / 1e6)
        .collect();
    let alphas = doc["alpha"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| u8::try_from(n.as_u64().unwrap()).unwrap())
        .collect();
    assert_eq!(doc["frame_time_start"], 0);
    assert_eq!(doc["flicker_rate"], 8.0);
    assert_eq!(doc["flicker_intensity"], 0.02);
    assert_eq!(doc["flicker_max_alpha"], 15);
    Flicker { dts, alphas }
}

#[test]
fn legacy_params() {
    let p = CrtParams::legacy();
    assert_eq!(p.scanline_every, 3);
    assert_eq!(p.scanline_alpha, 35);
    assert_eq!(
        p.vignette,
        Vignette::LegacyBands {
            steps: 20,
            max_alpha: 120.0,
            band: None
        }
    );
    assert_eq!(p.flicker_rate, 8.0);
    assert_eq!(p.flicker_intensity, 0.02);
    assert_eq!(p.flicker_max_alpha, 15);
}

#[test]
fn planes_match_pygame() {
    let p = CrtParams::legacy();
    let mut bands = Vec::new();
    for w in WIDTHS {
        let m = CrtMasks::build(w, HEIGHT, &p);

        let want = load_plane(&golden(&format!("scanline-{w}x{HEIGHT}.png")));
        let scan_px = differing_pixels(&m.scanlines, &want);
        let rows = (0..HEIGHT as i32)
            .filter(|&y| m.scanlines.get(0, y).unwrap().a == 35)
            .count();
        assert_eq!(rows, 200, "{w}: scanline rows");
        for y in 0..HEIGHT as i32 {
            let a = if y % 3 == 0 { 35 } else { 0 };
            for x in [0, w as i32 / 2, w as i32 - 1] {
                assert_eq!(m.scanlines.get(x, y).unwrap(), Rgba8::new(0, 0, 0, a));
            }
        }

        let want = load_plane(&golden(&format!("vignette-{w}x{HEIGHT}.png")));
        let vig_px = differing_pixels(&m.vignette, &want);

        let band = crt::legacy_band_width(w, HEIGHT, 20);
        bands.push(band);
        println!(
            "crt {w}x{HEIGHT}: scanline {scan_px} differing px ({rows} rows), \
             vignette {vig_px} differing px, band {band}"
        );
        assert_eq!(scan_px, 0, "scanline {w}");
        assert_eq!(vig_px, 0, "vignette {w}");
    }
    assert_eq!(bands, [35, 40, 43, 54]);
}

#[test]
fn flicker_matches_pygame() {
    let g = flicker_golden();
    assert_eq!(g.dts.len(), 60);
    assert_eq!(g.alphas.len(), 60);
    let p = CrtParams::legacy();
    // apply() on a 1x1 surface returns the alpha it used for each frame.
    let m = CrtMasks::build(1, 1, &p);
    let mut s = Surface::new(1, 1, false);
    let mut st = CrtState::default();
    let got: Vec<u8> = g
        .dts
        .iter()
        .map(|&dt| crt::apply(&mut s, &m, &p, &mut st, dt))
        .collect();
    println!("flicker {got:?}");
    assert!(g.alphas.iter().all(|&a| a <= 5));
    assert!(g.alphas.iter().any(|&a| a > 0));
    assert_eq!(got, g.alphas);
}

#[test]
fn apply_three_frames_matches_pygame() {
    let g = flicker_golden();
    let p = CrtParams::legacy();
    for w in WIDTHS {
        let m = CrtMasks::build(w, HEIGHT, &p);
        let mut s = Surface::new(w, HEIGHT, false);
        s.fill(Rgba8::rgb(0, 170, 0));
        let mut st = CrtState::default();
        let alphas: Vec<u8> = g.dts[..3]
            .iter()
            .map(|&dt| crt::apply(&mut s, &m, &p, &mut st, dt))
            .collect();
        let want = load_rgb(&golden(&format!("apply-{w}x{HEIGHT}-f3.png")));
        let px = differing_pixels(&s, &want);
        println!("apply {w}x{HEIGHT} f3: {px} differing px, flicker {alphas:?}");
        assert_eq!(px, 0, "apply {w}");
    }
}

#[test]
fn resize_rebuilds_planes_for_each_width() {
    let p = CrtParams::legacy();
    let mut m = CrtMasks::build(WIDTHS[0], HEIGHT, &p);
    for w in WIDTHS {
        m.resize(w, HEIGHT, &p);
        assert_eq!(m, CrtMasks::build(w, HEIGHT, &p));
        assert_eq!(m.size(), (w, HEIGHT));
    }
}
