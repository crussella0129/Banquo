//! T217.1 `test_presenter_host_owned_loop`: a host-owned winit 0.30 loop creates
//! the presenter after a pre-device hook, once per backend option.
//!
//! Custom harness (`harness = false`): winit needs the main thread for its
//! event loop on Windows and macOS, and libtest runs tests on worker threads.
//!
//! Skips (exit 0, with a reason) when no event loop can be created (headless CI
//! without a display). A backend with no adapter on this machine is reported as
//! skipped, but its construction trace must still show the hook ran first.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use banquo_presenter::{
    wgpu, winit, Backend, FrameOutcome, InitStep, PresenterBuilder, PresenterError, RedrawPolicy,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Order counter shared by the hook and the trace check: the hook takes ticket 0.
static TICKETS: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct Host {
    results: Vec<(Backend, Result<String, String>)>,
    failures: Vec<String>,
    done: bool,
}

impl Host {
    fn check_backend(&mut self, event_loop: &ActiveEventLoop, requested: Backend) {
        let attrs = Window::default_attributes()
            .with_title(format!("banquo-presenter test ({requested})"))
            .with_inner_size(winit::dpi::PhysicalSize::new(256u32, 144u32))
            .with_visible(false);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.results
                    .push((requested, Err(format!("window creation failed: {e}"))));
                return;
            }
        };

        TICKETS.store(0, Ordering::SeqCst);
        let (built, hook_ticket) = PresenterBuilder::new()
            .backend(requested)
            .pre_device(|| TICKETS.fetch_add(1, Ordering::SeqCst))
            .build_for_window(&window);

        if hook_ticket != 0 {
            self.failures
                .push(format!("{requested}: pre-device hook did not run first"));
        }

        match built {
            Ok(mut presenter) => {
                let trace = presenter.init_trace().to_vec();
                check_trace(requested, &trace, &mut self.failures);
                if presenter.redraw_policy() != RedrawPolicy::Continuous {
                    self.failures
                        .push(format!("{requested}: default redraw policy not continuous"));
                }
                if presenter.present_mode() != wgpu::PresentMode::Fifo {
                    self.failures
                        .push(format!("{requested}: default present mode not Fifo"));
                }
                if presenter.control_flow() != winit::event_loop::ControlFlow::Poll {
                    self.failures
                        .push(format!("{requested}: continuous policy should poll"));
                }
                // A tiny RGBA canvas and a clear; a hidden window may report
                // Skipped/Reconfigured instead of Presented, which is fine.
                let canvas = [255u8, 0, 255, 255].repeat(4 * 4);
                let a = presenter.present_rgba(&canvas, 4, 4, wgpu::Color::BLACK);
                let b = presenter.present_clear(wgpu::Color::BLUE);
                let ok = |o: FrameOutcome| {
                    matches!(
                        o,
                        FrameOutcome::Presented
                            | FrameOutcome::Skipped
                            | FrameOutcome::Reconfigured
                    )
                };
                if !ok(a) || !ok(b) {
                    self.failures.push(format!(
                        "{requested}: unexpected frame outcomes {a:?} {b:?}"
                    ));
                }
                self.results.push((
                    requested,
                    Ok(format!(
                        "used {} ({}), frames {a:?}/{b:?}",
                        presenter.backend(),
                        presenter.adapter_info().name
                    )),
                ));
            }
            Err(e) => {
                check_trace(requested, e.trace(), &mut self.failures);
                if !matches!(e, PresenterError::NoUsableBackend { .. }) {
                    self.failures.push(format!("{requested}: {e}"));
                }
                self.results.push((requested, Err(format!("skipped: {e}"))));
            }
        }
        // The presenter (and its surface) drops here, before the window.
    }
}

fn check_trace(requested: Backend, trace: &[InitStep], failures: &mut Vec<String>) {
    if trace.first() != Some(&InitStep::PreDeviceHook) {
        failures.push(format!(
            "{requested}: trace must start with the pre-device hook: {trace:?}"
        ));
    }
    let first_instance = trace
        .iter()
        .position(|s| matches!(s, InitStep::InstanceCreated(_)));
    match first_instance {
        Some(i) if i > 0 => {
            if trace[i] != InitStep::InstanceCreated(requested.resolve()) {
                failures.push(format!(
                    "{requested}: first instance should use the requested backend: {trace:?}"
                ));
            }
        }
        _ => failures.push(format!(
            "{requested}: no instance after the hook: {trace:?}"
        )),
    }
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.done {
            return;
        }
        self.done = true;
        for backend in Backend::ALL {
            self.check_backend(event_loop, backend);
        }
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            println!("test_presenter_host_owned_loop ... skipped (no event loop: {e})");
            return;
        }
    };
    let mut host = Host::default();
    if let Err(e) = event_loop.run_app(&mut host) {
        println!("test_presenter_host_owned_loop ... skipped (event loop error: {e})");
        return;
    }
    for (backend, result) in &host.results {
        match result {
            Ok(msg) => println!("  {backend}: ok, {msg}"),
            Err(msg) => println!("  {backend}: {msg}"),
        }
    }
    if host.results.len() != Backend::ALL.len() {
        host.failures.push(format!(
            "expected {} backend results, got {}",
            Backend::ALL.len(),
            host.results.len()
        ));
    }
    if host.failures.is_empty() {
        println!("test_presenter_host_owned_loop ... ok");
    } else {
        for f in &host.failures {
            eprintln!("FAIL: {f}");
        }
        panic!("test_presenter_host_owned_loop failed");
    }
}
