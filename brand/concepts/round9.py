#!/usr/bin/env python3
"""Round 9 — FINAL ARTWORK.

  A  THE SEAM   — a ring divided by one seam; the upper half slips along it.
  B  THE SLIP   — the same argument as one solid mass (icon / embroidery cut).
  C  THE THREAD — monoline wordmark 'evo' with a constructed e, v, o.
Plus the horizontal lockup. Writes real SVG, then a presentation sheet.
"""
import math, os
from PIL import Image, ImageDraw

K = 6
INK = "#000000"

# ============================================================== SVG AUTHORING
def seam_paths(R=100.0, r=62.0, slot=10.0, shear=34.0):
    s2 = slot / 2.0
    up = (f"M {-R:.3f} {-s2:.3f} A {R:.3f} {R:.3f} 0 0 1 {R:.3f} {-s2:.3f} "
          f"L {r:.3f} {-s2:.3f} A {r:.3f} {r:.3f} 0 0 0 {-r:.3f} {-s2:.3f} Z")
    lo = (f"M {shear-R:.3f} {s2:.3f} A {R:.3f} {R:.3f} 0 0 0 {shear+R:.3f} {s2:.3f} "
          f"L {shear+r:.3f} {s2:.3f} A {r:.3f} {r:.3f} 0 0 1 {shear-r:.3f} {s2:.3f} Z")
    return [up, lo], (-R, -R - s2, shear + R, R + s2)


def slip_paths(R=100.0, slot=9.0, shear=40.0):
    s2 = slot / 2.0
    up = f"M {-R:.3f} {-s2:.3f} A {R:.3f} {R:.3f} 0 0 1 {R:.3f} {-s2:.3f} Z"
    lo = (f"M {shear-R:.3f} {s2:.3f} A {R:.3f} {R:.3f} 0 0 0 {shear+R:.3f} {s2:.3f} Z")
    return [up, lo], (-R, -R - s2, shear + R, R + s2)


def write_symbol(paths, bbox, name, pad=0.055):
    x0, y0, x1, y1 = bbox
    w, h = x1 - x0, y1 - y0
    B = max(w, h) * (1 + pad * 2)
    ox = (B - w) / 2.0 - x0
    oy = (B - h) / 2.0 - y0
    body = "\n".join(f'    <path d="{p}" fill="{INK}"/>' for p in paths)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {B:.3f} {B:.3f}" '
           f'width="{B:.3f}" height="{B:.3f}">\n'
           f'  <g transform="translate({ox:.3f} {oy:.3f})">\n{body}\n  </g>\n</svg>\n')
    open(name, "w").write(svg); print("wrote", name); return svg, B


# ----- wordmark: monoline geometric 'evo', every letter constructed
def wordmark_geometry(sw=26.0, xh=104.0, gap=27.0):
    ro = (xh - sw) / 2.0
    g = {}
    ecx, ecy = ro + sw / 2, xh / 2
    a0, a1 = 55.0, 385.0
    p0 = (ecx + ro * math.cos(math.radians(a0)), ecy + ro * math.sin(math.radians(a0)))
    p1 = (ecx + ro * math.cos(math.radians(a1)), ecy + ro * math.sin(math.radians(a1)))
    g["e_bowl"] = (f"M {p0[0]:.3f} {p0[1]:.3f} A {ro:.3f} {ro:.3f} 0 1 1 "
                   f"{p1[0]:.3f} {p1[1]:.3f}")
    g["e_bar"] = f"M {ecx-ro:.3f} {ecy:.3f} L {ecx+ro:.3f} {ecy:.3f}"
    e_right = ecx + ro + sw / 2
    half = xh * math.tan(math.radians(21.0))
    vcx = e_right + gap + half
    g["v"] = f"M {vcx-half:.3f} 0 L {vcx:.3f} {xh:.3f} L {vcx+half:.3f} 0"
    v_right = vcx + half
    ocx = v_right + gap + ro + sw / 2
    ro2 = ro - 0.5
    g["o"] = (f"M {ocx-ro2:.3f} {xh/2:.3f} "
              f"A {ro2:.3f} {ro2:.3f} 0 1 1 {ocx+ro2:.3f} {xh/2:.3f} "
              f"A {ro2:.3f} {ro2:.3f} 0 1 1 {ocx-ro2:.3f} {xh/2:.3f} Z")
    g["_bbox"] = (ecx - ro - sw / 2, 0.0, ocx + ro2 + sw / 2, xh)
    g["_sw"], g["_xh"] = sw, xh
    return g


