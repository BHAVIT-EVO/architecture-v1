#!/usr/bin/env python3
"""Round 2 — kill the pin/swirl/spinner reads. New formal ideas."""
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
    if closed:
        return up + list(reversed(lo))
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
    dr = ImageDraw.Draw(img)
    fn(dr, view, size * K)
    return img.resize((size, size), Image.LANCZOS)


def save(img, name, bg=None):
    if bg is not None:
        f = Image.new("RGBA", img.size, bg); f.alpha_composite(img); img = f
    img.save(name); print("wrote", name)


# ======================================================= VARIANT 1
# OFF-CENTRE FOLD — a ring folded at 45 deg, not at the bottom. Kills the pin.
def v1_fold_offcentre(a_fold=45.0, beta=88.0, R=94.0, sw=26.0):
    """Fold is placed on the ring at compass `a_fold` instead of straight down."""
    half = (180.0 - beta) / 2.0
    # tangent points sit +/- half from the fold's anti-direction... place fold centre
    f = 1.0 / math.cos(math.radians(half))
    # the two tangency angles around the fold direction
    a1 = a_fold - half
    a2 = a_fold + half
    # vertex lies outward along a_fold at distance R*f from the ring CENTRE
    # (ring centre is offset from the mark centre)
    vx = R * f * math.cos(math.radians(a_fold))
    vy = R * f * math.sin(math.radians(a_fold))
    return dict(R=R, a1=a1, a2=a2, half=half, vx=vx, vy=vy, sw=sw)


def make_v1(dr, view, px, **kw):
    sw = kw.get("sw", 26.0)
    g = v1_fold_offcentre(**kw)
    R, a1, a2, half, sw = g["R"], g["a1"], g["a2"], g["half"], g["sw"]
    # ring centre at origin; build locally then recentre
    tr = (R * math.cos(math.radians(a1)), R * math.sin(math.radians(a1)))
    tl = (R * math.cos(math.radians(a2)), R * math.sin(math.radians(a2)))
    v  = (R * g["vx"] / (R * g["R"] if False else 1) * 0 + g["vx"], g["vy"]) if False else (g["vx"], g["vy"])
    arc = arc_pts(0, 0, R, a1, a2, n=300)     # the long way round
    pts = arc + [v, tr]
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    m = sw / 2
    w = (max(xs) + m) - (min(xs) - m); h = (max(ys) + m) - (min(ys) - m)
    dx = -((min(xs) - m) + (max(xs) + m)) / 2
    dy = -((min(ys) - m) + (max(ys) + m)) / 2
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True,
                scale=px / view, dx=dx, dy=dy)


# ======================================================= VARIANT 2
# THE TAPER — one continuous ring whose weight grows; butt splice at the top.
def make_v2(dr, view, px, R=100.0, wmin=12.0, wmax=40.0, start=270.0):
    n = 260
    pts = arc_pts(128, 128, R, start + 4, start + 360 - 4, n=n)
    def w(i):
        t = i / (n - 1)
        return wmin + (wmax - wmin) * t
    draw_stroke(dr, pts, w, (0, 0, 0, 255), caps=False, joins=False, scale=px / view)


# ======================================================= VARIANT 3
# THE NOTCH — a ring with an inward fold at the top (the interruption).
def make_v3(dr, view, px, R=104.0, sw=26.0, span=30.0, depth=52.0):
    apex = (128.0, 128.0 - R + depth)
    left  = arc_pts(128, 128, R, 270 + span, 270 + 360 - span, n=280)
    p_l = (128 + R * math.sin(math.radians(span)), 128 - R * math.cos(math.radians(span)))
    p_r = (128 - R * math.sin(math.radians(span)), 128 - R * math.cos(math.radians(span)))
    pts = [p_l] + left + [p_r, apex]
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True, scale=px / view)


# ======================================================= VARIANT 4
# THE STRAND — two strokes of different weights running as one band, folding.
def make_v4(dr, view, px, R=104.0, sw=22.0, gap=16.0, span=46.0):
    """Two concentric arcs, same centre, different radii: one thick, one thin."""
    inner = arc_pts(128, 128, R - gap / 2 - sw / 2, 270 + span, 270 + 360 - span, n=260)
    outer = arc_pts(128, 128, R + gap / 2 + sw / 2, 270 + span, 270 + 360 - span, n=260)
    draw_stroke(dr, inner, sw * 0.62, (0, 0, 0, 255), caps=True, joins=False, scale=px / view)
    draw_stroke(dr, outer, sw, (0, 0, 0, 255), caps=True, joins=False, scale=px / view)


# ======================================================= VARIANT 5
# THE HINGE — a squared band (rounded square) with one corner folded in.
def make_v5(dr, view, px, s=196.0, r=42.0, sw=26.0, fold=0.30):
    """Rounded square, but the bottom-left corner cuts inward to the centre-ish."""
    x0, y0, x1, y1 = 128 - s / 2, 128 - s / 2, 128 + s / 2, 128 + s / 2
    pts = []
    pts += arc_pts(x0 + r, y0 + r, r, 180, 270, n=40)          # top-left
    pts += [(x1 - r, y0)]
    pts += arc_pts(x1 - r, y0 + r, r, 270, 360, n=40)          # top-right
    pts += [(x1, y1 - r)]
    pts += arc_pts(x1 - r, y1 - r, r, 0, 90, n=40)             # bottom-right
    pts += [(x0 + r, y1)]
    # bottom-left corner folded inward
    k = s * fold
    pts += [(x0 + k, y1 - k)]
    pts += arc_pts(x0 + k, y1 - k, k * 0.5, 135, 225, n=30)
    pts += [(x0, y0 + r)]
    pts += arc_pts(x0 + r, y0 + r, r, 90, 180, n=40)           # back to start
    draw_stroke(dr, pts, sw, (0, 0, 0, 255), closed=True, joins=True, scale=px / view)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    tests = [
        ("v1-fold-45",  make_v1, dict(a_fold=45.0, beta=88.0, R=96.0)),
        ("v1-fold-135", make_v1, dict(a_fold=135.0, beta=88.0, R=96.0)),
        ("v1-fold-0",   make_v1, dict(a_fold=0.0, beta=88.0, R=96.0)),
        ("v2-taper",    make_v2, {}),
        ("v3-notch",    make_v3, {}),
        ("v3-notch-44", make_v3, dict(span=44.0, depth=40.0)),
        ("v4-strand",   make_v4, {}),
        ("v5-hinge",    make_v5, {}),
    ]
    for name, fn, kw in tests:
        def mk(f=fn, k=kw):
            return lambda dr, view, px: f(dr, view, px, **k)
        for size in (320, 16):
            save(render(mk(), size), f"renders/{name}-{size}.png")
    print("done")
