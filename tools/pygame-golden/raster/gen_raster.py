"""Generate the banquo-canvas raster and CRT goldens with pygame.

Usage:
    python -I tools/pygame-golden/raster/gen_raster.py --out <dir>

Writes <dir>/raster/** and <dir>/crt/**, each with a MANIFEST.json that lists
the SHA-256 of every file and a {python, pygame, sdl} header. The committed
copy lives in crates/banquo-canvas/tests/golden/.

The tool imports only the standard library and pygame. Every plane is drawn
with generic pygame calls (Surface.fill, Surface.blit, pygame.draw.*),
parameterised the same way as banquo-canvas's CrtParams. The CRT planes use
the legacy CRT parameters: scanlines every 3 rows at alpha 35, 20 vignette
bands up to alpha 120, flicker rate 8.0, intensity 0.02, max alpha 15.

PNG files are written by this script (stdlib zlib), not by pygame, so the
bytes depend only on the pixels: alpha planes are 8-bit grayscale, colour
images are 8-bit RGB.
"""

import argparse
import hashlib
import json
import math
import os
import platform
import struct
import sys
import zlib
from pathlib import Path

os.environ["PYGAME_HIDE_SUPPORT_PROMPT"] = "1"
import pygame  # noqa: E402

PINNED = {"python": "3.13.11", "pygame": "2.6.1", "sdl": "2.28.4"}

WHITE = (255, 255, 255, 255)

# Blend samples: SRCALPHA source blitted onto an opaque destination.
BLEND_ALPHAS = [1, 35, 64, 108, 128, 200, 210, 254, 255]
BLEND_GRID = (32, 24)  # 768 pixels per alpha, 6912 samples in all

# draw.rect masks: radii 0-10 x widths 0-3 x these sizes.
RECT_SIZES = [(6, 6), (38, 38), (67, 36), (67, 52), (71, 56)]
RECT_RADII = range(0, 11)
RECT_WIDTHS = range(0, 4)
MASK_MARGIN = 2

# draw.circle masks.
CIRCLE_RADII = range(1, 7)
CIRCLE_WIDTHS = range(0, 4)
CIRCLE_SURFACE = 16

# 1 px draw.line masks on a 48x40 surface: (name, start, end).
LINE_SURFACE = (48, 40)
LINES = [
    ("h-right", (2, 5), (40, 5)),
    ("h-left", (40, 9), (2, 9)),
    ("h-point", (7, 13), (7, 13)),
    ("v-down", (4, 2), (4, 35)),
    ("v-up", (9, 35), (9, 2)),
    ("d-45-se", (2, 2), (30, 30)),
    ("d-45-nw", (30, 30), (2, 2)),
    ("d-45-ne", (2, 35), (35, 2)),
    ("d-45-sw", (35, 2), (2, 35)),
    ("d-shallow-e", (3, 4), (44, 17)),
    ("d-shallow-w", (44, 17), (3, 4)),
    ("d-steep-s", (5, 1), (14, 38)),
    ("d-steep-n", (14, 38), (5, 1)),
    ("d-odd-1", (1, 1), (46, 2)),
    ("d-odd-2", (1, 38), (2, 1)),
    ("d-odd-3", (0, 0), (47, 39)),
    ("d-odd-4", (47, 0), (0, 39)),
    ("d-odd-5", (10, 20), (37, 31)),
    ("d-odd-6", (37, 31), (10, 20)),
    ("d-odd-7", (20, 3), (29, 36)),
]

# Clipping: each primitive drawn at these offsets on a 64x48 opaque surface.
CLIP_SURFACE = (64, 48)
CLIP_OFFSETS = [(-3, -2), (-40, -40), (62, 47)]
CLIP_BACKGROUND = (16, 32, 48)

# CRT planes at the four terminal widths.
CRT_WIDTHS = [800, 1020, 1144, 1580]
CRT_HEIGHT = 600
CRT_FILL = (0, 170, 0)
CRT_APPLY_FRAMES = 3

LEGACY = {
    "scanline_every": 3,
    "scanline_alpha": 35,
    "steps": 20,
    "max_alpha": 120.0,
    "band": None,
    "flicker_rate": 8.0,
    "flicker_intensity": 0.02,
    "flicker_max_alpha": 15,
}

