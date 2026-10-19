#!/usr/bin/env python3
"""THE LIGATURE — v and o as one continuous monoline stroke.

Key construction: the v's rising arm is TANGENT to the o's centreline circle.
That makes the arm's two edges tangent to the o's outer circle and its counter
in the same instant, so the arm blends in with no junction, no wedge, and the
counter stays a perfect circle.

    Rc   o centreline radius        R_out = Rc + w/2
    w    stroke width               R_in  = Rc - w/2
    V    the v's vertex (centreline)
    T    the tangency point (V->T is tangent to the Rc circle)

Everything is derived from V and the left arm's lean, so the letterform is
described the way a designer describes it.
"""
import math, os
from PIL import Image, ImageDraw, ImageFont

K = 6
BLACK = (0, 0, 0, 255)
CLEAR = (0, 0, 0, 0)
CARAMEL = (200, 155, 107, 255)
CHARCOAL = (21, 24, 26, 255)
INK = (236, 234, 230, 255)
PAPER = (244, 243, 240, 255)


# ------------------------------------------------------------------ geometry
def build(vx=-132.0, vy=100.0, py=-100.0, lam=26.0, w=78.0, Rc=100.0, mitre=False):
    R_out, R_in = Rc + w / 2.0, Rc - w / 2.0
    V = (vx, vy)
    d = math.hypot(*V)
    delta = math.acos(Rc / d)
    alpha = math.atan2(V[1], V[0])
    cands = []
    for s in (1, -1):
        ang = alpha + s * delta
        cands.append(((Rc * math.cos(ang), Rc * math.sin(ang)), ang))
    T, a = min(cands, key=lambda c: c[0][1])        # the upper tangent point
    assert abs((T[0] - V[0]) * T[0] + (T[1] - V[1]) * T[1]) < 1e-6, "not tangent"
    ur = math.radians(lam)
    u = (-math.sin(ur), -math.cos(ur))              # left arm, up and to the left
    H = (vy - py) / math.cos(ur)
    P = (V[0] + H * u[0], V[1] + H * u[1])
    return dict(R_out=R_out, R_in=R_in, Rc=Rc, V=V, T=T, P=P, u=u, w=w,
                mitre=mitre, ang=math.degrees(a),
                apex=math.degrees(math.acos(
                    ((T[0]-V[0]) * u[0] + (T[1]-V[1]) * u[1]) /
                    (math.hypot(T[0]-V[0], T[1]-V[1])))))


def _quad(p, q, w):
    vx, vy = q[0] - p[0], q[1] - p[1]
    L = math.hypot(vx, vy) or 1.0
    nx, ny = -vy / L * w / 2.0, vx / L * w / 2.0
    return [(p[0] + nx, p[1] + ny), (q[0] + nx, q[1] + ny),
            (q[0] - nx, q[1] - ny), (p[0] - nx, p[1] - ny)]


def _disc(c, r, n=72):
    return [(c[0] + r * math.cos(2 * math.pi * i / n),
             c[1] + r * math.sin(2 * math.pi * i / n)) for i in range(n)]


