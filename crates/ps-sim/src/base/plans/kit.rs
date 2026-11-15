//! What stands in the yards: small buildings (a generator annex, barracks, a comms hut), open
//! sheds, canopies on posts, low cover.

use glam::{Vec2, Vec3};

use crate::base::builder::{Builder, door};
use crate::base::building::generator;
use crate::base::facade::{Dress, Face, facade};
use crate::base::*;

/// A wall of a rectangle: 0 the front (-Z), 1 the back, 2 the left (-X), 3 the right.
pub(in crate::base) type Side = u8;

/// A rectangle's wall: its ends (the way `Builder::wall` runs it) and which way is out.
fn side_of(r: (f32, f32, f32, f32), side: Side) -> (Vec2, Vec2, Vec2) {
    let (x0, z0, x1, z1) = r;
    match side {
        0 => (Vec2::new(x0, z0), Vec2::new(x1, z0), Vec2::NEG_Y),
        1 => (Vec2::new(x0, z1), Vec2::new(x1, z1), Vec2::Y),
        2 => (Vec2::new(x0, z0), Vec2::new(x0, z1), Vec2::NEG_X),
        _ => (Vec2::new(x1, z0), Vec2::new(x1, z1), Vec2::X),
    }
}

/// The width and height of the small buildings' doors.
const HUT_DOOR: (f32, f32) = (3.0, 3.2);

/// A small building over the rectangle (x0, z0, x1, z1), its walls `h` high: a doorway in
/// wall `side` (`off` along it from its start), a flat roof behind a parapet, dressed in
/// `style`. Its walk nodes outside and inside the door.
pub(in crate::base) fn hut(
    b: &mut Builder,
    r: (f32, f32, f32, f32),
    h: f32,
    side: Side,
    off: f32,
    style: u8,
) -> (usize, usize) {
    let mut faces = Vec::new();
    for k in 0..4 {
        let (a, e, out) = side_of(r, k);
        let gaps = if k == side {
            vec![door(off, HUT_DOOR.0, HUT_DOOR.1)]
        } else {
            Vec::new()
        };
        b.wall(a, e, T, 0.0, h, &gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, &gaps));
    }
    facade(
        b,
        &faces,
        &Dress {
            top: h,
            storeys: 1,
            glazed: false,
            bays: true,
            style,
        },
    );
    let roof = h + 0.6;
    b.slab(
        (r.0 - T / 2.0, r.1 - T / 2.0, r.2 + T / 2.0, r.3 + T / 2.0),
        roof,
        &[],
        Style::Canopy,
    );
    for k in 0..4 {
        let (a, e, _) = side_of(r, k);
        b.wall(a, e, 0.4, roof, roof + 1.0, &[], Style::Parapet);
    }
    b.lamp((r.0 + r.2) / 2.0, h - 0.6, (r.1 + r.3) / 2.0);
    let a = b.p(r.0, 0.0, r.1);
    let e = b.p(r.2, h, r.3);
    b.out.interiors.push((a.min(e), a.max(e)));
    // The walk through the door.
    let (a, e, out) = side_of(r, side);
    let u = (e - a).normalize();
    let mid = a + u * (off + HUT_DOOR.0 / 2.0);
    let (po, pi) = (mid + out * 2.5, mid - out * 2.0);
    let n = b.chain(None, 0.0, &[(po.x, po.y), (pi.x, pi.y)]);
    (n[0], n[1])
}

