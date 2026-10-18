#!/usr/bin/env python3
"""Round 5 — the SEAM family (sheared disc / split form) + the hard mitred return.

Idea under test: a form is split by one straight line and one half is displaced
along that line. Not a spinner, not a pin, not a swirl: a tectonic step in an
otherwise pure geometric mass. Conceptually: the interruption made visible, and
the record continuing across it.
"""
import math, os
from PIL import Image, ImageDraw

PI = math.pi
K = 4


# ------------------------------------------------------------------ helpers
def arc_range(c, R, a0, a1, n=200):
    return [(c[0] + R * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
             c[1] + R * math.sin(math.radians(a0 + (a1 - a0) * i / n)))
            for i in range(n + 1)]


def pt_add(p, d, k): return (p[0] + d[0] * k, p[1] + d[1] * k)


def unit(a_deg):
    a = math.radians(a_deg); return (math.cos(a), math.sin(a))


def line_intersect(p, t, q, u):
    """Intersection of line(p,t) and line(q,u); None if parallel."""
    den = t[0] * u[1] - t[1] * u[0]
    if abs(den) < 1e-12: return None
    dx, dy = q[0] - p[0], q[1] - p[1]
    k = (dx * u[1] - dy * u[0]) / den
    return (p[0] + t[0] * k, p[1] + t[1] * k)


def render(fn, size, view=256.0, bg=(0, 0, 0, 0)):
    img = Image.new("RGBA", (size * K, size * K), bg)
    fn(ImageDraw.Draw(img), view, size * K)
    return img.resize((size, size), Image.LANCZOS)


def S(px, view): return px / view


# ================================================================ SEAM DISC
def make_seam_disc(dr, view, px, seam_deg=0.0, shear=0.0, slot=0.0, R=100.0,
                   inner=None, SCL=None):
    sc = S(px, view)
    c = (128.0, 128.0)
    u = unit(seam_deg)
    n = unit(seam_deg + 90.0)
    cA = pt_add(c, n, -slot / 2.0)
    cB = pt_add(pt_add(c, n, slot / 2.0), u, shear)

    # half A : the -n side
    a_out = arc_range(cA, R, seam_deg + 180, seam_deg + 360)
    polyA = a_out if inner is None else None
    if inner is None:
        dr.polygon([(x * sc, y * sc) for x, y in a_out], fill=(0, 0, 0, 255))
    else:
        a_in = arc_range(cA, inner, seam_deg + 360, seam_deg + 180)
        dr.polygon([(x * sc, y * sc) for x, y in (a_out + a_in)],
                   fill=(0, 0, 0, 255))
    # half B : the +n side, sheared
    b_out = arc_range(cB, R, seam_deg, seam_deg + 180)
    if inner is None:
        dr.polygon([(x * sc, y * sc) for x, y in b_out], fill=(0, 0, 0, 255))
    else:
        b_in = arc_range(cB, inner, seam_deg + 180, seam_deg)
        dr.polygon([(x * sc, y * sc) for x, y in (b_out + b_in)],
                   fill=(0, 0, 0, 255))


# ================================================================ HARD RETURN
def make_return(dr, view, px, sw=30.0, ang1=118.0, ang2=28.0,
                L1=120.0, L2=78.0):
    """Two straight arms meeting at a mitred vertex. Unequal, asymmetric."""
    sc = S(px, view)
    v = (128.0, 168.0)
    d1 = (-math.cos(math.radians(ang1 - 90)), -math.sin(math.radians(ang1 - 90)))
    d2 = (math.cos(math.radians(ang2)), -math.sin(math.radians(ang2)))
    p0 = pt_add(v, d1, L1)          # far end of arm 1
    p2 = pt_add(v, d2, L2)          # far end of arm 2
    t1 = unit(math.degrees(math.atan2(v[1] - p0[1], v[0] - p0[0])))
    t2 = unit(math.degrees(math.atan2(p2[1] - v[1], p2[0] - v[0])))
    n1 = (-t1[1], t1[0]); n2 = (-t2[1], t2[0])
    w = sw / 2.0
    left = [pt_add(p0, n1, w), None, pt_add(p2, n2, w)]
    right = [pt_add(p0, n1, -w), None, pt_add(p2, n2, -w)]
    left[1] = line_intersect(pt_add(v, n1, w), t1, pt_add(v, n2, w), t2)
    right[1] = line_intersect(pt_add(v, n1, -w), t1, pt_add(v, n2, -w), t2)
    poly = left + list(reversed(right))
    if any(p is None for p in poly):
        return
    dr.polygon([(x * sc, y * sc) for x, y in poly], fill=(0, 0, 0, 255))
    # square-cut caps
    for p, t, nn in ((p0, t1, n1), (p2, t2, n2)):
        cap = [pt_add(p, nn, w), pt_add(p, nn, -w)]
        # extend a hair to close
    return poly


# ================================================================ sheets
def contact_sheet(items, cols, cell, out, bg=(18, 20, 21, 255), gap=16, label=True):
    rows = (len(items) + cols - 1) // cols
    lh = 22 if label else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sheet = Image.new("RGBA", (W, H), bg)
    dr = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(items):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap); y = gap + r * (cell + lh + gap)
        tile = Image.new("RGBA", (cell, cell), (246, 246, 244, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sheet.alpha_composite(tile, (x, y))
        if label: dr.text((x + 2, y + cell + 4), name, fill=(210, 212, 208, 255))
    sheet.convert("RGB").save(out); print("wrote", out)


CANDS = [
    ("d1-shear",   dict(seam_deg=0.0,  shear=30.0, slot=0.0)),
    ("d2-slotshear",dict(seam_deg=0.0, shear=26.0, slot=13.0)),
    ("d3-ring",    dict(seam_deg=0.0,  shear=26.0, slot=12.0, inner=62.0)),
    ("d4-diag",    dict(seam_deg=45.0, shear=26.0, slot=12.0)),
    ("d5-diagring",dict(seam_deg=45.0, shear=24.0, slot=11.0, inner=62.0)),
    ("d6-shear45", dict(seam_deg=45.0, shear=34.0, slot=0.0)),
    ("e1-return",  None),
]

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    big, tiny = [], []
    for name, kw in CANDS:
        if kw is None:
            mk = lambda dr, view, px: make_return(dr, view, px)
        else:
            mk = (lambda k=kw: (lambda dr, view, px: make_seam_disc(dr, view, px, **k)))()
        for s in (512, 32, 16):
            render(mk, s).save(f"renders/r5-{name}-{s}.png")
        big.append((name, render(mk, 288)))
        tiny.append((name, render(mk, 16)))
    contact_sheet(big, 4, 288, "renders/sheet-r5-288.png")
    contact_sheet(tiny, 7, 112, "renders/sheet-r5-16.png")
    print("done")
