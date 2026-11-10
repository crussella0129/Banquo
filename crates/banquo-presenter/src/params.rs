//! Presentation parameters: how a canvas is scaled into the surface, how it is
//! filtered, what fills the bars, and the matching coordinate mapping.
//!
//! Everything here is pure arithmetic (no device needed), so hosts and tests
//! can predict exactly where a canvas lands.

/// How a canvas is scaled into the surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    /// Whole-number scaling, centred: scale `max(1, min(sw / cw, sh / ch))`
    /// (integer division). A surface smaller than the canvas keeps scale 1
    /// and crops. This is pygame `SCALED` in a window. Default.
    #[default]
    Integer,
    /// Aspect-preserving fractional scaling, centred: the limiting axis fills
    /// the surface exactly, the other is floored. This is SDL 2's
    /// `SDL_RenderSetLogicalSize` letterbox (and pygame `SCALED` fullscreen).
    Fit,
}

/// Texture filter used when the canvas is scaled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Nearest texel: crisp pixels. Default.
    #[default]
    Nearest,
    /// Bilinear.
    Linear,
}

/// Presentation parameters ([`crate::Presenter::set_present_params`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentParams {
    /// Scaling rule (default [`ScaleMode::Integer`]).
    pub scale: ScaleMode,
    /// Texture filter (default [`Filter::Nearest`]).
    pub filter: Filter,
    /// Colour of the bars around the canvas (default opaque black). Like any
    /// `wgpu::Color` clear value it is in linear space.
    pub background: wgpu::Color,
}

impl Default for PresentParams {
    fn default() -> Self {
        Self {
            scale: ScaleMode::Integer,
            filter: Filter::Nearest,
            background: wgpu::Color::BLACK,
        }
    }
}

/// Where the canvas is drawn on the surface, in surface pixels. `x`/`y` can be
/// negative when an integer-scaled canvas is larger than the surface (cropped).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Viewport {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl Viewport {
    /// True when the viewport covers no pixel.
    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// Maps a surface (window) position to the canvas pixel under it; see
    /// [`window_to_logical`].
    pub fn window_to_logical(&self, canvas: (u32, u32), x: f64, y: f64) -> Option<(i32, i32)> {
        window_to_logical(*self, canvas, x, y)
    }
}

/// Floor of `a / 2` for possibly negative `a`, as an `i32`.
fn floor_half(a: i64) -> i32 {
    a.div_euclid(2) as i32
}

/// The viewport a `canvas`-sized image occupies on a `surface`-sized target
/// under `params.scale`. Offsets are floor-centred. A zero-sized canvas or
/// surface gives an empty viewport at the origin.
///
/// - [`ScaleMode::Integer`]: scale `s = max(1, min(sw / cw, sh / ch))` with
///   integer division; `w = cw * s`, `h = ch * s`.
/// - [`ScaleMode::Fit`]: when the canvas is relatively wider than the surface
///   (`cw * sh > sw * ch`) the width fills (`w = sw`, `h = floor(ch * sw / cw)`),
///   otherwise the height fills (`h = sh`, `w = floor(cw * sh / ch)`). This is
///   SDL 2.28's `UpdateLogicalSize` letterbox, in exact integer arithmetic.
pub fn viewport_for(canvas: (u32, u32), surface: (u32, u32), params: &PresentParams) -> Viewport {
    viewport_for_mode(canvas, surface, params.scale)
}

/// [`viewport_for`] with only the scale mode.
pub fn viewport_for_mode(canvas: (u32, u32), surface: (u32, u32), mode: ScaleMode) -> Viewport {
    let (cw, ch) = (u64::from(canvas.0), u64::from(canvas.1));
    let (sw, sh) = (u64::from(surface.0), u64::from(surface.1));
    if cw == 0 || ch == 0 || sw == 0 || sh == 0 {
        return Viewport::default();
    }
    let (w, h) = match mode {
        ScaleMode::Integer => {
            let s = (sw / cw).min(sh / ch).max(1);
            (cw * s, ch * s)
        }
        ScaleMode::Fit => {
            if cw * sh > sw * ch {
                (sw, ch * sw / cw)
            } else {
                (cw * sh / ch, sh)
            }
        }
    };
    let clamp = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
    Viewport {
        x: floor_half(sw as i64 - w as i64),
        y: floor_half(sh as i64 - h as i64),
        w: clamp(w),
        h: clamp(h),
    }
}

/// Maps a surface (window) position `(x, y)` to the canvas pixel under it:
/// `Some(((x - vx) * cw / vw) as i32, ((y - vy) * ch / vh) as i32)` when the
/// point lies inside `viewport` (left/top edges inside, right/bottom outside),
/// else `None`.
pub fn window_to_logical(
    viewport: Viewport,
    canvas: (u32, u32),
    x: f64,
    y: f64,
) -> Option<(i32, i32)> {
    if viewport.is_empty() || canvas.0 == 0 || canvas.1 == 0 {
        return None;
    }
    let (vx, vy) = (f64::from(viewport.x), f64::from(viewport.y));
    let (vw, vh) = (f64::from(viewport.w), f64::from(viewport.h));
    if !(x >= vx && x < vx + vw && y >= vy && y < vy + vh) {
        return None;
    }
    let lx = ((x - vx) * f64::from(canvas.0) / vw) as i32;
    let ly = ((y - vy) * f64::from(canvas.1) / vh) as i32;
    Some((lx, ly))
}

