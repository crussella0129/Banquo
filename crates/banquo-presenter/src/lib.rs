#![forbid(unsafe_code)]
//! # banquo-presenter
//!
//! Banquo's embeddable wgpu presenter (ADR-014). It owns the wgpu instance,
//! adapter, device and surface lifecycle, the backend and present-mode policy,
//! a **pre-device hook**, and the presentation (letterboxed) of a host-provided
//! CPU canvas. It does **not** own an event loop and has no eframe/egui or
//! Steamworks dependency.
//!
//! ## Contract
//!
//! 1. **The host owns the event loop.** The presenter never calls `run_app`.
//!    The host creates the window (in winit's `resumed`), forwards resizes with
//!    [`Presenter::resize`], and calls [`Presenter::present_rgba`] /
//!    [`Presenter::present_clear`] when it redraws.
//! 2. **Pre-device hook.** [`PresenterBuilder::pre_device`] takes a closure that
//!    runs **before** `wgpu::Instance::new`. Its result is handed back to the
//!    host. Steam (`SteamAPI_Init`) belongs in that hook, because the Steam
//!    overlay hooks device and swap-chain creation. [`Presenter::init_trace`]
//!    records that the hook ran first.
//! 3. **One backend per instance.** [`Backend::Dx12`], [`Backend::Vulkan`],
//!    [`Backend::Gl`] or [`Backend::Auto`]. When the requested backend yields no
//!    adapter the presenter walks the platform fallback order
//!    ([`Backend::fallback_order`]) and reports the backend actually used.
//! 4. **Continuous redraw by default** ([`RedrawPolicy::Continuous`]): the host
//!    presents every frame, including while an overlay is open or the game is
//!    paused. [`RedrawPolicy::OnDemand`] exists for non-Steam tools and breaks
//!    the Steam overlay.
//! 5. **`Fifo` by default.** `Mailbox` / `Immediate` are opt-in and fall back to
//!    `Fifo` when the surface lacks them; [`PresentMode::Uncapped`] tries
//!    `Mailbox`, then `Immediate`, then `Fifo`. `Lost` / `Outdated` surfaces are
//!    reconfigured on the next frame.
//! 6. **DX12** always uses `Dx12SwapchainKind::DxgiFromHwnd` with an opaque
//!    surface.
//!
//! ## Platform rules for hosts
//!
//! - **Linux:** force winit's X11 backend (`EventLoopBuilderExtX11::with_x11()`)
//!   when the host may run under Steam. winit prefers Wayland, and the Steam
//!   overlay cannot hook Wayland surfaces.
//! - **Windows:** a Steam-launched host must never relaunch itself detached
//!   (Banquo's `os::ensure_detached()`, ADR-012): Steam would think the game
//!   exited.
//!
//! ## Presentation parameters
//!
//! [`PresentParams`] picks the scaling rule ([`ScaleMode::Integer`], the
//! default, is pygame `SCALED` in a window: whole-number scale, centred;
//! [`ScaleMode::Fit`] is SDL 2's `RenderSetLogicalSize` letterbox), the texture
//! filter ([`Filter::Nearest`] by default) and the bar colour.
//! [`viewport_for`] and [`window_to_logical`] are the pure arithmetic behind
//! [`Presenter::viewport`] and [`Presenter::window_to_logical`]. The surface is
//! always written through an sRGB view ([`choose_surface_format`]); a backend
//! that offers no sRGB-capable format is reported unavailable.
//!
//! ## Hosts without winit
//!
//! The core API needs only `raw-window-handle` 0.6. A host with its own
//! windowing library (SDL2, for example) depends on this crate with
//! `default-features = false` (no winit in the tree) and wraps its window's raw
//! handles in a small newtype that implements `HasWindowHandle`,
//! `HasDisplayHandle`, `Debug`, `Send`, `Sync` and `Clone` (the
//! [`PresentTarget`] bounds). That newtype goes straight into
//! [`PresenterBuilder::build`], so the pre-device hook, the backend walk and
//! DX12's `DxgiFromHwnd` all apply unchanged; this has been verified with an
//! SDL 2.28.4 window on DX12, Vulkan and GL. The host must keep the window
//! alive for as long as the presenter exists, forward size changes to
//! [`Presenter::resize`], and present every frame. [`Presenter::from_parts`]
//! remains for hosts that must create the surface themselves.
//!
//! ## Safety
//!
//! `#![forbid(unsafe_code)]`. Surfaces are created through wgpu's safe
//! `create_surface` from an owned window handle (`Arc<Window>` or any
//! `raw-window-handle` 0.6 provider). A host that must create a surface
//! unsafely does so itself and passes it to [`Presenter::from_parts`].

use std::fmt;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

mod params;

