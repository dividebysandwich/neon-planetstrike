"""Builds the planetstrike base equipment: assets-src/blend/props.blend and assets/models/props.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/props.py

One mesh per prop, origin on the floor (y = 0) in its middle, front/screen facing -Z, X mirror
symmetric:
- `terminal_equipment`, `terminal_medical`, `terminal_vehicle`: free-standing terminals 1.0 m
  wide (x +-0.5), 0.5 m deep (z +-0.25), 1.9 m tall; a white screen (`glow_white`) in a team
  `accent` frame on a face slanted back towards +Z, and the terminal's motif (equipment: a gun
  over an armour shield; medical: a plus; vehicle: two chevrons) on the screen and big on the
  column front.
- `console_core`: the core computer: hexagonal pedestal (circumradius 0.6 m) whose top slants
  from 1.1 m at the back to 0.9 m at the front (facing -Z), carrying a console screen; above it
  floats a hologram (two team rings and a crystal) up to 2.55 m.
- `turret_base`: wall-top mount, origin at the mount, octagonal pedestal 1.2 m tall (vertex
  radius 0.95 m at the foot).
- `turret_head`: the rotating gun head, ORIGIN AT ITS PIVOT (1.6 m above the mount; the head's
  yoke sits on the pedestal top at local y = -0.4). Twin barrels at x = +-0.5 along -Z, tips at
  z = -1.4; empties `turret_head_muzzle_l` (-0.5, 0.02, -1.4) and `turret_head_muzzle_r`
  (0.5, 0.02, -1.4) in the head frame. 1.24 m wide.
- `vehicle_pad`: flat pad, radius 7.0 m, 0.038 m tall with its inlays; team ring and chevrons
  pointing -Z.
- `rearm_pad`: flat pad, radius 8.0 m (0.038 m tall), white rings and `flow` rings; four 3 m
  light pylons at radius 9.2 m on the diagonals.
- `spawn_tube`: floor disc (radius 0.65 m, 0.048 m with inlays) plus an overhead ring
  (2.28-2.4 m) on two thin posts at x = +-0.68, so players walk through along Z.
- `wall_turret_base`: the heavy wall turret's mount, origin at the mount (wall top), octagonal
  pedestal up to y = 1.8 m, foot 2.6 m across corner to corner (vertex radius 1.3 m).
- `wall_turret_head`: the heavy rotating gun head, ORIGIN AT ITS PIVOT (2.4 m above the mount;
  its turntable sits on the pedestal top at local y = -0.6). Armoured side pods make it 2.6 m
  wide (x +-1.3); between them an open-top gunner's cockpit (seat, backrest, hand grips, a
  white gunsight ring) for a gunner whose eye is at (0, 0.5, 0.6); twin heavy barrels at
  x = +-0.55, y = 0 along -Z with muzzles at z = -2.6: empties `wall_turret_muzzle_l`
  (-0.55, 0, -2.6) and `wall_turret_muzzle_r` (0.55, 0, -2.6) in the head frame.
- `generator`: the base's power generator, 4 m wide (x), 4 m deep, 3.5 m tall, origin floor
  centre: a reactor chamber (an open cage of six struts between a base drum and a cap drum, so
  `generator_core` shows through, framed in team light at the front and back) between two
  machine blocks with vent grilles, coolant pipes arching from the blocks into the cap.
- `generator_core`: just the reactor element, a 16-sided `glow_white` capsule (radius 0.6 m,
  y 0.8-2.4, centred at (0, 1.6, 0)) that the game pulses or hides.
- `resource_silo`: the resource tank, origin at its ground centre: a 24-sided double wall
  (outer radius 3.85 m, inner 3.7 m) on a 4 m plinth ring, 12 m tall with its roof, four
  `accent`-banded ribs (r 4.0 m), four open window slots (y 1.2-10.8 m, team framed) on the
  diagonals and a ladder at the front (-Z, standing out to z = -4.25). The inside is open.
- `resource_silo_fill`: the fill column, 24-sided `accent` cylinder of radius 3.6 m from
  y = 0 to 1 (the game scales it in Y by the fill level), smooth shaded.
- `resource_node`: a cluster of `resource` crystals (the game colours them amber) on `body`
  rocks, origin on the ground, ~4 m across, 3.5 m tall. Not mirror symmetric.
- `spitfire_base`: the engineer's Spitfire turret tripod, origin on the ground at its centre:
  three splayed legs (feet at radius 0.4 m, one at +Z) to a hub at 0.55 m with a white light,
  a post and a turntable topping out at 0.86 m.
- `spitfire_head`: its gun head, ORIGIN AT ITS PIVOT (0.9 m above the ground): a wedge housing
  with ammo boxes on its flanks (x +-0.2), a sensor eye on top, twin barrels at x = +-0.06 along
  -Z; the empty `spitfire_head_muzzle` (0, 0, -0.44) marks the barrels' tips (between them).
- `at_mine`: anti-tank mine, a bevelled disc 0.7 m across and 0.1 m tall with a team ring on
  top and a hexagonal pressure plate (white centre) up to 0.12 m.
- `ammo_pack`: supply crate 0.6 x 0.4 x 0.4 m (x, y, z) with a lid, carry handles on both ends,
  a three-round ammo symbol in team light front and back and a white strip on the lid.
- Engineer construction (field fortifications). Each piece's `body` fills its collider box
  exactly; accents stand at most 8 mm proud of faces:
  - `build_wall`: barrier wall, (-3, 0, -0.4)..(3, 3.5, 0.4): chamfered plinth and coping (team
    line along the top edge), four full-depth buttress posts (team seams), two armour plates
    per bay on both sides with a white light in the seam between them.
  - `bastion_base`: bunker, (-1.6, 0, -1.6)..(1.6, 2.9, 1.6): chamfered square walls with
    white firing slits, a belt at 1.68-1.82 m, slanted upper walls to a deck at 2.55 m and a
    12-sided turntable topping out at 2.9 m.
  - `bastion_head`: its gun head, ORIGIN AT ITS PIVOT (3.3 m above the ground), bounds
    (-0.9, -0.4, -1.7)..(0.9, 0.7, 0.9): a turret ring down to local y = -0.4, a faceted
    housing, a sensor eye on top, twin barrels at x = +-0.3 (1.5 m); the empty
    `bastion_head_muzzle` (0, 0, -1.7) marks the barrels' tips (between them).
  - `forward_tube`: field spawn beacon, (-1.2, 0, -1.2)..(1.2, 3.2, 1.2): octagonal pod (flats
    on the box faces) on a foot, four diagonal fins, a white-lit doorway facing -Z, an emitter
    collar, a dome and a mast with a white beacon to 3.2 m.
  - `mobile_silo`: field resource silo, (-1.8, 0, -1.8)..(1.8, 4.5, 1.8): an octagonal tank
    (flats on the axes, apothem 1.5 m) on a plinth with four diagonal bracing fins and eight
    armour ribs; a window band y 1.4-3.3 m (eight team-framed openings between corner posts,
    open inside); an unloading hopper with a recessed mouth facing -Z; team belt, roof cone
    and a mast with a white beacon to 4.5 m.
  - `capture_pylon`: a capture point's pylon, origin on the floor at its axis, (-0.85, 0, -0.85)
    ..(0.85, 4.0, 0.85): a hexagonal mast (apothem 0.39 m at the foot, flats on +-Z) on a plinth,
    team lines up its faces, a team crown ring at 3.75 m and a white beacon on top.
  - `mobile_silo_fill`: its fill column, octagonal `accent` prism of apothem 1.36 m from
    y = 0 to 1; the game puts it at y = 1.4 (the window's bottom) and scales it in Y by
    1.9 m (the window's height) x the fill fraction.
"""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, HALO, X, Y, Z, NX, NY, NZ  # noqa: E402

LIMIT = 2000
PYLON_R = 0.45  # ps_sim::base::PYLON_R


# ---------------------------------------------------------------------------------------------
# Terminals.

SCREEN_LO = (-0.25, 1.15)  # (z, y) of the screen face's lower edge
SCREEN_HI = (-0.08, 1.9)


def terminal(motif):
    g = L.Geo()
    L.box(g, (-0.5, 0.0, -0.25), (0.5, 0.1, 0.25))  # plinth
    L.box(g, (-0.42, 0.1, -0.15), (0.42, 0.98, 0.22))  # column
    # Head with the slanted screen face (prism along X over a (z, y) profile).
    prof = [(-0.25, 0.95), (0.25, 0.95), (0.25, 1.9), SCREEN_HI, SCREEN_LO]
    L.prism_x(g, prof, -0.5, 0.5)
    # Keyboard shelf.
    L.prism_x(g, [(-0.25, 0.98), (-0.15, 0.98), (-0.15, 1.12), (-0.25, 1.06)], -0.4, 0.4)
    # Screen.
    dz, dy = SCREEN_HI[0] - SCREEN_LO[0], SCREEN_HI[1] - SCREEN_LO[1]
    n = L.norm((0, dz, -dy))  # outward: forward (-Z) and up

    def sp(x, t, h=0.0):
        """Point on the screen face: x across, t = 0..1 up the slope."""
        return (x, SCREEN_LO[1] + dy * t, SCREEN_LO[0] + dz * t)

    P.disc(g, [sp(-0.38, 0.1), sp(0.38, 0.1), sp(0.38, 0.88), sp(-0.38, 0.88)], n, mat=LIGHT,
           glow=0.0)
    P.accent(g, [sp(-0.42, 0.06), sp(0.42, 0.06), sp(0.42, 0.92), sp(-0.42, 0.92)], n, 0.035,
             closed=True, glow=0.04)
    # Small motif on the screen (accent over the white, a little prouder).
    f = P.plane(sp(0, 0.6), n)
    draw_motif(g, f, motif, (0.0, 0.0), 0.12, h=0.016, glow=0.0)
    # Big motif on the column front.
    fc = P.plane((0, 0.56, -0.15), NZ)
    draw_motif(g, fc, motif, (0.0, 0.0), 0.3, h=L.ACCENT_H, glow=0.05)
    # Column side lines, plinth front light, keyboard glow.
    for s in (-1, 1):
        P.accent(g, [(s * 0.42, 0.2, -0.1), (s * 0.42, 0.9, -0.1)], (s, 0, 0), 0.03, glow=0.03)
        P.flow(g, [(s * 0.5, 1.05, 0.2), (s * 0.5, 1.8, 0.2)], (s, 0, 0), 0.02)
    P.white(g, [(-0.4, 0.05, -0.25), (0.4, 0.05, -0.25)], NZ, 0.03, glow=0.03)
    kq = [(-0.35, 1.06, -0.25), (0.35, 1.06, -0.25), (0.35, 1.12, -0.15), (-0.35, 1.12, -0.15)]
    kn = P.quad_normal(kq, (0, 1, -1))
    P.accent(g, [P.lerp(kq[0], kq[3], 0.5), P.lerp(kq[1], kq[2], 0.5)], kn, 0.03, glow=0.02)
    return g


