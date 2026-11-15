//! The towers' plans.
//!
//! - 0: closed walls with slits, the console on the top floor, two turrets on the roof.
//! - 1: the same with a railed balcony round the top floor, doors out onto it front and back.
//! - 2: the top floor an open deck between corner piers, railed; the console on the floor
//!   below; four turrets on the roof.

use glam::{Vec2, Vec3};

use crate::base::blocks::{rail, turret};
use crate::base::builder::{Builder, Gap, door};
use crate::base::facade::{Dress, Face, facade, vent};
use crate::base::*;

/// A tower: three storeys up walled stairs inside, the console upstairs, turrets on the roof
/// and the spawn bunker under the ground.
pub(in crate::base) fn tower(b: &mut Builder, variant: u8) {
    let h = TOWER_HALF;
    let top = STOREY * 3.0;
    let (balcony, deck) = (variant == 1, variant == 2);
    // (The walls' top: under the open deck, else the roof.)
    let walls = if deck { STOREY * 2.0 } else { top };
    let r90 = std::f32::consts::FRAC_PI_2;
    // Outer walls: a door front and back on the ground floor, slits above.
    let slits = |len: f32| -> Vec<Gap> {
        let mut g = vec![door(len / 2.0 - 2.0, 4.0, DOOR_H)];
        if balcony {
            // (A door out onto the balcony over the one on the ground.)
            g.push(Gap {
                off: len / 2.0 - 2.0,
                w: 4.0,
                lo: STOREY * 2.0,
                hi: STOREY * 2.0 + 2.8,
            });
        }
        for k in 1..if deck { 2 } else { 3 } {
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
        b.wall(a, e, T, 0.0, walls, gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, gaps));
    }
    let y2 = STOREY * 2.0;
    if deck {
        // Piers on the corners up to the roof, railings between them.
        let (p0, p1) = (h - 1.7, h + T / 2.0);
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (a, e) = (
                Vec3::new(sx * p0, y2, sz * p0),
                Vec3::new(sx * p1, top, sz * p1),
            );
            b.boxed(a.min(e), a.max(e), Style::Building);
        }
        for s in [-1.0f32, 1.0] {
            let (e0, e1) = (s * (h - RAIL_T), s * h);
            rail(b, (-p0, e0.min(e1), p0, e0.max(e1)), y2);
            rail(b, (e0.min(e1), -p0, e0.max(e1), p0), y2);
        }
        // (The stairs up to the roof keep their outer wall.)
        b.boxed(
            Vec3::new(-h - T / 2.0, y2, -6.3),
            Vec3::new(-h + T / 2.0, top + 0.6, 4.0),
            Style::Stairwell,
        );
    }
    if balcony {
        // The balcony round the top floor: out past the walls, railed.
        let (w0, w1) = (h + T / 2.0, h + T / 2.0 + 1.6);
        for (min, max) in [
            ((-w1, -w1), (w1, -w0)),
            ((-w1, w0), (w1, w1)),
            ((-w1, -w0), (-w0, w0)),
            ((w0, -w0), (w1, w0)),
        ] {
            b.boxed(
                Vec3::new(min.0, y2 - SLAB, min.1),
                Vec3::new(max.0, y2, max.1),
                Style::Floor,
            );
        }
        for s in [-1.0f32, 1.0] {
            let (e0, e1) = (s * (w1 - RAIL_T), s * w1);
            rail(b, (-w1, e0.min(e1), w1, e0.max(e1)), y2);
            rail(b, (e0.min(e1), -w1, e0.max(e1), w1), y2);
        }
    }
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys: 3,
            glazed: false,
            bays: false,
            style: 0,
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
    // The stairs: a ramp up the left wall from the ground floor, one up the right from the
    // first, one up the left again onto the roof; each walled off from its floor, the floor
    // above open over it and railed, the roof's under a head house.
    let (lx0, lx1) = (-h + T / 2.0, -h + T / 2.0 + RAMP_W);
    let (rx0, rx1) = (h - T / 2.0 - RAMP_W, h - T / 2.0);
    let r = top + 0.6;
    let left = (-h - T / 2.0, -6.0, lx1, 4.0);
    let right = (rx0, -4.0, h + T / 2.0, 6.0);
    let down = (rx0, -5.0, h + T / 2.0, 5.0);
    // The ground floor over the bunker (the stairwell down on the right).
    b.slab((-h, -h, h, h), 0.02, &[down], Style::Floor);
    // Floors 1 and 2 and the roof, each with its stairwell.
    b.slab((-h, -h, h, h), STOREY, &[left], Style::Floor);
    b.slab((-h, -h, h, h), STOREY * 2.0, &[right], Style::Floor);
    b.slab(
        (-h - T / 2.0, -h - T / 2.0, h + T / 2.0, h + T / 2.0),
        r,
        &[left],
        Style::Roof,
    );
    b.ramp(Vec3::new(lx0, 0.0, -6.0), Vec3::new(lx1, STOREY, 4.0), 0);
    b.ramp(
        Vec3::new(rx0, STOREY, -4.0),
        Vec3::new(rx1, STOREY * 2.0, 6.0),
        2,
    );
    b.ramp(
        Vec3::new(lx0, STOREY * 2.0, -6.0),
        Vec3::new(lx1, r, 4.0),
        0,
    );
    let d = -STOREY;
    b.ramp(Vec3::new(rx0, d, -5.0), Vec3::new(rx1, 0.0, 5.0), 2);
    // (Each stair: its inner side's x and which way is in, its foot's and head's z, the
    // floors it joins.)
    let pw = 0.3;
    for (x, sx, foot, head, y0, y1) in [
        (lx1, 1.0f32, -6.0f32, 4.0f32, 0.0f32, STOREY),
        (rx0, -1.0, 6.0, -4.0, STOREY, STOREY * 2.0),
        (lx1, 1.0, -6.0, 4.0, STOREY * 2.0, r),
        (rx0, -1.0, 5.0, -5.0, d, 0.02),
    ] {
        let (z0, z1) = (foot.min(head), foot.max(head));
        let (x0, x1) = (x.min(x + sx * pw), x.max(x + sx * pw));
        // The wall along its open side, up to the floor above.
        let roofed = y1 == r;
        let wall_top = if roofed { y1 + 2.6 } else { y1 - SLAB };
        b.boxed(
            Vec3::new(x0, y0, z0),
            Vec3::new(x1, wall_top, z1),
            Style::Stairwell,
        );
        // Across the foot's end of the stairwell above: a wall on the roof, else a railing
        // (and one along the well's side, over the wall).
        let fz = if foot < head {
            (z0 - pw, z0)
        } else {
            (z1, z1 + pw)
        };
        let (wx0, wx1) = if sx > 0.0 {
            (-h - T / 2.0, x1)
        } else {
            (x0, h + T / 2.0)
        };
        if roofed {
            b.boxed(
                Vec3::new(wx0, y1, fz.0),
                Vec3::new(wx1, wall_top, fz.1),
                Style::Stairwell,
            );
            // The head house's outer wall (on the parapet's line) and its roof.
            let ox = if sx > 0.0 { -h } else { h };
            b.boxed(
                Vec3::new(ox - T / 2.0, y1, z0 - pw),
                Vec3::new(ox + T / 2.0, wall_top, z1),
                Style::Stairwell,
            );
            b.boxed(
                Vec3::new(wx0 - 0.2, wall_top, z0 - pw - 0.2),
                Vec3::new(wx1 + 0.2, wall_top + 0.3, z1 + 0.6),
                Style::Canopy,
            );
        } else {
            rail(b, (x0, z0, x1, z1), y1);
            let (ix0, ix1) = if sx > 0.0 {
                (-h + T / 2.0, x1)
            } else {
                (x0, h - T / 2.0)
            };
            let fr = if foot < head {
                (z0 - RAIL_T, z0)
            } else {
                (z1, z1 + RAIL_T)
            };
            rail(b, (ix0, fr.0, ix1, fr.1), y1);
        }
    }
    // Roof parapet, two turrets by its corners.
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
    turret(b, (h - 2.5, r, -h + 2.5), (h - 4.4, -h + 3.4));
    turret(b, (-2.5, r, h - 2.5), (-0.9, h - 4.1));
    if deck {
        turret(b, (h - 2.5, r, h - 2.5), (h - 4.4, h - 3.4));
        turret(b, (-2.0, r, -h + 2.4), (-0.4, -h + 4.0));
    }
    // The console: upstairs, by the far wall.
    let cy = if deck { STOREY } else { y2 };
    let at = b.p(0.0, cy, 4.5);
    let yaw = b.yaw(0.0);
    b.out.console = Some((at, yaw));
    b.out.colliders.push(round_prism(at, 0.6, 1.1, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    b.decor(-4.0, cy, 6.5, Vec3::new(2.0, 1.0, 1.0));
    if deck {
        // (Cover on the open deck.)
        b.decor(0.0, y2, 0.0, Vec3::new(3.0, 1.2, 0.6));
        b.decor(-1.0, y2, 5.0, Vec3::new(0.6, 1.2, 3.0));
    }
    b.decor(3.0, STOREY, -6.5, Vec3::new(1.5, 1.5, 1.5));
    // The bunker: walls, floor, a forcefield at the foot of the ramp down.
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
    b.terminal(TerminalKind::Medical, -h + 4.0, d, h - 1.0, 0.0);
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
    b.out.interiors.push((a.min(e), a.max(e)));
    // Bots: in through the front door, up the ramps to the console; out of the bunker.
    let ramp_l = (lx0 + RAMP_W / 2.0, 0.0);
    let rr = rx0 + RAMP_W / 2.0;
    let mut way = vec![
        b.node(0.0, 0.0, -h - 5.0),
        b.node(0.0, 0.0, -h + 1.5),
        b.node(ramp_l.0, 0.0, -7.0),
        b.node(ramp_l.0, STOREY, 5.5),
    ];
    if deck {
        way.push(b.node(0.0, STOREY, 2.8));
    } else {
        way.extend([
            b.node(rr, STOREY, 7.0),
            b.node(rr, y2, -5.5),
            b.node(1.0, y2, -6.0),
            b.node(0.0, y2, 2.8),
        ]);
    }
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
    b.mark("console", *way.last().unwrap());
    b.mark("spawn", out[0]);
    b.out.to_console = b.walk("gate_out", "console");
    b.out.from_spawn = b.walk("spawn", "gate_out");
    b.out.rally = b.p(0.0, 0.0, -h - 6.0);
}
