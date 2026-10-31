"""Builds assets/fonts/ScorchMono.ttf: Terminus TTF with the official dv1
variant (standard Cyrillic ve and de, U+0432/U+0434), a standard Cyrillic i
and short i (U+0438/U+0439; Terminus draws them like a Latin u), and a new
name, as the OFL's Reserved Font Name clause requires for modified versions."""
import sys
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen

src_ttf, bdf_path, out = sys.argv[1:4]
PX, BASE = 62.5, 20  # units per pixel, baseline y (measured from the TTF)

def bitmap(code):
    lines = open(bdf_path).read().split('\n')
    i = lines.index(f'ENCODING {code}')
    j = lines.index('BITMAP', i)
    rows = []
    for l in lines[j + 1:]:
        if l == 'ENDCHAR':
            break
        rows.append(int(l, 16))
    return rows  # 16 rows, top first; rows 0..11 above the baseline

def outline(rows):
    pen = TTGlyphPen(None)
    for r, bits in enumerate(rows):
        y0 = BASE + (11 - r) * PX
        c = 0
        while c < 8:
            if bits & (0x80 >> c):
                start = c
                while c < 8 and bits & (0x80 >> c):
                    c += 1
                x0, x1, y1 = start * PX, c * PX, y0 + PX
                pen.moveTo((round(x0), round(y0)))
                pen.lineTo((round(x0), round(y1)))
                pen.lineTo((round(x1), round(y1)))
                pen.lineTo((round(x1), round(y0)))
                pen.closePath()
            else:
                c += 1
    return pen.glyph()

# Standard print U+0438 / U+0439 with a diagonal, 8x16 rows like the BDF (top first).
BODY = [0x42, 0x42, 0x46, 0x4A, 0x52, 0x62, 0x42]
OWN = {
    0x0438: [0, 0, 0, 0, 0] + BODY + [0, 0, 0, 0],
    0x0439: [0, 0, 0x24, 0x18, 0] + BODY + [0, 0, 0, 0],
}

font = TTFont(src_ttf)
cmap = font.getBestCmap()
glyf = font['glyf']
if sys.argv[4:] == ['check']:
    g = outline(bitmap(0x043D))
    g.recalcBounds(glyf)
    ref = glyf[cmap[0x043D]]
    print('U+043D drawn', (g.xMin, g.yMin, g.xMax, g.yMax), 'font', (ref.xMin, ref.yMin, ref.xMax, ref.yMax))
    sys.exit()
for code in (0x0432, 0x0434):
    glyf[cmap[code]] = outline(bitmap(code))
for code, rows in OWN.items():
    glyf[cmap[code]] = outline(rows)
for tag in ('EBDT', 'EBLC', 'BDF '):
    if tag in font:
        del font[tag]
name = font['name']
for rec in name.names:
    if rec.nameID in (1, 16):
        rec.string = 'Scorch Mono'
    elif rec.nameID == 4:
        rec.string = 'Scorch Mono'
    elif rec.nameID == 6:
        rec.string = 'ScorchMono-Regular'
    elif rec.nameID == 3:
        rec.string = 'ScorchMono-Regular;based on Terminus (TTF) 4.49.3'
font.save(out)
print('saved', out)
