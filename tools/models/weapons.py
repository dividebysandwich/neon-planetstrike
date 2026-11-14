"""Builds the held infantry weapons: assets-src/blend/weapons.blend and assets/models/weapons.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/weapons.py

All in the blaster's frame: barrel along -Z, up +Y, origin at the middle of the grip hold. Every
weapon uses the blaster's grip (tilted back 22 degrees, 0.03 x 0.044 m section) and trigger
guard, so the shared `hand_grip_*` gloves fit; an empty `<name>_muzzle` marks the barrel tip.

| mesh      | length | muzzle node at          | notes |
|-----------|--------|-------------------------|-------|
| `rifle`   | 0.70 m | (0, 0.085, -0.43)       | stock, magazine, handguard, rail |
| `scatter` | 0.60 m | (0, 0.088, -0.38)       | flared rectangular muzzle 0.12 m wide, pump |
| `lance`   | 1.00 m | (0, 0.085, -0.68)       | long barrel, scope with team lenses |
| `striker` | 1.10 m | (0, 0.15, -0.62)        | shoulder rocket tube over the grip, fore grip |
| `pistol`  | 0.25 m | (0, 0.085, -0.19)       | compact |
| `medgun`  | 0.44 m | (0, 0.09, -0.29)        | emitter dish, white vial and lights |
| `repair_gun` | 0.55 m | (0, 0.09, -0.37)     | chunky tool: tapered nozzle with cooling rings, two arc prongs, white-windowed canister on top |
| `skyhook` | 1.05 m | (0, 0.15, -0.6)        | boxy anti-air tube, seeker lens, sensor fin |
| `pounder` | 0.82 m | (0, 0.135, -0.6)       | MAX anti-infantry: twin six-barrel rotary guns at x = +-0.085, ammo drum on top |
| `mauler`  | 0.82 m | (0, 0.14, -0.62)       | MAX anti-armour: twin short cannons at x = +-0.085 in recoil sleeves, fat slotted muzzle brakes, shell hopper |
| `burster` | 1.22 m | (0, 0.21, -1.0)        | MAX anti-air: twin thin flak barrels at x = +-0.07 rising from y 0.13 (z -0.2) to 0.21, ribbed cooling jackets, box magazine on top |

The MAX weapons' muzzle nodes sit midway between their two barrels' tips.

`max_shell` is not held: it is the MAX power armour worn over the avatar's `torso` (parented to
it with an identity transform), so it is authored in the torso's frame (origin at the neck
point, hanging down -Y, front -Z). A 2.5 cm thick octagonal hull (y 0.03 .. -0.49, open at the
collar and hem) clears the torso by 2.8-5 cm all round (`SHELL_IN`), with a raised chest plate
and two belly plates in front, a back pack with two exhaust stacks (out to z = 0.31), and stepped
pauldrons over the shoulders reaching x = +-0.4; team lines on the plate edges, collar and hem.

Materials: `body`, `accent` (team lines and muzzle rings), `glow` (small halos),
`glow_white`/`halo_white` (the medgun's and repair gun's white details, muzzle emitters).
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, X, Y, Z, NX, NY, NZ  # noqa: E402

GRIP_TILT = math.radians(22)
LIMIT = 2000
NODES = []


def grip(g):
    """The blaster's grip and trigger guard (identical, so the gripping gloves fit)."""
    a = (0, math.cos(GRIP_TILT), -math.sin(GRIP_TILT))
    d = (0, math.sin(GRIP_TILT), math.cos(GRIP_TILT))

    def section(t, hx=0.015, hd=0.022, ch=0.006):
        c = L.mul(a, t)
        pts = [(hx, hd - ch), (hx - ch, hd), (-hx + ch, hd), (-hx, hd - ch),
               (-hx, -hd + ch), (-hx + ch, -hd), (hx - ch, -hd), (hx, -hd + ch)]
        return [L.add(c, L.add((x, 0, 0), L.mul(d, dd))) for x, dd in pts]

    L.loft(g, [section(-0.074, 0.017, 0.025), section(-0.066, 0.017, 0.025),
               section(-0.064, 0.015, 0.022), section(0.066)], a, cap0="body", cap1="body")
    for s in (-1, 1):
        f = L.Frame((s * 0.015, 0, 0), L.mul(d, -s), a)
        L.line(g, f, (0, -0.05), (0, 0.045), 0.005, glow=0, h=0.0015)
    L.band_x(g, [(-0.08, 0.052), (-0.082, 0.024), (-0.072, 0.004), (-0.05, -0.004),
                 (-0.02, -0.006)], 0.005, -0.0045, 0.0045)
    L.band_x(g, [(-0.056, 0.052), (-0.059, 0.036), (-0.055, 0.022), (-0.048, 0.014)], 0.006,
             -0.0035, 0.0035)


def side_line(g, xh, pts_zy, w=0.005):
    """Team line on both side planes x = +-xh through (z, y) points."""
    for s in (-1, 1):
        P.accent(g, [(s * xh, y, z) for z, y in pts_zy], (s, 0, 0), w, glow=0.0, h=0.0015)


def top_line(g, y, x, z0, z1, w=0.005):
    P.accent(g, [(x, y, z0), (x, y, z1)], Y, w, glow=0.0, h=0.0015)


