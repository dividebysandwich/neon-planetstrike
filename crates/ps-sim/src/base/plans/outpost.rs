//! The outposts' plans.
//!
//! - 0, the keep: the main building in the middle, bastions on all four corners, stair towers
//!   on the side walls.
//! - 1, the split yard: the main building to the left, the generator in its own annex to the
//!   right, a canopy over the vehicle terminal; bastions on the front corners, stair towers on
//!   the front and back walls.
//! - 2, the gate court: the main building set back behind a forecourt between an open shed
//!   and the comms hut by the side walls, cover by its door; bastions on the back corners,
//!   stair towers on the back wall and the front of the side walls.

use glam::Vec2;

use super::kit::{annex, canopy, comms, cover, shed};
use super::{Ctx, join, point, point_at, yard};
use crate::base::builder::Builder;
use crate::base::building::{Bld, building};
use crate::base::ring::{Ring, ring};
use crate::base::*;

pub(in crate::base) fn plan(b: &mut Builder, cx: &mut Ctx) {
    let half = cx.half;
    let (bastions, stairs): ([bool; 4], Vec<(u8, f32)>) = match cx.variant {
        0 => (
            [true; 4],
            vec![(2, -30.0), (2, 30.0), (3, -30.0), (3, 30.0)],
        ),
        1 => (
            [true, true, false, false],
            vec![(0, -46.0), (0, 46.0), (1, -46.0), (1, 46.0)],
        ),
        _ => (
            [false, false, true, true],
            vec![(2, -30.0), (3, -30.0), (1, -50.0), (1, 50.0)],
        ),
    };
    ring(
        b,
        &Ring {
            half,
            bastions,
            stairs,
            more_turrets: false,
        },
    );
    let (c, bhalf) = match cx.variant {
        0 => (BLD_C, BLD_HALF),
        1 => (Vec2::new(-22.0, 10.0), Vec2::new(16.0, 13.0)),
        _ => (Vec2::new(0.0, 16.0), Vec2::new(20.0, 13.0)),
    };
    let rooms = building(
        b,
        &Bld {
            c,
            half: bhalf,
            storeys: cx.storeys,
            console: !cx.points,
            spawns: true,
            generator: cx.variant != 1,
            style: cx.variant,
        },
        cx.rng,
    );
    join(b, "gate_in", "door_out");
    join(b, "back_out", "back_gate_in");
    // Capture points: A in the front yard on the way in, B in the core room (where the console
    // would be), C at the back of the yard.
    let c_point = match cx.variant {
        0 => {
            yard(b, &[(-34.0, -36.0)], &[(34.0, -36.0)], (42.0, 34.0));
            (
                (-38.0, 36.0),
                (-35.5, 33.5),
                vec![(-30.0, -12.0), (-30.0, 24.0)],
            )
        }
        1 => {
            let out = annex(b, (44.0, 4.0, 58.0, 16.0), 2, 1);
            let side = b.marked("side_out");
            b.link(out, side);
            yard(b, &[(-36.0, -40.0)], &[(36.0, -40.0)], (46.0, 40.0));
            canopy(b, (-28.0, -44.0, -22.0, -36.0), 4.2);
            (
                (30.0, 30.0),
                (27.5, 27.5),
                vec![(14.0, -10.0), (14.0, 26.0)],
            )
        }
        _ => {
            shed(b, (-58.0, -14.0, -44.0, 0.0), 3);
            let out = comms(b, (52.0, -10.0, 58.0, -4.0), 2);
            let door = b.marked("door_out");
            b.link(out, door);
            for x in [-8.0, 8.0] {
                cover(b, x, 0.0, 4.0, true);
            }
            yard(b, &[(-38.0, -38.0)], &[(38.0, -38.0)], (40.0, 42.0));
            (
                (-40.0, 38.0),
                (-37.5, 35.5),
                vec![(-26.0, -12.0), (-26.0, 26.0)],
            )
        }
    };
    if cx.points {
        point(
            b,
            (0.0, -30.0),
            8.0,
            (4.0, -30.0),
            (-2.5, -32.0),
            "gate_in",
            &[],
        );
        let console = b.marked("console");
        point_at(
            b,
            rooms.core,
            5.0,
            (rooms.core.0, rooms.core.1 + 3.0),
            console,
        );
        let (at, stand, via) = c_point;
        point(b, at, 8.0, at, stand, "gate_in", &via);
    }
}
