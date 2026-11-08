#![forbid(unsafe_code)]
//! Steam overlay spike for `banquo-presenter` (ADR-014, WordCrack T-217).
//!
//! ```text
//! steam_overlay_spike [--renderer dx12|vulkan|gl|auto] [--run-seconds N] [--no-steam]
//! ```
//!
//! - Initialises Steam (App ID 480, Spacewar) in the presenter's pre-device
//!   hook, i.e. before `wgpu::Instance::new`.
//! - Presents continuously (`RedrawPolicy::Continuous`, `Fifo`).
//! - Draws a test pattern: colour bars, a sweeping bar (proves every frame is
//!   new), the backend in use and the measured FPS.
//! - Logs `IsOverlayEnabled` once a second and every `GameOverlayActivated`
//!   callback.
//! - Without Steam (or with `--no-steam`) it still runs and shows `steam: off`.
//!
//! The Steamworks redistributable and `steam_appid.txt` are placed beside the
//! binary by `build.rs`; see `docs/steam-overlay.md` for the Deck steps.

mod font;

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use banquo_presenter::{winit, Backend, FrameOutcome, Presenter, PresenterBuilder};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use font::Canvas;

const APP_ID: u32 = 480;
const CANVAS_W: u32 = 320;
const CANVAS_H: u32 = 180;

macro_rules! log {
    ($($arg:tt)*) => {{
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "[spike] {}", format!($($arg)*));
        let _ = out.flush();
    }};
}

#[derive(Debug)]
struct Args {
    renderer: Backend,
    run_seconds: Option<f64>,
    no_steam: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        renderer: Backend::Auto,
        run_seconds: None,
        no_steam: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--renderer" => {
                let v = it.next().ok_or("--renderer needs dx12|vulkan|gl|auto")?;
                args.renderer =
                    Backend::parse(&v).ok_or_else(|| format!("unknown renderer `{v}`"))?;
            }
            "--run-seconds" => {
                let v = it.next().ok_or("--run-seconds needs a number")?;
                let n: f64 = v.parse().map_err(|_| format!("bad --run-seconds `{v}`"))?;
                args.run_seconds = Some(n);
            }
            "--no-steam" => args.no_steam = true,
            "-h" | "--help" => {
                println!(
                    "steam_overlay_spike [--renderer dx12|vulkan|gl|auto] [--run-seconds N] [--no-steam]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(args)
}

/// Steam state owned by the host (never by banquo-presenter).
struct Steam {
    client: steamworks::Client,
    overlay_active: Arc<AtomicBool>,
    _overlay_cb: steamworks::CallbackHandle,
}

/// Writes `steam_appid.txt` (480) beside the executable if it is missing, for
/// runs started outside Steam. `build.rs` normally already put it there.
fn ensure_appid_file() {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        let path = dir.join("steam_appid.txt");
        if !path.exists() {
            match std::fs::write(&path, APP_ID.to_string()) {
                Ok(()) => log!("wrote {}", path.display()),
                Err(e) => log!("could not write {}: {e}", path.display()),
            }
        }
    }
}

/// The pre-device hook: runs before the wgpu instance exists.
fn init_steam(no_steam: bool) -> Option<Steam> {
    log!("pre-device hook: initialising Steam before wgpu::Instance::new");
    if no_steam {
        log!("steam: off (--no-steam)");
        return None;
    }
    ensure_appid_file();
    match steamworks::Client::init_app(APP_ID) {
        Ok(client) => {
            let overlay_active = Arc::new(AtomicBool::new(false));
            let flag = overlay_active.clone();
            let cb = client.register_callback(move |e: steamworks::GameOverlayActivated| {
                flag.store(e.active, Ordering::SeqCst);
                log!("GameOverlayActivated active={}", e.active);
            });
            log!(
                "steam: on (app {APP_ID}), IsOverlayEnabled={} (may read false until the overlay finishes attaching)",
                client.utils().is_overlay_enabled()
            );
            Some(Steam {
                client,
                overlay_active,
                _overlay_cb: cb,
            })
        }
        Err(e) => {
            log!("steam: off ({e})");
            None
        }
    }
}

struct Stats {
    started: Instant,
    window_start: Instant,
    window_frames: u32,
    total_frames: u64,
    fps: f64,
    min_fps: Option<f64>,
    skipped: u64,
}

impl Stats {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            window_start: now,
            window_frames: 0,
            total_frames: 0,
            fps: 0.0,
            min_fps: None,
            skipped: 0,
        }
    }
}