# Frame times for the flicker golden, in microseconds; dt = n / 1e6 exactly
# as Rust computes it (no float text to parse). The first step lands in the
# negative half of the sine so the 3-frame apply golden darkens.
FLICKER_FRAMES = 60
FLICKER_DT_MICROS = [450000] + [
    (16000, 17000, 16000, 33000)[i % 4] for i in range(FLICKER_FRAMES - 1)
]


# --------------------------------------------------------------------------
# PNG and manifest output


def png_bytes(width, height, rows, color_type):
    """An 8-bit PNG; color_type 0 = gray, 2 = RGB. rows = list of bytes."""

    def chunk(tag, data):
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    raw = b"".join(b"\x00" + row for row in rows)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def alpha_png(surf, rgb=None):
    """A gray PNG of surf's alpha channel. rgb, when given, is the only colour
    a pixel with non-zero alpha may have; a zero-alpha pixel must be all 0."""
    w, h = surf.get_size()
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            c = surf.get_at((x, y))
            if c.a == 0:
                assert (c.r, c.g, c.b) == (0, 0, 0), (x, y, c)
            elif rgb is not None:
                assert (c.r, c.g, c.b) == rgb, (x, y, c)
            row.append(c.a)
        rows.append(bytes(row))
    return png_bytes(w, h, rows, 0)


def mask_png(surf):
    """A gray PNG of a white-on-transparent mask: every pixel is (0,0,0,0) or
    (255,255,255,255), stored as gray 0 or 255."""
    w, h = surf.get_size()
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            c = tuple(surf.get_at((x, y)))
            assert c in ((0, 0, 0, 0), WHITE), (x, y, c)
            row.append(c[3])
        rows.append(bytes(row))
    return png_bytes(w, h, rows, 0)


def rgb_png(surf):
    w, h = surf.get_size()
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            c = surf.get_at((x, y))
            row += bytes((c.r, c.g, c.b))
        rows.append(bytes(row))
    return png_bytes(w, h, rows, 2)


def json_bytes(obj):
    return (json.dumps(obj, indent=2, sort_keys=True) + "\n").encode("ascii")


class Tree:
    """Files of one golden set, written under root and listed in a manifest."""

    def __init__(self, root):
        self.root = root
        self.files = {}

    def put(self, rel, data):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        self.files[rel] = hashlib.sha256(data).hexdigest()

    def write_manifest(self, header):
        manifest = {"header": header, "files": dict(sorted(self.files.items()))}
        (self.root / "MANIFEST.json").write_bytes(json_bytes(manifest))


# --------------------------------------------------------------------------
# raster


def blend_samples():
    gw, gh = BLEND_GRID
    n = gw * gh
    extremes = [(0, 255), (255, 0), (0, 0), (255, 255)]
    samples = []
    for a in BLEND_ALPHAS:
        src = pygame.Surface((gw, gh), pygame.SRCALPHA)
        dst = pygame.Surface((gw, gh))
        cases = []
        for k in range(n):
            if k < len(extremes):
                s, d = extremes[k]
                sv, dv = (s, s, s), (d, d, d)
            else:
                sv = (k % 256, (k * 53 + 7) % 256, (k * 181 + 101) % 256)
                dv = ((k * 97 + 31) % 256, (255 - k) % 256, (k * 29 + 200) % 256)
            x, y = k % gw, k // gw
            src.set_at((x, y), (*sv, a))
            dst.set_at((x, y), dv)
            cases.append((x, y, sv, dv))
        dst.blit(src, (0, 0))
        for x, y, sv, dv in cases:
            out = dst.get_at((x, y))
            samples.append([*sv, a, *dv, out.r, out.g, out.b])
    lines = ",\n".join("    " + json.dumps(s, separators=(",", ":")) for s in samples)
    head = {
        "alphas": BLEND_ALPHAS,
        "columns": ["sr", "sg", "sb", "a", "dr", "dg", "db", "rr", "rg", "rb"],
        "source": "SRCALPHA source blitted onto an opaque 32-bit destination",
    }
    text = json.dumps(head, indent=2, sort_keys=True)[:-2]
    text += ',\n  "samples": [\n' + lines + "\n  ]\n}\n"
    json.loads(text)
    return text.encode("ascii")


