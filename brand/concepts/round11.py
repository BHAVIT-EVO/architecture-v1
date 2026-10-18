#!/usr/bin/env python3
"""Round 11 — the concept checkpoint sheet shown to the user."""
import math, os
from PIL import Image, ImageDraw, ImageFont
from round9 import draw_seam, draw_thread, render

CARAMEL = (200, 155, 107, 255)
CHARCOAL_BG = (24, 27, 29, 255)
TILE = (246, 246, 244, 255)
INK = (17, 20, 22, 255)

F = "/System/Library/Fonts/SFNS.ttf"


def font(sz, bold=False):
    try:
        return ImageFont.truetype("/System/Library/Fonts/SFNSMono.ttf" if bold else F, sz)
    except Exception:
        return ImageFont.load_default()


def seamC(dr, px, fill=INK):  draw_seam(dr, px, inner=62.0, slot=10.0, shear=34.0, fill=fill)
def slipC(dr, px, fill=INK):  draw_seam(dr, px, inner=None, slot=9.0, shear=40.0, fill=fill)
def threadC(dr, px, fill=INK): draw_thread(dr, px, fill=fill)


def panel(fn, w, h, bg, colour, frac=0.74, pad_top=0):
    p = Image.new("RGBA", (w, h), bg)
    s = min(w, h - pad_top)
    im = render(lambda dr, px: fn(dr, px, fill=colour), int(s * frac))
    p.alpha_composite(im, ((w - im.size[0]) // 2, pad_top + (h - pad_top - im.size[1]) // 2))
    return p


def build():
    W, H = 1500, 1000
    sh = Image.new("RGBA", (W, H), (17, 19, 21, 255))
    dr = ImageDraw.Draw(sh)

    f_title = font(30); f_sub = font(17); f_lab = font(20); f_small = font(14); f_mono = font(14)

    dr.text((58, 44), "Evo — three directions", fill=(238, 236, 232, 255), font=f_title)
    dr.text((58, 86), "silhouette only, no colour  ·  every mark is one shape or one stroke",
            fill=(140, 144, 142, 255), font=f_sub)

    cw, ch = 452, 470
    x0, y0 = 58, 140
    gap = 24

    specs = [
        ("A", "THE SEAM", "one ring, one seam, the top half slipped along it",
         seamC, "recommended", CARAMEL),
        ("B", "THE SLIP", "the same argument as a single solid mass",
         slipC, "", None),
        ("C", "THE THREAD", "the name, drawn as one unbroken stroke",
         threadC, "", None),
    ]

    for i, (letter, name, sub, fn, badge, tint) in enumerate(specs):
        x = x0 + i * (cw + gap)
        # main tile, light
        t = panel(fn, cw, ch, TILE, (17, 20, 22, 255), frac=0.70)
        sh.alpha_composite(t, (x, y0))
        # 16 px true-size test, dark, aligned bottom-left of the tile
        tiny = Image.new("RGBA", (78, 78), (17, 19, 21, 255))
        im = render(lambda d, px: fn(d, px, fill=(255, 255, 255, 255)), 16)
        tiny.alpha_composite(im, (31, 31))
        sh.alpha_composite(tiny, (x + cw - 88, y0 + ch - 88))
        dr.text((x + cw - 84, y0 + ch - 8), "16 px", fill=(120, 124, 122, 255), font=f_small)

        if tint:                                     # colour hint, recommendation only
            hint = panel(fn, 150, 150, CHARCOAL_BG, tint, frac=0.66)
            sh.alpha_composite(hint, (x + cw - 158, y0 + 10))

        # letter + name
        dr.text((x, y0 + ch + 22), f"{letter}  {name}", fill=(238, 236, 232, 255), font=f_lab)
        if badge:
            bb = dr.textbbox((0, 0), badge, font=f_small)
            bw = bb[2] - bb[0] + 20
            dr.rounded_rectangle([x + cw - bw, y0 + ch + 22, x + cw, y0 + ch + 46],
                                 radius=12, fill=(200, 155, 107, 255))
            dr.text((x + cw - bw + 10, y0 + ch + 26), badge, fill=(26, 18, 6, 255), font=f_small)
        dr.text((x, y0 + ch + 52), sub, fill=(146, 150, 148, 255), font=f_sub)

    dr.text((58, H - 58),
            "A is one ring divided by a single seam — the record never breaks, it slips.",
            fill=(150, 154, 152, 255), font=f_sub)
    sh.convert("RGB").save("renders/CONCEPT-SHEET.png")
    print("wrote renders/CONCEPT-SHEET.png")


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    build()
