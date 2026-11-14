"""Extra helpers for the planetstrike vehicle/prop/weapon build scripts (vehicles.py, props.py,
warpgate.py, weapons.py), on top of ltlib.py. All coordinates are GAME coordinates (Y up, front
-Z, metres), as in ltlib.

- `materials()` creates every material name these scripts use: `body`, `accent`, `glow`,
  `glow_white`, `halo_white`, `flow`, `tread` and the plain surfaces (ltlib.SURFACES).
- `plane(o, n)`: a ltlib.Frame on the plane through o with outward normal n (V = up projected
  on the plane, or -Z on horizontal planes; U = the viewer's right), plus `uv(frame, p3)` to
  author accents with 3D points that lie on the plane.
- `lathe`, `tube`, `circle`: n-gon solids of revolution about any axis (flats, not vertices,
  face the basis directions).
- `extrude`: a slab from a planar convex polygon pushed along a vector.
- `accent`, `white`, `flow`: strips (team neon with `glow`, white light with `halo_white`,
  flow lines without halo) along 3D polylines on a plane.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
from ltlib import Frame, X, Y, Z, NX, NY, NZ  # noqa: E402,F401

LIGHT, HALO, FLOW, TREAD = "glow_white", "halo_white", "flow", "tread"


def materials():
    L.standard_materials()
    L.material(LIGHT, base=(1, 1, 1), emit=(1, 1, 1), strength=4.0)
    L.material(HALO, base=(1, 1, 1), rough=1.0, emit=(1, 1, 1), strength=1.0, alpha=0.35)
    L.material(FLOW, base=(0.5, 0.5, 0.5), emit=(1, 1, 1), strength=2.0)
    # A track's running surface: lit like `flow`, at the vehicle's own speed.
    L.material(TREAD, base=(0.5, 0.5, 0.5), emit=(1, 1, 1), strength=2.0)


def drop_unused_materials():
    for m in list(L.bpy.data.materials):
        if m.users == 0:
            L.bpy.data.materials.remove(m)


# ---------------------------------------------------------------------------------------------
# Planes and frames.

def plane(o, n, up=Y):
    """Frame on the plane through o with outward normal n: V = `up` projected onto the plane
    (-Z if the plane is horizontal), U = V x N (the viewer's right), so U x V = N."""
    n = L.norm(n)
    v = L.sub(up, L.mul(n, L.dot(up, n)))
    if L.length(v) < 1e-6:
        v = L.sub(NZ, L.mul(n, L.dot(NZ, n)))
    v = L.norm(v)
    return Frame(o, L.cross(v, n), v)


def uv(f, p):
    d = L.sub(p, f.o)
    return (L.dot(d, f.U), L.dot(d, f.V))


def uvs(f, pts):
    return [uv(f, p) for p in pts]


def lerp(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


# ---------------------------------------------------------------------------------------------
# Solids.

def basis(axis):
    a = L.norm(axis)
    ref = Y if abs(a[1]) < 0.9 else NZ
    u = L.norm(L.cross(ref, a))
    v = L.cross(a, u)
    return u, v


def circle(c, axis, r, n, phase=None, rv=None):
    """n-gon about `axis` through c (vertex radius r, or an ellipse r x rv). With the default
    phase the flats face the basis directions (for axis -Z: -X/+Y, i.e. a flat on top)."""
    u, v = basis(axis)
    phase = math.pi / n if phase is None else phase
    rv = r if rv is None else rv
    return [L.add(c, L.add(L.mul(u, r * math.cos(phase + 2 * math.pi * i / n)),
                           L.mul(v, rv * math.sin(phase + 2 * math.pi * i / n))))
            for i in range(n)]


def lathe(g, o, axis, prof, n=8, mats="body", cap0="body", cap1="body", phase=None, rv=None):
    """Solid of revolution: prof is [(t, r)] with t along `axis` from o (r > 0)."""
    a = L.norm(axis)
    rings = [circle(L.add(o, L.mul(a, t)), a, r, n, phase,
                    None if rv is None else r * rv) for t, r in prof]
    L.loft(g, rings, a, mats=mats, cap0=cap0, cap1=cap1)


def tube(g, a, b, r, n=8, mat="body", caps=True, phase=None):
    d = L.sub(b, a)
    lathe(g, a, d, [(0, r), (L.length(d), r)], n, mats=mat,
          cap0=mat if caps else None, cap1=mat if caps else None, phase=phase)


def extrude(g, poly, off, mat="body", side_mat=None, caps=(True, True)):
    """Prism from a planar convex 3D polygon and the same polygon moved by `off`."""
    top = [L.add(p, off) for p in poly]
    c = L.mul(tuple(map(sum, zip(*poly))), 1.0 / len(poly))
    n = len(poly)
    for i in range(n):
        j = (i + 1) % n
        q = [poly[i], poly[j], top[j], top[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        out = L.sub(mid, L.add(c, L.mul(off, 0.5)))
        out = L.sub(out, L.mul(L.norm(off), L.dot(out, L.norm(off))))
        g.face(q, side_mat or mat, want=out)
    if caps[0]:
        g.face(poly, mat, want=L.mul(off, -1))
    if caps[1]:
        g.face(top, mat, want=off)


def section_loft(g, template, stations, axis, mats="body", cap0="body", cap1="body"):
    """Loft along `axis` through stations: each station is (t, params) and template(t, params)
    returns the ring's 3D points."""
    L.loft(g, [template(t, p) for t, p in stations], axis, mats=mats, cap0=cap0, cap1=cap1)


# ---------------------------------------------------------------------------------------------
# Accents on planes, authored as 3D points lying on the plane with normal n.

def _strip3(g, pts, n, w, closed, glow, mat, glow_mat, h, o=None):
    f = plane(o or pts[0], n)
    p2 = uvs(f, pts)
    if closed:
        p2 = L.ccw(p2)
    g_ = glow if isinstance(glow, tuple) else (glow, glow)
    L.strip(g, f, p2, w, closed=closed, glow=g_, mat=mat, glow_mat=glow_mat, h=h, sides=False)


def accent(g, pts, n, w, closed=False, glow=0.06, h=L.ACCENT_H):
    _strip3(g, pts, n, w, closed, glow, "accent", "glow", h)


def white(g, pts, n, w, closed=False, glow=0.05, h=L.ACCENT_H):
    _strip3(g, pts, n, w, closed, glow, LIGHT, HALO, h)


def flow(g, pts, n, w, closed=False, h=0.004):
    f = plane(pts[0], n)
    L.strip(g, f, uvs(f, pts), w, closed=closed, glow=(0, 0), mat=FLOW, h=h, sides=False,
            caps=False)


def disc(g, pts, n, mat="accent", glow=0.05, h=L.ACCENT_H):
    """Filled convex polygon (3D points on the plane) with a halo round it."""
    f = plane(pts[0], n)
    gm = HALO if mat == LIGHT else "glow"
    L.filled(g, f, uvs(f, pts), glow=glow, mat=mat, h=h, glow_mat=gm)


def on_quad(A, B, C, D, s, t):
    """Bilinear point on quad ABCD (s along AB, t along AD)."""
    return lerp(lerp(A, B, s), lerp(D, C, s), t)


def quad_normal(q, want):
    n = L.norm(L.newell(q))
    return n if L.dot(n, want) >= 0 else L.mul(n, -1)


# ---------------------------------------------------------------------------------------------
# Detail pieces (hard-surface dressing shared by the vehicles, weapons and suits). Each adds
# closed little solids standing on an existing face: nothing is cut out of the surface below.

def bevel_box(g, mn, mx, c, mat="body", mats=None):
    """Axis-aligned box with every edge chamfered by c. `mats` may give a material per side
    ("+x" .. "-z"); the chamfers take `mat`."""
    mats = mats or {}
    lo, hi = (mn, mx)

    def v(corner, ax):
        # The corner's point on the face across axis `ax`: inset by c on the other two.
        return tuple((hi[i] if corner[i] else lo[i]) if i == ax
                     else (hi[i] - c if corner[i] else lo[i] + c) for i in range(3))

    def unit(ax, s):
        return tuple((1.0 if s else -1.0) if i == ax else 0.0 for i in range(3))

    for ax in range(3):
        o1, o2 = [i for i in range(3) if i != ax]
        for s in (0, 1):
            cs = []
            for a, b in ((0, 0), (1, 0), (1, 1), (0, 1)):
                k = [0, 0, 0]
                k[ax], k[o1], k[o2] = s, a, b
                cs.append(v(k, ax))
            g.face(cs, mats.get("-+"[s] + "xyz"[ax], mat), want=unit(ax, s))
    for e in range(3):  # the edges along axis e
        a, b = [i for i in range(3) if i != e]
        for sa in (0, 1):
            for sb in (0, 1):
                k0, k1 = [0, 0, 0], [0, 0, 0]
                k0[a] = k1[a] = sa
                k0[b] = k1[b] = sb
                k1[e] = 1
                g.face([v(k0, a), v(k1, a), v(k1, b), v(k0, b)], mat,
                       want=L.add(unit(a, sa), unit(b, sb)))
    for sx in (0, 1):
        for sy in (0, 1):
            for sz in (0, 1):
                k = (sx, sy, sz)
                g.face([v(k, 0), v(k, 1), v(k, 2)], mat,
                       want=tuple(1.0 if s else -1.0 for s in k))


def _inset2(p2, d):
    """A convex CCW 2D polygon with every edge moved inward by d."""
    n = len(p2)
    lines = []
    for i in range(n):
        a, b = p2[i], p2[(i + 1) % n]
        e = (b[0] - a[0], b[1] - a[1])
        l = math.hypot(*e) or 1.0
        nx, ny = -e[1] / l, e[0] / l  # inward for CCW
        lines.append(((a[0] + nx * d, a[1] + ny * d), e))
    out = []
    for i in range(n):
        (p, r), (q, s) = lines[i - 1], lines[i]
        den = r[0] * s[1] - r[1] * s[0]
        if abs(den) < 1e-12:
            out.append(q)
            continue
        t = ((q[0] - p[0]) * s[1] - (q[1] - p[1]) * s[0]) / den
        out.append((p[0] + r[0] * t, p[1] + r[1] * t))
    return out


def _on_plane(poly, n):
    f = plane(poly[0], n)
    return f, L.ccw(uvs(f, poly))


def plate(g, poly, n, h, mat="armor", bevel=0.0, side_mat=None, h0=0.0):
    """A raised plate over a planar convex polygon (3D points, outward normal n), h proud of
    it, its top edge chamfered by `bevel`."""
    f, p2 = _on_plane(poly, n)
    top2 = _inset2(p2, bevel) if bevel > 0 else p2
    base = [f.p(u, v_, h0) for u, v_ in p2]
    top = [f.p(u, v_, h0 + h) for u, v_ in top2]
    k = len(base)
    c = f.p(sum(u for u, _ in p2) / k, sum(v_ for _, v_ in p2) / k, h0)
    for i in range(k):
        j = (i + 1) % k
        q = [base[i], base[j], top[j], top[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, side_mat or mat, want=L.add(L.sub(mid, c), L.mul(f.N, 1e-3)))
    g.face(top, mat, want=f.N)
    return f


def panel_inset(g, poly, n, h=0.012, rim=0.03, mat="armor", mat_in="body", floor=0.3):
    """A framed panel: a rim `rim` wide standing h proud of the face along the polygon's edge,
    with a sunken field of `mat_in` inside it (at h * floor), so it reads as a recessed panel
    with a seam round it."""
    f, p2 = _on_plane(poly, n)
    in2 = _inset2(p2, rim)
    k = len(p2)
    o0 = [f.p(u, v_, 0.0) for u, v_ in p2]
    o1 = [f.p(u, v_, h) for u, v_ in p2]
    i1 = [f.p(u, v_, h) for u, v_ in in2]
    i0 = [f.p(u, v_, h * floor) for u, v_ in in2]
    c = f.p(sum(u for u, _ in p2) / k, sum(v_ for _, v_ in p2) / k, 0.0)
    for i in range(k):
        j = (i + 1) % k
        q = [o0[i], o0[j], o1[j], o1[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, mat, want=L.sub(mid, c))
        g.face([o1[i], o1[j], i1[j], i1[i]], mat, want=f.N)
        q = [i1[i], i1[j], i0[j], i0[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, mat, want=L.sub(c, mid))
    g.face(i0, mat_in, want=f.N)
    return f


def rect_on(o, n, w, h, up=Y):
    """The corners of a w x h rectangle centred on o in the plane with normal n."""
    f = plane(o, n, up)
    return [f.p(-w / 2, -h / 2), f.p(w / 2, -h / 2), f.p(w / 2, h / 2), f.p(-w / 2, h / 2)]


def bolt_row(g, a, b, n, count, r=0.012, h=0.006, mat="metal", ends=False):
    """`count` hex bolt heads standing on the face with normal n, spread from a to b (at the
    very ends too with `ends`)."""
    for i in range(count):
        t = (i / (count - 1) if count > 1 else 0.5) if ends else (i + 0.5) / count
        lathe(g, lerp(a, b, t), n, [(0, r), (h, r * 0.9)], n=6, mats=mat, cap0=None, cap1=mat)


def vent(g, o, n, w, h, slats=4, depth=0.02, mat="body", mat_in="rubber", up=Y, frame=0.012):
    """A louvred vent w x h centred on o on the face with normal n: a frame, a dark back and
    slanted slats."""
    f = plane(o, n, up)
    hw, hh = w / 2, h / 2
    for u0, v0, u1, v1 in ((-hw, -hh, hw, -hh + frame), (-hw, hh - frame, hw, hh),
                           (-hw, -hh + frame, -hw + frame, hh - frame),
                           (hw - frame, -hh + frame, hw, hh - frame)):
        extrude(g, [f.p(u0, v0), f.p(u1, v0), f.p(u1, v1), f.p(u0, v1)], L.mul(f.N, depth), mat)
    g.face([f.p(-hw + frame, -hh + frame, 0.002), f.p(hw - frame, -hh + frame, 0.002),
            f.p(hw - frame, hh - frame, 0.002), f.p(-hw + frame, hh - frame, 0.002)],
           mat_in, want=f.N)
    pitch = (h - 2 * frame) / slats
    for i in range(slats):
        v0 = -hh + frame + pitch * i
        q = [f.p(-hw + frame, v0 + pitch * 0.15, 0.004), f.p(hw - frame, v0 + pitch * 0.15, 0.004),
             f.p(hw - frame, v0 + pitch * 0.85, depth), f.p(-hw + frame, v0 + pitch * 0.85, depth)]
        thick = L.mul(L.norm(L.newell(q)), 0.005)
        if L.dot(thick, f.N) < 0:
            thick = L.mul(thick, -1)
        extrude(g, [L.sub(p, thick) for p in q], thick, mat)
    return f


def hose(g, path, r, n=6, mat="rubber", caps=True, collars=None, collar_mat="metal"):
    """A tube swept through the 3D points of `path` (mitred at the bends). `collars` puts a
    short wider ring of `collar_mat` at those path indices (hose clamps, pipe unions)."""
    k = len(path)
    tans = []
    for i in range(k):
        a = path[max(i - 1, 0)]
        b = path[min(i + 1, k - 1)]
        tans.append(L.norm(L.sub(b, a)))
    u, _ = basis(tans[0])
    rings = []
    for p, t in zip(path, tans):
        u = L.sub(u, L.mul(t, L.dot(u, t)))  # carried along, so the tube does not twist
        u = L.norm(u)
        w = L.cross(t, u)
        rings.append([L.add(p, L.add(L.mul(u, r * math.cos(2 * math.pi * j / n)),
                                     L.mul(w, r * math.sin(2 * math.pi * j / n))))
                      for j in range(n)])
    for i in range(k - 1):
        cm = L.mul(L.add(path[i], path[i + 1]), 0.5)
        for j in range(n):
            j1 = (j + 1) % n
            q = [rings[i][j], rings[i][j1], rings[i + 1][j1], rings[i + 1][j]]
            mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
            g.face(q, mat, want=L.sub(mid, cm))
    if caps:
        g.face(rings[0], mat, want=L.mul(tans[0], -1))
        g.face(rings[-1], mat, want=tans[-1])
    for i in collars or ():
        t = tans[i]
        lathe(g, L.sub(path[i], L.mul(t, r * 0.8)), t, [(0, r * 1.35), (r * 1.6, r * 1.35)],
              n=n, mats=collar_mat, cap0=collar_mat, cap1=collar_mat)


def arc(c, axis, r, a0, a1, steps, start=None):
    """Points on a circular arc about `axis` through c from angle a0 to a1 (radians), measured
    from `start` (a direction across the axis; the basis' first by default): for hose paths."""
    a = L.norm(axis)
    if start is None:
        u, w = basis(a)
    else:
        u = L.norm(L.sub(start, L.mul(a, L.dot(start, a))))
        w = L.cross(a, u)
    return [L.add(c, L.add(L.mul(u, r * math.cos(a0 + (a1 - a0) * i / steps)),
                           L.mul(w, r * math.sin(a0 + (a1 - a0) * i / steps))))
            for i in range(steps + 1)]


def greeble(g, o, n, w, h, seed, count=5, mats=("body", "metal", "armor"), up=Y, tall=0.04):
    """Seeded small fittings scattered over a w x h rectangle centred on o on the face with
    normal n: boxes, round caps and grab handles. The same seed gives the same pieces."""
    import random
    rng = random.Random(seed)
    f = plane(o, n, up)
    for _ in range(count):
        kind = rng.choice(("box", "box", "cap", "handle"))
        sw, sh = rng.uniform(0.12, 0.3) * w, rng.uniform(0.12, 0.3) * h
        u = rng.uniform(-w / 2 + sw / 2, w / 2 - sw / 2)
        v_ = rng.uniform(-h / 2 + sh / 2, h / 2 - sh / 2)
        t = rng.uniform(0.3, 1.0) * tall
        mat = rng.choice(mats)
        if kind == "box":
            plate(g, [f.p(u - sw / 2, v_ - sh / 2), f.p(u + sw / 2, v_ - sh / 2),
                      f.p(u + sw / 2, v_ + sh / 2), f.p(u - sw / 2, v_ + sh / 2)], f.N, t, mat,
                  bevel=min(sw, sh) * 0.15)
        elif kind == "cap":
            rr = min(sw, sh) / 2
            lathe(g, f.p(u, v_), f.N, [(0, rr), (t * 0.7, rr), (t, rr * 0.7)], n=8, mats=mat,
                  cap0=None, cap1=mat)
        else:
            rr = max(min(sw, sh) * 0.08, 0.004)
            a, b = f.p(u - sw / 2, v_), f.p(u + sw / 2, v_)
            up_ = L.mul(f.N, t)
            hose(g, [a, L.add(a, up_), L.add(b, up_), b], rr, n=4, mat="metal", caps=False)


def tyre(g, c, axis, r, w, n=16, lugs=0, hub=0.58, mat="rubber", hub_mat="metal",
         cap_mat="body", lug_h=None, nuts=5):
    """A wheel about `axis` through its centre c: a tyre of radius r and width w with shoulders,
    `lugs` tread blocks round it, a dished hub with wheel nuts and a centre cap on both sides
    (the same mesh serves either side of a vehicle)."""
    a = L.norm(axis)
    hw = w / 2
    sh = min(w * 0.18, r * 0.12)
    o = L.sub(c, L.mul(a, hw))
    lathe(g, o, a, [(0, r * 0.8), (sh, r), (w - sh, r), (w, r * 0.8)], n=n, mats=mat,
          cap0=mat, cap1=mat)
    hr = r * hub
    lathe(g, L.sub(c, L.mul(a, hw + 0.012)), a,
          [(0, hr * 0.92), (0.012, hr), (w + 0.012, hr), (w + 0.024, hr * 0.92)], n=n,
          mats=hub_mat, cap0=hub_mat, cap1=hub_mat)
    for s in (-1, 1):
        d = L.mul(a, s)
        face = L.add(c, L.mul(d, hw + 0.012))
        lathe(g, face, d, [(0, hr * 0.34), (0.03, hr * 0.3), (0.045, hr * 0.16)], n=8,
              mats=cap_mat, cap0=None, cap1=cap_mat)
        for p in circle(face, d, hr * 0.62, nuts, phase=0.0):
            lathe(g, p, d, [(0, hr * 0.08), (0.014, hr * 0.07)], n=6, mats=cap_mat, cap0=None,
                  cap1=cap_mat)
    if lugs:
        lh = lug_h if lug_h is not None else r * 0.06
        u, v_ = basis(a)
        for i in range(lugs):
            ang = 2 * math.pi * i / lugs
            half = math.pi / lugs * 0.55
            side = -1 if i % 2 else 1  # staggered left and right, as a tractor's
            z0, z1 = (-hw + sh * 0.5, hw * 0.15) if side < 0 else (-hw * 0.15, hw - sh * 0.5)

            def at(th, z, rad):
                return L.add(c, L.add(L.mul(a, z), L.add(L.mul(u, rad * math.cos(th)),
                                                         L.mul(v_, rad * math.sin(th)))))
            rr = r * math.cos(math.pi / n) - 0.002
            base = [at(ang - half, z0, rr), at(ang + half, z0, rr), at(ang + half, z1, rr),
                    at(ang - half, z1, rr)]
            out = L.norm(L.add(L.mul(u, math.cos(ang)), L.mul(v_, math.sin(ang))))
            extrude(g, base, L.mul(out, r - rr + lh), mat)


def road_wheel(g, c, axis, r, w, n=12, mat="rubber", hub_mat="metal", cap_mat="body"):
    """A tank's road wheel about `axis`: a metal dish with a rubber rim and a hub cap, both
    sides alike."""
    a = L.norm(axis)
    hw = w / 2
    lathe(g, L.sub(c, L.mul(a, hw)), a, [(0, r), (w, r)], n=n, mats=mat, cap0=mat, cap1=mat)
    lathe(g, L.sub(c, L.mul(a, hw + 0.008)), a,
          [(0, r * 0.8), (w + 0.016, r * 0.8)], n=n, mats=hub_mat, cap0=hub_mat, cap1=hub_mat)
    for s in (-1, 1):
        d = L.mul(a, s)
        face = L.add(c, L.mul(d, hw + 0.008))
        lathe(g, face, d, [(0, r * 0.3), (0.025, r * 0.26), (0.035, r * 0.14)], n=8,
              mats=cap_mat, cap0=None, cap1=cap_mat)
        for p in circle(face, d, r * 0.55, 6, phase=0.0):
            lathe(g, p, d, [(0, r * 0.06), (0.01, r * 0.05)], n=6, mats=cap_mat, cap0=None,
                  cap1=cap_mat)
