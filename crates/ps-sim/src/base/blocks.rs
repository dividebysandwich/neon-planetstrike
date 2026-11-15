//! The pieces bases are built of: railings, enclosed stairs, turret mounts.

use glam::{Vec2, Vec3};

use super::builder::{Builder, Gap};
use super::*;

/// A railing along an edge: over the rectangle (x0, z0, x1, z1) standing on `y`. Too high to
/// walk over, low enough to jump.
pub(super) fn rail(b: &mut Builder, r: (f32, f32, f32, f32), y: f32) {
    b.boxed(
        Vec3::new(r.0, y, r.1),
        Vec3::new(r.2, y + RAIL_H, r.3),
        Style::Rail,
    );
}

/// A turret on its pillar standing on (x, y, z), and where its gunner gets out.
pub(super) fn turret(b: &mut Builder, at: (f32, f32, f32), stand: (f32, f32)) {
    let foot = b.p(at.0, at.1, at.2);
    b.out.turrets.push(foot + Vec3::Y * TURRET_RISE);
    b.out
        .colliders
        .push(round_prism(foot, TURRET_PILLAR, TURRET_RISE, 8, 0.0));
    b.out.styles.push(Style::Mount);
    let s = b.p(stand.0, at.1, stand.1);
    b.out.turret_stands.push(s);
}

/// The lanes' width, the spine wall's between them, the shell's walls' and the landings' depth.
pub(super) const ST_LANE: f32 = 2.0;
pub(super) const ST_SPINE: f32 = 0.4;
pub(super) const ST_WALL: f32 = 0.4;
pub(super) const ST_LAND: f32 = 2.2;
/// Half the stairs' width inside, and over the shell.
pub(super) const ST_HALF: f32 = ST_LANE + ST_SPINE / 2.0;
pub(super) const ST_OUT: f32 = ST_HALF + ST_WALL;
const ST_DOOR_H: f32 = 2.6;

/// Enclosed switchback stairs, in their own frame: inside x -`ST_HALF`..`ST_HALF` (two lanes
/// either side of a spine wall), z 0 (the far end) .. `len()` (the near end, where one walks
/// in on the ground). Flights climb from the near end to the far in the left lane (x < 0) and
/// back in the right, with a landing across both lanes at each end: `heights[k]` is the k-th
/// landing's (0: the ground at the near end; odd ones at the far end).
pub(super) struct Stairs<'a> {
    pub(super) heights: &'a [f32],
    /// A flight's length.
    pub(super) run: f32,
    /// The walls' top (the roof lies on it).
    pub(super) top: f32,
    /// From what height each side's wall is built (-x, +x, far, near): `None` where the host
    /// has a wall all the way up, a height where its wall ends.
    pub(super) walls: [Option<f32>; 4],
    /// The landings with a doorway out (through their end's wall, in line with the left lane).
    pub(super) doors: &'a [usize],
    pub(super) style: Style,
}

impl Stairs<'_> {
    pub(super) fn len(&self) -> f32 {
        2.0 * ST_LAND + self.run
    }
}

