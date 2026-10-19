#!/usr/bin/env python3
"""Evo logo concept generator — geometry only, no effects."""
import math, os

PI = math.pi
def pt(x, y): return (x, y)

# ---------------------------------------------------------------- helpers
def catmull_path(pts, closed=False):
    """Catmull-Rom through pts -> cubic bezier path string (smooth)."""
    if len(pts) < 2: return ""
    p = list(pts)
    if closed:
        p = [p[-1]] + p + [p[0], p[1]]
    else:
        p = [p[0]] + p + [p[-1]]
    d = f"M {p[1][0]:.3f} {p[1][1]:.3f}"
    for i in range(1, len(p) - 2):
        p0, p1, p2, p3 = p[i-1], p[i], p[i+1], p[i+2]
        c1 = (p1[0] + (p2[0]-p0[0])/6.0, p1[1] + (p2[1]-p0[1])/6.0)
        c2 = (p2[0] - (p3[0]-p1[0])/6.0, p2[1] - (p3[1]-p1[1])/6.0)
        d += f" C {c1[0]:.3f} {c1[1]:.3f} {c2[0]:.3f} {c2[1]:.3f} {p2[0]:.3f} {p2[1]:.3f}"
    if closed: d += " Z"
    return d

def bbox_of(pts):
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)

def shift_pts(pts, dx, dy): return [(x+dx, y+dy) for x, y in pts]

def svg(body, extra=""):
    return ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" '
            'width="256" height="256">\n' + body + '\n</svg>\n')

def write(name, body, extra=""):
    with open(name, "w") as f: f.write(svg(body, extra))
    print("wrote", name)

# ================================================================ CONCEPT A
# THE FOLD — one unbroken stroke: a ring whose radius is folded at a single
# vertex. beta = interior angle of the fold.
def concept_a(beta_deg, R, sw, fname):
    phi = (180.0 - beta_deg) / 2.0            # angle of tangent pt from vertical
    D   = R / math.cos(math.radians(phi))     # vertex distance from ring centre
    ph  = math.radians(phi)
    # tangent points, y-down: offset phi from the downward direction
    tr = ( R*math.sin(ph),  R*math.cos(ph))
    tl = (-R*math.sin(ph),  R*math.cos(ph))
    v  = (0.0, D)
    # centre the whole mark (incl. stroke) in the 256 box
    top = -R - sw/2; bot = D + sw/2; lef = -R - sw/2; rig = R + sw/2
    dx = 128 - (lef + rig)/2.0
    dy = 128 - (top + bot)/2.0
    (tr, tl, v) = [shift_pts([p], dx, dy)[0] for p in (tr, tl, v)]
    arc = 360.0 - 2*phi                       # degrees of ring that survives
    saf = 0 if arc > 180 else 1
    d = (f"M {tr[0]:.3f} {tr[1]:.3f} "
         f"A {R:.3f} {R:.3f} 0 1 {saf} {tl[0]:.3f} {tl[1]:.3f} "
         f"L {v[0]:.3f} {v[1]:.3f} Z")
    body = (f'  <path d="{d}" fill="none" stroke="#000000" '
            f'stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>')
    write(fname, body)

# ================================================================ CONCEPT B
# THE THREAD — a single continuous monotone stroke, tapered, still growing.
def concept_b(fname, turns=1.18, r0=108.0, growth=0.58, w0=30.0, w1=15.0, n=140):
    k = growth
    th0, th1 = math.radians(10.0), math.radians(10.0 + 360.0*turns)
    up, lo = [], []
    for i in range(n+1):
        t  = i/n
        th = th0 + (th1-th0)*t
        r  = r0 * k**(th/(2*PI))
        P  = (r*math.cos(th), r*math.sin(th))
        dr = r*math.log(k)/(2*PI)
        T  = (dr*math.cos(th) - r*math.sin(th), dr*math.sin(th) + r*math.cos(th))
        L  = math.hypot(*T); Tn = (T[0]/L, T[1]/L); Nn = (-Tn[1], Tn[0])
        w  = w0 + (w1-w0)*t
        up.append((P[0]+Nn[0]*w/2, P[1]+Nn[1]*w/2))
        lo.append((P[0]-Nn[0]*w/2, P[1]-Nn[1]*w/2))
    # round caps
    def cap(Pc, Nn, wd, forward=True):
        a0 = math.atan2(Nn[1], Nn[0]); out = []
        steps = 12
        for j in range(steps+1):
            a = a0 - (PI*j/steps if forward else -PI*j/steps)
            out.append((Pc[0] + math.cos(a)*wd/2, Pc[1] + math.sin(a)*wd/2))
        return out
    # rebuild normal at the two ends
    def nrm(i):
        t  = i/n
        th = th0 + (th1-th0)*t
        r  = r0 * k**(th/(2*PI))
        dr = r*math.log(k)/(2*PI)
        T  = (dr*math.cos(th) - r*math.sin(th), dr*math.sin(th) + r*math.cos(th))
        L  = math.hypot(*T); Tn = (T[0]/L, T[1]/L)
        return (T[0]/L, T[1]/L), (-Tn[1], Tn[0]), (r*math.cos(th), r*math.sin(th))
    _, n_end, p_end = nrm(n)
    _, n_beg, p_beg = nrm(0)
    outline = up + cap(p_end, n_end, w1, True)[1:] + list(reversed(lo)) \
                  + cap(p_beg, n_beg, w0, False)[1:]
    bx0, by0, bx1, by1 = bbox_of(outline)
    outline = shift_pts(outline, 128-(bx0+bx1)/2, 128-(by0+by1)/2)
    write(fname, '  <path d="%s" fill="#000000"/>' % catmull_path(outline, closed=True))