def draw_motif(g, f, motif, c, s, h, glow):
    """Motif in frame f around (u, v) = c, scale s (half size in metres)."""
    cu, cv = c
    w = s * 0.12
    kw = dict(mat="accent", glow_mat="glow", h=h, sides=False)
    if motif == "medical":
        a = s * 0.3
        L.filled(g, f, L.rect(cu - a, cv - s, cu + a, cv + s), glow=glow, h=h)
        L.filled(g, f, L.rect(cu - s, cv - a, cu - a, cv + a), glow=glow, h=h)
        L.filled(g, f, L.rect(cu + a, cv - a, cu + s, cv + a), glow=glow, h=h)
    elif motif == "vehicle":
        for dv in (-0.45 * s, 0.25 * s):
            L.strip(g, f, L.chevron(cu, cv + dv + 0.5 * s, s, 0.55 * s, up=True), w * 1.4,
                    glow=(glow, glow), **kw)
    else:  # equipment: a rifle over an armour shield
        sh = [(cu - 0.55 * s, cv + 0.05 * s), (cu + 0.55 * s, cv + 0.05 * s),
              (cu + 0.55 * s, cv - 0.45 * s), (cu, cv - s), (cu - 0.55 * s, cv - 0.45 * s)]
        L.strip(g, f, L.ccw(sh), w, closed=True, glow=(glow, glow), **kw)
        y = cv + 0.5 * s
        for pts, k in (([(cu - s, y), (cu + s, y)], 1.6),  # barrel and receiver
                       ([(cu - s, y + 0.12 * s), (cu - s, y - 0.25 * s)], 1.3),  # butt
                       ([(cu - 0.4 * s, y), (cu - 0.5 * s, y - 0.3 * s)], 1.3),  # grip
                       ([(cu + 0.05 * s, y), (cu + 0.12 * s, y - 0.28 * s)], 1.3),  # magazine
                       ([(cu - 0.3 * s, y + 0.2 * s), (cu + 0.2 * s, y + 0.2 * s)], 1.0)):  # scope
            L.strip(g, f, pts, w * k, glow=(glow, glow), **kw)


# ---------------------------------------------------------------------------------------------
# Core console.

def console_core():
    g = L.Geo()
    R = 0.6
    hexa = [(R * math.cos(math.radians(60 * k)), R * math.sin(math.radians(60 * k)))
            for k in range(6)]  # (x, z); flats face +-Z
    ztop = lambda z: 1.0 + 0.1 * z / (R * math.cos(math.pi / 6))
    # Plinth step, then the slanted-top pedestal.
    P.lathe(g, (0, 0, 0), Y, [(0, 0.72), (0.08, 0.72), (0.1, 0.66)], n=6, phase=0.0, cap0=None)
    for k in range(6):
        a, b = hexa[k], hexa[(k + 1) % 6]
        q = [(a[0], 0.1, a[1]), (b[0], 0.1, b[1]), (b[0], ztop(b[1]), b[1]),
             (a[0], ztop(a[1]), a[1])]
        mid = ((a[0] + b[0]) / 2, 0, (a[1] + b[1]) / 2)
        g.face(q, "body", want=mid)
        # Vertical team line in the middle of each side face, white base light.
        m = ((a[0] + b[0]) / 2 * 1.0, (a[1] + b[1]) / 2)
        P.accent(g, [(m[0], 0.2, m[1]), (m[0], ztop(m[1]) - 0.12, m[1])], mid, 0.035,
                 glow=0.04)
    top = [(x, ztop(z), z) for x, z in hexa]
    tn = P.quad_normal(top[:4], (0, 1, -0.5))
    g.face(top, "body", want=tn)
    # Console: white screen and team frame on the slanted top, ring of keys.
    ins = lambda s: [(x * s, ztop(z * s), z * s) for x, z in hexa]
    P.accent(g, ins(0.85), tn, 0.03, closed=True, glow=0.04)
    scr = [(-0.3, ztop(-0.32), -0.32), (0.3, ztop(-0.32), -0.32), (0.3, ztop(0.02), 0.02),
           (-0.3, ztop(0.02), 0.02)]
    P.disc(g, scr, tn, mat=LIGHT, glow=0.03)
    P.flow(g, [(-0.3, ztop(0.2), 0.2), (0.3, ztop(0.2), 0.2)], tn, 0.025)
    # Emitter post at the back and the floating hologram.
    P.lathe(g, (0, ztop(0.3) - 0.05, 0.3), Y, [(0, 0.08), (0.2, 0.05)], n=6)
    P.disc(g, P.circle((0, ztop(0.3) + 0.15, 0.3), Y, 0.04, 6), Y, mat=LIGHT, glow=0.03)
    holo_ring(g, 1.5, 0.55, 0.05, tilt=0.0)
    holo_ring(g, 2.0, 0.42, 0.04, tilt=math.radians(20))
    # Crystal: hexagonal bipyramid 1.6..2.55 m.
    c, r, lo, hi = 2.0, 0.18, 1.6, 2.55
    mid = P.circle((0, c, 0), Y, r, 6, phase=0.0)
    for k in range(6):
        a, b = mid[k], mid[(k + 1) % 6]
        out = ((a[0] + b[0]) / 2, 0, (a[2] + b[2]) / 2)
        g.face([a, b, (0, hi, 0)], "accent", want=L.add(out, (0, 0.2, 0)))
        g.face([a, b, (0, lo, 0)], "accent", want=L.add(out, (0, -0.2, 0)))
    return g


def holo_ring(g, y, r, w, tilt):
    """Flat team ring (both faces) with halos inside and out; tilted about X by `tilt`."""
    n = 16
    def rot(p):
        yy, zz = p[1] - y, p[2]
        return (p[0], y + yy * math.cos(tilt) - zz * math.sin(tilt),
                zz * math.cos(tilt) + yy * math.sin(tilt))
    up = rot((0, y + 1, 0))
    nrm = L.sub(up, (0, y, 0))
    rings = {k: [rot(p) for p in P.circle((0, y, 0), Y, rr, n)]
             for k, rr in (("gi", r - w / 2 - 0.12), ("i", r - w / 2), ("o", r + w / 2),
                           ("go", r + w / 2 + 0.15))}
    for i in range(n):
        j = (i + 1) % n
        a, b = rings["i"], rings["o"]
        q = [a[i], a[j], b[j], b[i]]
        g.face(q, "accent", want=nrm)
        g.face(list(q), "accent", want=L.mul(nrm, -1))
        for inner, outer in (("i", "gi"), ("o", "go")):
            p0, p1 = rings[inner], rings[outer]
            seg = L.length(L.sub(p0[j], p0[i]))
            g.face([p0[i], p0[j], p1[j], p1[i]], "glow",
                   [(i * seg, 0), ((i + 1) * seg, 0), ((i + 1) * seg, 1), (i * seg, 1)],
                   want=nrm)


# ---------------------------------------------------------------------------------------------
# Wall turret.

def turret_base():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.95), (0.12, 0.95), (0.18, 0.75), (0.18, 0.55),
                              (1.05, 0.42), (1.05, 0.52), (1.2, 0.52)], n=8, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(1.08, 0.528), (1.13, 0.528)], n=8, mats="accent", cap0=None,
            cap1=None)
    for k in range(8):  # vertical lines on the column flats
        a = 2 * math.pi * k / 8
        d = (math.cos(a), 0, math.sin(a))
        r0 = 0.55 * math.cos(math.pi / 8)
        r1 = 0.42 * math.cos(math.pi / 8)
        p0 = L.add(L.mul(d, r0), (0, 0.24, 0))
        p1 = L.add(L.mul(d, r1), (0, 1.0, 0))
        nrm = L.norm(L.add(d, (0, (r0 - r1) / 0.87, 0)))
        if k % 2 == 0:
            P.accent(g, [p0, p1], nrm, 0.04, glow=0.04)
        else:
            P.flow(g, [p0, p1], nrm, 0.02)
    P.accent(g, P.circle((0, 1.2, 0), Y, 0.4, 8), Y, 0.03, closed=True, glow=0.04)
    return g


def turret_head():
    g = L.Geo()
    P.lathe(g, (0, -0.4, 0), Y, [(0, 0.42), (0.2, 0.36), (0.25, 0.3)], n=8)

    def hs(z, p):
        w, yb, yt = p
        return [(-w * 0.7, yb, z), (w * 0.7, yb, z), (w, (yb + yt) / 2, z), (w * 0.75, yt, z),
                (-w * 0.75, yt, z), (-w, (yb + yt) / 2, z)]
    rings = [hs(z, p) for z, p in ((0.55, (0.35, -0.15, 0.28)), (-0.1, (0.42, -0.22, 0.38)),
                                   (-0.5, (0.3, -0.12, 0.22)))]
    L.loft(g, rings, NZ)
    # Sensor eye on the front face.
    P.disc(g, [(0, -0.03, -0.5), (0.09, 0.05, -0.5), (0, 0.13, -0.5), (-0.09, 0.05, -0.5)],
           NZ, glow=0.05)
    # Top: team chevron; sides: lines.
    q = [rings[1][3], rings[1][4], rings[2][4], rings[2][3]]
    n = P.quad_normal(q, (0, 1, -0.5))
    P.accent(g, [P.on_quad(*q, 0.2, 0.75), P.on_quad(*q, 0.5, 0.25), P.on_quad(*q, 0.8, 0.75)],
             n, 0.04, glow=0.04)
    q = [rings[0][3], rings[0][4], rings[1][4], rings[1][3]]
    n = P.quad_normal(q, Y)
    P.flow(g, [P.on_quad(*q, 0.5, 0.1), P.on_quad(*q, 0.5, 0.9)], n, 0.025)
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.4, s * 0.62))
        L.box(g, (x0, -0.12, -0.55), (x1, 0.16, 0.4))
        xo = s * 0.62
        P.accent(g, [(xo, 0.06, -0.48), (xo, 0.06, 0.33)], (s, 0, 0), 0.04, glow=0.04)
        P.flow(g, [(xo, -0.05, -0.48), (xo, -0.05, 0.33)], (s, 0, 0), 0.02)
        o = (s * 0.5, 0.02, -0.55)
        P.lathe(g, o, NZ, [(0, 0.09), (0.15, 0.07), (0.65, 0.06), (0.65, 0.08), (0.85, 0.08),
                           (0.85, 0.045), (0.83, 0.045)], n=8, cap1=LIGHT)
        P.lathe(g, o, NZ, [(0.4, 0.066), (0.45, 0.066)], n=8, mats="accent", cap0=None,
                cap1=None)
    return g


# ---------------------------------------------------------------------------------------------
# Pads and spawn tube.

def pad_disc(g, R, n):
    P.lathe(g, (0, 0, 0), Y, [(0, R), (0.018, R), (0.03, R - 0.012)], n=n, cap0=None)
    return P.plane((0, 0.03, 0), Y)


def vehicle_pad():
    g = L.Geo()
    f = pad_disc(g, 7.0, 32)
    ring = lambda r, n=32: L.ngon(0, 0, r, n, 0.0)
    L.ring(g, f, ring(6.55), 0.16, glow_in=0.25, glow_out=0.2, sides=False)
    L.ring(g, f, ring(4.2), 0.06, glow_in=0.12, glow_out=0.12, sides=False)
    for k in range(16):  # tick marks between the rings
        a = 2 * math.pi * (k + 0.5) / 16
        d = (math.cos(a), math.sin(a))
        L.strip(g, f, [(d[0] * 5.0, d[1] * 5.0), (d[0] * 5.9, d[1] * 5.9)], 0.06,
                glow=(0.06, 0.06), sides=False)
    # Chevrons pointing -Z (frame v = -z: up=True points to -Z).
    for v in (-2.6, -1.0, 0.6, 2.2):
        L.strip(g, f, L.chevron(0, v + 0.9, 1.2, 0.9, up=True), 0.18, glow=(0.15, 0.15),
                sides=False)
    return g


