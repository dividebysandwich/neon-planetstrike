"""Builds the planetstrike vehicles: assets-src/blend/vehicles.blend and assets/models/vehicles.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/vehicles.py

Hull frame (every `*_hull`, `spawn_bus_hull`, `vtol_body`): origin at the bottom middle (ground
contact), front -Z, up +Y; each stays inside its hit box x in [-hx, hx], y in [0, 2 hy],
z in [-hz, hz] (with 0.1 m of slack; turrets and barrels poke out as they rotate).

Moving parts are separate meshes authored in their own pivot frame (origin = pivot):
- `<v>_turret`: yawed by the game about +Y at the pivot; `<v>_barrel`: same pivot, pitched
  about +X; barrels point along -Z. Pivots in the hull frame:
  light_tank (0, 1.85, 0.3), light_aaa (0, 1.9, 0.4), heavy_tank (0, 2.2, 0.5) and the heavy
  tank's `heavy_tank_mg_turret` / `heavy_tank_mg` at (0, 3.0, 1.2) on the main turret's rear.
  Empties `<v>_muzzle` (`light_tank_muzzle`, `light_aaa_muzzle`, `heavy_tank_muzzle`,
  `heavy_tank_mg_muzzle`) mark the barrel tips IN THE BARREL'S PIVOT FRAME.
- `spawn_bus_wheel`: one wheel, origin at its axle centre, axle along X (radius 0.7, 0.46 wide, hubs to +-0.29);
  the game places 6 at (+-1.55, 0.7, -3.4 / 0 / 3.4) and spins them about X.
- `spawn_bus_mast`: deployable emitter mast, origin at its foot, which stands on the roof
  housing at (0, 3.6, 1.5) in the hull frame; 3.1 m tall.
- `vtol_engine`: one engine pod, origin at its tilt pivot, authored in HOVER pose (fan duct
  horizontal, exhaust down); the game places two at (+-2.6, 1.1, 0.2) and tilts them about X.
  The stub wings end at |x| = 1.6 and a thin shaft (r 0.1) carries on to the pod, so the duct
  (radius 0.85) can turn without touching the body. Empties `vtol_muzzle_l` (-2.6, 0.8, -3.0)
  and `vtol_muzzle_r` (2.6, 0.8, -3.0) are the rocket pods' tips in the body frame.
- Harvester (hit box half extents (2.0, 1.8, 5.5)): `harvester_hull` (cab at the front, turret
  deck behind it, a tapered cargo hopper over the back half with team-lit level gauges on
  each side, chassis bottom at y = 0.5); `harvester_wheel` (one wheel, origin at its axle
  centre, axle along X, radius 0.9, 0.56 wide; the game places 6 at (+-1.85, 0.9, -3.8 / 0 /
  3.8)); `harvester_turret` (the beam emitter's mount, yawed about its pivot at (0, 3.2, -2.0)
  in the hull frame, sitting on the deck at local y = -0.45); `harvester_barrel` (the emitter
  arm, same pivot, pitched about X: a coil-ringed arm, a dish with a white lens and three
  prongs round the emitter spike, tip 2.0 m ahead). Empty `harvester_muzzle` (0, 0, -2.0) in
  the barrel's pivot frame.

- Rover (hit box half extents (1.25, 0.95, 2.3)): `rover_hull` (an open buggy: chassis,
  roll cage, a gun post at the back); `rover_wheel` (one wheel, origin at its axle centre, axle
  along X, radius 0.5, 0.36 wide; the game places 4 at (+-1.1, 0.5, -1.45 / 1.45));
  `rover_turret` / `rover_barrel` at the pivot (0, 1.85, 1.2), tip 1.1 m ahead; empty
  `rover_muzzle` (0, 0, -1.1) in the barrel's pivot frame.
- Interceptor (hit box half extents (3.0, 1.1, 4.0)): `interceptor_hull` (a slim fuselage,
  swept wings, twin canted fins, two nose cannons whose tips are at (+-0.4, 0.9, -4.1) and
  missile rails under the wings whose tips are at (+-2.2, 0.6, -1.5)); empties
  `interceptor_muzzle_l` / `_r` at the cannon tips.
- Dropship (hit box half extents (4.2, 2.0, 7.0)): `dropship_hull` (a long boxy fuselage with
  a glass nose, stub wings carrying an engine nacelle on each side, gun sponsons and a rear
  ramp); `dropship_turret` (a ball mount, yawed about its pivot at (+-3.6, 1.9, 0.5) in the
  hull frame) and `dropship_barrel` (pitched; tip 1.3 m ahead); empty `dropship_muzzle`
  (0, 0, -1.3) in the barrel's pivot frame.

Tracked ones: the hull carries the belts (rubber, open in the middle, a `tread` line round
each one's outer face); `light_tank_wheel` / `light_tank_sprocket` (shared with the AAA) and
`heavy_tank_wheel` / `heavy_tank_sprocket` are one road wheel and one sprocket, origin at the
axle's centre, axle along X: the game places and turns them (src/vehicles.rs). `vtol_fan` is the
fan in a VTOL engine's duct, in the engine's frame, spun about +Y.

Materials: `body` (dark structure), `armor` (plates), `paint` (team paint), `metal`, `rubber`,
`glass`, `heat` (exhausts, brakes, nozzles); `accent` (team neon) with `glow` halos,
`glow_white` (lamps), `flow` (pulsing lines) and `tread` (the tracks' running light). See
README.md (Planetstrike models) for what each looks like in the game; at most ten a mesh.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, X, Y, Z, NX, NY, NZ  # noqa: E402,F401

PARTS = []  # (name, geo, smooth)


def part(name, g, smooth=None):
    """A mesh of the file; `smooth` shades it smoothly across edges bending less than that
    many degrees (round parts: wheels, barrels, canopies), flat otherwise."""
    PARTS.append((name, g, smooth))

L_MUZZLES = []
LIMIT = 4000


# ---------------------------------------------------------------------------------------------
# The kit the vehicles are built from: skins lofted through sections, with every face's quad
# and normal kept (`Surf`) to dress it with plates, panels, vents, stripes and lines.

ARMOR, PAINT, METAL, RUBBER, GLASS, HEAT, POLY = ("armor", "paint", "metal", "rubber", "glass",
                                                  "heat", "polymer")
TREAD, FLOW, HALO = P.TREAD, P.FLOW, P.HALO


def cen(pts):
    return L.mul(tuple(map(sum, zip(*pts))), 1.0 / len(pts))


def sec(z, half, x0=0.0):
    """A closed section at z, X-symmetric about x0: `half` is its right side [(x, y)] from the
    bottom up. Edges 0..n-2 are the right side's, n-1 the top across, then the left side's
    (mirrored, top down) and last the bottom across."""
    r = [(x0 + x, y, z) for x, y in half]
    return r + [(x0 - x, y, z) for x, y in reversed(half)]


def sym(rows):
    """Material chooser for a skin of `sec` sections: rows[k] (or one row for all) lists the
    half's edges' materials from the bottom up, then the top's and the bottom's."""
    def f(k, i):
        row = rows[min(k, len(rows) - 1)] if isinstance(rows[0], (list, tuple)) else rows
        h = len(row) - 1
        if i < h:
            return row[i]
        if i < 2 * h - 1:
            return row[2 * h - 2 - i]
        return row[h]
    return f


def _clean(q):
    out = []
    for p in q:
        if not out or L.length(L.sub(p, out[-1])) > 1e-6:
            out.append(p)
    if len(out) > 1 and L.length(L.sub(out[0], out[-1])) < 1e-6:
        out.pop()
    return out


def rivets(g, a, b, n, count, r=0.018, h=0.008, mat=METAL, ends=False):
    """A row of `count` rivet heads from a to b on the face with normal n (cheap ones: ten
    triangles each)."""
    for i in range(count):
        t = (i / (count - 1) if count > 1 else 0.5) if ends else (i + 0.5) / count
        P.lathe(g, P.lerp(a, b, t), n, [(0, r), (h, r * 0.75)], n=4, mats=mat, cap0=None, cap1=mat)


class Surf:
    """A lofted skin's faces: `q(k, i)` the quad between rings k and k+1 on edge i (ring k's
    edge first), `n(k, i)` its outward normal."""

    def __init__(self, rings, normals):
        self.rings, self.normals = rings, normals

    def q(self, k, i):
        a, b = self.rings[k], self.rings[k + 1]
        j = (i + 1) % len(a)
        return a[i], a[j], b[j], b[i]

    def n(self, k, i):
        return self.normals[(k, i)]

    def at(self, k, i, s, t, h=0.0):
        """The point at s along the ring's edge and t along the loft, h proud of the face."""
        return L.add(P.on_quad(*self.q(k, i), s, t), L.mul(self.n(k, i), h))

    def rect(self, k, i, s0, t0, s1, t1, h=0.0):
        return [self.at(k, i, s0, t0, h), self.at(k, i, s1, t0, h), self.at(k, i, s1, t1, h),
                self.at(k, i, s0, t1, h)]

    def panel(self, g, k, i, s0, t0, s1, t1, **kw):
        P.panel_inset(g, self.rect(k, i, s0, t0, s1, t1), self.n(k, i), **kw)

    def plate(self, g, k, i, s0, t0, s1, t1, h=0.02, mat=ARMOR, **kw):
        P.plate(g, self.rect(k, i, s0, t0, s1, t1), self.n(k, i), h, mat, **kw)

    def decal(self, g, k, i, pts, mat=PAINT, h=0.004):
        """A flat patch through the (s, t) points of the face."""
        g.face([self.at(k, i, s, t, h) for s, t in pts], mat, want=self.n(k, i))

    def line(self, g, k, i, a, b, w, kind="accent", glow=0.05):
        pa, pb = self.at(k, i, *a), self.at(k, i, *b)
        strip(g, [pa, pb], self.n(k, i), w, kind, glow)

    def vent(self, g, k, i, s0, t0, s1, t1, **kw):
        r = self.rect(k, i, s0, t0, s1, t1)
        P.vent(g, cen(r), self.n(k, i), L.length(L.sub(r[1], r[0])),
               L.length(L.sub(r[3], r[0])), up=L.sub(r[3], r[0]), **kw)

    def bolts(self, g, k, i, a, b, count, **kw):
        rivets(g, self.at(k, i, *a), self.at(k, i, *b), self.n(k, i), count, **kw)


def skin(g, rings, mats="body", cap0="body", cap1="body"):
    """Lofts a skin through `rings` (equal-length closed sections, concave ones too, in any
    direction). `mats` is one material or f(k, i) for ring k's edge i (None leaves the face
    out). Returns its `Surf`."""
    n = len(rings[0])
    along = L.sub(cen(rings[-1]), cen(rings[0]))
    normals = {}
    for k in range(len(rings) - 1):
        a, b = rings[k], rings[k + 1]
        nrm = L.newell(a)
        if L.length(nrm) < 1e-9:
            nrm = L.newell(b)
        axis = L.norm(nrm)
        if L.dot(axis, along) < 0:
            axis = L.mul(axis, -1)
        for i in range(n):
            j = (i + 1) % n
            q = _clean([a[i], a[j], b[j], b[i]])
            if len(q) < 3:
                continue
            e = L.add(L.sub(a[j], a[i]), L.sub(b[j], b[i]))
            out = L.cross(e, nrm)
            if L.length(out) < 1e-12:
                continue
            out = L.norm(out)
            ma, mb = L.mul(L.add(a[i], a[j]), 0.5), L.mul(L.add(b[i], b[j]), 0.5)
            d = L.sub(mb, ma)
            want = L.add(L.mul(out, abs(L.dot(d, axis)) + 1e-9), L.mul(axis, -L.dot(d, out)))
            fn = L.newell(q)
            if L.length(fn) < 1e-12:
                continue
            fn = L.norm(fn)
            if L.dot(fn, want) < 0:
                fn = L.mul(fn, -1)
            normals[(k, i)] = fn
            m = mats if isinstance(mats, str) else mats(k, i)
            if m is not None:
                g.face(q, m, want=fn)
    if cap0:
        g.face(rings[0], cap0, want=L.mul(along, -1))
    if cap1:
        g.face(rings[-1], cap1, want=along)
    return Surf(rings, normals)


def strip(g, pts, n, w, kind="accent", glow=0.05, closed=False, h=0.012):
    """A lit strip through 3D points on the plane with normal n: `accent` (team) and `white`
    with halos, `flow` and `tread` (pulses along it) without."""
    if kind == "accent":
        P.accent(g, pts, n, w, closed=closed, glow=glow, h=h)
    elif kind == "white":
        P.white(g, pts, n, w, closed=closed, glow=glow, h=h)
    else:
        f = P.plane(pts[0], n)
        L.strip(g, f, P.uvs(f, pts), w, closed=closed, glow=(0, 0),
                mat=TREAD if kind == "tread" else FLOW, h=min(h, 0.006), sides=False, caps=False)


def decal(g, pts, n, mat=PAINT, h=0.004):
    """A flat patch of `mat` through 3D points, h proud of the face below (normal n)."""
    g.face([L.add(p, L.mul(L.norm(n), h)) for p in pts], mat, want=n)


def lamp(g, c, n, r, depth=0.08, mat="body", lens=LIGHT, sides=8, glow=0.0):
    """A lamp: a housing standing `depth` proud of c along n with a lit lens in its face."""
    n = L.norm(n)
    P.lathe(g, c, n, [(0, r * 1.15), (depth, r * 1.15), (depth, r * 0.9)], n=sides, mats=mat,
            cap0=None, cap1=mat)
    face = L.add(c, L.mul(n, depth))
    P.disc(g, P.circle(face, n, r * 0.85, sides), n, mat=lens, glow=glow, h=0.004)


def handle(g, a, b, n, h=0.05, r=0.012, mat=METAL):
    """A grab handle from a to b standing h proud along n."""
    up = L.mul(L.norm(n), h)
    P.hose(g, [a, L.add(a, up), L.add(b, up), b], r, n=4, mat=mat, caps=False)


def antenna(g, foot, height, r=0.012, tip="accent"):
    P.lathe(g, foot, Y, [(0, r * 3), (0.06, r * 3), (0.08, r)], n=6, mats=METAL, cap0=None,
            cap1=METAL)
    top = L.add(foot, (0, height, 0))
    P.tube(g, L.add(foot, (0, 0.08, 0)), top, r, n=4, mat=METAL, caps=False)
    P.tube(g, top, L.add(top, (0, 0.07, 0)), r * 1.8, n=4, mat=tip)