pub use params::{
    choose_surface_format, sampler_for, viewport_for, viewport_for_mode, window_to_logical, Filter,
    PresentParams, ScaleMode, Viewport,
};

pub use wgpu;
#[cfg(feature = "winit")]
pub use winit;

// ---------------------------------------------------------------------------
// Policy types
// ---------------------------------------------------------------------------

/// The graphics backend a presenter instance is created with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Direct3D 12 (Windows).
    Dx12,
    /// Vulkan (Windows, Linux, Steam Deck).
    Vulkan,
    /// OpenGL / GLES (WGL on Windows, EGL on Linux).
    Gl,
    /// The platform default: DX12 on Windows, Vulkan elsewhere.
    #[default]
    Auto,
}

impl Backend {
    /// Every option a host can expose, in launch-flag order.
    pub const ALL: [Backend; 4] = [Backend::Dx12, Backend::Vulkan, Backend::Gl, Backend::Auto];

    /// Parses a launch-option value (`dx12`, `vulkan`/`vk`, `gl`/`opengl`, `auto`),
    /// case-insensitively.
    pub fn parse(value: &str) -> Option<Backend> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dx12" | "d3d12" => Some(Backend::Dx12),
            "vulkan" | "vk" => Some(Backend::Vulkan),
            "gl" | "opengl" | "gles" => Some(Backend::Gl),
            "auto" => Some(Backend::Auto),
            _ => None,
        }
    }

    /// The lower-case launch-option name.
    pub fn name(self) -> &'static str {
        match self {
            Backend::Dx12 => "dx12",
            Backend::Vulkan => "vulkan",
            Backend::Gl => "gl",
            Backend::Auto => "auto",
        }
    }

    /// Resolves [`Backend::Auto`] to this platform's default; other values are
    /// returned unchanged.
    pub fn resolve(self) -> Backend {
        match self {
            Backend::Auto if cfg!(windows) => Backend::Dx12,
            Backend::Auto => Backend::Vulkan,
            other => other,
        }
    }

    /// The platform's fallback order (ADR-014 item 4).
    pub fn platform_order() -> &'static [Backend] {
        if cfg!(windows) {
            &[Backend::Dx12, Backend::Vulkan, Backend::Gl]
        } else {
            &[Backend::Vulkan, Backend::Gl]
        }
    }

    /// The backends tried, in order, when this one is requested: the resolved
    /// request first, then the rest of the platform order.
    pub fn fallback_order(self) -> Vec<Backend> {
        let first = self.resolve();
        let mut order = vec![first];
        order.extend(
            Backend::platform_order()
                .iter()
                .copied()
                .filter(|b| *b != first),
        );
        order
    }

    /// The single `wgpu::Backends` flag this backend creates its instance with.
    /// `Auto` maps to its resolved backend, so it is still exactly one flag.
    pub fn wgpu_backends(self) -> wgpu::Backends {
        match self.resolve() {
            Backend::Dx12 => wgpu::Backends::DX12,
            Backend::Vulkan => wgpu::Backends::VULKAN,
            Backend::Gl | Backend::Auto => wgpu::Backends::GL,
        }
    }

    /// Maps a wgpu adapter backend back to a presenter backend.
    pub fn from_wgpu(backend: wgpu::Backend) -> Option<Backend> {
        match backend {
            wgpu::Backend::Dx12 => Some(Backend::Dx12),
            wgpu::Backend::Vulkan => Some(Backend::Vulkan),
            wgpu::Backend::Gl => Some(Backend::Gl),
            _ => None,
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// When the host asks for frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedrawPolicy {
    /// Present every frame, always. Required for the Steam overlay. Default.
    #[default]
    Continuous,
    /// Present only when the host has something new. **Breaks the Steam
    /// overlay** (it draws into the game's own frames); for non-Steam tools only.
    OnDemand,
}

/// Requested present mode. Anything but `Fifo` falls back to `Fifo` when the
/// surface does not support it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentMode {
    /// V-synced queue. Always supported. Default.
    #[default]
    Fifo,
    /// Low-latency v-synced replace (opt-in).
    Mailbox,
    /// Un-synced (opt-in; may tear).
    Immediate,
    /// Not tied to the display refresh, like pygame's default (no vsync):
    /// `Mailbox`, else `Immediate`, else `Fifo` (opt-in). The host paces
    /// frames itself (for example with a 60 Hz tick).
    Uncapped,
}

impl PresentMode {
    /// The wgpu modes this request tries, in order; `Fifo` is always last.
    fn chain(self) -> &'static [wgpu::PresentMode] {
        match self {
            PresentMode::Fifo => &[wgpu::PresentMode::Fifo],
            PresentMode::Mailbox => &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo],
            PresentMode::Immediate => &[wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo],
            PresentMode::Uncapped => &[
                wgpu::PresentMode::Mailbox,
                wgpu::PresentMode::Immediate,
                wgpu::PresentMode::Fifo,
            ],
        }
    }
}