def rearm_pad():
    g = L.Geo()
    f = pad_disc(g, 8.0, 32)
    ring = lambda r, n=32: L.ngon(0, 0, r, n, 0.0)
    kw = dict(mat=LIGHT, glow_mat=HALO, sides=False)
    L.ring(g, f, ring(7.5), 0.14, glow_in=0.2, glow_out=0.2, **kw)
    L.ring(g, f, ring(2.2, 16), 0.1, glow_in=0.15, glow_out=0.15, **kw)
    for r in (6.4, 5.2, 4.0, 3.0):
        L.strip(g, f, ring(r, 32 if r > 4 else 24), 0.05, closed=True, glow=(0, 0), mat="flow",
                h=0.004, sides=False)
    L.filled(g, f, L.ngon(0, 0, 0.9, 6, 0.0), glow=0.15, mat=LIGHT, glow_mat=HALO)
    for k in range(4):  # spokes of white dashes
        a = math.pi / 4 + k * math.pi / 2
        d = (math.cos(a), math.sin(a))
        for r0 in (2.7, 3.5, 4.6, 5.8):
            L.strip(g, f, [(d[0] * r0, d[1] * r0), (d[0] * (r0 + 0.35), d[1] * (r0 + 0.35))],
                    0.12, glow=(0.1, 0.1), **kw)
    # Light pylons at radius 9.2 m on the diagonals.
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        c = (9.2 * math.cos(a), 0, 9.2 * math.sin(a))
        inward = L.norm((-c[0], 0, -c[2]))
        P.lathe(g, c, Y, [(0, 0.3), (0.08, 0.3), (0.14, 0.1)], n=6, cap0=None)
        P.lathe(g, c, Y, [(0.14, 0.07), (2.85, 0.07)], n=4, phase=math.pi / 4 + a, cap0=None,
                cap1=None)
        P.lathe(g, L.add(c, (0, 2.85, 0)), Y, [(0, 0.13), (0.15, 0.1)], n=6)
        P.disc(g, P.circle(L.add(c, (0, 3.0, 0)), Y, 0.06, 6), Y, mat=LIGHT, glow=0.04)
        # White strip up the face looking at the pad (flats of the square face the radius).
        fo = L.add(c, L.mul(inward, 0.07 * math.cos(math.pi / 4)))
        P.white(g, [L.add(fo, (0, 0.3, 0)), L.add(fo, (0, 2.75, 0))], inward, 0.04, glow=0.03)
    return g


def spawn_tube():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.65), (0.03, 0.65), (0.04, 0.62)], n=16, cap0=None)
    f = P.plane((0, 0.04, 0), Y)
    L.ring(g, f, L.ngon(0, 0, 0.55, 16, 0.0), 0.03, glow_in=0.05, glow_out=0.04, sides=False)
    L.filled(g, f, L.ngon(0, 0, 0.3, 6, 0.0), glow=0.06, mat=LIGHT, glow_mat=HALO)
    L.ring(g, f, L.ngon(0, 0, 0.42, 16, 0.0), 0.012, glow_in=0, glow_out=0, mat="flow",
           sides=False, h=0.004)
    # Posts at x = +-0.68 (outside the disc, the walk-through axis is Z).
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.65, s * 0.71))
        L.box(g, (x0, 0.0, -0.03), (x1, 2.32, 0.03))
        xi = s * 0.65
        P.white(g, [(xi, 0.15, 0), (xi, 2.2, 0)], (-s, 0, 0), 0.02, glow=0.02)
    # Overhead ring: 16-gon box section 0.58..0.74 m, 2.28..2.4 m.
    y0, y1, ri, ro, n = 2.28, 2.4, 0.58, 0.74, 16
    P.lathe(g, (0, y0, 0), Y, [(0, ro), (y1 - y0, ro)], n=n, cap0=None, cap1=None)
    a, b = P.circle((0, y0, 0), Y, ri, n), P.circle((0, y1, 0), Y, ri, n)
    for i in range(n):
        j = (i + 1) % n
        q = [a[i], a[j], b[j], b[i]]
        mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
        g.face(q, "body", want=(-mid[0], 0, -mid[2]))
    for y, want in ((y0, NY), (y1, Y)):
        a, b = P.circle((0, y, 0), Y, ri, n), P.circle((0, y, 0), Y, ro, n)
        for i in range(n):
            j = (i + 1) % n
            g.face([a[i], a[j], b[j], b[i]], "body", want=want)
    rm = (ri + ro) / 2 / math.cos(math.pi / n)
    P.accent(g, P.circle((0, y0, 0), Y, rm, n), NY, 0.04, closed=True, glow=0.04)
    P.flow(g, P.circle((0, y1, 0), Y, rm, n), Y, 0.02, closed=True)
    P.white(g, P.circle((0, y0, 0), Y, ri + 0.025, n), NY, 0.015, closed=True, glow=0.0)
    return g


# ---------------------------------------------------------------------------------------------
# Heavy wall turret.

def wall_turret_base():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 1.3), (0.18, 1.3), (0.28, 1.05), (0.28, 0.82),
                              (1.6, 0.68), (1.6, 0.8), (1.8, 0.8)], n=8, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(1.64, 0.81), (1.7, 0.81)], n=8, mats="accent", cap0=None,
            cap1=None)
    P.lathe(g, (0, 0, 0), Y, [(0.08, 1.31), (0.13, 1.31)], n=8, mats="accent", cap0=None,
            cap1=None)
    for k in range(8):  # vertical lines on the column flats, armour ribs between
        a = 2 * math.pi * k / 8
        d = (math.cos(a), 0, math.sin(a))
        r0 = 0.82 * math.cos(math.pi / 8)
        r1 = 0.68 * math.cos(math.pi / 8)
        p0 = L.add(L.mul(d, r0), (0, 0.4, 0))
        p1 = L.add(L.mul(d, r1), (0, 1.5, 0))
        nrm = L.norm(L.add(d, (0, (r0 - r1) / 1.32, 0)))
        if k % 2 == 0:
            P.accent(g, [p0, p1], nrm, 0.06, glow=0.06)
        else:
            P.flow(g, [p0, p1], nrm, 0.03)
    P.accent(g, P.circle((0, 1.8, 0), Y, 0.6, 8), Y, 0.04, closed=True, glow=0.05)
    P.disc(g, P.circle((0, 1.8, 0), Y, 0.2, 8), Y, mat=LIGHT, glow=0.05)
    return g


def wall_turret_head():
    g = L.Geo()
    # Turntable on the pedestal top (local y = -0.6) and the floor deck.
    P.lathe(g, (0, -0.6, 0), Y, [(0, 0.8), (0.12, 0.8), (0.18, 1.0)], n=8, cap0="body")
    P.lathe(g, (0, -0.6, 0), Y, [(0.04, 0.81), (0.09, 0.81)], n=8, mats="accent", cap0=None,
            cap1=None)
    L.box(g, (-1.2, -0.42, -1.1), (1.2, -0.3, 1.3))
    # Armoured side pods (ammunition and drives), x 0.75..1.3.
    pod = [(-1.15, -0.42), (1.3, -0.42), (1.3, 0.1), (1.0, 0.42), (-0.55, 0.42), (-1.15, 0.1)]
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.75, s * 1.3))
        L.prism_x(g, pod, x0, x1)
        xo = s * 1.3
        P.accent(g, [(xo, 0.0, -1.0), (xo, 0.0, 1.15)], (s, 0, 0), 0.07, glow=0.06)
        P.flow(g, [(xo, -0.25, -1.0), (xo, -0.25, 1.15)], (s, 0, 0), 0.03)
        # Top of the pod: a team line, and slanted front with a white sensor slit.
        P.accent(g, [(s * 1.025, 0.42, -0.45), (s * 1.025, 0.42, 0.9)], Y, 0.06, glow=0.06)
        fq = [(x0, 0.1, -1.15), (x1, 0.1, -1.15), (x1, 0.42, -0.55), (x0, 0.42, -0.55)]
        fn = P.quad_normal(fq, (0, 1, -1))
        P.white(g, [P.on_quad(*fq, 0.2, 0.5), P.on_quad(*fq, 0.8, 0.5)], fn, 0.05, glow=0.04)
        # Gun: breech block x 0.4..0.7 between the pod and the cockpit, heavy barrel ahead.
        bx0, bx1 = sorted((s * 0.4, s * 0.72))
        L.prism_x(g, [(-1.25, -0.25), (0.15, -0.25), (0.15, 0.12), (-0.1, 0.25), (-1.25, 0.25)],
                  bx0, bx1)
        P.accent(g, [(s * 0.56, 0.25, -1.1), (s * 0.56, 0.25, -0.15)], Y, 0.05, glow=0.04)
        o = (s * 0.55, 0.0, -1.25)
        P.lathe(g, o, NZ, [(0, 0.2), (0.12, 0.15), (0.75, 0.13), (0.75, 0.17), (1.0, 0.17),
                           (1.0, 0.13), (1.12, 0.13), (1.12, 0.19), (1.35, 0.19), (1.35, 0.1),
                           (1.33, 0.1)], n=8, cap1=LIGHT)
        for t, r in ((0.45, 0.138), (1.18, 0.198)):
            P.lathe(g, o, NZ, [(t, r), (t + 0.06, r)], n=8, mats="accent", cap0=None,
                    cap1=None)
    # Front armour between the guns, below the gunner's line of sight.
    L.prism_x(g, [(-1.2, -0.3), (-0.55, -0.3), (-0.55, 0.12), (-0.85, 0.2), (-1.2, 0.0)],
              -0.4, 0.4)
    fq = [(-0.4, 0.0, -1.2), (0.4, 0.0, -1.2), (0.4, 0.2, -0.85), (-0.4, 0.2, -0.85)]
    fn = P.quad_normal(fq, (0, 0.5, -1))
    P.accent(g, [P.on_quad(*fq, 0.2, 0.3), P.on_quad(*fq, 0.5, 0.75), P.on_quad(*fq, 0.8, 0.3)],
             fn, 0.05, glow=0.05)
    # Cockpit: seat, backrest, rear coaming; open on top.
    L.box(g, (-0.3, -0.3, 0.5), (0.3, -0.12, 1.0))
    L.box(g, (-0.3, -0.3, 1.0), (0.3, 0.42, 1.12))
    L.box(g, (-0.75, -0.3, 1.12), (0.75, 0.05, 1.3))
    P.accent(g, [(-0.7, 0.05, 1.21), (0.7, 0.05, 1.21)], Y, 0.04, glow=0.05)
    P.white(g, [(-0.2, 0.3, 1.0), (0.2, 0.3, 1.0)], NZ, 0.03, glow=0.03)
    # Hand grips on a console in front of the seat, gunsight ring on a post.
    L.box(g, (-0.35, -0.3, 0.05), (0.35, -0.05, 0.25))
    P.accent(g, [(-0.3, -0.05, 0.15), (0.3, -0.05, 0.15)], Y, 0.03, glow=0.03)
    for s in (-1, 1):
        P.tube(g, (s * 0.25, -0.05, 0.18), (s * 0.25, 0.12, 0.22), 0.03, n=6)
        P.tube(g, (s * 0.25, 0.12, 0.22), (s * 0.25, 0.16, 0.23), 0.032, n=6, mat="accent")
    P.tube(g, (0, -0.05, 0.05), (0, 0.42, -0.25), 0.025, n=4)
    sight = P.circle((0, 0.52, -0.3), Z, 0.09, 12)
    P.white(g, sight, Z, 0.015, closed=True, glow=0.0)
    return g


# ---------------------------------------------------------------------------------------------
# Generator.

