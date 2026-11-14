"""Builds the alien base equipment of the canyon caves (Hollow Reach):
assets-src/blend/alien.blend and assets/models/alien.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/alien.py

Each mesh stands in for the props.glb one without the `alien_` prefix, in the same frame and
within the same bounds (the game swaps them in on a cave continent; the colliders stay): origin
on the floor in its middle, front facing -Z. Organic and crystalline: lathed pods and bulbs,
ribbed carapaces, crystal spikes and shards, the team's neon in rings and veins.

- `alien_terminal_equipment`, `_medical`, `_vehicle`: an octagonal monolith (0.9 x 0.44 m)
  with a crystal crown to 1.9 m; a white screen on its flat front and the terminal's motif
  (a diamond, a cross, a chevron) in team light under it.
- `alien_console_core`: a mushroom stalk flaring into a cap at 1.15 m with a white disc on top;
  a team ring and a floating crystal over it, up to 2.55 m.
- `alien_generator`: a ribbed pod 4 m across: a base drum and a domed cap on six curved claws,
  the core showing between them; team rings, crystal spikes on top to 3.5 m.
- `alien_generator_core`: a white crystal (bipyramid, radius 0.6 m, y 0.8-2.4).
- `alien_vehicle_pad`, `alien_rearm_pad`: discs (radius 7 and 8 m) with team rings and veins;
  the rearm pad's four crystal pylons (to 3 m) at radius 9 m on the diagonals.
- `alien_spawn_tube`: a floor disc and a ring overhead (to 2.4 m) on two crystal posts at
  x = +-0.68 (walked through along Z).
- `alien_resource_silo`: eight bulging ribs round the fill (radius 4 m) between a base drum and
  a domed cap (to 12.3 m), open between them; `alien_resource_silo_fill`: a 16-sided `accent`
  column of radius 3.6 m from y = 0 to 1 (the game scales it in Y).
- `alien_wall_turret_base`: a flared organic pedestal to 1.8 m (radius 1.25 m at the foot).
- `alien_wall_turret_head`: ORIGIN AT ITS PIVOT: a turntable down to y = -0.6, carapace pods
  either side (x +-0.95), twin barrels at x = +-0.55 to z = -2.6, spines up to 0.8 m; the
  gunner sits open behind them (eye at (0, 0.5, 0.6)).
- `alien_capture_pylon`: a twisted crystal obelisk to 4.0 m with a team crown ring.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, Y, NY, NZ  # noqa: E402

LIMIT = 2000


def crystal(g, base, axis, length, r, n=6, tip=0.3, twist=0.0, mat="body"):
    """A faceted crystal: an n-gon prism from `base` along `axis`, a pointed tip of `tip` x
    its length."""
    a = L.norm(axis)
    body = length * (1 - tip)
    rings = [P.circle(base, a, r * 0.8, n, phase=twist),
             P.circle(L.add(base, L.mul(a, body * 0.5)), a, r, n, phase=twist + 0.2),
             P.circle(L.add(base, L.mul(a, body)), a, r * 0.9, n, phase=twist + 0.4)]
    L.loft(g, rings, a, mats=mat, cap0=mat)
    apex = L.add(base, L.mul(a, length))
    top = rings[-1]
    for i in range(n):
        j = (i + 1) % n
        mid = L.mul(L.add(top[i], top[j]), 0.5)
        out = L.add(L.sub(mid, L.add(base, L.mul(a, body))), L.mul(a, r))
        g.face([top[i], top[j], apex], mat, want=out)


def ring(g, y, r, w, n=24, glow=0.05):
    """A team ring round the Y axis at height y, facing up."""
    P.accent(g, P.circle((0, y, 0), Y, r, n), Y, w, closed=True, glow=glow)


def ring_at(g, c, r, w, n=16, glow=0.05):
    P.accent(g, P.circle(c, Y, r, n), Y, w, closed=True, glow=glow)


def claw(g, a, b, bulge, r, segs=5, mat="body"):
    """A curved claw from a to b bowing out by `bulge` (a vector), in short tubes."""
    pts = []
    for k in range(segs + 1):
        t = k / segs
        bow = 4 * t * (1 - t)
        pts.append(L.add(P.lerp(a, b, t), L.mul(bulge, bow)))
    for k in range(segs):
        rr = r * (1.0 - 0.35 * abs(k / segs - 0.5))
        P.tube(g, pts[k], pts[k + 1], rr, n=5, mat=mat)


# ---------------------------------------------------------------------------------------------
# Terminals and the console.

TERM = [(-0.3, -0.22), (0.3, -0.22), (0.45, -0.08), (0.45, 0.08), (0.3, 0.22), (-0.3, 0.22),
        (-0.45, 0.08), (-0.45, -0.08)]


def terminal(motif):
    g = L.Geo()
    foot = [(x, 0.0, z) for x, z in TERM]
    P.extrude(g, foot, (0, 1.6, 0), caps=(False, False))
    # Crystal crown: from the top rim to a point at 1.9 m.
    top = [(x * 0.95, 1.6, z * 0.95) for x, z in TERM]
    # (Close the top's step.)
    for i in range(len(TERM)):
        j = (i + 1) % len(TERM)
        a, b = (TERM[i][0], 1.6, TERM[i][1]), (TERM[j][0], 1.6, TERM[j][1])
        g.face([a, b, top[j], top[i]], "body", want=Y)
        mid = L.mul(L.add(top[i], top[j]), 0.5)
        g.face([top[i], top[j], (0, 1.9, 0)], "body", want=L.add((mid[0], 0, mid[2]), (0, 0.3, 0)))
    # A team vein round the crown's foot, and up the back corners.
    P.accent(g, [(x * 0.95, 1.62, z * 0.95) for x, z in TERM], Y, 0.03, closed=True, glow=0.03)
    # The screen on the flat front.
    z = -0.22 - 0.002
    P.disc(g, [(-0.25, 0.95, z), (0.25, 0.95, z), (0.25, 1.48, z), (-0.25, 1.48, z)], NZ,
           mat=LIGHT, glow=0.03)
    # The motif under it, in team light.
    c = (0.0, 0.55, z)
    if motif == "equipment":
        P.accent(g, [(0, 0.75, z), (0.17, 0.55, z), (0, 0.35, z), (-0.17, 0.55, z)], NZ, 0.04,
                 closed=True, glow=0.03)
    elif motif == "medical":
        P.accent(g, [(0, 0.35, z), (0, 0.75, z)], NZ, 0.07, glow=0.03)
        P.accent(g, [(-0.2, 0.55, z), (0.2, 0.55, z)], NZ, 0.07, glow=0.03)
    else:
        for dy in (0.0, 0.16):
            P.accent(g, [(-0.18, 0.45 + dy, z), (0, 0.6 + dy, z), (0.18, 0.45 + dy, z)], NZ, 0.05,
                     glow=0.03)
    del c
    return g


def console_core():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.66), (0.12, 0.5), (0.3, 0.32), (0.8, 0.26), (1.0, 0.55),
                              (1.12, 0.7), (1.15, 0.6)], n=10, cap0=None)
    ring(g, 1.15 + 0.002, 0.5, 0.04, n=10)
    P.disc(g, P.circle((0, 1.153, 0), Y, 0.32, 10), Y, mat=LIGHT, glow=0.03)
    # Veins down the stalk.
    for k in range(5):
        a = math.pi / 2 + 2 * math.pi * k / 5
        n = (math.cos(a), 0.0, math.sin(a))
        p0 = (0.3 * n[0], 0.35, 0.3 * n[2])
        p1 = (0.26 * n[0], 0.8, 0.26 * n[2])
        P.accent(g, [p0, p1], n, 0.03, glow=0.03)
    # The floating ring and crystal.
    ring(g, 1.5, 0.55, 0.05, n=16)
    crystal(g, (0, 1.6, 0), Y, 0.95, 0.2, n=6, tip=0.5, mat="accent")
    return g


# ---------------------------------------------------------------------------------------------
# The generator.

def generator():
    g = L.Geo()
    # The base drum (lower and wider than the cap) and the domed cap.
    P.lathe(g, (0, 0, 0), Y, [(0, 1.95), (0.2, 1.98), (0.55, 1.7), (0.7, 1.2)], n=12, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(2.55, 1.15), (2.75, 1.6), (3.0, 1.5), (3.2, 1.0), (3.3, 0.45)],
            n=12, cap0="body")
    ring(g, 0.56, 1.72, 0.06, n=24)
    ring(g, 2.77, 1.62, 0.05, n=24)
    # Six claws from the drum up into the cap, bowing out (the core shows between them).
    for k in range(6):
        a = math.pi / 6 + 2 * math.pi * k / 6
        d = (math.cos(a), 0.0, math.sin(a))
        lo = (1.25 * d[0], 0.65, 1.25 * d[2])
        hi = (1.2 * d[0], 2.6, 1.2 * d[2])
        claw(g, lo, hi, L.mul(d, 0.55), 0.16)
    # Crystal spikes on top.
    crystal(g, (0, 3.25, 0), Y, 0.25, 0.25, n=6, tip=0.6, mat="accent")
    for k in range(4):
        a = math.pi / 4 + math.pi / 2 * k
        base = (0.75 * math.cos(a), 3.05, 0.75 * math.sin(a))
        crystal(g, base, (0.3 * math.cos(a), 1.0, 0.3 * math.sin(a)), 0.42, 0.12, n=5, tip=0.5)
    return g


def generator_core():
    g = L.Geo()
    n, r, lo, hi, c = 8, 0.6, 0.8, 2.4, 1.6
    mid = P.circle((0, c, 0), Y, r, n)
    for i in range(n):
        j = (i + 1) % n
        out = (mid[i][0] + mid[j][0], 0, mid[i][2] + mid[j][2])
        g.face([mid[i], mid[j], (0, hi, 0)], LIGHT, want=L.add(out, (0, 0.5, 0)))
        g.face([mid[i], mid[j], (0, lo, 0)], LIGHT, want=L.add(out, (0, -0.5, 0)))
    return g


# ---------------------------------------------------------------------------------------------
# Pads and the spawn tube.

def pad(R, n=32):
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, R), (0.03, R - 0.04), (0.036, R - 0.15)], n=n, cap0=None,
            cap1="body")
    y = 0.037
    ring(g, y, R - 0.5, 0.12, n=n)
    ring(g, y, R * 0.45, 0.08, n=n)
    # Veins from the inner ring to the outer, curling.
    for k in range(8):
        a0 = 2 * math.pi * k / 8
        pts = []
        for s in range(5):
            t = s / 4
            a = a0 + 0.35 * t
            rr = R * 0.45 + (R - 0.5 - R * 0.45) * t
            pts.append((rr * math.cos(a), y, rr * math.sin(a)))
        P.flow(g, pts, Y, 0.06)
    return g


def vehicle_pad():
    return pad(7.0)


def rearm_pad():
    g = pad(8.0)
    for k in range(4):
        a = math.pi / 4 + math.pi / 2 * k
        base = (9.0 * math.cos(a), 0.0, 9.0 * math.sin(a))
        crystal(g, base, Y, 3.0, 0.32, n=5, tip=0.35)
        ring_at(g, (base[0], 2.0, base[2]), 0.3, 0.05, n=5)
    return g


def spawn_tube():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.65), (0.04, 0.6)], n=12, cap0=None)
    ring(g, 0.042, 0.45, 0.05, n=12)
    P.disc(g, P.circle((0, 0.043, 0), Y, 0.2, 12), Y, mat=LIGHT, glow=0.03)
    for s in (-1, 1):
        crystal(g, (s * 0.68, 0.0, 0.0), Y, 2.4, 0.07, n=4, tip=0.15)
    P.lathe(g, (0, 2.28, 0), Y, [(0, 0.72), (0.12, 0.72)], n=12, cap0="accent", cap1="accent",
            mats="accent")
    return g


# ---------------------------------------------------------------------------------------------
# The silo.

def resource_silo():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 4.0), (0.6, 3.95), (0.9, 3.6)], n=16, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(10.9, 3.6), (11.3, 3.95), (11.8, 3.3), (12.3, 1.2)], n=16,
            cap0="body")
    ring(g, 0.91, 3.6, 0.1, n=16)
    ring(g, 11.31, 3.9, 0.1, n=16)
    for k in range(8):
        a = math.pi / 8 + 2 * math.pi * k / 8
        c = (3.75 * math.cos(a), 0.0, 3.75 * math.sin(a))
        P.lathe(g, c, Y, [(0.8, 0.22), (4.0, 0.3), (8.0, 0.26), (11.0, 0.18)], n=4,
                phase=a)
        # A team vein down each rib's outer face.
        d = (math.cos(a), 0.0, math.sin(a))
        o = L.add(c, L.mul(d, 0.21))
        P.accent(g, [L.add(o, (0, 1.2, 0)), L.add(o, (0, 10.4, 0))], d, 0.05, glow=0.04)
    return g


def resource_silo_fill():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 3.6), (1.0, 3.6)], n=16, mats="accent", cap0="accent",
            cap1="accent")
    return g


# ---------------------------------------------------------------------------------------------
# The wall turret.

def wall_turret_base():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 1.25), (0.3, 1.0), (0.9, 0.65), (1.4, 0.8), (1.8, 1.05)],
            n=10, cap0=None)
    ring(g, 1.801, 0.8, 0.07, n=10)
    for k in range(5):
        a = math.pi / 2 + 2 * math.pi * k / 5
        d = (math.cos(a), 0.0, math.sin(a))
        crystal(g, (0.95 * d[0], 0.06, 0.95 * d[2]), (0.15 * d[0], 1.0, 0.15 * d[2]), 1.0, 0.11,
                n=4, tip=0.4)
    return g


def wall_turret_head():
    g = L.Geo()
    # The turntable.
    P.lathe(g, (0, -0.6, 0), Y, [(0, 1.0), (0.3, 0.9)], n=10, cap0="body")
    ring(g, -0.299, 0.7, 0.05, n=10)
    # Carapace pods either side, along Z.
    for s in (-1, 1):
        P.lathe(g, (s * 0.95, 0.05, 1.2), NZ,
                [(0, 0.12), (0.4, 0.38), (1.4, 0.4), (2.1, 0.22), (2.3, 0.05)], n=6)
        P.accent(g, [(s * 1.37, 0.05, 0.6), (s * 1.37, 0.05, -0.6)], (s, 0, 0), 0.05, glow=0.04)
        # A spine on top.
        crystal(g, (s * 0.95, 0.38, 0.2), (0, 1.0, 0.25), 0.42, 0.1, n=4, tip=0.5)
    # The barrels.
    for s in (-1, 1):
        P.tube(g, (s * 0.55, 0.0, -0.6), (s * 0.55, 0.0, -2.6), 0.1, n=6)
        P.tube(g, (s * 0.55, 0.0, -2.25), (s * 0.55, 0.0, -2.6), 0.14, n=6, mat="accent")
    # The yoke between the pods and the barrels, and the gunner's back shield.
    L.box(g, (-0.95, -0.3, -0.8), (0.95, 0.1, -0.2))
    L.box(g, (-0.6, -0.3, 1.0), (0.6, 0.45, 1.3))
    P.accent(g, [(-0.45, 0.3, 0.998), (0.45, 0.3, 0.998)], NZ, 0.04, glow=0.03)
    return g


# ---------------------------------------------------------------------------------------------
# The capture pylon.

def capture_pylon():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.75), (0.2, 0.6), (0.3, 0.45)], n=6, phase=0.0, cap0=None)
    # Three twisted shards round a core crystal.
    crystal(g, (0, 0.3, 0), Y, 3.7, 0.3, n=6, tip=0.25)
    for k in range(3):
        a = math.pi / 2 + 2 * math.pi * k / 3
        base = (0.42 * math.cos(a), 0.2, 0.42 * math.sin(a))
        crystal(g, base, (-0.12 * math.cos(a), 1.0, -0.12 * math.sin(a)), 2.6, 0.14, n=4,
                tip=0.3, twist=a)
    ring(g, 3.0, 0.6, 0.06, n=12)
    P.white(g, P.circle((0, 0.31, 0), Y, 0.5, 6, phase=0.0), Y, 0.04, closed=True, glow=0.03)
    return g


def main():
    L.reset_scene()
    P.materials()
    props = [("alien_terminal_equipment", terminal("equipment")),
             ("alien_terminal_medical", terminal("medical")),
             ("alien_terminal_vehicle", terminal("vehicle")),
             ("alien_console_core", console_core()),
             ("alien_generator", generator()),
             ("alien_generator_core", generator_core()),
             ("alien_vehicle_pad", vehicle_pad()),
             ("alien_rearm_pad", rearm_pad()),
             ("alien_spawn_tube", spawn_tube()),
             ("alien_resource_silo", resource_silo()),
             ("alien_resource_silo_fill", resource_silo_fill()),
             ("alien_wall_turret_base", wall_turret_base()),
             ("alien_wall_turret_head", wall_turret_head()),
             ("alien_capture_pylon", capture_pylon())]
    smooth = {"alien_resource_silo_fill": 30}
    for name, g in props:
        if g.tris() > LIMIT:
            raise SystemExit("%s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
        L.make_object(name, g, smooth=smooth.get(name))
        print("prop %-26s %5d tris" % (name, g.tris()))
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/alien.blend")
    L.export_glb("assets/models/alien.glb")


main()
