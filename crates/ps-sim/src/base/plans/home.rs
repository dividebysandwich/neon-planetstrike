//! The sanctuary homes' plans (no console: they aren't taken).
//!
//! - 0, the parade ground: the headquarters at the back of a wide yard, open sheds either
//!   side; bastions on all four corners.
//! - 1, the twin barracks: a command building in the middle, the spawn tubes in barracks
//!   left and right of it, the comms hut in the front yard; bastions on the front corners.
//! - 2, the depot: the main building in the middle, rows of open sheds along the back wall;
//!   bastions on all four corners.

use glam::Vec2;

use super::kit::{barracks, comms, cover, shed};
use super::{Ctx, join, yard};
use crate::base::builder::Builder;
use crate::base::building::{Bld, building};
use crate::base::ring::{Ring, ring};
use crate::base::*;

pub(in crate::base) fn plan(b: &mut Builder, cx: &mut Ctx) {
    let half = cx.half;
    let (bastions, stairs): ([bool; 4], Vec<(u8, f32)>) = match cx.variant {
        0 => (
            [true; 4],
            vec![(2, -40.0), (2, 40.0), (3, -40.0), (3, 40.0)],
        ),
        1 => (
            [true, true, false, false],
            vec![(0, -50.0), (0, 50.0), (1, -50.0), (1, 50.0)],
        ),
        _ => (
            [true; 4],
            vec![(2, -50.0), (2, 20.0), (3, -50.0), (3, 20.0)],
        ),
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
        0 => (Vec2::new(0.0, 44.0), Vec2::new(26.0, 16.0)),
        1 => (Vec2::new(0.0, 36.0), Vec2::new(16.0, 13.0)),
        _ => (BLD_C, BLD_HALF),
    };
    building(
        b,
        &Bld {
            c,
            half: bhalf,
            storeys: cx.storeys,
            console: false,
            spawns: cx.variant != 1,
            generator: true,
            style: cx.variant,
        },
        cx.rng,
    );
    join(b, "gate_in", "door_out");
    join(b, "back_out", "back_gate_in");
    match cx.variant {
        0 => {
            shed(b, (-74.0, -8.0, -56.0, 10.0), 3);
            shed(b, (56.0, -8.0, 74.0, 10.0), 2);
            for x in [-12.0, 12.0] {
                cover(b, x, 25.0, 5.0, true);
            }
            yard(
                b,
                &[(-40.0, -46.0), (40.0, -46.0)],
                &[(-60.0, 34.0), (60.0, 34.0)],
                (70.0, 64.0),
            );
        }
        1 => {
            let gate = b.marked("gate_in");
            for (k, r) in [(-58.0, 10.0, -34.0, 28.0), (34.0, 10.0, 58.0, 28.0)]
                .into_iter()
                .enumerate()
            {
                let (out, inn) = barracks(b, r, 0, 10.5, 3, 2);
                b.link(out, gate);
                if k == 0 {
                    b.mark("spawn", inn);
                }
            }
            comms(b, (20.0, -30.0, 26.0, -24.0), 2);
            yard(
                b,
                &[(-50.0, -50.0), (50.0, -50.0)],
                &[(42.0, -20.0), (-42.0, -20.0)],
                (60.0, 60.0),
            );
        }
        _ => {
            shed(b, (-70.0, 72.0, -20.0, 88.0), 0);
            shed(b, (20.0, 72.0, 70.0, 88.0), 0);
            yard(
                b,
                &[(-40.0, -46.0), (40.0, 46.0)],
                &[(42.0, -46.0), (-42.0, 46.0)],
                (58.0, -4.0),
            );
        }
    }
}
