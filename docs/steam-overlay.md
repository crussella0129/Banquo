# The Steam overlay on a wgpu surface

**Status:** research note, 2026-11-07. Written for WordCrack Sprint 1 task T-216 on the
`feat/embeddable-presenter` branch. It is the evidence behind ADR-014 in `decisions.md`.
Nothing here has been run on hardware yet: the verdicts below are expectations, and the
spike (`examples/steam_overlay_spike`, WordCrack T-217) and the owner's check (T-218)
confirm or overturn them.

**Legend.** Anything not read directly in a cited source or in pinned crate source is
marked **(inferred)**.

**Versions this note is pinned to.** wgpu / wgpu-hal / wgpu-types 29.0.3, winit 0.30.13,
eframe / egui / egui-wgpu 0.34.3 (`Cargo.lock`). Source lines quoted from wgpu were read
at tag `v29.0.3`; the egui lines at tag `0.34.3`.

---

## 1. What the overlay needs, in general

Valve's overlay page [S1] states three requirements that apply to every backend:

1. **Init order.** Call `SteamAPI_Init` *before* the graphics device is created, "otherwise
   it won't be able to hook the device creation" [S1]. Community reports agree: Bevy's
   overlay flickered until Steam was initialised before Bevy's `RenderPlugin` created the
   wgpu device [S9]; Unreal [S16] and MonoGame [S17] threads report the same symptom.
2. **Launch through Steam.** The overlay hooks "games launched from Steam"; if it does not
   appear, launch from the Steam client or call `SteamAPI_RestartAppIfNecessary` [S1].
   A process started outside Steam may not be hooked [S18]; a Bevy user's overlay only
   worked once the game was launched from Steam [S11].
3. **Continuous presentation.** The overlay "requires a game consistently render frames,
   not pausing rendering or rendering only part of the screen based on dirty rects" [S1].
   It draws into the game's own frames, so a game that presents only when something
   changes leaves the overlay frozen or invisible [S18].

Two more points from [S1]: the overlay is disabled for apps of type "Software", and it
does not run in software-rasterised games. Presenting a CPU-drawn canvas through GDI
(for example `softbuffer`) therefore does **not** count as a hookable present (inferred).

`GameOverlayActivated_t` is posted when the overlay opens and closes [S1]. A host may
pause on it, but it must **keep presenting** while the overlay is open.

### 1.1 What this means for wgpu specifically