/// Picks the first mode of the request's chain that the surface supports, else
/// `Fifo` (see [`PresentMode`]).
pub fn choose_present_mode(
    requested: PresentMode,
    supported: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    requested
        .chain()
        .iter()
        .copied()
        .find(|m| supported.contains(m))
        .unwrap_or(wgpu::PresentMode::Fifo)
}

/// Picks `Opaque` when supported (DX12 flip-model swap chains and the overlay
/// both want an opaque surface), else the surface's first alpha mode.
pub fn choose_alpha_mode(supported: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    if supported.contains(&wgpu::CompositeAlphaMode::Opaque) || supported.is_empty() {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        supported[0]
    }
}

/// Presenter settings chosen by the host.
#[derive(Clone, Copy, Debug)]
pub struct PresenterConfig {
    /// Requested backend (default [`Backend::Auto`]).
    pub backend: Backend,
    /// Redraw policy (default [`RedrawPolicy::Continuous`]).
    pub redraw_policy: RedrawPolicy,
    /// Present mode (default [`PresentMode::Fifo`]).
    pub present_mode: PresentMode,
    /// Adapter power preference (default high performance).
    pub power_preference: wgpu::PowerPreference,
    /// Walk the platform fallback order when the requested backend has no
    /// adapter (default `true`).
    pub allow_fallback: bool,
}

impl Default for PresenterConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Auto,
            redraw_policy: RedrawPolicy::Continuous,
            present_mode: PresentMode::Fifo,
            power_preference: wgpu::PowerPreference::HighPerformance,
            allow_fallback: true,
        }
    }
}

/// One step of presenter construction, in the order it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitStep {
    /// The host's pre-device hook ran.
    PreDeviceHook,
    /// A `wgpu::Instance` was created with exactly this backend.
    InstanceCreated(Backend),
    /// The host handed over its own instance and surface ([`Presenter::from_parts`]).
    ExternalInstance,
    /// No usable adapter on this backend; the next one in the order is tried.
    BackendUnavailable(Backend, String),
    /// An adapter was selected (backend, adapter name).
    AdapterSelected(Backend, String),
    /// The device and queue were created.
    DeviceCreated,
    /// The surface was configured (present mode used).
    SurfaceConfigured(wgpu::PresentMode),
}

/// Why a presenter could not be built.
#[derive(Debug)]
pub enum PresenterError {
    /// No backend in the order produced a surface, adapter and device.
    NoUsableBackend {
        /// The backends tried, in order.
        tried: Vec<Backend>,
        /// Everything that happened, including the pre-device hook.
        trace: Vec<InitStep>,
    },
    /// The device could not be created on an external instance.
    Device(String),
}

impl fmt::Display for PresenterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresenterError::NoUsableBackend { tried, trace } => {
                let names: Vec<&str> = tried.iter().map(|b| b.name()).collect();
                write!(f, "no usable wgpu backend (tried {}): ", names.join(", "))?;
                for step in trace {
                    if let InitStep::BackendUnavailable(b, why) = step {
                        write!(f, "[{b}: {why}] ")?;
                    }
                }
                Ok(())
            }
            PresenterError::Device(why) => write!(f, "device creation failed: {why}"),
        }
    }
}

impl std::error::Error for PresenterError {}

impl PresenterError {
    /// The construction trace, when there is one.
    pub fn trace(&self) -> &[InitStep] {
        match self {
            PresenterError::NoUsableBackend { trace, .. } => trace,
            PresenterError::Device(_) => &[],
        }
    }
}

/// What happened to one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOutcome {
    /// The frame was rendered and presented.
    Presented,
    /// The surface was lost or outdated; it was reconfigured and the next frame
    /// will present.
    Reconfigured,
    /// Nothing to present to (zero-sized, occluded or timed out).
    Skipped,
}

/// A window handle the presenter can create a surface for and keep alive.
/// Implemented by `Arc<winit::window::Window>` and any owned, cloneable
/// raw-window-handle 0.6 provider.
pub trait PresentTarget:
    HasWindowHandle + HasDisplayHandle + fmt::Debug + Send + Sync + Clone + 'static
{
}