def gen_masks(tree):
    count = 0
    for w, h in RECT_SIZES:
        for r in RECT_RADII:
            for width in RECT_WIDTHS:
                surf = pygame.Surface(
                    (w + 2 * MASK_MARGIN, h + 2 * MASK_MARGIN), pygame.SRCALPHA
                )
                pygame.draw.rect(
                    surf,
                    WHITE,
                    pygame.Rect(MASK_MARGIN, MASK_MARGIN, w, h),
                    width=width,
                    border_radius=r,
                )
                tree.put(f"masks/rect-{w}x{h}-r{r:02d}-w{width}.png", mask_png(surf))
                count += 1
    c = CIRCLE_SURFACE // 2
    for r in CIRCLE_RADII:
        for width in CIRCLE_WIDTHS:
            surf = pygame.Surface((CIRCLE_SURFACE, CIRCLE_SURFACE), pygame.SRCALPHA)
            pygame.draw.circle(surf, WHITE, (c, c), r, width=width)
            tree.put(f"masks/circle-r{r}-w{width}.png", mask_png(surf))
            count += 1
    for name, start, end in LINES:
        surf = pygame.Surface(LINE_SURFACE, pygame.SRCALPHA)
        pygame.draw.line(surf, WHITE, start, end, 1)
        tree.put(f"masks/line-{name}.png", mask_png(surf))
        count += 1
    return count


def clip_sprite():
    """A 50x45 SRCALPHA sprite with varied colour and alpha."""
    sprite = pygame.Surface((50, 45), pygame.SRCALPHA)
    for y in range(45):
        for x in range(50):
            sprite.set_at(
                (x, y),
                ((x * 5) % 256, (y * 5) % 256, (x * y) % 256, (x * 7 + y * 3) % 256),
            )
    return sprite


def gen_clip(tree):
    sprite = clip_sprite()
    # fill_rect is draw.rect(width=0), which clips. Surface.fill(color, rect)
    # is not used: for a rect with a negative x or y it moves the rect to 0
    # instead of clipping it.
    prims = {
        "fill_rect": lambda s, ox, oy: pygame.draw.rect(
            s, (200, 60, 20), pygame.Rect(ox, oy, 50, 45)
        ),
        "draw_rect": lambda s, ox, oy: (
            pygame.draw.rect(
                s, (220, 200, 40), pygame.Rect(ox, oy, 50, 45), 3, border_radius=8
            ),
            pygame.draw.rect(
                s, (40, 200, 220), pygame.Rect(ox + 6, oy + 6, 38, 33), 0, 5
            ),
        ),
        "draw_circle": lambda s, ox, oy: (
            pygame.draw.circle(s, (250, 250, 250), (ox + 3, oy + 3), 6),
            pygame.draw.circle(s, (250, 120, 250), (ox + 43, oy + 43), 6, 1),
            pygame.draw.circle(s, (120, 250, 250), (ox + 44, oy + 3), 5, 1),
        ),
        "draw_line": lambda s, ox, oy: (
            pygame.draw.line(s, (255, 255, 0), (ox, oy), (ox + 50, oy + 45)),
            pygame.draw.line(s, (0, 255, 255), (ox, oy + 1), (ox + 60, oy + 1)),
            pygame.draw.line(s, (255, 0, 255), (ox + 2, oy), (ox + 2, oy + 50)),
            pygame.draw.line(s, (255, 128, 0), (ox + 49, oy), (ox, oy + 44)),
        ),
        "blit": lambda s, ox, oy: s.blit(sprite, (ox, oy)),
    }
    for name, draw in prims.items():
        for ox, oy in CLIP_OFFSETS:
            surf = pygame.Surface(CLIP_SURFACE)
            surf.fill(CLIP_BACKGROUND)
            draw(surf, ox, oy)
            tree.put(f"clip/{name}_{ox}_{oy}.png", rgb_png(surf))
    return len(prims) * len(CLIP_OFFSETS)


# --------------------------------------------------------------------------
# crt, parameterised like CrtParams


def legacy_band_width(w, h, steps):
    return max(2, int((w + h) / steps / 2))


def scanline_plane(w, h, every, alpha):
    plane = pygame.Surface((w, h), pygame.SRCALPHA)
    for y in range(0, h, every):
        pygame.draw.line(plane, (0, 0, 0, alpha), (0, y), (w, y))
    return plane