# ================================================================ CONCEPT C
# THE KEYSTONE — a ring assembled from witnessed spans; one span is absent.
def concept_c(fname, R=100.0, sw=26.0, slots=4, span_deg=66.0):
    gap = 360.0/slots - span_deg
    parts = []
    for i in range(1, slots):                    # span 0 absent
        a0 = i*(360.0/slots) + gap/2
        a1 = a0 + span_deg
        p0 = (R*math.cos(math.radians(a0)), R*math.sin(math.radians(a0)))
        p1 = (R*math.cos(math.radians(a1)), R*math.sin(math.radians(a1)))
        parts.append(f"M {p0[0]:.3f} {p0[1]:.3f} "
                     f"A {R:.3f} {R:.3f} 0 0 1 {p1[0]:.3f} {p1[1]:.3f}")
    body = ('  <g transform="translate(128 128)">\n'
            '  <path d="' + " ".join(parts) + f'" fill="none" stroke="#000000" '
            f'stroke-width="{sw}" stroke-linecap="butt"/>\n  </g>')
    write(fname, body)

# ================================================================ WORDMARK
def build_evo(x0=0.0, s=26.0):
    """Constructed geometric lowercase 'evo'. Returns (path_d, width, xheight)."""
    XH = 104.0
    r_o = (XH - s)/2.0            # centreline radius of o / e bowl
    d = []
    # ---- e : bowl open at lower right + crossbar
    ecx, ecy = x0 + r_o + s/2, -XH/2
    a_end = math.radians(58.0)
    ex_e, ey_e = ecx + r_o*math.cos(a_end), ecy + r_o*math.sin(a_end)
    d.append(f"M {ecx + r_o:.3f} {ecy:.3f} "
             f"A {r_o:.3f} {r_o:.3f} 0 1 0 {ex_e:.3f} {ey_e:.3f}")
    d.append(f"M {ecx - r_o:.3f} {ecy:.3f} L {ecx + r_o:.3f} {ecy:.3f}")
    e_right = ecx + r_o + s/2
    # ---- v
    aw = 42.0                                   # half width of v
    vcx = e_right + 24.0 + aw
    d.append(f"M {vcx - aw:.3f} 0 L {vcx:.3f} {-XH:.3f} L {vcx + aw:.3f} 0")
    v_right = vcx + aw
    # ---- o
    ocx = v_right + 24.0 + r_o + s/2 - 2.0
    ocy = -XH/2
    d.append(f"M {ocx - r_o:.3f} {ocy:.3f} A {r_o:.3f} {r_o:.3f} 0 1 1 "
             f"{ocx + r_o:.3f} {ocy:.3f} A {r_o:.3f} {r_o:.3f} 0 1 1 {ocx - r_o:.3f} {ocy:.3f} Z")
    return " ".join(d), ocx + r_o + s/2 - x0, XH

def concept_wordmark(fname):
    d, w, xh = build_evo(0.0, 26.0)
    body = f'  <path d="{d}" fill="none" stroke="#000000" stroke-width="26" stroke-linecap="round"/>'
    write(fname, body)

# ================================================================ LOCKUPS
def lockup(symbol_svg_path_snippet, fname, sw=22.0):
    """symbol drawn at 132 tall on the baseline-left, wordmark to its right."""
    pass

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    for beta in (84.0, 96.0, 108.0):
        phi = (180.0 - beta)/2.0
        f = 1.0/math.cos(math.radians(phi))
        R = (256.0 - 26.0)/(1.0 + f) - 6.0
        concept_a(beta, R, 26.0, f"a-fold-b{int(beta)}.svg")
    concept_a(96.0, (256.0-26.0)/1.346 - 6.0, 26.0, "a-fold-v1.svg")
    concept_b("b-thread-v1.svg")
    concept_c("c-keystone-v1.svg")
    concept_wordmark("evo-wordmark.svg")
    d, w, xh = build_evo()
    print(f"wordmark width={w:.1f} xheight={xh}")