- **"Before device creation" means before `wgpu::Instance::new`.** wgpu creates
  backend objects at instance creation, not only at `request_device`:
  - the DX12 backend creates its DXGI factory there;
  - the Vulkan backend calls `vkCreateInstance`, which is when the Vulkan loader decides
    which implicit layers (including Steam's) to load;
  - the GL backend on Windows spawns a "wgpu-hal WGL Instance Thread" that owns a hidden
    window and creates the WGL context [S6].

  So the Steam init hook must run before the `Instance` exists, not merely before
  `request_device` (inferred from the above).
- **Create the instance with only the selected backend.** The usual default,
  `Backends::PRIMARY | Backends::GL` (egui-wgpu's default [S8]), initialises Vulkan *and*
  DX12 *and* WGL in one process. That loads the Steam Vulkan layer and a GL context into a
  DX12 game, which gives the overlay several devices to choose from (inferred). One
  backend per instance avoids this.
- **On-demand repaint freezes the overlay.** eframe runs winit with `ControlFlow::Wait` /
  `WaitUntil` and paints only when a repaint is requested [S7]. An eframe app that does not
  call `request_repaint()` every frame stops presenting while idle, which violates
  requirement 3. Banquo the terminal happens to request a repaint every frame
  (`src/app.rs:1220`), so it presents continuously, but that is a side effect, not a
  policy.
- **The host must own the loop.** Steam callbacks (`SteamAPI_RunCallbacks`), gamepad
  polling and fixed-step logic belong in the game's loop. eframe owns the winit loop
  through `run_native`. It is still possible to call `SteamAPI_Init` in `main()` before
  `run_native`, so eframe does not make the init order impossible. But eframe's
  repaint-on-demand model and its default backend set work against requirement 3 and
  §1.1's single-backend rule.

### 1.2 How Banquo presents today

`src/main.rs` calls `eframe::run_native` with `Renderer::Wgpu` and a **transparent,
frameless** viewport. eframe 0.34.3 / egui-wgpu 0.34.3 then:
- picks backends from `WGPU_BACKEND` or `PRIMARY | GL` [S8];
- presents with `PresentMode::AutoVsync` [S8];
- owns the winit 0.30 event loop and sleeps between requested repaints [S7].

`src/render/` holds an unwired `egui_wgpu::CallbackTrait` scaffold that draws inside
egui's pass. There is no Steam code, and there should not be (ADR-002, design §VII).
`os::ensure_detached()` (ADR-012) re-spawns the process on Windows release builds. A
Steam-launched process that re-spawns itself is recorded by Steam as having exited, so no
Steam-launched host may run that guard (inferred; Steam tracks the launched process).

---

## 2. Platform matrix

### 2.1 Windows, DX12

| Aspect | Finding |
|---|---|
| **Attach mechanism** | Steam injects `GameOverlayRenderer64.dll` into processes it launches; `steam_api64.dll` loads it when `SteamAPI_Init` runs (inferred from [S1], [S18]). For D3D12 the overlay hooks DXGI swap-chain creation and `IDXGISwapChain::Present`, and needs the swap chain's `ID3D12CommandQueue` to submit its own draw. A swap chain does not expose its queue, so DX12 overlays commonly hook `ID3D12CommandQueue::ExecuteCommandLists` or `CreateSwapChainForHwnd` to capture it [S19], [S20]. The Steam-specific hook set is not published (inferred). wgpu 29 presents with a `DXGI_SWAP_EFFECT_FLIP_DISCARD` swap chain created by `CreateSwapChainForHwnd` (default `Dx12SwapchainKind::DxgiFromHwnd`) and calls `IDXGISwapChain::Present` [S4], [S5]. |
| **Required init order** | `SteamAPI_Init` → `wgpu::Instance::new(Backends::DX12)` (creates the DXGI factory) → `create_surface` → `request_adapter` / `request_device` (creates the D3D12 device and the direct queue) → `surface.configure` (creates the swap chain) [S1], [S4]. |
| **Presentation requirement** | Present every frame (continuous redraw, target ≥ 60 FPS). Present mode: `Fifo` (sync interval 1) is the safe default. `Immediate` maps to sync interval 0 with `DXGI_PRESENT_ALLOW_TEARING` [S5]; the overlay draws before the present either way (inferred). Use `DxgiFromHwnd`, not `DxgiFromVisual`: the DirectComposition path exists for transparent windows [S4], and third-party hooks are commonly written against plain HWND swap chains (inferred). The presenter window must therefore be opaque, unlike Banquo's transparent terminal window. |
| **Known issues** | (1) Steam initialised after the wgpu device: the overlay flickers when opened (Bevy 0.12 era; fixed by initialising Steam before `RenderPlugin`) [S9], [S10]. (2) winit's Windows "main thread" check is wrong when winit is built as a dylib and the process is started by DLL injection, which is how Steam starts a game [S12]. Not hit by a statically linked binary. (3) Not launched through Steam: no overlay [S1], [S11]. (4) Other injectors (RTSS, capture tools, anti-cheat) can block or race the hook [S18]. No wgpu issue specific to the Steam overlay on DX12 was found (searched gfx-rs/wgpu, rust-windowing/winit, emilk/egui and bevyengine/bevy on 2026-11-07). |
| **Verdict** | **works** (expected). DX12 is a first-class wgpu backend on Windows [S13]. wgpu presents through a standard flip-model HWND swap chain, which is the case Steam's D3D12 support targets [S1]. The only failure reported on a wgpu stack (flicker) was an init-order bug the presenter design removes [S9]. Recommended Windows default. |

### 2.2 Windows, Vulkan

| Aspect | Finding |
|---|---|
| **Attach mechanism** | A Vulkan **implicit layer**, `VK_LAYER_VALVE_steam_overlay`. Its manifest, `SteamOverlayVulkanLayer64.json` (library `SteamOverlayVulkanLayer64.dll`), is registered under `SOFTWARE\Khronos\Vulkan\ImplicitLayers` [S14], [S15]. The layer is **disabled by default**. It loads only when `ENABLE_VK_LAYER_VALVE_steam_overlay_1` is set, and `DISABLE_VK_LAYER_VALVE_steam_overlay_1` turns it off [S14]. Steam sets the enable variable for the processes it launches (inferred from [S14] and [S1]'s launch requirement). The layer intercepts swap-chain creation and `vkQueuePresentKHR` on wgpu's `VK_KHR_win32_surface` swap chain (inferred). |
| **Required init order** | Launched by Steam, so the enable variable is in the environment → `SteamAPI_Init` → `wgpu::Instance::new(Backends::VULKAN)`. The loader resolves implicit layers at `vkCreateInstance`, so an instance created before Steam is ready can carry an inert layer → `request_device` → `surface.configure` [S1], [S14]. Running the exe directly, with `steam_appid.txt`, leaves the layer disabled even after `SteamAPI_Init` succeeds (inferred from [S14]). |
| **Presentation requirement** | Continuous `vkQueuePresentKHR` every frame. `Fifo` default. `Mailbox` where the driver offers it. Avoid presenting from a compute-only queue: Mesa notes that overlays such as Steam's need presentation from a graphics queue, and wgpu presents from its single graphics queue (inferred) [S21]. |
| **Known issues** | (1) The layer is gated on an environment variable, so a manual launch (debugger, terminal) shows no overlay while DX12 might [S14]. (2) Steam client regressions in the Vulkan overlay layer: Valve fixed a "crash when Vulkan games recreate their swapchain" in the 2026-10-01 beta (reported on Linux; wgpu recreates the swap chain on every resize) [S22], and fixed periodic lag in Vulkan games with the overlay disabled in June 2025 [S23]. (3) Same init-order and injection issues as DX12 [S9], [S12]. (4) Anything that removes or zeroes Steam's `ImplicitLayers` registry value removes the overlay layer, because the loader finds the layer only through that key [S15] (inferred). |
| **Verdict** | **risk**. The mechanism is supported [S1], [S13]. But it depends on Steam's launch environment, which the developer's own test loop does not have, and the Vulkan layer has had recent swap-chain regressions [S22]. Keep it as a selectable fallback, not the Windows default. |

### 2.3 Windows, OpenGL

| Aspect | Finding |
|---|---|
| **Attach mechanism** | `GameOverlayRenderer64.dll` hooks the WGL swap path (`wglSwapBuffers` / GDI `SwapBuffers`) and draws with GL on the current context (inferred: [S1] lists OpenGL; the hook point is not published). wgpu's GL backend on Windows uses **WGL by default**, with ANGLE opt-in [S13]. At instance creation it spawns a thread that owns a hidden window and creates the WGL context [S6]. At present time it gets the target window's DC, blits its internal framebuffer to the default framebuffer, and calls GDI `SwapBuffers(dc)` [S6]. GDI `SwapBuffers` routes into `opengl32`, so a `wglSwapBuffers` hook should still see it (inferred). |
| **Required init order** | `SteamAPI_Init` **before `wgpu::Instance::new(Backends::GL)`**, because the WGL context is created at instance creation on the helper thread [S6], and [S1] requires Steam before the context exists → `create_surface` → `request_device` → `configure`. |
| **Presentation requirement** | Continuous `SwapBuffers`. Present mode maps to `WGL_EXT_swap_control`: `Fifo` = swap interval 1; anything else = 0 [S6]. Expect the overlay to share GL state with wgpu's context; wgpu rebinds what it needs each frame (inferred). |
| **Known issues** | (1) GL is a **downlevel / best-effort** wgpu backend on Windows (GL 3.3+, or GLES 3.0 through ANGLE) [S13]. (2) The context lives on a hidden window owned by another thread and is made current against the real window's DC at present [S6]. An overlay that assumes the context was created for the presented window may mis-size or skip it (inferred). (3) Early-init overlay hooks generally need Steam before context creation [S1], [S16], [S17]. No Steam report specific to wgpu on GL was found. |
| **Verdict** | **risk**. Supported by Steam in principle [S1], but wgpu's WGL path is an unusual shape for a hooked GL game (hidden-window context, GDI swap) and is the least-tested wgpu backend on Windows [S13]. It is the last-resort Windows backend, kept selectable for the spike and the escape hatch. |

### 2.4 Steam Deck (SteamOS, gamescope), Vulkan

| Aspect | Finding |
|---|---|
| **Attach mechanism** | Two layers act together (partly inferred). (a) **In-process:** Steam installs `VK_LAYER_VALVE_steam_overlay_32/_64` manifests in `~/.local/share/vulkan/implicit_layer.d/steamoverlay_*.json` [S24] and preloads `gameoverlayrenderer.so` [S11b], [S25]. The Vulkan layer hooks only `VK_KHR_xcb_surface` and `VK_KHR_xlib_surface`, **not `VK_KHR_wayland_surface`**, and the GL hook covers GLX but not EGL [S2]. (b) **Compositor:** in Game Mode, gamescope composites windows that carry the `STEAM_OVERLAY` or `GAMESCOPE_EXTERNAL_OVERLAY` X property as overlay layers above the game [S26]. Game frames reach gamescope through Xwayland [S27]. The Steam UI that Steam and Shift+Tab open in Game Mode is such a composited Steam window (inferred from [S26]). |
| **Required init order** | Launched by Steam (in Game Mode: a Steam title, or a "non-Steam game" added in Desktop Mode, then started from Game Mode), so the preload and layer environment are present → `SteamAPI_Init` (with `libsteam_api.so` beside the binary) → winit on **X11** → `wgpu::Instance::new(Backends::VULKAN)` → device → `configure` [S1], [S2], [S11b]. |
| **Presentation requirement** | Continuous presents through the X11 (Xwayland) WSI; `Fifo` (gamescope paces the composite). winit 0.30 picks **Wayland first** whenever `WAYLAND_DISPLAY` or `WAYLAND_SOCKET` is set [S3]. The host must force X11 (winit `EventLoopBuilderExtX11::with_x11()`), or the in-process overlay has no surface it can hook [S2]. The Wayland-native case loses the overlay on desktop Linux [S2], [S28]. |
| **Known issues** | (1) **Bevy/wgpu, native Linux, Vulkan:** the overlay was invisible (open event received, input rerouted, FPS counter gone) on three GPUs, X11 and Xwayland, Bevy 0.15-0.17, while the same game showed it on Windows and under Proton. Open since 2025-11-09 [S11b]. This is the closest known report to this stack. (2) Wayland-native apps get no overlay [S2], [S28]. (3) Nested gamescope (desktop) can apply the overlay preload to gamescope instead of the game [S25]. (4) winit X11: the overlay does not receive mouse motion or buttons because of winit's XInput2 event mask; open since 2019 [S29]. (5) Steam overlay Vulkan layer crashed games that recreate their swap chain (fixed in the 2026-10-01 beta) [S22]. (6) `mesa_no_error` in `~/.drirc` makes the overlay a grey rectangle [S30]. |
| **Verdict** | **risk**. Game Mode composites the Steam UI in gamescope, which should make the Deck less dependent on the in-process hook than desktop Linux (inferred from [S26]). But the only wgpu-on-native-Linux report found shows an invisible overlay [S11b]. winit defaults to Wayland where it can [S3]. The Deck check (T-218) decides this row. |

### 2.5 Summary

| Platform | Verdict | Default? |
|---|---|---|
| Windows DX12 | works (expected) | **Yes**, Windows default |
| Windows Vulkan | risk | fallback 1 on Windows |
| Windows OpenGL | risk | fallback 2 on Windows |
| Steam Deck Vulkan | risk | the only Deck option (GL as an untested escape hatch) |

---

## 3. Requirements this puts on a Banquo presenter

1. **The host owns the event loop.** The presenter is a library; no `run_native`.
2. **A pre-device hook** runs before `wgpu::Instance::new`. Steam stays in the host; Banquo
   crates never link Steamworks.
3. **One backend per instance:** `Dx12 | Vulkan | Gl`, selected by the host (CLI or
   config), with a per-platform default and fallback order.
4. **Continuous redraw is the default policy.** On-demand redraw exists only as an explicit
   opt-out for non-Steam tools.
5. **Present mode** defaults to `Fifo`. `Mailbox` / `Immediate` are opt-in and fall back to
   `Fifo` when the surface does not support them. Lost and outdated surfaces are
   reconfigured, never skipped for more than a frame.
6. **DX12:** always `Dx12SwapchainKind::DxgiFromHwnd`, with an opaque surface.
7. **Linux:** the host forces winit's X11 backend when it may run under Steam.
8. **No `ensure_detached()`** in any Steam-launched host.

---

## 4. Sources

Accessed 2026-11-07.

- [S1] Steamworks documentation, *Steam Overlay*: https://partner.steamgames.com/doc/features/overlay
- [S2] ValveSoftware/steam-for-linux #8020, *Add Wayland support to the Steam overlay* (open): https://github.com/ValveSoftware/steam-for-linux/issues/8020
- [S3] winit 0.30.13, Linux backend selection (Wayland first): https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/linux/mod.rs
- [S4] wgpu-types 29.0.3, `Dx12SwapchainKind` (`DxgiFromHwnd` default; `DxgiFromVisual` for transparency): https://github.com/gfx-rs/wgpu/blob/v29.0.3/wgpu-types/src/backend.rs
- [S5] wgpu-hal 29.0.3, DX12 swap chain (`FLIP_DISCARD`, `ALLOW_TEARING` for `Immediate`, `Present`): https://github.com/gfx-rs/wgpu/blob/v29.0.3/wgpu-hal/src/dx12/mod.rs
- [S6] wgpu-hal 29.0.3, WGL backend (hidden-window instance thread, `SwapBuffers`, swap interval): https://github.com/gfx-rs/wgpu/blob/v29.0.3/wgpu-hal/src/gles/wgl.rs
- [S7] eframe 0.34.3, native run loop (`ControlFlow::Wait` / `WaitUntil`): https://github.com/emilk/egui/blob/0.34.3/crates/eframe/src/native/run.rs
- [S8] egui-wgpu 0.34.3, defaults (`AutoVsync`; `PRIMARY | GL`): https://github.com/emilk/egui/blob/0.34.3/crates/egui-wgpu/src/lib.rs and https://github.com/emilk/egui/blob/0.34.3/crates/egui-wgpu/src/setup.rs
- [S9] HouraiTeahouse/bevy_steamworks #31, *Steam overlay is flickering* (init order vs `RenderPlugin`): https://github.com/HouraiTeahouse/bevy_steamworks/issues/31
- [S10] bevyengine/bevy #1255, *Order independence of App construction*: https://github.com/bevyengine/bevy/issues/1255
- [S11] HouraiTeahouse/bevy_steamworks #4, *Steam Overlay Not Opening In Bevy App* (works when launched from Steam): https://github.com/HouraiTeahouse/bevy_steamworks/issues/4
- [S11b] HouraiTeahouse/bevy_steamworks #71, *Steam Overlay invisible in Bevy App with Vulkan on Linux native*: https://github.com/HouraiTeahouse/bevy_steamworks/issues/71
- [S12] rust-windowing/winit #3999, main-thread check vs DLL injection: https://github.com/rust-windowing/winit/issues/3999
- [S13] gfx-rs/wgpu README, supported platforms (Windows: DX12 and Vulkan first-class, GL best-effort; WGL default, ANGLE opt-in): https://github.com/gfx-rs/wgpu
- [S14] LunarG vkvia report showing `VK_LAYER_VALVE_steam_overlay` (manifest, enable/disable variables, disabled by default): https://vulkan.lunarg.com/issue/file/1161/upload/1685021852_vkvia.html
- [S15] Windows Vulkan loader log (gist) showing the loader reading `SOFTWARE\Khronos\Vulkan\ImplicitLayers` and finding `SteamOverlayVulkanLayer64.json`: https://gist.github.com/tuket/e97daf8100bad8f4ac3b8b9b6688b904
- [S16] Unreal Engine forums, *How to call SteamAPI_Init before D3D*: https://forums.unrealengine.com/t/how-to-call-steamapi_init-before-d3d/121170
- [S17] MonoGame community, *Steam Overlay not showing with windows assembly of MonoGame 3.6*: https://community.monogame.net/t/steam-overlay-not-showing-with-windows-assembly-of-monogame-3-6/8926
- [S18] Bugnet, *Fix Steam overlay not working in your game* (secondary): https://bugnet.io/blog/fix-steam-overlay-not-working-in-your-game
- [S19] Stack Overflow 36286425, *How do I get the DirectX 12 command queue from a swap chain*: https://stackoverflow.com/questions/36286425/how-do-i-get-the-directx-12-command-queue-from-a-swap-chain
- [S20] asdf-overlay (Rust) DX12 hook source (`ExecuteCommandLists` capture): https://docs.rs/crate/asdf-overlay/latest/source/src/hook/dx/dx12.rs
- [S21] Mesa #946, Vulkan overlay layer and async compute: https://gitlab.freedesktop.org/mesa/mesa/-/issues/946
- [S22] mixed-news, Steam beta overlay crashed Vulkan games on swap-chain recreation (fixed 2026-10-01): https://mixed-news.com/de/steams-beta-overlay-lies-vulkan-spiele-absturzen-und-valves-eigene-notiz-dokumentiert-die-losung/
- [S23] GamingOnLinux, *Valve fix lag and freezing in games with the Steam overlay disabled* (June 2025): https://www.gamingonlinux.com/2025/06/valve-fix-lag-and-freezing-in-games-with-the-steam-overlay-disabled/page=1
- [S24] ValveSoftware/steam-runtime #564 (Steam overlay Vulkan layer manifests on Linux): https://github.com/ValveSoftware/steam-runtime/issues/564
- [S25] ValveSoftware/gamescope #835, *Gamescope breaks Steam Overlay* (preload applied to gamescope): https://github.com/ValveSoftware/gamescope/issues/835
- [S26] gamescope `steamcompmgr.cpp` (`STEAM_OVERLAY` / `GAMESCOPE_EXTERNAL_OVERLAY` windows composited as overlay layers), master at 89b9e6e: https://github.com/ValveSoftware/gamescope/blob/89b9e6ee61513767f4771e0022b669a48efb5780/src/steamcompmgr.cpp
- [S27] gamescope README (game frames via Wayland by way of Xwayland): https://github.com/ValveSoftware/gamescope
- [S28] Factorio forums, *Steam Overlay is not working with Wayland video driver*: https://forums.factorio.com/130412
- [S29] rust-windowing/winit #843, *Steam overlay unresponsive on Linux* (XInput2 mask; open): https://github.com/rust-windowing/winit/issues/843
- [S30] ValveSoftware/steam-for-linux #6691, overlay broken with `mesa_no_error`: https://github.com/ValveSoftware/steam-for-linux/issues/6691