impl<T> PresentTarget for T where
    T: HasWindowHandle + HasDisplayHandle + fmt::Debug + Send + Sync + Clone + 'static
{
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builds a [`Presenter`]. The host calls [`PresenterBuilder::build`] from its
/// own event loop (winit: inside `resumed`, after creating the window).
pub struct PresenterBuilder<F> {
    config: PresenterConfig,
    pre_device: F,
}

impl PresenterBuilder<fn()> {
    /// A builder with default policy and an empty pre-device hook.
    pub fn new() -> Self {
        Self {
            config: PresenterConfig::default(),
            pre_device: || {},
        }
    }
}

impl Default for PresenterBuilder<fn()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F, R> PresenterBuilder<F>
where
    F: FnOnce() -> R,
{
    /// Replaces the whole configuration.
    pub fn config(mut self, config: PresenterConfig) -> Self {
        self.config = config;
        self
    }

    /// Requested backend.
    pub fn backend(mut self, backend: Backend) -> Self {
        self.config.backend = backend;
        self
    }

    /// Redraw policy.
    pub fn redraw_policy(mut self, policy: RedrawPolicy) -> Self {
        self.config.redraw_policy = policy;
        self
    }

    /// Requested present mode.
    pub fn present_mode(mut self, mode: PresentMode) -> Self {
        self.config.present_mode = mode;
        self
    }

    /// Whether to walk the platform fallback order.
    pub fn allow_fallback(mut self, allow: bool) -> Self {
        self.config.allow_fallback = allow;
        self
    }

    /// Sets the hook that runs before `wgpu::Instance::new` (Steam init goes
    /// here). Its return value is handed back by [`PresenterBuilder::build`].
    pub fn pre_device<G, S>(self, hook: G) -> PresenterBuilder<G>
    where
        G: FnOnce() -> S,
    {
        PresenterBuilder {
            config: self.config,
            pre_device: hook,
        }
    }

    /// The configuration this builder will use.
    pub fn current_config(&self) -> &PresenterConfig {
        &self.config
    }

    /// Runs the pre-device hook, then creates instance, surface, adapter and
    /// device on the first backend in the fallback order that works, and
    /// configures the surface at `size` (physical pixels).
    ///
    /// The hook's result is returned even when building fails, so a host can
    /// keep (or shut down) whatever the hook initialised.
    pub fn build<T: PresentTarget>(
        self,
        target: T,
        size: (u32, u32),
    ) -> (Result<Presenter, PresenterError>, R) {
        let mut trace = Vec::new();
        // ADR-014 item 3: the hook runs before any wgpu object exists.
        let hook_result = (self.pre_device)();
        trace.push(InitStep::PreDeviceHook);

        let order = if self.config.allow_fallback {
            self.config.backend.fallback_order()
        } else {
            vec![self.config.backend.resolve()]
        };

        for &backend in &order {
            let instance = create_instance(backend, &target);
            trace.push(InitStep::InstanceCreated(backend));
            let surface = match instance.create_surface(target.clone()) {
                Ok(s) => s,
                Err(e) => {
                    trace.push(InitStep::BackendUnavailable(
                        backend,
                        format!("surface: {e}"),
                    ));
                    continue;
                }
            };
            match Presenter::finish(instance, surface, size, self.config, &mut trace) {
                Ok(mut presenter) => {
                    presenter.requested = self.config.backend;
                    presenter.trace = trace;
                    return (Ok(presenter), hook_result);
                }
                Err(why) => trace.push(InitStep::BackendUnavailable(backend, why)),
            }
        }
        (
            Err(PresenterError::NoUsableBackend {
                tried: order,
                trace,
            }),
            hook_result,
        )
    }

    /// winit convenience: builds for `window` at its current inner size.
    #[cfg(feature = "winit")]
    pub fn build_for_window(
        self,
        window: &std::sync::Arc<winit::window::Window>,
    ) -> (Result<Presenter, PresenterError>, R) {
        let size = window.inner_size();
        self.build(window.clone(), (size.width, size.height))
    }
}

/// Process-wide count of `wgpu::Instance`s this crate has created.
static INSTANCES_CREATED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Test seam: how many `wgpu::Instance`s this crate has created in this
/// process. A pre-device hook that reads it can prove that no instance (and so
/// no adapter, device or surface) existed when it ran, independently of
/// [`Presenter::init_trace`]. Not part of the supported API.
#[doc(hidden)]
pub fn wgpu_instances_created() -> usize {
    INSTANCES_CREATED.load(std::sync::atomic::Ordering::SeqCst)
}

fn create_instance<T: PresentTarget>(backend: Backend, target: &T) -> wgpu::Instance {
    INSTANCES_CREATED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut backend_options = wgpu::BackendOptions::default();
    // ADR-014 item 4: never the DirectComposition swap chain.
    backend_options.dx12.presentation_system = wgpu::Dx12SwapchainKind::DxgiFromHwnd;
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backend.wgpu_backends();
    desc.backend_options = backend_options;
    if backend.resolve() == Backend::Gl {
        // GL (EGL in particular) needs the display connection at instance time.
        desc = desc.with_display_handle(Box::new(target.clone()));
    }
    wgpu::Instance::new(desc)
}

// ---------------------------------------------------------------------------
// Presenter
// ---------------------------------------------------------------------------

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var canvas_tex: texture_2d<f32>;
@group(0) @binding(1) var canvas_smp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    // One full-viewport triangle.
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var out: VsOut;
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(canvas_tex, canvas_smp, in.uv);
}
"#;

