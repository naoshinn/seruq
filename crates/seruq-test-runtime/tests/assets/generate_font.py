"""Regenerate color-test.ttf using fontTools (only needed to rebuild the fixture).

Original test artwork: X is a solid rectangle; space is empty.
The generated font and this script use the repository's MIT/Apache-2.0 licenses.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

builder = FontBuilder(1000, isTTF=True)
builder.setupGlyphOrder([".notdef", "space", "X"])
builder.setupCharacterMap({32: "space", 88: "X"})
glyphs = {}
for name in [".notdef", "space", "X"]:
    pen = TTGlyphPen(None)
    if name != "space":
        pen.moveTo((100, 0))
        pen.lineTo((700, 0))
        pen.lineTo((700, 700))
        pen.lineTo((100, 700))
        pen.closePath()
    glyphs[name] = pen.glyph()
builder.setupGlyf(glyphs)
builder.setupHorizontalMetrics({name: (800, 100) for name in glyphs})
builder.setupHorizontalHeader(ascent=800, descent=-200)
builder.setupNameTable({
    "familyName": "Seruq Color Test", "styleName": "Regular",
    "uniqueFontIdentifier": "SeruqColorTest-Regular-1",
    "fullName": "Seruq Color Test Regular", "psName": "SeruqColorTest-Regular",
    "version": "Version 1.0",
    "licenseDescription": "MIT OR Apache-2.0; see repository licenses.",
})
builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
builder.setupPost()
builder.font["head"].created = 2082844800
builder.font["head"].modified = 2082844800
builder.save(Path(__file__).with_name("color-test.ttf"))