def barrel(g, z0, yc, prof, n=8, ring_at=(), white_tip=False):
    """Lathe along -Z from (0, yc, z0); prof [(t, r)] with a muzzle cap at the end."""
    P.lathe(g, (0, yc, z0), NZ, prof, n=n, cap0="body", cap1=LIGHT if white_tip else "accent")
    for t, r in ring_at:
        P.lathe(g, (0, yc, z0), NZ, [(t, r), (t + 0.006, r)], n=n, mats="accent", cap0=None,
                cap1=None)


def muzzle(name, pos):
    NODES.append((name + "_muzzle", pos))


# ---------------------------------------------------------------------------------------------

def rifle():
    g = L.Geo()
    grip(g)
    xh = 0.022
    L.prism_x(g, [(-0.14, 0.05), (0.1, 0.05), (0.12, 0.07), (0.12, 0.112), (-0.14, 0.112)],
              -xh, xh)  # receiver
    L.prism_x(g, [(0.1, 0.06), (0.27, 0.0), (0.27, 0.105), (0.12, 0.112)], -0.017, 0.017)  # stock
    L.prism_x(g, [(-0.33, 0.06), (-0.14, 0.055), (-0.14, 0.112), (-0.33, 0.108)], -0.02, 0.02)
    L.prism_x(g, [(-0.16, 0.055), (-0.115, 0.055), (-0.125, -0.06), (-0.17, -0.055)], -0.013,
              0.013)  # magazine
    L.box(g, (-0.008, 0.112, -0.3), (0.008, 0.122, 0.08))  # rail
    for z in (-0.25, -0.15, -0.05, 0.05):
        L.box(g, (-0.009, 0.122, z), (0.009, 0.126, z + 0.012), mat="accent")
    barrel(g, -0.33, 0.085, [(0, 0.013), (0.07, 0.012), (0.07, 0.018), (0.1, 0.018)],
           ring_at=((0.03, 0.0135),))
    side_line(g, xh, [(-0.13, 0.095), (0.09, 0.095)])
    side_line(g, 0.02, [(-0.32, 0.084), (-0.15, 0.084)], 0.004)
    side_line(g, 0.017, [(0.12, 0.08), (0.25, 0.04)], 0.004)
    muzzle("rifle", (0, 0.085, -0.43))
    return g


def scatter():
    g = L.Geo()
    grip(g)
    xh = 0.026
    L.prism_x(g, [(-0.1, 0.05), (0.09, 0.05), (0.1, 0.065), (0.1, 0.12), (-0.1, 0.12)], -xh, xh)
    L.prism_x(g, [(0.09, 0.055), (0.22, 0.01), (0.22, 0.1), (0.1, 0.12)], -0.018, 0.018)

    def rs(z, p):
        hx, y0, y1 = p
        return [(-hx, y0, z), (hx, y0, z), (hx, y1, z), (-hx, y1, z)]
    rings = [rs(z, p) for z, p in ((-0.1, (0.026, 0.06, 0.116)), (-0.3, (0.028, 0.062, 0.114)),
                                   (-0.38, (0.06, 0.05, 0.126)))]
    L.loft(g, rings, NZ, cap0="body", cap1="body")
    # Muzzle face: three wide team slots.
    for y in (0.068, 0.088, 0.108):
        P.disc(g, [(-0.048, y - 0.005, -0.38), (0.048, y - 0.005, -0.38),
                   (0.048, y + 0.005, -0.38), (-0.048, y + 0.005, -0.38)], NZ, glow=0.0,
               h=0.0015)
    # Pump.
    L.box(g, (-0.022, 0.03, -0.27), (0.022, 0.06, -0.14))
    for z in (-0.25, -0.21, -0.17):
        L.box(g, (-0.023, 0.032, z), (0.023, 0.058, z + 0.01), mat="accent")
    side_line(g, xh, [(-0.09, 0.1), (0.09, 0.1)])
    for s in (-1, 1):
        q = (rings[1][1], rings[1][2], rings[2][2], rings[2][1]) if s > 0 else \
            (rings[1][3], rings[1][0], rings[2][0], rings[2][3])
        n = P.quad_normal(q, (s, 0, -0.3))
        P.accent(g, [P.on_quad(*q, 0.5, 0.1), P.on_quad(*q, 0.5, 0.9)], n, 0.006, glow=0.0,
                 h=0.0015)
    top_line(g, 0.12, 0, -0.09, 0.08)
    muzzle("scatter", (0, 0.088, -0.38))
    return g


