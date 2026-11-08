//! T217.1 `test_presenter_host_owned_loop`: a host-owned winit 0.30 loop creates
//! the presenter after a pre-device hook, once per backend option.
//!
//! Custom harness (`harness = false`): winit needs the main thread for its
//! event loop on Windows and macOS, and libtest runs tests on worker threads.
//!
//! Every backend option is built with `allow_fallback(false)`, so the presenter
//! either runs on exactly the requested backend (the adapter selected in the
//! trace and `Presenter::backend()` must both be that backend) or fails with
//! `NoUsableBackend` for that backend alone. A separate default-config case
//! (fallback on) checks that any fallback is logged in the trace.
//!
//! A missing event loop, a window that cannot be created, or a backend with no
//! adapter is a FAILURE. Set `BANQUO_ALLOW_HEADLESS=1` (headless CI, GPU-less
//! runners) to report those as skipped instead; the summary line then says
//! `skipped` rather than `ok`, so a gate log can tell the two apart.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use banquo_presenter::{
    wgpu, winit, Backend, FrameOutcome, InitStep, Presenter, PresenterBuilder, PresenterError,
    RedrawPolicy,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Order counter shared by the hook and the trace check: the hook takes ticket 0.
static TICKETS: AtomicUsize = AtomicUsize::new(0);

/// The variable that turns "no display / no adapter" from a failure into a skip.
const ALLOW_HEADLESS: &str = "BANQUO_ALLOW_HEADLESS";

fn headless_allowed() -> bool {
    std::env::var(ALLOW_HEADLESS)
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

/// Which builder configuration a case uses.
#[derive(Clone, Copy, Debug)]
enum Case {
    /// `.backend(b).allow_fallback(false)`: the requested backend or nothing.
    Strict(Backend),
    /// `PresenterBuilder::new()` untouched: `Auto` with fallback on.
    DefaultConfig,
}

impl Case {
    fn requested(self) -> Backend {
        match self {
            Case::Strict(b) => b,
            Case::DefaultConfig => Backend::default(),
        }
    }

    fn label(self) -> String {
        match self {
            Case::Strict(b) => b.to_string(),
            Case::DefaultConfig => "default-config".to_string(),
        }
    }
}

struct Host {
    allow_headless: bool,
    results: Vec<(String, Result<String, String>)>,
    failures: Vec<String>,
    skipped: usize,
    done: bool,
}

impl Host {
    fn new(allow_headless: bool) -> Self {
        Self {
            allow_headless,
            results: Vec::new(),
            failures: Vec::new(),
            skipped: 0,
            done: false,
        }
    }

    /// Records an environment-caused miss: a failure unless headless runs are allowed.
    fn miss(&mut self, label: &str, why: String) {
        if self.allow_headless {
            self.skipped += 1;
            self.results.push((
                label.to_string(),
                Err(format!("skipped ({ALLOW_HEADLESS}): {why}")),
            ));
        } else {
            self.failures.push(format!(
                "{label}: {why} (set {ALLOW_HEADLESS}=1 only on a runner without a GPU or display)"
            ));
            self.results.push((label.to_string(), Err(why)));
        }
    }

    fn check_case(&mut self, event_loop: &ActiveEventLoop, case: Case) {
        let label = case.label();
        let requested = case.requested();
        let attrs = Window::default_attributes()
            .with_title(format!("banquo-presenter test ({label})"))
            .with_inner_size(winit::dpi::PhysicalSize::new(256u32, 144u32))
            .with_visible(false);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.miss(&label, format!("window creation failed: {e}"));
                return;
            }
        };

        TICKETS.store(0, Ordering::SeqCst);
        let builder = match case {
            Case::Strict(b) => PresenterBuilder::new().backend(b).allow_fallback(false),
            Case::DefaultConfig => PresenterBuilder::new(),
        };
        let (built, hook_ticket) = builder
            .pre_device(|| TICKETS.fetch_add(1, Ordering::SeqCst))
            .build_for_window(&window);

        if hook_ticket != 0 {
            self.failures
                .push(format!("{label}: pre-device hook did not run first"));
        }

        match built {
            Ok(mut presenter) => {
                let trace = presenter.init_trace().to_vec();
                check_trace(&label, requested, &trace, &mut self.failures);
                match case {
                    Case::Strict(_) => {
                        check_strict_ok(&label, requested, &presenter, &trace, &mut self.failures)
                    }
                    Case::DefaultConfig => check_fallback_logged(
                        &label,
                        requested,
                        &presenter,
                        &trace,
                        &mut self.failures,
                    ),
                }
                check_defaults_and_frames(&label, &mut presenter, &mut self.failures);
                self.results.push((
                    label,
                    Ok(format!(
                        "used {} ({})",
                        presenter.backend(),
                        presenter.adapter_info().name
                    )),
                ));
            }
            Err(e) => {
                check_trace(&label, requested, e.trace(), &mut self.failures);
                match (&e, case) {
                    (PresenterError::NoUsableBackend { tried, .. }, Case::Strict(_)) => {
                        // allow_fallback(false): exactly the requested backend was tried.
                        if tried.as_slice() != [requested.resolve()] {
                            self.failures.push(format!(
                                "{label}: fallback off, but tried {tried:?} (want [{}])",
                                requested.resolve()
                            ));
                        }
                        self.miss(&label, format!("no usable backend: {e}"));
                    }
                    (PresenterError::NoUsableBackend { tried, .. }, Case::DefaultConfig) => {
                        if tried.as_slice() != requested.fallback_order().as_slice() {
                            self.failures.push(format!(
                                "{label}: tried {tried:?}, want the platform order {:?}",
                                requested.fallback_order()
                            ));
                        }
                        self.miss(&label, format!("no usable backend: {e}"));
                    }
                    (other, _) => self.failures.push(format!("{label}: {other}")),
                }
            }
        }
        // The presenter (and its surface) drops here, before the window.
    }
}

