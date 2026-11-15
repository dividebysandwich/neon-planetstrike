//! The main building: the hall with its terminals, the spawn room behind its forcefield, the
//! core room with the console, the generator room, enclosed stairs in its front left corner up
//! to the offices and out onto the roof.

use glam::{Vec2, Vec3};

use super::blocks::{ST_HALF, Stairs, stairs};
use super::builder::{Builder, Gap, door};
use super::facade::{Dress, Face, facade};
use super::*;
use crate::noise::Seq;

/// How a main building is built.
pub(super) struct Bld {
    /// Its middle and half extents (its front towards -Z).
    pub(super) c: Vec2,
    pub(super) half: Vec2,
    pub(super) storeys: u8,
    /// The console in the core room.
    pub(super) console: bool,
    /// The spawn room's tubes and forcefield (else it's a ready room with its terminals).
    pub(super) spawns: bool,
    /// The generator in its room (else a store).
    pub(super) generator: bool,
    /// How its facade is dressed (`Dress::style`).
    pub(super) style: u8,
}

/// Where the building's rooms are, for the plans (base frame).
pub(super) struct Rooms {
    /// The core room's middle.
    pub(super) core: (f32, f32),
}

/// The stairs' heights and flights' run in a building of `storeys`.
fn flights(storeys: u8) -> (Vec<f32>, f32) {
    let roof = STOREY * storeys as f32 + 0.6;
    let mut h = vec![0.0];
    for k in 1..storeys {
        let y = k as f32 * STOREY;
        h.extend([y - STOREY / 2.0, y]);
    }
    let last = *h.last().unwrap();
    h.extend([(last + roof) / 2.0, roof]);
    (h, 4.8)
}

