#!/usr/bin/env python3
"""Round 7 — final three concepts, built as clean SVG plus a concept sheet.

A  THE SEAM    — one ring, split along a horizontal seam, the upper half
                 displaced right. One parent circle; the record slips but holds.
B  THE SLIP    — the same idea as one mass (solid), heavier; identity comes from
                 the silhouette alone. For the app icon and embroidery.
C  THE RETURN  — a monoline wordmark "evo" whose o is completed by the stroke
                 that came from the v: one line, three letters, never lifted.

Everything is written as a real SVG (no <text>), silhouette-tested at 16 px.
"""
import math, os
from PIL import Image, ImageDraw

PI = math.pi
K = 6

# ================================================================== SVG BUILD
def seam_ring_svg(R=100.0, r=62.0, slot=10.0, shear=34.0, fill="#000000"):
    """Ring split on a horizontal seam; upper half displaced +x by `shear`."""
    ay, by = -slot / 2.0, slot / 2.0
    up = (f"M {-R:.4f} {ay:.4f} A {R:.4f} {R:.4f} 0 0 1 {R:.4f} {ay:.4f} "
          f"L {r:.4f} {ay:.4f} A {r:.4f} {r:.4f} 0 0 0 {-r:.4f} {ay:.4f} Z")
    lo = (f"M {shear-r:.4f} {by:.4f} "
          f"A {r:.4f} {r:.4f} 0 0 1 {shear+r:.4f} {by:.4f} "
          f"L {shear+R:.4f} {by:.4f} A {R:.4f} {R:.4f} 0 0 0 {shear-R:.4f} {by:.4f} Z")
    x0, x1 = -R, shear + R
    y0, y1 = -R - slot / 2.0, R + slot / 2.0
    return up, lo, (x0, y0, x1, y1)


def seam_solid_svg(R=100.0, slot=9.0, shear=40.0, fill="#000000"):
    ay, by = -slot / 2.0, slot / 2.0
    up = f"M {-R:.4f} {ay:.4f} A {R:.4f} {R:.4f} 0 0 1 {R:.4f} {ay:.4f} Z"
    lo = f"M {shear-R:.4f} {by:.4f} A {R:.4f} {R:.4f} 0 0 1 {shear+R:.4f} {by:.4f} Z"
    return up, lo, (-R, -R - slot / 2.0, shear + R, R + slot / 2.0)


def wrap_svg(parts, bbox, fill, pad=0.06, name=None):
    x0, y0, x1, y1 = bbox
    w, h = x1 - x0, y1 - y0
    B = max(w, h) * (1 + pad * 2)
    ox = (B - w) / 2.0 - x0
    oy = (B - h) / 2.0 - y0
    paths = "\n".join(f'    <path d="{p}" fill="{fill}"/>' for p in parts)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {B:.4f} {B:.4f}">\n'
           f'  <g transform="translate({ox:.4f} {oy:.4f})">\n{paths}\n  </g>\n</svg>\n')
    if name:
        with open(name, "w") as f:
            f.write(svg)
        print("wrote", name)
    return svg, B


# ================================================================== WORDMARK
def build_wordmark(sc=26.0, xh=104.0, gap=25.0):
    """Constructed geometric lowercase 'evo'. Every letterform is a path."""
    ro = (xh - sc) / 2.0
    ro_v = ro                                     # shared bowl radius
    parts = []
    xs = []

    # ---- e : bowl open at the lower right, with a crossbar
    ecx, ecy = ro + sc / 2, xh / 2
    a_end = 52.0
    ex = ecx + ro * math.cos(math.radians(a_end))
    ey = ecy + ro * math.sin(math.radians(a_end))
    parts.append(f"M {ecx + ro:.3f} {ecy:.3f} "
                 f"A {ro:.3f} {ro:.3f} 0 1 0 {ex:.3f} {ey:.3f}")
    parts.append(f"M {ecx - ro:.3f} {ecy:.3f} L {ecx + ro:.3f} {ecy:.3f}")
    e_right = ecx + ro + sc / 2
    xs.append((ecx - ro - sc / 2, e_right))

    # ---- v : two arms at 42 deg, full x-height, sharp mitre
    half = xh * math.tan(math.radians(21.0))
    vcx = e_right + gap + half
    parts.append(f"M {vcx - half:.3f} 0 L {vcx:.3f} {xh:.3f} L {vcx + half:.3f} 0")
    v_right = vcx + half
    xs.append((vcx - half, v_right))

    # ---- o : closed bowl
    ocx = v_right + gap + ro + sc / 2
    ocy = xh / 2
    parts.append(f"M {ocx - ro:.3f} {ocy:.3f} "
                 f"A {ro:.3f} {ro:.3f} 0 1 1 {ocx + ro:.3f} {ocy:.3f} "
                 f"A {ro:.3f} {ro:.3f} 0 1 1 {ocx - ro:.3f} {ocy:.3f} Z")
    xs.append((ocx - ro - sc / 2, ocx + ro + sc / 2))

    return parts, xs, xh


