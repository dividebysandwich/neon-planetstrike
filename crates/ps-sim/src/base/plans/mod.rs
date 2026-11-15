//! The bases' floor plans, by kind, and what they share: the walks through the gateways, the
//! yard's pads and silo, the capture points.

pub(super) mod tower;

use glam::Vec3;

use crate::base::builder::Builder;
use crate::base::building::{Bld, building};
use crate::base::ring::{Ring, ring};
use crate::base::*;
use crate::noise::Seq;

/// What a compound's plan is built from.
pub(super) struct Ctx<'a> {
    pub(super) kind: BaseKind,
    pub(super) half: f32,
    pub(super) storeys: u8,
    /// Capture points instead of the console.
    pub(super) points: bool,
    pub(super) rng: &'a mut Seq,
}

/// A compound (an outpost, a center base, a home) to its plan.
pub(super) fn compound(b: &mut Builder, cx: &mut Ctx) {
    gate_walks(b, cx.half);
    classic(b, cx);
    finish(b, cx.half);
}

/// The walk nodes outside and inside the gateways: `gate_out`, `gate_in` (the front's),
/// `back_gate_in`, `back_gate_out`.
fn gate_walks(b: &mut Builder, half: f32) {
    let inner = half - WALL_T;
    let f = b.chain(None, 0.0, &[(0.0, -half - 8.0), (0.0, -inner + 5.0)]);
    b.mark("gate_out", f[0]);
    b.mark("gate_in", f[1]);
    let k = b.chain(None, 0.0, &[(0.0, inner - 5.0), (0.0, half + 8.0)]);
    b.mark("back_gate_in", k[0]);
    b.mark("back_gate_out", k[1]);
}

/// A straight walk between two marked nodes.
pub(super) fn join(b: &mut Builder, from: &str, to: &str) {
    let (a, e) = (b.marked(from), b.marked(to));
    b.link(a, e);
}

/// The ways bots take, from the walk graph; where they rally.
fn finish(b: &mut Builder, half: f32) {
    b.out.to_console = b.walk("gate_out", "console");
    b.out.from_spawn = b.walk("spawn", "gate_out");
    if b.out.rally == Vec3::ZERO {
        b.out.rally = b.p(0.0, 0.0, -half * 0.5);
    }
}

/// The yard's fixtures: vehicle pads with their terminals, rearm pads, the silo.
pub(super) fn yard(b: &mut Builder, pads: &[(f32, f32)], rearm: &[(f32, f32)], silo: (f32, f32)) {
    for &(x, z) in pads {
        let i = b.out.pads.len();
        let at = b.p(x, 0.0, z);
        let yaw = b.yaw(0.0);
        b.out.pads.push((at, yaw));
        // The terminal beside the pad (towards the middle), facing the pad.
        let tx = x + if x < 0.0 { 11.0 } else { -11.0 };
        let pos = b.p(tx, 0.0, z);
        let yaw = b.yaw(if x < 0.0 {
            -std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        });
        b.out.terminals.push(Terminal {
            kind: TerminalKind::Vehicle,
            pos,
            yaw,
            pad: Some(i),
        });
    }
    for &(x, z) in rearm {
        let at = b.p(x, 0.0, z);
        b.out.rearm.push(at);
    }
    let s = b.p(silo.0, 0.0, silo.1);
    b.out.silo = Some(s);
    b.out
        .colliders
        .push(round_prism(s - Vec3::Y, SILO_R, SILO_H + 1.0, 8, 0.0));
    b.out.styles.push(Style::Pillar);
}

/// A capture point round `at`: its pylon, where bots stand in it, and their walk to it from
/// outside the front gateway: through the marked node `from`, then the points `via`.
pub(super) fn point(
    b: &mut Builder,
    at: (f32, f32),
    radius: f32,
    pylon: (f32, f32),
    stand: (f32, f32),
    from: &str,
    via: &[(f32, f32)],
) {
    let from = b.marked(from);
    let mut pts = via.to_vec();
    pts.push(stand);
    let to = *b.chain(Some(from), 0.0, &pts).last().unwrap();
    point_at(b, at, radius, pylon, to);
}

/// A capture point whose bots stand on walk node `stand`.
pub(super) fn point_at(
    b: &mut Builder,
    at: (f32, f32),
    radius: f32,
    pylon: (f32, f32),
    stand: usize,
) {
    let path = b.out.graph.path(b.marked("gate_out"), stand).unwrap();
    let pylon = b.p(pylon.0, 0.0, pylon.1);
    b.out
        .colliders
        .push(round_prism(pylon, PYLON_R, PYLON_H, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    let point = CapturePoint {
        pos: b.p(at.0, 0.0, at.1),
        radius,
        pylon,
        stand: b.out.graph.nodes[stand],
        path,
    };
    b.out.points.push(point);
}

/// The plan every compound had: the main building in the middle, bastions on the corners,
/// stair towers on the side walls.
fn classic(b: &mut Builder, cx: &mut Ctx) {
    let half = cx.half;
    let st = if cx.kind == BaseKind::Outpost {
        30.0
    } else {
        40.0
    };
    ring(
        b,
        &Ring {
            half,
            bastions: [true; 4],
            stairs: vec![(2, -st), (2, st), (3, -st), (3, st)],
            more_turrets: cx.kind != BaseKind::Outpost,
        },
    );
    let rooms = building(
        b,
        &Bld {
            c: BLD_C,
            half: BLD_HALF,
            storeys: cx.storeys,
            console: cx.kind != BaseKind::Home && !cx.points,
            spawns: true,
            generator: true,
            style: 0,
        },
        cx.rng,
    );
    join(b, "gate_in", "door_out");
    join(b, "back_out", "back_gate_in");
    if cx.points {
        // A in the front yard on the way in, B in the core room (where the console would
        // be), C in the back left of the yard.
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
        point(
            b,
            (-38.0, 36.0),
            8.0,
            (-38.0, 36.0),
            (-35.5, 33.5),
            "gate_in",
            &[(-30.0, -12.0), (-30.0, 24.0)],
        );
    }
    if cx.kind == BaseKind::Outpost {
        yard(b, &[(-34.0, -36.0)], &[(34.0, -36.0)], (42.0, 34.0));
    } else {
        yard(
            b,
            &[(-40.0, -46.0), (40.0, 46.0)],
            &[(42.0, -46.0), (-42.0, 46.0)],
            (58.0, -4.0),
        );
    }
}