def _stroke_outline(pts, w):
    """Mitred stroke of a polyline: averaged normals, perpendicular end cuts."""
    T = []
    for i in range(len(pts)):
        a = pts[max(i - 1, 0)]; b = pts[min(i + 1, len(pts) - 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy) or 1.0
        T.append((dx / L, dy / L))
    N = [(-t[1], t[0]) for t in T]
    up = [(p[0] + n[0] * w / 2, p[1] + n[1] * w / 2) for p, n in zip(pts, N)]
    lo = [(p[0] - n[0] * w / 2, p[1] - n[1] * w / 2) for p, n in zip(pts, N)]
    return [up + list(reversed(lo))]


def arm_polys(g):
    """Arm as polygons (+ round join/cap discs, or one mitred outline)."""
    P, V, T, w = g["P"], g["V"], g["T"], g["w"]
    if g["mitre"]:
        return _stroke_outline([P, V, T], w)
    return [_quad(P, V, w), _quad(V, T, w), _disc(P, w / 2.0), _disc(V, w / 2.0)]


def bbox(g):
    w, P, V = g["w"], g["P"], g["V"]
    x0 = min(P[0], V[0]) - w - g["R_out"] * 0
    x0 = P[0] - w / 2.0
    x1 = g["R_out"]
    y0 = min(P[1], -g["R_out"]) - w / 2.0
    y1 = max(V[1], g["R_out"]) + g["w"]
    return x0, y0, x1, y1


# ------------------------------------------------------------------ raster
def render(size, bg=CLEAR, colour=BLACK, frac=0.86, **kw):
    g = build(**kw)
    x0, y0, x1, y1 = bbox(g)
    bw, bh = x1 - x0, y1 - y0
    pad = max(bw, bh) * (1 - frac) / 2.0
    x0 -= pad; y0 -= pad; x1 += pad; y1 += pad
    bw, bh = x1 - x0, y1 - y0
    s = size / max(bw, bh)

    img = Image.new("RGBA", (size, size), bg)
    dr = ImageDraw.Draw(img)
    ox, oy = -x0 * s, -y0 * s

    def T(p): return (ox + p[0] * s, oy + p[1] * s)

    # 1. the o as a solid disc (the counter is cut last)
    dr.ellipse([T((-g["R_out"], -g["R_out"]))[0], T((-g["R_out"], -g["R_out"]))[1],
                T((g["R_out"], g["R_out"]))[0], T((g["R_out"], g["R_out"]))[1]],
               fill=colour)
    # 2. the arm
    for poly in arm_polys(g):
        dr.polygon([T(p) for p in poly], fill=colour)
    # 3. cut the counter (the arm is tangent to it, never inside it)
    dr.ellipse([T((-g["R_in"], -g["R_in"]))[0], T((-g["R_in"], -g["R_in"]))[1],
                T((g["R_in"], g["R_in"]))[0], T((g["R_in"], g["R_in"]))[1]],
               fill=CLEAR)
    return img


def render_hi(size, **kw):
    return render(size * K, **kw).resize((size, size), Image.LANCZOS)


# ------------------------------------------------------------------ svg
def svg_text(**kw):
    g = build(**kw)
    w = g["w"]; R_out, R_in = g["R_out"], g["R_in"]
    P, V, T = g["P"], g["V"], g["T"]
    x0, y0, x1, y1 = bbox(g)
    x0 -= w / 4; y0 -= w / 4; x1 += w / 4; y1 += w / 4
    W, H = x1 - x0, y1 - y0

    def S(p): return (p[0] - x0, p[1] - y0)

    Ps, Vs, Ts = S(P), S(V), S(T)
    c = S((0.0, 0.0))
    ann = (f"M {c[0]-R_out:.4f} {c[1]:.4f} "
           f"A {R_out:.4f} {R_out:.4f} 0 1 0 {c[0]+R_out:.4f} {c[1]:.4f} "
           f"A {R_out:.4f} {R_out:.4f} 0 1 0 {c[0]-R_out:.4f} {c[1]:.4f} Z "
           f"M {c[0]-R_in:.4f} {c[1]:.4f} "
           f"A {R_in:.4f} {R_in:.4f} 0 1 1 {c[0]+R_in:.4f} {c[1]:.4f} "
           f"A {R_in:.4f} {R_in:.4f} 0 1 1 {c[0]-R_in:.4f} {c[1]:.4f} Z")
    arm = (f"M {Ps[0]:.4f} {Ps[1]:.4f} L {Vs[0]:.4f} {Vs[1]:.4f} "
           f"L {Ts[0]:.4f} {Ts[1]:.4f}")
    join = "miter" if g["mitre"] else "round"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W:.4f} {H:.4f}" '
            f'width="{W:.4f}" height="{H:.4f}" role="img" aria-label="Evo">\n'
            f'  <title>Evo</title>\n'
            f'  <g fill="#000000">\n'
            f'    <path d="{ann}" fill-rule="evenodd"/>\n'
            f'  </g>\n'
            f'  <path d="{arm}" fill="none" stroke="#000000" stroke-width="{w:.4f}" '
            f'stroke-linecap="round" stroke-linejoin="{join}"/>\n'
            f'</svg>\n'), (W, H)


# ------------------------------------------------------------------ sheet
def sheet(items, cols, cell, out, labels=None, bg=(20, 23, 25, 255), gap=16, dark=False):
    try: f = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", 13)
    except Exception: f = ImageFont.load_default()
    rows = (len(items) + cols - 1) // cols
    lh = 20 if labels else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sh = Image.new("RGBA", (W, H), bg); dr = ImageDraw.Draw(sh)
    for i, img in enumerate(items):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap); y = gap + r * (cell + lh + gap)
        tile = Image.new("RGBA", (cell, cell), (246, 246, 244, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sh.alpha_composite(tile, (x, y))
        if labels: dr.text((x + 2, y + cell + 3), labels[i], fill=(206, 209, 205, 255), font=f)
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    items, labels = [], []
    for vx in (-118.0, -132.0, -148.0):
        items.append(render_hi(300, vx=vx)); labels.append(f"vx {int(vx)}")
    for lam in (20.0, 26.0, 32.0):
        items.append(render_hi(300, lam=lam)); labels.append(f"lean {int(lam)}deg")
    for w in (66.0, 78.0, 90.0):
        items.append(render_hi(300, w=w)); labels.append(f"stroke {int(w)}")
    items.append(render_hi(300, mitre=True)); labels.append("mitred vertex")
    sheet(items, 4, 300, "renders/vo-study.png", labels=labels)

    tiny, tl = [], []
    for nm, kw in (("base", {}), ("vx-148", dict(vx=-148.0)),
                   ("w90", dict(w=90.0)), ("lean32", dict(lam=32.0)),
                   ("mitre", dict(mitre=True))):
        tiny.append(render_hi(16, **kw)); tl.append(nm)
    sheet(tiny, 5, 110, "renders/vo-16.png", labels=tl)
