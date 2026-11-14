//! The pieces bases are built of: the compound's walls and the main building.

use glam::{Vec2, Vec3};

use super::builder::{Builder, Gap, door};
use super::facade::{Dress, Face, facade, ramparts};
use super::*;
use crate::noise::Seq;

/// The walls with their walkways, gateways front and back with their forcefields, ramps up
/// along every wall, turrets and floodlights.
pub(super) fn compound(b: &mut Builder, half: f32, kind: BaseKind) {
    let inner = half - WALL_T;
    let mid = half - WALL_T / 2.0;
    let gate = [door(half - GATEWAY / 2.0, GATEWAY, 100.0)];
    for (z, gaps) in [(-mid, &gate[..]), (mid, &gate[..])] {
        b.wall(
            Vec2::new(-half, z),
            Vec2::new(half, z),
            WALL_T,
            -2.0,
            WALL_H,
            gaps,
            Style::Wall,
        );
        let pz = if z < 0.0 { -half + 0.3 } else { half - 0.3 };
        b.wall(
            Vec2::new(-half, pz),
            Vec2::new(half, pz),
            0.6,
            WALL_H,
            WALL_H + 1.3,
            gaps,
            Style::Parapet,
        );
        // The gateway's forcefield.
        let f = b.world_box(
            Vec3::new(-GATEWAY / 2.0, 0.0, z - 0.4),
            Vec3::new(GATEWAY / 2.0, WALL_H, z + 0.4),
        );
        b.out.gate_fields.push(f);
    }
    for x in [-mid, mid] {
        b.wall(
            Vec2::new(x, -half + WALL_T),
            Vec2::new(x, half - WALL_T),
            WALL_T,
            -2.0,
            WALL_H,
            &[],
            Style::Wall,
        );
        let px = if x < 0.0 { -half + 0.3 } else { half - 0.3 };
        b.wall(
            Vec2::new(px, -half + WALL_T),
            Vec2::new(px, half - WALL_T),
            0.6,
            WALL_H,
            WALL_H + 1.3,
            &[],
            Style::Parapet,
        );
    }
    ramparts(b, half);
    // Ramps (stairs) up the inside of every wall, two on each, climbing from near the corners.
    let (a0, a1) = (-inner + 6.0, -inner + 6.0 + RAMP_LEN);
    for x in [-inner, inner - RAMP_W] {
        b.ramp(Vec3::new(x, 0.0, a0), Vec3::new(x + RAMP_W, WALL_H, a1), 0);
        b.ramp(
            Vec3::new(x, 0.0, -a1),
            Vec3::new(x + RAMP_W, WALL_H, -a0),
            2,
        );
    }
    for z in [-inner, inner - RAMP_W] {
        b.ramp(Vec3::new(a0, 0.0, z), Vec3::new(a1, WALL_H, z + RAMP_W), 1);
        b.ramp(
            Vec3::new(-a1, 0.0, z),
            Vec3::new(-a0, WALL_H, z + RAMP_W),
            3,
        );
    }
    // Turrets: the corners and beside the gateways (and the side walls' middles on the big
    // bases).
    let g = GATEWAY / 2.0 + 4.0;
    let mut mounts = vec![
        (-mid, -mid),
        (mid, -mid),
        (-mid, mid),
        (mid, mid),
        (-g, -mid),
        (g, -mid),
    ];
    if kind != BaseKind::Outpost {
        mounts.extend([(-mid, 0.0), (mid, 0.0), (-g, mid), (g, mid)]);
    }
    for (x, z) in mounts {
        let p = b.p(x, WALL_H + TURRET_RISE, z);
        b.out.turrets.push(p);
    }
    // Floodlights on the corners, lighting the yard.
    for (x, z) in [(-mid, -mid), (mid, -mid), (-mid, mid), (mid, mid)] {
        b.flood((x * 0.97, WALL_H + 3.5, z * 0.97), (x * 0.3, 0.0, z * 0.3));
    }
}