/// The sampler the blit uses for `filter`: `mag_filter` and `min_filter` both
/// follow it; addressing clamps to the edge.
pub fn sampler_for(filter: Filter) -> wgpu::SamplerDescriptor<'static> {
    let mode = match filter {
        Filter::Nearest => wgpu::FilterMode::Nearest,
        Filter::Linear => wgpu::FilterMode::Linear,
    };
    wgpu::SamplerDescriptor {
        label: Some("banquo-presenter canvas"),
        mag_filter: mode,
        min_filter: mode,
        ..Default::default()
    }
}

/// Picks the surface format so that the blit always writes through an sRGB
/// view (the canvas is sRGB; a linear target would darken it).
///
/// Returns `(surface format, view format)`:
/// - the first sRGB format offered: `(it, None)`;
/// - else the first format with an sRGB twin (`Bgra8Unorm`, `Rgba8Unorm`):
///   `(it, Some(twin))`, and the twin goes in
///   `SurfaceConfiguration::view_formats` and is the render target format;
/// - else an error (the backend is reported unavailable).
pub fn choose_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Result<(wgpu::TextureFormat, Option<wgpu::TextureFormat>), String> {
    if let Some(f) = formats.iter().copied().find(|f| f.is_srgb()) {
        return Ok((f, None));
    }
    if let Some(f) = formats.iter().copied().find(|f| f.add_srgb_suffix() != *f) {
        return Ok((f, Some(f.add_srgb_suffix())));
    }
    Err(format!(
        "no sRGB-capable surface format (offered {formats:?})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::TextureFormat as T;

    fn vp(x: i32, y: i32, w: u32, h: u32) -> Viewport {
        Viewport { x, y, w, h }
    }

    fn params(scale: ScaleMode) -> PresentParams {
        PresentParams {
            scale,
            ..PresentParams::default()
        }
    }

    // Measured with SDL 2.28.4 RenderSetLogicalSize (pygame 2.6.1 SCALED and
    // the SDL renderer's logical size), window pixels from a capture.
    // (canvas, surface, viewport x, y, w, h)
    type Vector = ((u32, u32), (u32, u32), (i32, i32, u32, u32));

    /// Windowed SCALED content rects: whole-number scaling.
    const MEASURED_INTEGER: [Vector; 3] = [
        ((900, 700), (1280, 800), (190, 50, 900, 700)),
        ((900, 700), (1366, 768), (233, 34, 900, 700)),
        ((900, 700), (1600, 900), (350, 100, 900, 700)),
    ];

    /// Logical-size viewports with integer scale off.
    const MEASURED_FIT: [Vector; 16] = [
        ((900, 700), (1280, 800), (126, 0, 1028, 800)),
        ((900, 700), (1366, 768), (189, 0, 987, 768)),
        ((900, 700), (1920, 1080), (266, 0, 1388, 1080)),
        ((900, 700), (2560, 1080), (586, 0, 1388, 1080)),
        ((1120, 700), (1280, 800), (0, 0, 1280, 800)),
        ((1120, 700), (1366, 768), (69, 0, 1228, 768)),
        ((1120, 700), (1920, 1080), (96, 0, 1728, 1080)),
        ((1120, 700), (2560, 1080), (416, 0, 1728, 1080)),
        ((1244, 700), (1280, 800), (0, 40, 1280, 720)),
        ((1244, 700), (1366, 768), (1, 0, 1364, 768)),
        ((1244, 700), (1920, 1080), (0, 0, 1919, 1080)),
        ((1244, 700), (2560, 1080), (320, 0, 1919, 1080)),
        ((1680, 700), (1280, 800), (0, 133, 1280, 533)),
        ((1680, 700), (1366, 768), (0, 99, 1366, 569)),
        ((1680, 700), (1920, 1080), (0, 140, 1920, 800)),
        ((1680, 700), (2560, 1080), (0, 7, 2560, 1066)),
    ];

    /// Fullscreen SCALED content rect at the desktop size (fractional scale).
    const MEASURED_FULLSCREEN: [Vector; 1] = [((900, 700), (2560, 1440), (354, 0, 1851, 1440))];

    fn matches(mode: ScaleMode, vectors: &[Vector]) -> usize {
        vectors
            .iter()
            .filter(|(c, s, (x, y, w, h))| {
                let got = viewport_for(*c, *s, &params(mode));
                let ok = got == vp(*x, *y, *w, *h);
                if !ok {
                    eprintln!("{mode:?} {c:?} in {s:?}: got {got:?}, measured {x},{y},{w},{h}");
                }
                ok
            })
            .count()
    }

    #[test]
    fn default_params_are_integer_nearest_black() {
        let p = PresentParams::default();
        assert_eq!(p.scale, ScaleMode::Integer);
        assert_eq!(p.filter, Filter::Nearest);
        assert_eq!(p.background, wgpu::Color::BLACK);
        assert_eq!(p.background.a, 1.0);
    }

    #[test]
    fn integer_viewport_spec_vectors() {
        let p = params(ScaleMode::Integer);
        assert_eq!(
            viewport_for((900, 700), (1920, 1080), &p),
            vp(510, 190, 900, 700)
        );
        assert_eq!(
            viewport_for((900, 700), (1800, 1400), &p),
            vp(0, 0, 1800, 1400)
        );
        assert_eq!(
            viewport_for((1244, 700), (1600, 900), &p),
            vp(178, 100, 1244, 700)
        );
        // Smaller than the canvas: scale stays 1 and the canvas is cropped.
        assert_eq!(
            viewport_for((900, 700), (800, 600), &p),
            vp(-50, -50, 900, 700)
        );
    }

    #[test]
    fn viewports_match_measured_sdl() {
        let integer = matches(ScaleMode::Integer, &MEASURED_INTEGER);
        let fit = matches(ScaleMode::Fit, &MEASURED_FIT);
        let fullscreen = matches(ScaleMode::Fit, &MEASURED_FULLSCREEN);
        println!("Integer {integer}/3, Fit {fit}/16, fullscreen {fullscreen}/1");
        assert_eq!((integer, fit, fullscreen), (3, 16, 1));
    }

    #[test]
    fn zero_sizes_give_empty_viewport() {
        for mode in [ScaleMode::Integer, ScaleMode::Fit] {
            assert!(viewport_for((0, 700), (1280, 800), &params(mode)).is_empty());
            assert!(viewport_for((900, 700), (1280, 0), &params(mode)).is_empty());
        }
    }

    #[test]
    fn window_to_logical_integer_edges() {
        let c = (900, 700);
        let v = viewport_for(c, (1920, 1080), &params(ScaleMode::Integer));
        assert_eq!(window_to_logical(v, c, 510.0, 190.0), Some((0, 0)));
        assert_eq!(window_to_logical(v, c, 1409.99, 889.99), Some((899, 699)));
        assert_eq!(window_to_logical(v, c, 509.99, 190.0), None);
        assert_eq!(window_to_logical(v, c, 1410.0, 190.0), None);
        assert_eq!(window_to_logical(v, c, 600.0, 189.99), None);
        assert_eq!(window_to_logical(v, c, 600.0, 890.0), None);
    }

    #[test]
    fn window_to_logical_measured_fit_corners() {
        let mut ok = 0;
        for (c, s, _) in MEASURED_FIT.iter().chain(MEASURED_FULLSCREEN.iter()) {
            let v = viewport_for(*c, *s, &params(ScaleMode::Fit));
            let (vx, vy) = (f64::from(v.x), f64::from(v.y));
            let (vw, vh) = (f64::from(v.w), f64::from(v.h));
            let first = window_to_logical(v, *c, vx, vy);
            let last = window_to_logical(v, *c, vx + vw - 0.01, vy + vh - 0.01);
            let expected_last = Some((c.0 as i32 - 1, c.1 as i32 - 1));
            if first == Some((0, 0)) && last == expected_last {
                ok += 1;
            } else {
                eprintln!("{c:?} in {s:?}: first {first:?}, last {last:?}");
            }
        }
        println!("window_to_logical Fit corners {ok}/17");
        assert_eq!(ok, 17);
    }

    #[test]
    fn sampler_follows_filter() {
        let n = sampler_for(Filter::Nearest);
        assert_eq!(n.mag_filter, wgpu::FilterMode::Nearest);
        assert_eq!(n.min_filter, wgpu::FilterMode::Nearest);
        let l = sampler_for(Filter::Linear);
        assert_eq!(l.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(l.min_filter, wgpu::FilterMode::Linear);
    }

    #[test]
    fn surface_format_prefers_srgb_and_guards_linear() {
        assert_eq!(
            choose_surface_format(&[T::Bgra8UnormSrgb, T::Bgra8Unorm]),
            Ok((T::Bgra8UnormSrgb, None))
        );
        assert_eq!(
            choose_surface_format(&[T::Bgra8Unorm]),
            Ok((T::Bgra8Unorm, Some(T::Bgra8UnormSrgb)))
        );
        assert_eq!(
            choose_surface_format(&[T::Rgb10a2Unorm, T::Rgba8Unorm]),
            Ok((T::Rgba8Unorm, Some(T::Rgba8UnormSrgb)))
        );
        assert!(choose_surface_format(&[T::Rgb10a2Unorm]).is_err());
        assert!(choose_surface_format(&[]).is_err());
    }
}
