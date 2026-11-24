"""Rebuild the original MIT-licensed test face with fontTools 4.68.0."""
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

# Hand-drawn 3-by-5 bitmap digits, one octal row per character.
DIGITS = ["75557", "26227", "71747", "71717", "55711",
          "74717", "74757", "71111", "75757", "75717"]


def glyph(rows):
    pen = TTGlyphPen(None)
    for y, row in enumerate(reversed(rows)):
        for x in range(3):
            if int(row) & (4 >> x):
                left, bottom = x * 120, y * 120
                pen.moveTo((left, bottom))
                pen.lineTo((left, bottom + 100))
                pen.lineTo((left + 100, bottom + 100))
                pen.lineTo((left + 100, bottom))
                pen.closePath()
    return pen.glyph()


names = [".notdef"] + [f"digit{i}" for i in range(10)]
font = FontBuilder(1000, isTTF=True)
font.setupGlyphOrder(names)
font.setupCharacterMap({ord(str(i)): names[i + 1] for i in range(10)})
font.setupGlyf(dict(zip(names, map(glyph, [""] + DIGITS))))
font.setupHorizontalMetrics({name: (500, 0) for name in names})
font.setupHorizontalHeader(ascent=800, descent=-200)
font.setupNameTable({
    "familyName": "Elidex Test Digits", "styleName": "Regular",
    "fullName": "Elidex Test Digits", "psName": "ElidexTestDigits",
    "copyright": "Copyright 2026 elidex contributors",
    "licenseDescription": "MIT License (see the repository LICENSE).",
})
font.setupOS2(fsType=0, sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
font.setupPost()
font.font["head"].created = font.font["head"].modified = 3900000000
font.font.recalcTimestamp = False
font.save(Path(__file__).with_name("digits.ttf"))
