//! GPU readback identity: the presenter's blit (Nearest, Integer) draws a
//! random opaque sRGB canvas into an offscreen `Rgba8UnormSrgb` target and the
//! bytes read back equal the canvas at 1x and the CPU reference at 2x.
//!
//! Needs a real adapter, so it is `#[ignore]` (CI never runs it). Run it once
//! per backend:
//!
//! ```text
//! BANQUO_PRESENTER_BACKEND=dx12 cargo test -p banquo-presenter --test readback -- --ignored gpu_readback_identity_1x
//! ```
//!
//! `BANQUO_PRESENTER_BACKEND` is `dx12`, `vulkan` or `gl` (default: the
//! platform default). There is no fallback to another backend. With
//! `BANQUO_ALLOW_HEADLESS` set, a missing adapter skips instead of failing.

use banquo_canvas::headless::{self, HeadlessParams};
use banquo_canvas::{Rgba8, Surface};
use banquo_presenter::{render_offscreen, wgpu, Backend, Filter, PresentParams, ScaleMode};

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

fn diff_pixels(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len());
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(p, q)| p != q)
        .count()
}

/// The requested backend's device, or the reason there is none.
fn device_for(backend: Backend) -> Result<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo), String> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backend.wgpu_backends();
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .map_err(|e| format!("adapter: {e}"))?;
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("banquo-presenter readback"),
        required_limits:
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))
    .map_err(|e| format!("device: {e}"))?;
    Ok((device, queue, info))
}

#[test]
#[ignore = "needs a GPU adapter; run locally with --ignored per backend"]
fn gpu_readback_identity_1x() {
    let requested = std::env::var("BANQUO_PRESENTER_BACKEND").unwrap_or_else(|_| "auto".into());
    let backend = Backend::parse(&requested)
        .unwrap_or_else(|| panic!("BANQUO_PRESENTER_BACKEND={requested:?} is not a backend"));
    let (device, queue, info) = match device_for(backend) {
        Ok(d) => d,
        Err(why) if std::env::var_os("BANQUO_ALLOW_HEADLESS").is_some() => {
            println!("readback backend={backend} skipped (no adapter): {why}");
            return;
        }
        Err(why) => panic!("readback backend={backend}: no adapter: {why}"),
    };
    assert_eq!(
        Backend::from_wgpu(info.backend),
        Some(backend.resolve()),
        "adapter is on another backend"
    );

    let frame = random_opaque(900, 700, 0x5EED_0325);
    let params = PresentParams {
        scale: ScaleMode::Integer,
        filter: Filter::Nearest,
        background: wgpu::Color::BLACK,
    };

    let at_1x = render_offscreen(
        &device,
        &queue,
        frame.as_bytes(),
        (900, 700),
        (900, 700),
        &params,
    );
    let diff_1x = diff_pixels(&at_1x, frame.as_bytes());

    let at_2x = render_offscreen(
        &device,
        &queue,
        frame.as_bytes(),
        (900, 700),
        (1800, 1400),
        &params,
    );
    let reference = headless::render_headless(
        &frame,
        (1800, 1400),
        &HeadlessParams {
            scale: headless::ScaleMode::Integer,
            background: Rgba8::rgb(0, 0, 0),
        },
    );
    let diff_2x = diff_pixels(&at_2x, reference.as_bytes());

    println!(
        "readback backend={} adapter={:?} driver={:?} diff_1x={diff_1x}/630000 diff_2x={diff_2x}/2520000",
        backend.resolve(),
        info.name,
        info.driver_info
    );
    assert_eq!(diff_1x, 0, "1x readback differs from the canvas");
    assert_eq!(diff_2x, 0, "2x readback differs from render_headless");
}
