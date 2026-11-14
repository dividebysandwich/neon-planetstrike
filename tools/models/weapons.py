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


def osec(z, hx, y0, y1, c=0.006, x=0.0):
    """An octagonal section (a rectangle x +- hx, y0..y1 with chamfered corners) at z."""
    return [(x + hx, y0 + c, z), (x + hx, y1 - c, z), (x + hx - c, y1, z), (x - hx + c, y1, z),
            (x - hx, y1 - c, z), (x - hx, y0 + c, z), (x - hx + c, y0, z), (x + hx - c, y0, z)]


def hull(g, secs, mat="body", cap0=True, cap1=True):
    """A body lofted forward (-Z) through sections (z, hx, y0, y1[, chamfer[, x]]): receivers,
    stocks and handguards with real width, taper and chamfered edges."""
    first = mat if isinstance(mat, str) else mat[0]
    last = mat if isinstance(mat, str) else mat[-1]
    L.loft(g, [osec(*s) for s in secs], NZ, mats=mat, cap0=first if cap0 else None,
           cap1=last if cap1 else None)


def ysec(y, hx, z0, z1, c=0.005):
    return [(hx, y, z0 + c), (hx, y, z1 - c), (hx - c, y, z1), (-hx + c, y, z1),
            (-hx, y, z1 - c), (-hx, y, z0 + c), (-hx + c, y, z0), (hx - c, y, z0)]


def drop(g, secs, mat="polymer"):
    """A body lofted downward through sections (y, hx, z0, z1[, chamfer]): magazines, fore
    grips."""
    L.loft(g, [ysec(*s) for s in secs], NY, mats=mat, cap0=mat, cap1=mat)


def side_rect(s, x, z0, z1, y0, y1):
    return [(s * x, y0, z0), (s * x, y0, z1), (s * x, y1, z1), (s * x, y1, z0)]


def sbox(g, a, b, mat="body"):
    """A box between two opposite corners, in any order."""
    L.box(g, tuple(map(min, a, b)), tuple(map(max, a, b)), mat=mat)


def rail(g, y, z0, z1, hx=0.009, pitch=0.02, mat="metal"):
    """An accessory rail along the top at height y from z0 (front) to z1: a bar with teeth."""
    L.box(g, (-hx * 0.75, y, z0), (hx * 0.75, y + 0.004, z1), mat=mat)
    z = z0 + 0.003
    while z + pitch * 0.55 < z1:
        L.box(g, (-hx, y + 0.004, z), (hx, y + 0.009, z + pitch * 0.55), mat=mat)
        z += pitch


def tz(g, x, y, z0, z1, r, n=8, mat="metal", cap0=True, cap1=True, rv=None):
    """A tube along the bore from z0 (back) forward to z1."""
    P.lathe(g, (x, y, z0), NZ, [(0, r), (z0 - z1, r)], n=n, mats=mat,
            cap0=mat if cap0 else None, cap1=mat if cap1 else None, rv=rv)


def side_line(g, xh, pts_zy, w=0.005):
    """Team line on both side planes x = +-xh through (z, y) points."""
    for s in (-1, 1):
        P.accent(g, [(s * xh, y, z) for z, y in pts_zy], (s, 0, 0), w, glow=0.0, h=0.0015)


def top_line(g, y, x, z0, z1, w=0.005):
    P.accent(g, [(x, y, z0), (x, y, z1)], Y, w, glow=0.0, h=0.0015)


def barrel(g, z0, yc, prof, n=12, ring_at=(), white_tip=False, mat="metal", x=0.0):
    """Lathe along -Z from (x, yc, z0); prof [(t, r)] with a lit muzzle cap at the end."""
    P.lathe(g, (x, yc, z0), NZ, prof, n=n, mats=mat, cap0=mat if isinstance(mat, str) else mat[0],
            cap1=LIGHT if white_tip else "accent")
    for t, r in ring_at:
        P.lathe(g, (x, yc, z0), NZ, [(t, r), (t + 0.006, r)], n=n, mats="accent", cap0=None,
                cap1=None)


def butt(g, z, hx, y0, y1, t=0.014):
    """A rubber butt pad on the back of a stock."""
    P.bevel_box(g, (-hx, y0, z), (hx, y1, z + t), 0.004, mat="rubber")


def lens(g, c, axis, r, n=12, ring=True):
    """A glass lens facing along `axis` at c, a team ring round it."""
    g.face(P.circle(c, axis, r, n), "glass", want=axis)
    if ring:
        P.accent(g, P.circle(c, axis, r * 1.02, n), axis, r * 0.18, closed=True, glow=0.0,
                 h=0.001)


def flat(g, o, n, along, hw, hl, h, mat):
    """A small plate centred on o on the face with normal n: half length hl along `along`,
    half width hw across it (ports, perforations, cells on round bodies)."""
    n = L.norm(n)
    a = L.norm(along)
    t = L.cross(n, a)
    q = [L.add(o, L.add(L.mul(t, -hw), L.mul(a, -hl))), L.add(o, L.add(L.mul(t, hw), L.mul(a, -hl))),
         L.add(o, L.add(L.mul(t, hw), L.mul(a, hl))), L.add(o, L.add(L.mul(t, -hw), L.mul(a, hl)))]
    P.plate(g, q, n, h, mat)


def muzzle(name, pos):
    NODES.append((name + "_muzzle", pos))


# ---------------------------------------------------------------------------------------------
# The infantry's guns.

