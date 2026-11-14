"""Extra helpers for the planetstrike vehicle/prop/weapon build scripts (vehicles.py, props.py,
warpgate.py, weapons.py), on top of ltlib.py. All coordinates are GAME coordinates (Y up, front
-Z, metres), as in ltlib.

- `materials()` creates every material name these scripts use: `body`, `accent`, `glow`,
  `glow_white`, `halo_white`, `flow`.
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

LIGHT, HALO, FLOW = "glow_white", "halo_white", "flow"


def materials():
    L.standard_materials()
    L.material(LIGHT, base=(1, 1, 1), emit=(1, 1, 1), strength=4.0)
    L.material(HALO, base=(1, 1, 1), rough=1.0, emit=(1, 1, 1), strength=1.0, alpha=0.35)
    L.material(FLOW, base=(0.5, 0.5, 0.5), emit=(1, 1, 1), strength=2.0)


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
