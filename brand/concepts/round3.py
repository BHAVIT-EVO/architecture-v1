#!/usr/bin/env python3
"""Round 3 — one-line marks. Build candidates + a single contact sheet."""
import math, os
from PIL import Image, ImageDraw

PI = math.pi


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


def stroke_outline(pts, widths, closed=False):
    T = tangents(pts, closed)
    N = [(-t[1], t[0]) for t in T]
    up = [(p[0] + n[0] * w / 2, p[1] + n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    lo = [(p[0] - n[0] * w / 2, p[1] - n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    return up + list(reversed(lo))


def arc_pts(cx, cy, r, a0, a1, n=180):
    a0, a1 = math.radians(a0), math.radians(a1)
    if a1 < a0: a1 += 2 * PI
    return [(cx + r * math.cos(a0 + (a1 - a0) * i / n),
             cy + r * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


def draw_stroke(dr, pts, widths, fill, closed=False, caps=False, joins=True,
                scale=1.0, dx=0.0, dy=0.0):
    if isinstance(widths, (int, float)):
        widths = [float(widths)] * len(pts)
    if callable(widths):
        widths = [widths(i / max(1, len(pts) - 1)) for i in range(len(pts))]
    P = [((x + dx) * scale, (y + dy) * scale) for x, y in pts]
    W = [w * scale for w in widths]
    dr.polygon(stroke_outline(P, W, closed), fill=fill)
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


# ============================================================ CANDIDATES

# --- C1: THE HAIRPIN — out, fold 180deg, back. Continuity folded, not broken.
def c1_hairpin(dr, view, px, sw=26.0, span=104.0, Len=118.0):
    """Two parallel strokes joined by a 180 turn; ends open and staggered."""
    r = span / 2
    cx, cy = 128.0, 128.0 - r + 6
    # turn at the top; strokes descend
    pts = arc_pts(cx, cy, r, 180, 0, n=120)
    pts = [(cx - r, cy)] + pts[0:]                      # left end at top-left
    left_end = (cx - r, cy + Len)
    right_end = (cx + r, cy + Len * 0.62)
    full = [(cx - r, cy + Len)] + pts + [(cx + r, cy + Len * 0.62)]
    draw_stroke(dr, full, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# --- C2: THE V-FOLD, deep — vertex well inside the ring, heavy weight.
def c2_deepV(dr, view, px, beta=74.0, R=112.0, sw=30.0):
    phi = (180.0 - beta) / 2.0
    f = 1.0 / math.cos(math.radians(phi))
    H = R * (1 + f) + sw
    cy = (256.0 - H) / 2.0 + R + sw / 2
    cx = 128.0
    tr = (cx + R * math.sin(math.radians(phi)), cy + R * math.cos(math.radians(phi)))
    v  = (cx, cy + R * f)
    arc = arc_pts(cx, cy, R, 90 + phi, 360 + 90 - phi, n=300)
    draw_stroke(dr, arc + [v, tr], sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# --- C3: VO LIGATURE — V and O as one unbroken monoline stroke.
def c3_vo(dr, view, px, sw=26.0):
    """v descends to a vertex; the rise continues and closes the o."""
    R = 42.0                       # o radius
    vertex = (108.0, 176.0)
    top    = (54.0, 78.0)          # v's left arm top
    # o centre
    ocx, ocy = 178.0, 118.0
    # v left arm: top -> vertex
    v_arm = [top, vertex]
    # from vertex, rise to the o's left tangent point, then close the o
    a_start = 150.0
    lx = ocx + R * math.cos(math.radians(a_start))
    ly = ocy + R * math.sin(math.radians(a_start))
    o_pts = arc_pts(ocx, ocy, R, a_start, a_start + 360, n=200)
    pts = [top, vertex, (lx, ly)] + o_pts
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# --- C4: THE WITNESSED ARC — an arc assembled from spans of unequal length.
def c4_witness(dr, view, px, R=104.0, sw=26.0):
    spans = [58, 34, 44, 30, 52]
    gaps  = [10, 8, 13, 9]
    a = 205.0
    for i, s in enumerate(spans):
        pts = arc_pts(128, 128, R, a, a + s, n=50)
        draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=False, joins=False, scale=px / view)
        a += s
        if i < len(gaps):
            a += gaps[i]


# --- C5: THE E-BAR — a single stroke folded three times (a minimal E).
def c5_ebar(dr, view, px, sw=26.0, h=150.0, w=118.0, x0=74.0, y0=53.0):
    """One continuous stroke: top bar out, down, bottom bar back, and a mid bar."""
    sl = w * 0.62
    pts = ([(x0 + w, y0), (x0, y0), (x0, y0 + h), (x0 + w, y0 + h)] +
           [(x0, y0 + h)] )
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)
    mid = [(x0, y0 + h / 2), (x0 + sl, y0 + h / 2)]
    draw_stroke(dr, mid, sw, (0, 0, 0, 255), caps=True, joins=False, scale=px / view)


# --- C6: THE THREAD, straightened — a line that folds back once, long and thin.
def c6_foldback(dr, view, px, sw=26.0):
    """Out along the bottom, up at the right, back along the top: a flattened Z."""
    y_bot, y_top = 186.0, 96.0
    x0, x1 = 52.0, 204.0
    pts = [(x0, y_bot), (x1, y_bot), (x1, y_top), (x0 + 34, y_top)]
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


CANDIDATES = [
    ("c1-hairpin", c1_hairpin, {}),
    ("c2-deepV",   c2_deepV,   {}),
    ("c3-vo",      c3_vo,      {}),
    ("c4-witness", c4_witness, {}),
    ("c5-ebar",    c5_ebar,    {}),
    ("c6-foldback",c6_foldback,{}),
]


def contact_sheet(items, cols, cell, out, bg=(22, 24, 26, 255), gap=18, label=True):
    rows = (len(items) + cols - 1) // cols
    lh = 26 if label else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sheet = Image.new("RGBA", (W, H), bg)
    dr = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(items):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap)
        y = gap + r * (cell + lh + gap)
        tile = Image.new("RGBA", (cell, cell), (255, 255, 255, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sheet.alpha_composite(tile, (x, y))
        if label:
            dr.text((x + 2, y + cell + 5), name, fill=(232, 234, 230, 255))
    sheet.convert("RGB").save(out)
    print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    big, small = [], []
    for name, fn, kw in CANDIDATES:
        mk = (lambda f=fn, k=kw: (lambda dr, view, px: f(dr, view, px, **k)))()
        big.append((name, render(mk, 256)))
        small.append((name, render(mk, 16)))
    contact_sheet(big, 3, 260, "renders/sheet-round3-256.png")
    contact_sheet(small, 6, 96, "renders/sheet-round3-16.png")
