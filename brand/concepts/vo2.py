#!/usr/bin/env python3
"""vo refinement — match the proportions of the reference, fix the vertex nub.

Measured off the reference image (512 px, normalised to Rc = 100):
    o ring thickness / centreline radius  w/Rc ~= 0.70
    |o centre -> vertex| / Rc             ~= 1.99
    left arm lean from vertical           ~= 26 deg
"""
import math, os
from PIL import Image, ImageDraw, ImageFont
import vo

K = 6
CLEAR = (0, 0, 0, 0)
BLACK = (0, 0, 0, 255)


def render2(size, bg=CLEAR, colour=BLACK, frac=0.86, **kw):
    """vo.render but with the vertex clearance handled properly."""
    return vo.render(size, bg=bg, colour=colour, frac=frac, **kw)


def sheet(items, cols, cell, out, labels=None, bg=(20, 23, 25, 255), gap=16):
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
        t = Image.new("RGBA", (cell, cell), (246, 246, 244, 255))
        t.alpha_composite(img.resize((cell, cell), Image.LANCZOS))
        sh.alpha_composite(t, (x, y))
        if labels: dr.text((x + 2, y + cell + 3), labels[i], fill=(206, 209, 205, 255), font=f)
    sh.convert("RGB").save(out); print("wrote", out)


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)

    items, labels = [], []
    # sweep the vertex distance: Rc=100, w=70 (w/Rc = 0.70, matching the reference)
    for vx in (-140.0, -160.0, -180.0, -200.0):
        items.append(vo.render_hi(300, vx=vx, vy=110.0, w=70.0))
        labels.append(f"vx {int(vx)}")
    sheet(items, 4, 300, "renders/vo-sweep-vx.png", labels=labels)

    items, labels = [], []
    for vy in (95.0, 110.0, 125.0):
        items.append(vo.render_hi(300, vx=-180.0, vy=vy, w=70.0))
        labels.append(f"vy {int(vy)}")
    for w in (62.0, 70.0, 78.0):
        items.append(vo.render_hi(300, vx=-180.0, vy=110.0, w=w))
        labels.append(f"w {int(w)}")
    for lam in (22.0, 26.0, 30.0):
        items.append(vo.render_hi(300, vx=-180.0, vy=110.0, w=70.0, lam=lam))
        labels.append(f"lean {int(lam)}")
    items.append(vo.render_hi(300, vx=-180.0, vy=110.0, w=70.0, mitre=True))
    labels.append("mitred")
    sheet(items, 4, 300, "renders/vo-sweep-2.png", labels=labels)

    tiny, tl = [], []
    for nm, kw in (("vx160", dict(vx=-160.0, vy=110.0, w=70.0)),
                   ("vx180", dict(vx=-180.0, vy=110.0, w=70.0)),
                   ("vx200", dict(vx=-200.0, vy=110.0, w=70.0)),
                   ("vx180w78", dict(vx=-180.0, vy=110.0, w=78.0)),
                   ("vx180mitre", dict(vx=-180.0, vy=110.0, w=70.0, mitre=True))):
        tiny.append(vo.render_hi(16, **kw)); tl.append(nm)
    sheet(tiny, 5, 110, "renders/vo-16b.png", labels=tl)
