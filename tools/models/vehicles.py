"""Builds the planetstrike vehicles: assets-src/blend/vehicles.blend and assets/models/vehicles.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/vehicles.py

Hull frame (every `*_hull`, `spawn_bus_hull`, `vtol_body`): origin at the bottom middle (ground
contact), front -Z, up +Y; each stays inside its hit box x in [-hx, hx], y in [0, 2 hy],
z in [-hz, hz] (turrets and barrels may poke out a little, as they rotate).

Moving parts are separate meshes authored in their own pivot frame (origin = pivot):
- `<v>_turret`: yawed by the game about +Y at the pivot; `<v>_barrel`: same pivot, pitched
  about +X; barrels point along -Z. Pivots in the hull frame:
  light_tank (0, 1.85, 0.3), light_aaa (0, 1.9, 0.4), heavy_tank (0, 2.2, 0.5) and the heavy
  tank's `heavy_tank_mg_turret` / `heavy_tank_mg` at (0, 3.0, 1.2) on the main turret's rear.
  Empties `<v>_muzzle` (`light_tank_muzzle`, `light_aaa_muzzle`, `heavy_tank_muzzle`,
  `heavy_tank_mg_muzzle`) mark the barrel tips IN THE BARREL'S PIVOT FRAME.
- `spawn_bus_wheel`: one wheel, origin at its axle centre, axle along X (radius 0.7, 0.5 wide with hubs);
  the game places 6 at (+-1.55, 0.7, -3.4 / 0 / 3.4) and spins them about X.
- `spawn_bus_mast`: deployable emitter mast, origin at its foot, which stands on the roof
  housing at (0, 3.6, 1.5) in the hull frame; 3.1 m tall.
- `vtol_engine`: one engine pod, origin at its tilt pivot, authored in HOVER pose (fan duct
  horizontal, exhaust down); the game places two at (+-2.6, 1.1, 0.2) and tilts them about X.
  The stub wings end at |x| = 1.6 and a thin shaft (r 0.1) carries on to the pod, so the duct
  (radius 0.85) can turn without touching the body. Empties `vtol_muzzle_l` (-2.6, 0.8, -3.0)
  and `vtol_muzzle_r` (2.6, 0.8, -3.0) are the rocket pods' tips in the body frame.
- Harvester (hit box half extents (2.0, 1.8, 5.5)): `harvester_hull` (cab at the front, turret
  deck behind it, a tapered cargo hopper over the back half with a team-lit cargo window on
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

Materials: `body`, `accent` (team neon) with `glow` halos, `glow_white`/`halo_white` (white
lights), `flow` (pulsing lines; on the track tops they read as moving treads).
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, X, Y, Z, NX, NY, NZ  # noqa: E402

PARTS = []  # (name, geo, smooth)


def part(name, g, smooth=None):
    """A mesh of the file; `smooth` shades it smoothly across edges bending less than that
    many degrees (round parts: wheels, barrels, canopies), flat otherwise."""
    PARTS.append((name, g, smooth))


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

LIGHT_STATIONS = [(-2.8, (1.45, 0.92, 1.05, 1.12, 1.1)), (-1.5, (1.6, 0.92, 1.25, 1.5, 1.25)),
                  (2.5, (1.6, 0.92, 1.25, 1.5, 1.25)), (2.8, (1.5, 0.92, 1.2, 1.35, 1.2))]


def light_hull(ring_z):
    g = L.Geo()
    for s in (-1, 1):
        track(g, s * 1.02, s * 1.55, 2.8, 0.95, 0.55, wheels=(-1.7, -0.6, 0.5, 1.6))
    L.box(g, (-1.02, 0.32, -2.5), (1.02, 0.96, 2.55))
    chassis(g, LIGHT_STATIONS, 1)
    turret_ring(g, ring_z, 1.5, 0.85)
    return g


def light_tank():
    part("light_tank_hull", light_hull(0.3))
    # Turret (pivot frame; hull deck at local y = -0.25).
    g = L.Geo()
    foot = [(0.45, -1.25), (0.95, -0.75), (1.0, 0.65), (0.8, 1.2), (-0.8, 1.2), (-1.0, 0.65),
            (-0.95, -0.75), (-0.45, -1.25)]
    rings = footprint_loft(g, foot, [(-0.25, 1, 1, 0), (0.12, 1, 1, 0), (0.38, 0.8, 0.82, 0.05)])
    belt_lines(g, rings, (0, 1, 5, 6))
    top_outline(g, rings[2], 0.85)
    hatch(g, (0.35, 0.38, 0.45), 0.28)
    for s in (-1, 1):  # rear lights
        P.disc(g, [(s * 0.25, -0.1, 1.2), (s * 0.55, -0.1, 1.2), (s * 0.55, 0.0, 1.2),
                   (s * 0.25, 0.0, 1.2)], Z, mat=LIGHT, glow=0.04)
    P.tube(g, (-0.6, 0.3, 0.95), (-0.6, 0.8, 0.95), 0.02, n=4)
    P.tube(g, (-0.6, 0.8, 0.95), (-0.6, 0.88, 0.95), 0.03, n=4, mat="accent")
    P.lathe(g, (0.68, 0.05, -0.75), NZ, [(0, 0.09), (0.25, 0.09)], n=6, cap1=LIGHT)
    part("light_tank_turret", g)
    # Barrel.
    g = L.Geo()
    mantlet(g, [(-0.75, (0.36, 0.24)), (-1.15, (0.32, 0.21)), (-1.3, (0.2, 0.15))])
    gun(g, (0, 0, -1.25), [(0, 0.12), (1.15, 0.11), (1.15, 0.14), (1.45, 0.14), (1.45, 0.11),
                           (1.6, 0.11), (1.6, 0.155), (1.95, 0.155), (1.95, 0.085),
                           (1.93, 0.085)], rings=((0.55, 0.125), (1.66, 0.163)))
    part("light_tank_barrel", g)
    L_MUZZLES.append(("light_tank_muzzle", (0, 0, -3.2)))


def light_aaa():
    part("light_aaa_hull", light_hull(0.4))
    g = L.Geo()
    P.lathe(g, (0, -0.3, 0), Y, [(0, 0.95), (0.2, 0.95), (0.28, 0.85)], n=8)
    P.accent(g, P.circle((0, -0.02, 0), Y, 0.7, 8), Y, 0.05, closed=True, glow=0.05)
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.5, s * 0.68))
        L.prism_x(g, [(-0.55, -0.05), (0.6, -0.05), (0.6, 0.25), (0.25, 0.5), (-0.3, 0.5),
                      (-0.55, 0.2)], x0, x1)
        xo = s * 0.68
        P.accent(g, [(xo, 0.12, -0.45), (xo, 0.12, 0.5)], (s, 0, 0), 0.05, glow=0.04)
        P.flow(g, [(xo, 0.3, -0.35), (xo, 0.3, 0.45)], (s, 0, 0), 0.025)
    L.box(g, (-0.45, -0.05, 0.6), (0.45, 0.3, 0.95))
    P.tube(g, (0, 0.3, 0.8), (0, 0.48, 0.8), 0.05, n=6)
    L.box(g, (-0.36, 0.45, 0.76), (0.36, 0.68, 0.82))
    P.accent(g, [(-0.28, 0.5, 0.82), (0.28, 0.5, 0.82), (0.28, 0.63, 0.82), (-0.28, 0.63, 0.82)],
             Z, 0.03, closed=True, glow=0.03)
    part("light_aaa_turret", g)
    # Barrel cluster: cradle, trunnion, 2x2 barrels 0.32 m apart, tips 2.4 m ahead.
    g = L.Geo()
    rings = mantlet(g, [(0.4, (0.4, 0.28)), (-0.55, (0.4, 0.28)), (-0.68, (0.3, 0.22))])
    P.tube(g, (-0.52, 0, 0), (0.52, 0, 0), 0.1, n=8)
    for s in (-1, 1):
        xo = s * 0.4
        P.flow(g, [(xo, 0, 0.3), (xo, 0, -0.5)], (s, 0, 0), 0.025)
    for bx in (-0.16, 0.16):
        for by in (-0.16, 0.16):
            gun(g, (bx, by, -0.6), [(0, 0.06), (1.5, 0.05), (1.5, 0.07), (1.8, 0.07),
                                    (1.8, 0.04), (1.78, 0.04)], n=6, rings=((0.9, 0.058),))
    part("light_aaa_barrel", g)
    L_MUZZLES.append(("light_aaa_muzzle", (0, 0, -2.4)))


# ---------------------------------------------------------------------------------------------
# Heavy tank.

def heavy_tank():
    g = L.Geo()
    for s in (-1, 1):
        track(g, s * 1.45, s * 2.1, 3.8, 1.15, 0.7)
        x0, x1 = sorted((s * 2.1, s * 2.2))
        L.prism_x(g, [(-3.35, 1.15), (-3.05, 0.4), (3.15, 0.4), (3.45, 1.15)], x0, x1)
        xo = s * 2.2
        P.accent(g, [(xo, 0.95, -3.0), (xo, 0.95, 3.1)], (s, 0, 0), 0.06, glow=0.05)
        for z in (-2.0, -0.7, 0.6, 1.9):
            P.accent(g, [(xo, 0.5, z), (xo, 0.78, z + 0.2)], (s, 0, 0), 0.05, glow=0.04)
    L.box(g, (-1.45, 0.4, -3.4), (1.45, 1.12, 3.5))
    stations = [(-3.8, (2.0, 1.1, 1.25, 1.45, 1.6)), (-2.2, (2.15, 1.1, 1.55, 1.9, 1.8)),
                (3.4, (2.15, 1.1, 1.55, 1.9, 1.8)), (3.8, (2.05, 1.1, 1.5, 1.75, 1.7))]
    chassis(g, stations, 1)
    turret_ring(g, 0.5, 1.9, 1.15)
    part("heavy_tank_hull", g)
    # Main turret (pivot (0, 2.2, 0.5); ring top at local -0.2).
    g = L.Geo()
    foot = [(0.75, -1.9), (1.45, -1.25), (1.5, 1.3), (1.2, 1.75), (-1.2, 1.75), (-1.5, 1.3),
            (-1.45, -1.25), (-0.75, -1.9)]
    rings = footprint_loft(g, foot, [(-0.2, 1, 1, 0), (0.2, 1.03, 1.02, 0),
                                     (0.6, 0.82, 0.85, 0.05)])
    belt_lines(g, rings, (0, 6), w=0.07)
    top_outline(g, rings[2], 0.88, w=0.06)
    hatch(g, (-0.6, 0.6, 0.15), 0.3)
    for s in (-1, 1):  # applique side slabs
        x0, x1 = sorted((s * 1.47, s * 1.64))
        L.box(g, (x0, -0.12, -1.0), (x1, 0.32, 1.15))
        xo = s * 1.64
        P.accent(g, [(xo, 0.1, -0.88), (xo, 0.1, 1.03)], (s, 0, 0), 0.07, glow=0.05)
        P.flow(g, [(xo, 0.24, -0.88), (xo, 0.24, 1.03)], (s, 0, 0), 0.025)
        for k in range(3):  # smoke launchers
            a = L.add((s * (1.1 + 0.12 * k), 0.62, -1.3 - 0.1 * k), (0, 0, 0))
            b = L.add(a, (s * 0.06, 0.12, -0.22))
            P.tube(g, a, b, 0.05, n=6)
    L.box(g, (-1.0, -0.1, 1.65), (1.0, 0.4, 1.95))
    P.white(g, [(-0.8, 0.15, 1.95), (0.8, 0.15, 1.95)], Z, 0.04, glow=0.04)
    part("heavy_tank_turret", g)
    # Main gun (4.4 m).
    g = L.Geo()
    mantlet(g, [(-1.2, (0.55, 0.34)), (-2.0, (0.46, 0.3)), (-2.15, (0.3, 0.22))])
    gun(g, (0, 0, -2.1), [(0, 0.19), (0.8, 0.17), (0.8, 0.24), (1.3, 0.24), (1.3, 0.17),
                          (1.95, 0.165), (1.95, 0.25), (2.3, 0.25), (2.3, 0.11), (2.28, 0.11)],
        n=10, rings=((0.35, 0.185), (1.0, 0.245), (2.02, 0.255)))
    part("heavy_tank_barrel", g)
    L_MUZZLES.append(("heavy_tank_muzzle", (0, 0, -4.4)))
    # MG cupola (pivot (0, 3.0, 1.2); main turret top at local -0.2).
    g = L.Geo()
    P.lathe(g, (0, -0.22, 0), Y, [(0, 0.42), (0.1, 0.42), (0.14, 0.34)], n=8)
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.12, s * 0.18))
        L.box(g, (x0, -0.08, -0.09), (x1, 0.08, 0.09))
    P.accent(g, P.circle((0, -0.08, 0), Y, 0.28, 8), Y, 0.035, closed=True, glow=0.03)
    zf = -0.42 * math.cos(math.pi / 8)
    P.disc(g, [(-0.08, -0.2, zf), (0.08, -0.2, zf), (0.08, -0.15, zf), (-0.08, -0.15, zf)],
           NZ, mat=LIGHT, glow=0.02)
    part("heavy_tank_mg_turret", g)
    # Chaingun (1.1 m).
    g = L.Geo()
    L.box(g, (-0.09, -0.08, -0.25), (0.09, 0.09, 0.25))
    L.box(g, (0.09, -0.11, -0.1), (0.24, 0.05, 0.15))
    for s in (-1, 1):
        P.accent(g, [(s * 0.09, 0.03, -0.2), (s * 0.09, 0.03, 0.2)], (s, 0, 0), 0.025,
                 glow=0.025)
    P.flow(g, [(0.24, -0.03, -0.06), (0.24, -0.03, 0.11)], X, 0.02)
    for k in range(3):
        a = math.radians(90 + 120 * k)
        bx, by = 0.045 * math.cos(a), 0.045 * math.sin(a)
        P.tube(g, (bx, by, -0.25), (bx, by, -1.1), 0.022, n=5)
    P.lathe(g, (0, 0, -0.25), NZ, [(0, 0.08), (0.08, 0.08)], n=8)
    P.lathe(g, (0, 0, -0.25), NZ, [(0.72, 0.075), (0.78, 0.075)], n=8, mats="accent")
    part("heavy_tank_mg", g)
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

L_MUZZLES = []
LIMIT = 4000


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
            raise SystemExit("%s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
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
