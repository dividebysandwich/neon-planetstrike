//! The architecture's drawn-only dress: the buildings' facades and the compound walls'
//! ramparts.

use glam::{Vec2, Vec3};

use super::blocks::{ST_HALF, ST_LANE, ST_OUT, ST_WALL, Stairs};
use super::builder::{Builder, Gap};
use super::*;

/// A section swept along a face: four corners (out from the face, height), in order round it.
pub(super) type Sec = [(f32, f32); 4];

/// A building's outer face, for its details: from its corner `a` (base frame XZ) along `u`,
/// facing `out`, `len` to the next corner; its doorways and windows (along it from `a`), and
/// stretches kept clear above the roof line (where a ramp comes up).
pub(super) struct Face {
    pub(super) a: Vec2,
    pub(super) u: Vec2,
    pub(super) out: Vec2,
    pub(super) len: f32,
    pub(super) doors: Vec<(f32, f32, f32)>,
    pub(super) windows: Vec<(f32, f32, f32, f32)>,
    pub(super) clear: Vec<(f32, f32)>,
}

impl Face {
    pub(super) fn at(&self, along: f32, out: f32) -> Vec2 {
        self.a + self.u * along + self.out * out
    }

    /// The face of a wall from `a` to `b` (its middle line, `t` thick) facing `out`, with the
    /// wall's gaps (doors from the foot, the rest windows).
    pub(super) fn of(a: Vec2, b: Vec2, t: f32, out: Vec2, gaps: &[Gap]) -> Face {
        let u = (b - a).normalize();
        let a = a + out * (t / 2.0) - u * (t / 2.0);
        let len = a.distance(b + out * (t / 2.0) + u * (t / 2.0));
        let sh = t / 2.0;
        Face {
            a,
            u,
            out,
            len,
            doors: gaps
                .iter()
                .filter(|g| g.lo <= 0.0)
                .map(|g| (g.off + sh, g.off + sh + g.w, g.hi))
                .collect(),
            windows: gaps
                .iter()
                .filter(|g| g.lo > 0.0)
                .map(|g| (g.off + sh, g.off + sh + g.w, g.lo, g.hi))
                .collect(),
            clear: Vec::new(),
        }
    }

    /// Whether `s`..`e` along it keeps `m` away from its doorways.
    pub(super) fn free(&self, s: f32, e: f32, m: f32) -> bool {
        s > 0.8
            && e < self.len - 0.8
            && self
                .doors
                .iter()
                .map(|d| (d.0, d.1))
                .chain(self.clear.iter().copied())
                .all(|(a, z)| e + m < a || s - m > z)
    }
}

/// How a facade is dressed.
pub(super) struct Dress {
    /// Its roof line (the walls' top) and the storeys' floors above the ground.
    pub(super) top: f32,
    pub(super) storeys: u8,
    /// Glazed windows along the ground floor, and pilasters between the bays.
    pub(super) glazed: bool,
    pub(super) bays: bool,
    /// Its look: 0 pilasters between the bays; 1 plain, a neon band over the ground floor
    /// and a taller cornice; 2 heavy piers and tall glazing.
    pub(super) style: u8,
}

