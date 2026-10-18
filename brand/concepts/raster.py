#!/usr/bin/env python3
"""Fast analytic rasteriser for the Evo concepts.

Strokes are converted to filled outlines (offset polygon + round joins/caps)
and drawn with PIL's C-level polygon/ellipse fill. Geometry is exact, so the
16 px frames are a fair test of the real vector.
"""
import math, os
from PIL import Image, ImageDraw

PI = math.pi


# ------------------------------------------------------------------ vector ops
def tangents(pts, closed=False):
    n = len(pts)
    out = []
    for i in range(n):
        if closed:
            a, b = pts[(i - 1) % n], pts[(i + 1) % n]
        else:
            a = pts[max(i - 1, 0)]
            b = pts[min(i + 1, n - 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy) or 1.0
        out.append((dx / L, dy / L))
    return out


def stroke_outline(pts, widths, closed=False):
    """Round-join, round-cap stroke -> single outline polygon (y-down)."""
    T = tangents(pts, closed)
    N = [(-t[1], t[0]) for t in T]
    up = [(p[0] + n[0] * w / 2, p[1] + n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    lo = [(p[0] - n[0] * w / 2, p[1] - n[1] * w / 2) for p, n, w in zip(pts, N, widths)]
    if closed:
        return up + list(reversed(lo))
    return up + list(reversed(lo))


def arc_pts(cx, cy, r, a0, a1, n=180):
    a0, a1 = math.radians(a0), math.radians(a1)
    if a1 < a0:
        a1 += 2 * PI
    return [(cx + r * math.cos(a0 + (a1 - a0) * i / n),
             cy + r * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


def spiral_pts(r0, k, th0, th1, n=260):
    out = []
    for i in range(n + 1):
        th = math.radians(th0 + (th1 - th0) * i / n)
        r = r0 * k ** (th / (2 * PI))
        out.append((r * math.cos(th), r * math.sin(th)))
    return out


# ------------------------------------------------------------------ drawing
def draw_stroke(dr, pts, widths, fill, closed=False, caps=True, joins=True,
                scale=1.0, dx=0.0, dy=0.0):
    if callable(widths):
        widths = [widths(i / (len(pts) - 1)) for i in range(len(pts))]
    if isinstance(widths, (int, float)):
        widths = [float(widths)] * len(pts)
    P = [((x + dx) * scale, (y + dy) * scale) for x, y in pts]
    W = [w * scale for w in widths]
    poly = stroke_outline(P, W, closed)
    dr.polygon(poly, fill=fill)
    if joins and not closed:
        for (x, y), w in zip(P[1:-1], W[1:-1]):
            rr = w / 2
            dr.ellipse([x - rr, y - rr, x + rr, y + rr], fill=fill)
    if caps and not closed:
        for idx in (0, -1):
            x, y = P[idx]; rr = W[idx] / 2
            dr.ellipse([x - rr, y - rr, x + rr, y + rr], fill=fill)
    return poly


def new_canvas(size, bg=(0, 0, 0, 0)):
    return Image.new("RGBA", (size, size), bg)


def save(img, name, bg=None):
    if bg is not None:
        flat = Image.new("RGBA", img.size, bg)
        flat.alpha_composite(img)
        img = flat
    img.save(name)
    print("wrote", name)


# ------------------------------------------------------------------ concepts
def fold(beta=96.0, stroke=26.0):
    """One unbroken stroke: a ring folded at a single vertex."""
    phi = (180.0 - beta) / 2.0
    f = 1.0 / math.cos(math.radians(phi))
    R = (256.0 - stroke) / (1.0 + f) - 6.0
    H = R * (1 + f) + stroke
    cy = (256.0 - H) / 2.0 + R + stroke / 2
    cx = 128.0
    tr = (cx + R * math.cos(math.radians(90 + phi)), cy + R * math.sin(math.radians(90 + phi)))
    tl = (cx - R * math.sin(math.radians(phi)), cy + R * math.cos(math.radians(phi)))
    v = (cx, cy + R * f)
    arc = arc_pts(cx, cy, R, 90 + phi, 360 + 90 - phi, n=300)
    return arc + [v, tr], stroke


def thread(turns=1.18, r0=108.0, growth=0.58, w0=30.0, w1=15.0):
    p = spiral_pts(r0, growth, 10.0, 10.0 + 360.0 * turns, n=260)
    xs = [q[0] for q in p]; ys = [q[1] for q in p]
    dx = 128 - (min(xs) + max(xs)) / 2
    dy = 128 - (min(ys) + max(ys)) / 2
    p = [(x + dx, y + dy) for x, y in p]
    return p, ((lambda t: w0 + (w1 - w0) * t) if False else None)


def thread_widths(n, w0=30.0, w1=15.0):
    return [w0 + (w1 - w0) * i / n for i in range(n + 1)]


def keystone(R=100.0, stroke=26.0, slots=4, span=66.0):
    parts = []
    step = 360.0 / slots
    for i in range(1, slots):
        a0 = i * step + (step - span) / 2
        parts.append(arc_pts(128, 128, R, a0, a0 + span, n=90))
    return parts, stroke


def wordmark_parts():
    XH, s = 104.0, 26.0
    ro = (XH - s) / 2
    out = []
    ecx, ecy = ro + s / 2, XH / 2
    out.append(("arc", arc_pts(ecx, ecy, ro, 0, 58, n=90), s))
    out.append(("line", [(ecx + ro, ecy), (ecx - ro, ecy)], s))
    e_right = ecx + ro + s / 2
    aw = 42.0
    vcx = e_right + 24 + aw
    out.append(("line", [(vcx - aw, 0.0), (vcx, XH), (vcx + aw, 0.0)], s))
    ocx = vcx + aw + 24 + ro + s / 2 - 2
    out.append(("arc", arc_pts(ocx, XH / 2, ro, 0, 360, n=150), s))
    return out, ocx + ro + s / 2


# ------------------------------------------------------------------ main
def main():
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    K = 4  # supersample for antialiasing

    def render(fn, size, view=256.0, bg=(0, 0, 0, 0)):
        img = new_canvas(size * K, bg)
        dr = ImageDraw.Draw(img)
        fn(dr, view, size * K)
        return img.resize((size, size), Image.LANCZOS)

    # --- A: the fold
    def draw_a(dr, view, px, beta=96.0, stroke=26.0):
        pts, sw = fold(beta, stroke)
        draw_stroke(dr, pts, sw, (0, 0, 0, 255), caps=True, joins=True,
                    scale=px / view)

    # --- B: the thread
    def draw_b(dr, view, px):
        pts, _ = thread()
        n = len(pts)
        draw_stroke(dr, pts, thread_widths(n), (0, 0, 0, 255),
                    caps=True, joins=False, scale=px / view)

    # --- C: the keystone
    def draw_c(dr, view, px):
        parts, sw = keystone()
        for p in parts:
            draw_stroke(dr, p, sw, (0, 0, 0, 255), caps=False, joins=False,
                        scale=px / view)

    for name, fn in (("a-fold-v1", draw_a), ("b-thread-v1", draw_b), ("c-keystone-v1", draw_c)):
        for size in (512, 128, 64, 32, 16):
            save(render(fn, size), f"renders/{name}-{size}.png")
    for beta in (84.0, 96.0, 108.0):
        def mk(b):
            return lambda dr, view, px: draw_a(dr, view, px, beta=b)
        save(render(mk(beta), 320), f"renders/fold-b{int(beta)}-320.png")
    print("done")


if __name__ == "__main__":
    main()
