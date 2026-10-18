#!/usr/bin/env python3
"""Round 10 — colour + craft checks for the chosen direction (A, The Seam)."""
import math, os
from PIL import Image, ImageDraw
from round9 import draw_seam, draw_thread, render

K = 6
CARAMEL = (200, 155, 107, 255)      # #C89B6B  (Evo accent, from DESIGN.md)
CHARCOAL = (21, 24, 26, 255)        # #15181A
INK = (236, 234, 230, 255)          # #ECEAE6
TEAL = (91, 138, 143, 255)          # #5B8A8F
PAPER = (244, 243, 240, 255)


def mono(fn, px, colour, bg, size=None, frac=0.82):
    size = size or px
    t = Image.new("RGBA", (size, size), bg)
    im = render(lambda dr, p: fn(dr, p, fill=colour), int(px * frac))
    t.alpha_composite(im, ((size - im.size[0]) // 2, (size - im.size[1]) // 2))
    return t


def seamC(dr, px, fill=(0, 0, 0, 255)):
    draw_seam(dr, px, inner=62.0, slot=10.0, shear=34.0, fill=fill)


def threadC(dr, px, fill=(0, 0, 0, 255)):
    draw_thread(dr, px, fill=fill)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)

    # ---- colour study : A in caramel on charcoal (the Evo palette)
    mono(seamC, 300, CARAMEL, CHARCOAL).save("renders/A-color-caramel-charcoal.png")
    mono(seamC, 300, CHARCOAL, CARAMEL).save("renders/A-color-charcoal-caramel.png")
    mono(seamC, 300, INK, CHARCOAL).save("renders/A-mono-white-charcoal.png")
    mono(seamC, 300, (0, 0, 0, 255), PAPER).save("renders/A-mono-black-paper.png")
    mono(seamC, 300, CARAMEL, PAPER).save("renders/A-mono-caramel-paper.png")

    # ---- the one-colour test: does A survive as a single flat shape?
    #        (this is also the embroidery / laser / stamp test)
    for name, fn, colour, bg in [
        ("one-colour-black", seamC, (17, 20, 22, 255), (255, 255, 255, 255)),
        ("one-colour-white", seamC, (255, 255, 255, 255), (17, 20, 22, 255)),
    ]:
        t = Image.new("RGBA", (300, 300), bg)
        im = render(lambda dr, p: fn(dr, p, fill=colour), 246)
        t.alpha_composite(im, (27, 27)); t.save(f"renders/A-{name}.png")

    # ---- the 16px "recognisability" strip on light and dark
    strip = Image.new("RGBA", (2 * 260 + 60, 200), (26, 29, 31, 255))
    for i, (colour, bg) in enumerate([((0, 0, 0, 255), PAPER), (INK, CHARCOAL)]):
        x = 30 + i * 260
        tile = Image.new("RGBA", (230, 140), bg)
        xx = 10
        for s in (16, 24, 32, 48, 64):
            im = render(lambda dr, p: seamC(dr, p, fill=colour), s)
            tile.alpha_composite(im, (xx, 70 - s // 2)); xx += s + 14
        strip.alpha_composite(tile, (x, 30))
    strip.convert("RGB").save("renders/A-scale-strip.png")

    # ---- B and C in the palette too
    mono(lambda dr, px, fill: draw_seam(dr, px, inner=None, slot=9.0, shear=40.0, fill=fill),
         300, CARAMEL, CHARCOAL).save("renders/B-color.png")
    mono(threadC, 300, INK, CHARCOAL, frac=0.86).save("renders/C-color-dark.png")
    mono(threadC, 300, (0, 0, 0, 255), PAPER, frac=0.86).save("renders/C-color-light.png")
    print("done")
