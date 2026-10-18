#!/usr/bin/env python3
"""Round 4 — refine the three survivors from round 3 into presentable concepts.

Survivors:
  A  THE RETURN (from c6) — one line that goes, turns and comes back shorter.
  B  THE VO (from c3)     — v and o as one unbroken monoline stroke.
  C  THE WITNESS (from c4) — a ring of unequal spans with honest gaps.
"""
import math, os
from PIL import Image, ImageDraw

PI = math.pi


# ------------------------------------------------------------------ geometry
def tangents(pts, closed=False):
    n = len(pts); out = []
    for i in range(n):
        if closed:
            a, b = pts[(i - 1) % n], pts[(i + 1) % n]
        else:
            a, b = pts[max(i - 1, 0)], pts[min(i + 1, n - 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy) or 1.0
        out.append((dx / L, dy / L))
    return out


def stroke_outline(pts, widths):
    T = tangents(pts)
    N = [(-t[1], t[0]) for t in T]
    up = [(p[0] + n[0] * w / 2, p[1] + n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    lo = [(p[0] - n[0] * w / 2, p[1] - n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    return up + list(reversed(lo))


def arc_pts(cx, cy, r, a0, a1, n=180):
    a0, a1 = math.radians(a0), math.radians(a1)
    if a1 < a0: a1 += 2 * PI
    return [(cx + r * math.cos(a0 + (a1 - a0) * i / n),
             cy + r * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


def draw_stroke(dr, pts, widths, fill, caps=True, joins=True, scale=1.0, dx=0.0, dy=0.0):
    if isinstance(widths, (int, float)):
        widths = [float(widths)] * len(pts)
    if callable(widths):
        widths = [widths(i / max(1, len(pts) - 1)) for i in range(len(pts))]
    P = [((x + dx) * scale, (y + dy) * scale) for x, y in pts]
    W = [w * scale for w in widths]
    dr.polygon(stroke_outline(P, W), fill=fill)
    if joins:
        for (x, y), w in zip(P[1:-1], W[1:-1]):
            r = w / 2
            dr.ellipse([x - r, y - r, x + r, y + r], fill=fill)
    if caps:
        for i in (0, -1):
            x, y = P[i]; r = W[i] / 2
            dr.ellipse([x - r, y - r, x + r, y + r], fill=fill)


K = 4
def render(fn, size, view=256.0, bg=(0, 0, 0, 0)):
    img = Image.new("RGBA", (size * K, size * K), bg)
    fn(ImageDraw.Draw(img), view, size * K)
    return img.resize((size, size), Image.LANCZOS)


# ================================================================ CONCEPT A
# THE RETURN — one continuous line: out, turn, back. The return is shorter than
# the going. Reads as V (the descent and the rise) and as a returning thread.
def make_A(dr, view, px, sw=26.0, x0=54.0, x1=202.0, y_go=182.0, y_back=100.0,
           ret=0.60, r=34.0):
    """Rounded 180deg turn at the right; the returning arm stops short."""
    cx, cy = x1 - r, (y_go + y_back) / 2.0
    # going arm (bottom), turn, returning arm (top)
    turn = arc_pts(cx, cy, r, 90, -90, n=90)     # right-hand semicircle
    ret_end = x0 + (x1 - x0) * (1 - ret)
    pts = ([(x0, y_go)] + turn + [(ret_end, y_back)])
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# ================================================================ CONCEPT B
# THE VO — lowercase v and o joined by one unbroken stroke: the v's rising arm
# runs straight into the o's circle. "vo" is the operative part of the name.
def make_B(dr, view, px, sw=26.0):
    R = 40.0
    ocx, ocy = 176.0, 116.0
    vertex = (112.0, 174.0)
    top_l  = (66.0, 74.0)
    a_start = 143.0                                   # where the arm meets the o
    lx = ocx + R * math.cos(math.radians(a_start))
    ly = ocy + R * math.sin(math.radians(a_start))
    o_pts = arc_pts(ocx, ocy, R, a_start, a_start + 360, n=200)
    pts = [top_l, vertex, (lx, ly)] + o_pts
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# ================================================================ CONCEPT C
# THE WITNESS — a ring of witnessed spans. None equal, two gaps of different
# widths. The ring is nearly closed: the record is nearly complete.
def make_C(dr, view, px, R=103.0, sw=26.0, rot=196.0):
    spans = [61.0, 33.0, 47.0, 29.0, 55.0]
    gaps  = [11.0, 8.0, 14.0, 9.0]
    a = rot
    for i, s in enumerate(spans):
        draw_stroke(dr, arc_pts(128, 128, R, a, a + s, n=60), sw,
                    (0, 0, 0, 255), caps=False, joins=False, scale=px / view)
        a += s
        if i < len(gaps):
            a += gaps[i]


CANDIDATES = [("A-return", make_A, {}), ("B-vo", make_B, {}), ("C-witness", make_C, {})]


# ------------------------------------------------------------------ sheets
def contact_sheet(items, cols, cell, out, bg=(18, 20, 21, 255), gap=16, label=True):
    rows = (len(items) + cols - 1) // cols
    lh = 24 if label else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sheet = Image.new("RGBA", (W, H), bg)
    dr = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(items):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap); y = gap + r * (cell + lh + gap)
        tile = Image.new("RGBA", (cell, cell), (255, 255, 255, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sheet.alpha_composite(tile, (x, y))
        if label: dr.text((x + 2, y + cell + 4), name, fill=(226, 228, 224, 255))
    sheet.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    big, tiny = [], []
    for name, fn, kw in CANDIDATES:
        mk = (lambda f=fn, k=kw: (lambda dr, view, px: f(dr, view, px, **k)))()
        for s in (512, 128, 64, 32, 16):
            render(mk, s).save(f"renders/final-{name}-{s}.png")
        big.append((name, render(mk, 288)))
        tiny.append((name, render(mk, 16)))
    contact_sheet(big, 3, 288, "renders/sheet-final-288.png")
    contact_sheet(tiny, 3, 112, "renders/sheet-final-16.png")
    print("done")
