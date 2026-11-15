//! The compound's ring: the walls with their walkways (a parapet outside, a railing inside),
//! the gateways under their lintels with galleries over them, bastions on the corners,
//! enclosed stair towers up to the walkways, turrets and floodlights.

use glam::{Vec2, Vec3};

use super::blocks::{ST_OUT, Stairs, rail, stairs, turret};
use super::builder::{Builder, Gap, rot};
use super::facade::{bastion_dress, gallery_dress, ramparts, stair_dress};
use super::*;

/// How a compound's ring is built.
pub(super) struct Ring {
    pub(super) half: f32,
    /// Bastions on the corners: front left, front right, back left, back right.
    pub(super) bastions: [bool; 4],
    /// Stair towers: on which wall (0 front, 1 back, 2 left, 3 right) and where along it (the
    /// base's x or z of its middle).
    pub(super) stairs: Vec<(u8, f32)>,
    /// Turrets on the side walls' middles and beside the back gateway (the big bases).
    pub(super) more_turrets: bool,
}

/// A wall's frame: it runs along x, its inner face on z = 0, the yard towards +z.
fn wall_frame(w: u8, inner: f32) -> (Vec2, u8) {
    match w {
        0 => (Vec2::new(0.0, -inner), 0),
        1 => (Vec2::new(0.0, inner), 2),
        2 => (Vec2::new(-inner, 0.0), 1),
        _ => (Vec2::new(inner, 0.0), 3),
    }
}

/// Which corner (front left, front right, back left, back right) the base-frame point is in.
fn corner_of(p: Vec2) -> usize {
    (p.x > 0.0) as usize + 2 * (p.y > 0.0) as usize
}

/// How far the turrets' pillars stand in from the walls' outer face, and their gunners from
/// the inner.
const TURRET_IN: f32 = 1.45;
const STAND_IN: f32 = 0.95;