/// Builds the stairs; the walk nodes outside each doorway, in `doors`' order.
pub(super) fn stairs(b: &mut Builder, s: &Stairs) -> Vec<usize> {
    let len = s.len();
    let (h, n) = (s.heights, s.heights.len() - 1);
    let (near0, far1) = (len - ST_LAND, ST_LAND);
    let lane = |k: usize| -> (f32, f32) {
        if k.is_multiple_of(2) {
            (-ST_HALF, -ST_SPINE / 2.0)
        } else {
            (ST_SPINE / 2.0, ST_HALF)
        }
    };
    // The flights.
    for k in 0..n {
        let (x0, x1) = lane(k);
        b.ramp(
            Vec3::new(x0, h[k], far1),
            Vec3::new(x1, h[k + 1], near0),
            if k.is_multiple_of(2) { 2 } else { 0 },
        );
    }
    // The landings.
    for (k, &y) in h.iter().enumerate().skip(1) {
        let (z0, z1) = if k % 2 == 1 {
            (0.0, far1)
        } else {
            (near0, len)
        };
        b.boxed(
            Vec3::new(-ST_HALF, y - SLAB, z0),
            Vec3::new(ST_HALF, y, z1),
            Style::Floor,
        );
    }
    // The spine between the lanes, the wall closing the space under the second flight, and a
    // railing across the open lane at the top landing.
    b.boxed(
        Vec3::new(-ST_SPINE / 2.0, 0.0, far1),
        Vec3::new(ST_SPINE / 2.0, s.top, near0),
        s.style,
    );
    if n > 1 {
        b.boxed(
            Vec3::new(ST_SPINE / 2.0, 0.0, near0 - 0.2),
            Vec3::new(ST_HALF, h[1], near0),
            s.style,
        );
    }
    {
        let (x0, x1) = lane(n);
        let z = if n % 2 == 1 { far1 } else { near0 - RAIL_T };
        rail(b, (x0, z, x1, z + RAIL_T), h[n]);
    }
    // The shell: side walls with slits at the landings, end walls with the doorways.
    let slit = |y: f32| (y + 1.3, y + 2.1);
    for (side, x) in [
        (0usize, -ST_HALF - ST_WALL / 2.0),
        (1, ST_HALF + ST_WALL / 2.0),
    ] {
        let Some(y0) = s.walls[side] else { continue };
        let mut gaps = Vec::new();
        for (k, &y) in h.iter().enumerate().skip(1) {
            let (lo, hi) = slit(y);
            if lo > y0 + 0.5 && hi < s.top - 0.4 {
                let off = if k % 2 == 1 { 0.6 } else { len - 1.6 };
                gaps.push(Gap {
                    off,
                    w: 1.0,
                    lo: lo - y0,
                    hi: hi - y0,
                });
            }
        }
        b.wall(
            Vec2::new(x, 0.0),
            Vec2::new(x, len),
            ST_WALL,
            y0,
            s.top,
            &gaps,
            s.style,
        );
    }
    for (side, z) in [(2usize, -ST_WALL / 2.0), (3, len + ST_WALL / 2.0)] {
        let Some(y0) = s.walls[side] else { continue };
        let far = side == 2;
        let mut gaps = Vec::new();
        for (k, &y) in h.iter().enumerate() {
            if (k % 2 == 1) != far {
                continue;
            }
            if s.doors.contains(&k) {
                gaps.push(Gap {
                    off: ST_WALL,
                    w: ST_LANE,
                    lo: y - y0,
                    hi: y - y0 + ST_DOOR_H,
                });
            }
            // A slit in line with the right lane.
            let (lo, hi) = slit(y);
            if k > 0 && lo > y0 + 0.5 && hi < s.top - 0.4 {
                gaps.push(Gap {
                    off: ST_OUT + 0.7,
                    w: 1.0,
                    lo: lo - y0,
                    hi: hi - y0,
                });
            }
        }
        b.wall(
            Vec2::new(-ST_OUT, z),
            Vec2::new(ST_OUT, z),
            ST_WALL,
            y0,
            s.top,
            &gaps,
            s.style,
        );
    }
    // The roof.
    let o = ST_OUT + 0.3;
    b.boxed(
        Vec3::new(-o, s.top, -ST_WALL - 0.3),
        Vec3::new(o, s.top + 0.3, len + ST_WALL + 0.3),
        Style::Canopy,
    );
    b.lamp(0.0, s.top - 0.5, len / 2.0);
    // The walk: in at the ground, up lane by lane, out of each doorway.
    let (xa, xb) = (-ST_HALF + ST_LANE / 2.0, ST_HALF - ST_LANE / 2.0);
    let zk = |k: usize| {
        if k % 2 == 1 {
            far1 / 2.0
        } else {
            len - far1 / 2.0
        }
    };
    let mut outs = Vec::new();
    let mut last = b.node(xa, h[0], zk(0));
    let mut left = last;
    for (k, &y) in h.iter().enumerate() {
        if k > 0 {
            // (Up flight k - 1 in its lane, then across the landing.)
            let (xin, xout) = if (k - 1).is_multiple_of(2) {
                (xa, xb)
            } else {
                (xb, xa)
            };
            let a = b.node(xin, y, zk(k));
            let c = b.node(xout, y, zk(k));
            b.link(last, a);
            b.link(a, c);
            last = c;
            left = if xin < 0.0 { a } else { c };
        }
        if s.doors.contains(&k) {
            let z = if k % 2 == 1 {
                -ST_WALL - 1.6
            } else {
                len + ST_WALL + 1.6
            };
            let out = b.node(xa, y, z);
            b.link(left, out);
            outs.push(out);
        }
    }
    outs
}
