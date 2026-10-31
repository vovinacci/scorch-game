# Fonts

`ScorchMono.ttf` is Terminus (TTF) 4.49.3 by Dimitar Zhekov and Tilman Blumenbach, modified: the Cyrillic letters в and
д use Terminus' own alternative variant `dv1` (standard print shapes instead of the default handwritten-style ones), and
и and й are redrawn with the standard diagonal (Terminus draws them like a Latin u at this size). Embedded bitmaps were
removed, so all sizes use the outlines.

The SIL Open Font License (`LICENSE-OFL.txt`) reserves the names "Terminus Font" and "Terminus (TTF)", so the modified
font is renamed "Scorch Mono".

Rebuild with `tools/patch_font.py` (needs Python with fontTools):

    python3 tools/patch_font.py TerminusTTF-4.49.3.ttf ter-u16n.bdf assets/fonts/ScorchMono.ttf

where `TerminusTTF-4.49.3.ttf` comes from the Terminus (TTF) release and `ter-u16n.bdf` from the Terminus 4.49 source
with `alt/dv1.diff` applied.