/// Fallback off and building succeeded: it must be on the requested backend.
fn check_strict_ok(
    label: &str,
    requested: Backend,
    presenter: &Presenter,
    trace: &[InitStep],
    failures: &mut Vec<String>,
) {
    let want = requested.resolve();
    if presenter.backend() != want {
        failures.push(format!(
            "{label}: presenter runs on {} but {want} was requested with fallback off",
            presenter.backend()
        ));
    }
    if presenter.requested_backend() != requested {
        failures.push(format!(
            "{label}: requested_backend() is {}, want {requested}",
            presenter.requested_backend()
        ));
    }
    let adapters: Vec<Backend> = trace
        .iter()
        .filter_map(|s| match s {
            InitStep::AdapterSelected(b, _) => Some(*b),
            _ => None,
        })
        .collect();
    if adapters != [want] {
        failures.push(format!(
            "{label}: trace should select exactly one {want} adapter, got {adapters:?}: {trace:?}"
        ));
    }
    let instances = trace
        .iter()
        .filter(|s| matches!(s, InitStep::InstanceCreated(_)))
        .count();
    if instances != 1 {
        failures.push(format!(
            "{label}: fallback off, but {instances} instances were created: {trace:?}"
        ));
    }
    if trace
        .iter()
        .any(|s| matches!(s, InitStep::BackendUnavailable(..)))
    {
        failures.push(format!(
            "{label}: fallback off, but a backend was reported unavailable: {trace:?}"
        ));
    }
}

/// Fallback on: every backend tried before the one in use must be logged as
/// unavailable, in the platform order, and the adapter must match the backend.
fn check_fallback_logged(
    label: &str,
    requested: Backend,
    presenter: &Presenter,
    trace: &[InitStep],
    failures: &mut Vec<String>,
) {
    let used = presenter.backend();
    let order = requested.fallback_order();
    let Some(pos) = order.iter().position(|b| *b == used) else {
        failures.push(format!(
            "{label}: used {used}, which is not in the fallback order {order:?}"
        ));
        return;
    };
    let unavailable: Vec<Backend> = trace
        .iter()
        .filter_map(|s| match s {
            InitStep::BackendUnavailable(b, _) => Some(*b),
            _ => None,
        })
        .collect();
    if unavailable != order[..pos] {
        failures.push(format!(
            "{label}: used {used}; the trace must log {:?} as unavailable first, logged {unavailable:?}: {trace:?}",
            &order[..pos]
        ));
    }
    let last_adapter = trace.iter().rev().find_map(|s| match s {
        InitStep::AdapterSelected(b, _) => Some(*b),
        _ => None,
    });
    if last_adapter != Some(used) {
        failures.push(format!(
            "{label}: presenter runs on {used} but the last adapter selected was {last_adapter:?}"
        ));
    }
    if used != requested.resolve() {
        println!(
            "  {label}: fell back from {} to {used}",
            requested.resolve()
        );
    }
}