/// The architecture over a building's (or tower's) outer faces: an angled plinth at the foot,
/// corner piers, pilasters between the bays, framed windows (glazed on the ground floor),
/// framed doorways, a string course at every floor, vents, and an angled cornice leaning in
/// to the roof's parapet with a neon band below it.
pub(super) fn facade(b: &mut Builder, faces: &[Face], d: &Dress) {
    use Finish::{Body, Glass, Trim};
    let yb = d.top - 1.2;
    let ct = d.top + if d.style == 1 { 2.1 } else { 1.7 };
    let pier = if d.style == 2 { 1.7 } else { 1.1 };
    for f in faces {
        let doors: Vec<(f32, f32)> = f.doors.iter().map(|d| (d.0 - 0.5, d.1 + 0.5)).collect();
        // The plinth: a sloped course at the foot, broken by the doorways.
        b.runs(
            f,
            &doors,
            [(-0.05, 0.0), (0.55, 0.0), (0.2, 0.75), (-0.05, 0.75)],
            Body,
        );
        // Corner piers.
        for (s, e) in [(0.0, pier), (f.len - pier, f.len)] {
            b.run(
                f,
                s,
                e,
                [(-0.05, 0.0), (0.4, 0.0), (0.4, yb), (-0.05, yb)],
                Body,
            );
        }
        // Doorways: jambs, a sloped lintel with a lit strip under its front.
        for &(s, e, h) in &f.doors {
            for (js, je) in [(s - 0.45, s), (e, e + 0.45)] {
                b.run(
                    f,
                    js,
                    je,
                    [(-0.05, 0.0), (0.35, 0.0), (0.35, h + 0.2), (-0.05, h + 0.2)],
                    Body,
                );
            }
            b.run(
                f,
                s - 0.6,
                e + 0.6,
                [(-0.05, h), (0.65, h), (0.65, h + 0.25), (-0.05, h + 0.75)],
                Body,
            );
            b.run(
                f,
                s,
                e,
                [(0.3, h - 0.07), (0.5, h - 0.07), (0.5, h), (0.3, h)],
                Trim,
            );
        }
        // Windows: the wall's openings, and glazed ones along the ground floor.
        let mut wins: Vec<(f32, f32, f32, f32, bool)> = f
            .windows
            .iter()
            .map(|w| (w.0, w.1, w.2, w.3, false))
            .collect();
        let mut bays = Vec::new();
        let n = ((f.len - 2.0) / 7.0) as usize;
        for i in 0..n {
            let s = 2.3 + i as f32 * 7.0;
            bays.push(s);
            if d.glazed && f.free(s - 0.4, s + 3.9, 0.4) {
                let (lo, hi) = if d.style == 2 { (0.9, 3.7) } else { (1.5, 3.2) };
                wins.push((s, s + 3.5, lo, hi, true));
            } else if d.glazed && f.free(s + 0.6, s + 2.9, 0.3) {
                // No room for a window: a vent.
                vent(b, f, s + 0.6, s + 2.9, 0.9);
            }
        }
        for &(s, e, lo, hi, glass) in &wins {
            if glass {
                b.run(
                    f,
                    s,
                    e,
                    [(-0.05, lo), (0.06, lo), (0.06, hi), (-0.05, hi)],
                    Glass,
                );
                let m = (s + e) / 2.0;
                b.run(
                    f,
                    m - 0.07,
                    m + 0.07,
                    [(-0.05, lo), (0.14, lo), (0.14, hi), (-0.05, hi)],
                    Body,
                );
            }
            for (js, je) in [(s - 0.22, s), (e, e + 0.22)] {
                b.run(
                    f,
                    js,
                    je,
                    [(-0.05, lo), (0.25, lo), (0.25, hi), (-0.05, hi)],
                    Body,
                );
            }
            // A sloped sill, and a hood over it with a lit strip underneath.
            b.run(
                f,
                s - 0.4,
                e + 0.4,
                [
                    (-0.05, lo - 0.3),
                    (0.42, lo - 0.3),
                    (0.42, lo - 0.14),
                    (-0.05, lo),
                ],
                Body,
            );
            b.run(
                f,
                s - 0.4,
                e + 0.4,
                [(-0.05, hi), (0.5, hi), (0.5, hi + 0.12), (-0.05, hi + 0.5)],
                Body,
            );
            b.run(
                f,
                s,
                e,
                [(0.28, hi - 0.06), (0.44, hi - 0.06), (0.44, hi), (0.28, hi)],
                Trim,
            );
        }
        // Pilasters between the bays, their tops sloping out under the cornice.
        if d.bays && d.style != 1 {
            for &s in &bays {
                let m = s - 1.75;
                if m > 2.0 && f.free(m - 0.45, m + 0.45, 0.3) {
                    b.run(
                        f,
                        m - 0.45,
                        m + 0.45,
                        [
                            (-0.05, 0.5),
                            (0.45, 0.5),
                            (0.45, yb - 0.75),
                            (-0.05, yb - 0.1),
                        ],
                        Body,
                    );
                }
            }
        }
        if d.style == 1 {
            // A neon band over the ground floor's windows, broken by the doorways.
            b.runs(
                f,
                &doors,
                [(-0.02, 4.25), (0.05, 4.25), (0.05, 4.45), (-0.02, 4.45)],
                Trim,
            );
        }
        // A string course at every floor above the ground, a neon line under its lip.
        for k in 1..d.storeys {
            let y = k as f32 * STOREY;
            b.runs(
                f,
                &[],
                [
                    (-0.05, y - 0.3),
                    (0.3, y - 0.15),
                    (0.3, y + 0.08),
                    (-0.05, y + 0.08),
                ],
                Body,
            );
            b.runs(
                f,
                &[],
                [(0.3, y - 0.12), (0.34, y - 0.12), (0.34, y), (0.3, y)],
                Trim,
            );
        }
        // The cornice: leaning in from its lip to the parapet's top, a neon band across its
        // slope and another under it.
        let slope = |y: f32| 1.35 - 1.1 * (y - yb) / (ct - yb);
        b.runs(
            f,
            &f.clear,
            [(-0.05, yb), (1.35, yb), (0.25, ct), (-0.05, ct)],
            Body,
        );
        let (y0, y1) = (yb + 0.45, yb + 0.7);
        b.runs(
            f,
            &f.clear,
            [
                (slope(y0) - 0.02, y0),
                (slope(y0) + 0.04, y0),
                (slope(y1) + 0.04, y1),
                (slope(y1) - 0.02, y1),
            ],
            Trim,
        );
        b.runs(
            f,
            &f.clear,
            [
                (-0.05, yb - 0.3),
                (0.32, yb - 0.3),
                (0.32, yb - 0.16),
                (-0.05, yb - 0.16),
            ],
            Trim,
        );
    }
}