def generator():
    g = L.Geo()
    L.prism_y(g, L.octagon_xz(2.0, 2.0, 0.3), 0.0, 0.25)
    P.accent(g, [(-1.6, 0.13, -2.0), (1.6, 0.13, -2.0)], NZ, 0.05, glow=0.05)
    P.accent(g, [(-1.6, 0.13, 2.0), (1.6, 0.13, 2.0)], Z, 0.05, glow=0.05)
    # Base and cap drums of the reactor chamber.
    P.lathe(g, (0, 0.25, 0), Y, [(0, 1.3), (0.4, 1.3), (0.5, 1.15)], n=12, cap0=None)
    P.lathe(g, (0, 2.45, 0), Y, [(0, 1.15), (0.1, 1.3), (0.6, 1.3), (0.75, 0.9), (0.85, 0.5)],
            n=12)
    P.lathe(g, (0, 2.45, 0), Y, [(0.85, 0.15), (1.05, 0.06)], n=6, mats="accent", cap0=None)
    for y, mat in ((0.45, LIGHT), (2.7, "accent")):  # rings round the drums
        P.lathe(g, (0, y, 0), Y, [(0, 1.31), (0.06, 1.31)], n=12, mats=mat, cap0=None,
                cap1=None)
    P.white(g, P.circle((0, 0.75, 0), Y, 0.95, 12), Y, 0.04, closed=True, glow=0.04)
    P.white(g, P.circle((0, 2.45, 0), NY, 0.95, 12), NY, 0.04, closed=True, glow=0.04)
    # Cage struts with white lines facing out.
    for k in range(6):
        a = 2 * math.pi * k / 6
        d = (math.cos(a), 0, math.sin(a))
        c = L.mul(d, 1.12)
        t = (-d[2], 0, d[0])
        foot = [L.add(c, L.add(L.mul(d, -0.07), L.mul(t, -0.07))),
                L.add(c, L.add(L.mul(d, 0.07), L.mul(t, -0.07))),
                L.add(c, L.add(L.mul(d, 0.07), L.mul(t, 0.07))),
                L.add(c, L.add(L.mul(d, -0.07), L.mul(t, 0.07)))]
        P.extrude(g, [L.add(p, (0, 0.75, 0)) for p in foot], (0, 1.7, 0), caps=(False, False))
        if k not in (0, 3):  # (those two stand inside the machine blocks)
            o = L.add(c, L.mul(d, 0.07))
            P.white(g, [L.add(o, (0, 0.9, 0)), L.add(o, (0, 2.3, 0))], d, 0.03, glow=0.03)
    # Machine blocks either side, vent grilles on their outer faces.
    prof = [(-1.6, 0.25), (1.6, 0.25), (1.6, 2.3), (1.3, 2.6), (-1.3, 2.6), (-1.6, 2.3)]
    for s in (-1, 1):
        x0, x1 = sorted((s * 1.25, s * 2.0))
        L.prism_x(g, prof, x0, x1)
        xo = s * 2.0
        for k in range(5):
            y = 0.75 + 0.3 * k
            vx0, vx1 = sorted((xo, xo - s * 0.04))
            L.box(g, (vx0, y, -1.15), (vx1, y + 0.12, 1.15))
            P.flow(g, [(xo, y + 0.18, -1.1), (xo, y + 0.18, 1.1)], (s, 0, 0), 0.03)
        P.accent(g, [(xo, 0.55, -1.3), (xo, 0.55, 1.3), (xo, 2.15, 1.3), (xo, 2.15, -1.3)],
                 (s, 0, 0), 0.06, closed=True, glow=0.06)
        # Core window frame: team line up the block's front and back faces by the chamber.
        for zf, n in ((-1.6, NZ), (1.6, Z)):
            xi = s * 1.4
            P.accent(g, [(xi, 0.5, zf), (xi, 2.4, zf)], n, 0.08, glow=0.08)
            P.white(g, [(s * 1.75, 0.6, zf), (s * 1.75, 1.6, zf)], n, 0.05, glow=0.04)
        # Top: status light and coolant pipes arching into the cap.
        P.accent(g, [(s * 1.62, 2.6, -1.2), (s * 1.62, 2.6, 1.2)], Y, 0.05, glow=0.05)
        for z in (-0.7, 0.7):
            pts = [(s * 1.65, 2.5, z), (s * 1.65, 3.15, z), (s * 0.85, 3.15, z),
                   (s * 0.85, 2.95, z)]
            for a, b in zip(pts, pts[1:]):
                P.tube(g, a, b, 0.1, n=6)
            for p in pts[1:3]:
                P.lathe(g, L.add(p, (0, -0.13, 0)), Y, [(0, 0.13), (0.26, 0.13)], n=6)
            P.tube(g, (s * 1.65, 2.75, z), (s * 1.65, 2.85, z), 0.115, n=6, mat="accent")
    return g


def generator_core():
    g = L.Geo()
    r, y0, y1, n = 0.6, 0.8, 2.4, 16
    prof = []
    for k in range(5):  # lower hemisphere
        t = -math.pi / 2 + math.pi / 2 * k / 4
        prof.append((y0 + r + r * math.sin(t), max(r * math.cos(t), 0.0)))
    for k in range(5):  # upper hemisphere
        t = math.pi / 2 * k / 4
        prof.append((y1 - r + r * math.sin(t), r * math.cos(t)))
    prof = [(t, rr) for t, rr in prof if rr > 1e-6]
    P.lathe(g, (0, 0, 0), Y, prof, n=n, mats=LIGHT, cap0=None, cap1=None)
    # Poles.
    lo, hi = P.circle((0, prof[0][0], 0), Y, prof[0][1], n), P.circle((0, prof[-1][0], 0), Y,
                                                                      prof[-1][1], n)
    for i in range(n):
        j = (i + 1) % n
        g.face([lo[i], lo[j], (0, y0, 0)], LIGHT, want=NY)
        g.face([hi[i], hi[j], (0, y1, 0)], LIGHT, want=Y)
    return g


def capture_pylon():
    """A capture point's pylon: a hexagonal mast tapering up from a plinth, team lines up its
    faces, a team crown ring and a white beacon on top."""
    g = L.Geo()
    R = PYLON_R
    P.lathe(g, (0, 0, 0), Y, [(0, R + 0.25), (0.25, R + 0.2), (0.35, R)], n=6, phase=0.0,
            cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(0.35, R), (3.5, R * 0.6), (3.7, R * 0.8), (3.9, R * 0.5)],
            n=6, phase=0.0, cap0=None)
    # (The mast's corner radius at height y; a face's middle is at that times the edge's
    # middle on the unit hexagon.)
    rad = lambda y: R * (1.0 - 0.4 * (y - 0.35) / 3.15)
    hexa = P.circle((0, 0, 0), Y, 1.0, 6, phase=0.0)
    for k in range(6):
        a, b = hexa[k], hexa[(k + 1) % 6]
        m = ((a[0] + b[0]) / 2, (a[2] + b[2]) / 2)
        out = (m[0], 0.0, m[1])
        P.accent(g, [(m[0] * rad(y), y, m[1] * rad(y)) for y in (0.5, 3.3)], out, 0.05,
                 glow=0.05)
    holo_ring(g, 3.75, 0.62, 0.07, tilt=0.0)
    P.disc(g, P.circle((0, 3.9 + L.ACCENT_H, 0), Y, R * 0.42, 6, phase=0.0), Y, mat=LIGHT,
           glow=0.04)
    return g


# ---------------------------------------------------------------------------------------------
# Resource silo, fill and node.

SILO_N = 20
SILO_WINDOWS = (2, 7, 12, 17)  # segments (18 degrees each from +X) left open, X symmetric


def arc_box(g, r0, r1, y0, y1, segs, n=SILO_N, mat="body", top=True, bottom=True, inner=True,
            ends=True):
    """Annular box section over the n-gon segments `segs` (vertex k at angle 2 pi k / n), with
    end walls where a neighbouring segment is not in the set."""
    segs = set(segs)
    pt = lambda r, k, y: (r * math.cos(2 * math.pi * k / n), y, r * math.sin(2 * math.pi * k / n))
    for i in segs:
        j = i + 1
        am = 2 * math.pi * (i + 0.5) / n
        rad = (math.cos(am), 0, math.sin(am))
        g.face([pt(r1, i, y0), pt(r1, j, y0), pt(r1, j, y1), pt(r1, i, y1)], mat, want=rad)
        if inner:
            g.face([pt(r0, i, y0), pt(r0, j, y0), pt(r0, j, y1), pt(r0, i, y1)], mat,
                   want=L.mul(rad, -1))
        if top:
            g.face([pt(r0, i, y1), pt(r0, j, y1), pt(r1, j, y1), pt(r1, i, y1)], mat, want=Y)
        if bottom:
            g.face([pt(r0, i, y0), pt(r0, j, y0), pt(r1, j, y0), pt(r1, i, y0)], mat, want=NY)
        for k, other, sgn in ((i, (i - 1) % n, -1), (j, j % n, 1)):
            if other in segs or not ends:
                continue
            a = 2 * math.pi * k / n
            tang = (-math.sin(a) * sgn, 0, math.cos(a) * sgn)
            g.face([pt(r0, k, y0), pt(r1, k, y0), pt(r1, k, y1), pt(r0, k, y1)], mat, want=tang)


def resource_silo():
    g = L.Geo()
    n = SILO_N
    allseg = range(n)
    solid = [i for i in allseg if i not in SILO_WINDOWS]
    RO, RI, TOP = 3.85, 3.7, 11.4
    # Plinth ring, wall (window slots y 1.2..10.8), ribs.
    arc_box(g, RO, 4.0, 0.0, 0.6, allseg, bottom=False, inner=False)
    arc_box(g, RI, RO, 0.0, TOP, solid, top=False, bottom=False)
    arc_box(g, RI, RO, 0.0, 1.2, SILO_WINDOWS, bottom=False)
    arc_box(g, RI, RO, 10.8, TOP, SILO_WINDOWS, top=False)
    for y in (2.6, 5.0, 7.4, 9.8):
        arc_box(g, RO, 3.98, y - 0.15, y + 0.15, solid, inner=False)
        # Team band: the rib's outer face, 1 cm proud, between y -+ 0.06.
        arc_box(g, 3.99, 3.99, y - 0.06, y + 0.06, solid, mat="accent", top=False,
                bottom=False, inner=False, ends=False)
    # Team frames along the window jambs (on the neighbouring segments' outer faces).
    for w in SILO_WINDOWS:
        # k: the jamb's vertex; seg: the solid segment beside it; far: its other vertex.
        for k, seg, far_k in ((w, w - 1, w - 1), (w + 1, w + 1, w + 2)):
            a = 2 * math.pi * k / n
            am = 2 * math.pi * (seg + 0.5) / n
            nrm = (math.cos(am), 0, math.sin(am))
            # Line on the plane of segment `seg`, 0.15 m from the jamb.
            jamb = (RO * math.cos(a), 0, RO * math.sin(a))
            fa = 2 * math.pi * far_k / n
            far = (RO * math.cos(fa), 0, RO * math.sin(fa))
            base = L.add(jamb, L.mul(L.norm(L.sub(far, jamb)), 0.15))
            segs = [(0.8, 2.45), (2.75, 4.85), (5.15, 7.25), (7.55, 9.65), (9.95, 11.2)]
            for y0, y1 in segs:
                P.accent(g, [(base[0], y0, base[2]), (base[0], y1, base[2])], nrm, 0.08,
                         glow=0.0, h=0.008)
    # Roof: a low cone over the wall with a hatch and white beacon.
    P.lathe(g, (0, TOP, 0), Y, [(0, 4.0), (0.25, 4.0), (0.45, 3.4), (0.6, 1.0)], n=n,
            cap0="body", cap1="body", phase=0.0)
    P.lathe(g, (0, TOP, 0), Y, [(0.08, 4.008), (0.17, 4.008)], n=n, mats="accent", cap0=None,
            cap1=None, phase=0.0)
    P.accent(g, P.circle((0, 12.0, 0), Y, 0.7, 8), Y, 0.06, closed=True, glow=0.06)
    P.disc(g, P.circle((0, 12.0, 0), Y, 0.25, 8), Y, mat=LIGHT, glow=0.06)
    # Ladder up the front (-Z), standing off the ribs.
    zr = -4.18
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.25, s * 0.3))
        L.box(g, (x0, 0.0, zr - 0.04), (x1, 12.3, zr + 0.04))
        for y in (1.0, 6.0, 11.0):
            L.box(g, (x0, y, zr + 0.04), (x1, y + 0.08, -3.84))
        P.flow(g, [(s * 0.3, 0.2, zr), (s * 0.3, 12.1, zr)], (s, 0, 0), 0.025, h=0.003)
    for k in range(34):
        y = 0.35 + 0.35 * k
        L.box(g, (-0.25, y, zr - 0.025), (0.25, y + 0.04, zr + 0.025), skip=("-x", "+x"))
    P.white(g, [(-0.2, 12.3, zr), (0.2, 12.3, zr)], Y, 0.03, glow=0.0)
    return g