/// The blit: one pipeline (for one target format), its bind-group layout and
/// the sampler chosen by [`sampler_for`]. Shared by the on-screen presenter
/// and [`render_offscreen`], so both draw identically.
struct Blit {
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    filter: Filter,
}

impl Blit {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, filter: Filter) -> Blit {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("banquo-presenter blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("banquo-presenter canvas"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("banquo-presenter blit"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("banquo-presenter blit"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&sampler_for(filter));
        Blit {
            pipeline,
            bind_layout,
            sampler,
            filter,
        }
    }

    /// Replaces the sampler when the filter changes; returns whether it did.
    fn set_filter(&mut self, device: &wgpu::Device, filter: Filter) -> bool {
        if filter == self.filter {
            return false;
        }
        self.sampler = device.create_sampler(&sampler_for(filter));
        self.filter = filter;
        true
    }

    fn bind_group(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("banquo-presenter canvas"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Clears `target` to `background` and, when given, draws the canvas into
    /// `viewport`.
    fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        canvas: Option<(&wgpu::BindGroup, Viewport)>,
        background: wgpu::Color,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("banquo-presenter frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some((bind_group, vp)) = canvas {
            if !vp.is_empty() {
                pass.set_viewport(vp.x as f32, vp.y as f32, vp.w as f32, vp.h as f32, 0.0, 1.0);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }
}

/// Creates an sRGB RGBA8 canvas texture and uploads `rgba` into it.
fn create_canvas_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("banquo-presenter canvas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    write_canvas(queue, &texture, rgba, width, height);
    texture
}

fn write_canvas(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    rgba: &[u8],
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

/// Draws an sRGB RGBA8 canvas (`canvas.0 * canvas.1 * 4` bytes) through the
/// presenter's blit (same WGSL, same sampler rule, same [`viewport_for`]) into
/// an offscreen `Rgba8UnormSrgb` target of `target` size, and reads it back as
/// row-major RGBA8 bytes. Needs no window or surface; used for readback tests
/// and screenshots.
///
/// # Panics
///
/// If the buffer size does not match `canvas`, a dimension is zero, or the
/// device fails to map the readback buffer.
pub fn render_offscreen(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[u8],
    canvas: (u32, u32),
    target: (u32, u32),
    params: &PresentParams,
) -> Vec<u8> {
    let (cw, ch) = canvas;
    let (tw, th) = target;
    assert!(
        cw > 0 && ch > 0 && tw > 0 && th > 0,
        "sizes must be non-empty"
    );
    assert_eq!(
        rgba.len(),
        cw as usize * ch as usize * 4,
        "canvas buffer must be width * height * 4 bytes"
    );
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let blit = Blit::new(device, format, params.filter);
    let texture = create_canvas_texture(device, queue, rgba, cw, ch);
    let canvas_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = blit.bind_group(device, &canvas_view);

    let out = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("banquo-presenter offscreen"),
        size: wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out.create_view(&wgpu::TextureViewDescriptor::default());
    let row = tw * 4;
    let padded_row =
        row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("banquo-presenter readback"),
        size: u64::from(padded_row) * u64::from(th),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("banquo-presenter offscreen"),
    });
    let vp = viewport_for(canvas, target, params);
    blit.encode(
        &mut encoder,
        &out_view,
        Some((&bind_group, vp)),
        params.background,
    );
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &out,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(th),
            },
        },
        wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    rx.recv()
        .expect("map callback dropped")
        .expect("readback buffer map failed");
    let mapped = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity(row as usize * th as usize);
    for y in 0..th as usize {
        let start = y * padded_row as usize;
        pixels.extend_from_slice(&mapped[start..start + row as usize]);
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

struct Canvas {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
}

/// A configured surface plus device, driven by the host's event loop.
pub struct Presenter {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    /// The format the blit renders in: the surface format, or its sRGB view.
    render_format: wgpu::TextureFormat,
    config: PresenterConfig,
    params: PresentParams,
    requested: Backend,
    backend: Backend,
    needs_reconfigure: bool,
    blit: Blit,
    canvas: Option<Canvas>,
    trace: Vec<InitStep>,
}

impl fmt::Debug for Presenter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Presenter")
            .field("requested", &self.requested)
            .field("backend", &self.backend)
            .field("size", &self.size())
            .field("present_mode", &self.surface_config.present_mode)
            .field("redraw_policy", &self.config.redraw_policy)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl Presenter {
    /// Builds on an instance and surface the host created itself (non-winit
    /// hosts, or hosts that need `create_surface_unsafe`). The host is then
    /// responsible for having run its pre-device work before creating
    /// `instance`.
    pub fn from_parts(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        config: PresenterConfig,
    ) -> Result<Presenter, PresenterError> {
        let mut trace = vec![InitStep::ExternalInstance];
        let mut presenter = Presenter::finish(instance, surface, size, config, &mut trace)
            .map_err(PresenterError::Device)?;
        presenter.requested = config.backend;
        presenter.trace = trace;
        Ok(presenter)
    }

    fn finish(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        config: PresenterConfig,
        trace: &mut Vec<InitStep>,
    ) -> Result<Presenter, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: config.power_preference,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .map_err(|e| format!("adapter: {e}"))?;
        let info = adapter.get_info();
        let backend = Backend::from_wgpu(info.backend)
            .ok_or_else(|| format!("unsupported adapter backend {:?}", info.backend))?;
        trace.push(InitStep::AdapterSelected(backend, info.name.clone()));

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("banquo-presenter"),
            required_limits:
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| format!("device: {e}"))?;
        trace.push(InitStep::DeviceCreated);

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err("surface is not supported by this adapter".into());
        }
        // Never a linear target: the canvas is sRGB and would come out darker.
        let (format, view_format) = choose_surface_format(&caps.formats)?;
        let render_format = view_format.unwrap_or(format);
        let present_mode = choose_present_mode(config.present_mode, &caps.present_modes);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: choose_alpha_mode(&caps.alpha_modes),
            view_formats: view_format.into_iter().collect(),
        };
        let configured = size.0 > 0 && size.1 > 0;
        if configured {
            surface.configure(&device, &surface_config);
        }
        trace.push(InitStep::SurfaceConfigured(present_mode));

        let params = PresentParams::default();
        let blit = Blit::new(&device, render_format, params.filter);

        Ok(Presenter {
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_config,
            render_format,
            config,
            params,
            requested: config.backend,
            backend,
            needs_reconfigure: !configured,
            blit,
            canvas: None,
            trace: Vec::new(),
        })
    }

    /// The backend the host asked for.
    pub fn requested_backend(&self) -> Backend {
        self.requested
    }

    /// The backend actually in use (after `Auto` resolution and any fallback).
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// The adapter's description.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// The redraw policy the host should follow.
    pub fn redraw_policy(&self) -> RedrawPolicy {
        self.config.redraw_policy
    }

    /// The present mode in use (after fallback).
    pub fn present_mode(&self) -> wgpu::PresentMode {
        self.surface_config.present_mode
    }

    /// The surface format in use.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// The format the canvas is rendered in: the surface format when it is
    /// sRGB, else its sRGB view format. Always sRGB.
    pub fn render_format(&self) -> wgpu::TextureFormat {
        self.render_format
    }

    /// Current surface size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// The presentation parameters in use.
    pub fn present_params(&self) -> PresentParams {
        self.params
    }

    /// Sets the presentation parameters (scale mode, filter, background) used
    /// from the next frame on.
    pub fn set_present_params(&mut self, params: PresentParams) {
        if self.blit.set_filter(&self.device, params.filter) {
            // The sampler is part of the canvas bind group.
            if let Some(c) = self.canvas.as_mut() {
                let view = c
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                c.bind_group = self.blit.bind_group(&self.device, &view);
            }
        }
        self.params = params;
    }

    /// Where the last presented canvas lands on the current surface:
    /// [`viewport_for`] of the last canvas size and [`Presenter::size`]. `None`
    /// before any canvas was presented.
    pub fn viewport(&self) -> Option<Viewport> {
        let canvas = self.canvas.as_ref()?.size;
        Some(viewport_for(canvas, self.size(), &self.params))
    }

    /// Maps a window position (physical pixels) to the canvas pixel under it,
    /// for the last presented canvas; `None` outside the viewport or before
    /// any canvas was presented. See [`window_to_logical`].
    pub fn window_to_logical(&self, x: f64, y: f64) -> Option<(i32, i32)> {
        let canvas = self.canvas.as_ref()?.size;
        window_to_logical(self.viewport()?, canvas, x, y)
    }

    /// What happened during construction, in order. With the builder the
    /// first entry is always [`InitStep::PreDeviceHook`].
    pub fn init_trace(&self) -> &[InitStep] {
        &self.trace
    }

    /// The wgpu instance (for hosts that render their own passes).
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// The wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Records a new surface size; the surface is reconfigured before the next
    /// frame. A zero dimension (minimised) skips frames until it grows again.
    pub fn resize(&mut self, width: u32, height: u32) {
        if (width, height) != self.size() || self.needs_reconfigure {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.needs_reconfigure = true;
        }
    }

    /// Presents a solid colour frame.
    pub fn present_clear(&mut self, color: wgpu::Color) -> FrameOutcome {
        self.present_inner(None, color)
    }

    /// Uploads an sRGB RGBA8 canvas (`width * height * 4` bytes, row-major),
    /// draws it into [`Presenter::viewport`] (scale mode and filter from
    /// [`Presenter::present_params`]) over `background`, and presents.
    ///
    /// # Panics
    ///
    /// If `rgba.len() != width * height * 4` or a dimension is zero.
    pub fn present_rgba(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        background: wgpu::Color,
    ) -> FrameOutcome {
        assert!(width > 0 && height > 0, "canvas must be non-empty");
        assert_eq!(
            rgba.len(),
            width as usize * height as usize * 4,
            "canvas buffer must be width * height * 4 bytes"
        );
        self.upload_canvas(rgba, width, height);
        self.present_inner(Some((width, height)), background)
    }

    /// [`Presenter::present_rgba`] with the background from
    /// [`Presenter::present_params`].
    pub fn present_frame(&mut self, rgba: &[u8], width: u32, height: u32) -> FrameOutcome {
        let background = self.params.background;
        self.present_rgba(rgba, width, height, background)
    }

    /// winit convenience: what the host's `ControlFlow` should be for this
    /// presenter's redraw policy.
    #[cfg(feature = "winit")]
    pub fn control_flow(&self) -> winit::event_loop::ControlFlow {
        match self.config.redraw_policy {
            RedrawPolicy::Continuous => winit::event_loop::ControlFlow::Poll,
            RedrawPolicy::OnDemand => winit::event_loop::ControlFlow::Wait,
        }
    }

    /// winit convenience: call after each frame. Under
    /// [`RedrawPolicy::Continuous`] it requests the next redraw.
    #[cfg(feature = "winit")]
    pub fn after_frame(&self, window: &winit::window::Window) {
        if self.config.redraw_policy == RedrawPolicy::Continuous {
            window.request_redraw();
        }
    }

    fn upload_canvas(&mut self, rgba: &[u8], width: u32, height: u32) {
        match &self.canvas {
            Some(c) if c.size == (width, height) => {
                write_canvas(&self.queue, &c.texture, rgba, width, height);
            }
            _ => {
                let texture = create_canvas_texture(&self.device, &self.queue, rgba, width, height);
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let bind_group = self.blit.bind_group(&self.device, &view);
                self.canvas = Some(Canvas {
                    texture,
                    bind_group,
                    size: (width, height),
                });
            }
        }
    }

    fn reconfigure(&mut self) -> bool {
        let (w, h) = self.size();
        if w == 0 || h == 0 {
            return false;
        }
        self.surface.configure(&self.device, &self.surface_config);
        self.needs_reconfigure = false;
        true
    }

    fn present_inner(
        &mut self,
        canvas: Option<(u32, u32)>,
        background: wgpu::Color,
    ) -> FrameOutcome {
        if self.needs_reconfigure && !self.reconfigure() {
            return FrameOutcome::Skipped;
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                // Present this one; reconfigure before the next.
                self.needs_reconfigure = true;
                t
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                // ADR-014 item 5: reconfigure, never leave the surface unpresented.
                self.needs_reconfigure = true;
                return FrameOutcome::Reconfigured;
            }
            _ => return FrameOutcome::Skipped,
        };
        // Render through the sRGB view (the surface format itself when it is sRGB).
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("banquo-presenter frame"),
            });
        let drawn = match (canvas, self.canvas.as_ref()) {
            (Some(size), Some(c)) => {
                Some((&c.bind_group, viewport_for(size, self.size(), &self.params)))
            }
            _ => None,
        };
        self.blit.encode(&mut encoder, &view, drawn, background);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        FrameOutcome::Presented
    }
}

