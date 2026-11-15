use super::*;
use crate::collide::{BODY_HEIGHT, STEP, World, push_out};

fn site(kind: BaseKind, facing: f32, capture: Capture) -> BaseSite {
    BaseSite {
        name: "t".into(),
        kind,
        pos: Vec2::new(100.0, -50.0),
        facing,
        capture,
    }
}

/// The layout's colliders with the land under it (flat at its pad's height, a tower's bunker's
/// hole in it).
fn world_of(l: &BaseLayout) -> World {
    let mut cs = l.colliders.clone();
    let (c, y) = (l.center, l.center.y);
    let far = 500.0;
    if let Some(&(lo, hi)) = l.holes.first() {
        for (a, e) in [
            (Vec2::new(c.x - far, c.z - far), Vec2::new(c.x + far, lo.y)),
            (Vec2::new(c.x - far, hi.y), Vec2::new(c.x + far, c.z + far)),
            (Vec2::new(c.x - far, lo.y), Vec2::new(lo.x, hi.y)),
            (Vec2::new(hi.x, lo.y), Vec2::new(c.x + far, hi.y)),
        ] {
            cs.push(Collider::Box {
                min: Vec3::new(a.x, y - 1.0, a.y),
                max: Vec3::new(e.x, y, e.y),
            });
        }
    } else {
        cs.push(Collider::Box {
            min: Vec3::new(c.x - far, y - 1.0, c.z - far),
            max: Vec3::new(c.x + far, y, c.z + far),
        });
    }
    World::new(cs, None)
}

/// One layout of every plan of every kind, mirrored and not (and the outposts with one storey
/// and two, with a console and with capture points), each turned a different way.
fn every_plan() -> Vec<BaseLayout> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for kind in [
        BaseKind::Outpost,
        BaseKind::Center,
        BaseKind::Home,
        BaseKind::Tower,
    ] {
        for capture in [Capture::Console, Capture::Points] {
            if capture == Capture::Points && kind != BaseKind::Outpost {
                continue;
            }
            for seed in 0..120u32 {
                let q = (seed + out.len() as u32) % 4;
                let s = site(kind, q as f32 * std::f32::consts::FRAC_PI_2, capture);
                let l = layout(&s, 10.0, seed);
                let key = (kind, capture, l.variant, l.flip, l.storeys);
                if !seen.contains(&key) {
                    seen.push(key);
                    out.push(l);
                }
            }
        }
    }
    out
}