def pipe(g, a, b, r, mat=HEAT, n=8, bore="rubber", lip=0.0):
    """An open-ended pipe from a to b: its mouth at b a dark bore."""
    d = L.sub(b, a)
    ln = L.length(d)
    prof = [(0, r), (ln, r)] if lip <= 0 else [(0, r), (ln - lip, r), (ln - lip, r * 1.15),
                                              (ln, r * 1.15)]
    P.lathe(g, a, d, prof, n=n, mats=mat, cap0=mat, cap1=None)
    rr = prof[-1][1]
    end = L.norm(d)
    ring = P.circle(b, end, rr, n)
    inner = P.circle(L.sub(b, L.mul(end, r * 0.5)), end, rr * 0.75, n)
    for i in range(n):
        j = (i + 1) % n
        g.face([ring[i], ring[j], inner[j], inner[i]], mat, want=end)
    g.face(inner, bore, want=end)


def track_run(g, s, x0, x1, path, thick=0.07, cleats=(), cleat_gap=0.24, line=True):
    """One side's track (s = +-1): a rubber belt `thick` deep round the closed (z, y) `path`
    (its outside), between |x| = x0 and x1, open in the middle for the wheels. `cleats` are the
    path's edges that get grousers (those seen: the ends' slopes). A `tread` line runs round
    its outer face, U growing the way the top run moves when driving forwards."""
    path = L.ccw(path)
    inner = P._inset2(path, thick)
    xa, xb = sorted((s * x0, s * x1))
    k = len(path)
    for i in range(k):
        j = (i + 1) % k
        (z0, y0), (z1, y1) = path[i], path[j]
        (zi0, yi0), (zi1, yi1) = inner[i], inner[j]
        d = (z1 - z0, y1 - y0)
        ln = math.hypot(*d) or 1.0
        out = (0, -d[0] / ln, d[1] / ln)  # (z, y) CCW: outward = (dy, -dz)
        g.face([(xa, y0, z0), (xa, y1, z1), (xb, y1, z1), (xb, y0, z0)], RUBBER, want=out)
        g.face([(xa, yi0, zi0), (xa, yi1, zi1), (xb, yi1, zi1), (xb, yi0, zi0)], RUBBER,
               want=L.mul(out, -1))
        for x, w in ((xa, NX), (xb, X)):
            g.face([(x, y0, z0), (x, y1, z1), (x, yi1, zi1), (x, yi0, zi0)], RUBBER, want=w)
        if i in cleats:
            m = max(int(ln / cleat_gap), 1)
            for c in range(m):
                t = (c + 0.5) / m
                zc, yc = z0 + d[0] * t, y0 + d[1] * t
                dz, dy = d[0] / ln * 0.04, d[1] / ln * 0.04
                base = [(xa, yc - dy, zc - dz), (xa, yc + dy, zc + dz), (xb, yc + dy, zc + dz),
                        (xb, yc - dy, zc - dz)]
                P.extrude(g, base, L.mul(out, 0.03), RUBBER, caps=(False, True))
    if line:
        xo = s * x1
        mid = P._inset2(path, thick * 0.5)
        pts = [(xo, y, z) for z, y in mid]
        # (CCW in (z, y): along the top run U grows towards -Z, the front.)
        f = P.plane(pts[0], (s, 0, 0))
        L.strip(g, f, P.uvs(f, pts), thick * 0.3, closed=True, glow=(0, 0), mat=TREAD,
                h=0.004, sides=False, caps=False)


def skirt(g, s, x, y0, y1, spans, t=0.035, stripe=None, bolts=3):
    """Armour plates hung along a side (s = +-1) at |x|, over the z `spans`; `stripe` is the
    index of the one carrying a slash of team paint."""
    n = (s, 0, 0)
    for idx, (z0, z1) in enumerate(spans):
        poly = [(s * x, y0, z0), (s * x, y0, z1), (s * x, y1, z1), (s * x, y1, z0)]
        P.plate(g, poly, n, t, ARMOR, bevel=0.03)
        rivets(g, (s * (x + t), y1 - 0.07, z0 + 0.08), (s * (x + t), y1 - 0.07, z1 - 0.08),
               n, bolts, r=0.02)
        if idx == stripe:
            w = (z1 - z0)
            decal(g, [(s * (x + t), y0 + 0.05, z0 + w * 0.3), (s * (x + t), y0 + 0.05, z0 + w * 0.5),
                      (s * (x + t), y1 - 0.14, z0 + w * 0.7), (s * (x + t), y1 - 0.14, z0 + w * 0.5)],
                  n)


def stow_box(g, mn, mx, lid=ARMOR, c=0.025, latch="-z"):
    """A stowage bin: a bevelled box with a lid of `lid` and two latches on one side."""
    P.bevel_box(g, mn, mx, c, mat="body", mats={"+y": lid})
    ax = "xyz".index(latch[1])
    sgn = 1 if latch[0] == "+" else -1
    o = [(a + b) / 2 for a, b in zip(mn, mx)]
    o[ax] = mx[ax] if sgn > 0 else mn[ax]
    other = 0 if ax == 2 else 2
    n = tuple(sgn if i == ax else 0 for i in range(3))
    for d in (-0.3, 0.3):
        c2 = list(o)
        c2[other] += (mx[other] - mn[other]) * d
        c2[1] = mx[1] - (mx[1] - mn[1]) * 0.25
        P.plate(g, P.rect_on(tuple(c2), n, 0.05, 0.09), n, 0.012, METAL)


def hatch_lid(g, c, r, n=10, mat=ARMOR, h=0.05, grip=True):
    """A round hatch on a horizontal face: a rim, a domed lid, hinge and handle."""
    P.lathe(g, c, Y, [(0, r * 1.12), (h * 0.5, r * 1.12), (h * 0.5, r), (h, r * 0.92),
                      (h * 1.3, r * 0.6)], n=n, mats=["body", "body", mat, mat], cap0=None,
            cap1=mat)
    top = L.add(c, (0, h * 1.3, 0))
    if grip:
        handle(g, L.add(top, (-r * 0.25, 0, 0)), L.add(top, (r * 0.25, 0, 0)), Y, h=0.04)
    P.bevel_box(g, L.add(c, (-r * 0.3, 0, r * 0.95)), L.add(c, (r * 0.3, h * 1.1, r * 1.2)),
                0.01, mat=METAL)


def periscope(g, c, yaw=0.0, w=0.14, h=0.08, d=0.1, lens=GLASS):
    """A vision block on a horizontal face at c, looking along -Z turned `yaw` (radians)."""
    cs, sn = math.cos(yaw), math.sin(yaw)

    def rot(p):
        return L.add(c, (p[0] * cs + p[2] * sn, p[1], -p[0] * sn + p[2] * cs))
    b = [(-w / 2, 0, -d / 2), (w / 2, 0, -d / 2), (w / 2, 0, d / 2), (-w / 2, 0, d / 2)]
    t = [(-w / 2, h, -d / 2 + 0.02), (w / 2, h, -d / 2 + 0.02), (w / 2, h, d / 2),
         (-w / 2, h, d / 2)]
    B, T = [rot(p) for p in b], [rot(p) for p in t]
    fwd = (-sn, 0, -cs)
    g.face([B[0], B[1], T[1], T[0]], lens, want=fwd)
    g.face([B[2], B[3], T[3], T[2]], "body", want=L.mul(fwd, -1))
    g.face([B[1], B[2], T[2], T[1]], "body", want=(cs, 0, -sn))
    g.face([B[3], B[0], T[0], T[3]], "body", want=(-cs, 0, sn))
    g.face(T, "body", want=Y)


def footprint(g, foot, layers, mats=ARMOR, cap0="body", cap1=ARMOR):
    """A skin up +Y through an (x, z) footprint scaled per layer (y, sx, sz, dz)."""
    foot = L.ccw([(x, -z) for x, z in foot])  # CCW seen from above
    rings = [[(x * sx, y, -z * sz + dz) for x, z in foot] for y, sx, sz, dz in layers]
    return skin(g, rings, mats, cap0, cap1)


def tow_hook(g, c, n, w=0.16, d=0.1):
    """A towing eye on a vertical face."""
    P.plate(g, P.rect_on(c, n, w, w * 0.8), n, 0.02, METAL, bevel=0.015)
    f = P.plane(c, n)
    P.hose(g, [f.p(-w * 0.28, 0, 0.02), f.p(-w * 0.28, 0, d), f.p(w * 0.28, 0, d),
               f.p(w * 0.28, 0, 0.02)], 0.018, n=4, mat=METAL, caps=False)


def machine_gun(g, reach, box=True, grips=True, r=0.06):
    """A heavy machine gun in its pivot frame, the muzzle `reach` ahead (-Z): receiver, an
    ammunition box with its belt, a slotted jacket, the barrel and its flash hider."""
    P.bevel_box(g, (-0.08, -0.07, -0.26), (0.08, 0.09, 0.26), 0.015, mat=METAL)
    if box:
        P.bevel_box(g, (0.09, -0.14, -0.1), (0.27, 0.06, 0.16), 0.02, mat=PAINT)
        P.hose(g, [(0.18, 0.06, 0.02), (0.14, 0.13, 0.0), (0.04, 0.1, -0.02)], 0.03, n=4,
               mat=METAL, caps=False)
    ln = reach - 0.25
    j = min(0.4, ln * 0.45)
    P.lathe(g, (0, 0.01, -0.25), NZ, [(0, r), (j, r), (j, r * 0.58), (ln - 0.13, r * 0.53),
                                      (ln - 0.13, r * 0.83), (ln, r * 0.92), (ln, r * 0.5),
                                      (ln - 0.07, r * 0.5)],
            n=8, mats=["body", "body", METAL, METAL, HEAT, HEAT, "body"], cap0=None, cap1=RUBBER)
    k = max(int(j / 0.1) - 1, 1)
    for i in range(k):
        z = -0.25 - j * (i + 0.7) / (k + 0.4)
        for n_ in (Y, NY):
            decal(g, P.rect_on((0, 0.01 + n_[1] * r * math.cos(math.pi / 8), z), n_, r * 0.66,
                               0.06, up=NZ), n_, RUBBER, h=0.003)
    if grips:
        for sx in (-1, 1):
            handle(g, (sx * 0.06, 0.0, 0.26), (sx * 0.06, 0.0, 0.3), (sx, 0, 0.3), h=0.07,
                   r=0.012)
    strip(g, [(-0.08, 0.04, -0.2), (-0.08, 0.04, 0.2)], NX, 0.02, glow=0.02)


# ---------------------------------------------------------------------------------------------
# Shared pieces.

def hsec(z, p):
    """Hull section: (hw, yb, ym, yt, tw) -> 6 points; edges 0 bottom, 1 right side,
    2 right slant, 3 top, 4 left slant, 5 left side."""
    hw, yb, ym, yt, tw = p
    return [(-hw, yb, z), (hw, yb, z), (hw, ym, z), (tw, yt, z), (-tw, yt, z), (-hw, ym, z)]


def quad(rings, k, i):
    a, b = rings[k], rings[k + 1]
    j = (i + 1) % len(a)
    return a[i], a[j], b[j], b[i]


def quad_mid(q):
    return L.mul(tuple(map(sum, zip(*q))), 0.25)


def loft_line(g, rings, edge, s, k0, k1, w, kind="accent", t0=0.03, t1=0.97, centre=None):
    """A line along a loft's edge `edge` at fraction s across it, over stations k0..k1."""
    for k in range(k0, k1):
        q = quad(rings, k, edge)
        c = centre or L.mul(tuple(map(sum, zip(*rings[k]))), 1.0 / len(rings[k]))
        n = P.quad_normal(q, L.sub(quad_mid(q), c))
        a = P.on_quad(*q, s, t0 if k == k0 else 0.0)
        b = P.on_quad(*q, s, t1 if k == k1 - 1 else 1.0)
        if kind == "accent":
            P.accent(g, [a, b], n, w, glow=0.05, h=0.012)
        elif kind == "white":
            P.white(g, [a, b], n, w, glow=0.04, h=0.012)
        else:
            P.flow(g, [a, b], n, w, h=0.012)


def track(g, xin, xout, half_len, h, nose, wheels=(), wheel_r=0.3):
    s = 1 if xout > 0 else -1
    x0, x1 = sorted((xin, xout))
    prof = [(-half_len + nose, 0.0), (half_len - nose, 0.0), (half_len, h * 0.55),
            (half_len - nose * 0.5, h), (-half_len + nose * 0.5, h), (-half_len, h * 0.55)]
    L.prism_x(g, prof, x0, x1)
    for z in wheels:
        P.lathe(g, (xout, wheel_r + 0.1, z), (s, 0, 0),
                [(0, wheel_r), (0.035, wheel_r), (0.05, wheel_r * 0.55)], n=8, cap0=None,
                cap1="accent")
    y = h - 0.16
    zl = half_len - nose * 0.75
    P.accent(g, [(xout, y, -zl), (xout, y, zl)], (s, 0, 0), 0.06, glow=0.05)
    # Treads: flow lines along the track top.
    xm = (x0 + x1) / 2
    for dx in (-0.12, 0.12):
        P.flow(g, [(xm + dx, h, -half_len + nose * 0.5 + 0.05),
                   (xm + dx, h, half_len - nose * 0.5 - 0.05)], Y, 0.035)


def turret_ring(g, z, y, r):
    P.lathe(g, (0, y, z), Y, [(0, r), (0.1, r)], n=12, cap0=None, cap1="body")
    P.lathe(g, (0, y, z), Y, [(0.035, r + 0.008), (0.065, r + 0.008)], n=12, mats="accent",
            cap0=None, cap1=None)