def resource_silo_fill():
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 3.6), (1.0, 3.6)], n=SILO_N, mats="accent", cap0="accent",
            cap1="accent", phase=0.0)
    return g


def crystal(g, base, axis, length, r, n=6, tip=0.3, twist=0.0):
    """Faceted crystal: n-gon prism from `base` along `axis`, a pointed tip of `tip` * length."""
    a = L.norm(axis)
    body = length * (1 - tip)
    rings = [P.circle(base, a, r * 0.85, n, phase=twist),
             P.circle(L.add(base, L.mul(a, body * 0.5)), a, r, n, phase=twist),
             P.circle(L.add(base, L.mul(a, body)), a, r * 0.92, n, phase=twist)]
    L.loft(g, rings, a, mats="resource", cap0="resource")
    apex = L.add(base, L.mul(a, length))
    top = rings[-1]
    for i in range(n):
        j = (i + 1) % n
        mid = L.mul(L.add(top[i], top[j]), 0.5)
        out = L.add(L.sub(mid, L.add(base, L.mul(a, body))), L.mul(a, r))
        g.face([top[i], top[j], apex], "resource", want=out)


def boulder(g, c, rx, ry, rz, rng, n=7):
    rows = [(-0.3, 0.75), (0.25, 1.0), (0.7, 0.75), (1.0, 0.3)]
    rings = []
    for t, s in rows:
        ring = []
        for i in range(n):
            a = 2 * math.pi * i / n + rng.uniform(-0.15, 0.15)
            k = s * rng.uniform(0.85, 1.12)
            ring.append((c[0] + rx * k * math.cos(a), c[1] + ry * t + rng.uniform(-0.04, 0.04) * ry,
                         c[2] + rz * k * math.sin(a)))
        rings.append(ring)
    # (Rows jitter in height, so their faces are split into triangles.)
    cs = [L.mul(tuple(map(sum, zip(*r))), 1.0 / n) for r in rings]
    for k in range(len(rings) - 1):
        for i in range(n):
            j = (i + 1) % n
            a, b, cc, d = rings[k][i], rings[k][j], rings[k + 1][j], rings[k + 1][i]
            mid = L.mul(L.add(cs[k], cs[k + 1]), 0.5)
            for tri in ((a, b, cc), (a, cc, d)):
                tm = L.mul(tuple(map(sum, zip(*tri))), 1 / 3)
                g.face(list(tri), "body", want=L.sub(tm, mid))
    top = rings[-1]
    for i in range(n):
        j = (i + 1) % n
        tm = L.mul(L.add(top[i], top[j]), 0.5)
        g.face([top[i], top[j], cs[-1]], "body", want=L.sub(L.add(tm, (0, 0.3 * ry, 0)), cs[-2]))


def resource_node():
    g = L.Geo()
    rng = random.Random(7)
    for c, rx, ry, rz in (((0.0, 0.0, 0.0), 1.5, 0.55, 1.3), ((1.1, 0.0, 0.7), 0.8, 0.4, 0.7),
                          ((-1.2, 0.0, -0.4), 0.75, 0.45, 0.8), ((0.4, 0.0, -1.2), 0.65, 0.3, 0.6),
                          ((-0.6, 0.0, 1.2), 0.6, 0.35, 0.55)):
        boulder(g, c, rx, ry, rz, rng)
    # Crystals: (base, lean direction (x, z), lean, length, radius).
    spec = [((0.0, 0.35, 0.0), (0.0, 0.0), 0.0, 3.1, 0.38),
            ((0.35, 0.3, 0.25), (1.0, 0.4), 0.45, 2.2, 0.28),
            ((-0.35, 0.3, 0.2), (-1.0, 0.6), 0.5, 2.0, 0.26),
            ((0.1, 0.3, -0.4), (0.2, -1.0), 0.45, 2.3, 0.27),
            ((-0.3, 0.3, -0.3), (-0.8, -0.9), 0.75, 1.5, 0.2),
            ((1.1, 0.2, 0.7), (0.8, 0.6), 0.6, 1.3, 0.2),
            ((-1.2, 0.25, -0.4), (-1.0, -0.2), 0.55, 1.4, 0.22),
            ((0.45, 0.15, -1.15), (0.3, -1.0), 0.8, 0.9, 0.15),
            ((-0.6, 0.15, 1.2), (-0.4, 1.0), 0.7, 1.0, 0.16),
            ((0.6, 0.35, -0.1), (1.0, -0.6), 1.0, 1.1, 0.16)]
    for base, (lx, lz), lean, length, r in spec:
        d = L.norm((lx, 0, lz)) if lx or lz else (0, 0, 0)
        axis = L.norm(L.add((0, 1, 0), L.mul(d, lean)))
        crystal(g, base, axis, length, r, twist=rng.uniform(0, math.pi))
    return g


# ---------------------------------------------------------------------------------------------

# ---------------------------------------------------------------------------------------------
# Engineer deployables.

SPITFIRE_PIVOT = 0.9  # the head's pivot above the ground (the base's turntable tops out 4 cm below)


def spitfire_base():
    """The Spitfire turret's tripod: three splayed legs on foot pads, a hub, a post and the
    turntable the head sits on."""
    g = L.Geo()
    hub = 0.55
    P.lathe(g, (0, hub - 0.06, 0), Y, [(0, 0.07), (0.12, 0.07)], n=8)
    P.lathe(g, (0, hub - 0.06, 0), Y, [(0.05, 0.073), (0.062, 0.073)], n=8, mats="accent",
            cap0=None, cap1=None)
    P.lathe(g, (0, hub + 0.06, 0), Y, [(0, 0.042), (0.2, 0.042)], n=8, cap0=None)
    P.lathe(g, (0, 0.8, 0), Y, [(0, 0.08), (0.02, 0.11), (0.06, 0.11)], n=12, cap0="body")
    P.lathe(g, (0, 0.8, 0), Y, [(0.033, 0.113), (0.045, 0.113)], n=12, mats="accent",
            cap0=None, cap1=None)
    # White ready light on the hub's front flat.
    zf = -0.07 * math.cos(math.pi / 8)
    P.disc(g, [(-0.018, hub - 0.03, zf), (0.018, hub - 0.03, zf), (0.018, hub + 0.03, zf),
               (-0.018, hub + 0.03, zf)], NZ, mat=LIGHT, glow=0.012, h=0.002)
    R = 0.4
    for dx, dz in ((0.0, 1.0), (math.sin(math.pi / 3), -0.5), (-math.sin(math.pi / 3), -0.5)):
        top = (dx * 0.05, hub, dz * 0.05)
        foot = (dx * R, 0.05, dz * R)
        P.tube(g, top, foot, 0.026, n=6)
        d = L.sub(foot, top)
        P.lathe(g, top, d, [(0.3, 0.029), (0.33, 0.029)], n=6, mats="accent", cap0=None,
                cap1=None)
        P.lathe(g, (foot[0], 0.0, foot[2]), Y, [(0, 0.055), (0.03, 0.055), (0.05, 0.035)], n=6,
                cap0=None)
    return g


def spitfire_head():
    """The Spitfire's gun head, ORIGIN AT ITS PIVOT: a wedge housing with ammo boxes on its
    flanks, a sensor eye on top and two short barrels along -Z."""
    g = L.Geo()
    P.lathe(g, (0, -0.04, 0), Y, [(0, 0.08), (0.04, 0.08)], n=8, cap1=None)

    def hs(z, p):
        w, yb, yt = p
        return [(-w * 0.7, yb, z), (w * 0.7, yb, z), (w, (yb + yt) / 2, z), (w * 0.75, yt, z),
                (-w * 0.75, yt, z), (-w, (yb + yt) / 2, z)]
    rings = [hs(z, p) for z, p in ((0.16, (0.1, -0.02, 0.09)), (0.0, (0.14, -0.04, 0.12)),
                                   (-0.14, (0.1, -0.03, 0.08)))]
    L.loft(g, rings, NZ, cap0="body", cap1="body")
    # Top: team chevron behind the eye, flow line down the back.
    q = [rings[0][3], rings[0][4], rings[1][4], rings[1][3]]
    n = P.quad_normal(q, (0, 1, 0.3))
    P.accent(g, [P.on_quad(*q, 0.2, 0.3), P.on_quad(*q, 0.5, 0.75), P.on_quad(*q, 0.8, 0.3)],
             n, 0.012, glow=0.012)
    # Sensor eye: a small block on top with a team lens and a white glint.
    L.box(g, (-0.04, 0.115, -0.11), (0.04, 0.165, -0.02))
    P.disc(g, [(0.022 * math.cos(a), 0.14 + 0.018 * math.sin(a), -0.11)
               for a in [math.radians(60 * k) for k in range(6)]], NZ, glow=0.01, h=0.002)
    P.white(g, [(-0.025, 0.165, -0.08), (0.025, 0.165, -0.08)], Y, 0.006, glow=0.0, h=0.0015)
    # Ammo boxes on the flanks.
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.13, s * 0.2))
        L.box(g, (x0, -0.03, -0.08), (x1, 0.09, 0.12))
        P.accent(g, [(s * 0.2, 0.05, -0.06), (s * 0.2, 0.05, 0.1)], (s, 0, 0), 0.012,
                 glow=0.012)
        P.flow(g, [(s * 0.2, 0.0, -0.06), (s * 0.2, 0.0, 0.1)], (s, 0, 0), 0.006)
    # Twin barrels: shroud with a team ring, thin barrel, flared muzzle with a white bore.
    for s in (-1, 1):
        o = (s * 0.06, 0.0, -0.1)
        P.lathe(g, o, NZ, [(0, 0.032), (0.14, 0.032), (0.16, 0.022)], n=8, cap0=None)
        P.lathe(g, o, NZ, [(0.09, 0.035), (0.105, 0.035)], n=8, mats="accent", cap0=None,
                cap1=None)
        P.lathe(g, o, NZ, [(0.16, 0.014), (0.31, 0.014), (0.31, 0.021), (0.34, 0.021)], n=8,
                cap0=None, cap1=LIGHT)
    return g


def at_mine():
    """Anti-tank mine: a flat bevelled disc with a team ring and a small pressure plate."""
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, 0.32), (0.025, 0.35), (0.07, 0.35), (0.1, 0.3)], n=12,
            cap0="body")
    P.lathe(g, (0, 0, 0), Y, [(0.04, 0.353), (0.052, 0.353)], n=12, mats="accent", cap0=None,
            cap1=None)
    f = P.plane((0, 0.1, 0), Y)
    L.ring(g, f, L.ngon(0, 0, 0.22, 12, 0.0), 0.025, glow_in=0.03, glow_out=0.04, sides=False)
    # Pressure plate: a raised hexagonal button, a white centre.
    P.lathe(g, (0, 0.1, 0), Y, [(0, 0.1), (0.015, 0.1), (0.022, 0.085)], n=6, cap0=None)
    P.disc(g, [(0.03 * math.cos(a), 0.122, -0.03 * math.sin(a))
               for a in [math.radians(30 + 60 * k) for k in range(6)]], Y, mat=LIGHT,
           glow=0.0, h=0.002)
    # Four arming studs round the ring.
    for a in (45, 135, 225, 315):
        c = (0.28 * math.cos(math.radians(a)), 0.1, 0.28 * math.sin(math.radians(a)))
        P.lathe(g, c, Y, [(0, 0.018), (0.012, 0.018)], n=6, cap0=None)
    return g