/// The main building around `BLD_C` (its front towards -Z), `storeys` high.
pub(super) fn building(b: &mut Builder, storeys: u8, console: bool, rng: &mut Seq) {
    let c = BLD_C;
    let (hx, hz) = (BLD_HALF.x, BLD_HALF.y);
    let (x0, x1, z0, z1) = (c.x - hx, c.x + hx, c.y - hz, c.y + hz);
    let top = STOREY * storeys as f32;
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
    let upper = |len: f32| {
        if storeys > 1 {
            windows(len)
        } else {
            Vec::new()
        }
    };
    // Outer walls: doors front and back, and on the right side into the generator room.
    let front = with(vec![middle(2.0 * hx)], upper(2.0 * hx));
    let back = with(vec![middle(2.0 * hx)], upper(2.0 * hx));
    let left = upper(2.0 * hz);
    let right = with(vec![door(4.0, DOOR_W, DOOR_H)], upper(2.0 * hz));
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
    // (The roof's ramp comes up the back, left of the door.)
    faces[1]
        .clear
        .push((4.0 + T / 2.0 - 0.2, 8.0 + T / 2.0 + 0.2));
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys,
            glazed: true,
            bays: true,
        },
    );
    // The upper floor (with the stairwell over the stair ramp), and the roof.
    let stair = (x0, z0, x0 + RAMP_W + 0.5, z0 + 12.0);
    if storeys > 1 {
        b.slab((x0, z0, x1, z1), STOREY, &[stair], Style::Floor);
        b.ramp(
            Vec3::new(x0 + T / 2.0, 0.0, z0 + 1.0),
            Vec3::new(x0 + T / 2.0 + RAMP_W, STOREY, z0 + 11.0),
            0,
        );
    }
    b.slab(
        (x0 - T / 2.0, z0 - T / 2.0, x1 + T / 2.0, z1 + T / 2.0),
        top + 0.6,
        &[],
        Style::Roof,
    );
    // A parapet round the roof.
    let r = top + 0.6;
    b.wall(
        Vec2::new(x0, z0),
        Vec2::new(x1, z0),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x0, z1),
        Vec2::new(x1, z1),
        0.4,
        r,
        r + 1.0,
        &[door(4.0, 4.0, 2.0)],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x0, z0),
        Vec2::new(x0, z1),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x1, z0),
        Vec2::new(x1, z1),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    // The roof's ramp up the back (to the parapet's gap).
    let len = r * 2.0;
    b.ramp(
        Vec3::new(x0 + 4.0, 0.0, z1 + T / 2.0),
        Vec3::new(x0 + 8.0, r, z1 + T / 2.0 + len),
        2,
    );
    // On the roof: vent stacks and plant on the right (clear of the ramp's landing), an
    // antenna mast.
    b.decor(x1 - 5.0, r, z0 + 5.0, Vec3::new(4.0, 1.4, 3.0));
    b.decor(x1 - 5.0, r + 1.4, z0 + 5.0, Vec3::new(2.4, 0.6, 2.0));
    b.decor(x1 - 4.0, r, z1 - 6.0, Vec3::new(2.4, 1.0, 4.0));
    b.decor(c.x + 4.0, r, z1 - 3.0, Vec3::new(1.6, 2.2, 1.6));
    let mast = b.p(x1 - 2.5, r, c.y);
    b.out.colliders.push(round_prism(mast, 0.3, 7.0, 6, 0.0));
    b.out.styles.push(Style::Pillar);
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
    let r90 = std::f32::consts::FRAC_PI_2;
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
    if console {
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
    let gen_at = b.p(x1 - 5.5, 0.0, z0 + 6.0);
    b.out.generator = Some(gen_at);
    let gh = GENERATOR_HALF;
    let gen_c = b.world_box(
        Vec3::new(x1 - 5.5 - gh.x, 0.0, z0 + 6.0 - gh.z),
        Vec3::new(x1 - 5.5 + gh.x, gh.y * 2.0, z0 + 6.0 + gh.z),
    );
    b.out.colliders.push(gen_c);
    b.out.styles.push(Style::Decor);
    // The hall: terminals, pillars, crates.
    b.terminal(TerminalKind::Equipment, x0 + 1.0, 0.0, z0 + 14.0, -r90);
    b.terminal(TerminalKind::Medical, x0 + 1.0, 0.0, z0 + 17.5, -r90);
    for (px, pz) in [(c.x - 6.0, c.y - 1.0), (c.x + 4.0, c.y - 1.0)] {
        let p = b.p(px, 0.0, pz);
        b.out.colliders.push(round_prism(p, 0.5, STOREY, 8, 0.0));
        b.out.styles.push(Style::Pillar);
    }
    for _ in 0..4 {
        // Crates stacked against the walls (out of the paths).
        let (x, z) = match rng.int(0, 3) {
            0 => (rng.range(x0 + 6.0, cx - 2.0), z0 + 1.2),
            1 => (rng.range(sx + 2.0, cx - 2.0), z1 - 1.2),
            // (Clear of the way along to the core room's door.)
            _ => (gx - 1.5, rng.range(gz + 1.5, gz + 2.5)),
        };
        let s = rng.range(1.0, 1.8);
        b.decor(x, 0.0, z, Vec3::splat(s));
        if rng.chance(0.5) {
            b.decor(x, s, z, Vec3::splat(s * 0.8));
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
            &[door(10.0, 3.0, DOOR_H), door(20.0, 3.0, DOOR_H)],
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
        b.decor(x0 + 14.0, y, c.y, Vec3::new(1.4, 1.4, 1.4));
        b.terminal(TerminalKind::Equipment, x0 + 1.0, y, z1 - 4.0, -r90);
        b.lamp(c.x - 8.0, top - 0.6, c.y);
        b.lamp(c.x + 10.0, top - 0.6, z0 + 6.0);
        b.lamp(c.x + 10.0, top - 0.6, z1 - 6.0);
    }
    let a = b.p(x0, 0.0, z0);
    let e = b.p(x1, top, z1);
    b.out.building = (a.min(e), a.max(e));
}