/// A vent grille on face `f` from `s` to `e`, its foot at `y`: a frame and slats.
pub(super) fn vent(b: &mut Builder, f: &Face, s: f32, e: f32, y: f32) {
    use Finish::Body;
    b.run(
        f,
        s,
        e,
        [(-0.05, y), (0.12, y), (0.12, y + 1.1), (-0.05, y + 1.1)],
        Body,
    );
    for k in 0..4 {
        let h = y + 0.18 + k as f32 * 0.24;
        b.run(
            f,
            s + 0.12,
            e - 0.12,
            [
                (0.1, h),
                (0.26, h - 0.05),
                (0.26, h + 0.06),
                (0.1, h + 0.12),
            ],
            Body,
        );
    }
}

/// The compound walls' dress: sloped buttresses along the outside, a corbelled fascia under
/// the walkway's edge with a neon line along it, and tall pylons either side of the gateways,
/// inside and out.
pub(super) fn ramparts(b: &mut Builder, half: f32) {
    use Finish::{Body, Trim};
    let inner = half - WALL_T;
    let g = GATEWAY / 2.0;
    let h = WALL_H;
    // Outer faces, then inner ones (front, back, left, right), the gateways midway on the
    // front and back.
    let mut faces = Vec::new();
    // (The inner faces' "out" points into the yard; no piece on them reaches a corner, where
    // the mitres would be the wrong way round.)
    for (outside, r) in [(true, half), (false, inner)] {
        let sgn = if outside { 1.0 } else { -1.0 };
        for (a, u, out, gate) in [
            (Vec2::new(-r, -r), Vec2::X, Vec2::NEG_Y, true),
            (Vec2::new(-r, r), Vec2::X, Vec2::Y, true),
            (Vec2::new(-r, -r), Vec2::Y, Vec2::NEG_X, false),
            (Vec2::new(r, -r), Vec2::Y, Vec2::X, false),
        ] {
            let len = 2.0 * r;
            let doors = if gate {
                vec![(r - g, r + g, h)]
            } else {
                Vec::new()
            };
            faces.push((
                outside,
                Face {
                    a,
                    u,
                    out: out * sgn,
                    len,
                    doors,
                    windows: Vec::new(),
                    clear: Vec::new(),
                },
            ));
        }
    }
    for (outside, f) in &faces {
        let gate: Vec<(f32, f32)> = f.doors.iter().map(|d| (d.0, d.1)).collect();
        // Pylons either side of the gateway, leaning in, banded.
        let foot = if *outside { -1.0 } else { 0.0 };
        let o = |y: f32| 1.6 - 0.5 * (y - foot) / (h + 3.0 - foot);
        for &(s, e) in &gate {
            for (ps, pe) in [(s - 2.6, s), (e, e + 2.6)] {
                b.run(
                    f,
                    ps,
                    pe,
                    [
                        (-0.05, foot),
                        (1.6, foot),
                        (o(h + 3.0), h + 3.0),
                        (-0.05, h + 3.0),
                    ],
                    Body,
                );
                b.run(
                    f,
                    ps - 0.1,
                    pe + 0.1,
                    [
                        (-0.05, h + 3.0),
                        (1.25, h + 3.0),
                        (1.0, h + 3.4),
                        (-0.05, h + 3.4),
                    ],
                    Body,
                );
                for y in [3.0, 7.0, h + 2.2] {
                    b.run(
                        f,
                        ps + 0.3,
                        pe - 0.3,
                        [
                            (o(y) - 0.02, y),
                            (o(y) + 0.04, y),
                            (o(y + 0.2) + 0.04, y + 0.2),
                            (o(y + 0.2) - 0.02, y + 0.2),
                        ],
                        Trim,
                    );
                }
            }
        }
        if !*outside {
            continue;
        }
        // Buttresses, sloping up into the corbels.
        let mut s = 10.0;
        while s < f.len - 10.0 {
            if gate
                .iter()
                .all(|&(a, e)| s + 1.0 < a - 4.0 || s - 1.0 > e + 4.0)
            {
                b.run(
                    f,
                    s - 1.0,
                    s + 1.0,
                    [
                        (-0.05, -1.0),
                        (1.8, -1.0),
                        (0.35, h - 1.6),
                        (-0.05, h - 1.0),
                    ],
                    Body,
                );
            }
            s += 16.0;
        }
        // The corbel and the fascia over it, up to the parapet's top, a neon line along it.
        b.runs(
            f,
            &[],
            [
                (-0.05, h - 1.6),
                (0.1, h - 1.6),
                (0.7, h - 0.4),
                (-0.05, h - 0.4),
            ],
            Body,
        );
        b.runs(
            f,
            &[],
            [
                (-0.05, h - 0.4),
                (0.7, h - 0.4),
                (0.7, h + 1.35),
                (-0.05, h + 1.35),
            ],
            Body,
        );
        b.runs(
            f,
            &[],
            [
                (0.7, h + 0.2),
                (0.74, h + 0.2),
                (0.74, h + 0.34),
                (0.7, h + 0.34),
            ],
            Trim,
        );
    }
}