fn flat(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// Walks from `feet` straight towards `to`; where it gets.
fn walk(w: &World, mut feet: Vec3, to: Vec3) -> Vec3 {
    let mut vy = 0.0;
    for _ in 0..2400 {
        let d = Vec2::new(to.x - feet.x, to.z - feet.z);
        if d.length() < 0.4 {
            break;
        }
        let step = d.normalize() * (5.0f32 / 60.0).min(d.length());
        (feet, vy) = w.move_body(feet, step, vy, 1.0 / 60.0);
    }
    feet
}

fn free(w: &World, at: Vec3) -> bool {
    let mut p = at;
    !w.colliders.iter().any(|c| push_out(c, &mut p))
}

#[test]
fn spawns_are_free_and_walk_out_of_the_building() {
    for l in every_plan() {
        let kind = l.kind;
        assert_eq!(l.colliders.len(), l.styles.len());
        let w = world_of(&l);
        assert!(!l.spawns.is_empty(), "{kind:?}");
        for &(s, _) in &l.spawns {
            assert!(free(&w, s), "{kind:?} {}: spawn {s} blocked", l.variant);
            assert!(l.in_building(s), "{kind:?} spawn {s} outside");
            // Something to stand on.
            let g = w.ground(s.x, s.z, s.y + 0.2).unwrap();
            assert!((g - s.y).abs() < 0.05, "{kind:?} spawn {s} floor {g}");
        }
        assert_eq!(
            l.console.is_some(),
            kind != BaseKind::Home && l.points.is_empty()
        );
        assert!(
            l.terminals
                .iter()
                .any(|t| t.kind == TerminalKind::Equipment)
        );
        assert!(l.in_building((l.building.0 + l.building.1) / 2.0));
        if kind != BaseKind::Tower {
            assert!(l.terminals.iter().any(|t| t.kind == TerminalKind::Vehicle));
            assert!(l.generator.is_some() && l.silo.is_some());
            assert_eq!(l.gate_fields.len(), 2);
            assert!(!l.zip_anchors.is_empty());
            for &a in &l.zip_anchors {
                assert!(free(&w, a), "{kind:?} {}: zip anchor {a}", l.variant);
                let g = w.ground(a.x, a.z, a.y + 0.2).unwrap();
                assert!((g - a.y).abs() < 0.05, "{kind:?}: zip anchor {a} floor {g}");
            }
            // The vehicle pads and their terminals are clear.
            for &(p, _) in &l.pads {
                for k in 0..8 {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let q = p + Vec3::new(a.cos(), 0.0, a.sin()) * (PAD_RADIUS - 0.5);
                    assert!(
                        free(&w, q),
                        "{kind:?} {}: pad {p} blocked at {q}",
                        l.variant
                    );
                    assert!(
                        w.ceiling(q.x, q.z, q.y + 0.5).is_none(),
                        "{kind:?}: pad roofed"
                    );
                }
            }
        } else {
            assert_eq!(l.holes.len(), 1);
        }
        // Terminals can be walked up to.
        for t in &l.terminals {
            let front = t.pos + glam::Quat::from_rotation_y(t.yaw) * Vec3::NEG_Z * 1.2;
            assert!(
                free(&w, front),
                "{kind:?} {}: terminal at {} faces a wall",
                l.variant,
                t.pos
            );
        }
        // The rally point is clear, in the open.
        assert!(free(&w, l.rally), "{kind:?} {}: rally", l.variant);
        assert!(!l.in_building(l.rally));
    }
}

#[test]
fn capture_points_are_clear_and_bots_reach_them() {
    for l in every_plan() {
        if l.points.is_empty() {
            continue;
        }
        assert!(l.console.is_none());
        assert_eq!(l.points.len(), 3);
        let w = world_of(&l);
        let v = l.variant;
        let mut inside = 0;
        for (i, pt) in l.points.iter().enumerate() {
            // The stand spot is clear, on the floor, in the zone.
            assert!(
                free(&w, pt.stand),
                "{v}: point {i}: stand {} blocked",
                pt.stand
            );
            assert!(flat(pt.stand, pt.pos) < pt.radius * 0.6, "{v}: point {i}");
            assert!(flat(pt.pylon, pt.pos) < pt.radius);
            // Walking the path gets there.
            let mut feet = pt.path[0];
            for &target in &pt.path[1..] {
                feet = walk(&w, feet, target);
                assert!(
                    flat(target, feet) < 0.6,
                    "{v}: point {i}: stuck at {feet} on the way to {target}"
                );
            }
            inside += l.in_building(pt.pos) as usize;
            // The zones are apart.
            for other in &l.points[..i] {
                assert!(flat(other.pos, pt.pos) > other.radius + pt.radius, "{v}");
            }
        }
        assert_eq!(inside, 1, "{v}: one point indoors");
    }
}

#[test]
fn gateways_are_where_the_forcefields_are() {
    for q in 0..4 {
        for kind in [BaseKind::Outpost, BaseKind::Center, BaseKind::Home] {
            let s = site(
                kind,
                q as f32 * std::f32::consts::FRAC_PI_2,
                Capture::Console,
            );
            for seed in 0..6 {
                let l = layout(&s, 0.0, seed);
                let gw = gateways(&s);
                assert_eq!(gw.len(), 2);
                for f in &l.gate_fields {
                    let Collider::Box { min, max } = *f else {
                        panic!()
                    };
                    let mid = Vec2::new(min.x + max.x, min.z + max.z) / 2.0;
                    // Each forcefield just inside a gateway, the way out away from the middle.
                    assert!(
                        gw.iter()
                            .any(|(at, out)| at.distance(mid) < WALL_T
                                && (*at - s.pos).dot(*out) > 0.0),
                        "{q}: {mid} {gw:?}"
                    );
                    // The gateway is open under its lintel, and the walkway runs on over it.
                    let w = world_of(&l);
                    assert!(!w.occupied(mid.x, mid.y, 0.5, GATE_CLEAR - 0.5));
                    let g = w.ground(mid.x, mid.y, WALL_H + 0.2).unwrap();
                    assert!((g - WALL_H).abs() < 0.01, "walkway over the gateway at {g}");
                }
            }
        }
    }
}

#[test]
fn bots_paths_reach_the_console() {
    // Walking each path point to point gets to the console (up the stairs in a tower), and
    // out of the spawn room.
    for l in every_plan() {
        let (kind, v) = (l.kind, l.variant);
        let w = world_of(&l);
        for path in [&l.to_console, &l.from_spawn] {
            let mut feet = path[0];
            for &target in &path[1..] {
                feet = walk(&w, feet, target);
                assert!(
                    flat(target, feet) < 0.6,
                    "{kind:?} {v}: stuck at {feet} on the way to {target}"
                );
            }
            if let Some((console, _)) = l.console
                && std::ptr::eq(path, &l.to_console)
            {
                assert!(
                    (console.y - feet.y).abs() < 0.3,
                    "{kind:?} {v}: at {feet}, console {console}"
                );
                assert!(
                    (console - feet).length() < USE_REACH,
                    "{kind:?} {v}: {feet} vs {console}"
                );
            }
        }
        assert!(l.in_building(l.from_spawn[0]), "{kind:?} {v}");
        assert!(!l.inside(*l.from_spawn.last().unwrap()) || kind == BaseKind::Tower);
    }
}

#[test]
fn every_walk_of_the_graph_can_be_walked() {
    for l in every_plan() {
        let (kind, v) = (l.kind, l.variant);
        let w = world_of(&l);
        assert!(!l.graph.links.is_empty());
        for &(a, b) in &l.graph.links {
            let (a, b) = (l.graph.nodes[a as usize], l.graph.nodes[b as usize]);
            for (from, to) in [(a, b), (b, a)] {
                assert!(free(&w, from), "{kind:?} {v}: node {from} blocked");
                let g = w.ground(from.x, from.z, from.y + 0.3).unwrap();
                assert!(
                    (g - from.y).abs() < 0.1,
                    "{kind:?} {v}: node {from} floor {g}"
                );
                let got = walk(&w, from, to);
                assert!(
                    flat(got, to) < 0.6 && (got.y - to.y).abs() < 0.3,
                    "{kind:?} {v}: from {from} to {to}: got to {got}"
                );
            }
        }
    }
}

#[test]
fn stairs_come_out_onto_a_floor() {
    // Beyond the top of every flight there's floor level with it to step out onto: not a
    // drop, not a wall.
    for l in every_plan() {
        let (kind, v) = (l.kind, l.variant);
        let w = world_of(&l);
        let mut flights = 0;
        for c in &l.colliders {
            let Collider::Ramp { min, max, dir } = *c else {
                continue;
            };
            flights += 1;
            let mid = (min + max) / 2.0;
            let (edge, out) = match dir {
                0 => (Vec3::new(mid.x, max.y, max.z), Vec3::Z),
                1 => (Vec3::new(max.x, max.y, mid.z), Vec3::X),
                2 => (Vec3::new(mid.x, max.y, min.z), Vec3::NEG_Z),
                _ => (Vec3::new(min.x, max.y, mid.z), Vec3::NEG_X),
            };
            for d in [0.5, 1.0, 1.5] {
                let p = edge + out * d;
                let g = w.ground(p.x, p.z, p.y + STEP);
                assert!(
                    g.is_some_and(|g| (g - p.y).abs() < 0.05),
                    "{kind:?} {v}: stairs to {edge} come out over {g:?} at {p}"
                );
                assert!(
                    free(&w, p),
                    "{kind:?} {v}: stairs to {edge} come out at a wall"
                );
            }
        }
        assert!(flights >= 3, "{kind:?} {v}");
    }
}

#[test]
fn no_edge_can_be_walked_off() {
    // From anywhere one can stand above the ground (on the walls, bastions, floors, roofs,
    // stairs and landings), walking a metre any way doesn't end in a drop: there's a wall, a
    // parapet or a railing. (Sampled along every such surface's edges.)
    for l in every_plan() {
        let (kind, v) = (l.kind, l.variant);
        let w = world_of(&l);
        let pad = l.center.y;
        let mut tried = 0;
        for (c, style) in l.colliders.iter().zip(&l.styles) {
            if !matches!(
                style,
                Style::Wall | Style::Bastion | Style::Floor | Style::Roof | Style::Ramp
            ) {
                continue;
            }
            let (min, max) = c.bounds();
            if max.y < pad + 2.5 {
                continue;
            }
            let (nx, nz) = (
                ((max.x - min.x) / 0.5).ceil() as i32,
                ((max.z - min.z) / 0.5).ceil() as i32,
            );
            for i in 0..=nx {
                for j in 0..=nz {
                    // (Only within a metre of its edges.)
                    let (x, z) = (
                        (min.x + i as f32 * 0.5).min(max.x) + 0.013,
                        (min.z + j as f32 * 0.5).min(max.z) + 0.017,
                    );
                    let near_edge =
                        x - min.x < 1.2 || max.x - x < 1.2 || z - min.z < 1.2 || max.z - z < 1.2;
                    let Some(y) = c.top_at(x, z) else { continue };
                    if !near_edge || y < pad + 2.5 {
                        continue;
                    }
                    let at = Vec3::new(x, y, z);
                    // Somewhere one can stand: nothing on it, headroom over it.
                    if w.ground(x, z, y + 0.05) != Some(y)
                        || !free(&w, at)
                        || w.ceiling(x, z, y + 0.1)
                            .is_some_and(|c| c < y + BODY_HEIGHT)
                    {
                        continue;
                    }
                    tried += 1;
                    for d in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
                        let (mut feet, mut vy) = (at, 0.0);
                        for k in 0..36 {
                            // (A metre's walk, then standing a while: time to fall.)
                            let step = if k < 12 { d * (5.0 / 60.0) } else { Vec2::ZERO };
                            (feet, vy) = w.move_body(feet, step, vy, 1.0 / 60.0);
                            let under = w.ground(feet.x, feet.z, feet.y + STEP).unwrap_or(-99.0);
                            assert!(
                                under > at.y - 1.5 && feet.y > at.y - 1.5,
                                "{kind:?} {v} flip {}: walked off {style:?} at {at} towards \
                                 {d}: over {under} at {feet}",
                                l.flip
                            );
                        }
                    }
                }
            }
        }
        assert!(tried > 200, "{kind:?} {v}: {tried}");
    }
}

