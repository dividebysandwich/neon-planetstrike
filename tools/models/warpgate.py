"""Builds the warpgate: assets-src/blend/warpgate.blend and assets/models/warpgate.glb.

    blender -b --factory-startup --python-exit-code 1 --python tools/models/warpgate.py

Origin at the gate's middle on the ground, Y up.

`warpgate`: eight hexagonal pillars matching the game's colliders exactly: pillar i (0..7) stands
at angle a = i/8 * 2 pi + 0.26, centre (40 cos a, 40 sin a) in (x, z), a 6-sided prism of
circumradius 2.4 m whose vertex k is at angle a + k * 60 degrees about its centre (x = cos,
z = sin), from y = -1 to y = 55. Team light bands ring each pillar; an octagon of beams
(51.5-54.5 m, lit on the inner face and the underside, `flow` on top) joins the pillar tops,
each span carried by two knee braces from y = 45 into an arch; a ground ring (radius
43.4-44.6 m) with a team line and an inner `flow` ring at 36 m.

`warpgate_core`: the swirling core volume, all `glow`: an outer 24-sided cylinder (radius 14 m,
y 0-50 m) and an inner one (radius 9 m) twisting 30 degrees per 6.25 m step. UV: U = metres
around (arc length), V = height / 50 (0 at the ground, 1 at the top: the game fades by V).

`warpgate_shell`: the translucent lens over the gate, all `glow`: an ellipsoidal dome, 32
segments round, radius 38 m at the ground rising to a point at y = 48 m (the profile
r = 38 cos t, y = 48 sin t), faces outward. UV: U = metres around at the base radius
(38 * angle), V = height / 48.

`warpgate_rings`: four thin `accent` rings (0.6 m wide, 0.35 m tall box section, each broken
into six arcs with gaps so the game's spin reads) at radius 22 / 19 / 16.5 / 20.5 m and
y = 9 / 19 / 29 / 38 m, with `glow` halos on their top faces. Spun by the game about +Y.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ltlib as L  # noqa: E402
import pslib as P  # noqa: E402
from pslib import LIGHT, X, Y, Z, NX, NY, NZ  # noqa: E402

N_PILLARS = 8
RING_R = 40.0
PILLAR_R = 2.4
Y0, Y1 = -1.0, 55.0
CORE_R, CORE_H = 14.0, 50.0
SHELL_R, SHELL_H = 38.0, 48.0
BEAM_Y0, BEAM_Y1, BEAM_HW = 51.5, 54.5, 1.2
GROUND_R0, GROUND_R1 = 43.4, 44.6
RINGS = ((22.0, 9.0), (19.0, 19.0), (16.5, 29.0), (20.5, 38.0))  # (radius, y)


def pillar_angle(i):
    return i / N_PILLARS * 2 * math.pi + 0.26


def pillar_centre(i):
    a = pillar_angle(i)
    return (RING_R * math.cos(a), RING_R * math.sin(a))


def pillar_hex(i):
    a = pillar_angle(i)
    cx, cz = pillar_centre(i)
    return [(cx + PILLAR_R * math.cos(a + k * math.pi / 3), cz + PILLAR_R * math.sin(a + k * math.pi / 3))
            for k in range(6)]


def gate():
    g = L.Geo()
    bands = (2.0, 12.0, 22.0, 32.0, 42.0)
    for i in range(N_PILLARS):
        hexa = pillar_hex(i)
        L.prism_y(g, hexa, Y0, Y1)
        cx, cz = pillar_centre(i)
        for k in range(6):
            a, b = hexa[k], hexa[(k + 1) % 6]
            nrm = ((a[0] + b[0]) / 2 - cx, 0, (a[1] + b[1]) / 2 - cz)
            pa = lambda y, t: (a[0] + (b[0] - a[0]) * t, y, a[1] + (b[1] - a[1]) * t)
            # Bands round the pillar.
            for y in bands:
                P.accent(g, [pa(y, 0.0), pa(y, 1.0)], nrm, 0.35, glow=0.5, h=0.02)
            # The two faces looking at the gate's middle carry a vertical light line between the
            # bands; the outer faces flow lines.
            if k in (2, 3):
                for y0, y1 in zip(bands, bands[1:] + (50.5,)):
                    P.accent(g, [pa(y0 + 0.8, 0.5), pa(y1 - 0.8, 0.5)], nrm, 0.2, glow=0.35,
                             h=0.02)
            if k in (0, 5):
                P.flow(g, [pa(2.8, 0.5), pa(50.0, 0.5)], nrm, 0.12, h=0.02)
        # White beacon on the top.
        P.disc(g, P.circle((cx, Y1, cz), Y, 0.9, 6), Y, mat=LIGHT, glow=0.45, h=0.02)
    # Beams joining the pillar tops, each span an arch on two knee braces.
    hw = BEAM_HW
    for i in range(N_PILLARS):
        a, b = pillar_centre(i), pillar_centre((i + 1) % N_PILLARS)
        span = math.hypot(b[0] - a[0], b[1] - a[1])
        d = L.norm((b[0] - a[0], 0, b[1] - a[1]))
        n = (-d[2], 0, d[0])  # in the plane, perpendicular
        if n[0] * (a[0] + b[0]) + n[2] * (a[1] + b[1]) > 0:
            n = L.mul(n, -1)  # n points at the gate's middle
        A, B = (a[0], BEAM_Y0, a[1]), (b[0], BEAM_Y0, b[1])
        poly = [L.add(A, L.mul(n, -hw)), L.add(B, L.mul(n, -hw)), L.add(B, L.mul(n, hw)),
                L.add(A, L.mul(n, hw))]
        P.extrude(g, poly, (0, BEAM_Y1 - BEAM_Y0, 0))
        inset = 2.6  # stay off the pillars
        A2, B2 = L.add(A, L.mul(d, inset)), L.sub(B, L.mul(d, inset))
        face = lambda p, y: L.add((p[0], y, p[2]), L.mul(n, hw))
        P.accent(g, [face(A2, 53.0), face(B2, 53.0)], n, 0.45, glow=0.6, h=0.02)
        under = lambda p: (p[0], BEAM_Y0, p[2])
        top = lambda p: (p[0], BEAM_Y1, p[2])
        P.flow(g, [top(A2), top(B2)], Y, 0.18, h=0.02)
        # Knee braces: from the pillar's side at y = 45 up into the beam's underside a fifth
        # of the span in; a white line under the middle part between them.
        bw = 0.7
        for s in (0, 1):
            P0 = (a[0], 0, a[1]) if s == 0 else (b[0], 0, b[1])
            dd = d if s == 0 else L.mul(d, -1)
            foot = L.add(P0, L.add(L.mul(dd, PILLAR_R * 0.8), (0, 45.0, 0)))
            head = L.add(P0, L.add(L.mul(dd, span * 0.2), (0, BEAM_Y0 + 0.05, 0)))
            ax = L.sub(head, foot)
            side = L.norm(L.cross(ax, n))
            quad = [L.add(foot, L.mul(n, -bw)), L.add(foot, L.mul(n, bw)),
                    L.add(L.add(foot, L.mul(side, 0.9)), L.mul(n, bw)),
                    L.add(L.add(foot, L.mul(side, 0.9)), L.mul(n, -bw))]
            P.extrude(g, quad, ax)
        C0 = L.add(A, L.mul(d, span * 0.2 + 0.6))
        C1 = L.sub(B, L.mul(d, span * 0.2 + 0.6))
        P.white(g, [under(C0), under(C1)], NY, 0.35, glow=0.5, h=0.02)
    # Ground ring.
    n = 64
    P.lathe(g, (0, 0, 0), Y, [(0, GROUND_R1), (0.06, GROUND_R1)], n=n, cap0=None, cap1=None)
    a, b = P.circle((0, 0.06, 0), Y, GROUND_R0, n), P.circle((0, 0.06, 0), Y, GROUND_R1, n)
    for i in range(n):
        j = (i + 1) % n
        g.face([a[i], a[j], b[j], b[i]], "body", want=Y)
        q = [a[i], a[j], (a[j][0], 0, a[j][2]), (a[i][0], 0, a[i][2])]
        g.face(q, "body", want=(-a[i][0], 0, -a[i][2]))
    f = P.plane((0, 0.06, 0), Y)
    L.ring(g, f, L.ngon(0, 0, (GROUND_R0 + GROUND_R1) / 2, n, 0.0), 0.4, glow_in=0.5,
           glow_out=0.5, sides=False)
    f0 = P.plane((0, 0.0, 0), Y)
    L.strip(g, f0, L.ngon(0, 0, 36.0, n, 0.0), 0.2, closed=True, glow=(0, 0), mat="flow",
            h=0.01, sides=False)
    return g


def core():
    g = L.Geo()
    n = 24

    def cyl(r, steps, twist):
        seg = 2 * math.pi * r / n
        for s in range(steps):
            y0, y1 = CORE_H * s / steps, CORE_H * (s + 1) / steps
            for i in range(n):
                a0 = 2 * math.pi * i / n
                a1 = 2 * math.pi * (i + 1) / n
                t0, t1 = twist * s, twist * (s + 1)
                q = [(r * math.cos(a0 + t0), y0, r * math.sin(a0 + t0)),
                     (r * math.cos(a1 + t0), y0, r * math.sin(a1 + t0)),
                     (r * math.cos(a1 + t1), y1, r * math.sin(a1 + t1)),
                     (r * math.cos(a0 + t1), y1, r * math.sin(a0 + t1))]
                uv = [(i * seg, y0 / CORE_H), ((i + 1) * seg, y0 / CORE_H),
                      ((i + 1) * seg, y1 / CORE_H), (i * seg, y1 / CORE_H)]
                mid = L.mul(tuple(map(sum, zip(*q))), 0.25)
                # Two triangles: twisted quads are not planar.
                g.face([q[0], q[1], q[2]], "glow", [uv[0], uv[1], uv[2]], want=(mid[0], 0, mid[2]))
                g.face([q[0], q[2], q[3]], "glow", [uv[0], uv[2], uv[3]], want=(mid[0], 0, mid[2]))

    cyl(CORE_R, 4, 0.0)
    cyl(9.0, 8, math.radians(30))
    return g


def shell():
    """Ellipsoidal glow dome: r = R cos t, y = H sin t, t = 0..90 degrees."""
    g = L.Geo()
    n, m = 32, 10
    ts = [math.pi / 2 * k / m for k in range(m + 1)]
    rows = [(SHELL_R * math.cos(t), SHELL_H * math.sin(t)) for t in ts]
    rows[-1] = (0.0, SHELL_H)
    for k in range(m):
        (r0, y0), (r1, y1) = rows[k], rows[k + 1]
        for i in range(n):
            a0, a1 = 2 * math.pi * i / n, 2 * math.pi * (i + 1) / n
            u0, u1 = SHELL_R * a0, SHELL_R * a1
            p00 = (r0 * math.cos(a0), y0, r0 * math.sin(a0))
            p01 = (r0 * math.cos(a1), y0, r0 * math.sin(a1))
            v0, v1 = y0 / SHELL_H, y1 / SHELL_H
            mid = (math.cos((a0 + a1) / 2) * r0, (y0 + y1) / 2 - 0.0, math.sin((a0 + a1) / 2) * r0)
            out = L.add(L.mul((mid[0], 0, mid[2]), 1.0 / SHELL_R ** 2),
                        (0, (y0 + y1) / 2 / SHELL_H ** 2, 0))  # ellipsoid normal
            if r1 < 1e-6:
                g.face([p00, p01, (0, y1, 0)], "glow", [(u0, v0), (u1, v0), ((u0 + u1) / 2, v1)],
                       want=out)
                continue
            p11 = (r1 * math.cos(a1), y1, r1 * math.sin(a1))
            p10 = (r1 * math.cos(a0), y1, r1 * math.sin(a0))
            g.face([p00, p01, p11, p10], "glow", [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                   want=out)
    return g


def rings():
    g = L.Geo()
    n_arcs, per_arc, gap = 6, 8, math.radians(9)
    w, h = 0.3, 0.175  # half width, half height of the box section
    for r, y in RINGS:
        span = 2 * math.pi / n_arcs - gap
        for k in range(n_arcs):
            a0 = 2 * math.pi * k / n_arcs + gap / 2
            angs = [a0 + span * t / per_arc for t in range(per_arc + 1)]
            sec = lambda a: [((r - w) * math.cos(a), y - h, (r - w) * math.sin(a)),
                             ((r + w) * math.cos(a), y - h, (r + w) * math.sin(a)),
                             ((r + w) * math.cos(a), y + h, (r + w) * math.sin(a)),
                             ((r - w) * math.cos(a), y + h, (r - w) * math.sin(a))]
            secs = [sec(a) for a in angs]
            for t in range(per_arc):
                s0, s1 = secs[t], secs[t + 1]
                am = (angs[t] + angs[t + 1]) / 2
                rad = (math.cos(am), 0, math.sin(am))
                for e, want in ((0, NY), (1, rad), (2, Y), (3, L.mul(rad, -1))):
                    e1 = (e + 1) % 4
                    g.face([s0[e], s0[e1], s1[e1], s1[e]], "accent", want=want)
                # Halos on the top face, inside and outside the ring.
                seg = L.length(L.sub(s1[2], s0[2]))
                for rr0, rr1 in ((r + w, r + w + 0.9), (r - w, r - w - 0.9)):
                    q = [(rr0 * math.cos(angs[t]), y + h + 0.002, rr0 * math.sin(angs[t])),
                         (rr0 * math.cos(angs[t + 1]), y + h + 0.002, rr0 * math.sin(angs[t + 1])),
                         (rr1 * math.cos(angs[t + 1]), y + h + 0.002, rr1 * math.sin(angs[t + 1])),
                         (rr1 * math.cos(angs[t]), y + h + 0.002, rr1 * math.sin(angs[t]))]
                    g.face(q, "glow", [(t * seg, 0), ((t + 1) * seg, 0), ((t + 1) * seg, 1),
                                       (t * seg, 1)], want=Y)
            for s, a in ((secs[0], angs[0]), (secs[-1], angs[-1])):
                tang = (-math.sin(a), 0, math.cos(a))
                g.face(s, "accent", want=tang if a == angs[-1] else L.mul(tang, -1))
    return g


def main():
    L.reset_scene()
    P.materials()
    for name, g in (("warpgate", gate()), ("warpgate_core", core()), ("warpgate_shell", shell()),
                    ("warpgate_rings", rings())):
        L.make_object(name, g)
        print("warpgate %-14s %5d tris" % (name, g.tris()))
    P.drop_unused_materials()
    L.save_blend("assets-src/blend/warpgate.blend")
    L.export_glb("assets/models/warpgate.glb")


main()