fn check_defaults_and_frames(label: &str, presenter: &mut Presenter, failures: &mut Vec<String>) {
    if presenter.redraw_policy() != RedrawPolicy::Continuous {
        failures.push(format!("{label}: default redraw policy not continuous"));
    }
    if presenter.present_mode() != wgpu::PresentMode::Fifo {
        failures.push(format!("{label}: default present mode not Fifo"));
    }
    if presenter.control_flow() != winit::event_loop::ControlFlow::Poll {
        failures.push(format!("{label}: continuous policy should poll"));
    }
    // A tiny RGBA canvas and a clear; a hidden window may report
    // Skipped/Reconfigured instead of Presented, which is fine.
    let canvas = [255u8, 0, 255, 255].repeat(4 * 4);
    let a = presenter.present_rgba(&canvas, 4, 4, wgpu::Color::BLACK);
    let b = presenter.present_clear(wgpu::Color::BLUE);
    let ok = |o: FrameOutcome| {
        matches!(
            o,
            FrameOutcome::Presented | FrameOutcome::Skipped | FrameOutcome::Reconfigured
        )
    };
    if !ok(a) || !ok(b) {
        failures.push(format!("{label}: unexpected frame outcomes {a:?} {b:?}"));
    }
}

fn check_trace(label: &str, requested: Backend, trace: &[InitStep], failures: &mut Vec<String>) {
    if trace.first() != Some(&InitStep::PreDeviceHook) {
        failures.push(format!(
            "{label}: trace must start with the pre-device hook: {trace:?}"
        ));
    }
    let first_instance = trace
        .iter()
        .position(|s| matches!(s, InitStep::InstanceCreated(_)));
    match first_instance {
        Some(i) if i > 0 => {
            if trace[i] != InitStep::InstanceCreated(requested.resolve()) {
                failures.push(format!(
                    "{label}: first instance should use the requested backend: {trace:?}"
                ));
            }
        }
        _ => failures.push(format!("{label}: no instance after the hook: {trace:?}")),
    }
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.done {
            return;
        }
        self.done = true;
        for backend in Backend::ALL {
            self.check_case(event_loop, Case::Strict(backend));
        }
        self.check_case(event_loop, Case::DefaultConfig);
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() {
    let allow_headless = headless_allowed();
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) if allow_headless => {
            println!(
                "test_presenter_host_owned_loop ... skipped (no event loop: {e}; {ALLOW_HEADLESS} set)"
            );
            return;
        }
        Err(e) => panic!(
            "test_presenter_host_owned_loop: no event loop: {e} (set {ALLOW_HEADLESS}=1 only on a headless runner)"
        ),
    };
    let mut host = Host::new(allow_headless);
    if let Err(e) = event_loop.run_app(&mut host) {
        if allow_headless {
            println!(
                "test_presenter_host_owned_loop ... skipped (event loop error: {e}; {ALLOW_HEADLESS} set)"
            );
            return;
        }
        panic!("test_presenter_host_owned_loop: event loop error: {e}");
    }
    for (label, result) in &host.results {
        match result {
            Ok(msg) => println!("  {label}: ok, {msg}"),
            Err(msg) => println!("  {label}: {msg}"),
        }
    }
    let expected = Backend::ALL.len() + 1;
    if host.results.len() != expected {
        host.failures.push(format!(
            "expected {expected} case results, got {}",
            host.results.len()
        ));
    }
    if !host.failures.is_empty() {
        for f in &host.failures {
            eprintln!("FAIL: {f}");
        }
        panic!("test_presenter_host_owned_loop failed");
    }
    if host.skipped > 0 {
        println!(
            "test_presenter_host_owned_loop ... skipped ({} of {expected} cases had no window or adapter; {ALLOW_HEADLESS} set)",
            host.skipped
        );
    } else {
        println!("test_presenter_host_owned_loop ... ok");
    }
}
