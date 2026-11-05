#!/usr/bin/env python3
"""Round 12 — push the Seam: seam angle, slip amount, translation vs rotation."""
import math, os
from PIL import Image, ImageDraw

K = 6


def draw_seam2(dr, px, R=100.0, inner=62.0, slot=10.0, slip=34.0, ang=0.0,
               fill=(0, 0, 0, 255), frac=0.90):
    """Ring split on a seam at `ang` degrees; the far half slips by `slip`
    ALONG the seam. ang is measured clockwise from horizontal."""
    a = math.radians(ang)
    se = (math.cos(a), math.sin(a))          # along the seam
    nn = (-se[1], se[0])                     # normal to the seam

    def rot(p):
        return (p[0] * se[0] - p[1] * se[1], p[0] * se[1] + p[1] * se[0])

    # build in seam-local coords (seam = x axis), then rotate to `ang`
    def arc_pts_local(R_, a0, a1, n):
        return [(R_ * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
                 R_ * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]

    s2 = slot / 2.0
    # upper half: outer arc from 180..360 at y=-s2, inner arc back
    upo = [(x, -math.sqrt(max(0.0, R * R - x * x)) - s2)
           for x in [(-R + 2 * R * i / 260) for i in range(261)]]
    upi = [(x, -math.sqrt(max(0.0, inner * inner - x * x)) - s2)
           for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]
    loo = [(slip + x, math.sqrt(max(0.0, R * R - x * x)) + s2)
           for x in [(-R + 2 * R * i / 260) for i in range(261)]]
    loi = [(slip + x, math.sqrt(max(0.0, inner * inner - x * x)) + s2)
           for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]

    # half-extent of the mark along x and y, in local units
    half_x = R + abs(slip) / 2.0
    half_y = R + slot / 2.0
    ext = max(half_x, half_y) * 2.0
    sc = (px * frac) / ext
    c = px / 2.0

    def poly(pts):
        dr.polygon([(c + rot(p)[0] * sc, c + rot(p)[1] * sc) for p in pts], fill=fill)

    poly(upo + list(reversed(upi)))
    poly(loo + list(reversed(loi)))


def rot_seam(dr, px, R=100.0, inner=62.0, slot=10.0, tw=13.0,
             fill=(0, 0, 0, 255), frac=0.88):
    """Instead of translating, ROTATE the two halves about the centre by +-tw."""
    s2 = slot / 2.0

    def band(a0, a1, n=200):
        o = [((R) * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
              (R) * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]
        ii = [((inner) * math.cos(math.radians(a1 + (a0 - a1) * i / n)),
               (inner) * math.sin(math.radians(a1 + (a0 - a1) * i / n))) for i in range(n + 1)]
        return o + ii

    sc = (px * frac) / (2 * R)
    c = px / 2.0
    for half, sign in ((0, 1), (1, -1)):
        a0 = -90 + (s2 / R) * 180 / math.pi if half == 0 else 90 + (s2 / R) * 180 / math.pi
        a1 = a0 + 180 - 2 * (s2 / R) * 180 / math.pi
        pts = band(a0, a1)
        twr = math.radians(tw * sign)
        dr.polygon([(c + (p[0] * math.cos(twr) - p[1] * math.sin(twr)) * sc,
                     c + (p[0] * math.sin(twr) + p[1] * math.cos(twr)) * sc)
                    for p in pts], fill=fill)


def render(fn, size):
    hi = Image.new("RGBA", (size * K, size * K), (0, 0, 0, 0))
    fn(ImageDraw.Draw(hi), size * K)
    return hi.resize((size, size), Image.LANCZOS)


def sheet(cells, cols, cell, out, labels=None, bg=(20, 23, 25, 255), gap=16):
    from PIL import ImageFont
    try: f = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", 13)
    except Exception: f = ImageFont.load_default()
    rows = (len(cells) + cols - 1) // cols
    lh = 20 if labels else 0
    W = cols * cell + (cols + 1) * gap
    H = rows * (cell + lh) + (rows + 1) * gap
    sh = Image.new("RGBA", (W, H), bg); dr = ImageDraw.Draw(sh)
    for i, img in enumerate(cells):
        r, c = divmod(i, cols)
        x = gap + c * (cell + gap); y = gap + r * (cell + lh + gap)
        t = Image.new("RGBA", (cell, cell), (246, 246, 244, 255))
        t.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sh.alpha_composite(t, (x, y))
        if labels: dr.text((x + 2, y + cell + 3), labels[i], fill=(206, 209, 205, 255), font=f)
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    v = []
    lbl = []
    # seam angle study
    for ang in (0.0, -8.0, 8.0, -14.0):
        v.append(render(lambda d, p, a=ang: draw_seam2(d, p, ang=a), 200))
        lbl.append(f"angle {ang:+.0f}deg")
    # slip study
    for sl in (22.0, 34.0, 46.0):
        v.append(render(lambda d, p, s=sl: draw_seam2(d, p, slip=s), 200))
        lbl.append(f"slip {int(sl)}")
    # slot study
    for st in (6.0, 10.0, 15.0):
        v.append(render(lambda d, p, s=st: draw_seam2(d, p, slot=s), 200))
        lbl.append(f"slot {int(st)}")
    # rotational shear
    for tw in (8.0, 13.0, 18.0):
        v.append(render(lambda d, p, t=tw: rot_seam(d, p, tw=t), 200))
        lbl.append(f"rot {int(tw)}deg")
    sheet(v, 4, 200, "renders/r12-seam-study.png", labels=lbl)
    # 16px for the leading candidates
    tiny = []
    tl = []
    for nm, fn in (("ang-8", lambda d, p: draw_seam2(d, p, ang=-8.0)),
                   ("ang0",  lambda d, p: draw_seam2(d, p, ang=0.0)),
                   ("slip46", lambda d, p: draw_seam2(d, p, slip=46.0)),
                   ("slot15", lambda d, p: draw_seam2(d, p, slot=15.0)),
                   ("rot13", lambda d, p: rot_seam(d, p, tw=13.0))):
        tiny.append(render(fn, 16)); tl.append(nm)
    sheet(tiny, 5, 110, "renders/r12-seam-16.png", labels=tl)