def chassis(g, stations, deck_k, front_k=0):
    """Upper hull loft along +Z plus its accents. deck_k: station index where the flat deck
    starts (deck = stations deck_k..deck_k+1)."""
    rings = [hsec(z, p) for z, p in stations]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    k = deck_k
    # Glacis chevrons and headlights.
    q = quad(rings, front_k, 3)
    n = P.quad_normal(q, (0, 1, -1))
    for t in (0.3, 0.62):
        P.accent(g, [P.on_quad(*q, 0.22, t + 0.25), P.on_quad(*q, 0.5, t),
                     P.on_quad(*q, 0.78, t + 0.25)], n, 0.07, glow=0.06)
    for s0, s1 in ((0.05, 0.16), (0.84, 0.95)):
        P.disc(g, [P.on_quad(*q, s0, 0.06), P.on_quad(*q, s1, 0.06), P.on_quad(*q, s1, 0.16),
                   P.on_quad(*q, s0, 0.16)], n, mat=LIGHT, glow=0.05)
    # Side slants: team lines; sides: flow lines; deck edges: lines.
    loft_line(g, rings, 2, 0.45, k, k + 1, 0.07)
    loft_line(g, rings, 4, 0.55, k, k + 1, 0.07)
    loft_line(g, rings, 1, 0.5, k, k + 1, 0.03, kind="flow")
    loft_line(g, rings, 5, 0.5, k, k + 1, 0.03, kind="flow")
    loft_line(g, rings, 3, 0.07, k, k + 1, 0.05)
    loft_line(g, rings, 3, 0.93, k, k + 1, 0.05)
    # Engine grille on the rear deck: flow lines across.
    z1 = stations[k + 1][0]
    yt, tw = stations[k][1][3], stations[k][1][4]
    for dz in (0.2, 0.35, 0.5):
        P.flow(g, [(-tw * 0.6, yt, z1 - dz), (tw * 0.6, yt, z1 - dz)], Y, 0.03)
    # Tail lights on the rear face.
    zr, (hw, yb, ym, _, _) = stations[-1]
    for s in (-1, 1):
        x0, x1 = sorted((s * hw * 0.5, s * hw * 0.8))
        y0 = yb + 0.08
        P.disc(g, [(x0, y0, zr), (x1, y0, zr), (x1, y0 + 0.1, zr), (x0, y0 + 0.1, zr)], Z,
               mat=LIGHT, glow=0.04)
    return rings


def mantlet(g, stations):
    def rs(z, p):
        hx, hy = p
        return [(-hx, -hy, z), (hx, -hy, z), (hx, hy, z), (-hx, hy, z)]
    rings = [rs(z, p) for z, p in stations]
    L.loft(g, rings, NZ, cap0="body", cap1="body")
    q = quad(rings, 0, 2)
    n = P.quad_normal(q, Y)
    for s in (0.2, 0.8):
        P.accent(g, [P.on_quad(*q, s, 0.12), P.on_quad(*q, s, 0.88)], n, 0.04, glow=0.04)
    return rings


def gun(g, o, prof, n=8, rings=(), ring_r=None):
    """Barrel lathe along -Z from o; prof ends at the tip with a white bore cap."""
    mats = ["body"] * (len(prof) - 1)
    P.lathe(g, o, NZ, prof, n=n, mats=mats, cap0="body", cap1=LIGHT)
    for t, r in rings:
        P.lathe(g, o, NZ, [(t, r), (t + 0.06, r)], n=n, mats="accent", cap0=None, cap1=None)


def footprint_loft(g, foot, layers):
    """Loft along +Y of a footprint scaled per layer: layers = [(y, sx, sz, dz)]."""
    rings = [[(x * sx, y, z * sz + dz) for x, z in foot] for y, sx, sz, dz in layers]
    L.loft(g, rings, Y, cap0="body", cap1="body")
    return rings


def belt_lines(g, rings, edges, k=0, w=0.06):
    c = (0, 0, 0)
    for e in edges:
        q = quad(rings, k, e)
        n = P.quad_normal(q, L.sub(quad_mid(q), (0, quad_mid(q)[1], 0)))
        P.accent(g, [P.on_quad(*q, 0.1, 0.5), P.on_quad(*q, 0.9, 0.5)], n, w, glow=0.05)


def top_outline(g, ring, scale, w=0.05):
    cx = sum(p[0] for p in ring) / len(ring)
    cz = sum(p[2] for p in ring) / len(ring)
    y = ring[0][1]
    pts = [(cx + (x - cx) * scale, y, cz + (z - cz) * scale) for x, _, z in ring]
    P.accent(g, pts, Y, w, closed=True, glow=0.05)


def hatch(g, c, r):
    P.lathe(g, c, Y, [(0, r), (0.08, r * 0.9)], n=8, cap0=None, cap1="body")
    top = L.add(c, (0, 0.08, 0))
    P.white(g, P.circle(top, Y, r * 0.55, 8), Y, 0.03, closed=True, glow=0.03)


# ---------------------------------------------------------------------------------------------
# Light tank and light AAA (shared chassis).

# The track's outside in (z, y), and where the game puts the wheels (src/vehicles.rs).
LT_TRACK = [(-1.95, 0.0), (1.95, 0.0), (2.5, 0.22), (2.74, 0.55), (2.66, 0.86), (2.38, 0.98),
            (-2.38, 0.98), (-2.66, 0.86), (-2.74, 0.55), (-2.5, 0.22)]
LT_WHEEL_R = 0.31
LT_SPROCKET_R = 0.3


def light_hull(ring_z, aaa=False):
    g = L.Geo()
    for s in (-1, 1):
        track_run(g, s, 1.04, 1.54, LT_TRACK, cleats=(1, 2, 3, 7, 8, 9))
        skirt(g, s, 1.6, 0.56, 1.03, [(-2.05, -1.08), (-1.03, -0.03), (0.02, 1.02), (1.07, 2.05)],
              stripe=1 if s > 0 else 2)
    # The hull: a tub between the tracks, fenders over them, sloped shoulders up to the deck.
    st = [(-2.86, 0.8, 1.48, 1.06, 1.12, 1.1), (-2.55, 0.46, 1.6, 1.12, 1.2, 1.2),
          (-1.55, 0.36, 1.63, 1.26, 1.25, 1.5), (2.25, 0.36, 1.63, 1.26, 1.25, 1.5),
          (2.86, 0.6, 1.56, 1.2, 1.2, 1.36)]
    rings = [sec(z, [(1.0, yb), (1.0, 1.02), (xf, 1.02), (xf, ysh), (xd, yt)])
             for z, yb, xf, ysh, xd, yt in st]
    row = ["body", "body", "body", ARMOR, ARMOR, "body"]
    s = skin(g, rings, sym([row, row, row, ["body", "body", "body", ARMOR, "body", "body"]]))
    TOP, SHR, SHL = 4, 3, 5
    # Glacis: two framed plates, a slash of paint, the driver's vision blocks at its top.
    for a, b in ((0.06, 0.44), (0.56, 0.94)):
        s.panel(g, 1, TOP, a, 0.1, b, 0.9, h=0.014, rim=0.045, mat_in="body")
    for a in (0.1, 0.6):
        s.decal(g, 1, TOP, [(a + 0.02, 0.3), (a + 0.1, 0.3), (a + 0.3, 0.7), (a + 0.22, 0.7)],
                h=0.008)
    s.bolts(g, 0, TOP, (0.05, 0.5), (0.95, 0.5), 8, r=0.02)
    for x in (-0.32, 0.0, 0.32):
        periscope(g, (x, 1.5, -1.42), yaw=-x * 0.5, w=0.2, h=0.09, lens=RUBBER)
    hatch_lid(g, (0.0, 1.5, -1.05), 0.3)
    # Headlights on the fenders' fronts, each behind a guard; towing eyes on the bow.
    for sx in (-1, 1):
        lamp(g, (sx * 1.36, 1.21, -2.5), (0, 0.12, -1), 0.1, depth=0.1)
        handle(g, (sx * 1.2, 1.2, -2.56), (sx * 1.52, 1.16, -2.56), (0, 0.4, -1), h=0.16,
               r=0.014)
        tow_hook(g, (sx * 0.6, 0.72, -2.78), (0, -0.55, -1))
        strip(g, [(sx * 1.0, 1.07, -2.865), (sx * 0.3, 1.09, -2.865)], NZ, 0.03, glow=0.03)
    # Shoulders: plates with bolts, a team line under the deck's edge, stowage and tools.
    for e, sx in ((SHR, 1), (SHL, -1)):
        lo, hi = (0.0, 1.0) if e == SHR else (1.0, 0.0)

        def u(v, lo=lo, hi=hi):
            return lo + (hi - lo) * v
        for t0, t1 in ((0.03, 0.3), (0.33, 0.6)):
            s.panel(g, 2, e, u(0.12), t0, u(0.7), t1, h=0.012, rim=0.04, mat_in="body")
        s.line(g, 2, e, (u(0.86), 0.03), (u(0.86), 0.97), 0.035, glow=0.04)
        s.bolts(g, 2, e, (u(0.06), 0.04), (u(0.06), 0.96), 12, r=0.016)
    stow_box(g, (1.3, 1.3, 1.0), (1.62, 1.56, 1.95), latch="+x")
    stow_box(g, (-1.62, 1.3, 1.25), (-1.3, 1.52, 1.95), latch="-x")
    # (A shovel and a pry bar clipped to the left shoulder.)
    P.hose(g, [(-1.45, 1.42, -0.2), (-1.45, 1.42, 0.9)], 0.02, n=5, mat=METAL)
    P.plate(g, [(-1.55, 1.4, -0.5), (-1.35, 1.4, -0.5), (-1.37, 1.4, -0.2), (-1.53, 1.4, -0.2)],
            Y, 0.03, METAL, bevel=0.01)
    for z in (0.0, 0.7):
        P.bevel_box(g, (-1.5, 1.36, z), (-1.4, 1.46, z + 0.06), 0.01)
    # Deck: the turret ring, the engine's louvres (lit from inside) and filler caps.
    P.lathe(g, (0, 1.5, ring_z), Y, [(0, 0.98), (0.04, 0.98), (0.04, 0.9), (0.11, 0.9)], n=16,
            mats=[ARMOR, ARMOR, METAL], cap0=None, cap1="body")
    for sx in (-1, 1):
        o = (sx * 0.62, 1.5, 1.72)
        P.vent(g, o, Y, 0.9, 0.82, slats=6, depth=0.035, mat=ARMOR, mat_in=HEAT, up=NZ,
               frame=0.04)
        P.lathe(g, (sx * 1.0, 1.5, 1.05), Y, [(0, 0.08), (0.04, 0.08), (0.05, 0.05)], n=8,
                mats=METAL, cap0=None, cap1=METAL)
    rivets(g, (0, 1.5, 1.34), (0, 1.5, 2.1), Y, 6, r=0.018)
    # Rear: exhausts out of the shoulders, tail lights, a towing eye, the engine's doors.
    for sx in (-1, 1):
        pipe(g, (sx * 1.38, 1.36, 2.3), (sx * 1.38, 1.44, 2.72), 0.09, lip=0.08)
        P.bevel_box(g, (sx * 1.38 - 0.14, 1.3, 2.2), (sx * 1.38 + 0.14, 1.45, 2.5), 0.02,
                    mat=ARMOR)
        P.disc(g, P.rect_on((sx * 1.2, 1.0, 2.86), Z, 0.26, 0.08), Z, glow=0.04)
        P.panel_inset(g, P.rect_on((sx * 0.45, 0.95, 2.86), Z, 0.7, 0.5), Z, h=0.012,
                      rim=0.04, mat="body", mat_in=ARMOR)
        rivets(g, (sx * 0.45 - 0.28, 0.73, 2.87), (sx * 0.45 + 0.28, 0.73, 2.87), Z, 4,
                   r=0.016)
    tow_hook(g, (0, 0.7, 2.86), Z, d=0.05)
    if aaa:
        # (Ready ammunition where the tank carries stowage.)
        for z in (-0.95, -0.55):
            P.lathe(g, (1.42, 1.3, z), Y, [(0, 0.15), (0.3, 0.15), (0.34, 0.1)], n=10,
                    mats=[METAL, ARMOR], cap0=None, cap1=ARMOR)
    return g


def light_wheels():
    g = L.Geo()
    P.road_wheel(g, (0, 0, 0), X, LT_WHEEL_R, 0.36, n=14, cap_mat=METAL)
    part("light_tank_wheel", g, smooth=40)
    g = L.Geo()
    P.tyre(g, (0, 0, 0), X, LT_SPROCKET_R * 0.9, 0.34, n=12, lugs=12, hub=0.62, mat=METAL,
           hub_mat="body", cap_mat=METAL, lug_h=LT_SPROCKET_R * 0.1)
    part("light_tank_sprocket", g, smooth=40)


def smoke_launchers(g, s, c, n=3):
    """A bank of launcher tubes on a turret's cheek (s = +-1), fanned out and up."""
    for i in range(n):
        a = L.add(c, (0, 0.0, i * 0.13))
        d = L.norm((s * (0.75 + 0.1 * i), 0.5, -0.6))
        pipe(g, a, L.add(a, L.mul(d, 0.2)), 0.045, mat=METAL, n=6)
    P.bevel_box(g, L.add(c, (-0.05, -0.07, -0.08)), L.add(c, (0.05, 0.03, n * 0.13)), 0.012)