def legacy_bands_plane(w, h, steps, max_alpha, band):
    """Concentric band frames, each drawn over the last with draw.rect; the
    rect edges come from f64 products truncated with int()."""
    plane = pygame.Surface((w, h), pygame.SRCALPHA)
    cx, cy = w // 2, h // 2
    b = legacy_band_width(w, h, steps) if band is None else band
    for i in range(steps):
        t = i / steps
        color = (0, 0, 0, int(t * t * max_alpha))
        mx = int(cx * (1.0 - t))
        my = int(cy * (1.0 - t))
        r = pygame.Rect(cx - mx, cy - my, mx * 2, my * 2)
        for edge in (
            pygame.Rect(r.left, r.top, r.width, b),
            pygame.Rect(r.left, r.bottom - b, r.width, b),
            pygame.Rect(r.left, r.top, b, r.height),
            pygame.Rect(r.right - b, r.top, b, r.height),
        ):
            pygame.draw.rect(plane, color, edge)
    return plane


def flicker_alpha(frame_time, rate, intensity, max_alpha):
    """The darkening alpha for one frame, 0 when the sine is not negative."""
    v = math.sin(frame_time * rate) * intensity
    if v < 0:
        return min(int(abs(v) * 255), max_alpha)
    return 0


def apply_crt(surf, scan, vig, state, dt, p):
    """One frame: scanlines, then vignette, then flicker, in place."""
    surf.blit(scan, (0, 0))
    surf.blit(vig, (0, 0))
    state["frame_time"] += dt
    v = math.sin(state["frame_time"] * p["flicker_rate"]) * p["flicker_intensity"]
    if v < 0:
        dark = pygame.Surface(surf.get_size(), pygame.SRCALPHA)
        dark.fill((0, 0, 0, min(int(abs(v) * 255), p["flicker_max_alpha"])))
        surf.blit(dark, (0, 0))


def gen_crt(tree):
    p = LEGACY
    dts = [n / 1e6 for n in FLICKER_DT_MICROS]
    ft = 0.0
    alphas = []
    for dt in dts:
        ft += dt
        alphas.append(
            flicker_alpha(
                ft, p["flicker_rate"], p["flicker_intensity"], p["flicker_max_alpha"]
            )
        )
    assert all(0 <= a <= 5 for a in alphas), alphas
    tree.put(
        "flicker.json",
        json_bytes(
            {
                "alpha": alphas,
                "dt_micros": FLICKER_DT_MICROS,
                "flicker_intensity": p["flicker_intensity"],
                "flicker_max_alpha": p["flicker_max_alpha"],
                "flicker_rate": p["flicker_rate"],
                "frame_time_start": 0,
            }
        ),
    )
    bands = {}
    h = CRT_HEIGHT
    for w in CRT_WIDTHS:
        scan = scanline_plane(w, h, p["scanline_every"], p["scanline_alpha"])
        vig = legacy_bands_plane(w, h, p["steps"], p["max_alpha"], p["band"])
        tree.put(f"scanline-{w}x{h}.png", alpha_png(scan, (0, 0, 0)))
        tree.put(f"vignette-{w}x{h}.png", alpha_png(vig, (0, 0, 0)))
        surf = pygame.Surface((w, h))
        surf.fill(CRT_FILL)
        state = {"frame_time": 0.0}
        for f in range(CRT_APPLY_FRAMES):
            apply_crt(surf, scan, vig, state, dts[f], p)
        tree.put(f"apply-{w}x{h}-f{CRT_APPLY_FRAMES}.png", rgb_png(surf))
        bands[w] = legacy_band_width(w, h, p["steps"])
    return bands, alphas


# --------------------------------------------------------------------------


def versions():
    return {
        "python": platform.python_version(),
        "pygame": pygame.version.ver,
        "sdl": ".".join(str(v) for v in pygame.get_sdl_version()),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", required=True, help="golden root directory")
    ap.add_argument(
        "--any-version",
        action="store_true",
        help="run on unpinned Python/pygame/SDL versions (not for committing)",
    )
    args = ap.parse_args()
    header = versions()
    if header != PINNED and not args.any_version:
        sys.exit(f"version mismatch: have {header}, goldens need {PINNED}")
    pygame.init()
    out = Path(args.out)

    raster = Tree(out / "raster")
    raster.put("blend.json", blend_samples())
    masks = gen_masks(raster)
    clips = gen_clip(raster)
    raster.write_manifest(header)

    crt = Tree(out / "crt")
    bands, alphas = gen_crt(crt)
    crt.write_manifest(header)

    print(f"raster: blend {len(BLEND_ALPHAS) * BLEND_GRID[0] * BLEND_GRID[1]}")
    print(f"raster: masks {masks}, clip {clips}")
    print(f"crt: band widths {bands}")
    print(f"crt: flicker {alphas}")


if __name__ == "__main__":
    main()
