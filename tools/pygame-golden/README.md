# pygame-golden

Reference images and JSON goldens for `banquo-canvas` are generated with pygame,
so the Rust canvas can be checked against pygame's Surface and Rect behaviour.
Golden files live under `crates/banquo-canvas/tests/golden/` and are committed
byte-exact (see `.gitattributes`); each set carries a SHA-256 manifest.

## Version pins

| Component | Version |
|-----------|---------|
| Python    | 3.13.11 |
| pygame    | 2.6.1   |
| SDL       | 2.28.4  |

Regenerate goldens only with these versions; a different pygame or SDL build can
change anti-aliasing and font rasterising.

## raster

`raster/gen_raster.py` writes the raster and CRT goldens. It imports only the
standard library and pygame, draws everything with generic pygame calls
(`Surface.blit`, `pygame.draw.*`) and writes its own PNGs with `zlib`, so the
bytes depend only on the pixels. Alpha planes and masks are 8-bit grayscale,
colour images are 8-bit RGB.

```
python -I tools/pygame-golden/raster/gen_raster.py --out crates/banquo-canvas/tests/golden
```

It refuses to run on other versions than the pins above (`--any-version`
overrides that, for experiments only). Two runs write byte-identical files.

| Set | Files | What it checks |
|-----|-------|----------------|
| `raster/blend.json` | 6,912 samples | an SRCALPHA source blitted onto an opaque destination at alphas 1, 35, 64, 108, 128, 200, 210, 254, 255 |
| `raster/masks/rect-*` | 220 | `draw.rect(width, border_radius)`: radii 0-10 x widths 0-3 x sizes 6x6, 38x38, 67x36, 67x52, 71x56 |
| `raster/masks/circle-*` | 24 | `draw.circle` for radii 1-6, widths 0-3 |
| `raster/masks/line-*` | 20 | 1 px `draw.line`: horizontal, vertical and diagonal |
| `raster/clip/*` | 15 | fill, rect, circle, line and blit at offsets (-3, -2), (-40, -40) and (62, 47) on 64x48 |
| `crt/scanline-<w>x600` | 4 | the scanline plane at terminal widths 800, 1020, 1144, 1580 |
| `crt/vignette-<w>x600` | 4 | the legacy band vignette plane at the same widths |
| `crt/apply-<w>x600-f3` | 4 | three frames (scanlines, vignette, flicker) on (0, 170, 0) |
| `crt/flicker.json` | 1 | 60 frame times (integer microseconds) and the flicker alphas |

The CRT planes use the legacy CRT parameters: a scanline every 3 rows at
alpha 35; 20 vignette bands with alpha `int(t * t * 120)` and band width
`max(2, int((w + h) / 20 / 2))`, rect edges truncated from `f64`; flicker
`min(int(|sin(8 * t) * 0.02| * 255), 15)` when the sine is negative.

Known gaps, kept in the goldens as references: rings with `1 < width < radius`
(`circle-r3-w2` and up) are not pixel-exact in `banquo-canvas` yet, so their
check is an ignored test. `fill_rect` follows `draw.rect(width=0)`, which
clips; pygame's `Surface.fill(color, rect)` instead moves a rect with a
negative x or y to 0.

## text