/// A stair tower's dress (in the stairs' frame): piers on its yard-side corners with a neon
/// riser up each, a lit frame round its door, a band under its roof, glass in its slits.
pub(super) fn stair_dress(b: &mut Builder, s: &Stairs) {
    use Finish::{Body, Trim};
    let len = s.len();
    let (o, z1) = (ST_OUT, len + ST_WALL);
    for sx in [-1.0f32, 1.0] {
        // The corner's pier, and the riser on its yard face.
        let (x0, x1) = (sx * (o - 0.5), sx * (o + 0.12));
        b.block(
            Vec3::new(x0.min(x1), 0.0, z1 - 0.5),
            Vec3::new(x0.max(x1), s.top - 0.2, z1 + 0.12),
            Body,
        );
        let (x0, x1) = (sx * (o - 0.3), sx * (o - 0.16));
        b.block(
            Vec3::new(x0.min(x1), 0.5, z1 + 0.12),
            Vec3::new(x0.max(x1), s.top - 0.8, z1 + 0.16),
            Trim,
        );
        // A plinth along each side.
        let (x0, x1) = (sx * o, sx * (o + 0.18));
        b.block(
            Vec3::new(x0.min(x1), 0.0, 0.0),
            Vec3::new(x0.max(x1), 0.6, z1 - 0.5),
            Body,
        );
        // A band round under the roof.
        let (x0, x1) = (sx * o, sx * (o + 0.05));
        b.block(
            Vec3::new(x0.min(x1), s.top - 0.5, 0.0),
            Vec3::new(x0.max(x1), s.top - 0.36, z1),
            Trim,
        );
    }
    b.block(
        Vec3::new(-o, s.top - 0.5, z1),
        Vec3::new(o, s.top - 0.36, z1 + 0.05),
        Trim,
    );
    // The door (in line with the left lane): jambs, a hood, a lit strip under it.
    let (d0, d1, dh) = (-ST_HALF, -ST_HALF + ST_LANE, 2.6);
    for (x0, x1) in [(d0 - 0.3, d0), (d1, d1 + 0.3)] {
        b.block(
            Vec3::new(x0, 0.0, z1),
            Vec3::new(x1, dh + 0.25, z1 + 0.2),
            Body,
        );
    }
    b.block(
        Vec3::new(d0 - 0.45, dh, z1),
        Vec3::new(d1 + 0.45, dh + 0.4, z1 + 0.5),
        Body,
    );
    b.block(
        Vec3::new(d0, dh - 0.07, z1 + 0.3),
        Vec3::new(d1, dh, z1 + 0.46),
        Trim,
    );
    // A level marker: short strips up the yard face beside the door.
    for k in 0..3 {
        let y = 4.0 + k as f32 * 3.2;
        b.block(
            Vec3::new(0.5, y, z1),
            Vec3::new(2.0, y + 0.1, z1 + 0.05),
            Trim,
        );
    }
}

