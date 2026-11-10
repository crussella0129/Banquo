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

## text