def rifle():
    """Pulse rifle: an armoured upper over a polymer lower and vented handguard, a railed
    top with a holo sight, a lit cell magazine."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.1, 0.019, 0.048, 0.086), (-0.15, 0.019, 0.048, 0.086)], "polymer")  # lower
    hull(g, [(0.125, 0.021, 0.078, 0.11, 0.008), (0.1, 0.024, 0.076, 0.116, 0.008),
             (-0.13, 0.024, 0.076, 0.116, 0.008), (-0.165, 0.02, 0.08, 0.112, 0.008)], "armor")
    # Stock: tapering back, a rubber pad, a cheek rest on top.
    hull(g, [(0.125, 0.017, 0.056, 0.108), (0.2, 0.016, 0.026, 0.106), (0.262, 0.018, 0.004, 0.106)],
         "polymer")
    butt(g, 0.262, 0.019, 0.0, 0.11)
    P.bevel_box(g, (-0.012, 0.104, 0.14), (0.012, 0.114, 0.235), 0.004, mat="rubber")
    # Handguard, its vents, the barrel with its muzzle device.
    hull(g, [(-0.15, 0.022, 0.05, 0.112, 0.008), (-0.3, 0.02, 0.054, 0.108, 0.008),
             (-0.335, 0.016, 0.064, 0.104, 0.007)], "polymer")
    for s in (-1, 1):
        P.vent(g, (s * 0.0212, 0.092, -0.235), (s, 0, 0), 0.026, 0.11, slats=6, depth=0.004,
               mat="polymer", mat_in="metal", up=NZ, frame=0.004)
        P.plate(g, side_rect(s, 0.0235, -0.12, 0.085, 0.082, 0.111), (s, 0, 0), 0.003, "armor",
                bevel=0.004)
        P.bolt_row(g, (s * 0.0265, 0.087, -0.11), (s * 0.0265, 0.087, 0.075), (s, 0, 0), 4,
                   r=0.0035, h=0.002, ends=True)
        P.accent(g, [(s * 0.0265, 0.102, -0.11), (s * 0.0265, 0.102, 0.075)], (s, 0, 0), 0.005,
                 glow=0.0, h=0.0015)
        P.accent(g, [(s * 0.0212, 0.064, -0.3), (s * 0.0218, 0.064, -0.17)], (s, 0, 0), 0.004,
                 glow=0.0, h=0.0015)
        P.accent(g, [(s * 0.0172, 0.07, 0.14), (s * 0.0164, 0.05, 0.24)], (s, 0, 0), 0.004,
                 glow=0.0, h=0.0015)
    P.panel_inset(g, side_rect(1, 0.0265, -0.06, 0.0, 0.09, 0.106), X, h=0.002, rim=0.003,
                  mat="metal", mat_in="body")  # ejection port
    L.box(g, (0.0265, 0.094, 0.02), (0.036, 0.102, 0.04), mat="metal")  # charging handle
    barrel(g, -0.335, 0.085, [(0, 0.0125), (0.045, 0.0115), (0.045, 0.017), (0.052, 0.019),
                              (0.09, 0.019), (0.095, 0.016)], ring_at=((0.02, 0.013),))
    for t in (0.06, 0.074):  # muzzle device ports
        for s in (-1, 1):
            sbox(g, (s * 0.0165, 0.08, -0.335 - t - 0.008), (s * 0.02, 0.09, -0.335 - t), "heat")
    tz(g, 0, 0.106, -0.165, -0.3, 0.006, n=6)  # gas tube over the barrel
    # Rail and holo sight: a hooded pane of glass with the team's reticle bar.
    rail(g, 0.116, -0.3, 0.09)
    P.bevel_box(g, (-0.012, 0.125, -0.04), (0.012, 0.135, 0.03), 0.003, mat="metal")
    for s in (-1, 1):
        sbox(g, (s * 0.011, 0.135, -0.03), (s * 0.014, 0.168, -0.012), "armor")
    L.box(g, (-0.014, 0.168, -0.03), (0.014, 0.171, -0.012), mat="armor")
    L.box(g, (-0.011, 0.136, -0.022), (0.011, 0.168, -0.02), mat="glass")
    P.accent(g, [(-0.004, 0.152, -0.0199), (0.004, 0.152, -0.0199)], Z, 0.003, glow=0.0,
             h=0.0005)
    L.box(g, (-0.004, 0.125, -0.29), (0.004, 0.14, -0.28), mat="metal")  # front post
    # Magazine: a power cell with a lit window, a metal floor plate.
    drop(g, [(0.05, 0.013, -0.168, -0.112), (-0.055, 0.013, -0.178, -0.122)])
    P.bevel_box(g, (-0.015, -0.064, -0.182), (0.015, -0.055, -0.12), 0.003, mat="metal")
    for s in (-1, 1):
        P.accent(g, [(s * 0.013, 0.03, -0.142), (s * 0.013, -0.04, -0.149)], (s, 0, 0), 0.012,
                 glow=0.0, h=0.0015)
    muzzle("rifle", (0, 0.085, -0.43))
    return g


def scatter():
    """Scattergun: a slab receiver, a heat-shielded barrel flaring to a wide slotted muzzle,
    a ribbed pump on the magazine tube, shells in a side saddle."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.1, 0.022, 0.05, 0.118, 0.01), (0.07, 0.027, 0.05, 0.122, 0.01),
             (-0.1, 0.027, 0.05, 0.122, 0.01)], "armor")
    hull(g, [(0.1, 0.019, 0.052, 0.114), (0.16, 0.017, 0.03, 0.108), (0.212, 0.019, 0.01, 0.104)],
         "polymer")
    butt(g, 0.212, 0.02, 0.006, 0.108)
    # Barrel shroud flaring to the muzzle; the muzzle face's three wide team slots.
    hull(g, [(-0.1, 0.026, 0.062, 0.116, 0.01), (-0.3, 0.028, 0.064, 0.114, 0.01),
             (-0.36, 0.055, 0.052, 0.124, 0.01), (-0.38, 0.06, 0.05, 0.126, 0.01)],
         ["body", "body", "metal"])
    for y in (0.068, 0.088, 0.108):
        P.disc(g, [(-0.048, y - 0.005, -0.38), (0.048, y - 0.005, -0.38),
                   (0.048, y + 0.005, -0.38), (-0.048, y + 0.005, -0.38)], NZ, glow=0.0,
               h=0.0015)
    # Heat shield over the barrel: a raised plate with a row of scorched slots.
    L.box(g, (-0.02, 0.114, -0.29), (0.02, 0.121, -0.11), mat="metal")
    for i in range(6):
        z = -0.27 + i * 0.028
        L.box(g, (-0.011, 0.121, z), (0.011, 0.1222, z + 0.014), mat="heat")
    # Magazine tube and pump.
    tz(g, 0, 0.046, -0.1, -0.31, 0.013, n=8)
    hull(g, [(-0.135, 0.021, 0.03, 0.062, 0.008), (-0.15, 0.025, 0.026, 0.064, 0.008),
             (-0.26, 0.025, 0.026, 0.064, 0.008), (-0.275, 0.021, 0.03, 0.062, 0.008)], "polymer")
    for z in (-0.25, -0.225, -0.2, -0.175):
        L.box(g, (-0.0262, 0.036, z), (0.0262, 0.056, z + 0.01), mat="rubber")
    for s in (-1, 1):
        tz(g, s * 0.021, 0.07, -0.1, -0.2, 0.004, n=6)  # action bars
        P.accent(g, [(s * 0.0272, 0.1, -0.09), (s * 0.0272, 0.1, 0.06)], (s, 0, 0), 0.005,
                 glow=0.0, h=0.0015)
        P.bolt_row(g, (s * 0.027, 0.064, -0.085), (s * 0.027, 0.064, 0.055), (s, 0, 0), 3,
                   r=0.004, h=0.002, ends=True)
    # Shell saddle on the left: four shells, metal heads up, team bodies.
    L.box(g, (-0.033, 0.072, -0.07), (-0.027, 0.092, 0.03), mat="polymer")
    for i in range(4):
        z = -0.055 + i * 0.024
        P.lathe(g, (-0.041, 0.064, z), Y, [(0, 0.008), (0.03, 0.008)], n=8, mats="paint",
                cap0="paint", cap1=None)
        P.lathe(g, (-0.041, 0.094, z), Y, [(0, 0.0088), (0.006, 0.0088)], n=8, mats="metal",
                cap0="metal", cap1="metal")
    P.panel_inset(g, side_rect(1, 0.027, -0.06, 0.01, 0.084, 0.108), X, h=0.002, rim=0.003,
                  mat="metal", mat_in="body")
    # Sights and the top line.
    L.box(g, (-0.008, 0.122, 0.04), (0.008, 0.13, 0.06), mat="metal")
    L.box(g, (-0.003, 0.121, -0.3), (0.003, 0.13, -0.29), mat="metal")
    top_line(g, 0.122, 0, -0.09, 0.03)
    for s in (-1, 1):
        nrm = L.norm((s * 0.06, 0, -0.027))
        q = [(s * 0.0305, 0.089, -0.305), (s * 0.053, 0.088, -0.355)]
        P.accent(g, [L.add(p, L.mul(nrm, 0.0005)) for p in q], nrm, 0.006, glow=0.0, h=0.0015)
    muzzle("scatter", (0, 0.088, -0.38))
    return g