#[test]
fn railings_are_jumped_over_but_not_walked_through() {
    let w = World::new(
        vec![
            Collider::Box {
                min: Vec3::new(-10.0, -1.0, -10.0),
                max: Vec3::new(10.0, 0.0, 10.0),
            },
            Collider::Box {
                min: Vec3::new(-10.0, 0.0, 2.0),
                max: Vec3::new(10.0, RAIL_H, 2.0 + RAIL_T),
            },
        ],
        None,
    );
    let dt = 1.0 / 60.0;
    // Sprinting at it: stopped.
    let (mut feet, mut vy) = (Vec3::ZERO, 0.0);
    for _ in 0..120 {
        (feet, vy) = w.move_body(feet, Vec2::Y * 8.0 * dt, vy, dt);
    }
    assert!(feet.z < 2.0 && feet.y == 0.0, "{feet}");
    // A running jump: over.
    let (mut feet, mut vy) = (Vec3::new(0.0, 0.0, 0.6), crate::player::JUMP_SPEED);
    for _ in 0..120 {
        (feet, vy) = w.move_body(feet, Vec2::Y * 5.2 * dt, vy, dt);
    }
    assert!(feet.z > 2.5, "{feet}");
}

#[test]
fn gunners_get_out_onto_the_walk_beside_their_turret() {
    for l in every_plan() {
        let (kind, v) = (l.kind, l.variant);
        let w = world_of(&l);
        assert_eq!(l.turrets.len(), l.turret_stands.len());
        assert!(!l.turrets.is_empty());
        for (&t, &s) in l.turrets.iter().zip(&l.turret_stands) {
            assert!(free(&w, s), "{kind:?} {v}: turret {t}: stand {s} blocked");
            let g = w.ground(s.x, s.z, s.y + 0.2).unwrap();
            assert!((g - s.y).abs() < 0.05, "{kind:?} {v}: stand {s} floor {g}");
            // Within reach to get back in (as `enter_turret` has it).
            let eye = s + Vec3::Y * 1.6;
            assert!((t + Vec3::Y - eye).length() < 6.5, "{kind:?} {v}: {t} {s}");
            // The pillar under it is solid.
            assert!(w.occupied(t.x, t.z, s.y + 0.5, s.y + 1.5), "{kind:?} {v}");
        }
    }
}