pub(super) fn ring(b: &mut Builder, r: &Ring) {
    let half = r.half;
    let inner = half - WALL_T;
    let mid = half - WALL_T / 2.0;
    let top = WALL_H;
    for w in 0..4u8 {
        let (o, turn) = wall_frame(w, inner);
        // Its ends: at a bastion's edge, else the corner (the front and back walls') or the
        // inner corner (the side walls').
        let end = |sx: f32| -> (f32, f32) {
            let c = corner_of(o + rot(turn, Vec2::new(sx * half, -1.0)));
            if r.bastions[c] {
                (half - BASTION_IN, half - BASTION_IN)
            } else if w < 2 {
                (half, inner)
            } else {
                (inner, inner)
            }
        };
        let ((a0, r0), (a1, r1)) = (end(-1.0), end(1.0));
        let towers: Vec<f32> = r
            .stairs
            .iter()
            .filter(|s| s.0 == w)
            // (Where along the wall, in its frame.)
            .map(|s| match w {
                0 => s.1,
                1 => -s.1,
                2 => -s.1,
                _ => s.1,
            })
            .collect();
        b.at(o, 0.0, turn, false, |b| {
            // The wall; a gateway under its lintel on the front and back.
            let gate = [Gap {
                off: a0 - GATEWAY / 2.0,
                w: GATEWAY,
                lo: 0.0,
                hi: GATE_CLEAR + 2.0,
            }];
            let gaps: &[Gap] = if w < 2 { &gate } else { &[] };
            b.wall(
                Vec2::new(-a0, -WALL_T / 2.0),
                Vec2::new(a1, -WALL_T / 2.0),
                WALL_T,
                -2.0,
                top,
                gaps,
                Style::Wall,
            );
            b.wall(
                Vec2::new(-a0, -WALL_T + 0.3),
                Vec2::new(a1, -WALL_T + 0.3),
                0.6,
                top,
                top + PARAPET_H,
                &[],
                Style::Parapet,
            );
            // The railing along the walkway's inner edge, open where a stair tower comes up.
            let opens: Vec<Gap> = towers
                .iter()
                .map(|&x| Gap {
                    off: x - ST_OUT + r0,
                    w: 2.0 * ST_OUT,
                    lo: 0.0,
                    hi: 100.0,
                })
                .collect();
            b.wall(
                Vec2::new(-r0, -RAIL_T / 2.0),
                Vec2::new(r1, -RAIL_T / 2.0),
                RAIL_T,
                top,
                top + RAIL_H,
                &opens,
                Style::Rail,
            );
            for &x in &towers {
                b.at(Vec2::new(x, 0.0), 0.0, 0, false, |b| {
                    let s = Stairs {
                        heights: &[0.0, top / 3.0, top * 2.0 / 3.0, top],
                        run: 7.0,
                        top: top + 3.0,
                        walls: [Some(0.0), Some(0.0), Some(top), Some(0.0)],
                        doors: &[0, 3],
                        style: Style::Stairwell,
                    };
                    stairs(b, &s);
                    stair_dress(b, &s);
                });
            }
            if w < 2 {
                // The gateway's forcefield, and the gallery over the walkway above it.
                let f = b.world_box(
                    Vec3::new(-GATEWAY / 2.0, 0.0, -WALL_T / 2.0 - 0.4),
                    Vec3::new(GATEWAY / 2.0, GATE_CLEAR, -WALL_T / 2.0 + 0.4),
                );
                b.out.gate_fields.push(f);
                let (gx, gy) = (GATEWAY / 2.0 + 2.0, top + 3.0);
                b.boxed(
                    Vec3::new(-gx, gy, -WALL_T - 0.3),
                    Vec3::new(gx, gy + 0.4, 0.3),
                    Style::Canopy,
                );
                for s in [-1.0, 1.0] {
                    let (x0, x1) = (s * (gx - 0.5), s * gx);
                    b.boxed(
                        Vec3::new(x0.min(x1), top + PARAPET_H, -WALL_T),
                        Vec3::new(x0.max(x1), gy, -WALL_T + 0.6),
                        Style::Gatehouse,
                    );
                    b.boxed(
                        Vec3::new(x0.min(x1), top, -0.5),
                        Vec3::new(x0.max(x1), gy, 0.0),
                        Style::Gatehouse,
                    );
                }
                gallery_dress(b, gx, gy);
            }
        });
    }
    ramparts(b, half);
    // Bastions on the corners.
    for (c, &on) in r.bastions.iter().enumerate() {
        if on {
            // (Built for the back right corner, mirrored and turned to the others.)
            let (turn, flip) = [(2, false), (2, true), (0, true), (0, false)][c];
            b.at(Vec2::ZERO, 0.0, turn, flip, |b| bastion(b, half));
        }
    }
    // Turrets: the corners, beside the front gateway (and on the big bases the side walls'
    // middles and beside the back gateway).
    for (c, &on) in r.bastions.iter().enumerate() {
        let (sx, sz) = (
            if c % 2 == 1 { 1.0 } else { -1.0 },
            if c >= 2 { 1.0 } else { -1.0 },
        );
        if on {
            let (m, s) = (half - 3.5, half - 5.7);
            turret(b, (sx * m, top, sz * m), (sx * s, sz * s));
        } else {
            turret(b, (sx * mid, top, sz * mid), (sx * mid, sz * (mid - 2.4)));
        }
    }
    let g = GATEWAY / 2.0 + 4.0;
    let mut on_walls = vec![(0u8, -g), (0, g)];
    if r.more_turrets {
        on_walls.extend([(2, 0.0), (3, 0.0), (1, g), (1, -g)]);
    }
    for (w, x) in on_walls {
        let (o, turn) = wall_frame(w, inner);
        b.at(o, 0.0, turn, false, |b| {
            turret(b, (x, top, -WALL_T + TURRET_IN), (x, -STAND_IN));
        });
    }
    // Floodlights on the corners, lighting the yard.
    for (x, z) in [(-mid, -mid), (mid, -mid), (-mid, mid), (mid, mid)] {
        b.flood((x * 0.97, top + 3.5, z * 0.97), (x * 0.3, 0.0, z * 0.3));
    }
}

/// A bastion on the back right corner: a platform out past the walls' faces and into the
/// yard's corner, level with the walkways, a parapet round its outside and railings inside.
fn bastion(b: &mut Builder, half: f32) {
    let (a, e) = (half - BASTION_IN, half + BASTION_OUT);
    let inner = half - WALL_T;
    let (y, p) = (WALL_H, WALL_H + PARAPET_H);
    b.boxed(Vec3::new(a, -2.0, a), Vec3::new(e, y, e), Style::Bastion);
    for (min, max) in [
        // The outer sides, and the returns to the walls' parapets.
        ((e - 0.6, a), (e, e)),
        ((a, e - 0.6), (e - 0.6, e)),
        ((half, a), (e - 0.6, a + 0.6)),
        ((a, half), (a + 0.6, e - 0.6)),
    ] {
        b.boxed(
            Vec3::new(min.0, y, min.1),
            Vec3::new(max.0, p, max.1),
            Style::Parapet,
        );
    }
    rail(b, (a, a, a + RAIL_T, inner), y);
    rail(b, (a + RAIL_T, a, inner, a + RAIL_T), y);
    bastion_dress(b, half);
}
