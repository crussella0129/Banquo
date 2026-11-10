"""Generate the banquo-canvas text goldens and the baked glyph atlas with pygame.

Usage:
    python -I tools/pygame-golden/text/gen_text.py --out <dir>

``<dir>`` is the test root (``crates/banquo-canvas/tests``). The tool reads
``<dir>/fixtures/fonts/JetBrainsMono-Regular.ttf`` (SIL OFL 1.1, ``OFL.txt``
beside it) and ``sizes.json`` next to this script, and writes:

* ``<dir>/fixtures/atlas/jetbrains-mono-regular/atlas.json`` (atlas format 2)
  and one page ``atlas-<size>.png`` per size;
* ``<dir>/golden/text/corpus.json``: every corpus entry (size, index,
  category, text, and whether pygame rendered it);
* ``<dir>/golden/text/<size>/<index>.png``: pygame's
  ``Font(font, size).render(text, True, COLOUR)`` as an 8-bit RGBA PNG;
* ``<dir>/golden/text/sizes.json``: pygame's ``Font.size(text)`` per entry;
* ``<dir>/golden/text/MANIFEST.json``: the SHA-256 of every file above.

It never writes ``golden/text/approved_differences.json`` (the known
differences, maintained by hand) and the manifest does not list it.

Atlas bake. fontTools copies the font with every glyph ID g mapped to the
private-use code point U+E000 + g (GSUB, GPOS and GDEF dropped, hinting and
metrics tables untouched), and pygame renders each code point, blended and
white. ``Font.metrics`` gives SDL_ttf's bitmap box, from which the glyph's
alpha is cut out of the render; every render is checked against that box.
Glyph entries are ``[px, py, w, h, ox, oy]``: the place on the page, the size
and the offset of the bitmap's top-left from the pen origin, y = 0 being the
top of the line box. Pages are RGB PNGs with the alpha in the red channel.
The full sizes hold every glyph ID; the other sizes hold ``.notdef`` and the
glyphs of the subset characters.

Overlap rule. Each rendered corpus entry is assembled from the atlas (glyph
IDs from uharfbuzz, LTR, script Zzzz, no language, no user features, like
SDL_ttf 2.20.1; the pen moves by the size's advance after every glyph with a
non-zero shaped advance) under the rules max, over and or. The rule with the
fewest mismatches against pygame is recorded (ties: max, over, or), or
``no-overlap-observed`` when the three rules agree on every entry.

PNG files are written by this script (stdlib zlib), so their bytes depend only
on the pixels. Two runs write byte-identical files.
"""

import argparse
import hashlib
import json
import os
import platform
import random
import struct
import sys
import sysconfig
import tempfile
import warnings
import zlib
from pathlib import Path

os.environ["PYGAME_HIDE_SUPPORT_PROMPT"] = "1"
os.environ.setdefault("SDL_VIDEODRIVER", "dummy")
os.environ.setdefault("SDL_AUDIODRIVER", "dummy")
with warnings.catch_warnings():
    warnings.simplefilter("ignore")
    import pygame  # noqa: E402
    import pygame.font  # noqa: E402

PINNED = {
    "python": "3.13.11",
    "pygame": "2.6.1",
    "sdl": "2.28.4",
    "sdl_ttf": "2.20.1",
    "fonttools": "4.62.1",
    "uharfbuzz": "0.53.3",
}

FONT_NAME = "JetBrainsMono-Regular.ttf"
FONT_SHA256 = "a0bf60ef0f83c5ed4d7a75d45838548b1f6873372dfac88f71804491898d138f"
ATLAS_SUBDIR = "jetbrains-mono-regular"
FORMAT = 2
PUA_BASE = 0xE000
PAGE_WIDTH_FULL = 1024
PAGE_WIDTH_SUBSET = 256
PAD = 1
WHITE = (255, 255, 255)
#: The colour every golden render uses (pygame always renders it opaque).
COLOUR = (230, 180, 60)
RULES = ("max", "over", "or")
NO_OVERLAP = "no-overlap-observed"
ZERO_WIDTH = "zero_width_error"
RENDERED = "png"

RANDOM_SEED = 324
RANDOM_COUNT = 64
RANDOM_MAX_LEN = 40
PRINTABLE = "".join(chr(c) for c in range(0x20, 0x7F))