/// The generator's annex: a small building with the generator in it and vents on its roof.
/// Its walk node outside the door.
pub(in crate::base) fn annex(
    b: &mut Builder,
    r: (f32, f32, f32, f32),
    side: Side,
    style: u8,
) -> usize {
    let (a, e, _) = side_of(r, side);
    let off = (e - a).length() / 2.0 - HUT_DOOR.0 / 2.0;
    let (out, _) = hut(b, r, 4.6, side, off, style);
    // (The generator towards the wall across from the door.)
    let (_, _, o) = side_of(r, side);
    let c = Vec2::new((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0) - o * 2.0;
    generator(b, c.x, c.y);
    let roof = 4.6 + 0.6;
    b.decor(c.x + 2.0, roof, c.y, Vec3::new(1.6, 1.6, 1.6));
    b.decor(c.x - 2.0, roof, c.y, Vec3::new(1.6, 1.0, 2.4));
    out
}

/// Barracks: a small building with `n` spawn tubes behind a forcefield across its door, and
/// its terminals. Its walk nodes outside and inside the door.
pub(in crate::base) fn barracks(
    b: &mut Builder,
    r: (f32, f32, f32, f32),
    side: Side,
    off: f32,
    n: usize,
    style: u8,
) -> (usize, usize) {
    let nodes = hut(b, r, 4.6, side, off, style);
    let (a, e, out) = side_of(r, side);
    let u = (e - a).normalize();
    // The forcefield across the doorway.
    let (s0, s1) = (
        a + u * off - out * 0.15,
        a + u * (off + HUT_DOOR.0) + out * 0.15,
    );
    let f0 = b.p(s0.x, 0.0, s0.y);
    let f1 = b.p(s1.x, HUT_DOOR.1, s1.y);
    b.out.shields.push(Shield {
        collider: Collider::Box {
            min: f0.min(f1),
            max: f0.max(f1),
        },
    });
    // The tubes along the wall across from the door, the terminals by the end walls.
    let (c, half) = (
        Vec2::new((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0),
        Vec2::new((r.2 - r.0) / 2.0, (r.3 - r.1) / 2.0),
    );
    let deep = if side < 2 { half.y } else { half.x };
    let wide = if side < 2 { half.x } else { half.y };
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32 * 2.0 - 1.0;
        let p = c - out * (deep - 2.4) + u * t * (wide - 2.4);
        let at = b.p(p.x, 0.0, p.y);
        // (Facing the door's wall.)
        let yaw = b.yaw(out.x.atan2(out.y) + std::f32::consts::PI);
        b.out.spawns.push((at, yaw));
    }
    for (s, kind) in [
        (-1.0, TerminalKind::Equipment),
        (1.0, TerminalKind::Medical),
    ] {
        // (Against the back wall, its screen towards the door's.)
        let p = c + u * s * (wide - 1.0) - out * (deep - 1.0);
        let yaw = (-out.x).atan2(-out.y);
        b.terminal(kind, p.x, 0.0, p.y, yaw);
    }
    nodes
}

/// An open shed: a roof on three walls, open on `open`'s side; crates along its back.
pub(in crate::base) fn shed(b: &mut Builder, r: (f32, f32, f32, f32), open: Side) {
    let h = 5.0;
    for k in 0..4 {
        if k != open {
            let (a, e, _) = side_of(r, k);
            b.wall(a, e, T, 0.0, h, &[], Style::Building);
        }
    }
    b.slab(
        (r.0 - 0.6, r.1 - 0.6, r.2 + 0.6, r.3 + 0.6),
        h + 0.4,
        &[],
        Style::Canopy,
    );
    // A lit lintel over the open side, piers at its ends.
    let (a, e, out) = side_of(r, open);
    let u = (e - a).normalize();
    let len = (e - a).length();
    let f = Face {
        a: a + out * 0.3,
        u,
        out,
        len,
        doors: Vec::new(),
        windows: Vec::new(),
        clear: Vec::new(),
    };
    b.run(
        &f,
        0.0,
        len,
        [(-0.6, h - 0.9), (0.2, h - 0.9), (0.2, h), (-0.6, h)],
        Finish::Body,
    );
    b.run(
        &f,
        0.6,
        len - 0.6,
        [
            (0.2, h - 0.6),
            (0.25, h - 0.6),
            (0.25, h - 0.4),
            (0.2, h - 0.4),
        ],
        Finish::Trim,
    );
    // Crates and racks along the back wall.
    let c = Vec2::new((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let deep = if open < 2 {
        (r.3 - r.1) / 2.0
    } else {
        (r.2 - r.0) / 2.0
    };
    for (t, w, hh) in [(-0.7f32, 1.8f32, 1.8f32), (-0.3, 1.4, 1.4), (0.5, 2.2, 1.2)] {
        let p = c - out * (deep - 1.5) + u * t * (len / 2.0 - 2.0);
        b.decor(p.x, 0.0, p.y, Vec3::new(w, hh, w));
    }
    b.lamp(c.x, h - 0.6, c.y);
}

/// A canopy on posts over the rectangle, `h` to its underside.
pub(in crate::base) fn canopy(b: &mut Builder, r: (f32, f32, f32, f32), h: f32) {
    b.slab(
        (r.0 - 0.4, r.1 - 0.4, r.2 + 0.4, r.3 + 0.4),
        h + 0.4,
        &[],
        Style::Canopy,
    );
    let along_x = r.2 - r.0 >= r.3 - r.1;
    let len = if along_x { r.2 - r.0 } else { r.3 - r.1 };
    let n = (len / 6.5).ceil().max(1.0) as usize;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        for far in [false, true] {
            let (x, z) = if along_x {
                (r.0 + len * t, if far { r.3 } else { r.1 })
            } else {
                (if far { r.2 } else { r.0 }, r.1 + len * t)
            };
            b.boxed(
                Vec3::new(x - 0.2, 0.0, z - 0.2),
                Vec3::new(x + 0.2, h, z + 0.2),
                Style::Gatehouse,
            );
        }
    }
    b.lamp((r.0 + r.2) / 2.0, h - 0.4, (r.1 + r.3) / 2.0);
}

/// The comms hut: a small building with a mast on its roof. Its walk node outside the door.
pub(in crate::base) fn comms(b: &mut Builder, r: (f32, f32, f32, f32), side: Side) -> usize {
    let (a, e, _) = side_of(r, side);
    let off = (e - a).length() / 2.0 - HUT_DOOR.0 / 2.0;
    let (out, _) = hut(b, r, 3.8, side, off, 1);
    let c = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let roof = 3.8 + 0.6;
    let foot = b.p(c.0, roof, c.1);
    b.out
        .colliders
        .push(round_prism(foot, 0.35, MAX_HEIGHT - 2.0 - roof, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    // Dishes up the mast, a desk inside.
    for (k, y) in [roof + 4.0, roof + 7.5, roof + 10.5]
        .into_iter()
        .enumerate()
    {
        let s = if k % 2 == 0 { 1.0 } else { -1.0 };
        b.block(
            Vec3::new(c.0 - 0.9, y, c.1 + s * 0.35),
            Vec3::new(c.0 + 0.9, y + 1.1, c.1 + s * 0.55),
            Finish::Body,
        );
        b.block(
            Vec3::new(c.0 - 0.7, y + 0.5, c.1 + s * 0.55),
            Vec3::new(c.0 + 0.7, y + 0.6, c.1 + s * 0.58),
            Finish::Trim,
        );
    }
    out
}

/// Low cover: a block `len` long at (x, z), along x or z.
pub(in crate::base) fn cover(b: &mut Builder, x: f32, z: f32, len: f32, along_x: bool) {
    let (hx, hz) = if along_x {
        (len / 2.0, 0.3)
    } else {
        (0.3, len / 2.0)
    };
    b.boxed(
        Vec3::new(x - hx, 0.0, z - hz),
        Vec3::new(x + hx, 1.2, z + hz),
        Style::Cover,
    );
}