/// A bastion's dress (on the back right corner): a battered skirt round its foot, a corbel and
/// fascia under its parapet as the walls have, neon bands.
pub(super) fn bastion_dress(b: &mut Builder, half: f32) {
    use Finish::{Body, Trim};
    let (a, e) = (half - BASTION_IN, half + BASTION_OUT);
    let h = WALL_H;
    let len = e - a;
    for (at, u, out) in [
        (Vec2::new(e, a), Vec2::Y, Vec2::X),
        (Vec2::new(a, e), Vec2::X, Vec2::Y),
    ] {
        let f = Face {
            a: at,
            u,
            out,
            len,
            doors: Vec::new(),
            windows: Vec::new(),
            clear: Vec::new(),
        };
        // (Only out past the walls' faces: the rest is inside the walls.)
        let s0 = BASTION_IN + 0.02;
        b.run(
            &f,
            s0,
            len,
            [(-0.05, -1.0), (1.5, -1.0), (0.25, 4.5), (-0.05, 4.5)],
            Body,
        );
        b.run(
            &f,
            s0,
            len,
            [
                (-0.05, h - 1.6),
                (0.1, h - 1.6),
                (0.7, h - 0.4),
                (-0.05, h - 0.4),
            ],
            Body,
        );
        b.run(
            &f,
            s0,
            len,
            [
                (-0.05, h - 0.4),
                (0.7, h - 0.4),
                (0.7, h + 1.35),
                (-0.05, h + 1.35),
            ],
            Body,
        );
        b.run(
            &f,
            s0,
            len,
            [
                (0.7, h + 0.2),
                (0.74, h + 0.2),
                (0.74, h + 0.34),
                (0.7, h + 0.34),
            ],
            Trim,
        );
        for y in [5.4, 8.2] {
            b.run(
                &f,
                s0,
                len,
                [(-0.02, y), (0.05, y), (0.05, y + 0.16), (-0.02, y + 0.16)],
                Trim,
            );
        }
        // A pier up the outer corner's side, capped.
        b.run(
            &f,
            len - 1.6,
            len,
            [(-0.05, 4.5), (0.45, 4.5), (0.45, h - 1.6), (-0.05, h - 1.6)],
            Body,
        );
    }
}

/// The gallery over a gateway (in the wall's frame; its canopy `gx` either way, at `gy`): lit
/// strips under the canopy's edges, a sign bar over the gateway outside.
pub(super) fn gallery_dress(b: &mut Builder, gx: f32, gy: f32) {
    use Finish::{Body, Trim};
    for (z0, z1) in [(-WALL_T - 0.32, -WALL_T - 0.26), (0.26, 0.32)] {
        b.block(
            Vec3::new(-gx, gy - 0.14, z0),
            Vec3::new(gx, gy - 0.02, z1),
            Trim,
        );
    }
    // The canopy's ribs across the walkway.
    let n = 5;
    for k in 0..n {
        let x = -gx + 1.0 + k as f32 * (2.0 * gx - 2.0) / (n - 1) as f32;
        b.block(
            Vec3::new(x - 0.12, gy - 0.3, -WALL_T - 0.3),
            Vec3::new(x + 0.12, gy, 0.3),
            Body,
        );
    }
}