def lance():
    g = L.Geo()
    grip(g)
    xh = 0.02
    L.prism_x(g, [(-0.18, 0.05), (0.12, 0.05), (0.13, 0.07), (0.13, 0.11), (-0.18, 0.11)],
              -xh, xh)
    # Skeleton stock: upper bar, butt plate, lower strut.
    L.box(g, (-0.012, 0.09, 0.12), (0.012, 0.108, 0.3))
    L.box(g, (-0.016, 0.0, 0.29), (0.016, 0.115, 0.32))
    L.band_x(g, [(0.12, 0.055), (0.29, 0.01)], 0.014, -0.01, 0.01)
    L.prism_x(g, [(-0.4, 0.062), (-0.18, 0.058), (-0.18, 0.108), (-0.4, 0.104)], -0.018, 0.018)
    L.prism_x(g, [(-0.12, 0.05), (-0.09, 0.05), (-0.095, -0.02), (-0.125, -0.02)], -0.012,
              0.012)  # magazine
    barrel(g, -0.4, 0.085, [(0, 0.011), (0.25, 0.01), (0.25, 0.015), (0.28, 0.015)],
           ring_at=((0.1, 0.0115), (0.18, 0.0112)))
    # Scope.
    for z in (-0.12, 0.0):
        L.box(g, (-0.006, 0.11, z - 0.008), (0.006, 0.138, z + 0.008))
    sy = 0.15
    P.lathe(g, (0, sy, 0.07), NZ, [(0, 0.016), (0.03, 0.016), (0.05, 0.013), (0.2, 0.013),
                                    (0.23, 0.022), (0.28, 0.022)], n=8, cap0="accent",
            cap1="accent")
    P.lathe(g, (0, sy, 0.07), NZ, [(0.1, 0.0137), (0.108, 0.0137)], n=8, mats="accent",
            cap0=None, cap1=None)
    side_line(g, xh, [(-0.17, 0.095), (0.12, 0.095)])
    side_line(g, 0.018, [(-0.39, 0.075), (-0.19, 0.075)], 0.004)
    muzzle("lance", (0, 0.085, -0.68))
    return g


def striker():
    g = L.Geo()
    grip(g)
    yc = 0.15
    L.box(g, (-0.02, 0.05, -0.12), (0.02, 0.085, 0.06))  # mount between grip and tube
    P.lathe(g, (0, yc, 0.48), NZ, [(0, 0.06), (0.03, 0.09), (0.1, 0.09), (0.12, 0.072),
                                    (0.98, 0.072), (1.0, 0.088), (1.1, 0.088), (1.1, 0.06),
                                    (1.06, 0.06)], n=10, cap0="body", cap1=LIGHT)
    for t in (0.05, 0.3, 1.04):
        P.lathe(g, (0, yc, 0.48), NZ, [(t, 0.092 if t < 0.1 or t > 1 else 0.075),
                                        (t + 0.02, 0.092 if t < 0.1 or t > 1 else 0.075)],
                n=10, mats="accent", cap0=None, cap1=None)
    # Fore grip.
    L.band_x(g, [(-0.3, 0.09), (-0.32, -0.02)], 0.03, -0.014, 0.014)
    # Sight box on the left.
    L.box(g, (-0.11, 0.17, -0.2), (-0.065, 0.215, -0.08))
    P.disc(g, [(-0.105, 0.177, -0.2), (-0.07, 0.177, -0.2), (-0.07, 0.208, -0.2),
               (-0.105, 0.208, -0.2)], NZ, glow=0.0, h=0.0015)
    # Team line along the tube's top flat.
    yt = yc + 0.072 * math.cos(math.pi / 10)
    P.accent(g, [(0, yt, 0.32), (0, yt, -0.45)], Y, 0.008, glow=0.0, h=0.0015)
    muzzle("striker", (0, yc, -0.62))
    return g


def skyhook():
    """The anti-air launcher: a boxy tube with a seeker lens ahead and a sensor fin on top."""
    g = L.Geo()
    grip(g)
    yc = 0.15
    L.box(g, (-0.02, 0.05, -0.12), (0.02, 0.085, 0.06))
    L.box(g, (-0.085, yc - 0.075, -0.55), (0.085, yc + 0.075, 0.45))
    # Seeker: a ring and a white lens at the mouth.
    P.lathe(g, (0, yc, -0.55), NZ, [(0, 0.07), (0.05, 0.07)], n=10, cap0=None, cap1=LIGHT)
    P.accent(g, P.circle((0, yc, -0.6), NZ, 0.075, 10), NZ, 0.008, closed=True, glow=0.0,
             h=0.0015)
    # Team lines along both sides, and round the back.
    for s in (-1, 1):
        P.accent(g, [(s * 0.085, yc, 0.4), (s * 0.085, yc, -0.5)], (s, 0, 0), 0.01, glow=0.0,
                 h=0.0015)
    P.accent(g, [(-0.07, yc - 0.06, 0.45), (0.07, yc - 0.06, 0.45), (0.07, yc + 0.06, 0.45),
                 (-0.07, yc + 0.06, 0.45)], Z, 0.008, closed=True, glow=0.0, h=0.0015)
    # Sensor fin on top.
    P.extrude(g, [(0, yc + 0.075, -0.3), (0, yc + 0.075, -0.05), (0, yc + 0.13, -0.08),
                  (0, yc + 0.13, -0.24)], (0.01, 0, 0))
    P.white(g, [(0.011, yc + 0.11, -0.22), (0.011, yc + 0.11, -0.1)], X, 0.008, glow=0.0)
    # Fore grip.
    L.band_x(g, [(-0.3, 0.075), (-0.32, -0.02)], 0.03, -0.014, 0.014)
    muzzle("skyhook", (0, yc, -0.6))
    return g