/// Builds it; its walk nodes are marked `door_out`, `door_in`, `hall`, `back_in`, `back_out`,
/// `side_out`, `console` and `spawn`.
pub(super) fn building(b: &mut Builder, s: &Bld, rng: &mut Seq) -> Rooms {
    let c = s.c;
    let (hx, hz) = (s.half.x, s.half.y);
    let (x0, x1, z0, z1) = (c.x - hx, c.x + hx, c.y - hz, c.y + hz);
    let storeys = s.storeys;
    let top = STOREY * storeys as f32;
    let r = top + 0.6;
    let middle = |len: f32| door(len / 2.0 - DOOR_W / 2.0, DOOR_W, DOOR_H);
    // Windows on the upper storey: a row of gaps from 1.2 m to 3.6 m above its floor.
    let windows = |len: f32| -> Vec<Gap> {
        let n = (len / 7.0) as usize;
        (0..n)
            .map(|i| Gap {
                off: 2.0 + i as f32 * 7.0,
                w: 3.5,
                lo: STOREY + 1.2,
                hi: STOREY + 3.6,
            })
            .filter(|g| g.off + g.w < len - 1.0)
            .collect()
    };
    // (Windows that would cut into a door are left out.)
    let with = |mut a: Vec<Gap>, b2: Vec<Gap>| {
        let keep: Vec<Gap> = b2
            .into_iter()
            .filter(|w| {
                a.iter()
                    .all(|d| w.off + w.w < d.off - 0.5 || w.off > d.off + d.w + 0.5)
            })
            .collect();
        a.extend(keep);
        a
    };
    // (None over the stairs in the front left corner.)
    let upper = |len: f32, from: f32| -> Vec<Gap> {
        if storeys > 1 {
            windows(len).into_iter().filter(|g| g.off > from).collect()
        } else {
            Vec::new()
        }
    };
    // The stairs' place: inside the front left corner.
    let (heights, run) = flights(storeys);
    let st_len = 2.0 * super::blocks::ST_LAND + run;
    let (sx0, sz0) = (x0 + T / 2.0, z0 + T / 2.0);
    let (sx1, sz1) = (sx0 + 2.0 * ST_HALF, sz0 + st_len);
    // Outer walls: doors front and back, and on the right side into the generator room.
    let front = with(vec![middle(2.0 * hx)], upper(2.0 * hx, sx1 - x0 + 1.0));
    let back = with(vec![middle(2.0 * hx)], upper(2.0 * hx, 0.0));
    let left = upper(2.0 * hz, sz1 - z0 + 1.0);
    let right = with(vec![door(4.0, DOOR_W, DOOR_H)], upper(2.0 * hz, 0.0));
    let sides = [
        (Vec2::new(x0, z0), Vec2::new(x1, z0), Vec2::NEG_Y, &front),
        (Vec2::new(x0, z1), Vec2::new(x1, z1), Vec2::Y, &back),
        (Vec2::new(x0, z0), Vec2::new(x0, z1), Vec2::NEG_X, &left),
        (Vec2::new(x1, z0), Vec2::new(x1, z1), Vec2::X, &right),
    ];
    let mut faces = Vec::new();
    for (a, e, out, gaps) in sides {
        b.wall(a, e, T, 0.0, top, gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, gaps));
    }
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys,
            glazed: true,
            bays: true,
            style: s.style,
        },
    );
    // The floors and the roof, open over the stairs.
    for k in 1..storeys {
        b.slab(
            (x0, z0, x1, z1),
            k as f32 * STOREY,
            &[(x0, z0, sx1, sz1)],
            Style::Floor,
        );
    }
    b.slab(
        (x0 - T / 2.0, z0 - T / 2.0, x1 + T / 2.0, z1 + T / 2.0),
        r,
        &[(x0 - T / 2.0, z0 - T / 2.0, sx1, sz1)],
        Style::Roof,
    );
    // The stairs: a doorway on every floor and out onto the roof from their head house.
    let doors: Vec<usize> = (0..heights.len()).step_by(2).collect();
    let outs = b.at(Vec2::new(sx0 + ST_HALF, sz0), 0.0, 0, false, |b| {
        stairs(
            b,
            &Stairs {
                heights: &heights,
                run,
                top: r + 2.8,
                walls: [Some(top), Some(0.0), Some(top), Some(0.0)],
                doors: &doors,
                style: Style::Stairwell,
            },
        )
    });
    // A parapet round the roof.
    for (a, e) in [
        ((x0, z0), (x1, z0)),
        ((x0, z1), (x1, z1)),
        ((x0, z0), (x0, z1)),
        ((x1, z0), (x1, z1)),
    ] {
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
    // On the roof: vent stacks and plant on the right, an antenna mast.
    b.decor(x1 - 5.0, r, z0 + 5.0, Vec3::new(4.0, 1.4, 3.0));
    b.decor(x1 - 5.0, r + 1.4, z0 + 5.0, Vec3::new(2.4, 0.6, 2.0));
    b.decor(x1 - 4.0, r, z1 - 6.0, Vec3::new(2.4, 1.0, 4.0));
    b.decor(c.x + 4.0, r, z1 - 3.0, Vec3::new(1.6, 2.2, 1.6));
    let mast = b.p(x1 - 2.5, r, c.y);
    b.out.colliders.push(round_prism(mast, 0.3, 7.0, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    // (Ziplines land by the corners away from the stairs' head house.)
    for (x, z) in [
        (x1 - 2.5, z0 + 2.5),
        (x1 - 2.5, z1 - 2.5),
        (x0 + 2.5, z1 - 2.5),
    ] {
        let at = b.p(x, r, z);
        b.out.zip_anchors.push(at);
    }
    // The spawn room: the back left corner, its doorway towards the hall closed by a
    // forcefield.
    let sx = x0 + 12.0;
    let sz = z1 - 11.0;
    let sdoor = door(4.5, 3.0, 3.2);
    b.wall(
        Vec2::new(x0, sz),
        Vec2::new(sx, sz),
        T,
        0.0,
        STOREY,
        &[sdoor],
        Style::Interior,
    );
    b.wall(
        Vec2::new(sx, sz),
        Vec2::new(sx, z1),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    let r90 = std::f32::consts::FRAC_PI_2;
    if s.spawns {
        let f0 = b.p(x0 + 4.5, 0.0, sz - 0.15);
        let f1 = b.p(x0 + 7.5, 3.2, sz + 0.15);
        b.out.shields.push(Shield {
            collider: Collider::Box {
                min: f0.min(f1),
                max: f0.max(f1),
            },
        });
        for i in 0..6 {
            let (px, pz) = (
                x0 + 2.4 + (i % 3) as f32 * 3.4,
                z1 - 2.4 - (i / 3) as f32 * 3.6,
            );
            let at = b.p(px, 0.0, pz);
            let yaw = b.yaw(0.0);
            b.out.spawns.push((at, yaw));
        }
    } else {
        // A ready room: lockers and benches.
        for k in 0..3 {
            b.decor(
                x0 + 2.5 + k as f32 * 3.2,
                0.0,
                z1 - 1.0,
                Vec3::new(2.4, 2.2, 0.9),
            );
        }
        b.decor(x0 + 6.0, 0.0, z1 - 5.0, Vec3::new(3.0, 0.5, 0.8));
    }
    b.terminal(TerminalKind::Equipment, x0 + 1.0, 0.0, sz + 2.5, -r90);
    b.terminal(TerminalKind::Medical, sx - 1.0, 0.0, z1 - 3.0, r90);
    // The core room: the back right corner, one doorway into the hall; server racks.
    let cx = x1 - 13.0;
    let cz = z1 - 12.0;
    b.wall(
        Vec2::new(cx, cz),
        Vec2::new(x1, cz),
        T,
        0.0,
        STOREY,
        &[door(4.0, 4.0, DOOR_H)],
        Style::Interior,
    );
    b.wall(
        Vec2::new(cx, cz),
        Vec2::new(cx, z1),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    if s.console {
        let at = b.p(x1 - 7.0, 0.0, z1 - 5.0);
        let yaw = b.yaw(0.0);
        b.out.console = Some((at, yaw));
        b.out.colliders.push(round_prism(at, 0.6, 1.1, 6, 0.0));
        b.out.styles.push(Style::Pillar);
    }
    for k in 0..3 {
        b.decor(
            x1 - 1.0,
            0.0,
            cz + 3.0 + k as f32 * 2.6,
            Vec3::new(1.2, 2.4, 2.0),
        );
    }
    b.decor(cx + 1.2, 0.0, z1 - 2.0, Vec3::new(1.2, 2.4, 2.0));
    // The generator room: the front right corner, a doorway from the hall and the outer door.
    let gx = x1 - 11.0;
    let gz = z0 + 10.0;
    b.wall(
        Vec2::new(gx, z0),
        Vec2::new(gx, gz),
        T,
        0.0,
        STOREY,
        &[door(4.0, 3.5, DOOR_H)],
        Style::Interior,
    );
    b.wall(
        Vec2::new(gx, gz),
        Vec2::new(x1, gz),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    if s.generator {
        generator(b, x1 - 5.5, z0 + 6.0);
    } else {
        // A store: stacked crates against the front wall.
        for (x, z, w) in [(x1 - 3.0, z0 + 1.6, 1.8), (x1 - 5.4, z0 + 1.4, 1.4)] {
            b.decor(x, 0.0, z, Vec3::splat(w));
        }
        b.decor(x1 - 3.0, 1.8, z0 + 1.6, Vec3::splat(1.2));
    }
    // The hall: terminals on the front wall beside the stairs, pillars, crates.
    let pi = std::f32::consts::PI;
    b.terminal(TerminalKind::Equipment, sx1 + 2.4, 0.0, z0 + 1.0, pi);
    b.terminal(TerminalKind::Medical, sx1 + 4.6, 0.0, z0 + 1.0, pi);
    for px in [x0 + 12.0, x1 - 14.0] {
        let p = b.p(px, 0.0, z1 - 15.0);
        b.out.colliders.push(round_prism(p, 0.5, STOREY, 8, 0.0));
        b.out.styles.push(Style::Pillar);
    }
    // Crates stacked against the walls, out of the paths and the doorways: along the front
    // wall right of the door, the back wall either side of its door, by the generator room.
    let spans: Vec<(f32, f32, f32)> = [
        (c.x + 4.5, gx - 1.8, z0 + 1.2),
        (sx + 1.6, c.x - 4.2, z1 - 1.2),
        (c.x + 4.2, cx - 1.6, z1 - 1.2),
    ]
    .into_iter()
    .filter(|s| s.1 - s.0 > 0.5)
    .collect();
    for _ in 0..4 {
        let k = rng.int(0, spans.len() as i32 + 1) as usize;
        let (x, z) = match spans.get(k) {
            Some(&(a, e, z)) => (rng.range(a, e), z),
            // (Clear of the way along to the core room's door.)
            None => (gx - 1.5, rng.range(gz + 1.5, gz + 2.5)),
        };
        let w = rng.range(1.0, 1.8);
        b.decor(x, 0.0, z, Vec3::splat(w));
        if rng.chance(0.5) {
            b.decor(x, w, z, Vec3::splat(w * 0.8));
        }
    }
    b.lamp(c.x - 3.0, STOREY - 0.6, c.y - 4.0);
    b.lamp(x0 + 6.0, STOREY - 0.6, z1 - 5.0);
    b.lamp(x1 - 7.0, STOREY - 0.6, z1 - 6.0);
    b.lamp(x1 - 5.5, STOREY - 0.6, z0 + 5.0);
    if storeys > 1 {
        // Upstairs: offices behind a partition, desks and consoles.
        let y = STOREY;
        let px = c.x + 2.0;
        b.wall(
            Vec2::new(px, z0),
            Vec2::new(px, z1),
            T,
            y,
            y + STOREY - SLAB,
            &[door(hz - 4.0, 3.0, DOOR_H), door(hz + 6.0, 3.0, DOOR_H)],
            Style::Interior,
        );
        b.wall(
            Vec2::new(px, c.y + 2.0),
            Vec2::new(x1, c.y + 2.0),
            T,
            y,
            y + STOREY - SLAB,
            &[door(5.0, 3.0, DOOR_H)],
            Style::Interior,
        );
        for k in 0..3 {
            b.decor(
                px + 4.0 + k as f32 * 4.0,
                y,
                z0 + 4.0,
                Vec3::new(2.4, 0.9, 1.0),
            );
            b.decor(
                px + 4.0 + k as f32 * 4.0,
                y,
                z1 - 4.0,
                Vec3::new(2.4, 0.9, 1.0),
            );
        }
        b.decor(x0 + 10.0, y, z1 - 1.0, Vec3::new(4.0, 2.2, 1.0));
        b.decor(x0 + 14.0, y, c.y + 3.0, Vec3::new(1.4, 1.4, 1.4));
        b.terminal(TerminalKind::Equipment, x0 + 1.0, y, z1 - 4.0, -r90);
        b.lamp(c.x - 8.0, top - 0.6, c.y);
        b.lamp(c.x + 10.0, top - 0.6, z0 + 6.0);
        b.lamp(c.x + 10.0, top - 0.6, z1 - 6.0);
    }
    let a = b.p(x0, 0.0, z0);
    let e = b.p(x1, top, z1);
    b.out.building = (a.min(e), a.max(e));
    b.out.interiors.push((a.min(e), a.max(e)));
    b.out.storeys = storeys;
    // The walk: the front door, the hall (between its pillar and the core room's wall), the
    // core room, the console; the spawn room; the back and side doors; the stairs.
    let way = b.chain(
        None,
        0.0,
        &[
            (c.x, z0 - 3.0),
            (c.x, z0 + 2.5),
            (x1 - 16.0, z1 - 13.5),
            (x1 - 7.0, z1 - 13.5),
            (x1 - 7.0, z1 - 10.0),
            (x1 - 7.0, z1 - 6.6),
        ],
    );
    b.mark("door_out", way[0]);
    b.mark("door_in", way[1]);
    b.mark("hall", way[2]);
    b.mark("console", way[5]);
    let spawn = b.chain(
        Some(way[1]),
        0.0,
        &[(x0 + 6.0, sz - 2.0), (x0 + 6.0, sz + 2.0)],
    );
    b.mark("spawn", spawn[1]);
    let back = b.chain(Some(way[2]), 0.0, &[(c.x, z1 - 2.5), (c.x, z1 + 3.0)]);
    b.mark("back_in", back[0]);
    b.mark("back_out", back[1]);
    let side = b.chain(
        Some(way[1]),
        0.0,
        &[(gx - 2.2, z0 + 5.75), (x1 - 8.5, z0 + 6.5)],
    );
    let side = if s.generator {
        // (Round the generator to the side door.)
        b.chain(
            Some(side[1]),
            0.0,
            &[
                (x1 - 8.5, z0 + 9.0),
                (x1 - 2.0, z0 + 9.0),
                (x1 - 2.0, z0 + 6.5),
            ],
        )[2]
    } else {
        b.chain(Some(side[1]), 0.0, &[(x1 - 2.0, z0 + 6.5)])[0]
    };
    let out = b.chain(Some(side), 0.0, &[(x1 + 3.0, z0 + 6.5)])[0];
    b.mark("side_out", out);
    // (From the stairs' doorways: into the hall downstairs, the offices above, onto the roof.)
    b.link(outs[0], spawn[0]);
    b.mark("roof", *outs.last().unwrap());
    Rooms {
        core: (x1 - 7.0, z1 - 7.0),
    }
}

/// The generator standing on the floor at (x, z).
pub(super) fn generator(b: &mut Builder, x: f32, z: f32) {
    let at = b.p(x, 0.0, z);
    b.out.generator = Some(at);
    let gh = GENERATOR_HALF;
    let c = b.world_box(
        Vec3::new(x - gh.x, 0.0, z - gh.z),
        Vec3::new(x + gh.x, gh.y * 2.0, z + gh.z),
    );
    b.out.colliders.push(c);
    b.out.styles.push(Style::Decor);
}