#[test]
fn nothing_stands_over_the_caves_headroom() {
    for l in every_plan() {
        for c in &l.colliders {
            let (_, max) = c.bounds();
            assert!(
                max.y <= l.center.y + MAX_HEIGHT,
                "{:?} {}: {max}",
                l.kind,
                l.variant
            );
        }
        for d in &l.details {
            for p in &d.pts {
                assert!(p.y <= l.center.y + MAX_HEIGHT);
            }
        }
    }
}

/// A fingerprint of a layout's solid parts and fixtures.
fn fingerprint(l: &BaseLayout) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    let mut eat = |v: f32| {
        // (Rounded to the millimetre.)
        for b in ((v * 1000.0).round() as i64).to_le_bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    for c in &l.colliders {
        let (a, b) = c.bounds();
        for v in [a, b] {
            eat(v.x);
            eat(v.y);
            eat(v.z);
        }
        if let Collider::Ramp { dir, .. } = c {
            eat(*dir as f32);
        }
    }
    for t in &l.terminals {
        eat(t.pos.x);
        eat(t.pos.y);
        eat(t.pos.z);
        eat(t.yaw);
    }
    for (s, yaw) in &l.spawns {
        eat(s.x);
        eat(s.y);
        eat(s.z);
        eat(*yaw);
    }
    h
}

