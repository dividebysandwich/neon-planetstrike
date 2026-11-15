"""Builds the MAX exosuit: assets-src/blend/max.blend and assets/models/max.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/max.py

A MAX is drawn with these in place of the infantry avatar's parts (src/actors.rs), posed by the
same rig (src/rig.rs, with the MAX's wider shoulders and hips), so every mesh is in the frame of
the infantry part it replaces and as long between its joints:

| mesh            | frame (origin, as the infantry part)         | notes |
|-----------------|----------------------------------------------|-------|
| `max_torso`     | the chest's top centre, hanging down -Y      | barrel chest with a dark breastplate carrying `hit_1..3` (and again on the back pack), gorget, shoulder sockets at (+-0.32, -0.07, 0), bellows waist and belt, a reactor pack with exhaust stacks and hoses to the shoulders |
| `max_helmet`    | between the eyes, looking along -Z           | closed helm: glass visor band with a lit slit (`eyes`), respirator with filter cans, crest, sensor pod, neck gaiter down into the gorget |
| `max_pauldron`  | the shoulder joint, outward +X               | the right one; the left is the same mesh turned half round about Y (it is alike front and back) |
| `max_upper_arm` | the shoulder, 0.27 m down to the elbow       | under the pauldron: sleeve, side plates, a pair of pistons behind |
| `max_forearm`   | the elbow, 0.25 m down to the wrist          | elbow drum, vambrace, wrist collar |
| `max_fist`      | the glove's (palm at the origin, fingers -Z) | a clenched gauntlet with its wrist ball at (0, 0, 0.075), alike on both sides, so it serves either hand (the gun hand's is part of the arm gun: weapons.py) |
| `max_pelvis`    | top centre, hips at (+-0.12, -0.12, 0)       | belt, front and back tassets, hip plates |
| `max_thigh`     | the hip, 0.38 m down to the knee             | cuisse plates, knee drum and cop |
| `max_shin`      | the knee, 0.37 m down to the ankle           | flared greave, calf pistons, ankle collar |
| `max_boot`      | the ankle (sole 0.08 m below, toe -Z)        | wide two-toed boot on a rubber sole |

Every mesh but the pauldron is X mirror symmetric (one mesh serves both sides). Materials:
`armor` (plates), `body` and `polymer` (dark structure and undersuit), `rubber`, `metal`,
`heat`, `glass`, `paint` (team markings), `accent`/`glow`, `glow_white`, `flow`, `eyes` (the
visor's slit), `hit_1..3`.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, X, Y, Z, NX, NY, NZ  # noqa: E402

LIMIT = 2500


def oct(g, secs, mat="armor", cap0=True, cap1=True, zc=0.0):
    """A body lofted downward through sections (y, half width, half depth, chamfer)."""
    rings = [[(x, y, z + zc) for x, z in L.octagon_xz(hx, hz, ch)] for y, hx, hz, ch in secs]
    first = mat if isinstance(mat, str) else mat[0]
    last = mat if isinstance(mat, str) else mat[-1]
    L.loft(g, rings, NY, mats=mat, cap0=first if cap0 else None, cap1=last if cap1 else None)


def band(g, y0, y1, hx, hz, ch, mat="accent", zc=0.0):
    """A thin band round an octagonal body."""
    oct(g, [(y0, hx, hz, ch), (y1, hx, hz, ch)], mat, cap0=False, cap1=False, zc=zc)


def slab_z(g, pts_xy, z0, dz, mat="armor"):
    """A plate over an (x, y) polygon at z0, dz thick (negative: towards the front)."""
    P.extrude(g, [(x, y, z0) for x, y in pts_xy], (0, 0, dz), mat)


def sbox(g, a, b, mat="body"):
    L.box(g, tuple(map(min, a, b)), tuple(map(max, a, b)), mat=mat)


def strip(g, pts, n, w, mat="accent"):
    P._strip3(g, pts, n, w, False, 0.0, mat, "glow", 0.0015)


def bar(g, n, x0, y0, x1, y1, z, mat):
    """A filled lit bar on the plane z (normal n = +-Z)."""
    P.disc(g, [(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)], n, mat=mat, glow=0.0,
           h=0.002)


def piston(g, a, b, r, n=8):
    """A hydraulic strut from a to b: a fat sleeve over the first half, a bright rod, collars."""
    mid = P.lerp(a, b, 0.55)
    P.tube(g, a, mid, r, n=n, mat="body")
    P.tube(g, mid, b, r * 0.55, n=n, mat="metal")
    d = L.norm(L.sub(b, a))
    for p in (a, mid):
        P.lathe(g, L.sub(p, L.mul(d, r * 0.3)), d, [(0, r * 1.25), (r * 0.6, r * 1.25)], n=n,
                mats="metal", cap0="metal", cap1="metal")


def wrist_ball(g, n=12):
    """The ball of a MAX's wrist in the glove's frame (weapons.py has the same on its arm
    guns): whichever way the forearm comes in, its collar ends in this."""
    P.lathe(g, (0, 0, 0.16), NZ, [(0, 0.035), (0.02, 0.062), (0.05, 0.078), (0.1, 0.078),
                                   (0.13, 0.062), (0.15, 0.04)], n=n, mats="metal",
            cap0="metal", cap1="metal")
    P.lathe(g, (0, 0, 0.16), NZ, [(0.068, 0.0795), (0.082, 0.0795)], n=n, mats="accent",
            cap0=None, cap1=None)


# ---------------------------------------------------------------------------------------------

def torso():
    g = L.Geo()
    oct(g, [(0.05, 0.14, 0.13, 0.045), (0.0, 0.21, 0.17, 0.06), (-0.07, 0.3, 0.2, 0.07),
            (-0.24, 0.29, 0.2, 0.07), (-0.35, 0.21, 0.165, 0.055)], "armor")
    # Bellows waist with ribs, and the belt.
    oct(g, [(-0.33, 0.2, 0.15, 0.05), (-0.5, 0.2, 0.15, 0.05)], "rubber")
    for y in (-0.375, -0.41, -0.445):
        band(g, y, y - 0.014, 0.206, 0.156, 0.05, "body")
    oct(g, [(-0.468, 0.218, 0.168, 0.055), (-0.52, 0.218, 0.168, 0.055)], "metal")
    band(g, -0.486, -0.5, 0.2195, 0.1695, 0.055)
    # Gorget: a raised collar round the neck.
    P.lathe(g, (0, 0.03, 0.02), Y, [(0, 0.15), (0.05, 0.135), (0.06, 0.12)], n=12, mats="metal",
            cap0=None, cap1="rubber")
    band(g, 0.02, 0.008, 0.185, 0.155, 0.053)
    # Breastplate: a dark faceted slab, outlined in the team's light, the health bars on it.
    zf = -0.2
    chest = [(-0.2, -0.05), (0.2, -0.05), (0.225, -0.09), (0.21, -0.22), (0.13, -0.3),
             (-0.13, -0.3), (-0.21, -0.22), (-0.225, -0.09)]
    slab_z(g, chest, zf + 0.01, -0.05, "body")
    zc = zf - 0.04
    P.accent(g, [(x * 0.92, -0.17 + (y + 0.17) * 0.9, zc) for x, y in chest], NZ, 0.009,
             closed=True, glow=0.012, h=0.002)
    for mat, y in (("hit_1", -0.115), ("hit_2", -0.165), ("hit_3", -0.215)):
        bar(g, NZ, -0.1, y - 0.016, 0.1, y + 0.016, zc, mat)
    for s in (-1, 1):
        P.bolt_row(g, (s * 0.165, -0.08, zc), (s * 0.165, -0.2, zc), NZ, 4, r=0.008, h=0.005,
                   ends=True)
    # Two belly lames under it.
    for y0, y1, hx in ((-0.295, -0.33, 0.15), (-0.325, -0.355, 0.125)):
        slab_z(g, [(-hx, y0), (hx, y0), (hx - 0.02, y1), (-hx + 0.02, y1)], zf + 0.03, -0.045,
               "armor")
    # Shoulder sockets.
    for s in (-1, 1):
        P.lathe(g, (s * 0.265, -0.07, 0), (s, 0, 0), [(0, 0.1), (0.05, 0.1), (0.065, 0.08)],
                n=12, mats="metal", cap0=None, cap1="rubber")
        P.plate(g, [(s * 0.1, 0.0005, -0.1), (s * 0.19, 0.0005, -0.1), (s * 0.19, 0.0005, 0.1),
                    (s * 0.1, 0.0005, 0.1)], L.norm((s * 0.7, 1, 0)), 0.004, "paint")
    # The pack: a reactor housing on the back, its core glowing through a ring, exhaust
    # stacks, side vents, and hoses over to the shoulders. The health bars again below.
    zb = 0.2
    P.bevel_box(g, (-0.21, -0.33, zb - 0.02), (0.21, -0.03, zb + 0.15), 0.03, mat="body")
    zp = zb + 0.15
    P.lathe(g, (0, -0.1, zp - 0.005), Z, [(0, 0.062), (0.02, 0.055)], n=12, mats="metal",
            cap0=None, cap1=LIGHT)
    P.accent(g, P.circle((0, -0.1, zp), Z, 0.07, 12), Z, 0.008, closed=True, glow=0.015,
             h=0.002)
    for mat, y in (("hit_1", -0.19), ("hit_2", -0.235), ("hit_3", -0.28)):
        bar(g, Z, -0.1, y - 0.014, 0.1, y + 0.014, zp, mat)
    for s in (-1, 1):
        P.lathe(g, (s * 0.14, -0.06, zb + 0.07), Y, [(0, 0.045), (0.13, 0.045), (0.15, 0.036)],
                n=10, mats="heat", cap0="heat", cap1="rubber")
        P.lathe(g, (s * 0.14, 0.0, zb + 0.07), Y, [(0, 0.048), (0.014, 0.048)], n=10,
                mats="accent", cap0=None, cap1=None)
        P.vent(g, (s * 0.21, -0.18, zb + 0.065), (s, 0, 0), 0.09, 0.2, slats=6, depth=0.012,
               mat="metal", mat_in="rubber", frame=0.01)
        P.hose(g, [(s * 0.08, -0.04, zb + 0.1), (s * 0.1, 0.03, zb + 0.06), (s * 0.17, 0.04, 0.1),
                   (s * 0.23, 0.0, 0.03)], 0.02, n=6, collars=(1, 2))
    return g


def helmet():
    g = L.Geo()
    zc = 0.07
    oct(g, [(0.165, 0.075, 0.09, 0.035), (0.125, 0.128, 0.145, 0.05), (0.04, 0.145, 0.165, 0.055),
            (-0.06, 0.145, 0.165, 0.055), (-0.14, 0.125, 0.15, 0.05), (-0.19, 0.1, 0.12, 0.04)],
        "armor", zc=zc)
    zf = zc - 0.165
    # Visor: a glass band across the front and round the corners, a lit slit in it.
    slab_z(g, [(-0.092, -0.035), (0.092, -0.035), (0.092, 0.03), (-0.092, 0.03)], zf + 0.005,
           -0.017, "glass")
    for s in (-1, 1):
        nrm = L.norm((s, 0, -1))
        P.plate(g, [(s * 0.09, -0.035, zf), (s * 0.145, -0.035, zf + 0.055),
                    (s * 0.145, 0.03, zf + 0.055), (s * 0.09, 0.03, zf)], nrm, 0.012, "glass")
        strip(g, [L.add((s * 0.098, 0.0, zf + 0.008), L.mul(nrm, 0.0125)),
                  L.add((s * 0.137, 0.0, zf + 0.047), L.mul(nrm, 0.0125))], nrm, 0.012, "eyes")
    strip(g, [(-0.08, 0.0, zf - 0.0125), (0.08, 0.0, zf - 0.0125)], NZ, 0.012, "eyes")
    # Brow, cheek guards and the respirator with its filter cans.
    P.bevel_box(g, (-0.105, 0.03, zf - 0.03), (0.105, 0.075, zf + 0.02), 0.012, mat="body")
    P.bevel_box(g, (-0.06, -0.16, zf - 0.04), (0.06, -0.04, zf + 0.02), 0.014, mat="body")
    for s in (-1, 1):
        P.vent(g, (s * 0.03, -0.1, zf - 0.04), NZ, 0.035, 0.07, slats=4, depth=0.006,
               mat="metal", mat_in="rubber", frame=0.004)
        d = L.norm((s, -0.25, -0.55))
        P.lathe(g, (s * 0.06, -0.11, zf + 0.0), d, [(0, 0.03), (0.05, 0.03), (0.06, 0.022)],
                n=10, mats="metal", cap0=None, cap1="rubber")
        P.bevel_box(g, tuple(map(min, (s * 0.1, -0.08, zf + 0.01), (s * 0.15, -0.035, zf + 0.09))),
                    tuple(map(max, (s * 0.1, -0.08, zf + 0.01), (s * 0.15, -0.035, zf + 0.09))),
                    0.01, mat="body")
        # Ear discs.
        P.lathe(g, (s * 0.14, -0.02, zc + 0.02), (s, 0, 0), [(0, 0.055), (0.018, 0.05), (0.024, 0.03)],
                n=10, mats="metal", cap0=None, cap1="body")
        P.accent(g, P.circle((s * 0.1585, -0.02, zc + 0.02), (s, 0, 0), 0.04, 10), (s, 0, 0),
                 0.006, closed=True, glow=0.0, h=0.001)
    # Crest in the team's paint, a sensor pod with an aerial on the right.
    P.extrude(g, [(-0.014, 0.155, zc - 0.11), (-0.014, 0.155, zc + 0.1), (-0.014, 0.2, zc + 0.07),
                  (-0.014, 0.2, zc - 0.06)], (0.028, 0, 0), mat="paint")
    P.bevel_box(g, (0.14, 0.03, zc - 0.07), (0.175, 0.085, zc + 0.04), 0.008, mat="metal")
    g.face([(0.146, 0.04, zc - 0.0702), (0.169, 0.04, zc - 0.0702), (0.169, 0.075, zc - 0.0702),
            (0.146, 0.075, zc - 0.0702)], LIGHT, want=NZ)
    P.tube(g, (0.158, 0.085, zc + 0.02), (0.158, 0.22, zc + 0.03), 0.004, n=4, mat="metal")
    # Gaiter down into the gorget.
    P.lathe(g, (0, -0.17, zc + 0.01), NY, [(0, 0.1), (0.05, 0.085), (0.13, 0.1)], n=12,
            mats="rubber", cap0=None, cap1=None)
    return g


def pauldron():
    g = L.Geo()

    def ring(x, hz, y0, y1, c):
        return [(x, y0, -hz), (x, y1 - c, -hz), (x, y1, -hz + c), (x, y1, hz - c),
                (x, y1 - c, hz), (x, y0, hz)]
    L.loft(g, [ring(-0.1, 0.1, 0.0, 0.1, 0.04), ring(-0.02, 0.165, -0.04, 0.15, 0.06),
               ring(0.09, 0.175, -0.1, 0.135, 0.07), ring(0.17, 0.15, -0.16, 0.05, 0.06),
               ring(0.19, 0.11, -0.16, -0.02, 0.04)], X, mats="armor", cap0="armor",
           cap1="armor")
    # A second lame hanging under the first, and a raised ridge over the top.
    L.loft(g, [ring(0.07, 0.13, -0.2, -0.06, 0.04), ring(0.17, 0.135, -0.27, -0.1, 0.05),
               ring(0.2, 0.1, -0.27, -0.17, 0.03)], X, mats="body", cap0="body", cap1="body")
    P.accent(g, [(0.2005, -0.255, -0.08), (0.2005, -0.185, -0.08), (0.2005, -0.185, 0.08),
                 (0.2005, -0.255, 0.08)], X, 0.01, closed=True, glow=0.0, h=0.0015)
    L.loft(g, [ring(-0.06, 0.03, 0.1, 0.14, 0.012), ring(-0.02, 0.035, 0.12, 0.172, 0.014),
               ring(0.09, 0.035, 0.1, 0.158, 0.014), ring(0.16, 0.03, 0.02, 0.085, 0.012)], X,
           mats="metal", cap0="metal", cap1="metal")
    # The team's chevrons either side of the ridge, rivets along the rim.
    nrm = L.norm((0.136, 1, 0))
    for sz in (-1, 1):
        for x0 in (0.0, 0.045):
            y0, y1 = 0.15 - 0.136 * (x0 + 0.02) + 0.0005, 0.15 - 0.136 * (x0 + 0.05) + 0.0005
            P.plate(g, [(x0, y0 + 0.0027, sz * 0.045), (x0 + 0.02, y0, sz * 0.045),
                        (x0 + 0.05, y1, sz * 0.1), (x0 + 0.03, y1 + 0.0027, sz * 0.1)], nrm, 0.004,
                    "paint")
        fn = L.norm((0.1, 0, sz))
        P.bolt_row(g, (-0.01, -0.01, sz * 0.166), (0.08, -0.07, sz * 0.1755), fn, 4, r=0.009,
                   h=0.005, ends=True)
    return g


def upper_arm():
    g = L.Geo()
    oct(g, [(0.04, 0.05, 0.05, 0.018), (-0.02, 0.078, 0.078, 0.026), (-0.2, 0.072, 0.072, 0.024),
            (-0.275, 0.056, 0.056, 0.018)], "polymer")
    band(g, -0.215, -0.23, 0.071, 0.071, 0.024)
    for s in (-1, 1):
        P.bevel_box(g, tuple(map(min, (s * 0.062, -0.21, -0.055), (s * 0.092, -0.06, 0.055))),
                    tuple(map(max, (s * 0.062, -0.21, -0.055), (s * 0.092, -0.06, 0.055))), 0.012,
                    mat="armor")
        piston(g, (s * 0.032, -0.03, 0.082), (s * 0.032, -0.255, 0.074), 0.016)
    slab_z(g, [(-0.045, -0.06), (0.045, -0.06), (0.035, -0.22), (-0.035, -0.22)], -0.066,
           -0.022, "armor")
    return g


def forearm():
    g = L.Geo()
    P.lathe(g, (-0.09, 0, 0), X, [(0, 0.05), (0.012, 0.068), (0.168, 0.068), (0.18, 0.05)],
            n=12, mats="metal", cap0="metal", cap1="metal")
    for s in (-1, 1):
        P.accent(g, P.circle((s * 0.0905, 0, 0), (s, 0, 0), 0.034, 12), (s, 0, 0), 0.007,
                 closed=True, glow=0.0, h=0.001)
    oct(g, [(-0.02, 0.058, 0.06, 0.02), (-0.075, 0.086, 0.092, 0.03), (-0.19, 0.08, 0.086, 0.028),
            (-0.235, 0.068, 0.07, 0.024)], "armor")
    P.lathe(g, (0, -0.215, 0), NY, [(0, 0.082), (0.04, 0.082)], n=12, mats="metal",
            cap0="metal", cap1="rubber")
    P.lathe(g, (0, -0.228, 0), NY, [(0, 0.0835), (0.012, 0.0835)], n=12, mats="accent",
            cap0=None, cap1=None)
    # A raised strake down the front in the team's paint, service panels on the sides.
    slab_z(g, [(-0.03, -0.07), (0.03, -0.07), (0.024, -0.19), (-0.024, -0.19)], -0.086, -0.016,
           "body")
    P.plate(g, [(-0.014, -0.085, -0.1022), (0.014, -0.085, -0.1022), (0.011, -0.175, -0.1022),
                (-0.011, -0.175, -0.1022)], NZ, 0.003, "paint")
    for s in (-1, 1):
        P.panel_inset(g, [(s * 0.0835, -0.17, -0.04), (s * 0.0835, -0.17, 0.04),
                          (s * 0.0835, -0.09, 0.04), (s * 0.0835, -0.09, -0.04)], (s, 0, 0),
                      h=0.006, rim=0.008, mat="armor", mat_in="body")
        P.hose(g, [(s * 0.045, -0.02, 0.07), (s * 0.05, -0.1, 0.1), (s * 0.04, -0.2, 0.082)],
               0.012, n=6, collars=(1,))
    return g


def fist():
    g = L.Geo()
    wrist_ball(g)
    # The hand: thin across the palm (X), the four fingers clenched in a stack (Y).
    P.bevel_box(g, (-0.05, -0.06, -0.06), (0.05, 0.06, 0.04), 0.014, mat="armor")
    for i in range(4):
        y = -0.057 + i * 0.029
        P.bevel_box(g, (-0.046, y, -0.118), (0.046, y + 0.027, -0.05), 0.008, mat="body")
        P.bevel_box(g, (-0.05, y + 0.003, -0.098), (0.05, y + 0.024, -0.064), 0.006, mat="metal")
    for s in (-1, 1):
        strip(g, [(s * 0.0505, -0.045, -0.02), (s * 0.0505, 0.045, -0.02)], (s, 0, 0), 0.008)
        P.bolt_row(g, (s * 0.05, -0.04, 0.015), (s * 0.05, 0.04, 0.015), (s, 0, 0), 3, r=0.006,
                   h=0.004, ends=True)
    return g


def pelvis():
    g = L.Geo()
    oct(g, [(0.0, 0.13, 0.11, 0.04), (-0.06, 0.19, 0.14, 0.05), (-0.14, 0.2, 0.14, 0.05),
            (-0.2, 0.12, 0.1, 0.04), (-0.245, 0.07, 0.07, 0.025)], "polymer")
    oct(g, [(-0.035, 0.2, 0.152, 0.052), (-0.09, 0.208, 0.156, 0.052)], "metal")
    band(g, -0.056, -0.07, 0.2095, 0.1575, 0.052)
    # Buckle, the front and back tassets, the hip plates.
    P.bevel_box(g, (-0.04, -0.085, -0.172), (0.04, -0.04, -0.15), 0.008, mat="body")
    g.face([(-0.025, -0.075, -0.1722), (0.025, -0.075, -0.1722), (0.025, -0.05, -0.1722),
            (-0.025, -0.05, -0.1722)], LIGHT, want=NZ)
    for z0, dz in ((-0.13, -0.035), (0.13, 0.035)):
        slab_z(g, [(-0.075, -0.085), (0.075, -0.085), (0.05, -0.26), (-0.05, -0.26)], z0, dz,
               "armor")
        n = NZ if dz < 0 else Z
        strip(g, [(0, -0.11, z0 + dz), (0, -0.235, z0 + dz)], n, 0.012, "paint")
    for s in (-1, 1):
        P.bevel_box(g, tuple(map(min, (s * 0.195, -0.23, -0.1), (s * 0.24, -0.08, 0.1))),
                    tuple(map(max, (s * 0.195, -0.23, -0.1), (s * 0.24, -0.08, 0.1))), 0.02,
                    mat="armor")
        strip(g, [(s * 0.2405, -0.205, -0.07), (s * 0.2405, -0.205, 0.07)], (s, 0, 0), 0.01)
        P.bolt_row(g, (s * 0.24, -0.11, -0.065), (s * 0.24, -0.11, 0.065), (s, 0, 0), 3,
                   r=0.008, h=0.005, ends=True)
    return g


def thigh():
    g = L.Geo()
    oct(g, [(0.05, 0.05, 0.05, 0.018), (-0.03, 0.095, 0.095, 0.03), (-0.2, 0.09, 0.09, 0.03),
            (-0.33, 0.07, 0.072, 0.024), (-0.4, 0.056, 0.056, 0.018)], "polymer")
    # Cuisse: a front plate with the team's stripe, side plates.
    slab_z(g, [(-0.072, -0.03), (0.072, -0.03), (0.058, -0.3), (-0.058, -0.3)], -0.075, -0.035,
           "armor")
    P.plate(g, [(-0.05, -0.07, -0.1102), (0.05, -0.07, -0.1102), (0.047, -0.1, -0.1102),
                (-0.047, -0.1, -0.1102)], NZ, 0.004, "paint")
    for s in (-1, 1):
        P.bevel_box(g, tuple(map(min, (s * 0.078, -0.27, -0.06), (s * 0.108, -0.05, 0.065))),
                    tuple(map(max, (s * 0.078, -0.27, -0.06), (s * 0.108, -0.05, 0.065))), 0.014,
                    mat="armor")
        P.flow(g, [(s * 0.1085, -0.08, 0.0), (s * 0.1085, -0.24, 0.0)], (s, 0, 0), 0.008)
        piston(g, (s * 0.04, -0.06, 0.098), (s * 0.035, -0.33, 0.07), 0.016)
    # Knee: a drum across the joint, a cop over the front.
    P.lathe(g, (-0.085, -0.38, 0), X, [(0, 0.05), (0.012, 0.066), (0.158, 0.066), (0.17, 0.05)],
            n=12, mats="metal", cap0="metal", cap1="metal")
    P.bevel_box(g, (-0.06, -0.44, -0.11), (0.06, -0.315, -0.055), 0.018, mat="armor")
    strip(g, [(-0.035, -0.378, -0.1105), (0.035, -0.378, -0.1105)], NZ, 0.01)
    return g


def shin():
    g = L.Geo()
    oct(g, [(0.02, 0.055, 0.055, 0.02), (-0.06, 0.088, 0.094, 0.03), (-0.24, 0.078, 0.088, 0.028),
            (-0.34, 0.066, 0.072, 0.024), (-0.395, 0.05, 0.05, 0.018)], "armor")
    # A raised ridge down the greave with a light line, pistons behind the calf, the collar.
    slab_z(g, [(-0.045, -0.04), (0.045, -0.04), (0.034, -0.33), (-0.034, -0.33)], -0.07, -0.04,
           "body")
    P.flow(g, [(0, -0.06, -0.1105), (0, -0.31, -0.1105)], NZ, 0.01)
    for s in (-1, 1):
        piston(g, (s * 0.036, -0.03, 0.1), (s * 0.034, -0.3, 0.092), 0.017)
        P.panel_inset(g, [(s * 0.0795, -0.22, -0.035), (s * 0.0795, -0.22, 0.04),
                          (s * 0.0795, -0.1, 0.04), (s * 0.0795, -0.1, -0.035)], (s, 0, 0),
                      h=0.008, rim=0.008, mat="armor", mat_in="body")
    P.lathe(g, (0, -0.325, 0), NY, [(0, 0.085), (0.04, 0.085)], n=12, mats="metal",
            cap0="metal", cap1="metal")
    P.lathe(g, (0, -0.338, 0), NY, [(0, 0.0865), (0.012, 0.0865)], n=12, mats="accent",
            cap0=None, cap1=None)
    return g


def boot():
    g = L.Geo()

    def ring(z, hx, y0, y1, ch):
        return [(x, y, z) for x, y in L.chamfer_rect(-hx, y0, hx, y1, ch)]
    L.loft(g, [ring(0.1, 0.06, -0.07, 0.0, 0.02), ring(0.05, 0.082, -0.07, 0.05, 0.026),
               ring(-0.06, 0.088, -0.07, 0.03, 0.026), ring(-0.135, 0.084, -0.07, -0.005, 0.02)],
           NZ, mats="armor", cap0="armor", cap1="armor")
    oct(g, [(0.065, 0.072, 0.072, 0.024), (-0.02, 0.08, 0.08, 0.026)], "metal", zc=0.01)
    # Two toes with metal caps, a heel spur, the rubber sole under it all.
    for s in (-1, 1):
        a, b = (s * 0.008, -0.07, -0.235), (s * 0.084, -0.012, -0.12)
        P.bevel_box(g, tuple(map(min, a, b)), tuple(map(max, a, b)), 0.012, mat="body")
        a, b = (s * 0.004, -0.045, -0.24), (s * 0.088, -0.006, -0.185)
        P.bevel_box(g, tuple(map(min, a, b)), tuple(map(max, a, b)), 0.01, mat="metal")
        strip(g, [(s * 0.0885, -0.035, -0.04), (s * 0.0885, -0.035, 0.07)], (s, 0, 0), 0.008)
    P.bevel_box(g, (-0.045, -0.07, 0.09), (0.045, -0.015, 0.135), 0.012, mat="body")
    P.bevel_box(g, (-0.092, -0.08, -0.242), (0.092, -0.064, 0.14), 0.005, mat="rubber")
    P.flow(g, [(0, 0.0305, -0.058), (0, -0.0045, -0.133)], L.norm((0, 0.075, -0.035)), 0.01)
    return g


MESHES = (("max_torso", torso, None), ("max_helmet", helmet, None), ("max_pauldron", pauldron, None),
          ("max_upper_arm", upper_arm, None), ("max_forearm", forearm, 35),
          ("max_fist", fist, 35), ("max_pelvis", pelvis, None), ("max_thigh", thigh, 35),
          ("max_shin", shin, 35), ("max_boot", boot, None))


def main():
    L.reset_scene()
    P.materials()
    L.material("eyes", base=(1, 1, 1), emit=(1, 1, 1), strength=5.0)
    for i in (1, 2, 3):
        L.material("hit_%d" % i, base=(1, 1, 1), emit=(1, 1, 1), strength=3.0)
    for name, fn, smooth in MESHES:
        g = fn()
        if g.tris() > LIMIT:
            raise SystemExit("%s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
        L.make_object(name, g, smooth=smooth)
        print("max %-14s %5d tris" % (name, g.tris()))
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/max.blend")
    L.export_glb("assets/models/max.glb")


main()
