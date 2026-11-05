#!/usr/bin/env python3
"""Round 6 — parameter study for the SEAM mark.

A disc is split along one straight line; one half is displaced along that line.
Flush (slot 0) reads as one body with a slip; a slot reads as a seam.
Study: shear x slot x half (solid vs ring).
"""
import math, os
from PIL import Image, ImageDraw

PI = math.pi
K = 6            # high supersample: we are judging 16px here


# ------------------------------------------------------------------ geometry
def half_disc(cx, cy, R, upper=True):
    """SVG path for a filled half-disc, flat side on the horizontal."""
    s = 1 if upper else 0
    return (f"M {cx-R:.4f} {cy:.4f} A {R:.4f} {R:.4f} 0 0 {s} "
            f"{cx+R:.4f} {cy:.4f} Z")


def half_ring(cx, cy, R, r, upper=True):
    so, si = (1, 0) if upper else (0, 1)
    return (f"M {cx-R:.4f} {cy:.4f} A {R:.4f} {R:.4f} 0 0 {so} {cx+R:.4f} {cy:.4f} "
            f"L {cx+r:.4f} {cy:.4f} A {r:.4f} {r:.4f} 0 0 {si} {cx-r:.4f} {cy:.4f} Z")


def seam_parts(R=100.0, shear=32.0, slot=0.0, inner=None):
    """Returns (path_upper, path_lower) in local coords, centres as computed."""
    ay, by = -slot / 2.0, slot / 2.0
    if inner is None:
        return (half_disc(0.0, ay, R, True), half_disc(shear, by, R, False))
    return (half_ring(0.0, ay, R, inner, True), half_ring(shear, by, R, inner, False))


def seam_bbox(R, shear, slot=0.0):
    x0 = -R; x1 = max(R, shear + R)
    y0 = -R - slot / 2.0; y1 = R + slot / 2.0
    return x0, y0, x1, y1


def to_svg(R, shear, slot, inner, fill="#000000", pad=0.045):
    pu, pl = seam_parts(R, shear, slot, inner)
    x0, y0, x1, y1 = seam_bbox(R, shear, slot)
    w, h = x1 - x0, y1 - y0
    B = max(w, h) * (1 + pad * 2)
    ox = (B - w) / 2.0 - x0
    oy = (B - h) / 2.0 - y0
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {B:.4f} {B:.4f}" '
            f'width="{B:.4f}" height="{B:.4f}">\n'
            f'  <g transform="translate({ox:.4f} {oy:.4f})">\n'
            f'    <path d="{pu}" fill="{fill}"/>\n'
            f'    <path d="{pl}" fill="{fill}"/>\n'
            f'  </g>\n</svg>\n'), B


# ------------------------------------------------------------------ raster
def draw_seam(dr, px, R=100.0, shear=32.0, slot=0.0, inner=None, fill=(0, 0, 0, 255)):
    """Draw in a px-sized box, fitted so the mark fills 90% of the square."""
    x0, y0, x1, y1 = seam_bbox(R, shear, slot)
    w, h = x1 - x0, y1 - y0
    sc = (px * 0.90) / max(w, h)
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc

    def poly(pts): dr.polygon([(ox + p[0] * sc, oy + p[1] * sc) for p in pts], fill=fill)

    # upper: arc through the top, then the diameter closes it
    up = [( -R * math.cos(math.radians(i * 180 / 120)), -slot / 2.0 - R * math.sin(math.radians(i * 180 / 120)))
          for i in range(121)]
    up = [(x, -math.sqrt(max(0.0, R * R - x * x)) - slot / 2.0)
          for x in [(-R + 2 * R * i / 240) for i in range(241)]]
    lo = [(shear + x, math.sqrt(max(0.0, R * R - x * x)) + slot / 2.0)
          for x in [(-R + 2 * R * i / 240) for i in range(241)]]
    if inner is None:
        poly(up); poly(lo)
    else:
        r2, R2, s2 = inner * inner, R * R, slot / 2.0
        upi = [(x, -math.sqrt(max(0.0, r2 - x * x)) - s2)
               for x in [(-inner + 2 * inner * i / 200) for i in range(201)]]
        loi = [(shear + x, math.sqrt(max(0.0, r2 - x * x)) + s2)
               for x in [(-inner + 2 * inner * i / 200) for i in range(201)]]
        poly(up + list(reversed(upi)))
        poly(lo + list(reversed(loi)))


def render(size, bg=(0, 0, 0, 0), **kw):
    img = Image.new("RGBA", (size * K, size * K), bg)
    draw_seam(ImageDraw.Draw(img), size * K, **kw)
    return img.resize((size, size), Image.LANCZOS)


def save(img, name, bg=None):
    if bg is not None:
        f = Image.new("RGBA", img.size, bg); f.alpha_composite(img); img = f
    img.save(name); print("wrote", name)


# ------------------------------------------------------------------ sheet
def sheet(items, cols, cell, out, bg=(21, 24, 26, 255), gap=14, label=True):
    rows = (len(items) + cols - 1) // cols
    lh = 20 if label else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sh = Image.new("RGBA", (W, H), bg); dr = ImageDraw.Draw(sh)
    for i, (name, img) in enumerate(items):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap); y = gap + r * (cell + lh + gap)
        tile = Image.new("RGBA", (cell, cell), (245, 245, 243, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sh.alpha_composite(tile, (x, y))
        if label: dr.text((x + 2, y + cell + 3), name, fill=(208, 210, 206, 255))
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)

    # parameter study on the solid disc
    variants = [(26.0, 0.0), (32.0, 0.0), (40.0, 0.0), (48.0, 0.0),
                (32.0, 8.0), (40.0, 10.0), (48.0, 12.0)]
    big, tiny = [], []
    for shear, slot in variants:
        nm = f"sh{int(shear)}-sl{int(slot)}"
        big.append((nm, render(288, shear=shear, slot=slot)))
        tiny.append((nm, render(16, shear=shear, slot=slot)))
    sheet(big, 4, 288, "renders/r6-solid-288.png")
    sheet(tiny, 7, 112, "renders/r6-solid-16.png")

    # ring construction
    ringv = [(30.0, 12.0, 60.0), (34.0, 12.0, 62.0), (34.0, 0.0, 62.0), (40.0, 14.0, 56.0)]
    rb, rt = [], []
    for shear, slot, inner in ringv:
        nm = f"r-sh{int(shear)}-sl{int(slot)}-in{int(inner)}"
        rb.append((nm, render(288, shear=shear, slot=slot, inner=inner)))
        rt.append((nm, render(16, shear=shear, slot=slot, inner=inner)))
    sheet(rb, 4, 288, "renders/r6-ring-288.png")
    sheet(rt, 4, 112, "renders/r6-ring-16.png")
    print("done")