#: Fixed strings at the full sizes, in corpus order: (category, text).
FIXED = [
    ("overhang", "__"),
    ("overhang", "->"),
    ("overhang", "&&"),
    ("control", "\t"),
    ("control", "a\nb"),
    ("notdef", "中"),
]
LATIN1 = [chr(c) for c in range(0x00A0, 0x0100)]
CYRILLIC = [chr(c) for c in range(0x0410, 0x0450)]
GREEK = [chr(c) for c in range(0x0391, 0x03CA)]
#: Code-ligature (calt) trigger sequences.
CALT = [
    "=>", "<-", "<=", ">=", "!=", "==", "===", "!==", "=/=", "<>", "</", "/>",
    "</>", "//", "///", "/*", "*/", "||", "::", ":::", "..", "...", "..<", "?.",
    "??", "?:", "++", "+++", "--", "---", "<<", ">>", "<<<", ">>>", "|>", "<|",
    "<|>", "<=>", "-->", "<--", "<->", "==>", "<==", "~~", "~>", "<~", "#{",
    "#[", "]#", "##", "###", "0x10", "www", ";;", "%%", "|=", "^=", "<*>",
    "<+>", ":=", "=:", ">>=", "=<<", "<!--", "{|", "|}", "[|", "|]", "a->b",
    "x => y", "i++ + --j", "a != b && c == d",
]  # fmt: skip


# --------------------------------------------------------------------------
# Files