#[test]
fn layouts_are_the_same_every_time() {
    // Clients build the bases from the seed: the same layout for the same site and seed,
    // here and after any change that isn't meant to move a thing (the fingerprints; with one,
    // `PROTOCOL` changes too).
    let mut got = Vec::new();
    for (kind, facing) in [
        (BaseKind::Outpost, 0.0),
        (BaseKind::Center, 1.6),
        (BaseKind::Home, 3.1),
        (BaseKind::Tower, -1.5),
    ] {
        for seed in 0..4 {
            let s = site(kind, facing, Capture::Console);
            let a = fingerprint(&layout(&s, 10.0, seed));
            assert_eq!(a, fingerprint(&layout(&s, 10.0, seed)));
            got.push(a);
        }
    }
    assert_eq!(got, GOLDEN, "{got:#x?}");
}

#[test]
fn the_plans_of_some_seeds() {
    // Which plan a seed gives (for picking seeds to look at); every plan of every kind comes
    // up, mirrored and not.
    for kind in [
        BaseKind::Outpost,
        BaseKind::Center,
        BaseKind::Home,
        BaseKind::Tower,
    ] {
        let mut seen = Vec::new();
        for seed in 0..40 {
            let l = layout(&site(kind, 0.0, Capture::Console), 0.0, seed);
            println!(
                "{kind:?} seed {seed}: plan {} flip {} storeys {}",
                l.variant, l.flip, l.storeys
            );
            if !seen.contains(&(l.variant, l.flip)) {
                seen.push((l.variant, l.flip));
            }
        }
        assert!(seen.len() >= PLANS, "{kind:?}: {seen:?}");
    }
}

/// How many plans each kind has.
const PLANS: usize = 1;

const GOLDEN: [u64; 16] = [
    0x5aeb6c3d0e008e20,
    0xd533c318b50c8d22,
    0xef50e67b66cb00ca,
    0x62930aed2d4e1735,
    0x386006fe1d0f85a,
    0xa13ccbb2d0e7ce2d,
    0x28bafeb051371bde,
    0x12a7e520dd660d09,
    0x61fde3f23f0f5741,
    0x254495c08d02dc20,
    0xf945e52783cb1456,
    0xd7ca2873555b70f7,
    0x120c6fe819688d5a,
    0x120c6fe819688d5a,
    0x120c6fe819688d5a,
    0x120c6fe819688d5a,
];