def light_tank():
    part("light_tank_hull", light_hull(0.3))
    light_wheels()
    # Turret (pivot frame; the hull's deck at local y = -0.35).
    g = L.Geo()
    foot = [(0.48, -1.22), (0.96, -0.72), (1.02, 0.5), (0.8, 1.12), (-0.8, 1.12), (-1.02, 0.5),
            (-0.96, -0.72), (-0.48, -1.22)]
    s = footprint(g, foot, [(-0.24, 0.86, 0.86, 0.0), (-0.08, 1, 1, 0), (0.24, 0.95, 0.95, 0.02),
                            (0.42, 0.74, 0.76, 0.1)],
                  mats=lambda k, i: "body" if k == 0 else ARMOR, cap1=ARMOR)
    n = len(foot)
    for i in range(n):
        q = s.q(1, i)
        w = L.length(L.sub(q[1], q[0]))
        if w > 1.0:  # the long cheeks: a framed plate, bolts, a team line low down
            s.panel(g, 1, i, 0.08, 0.3, 0.92, 0.9, h=0.012, rim=0.04, mat_in="body")
            s.line(g, 1, i, (0.05, 0.14), (0.95, 0.14), 0.035, glow=0.04)
        elif w > 0.6:
            s.bolts(g, 1, i, (0.1, 0.5), (0.9, 0.5), 4, r=0.018)
    # Roof: the commander's cupola with its ring of vision blocks, the loader's hatch, a sight.
    cup = (-0.4, 0.42, 0.45)
    P.lathe(g, cup, Y, [(0, 0.33), (0.07, 0.33), (0.07, 0.31), (0.16, 0.31), (0.16, 0.3),
                        (0.2, 0.26)], n=10, mats=[ARMOR, ARMOR, GLASS, ARMOR, ARMOR], cap0=None,
            cap1=ARMOR)
    hatch_lid(g, L.add(cup, (0, 0.2, 0)), 0.22, h=0.035)
    hatch_lid(g, (0.42, 0.42, 0.5), 0.26, h=0.04)
    P.bevel_box(g, (0.34, 0.4, -0.72), (0.62, 0.62, -0.38), 0.025, mat=ARMOR,
                mats={"-z": GLASS})
    for x in (-0.62, -0.38):
        periscope(g, (x, 0.42, -0.4), w=0.16)
    for sx in (-1, 1):
        smoke_launchers(g, sx, (sx * 1.02, 0.1, -0.45))
        P.disc(g, P.rect_on((sx * 0.5, 0.05, 1.125), Z, 0.22, 0.07), Z, glow=0.035)
    # The bustle rack with its bins, and the aerial.
    P.hose(g, [(-0.78, 0.3, 1.12), (-0.78, 0.3, 1.5), (0.78, 0.3, 1.5), (0.78, 0.3, 1.12)], 0.02,
           n=4, mat=METAL, caps=False)
    P.hose(g, [(-0.78, -0.05, 1.12), (-0.78, -0.05, 1.5), (0.78, -0.05, 1.5),
               (0.78, -0.05, 1.12)], 0.02, n=4, mat=METAL, caps=False)
    for x in (-0.78, 0.0, 0.78):
        P.hose(g, [(x, -0.05, 1.5), (x, 0.3, 1.5)], 0.016, n=4, mat=METAL, caps=False)
    stow_box(g, (-0.7, -0.03, 1.14), (-0.08, 0.26, 1.46), lid="body", latch="+z")
    P.lathe(g, (0.4, 0.1, 1.3), X, [(-0.28, 0.13), (0.28, 0.13)], n=8, mats=POLY, cap0=POLY,
            cap1=POLY)
    antenna(g, (-0.55, 0.42, 0.82), 1.0)
    decal(g, [(0.1, 0.42, 0.74), (0.6, 0.42, 0.74), (0.56, 0.42, 0.9), (0.1, 0.42, 0.9)], Y,
          h=0.006)
    part("light_tank_turret", g)
    # Barrel: mantlet, sleeve, bore evacuator, muzzle brake; a coaxial gun beside it.
    g = L.Geo()

    def ms(z, hx, hy, c):
        return [(-hx + c, -hy, z), (hx - c, -hy, z), (hx, -hy + c, z), (hx, hy - c, z),
                (hx - c, hy, z), (-hx + c, hy, z), (-hx, hy - c, z), (-hx, -hy + c, z)]
    skin(g, [ms(-0.7, 0.42, 0.27, 0.06), ms(-1.12, 0.38, 0.25, 0.07), ms(-1.34, 0.22, 0.17, 0.06)],
         ARMOR, "body", ARMOR)
    P.lathe(g, (0, 0, -1.25), NZ,
            [(0, 0.15), (0.22, 0.15), (0.22, 0.115), (0.8, 0.108), (0.8, 0.15), (1.22, 0.15),
             (1.22, 0.105), (1.6, 0.1), (1.6, 0.145), (1.95, 0.145), (1.95, 0.075), (1.8, 0.075)],
            n=12, mats=[ARMOR, ARMOR, METAL, METAL, "body", METAL, METAL, METAL, HEAT, HEAT, "body"],
            cap0=None, cap1="rubber")
    for sx in (-1, 1):  # the brake's ports
        for z in (-2.93, -3.07):
            decal(g, P.rect_on((sx * 0.145 * math.cos(math.pi / 12), 0, z), (sx, 0, 0), 0.08, 0.11),
                  (sx, 0, 0), RUBBER, h=0.003)
    P.lathe(g, (0, 0, -2.42), NZ, [(0, 0.155), (0.04, 0.155)], n=12, mats="accent", cap0=None,
            cap1=None)
    pipe(g, (0.28, 0.05, -1.1), (0.28, 0.05, -1.75), 0.035, mat=METAL, n=6)
    rivets(g, (-0.3, 0.25, -0.9), (0.3, 0.25, -0.9), Y, 4, r=0.02)
    part("light_tank_barrel", g, smooth=40)
    L_MUZZLES.append(("light_tank_muzzle", (0, 0, -3.2)))


def light_aaa():
    part("light_aaa_hull", light_hull(0.4, aaa=True))
    # Turret (pivot frame; deck at local y = -0.4): an open ring mount, the gunner's shield
    # either side of the cradle, a tracking radar on the back.
    g = L.Geo()
    P.lathe(g, (0, -0.29, 0), Y, [(0, 0.88), (0.1, 0.94), (0.2, 0.94), (0.26, 0.8)], n=12,
            mats=["body", ARMOR, ARMOR], cap0=None, cap1="body")
    strip(g, P.circle((0, -0.03, 0), Y, 0.72, 12), Y, 0.035, closed=True, glow=0.04)
    for sx in (-1, 1):
        x0, x1 = sorted((sx * 0.7, sx * 0.86))
        # Trunnion cheeks.
        prof = [(-0.6, -0.05), (0.62, -0.05), (0.62, 0.22), (0.3, 0.5), (-0.28, 0.5),
                (-0.6, 0.2)]
        L.prism_x(g, prof, x0, x1, ARMOR)
        xo = sx * 0.86
        P.panel_inset(g, [(xo, 0.02, -0.5), (xo, 0.02, 0.5), (xo, 0.2, 0.5), (xo, 0.2, -0.5)],
                      (sx, 0, 0), h=0.012, rim=0.035, mat_in="body")
        P.lathe(g, (xo, 0.0, 0.0), (sx, 0, 0), [(0, 0.2), (0.03, 0.2), (0.05, 0.12)], n=10,
                mats=METAL, cap0=None, cap1=METAL)
        decal(g, [(xo, 0.26, -0.3), (xo, 0.26, 0.26), (xo, 0.44, 0.1), (xo, 0.44, -0.2)],
              (sx, 0, 0), h=0.006)
        # Ammunition drums behind the cheeks, a feed chute each.
        P.lathe(g, (sx * 0.55, 0.12, 0.62), (sx, 0, 0), [(-0.14, 0.26), (0.14, 0.26)], n=12,
                mats=METAL, cap0="body", cap1="body")
        P.hose(g, [(sx * 0.5, 0.3, 0.5), (sx * 0.5, 0.42, 0.25), (sx * 0.5, 0.3, 0.0)], 0.05, n=4,
               mat=RUBBER, caps=False)
    # The radar: a pedestal, a yoke and a dished antenna with its feed horn.
    L.box(g, (-0.3, -0.05, 0.62), (0.3, 0.2, 0.95), ARMOR)
    P.tube(g, (0, 0.2, 0.8), (0, 0.5, 0.8), 0.06, n=6, mat=METAL)
    P.lathe(g, (0, 0.62, 0.9), NZ, [(0, 0.1), (0.05, 0.28), (0.16, 0.4), (0.18, 0.4)], n=12,
            mats=[ARMOR, ARMOR, METAL], cap0=ARMOR, cap1=None)
    P.lathe(g, (0, 0.62, 0.9), NZ, [(0.045, 0.1), (0.055, 0.27), (0.165, 0.39)], n=12,
            mats=GLASS, cap0=GLASS, cap1=None)
    P.tube(g, (0, 0.62, 0.86), (0, 0.62, 0.62), 0.02, n=4, mat=METAL)
    P.tube(g, (0, 0.62, 0.62), (0, 0.62, 0.58), 0.04, n=6, mat="accent")
    part("light_aaa_turret", g, smooth=35)
    # The guns: a cradle on the trunnion, four barrels abreast 0.32 m apart (as the game fires
    # them), perforated jackets, flash hiders; tips 2.4 m ahead.
    g = L.Geo()
    P.tube(g, (-0.7, 0, 0), (0.7, 0, 0), 0.11, n=10, mat=METAL)
    P.bevel_box(g, (-0.66, -0.2, -0.62), (0.66, 0.2, 0.42), 0.04, mat=ARMOR, mats={"+z": "body"})
    P.panel_inset(g, P.rect_on((0, 0.2, -0.1), Y, 1.0, 0.7, up=NZ), Y, h=0.012, rim=0.04,
                  mat_in="body")
    for bx in (-0.48, -0.16, 0.16, 0.48):
        P.lathe(g, (bx, 0, -0.6), NZ,
                [(0, 0.085), (0.75, 0.085), (0.75, 0.05), (1.5, 0.045), (1.5, 0.07), (1.8, 0.085),
                 (1.8, 0.05), (1.7, 0.05)],
                n=8, mats=["body", "body", METAL, METAL, HEAT, HEAT, "body"], cap0=None,
                cap1="rubber")
        for z in (-0.78, -0.96, -1.14):  # the jacket's cooling slots
            for a in (0, 1):
                n_ = (0, 1, 0) if a == 0 else (0, -1, 0)
                decal(g, P.rect_on((bx, n_[1] * 0.085 * math.cos(math.pi / 8), z), n_, 0.05, 0.1,
                                   up=NZ), n_, RUBBER, h=0.003)
    for sx in (-1, 1):
        strip(g, [(sx * 0.66, 0, 0.3), (sx * 0.66, 0, -0.5)], (sx, 0, 0), 0.03, "flow")
    part("light_aaa_barrel", g, smooth=40)
    L_MUZZLES.append(("light_aaa_muzzle", (0, 0, -2.4)))


# ---------------------------------------------------------------------------------------------
# Heavy tank.

HT_TRACK = [(-2.7, 0.0), (2.7, 0.0), (3.4, 0.28), (3.72, 0.7), (3.62, 1.05), (3.3, 1.16),
            (-3.3, 1.16), (-3.62, 1.05), (-3.72, 0.7), (-3.4, 0.28)]
HT_WHEEL_R = 0.38
HT_SPROCKET_R = 0.34


def era(g, s, k, i, cols, rows, s0, t0, s1, t1, gap=0.012, h=0.05):
    """A field of reactive armour bricks on a skin's face."""
    for c in range(cols):
        for r in range(rows):
            a0 = s0 + (s1 - s0) * c / cols + gap
            a1 = s0 + (s1 - s0) * (c + 1) / cols - gap
            b0 = t0 + (t1 - t0) * r / rows + gap
            b1 = t0 + (t1 - t0) * (r + 1) / rows - gap
            s.plate(g, k, i, a0, b0, a1, b1, h=h, bevel=0.02)