def png_bytes(w, h, rows, color_type):
    """An 8-bit PNG (2 = RGB, 6 = RGBA) from raw rows, filter 0, zlib level 9."""

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    raw = b"".join(b"\x00" + r for r in rows)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, color_type, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def json_lines(items):
    """A JSON array with one compact, key-sorted item per line (ASCII)."""
    body = ",\n".join(
        "  " + json.dumps(i, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        for i in items
    )
    return ("[\n" + body + "\n]\n").encode("ascii")


class Tree:
    """Files written under ``root`` and their SHA-256s."""

    def __init__(self, root):
        self.root = Path(root)
        self.files = {}

    def put(self, rel, data):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        self.files[rel] = hashlib.sha256(data).hexdigest()


# --------------------------------------------------------------------------
# Versions


def versions():
    import fontTools
    import uharfbuzz

    return {
        "python": platform.python_version(),
        "pygame": pygame.version.ver,
        "sdl": ".".join(str(v) for v in pygame.get_sdl_version()),
        "sdl_ttf": ".".join(str(v) for v in pygame.font.get_sdl_ttf_version()),
        "fonttools": fontTools.version,
        "uharfbuzz": uharfbuzz.__version__,
    }


def platform_tag():
    return sysconfig.get_platform().replace("-", "_").replace(".", "_")


# --------------------------------------------------------------------------
# PUA font and atlas bake


def build_pua_font(font_path, out_dir):
    """The font with glyph ID g at U+E000 + g and no GSUB/GPOS/GDEF; returns (path, count)."""
    from fontTools.ttLib import TTFont
    from fontTools.ttLib.tables._c_m_a_p import cmap_format_4

    font = TTFont(str(font_path), recalcTimestamp=False)
    order = font.getGlyphOrder()
    if PUA_BASE + len(order) - 1 > 0xF8FF:
        sys.exit("glyph IDs do not fit below U+F8FF")
    for tag in ("GSUB", "GPOS", "GDEF"):
        if tag in font:
            del font[tag]
    mapping = {PUA_BASE + gid: name for gid, name in enumerate(order)}
    tables = []
    for platform_id, encoding_id in ((0, 3), (3, 1)):
        sub = cmap_format_4(4)
        sub.platformID, sub.platEncID, sub.language = platform_id, encoding_id, 0
        sub.cmap = dict(mapping)
        tables.append(sub)
    font["cmap"].tables = tables
    out = Path(out_dir) / "pua.ttf"
    font.save(str(out))
    font.close()
    return out, len(order)


def cmap_gids(font_path, chars):
    from fontTools.ttLib import TTFont

    font = TTFont(str(font_path), lazy=True)
    cmap = font.getBestCmap()
    order = {name: gid for gid, name in enumerate(font.getGlyphOrder())}
    gids = [order[cmap[ord(c)]] if ord(c) in cmap else 0 for c in chars]
    font.close()
    return gids


def alpha_of(surface):
    if surface.get_width() == 0:
        return b""
    return pygame.image.tobytes(surface, "RGBA")[3::4]


def bake_glyph(font, gid, ascent, height):
    """(w, h, ox, oy, alpha) of glyph ``gid`` from the PUA font."""
    ch = chr(PUA_BASE + gid)
    minx, maxx, miny, maxy, advance = font.metrics(ch)[0]
    left, width, top, rows = minx, maxx - minx, maxy, maxy - miny
    ox, oy = left, ascent - top
    try:
        surface = font.render(ch, True, WHITE)
    except pygame.error:
        if width or advance:
            sys.exit(f"glyph {gid}: zero-width render but metrics {width}/{advance}")
        return (0, 0, ox, oy, b"")
    sx0, sy0 = min(0, ox), min(0, oy)
    sw = max(ox + width, advance) - sx0
    sh = max(height, oy + rows) - sy0
    if surface.get_size() != (sw, sh):
        sys.exit(f"glyph {gid}: render {surface.get_size()} != box {(sw, sh)}")
    alpha = alpha_of(surface)
    bx, by = ox - sx0, oy - sy0
    out = bytearray(width * rows)
    for y in range(sh):
        row = alpha[y * sw : (y + 1) * sw]
        if by <= y < by + rows:
            out[(y - by) * width : (y - by + 1) * width] = row[bx : bx + width]
            outside = row[:bx] + row[bx + width :]
        else:
            outside = row
        if any(outside):
            sys.exit(f"glyph {gid}: ink outside the bitmap box")
    return (width, rows, ox, oy, bytes(out))


def pack(glyphs, page_width):
    """Shelf-pack in glyph-ID order; returns ({gid: (x, y)}, page height)."""
    places = {}
    x = y = shelf = 0
    for gid in sorted(glyphs):
        w, h = glyphs[gid][0], glyphs[gid][1]
        if w == 0 or h == 0:
            places[gid] = (0, 0)
            continue
        if x + w > page_width:
            x, y, shelf = 0, y + shelf + PAD, 0
        places[gid] = (x, y)
        x += w + PAD
        shelf = max(shelf, h)
    return places, max(1, y + shelf)


def page_png(glyphs, places, width, height):
    rows = [bytearray(width * 3) for _ in range(height)]
    for gid, (w, h, _ox, _oy, alpha) in glyphs.items():
        px, py = places[gid]
        for r in range(h):
            rows[py + r][px * 3 : (px + w) * 3 : 3] = alpha[r * w : (r + 1) * w]
    return png_bytes(width, height, [bytes(r) for r in rows], 2)


def bake(font_path, pua_path, glyph_count, full_sizes, sizes, subset_gids):
    """Every size: {size: metrics and glyphs} (pages packed later)."""
    out = {}
    for size in sizes:
        orig = pygame.font.Font(str(font_path), size)
        pua = pygame.font.Font(str(pua_path), size)
        advance = orig.metrics("M")[0][4]
        if orig.size("M")[0] != advance:
            sys.exit(f"size {size}: advance {advance} != size('M')")
        metrics = (
            orig.get_ascent(),
            orig.get_descent(),
            orig.get_height(),
            orig.get_linesize(),
        )
        if metrics != (
            pua.get_ascent(),
            pua.get_descent(),
            pua.get_height(),
            pua.get_linesize(),
        ):
            sys.exit(f"size {size}: PUA font metrics differ")
        ascent, descent, height, line_height = metrics
        gids = range(glyph_count) if size in full_sizes else sorted({0, *subset_gids})
        glyphs = {gid: bake_glyph(pua, gid, ascent, height) for gid in gids}
        out[size] = {
            "advance": advance,
            "ascent": ascent,
            "descent": descent,
            "height": height,
            "line_height": line_height,
            "glyphs": glyphs,
        }
    return out


def write_atlas(tree, baked, full_sizes, chars, glyph_count, source, rule):
    sizes = {}
    for size, b in baked.items():
        width = PAGE_WIDTH_FULL if size in full_sizes else PAGE_WIDTH_SUBSET
        places, height = pack(b["glyphs"], width)
        page = f"atlas-{size}.png"
        tree.put(page, page_png(b["glyphs"], places, width, height))
        sizes[str(size)] = {
            "advance": b["advance"],
            "ascent": b["ascent"],
            "descent": b["descent"],
            "glyph_count": len(b["glyphs"]),
            "glyphs": {
                str(gid): [*places[gid], g[0], g[1], g[2], g[3]]
                for gid, g in sorted(b["glyphs"].items())
            },
            "height": b["height"],
            "line_height": b["line_height"],
            "page": page,
        }
    doc = {
        "entry_fields": ["px", "py", "w", "h", "ox", "oy"],
        "font": FONT_NAME,
        "font_sha256": FONT_SHA256,
        "format": FORMAT,
        "glyph_count": glyph_count,
        "overlap_rule": rule,
        "oy_origin": "top of line box",
        "page_channel": "red",
        "pua_base": PUA_BASE,
        "sizes": sizes,
        "source": source,
        "tile_chars": chars,
    }
    text = json.dumps(doc, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
    tree.put("atlas.json", (text + "\n").encode("ascii"))


# --------------------------------------------------------------------------
# Corpus


def random_strings():
    rnd = random.Random(RANDOM_SEED)
    return [
        "".join(rnd.choice(PRINTABLE) for _ in range(rnd.randint(1, RANDOM_MAX_LEN)))
        for _ in range(RANDOM_COUNT)
    ]


def corpus(full_sizes, sizes, chars):
    full = list(FIXED)
    full += [("latin1", c) for c in LATIN1]
    full += [("cyrillic", c) for c in CYRILLIC]
    full += [("greek", c) for c in GREEK]
    full += [("calt", s) for s in CALT]
    full += [("random", s) for s in random_strings()]
    subset = [("subset", c) for c in chars] + [("subset_string", "".join(chars))]
    rows = []
    for size in sizes:
        entries = (full if size in full_sizes else []) + subset
        rows += [
            {"size": size, "index": i, "category": cat, "text": text}
            for i, (cat, text) in enumerate(entries)
        ]
    return rows


# --------------------------------------------------------------------------
# Overlap rule


def combine(rule, d, s):
    if rule == "max":
        return d if d > s else s
    if rule == "over":
        return s + (d * (255 - s) + 127) // 255
    return d | s


class Shaper:
    def __init__(self, font_path):
        import uharfbuzz as hb

        self.hb = hb
        self.font = hb.Font(hb.Face(hb.Blob.from_file_path(str(font_path))))

    def shape(self, text):
        """[(glyph ID, x_advance)]: LTR, script Zzzz, no language, no user features."""
        hb = self.hb
        buf = hb.Buffer()
        buf.add_str(text)
        buf.direction = "ltr"
        buf.script = "Zzzz"
        hb.shape(self.font, buf, {})
        return [
            (i.codepoint, p.x_advance)
            for i, p in zip(buf.glyph_infos, buf.glyph_positions)
        ]


def assemble(size_bake, seq, rule):
    """(w, h, alpha) under ``rule``, or None when a glyph is not baked at this size."""
    adv, height = size_bake["advance"], size_bake["height"]
    placed = []
    pen = minx = maxx = miny = 0
    maxy = height
    for gid, x_advance in seq:
        g = size_bake["glyphs"].get(gid)
        if g is None:
            return None
        w, h, ox, oy, alpha = g
        x = pen + ox
        minx, maxx = min(minx, x), max(maxx, x + w)
        miny, maxy = min(miny, oy), max(maxy, oy + h)
        placed.append((x, oy, w, h, alpha))
        if x_advance != 0:
            pen += adv
    maxx = max(maxx, pen)
    width, hgt = maxx - minx, maxy - miny
    out = bytearray(width * hgt)
    for x, y, w, h, alpha in placed:
        for r in range(h):
            base = (y - miny + r) * width + (x - minx)
            for c in range(w):
                s = alpha[r * w + c]
                d = out[base + c]
                out[base + c] = combine(rule, d, s) if s and d else (s or d)
    return (width, hgt, bytes(out))


# --------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "--out", required=True, help="test root (crates/banquo-canvas/tests)"
    )
    ap.add_argument(
        "--any-version",
        action="store_true",
        help="run on unpinned versions (not for committing)",
    )
    args = ap.parse_args()
    header = versions()
    if header != PINNED and not args.any_version:
        sys.exit(f"version mismatch: have {header}, goldens need {PINNED}")
    pygame.font.init()
    out = Path(args.out)
    font_path = out / "fixtures" / "fonts" / FONT_NAME
    if hashlib.sha256(font_path.read_bytes()).hexdigest() != FONT_SHA256:
        sys.exit(f"{font_path}: unexpected SHA-256")
    if not (font_path.parent / "OFL.txt").is_file():
        sys.exit("OFL.txt must travel with the font")
    spec = json.loads(
        (Path(__file__).resolve().parent / "sizes.json").read_text("ascii")
    )
    full_sizes, sizes, chars = (
        spec["full_sizes"],
        spec["subset_sizes"],
        spec["subset_chars"],
    )
    if not set(full_sizes) <= set(sizes):
        sys.exit("full_sizes must be a subset of subset_sizes")

    with tempfile.TemporaryDirectory(prefix="banquo-pua-") as tmp:
        pua_path, glyph_count = build_pua_font(font_path, tmp)
        baked = bake(
            font_path,
            pua_path,
            glyph_count,
            full_sizes,
            sizes,
            cmap_gids(font_path, chars),
        )
        del pua_path  # the PUA font is an intermediate, never written to <out>

    golden = Tree(out / "golden" / "text")
    rows = corpus(full_sizes, sizes, chars)
    fonts = {}
    shaper = Shaper(font_path)
    measured = []
    mismatches = {rule: 0 for rule in RULES}
    identical = True
    for row in rows:
        size, text = row["size"], row["text"]
        font = fonts.setdefault(size, pygame.font.Font(str(font_path), size))
        w, h = font.size(text)
        measured.append({"size": size, "index": row["index"], "w": w, "h": h})
        try:
            surface = font.render(text, True, COLOUR)
        except pygame.error:
            row["render"] = ZERO_WIDTH
            continue
        row["render"] = RENDERED
        sw, sh = surface.get_size()
        rgba = pygame.image.tobytes(surface, "RGBA") if sw else b""
        if any(tuple(rgba[i : i + 3]) != COLOUR for i in range(0, len(rgba), 4)):
            sys.exit(f"size {size} index {row['index']}: RGB is not the colour")
        rows_rgba = [rgba[y * sw * 4 : (y + 1) * sw * 4] for y in range(sh)]
        golden.put(f"{size}/{row['index']}.png", png_bytes(sw, sh, rows_rgba, 6))
        ref = (sw, sh, rgba[3::4])
        seq = shaper.shape(text)
        outs = {rule: assemble(baked[size], seq, rule) for rule in RULES}
        if len({outs[rule] for rule in RULES}) > 1:
            identical = False
        for rule in RULES:
            if outs[rule] != ref:
                mismatches[rule] += 1

    if identical:
        rule = NO_OVERLAP
    else:
        rule = min(RULES, key=lambda r: mismatches[r])

    source = {k: header[k] for k in ("fonttools", "pygame", "sdl", "sdl_ttf")}
    source["platform"] = platform_tag()
    atlas = Tree(out / "fixtures" / "atlas" / ATLAS_SUBDIR)
    write_atlas(atlas, baked, full_sizes, chars, glyph_count, source, rule)

    golden.put(
        "corpus.json",
        json_lines(
            {
                "category": r["category"],
                "index": r["index"],
                "render": r["render"],
                "size": r["size"],
                "text": r["text"],
            }
            for r in rows
        ),
    )
    golden.put("sizes.json", json_lines(measured))
    manifest = {
        "header": {**header, "colour": list(COLOUR), "font_sha256": FONT_SHA256},
        "files": dict(sorted(golden.files.items())),
        "fixtures": {
            f"atlas/{ATLAS_SUBDIR}/{k}": v for k, v in sorted(atlas.files.items())
        },
        "overlap_rule": rule,
        "rule_mismatches": mismatches,
    }
    text = json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
    (golden.root / "MANIFEST.json").write_bytes(text.encode("ascii"))

    refused = sum(1 for r in rows if r["render"] == ZERO_WIDTH)
    print(f"text: entries {len(rows)}, sizes {len(sizes)}, pygame refused {refused}")
    print(f"text: rule mismatches {mismatches}, overlap_rule {rule}")


if __name__ == "__main__":
    main()
