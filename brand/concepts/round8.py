#!/usr/bin/env python3
"""Final concept sheets — reuses the round-6 raster (which rendered correctly)."""
import math, os
from PIL import Image, ImageDraw

K = 6


def draw_seam(dr, px, R=100.0, inner=62.0, slot=10.0, shear=34.0, fill=(0, 0, 0, 255)):
    """Ring OR solid split on a horizontal seam; upper half displaced +x."""
    x0, x1 = -R, shear + R
    y0, y1 = -R - slot / 2.0, R + slot / 2.0
    w, h = x1 - x0, y1 - y0
    sc = (px * 0.90) / max(w, h)
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc
    s2 = slot / 2.0

    def poly(pts):
        dr.polygon([(ox + p[0] * sc, oy + p[1] * sc) for p in pts], fill=fill)

    N = 260
    if inner is None:
        up = [(x, -math.sqrt(max(0.0, R * R - x * x)) - s2)
              for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
        lo = [(shear + x, math.sqrt(max(0.0, R * R - x * x)) + s2)
              for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
        poly(up); poly(lo)
    else:
        r2 = inner * inner
        upo = [(x, -math.sqrt(max(0.0, R * R - x * x)) - s2)
               for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
        upi = [(x, -math.sqrt(max(0.0, r2 - x * x)) - s2)
               for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]
        loo = [(shear + x, math.sqrt(max(0.0, R * R - x * x)) + s2)
               for x in [(-R + 2 * R * i / N) for i in range(N + 1)]]
        loi = [(shear + x, math.sqrt(max(0.0, r2 - x * x)) + s2)
               for x in [(-inner + 2 * inner * i / 160) for i in range(161)]]
        poly(upo + list(reversed(upi)))
        poly(loo + list(reversed(loi)))


def draw_wordmark(dr, px, sw=26.0, xh=104.0, gap=25.0, fill=(0, 0, 0, 255), scale_hint=None):
    """Monoline constructed 'evo'. Drawn at built coords, then fitted."""
    ro = (xh - sw) / 2.0
    x0, y0, x1, y1 = 0.0, 0.0, 0.0, 0.0

    def seg(p, q, w):
        vx, vy = q[0] - p[0], q[1] - p[1]
        L = math.hypot(vx, vy) or 1.0
        nx, ny = -vy / L * w / 2, vx / L * w / 2
        return [(p[0] + nx, p[1] + ny), (q[0] + nx, q[1] + ny),
                (q[0] - nx, q[1] - ny), (p[0] - nx, p[1] - ny)]

    def arc(c, rr, a0, a1, w, n=120):
        out_o, out_i = [], []
        for i in range(n + 1):
            a = math.radians(a0 + (a1 - a0) * i / n)
            for lst, off in ((out_o, w / 2), (out_i, -w / 2)):
                lst.append((c[0] + (rr + off) * math.cos(a), c[1] + (rr + off) * math.sin(a)))
        return out_o + list(reversed(out_i))

    polys = []
    ecx, ecy = ro + sw / 2, xh / 2
    polys.append(arc((ecx, ecy), ro, 0, 360 - 52, sw))
    e_right = ecx + ro + sw / 2
    # crossbar of the e : short bar into the bowl
    polys.append(seg((ecx - ro, ecy), (ecx + ro * 1.0, ecy), sw))
    half = xh * math.tan(math.radians(21.0))
    vcx = e_right + gap + half
    polys.append(seg((vcx - half, 0.0), (vcx, xh), sw))
    polys.append(seg((vcx, xh), (vcx + half, 0.0), sw))
    v_right = vcx + half
    ocx = v_right + gap + ro + sw / 2
    polys.append(arc((ocx, xh / 2), ro, 0, 360, sw))
    xs = [p[0] for poly in polys for p in poly]; ys = [p[1] for poly in polys for p in poly]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    w, h = x1 - x0, y1 - y0
    sc = (px * 0.94) / w
    ox = (px - w * sc) / 2.0 - x0 * sc
    oy = (px - h * sc) / 2.0 - y0 * sc
    for poly in polys:
        dr.polygon([(ox + p[0] * sc, oy + p[1] * sc) for p in poly], fill=fill)


def render(fn, size, bg=(0, 0, 0, 0)):
    hi = Image.new("RGBA", (size * K, size * K), bg)
    fn(ImageDraw.Draw(hi), size * K)
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
        tile = Image.new("RGBA", (cell, cell), (246, 246, 244, 255))
        tile.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sh.alpha_composite(tile, (x, y))
        if label: dr.text((x + 2, y + cell + 3), name, fill=(205, 208, 204, 255))
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    import sys
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    A = lambda dr, px: draw_seam(dr, px, R=100.0, inner=62.0, slot=10.0, shear=34.0)
    B = lambda dr, px: draw_seam(dr, px, R=100.0, inner=None, slot=9.0, shear=40.0)
    C = lambda dr, px: draw_wordmark(dr, px)
    for nm, fn in (("A", A), ("B", B), ("C", C)):
        for s in (1024, 512, 256, 128, 64, 32, 16):
            img = render(fn, s)
            bg = (15, 18, 20, 255) if s in (512, 128, 16) else None
            if bg:
                f = Image.new("RGBA", img.size, bg); f.alpha_composite(img); img = f
            img.save(f"renders/FINAL-{nm}-{s}.png")
        print("rendered", nm)
    sheet([(n, render({ "A": A, "B": B, "C": C}[n], 288)) for n in ("A", "B", "C")],
          3, 288, "renders/SHEET-finals-288.png")
    sheet([(n, render({ "A": A, "B": B, "C": C}[n], 16)) for n in ("A", "B", "C")],
          3, 112, "renders/SHEET-finals-16.png")