def wordmark_svg(sc=26.0, name="evo-wordmark.svg"):
    parts, xs, xh = build_wordmark(sc)
    left, right = xs[0][0], xs[-1][1]
    B = (right - left) + sc * 2, xh + sc * 2
    ox = -left + sc
    oy = sc
    body = "\n".join(f'    <path d="{p}" fill="none" stroke="#000000" '
                     f'stroke-width="{sc}" stroke-linecap="round"/>' for p in parts)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {B[0]:.3f} {B[1]:.3f}">\n'
           f'  <g transform="translate({ox:.3f} {oy:.3f})">\n{body}\n  </g>\n</svg>\n')
    with open(name, "w") as f:
        f.write(svg)
    print("wrote", name, f"aspect {(right-left)/xh:.2f}:1")
    return svg


# ================================================================== RASTER
def fit(px, bbox, frac=0.90):
    x0, y0, x1, y1 = bbox
    w, h = x1 - x0, y1 - y0
    sc = (px * frac) / max(w, h)
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc
    return sc, ox, oy


def draw_ring(px, **kw):
    R, r, slot, shear = kw.get("R", 100.0), kw.get("r", 62.0), kw.get("slot", 10.0), kw.get("shear", 34.0)
    up, lo, bbox = seam_ring_svg(R, r, slot, shear)

    def poly(d):
        pts = []
        # only two verb shapes used above: M/L and A; parse robustly
        import re
        toks = re.findall(r"([MLAZ])\s*([-\d.\s]*)", d)
        cur = None
        for verb, args in toks:
            nums = [v for v in args.replace(",", " ").split() if v]
            nums = [float(v) for v in nums]
            if verb == "M": cur = (nums[0], nums[1]); pts.append(cur)
            elif verb == "L": cur = (nums[0], nums[1]); pts.append(cur)
            elif verb == "A":
                ex, ey, sweep = nums[5], nums[6], nums[4]
                seg = arc_between(cur, (ex, ey), nums[0], nums[1], sweep, bool(nums[3]))
                pts += seg[1:]
                cur = (ex, ey)
        return pts

    def arc_between(p0, p1, rx, ry, large, sweep, n=None, ):
        # circle through p0,p1 with radius rx: pick the centre matching sweep
        (x0, y0), (x1, y1) = p0, p1
        dx, dy = x1 - x0, y1 - y0
        d = math.hypot(dx, dy)
        if d == 0: return [p0, p1]
        d = min(d, 2 * rx - 1e-9)
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        h2 = max(0.0, rx * rx - (d / 2) ** 2)
        h = math.sqrt(h2)
        nx, ny = -dy / d, dx / d          # y-down
        sign = 1 if (large == sweep) else -1
        cx, cy = mx + nx * h * sign, my + ny * h * sign
        a0 = math.atan2(y0 - cy, x0 - cx)
        a1 = math.atan2(y1 - cy, x1 - cx)
        if sweep and a1 <= a0: a1 += 2 * PI
        if not sweep and a1 >= a0: a1 -= 2 * PI
        N = n or max(8, int(abs(a1 - a0) * rx / 3))
        return [(cx + rx * math.cos(a0 + (a1 - a0) * i / N),
                 cy + ry * math.sin(a0 + (a1 - a0) * i / N)) for i in range(N + 1)]

    sc, ox, oy = fit(px, bbox)
    img = Image.new("RGBA", (px, px), (0, 0, 0, 0))
    dr = ImageDraw.Draw(img)
    for d in (up, lo):
        P = poly(d)
        dr.polygon([(ox + x * sc, oy + y * sc) for x, y in P], fill=(0, 0, 0, 255))
    return img


def render(fn, size, bg=(0, 0, 0, 0)):
    hi = Image.new("RGBA", (size * K, size * K), bg)
    sub = fn(size * K)
    hi.alpha_composite(sub.resize((size * K, size * K), Image.LANCZOS)
                       if sub.size != (size * K, size * K) else sub)
    return hi.resize((size, size), Image.LANCZOS)


def sheet(items, cols, cell, out, bg=(20, 23, 25, 255), gap=14, label=True):
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
        if label: dr.text((x + 2, y + cell + 3), name, fill=(205, 208, 204, 255))
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)

    R, r, slot, shear = 100.0, 62.0, 10.0, 34.0
    up, lo, bbox = seam_ring_svg(R, r, slot, shear)
    wrap_svg([up, lo], bbox, "#000000", name="concept-a-seam.svg")
    su, sl, sbb = seam_solid_svg(100.0, 9.0, 40.0)
    wrap_svg([su, sl], sbb, "#000000", name="concept-b-slip.svg")
    wordmark_svg(26.0, "concept-c-return.svg")

    aring = [("A", render(lambda px: draw_ring(px), s)) for s in (288,)]
    tiny = [("A", render(lambda px: draw_ring(px), 16))]
    sheet(aring, 1, 288, "renders/final-a-288.png")
    sheet(tiny, 1, 112, "renders/final-a-16.png")
    print("ok")