def pistol():
    g = L.Geo()
    grip(g)
    xh = 0.016
    L.prism_x(g, [(-0.15, 0.055), (0.045, 0.05), (0.055, 0.07), (0.055, 0.108), (-0.14, 0.108),
                  (-0.15, 0.1)], -xh, xh)
    barrel(g, -0.15, 0.085, [(0, 0.012), (0.04, 0.012)], ring_at=())
    side_line(g, xh, [(-0.14, 0.095), (0.04, 0.095)], 0.004)
    top_line(g, 0.108, 0, -0.13, 0.03, 0.004)
    muzzle("pistol", (0, 0.085, -0.19))
    return g


def medgun():
    g = L.Geo()
    grip(g)
    yc = 0.09
    P.lathe(g, (0, yc, 0.15), NZ, [(0, 0.02), (0.03, 0.038), (0.3, 0.038), (0.34, 0.022),
                                    (0.37, 0.022)], n=8)
    L.box(g, (-0.022, 0.045, -0.1), (0.022, 0.06, 0.08))
    # Emitter dish: a cone opening forward, white core and ring.
    o = (0, yc, -0.22)
    P.lathe(g, o, NZ, [(0, 0.024), (0.06, 0.07), (0.07, 0.072)], n=10, cap0=None, cap1=None)
    fr, bk = (P.circle(L.add(o, (0, 0, -t)), NZ, r, 10) for t, r in ((0.07, 0.066), (0.03, 0.03)))
    for i in range(10):  # the dish's inside faces forward and in
        j = (i + 1) % 10
        q = [bk[i], bk[j], fr[j], fr[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, LIGHT, want=L.add((-mid[0], yc - mid[1], 0), (0, 0, -0.05)))
    g.face(bk, LIGHT, want=NZ)
    rim = P.circle(L.add(o, (0, 0, -0.07)), NZ, 0.072, 10)
    rin = P.circle(L.add(o, (0, 0, -0.07)), NZ, 0.066, 10)
    for i in range(10):
        j = (i + 1) % 10
        g.face([rin[i], rin[j], rim[j], rim[i]], "accent", want=NZ)
    # White vial on top, team and white lines.
    L.box(g, (-0.01, 0.125, -0.02), (0.01, 0.135, 0.1))
    P.lathe(g, (0, 0.152, 0.11), NZ, [(0, 0.018), (0.14, 0.018)], n=8, mats=LIGHT, cap0="body",
            cap1="body")
    for z in (0.11, -0.03):
        P.lathe(g, (0, 0.152, z), NZ, [(0, 0.021), (0.012, 0.021)], n=8)
    side = 0.038 * math.cos(math.pi / 8)
    for s in (-1, 1):
        P.accent(g, [(s * side, yc, 0.1), (s * side, yc, -0.15)], (s, 0, 0), 0.006, glow=0.0,
                 h=0.0015)
        P.white(g, [(s * side, yc - 0.018, 0.05), (s * side, yc - 0.018, -0.1)], (s, 0, 0),
                0.004, glow=0.004, h=0.0015)
    muzzle("medgun", (0, yc, -0.29))
    return g


def repair_gun():
    g = L.Geo()
    grip(g)
    yc = 0.09
    xh = 0.032
    L.prism_x(g, [(-0.17, 0.05), (0.14, 0.05), (0.18, 0.075), (0.18, 0.13), (0.15, 0.14),
                  (-0.17, 0.14)], -xh, xh)
    # Canister on top: white fluid window between body end caps, on two clamps.
    P.lathe(g, (0, 0.175, 0.16), NZ, [(0, 0.026), (0.02, 0.034), (0.05, 0.034), (0.15, 0.034),
                                       (0.18, 0.034), (0.2, 0.026)], n=8,
            mats=["body", "body", LIGHT, "body", "body"], cap0="body", cap1="body")
    for z in (0.1, -0.01):
        L.box(g, (-0.012, 0.135, z - 0.012), (0.012, 0.15, z + 0.012))
        P.lathe(g, (0, 0.175, z + 0.012), NZ, [(0, 0.037), (0.024, 0.037)], n=8, mats="accent",
                cap0=None, cap1=None)
    # Nozzle: tapering along -Z with cooling rings, white bore.
    o = (0, yc, -0.17)
    P.lathe(g, o, NZ, [(0, 0.042), (0.06, 0.042), (0.1, 0.03), (0.14, 0.022), (0.16, 0.018)],
            n=8, cap0="body", cap1=LIGHT)
    for t in (0.012, 0.03, 0.048):
        P.lathe(g, o, NZ, [(t, 0.046), (t + 0.008, 0.046)], n=8, mats="accent", cap0=None,
                cap1=None)
    # Arc prongs either side of the bore, white tips.
    for s in (-1, 1):
        P.tube(g, (s * 0.026, yc, -0.29), (s * 0.02, yc, -0.355), 0.006, n=4)
        P.tube(g, (s * 0.02, yc, -0.355), (s * 0.018, yc, -0.37), 0.007, n=4, mat=LIGHT)
        L.box(g, *sorted([(s * 0.018, yc - 0.012, -0.3), (s * 0.034, yc + 0.012, -0.27)],
                         key=lambda p: p[0]))
    side_line(g, xh, [(-0.16, 0.125), (0.14, 0.125)])
    side_line(g, xh, [(-0.16, 0.065), (-0.06, 0.065)], 0.004)
    top_line(g, 0.14, 0, -0.16, -0.05)
    muzzle("repair_gun", (0, yc, -0.37))
    return g


# ---------------------------------------------------------------------------------------------
# MAX exosuit arm weapons: heavy, twin-barrelled, the same grip (the suit's glove holds it).

TWIN_X = 0.085  # the barrels' half spacing


def heavy_receiver(g, xh, z0, z1, y1, rear_drop=0.04):
    """The bulky receiver over the grip: a chamfered block from z0 (front) to z1 (back), its
    underside at the grip's top (y = 0.05), top at y1, sloping down at the back."""
    L.prism_x(g, [(z0, 0.05), (z1 - 0.05, 0.05), (z1, 0.05 + rear_drop + 0.03),
                  (z1, y1 - 0.03), (z1 - 0.05, y1), (z0 + 0.03, y1), (z0, y1 - 0.03)], -xh, xh)


def pounder():
    """Anti-infantry: twin six-barrel rotary chainguns side by side, a motor housing each, an
    ammo drum on top."""
    g = L.Geo()
    grip(g)
    xh, yc = 0.062, 0.135
    heavy_receiver(g, xh, -0.25, 0.22, 0.225)
    # Feed chutes to the barrel housings.
    for s in (-1, 1):
        x0, x1 = sorted((s * xh, s * (TWIN_X + 0.02)))
        L.box(g, (x0, 0.09, -0.24), (x1, 0.19, -0.1))
    for s in (-1, 1):
        cx = s * TWIN_X
        # Motor housing, accent rings.
        P.lathe(g, (cx, yc, -0.08), NZ, [(0, 0.045), (0.02, 0.058), (0.2, 0.058), (0.23, 0.048)],
                n=8, cap0="body", cap1="body")
        for t in (0.05, 0.17):
            P.lathe(g, (cx, yc, -0.08), NZ, [(t, 0.061), (t + 0.012, 0.061)], n=8,
                    mats="accent", cap0=None, cap1=None)
        # Six barrels round the spindle, two clamps, white bore lights at the spindle tip.
        P.lathe(g, (cx, yc, -0.31), NZ, [(0, 0.012), (0.29, 0.012)], n=6, cap0=None, cap1=LIGHT)
        for k in range(6):
            a = 2 * math.pi * k / 6 + math.pi / 6
            o = (cx + 0.03 * math.cos(a), yc + 0.03 * math.sin(a))
            P.tube(g, (o[0], o[1], -0.3), (o[0], o[1], -0.6), 0.0095, n=5)
        for t0, t1, r, mat in ((-0.42, -0.45, 0.047, "body"), (-0.565, -0.59, 0.047, "body")):
            P.lathe(g, (cx, yc, t0), NZ, [(0, r), (t0 - t1, r)], n=8, cap0="body", cap1="body")
        P.lathe(g, (cx, yc, -0.59), NZ, [(0, 0.049), (0.006, 0.049)], n=8, mats="accent",
                cap0=None, cap1=None)
    # Ammo drum on top, across X, with team end rings.
    P.lathe(g, (-0.06, 0.28, 0.06), X, [(0, 0.06), (0.12, 0.06)], n=10, cap0="body", cap1="body")
    for x in (-0.066, 0.054):
        P.lathe(g, (x, 0.28, 0.06), X, [(0, 0.064), (0.012, 0.064)], n=10, mats="accent",
                cap0=None, cap1=None)
    L.box(g, (-0.05, 0.22, 0.0), (0.05, 0.245, 0.12))
    side_line(g, xh, [(-0.22, 0.2), (0.15, 0.2), (0.2, 0.17)], 0.008)
    side_line(g, xh, [(-0.22, 0.07), (0.1, 0.07)], 0.005)
    P.white(g, [(-0.04, 0.226, -0.2), (0.04, 0.226, -0.2)], Y, 0.01, glow=0.0, h=0.0015)
    muzzle("pounder", (0, yc, -0.6))
    return g


def mauler():
    """Anti-armour: twin short heavy cannons in recoil sleeves with fat slotted muzzle brakes."""
    g = L.Geo()
    grip(g)
    xh, yc = 0.075, 0.14
    heavy_receiver(g, xh, -0.22, 0.2, 0.25, rear_drop=0.06)
    # Armoured cheek plates either side of the receiver, carrying the cannons.
    for s in (-1, 1):
        x0, x1 = sorted((s * xh, s * 0.14))
        L.prism_x(g, [(-0.24, 0.07), (0.08, 0.07), (0.12, 0.11), (0.12, 0.2), (0.06, 0.225),
                      (-0.24, 0.225)], x0, x1)
        P.accent(g, [(s * 0.14, 0.2, -0.22), (s * 0.14, 0.2, 0.05), (s * 0.14, 0.17, 0.1)],
                 (s, 0, 0), 0.01, glow=0.0, h=0.0015)
        P.accent(g, [(s * 0.14, 0.09, -0.22), (s * 0.14, 0.09, 0.06)], (s, 0, 0), 0.006,
                 glow=0.0, h=0.0015)
    for s in (-1, 1):
        cx = s * TWIN_X
        o = (cx, yc, -0.24)
        # Recoil sleeve, barrel, brake (two fat baffles with a vented gap), white bore.
        P.lathe(g, o, NZ, [(0, 0.058), (0.1, 0.058), (0.12, 0.046)], n=8, cap0=None, cap1="body")
        P.lathe(g, o, NZ, [(0.12, 0.036), (0.27, 0.034)], n=8, cap0=None, cap1=None)
        P.lathe(g, o, NZ, [(0.27, 0.05), (0.29, 0.066), (0.32, 0.066), (0.32, 0.05),
                           (0.335, 0.05), (0.335, 0.066), (0.365, 0.066), (0.38, 0.056)],
                n=8, cap0="body", cap1="body")
        P.lathe(g, (cx, yc, -0.623), Z, [(0, 0.024), (0.004, 0.024)], n=8, cap0=LIGHT, cap1=None)
        for t in (0.04, 0.08):
            P.lathe(g, o, NZ, [(t, 0.061), (t + 0.01, 0.061)], n=8, mats="accent", cap0=None,
                    cap1=None)
        P.lathe(g, o, NZ, [(0.3, 0.069), (0.31, 0.069)], n=8, mats="accent", cap0=None,
                cap1=None)
    # Big shell hopper on the back of the receiver, team chevron on top.
    L.prism_x(g, [(0.02, 0.25), (0.2, 0.25), (0.2, 0.3), (0.16, 0.32), (0.04, 0.32)],
              -0.065, 0.065)
    P.accent(g, [(-0.05, 0.25, -0.05), (0, 0.25, -0.12), (0.05, 0.25, -0.05)], Y, 0.012,
             glow=0.0, h=0.0015)
    P.white(g, [(-0.045, 0.32, 0.08), (0.045, 0.32, 0.08)], Y, 0.01, glow=0.0, h=0.0015)
    muzzle("mauler", (0, yc, -0.62))
    return g


BURST_TIP = (0.21, -1.0)  # (y, z) of the flak barrels' muzzles
BURST_BASE = (0.13, -0.2)


def burster():
    """Anti-air: twin long thin flak barrels angled slightly up, cooling jackets at their
    base, a box magazine on top."""
    g = L.Geo()
    grip(g)
    xh = 0.06
    heavy_receiver(g, xh, -0.22, 0.22, 0.22)
    for s in (-1, 1):
        x0, x1 = sorted((s * xh, s * 0.11))
        L.prism_x(g, [(-0.26, 0.08), (0.06, 0.08), (0.06, 0.19), (-0.26, 0.19)], x0, x1)
        P.accent(g, [(s * 0.11, 0.165, -0.24), (s * 0.11, 0.165, 0.04)], (s, 0, 0), 0.008,
                 glow=0.0, h=0.0015)
    (yb, zb), (yt, zt) = BURST_BASE, BURST_TIP
    ax = L.norm((0, yt - yb, zt - zb))
    ln = L.length((0, yt - yb, zt - zb))
    for s in (-1, 1):
        o = (s * 0.07, yb, zb)
        # Cooling jacket (ribbed), thin barrel, flared flash hider with a white tip.
        P.lathe(g, L.add(o, (0, 0, 0.06)), ax, [(0, 0.04), (0.06, 0.04), (0.26, 0.034),
                                                (0.29, 0.024)], n=8, cap0="body", cap1="body")
        for t in (0.1, 0.15, 0.2, 0.25):
            P.lathe(g, L.add(o, (0, 0, 0.06)), ax, [(t, 0.04), (t + 0.012, 0.04)], n=8,
                    mats="accent", cap0=None, cap1=None)
        P.lathe(g, o, ax, [(0.2, 0.016), (ln - 0.08, 0.016)], n=6, cap0=None, cap1=None)
        P.lathe(g, o, ax, [(ln - 0.1, 0.018), (ln - 0.08, 0.024), (ln - 0.01, 0.026),
                           (ln, 0.02)], n=6, cap0="body", cap1=LIGHT)
        P.lathe(g, o, ax, [(ln - 0.06, 0.0275), (ln - 0.05, 0.0275)], n=6, mats="accent",
                cap0=None, cap1=None)
        # A brace between the barrels halfway.
    mid = L.add((0, yb, zb), L.mul(ax, ln * 0.55))
    L.box(g, (-0.07, mid[1] - 0.01, mid[2] - 0.015), (0.07, mid[1] + 0.01, mid[2] + 0.015))
    # Box magazine on top: team bands, white round-counter window.
    L.box(g, (-0.07, 0.22, -0.13), (0.07, 0.37, 0.11))
    L.box(g, (-0.074, 0.29, -0.135), (0.074, 0.305, 0.115), mat="accent")
    for s in (-1, 1):
        P.white(g, [(s * 0.07, 0.33, -0.1), (s * 0.07, 0.33, 0.08)], (s, 0, 0), 0.012,
                glow=0.0, h=0.0015)
    P.accent(g, [(-0.05, 0.37, -0.11), (0.05, 0.37, -0.11), (0.05, 0.37, 0.09),
                 (-0.05, 0.37, 0.09)], Y, 0.008, closed=True, glow=0.0, h=0.0015)
    muzzle("burster", (0, yt, zt))
    return g


# ---------------------------------------------------------------------------------------------
# MAX power armour shell, worn over the avatar's torso (in the `torso` mesh's frame: origin at
# its top centre / neck point, hanging down -Y, front -Z).

# Inner surface (y, half width, half depth, chamfer): the torso grown by 3.5-5 cm (2.8 cm at the
# corners of its shoulder plates), down to 1 cm above its hem.
SHELL_IN = [(0.03, 0.13, 0.1, 0.04), (-0.01, 0.2, 0.15, 0.05), (-0.05, 0.26, 0.16, 0.04),
            (-0.15, 0.26, 0.16, 0.04), (-0.4, 0.21, 0.16, 0.05), (-0.49, 0.185, 0.145, 0.05)]
SHELL_T = 0.025  # plate thickness


def shell_sec(y, grow=0.0):
    for (ya, ax_, az, ac), (yb, bx, bz, bc) in zip(SHELL_IN, SHELL_IN[1:]):
        if yb <= y <= ya:
            t = (ya - y) / (ya - yb)
            return [(x, y, z) for x, z in L.octagon_xz(ax_ + (bx - ax_) * t + grow,
                                                       az + (bz - az) * t + grow,
                                                       ac + (bc - ac) * t)]
    raise ValueError(y)


def band_z(g, pts, thick, z0, z1, mat="body"):
    """A bar of in-plane thickness `thick` following an (x, y) polyline, spanning z0..z1."""
    Lp = L.offset_line(pts, thick / 2, False)
    Rp = L.offset_line(pts, -thick / 2, False)
    Q = lambda q, z: (q[0], q[1], z)
    for i in range(len(pts) - 1):
        j = i + 1
        d = L._n2((pts[j][0] - pts[i][0], pts[j][1] - pts[i][1]))
        lf = L._left(d)
        ln = (lf[0], lf[1], 0)
        g.face([Q(Lp[i], z0), Q(Lp[j], z0), Q(Lp[j], z1), Q(Lp[i], z1)], mat, want=ln)
        g.face([Q(Rp[i], z0), Q(Rp[j], z0), Q(Rp[j], z1), Q(Rp[i], z1)], mat, want=L.mul(ln, -1))
        g.face([Q(Lp[i], z0), Q(Lp[j], z0), Q(Rp[j], z0), Q(Rp[i], z0)], mat, want=NZ)
        g.face([Q(Lp[i], z1), Q(Lp[j], z1), Q(Rp[j], z1), Q(Rp[i], z1)], mat, want=Z)
    for a, b, o in ((0, 1, 0), (-1, -2, -1)):
        d = L._n2((pts[a][0] - pts[b][0], pts[a][1] - pts[b][1]))
        g.face([Q(Lp[o], z0), Q(Rp[o], z0), Q(Rp[o], z1), Q(Lp[o], z1)], mat,
               want=(d[0], d[1], 0))


def max_shell():
    g = L.Geo()
    # The plated hull: outer and inner surfaces joined at the collar and the hem.
    ys = [s[0] for s in SHELL_IN]
    inner = [shell_sec(y) for y in ys]
    outer = [shell_sec(y, SHELL_T) for y in ys]
    n = len(inner[0])
    for rings, sgn in ((outer, 1), (inner, -1)):
        for k in range(len(rings) - 1):
            A, B = rings[k], rings[k + 1]
            for i in range(n):
                j = (i + 1) % n
                q = [A[i], A[j], B[j], B[i]]
                m = L.mul(tuple(map(sum, zip(*q))), 0.25)
                g.face(q, "body", want=(sgn * m[0], 0, sgn * m[2]))
    for k, up in ((0, Y), (-1, NY)):
        for i in range(n):
            j = (i + 1) % n
            g.face([outer[k][i], outer[k][j], inner[k][j], inner[k][i]], "body", want=up)
    # Team bands round the collar and the hem.
    for y0, y1 in ((0.022, 0.008), (-0.465, -0.477)):
        L.loft(g, [shell_sec(y0, SHELL_T + 0.003), shell_sec(y1, SHELL_T + 0.003)], NY,
               mats="accent", cap0=None, cap1=None)
    zf = -(0.16 + SHELL_T)  # the planar front (and back) between y = -0.05 and -0.4
    # Chest plate: a raised faceted slab, team outline round its edge.
    chest = [(-0.2, -0.065), (0.2, -0.065), (0.215, -0.1), (0.2, -0.25), (0.13, -0.315),
             (-0.13, -0.315), (-0.2, -0.25), (-0.215, -0.1)]
    P.extrude(g, [(x, y, zf) for x, y in chest], (0, 0, -0.03))
    inset = [(-0.185, -0.08), (0.185, -0.08), (0.198, -0.105), (0.185, -0.242),
             (0.122, -0.3), (-0.122, -0.3), (-0.185, -0.242), (-0.198, -0.105)]
    P.accent(g, [(x, y, zf - 0.03) for x, y in inset], NZ, 0.01, closed=True, glow=0.012,
             h=0.002)
    # A white status light in the middle of the chest, two belly plates below.
    P.disc(g, [(-0.04, -0.13, zf - 0.03), (0.04, -0.13, zf - 0.03), (0.05, -0.15, zf - 0.03),
               (0.04, -0.17, zf - 0.03), (-0.04, -0.17, zf - 0.03), (-0.05, -0.15, zf - 0.03)],
           NZ, mat=LIGHT, glow=0.015, h=0.002)
    for y0, y1, hx in ((-0.33, -0.36, 0.15), (-0.37, -0.395, 0.13)):
        P.extrude(g, [(-hx, y0, zf), (hx, y0, zf), (hx - 0.015, y1, zf),
                      (-hx + 0.015, y1, zf)], (0, 0, -0.018))
        P.accent(g, [(-hx + 0.02, (y0 + y1) / 2, zf - 0.018),
                     (hx - 0.02, (y0 + y1) / 2, zf - 0.018)], NZ, 0.006, glow=0.0, h=0.0015)
    # Back pack: power unit with two exhaust stacks, team outline and white core strip.
    zb = -zf
    pack = [(-0.17, -0.07), (0.17, -0.07), (0.19, -0.1), (0.19, -0.33), (0.15, -0.37),
            (-0.15, -0.37), (-0.19, -0.33), (-0.19, -0.1)]
    P.extrude(g, [(x, y, zb) for x, y in pack], (0, 0, 0.12))
    zp = zb + 0.12
    P.accent(g, [(x * 0.88, -0.22 + (y + 0.22) * 0.88, zp) for x, y in pack], Z, 0.01,
             closed=True, glow=0.012, h=0.002)
    P.white(g, [(0, -0.12, zp), (0, -0.31, zp)], Z, 0.02, glow=0.015, h=0.002)
    for s in (-1, 1):
        P.lathe(g, (s * 0.12, -0.08, zb + 0.07), Y, [(0, 0.04), (0.1, 0.04), (0.12, 0.034)],
                n=8, cap0="body", cap1=LIGHT)
        P.lathe(g, (s * 0.12, -0.08, zb + 0.07), Y, [(0.06, 0.043), (0.072, 0.043)], n=8,
                mats="accent", cap0=None, cap1=None)
    # Pauldrons: a heavy stepped plate over each shoulder (a convex slab, a narrower raised
    # one on its front and back faces) with an arched ridge plate on top; team lines along the
    # outer edges.
    pauld = [(0.16, 0.075), (0.26, 0.075), (0.33, 0.03), (0.365, -0.05), (0.365, -0.17),
             (0.31, -0.19), (0.24, -0.1)]
    cx_ = sum(x for x, _ in pauld) / len(pauld)
    cy_ = sum(y for _, y in pauld) / len(pauld)
    inner_p = [(cx_ + (x - cx_) * 0.78, cy_ + (y - cy_) * 0.78) for x, y in pauld]
    ridge = [(0.19, 0.1), (0.28, 0.098), (0.345, 0.05), (0.38, -0.04), (0.385, -0.12)]
    for s in (-1, 1):
        P.extrude(g, [(s * x, y, -0.13) for x, y in pauld], (0, 0, 0.26))
        P.extrude(g, [(s * x, y, -0.165) for x, y in inner_p], (0, 0, 0.33))
        r = [(s * x, y) for x, y in ridge]
        band_z(g, r if s > 0 else r[::-1], 0.03, -0.1, 0.1)
        edge = [P.lerp(pauld[k], inner_p[k], 0.5) for k in (1, 2, 3, 4, 5)]
        for zz, nz in ((-0.13, NZ), (0.13, Z)):
            P.accent(g, [(s * x, y, zz) for x, y in edge], nz, 0.008, glow=0.0, h=0.0015)
        for zz, nz in ((-0.1, NZ), (0.1, Z)):
            P.accent(g, [(s * x, y, zz) for x, y in ridge[1:]], nz, 0.006, glow=0.0, h=0.0015)
        P.white(g, [(s * 0.365, -0.1, -0.06), (s * 0.365, -0.1, 0.06)], (s, 0, 0), 0.012,
                glow=0.0, h=0.0015)
    return g


# Meshes shaded smoothly across edges bending less than this many degrees (the rest flat).
SMOOTH = {}


def main():
    L.reset_scene()
    P.materials()
    for name, fn in (("rifle", rifle), ("scatter", scatter), ("lance", lance),
                     ("striker", striker), ("pistol", pistol), ("medgun", medgun),
                     ("repair_gun", repair_gun), ("skyhook", skyhook),
                     ("pounder", pounder), ("mauler", mauler), ("burster", burster),
                     ("max_shell", max_shell)):
        g = fn()
        if g.tris() > LIMIT:
            raise SystemExit("%s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
        L.make_object(name, g, smooth=SMOOTH.get(name))
        zs = [p[2] for p in g.positions()]
        print("weapon %-8s %5d tris, z %.3f .. %.3f (%.2f m)" % (name, g.tris(), min(zs),
                                                                  max(zs), max(zs) - min(zs)))
    for name, pos in NODES:
        L.make_empty(name, pos)
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/weapons.blend")
    L.export_glb("assets/models/weapons.glb")


main()