def heavy_tank():
    g = L.Geo()
    for sx in (-1, 1):
        track_run(g, sx, 1.45, 2.12, HT_TRACK, thick=0.08, cleats=(1, 2, 3, 7, 8, 9),
                  cleat_gap=0.42)
        spans = [(-2.9, -1.76), (-1.72, -0.58), (-0.54, 0.6), (0.64, 1.82), (1.86, 2.9)]
        skirt(g, sx, 2.2, 0.62, 1.22, spans, t=0.05, stripe=2, bolts=3)
        for z0, z1 in spans[:4]:  # bricks on the skirts' upper halves
            for c in range(2):
                za = z0 + 0.06 + c * (z1 - z0 - 0.12) / 2 + 0.01
                zb = z0 + 0.06 + (c + 1) * (z1 - z0 - 0.12) / 2 - 0.01
                x = sx * 2.25
                P.plate(g, [(x, 0.66, za), (x, 0.66, zb), (x, 0.98, zb), (x, 0.98, za)],
                        (sx, 0, 0), 0.04, ARMOR, bevel=0.02)
    st = [(-3.86, 1.0, 2.0, 1.28, 1.5, 1.34), (-3.4, 0.5, 2.15, 1.4, 1.6, 1.5),
          (-2.1, 0.4, 2.2, 1.6, 1.75, 1.9), (3.2, 0.4, 2.2, 1.6, 1.75, 1.9),
          (3.86, 0.7, 2.1, 1.5, 1.65, 1.72)]
    rings = [sec(z, [(1.4, yb), (1.4, 1.2), (xf, 1.2), (xf, ysh), (xd, yt)])
             for z, yb, xf, ysh, xd, yt in st]
    row = ["body", "body", "body", ARMOR, ARMOR, "body"]
    s = skin(g, rings, sym([row, row, row, ["body", "body", "body", ARMOR, "body", "body"]]))
    TOP, SHR, SHL = 4, 3, 5
    # Glacis under reactive bricks, the driver's hatch and vision blocks above it.
    era(g, s, 1, TOP, 6, 2, 0.04, 0.12, 0.96, 0.8)
    s.bolts(g, 0, TOP, (0.05, 0.5), (0.95, 0.5), 10, r=0.022)
    s.decal(g, 1, TOP, [(0.3, 0.84), (0.7, 0.84), (0.66, 0.96), (0.34, 0.96)], h=0.006)
    for x in (-0.36, 0.0, 0.36):
        periscope(g, (x, 1.9, -1.95), yaw=-x * 0.45, w=0.22, h=0.1, lens=RUBBER)
    hatch_lid(g, (0.0, 1.9, -1.55), 0.32)
    # Bow: lamp clusters on the fenders, towing eyes, spare links, a tow cable over the glacis.
    for sx in (-1, 1):
        for dx in (0.0, 0.3):
            lamp(g, (sx * (1.72 + dx), 1.5, -3.38), (0, 0.15, -1), 0.1, depth=0.1, sides=6)
        handle(g, (sx * 1.56, 1.46, -3.46), (sx * 2.16, 1.4, -3.46), (0, 0.5, -1), h=0.2,
               r=0.016)
        tow_hook(g, (sx * 0.85, 0.82, -3.66), (0, -0.92, -1), w=0.2)
        strip(g, [(sx * 1.4, 1.3, -3.865), (sx * 0.35, 1.32, -3.865)], NZ, 0.035, glow=0.035)
    for i in range(5):
        x = -0.5 + i * 0.25
        P.bevel_box(g, (x - 0.11, 0.9, -3.84), (x + 0.11, 1.16, -3.72), 0.02, mat=RUBBER)
    P.hose(g, [(1.5, 1.52, -3.3), (1.2, 1.62, -2.95), (1.42, 1.78, -2.45), (1.6, 1.9, -2.0),
               (1.6, 1.93, -1.0)], 0.028, n=5, mat=METAL, collars=(0, 4))
    # Shoulders: plates, a team line, bins, tools, the exhaust stacks at the back.
    for e, sx in ((SHR, 1), (SHL, -1)):
        lo, hi = (0.0, 1.0) if e == SHR else (1.0, 0.0)

        def u(v, lo=lo, hi=hi):
            return lo + (hi - lo) * v
        for t0, t1 in ((0.03, 0.24), (0.27, 0.48)):
            s.panel(g, 2, e, u(0.12), t0, u(0.72), t1, h=0.014, rim=0.045, mat_in="body")
        s.line(g, 2, e, (u(0.88), 0.03), (u(0.88), 0.97), 0.04, glow=0.045)
        s.bolts(g, 2, e, (u(0.05), 0.03), (u(0.05), 0.97), 8, r=0.022)
        stow_box(g, (min(sx * 1.78, sx * 2.2), 1.64, 0.55), (max(sx * 1.78, sx * 2.2), 1.96, 1.6),
                 latch="+x" if sx > 0 else "-x")
        # Exhaust stack: a shrouded riser with a raked mouth.
        x = sx * 1.95
        P.bevel_box(g, (x - 0.2, 1.6, 2.15), (x + 0.2, 2.25, 2.75), 0.04, mat=ARMOR)
        P.vent(g, (x + sx * 0.2, 1.95, 2.45), (sx, 0, 0), 0.44, 0.42, slats=3, depth=0.03,
               mat="body")
        pipe(g, (x, 2.2, 2.45), (x, 2.62, 2.58), 0.13, lip=0.1, n=10)
    P.hose(g, [(-1.95, 1.76, -1.6), (-1.95, 1.76, -0.2)], 0.022, n=5, mat=METAL)
    P.plate(g, [(-2.08, 1.72, -1.95), (-1.84, 1.72, -1.95), (-1.86, 1.72, -1.6),
                (-2.06, 1.72, -1.6)], Y, 0.035, METAL, bevel=0.012)
    P.hose(g, [(1.95, 1.76, -1.7), (1.95, 1.76, -0.3)], 0.025, n=5, mat=METAL)
    # Deck: the turret ring on a raised collar, the engine deck behind it.
    P.lathe(g, (0, 1.9, 0.5), Y, [(0, 1.42), (0.05, 1.42), (0.05, 1.3), (0.1, 1.3)], n=16,
            mats=[ARMOR, ARMOR, METAL], cap0=None, cap1="body")
    P.bevel_box(g, (-1.6, 1.9, 2.0), (1.6, 2.04, 3.2), 0.04, mat=ARMOR)
    for sx in (-1, 1):
        o = (sx * 0.78, 2.04, 2.6)
        P.vent(g, o, Y, 1.3, 0.95, slats=5, depth=0.04, mat=ARMOR, mat_in=HEAT, up=NZ,
               frame=0.05)
        P.lathe(g, (sx * 1.35, 1.9, 1.8), Y, [(0, 0.1), (0.05, 0.1), (0.07, 0.06)], n=8,
                mats=METAL, cap0=None, cap1=METAL)
    rivets(g, (0, 2.04, 2.1), (0, 2.04, 3.1), Y, 7, r=0.02)
    # Stern: fuel drums on brackets, tail lights, the engine's doors, a towing eye.
    for sx in (-1, 1):
        P.lathe(g, (sx * 0.2, 2.03, 3.56), (sx, 0, 0), [(0, 0.24), (0.5, 0.24), (0.56, 0.24),
                                                       (1.16, 0.24)],
                n=10, mats=[METAL, RUBBER, METAL], cap0=METAL, cap1=METAL)
        P.disc(g, P.rect_on((sx * 1.8, 1.4, 3.865), Z, 0.3, 0.09), Z, glow=0.045)
        P.panel_inset(g, P.rect_on((sx * 0.6, 0.98, 3.865), Z, 0.9, 0.44), Z, h=0.014, rim=0.045,
                      mat="body", mat_in=ARMOR)
        P.bevel_box(g, (sx * 0.75 - 0.05, 1.7, 3.44), (sx * 0.75 + 0.05, 1.86, 3.7), 0.012,
                    mat=METAL)
    tow_hook(g, (0, 0.82, 3.865), Z, w=0.2, d=0.04)
    part("heavy_tank_hull", g, smooth=None)
    g = L.Geo()
    P.road_wheel(g, (0, 0, 0), X, HT_WHEEL_R, 0.48, n=14, cap_mat=METAL)
    part("heavy_tank_wheel", g, smooth=40)
    g = L.Geo()
    P.tyre(g, (0, 0, 0), X, HT_SPROCKET_R * 0.9, 0.46, n=12, lugs=14, hub=0.62, mat=METAL,
           hub_mat="body", cap_mat=METAL, lug_h=HT_SPROCKET_R * 0.1)
    part("heavy_tank_sprocket", g, smooth=40)

    # Main turret (pivot (0, 2.2, 0.5); the ring's top at local -0.2). The roof is kept flat and
    # clear within 1.2 m of the axis: the gunner's cupola stands over the hull at (0, 0.8, 0.7)
    # and the turret turns under it, on the race rings.
    g = L.Geo()
    foot = [(0.7, -2.0), (1.5, -1.2), (1.55, 1.1), (1.25, 1.72), (-1.25, 1.72), (-1.55, 1.1),
            (-1.5, -1.2), (-0.7, -2.0)]
    s = footprint(g, foot, [(-0.2, 0.88, 0.9, 0.0), (0.0, 1, 1, 0), (0.4, 0.97, 0.97, 0.0),
                            (0.6, 0.84, 0.86, 0.04)],
                  mats=lambda k, i: "body" if k == 0 else ARMOR, cap1=ARMOR)
    for i in range(len(foot)):
        q = s.q(1, i)
        w = L.length(L.sub(q[1], q[0]))
        mid = cen(q)
        if w > 2.0:  # the long cheeks: wedges of add-on armour, a slash of paint, a team line
            s.plate(g, 1, i, 0.04, 0.2, 0.62, 0.96, h=0.09, bevel=0.05)
            s.decal(g, 1, i, [(0.2, 0.3), (0.3, 0.3), (0.42, 0.86), (0.32, 0.86)], h=0.096)
            s.panel(g, 1, i, 0.66, 0.24, 0.95, 0.92, h=0.014, rim=0.04, mat_in="body")
            s.line(g, 1, i, (0.03, 0.09), (0.97, 0.09), 0.04, glow=0.045)
        elif mid[2] < 0 and w > 0.9:  # the front's facets: bricks
            era(g, s, 1, i, 3, 2, 0.06, 0.12, 0.94, 0.9, h=0.045)
        elif mid[2] > 0 and w < 1.0:
            s.bolts(g, 1, i, (0.12, 0.5), (0.88, 0.5), 4, r=0.02)
    for r in (0.3, 1.14):
        P.lathe(g, (0, 0.6, 0), Y, [(0, r - 0.03), (0.025, r - 0.03), (0.025, r + 0.03),
                                    (0, r + 0.03)], n=24, mats=METAL, cap0=None, cap1=None)
    # Roof, outside the race: the gunner's sight, the panoramic sight, hatch, wind mast.
    P.bevel_box(g, (0.5, 0.58, -1.62), (0.88, 0.86, -1.26), 0.03, mat=ARMOR, mats={"-z": GLASS})
    P.lathe(g, (-0.7, 0.6, -1.42), Y, [(0, 0.14), (0.12, 0.14), (0.12, 0.11), (0.3, 0.11),
                                       (0.34, 0.07)], n=10, mats=[ARMOR, ARMOR, GLASS, ARMOR],
            cap0=None, cap1=ARMOR)
    hatch_lid(g, (0.0, 0.6, -1.36), 0.2, h=0.035, grip=False)
    antenna(g, (-1.05, 0.6, 1.25), 1.3)
    antenna(g, (1.05, 0.6, 1.25), 0.9, tip=METAL)
    P.tube(g, (1.12, 0.6, -0.9), (1.12, 0.95, -0.9), 0.02, n=4, mat=METAL)
    P.lathe(g, (1.12, 0.95, -0.9), Y, [(0, 0.05), (0.03, 0.07), (0.06, 0.05)], n=6, mats=METAL,
            cap0=METAL, cap1=METAL)
    for sx in (-1, 1):
        smoke_launchers(g, sx, (sx * 1.3, 0.28, -1.55), n=4)
        P.disc(g, P.rect_on((sx * 0.85, 0.08, 1.725), Z, 0.3, 0.09), Z, glow=0.04)
    # The bustle rack: bins, fuel cans, a rolled tarp.
    for y in (-0.08, 0.36):
        P.hose(g, [(-1.2, y, 1.72), (-1.2, y, 2.2), (1.2, y, 2.2), (1.2, y, 1.72)], 0.024, n=4,
               mat=METAL, caps=False)
    for x in (-1.2, -0.4, 0.4, 1.2):
        P.hose(g, [(x, -0.08, 2.2), (x, 0.36, 2.2)], 0.018, n=4, mat=METAL, caps=False)
    stow_box(g, (-1.1, -0.05, 1.75), (-0.2, 0.32, 2.15), lid="body", latch="+z")
    for x in (0.0, 0.3):
        P.bevel_box(g, (x, -0.05, 1.78), (x + 0.24, 0.3, 2.12), 0.03, mat=PAINT)
        P.lathe(g, (x + 0.12, 0.3, 1.95), Y, [(0, 0.04), (0.04, 0.04)], n=6, mats=METAL, cap0=None,
                cap1=METAL)
    P.lathe(g, (0.62, 0.1, 1.96), X, [(0, 0.15), (0.5, 0.15)], n=8, mats=POLY, cap0=POLY, cap1=POLY)
    part("heavy_tank_turret", g)

    # Main gun (4.4 m): a deep mantlet, thermal sleeve, bore evacuator, a three-baffle brake.
    g = L.Geo()

    def ms(z, hx, hy, c):
        return [(-hx + c, -hy, z), (hx - c, -hy, z), (hx, -hy + c, z), (hx, hy - c, z),
                (hx - c, hy, z), (-hx + c, hy, z), (-hx, hy - c, z), (-hx, -hy + c, z)]
    skin(g, [ms(-1.15, 0.62, 0.36, 0.08), ms(-1.9, 0.52, 0.33, 0.1), ms(-2.18, 0.3, 0.23, 0.08)],
         ARMOR, "body", ARMOR)
    P.lathe(g, (0, 0, -2.1), NZ,
            [(0, 0.21), (0.3, 0.21), (0.3, 0.165), (0.75, 0.16), (0.75, 0.175), (1.0, 0.175),
             (1.0, 0.23), (1.45, 0.23), (1.45, 0.155), (1.85, 0.15), (1.85, 0.22), (1.97, 0.22),
             (1.97, 0.17), (2.05, 0.17), (2.05, 0.22), (2.17, 0.22), (2.17, 0.17), (2.2, 0.17),
             (2.2, 0.21), (2.3, 0.21), (2.3, 0.11), (2.1, 0.11)],
            n=12, mats=[ARMOR, ARMOR, METAL, "body", METAL, METAL, "body", METAL, METAL,
                        HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, HEAT, "body"],
            cap0=None, cap1="rubber")
    P.lathe(g, (0, 0, -3.4), NZ, [(0, 0.24), (0.05, 0.24)], n=12, mats="accent", cap0=None,
            cap1=None)
    pipe(g, (0.42, 0.08, -1.8), (0.42, 0.08, -2.6), 0.045, mat=METAL, n=6)
    rivets(g, (-0.45, 0.345, -1.5), (0.45, 0.345, -1.5), Y, 5, r=0.024)
    P.bevel_box(g, (-0.5, 0.2, -1.75), (-0.26, 0.42, -1.3), 0.03, mat="body", mats={"-z": GLASS})
    part("heavy_tank_barrel", g, smooth=40)
    L_MUZZLES.append(("heavy_tank_muzzle", (0, 0, -4.4)))

    # The gunner's cupola (pivot (0, 3.0, 1.2); the main turret's roof at local -0.2).
    g = L.Geo()
    P.lathe(g, (0, -0.2, 0), Y, [(0, 0.44), (0.06, 0.44), (0.06, 0.4), (0.13, 0.4),
                                 (0.13, 0.42), (0.17, 0.36)], n=12,
            mats=[METAL, METAL, GLASS, ARMOR, ARMOR], cap0=None, cap1=ARMOR)
    # (The hatch stands open behind the gunner; a shield either side of the gun.)
    P.plate(g, P.rect_on((0, 0.12, 0.4), (0, 0.25, 1), 0.52, 0.5), (0, 0.25, 1), 0.035, ARMOR,
            bevel=0.03)
    for sx in (-1, 1):
        poly = [(sx * 0.1, -0.03, -0.3), (sx * 0.46, -0.03, -0.24), (sx * 0.46, 0.3, -0.18),
                (sx * 0.1, 0.36, -0.24)]
        P.extrude(g, poly, (0, 0.012, 0.035), ARMOR)
        decal(g, [(sx * 0.2, 0.08, -0.288), (sx * 0.4, 0.08, -0.255), (sx * 0.4, 0.16, -0.24),
                  (sx * 0.2, 0.16, -0.273)], (0, -0.2, -1), h=0.004)
    strip(g, P.circle((0, -0.03, 0), Y, 0.3, 12), Y, 0.03, closed=True, glow=0.03)
    part("heavy_tank_mg_turret", g, smooth=35)
    # The chaingun (1.1 m): receiver, ammunition box and belt, a jacketed barrel.
    g = L.Geo()
    P.bevel_box(g, (-0.08, -0.07, -0.26), (0.08, 0.09, 0.26), 0.015, mat=METAL)
    P.bevel_box(g, (0.09, -0.14, -0.1), (0.27, 0.06, 0.16), 0.02, mat=PAINT)
    P.hose(g, [(0.18, 0.06, 0.02), (0.14, 0.13, 0.0), (0.04, 0.1, -0.02)], 0.03, n=4, mat=METAL,
           caps=False)
    P.lathe(g, (0, 0.01, -0.25), NZ, [(0, 0.06), (0.4, 0.06), (0.4, 0.035), (0.72, 0.032),
                                      (0.72, 0.05), (0.85, 0.055), (0.85, 0.03), (0.78, 0.03)],
            n=8, mats=["body", "body", METAL, METAL, HEAT, HEAT, "body"], cap0=None, cap1="rubber")
    for z in (-0.36, -0.46, -0.56):
        for n_ in (Y, NY):
            decal(g, P.rect_on((0, 0.01 + n_[1] * 0.06 * math.cos(math.pi / 8), z), n_, 0.04,
                               0.06, up=NZ), n_, RUBBER, h=0.003)
    for sx in (-1, 1):
        handle(g, (sx * 0.06, 0.0, 0.26), (sx * 0.06, 0.0, 0.3), (sx, 0, 0.3), h=0.07, r=0.012)
    strip(g, [(-0.08, 0.04, -0.2), (-0.08, 0.04, 0.2)], NX, 0.02, glow=0.02)
    part("heavy_tank_mg", g, smooth=40)
    L_MUZZLES.append(("heavy_tank_mg_muzzle", (0, 0, -1.1)))