def write_wordmark(name="concept-c-thread.svg", sw=26.0):
    g = wordmark_geometry(sw)
    x0, _, x1, xh = g["_bbox"]
    m = sw * 1.2
    W, H = (x1 - x0) + m * 2, xh + m * 2
    ox, oy = -x0 + m, m
    parts = []
    for k in ("e_bowl", "e_bar", "v"):
        parts.append(f'    <path d="{g[k]}" fill="none" stroke="{INK}" '
                     f'stroke-width="{sw}" stroke-linecap="butt" stroke-linejoin="miter"/>')
    parts.append(f'    <path d="{g["o"]}" fill="none" stroke="{INK}" '
                 f'stroke-width="{sw}" stroke-linejoin="round"/>')
    body = "\n".join(parts)
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W:.3f} {H:.3f}" '
           f'width="{W:.3f}" height="{H:.3f}">\n'
           f'  <g transform="translate({ox:.3f} {oy:.3f})">\n{body}\n  </g>\n</svg>\n')
    open(name, "w").write(svg); print("wrote", name, f"aspect {(x1-x0)/xh:.2f}:1")
    return svg, (x1 - x0, xh)


# ============================================================== RASTER
def fit(px, bbox, frac, box_w=None, box_h=None):
    x0, y0, x1, y1 = bbox
    w, h = x1 - x0, y1 - y0
    sc = ((box_w or px) * frac) / w if box_w else (px * frac) / max(w, h)
    if box_w:
        sc = min(sc, (box_h * frac) / h)
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc
    return sc, ox, oy