def lance():
    """Prism lance: a long fluted barrel with a slotted brake, a big scope with turrets, a
    skeleton stock with a cheek riser, a folded bipod."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.13, 0.017, 0.056, 0.106, 0.008), (0.1, 0.02, 0.05, 0.11, 0.008),
             (-0.18, 0.02, 0.05, 0.11, 0.008)], "armor")
    # Skeleton stock: upper bar, lower strut, butt plate and pad, cheek riser.
    hull(g, [(0.29, 0.011, 0.088, 0.108, 0.004), (0.13, 0.011, 0.088, 0.108, 0.004)], "polymer")
    L.band_x(g, [(0.125, 0.058), (0.29, 0.014)], 0.014, -0.009, 0.009, mat="polymer")
    P.bevel_box(g, (-0.015, 0.0, 0.288), (0.015, 0.114, 0.306), 0.004, mat="polymer")
    butt(g, 0.306, 0.016, 0.0, 0.116)
    P.bevel_box(g, (-0.013, 0.108, 0.15), (0.013, 0.126, 0.25), 0.005, mat="rubber")
    # Handguard with slots, then the barrel.
    hull(g, [(-0.18, 0.018, 0.058, 0.108, 0.008), (-0.38, 0.016, 0.062, 0.104, 0.008),
             (-0.405, 0.013, 0.068, 0.1, 0.006)], "polymer")
    for s in (-1, 1):
        for i in range(4):
            z = -0.36 + i * 0.045
            P.plate(g, side_rect(s, 0.0162, z, z + 0.03, 0.078, 0.092), (s, 0, 0), 0.0025,
                    "metal", bevel=0.003)
        P.plate(g, side_rect(s, 0.0195, -0.17, 0.09, 0.056, 0.104), (s, 0, 0), 0.003, "armor",
                bevel=0.005)
        P.accent(g, [(s * 0.0225, 0.094, -0.16), (s * 0.0225, 0.094, 0.08)], (s, 0, 0), 0.005,
                 glow=0.0, h=0.0015)
        P.bolt_row(g, (s * 0.0225, 0.064, -0.16), (s * 0.0225, 0.064, 0.08), (s, 0, 0), 5,
                   r=0.003, h=0.002, ends=True)
    barrel(g, -0.405, 0.085, [(0, 0.012), (0.03, 0.0105), (0.22, 0.0095), (0.22, 0.015),
                              (0.228, 0.017), (0.27, 0.017), (0.275, 0.014)],
           ring_at=((0.06, 0.0108), (0.19, 0.0102)))
    for k in range(4):  # flutes: dark grooves along the barrel
        a = math.pi / 4 + k * math.pi / 2
        c, s_ = math.cos(a), math.sin(a)
        flat(g, (c * 0.0094, 0.085 + s_ * 0.0094, -0.53), (c, s_, 0), Z, 0.0025, 0.05, 0.0012,
             "rubber")
    for t in (0.236, 0.254):  # the brake's ports
        for s in (-1, 1):
            sbox(g, (s * 0.0145, 0.079, -0.405 - t - 0.01), (s * 0.018, 0.091, -0.405 - t), "heat")
    # Folded bipod under the barrel.
    L.box(g, (-0.012, 0.066, -0.425), (0.012, 0.074, -0.405), mat="metal")
    for s in (-1, 1):
        tz(g, s * 0.009, 0.066, -0.41, -0.56, 0.0035, n=6)
        sbox(g, (s * 0.005, 0.06, -0.572), (s * 0.013, 0.07, -0.556), "rubber")
    # Scope: mounts on a rail, a tube with a wide objective, glass both ends, two turrets.
    rail(g, 0.11, -0.16, 0.1)
    for z in (-0.12, 0.0):
        P.bevel_box(g, (-0.007, 0.119, z - 0.01), (0.007, 0.14, z + 0.01), 0.002, mat="metal")
        P.lathe(g, (0, 0.152, z + 0.01), NZ, [(0, 0.0165), (0.02, 0.0165)], n=12, mats="metal",
                cap0="metal", cap1="metal")
    sy = 0.152
    P.lathe(g, (0, sy, 0.08), NZ, [(0, 0.014), (0.004, 0.017), (0.035, 0.017), (0.05, 0.0135),
                                    (0.2, 0.0135), (0.235, 0.023), (0.29, 0.023), (0.295, 0.021)],
            n=12, mats="armor", cap0="armor", cap1="armor")
    lens(g, (0, sy, -0.2152), NZ, 0.0195)
    lens(g, (0, sy, 0.0802), Z, 0.013)
    P.lathe(g, (0, sy + 0.013, -0.045), Y, [(0, 0.009), (0.012, 0.009), (0.014, 0.0075)], n=8,
            mats="metal", cap0=None, cap1="accent")
    P.lathe(g, (0.013, sy, -0.045), X, [(0, 0.009), (0.012, 0.009), (0.014, 0.0075)], n=8,
            mats="metal", cap0=None, cap1="accent")
    drop(g, [(0.05, 0.012, -0.13, -0.085), (-0.03, 0.012, -0.136, -0.091)])
    P.bevel_box(g, (-0.014, -0.038, -0.14), (0.014, -0.03, -0.088), 0.003, mat="metal")
    muzzle("lance", (0, 0.085, -0.68))
    return g


def launcher_grip(g):
    """The launchers' mount over the grip and their fore grip (a bar from (y 0.09, z -0.3)
    down to (-0.02, -0.32): `support` in src/actors.rs)."""
    hull(g, [(0.07, 0.02, 0.05, 0.09), (-0.13, 0.02, 0.05, 0.09)], "armor")
    L.band_x(g, [(-0.3, 0.09), (-0.32, -0.02)], 0.03, -0.014, 0.014, mat="rubber")
    for y in (0.06, 0.04, 0.02, 0.0):
        z = -0.3055 - (0.09 - y) * 0.02 / 0.11
        L.box(g, (-0.0152, y - 0.004, z - 0.0162), (0.0152, y + 0.004, z + 0.0162), mat="metal")
    for s in (-1, 1):
        P.bolt_row(g, (s * 0.02, 0.07, -0.11), (s * 0.02, 0.07, 0.05), (s, 0, 0), 4,
                   r=0.004, h=0.002, ends=True)


def striker():
    """Striker: a shoulder rocket tube with a scorched blast cone, a padded rest, a boxed
    sight wired to the tube, a fore grip."""
    g = L.Geo()
    grip(g)
    launcher_grip(g)
    yc = 0.15
    n = 16
    P.lathe(g, (0, yc, 0.48), NZ, [(0, 0.058), (0.03, 0.09), (0.1, 0.09), (0.12, 0.072),
                                    (0.98, 0.072), (1.0, 0.088), (1.1, 0.088), (1.1, 0.062),
                                    (1.06, 0.062)], n=n,
            mats=["heat", "heat", "heat", "body", "armor", "armor", "armor", "metal"],
            cap0="heat", cap1=LIGHT)
    for t, r in ((0.05, 0.092), (1.04, 0.09), (0.3, 0.0745), (0.9, 0.0745)):
        P.lathe(g, (0, yc, 0.48), NZ, [(t, r), (t + 0.016, r)], n=n, mats="accent", cap0=None,
                cap1=None)
    for t in (0.2, 0.42, 0.66):  # clamp bands
        P.lathe(g, (0, yc, 0.48), NZ, [(t, 0.0755), (t + 0.03, 0.0755)], n=n, mats="metal",
                cap0="metal", cap1="metal")
    for k in range(8):  # the blast cone's vanes
        a = 2 * math.pi * k / 8
        c, s_ = math.cos(a), math.sin(a)
        flat(g, (c * 0.089, yc + s_ * 0.089, 0.415), (c, s_, 0), Z, 0.008, 0.03, 0.007, "metal")
    # Shoulder rest under the rear of the tube.
    P.bevel_box(g, (-0.035, 0.05, 0.16), (0.035, 0.085, 0.34), 0.008, mat="rubber")
    L.box(g, (-0.02, 0.07, 0.07), (0.02, 0.085, 0.17), mat="armor")
    # Sight box on the left with a glass lens, on a bracket, its cable to the tube.
    L.box(g, (-0.085, 0.15, -0.17), (-0.06, 0.17, -0.11), mat="metal")
    P.bevel_box(g, (-0.125, 0.165, -0.21), (-0.07, 0.222, -0.07), 0.006, mat="armor")
    g.face([(-0.118, 0.172, -0.2102), (-0.077, 0.172, -0.2102), (-0.077, 0.215, -0.2102),
            (-0.118, 0.215, -0.2102)], "glass", want=NZ)
    P.accent(g, [(-0.118, 0.172, -0.2104), (-0.077, 0.172, -0.2104), (-0.077, 0.215, -0.2104),
                 (-0.118, 0.215, -0.2104)], NZ, 0.004, closed=True, glow=0.0, h=0.001)
    P.hose(g, [(-0.097, 0.19, -0.07), (-0.097, 0.2, -0.02), (-0.07, 0.215, 0.06),
               (-0.045, 0.215, 0.14)], 0.005, n=6, collars=(0, 3))
    # Team line along the tube's top.
    yt = yc + 0.072 * math.cos(math.pi / n)
    P.accent(g, [(0, yt, 0.16), (0, yt, -0.36)], Y, 0.008, glow=0.0, h=0.0015)
    muzzle("striker", (0, yc, -0.62))
    return g


def skyhook():
    """Skyhook: a boxy anti-air launcher, a glass seeker in a ringed mouth, a sensor fin, side
    cooling vents and armour panels."""
    g = L.Geo()
    grip(g)
    launcher_grip(g)
    yc = 0.15
    hull(g, [(0.45, 0.07, yc - 0.062, yc + 0.062, 0.02), (0.42, 0.085, yc - 0.075, yc + 0.075, 0.022),
             (-0.5, 0.085, yc - 0.075, yc + 0.075, 0.022), (-0.55, 0.078, yc - 0.068, yc + 0.068, 0.02)],
         ["heat", "body", "armor"])
    # Seeker: a ringed mouth, a dark glass dome with a white eye.
    P.lathe(g, (0, yc, -0.55), NZ, [(0, 0.066), (0.03, 0.07), (0.05, 0.066)], n=16, mats="metal",
            cap0=None, cap1=None)
    P.lathe(g, (0, yc, -0.555), NZ, [(0, 0.062), (0.025, 0.05), (0.04, 0.028)], n=16,
            mats="glass", cap0=None, cap1=LIGHT)
    P.accent(g, P.circle((0, yc, -0.6), NZ, 0.068, 16), NZ, 0.006, closed=True, glow=0.0,
             h=0.0015)
    for s in (-1, 1):
        P.panel_inset(g, side_rect(s, 0.085, -0.46, -0.16, yc - 0.045, yc + 0.045), (s, 0, 0),
                      h=0.004, rim=0.008, mat="armor", mat_in="body")
        P.accent(g, [(s * 0.0865, yc, -0.44), (s * 0.0865, yc, -0.18)], (s, 0, 0), 0.01,
                 glow=0.0, h=0.0015)
        P.vent(g, (s * 0.085, yc, 0.2), (s, 0, 0), 0.26, 0.07, slats=5, depth=0.008,
               mat="metal", mat_in="rubber", frame=0.008)
        P.bolt_row(g, (s * 0.085, yc + 0.045, -0.1), (s * 0.085, yc + 0.045, 0.38), (s, 0, 0), 6,
                   r=0.004, h=0.002, ends=True)
    P.accent(g, [(-0.05, yc - 0.042, 0.45), (0.05, yc - 0.042, 0.45), (0.05, yc + 0.042, 0.45),
                 (-0.05, yc + 0.042, 0.45)], Z, 0.008, closed=True, glow=0.0, h=0.0015)
    # Sensor fin on top, a rail behind it, the shoulder rest.
    P.extrude(g, [(-0.006, yc + 0.075, -0.3), (-0.006, yc + 0.075, -0.05),
                  (-0.006, yc + 0.135, -0.09), (-0.006, yc + 0.135, -0.24)], (0.012, 0, 0),
              mat="armor")
    for s in (-1, 1):
        P.white(g, [(s * 0.0062, yc + 0.11, -0.22), (s * 0.0062, yc + 0.11, -0.1)], (s, 0, 0),
                0.008, glow=0.0)
    rail(g, yc + 0.075, 0.0, 0.3, hx=0.011)
    P.bevel_box(g, (-0.035, 0.045, 0.16), (0.035, 0.076, 0.34), 0.008, mat="rubber")
    P.hose(g, [(0.035, yc + 0.075, -0.04), (0.045, yc + 0.095, 0.0), (0.045, yc + 0.095, 0.3),
               (0.035, yc + 0.07, 0.36)], 0.006, n=6, collars=(1, 2))
    muzzle("skyhook", (0, yc, -0.6))
    return g


def pistol():
    """Sidearm: a serrated slide over a polymer frame, a lit barrel tip, a lamp under the
    dust cover."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.05, 0.0145, 0.05, 0.08, 0.005), (-0.1, 0.0145, 0.05, 0.08, 0.005),
             (-0.145, 0.0145, 0.062, 0.08, 0.005)], "polymer")
    hull(g, [(0.058, 0.0145, 0.082, 0.106, 0.006), (0.05, 0.0165, 0.078, 0.11, 0.006),
             (-0.14, 0.0165, 0.078, 0.11, 0.006), (-0.15, 0.0145, 0.08, 0.106, 0.006)], "armor")
    for s in (-1, 1):
        for i in range(5):
            z = 0.008 + i * 0.008
            sbox(g, (s * 0.0165, 0.086, z), (s * 0.0178, 0.104, z + 0.004), "metal")
        P.accent(g, [(s * 0.0172, 0.097, -0.13), (s * 0.0172, 0.097, -0.02)], (s, 0, 0), 0.004,
                 glow=0.0, h=0.0015)
        P.lathe(g, (s * 0.0145, 0.066, -0.03), (s, 0, 0), [(0, 0.004), (0.002, 0.0035)], n=6,
                mats="metal", cap0=None, cap1="metal")
    P.panel_inset(g, side_rect(1, 0.0165, -0.075, -0.035, 0.088, 0.104), X, h=0.0015, rim=0.0025,
                  mat="metal", mat_in="body")
    barrel(g, -0.15, 0.085, [(0, 0.0105), (0.03, 0.0105), (0.03, 0.012), (0.04, 0.012)], n=10)
    # Sights, top line, the lamp under the frame.
    L.box(g, (-0.007, 0.11, 0.03), (0.007, 0.117, 0.045), mat="metal")
    L.box(g, (-0.002, 0.11, -0.14), (0.002, 0.117, -0.13), mat="metal")
    top_line(g, 0.11, 0, -0.12, 0.02, 0.004)
    P.bevel_box(g, (-0.012, 0.036, -0.14), (0.012, 0.062, -0.085), 0.004, mat="body")
    g.face([(-0.008, 0.041, -0.1402), (0.008, 0.041, -0.1402), (0.008, 0.057, -0.1402),
            (-0.008, 0.057, -0.1402)], LIGHT, want=NZ)
    muzzle("pistol", (0, 0.085, -0.19))
    return g