# ---------------------------------------------------------------------------------------------
# Spawn bus.

def bsec(z, p):
    hw, yb, yt, tw = p
    return hsec(z, (hw, yb, yt - 0.35, yt, tw))


def spawn_bus():
    g = L.Geo()
    L.box(g, (-1.2, 0.45, -4.85), (1.2, 1.5, 4.85))
    stations = [(-5.0, (1.62, 1.45, 2.25, 1.25)), (-3.3, (1.7, 1.45, 3.45, 1.35)),
                (4.8, (1.7, 1.45, 3.45, 1.35)), (5.0, (1.62, 1.45, 3.25, 1.25))]
    rings = [bsec(z, p) for z, p in stations]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    # Windshield.
    q = quad(rings, 0, 3)
    n = P.quad_normal(q, (0, 1, -1))
    P.disc(g, [P.on_quad(*q, s, t) for s, t in ((0.1, 0.12), (0.9, 0.12), (0.9, 0.85),
                                                 (0.1, 0.85))], n, glow=0.06)
    for s in (-1, 1):
        x = s * 1.7
        # Side windows, lines.
        for z0, z1 in ((-3.0, -1.7), (-1.4, -0.1), (0.2, 1.5), (1.8, 3.1), (3.4, 4.5)):
            P.disc(g, [(x, 2.5, z0), (x, 2.5, z1), (x, 2.88, z1), (x, 2.88, z0)], (s, 0, 0),
                   glow=0.05)
        P.accent(g, [(x, 1.68, -3.1), (x, 1.68, 4.6)], (s, 0, 0), 0.06, glow=0.05)
        P.flow(g, [(x, 2.2, -3.1), (x, 2.2, 4.6)], (s, 0, 0), 0.03)
        # Armoured skirts between the wheels.
        x0, x1 = sorted((s * 1.3, s * 1.68))
        for z0, z1 in ((-4.9, -4.2), (-2.6, -0.8), (0.8, 2.6), (4.2, 4.9)):
            L.box(g, (x0, 0.75, z0), (x1, 1.45, z1))
            P.accent(g, [(s * 1.68, 1.1, z0 + 0.1), (s * 1.68, 1.1, z1 - 0.1)], (s, 0, 0),
                     0.05, glow=0.04)
        # Headlights, tail lights.
        hx0, hx1 = sorted((s * 0.85, s * 1.35))
        P.disc(g, [(hx0, 1.55, -5.0), (hx1, 1.55, -5.0), (hx1, 1.72, -5.0), (hx0, 1.72, -5.0)],
               NZ, mat=LIGHT, glow=0.05)
        P.disc(g, [(hx0, 1.6, 5.0), (hx1, 1.6, 5.0), (hx1, 1.85, 5.0), (hx0, 1.85, 5.0)],
               Z, mat=LIGHT, glow=0.05)
        # Roof flow lines.
        P.flow(g, [(s * 1.05, 3.45, -3.1), (s * 1.05, 3.45, 4.6)], Y, 0.035)
    # Slants: team lines.
    loft_line(g, rings, 2, 0.5, 1, 2, 0.06)
    loft_line(g, rings, 4, 0.5, 1, 2, 0.06)
    # Bumper with a chevron.
    L.box(g, (-1.5, 0.5, -5.0), (1.5, 0.85, -4.85))
    P.accent(g, [(-0.5, 0.58, -5.0), (0.0, 0.77, -5.0), (0.5, 0.58, -5.0)], NZ, 0.06,
             glow=0.04)
    # Rear door outline.
    P.accent(g, [(-0.7, 1.55, 5.0), (0.7, 1.55, 5.0), (0.7, 3.0, 5.0), (-0.7, 3.0, 5.0)], Z,
             0.06, closed=True, glow=0.06)
    # Roof: light bar at the front, mast housing.
    L.box(g, (-0.9, 3.45, -3.15), (0.9, 3.56, -2.9))
    P.white(g, [(-0.8, 3.505, -3.15), (0.8, 3.505, -3.15)], NZ, 0.05, glow=0.03)
    P.lathe(g, (0, 3.45, 1.5), Y, [(0, 0.6), (0.15, 0.52)], n=8, cap0=None)
    P.accent(g, P.circle((0, 3.45, 1.5), Y, 0.78, 8), Y, 0.05, closed=True, glow=0.05)
    part("spawn_bus_hull", g)
    # Wheel.
    g = L.Geo()
    P.lathe(g, (-0.22, 0, 0), X, [(0, 0.45), (0.05, 0.66), (0.1, 0.7), (0.34, 0.7),
                                  (0.39, 0.66), (0.44, 0.45)], n=14)
    for s in (-1, 1):
        P.lathe(g, (s * 0.22, 0, 0), (s, 0, 0), [(0, 0.28), (0.03, 0.2)], n=8, cap0=None)
        P.accent(g, P.circle((s * 0.22, 0, 0), (s, 0, 0), 0.37, 14), (s, 0, 0), 0.035,
                 closed=True, glow=0.03)
        for k in range(3):
            a = math.radians(120 * k + 30)
            u, v = math.cos(a), math.sin(a)
            P.accent(g, [(s * 0.22, 0.22 * v, 0.22 * u), (s * 0.22, 0.33 * v, 0.33 * u)],
                     (s, 0, 0), 0.03, glow=0.02)
    part("spawn_bus_wheel", g)
    # Mast (foot at the origin).
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.32), (0.25, 0.26)], n=8, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(0.25, 0.12), (1.5, 0.12), (1.5, 0.09), (2.6, 0.09)], n=8,
            cap0=None, cap1=None)
    P.lathe(g, (0, 0, 0), Y, [(1.2, 0.128), (1.27, 0.128)], n=8, mats="accent", cap0=None,
            cap1=None)
    P.lathe(g, (0, 2.55, 0), Y, [(0, 0.12), (0.2, 0.38), (0.3, 0.38), (0.45, 0.15)], n=8)
    P.lathe(g, (0, 2.55, 0), Y, [(0.215, 0.39), (0.285, 0.39)], n=8, mats=LIGHT, cap0=None,
            cap1=None)
    P.tube(g, (0, 3.0, 0), (0, 3.1, 0), 0.03, n=4, mat="accent")
    P.accent(g, P.circle((0, 3.0, 0), Y, 0.11, 8), Y, 0.025, closed=True, glow=0.03)
    for a in (0.0, math.pi):  # flow up two flats of the pole
        d = (math.cos(a), 0, math.sin(a))
        f1 = 0.12 * math.cos(math.pi / 8)
        f2 = 0.09 * math.cos(math.pi / 8)
        P.flow(g, [L.add(L.mul(d, f1), (0, 0.3, 0)), L.add(L.mul(d, f1), (0, 1.45, 0))], d,
               0.025)
        P.flow(g, [L.add(L.mul(d, f2), (0, 1.55, 0)), L.add(L.mul(d, f2), (0, 2.5, 0))], d,
               0.02)
    part("spawn_bus_mast", g)


# ---------------------------------------------------------------------------------------------
# Harvester.

def harvester():
    g = L.Geo()
    # Chassis between the wheels, skirts in the gaps, fender deck over the wheels.
    L.box(g, (-1.4, 0.5, -5.3), (1.4, 1.55, 5.3))
    for s in (-1, 1):
        x0, x1 = sorted((s * 1.4, s * 2.0))
        L.box(g, (x0, 1.85, -5.0), (x1, 2.0, 5.2))
        P.accent(g, [(s * 2.0, 1.925, -4.85), (s * 2.0, 1.925, 5.05)], (s, 0, 0), 0.06,
                 glow=0.0, h=0.012)
        for z0, z1 in ((-2.7, -1.1), (1.1, 2.7), (4.9, 5.4)):
            sx0, sx1 = sorted((s * 1.4, s * 1.95))
            L.box(g, (sx0, 0.75, z0), (sx1, 1.85, z1))
            P.flow(g, [(s * 1.95, 1.3, z0 + 0.15), (s * 1.95, 1.3, z1 - 0.15)], (s, 0, 0), 0.03)
    # Cab at the front.
    st = [(-5.5, (1.85, 1.5, 1.95, 2.25, 1.45)), (-4.4, (1.95, 1.5, 2.1, 2.6, 1.6)),
          (-2.6, (1.95, 1.5, 2.1, 2.6, 1.6))]
    rings = [hsec(z, p) for z, p in st]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    q = quad(rings, 0, 3)  # windshield
    n = P.quad_normal(q, (0, 1, -1))
    P.disc(g, [P.on_quad(*q, a, b) for a, b in ((0.08, 0.15), (0.92, 0.15), (0.92, 0.85),
                                                 (0.08, 0.85))], n, glow=0.06, h=0.012)
    loft_line(g, rings, 2, 0.5, 1, 2, 0.06)
    loft_line(g, rings, 4, 0.5, 1, 2, 0.06)
    loft_line(g, rings, 3, 0.5, 1, 2, 0.04, kind="flow")
    for s in (-1, 1):
        x = s * 1.95  # side windows
        P.disc(g, [(x, 1.6 + 0.0, -4.2), (x, 1.6, -3.0), (x, 2.0, -3.0), (x, 2.0, -4.2)],
               (s, 0, 0), glow=0.04, h=0.012)
    # Front bumper with chevron and headlights; front face lights.
    L.box(g, (-1.9, 0.6, -5.5), (1.9, 1.45, -5.3))
    P.accent(g, [(-0.6, 0.75, -5.5), (0.0, 1.2, -5.5), (0.6, 0.75, -5.5)], NZ, 0.07, glow=0.05)
    for s in (-1, 1):
        hx0, hx1 = sorted((s * 1.1, s * 1.7))
        P.disc(g, [(hx0, 1.05, -5.5), (hx1, 1.05, -5.5), (hx1, 1.3, -5.5), (hx0, 1.3, -5.5)], NZ,
               mat=LIGHT, glow=0.05)
    # Turret deck behind the cab (top at y = 2.75: the turret's base sits there).
    L.box(g, (-1.1, 1.5, -2.6), (1.1, 2.75, -1.2))
    P.accent(g, P.circle((0, 2.75, -2.0), Y, 0.95, 10), Y, 0.04, closed=True, glow=0.04)
    for s in (-1, 1):  # exhaust stacks at the cab's back corners
        P.tube(g, (s * 1.6, 2.0, -2.45), (s * 1.6, 2.95, -2.45), 0.12, n=6)
        P.tube(g, (s * 1.6, 2.7, -2.45), (s * 1.6, 2.78, -2.45), 0.13, n=6, mat="accent")
    # Cargo hopper over the back: tapered, wider at the top.
    foot = [(1.3, -1.2), (1.6, -0.9), (1.6, 5.0), (1.3, 5.3), (-1.3, 5.3), (-1.6, 5.0),
            (-1.6, -0.9), (-1.3, -1.2)]
    hop = footprint_loft(g, foot, [(1.55, 1.0, 1.0, 0.0), (2.3, 1.2, 1.0, 0.0),
                                   (3.6, 1.2, 1.0, 0.0)])
    top_outline(g, hop[2], 0.93, w=0.07)
    for dz in (0.2, 1.6, 3.0, 4.4):  # ribs across the lid, flow conveyor lines
        P.accent(g, [(-1.5, 3.6, dz), (1.5, 3.6, dz)], Y, 0.05, glow=0.0, h=0.012)
    for x in (-0.6, 0.6):
        P.flow(g, [(x, 3.6, -0.8), (x, 3.6, 4.9)], Y, 0.04)
    for s in (-1, 1):  # glowing cargo window in a white frame
        x = s * 1.92
        P.disc(g, [(x, 2.55, -0.3), (x, 2.55, 4.5), (x, 3.25, 4.5), (x, 3.25, -0.3)], (s, 0, 0),
               glow=0.08, h=0.012)
        P.white(g, [(x, 2.42, -0.45), (x, 2.42, 4.65)], (s, 0, 0), 0.04, glow=0.0, h=0.012)
    # Rear face: tail lights and a team outline.
    P.accent(g, [(-1.5, 2.45, 5.3), (1.5, 2.45, 5.3), (1.5, 3.4, 5.3), (-1.5, 3.4, 5.3)], Z,
             0.05, closed=True, glow=0.05)
    for s in (-1, 1):
        tx0, tx1 = sorted((s * 0.7, s * 1.2))
        P.disc(g, [(tx0, 1.0, 5.3), (tx1, 1.0, 5.3), (tx1, 1.25, 5.3), (tx0, 1.25, 5.3)], Z,
               mat=LIGHT, glow=0.04)
    part("harvester_hull", g)
    # Wheel (axle centre at the origin, axle along X).
    g = L.Geo()
    rt = 0.9 / math.cos(math.pi / 16)  # vertex radius: the bottom flat touches y = -0.9
    P.lathe(g, (-0.28, 0, 0), X, [(0, 0.6), (0.05, rt - 0.05), (0.1, rt), (0.46, rt),
                                  (0.51, rt - 0.05), (0.56, 0.6)], n=16)
    for s in (-1, 1):
        P.lathe(g, (s * 0.28, 0, 0), (s, 0, 0), [(0, 0.36), (0.02, 0.26)], n=8, cap0=None)
        P.accent(g, P.circle((s * 0.28, 0, 0), (s, 0, 0), 0.47, 16), (s, 0, 0), 0.04,
                 closed=True, glow=0.03)
        for k in range(4):
            a = math.radians(90 * k + 45)
            u, v = math.cos(a), math.sin(a)
            P.accent(g, [(s * 0.28, 0.28 * v, 0.28 * u), (s * 0.28, 0.42 * v, 0.42 * u)],
                     (s, 0, 0), 0.04, glow=0.02)
    part("harvester_wheel", g)
    # Emitter mount (pivot frame; deck top at local y = -0.45).
    g = L.Geo()
    P.lathe(g, (0, -0.45, 0), Y, [(0, 0.85), (0.12, 0.85), (0.2, 0.7)], n=10)
    P.accent(g, P.circle((0, -0.25, 0), Y, 0.5, 10), Y, 0.04, closed=True, glow=0.04)
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.42, s * 0.58))
        L.prism_x(g, [(-0.35, -0.25), (0.45, -0.25), (0.45, 0.05), (0.15, 0.32), (-0.2, 0.32),
                      (-0.35, 0.1)], x0, x1)
        xo = s * 0.58
        P.accent(g, [(xo, -0.1, -0.25), (xo, -0.1, 0.35)], (s, 0, 0), 0.04, glow=0.03)
        P.flow(g, [(xo, 0.05, -0.15), (xo, 0.2, 0.05)], (s, 0, 0), 0.02)
    L.box(g, (-0.38, -0.25, 0.4), (0.38, 0.15, 0.8))
    P.white(g, [(-0.28, -0.05, 0.8), (0.28, -0.05, 0.8)], Z, 0.04, glow=0.03)
    part("harvester_turret", g)
    # Emitter arm (pivot frame, along -Z, tip at z = -2.0).
    g = L.Geo()
    P.tube(g, (-0.42, 0, 0), (0.42, 0, 0), 0.15, n=8)
    o = (0, 0, 0.35)
    P.lathe(g, o, NZ, [(0, 0.17), (0.3, 0.2), (1.3, 0.15), (1.5, 0.15)], n=8)
    for t in (0.55, 0.85, 1.15):
        r = 0.2 - 0.05 * (t - 0.3) + 0.012  # just proud of the tapering arm
        P.lathe(g, o, NZ, [(t, r), (t + 0.08, r)], n=8, mats="accent", cap0=None, cap1=None)
    # Dish: a cone opening forward with a white lens inside.
    d0, d1, rd = -1.15, -1.62, 0.5
    P.lathe(g, (0, 0, d0), NZ, [(0, 0.16), (d0 - d1 - 0.05, rd - 0.02), (d0 - d1, rd)], n=12,
            cap0=None, cap1=None)
    fr = P.circle((0, 0, d1), NZ, rd - 0.05, 12)
    bk = P.circle((0, 0, d0 - 0.12), NZ, 0.12, 12)
    for i in range(12):
        j = (i + 1) % 12
        q = [bk[i], bk[j], fr[j], fr[i]]
        mid = quad_mid(q)
        g.face(q, LIGHT, want=L.add((-mid[0], -mid[1], 0), (0, 0, -0.1)))
    g.face(bk, LIGHT, want=NZ)
    rim_o = P.circle((0, 0, d1), NZ, rd, 12)
    for i in range(12):
        j = (i + 1) % 12
        g.face([fr[i], fr[j], rim_o[j], rim_o[i]], "accent", want=NZ)
    # Emitter spike and three prongs.
    P.lathe(g, (0, 0, d0 - 0.12), NZ, [(0, 0.09), (0.5, 0.07), (0.6, 0.05), (0.73, 0.035)],
            n=6, cap1=LIGHT)
    P.lathe(g, (0, 0, d0 - 0.12), NZ, [(0.3, 0.083), (0.36, 0.08)], n=6, mats="accent",
            cap0=None, cap1=None)
    for k in range(3):
        a = math.radians(90 + 120 * k)
        u, v = math.cos(a), math.sin(a)
        P.tube(g, (0.44 * u, 0.44 * v, d1 + 0.02), (0.13 * u, 0.13 * v, -1.93), 0.035, n=4)
        P.tube(g, (0.13 * u, 0.13 * v, -1.93), (0.11 * u, 0.11 * v, -1.97), 0.04, n=4,
               mat="accent")
    part("harvester_barrel", g)
    L_MUZZLES.append(("harvester_muzzle", (0, 0, -2.0)))