def draw_seam(dr, px, R=100.0, inner=62.0, slot=10.0, shear=34.0, fill=(0, 0, 0, 255)):
    x0, x1 = -R, shear + R
    y0, y1 = -R - slot / 2.0, R + slot / 2.0
    w, h = x1 - x0, y1 - y0
    sc = (px * 0.90) / max(w, h)
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc
    s2 = slot / 2.0

    def poly(pts): dr.polygon([(ox + p[0] * sc, oy + p[1] * sc) for p in pts], fill=fill)

    N = 260
    upo = [(x, -math.sqrt(max(0.0, R * R - x * x)) - s2)
           for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
    loo = [(shear + x, math.sqrt(max(0.0, R * R - x * x)) + s2)
           for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
    if inner is None:
        poly(upo); poly(loo)
    else:
        r2 = inner * inner
        upi = [(x, -math.sqrt(max(0.0, r2 - x * x)) - s2)
               for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]
        loi = [(shear + x, math.sqrt(max(0.0, r2 - x * x)) + s2)
               for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]
        poly(upo + list(reversed(upi)))
        poly(loo + list(reversed(loi)))


def draw_thread(dr, px, fill=(0, 0, 0, 255), sw=26.0):
    """Same construction as the SVG, rasterised: butt caps, mitre joins."""
    g = wordmark_geometry(sw)
    x0, _, x1, xh = g["_bbox"]
    m = sw * 1.2
    W, H = (x1 - x0) + m * 2, xh + m * 2
    ox, oy = -x0 + m, m
    sc = (px * 0.94) / max(W, H)
    dx = (px - W * sc) / 2.0
    dy = (px - H * sc) / 2.0

    def T(p): return ((ox + p[0]) * sc + dx, (oy + p[1]) * sc + dy)
    def seg(p, q, w):
        vx, vy = q[0] - p[0], q[1] - p[1]
        L = math.hypot(vx, vy) or 1.0
        nx, ny = -vy / L * w / 2, vx / L * w / 2
        return [T((p[0] + nx, p[1] + ny)), T((q[0] + nx, q[1] + ny)),
                T((q[0] - nx, q[1] - ny)), T((p[0] - nx, p[1] - ny))]

    def arcband(c, rr, a0, a1, w, n=200):
        lo_, hi_ = [], []
        for i in range(n + 1):
            a = math.radians(a0 + (a1 - a0) * i / n)
            for lst, off in ((hi_, w / 2), (lo_, -w / 2)):
                lst.append(T((c[0] + (rr + off) * math.cos(a), c[1] + (rr + off) * math.sin(a))))
        return hi_ + list(reversed(lo_))

    ro = (xh - sw) / 2.0
    ecx, ecy = ro + sw / 2, xh / 2
    dr.polygon(arcband((ecx, ecy), ro, 55.0, 385.0, sw), fill=fill)
    dr.polygon(seg((ecx - ro, ecy), (ecx + ro, ecy), sw), fill=fill)
    e_right = ecx + ro + sw / 2
    half = xh * math.tan(math.radians(21.0))
    vcx = e_right + 27.0 + half
    dr.polygon(seg((vcx - half, 0.0), (vcx, xh), sw), fill=fill)
    dr.polygon(seg((vcx, xh), (vcx + half, 0.0), sw), fill=fill)
    v_right = vcx + half
    ocx = v_right + 27.0 + ro + sw / 2
    ro2 = ro - 0.5
    dr.polygon(arcband((ocx, xh / 2), ro2, 0, 360, sw, n=260), fill=fill)


def render(fn, size):
    hi = Image.new("RGBA", (size * K, size * K), (0, 0, 0, 0))
    fn(ImageDraw.Draw(hi), size * K)
    return hi.resize((size, size), Image.LANCZOS)


# ============================================================== SHEET
def sheet(rows, out, bg=(19, 22, 24, 255), pad=34, gap=22, title=None,
          labels=None, cell=300, tiny=110):
    """rows: list of list of RGBA tiles (already sized to cell or tiny)."""
    from PIL import ImageFont
    try:
        font = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", 15)
        fsm = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", 12)
    except Exception:
        font = fsm = None
    colw = [cell if r[1] else tiny for r in [(0, True)]]
    widths = []
    for row in rows:
        widths.append(sum(t.size[0] for t in row) + gap * (len(row) - 1))
    W = max(widths) + pad * 2
    H = pad * 2 + sum(max(t.size[1] for t in row) for row in rows) + gap * (len(rows) - 1) + (46 if labels else 0)
    sh = Image.new("RGBA", (W, H), bg)
    dr = ImageDraw.Draw(sh)
    y = pad
    for ri, row in enumerate(rows):
        x = (W - widths[ri]) / 2.0
        for tile in row:
            sh.alpha_composite(tile, (int(x), int(y)))
            x += tile.size[0] + gap
        y += max(t.size[1] for t in row) + gap
    if labels:
        x = (W - widths[0]) / 2.0
        for i, lab in enumerate(labels):
            dr.text((x, y + 6), lab, fill=(150, 154, 152, 255), font=font)
            x += rows[0][i].size[0] + gap
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    os.makedirs("../evo", exist_ok=True)

    ap, ab = seam_paths(); write_symbol(ap, ab, "../evo/mark-seam.svg")
    bp, bb = slip_paths(); write_symbol(bp, bb, "../evo/mark-slip.svg")
    write_wordmark("../evo/wordmark-thread.svg")

    A = lambda dr, px: draw_seam(dr, px, inner=62.0, slot=10.0, shear=34.0)
    B = lambda dr, px: draw_seam(dr, px, inner=None, slot=9.0, shear=40.0)
    C = lambda dr, px: draw_thread(dr, px)

    for nm, fn in (("A", A), ("B", B), ("C", C)):
        for s in (1024, 512, 256, 128, 64, 32, 16):
            img = render(fn, s)
            img.save(f"renders/FINAL-{nm}-{s}.png")
    print("rendered finals")

    def tile(px, fn, dark=True, size=None):
        size = size or px
        t = Image.new("RGBA", (px, px), (24, 27, 29, 255) if dark else (244, 244, 241, 255))
        im = render(fn, int(px * 0.82))
        t.alpha_composite(im, ((px - im.size[0]) // 2, (px - im.size[1]) // 2))
        return t

    big = [tile(300, A), tile(300, B), tile(300, C)]
    sn = [tile(110, A), tile(110, B), tile(110, C)]
    sheet([big, sn], "renders/SHEET-concepts.png",
          labels=["A — The Seam", "B — The Slip", "C — The Thread"])