def ammo_pack():
    """Engineer's ammo supply crate: a chamfered box with a lid, carry handles at both ends,
    a three-round ammo symbol front and back and a white light strip on the lid."""
    g = L.Geo()
    L.prism_x(g, [(-0.2, 0.03), (-0.17, 0.0), (0.17, 0.0), (0.2, 0.03), (0.2, 0.33),
                  (0.17, 0.355), (-0.17, 0.355), (-0.2, 0.33)], -0.27, 0.27)
    L.prism_x(g, [(-0.18, 0.35), (0.18, 0.35), (0.18, 0.38), (0.15, 0.4), (-0.15, 0.4),
                  (-0.18, 0.38)], -0.25, 0.25)  # lid
    # Lid seam: team line round the front and back.
    for zs in (-1, 1):
        P.accent(g, [(-0.25, 0.33, zs * 0.2), (0.25, 0.33, zs * 0.2)], (0, 0, zs), 0.008,
                 glow=0.012)
    P.white(g, [(-0.18, 0.4, 0), (0.18, 0.4, 0)], Y, 0.025, glow=0.02, h=0.002)
    # Handles on both ends.
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.27, s * 0.3))
        for zz in (-0.09, 0.07):
            L.box(g, (x0, 0.23, zz), (x1, 0.27, zz + 0.02))
        xa, xb = sorted((s * 0.285, s * 0.3))
        L.box(g, (xa, 0.23, -0.07), (xb, 0.27, 0.07))
        P.accent(g, [(s * 0.27, 0.1, -0.12), (s * 0.27, 0.1, 0.12)], (s, 0, 0), 0.01, glow=0.012)
    # Ammo symbol: three rounds, front and back.
    for zs in (-1, 1):
        z = zs * 0.2
        for x in (-0.07, 0.0, 0.07):
            P.disc(g, [(x - 0.018, 0.08, z), (x + 0.018, 0.08, z), (x + 0.018, 0.2, z),
                       (x, 0.245, z), (x - 0.018, 0.2, z)], (0, 0, zs), glow=0.012)
        P.accent(g, [(-0.1, 0.06, z), (0.1, 0.06, z)], (0, 0, zs), 0.008, glow=0.01)
    return g


# ---------------------------------------------------------------------------------------------
# Field fortifications (engineer construction). Each fills its collider box exactly; accents
# stand at most 8 mm proud of faces inside the box.

def chamfer_post(x0, x1, z0, z1, c, inner=(True, True)):
    """(x, z) footprint of a post with its vertical edges chamfered by c (only on the ends
    `inner` says: left at x0, right at x1)."""
    cl, cr = (c if inner[0] else 0.0), (c if inner[1] else 0.0)
    pts = [(x0, z0 + cl), (x0 + cl, z0), (x1 - cr, z0), (x1, z0 + cr), (x1, z1 - cr),
           (x1 - cr, z1), (x0 + cl, z1), (x0, z1 - cl)]
    out = []
    for p in pts:
        if not out or abs(p[0] - out[-1][0]) > 1e-9 or abs(p[1] - out[-1][1]) > 1e-9:
            out.append(p)
    if abs(out[0][0] - out[-1][0]) < 1e-9 and abs(out[0][1] - out[-1][1]) < 1e-9:
        out.pop()
    return out


WALL_BAYS = ((-2.72, -1.12), (-0.88, 0.88), (1.12, 2.72))


def build_wall():
    """Deployable barrier wall: exactly (-3, 0, -0.4)..(3, 3.5, 0.4). A chamfered plinth and
    coping, four buttress posts standing full depth, armour plates between them both sides."""
    g = L.Geo()
    HX, H, HZ = 3.0, 3.5, 0.4
    L.prism_x(g, [(-HZ, 0.0), (HZ, 0.0), (HZ, 0.3), (0.33, 0.37), (-0.33, 0.37), (-HZ, 0.3)],
              -HX, HX)
    L.box(g, (-HX, 0.37, -0.28), (HX, 3.24, 0.28))
    L.prism_x(g, [(-0.34, 3.24), (0.34, 3.24), (HZ, 3.29), (HZ, 3.42), (0.33, H), (-0.33, H),
                  (-HZ, 3.42), (-HZ, 3.29)], -HX, HX)
    posts = ((-HX, -2.72, (False, True)), (-1.12, -0.88, (True, True)),
             (0.88, 1.12, (True, True)), (2.72, HX, (True, False)))
    for x0, x1, inner in posts:
        L.prism_y(g, chamfer_post(x0, x1, -HZ, HZ, 0.05, inner), 0.3, 3.29)
    for zs in (-1, 1):
        n = (0, 0, zs)
        for x0, x1 in WALL_BAYS:
            # Lower plate upright, upper plate leaning back towards the coping.
            L.prism_x(g, [(zs * 0.28, 0.42), (zs * 0.37, 0.48), (zs * 0.37, 1.66),
                          (zs * 0.28, 1.72)], x0, x1)
            L.prism_x(g, [(zs * 0.28, 1.82), (zs * 0.37, 1.88), (zs * 0.37, 2.45),
                          (zs * 0.33, 3.14), (zs * 0.28, 3.2)], x0, x1)
            # Seam between the plates: white light; team tab on the lower plate.
            P.white(g, [(x0 + 0.08, 1.77, zs * 0.28), (x1 - 0.08, 1.77, zs * 0.28)], n, 0.035,
                    glow=0.025)
            xm = (x0 + x1) / 2
            for x in (x0 + 0.12, x1 - 0.12):  # team ticks in the lower plate's corners
                P.accent(g, [(x, 1.25, zs * 0.37), (x, 1.55, zs * 0.37)], n, 0.035, glow=0.03)
            P.flow(g, [(xm - 0.45, 0.62, zs * 0.37), (xm + 0.45, 0.62, zs * 0.37)], n, 0.02)
            # Upper plate: a chevron pointing up on the lower (upright) part.
            P.accent(g, [(xm - 0.35, 1.98, zs * 0.37), (xm, 2.3, zs * 0.37),
                          (xm + 0.35, 1.98, zs * 0.37)], n, 0.035, glow=0.03)
        # Neon along the top edge (the coping's face), seams down the buttresses.
        P.accent(g, [(-2.92, 3.355, zs * HZ), (2.92, 3.355, zs * HZ)], n, 0.04, glow=0.03)
        for x in (-1.0, 1.0):
            P.accent(g, [(x, 0.5, zs * HZ), (x, 3.12, zs * HZ)], n, 0.04, glow=0.03)
        for x in (-2.885, 2.885):
            P.flow(g, [(x, 0.5, zs * HZ), (x, 3.12, zs * HZ)], n, 0.025)
        P.flow(g, [(-2.9, 0.15, zs * HZ), (2.9, 0.15, zs * HZ)], n, 0.025)
    P.flow(g, [(-2.9, H, 0.0), (2.9, H, 0.0)], Y, 0.03)
    for s in (-1, 1):  # ends: a team line up the middle
        P.accent(g, [(s * HX, 0.5, 0.0), (s * HX, 3.12, 0.0)], (s, 0, 0), 0.04, glow=0.04)
    return g


def sq_ring(h, y, c):
    """Square of half size h in (x, z) at height y with its corners chamfered by c."""
    return [(h, y, -h + c), (h, y, h - c), (h - c, y, h), (-h + c, y, h), (-h, y, h - c),
            (-h, y, -h + c), (-h + c, y, -h), (h - c, y, -h)]


def turn(p, k):
    """p turned by k quarter turns about Y (k = 0: unchanged, front -Z)."""
    x, y, z = p
    for _ in range(k % 4):
        x, z = -z, x
    return (x, y, z)


BASTION_TOP = 2.9  # the turntable top; the head's pivot is 0.4 m above it


def bastion_base():
    """Squat armoured bunker: exactly (-1.6, 0, -1.6)..(1.6, 2.9, 1.6). Chamfered square walls
    with a recessed band, a belt, slanted upper walls to a deck at 2.55 m and a turntable."""
    g = L.Geo()
    c = 0.5
    st = [(1.6, 0.0), (1.6, 0.18), (1.54, 0.24), (1.54, 1.62), (1.6, 1.68), (1.6, 1.82),
          (1.2, 2.55)]
    L.loft(g, [sq_ring(h, y, c * h / 1.6) for h, y in st], Y, cap1="body")
    P.lathe(g, (0, 2.55, 0), Y, [(0, 1.04), (0.08, 1.04), (0.12, 0.96), (BASTION_TOP - 2.55, 0.96)],
            n=12)
    P.lathe(g, (0, 2.55, 0), Y, [(0.2, 0.968), (0.25, 0.968)], n=12, mats="accent", cap0=None,
            cap1=None)
    P.accent(g, P.circle((0, 2.55, 0), Y, 1.1, 12), Y, 0.04, closed=True, glow=0.04)
    sl = L.norm((0, 0.4, -0.73))  # the front slanted face's normal
    hs = lambda y: 1.6 - (y - 1.82) * 0.4 / 0.73  # its half size at height y
    for k in range(4):
        T = lambda p: turn(p, k)
        n = T((0, 0, -1))
        e = 1.6 - c - 0.1  # straight part of a face, inside the corner chamfers
        # Firing slit (white) under a team brow; flow line along the skirt.
        P.white(g, [T((-0.7, 1.25, -1.54)), T((0.7, 1.25, -1.54))], n, 0.07, glow=0.04)
        P.accent(g, [T((-0.8, 1.38, -1.54)), T((0.8, 1.38, -1.54))], n, 0.03, glow=0.03)
        P.flow(g, [T((-0.9, 0.09, -1.6)), T((0.9, 0.09, -1.6))], n, 0.025)
        # The belt's team line.
        P.accent(g, [T((-e, 1.75, -1.6)), T((e, 1.75, -1.6))], n, 0.04, glow=0.03)
        # Slanted face: two flow lines climbing and a short white sensor strip.
        for x in (-0.55, 0.55):
            P.flow(g, [T((x, 1.92, -hs(1.92))), T((x * 0.8, 2.45, -hs(2.45)))], T(sl), 0.025)
        P.white(g, [T((-0.25, 2.1, -hs(2.1))), T((0.25, 2.1, -hs(2.1)))], T(sl), 0.04,
                glow=0.03)
        # Corner chamfer: a vertical team line.
        a, b = T((1.54 - c * 1.54 / 1.6, 0, -1.54)), T((1.54, 0, -1.54 + c * 1.54 / 1.6))
        m = ((a[0] + b[0]) / 2, (a[2] + b[2]) / 2)
        cn = L.norm((m[0], 0, m[1]))
        P.accent(g, [(m[0], 0.4, m[1]), (m[0], 1.45, m[1])], cn, 0.04, glow=0.04)
        # Slanted corner chamfer above it: a team line up its middle.
        c0, c1 = c, c * 1.2 / 1.6
        lo = ((1.6 - c0 / 2), 1.82, -(1.6 - c0 / 2))
        hi = ((1.2 - c1 / 2), 2.55, -(1.2 - c1 / 2))
        lo, hi = T(P.lerp(lo, hi, 0.12)), T(P.lerp(lo, hi, 0.88))
        q = [T((1.6 - c0, 1.82, -1.6)), T((1.6, 1.82, -1.6 + c0)), T((1.2, 2.55, -1.2 + c1)),
             T((1.2 - c1, 2.55, -1.2))]
        P.accent(g, [lo, hi], P.quad_normal(q, L.add(cn, (0, 1, 0))), 0.04, glow=0.04)
    # Front: a hatch outline under the slit.
    f = (0, 0, -1)
    P.accent(g, [(-0.42, 0.3, -1.54), (0.42, 0.3, -1.54), (0.42, 0.95, -1.54),
                 (0.3, 1.07, -1.54), (-0.3, 1.07, -1.54), (-0.42, 0.95, -1.54)], f, 0.035,
             closed=True, glow=0.03)
    return g


