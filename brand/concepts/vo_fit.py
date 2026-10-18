#!/usr/bin/env python3
"""Fit the vo ligature's parameters to the reference PNG by mask IoU.

Measured from the reference (o centred at the origin, Rc = 100 after scaling):
    Rc (o centreline radius)  80.25 px  ->  scaled to 100
    w  (monoline stroke)      51.5 px   ->  64.2
    V  (the v's vertex)       (-159.5, 176.3)
    P  (arm's far end)        (-274.2, -136.4)
    tangency point T          on the Rc circle, so the arm blends with no junction
"""
import math, os
from PIL import Image, ImageDraw
import numpy as np

REF = ('/private/tmp/claude-501/-Users-bhavitsaini-Desktop-Evo-Evolution-nosync/'
       'fa293876-7241-47ba-a803-045c9bcb7b4f/images/1.png')
Rc = 100.0


# ---------------------------------------------------------------- model
def geom(vx, vy, py, lam, w, Rc=Rc):
    R_out, R_in = Rc + w / 2.0, Rc - w / 2.0
    V = (vx, vy)
    d = math.hypot(*V)
    delta = math.acos(min(1.0, Rc / d))
    alpha = math.atan2(V[1], V[0])
    cands = [((Rc * math.cos(alpha + s * delta), Rc * math.sin(alpha + s * delta)), 0)
             for s in (1, -1)]
    T = min(cands, key=lambda c: c[0][1])[0]
    ur = math.radians(lam)
    H = (py - vy) / -math.cos(ur)
    P = (V[0] - H * math.sin(ur), V[1] - H * math.cos(ur))
    return R_out, R_in, V, T, P


def raster(w_img, h_img, s, cx, cy, vx, vy, py, lam, w):
    """Draw at scale s with the o centre at (cx,cy). Returns a bool mask."""
    R_out, R_in, V, T, P = geom(vx, vy, py, lam, w)
    img = Image.new("L", (w_img, h_img), 0)
    dr = ImageDraw.Draw(img)

    def D(p, r):
        return [(cx + (p[0] + r * math.cos(2 * math.pi * i / 96)) * s,
                 cy + (p[1] + r * math.sin(2 * math.pi * i / 96)) * s)
                for i in range(96)]

    dr.polygon(D((0, 0), R_out), fill=255)
    dr.polygon(D((0, 0), R_in), fill=0)
    for a, b in ((P, V), (V, T)):
        dx, dy = b[0] - a[0], b[1] - a[1]
        L = math.hypot(dx, dy) or 1.0
        nx, ny = -dy / L * w / 2, dx / L * w / 2
        quad = [(a[0] + nx, a[1] + ny), (b[0] + nx, b[1] + ny),
                (b[0] - nx, b[1] - ny), (a[0] - nx, a[1] - ny)]
        dr.polygon([(cx + q[0] * s, cy + q[1] * s) for q in quad], fill=255)
    dr.polygon(D(P, w / 2), fill=255)
    dr.polygon(D(V, w / 2), fill=255)
    return np.array(img) > 127


def ref_mask():
    a = np.array(Image.open(REF).convert("RGBA"))[:, :, 3] > 128
    return a


def ref_o_centre():
    """o centre + radii measured from the reference."""
    a = ref_mask()
    cy = 231
    row = a[cy]
    runs = []; st = None
    for i, v in enumerate(row):
        if v and st is None: st = i
        if not v and st is not None: runs.append((st, i - 1)); st = None
    walls = [r for r in runs if r[0] > 200]
    l, r0 = walls[0][0], walls[-1][1]
    return (l + r0) / 2.0, cy, (r0 - l + 1) / 2.0


def iou(m1, m2):
    inter = np.logical_and(m1, m2).sum(); union = np.logical_or(m1, m2).sum()
    return inter / union if union else 0.0


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    os.makedirs("renders", exist_ok=True)
    ref = ref_mask()
    ox, oy, Ro = ref_o_centre()
    scale = Rc / (Ro - 0)                     # ref Ro maps onto our Rc
    print(f"reference: o centre ({ox:.1f},{oy:.1f})  Ro={Ro:.1f}  scale={scale:.4f}")

    best = None
    # seed from the measurement, then search a small neighbourhood
    seed = dict(vx=-159.5, vy=176.3, py=-136.4, lam=20.1, w=64.2)
    for dvx in (-8, -4, 0, 4, 8):
        for dvy in (-8, -4, 0, 4, 8):
            for dpy in (-10, -5, 0, 5, 10):
                for dlam in (-2, -1, 0, 1, 2):
                    for dw in (-4, -2, 0, 2, 4):
                        p = dict(seed)
                        p["vx"] += dvx; p["vy"] += dvy; p["py"] += dpy
                        p["lam"] += dlam; p["w"] += dw
                        m = raster(512, 512, scale, ox, oy, **p)
                        v = iou(m, ref)
                        if best is None or v > best[1]:
                            best = (dict(p), v)
    print("best IoU %.4f" % best[1])
    for k, v in best[0].items():
        print(f"   {k:>4}: {v:.1f}")
    np.save("renders/_best.npy", np.array([best[1]]))
    import json
    json.dump(dict(params=best[0], iou=best[1]), open("renders/vo-fit.json", "w"), indent=2)

    # overlay image: reference in grey, model in caramel, agreement in white
    m = raster(512, 512, scale, ox, oy, **best[0])
    vis = np.zeros((512, 512, 3), np.uint8)
    vis[ref] = [70, 74, 76]
    vis[m] = [200, 155, 107]
    vis[np.logical_and(ref, m)] = [240, 240, 236]
    Image.fromarray(vis).save("renders/vo-overlay.png")
    print("wrote renders/vo-overlay.png")