struct Spike {
    args: Args,
    window: Option<Arc<Window>>,
    presenter: Option<Presenter>,
    steam: Option<Steam>,
    canvas: Canvas,
    stats: Stats,
    exit_code: i32,
}

fn steam_label(steam: &Option<Steam>) -> &'static str {
    if steam.is_some() {
        "steam: on"
    } else {
        "steam: off"
    }
}

impl Spike {
    fn draw(&mut self) {
        let Some(presenter) = &self.presenter else {
            return;
        };
        let c = &mut self.canvas;
        c.fill([16, 18, 24, 255]);
        // Colour bars.
        let bars: [[u8; 4]; 8] = [
            [235, 235, 235, 255],
            [235, 235, 16, 255],
            [16, 235, 235, 255],
            [16, 235, 16, 255],
            [235, 16, 235, 255],
            [235, 16, 16, 255],
            [16, 16, 235, 255],
            [16, 16, 16, 255],
        ];
        let bw = CANVAS_W as i32 / bars.len() as i32;
        for (i, col) in bars.iter().enumerate() {
            c.rect(i as i32 * bw, 0, bw, 24, *col);
        }
        // A bar sweeping once a second: a frozen frame is visible at once.
        let t = self.stats.started.elapsed().as_secs_f64();
        let x = ((t.fract()) * CANVAS_W as f64) as i32;
        c.rect(x - 2, 26, 4, 6, [255, 200, 40, 255]);
        // Corner markers (letterbox check).
        for (mx, my) in [(0, 34), (CANVAS_W as i32 - 4, 34), (0, CANVAS_H as i32 - 4)] {
            c.rect(mx, my, 4, 4, [255, 80, 80, 255]);
        }

        let white = [240, 240, 240, 255];
        let green = [120, 230, 120, 255];
        let amber = [250, 190, 70, 255];
        let backend = presenter.backend();
        let requested = presenter.requested_backend();
        c.text(8, 40, 2, "BANQUO PRESENTER SPIKE", white);
        let backend_line = if requested.resolve() == backend {
            format!("backend: {backend}")
        } else {
            format!("backend: {backend} (asked {requested})")
        };
        c.text(8, 56, 2, &backend_line, green);
        c.text(8, 72, 2, &format!("fps: {:.1}", self.stats.fps), green);
        let steam_line = steam_label(&self.steam);
        c.text(
            8,
            88,
            2,
            steam_line,
            if self.steam.is_some() { green } else { amber },
        );
        if let Some(steam) = &self.steam {
            let enabled = steam.client.utils().is_overlay_enabled();
            c.text(
                8,
                104,
                2,
                &format!("overlay enabled: {}", if enabled { "yes" } else { "no" }),
                white,
            );
            let active = steam.overlay_active.load(Ordering::SeqCst);
            c.text(
                8,
                120,
                2,
                &format!("overlay active: {}", if active { "yes" } else { "no" }),
                white,
            );
        }
        c.text(
            8,
            CANVAS_H as i32 - 22,
            1,
            &format!(
                "frame {}  present {:?}  shift+tab: overlay",
                self.stats.total_frames,
                presenter.present_mode()
            ),
            white,
        );
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(steam) = &self.steam {
            steam.client.run_callbacks();
        }
        self.draw();
        let Some(presenter) = self.presenter.as_mut() else {
            return;
        };
        let outcome = presenter.present_rgba(
            &self.canvas.pixels,
            CANVAS_W,
            CANVAS_H,
            banquo_presenter::wgpu::Color {
                r: 0.02,
                g: 0.02,
                b: 0.03,
                a: 1.0,
            },
        );
        match outcome {
            FrameOutcome::Presented => {
                self.stats.total_frames += 1;
                self.stats.window_frames += 1;
            }
            FrameOutcome::Reconfigured | FrameOutcome::Skipped => self.stats.skipped += 1,
        }

        let now = Instant::now();
        let span = now - self.stats.window_start;
        if span >= Duration::from_secs(1) {
            self.stats.fps = self.stats.window_frames as f64 / span.as_secs_f64();
            self.stats.min_fps = Some(
                self.stats
                    .min_fps
                    .map_or(self.stats.fps, |m: f64| m.min(self.stats.fps)),
            );
            self.stats.window_frames = 0;
            self.stats.window_start = now;
            let overlay = match &self.steam {
                Some(s) => format!(
                    "IsOverlayEnabled={} overlay_active={}",
                    s.client.utils().is_overlay_enabled(),
                    s.overlay_active.load(Ordering::SeqCst)
                ),
                None => "IsOverlayEnabled=n/a (steam: off)".to_string(),
            };
            log!(
                "t={:.2}s backend={} fps={:.1} frames={} {} {}",
                self.stats.started.elapsed().as_secs_f64(),
                presenter.backend(),
                self.stats.fps,
                self.stats.total_frames,
                steam_label(&self.steam),
                overlay
            );
        }

        if let Some(limit) = self.args.run_seconds {
            if self.stats.started.elapsed().as_secs_f64() >= limit {
                log!("--run-seconds {limit} reached");
                event_loop.exit();
                return;
            }
        }
        if let Some(window) = &self.window {
            presenter.after_frame(window);
        }
    }
}