BASTION_MUZZLE = (0.0, 0.0, -1.7)


def bastion_head():
    """The bastion's gun head, ORIGIN AT ITS PIVOT (3.3 m above the ground): a turret ring down
    to local y = -0.4 (on the base's 2.9 m turntable), a wide faceted housing, a sensor eye on
    top and twin heavy barrels at x = +-0.3 along -Z, tips at z = -1.7."""
    g = L.Geo()
    P.lathe(g, (0, -0.4, 0), Y, [(0, 0.8), (0.1, 0.8), (0.16, 0.7), (0.22, 0.7)], n=12)
    P.lathe(g, (0, -0.4, 0), Y, [(0.03, 0.808), (0.07, 0.808)], n=12, mats="accent", cap0=None,
            cap1=None)

    def hs(z, p):
        w, yb, yt = p
        return [(-w * 0.7, yb, z), (w * 0.7, yb, z), (w, (yb + yt) / 2, z), (w * 0.75, yt, z),
                (-w * 0.75, yt, z), (-w, (yb + yt) / 2, z)]
    rings = [hs(z, p) for z, p in ((0.9, (0.62, -0.24, 0.36)), (0.3, (0.9, -0.28, 0.5)),
                                   (-0.35, (0.8, -0.26, 0.46)), (-0.75, (0.56, -0.2, 0.3)))]
    L.loft(g, rings, NZ, cap0="body", cap1="body")
    # Side faces: team lines on the upper flanks, flow on the lower.
    for k in range(3):
        for i, j, ii, jj in ((2, 3, 5, 4),):
            for a, b, mat in ((i, j, "accent"), (ii, jj, "accent")):
                q = [rings[k][a], rings[k][b], rings[k + 1][b], rings[k + 1][a]]
                side = 1 if q[0][0] > 0 else -1
                n = P.quad_normal(q, (side, 0.3, 0))
                p0, p1 = P.on_quad(*q, 0.45, 0.08), P.on_quad(*q, 0.45, 0.92)
                P.accent(g, [p0, p1], n, 0.035, glow=0.03)
            for a, b in ((1, 2), (0, 5)):
                q = [rings[k][a], rings[k][b], rings[k + 1][b], rings[k + 1][a]]
                side = 1 if q[0][0] > 0 else -1
                n = P.quad_normal(q, (side, -0.3, 0))
                P.flow(g, [P.on_quad(*q, 0.5, 0.1), P.on_quad(*q, 0.5, 0.9)], n, 0.02)
    # Top behind the eye: a team chevron pointing forward.
    q = [rings[0][3], rings[0][4], rings[1][4], rings[1][3]]
    n = P.quad_normal(q, Y)
    P.accent(g, [P.on_quad(*q, 0.25, 0.3), P.on_quad(*q, 0.5, 0.8), P.on_quad(*q, 0.75, 0.3)],
             n, 0.045, glow=0.04)
    # Rear hatch light.
    P.white(g, [(-0.25, 0.1, 0.9), (0.25, 0.1, 0.9)], Z, 0.05, glow=0.04)
    # Sensor eye: a wedge block on top, team lens facing -Z with a white glint above it.
    L.prism_x(g, [(-0.32, 0.4), (0.05, 0.4), (0.05, 0.7), (-0.2, 0.7), (-0.32, 0.6)], -0.14, 0.14)
    P.disc(g, [(0.055 * math.cos(a), 0.525 + 0.055 * math.sin(a), -0.32)
               for a in [math.radians(60 * k) for k in range(6)]], NZ, glow=0.025)
    P.white(g, [(-0.08, 0.7, -0.1), (0.08, 0.7, -0.1)], Y, 0.025, glow=0.0, h=0.003)
    P.accent(g, [(-0.1, 0.7, -0.03), (0.1, 0.7, -0.03)], Y, 0.02, glow=0.0, h=0.003)
    # Twin heavy barrels: shroud, barrel with a team ring, muzzle brake with a white bore.
    for s in (-1, 1):
        o = (s * 0.3, BASTION_MUZZLE[1], -0.2)
        P.lathe(g, o, NZ, [(0, 0.19), (0.72, 0.19), (0.8, 0.14), (1.18, 0.12), (1.18, 0.165),
                           (1.5, 0.165), (1.5, 0.09), (1.47, 0.09)], n=8, cap1=LIGHT)
        for t, r in ((0.95, 0.128), (1.3, 0.173)):
            P.lathe(g, o, NZ, [(t, r), (t + 0.05, r)], n=8, mats="accent", cap0=None,
                    cap1=None)
        P.lathe(g, o, NZ, [(0.6, 0.198), (0.66, 0.198)], n=8, mats="accent", cap0=None,
                cap1=None)
    return g


def forward_tube():
    """Field spawn beacon: exactly (-1.2, 0, -1.2)..(1.2, 3.2, 1.2). An octagonal pod (flats
    face the axes) on a wide foot with four diagonal stabiliser fins, an emitter collar near
    the top, a white-lit doorway facing -Z in a team frame, a dome with a mast to 3.2 m."""
    g = L.Geo()
    k8 = 1.0 / math.cos(math.pi / 8)  # apothem -> vertex radius
    A = 1.05
    prof = [(0, 1.2), (0.28, 1.2), (0.36, 1.12), (0.42, A), (2.2, A), (2.3, 1.19), (2.56, 1.19),
            (2.64, A), (2.8, 0.7), (2.95, 0.36)]
    P.lathe(g, (0, 0, 0), Y, [(y, a * k8) for y, a in prof], n=8, cap0=None)
    # Emitter collar: team band round it, white ring on the dome above.
    P.lathe(g, (0, 0, 0), Y, [(2.4, 1.198 * k8), (2.46, 1.198 * k8)], n=8, mats="accent",
            cap0=None, cap1=None)
    for k in range(8):
        a0, a1 = math.pi / 8 + k * math.pi / 4, math.pi / 8 + (k + 1) * math.pi / 4
        # (circle() vertex angle = phase + 2 pi i / n about Y, measured from +X towards -Z)
        def on(ap, y, t):
            u = (math.cos(a0), 0, -math.sin(a0))
            v = (math.cos(a1), 0, -math.sin(a1))
            p = P.lerp(u, v, t)
            return (p[0] * ap * k8, y, p[2] * ap * k8)
        mid = on(1.0, 0, 0.5)
        n = L.norm((mid[0], 0, mid[2]))
        if n[2] < -0.9:
            continue  # the door flat
        dn = L.norm(L.add(n, (0, (1.05 - 0.7) / 0.16, 0)))
        P.white(g, [on(0.875, 2.72, 0.25), on(0.875, 2.72, 0.75)], dn, 0.04, glow=0.03)
        if abs(n[0]) > 0.9 or n[2] > 0.9:
            P.accent(g, [on(A, 0.6, 0.5), on(A, 2.05, 0.5)], n, 0.05, glow=0.05)
        else:
            P.flow(g, [on(A, 0.6, 0.5), on(A, 2.05, 0.5)], n, 0.025)
        P.flow(g, [on(1.2, 0.14, 0.15), on(1.2, 0.14, 0.85)], n, 0.025)
    # Doorway on the -Z flat: white opening in jambs and a lintel, team lines on their fronts.
    zf = -A
    P.disc(g, [(-0.3, 0.45, zf), (0.3, 0.45, zf), (0.3, 1.8, zf), (0.18, 1.92, zf),
               (-0.18, 1.92, zf), (-0.3, 1.8, zf)], NZ, mat=LIGHT, glow=0.0)
    for s in (-1, 1):
        x0, x1 = sorted((s * 0.3, s * 0.42))
        L.box(g, (x0, 0.36, -1.15), (x1, 2.06, -0.95))
        P.accent(g, [(s * 0.36, 0.5, -1.15), (s * 0.36, 1.98, -1.15)], NZ, 0.04, glow=0.0)
    L.box(g, (-0.3, 1.94, -1.15), (0.3, 2.06, -0.95))
    P.accent(g, [(-0.26, 2.0, -1.15), (0.26, 2.0, -1.15)], NZ, 0.035, glow=0.0)
    # Stabiliser fins on the diagonals (tips stay inside the box corners).
    for dx in (-1, 1):
        for dz in (-1, 1):
            d = L.norm((dx, 0, dz))
            pp = L.norm((-dz, 0, dx))
            p = lambda r, y, o=0.0: L.add((d[0] * r, y, d[2] * r), L.mul(pp, o))
            poly = [p(0.9, 0.0, -0.08), p(1.6, 0.0, -0.08), p(1.6, 0.1, -0.08),
                    p(0.95, 0.85, -0.08), p(0.9, 0.85, -0.08)]
            P.extrude(g, poly, L.mul(pp, 0.16))
            q = [p(1.6, 0.1, -0.08), p(1.6, 0.1, 0.08), p(0.95, 0.85, 0.08), p(0.95, 0.85, -0.08)]
            tn = P.quad_normal(q, (d[0], 1, d[2]))
            P.accent(g, [p(1.45, 0.27), p(1.05, 0.73)], tn, 0.04, glow=0.0)
    # Mast with a white beacon tip and two short whips.
    P.disc(g, P.circle((0, 2.95, 0), Y, 0.25, 8), Y, glow=0.04)
    P.tube(g, (0, 2.9, 0), (0, 3.2, 0), 0.035, n=6)
    P.lathe(g, (0, 3.08, 0), Y, [(0, 0.06), (0.07, 0.06)], n=6, mats=LIGHT, cap0=LIGHT,
            cap1=LIGHT)
    for s in (-1, 1):
        P.tube(g, (s * 0.28, 2.85, 0.12), (s * 0.28, 3.1, 0.12), 0.015, n=4)
        P.tube(g, (s * 0.28, 3.1, 0.12), (s * 0.28, 3.16, 0.12), 0.02, n=4, mat="accent")
    return g


# Mobile silo (deployable field resource silo). Octagon with its flats on the axes (and
# diagonals): flat i faces (cos a, 0, -sin a), a = (i + 1) pi / 4; flat 1 faces -Z.
MSILO_A = 1.5        # tank apothem
MSILO_AI = 1.38      # window posts' inner apothem
MSILO_WIN = (1.4, 3.3)  # window band (the fill column's bottom and top)
MSILO_FILL_A = 1.36  # fill column apothem
K8 = 1.0 / math.cos(math.pi / 8)  # apothem -> vertex radius


def oct_v(a, i, y):
    """Vertex i (between flats i - 1 and i) of the octagon of apothem a at height y."""
    t = math.pi / 8 + i * math.pi / 4
    return (a * K8 * math.cos(t), y, -a * K8 * math.sin(t))


def oct_flat(i):
    """(normal, tangent) of flat i; the tangent points from vertex i towards vertex i + 1."""
    t = (i + 1) * math.pi / 4
    return (math.cos(t), 0.0, -math.sin(t)), (-math.sin(t), 0.0, -math.cos(t))


def oct_on(a, i, u, y):
    """Point on flat i of apothem a, u metres along its tangent from the flat's middle."""
    n, t = oct_flat(i)
    return (n[0] * a + t[0] * u, y, n[2] * a + t[2] * u)