# ---------------------------------------------------------------------------------------------


# ---------------------------------------------------------------------------------------------
# Rover.

def rover():
    g = L.Geo()
    # Chassis tub and nose.
    stations = [(-2.25, (0.7, 0.35, 0.6, 0.85, 0.55)), (-1.6, (0.85, 0.3, 0.75, 1.0, 0.75)),
                (1.9, (0.85, 0.3, 0.75, 1.0, 0.75)), (2.25, (0.75, 0.35, 0.7, 0.95, 0.65))]
    rings = [hsec(z, p) for z, p in stations]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    loft_line(g, rings, 2, 0.5, 1, 2, 0.05)
    loft_line(g, rings, 4, 0.5, 1, 2, 0.05)
    loft_line(g, rings, 1, 0.5, 1, 2, 0.03, kind="flow")
    loft_line(g, rings, 5, 0.5, 1, 2, 0.03, kind="flow")
    q = quad(rings, 0, 3)
    n = P.quad_normal(q, (0, 1, -1))
    P.accent(g, [P.on_quad(*q, 0.2, 0.75), P.on_quad(*q, 0.5, 0.35), P.on_quad(*q, 0.8, 0.75)],
             n, 0.05, glow=0.05)
    for s in (-1, 1):
        # Headlights on the nose, tail lights at the back.
        hx0, hx1 = sorted((s * 0.25, s * 0.55))
        P.disc(g, [(hx0, 0.5, -2.25), (hx1, 0.5, -2.25), (hx1, 0.62, -2.25), (hx0, 0.62, -2.25)],
               NZ, mat=LIGHT, glow=0.04)
        P.disc(g, [(hx0, 0.5, 2.25), (hx1, 0.5, 2.25), (hx1, 0.6, 2.25), (hx0, 0.6, 2.25)], Z,
               mat=LIGHT, glow=0.04)
        # Suspension arms to the wheel hubs, mudguards over the wheels.
        for z in (-1.45, 1.45):
            P.tube(g, (s * 0.8, 0.5, z), (s * 0.95, 0.5, z), 0.07, n=6)
            x0, x1 = sorted((s * 0.88, s * 1.3))
            L.box(g, (x0, 1.05, z - 0.6), (x1, 1.12, z + 0.6))
            P.accent(g, [(s * 1.3, 1.085, z - 0.5), (s * 1.3, 1.085, z + 0.5)], (s, 0, 0), 0.04,
                     glow=0.03)
        # Seats.
        L.box(g, (s * 0.15 - 0.25 if s > 0 else -0.65, 1.0, -0.7),
              (s * 0.15 + 0.25 if s < 0 else 0.65, 1.15, 0.0))
        # Roll cage.
        P.tube(g, (s * 0.7, 1.0, -1.0), (s * 0.6, 1.85, -0.7), 0.05, n=6)
        P.tube(g, (s * 0.7, 1.0, 0.4), (s * 0.6, 1.85, 0.2), 0.05, n=6)
        P.tube(g, (s * 0.6, 1.85, -0.7), (s * 0.6, 1.85, 0.2), 0.05, n=6)
        P.flow(g, [(s * 0.6, 1.9, -0.65), (s * 0.6, 1.9, 0.15)], Y, 0.02)
    P.tube(g, (-0.6, 1.85, -0.7), (0.6, 1.85, -0.7), 0.05, n=6)
    P.tube(g, (-0.6, 1.85, 0.2), (0.6, 1.85, 0.2), 0.05, n=6, mat="accent")
    # Windscreen frame.
    P.disc(g, [(-0.6, 1.05, -1.15), (0.6, 1.05, -1.15), (0.55, 1.5, -1.0), (-0.55, 1.5, -1.0)],
           (0, 0.3, -1), glow=0.04)
    # Gun post at the back (the turret sits on top at y = 1.7).
    P.lathe(g, (0, 1.0, 1.2), Y, [(0, 0.22), (0.7, 0.14)], n=8, cap0=None)
    P.accent(g, P.circle((0, 1.45, 1.2), Y, 0.17, 8), Y, 0.03, closed=True, glow=0.03)
    part("rover_hull", g)
    # Wheel.
    g = L.Geo()
    P.lathe(g, (-0.18, 0, 0), X, [(0, 0.3), (0.04, 0.47), (0.08, 0.5), (0.28, 0.5),
                                  (0.32, 0.47), (0.36, 0.3)], n=12)
    for s in (-1, 1):
        P.accent(g, P.circle((s * 0.18, 0, 0), (s, 0, 0), 0.26, 12), (s, 0, 0), 0.03,
                 closed=True, glow=0.02)
    part("rover_wheel", g)
    # Gun mount (pivot frame; the post's top at local y = -0.15).
    g = L.Geo()
    P.lathe(g, (0, -0.15, 0), Y, [(0, 0.3), (0.12, 0.28)], n=8)
    P.extrude(g, [(-0.32, -0.05, 0.3), (0.32, -0.05, 0.3), (0.32, -0.05, -0.2),
                  (-0.32, -0.05, -0.2)], (0, 0.5, 0))
    P.accent(g, [(-0.3, 0.2, -0.2), (0.3, 0.2, -0.2)], NZ, 0.04, glow=0.03)
    part("rover_turret", g)
    g = L.Geo()
    L.box(g, (-0.14, -0.12, -0.35), (0.14, 0.12, 0.25))
    gun(g, (0, 0, -0.35), [(0, 0.07), (0.75, 0.06)], n=6, rings=((0.2, 0.075),))
    part("rover_barrel", g)
    L_MUZZLES.append(("rover_muzzle", (0, 0, -1.1)))


# ---------------------------------------------------------------------------------------------
# VTOL gunship.

def vsec(z, p):
    hw, yb, yt = p
    ym = yb + 0.45 * (yt - yb)
    return [(-0.55 * hw, yb, z), (0.55 * hw, yb, z), (hw, ym, z), (0.6 * hw, yt, z),
            (-0.6 * hw, yt, z), (-hw, ym, z)]


def annulus(g, c, r0, r1, n, mat, want):
    a = P.circle(c, Y, r0, n)
    b = P.circle(c, Y, r1, n)
    for i in range(n):
        j = (i + 1) % n
        g.face([a[i], a[j], b[j], b[i]], mat, want=want)


def vtol():
    g = L.Geo()
    st = [(-4.2, (0.4, 1.15, 1.5)), (-3.0, (1.15, 0.8, 1.95)), (-1.0, (1.6, 0.6, 2.1)),
          (1.5, (1.5, 0.65, 2.0)), (3.4, (0.7, 1.05, 1.8)), (4.2, (0.45, 1.3, 1.7))]
    rings = [vsec(z, p) for z, p in st]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    # Canopy: glowing windscreen, armoured rear.
    def csec(z, p):
        w, yb, yt = p
        ym = (yb + yt) / 2
        return [(-w, yb, z), (w, yb, z), (w, ym, z), (w * 0.55, yt, z), (-w * 0.55, yt, z),
                (-w, ym, z)]
    crings = [csec(z, p) for z, p in ((-3.6, (0.3, 1.65, 1.75)), (-2.4, (0.6, 1.85, 2.35)),
                                      (-1.4, (0.55, 1.95, 2.3)))]
    L.loft(g, crings, Z, mats=[LIGHT, "body"], cap0=LIGHT, cap1="body")
    centre = (0, 1.4, 0)
    # Chines, spine and nose lights.
    loft_line(g, rings, 2, 0.15, 1, 4, 0.07, centre=None)
    loft_line(g, rings, 4, 0.85, 1, 4, 0.07, centre=None)
    loft_line(g, rings, 1, 0.5, 1, 4, 0.03, kind="flow")
    loft_line(g, rings, 5, 0.5, 1, 4, 0.03, kind="flow")
    loft_line(g, rings, 3, 0.5, 3, 5, 0.04, kind="flow")
    loft_line(g, rings, 2, 0.6, 0, 1, 0.05, kind="white")
    loft_line(g, rings, 4, 0.4, 0, 1, 0.05, kind="white")
    q = quad(rings, 2, 3)
    n = P.quad_normal(q, Y)
    for t in (0.35, 0.65):
        P.accent(g, [P.on_quad(*q, 0.15, t + 0.3), P.on_quad(*q, 0.5, t),
                     P.on_quad(*q, 0.85, t + 0.3)], n, 0.07, glow=0.06, h=0.012)
    q = quad(rings, 2, 0)
    n = P.quad_normal(q, NY)
    for s in (0.25, 0.75):
        P.disc(g, P.circle(P.on_quad(*q, s, 0.5), n, 0.08, 6), n, mat=LIGHT, glow=0.05,
               h=0.012)
    for s in (-1, 1):
        # Stub wing, shaft to the engine pivot.
        wy = 1.03
        P.extrude(g, [(s * 1.1, wy, -0.95), (s * 1.6, wy, -0.7), (s * 1.6, wy, 0.75),
                      (s * 1.1, wy, 0.95)], (0, 0.16, 0))
        P.accent(g, [(s * 1.25, wy + 0.16, -0.82), (s * 1.55, wy + 0.16, -0.65)], Y, 0.04,
                 glow=0.04)
        P.tube(g, (s * 1.5, 1.1, 0.2), (s * 2.25, 1.1, 0.2), 0.1, n=8)
        # Rocket pod on a forward strut.
        P.extrude(g, [(s * 1.1, 0.95, -1.75), (s * 2.6, 0.95, -1.7), (s * 2.6, 0.95, -1.25),
                      (s * 1.1, 0.95, -1.0)], (0, 0.12, 0))
        o = (s * 2.6, 0.8, -1.15)
        P.lathe(g, o, NZ, [(0, 0.18), (0.15, 0.27), (1.75, 0.27), (1.85, 0.25)], n=8)
        P.lathe(g, o, NZ, [(1.45, 0.278), (1.53, 0.278)], n=8, mats="accent", cap0=None,
                cap1=None)
        P.flow(g, [(s * 2.6, 0.8 + 0.27 * math.cos(math.pi / 8), -1.35),
                   (s * 2.6, 0.8 + 0.27 * math.cos(math.pi / 8), -2.55)], Y, 0.025)
        for dx in (-0.09, 0.09):
            for dy in (-0.09, 0.09):
                c = (s * 2.6 + dx, 0.8 + dy, -3.0)
                P.disc(g, P.circle(c, NZ, 0.05, 6), NZ, mat=LIGHT, glow=0.0)
        # Canted tail fin.
        out = L.norm((s * 0.4, 1.0, 0))
        o = (s * 0.5, 1.8, 2.3)
        pts = [L.add(o, L.add(L.mul(Z, a), L.mul(out, b)))
               for a, b in ((0, 0), (1.8, 0), (1.9, 0.95), (1.2, 0.95))]
        nrm = L.cross(out, Z)
        nrm = L.mul(nrm, 1 if nrm[0] * s > 0 else -1)
        P.extrude(g, pts, L.mul(nrm, 0.06))
        top = [L.add(p, L.mul(nrm, 0.06)) for p in pts]
        P.accent(g, [P.lerp(top[0], top[3], 0.6), P.lerp(top[1], top[2], 0.6)], nrm, 0.05,
                 glow=0.04)
        # Landing skid and struts.
        x = s * 1.1
        P.tube(g, (x, 0.06, 2.0), (x, 0.06, -2.4), 0.06, n=6)
        P.tube(g, (x, 0.06, -2.4), (x, 0.3, -2.75), 0.06, n=6)
        for z in (-1.6, 1.2):
            P.tube(g, (x, 0.08, z), (s * 0.8, 0.8, z), 0.05, n=6)
    part("vtol_body", g)
    # Engine pod (pivot frame, hover pose).
    g = L.Geo()
    P.lathe(g, (0, -0.75, 0), Y, [(0, 0.22), (0.15, 0.36), (0.9, 0.42), (1.15, 0.4),
                                  (1.25, 0.3)], n=8, cap0=LIGHT)
    P.lathe(g, (0, -0.75, 0), Y, [(0.68, 0.42), (0.74, 0.42)], n=8, mats="accent", cap0=None,
            cap1=None)
    y0, y1, ri, ro, n = 0.2, 0.5, 0.72, 0.85, 16
    P.lathe(g, (0, y0, 0), Y, [(0, ro), (y1 - y0, ro)], n=n, cap0=None, cap1=None)
    a, b = P.circle((0, y0, 0), Y, ri, n), P.circle((0, y1, 0), Y, ri, n)
    for i in range(n):
        j = (i + 1) % n
        q = [a[i], a[j], b[j], b[i]]
        mid = quad_mid(q)
        g.face(q, "body", want=(-mid[0], 0, -mid[2]))
    annulus(g, (0, y1, 0), ri, ro, n, "body", Y)
    annulus(g, (0, y0, 0), ri, ro, n, "body", NY)
    P.lathe(g, (0, y0, 0), Y, [(0.1, ro + 0.008), (0.17, ro + 0.008)], n=n, mats="accent",
            cap0=None, cap1=None)
    rm = (ri + ro) / 2 / math.cos(math.pi / n)
    P.accent(g, P.circle((0, y1, 0), Y, rm, n), Y, 0.03, closed=True, glow=0.04)
    P.white(g, P.circle((0, y0, 0), Y, rm, n), NY, 0.04, closed=True, glow=0.04)
    for k in range(4):  # spokes
        ang = math.pi / 4 + k * math.pi / 2
        d = (math.cos(ang), 0, math.sin(ang))
        t = (-d[2], 0, d[0])
        p0, p1 = L.mul(d, 0.38), L.mul(d, 0.76)
        P.extrude(g, [L.add(p0, L.add(L.mul(t, -0.03), (0, 0.28, 0))),
                      L.add(p1, L.add(L.mul(t, -0.03), (0, 0.28, 0))),
                      L.add(p1, L.add(L.mul(t, 0.03), (0, 0.28, 0))),
                      L.add(p0, L.add(L.mul(t, 0.03), (0, 0.28, 0)))], (0, 0.06, 0))
    for k in range(8):  # fan blades (thin pitched slabs)
        ang = 2 * math.pi * k / 8
        d = (math.cos(ang), 0, math.sin(ang))
        t = (-d[2], 0, d[0])
        pitch = L.add(L.mul(t, 0.07), (0, 0.03, 0))
        c0, c1 = L.add(L.mul(d, 0.4), (0, 0.38, 0)), L.add(L.mul(d, 0.7), (0, 0.38, 0))
        P.extrude(g, [L.sub(c0, pitch), L.sub(c1, pitch), L.add(c1, pitch), L.add(c0, pitch)],
                  (0, 0.012, 0))
    part("vtol_engine", g)


