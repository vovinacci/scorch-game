"""Build assets/fonts/ScorchMono.ttf from Terminus TTF.

Terminus TTF with the official dv1 variant (standard Cyrillic ve and de,
U+0432/U+0434), a standard Cyrillic i and short i (U+0438/U+0439; Terminus
draws them like a Latin u), and a new name, as the OFL's Reserved Font Name
clause requires for modified versions. Needs Python 3.15 (lazy imports).

    python3 tools/patch_font.py TerminusTTF-4.49.3.ttf ter-u16n.bdf out.ttf [check]

With "check", prints how a redrawn glyph lines up with the font's own and
writes nothing.
"""

import sys
from pathlib import Path

from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

# Only for type hints, so it loads only if something asks for it.
lazy from fontTools.ttLib.tables._g_l_y_f import Glyph

PX = 62.5  # font units per pixel
COLUMNS = 8  # pixels per glyph row
BASE = 20  # baseline y (measured from the TTF)

# Standard print U+0438 / U+0439 with a diagonal, 8x16 rows like the BDF (top first).
BODY = [0x42, 0x42, 0x46, 0x4A, 0x52, 0x62, 0x42]
OWN = {
    0x0438: [0, 0, 0, 0, 0, *BODY, 0, 0, 0, 0],
    0x0439: [0, 0, 0x24, 0x18, 0, *BODY, 0, 0, 0, 0],
}
# The dv1 variant's glyphs, taken from the patched BDF.
FROM_BDF = (0x0432, 0x0434)

NAMES = {
    1: "Scorch Mono",  # family
    4: "Scorch Mono",  # full name
    6: "ScorchMono-Regular",  # PostScript name
    16: "Scorch Mono",  # typographic family
    3: "ScorchMono-Regular;based on Terminus (TTF) 4.49.3",  # unique ID
}


def bitmap(bdf: list[str], code: int) -> list[int]:
    """Return the 16 rows of one BDF glyph, top first; rows 0..11 are above the baseline."""
    start = bdf.index("BITMAP", bdf.index(f"ENCODING {code}")) + 1
    end = bdf.index("ENDCHAR", start)
    return [int(row, 16) for row in bdf[start:end]]


def outline(rows: list[int]) -> Glyph:
    """Draw one rectangle per run of lit pixels, row by row."""
    pen = TTGlyphPen(None)
    for r, bits in enumerate(rows):
        y0 = BASE + (11 - r) * PX
        c = 0
        while c < COLUMNS:
            if not bits & (0x80 >> c):
                c += 1
                continue
            start = c
            while c < COLUMNS and bits & (0x80 >> c):
                c += 1
            x0, x1, y1 = start * PX, c * PX, y0 + PX
            pen.moveTo((round(x0), round(y0)))
            pen.lineTo((round(x0), round(y1)))
            pen.lineTo((round(x1), round(y1)))
            pen.lineTo((round(x1), round(y0)))
            pen.closePath()
    return pen.glyph()


def main(args: list[str]) -> None:
    """Patch the font, or with "check" only compare one glyph."""
    match args:
        case [src_ttf, bdf_path, out]:
            check = False
        case [src_ttf, bdf_path, out, "check"]:
            check = True
        case _:
            sys.exit(__doc__)

    bdf = Path(bdf_path).read_text(encoding="ascii").split("\n")
    font = TTFont(src_ttf)
    cmap = font.getBestCmap()
    glyf = font["glyf"]

    if check:
        g = outline(bitmap(bdf, 0x043D))
        g.recalcBounds(glyf)
        ref = glyf[cmap[0x043D]]
        print("U+043D drawn", (g.xMin, g.yMin, g.xMax, g.yMax), "font", (ref.xMin, ref.yMin, ref.xMax, ref.yMax))
        return

    for code in FROM_BDF:
        glyf[cmap[code]] = outline(bitmap(bdf, code))
    for code, rows in OWN.items():
        glyf[cmap[code]] = outline(rows)
    # Embedded bitmaps would still show the old glyphs at 16 px.
    for tag in ("EBDT", "EBLC", "BDF "):
        if tag in font:
            del font[tag]
    for rec in font["name"].names:
        if (new := NAMES.get(rec.nameID)) is not None:
            rec.string = new
    font.save(out)
    print("saved", out)


if __name__ == "__main__":
    main(sys.argv[1:])
