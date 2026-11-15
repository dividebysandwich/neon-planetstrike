//! The center bases' plans.
//!
//! - 0, the citadel: a big keep inside an inner bailey (four corner walls with their own
//!   walkways and stairs, open to the front, back and sides); bastions on all four corners.
//! - 1, the twin halls: the command hall to the left (console and generator), barracks to the
//!   right (the spawn tubes), a covered walk between them; bastions on the front corners.
//! - 2, the quarters: the keep in the middle, the generator in its annex, an open shed and the
//!   comms hut in the yard's quarters; bastions on all four corners.

use glam::{Vec2, Vec3};

use super::kit::{annex, barracks, canopy, comms, shed};
use super::{Ctx, join, yard};
use crate::base::blocks::{ST_OUT, Stairs, rail, stairs};
use crate::base::builder::{Builder, Gap};
use crate::base::building::{Bld, building};
use crate::base::facade::stair_dress;
use crate::base::ring::{Ring, ring};
use crate::base::*;

pub(in crate::base) fn plan(b: &mut Builder, cx: &mut Ctx) {
    let half = cx.half;
    let (bastions, stairs): ([bool; 4], Vec<(u8, f32)>) = match cx.variant {
        0 => (
            [true; 4],
            vec![(0, -60.0), (0, 60.0), (1, -60.0), (1, 60.0)],
        ),
        1 => (
            [true, true, false, false],
            vec![(0, -50.0), (0, 50.0), (1, -50.0), (1, 50.0)],
        ),
        _ => ([true; 4], vec![(0, -50.0), (0, 50.0), (2, 44.0), (3, 44.0)]),
    };
    ring(
        b,
        &Ring {
            half,
            bastions,
            stairs,
            more_turrets: true,
        },
    );
    let (c, bhalf) = match cx.variant {
        0 => (Vec2::new(0.0, 10.0), Vec2::new(22.0, 16.0)),
        1 => (Vec2::new(-28.0, 12.0), Vec2::new(16.0, 14.0)),
        _ => (BLD_C, BLD_HALF),
    };
    building(
        b,
        &Bld {
            c,
            half: bhalf,
            storeys: cx.storeys,
            console: true,
            spawns: cx.variant != 1,
            generator: cx.variant != 2,
            style: cx.variant,
        },
        cx.rng,
    );
    join(b, "gate_in", "door_out");
    join(b, "back_out", "back_gate_in");
    match cx.variant {
        0 => {
            for (turn, flip) in [(2, false), (2, true), (0, true), (0, false)] {
                b.at(c, 0.0, turn, flip, bailey);
            }
            yard(
                b,
                &[(-62.0, -50.0), (62.0, 50.0)],
                &[(62.0, -50.0), (-62.0, 50.0)],
                (34.0, -58.0),
            );
        }
        1 => {
            // (The barracks' door under the covered walk from the hall's side door.)
            let (out, inn) = barracks(b, (14.0, 2.0, 42.0, 22.0), 2, 1.0, 6, 2);
            b.mark("spawn", inn);
            let side = b.marked("side_out");
            b.link(out, side);
            let gate = b.marked("gate_in");
            b.link(out, gate);
            canopy(b, (-11.0, 2.2, 13.0, 6.8), 4.4);
            yard(
                b,
                &[(-40.0, -52.0), (40.0, 56.0)],
                &[(42.0, -52.0), (-42.0, 56.0)],
                (62.0, -20.0),
            );
        }
        _ => {
            let out = annex(b, (40.0, -6.0, 54.0, 6.0), 2, 2);
            let side = b.marked("side_out");
            b.link(out, side);
            shed(b, (-70.0, -20.0, -50.0, 0.0), 3);
            comms(b, (-60.0, 54.0, -54.0, 60.0), 0);
            yard(
                b,
                &[(-40.0, -46.0), (40.0, 46.0)],
                &[(42.0, -46.0), (-42.0, 46.0)],
                (64.0, -30.0),
            );
        }
    }
}

/// The inner bailey's walls: how far out from the keep's middle, how long each leg is, how
/// high.
const BAILEY: f32 = 40.0;
const BAILEY_LEG: f32 = 24.0;
const BAILEY_H: f32 = 6.0;

/// A corner of the inner bailey (the back right one, round the keep's middle): two legs of
/// wall with a railed walkway along their top, stairs up to it inside the right leg.
fn bailey(b: &mut Builder) {
    let (r, t, h) = (BAILEY, WALL_T, BAILEY_H);
    let a = r - BAILEY_LEG;
    b.boxed(Vec3::new(r - t, -1.0, a), Vec3::new(r, h, r), Style::Wall);
    b.boxed(
        Vec3::new(a, -1.0, r - t),
        Vec3::new(r - t, h, r),
        Style::Wall,
    );
    // Railings round the walkway: the outer edges, the ends, the inner edges (open at the
    // stairs' head).
    let e = RAIL_T;
    rail(b, (r - e, a, r, r), h);
    rail(b, (a, r - e, r - e, r), h);
    rail(b, (r - t, a, r - e, a + e), h);
    rail(b, (a, r - t, a + e, r - e), h);
    rail(b, (a + e, r - t, r - t, r - t + e), h);
    let zc = r - 14.0;
    b.wall(
        Vec2::new(r - t + e / 2.0, a + e),
        Vec2::new(r - t + e / 2.0, r - t),
        e,
        h,
        h + RAIL_H,
        &[Gap {
            off: zc - ST_OUT - (a + e),
            w: 2.0 * ST_OUT,
            lo: 0.0,
            hi: 100.0,
        }],
        Style::Rail,
    );
    b.at(Vec2::new(r - t, zc), 0.0, 3, false, |b| {
        let s = Stairs {
            heights: &[0.0, h / 3.0, h * 2.0 / 3.0, h],
            run: 3.5,
            top: h + 3.0,
            walls: [Some(0.0), Some(0.0), Some(h), Some(0.0)],
            doors: &[0, 3],
            style: Style::Stairwell,
        };
        stairs(b, &s);
        stair_dress(b, &s);
    });
}