def mobile_silo():
    """Deployable field resource silo: exactly (-1.8, 0, -1.8)..(1.8, 4.5, 1.8). An octagonal
    tank (flats on the axes) on a wide plinth braced by four diagonal fins; a window band all
    round (y 1.4-3.3, eight openings between corner posts, team framed) between eight
    armour ribs shows
    `mobile_silo_fill`; an unloading hopper with a recessed mouth faces -Z; a team belt, a
    low roof cone and a mast with a white beacon to 4.5 m."""
    g = L.Geo()
    A, AI = MSILO_A, MSILO_AI
    W0, W1 = MSILO_WIN
    k = K8
    # Plinth, lower tank (its top is the window floor), upper tank and roof (window ceiling).
    P.lathe(g, (0, 0, 0), Y, [(0, 1.8 * k), (0.22, 1.8 * k), (0.3, 1.68 * k)], n=8, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(0.3, A * k), (W0, A * k)], n=8, cap0=None)
    P.lathe(g, (0, 0, 0), Y, [(W1, A * k), (3.9, A * k), (3.98, 1.42 * k), (4.2, 0.6 * k)],
            n=8)
    # Armour ribs standing 0.1 m out of the tank's vertical edges, plinth to roof.
    Rv = A * k
    for i in range(8):
        v = oct_v(1.0, i, 0)
        d = L.norm((v[0], 0, v[2]))
        tg = (-d[2], 0.0, d[0])
        q = lambda r, y, o=0.0: (d[0] * r + tg[0] * o, y, d[2] * r + tg[2] * o)
        poly = [q(Rv - 0.1, 0.3, -0.07), q(Rv + 0.1, 0.3, -0.07), q(Rv + 0.1, 3.78, -0.07),
                q(Rv - 0.02, 3.96, -0.07), q(Rv - 0.1, 3.96, -0.07)]
        P.extrude(g, poly, L.mul(tg, 0.14))
        P.flow(g, [q(Rv + 0.1, 0.5, 0), q(Rv + 0.1, 3.6, 0)], d, 0.025)
    # Corner posts of the window band: each spans `p` metres along both flats from its vertex.
    p = 0.22
    flat_w = 2 * A * math.tan(math.pi / 8)
    tp = p / flat_w
    for i in range(8):
        vo, vi = oct_v(A, i, 0), oct_v(AI, i, 0)
        prev_o, prev_i = oct_v(A, i - 1, 0), oct_v(AI, i - 1, 0)
        next_o, next_i = oct_v(A, i + 1, 0), oct_v(AI, i + 1, 0)
        eo = [P.lerp(vo, prev_o, tp), P.lerp(vo, next_o, tp)]
        ei = [P.lerp(vi, prev_i, tp), P.lerp(vi, next_i, tp)]
        col = lambda q, y: (q[0], y, q[2])
        for e_o, e_i, flat in ((eo[0], ei[0], i - 1), (eo[1], ei[1], i)):
            n, t = oct_flat(flat % 8)
            g.face([col(e_o, W0), col(vo, W0), col(vo, W1), col(e_o, W1)], "body", want=n)
            g.face([col(e_i, W0), col(vi, W0), col(vi, W1), col(e_i, W1)], "body",
                   want=L.mul(n, -1))
            # Jamb: faces into the opening (away from the vertex).
            jd = L.norm(L.sub(e_o, vo))
            g.face([col(e_o, W0), col(e_i, W0), col(e_i, W1), col(e_o, W1)], "body", want=jd)
    # Team frame round each window (on the flat's plane), white tick under it.
    half = flat_w / 2 - p + 0.065
    for i in range(8):
        n, t = oct_flat(i)
        ring = [oct_on(A, i, u, y) for u, y in ((-half, W0 - 0.065), (half, W0 - 0.065),
                                                 (half, W1 + 0.065), (-half, W1 + 0.065))]
        P.accent(g, ring, n, 0.04, closed=True, glow=0.03)
    # Upper belt and roof band, team.
    P.lathe(g, (0, 0, 0), Y, [(3.66, (A + 0.008) * k), (3.74, (A + 0.008) * k)], n=8,
            mats="accent", cap0=None, cap1=None)
    cone = lambda y: 1.42 - (y - 3.98) / 0.22 * 0.82
    P.lathe(g, (0, 0, 0), Y, [(4.06, (cone(4.06) + 0.008) * k), (4.1, (cone(4.1) + 0.008) * k)],
            n=8, mats="accent", cap0=None, cap1=None)
    # Lower tank: flow lines and team ticks on the side and back flats; plinth flow all round.
    for i in range(8):
        n, t = oct_flat(i)
        if i in (3, 5, 7):
            P.flow(g, [oct_on(A, i, -0.45, 0.55), oct_on(A, i, 0.45, 0.55)], n, 0.025)
            P.accent(g, [oct_on(A, i, -0.3, 0.95), oct_on(A, i, 0.3, 0.95)], n, 0.04, glow=0.03)
            P.white(g, [oct_on(A, i, -0.12, 0.78), oct_on(A, i, 0.12, 0.78)], n, 0.035,
                    glow=0.025)
        if i != 1:
            P.flow(g, [oct_on(1.8, i, -0.55, 0.11), oct_on(1.8, i, 0.55, 0.11)], n, 0.025)
    # Bracing fins on the diagonal flats, team line along the sloped edge.
    for i in (0, 2, 4, 6):
        d, pp = oct_flat(i)
        q = lambda r, y, o=0.0: (d[0] * r + pp[0] * o, y, d[2] * r + pp[2] * o)
        poly = [q(1.45, 0.25, -0.08), q(1.75, 0.25, -0.08), q(1.75, 0.42, -0.08),
                q(1.52, 1.25, -0.08), q(1.45, 1.25, -0.08)]
        P.extrude(g, poly, L.mul(pp, 0.16))
        sq = [q(1.75, 0.42, -0.08), q(1.75, 0.42, 0.08), q(1.52, 1.25, 0.08),
              q(1.52, 1.25, -0.08)]
        tn = P.quad_normal(sq, (d[0], 0.3, d[2]))
        P.accent(g, [q(1.73, 0.52), q(1.55, 1.17)], tn, 0.04, glow=0.0)
        P.flow(g, [q(1.6, 0.4, -0.08), q(1.6, 1.0, -0.08)], L.mul(pp, -1), 0.02)
        P.flow(g, [q(1.6, 0.4, 0.08), q(1.6, 1.0, 0.08)], pp, 0.02)
    # Unloading hopper on the front flat (-Z): sloped top, recessed mouth in a frame.
    HX = 0.5
    L.prism_x(g, [(-1.72, 0.22), (-1.45, 0.22), (-1.45, 1.25), (-1.55, 1.25), (-1.72, 1.0)],
              -HX, HX)
    MX, MY0, MY1 = 0.36, 0.36, 0.86
    for s in (-1, 1):
        x0, x1 = sorted((s * MX, s * HX))
        L.box(g, (x0, 0.22, -1.8), (x1, 1.0, -1.72), skip=("+z",))
    L.box(g, (-MX, MY1, -1.8), (MX, 1.0, -1.72), skip=("+z",))
    L.box(g, (-MX, 0.22, -1.8), (MX, MY0, -1.72), skip=("+z",))
    for y in (0.48, 0.61, 0.74):
        P.flow(g, [(-0.3, y, -1.72), (0.3, y, -1.72)], NZ, 0.03)
    P.accent(g, [(-0.43, 0.3, -1.8), (-0.43, 0.93, -1.8), (0.43, 0.93, -1.8),
                 (0.43, 0.3, -1.8)], NZ, 0.04, glow=0.0)
    sn = L.norm((0, 0.17, -0.25))
    P.white(g, [(-0.3, 1.125, -1.635), (0.3, 1.125, -1.635)], sn, 0.04, glow=0.03)
    for s in (-1, 1):
        P.flow(g, [(s * HX, 0.35, -1.6), (s * HX, 0.95, -1.6)], (s, 0, 0), 0.025)
    # Roof: hatch collar, team ring, mast with a white beacon and a cap to 4.5 m.
    P.lathe(g, (0, 4.2, 0), Y, [(0, 0.32), (0.05, 0.32), (0.08, 0.24)], n=8)
    P.accent(g, P.circle((0, 4.2, 0), Y, 0.45, 8), Y, 0.035, closed=True, glow=0.03)
    P.tube(g, (0, 4.28, 0), (0, 4.33, 0), 0.06, n=8)
    P.lathe(g, (0, 4.33, 0), Y, [(0, 0.1), (0.09, 0.1)], n=8, mats=LIGHT, cap0=LIGHT,
            cap1=LIGHT)
    P.lathe(g, (0, 4.42, 0), Y, [(0, 0.12), (0.05, 0.12), (0.08, 0.05)], n=8)
    return g


def mobile_silo_fill():
    """Fill column inside the mobile silo's window band: `accent` octagonal prism of apothem
    1.36 m from y = 0 to 1 (the game puts it at y = 1.4 and scales it in Y by 1.9 x fill)."""
    g = L.Geo()
    P.lathe(g, (0, 0, 0), Y, [(0, MSILO_FILL_A * K8), (1.0, MSILO_FILL_A * K8)], n=8,
            mats="accent", cap0="accent", cap1="accent")
    return g


def main():
    L.reset_scene()
    P.materials()
    L.material("resource", base=(1.0, 0.55, 0.1), rough=0.3, emit=(1.0, 0.55, 0.1), strength=2.0)
    props = [("terminal_equipment", terminal("equipment")),
             ("terminal_medical", terminal("medical")),
             ("terminal_vehicle", terminal("vehicle")),
             ("console_core", console_core()),
             ("turret_base", turret_base()),
             ("turret_head", turret_head()),
             ("vehicle_pad", vehicle_pad()),
             ("rearm_pad", rearm_pad()),
             ("spawn_tube", spawn_tube()),
             ("wall_turret_base", wall_turret_base()),
             ("wall_turret_head", wall_turret_head()),
             ("generator", generator()),
             ("generator_core", generator_core()),
             ("resource_silo", resource_silo()),
             ("resource_silo_fill", resource_silo_fill()),
             ("resource_node", resource_node()),
             ("spitfire_base", spitfire_base()),
             ("spitfire_head", spitfire_head()),
             ("at_mine", at_mine()),
             ("ammo_pack", ammo_pack()),
             ("build_wall", build_wall()),
             ("bastion_base", bastion_base()),
             ("bastion_head", bastion_head()),
             ("forward_tube", forward_tube()),
             ("mobile_silo", mobile_silo()),
             ("mobile_silo_fill", mobile_silo_fill()),
             ("capture_pylon", capture_pylon())]
    asymmetric = ("terminal_equipment", "resource_node")  # (the gun motif; the crystals)
    smooth = {"generator_core": 50, "resource_silo_fill": 30}
    for name, g in props:
        bad = g.mirror_errors() if name not in asymmetric else []
        if bad:
            raise SystemExit("%s is not X mirror symmetric: %s" % (name, bad[:5]))
        if g.tris() > LIMIT:
            raise SystemExit("%s: %d tris (limit %d)" % (name, g.tris(), LIMIT))
        L.make_object(name, g, smooth=smooth.get(name))
        print("prop %-20s %5d tris" % (name, g.tris()))
    for s, side in ((-1, "l"), (1, "r")):
        L.make_empty("turret_head_muzzle_" + side, (s * 0.5, 0.02, -1.4))
        L.make_empty("wall_turret_muzzle_" + side, (s * 0.55, 0.0, -2.6))
    L.make_empty("spitfire_head_muzzle", (0.0, 0.0, -0.44))
    L.make_empty("bastion_head_muzzle", BASTION_MUZZLE)
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/props.blend")
    L.export_glb("assets/models/props.glb")


main()
