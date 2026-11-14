//! The towers.

use glam::{Vec2, Vec3};

use crate::base::builder::{Builder, Gap, door};
use crate::base::facade::{Dress, Face, facade, vent};
use crate::base::*;

/// A tower: three storeys up ramps inside, the console on the top floor, turrets on the roof
/// and the spawn bunker under the ground.
pub(in crate::base) fn tower(b: &mut Builder) {
    let h = TOWER_HALF;
    let top = STOREY * 3.0;
    let r90 = std::f32::consts::FRAC_PI_2;
    // Outer walls: a door front and back on the ground floor, slits above.
    let slits = |len: f32| -> Vec<Gap> {
        let mut g = vec![door(len / 2.0 - 2.0, 4.0, DOOR_H)];
        for k in 1..3 {
            g.push(Gap {
                off: 3.0,
                w: 2.0,
                lo: k as f32 * STOREY + 1.2,
                hi: k as f32 * STOREY + 2.6,
            });
            g.push(Gap {
                off: len - 5.0,
                w: 2.0,
                lo: k as f32 * STOREY + 1.2,
                hi: k as f32 * STOREY + 2.6,
            });
        }
        g
    };
    let (fb, sides) = (slits(2.0 * h), Vec::new());
    let mut faces = Vec::new();
    for (a, e, out, gaps) in [
        (Vec2::new(-h, -h), Vec2::new(h, -h), Vec2::NEG_Y, &fb),
        (Vec2::new(-h, h), Vec2::new(h, h), Vec2::Y, &fb),
        (Vec2::new(-h, -h), Vec2::new(-h, h), Vec2::NEG_X, &sides),
        (Vec2::new(h, -h), Vec2::new(h, h), Vec2::X, &sides),
    ] {
        b.wall(a, e, T, 0.0, top, gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, gaps));
    }
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys: 3,
            glazed: false,
            bays: false,
        },
    );
    // The blank sides: a vent low down and a conduit up to the cornice.
    for f in &faces[2..] {
        vent(b, f, f.len / 2.0 - 1.4, f.len / 2.0 + 1.4, 1.0);
        let m = f.len / 2.0 + 3.2;
        b.run(
            f,
            m - 0.18,
            m + 0.18,
            [
                (-0.05, 0.6),
                (0.36, 0.6),
                (0.36, top - 1.5),
                (-0.05, top - 1.5),
            ],
            Finish::Body,
        );
        for y in [2.0, 6.0, 10.0] {
            b.run(
                f,
                m - 0.26,
                m + 0.26,
                [(-0.05, y), (0.44, y), (0.44, y + 0.25), (-0.05, y + 0.25)],
                Finish::Body,
            );
        }
    }
    // The apron round its foot (over the land's cut-out round the bunker).
    let (w, a) = (h + T / 2.0, h + 5.5);
    for (min, max) in [
        (Vec3::new(-a, -0.5, -a), Vec3::new(a, 0.02, -w)),
        (Vec3::new(-a, -0.5, w), Vec3::new(a, 0.02, a)),
        (Vec3::new(-a, -0.5, -w), Vec3::new(-w, 0.02, w)),
        (Vec3::new(w, -0.5, -w), Vec3::new(a, 0.02, w)),
    ] {
        b.boxed(min, max, Style::Floor);
    }
    // Stairwells: over each ramp, ending where the ramp reaches the floor above.
    let left = (-h, -7.0, -h + RAMP_W + 0.5, 4.0);
    let right = (h - RAMP_W - 0.5, -4.0, h, 7.0);
    let down = (h - RAMP_W - 0.5, -6.0, h, 6.0);
    // The ground floor over the bunker (the stairwell down on the right).
    b.slab((-h, -h, h, h), 0.02, &[down], Style::Floor);
    // Floors 1 and 2 and the roof, each with its stairwell.
    b.slab((-h, -h, h, h), STOREY, &[left], Style::Floor);
    b.slab((-h, -h, h, h), STOREY * 2.0, &[right], Style::Floor);
    b.slab(
        (-h - T / 2.0, -h - T / 2.0, h + T / 2.0, h + T / 2.0),
        top + 0.6,
        &[left],
        Style::Roof,
    );
    let (lx0, lx1) = (-h + T / 2.0, -h + T / 2.0 + RAMP_W);
    let (rx0, rx1) = (h - T / 2.0 - RAMP_W, h - T / 2.0);
    b.ramp(Vec3::new(lx0, 0.0, -6.0), Vec3::new(lx1, STOREY, 4.0), 0);
    b.ramp(
        Vec3::new(rx0, STOREY, -4.0),
        Vec3::new(rx1, STOREY * 2.0, 6.0),
        2,
    );
    b.ramp(
        Vec3::new(lx0, STOREY * 2.0, -6.0),
        Vec3::new(lx1, top + 0.6, 4.0),
        0,
    );
    // Roof parapet, two turrets on its corners.
    let r = top + 0.6;
    for (a, e) in [((-h, -h), (h, -h)), ((-h, h), (h, h))] {
        b.wall(
            Vec2::new(a.0, a.1),
            Vec2::new(e.0, e.1),
            0.4,
            r,
            r + 1.0,
            &[],
            Style::Parapet,
        );
    }
    for x in [-h, h] {
        b.wall(
            Vec2::new(x, -h),
            Vec2::new(x, h),
            0.4,
            r,
            r + 1.0,
            &[],
            Style::Parapet,
        );
    }
    for (x, z) in [(h - 2.5, -h + 2.5), (-h + 2.5, h - 2.5)] {
        let p = b.p(x, r + TURRET_RISE, z);
        b.out.turrets.push(p);
    }
    // The console: the top floor, by the far wall.
    let y2 = STOREY * 2.0;
    let at = b.p(0.0, y2, 4.5);
    let yaw = b.yaw(0.0);
    b.out.console = Some((at, yaw));
    b.out.colliders.push(round_prism(at, 0.6, 1.1, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    b.decor(-4.0, y2, 6.5, Vec3::new(2.0, 1.0, 1.0));
    b.decor(3.0, STOREY, -6.5, Vec3::new(1.5, 1.5, 1.5));
    // The bunker: walls, floor, the ramp down, a forcefield at its foot.
    let d = -STOREY;
    b.slab((-h, -h, h, h), d, &[], Style::Floor);
    for (a, e) in [((-h, -h), (h, -h)), ((-h, h), (h, h))] {
        b.wall(
            Vec2::new(a.0, a.1),
            Vec2::new(e.0, e.1),
            T,
            d - 1.0,
            0.0,
            &[],
            Style::Interior,
        );
    }
    for x in [-h, h] {
        b.wall(
            Vec2::new(x, -h),
            Vec2::new(x, h),
            T,
            d - 1.0,
            0.0,
            &[],
            Style::Interior,
        );
    }
    b.ramp(Vec3::new(rx0, d, -6.0), Vec3::new(rx1, 0.0, 5.0), 2);
    let f0 = b.p(rx0, d, 5.2);
    let f1 = b.p(rx1, d + 3.0, 5.5);
    b.out.shields.push(Shield {
        collider: Collider::Box {
            min: f0.min(f1),
            max: f0.max(f1),
        },
    });
    for (px, pz) in [(-4.5, -3.5), (-4.5, 1.0), (-1.0, -3.5), (-1.0, 1.0)] {
        let at = b.p(px, d, pz);
        let yaw = b.yaw(0.0);
        b.out.spawns.push((at, yaw));
    }
    b.terminal(TerminalKind::Equipment, -h + 1.0, d, 5.5, -r90);
    b.terminal(
        TerminalKind::Medical,
        -h + 4.0,
        d,
        h - 1.0,
        std::f32::consts::PI,
    );
    b.lamp(0.0, -0.8, 0.0);
    b.lamp(0.0, STOREY - 0.6, 0.0);
    b.lamp(0.0, STOREY * 2.0 - 0.6, 0.0);
    b.lamp(0.0, top - 0.6, 0.0);
    let hole = {
        let a = b.p(-h - 0.5, 0.0, -h - 0.5);
        let e = b.p(h + 0.5, 0.0, h + 0.5);
        (
            Vec2::new(a.x.min(e.x), a.z.min(e.z)),
            Vec2::new(a.x.max(e.x), a.z.max(e.z)),
        )
    };
    b.out.holes.push(hole);
    b.out.storeys = 3;
    let a = b.p(-h, d, -h);
    let e = b.p(h, top, h);
    b.out.building = (a.min(e), a.max(e));
    // Bots: in through the front door, up the ramps to the console; out of the bunker.
    let ramp_l = (lx0 + RAMP_W / 2.0, 0.0);
    let rr = rx0 + RAMP_W / 2.0;
    let way = [
        b.node(0.0, 0.0, -h - 5.0),
        b.node(0.0, 0.0, -h + 1.5),
        b.node(ramp_l.0, 0.0, -7.0),
        b.node(ramp_l.0, STOREY, 5.5),
        b.node(rr, STOREY, 7.0),
        b.node(rr, y2, -5.5),
        b.node(1.0, y2, -6.0),
        b.node(0.0, y2, 2.8),
    ];
    let out = [
        b.node(-2.0, d, 6.5),
        b.node(rr, d, 6.5),
        b.node(rr, 0.0, -6.8),
    ];
    for w in way.windows(2).chain(out.windows(2)) {
        b.link(w[0], w[1]);
    }
    b.link(out[2], way[1]);
    b.mark("gate_out", way[0]);
    b.mark("console", way[7]);
    b.mark("spawn", out[0]);
    b.out.to_console = b.walk("gate_out", "console");
    b.out.from_spawn = b.walk("spawn", "gate_out");
}