/// The viewport `(x, y, w, h)` that fits a `canvas`-sized image inside a
/// `surface`-sized target with its aspect ratio preserved and centred: the
/// [`ScaleMode::Fit`] case of [`viewport_for`], as floats.
pub fn letterbox(canvas: (u32, u32), surface: (u32, u32)) -> (f32, f32, f32, f32) {
    let v = viewport_for_mode(canvas, surface, ScaleMode::Fit);
    (v.x as f32, v.y as f32, v.w as f32, v.h as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_continuous_fifo_auto() {
        let c = PresenterConfig::default();
        assert_eq!(c.redraw_policy, RedrawPolicy::Continuous);
        assert_eq!(c.present_mode, PresentMode::Fifo);
        assert_eq!(c.backend, Backend::Auto);
        assert!(c.allow_fallback);
        assert_eq!(RedrawPolicy::default(), RedrawPolicy::Continuous);
        assert_eq!(PresentMode::default(), PresentMode::Fifo);
    }

    #[test]
    fn backend_parse_round_trips() {
        for b in Backend::ALL {
            assert_eq!(Backend::parse(b.name()), Some(b));
        }
        assert_eq!(Backend::parse("VK"), Some(Backend::Vulkan));
        assert_eq!(Backend::parse("OpenGL"), Some(Backend::Gl));
        assert_eq!(Backend::parse("metal"), None);
    }

    #[test]
    fn auto_resolves_to_platform_default() {
        let expected = if cfg!(windows) {
            Backend::Dx12
        } else {
            Backend::Vulkan
        };
        assert_eq!(Backend::Auto.resolve(), expected);
        assert_eq!(Backend::Gl.resolve(), Backend::Gl);
    }

    #[test]
    fn every_backend_is_exactly_one_wgpu_flag() {
        for b in Backend::ALL {
            assert_eq!(b.wgpu_backends().bits().count_ones(), 1, "{b}");
        }
    }

    #[test]
    fn fallback_order_starts_with_request_and_has_no_duplicates() {
        for b in Backend::ALL {
            let order = b.fallback_order();
            assert_eq!(order[0], b.resolve());
            let mut dedup = order.clone();
            dedup.dedup();
            assert_eq!(dedup.len(), order.len());
            assert!(!order.contains(&Backend::Auto));
        }
        if cfg!(windows) {
            assert_eq!(
                Backend::Dx12.fallback_order(),
                vec![Backend::Dx12, Backend::Vulkan, Backend::Gl]
            );
            assert_eq!(
                Backend::Gl.fallback_order(),
                vec![Backend::Gl, Backend::Dx12, Backend::Vulkan]
            );
        } else {
            assert_eq!(
                Backend::Vulkan.fallback_order(),
                vec![Backend::Vulkan, Backend::Gl]
            );
        }
    }

    #[test]
    fn present_mode_falls_back_to_fifo() {
        let fifo_only = [wgpu::PresentMode::Fifo];
        assert_eq!(
            choose_present_mode(PresentMode::Mailbox, &fifo_only),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            choose_present_mode(PresentMode::Immediate, &fifo_only),
            wgpu::PresentMode::Fifo
        );
        let all = [
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
        ];
        assert_eq!(
            choose_present_mode(PresentMode::Mailbox, &all),
            wgpu::PresentMode::Mailbox
        );
        assert_eq!(
            choose_present_mode(PresentMode::Fifo, &all),
            wgpu::PresentMode::Fifo
        );
    }

    #[test]
    fn uncapped_walks_mailbox_immediate_fifo() {
        use wgpu::PresentMode as M;
        let pick = |s: &[M]| choose_present_mode(PresentMode::Uncapped, s);
        assert_eq!(pick(&[M::Fifo, M::Immediate, M::Mailbox]), M::Mailbox);
        assert_eq!(pick(&[M::Fifo, M::Immediate]), M::Immediate);
        assert_eq!(pick(&[M::Fifo]), M::Fifo);
        assert_eq!(pick(&[]), M::Fifo);
    }

    #[test]
    fn alpha_prefers_opaque() {
        use wgpu::CompositeAlphaMode as A;
        assert_eq!(choose_alpha_mode(&[A::PreMultiplied, A::Opaque]), A::Opaque);
        assert_eq!(choose_alpha_mode(&[A::Inherit]), A::Inherit);
    }

    #[test]
    fn letterbox_preserves_aspect_and_centres() {
        assert_eq!(
            letterbox((320, 180), (1280, 720)),
            (0.0, 0.0, 1280.0, 720.0)
        );
        // Taller window: bars top and bottom.
        assert_eq!(
            letterbox((320, 180), (1280, 1000)),
            (0.0, 140.0, 1280.0, 720.0)
        );
        // Wider window: bars left and right.
        assert_eq!(
            letterbox((320, 180), (2000, 720)),
            (360.0, 0.0, 1280.0, 720.0)
        );
        assert_eq!(letterbox((320, 180), (0, 720)), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn builder_carries_policy_and_hook_type() {
        let b = PresenterBuilder::new()
            .backend(Backend::Vulkan)
            .present_mode(PresentMode::Mailbox)
            .pre_device(|| 480u32);
        assert_eq!(b.current_config().backend, Backend::Vulkan);
        assert_eq!(b.current_config().present_mode, PresentMode::Mailbox);
        assert_eq!(b.current_config().redraw_policy, RedrawPolicy::Continuous);
    }
}