impl ApplicationHandler for Spike {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!(
                "Banquo Steam overlay spike ({})",
                self.args.renderer
            ))
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log!("error: window creation failed: {e}");
                self.exit_code = 1;
                event_loop.exit();
                return;
            }
        };

        let no_steam = self.args.no_steam;
        let (built, steam) = PresenterBuilder::new()
            .backend(self.args.renderer)
            .pre_device(move || init_steam(no_steam))
            .build_for_window(&window);
        self.steam = steam;
        match built {
            Ok(presenter) => {
                let info = presenter.adapter_info();
                log!("init trace: {:?}", presenter.init_trace());
                log!(
                    "backend={} requested={} adapter=\"{}\" driver=\"{}\" present_mode={:?} format={:?} redraw={:?}",
                    presenter.backend(),
                    presenter.requested_backend(),
                    info.name,
                    info.driver_info,
                    presenter.present_mode(),
                    presenter.surface_format(),
                    presenter.redraw_policy()
                );
                window.set_title(&format!(
                    "Banquo Steam overlay spike: {} | {}",
                    presenter.backend(),
                    steam_label(&self.steam)
                ));
                event_loop.set_control_flow(presenter.control_flow());
                self.presenter = Some(presenter);
                self.stats = Stats::new();
                window.request_redraw();
                self.window = Some(window);
            }
            Err(e) => {
                log!("error: {e}");
                log!("init trace: {:?}", e.trace());
                self.exit_code = 1;
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                log!("close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(p) = self.presenter.as_mut() {
                    p.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        // Continuous policy: keep frames coming even when nothing changed
        // (overlay open, game paused).
        if let (Some(p), Some(w)) = (&self.presenter, &self.window) {
            p.after_frame(w);
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        let secs = self.stats.started.elapsed().as_secs_f64();
        let backend = self
            .presenter
            .as_ref()
            .map_or("none".to_string(), |p| p.backend().to_string());
        log!(
            "exit: backend={} frames={} seconds={:.2} avg_fps={:.1} min_1s_fps={} skipped={} {}",
            backend,
            self.stats.total_frames,
            secs,
            if secs > 0.0 {
                self.stats.total_frames as f64 / secs
            } else {
                0.0
            },
            self.stats
                .min_fps
                .map_or("n/a".to_string(), |m| format!("{m:.1}")),
            self.stats.skipped,
            steam_label(&self.steam)
        );
        // Drop the presenter (surface, device) before the window and Steam.
        self.presenter = None;
        self.window = None;
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    log!("args: {args:?}");

    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        // ADR-014 item 6: the overlay cannot hook Wayland surfaces.
        use winit::platform::x11::EventLoopBuilderExtX11 as _;
        builder.with_x11();
        log!("winit: forcing X11");
    }
    let event_loop = match builder.build() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("error: cannot create event loop: {e}");
            std::process::exit(1);
        }
    };
    let mut spike = Spike {
        args,
        window: None,
        presenter: None,
        steam: None,
        canvas: Canvas::new(CANVAS_W, CANVAS_H),
        stats: Stats::new(),
        exit_code: 0,
    };
    if let Err(e) = event_loop.run_app(&mut spike) {
        eprintln!("error: event loop: {e}");
        std::process::exit(1);
    }
    let code = spike.exit_code;
    drop(spike);
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T217.3: with no Steam client (init skipped or `init_app` failed) the HUD,
    /// the window title and the log lines all say `steam: off`.
    #[test]
    fn steam_label_off_without_steam() {
        assert_eq!(steam_label(&None), "steam: off");
    }
}