def medgun():
    """Medical applicator: a rounded body, a white vial on clamps feeding the emitter dish
    through a hose, coil rings round the neck."""
    g = L.Geo()
    grip(g)
    yc = 0.09
    n = 12
    P.lathe(g, (0, yc, 0.15), NZ, [(0, 0.018), (0.02, 0.034), (0.05, 0.04), (0.26, 0.04),
                                    (0.3, 0.03), (0.34, 0.022), (0.37, 0.022)], n=n,
            mats=["polymer", "polymer", "armor", "armor", "metal", "metal"], cap0="polymer",
            cap1="metal")
    hull(g, [(0.08, 0.02, 0.046, 0.066), (-0.1, 0.02, 0.046, 0.066)], "polymer")
    for t in (0.305, 0.32, 0.335):  # coils round the neck
        P.lathe(g, (0, yc, 0.15), NZ, [(t, 0.027), (t + 0.007, 0.027)], n=n, mats="accent",
                cap0="metal", cap1="metal")
    # Emitter dish: a cone opening forward, white core and ring.
    o = (0, yc, -0.22)
    P.lathe(g, o, NZ, [(0, 0.024), (0.06, 0.07), (0.07, 0.072)], n=n, mats="armor", cap0=None,
            cap1=None)
    fr, bk = (P.circle(L.add(o, (0, 0, -t)), NZ, r, n) for t, r in ((0.07, 0.066), (0.03, 0.03)))
    for i in range(n):  # the dish's inside faces forward and in
        j = (i + 1) % n
        q = [bk[i], bk[j], fr[j], fr[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, LIGHT, want=L.add((-mid[0], yc - mid[1], 0), (0, 0, -0.05)))
    g.face(bk, LIGHT, want=NZ)
    rim = P.circle(L.add(o, (0, 0, -0.07)), NZ, 0.072, n)
    rin = P.circle(L.add(o, (0, 0, -0.07)), NZ, 0.066, n)
    for i in range(n):
        j = (i + 1) % n
        g.face([rin[i], rin[j], rim[j], rim[i]], "accent", want=NZ)
    for k in range(3):  # struts behind the dish
        a = math.pi / 2 + 2 * math.pi * k / 3
        c, s_ = math.cos(a), math.sin(a)
        P.hose(g, [(c * 0.03, yc + s_ * 0.03, -0.19), (c * 0.066, yc + s_ * 0.066, -0.275)],
               0.004, n=4, mat="metal")
    # White vial on clamps, its hose down to the neck.
    P.lathe(g, (0, 0.156, 0.11), NZ, [(0, 0.018), (0.14, 0.018)], n=n, mats=LIGHT, cap0="body",
            cap1="body")
    for z in (0.115, -0.03):
        P.lathe(g, (0, 0.156, z), NZ, [(0, 0.0215), (0.016, 0.0215)], n=n, mats="metal",
                cap0="metal", cap1="metal")
        L.box(g, (-0.009, 0.126, z - 0.016), (0.009, 0.14, z), mat="metal")
    P.hose(g, [(0, 0.156, -0.046), (0, 0.158, -0.08), (0, 0.14, -0.13), (0, 0.118, -0.17)],
           0.006, n=6, collars=(1,))
    side = 0.04 * math.cos(math.pi / n)
    for s in (-1, 1):
        P.accent(g, [(s * side, yc, 0.08), (s * side, yc, -0.1)], (s, 0, 0), 0.006, glow=0.0,
                 h=0.0015)
    P.plate(g, [(-0.012, 0.1285, 0.02), (0.012, 0.1285, 0.02), (0.012, 0.1285, -0.02),
                (-0.012, 0.1285, -0.02)], Y, 0.003, "metal", bevel=0.003)
    P.white(g, [(0, 0.1316, 0.012), (0, 0.1316, -0.012)], Y, 0.006, glow=0.004, h=0.0005)
    muzzle("medgun", (0, yc, -0.29))
    return g


def repair_gun():
    """Repair tool: a chunky armoured housing, a fluid canister on clamps piped to a tapering
    nozzle with cooling fins, two arc prongs."""
    g = L.Geo()
    grip(g)
    yc = 0.09
    xh = 0.032
    hull(g, [(0.18, 0.026, 0.078, 0.128, 0.01), (0.14, xh, 0.05, 0.14, 0.012),
             (-0.14, xh, 0.05, 0.14, 0.012), (-0.17, 0.028, 0.054, 0.134, 0.012)], "armor")
    # Canister on top: white fluid window between metal end caps, on two clamps.
    P.lathe(g, (0, 0.177, 0.16), NZ, [(0, 0.026), (0.02, 0.034), (0.05, 0.034), (0.15, 0.034),
                                       (0.18, 0.034), (0.2, 0.026)], n=12,
            mats=["metal", "metal", LIGHT, "metal", "metal"], cap0="metal", cap1="metal")
    for z in (0.1, -0.01):
        L.box(g, (-0.014, 0.138, z - 0.012), (0.014, 0.152, z + 0.012), mat="body")
        P.lathe(g, (0, 0.177, z + 0.012), NZ, [(0, 0.037), (0.024, 0.037)], n=12, mats="accent",
                cap0=None, cap1=None)
    P.hose(g, [(0, 0.177, -0.04), (0, 0.176, -0.09), (0, 0.15, -0.15), (0, 0.128, -0.19)],
           0.008, n=6, collars=(1,))
    # Nozzle: tapering along -Z with cooling fins, a scorched tip, white bore.
    o = (0, yc, -0.17)
    P.lathe(g, o, NZ, [(0, 0.042), (0.06, 0.042), (0.1, 0.03), (0.14, 0.022), (0.16, 0.018)],
            n=12, mats=["metal", "metal", "metal", "heat"], cap0="metal", cap1=LIGHT)
    for t in (0.01, 0.026, 0.042):
        P.lathe(g, o, NZ, [(t, 0.047), (t + 0.008, 0.047)], n=12, mats="accent", cap0="metal",
                cap1="metal")
    # Arc prongs either side of the bore, white tips.
    for s in (-1, 1):
        P.tube(g, (s * 0.026, yc, -0.29), (s * 0.02, yc, -0.355), 0.006, n=6, mat="metal")
        P.tube(g, (s * 0.02, yc, -0.355), (s * 0.018, yc, -0.37), 0.007, n=6, mat=LIGHT)
        sbox(g, (s * 0.018, yc - 0.012, -0.3), (s * 0.034, yc + 0.012, -0.27), "body")
        P.panel_inset(g, side_rect(s, xh, -0.12, 0.02, 0.066, 0.112), (s, 0, 0), h=0.003,
                      rim=0.006, mat="armor", mat_in="body")
        P.accent(g, [(s * (xh + 0.0012), 0.126, -0.13), (s * (xh + 0.0012), 0.126, 0.13)],
                 (s, 0, 0), 0.005, glow=0.0, h=0.0015)
        P.vent(g, (s * xh, 0.09, 0.085), (s, 0, 0), 0.08, 0.04, slats=4, depth=0.005,
               mat="metal", mat_in="rubber", frame=0.005)
        P.bolt_row(g, (s * xh, 0.058, -0.12), (s * xh, 0.058, 0.12), (s, 0, 0), 5, r=0.004,
                   h=0.002, ends=True)
    top_line(g, 0.14, 0, -0.16, -0.14)
    P.bevel_box(g, (-0.012, 0.14, -0.13), (0.012, 0.15, -0.07), 0.003, mat="metal")
    muzzle("repair_gun", (0, yc, -0.37))
    return g


def ember():
    """The red faction's rifle: short and heavy. A slab receiver with paint slashes, a thick
    ported heat shroud round a stub barrel, a drum under it, a stubby padded stock."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.11, 0.024, 0.052, 0.118, 0.01), (0.08, 0.03, 0.048, 0.126, 0.012),
             (-0.14, 0.03, 0.048, 0.126, 0.012), (-0.165, 0.026, 0.052, 0.122, 0.012)], "armor")
    hull(g, [(0.19, 0.022, 0.03, 0.114, 0.008), (0.11, 0.022, 0.05, 0.116, 0.008)], "polymer")
    butt(g, 0.19, 0.024, 0.026, 0.118, t=0.018)
    # Heat shroud: a fat octagonal jacket with ports, the stub barrel and its brake.
    P.lathe(g, (0, 0.085, -0.165), NZ, [(0, 0.026), (0.01, 0.031), (0.185, 0.031), (0.2, 0.024)],
            n=8, mats="heat", cap0="heat", cap1="heat")
    rr = 0.031 * math.cos(math.pi / 8)
    for k in range(8):
        a = 2 * math.pi * k / 8
        c, s_ = math.cos(a), math.sin(a)
        if s_ < -0.5:
            continue  # (not under the hand)
        for i in range(4):
            flat(g, (c * rr, 0.085 + s_ * rr, -0.2 - i * 0.037), (c, s_, 0), Z, 0.006, 0.012,
                 0.001, "rubber")
    barrel(g, -0.365, 0.085, [(0, 0.016), (0.012, 0.016), (0.012, 0.021), (0.035, 0.021)], n=8,
           ring_at=((0.02, 0.0222),))
    # Drum magazine across the receiver's front.
    P.lathe(g, (-0.034, 0.0, -0.105), X, [(0, 0.04), (0.006, 0.047), (0.062, 0.047), (0.068, 0.04)],
            n=12, mats="polymer", cap0="metal", cap1="metal")
    for s in (-1, 1):
        P.accent(g, P.circle((s * 0.0342, 0.0, -0.105), (s, 0, 0), 0.03, 12), (s, 0, 0), 0.005,
                 closed=True, glow=0.0, h=0.001)
        P.lathe(g, (s * 0.034, 0.0, -0.105), (s, 0, 0), [(0, 0.01), (0.005, 0.008)], n=6,
                mats="metal", cap0=None, cap1="metal")
        # Paint slashes, the team line, rivets.
        for i in range(3):
            z = -0.11 + i * 0.05
            P.plate(g, [(s * 0.03, 0.066, z), (s * 0.03, 0.066, z + 0.022),
                        (s * 0.03, 0.104, z + 0.046), (s * 0.03, 0.104, z + 0.024)], (s, 0, 0),
                    0.0015, "paint")
        P.accent(g, [(s * 0.0312, 0.114, -0.13), (s * 0.0312, 0.114, 0.07)], (s, 0, 0), 0.005,
                 glow=0.0, h=0.0015)
        P.bolt_row(g, (s * 0.03, 0.056, -0.13), (s * 0.03, 0.056, 0.07), (s, 0, 0), 5, r=0.0045,
                   h=0.0025, ends=True)
    L.box(g, (-0.012, 0.04, -0.135), (0.012, 0.05, -0.075), mat="metal")  # drum latch block
    # Carry-handle sight: two posts and a bar.
    for z in (-0.12, 0.04):
        L.box(g, (-0.006, 0.126, z - 0.008), (0.006, 0.152, z + 0.008), mat="metal")
    hull(g, [(0.06, 0.008, 0.15, 0.162, 0.004), (-0.14, 0.008, 0.15, 0.162, 0.004)], "armor")
    top_line(g, 0.162, 0, -0.13, 0.05, 0.004)
    muzzle("ember", (0, 0.085, -0.40))
    return g


def volt():
    """The blue faction's rifle: long and slim. Twin rails either side of an open bore, coil
    rings along them, a capacitor block, a skeletal stock, a tube sight."""
    g = L.Geo()
    grip(g)
    hull(g, [(0.1, 0.014, 0.052, 0.112, 0.006), (0.08, 0.017, 0.05, 0.116, 0.006),
             (-0.12, 0.017, 0.05, 0.116, 0.006), (-0.15, 0.013, 0.056, 0.112, 0.006)], "armor")
    # Twin rails (upper and lower) with the lit bore between them, joined by coil rings.
    for y0, y1 in ((0.097, 0.109), (0.061, 0.073)):
        hull(g, [(-0.14, 0.0085, y0, y1, 0.003), (-0.455, 0.0085, y0, y1, 0.003),
                 (-0.47, 0.006, y0 + 0.002, y1 - 0.002, 0.002)], "metal")
    for y, n_ in ((0.0968, NY), (0.0732, Y)):
        P.accent(g, [(0, y, -0.16), (0, y, -0.46)], n_, 0.007, glow=0.0, h=0.0006)
    for i in range(5):
        z = -0.2 - i * 0.055
        for s in (-1, 1):
            sbox(g, (s * 0.0085, 0.058, z - 0.014), (s * 0.0125, 0.112, z), "armor")
            P.accent(g, [(s * 0.0127, 0.064, z - 0.007), (s * 0.0127, 0.106, z - 0.007)],
                     (s, 0, 0), 0.004, glow=0.0, h=0.001)
        L.box(g, (-0.0125, 0.109, z - 0.014), (0.0125, 0.114, z), mat="armor")
        L.box(g, (-0.0125, 0.056, z - 0.014), (0.0125, 0.061, z), mat="armor")
    # Handguard under the lower rail (where the off hand cups it), capacitor block ahead of
    # the grip with lit cells.
    hull(g, [(-0.15, 0.016, 0.046, 0.06, 0.005), (-0.31, 0.014, 0.05, 0.06, 0.004)], "polymer")
    drop(g, [(0.05, 0.015, -0.14, -0.075), (-0.015, 0.015, -0.14, -0.08)], "body")
    for s in (-1, 1):
        for i in range(3):
            z = -0.128 + i * 0.018
            P.accent(g, [(s * 0.015, 0.04, z), (s * 0.015, -0.005, z)], (s, 0, 0), 0.008,
                     glow=0.0, h=0.001)
        P.accent(g, [(s * 0.0182, 0.1, -0.11), (s * 0.0182, 0.1, 0.07)], (s, 0, 0), 0.004,
                 glow=0.0, h=0.0015)
        P.plate(g, side_rect(s, 0.017, -0.11, 0.07, 0.058, 0.09), (s, 0, 0), 0.002, "polymer",
                bevel=0.004)
        P.bolt_row(g, (s * 0.019, 0.066, -0.1), (s * 0.019, 0.066, 0.06), (s, 0, 0), 3, r=0.003,
                   h=0.0015, ends=True)
    # Skeletal stock: two thin bars to a butt hook.
    for y in (0.104, 0.058):
        tz(g, 0, y, 0.3, 0.1, 0.006, n=6, mat="metal")
    P.bevel_box(g, (-0.012, 0.03, 0.29), (0.012, 0.116, 0.304), 0.004, mat="polymer")
    butt(g, 0.304, 0.013, 0.03, 0.116, t=0.01)
    L.box(g, (-0.005, 0.058, 0.19), (0.005, 0.104, 0.2), mat="metal")
    # Tube sight on two rings.
    for z in (-0.08, 0.03):
        L.box(g, (-0.004, 0.116, z - 0.006), (0.004, 0.128, z + 0.006), mat="metal")
    P.lathe(g, (0, 0.139, 0.06), NZ, [(0, 0.012), (0.17, 0.012), (0.18, 0.014), (0.2, 0.014)],
            n=12, mats="polymer", cap0="polymer", cap1="polymer")
    lens(g, (0, 0.139, -0.1402), NZ, 0.0115)
    lens(g, (0, 0.139, 0.0602), Z, 0.0095, ring=False)
    muzzle("volt", (0, 0.085, -0.47))
    return g


def hive():
    """The green faction's rifle: grown rather than built. A bulbous shell, three perforated
    barrels in a cluster, a helical magazine under them, a tail stock with fins."""
    g = L.Geo()
    grip(g)
    n = 12
    P.lathe(g, (0, 0.085, 0.13), NZ, [(0, 0.012), (0.03, 0.03), (0.09, 0.042), (0.17, 0.045),
                                       (0.25, 0.036), (0.31, 0.026), (0.33, 0.026)], n=n,
            mats="armor", cap0="armor", cap1="metal", rv=0.88)
    hull(g, [(0.06, 0.016, 0.046, 0.07), (-0.08, 0.016, 0.046, 0.07)], "polymer")
    # Tail stock: a tapering spine with a hooked butt and two dorsal fins.
    P.lathe(g, (0, 0.082, 0.28), NZ, [(0, 0.014), (0.03, 0.02), (0.15, 0.022)], n=n,
            mats="polymer", cap0="polymer", cap1=None, rv=1.5)
    P.bevel_box(g, (-0.013, 0.02, 0.272), (0.013, 0.112, 0.288), 0.005, mat="rubber")
    for z0 in (0.02, 0.1):
        P.extrude(g, [(-0.003, 0.12, z0), (-0.003, 0.12, z0 + 0.06), (-0.003, 0.15, z0 + 0.075),
                      (-0.003, 0.134, z0 + 0.02)], (0.006, 0, 0), mat="paint")
    # Barrel cluster: three thin barrels, bands holding them, lit tips.
    for k in range(3):
        a = math.pi / 2 + 2 * math.pi * k / 3
        ca, sa = math.cos(a), math.sin(a)
        bx, by = 0.012 * ca, 0.085 + 0.012 * sa
        barrel(g, -0.2, by, [(0, 0.0075), (0.2, 0.007), (0.2, 0.0085), (0.22, 0.0085)], n=8,
               x=bx)
        for i in range(5):  # perforations: dark dots along each barrel's outer flat
            flat(g, (bx + 0.0068 * ca, by + 0.0068 * sa, -0.275 - i * 0.022), (ca, sa, 0), Z,
                 0.002, 0.006, 0.0008, "rubber")
    for z in (-0.25, -0.375):
        P.lathe(g, (0, 0.085, z), NZ, [(0, 0.023), (0.014, 0.023)], n=n, mats="metal",
                cap0="metal", cap1="metal")
    P.lathe(g, (0, 0.085, -0.375), NZ, [(0.004, 0.0238), (0.01, 0.0238)], n=n, mats="accent",
            cap0=None, cap1=None)
    # Helical magazine under the barrels: a drum along the bore with a spiral of lit cells.
    my = 0.036
    P.lathe(g, (0, my, -0.13), NZ, [(0, 0.016), (0.012, 0.024), (0.17, 0.024), (0.185, 0.014)],
            n=n, mats="polymer", cap0="polymer", cap1="polymer")
    rr = 0.024 * math.cos(math.pi / n)
    for i in range(14):
        a = math.pi / n + (i * 5 % 12) * 2 * math.pi / n
        c, s_ = math.cos(a), math.sin(a)
        flat(g, (c * rr, my + s_ * rr, -0.15 - i * 0.0105), (c, s_, 0), Z, 0.004, 0.004, 0.0012,
             "accent")
    L.box(g, (-0.006, 0.058, -0.24), (0.006, 0.066, -0.16), mat="metal")
    # Team lines, shell studs, a compound-eye sight.
    for s in (-1, 1):
        P.accent(g, [(s * 0.0438, 0.085, 0.03), (s * 0.0438, 0.085, -0.03)], (s, 0, 0), 0.006,
                 glow=0.0, h=0.0015)
        for i in range(3):
            z = 0.045 - i * 0.045
            P.lathe(g, (s * 0.03, 0.105, z), L.norm((s, 0.9, 0)), [(0, 0.011), (0.012, 0.009),
                                                                   (0.016, 0.004)], n=6,
                    mats="paint", cap0=None, cap1="paint")
    P.lathe(g, (0, 0.118, -0.02), Y, [(0, 0.012), (0.018, 0.012), (0.024, 0.008)], n=8,
            mats="metal", cap0=None, cap1=None)
    P.lathe(g, (0, 0.136, -0.02), Y, [(0, 0.011), (0.008, 0.008), (0.011, 0.003)], n=8,
            mats="glass", cap0=None, cap1="glass")
    muzzle("hive", (0, 0.085, -0.42))
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
SMOOTH = {n: 35 for n in ("rifle", "scatter", "lance", "striker", "skyhook", "pistol", "medgun",
                          "repair_gun", "ember", "volt", "hive")}


def main():
    L.reset_scene()
    P.materials()
    for name, fn in (("rifle", rifle), ("scatter", scatter), ("lance", lance),
                     ("striker", striker), ("pistol", pistol), ("medgun", medgun),
                     ("repair_gun", repair_gun), ("skyhook", skyhook),
                     ("ember", ember), ("volt", volt), ("hive", hive),
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