# ---------------------------------------------------------------------------------------------
# Interceptor.

def interceptor():
    g = L.Geo()
    st = [(-4.0, (0.15, 0.85, 1.0)), (-3.0, (0.55, 0.65, 1.35)), (-1.5, (0.8, 0.55, 1.6)),
          (1.5, (0.85, 0.55, 1.6)), (3.3, (0.75, 0.6, 1.5)), (3.9, (0.6, 0.65, 1.4))]
    rings = [vsec(z, p) for z, p in st]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    def csec(z, p):
        w, yb, yt = p
        ym = (yb + yt) / 2
        return [(-w, yb, z), (w, yb, z), (w, ym, z), (w * 0.5, yt, z), (-w * 0.5, yt, z),
                (-w, ym, z)]
    crings = [csec(z, p) for z, p in ((-2.8, (0.25, 1.3, 1.4)), (-1.8, (0.45, 1.5, 2.0)),
                                      (-0.6, (0.4, 1.55, 1.85)), (0.4, (0.25, 1.55, 1.65)))]
    L.loft(g, crings, Z, mats=[LIGHT, LIGHT, "body"], cap0=LIGHT, cap1="body")
    loft_line(g, rings, 2, 0.3, 1, 4, 0.05)
    loft_line(g, rings, 4, 0.7, 1, 4, 0.05)
    loft_line(g, rings, 3, 0.5, 2, 5, 0.03, kind="flow")
    loft_line(g, rings, 1, 0.5, 1, 4, 0.03, kind="flow")
    loft_line(g, rings, 5, 0.5, 1, 4, 0.03, kind="flow")
    for s in (-1, 1):
        # Swept wing.
        wy = 0.85
        P.extrude(g, [(s * 0.75, wy, -1.6), (s * 3.0, wy, 1.0), (s * 3.0, wy, 1.8),
                      (s * 0.75, wy, 2.6)], (0, 0.12, 0))
        P.accent(g, [(s * 0.9, wy + 0.12, -1.35), (s * 2.9, wy + 0.12, 1.05)], Y, 0.05,
                 glow=0.05, h=0.012)
        P.white(g, [(s * 2.95, wy + 0.06, 1.1), (s * 2.95, wy + 0.06, 1.7)], (s, 0, 0), 0.05,
                glow=0.03)
        # Missile rail under the wing, a missile on it.
        P.extrude(g, [(s * 2.1, 0.75, -1.4), (s * 2.3, 0.75, -1.4), (s * 2.3, 0.75, 0.6),
                      (s * 2.1, 0.75, 0.6)], (0, 0.1, 0))
        o = (s * 2.2, 0.6, 0.5)
        P.lathe(g, o, NZ, [(0, 0.1), (1.6, 0.1), (2.0, 0.0)], n=6, cap0="body")
        P.lathe(g, o, NZ, [(1.1, 0.105), (1.2, 0.105)], n=6, mats="accent", cap0=None, cap1=None)
        # Nose cannon.
        P.tube(g, (s * 0.4, 0.9, -2.5), (s * 0.4, 0.9, -4.1), 0.06, n=6)
        P.tube(g, (s * 0.4, 0.9, -4.0), (s * 0.4, 0.9, -4.1), 0.07, n=6, mat="accent")
        # Canted tail fin.
        out = L.norm((s * 0.45, 1.0, 0))
        o = (s * 0.45, 1.5, 2.3)
        pts = [L.add(o, L.add(L.mul(Z, a), L.mul(out, b)))
               for a, b in ((0, 0), (1.6, 0), (1.6, 0.75), (1.0, 0.75))]
        nrm = L.cross(out, Z)
        nrm = L.mul(nrm, 1 if nrm[0] * s > 0 else -1)
        P.extrude(g, pts, L.mul(nrm, 0.05))
        top = [L.add(p, L.mul(nrm, 0.05)) for p in pts]
        P.accent(g, [P.lerp(top[0], top[3], 0.6), P.lerp(top[1], top[2], 0.6)], nrm, 0.04,
                 glow=0.04)
        # Engine nozzle, glowing.
        P.lathe(g, (s * 0.4, 1.05, 3.85), Z, [(0, 0.26), (0.2, 0.22)], n=8, cap0=None,
                cap1="body")
        P.lathe(g, (s * 0.4, 1.05, 4.05), Z, [(0, 0.16), (0.01, 0.16)], n=8, cap0=None,
                cap1=LIGHT)
        # Landing strut.
        P.tube(g, (s * 0.6, 0.02, 0.8), (s * 0.5, 0.6, 0.8), 0.05, n=6)
    P.tube(g, (0, 0.02, -2.6), (0, 0.6, -2.6), 0.05, n=6)
    part("interceptor_hull", g)


# ---------------------------------------------------------------------------------------------
# Dropship.

def dropship():
    g = L.Geo()
    st = [(-7.0, (1.0, 1.4, 3.0)), (-5.8, (1.7, 0.7, 3.5)), (4.5, (1.8, 0.6, 3.6)),
          (7.0, (1.4, 1.2, 3.4))]
    rings = [vsec(z, p) for z, p in st]
    L.loft(g, rings, Z, cap0="body", cap1="body")
    loft_line(g, rings, 2, 0.4, 1, 2, 0.08)
    loft_line(g, rings, 4, 0.6, 1, 2, 0.08)
    loft_line(g, rings, 1, 0.5, 1, 2, 0.04, kind="flow")
    loft_line(g, rings, 5, 0.5, 1, 2, 0.04, kind="flow")
    loft_line(g, rings, 3, 0.5, 1, 3, 0.04, kind="flow")
    # Glass nose.
    q = quad(rings, 0, 3)
    n = P.quad_normal(q, (0, 1, -1))
    P.disc(g, [P.on_quad(*q, s, t) for s, t in ((0.1, 0.1), (0.9, 0.1), (0.9, 0.9),
                                                 (0.1, 0.9))], n, mat=LIGHT, glow=0.05)
    # Rear ramp outline.
    P.accent(g, [(-1.0, 0.75, 7.0), (1.0, 0.75, 7.0), (0.9, 2.9, 7.0), (-0.9, 2.9, 7.0)], Z,
             0.07, closed=True, glow=0.06)
    for s in (-1, 1):
        # Stub wing and an engine nacelle along it.
        P.extrude(g, [(s * 1.6, 3.0, -1.8), (s * 3.2, 3.0, -1.4), (s * 3.2, 3.0, 1.6),
                      (s * 1.6, 3.0, 2.2)], (0, 0.25, 0))
        o = (s * 3.6, 3.15, -2.6)
        P.lathe(g, o, Z, [(0, 0.4), (0.4, 0.62), (4.4, 0.62), (5.0, 0.48)], n=10, cap0="body",
                cap1=LIGHT)
        P.lathe(g, o, Z, [(0.6, 0.63), (0.75, 0.63)], n=10, mats="accent", cap0=None, cap1=None)
        P.lathe(g, o, Z, [(4.0, 0.63), (4.15, 0.63)], n=10, mats="accent", cap0=None, cap1=None)
        # Gun sponson (the ball mount sits at (s 3.6, 1.9, 0.5)).
        x0, x1 = sorted((s * 1.7, s * 3.2))
        L.box(g, (x0, 1.5, -0.3), (x1, 1.6, 1.3))
        P.accent(g, [(s * 3.2, 1.55, -0.2), (s * 3.2, 1.55, 1.2)], (s, 0, 0), 0.05, glow=0.04)
        # Side door outline and windows.
        P.accent(g, [(s * 1.8, 0.9, -2.8), (s * 1.8, 0.9, -1.2), (s * 1.8, 2.8, -1.2),
                     (s * 1.8, 2.8, -2.8)], (s, 0, 0), 0.06, closed=True, glow=0.05)
        for z0 in (1.6, 3.0):
            P.disc(g, [(s * 1.8, 2.3, z0), (s * 1.8, 2.3, z0 + 0.9), (s * 1.8, 2.75, z0 + 0.9),
                       (s * 1.8, 2.75, z0)], (s, 0, 0), glow=0.04)
        # Landing struts and pads.
        for z in (-4.5, 4.0):
            P.tube(g, (s * 1.3, 0.05, z), (s * 1.1, 0.7, z), 0.09, n=6)
            P.lathe(g, (s * 1.3, 0.0, z), Y, [(0, 0.3), (0.06, 0.3)], n=6)
    # Tail fin.
    P.extrude(g, [(0, 3.5, 4.4), (0, 3.5, 6.8), (0, 3.95, 6.9), (0, 3.95, 5.6)], (0.1, 0, 0))
    P.accent(g, [(0.11, 3.75, 5.0), (0.11, 3.75, 6.6)], X, 0.05, glow=0.04)
    part("dropship_hull", g)
    # Ball mount (pivot frame).
    g = L.Geo()
    P.lathe(g, (0, -0.4, 0), Y, [(0, 0.15), (0.15, 0.36), (0.45, 0.4), (0.7, 0.3), (0.8, 0.1)],
            n=8)
    P.accent(g, P.circle((0, 0.05, 0), Y, 0.41, 8), Y, 0.04, closed=True, glow=0.04)
    part("dropship_turret", g)
    g = L.Geo()
    L.box(g, (-0.13, -0.11, -0.4), (0.13, 0.11, 0.2))
    gun(g, (0, 0, -0.4), [(0, 0.07), (0.9, 0.055)], n=6, rings=((0.25, 0.075),))
    part("dropship_barrel", g)
    L_MUZZLES.append(("dropship_muzzle", (0, 0, -1.3)))


def main():
    L.reset_scene()
    P.materials()
    light_tank()
    light_aaa()
    heavy_tank()
    spawn_bus()
    vtol()
    harvester()
    rover()
    interceptor()
    dropship()
    for name, g, smooth in PARTS:
        if g.tris() > LIMIT:
            print("vehicle %s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
            raise SystemExit(1)
        L.make_object(name, g, smooth=smooth)
        print("vehicle %-22s %5d tris" % (name, g.tris()))
    for name, pos in L_MUZZLES + [("vtol_muzzle_l", (-2.6, 0.8, -3.0)),
                                  ("vtol_muzzle_r", (2.6, 0.8, -3.0)),
                                  ("interceptor_muzzle_l", (-0.4, 0.9, -4.1)),
                                  ("interceptor_muzzle_r", (0.4, 0.9, -4.1))]:
        L.make_empty(name, pos)
        print("vehicle node %s at %s" % (name, pos))
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/vehicles.blend")
    L.export_glb("assets/models/vehicles.glb")


main()
