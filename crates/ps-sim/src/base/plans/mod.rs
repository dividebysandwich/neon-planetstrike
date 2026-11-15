//! The bases' floor plans: `PLANS` for each kind, picked by the base's seed (and mirrored left
//! to right or not), and what they share: the walks through the gateways, the yard's pads and
//! silo, the capture points.

pub(super) mod center;
pub(super) mod home;
pub(super) mod kit;
pub(super) mod outpost;
pub(super) mod tower;

use glam::Vec3;

use crate::base::builder::Builder;
use crate::base::*;
use crate::noise::Seq;

/// How many plans each kind of base has.
pub const PLANS: u8 = 3;

/// What a compound's plan is built from.
pub(super) struct Ctx<'a> {
    pub(super) kind: BaseKind,
    pub(super) half: f32,
    pub(super) storeys: u8,
    /// Which of its kind's plans.
    pub(super) variant: u8,
    /// Capture points instead of the console.
    pub(super) points: bool,
    pub(super) rng: &'a mut Seq,
}

/// A compound (an outpost, a center base, a home) to its plan.
pub(super) fn compound(b: &mut Builder, cx: &mut Ctx) {
    gate_walks(b, cx.half);
    match cx.kind {
        BaseKind::Outpost => outpost::plan(b, cx),
        BaseKind::Center => center::plan(b, cx),
        _ => home::plan(b, cx),
    }
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
