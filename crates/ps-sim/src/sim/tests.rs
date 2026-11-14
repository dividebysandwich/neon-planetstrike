use super::*;
use crate::bots::Bots;
use crate::events::HackTarget;
use crate::state::WorldState;
use crate::weapons::{ArmorKind, Kit};
use crate::worldgen::{WorldDef, WorldParams};
use crate::{Hand, Pose, VrInput};

fn setup() -> (WorldDef, WorldState, ContinentSim) {
    setup_on(false)
}

/// A front continent of world 11: the first (on the open land, as it was before the caves),
/// or the caves.
fn setup_on(caves: bool) -> (WorldDef, WorldState, ContinentSim) {
    let def = WorldDef::generate(WorldParams {
        seed: 11,
        contested: 6,
    });
    let def = if caves { def } else { def.without_caves() };
    let st = WorldState::initial(&def);
    // A front: a continent someone holds, next to someone else's.
    let c = (3..def.continents.len() as u8)
        .find(|&c| {
            st.continents[c as usize].owner.is_some()
                && st.contested(&def, c)
                && (!caves || def.continents[c as usize].theme.caves())
        })
        .unwrap();
    let map = Arc::new(ContinentMap::build(
        Arc::new(def.continents[c as usize].clone()),
        false,
    ));
    let mut sim = ContinentSim::new(c, map);
    sync(&def, &st, &mut sim);
    (def, st, sim)
}

fn sync(def: &WorldDef, st: &WorldState, sim: &mut ContinentSim) {
    let c = sim.id;
    for (b, bs) in st.continents[c as usize].bases.iter().enumerate() {
        let hackable = Team::ALL.map(|t| st.can_hack(def, c, b as BaseIdx, t));
        sim.set_base(b as BaseIdx, bs.owner, bs.hack, hackable);
    }
    sim.gate_owners = def.continents[c as usize]
        .gates
        .iter()
        .map(|g| st.continents[g.to as usize].owner)
        .collect();
}

#[test]
fn players_spawn_at_owned_bases_and_shoot_each_other() {
    let (_, _, mut sim) = setup();
    let owner = sim.bases[0].owner.unwrap_or(Team::Red);
    sim.bases[0].owner = Some(owner);
    let enemy = Team::ALL.into_iter().find(|t| *t != owner).unwrap();
    sim.add_player(1, "a", owner, false);
    sim.add_player(2, "b", enemy, false);
    for p in &mut sim.players {
        p.dead = Some(RESPAWN_TIME);
    }
    sim.request(
        1,
        Request::Spawn(SpawnPoint::Base {
            continent: sim.id,
            base: 0,
        }),
    );
    assert!(sim.player(1).unwrap().alive());
    // An enemy can't spawn there.
    sim.request(
        2,
        Request::Spawn(SpawnPoint::Base {
            continent: sim.id,
            base: 0,
        }),
    );
    assert!(!sim.player(2).unwrap().alive());
    // Put them in the open, facing each other 20 m apart, and let 1 fire.
    // In the open, far from every base's turrets.
    let mut c = Vec3::ZERO;
    'find: for r in [500.0f32, 700.0, 900.0, 1100.0] {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            if sim
                .map
                .bases
                .iter()
                .all(|b| (b.center - p).length() > 350.0)
            {
                c = p;
                break 'find;
            }
        }
    }
    let ground = |sim: &ContinentSim, p: Vec3| {
        Vec3::new(p.x, sim.world.ground(p.x, p.z, p.y + 50.0).unwrap(), p.z)
    };
    let a = ground(&sim, c);
    let b = ground(&sim, c + Vec3::new(0.0, 0.0, -20.0));
    sim.spawn_at(1, a, 0.0);
    sim.spawn_at(2, b, std::f32::consts::PI);
    for p in &mut sim.players {
        p.protect = 0.0;
    }
    let mut kills = 0;
    for _ in 0..600 {
        let (p1, p2) = (
            sim.player(1).unwrap().clone(),
            sim.player(2).unwrap().clone(),
        );
        if !p2.alive() {
            kills += 1;
            break;
        }
        let d = (p2.eye() - Vec3::Y * 0.6 - p1.eye()).normalize();
        let cmd = PlayerCmd {
            yaw: yaw_of(d),
            pitch: d.y.asin(),
            fire: true,
            ..Default::default()
        };
        sim.set_cmd(1, cmd);
        sim.step();
    }
    assert_eq!(kills, 1, "the enemy went down");
    let killed: Vec<_> = sim
        .out
        .iter()
        .filter(|o| matches!(o.ev, SimEvent::Killed { .. }))
        .collect();
    assert!(
        sim.out.iter().any(|o| matches!(
            o.ev,
            SimEvent::Killed {
                victim: 2,
                by: 1,
                ..
            }
        )),
        "{killed:?}"
    );
    let s = sim.snapshot_for(1, 0);
    assert!(s.me.is_some_and(|m| m.kills == 1));
}

#[test]
fn far_players_are_left_out_of_snapshots() {
    let (_, _, mut sim) = setup();
    sim.add_player(1, "a", Team::Red, false);
    sim.add_player(2, "b", Team::Blue, false);
    sim.add_player(3, "c", Team::Blue, false);
    sim.spawn_at(1, Vec3::new(0.0, 50.0, 0.0), 0.0);
    sim.spawn_at(2, Vec3::new(100.0, 50.0, 0.0), 0.0);
    sim.spawn_at(3, Vec3::new(1200.0, 50.0, 0.0), 0.0);
    let s = sim.snapshot_for(1, 0);
    let ids: Vec<_> = s.players.iter().map(|p| p.id).collect();
    assert!(
        ids.contains(&1) && ids.contains(&2) && !ids.contains(&3),
        "{ids:?}"
    );
    // A deployed enemy bus is cloaked beyond a short range.
    let bus = sim.add_vehicle(
        VehicleKind::SpawnBus,
        Team::Blue,
        Vec3::new(60.0, 50.0, 0.0),
        0.0,
    );
    sim.vehicles.last_mut().unwrap().deployed = true;
    assert!(!sim.snapshot_for(1, 0).vehicles.iter().any(|v| v.id == bus));
    assert!(sim.snapshot_for(2, 0).vehicles.iter().any(|v| v.id == bus));
}

#[test]
fn vehicles_are_spawned_entered_driven_and_left() {
    let (_, _, mut sim) = setup();
    let b = 1;
    let team = Team::Blue;
    sim.bases[b].owner = Some(team);
    sim.add_player(1, "a", team, false);
    let (ti, term) = sim.map.bases[b]
        .terminals
        .iter()
        .enumerate()
        .find(|(_, t)| t.kind == TerminalKind::Vehicle)
        .map(|(i, t)| (i, *t))
        .unwrap();
    sim.spawn_at(1, term.pos + Vec3::new(0.5, 0.0, 0.5), 0.0);
    sim.request(
        1,
        Request::SpawnVehicle {
            base: b as BaseIdx,
            terminal: ti as u8,
            kind: VehicleKind::LightTank,
            loadout: Default::default(),
        },
    );
    assert_eq!(sim.vehicles.len(), 1);
    let vid = sim.vehicles[0].id;
    assert_eq!(sim.player(1).unwrap().seat, Some((vid, 0)));
    let start = sim.vehicles[0].motion.pos;
    let mut cmd = PlayerCmd::default();
    cmd.drive.throttle = 1.0;
    sim.set_cmd(1, cmd);
    for _ in 0..120 {
        sim.step();
    }
    let moved = (sim.vehicles[0].motion.pos - start).length();
    assert!(moved > 5.0, "{moved}");
    sim.set_cmd(1, PlayerCmd::default());
    for _ in 0..120 {
        sim.step();
    }
    sim.request(1, Request::ExitVehicle);
    let p = sim.player(1).unwrap();
    assert!(p.seat.is_none() && p.alive());
    assert!(
        vehicle::hull_distance(
            &sim.vehicles[0].motion,
            VehicleKind::LightTank.spec().half,
            p.feet()
        ) < 4.0
    );
    // Another spawn so soon: the cooldown.
    sim.out.clear();
    sim.spawn_at(1, term.pos + Vec3::new(0.5, 0.0, 0.5), 0.0);
    sim.request(
        1,
        Request::SpawnVehicle {
            base: b as BaseIdx,
            terminal: ti as u8,
            kind: VehicleKind::Vtol,
            loadout: Default::default(),
        },
    );
    assert!(
        sim.out
            .iter()
            .any(|o| matches!(o.ev, SimEvent::Denied(Deny::Cooldown(_))))
    );
}

#[test]
fn bots_fight_and_hack() {
    let seed: u64 = std::env::var("PS_TEST_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    // (Seven minutes of a battle may by chance see no console solved: a few tries.)
    let mut tries = Vec::new();
    for s in seed..seed + 3 {
        let r = battle(s);
        tries.push(r);
        if r.2 > 0 {
            break;
        }
    }
    assert!(
        tries.iter().all(|r| r.0 > 0),
        "bots took vehicles: {tries:?}"
    );
    assert!(tries.iter().all(|r| r.1 > 0), "bots fought: {tries:?}");
    assert!(
        tries.iter().any(|r| r.2 > 0),
        "bots hacked a console: {tries:?}"
    );
}

#[test]
fn bots_fight_and_take_bases_in_the_caves() {
    let mut tries = Vec::new();
    // (Longer: everything is along the canyons.)
    for s in 1..4 {
        let r = battle_on(s, true, 900);
        tries.push(r);
        if r.2 > 0 {
            break;
        }
    }
    assert!(tries.iter().all(|r| r.1 > 0), "bots fought: {tries:?}");
    assert!(
        tries.iter().any(|r| r.2 > 0),
        "bots took something: {tries:?}"
    );
}

/// Seven minutes of bots fighting over a continent: the vehicles they took, kills, hacks.
fn battle(seed: u64) -> (usize, usize, usize) {
    battle_on(seed, false, 420)
}

fn battle_on(seed: u64, caves: bool, secs: u64) -> (usize, usize, usize) {
    let (def, mut st, mut sim) = setup_on(caves);
    let factions = st.factions_on(&def, sim.id);
    assert!(factions.len() >= 2, "{factions:?}");
    let mut bots = Bots::new(0.6, seed);
    let mut id = 10;
    for &t in &factions {
        for _ in 0..8 {
            bots.add(&mut sim, id, t);
            id += 1;
        }
    }
    let mut kills = 0;
    let mut hacks = 0;
    let mut vehicles_seen = std::collections::HashSet::new();
    let mut deployed = 0;
    let mut wrecks = 0;
    let (mut flown, mut crewed, mut far_driven) = (0.0f32, 0, 0.0f32);
    let mut starts: std::collections::HashMap<u16, Vec3> = Default::default();
    let mut shots: std::collections::BTreeMap<String, u32> = Default::default();
    let mut shot_ids = std::collections::HashSet::new();
    let start = std::time::Instant::now();
    for tick in 0..60 * secs {
        bots.think(&mut sim);
        sim.step();
        for o in sim.out.drain(..) {
            if matches!(o.ev, SimEvent::Killed { .. }) {
                kills += 1;
            }
            if matches!(o.ev, SimEvent::VehicleDestroyed { .. }) {
                wrecks += 1;
            }
        }
        for (b, team, _) in std::mem::take(&mut sim.solved) {
            if !st.console_solved(&def, sim.id, b, team).is_empty() {
                hacks += 1;
            }
        }
        for (b, team) in std::mem::take(&mut sim.majority) {
            let ch = st.points_held(&def, sim.id, b, team);
            if matches!(ch.first(), Some(crate::state::Change::HackStarted { .. })) {
                hacks += 1;
            }
        }
        for (_, f) in &sim.fx {
            if let crate::snapshot::Fx::Shot { id, ammo, .. } = f
                && shot_ids.insert(*id)
            {
                *shots.entry(format!("{ammo:?}")).or_default() += 1;
            }
        }
        for v in &sim.vehicles {
            vehicles_seen.insert(v.id);
            let from = *starts.entry(v.id).or_insert(v.motion.pos);
            if v.kind == VehicleKind::Vtol {
                let above =
                    v.motion.pos.y - sim.map.terrain.surface(v.motion.pos.x, v.motion.pos.z);
                flown = flown.max(above);
            } else {
                far_driven = far_driven.max((v.motion.pos - from).length());
            }
            if v.seats.iter().skip(1).any(|s| s.is_some()) && v.kind != VehicleKind::SpawnBus {
                crewed += 1;
            }
        }
        deployed = deployed.max(sim.vehicles.iter().filter(|v| v.deployed).count());
        if tick % 60 == 0 {
            st.tick_hacks(&def, sim.id, 1.0);
            sync(&def, &st, &mut sim);
        }
    }
    let per_tick = start.elapsed().as_secs_f64() / (60.0 * secs as f64) * 1000.0;
    println!(
        "kills {kills}, hacks {hacks}, vehicles {}, buses deployed {deployed}, wrecks {wrecks}, \
         VTOL height {flown:.0} m, crewed ticks {crewed}, driven {far_driven:.0} m, {per_tick:.3} ms/tick",
        vehicles_seen.len()
    );
    println!("shots {shots:?}");
    (vehicles_seen.len(), kills, hacks)
}

#[test]
fn walking_into_a_gate_core_warps() {
    let (def, _, mut sim) = setup();
    let g = sim.map.gates[0];
    sim.add_player(1, "a", Team::Red, false);
    // Just outside the core, walking in.
    let start = g + Vec3::new(ps_sim_gate_core() + 3.0, 0.0, 0.0);
    let start = Vec3::new(
        start.x,
        sim.world.ground(start.x, start.z, start.y + 5.0).unwrap(),
        start.z,
    );
    sim.spawn_at(1, start, std::f32::consts::FRAC_PI_2);
    let cmd = PlayerCmd {
        stick: Vec2::new(-1.0, 0.0),
        ..Default::default()
    };
    sim.set_cmd(1, cmd);
    let mut warped = None;
    for _ in 0..240 {
        sim.step();
        if let Some(w) = sim.warps.first() {
            warped = Some(*w);
            break;
        }
    }
    assert_eq!(warped, Some((Warper::Player(1), 0)));
    // Where it leads, and the way back.
    let (to, back) = def.gate_target(sim.id, 0);
    assert_eq!(def.gate_target(to, back), (sim.id, 0));
    // Arriving through the gate puts one outside the core, facing inland.
    let (at, _) = sim.arrival(0, false);
    assert!(Vec2::new(at.x - g.x, at.z - g.z).length() > ps_sim_gate_core());
}

fn ps_sim_gate_core() -> f32 {
    crate::base::GATE_CORE
}

#[test]
fn nobody_fires_in_a_warp_gate_and_shots_stop_at_its_bubble() {
    let (_, _, mut sim) = setup();
    let g = sim.map.gates[0];
    sim.add_player(1, "a", Team::Red, false);
    // Inside the bubble (outside the core): the trigger does nothing.
    let mut feet = g + Vec3::new(25.0, 0.0, 0.0);
    feet.y = sim
        .world
        .ground(feet.x, feet.z, feet.y + 5.0)
        .unwrap_or(feet.y);
    sim.spawn_at(1, feet, 0.0);
    sim.players[0].protect = 0.0;
    sim.set_cmd(
        1,
        PlayerCmd {
            fire: true,
            ..Default::default()
        },
    );
    for _ in 0..30 {
        sim.step();
    }
    assert!(sim.projectiles.is_empty());
    assert_eq!(
        sim.player(1).unwrap().guns[0].mag,
        Weapon::Rifle.stats().mag
    );
    // A shot from outside towards the middle ends on the shell.
    let from = g + Vec3::new(60.0, 3.0, 0.0);
    sim.launch(
        Ammo::Gun(Weapon::Rifle),
        1,
        Team::Blue,
        from,
        Vec3::new(-220.0, 0.0, 0.0),
        0,
        None,
    );
    // (Effects are kept a few ticks.)
    for _ in 0..10 {
        sim.step();
    }
    assert!(sim.projectiles.is_empty());
    let hit = sim.fx.iter().find_map(|(_, f)| match f {
        Fx::Impact { at, what, .. } => Some((*at, *what)),
        _ => None,
    });
    let (at, what) = hit.expect("an impact");
    assert_eq!(what, ImpactOn::Shield);
    assert!((Vec2::new(at.x - g.x, at.z - g.z).length() - crate::base::GATE_BUBBLE_R).abs() < 2.0);
}

#[test]
fn turrets_fire_slower_by_themselves() {
    let (_, _, mut sim) = setup();
    let ti = 0;
    let base = sim.turrets[ti].base as usize;
    let owner = Team::Red;
    sim.bases[base].owner = Some(owner);
    let head = sim.turrets[ti].pos + Vec3::Y * TURRET_PIVOT;
    // An enemy in the open before it.
    sim.add_player(2, "b", Team::Blue, false);
    let mut feet = head + Vec3::new(0.0, 0.0, -40.0);
    feet.y = sim
        .world
        .ground(feet.x, feet.z, feet.y)
        .unwrap_or(feet.y - 10.0);
    sim.spawn_at(2, feet, 0.0);
    sim.turrets[ti].target = Some(Target::Player(2));
    sim.turrets[ti].think = 100.0;
    sim.turrets[ti].cooldown = 0.0;
    sim.step();
    // Whatever it shot at, the next shot waits the automatic interval.
    let c = sim.turrets[ti].cooldown;
    assert!(c == 0.0 || c > TURRET_INTERVAL * 2.0, "{c}");
    if c > 0.0 {
        assert!((c - (TURRET_INTERVAL / TURRET_AUTO_RATE - TICK)).abs() < 0.02);
    }
}

#[test]
fn vr_players_fire_only_with_a_gun_drawn() {
    let (_, _, mut sim) = setup();
    sim.add_player(1, "a", Team::Red, false);
    let mut feet = sim.map.bases[0].center + Vec3::new(30.0, 0.0, 30.0);
    feet.y = sim
        .world
        .ground(feet.x, feet.z, feet.y + 50.0)
        .unwrap_or(feet.y);
    sim.spawn_at(1, feet, 0.0);
    sim.players[0].protect = 0.0;
    let full = Weapon::Rifle.stats().mag;
    let mut vr = VrInput {
        muzzle: Pose {
            pos: Vec3::new(0.2, 1.4, -0.4),
            rot: Quat::IDENTITY,
        },
        ..Default::default()
    };
    // Holstered (no gun hand): the trigger does nothing.
    sim.set_cmd(
        1,
        PlayerCmd {
            fire: true,
            vr: Some(vr),
            ..Default::default()
        },
    );
    for _ in 0..30 {
        sim.step();
    }
    assert_eq!(sim.player(1).unwrap().guns[0].mag, full);
    // Drawn: it fires.
    vr.gun_hand = Some(Hand::Right);
    sim.set_cmd(
        1,
        PlayerCmd {
            fire: true,
            vr: Some(vr),
            ..Default::default()
        },
    );
    for _ in 0..30 {
        sim.step();
    }
    assert!(sim.player(1).unwrap().guns[0].mag < full);
}

#[test]
fn nodes_come_in_three_sizes_go_when_mined_and_grow_back() {
    let (_, _, mut sim) = setup();
    sim.step_nodes(TICK);
    assert_eq!(sim.nodes.len(), NODES);
    for n in &sim.nodes {
        assert!(
            NODE_SIZES
                .iter()
                .any(|k| (k * CARGO - n.amount).abs() < 1e-3),
            "{}",
            n.amount
        );
        let t = sim.world.terrain.as_ref().unwrap();
        assert!(t.water(n.pos.x, n.pos.z) <= n.pos.y - 0.3);
    }
    // Mined out: gone, and back somewhere within the regrowth window.
    sim.nodes[0].amount = 0.0;
    sim.step_nodes(TICK);
    assert_eq!(sim.nodes.len(), NODES - 1);
    let mut t = 0.0;
    while sim.nodes.len() < NODES {
        sim.step_nodes(1.0);
        t += 1.0;
        assert!(t <= NODE_REGROW.1 + 1.0, "not back after {t} s");
    }
    assert!(t >= NODE_REGROW.0 - 1.0, "back too soon: {t} s");
}

/// A spot on the ground in the open, far from every base (and its turrets).
fn open_ground(sim: &ContinentSim) -> Vec3 {
    for r in [500.0f32, 700.0, 900.0, 1100.0] {
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            if sim
                .map
                .bases
                .iter()
                .all(|b| (b.center - p).length() > 350.0)
            {
                let y = sim.world.ground(p.x, p.z, 500.0).unwrap();
                return Vec3::new(p.x, y, p.z);
            }
        }
    }
    panic!("no open ground");
}

#[test]
fn medics_revive_the_downed_where_they_fell() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "medic", Team::Red, false);
    sim.add_player(2, "fallen", Team::Red, false);
    sim.add_player(3, "enemy", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(2, at + Vec3::new(0.0, 0.0, -4.0), 0.0);
    sim.spawn_at(3, at + Vec3::new(30.0, 0.0, 0.0), 0.0);
    sim.player_mut(1).unwrap().loadout.special = Weapon::Medgun;
    sim.player_mut(1).unwrap().refill();
    let fell_at = sim.player(2).unwrap().feet();
    let i = sim.players.iter().position(|p| p.id == 2).unwrap();
    sim.kill(i, 3, Cause::Weapon(Ammo::Gun(Weapon::Rifle)), false);
    assert!(sim.player(2).unwrap().revivable());
    // Its faction sees the body (downed) after the moment everyone does; the enemy doesn't.
    for _ in 0..240 {
        sim.step();
    }
    let body = |sim: &ContinentSim, viewer| {
        sim.snapshot_for(viewer, 0)
            .players
            .into_iter()
            .find(|p| p.id == 2)
    };
    assert!(body(&sim, 1).is_some_and(|p| p.has(flags::DOWNED) && p.has(flags::DEAD)));
    assert!(body(&sim, 3).is_none());
    assert!(sim.snapshot_for(2, 0).me.unwrap().downed.is_some());
    // The medic's beam on the body for long enough brings it back there.
    let eye = sim.player(1).unwrap().eye();
    let d = (fell_at + Vec3::Y * 0.3 - eye).normalize();
    let cmd = PlayerCmd {
        yaw: yaw_of(d),
        pitch: d.y.asin(),
        fire: true,
        slot: 2,
        ..Default::default()
    };
    let mut ticks = 0;
    while !sim.player(2).unwrap().alive() && ticks < 600 {
        sim.set_cmd(1, cmd);
        sim.step();
        ticks += 1;
    }
    let p = sim.player(2).unwrap();
    assert!(p.alive(), "revived");
    assert!((p.feet() - fell_at).length() < 0.5);
    assert_eq!(p.health, REVIVE_HEALTH);
    assert!(
        ticks as f32 * TICK >= REVIVE_TIME - 0.2,
        "not at once: {ticks} ticks"
    );
    assert!(
        sim.out
            .iter()
            .any(|o| matches!(o.ev, SimEvent::Revived { player: 2, by: 1 }))
    );
}

#[test]
fn no_revive_after_suicide_or_once_the_window_has_passed() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "a", Team::Red, false);
    sim.add_player(2, "b", Team::Red, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(2, at + Vec3::new(3.0, 0.0, 0.0), 0.0);
    sim.request(1, Request::Suicide);
    assert!(!sim.player(1).unwrap().alive());
    assert!(!sim.player(1).unwrap().revivable());
    let i = sim.players.iter().position(|p| p.id == 2).unwrap();
    sim.kill(i, 1, Cause::Fall, false);
    assert!(sim.player(2).unwrap().revivable());
    sim.player_mut(2).unwrap().dead = Some(REVIVE_WINDOW + 0.1);
    assert!(!sim.player(2).unwrap().revivable());
    // Spawning elsewhere ends it too.
    sim.player_mut(2).unwrap().dead = Some(RESPAWN_TIME);
    sim.spawn_at(2, at, 0.0);
    assert!(!sim.player(2).unwrap().downed);
}

#[test]
fn squad_members_spawn_on_their_leader() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "leader", Team::Red, false);
    sim.add_player(2, "member", Team::Red, false);
    sim.add_player(3, "enemy", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    for id in [2, 3] {
        sim.player_mut(id).unwrap().dead = Some(RESPAWN_TIME);
    }
    // Next to the leader on foot.
    sim.request(2, Request::Spawn(SpawnPoint::Squad(1)));
    let p = sim.player(2).unwrap();
    assert!(p.alive());
    assert!((p.feet() - at).length() < 3.0, "{:?} vs {at:?}", p.feet());
    // Not on an enemy.
    sim.request(3, Request::Spawn(SpawnPoint::Squad(1)));
    assert!(!sim.player(3).unwrap().alive());
    // Into a free seat of the leader's vehicle.
    let v = sim.add_vehicle(VehicleKind::HeavyTank, Team::Red, at + Vec3::Y, 0.0);
    sim.player_mut(1).unwrap().body.feet = at + Vec3::new(2.0, 0.0, 0.0);
    sim.request(1, Request::EnterVehicle(v));
    assert_eq!(sim.player(1).unwrap().seat.map(|s| s.0), Some(v));
    let i = sim.players.iter().position(|p| p.id == 2).unwrap();
    sim.kill(i, 3, Cause::Fall, false);
    sim.player_mut(2).unwrap().dead = Some(RESPAWN_TIME);
    sim.request(2, Request::Spawn(SpawnPoint::Squad(1)));
    assert_eq!(sim.player(2).unwrap().seat.map(|s| s.0), Some(v));
    // Not on a dead leader.
    sim.add_player(4, "late", Team::Red, false);
    sim.player_mut(4).unwrap().dead = Some(RESPAWN_TIME);
    let i = sim.players.iter().position(|p| p.id == 1).unwrap();
    sim.kill(i, 3, Cause::Fall, false);
    sim.request(4, Request::Spawn(SpawnPoint::Squad(1)));
    assert!(!sim.player(4).unwrap().alive());
    assert!(
        sim.out
            .iter()
            .any(|o| o.to == Audience::Player(4)
                && o.ev == SimEvent::Denied(Deny::LeaderUnavailable))
    );
}

#[test]
fn kills_and_assists_earn_xp() {
    use crate::progress::{self, Award};
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "killer", Team::Red, false);
    sim.add_player(2, "helper", Team::Red, false);
    sim.add_player(3, "victim", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(2, at + Vec3::new(3.0, 0.0, 0.0), 0.0);
    sim.spawn_at(3, at + Vec3::new(0.0, 0.0, -20.0), 0.0);
    sim.player_mut(3).unwrap().protect = 0.0;
    let vi = sim.players.iter().position(|p| p.id == 3).unwrap();
    let from = at;
    let cause = Cause::Weapon(Ammo::Gun(Weapon::Rifle));
    // The helper's damage, then the killer's headshot.
    sim.damage_player(vi, 30.0, 2, cause, from, false, true);
    sim.damage_player(vi, 200.0, 1, cause, from, true, true);
    assert!(!sim.player(3).unwrap().alive());
    let got = |id| -> Vec<Award> {
        sim.awards
            .iter()
            .filter(|a| a.0 == id)
            .map(|a| a.1)
            .collect()
    };
    assert_eq!(got(1), vec![Award::Kill, Award::Headshot]);
    assert_eq!(got(2), vec![Award::Assist]);
    assert_eq!(
        sim.player(1).unwrap().score,
        progress::KILL_XP + progress::HEADSHOT_XP
    );
    assert_eq!(sim.player(2).unwrap().assists, 1);
    assert!(got(3).is_empty());
    // The player is told.
    assert!(sim.out.iter().any(|o| o.to == Audience::Player(2)
        && o.ev
            == SimEvent::Xp {
                reason: Award::Assist,
                xp: progress::ASSIST_XP
            }));
}

/// A Skyhook gunner on open ground and an enemy VTOL flying across 150 m ahead, 60 m up (moved
/// by hand each tick, sideways). Returns the sim, the gunner's id and the VTOL's.
fn skyhook_range() -> (ContinentSim, PlayerId, VehicleId) {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "gunner", Team::Red, false);
    sim.spawn_at(1, at, 0.0);
    let p = sim.player_mut(1).unwrap();
    p.loadout.special = Weapon::Skyhook;
    p.refill();
    let v = sim.add_vehicle(
        VehicleKind::Vtol,
        Team::Blue,
        at + Vec3::new(0.0, 60.0, -150.0),
        0.0,
    );
    sim.vehicles.last_mut().unwrap().motion.grounded = false;
    (sim, 1, v)
}

/// Steps the range: the VTOL slides sideways at 15 m/s; the gunner aims at it and pulls the
/// trigger when `fire` says.
fn fly_by(sim: &mut ContinentSim, gunner: PlayerId, v: VehicleId, ticks: usize, fire: bool) {
    for _ in 0..ticks {
        if let Some(vi) = sim.vehicles.iter().position(|x| x.id == v) {
            let veh = &mut sim.vehicles[vi];
            veh.motion.pos.x += 15.0 * TICK;
            veh.motion.vel = Vec3::new(15.0, 0.0, 0.0);
            veh.motion.grounded = false;
        }
        let eye = sim.player(gunner).unwrap().eye();
        let c = sim.vehicle(v).map_or(eye + Vec3::NEG_Z, |x| x.center());
        let d = (c - eye).normalize();
        let cmd = PlayerCmd {
            yaw: yaw_of(d),
            pitch: d.y.asin(),
            slot: 2,
            fire,
            ..Default::default()
        };
        sim.set_cmd(gunner, cmd);
        sim.step();
    }
}

#[test]
fn skyhooks_lock_on_and_their_missiles_home() {
    let (mut sim, gunner, v) = skyhook_range();
    // Not locked at once; locked after holding it in the cone.
    fly_by(&mut sim, gunner, v, 30, false);
    assert!(
        sim.player(gunner)
            .unwrap()
            .lock
            .is_some_and(|l| l.0 == v && l.1 < weapons::LOCK_TIME)
    );
    fly_by(&mut sim, gunner, v, 90, false);
    assert!(
        sim.player(gunner)
            .unwrap()
            .lock
            .is_some_and(|l| l.1 >= weapons::LOCK_TIME)
    );
    // The pilot is warned (no pilot here: the warning is in a seat's snapshot; checked below).
    let before = sim.vehicle(v).unwrap().health;
    fly_by(&mut sim, gunner, v, 2, true);
    assert!(
        sim.projectiles.iter().any(|p| p.homing == Some(v)),
        "a homing missile"
    );
    // Aimed nowhere near it now: it still gets there.
    for _ in 0..240 {
        if let Some(vi) = sim.vehicles.iter().position(|x| x.id == v) {
            sim.vehicles[vi].motion.pos.x += 15.0 * TICK;
        }
        sim.set_cmd(gunner, PlayerCmd::default());
        sim.step();
    }
    let after = sim.vehicle(v).map_or(0.0, |x| x.health);
    assert!(after < before - 15.0, "hit: {before} -> {after}");
}

#[test]
fn flares_break_locks_and_lead_missiles_astray() {
    let (mut sim, gunner, v) = skyhook_range();
    fly_by(&mut sim, gunner, v, 120, false);
    fly_by(&mut sim, gunner, v, 2, true);
    assert!(sim.projectiles.iter().any(|p| p.homing == Some(v)));
    let vi = sim.vehicles.iter().position(|x| x.id == v).unwrap();
    let health = sim.vehicles[vi].health;
    sim.flare(vi);
    assert!(sim.projectiles.iter().all(|p| p.homing != Some(v)));
    assert!(sim.player(gunner).unwrap().lock.is_none());
    // No new lock while they burn; a second lot isn't ready yet.
    fly_by(&mut sim, gunner, v, 100, false);
    assert!(
        sim.player(gunner)
            .unwrap()
            .lock
            .is_none_or(|l| l.1 < weapons::LOCK_TIME)
    );
    assert_eq!(sim.vehicle(v).unwrap().health, health, "the missile missed");
    let vi = sim.vehicles.iter().position(|x| x.id == v).unwrap();
    assert!(sim.vehicles[vi].flare_wait > 0.0);
}

#[test]
fn a_seat_with_missiles_warns_the_target_crew() {
    let (mut sim, _, v) = skyhook_range();
    // A blue pilot in the VTOL; the red gunner locking on.
    sim.add_player(5, "pilot", Team::Blue, false);
    let vi = sim.vehicles.iter().position(|x| x.id == v).unwrap();
    let pos = sim.vehicles[vi].motion.pos;
    sim.spawn_at(5, pos, 0.0);
    sim.bot_board(5, v, 0);
    assert!(sim.player(5).unwrap().seat.is_some());
    fly_by(&mut sim, 1, v, 20, false);
    assert_eq!(sim.snapshot_for(5, 0).me.unwrap().warning, 1);
}

/// A base (not a sanctuary) with an equipment terminal, held by `owner`: the base, the
/// terminal's index and where it stands.
fn base_with_terminal(
    sim: &mut ContinentSim,
    owner: Team,
    kind: TerminalKind,
) -> (usize, usize, Vec3) {
    let (b, t) = sim
        .map
        .bases
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind != BaseKind::Home)
        .find_map(|(b, l)| {
            l.terminals
                .iter()
                .position(|t| t.kind == kind)
                .map(|t| (b, t))
        })
        .expect("a base with the terminal");
    sim.set_base(b as BaseIdx, Some(owner), None, [false; 3]);
    sim.bases[b].resources = 500.0;
    (b, t, sim.map.bases[b].terminals[t].pos)
}

fn loadout(armor: ArmorKind, rifle: Weapon, special: Weapon, kit: Kit) -> Loadout {
    Loadout {
        armor,
        rifle,
        sidearm: Weapon::Pistol,
        special,
        kit,
        grenade: Default::default(),
    }
}

#[test]
fn a_max_costs_resources_and_keeps_to_its_arms() {
    let (_, _, mut sim) = setup();
    let (b, _, term) = base_with_terminal(&mut sim, Team::Red, TerminalKind::Equipment);
    sim.add_player(1, "max", Team::Red, false);
    sim.spawn_at(1, term, 0.0);
    // Its arms only in the first slot; no rifle there.
    assert!(!loadout(ArmorKind::Max, Weapon::Rifle, Weapon::Medgun, Kit::None).valid(Team::Red));
    assert!(
        !loadout(ArmorKind::Light, Weapon::Pounder, Weapon::Medgun, Kit::None).valid(Team::Red)
    );
    // A faction's own rifle: only for its own.
    assert!(loadout(ArmorKind::Light, Weapon::Ember, Weapon::Medgun, Kit::None).valid(Team::Red));
    assert!(!loadout(ArmorKind::Light, Weapon::Ember, Weapon::Medgun, Kit::None).valid(Team::Blue));
    let before = sim.bases[b].resources;
    sim.request(
        1,
        Request::Loadout(loadout(
            ArmorKind::Max,
            Weapon::Pounder,
            Weapon::Striker,
            Kit::None,
        )),
    );
    let p = sim.player(1).unwrap();
    assert_eq!(p.loadout.armor, ArmorKind::Max);
    assert_eq!(p.armor, ArmorKind::Max.max());
    assert_eq!(p.grenades, 0);
    assert!((sim.bases[b].resources - (before - weapons::MAX_COST)).abs() < 1e-3);
    // Switching to the special: still the arms.
    sim.set_cmd(
        1,
        PlayerCmd {
            slot: 2,
            ..Default::default()
        },
    );
    sim.step();
    assert_eq!(sim.player(1).unwrap().weapon(), Weapon::Pounder);
    // An empty silo: no MAX.
    sim.bases[b].resources = 10.0;
    sim.request(
        1,
        Request::Loadout(loadout(
            ArmorKind::Max,
            Weapon::Mauler,
            Weapon::Striker,
            Kit::None,
        )),
    );
    assert_eq!(sim.player(1).unwrap().loadout.rifle, Weapon::Pounder);
    // It doesn't drive: a tank's seats are all controls or guns.
    let feet = sim.player(1).unwrap().feet();
    let tank = sim.add_vehicle(VehicleKind::LightTank, Team::Red, feet + Vec3::X * 3.0, 0.0);
    sim.request(1, Request::EnterVehicle(tank));
    assert!(sim.player(1).unwrap().seat.is_none());
    // A dropship's passenger seat will do.
    let ship = sim.add_vehicle(VehicleKind::Dropship, Team::Red, feet + Vec3::X * 6.0, 0.0);
    sim.request(1, Request::EnterVehicle(ship));
    let seat = sim.player(1).unwrap().seat.expect("aboard");
    let spec = VehicleKind::Dropship.spec();
    assert!(!spec.seats[seat.1 as usize].driver && spec.seats[seat.1 as usize].gun.is_none());
    // Nor does it switch into the pilot's or a gunner's seat.
    sim.step();
    for to in [0u8, 1] {
        let mut cmd = sim.player(1).unwrap().cmd;
        cmd.seat = to;
        cmd.seat_switches += 1;
        sim.set_cmd(1, cmd);
        sim.step();
        assert_eq!(sim.player(1).unwrap().seat, Some(seat));
    }
}

#[test]
fn medics_dont_heal_a_max_but_engineers_mend_it() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "max", Team::Red, false);
    sim.add_player(2, "medic", Team::Red, false);
    sim.add_player(3, "engineer", Team::Red, false);
    for (id, x) in [(1, 0.0), (2, -4.0), (3, 4.0)] {
        sim.spawn_at(id, at + Vec3::new(x, 0.0, 4.0), 0.0);
    }
    let p = sim.player_mut(1).unwrap();
    p.loadout = loadout(ArmorKind::Max, Weapon::Pounder, Weapon::Medgun, Kit::None);
    p.refill();
    p.armor = 100.0;
    p.health = 50.0;
    for (id, w) in [(2, Weapon::Medgun), (3, Weapon::RepairGun)] {
        let q = sim.player_mut(id).unwrap();
        q.loadout = loadout(ArmorKind::Light, Weapon::Rifle, w, Kit::None);
        q.refill();
    }
    let aim = |sim: &mut ContinentSim, id: PlayerId| {
        let from = sim.player(id).unwrap().eye();
        let to = sim.player(1).unwrap().feet() + Vec3::Y;
        let d = (to - from).normalize();
        sim.set_cmd(
            id,
            PlayerCmd {
                yaw: yaw_of(d),
                pitch: d.y.asin(),
                slot: 2,
                fire: true,
                ..Default::default()
            },
        );
    };
    for _ in 0..60 {
        aim(&mut sim, 2);
        sim.step();
    }
    assert_eq!(
        sim.player(1).unwrap().health,
        50.0,
        "the medgun passes it by"
    );
    sim.set_cmd(2, PlayerCmd::default());
    for _ in 0..120 {
        aim(&mut sim, 3);
        sim.step();
    }
    let p = sim.player(1).unwrap();
    assert!(p.armor > 140.0, "armour mended: {}", p.armor);
}

#[test]
fn a_cloak_hides_an_infiltrator_until_it_fires() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "spy", Team::Red, false);
    sim.add_player(2, "far", Team::Blue, false);
    sim.add_player(3, "near", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(2, at + Vec3::new(0.0, 0.0, 40.0), 0.0);
    sim.spawn_at(3, at + Vec3::new(5.0, 0.0, 0.0), 0.0);
    let p = sim.player_mut(1).unwrap();
    p.loadout = loadout(ArmorKind::Stealth, Weapon::Rifle, Weapon::Medgun, Kit::None);
    p.refill();
    // The deploy key cloaks.
    let mut cmd = PlayerCmd::default();
    sim.set_cmd(1, cmd);
    sim.step();
    cmd.deploys += 1;
    sim.set_cmd(1, cmd);
    sim.step();
    assert!(sim.player(1).unwrap().cloaked);
    let seen = |sim: &ContinentSim, by: PlayerId| {
        sim.snapshot_for(by, 0)
            .players
            .iter()
            .find(|p| p.id == 1)
            .map(|p| p.has(flags::CLOAKED))
    };
    assert_eq!(seen(&sim, 2), None, "hidden from afar");
    assert_eq!(seen(&sim, 3), Some(true), "faint up close");
    // It runs out.
    for _ in 0..(60.0 * (weapons::CLOAK_TIME + 1.0)) as usize {
        sim.step();
    }
    assert!(!sim.player(1).unwrap().cloaked);
    // Cloaked again, then firing drops it.
    for _ in 0..60 * 20 {
        sim.step();
    }
    cmd.deploys += 1;
    sim.set_cmd(1, cmd);
    sim.step();
    assert!(sim.player(1).unwrap().cloaked);
    cmd.fire = true;
    sim.set_cmd(1, cmd);
    sim.step();
    assert!(!sim.player(1).unwrap().cloaked);
    assert_eq!(seen(&sim, 2), Some(false));
}

#[test]
fn infiltrators_hack_enemy_terminals_and_turrets() {
    let (_, _, mut sim) = setup();
    let (b, t, term) = base_with_terminal(&mut sim, Team::Blue, TerminalKind::Equipment);
    sim.add_player(1, "spy", Team::Red, false);
    sim.spawn_at(1, term, 0.0);
    let what = HackTarget::Terminal {
        base: b as BaseIdx,
        index: t as u8,
    };
    // Not in a plain suit.
    sim.request(1, Request::Hack(what));
    assert!(sim.player(1).unwrap().hacking.is_none());
    let p = sim.player_mut(1).unwrap();
    p.loadout = loadout(ArmorKind::Stealth, Weapon::Rifle, Weapon::Medgun, Kit::None);
    p.refill();
    // An enemy terminal can't be used.
    let heavy = loadout(ArmorKind::Heavy, Weapon::Lance, Weapon::Striker, Kit::None);
    sim.request(1, Request::Loadout(heavy));
    assert_eq!(sim.player(1).unwrap().loadout.armor, ArmorKind::Stealth);
    sim.request(1, Request::Hack(what));
    assert!(sim.player(1).unwrap().hacking.is_some());
    for _ in 0..(60.0 * (weapons::HACK_TIME + 0.2)) as usize {
        sim.step();
    }
    assert_eq!(sim.terminal_team(b, t), Some(Team::Red));
    assert!(sim.snapshot_for(1, 0).bases[b].hacked & (1 << t) != 0);
    // Now it serves the hacker.
    sim.request(1, Request::Loadout(heavy));
    assert_eq!(sim.player(1).unwrap().loadout.armor, ArmorKind::Heavy);
    // The base changing hands clears it.
    sim.set_base(b as BaseIdx, Some(Team::Green), None, [false; 3]);
    assert_eq!(sim.terminal_team(b, t), Some(Team::Green));
    // A turret: moving away stops the hack; standing at it, it turns.
    let ti = (0..sim.turrets.len())
        .find(|&i| sim.turret_team(i) == Some(Team::Green))
        .expect("a turret of the base's");
    let p = sim.player_mut(1).unwrap();
    p.loadout = loadout(ArmorKind::Stealth, Weapon::Rifle, Weapon::Medgun, Kit::None);
    let tp = sim.turrets[ti].pos;
    let behind = sim.turrets[ti].pos + Vec3::new(0.0, 0.0, 0.0);
    sim.player_mut(1).unwrap().body.feet = behind;
    sim.request(1, Request::Hack(HackTarget::Turret(ti as u16)));
    assert!(sim.player(1).unwrap().hacking.is_some(), "near {tp}");
    for _ in 0..(60.0 * (weapons::HACK_TIME + 0.2)) as usize {
        sim.step();
        sim.player_mut(1).unwrap().body.feet = behind;
        sim.player_mut(1).unwrap().health = 100.0;
    }
    assert_eq!(sim.turret_team(ti), Some(Team::Red));
    assert_eq!(
        sim.snapshot_for(1, 0)
            .turrets
            .iter()
            .find(|s| s.index == sim.turrets[ti].index && s.base == sim.turrets[ti].base)
            .unwrap()
            .team,
        Some(Team::Red)
    );
}

#[test]
fn engineers_put_down_spitfires_mines_and_ammo() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "eng", Team::Red, false);
    sim.spawn_at(1, at, 0.0);
    let put = |sim: &mut ContinentSim, kit: Kit, yaw: f32| {
        let p = sim.player_mut(1).unwrap();
        p.loadout = loadout(ArmorKind::Medium, Weapon::Rifle, Weapon::RepairGun, kit);
        p.refill();
        let mut cmd = PlayerCmd {
            yaw,
            ..Default::default()
        };
        sim.set_cmd(1, cmd);
        sim.step();
        cmd.deploys += 1;
        sim.set_cmd(1, cmd);
        sim.step();
    };
    // A spitfire, facing south; an enemy walks up in front of it.
    put(&mut sim, Kit::Spitfire, std::f32::consts::PI);
    assert_eq!(sim.deployables.len(), 1);
    assert_eq!(sim.player(1).unwrap().kit_left, 0);
    let spit = sim.deployables[0].pos;
    sim.add_player(2, "x", Team::Blue, false);
    let ground = |sim: &ContinentSim, p: Vec3| {
        Vec3::new(p.x, sim.world.ground(p.x, p.z, p.y + 30.0).unwrap(), p.z)
    };
    let them = ground(&sim, spit + Vec3::new(0.0, 0.0, 18.0));
    sim.spawn_at(2, them, 0.0);
    sim.player_mut(2).unwrap().protect = 0.0;
    for _ in 0..60 * 4 {
        sim.step();
    }
    let hp = sim
        .player(2)
        .map_or(0.0, |p| if p.alive() { p.health } else { 0.0 });
    assert!(hp < 100.0, "the spitfire shot: {hp}");
    // Cloaked further than `CLOAK_SEEN`: it doesn't see one.
    sim.remove_player(2);
    sim.add_player(3, "spy", Team::Blue, false);
    sim.spawn_at(3, them, 0.0);
    let p = sim.player_mut(3).unwrap();
    p.protect = 0.0;
    p.loadout = loadout(ArmorKind::Stealth, Weapon::Rifle, Weapon::Medgun, Kit::None);
    p.refill();
    p.cloaked = true;
    for _ in 0..60 * 3 {
        sim.player_mut(3).unwrap().cloak = 1.0;
        sim.step();
    }
    assert_eq!(sim.player(3).unwrap().health, 100.0, "unseen");
    sim.remove_player(3);
    // Mines: three at most, the oldest going; an enemy tank over one is hurt.
    for k in 0..4 {
        put(&mut sim, Kit::Mines, k as f32 * 1.5);
    }
    let mines: Vec<Vec3> = sim
        .deployables
        .iter()
        .filter(|d| d.kind == Kit::Mines)
        .map(|d| d.pos)
        .collect();
    assert_eq!(mines.len(), 3);
    // (The engineer out of the blast.)
    let away = ground(&sim, at + Vec3::new(20.0, 0.0, 0.0));
    sim.player_mut(1).unwrap().body.feet = away;
    let tank = sim.add_vehicle(VehicleKind::LightTank, Team::Blue, mines[0], 0.0);
    sim.step();
    let health = sim.vehicle(tank).map_or(0.0, |v| v.health);
    assert!(health <= 100.0 - MINE_HIT, "the mine went off: {health}");
    // (Those under the hull went off; the mines lie close together.)
    assert!(
        sim.deployables
            .iter()
            .filter(|d| d.kind == Kit::Mines)
            .count()
            < 3
    );
    // An ammo pack fills a team mate's reserve.
    sim.remove_vehicle(tank);
    put(&mut sim, Kit::AmmoPack, 0.0);
    sim.add_player(4, "mate", Team::Red, false);
    let pack = sim
        .deployables
        .iter()
        .find(|d| d.kind == Kit::AmmoPack)
        .unwrap()
        .pos;
    sim.spawn_at(4, pack + Vec3::new(1.0, 0.0, 0.0), 0.0);
    sim.player_mut(4).unwrap().guns[0].reserve = 0;
    for _ in 0..60 * 2 {
        sim.step();
    }
    assert!(sim.player(4).unwrap().guns[0].reserve > 0);
    // Gone with their owner.
    sim.remove_player(1);
    assert!(sim.deployables.is_empty());
}

/// What a mine takes off a tank at least.
const MINE_HIT: f32 = 40.0;

/// An outpost of facility `f` (its index).
fn outpost_of(sim: &ContinentSim, f: crate::worldgen::Facility) -> usize {
    (0..sim.bases.len())
        .find(|&b| sim.map.def.facility(b as BaseIdx) == Some(f))
        .expect("an outpost of the kind")
}

#[test]
fn facilities_give_their_holders_benefits() {
    use crate::worldgen::Facility;
    let (_, _, mut sim) = setup();
    // Every outpost kind is on the continent.
    for f in Facility::ALL {
        outpost_of(&sim, f);
    }
    for b in 0..sim.bases.len() {
        sim.set_base(b as BaseIdx, Some(Team::Blue), None, [false; 3]);
    }
    let tech = outpost_of(&sim, Facility::TechPlant);
    let bio = outpost_of(&sim, Facility::BioLab);
    sim.set_base(tech as BaseIdx, Some(Team::Red), None, [false; 3]);
    assert!(sim.benefit(Team::Red, Facility::TechPlant));
    assert!(!sim.benefit(Team::Red, Facility::BioLab));
    assert_eq!(sim.respawn_time(Team::Red), RESPAWN_TIME);
    assert_eq!(sim.respawn_time(Team::Blue), RESPAWN_TIME * 0.5);
    // Its generator down: no benefit.
    sim.bases[tech].generator = 0.0;
    assert!(!sim.benefit(Team::Red, Facility::TechPlant));
    sim.bases[tech].generator = 100.0;
    // Advanced vehicles need the tech plant.
    sim.add_player(1, "a", Team::Red, false);
    sim.add_player(2, "b", Team::Blue, false);
    sim.bases[tech].resources = 1000.0;
    sim.bases[bio].resources = 1000.0;
    sim.spawn_at(1, sim.map.bases[tech].center, 0.0);
    assert!(sim.bot_vehicle(1, VehicleKind::HeavyTank, Some(tech), None));
    sim.set_base(tech as BaseIdx, Some(Team::Blue), None, [false; 3]);
    sim.set_base(bio as BaseIdx, Some(Team::Red), None, [false; 3]);
    sim.spawn_at(2, sim.map.bases[bio].center, 0.0);
    // (Blue holds the tech plant now; red, at the bio lab, has none.)
    sim.player_mut(2).unwrap().team = Team::Red;
    assert!(!sim.bot_vehicle(2, VehicleKind::Dropship, Some(bio), None));
    // Bio lab: the dead wait half as long.
    let i = sim.players.iter().position(|p| p.id == 1).unwrap();
    sim.kill(i, crate::types::HAZARD, crate::Cause::Fall, false);
    for _ in 0..(60.0 * RESPAWN_TIME * 0.55) as usize {
        sim.step();
    }
    assert!(sim.may_spawn(1));
}

#[test]
fn an_interlink_spots_enemies_near_its_bases() {
    use crate::worldgen::Facility;
    let (_, _, mut sim) = setup();
    for b in 0..sim.bases.len() {
        sim.set_base(b as BaseIdx, Some(Team::Blue), None, [false; 3]);
    }
    let link = outpost_of(&sim, Facility::Interlink);
    let other = outpost_of(&sim, Facility::BioLab);
    sim.add_player(1, "watcher", Team::Red, false);
    sim.add_player(2, "sneak", Team::Blue, false);
    let near = sim.map.bases[other].center;
    sim.spawn_at(1, near + Vec3::new(30.0, 0.0, 0.0), 0.0);
    sim.spawn_at(2, near, 0.0);
    let spotted = |sim: &ContinentSim| {
        sim.snapshot_for(1, 0)
            .players
            .iter()
            .find(|p| p.id == 2)
            .is_some_and(|p| p.has(flags::SPOTTED))
    };
    sim.set_base(other as BaseIdx, Some(Team::Red), None, [false; 3]);
    assert!(!spotted(&sim), "no interlink yet");
    sim.set_base(link as BaseIdx, Some(Team::Red), None, [false; 3]);
    assert!(spotted(&sim));
}

/// Somewhere open (out of the bases) to build `kind` on, with room around it.
fn build_site(sim: &ContinentSim, kind: crate::construct::StructureKind) -> Vec3 {
    for r in [500.0f32, 700.0, 900.0, 1100.0] {
        for k in 0..32 {
            let a = k as f32 / 32.0 * std::f32::consts::TAU;
            let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            let Some(y) = sim.world.ground(p.x, p.z, 500.0) else {
                continue;
            };
            let p = Vec3::new(p.x, y, p.z);
            let ok = |q: Vec3| sim.build_spot(kind, q, 0.0).is_some() && !sim.no_build(q);
            if [0.0, 6.0, -6.0, 12.0, -12.0, 20.0]
                .iter()
                .all(|dz| ok(p + Vec3::new(0.0, 0.0, *dz)))
                && ok(p + Vec3::new(8.0, 0.0, 0.0))
            {
                return p;
            }
        }
    }
    panic!("nowhere to build");
}

/// A red player (id 1) beside a red harvester (with a full load) at `at`: the harvester.
fn at_harvester(sim: &mut ContinentSim, at: Vec3) -> crate::types::VehicleId {
    let h = sim.add_vehicle(
        VehicleKind::Harvester,
        Team::Red,
        at + Vec3::new(8.0, 0.0, 0.0),
        0.0,
    );
    sim.vehicles.last_mut().unwrap().cargo = CARGO;
    sim.add_player(1, "builder", Team::Red, false);
    let half = VehicleKind::Harvester.spec().half;
    sim.spawn_at(1, at + Vec3::new(8.0 - half.x - 1.0, 0.0, 0.0), 0.0);
    h
}

#[test]
fn structures_come_from_a_harvesters_terminal_and_live_off_a_silo() {
    use crate::construct::{SILO_HOLDS, SILO_START, StructureKind};
    // Three harvester loads fill a mobile silo, six a base's.
    assert!((SILO_HOLDS / CARGO - 3.0).abs() < 1e-3);
    assert!((crate::state::SILO_CAP / CARGO - 6.0).abs() < 1e-3);
    let (_, _, mut sim) = setup();
    let site = build_site(&sim, StructureKind::Silo);
    let h = at_harvester(&mut sim, site);
    let build = |sim: &mut ContinentSim, kind, at| {
        sim.request(1, Request::Build { kind, at, yaw: 0.0 });
    };
    let pull = |sim: &mut ContinentSim, kind| {
        sim.request(1, Request::Pull { harvester: h, kind });
    };
    // Nothing to put down before pulling one.
    build(&mut sim, StructureKind::Silo, site);
    assert!(sim.structures.is_empty());
    pull(&mut sim, StructureKind::Silo);
    assert_eq!(sim.player(1).unwrap().carrying, Some(StructureKind::Silo));
    let cargo = sim.vehicle(h).unwrap().cargo;
    assert!((cargo - (CARGO - StructureKind::Silo.cost())).abs() < 1e-3);
    // One at a time.
    pull(&mut sim, StructureKind::Wall);
    assert_eq!(sim.vehicle(h).unwrap().cargo, cargo);
    // Not in a base.
    let base = sim.map.bases[0].center;
    build(&mut sim, StructureKind::Silo, base);
    assert!(sim.structures.is_empty());
    build(&mut sim, StructureKind::Silo, site);
    assert_eq!(sim.structures.len(), 1);
    assert_eq!(sim.structures[0].resources, SILO_START);
    assert_eq!(sim.player(1).unwrap().carrying, None);
    // A wall near it draws on it (the harvester driven off, not to unload into it).
    pull(&mut sim, StructureKind::Wall);
    build(
        &mut sim,
        StructureKind::Wall,
        site + Vec3::new(0.0, 0.0, -12.0),
    );
    assert_eq!(sim.structures.len(), 2);
    let away = site + Vec3::new(40.0, 0.0, 0.0);
    sim.vehicles
        .iter_mut()
        .find(|v| v.id == h)
        .unwrap()
        .motion
        .pos = away;
    let before = sim.structures[0].resources;
    for _ in 0..60 {
        sim.step();
    }
    assert!(sim.structures[0].resources < before, "drawn on");
    assert!(
        sim.structures
            .iter()
            .all(|s| s.supplied && s.health == 100.0)
    );
    // The harvester unloads into it (parked beside it).
    let had = sim.structures[0].resources;
    let hv = sim.vehicles.iter_mut().find(|v| v.id == h).unwrap();
    hv.motion.pos = site + Vec3::new(6.0, 0.0, 0.0);
    hv.motion.vel = Vec3::ZERO;
    for _ in 0..60 {
        sim.step();
    }
    assert!(sim.structures[0].resources > had + 10.0, "unloaded");
    // Kept over a restart.
    let kept = sim.saved_structures();
    let (_, _, mut again) = setup();
    again.restore_structures(kept.clone());
    assert_eq!(again.saved_structures(), kept);
    // Dry: a warning, and both decay till they fall.
    sim.vehicles.retain(|v| v.id != h);
    sim.structures[0].resources = 0.01;
    sim.out.clear();
    for _ in 0..30 {
        sim.step();
    }
    assert!(
        sim.out
            .iter()
            .any(|o| matches!(o.ev, SimEvent::SiloLow { empty: true, .. }))
    );
    assert!(
        sim.structures
            .iter()
            .all(|s| !s.supplied && s.health < 100.0)
    );
    for s in &mut sim.structures {
        s.health = 0.5;
    }
    for _ in 0..90 {
        sim.step();
    }
    assert!(sim.structures.is_empty(), "decayed away");
}

#[test]
fn walls_block_and_tubes_spawn() {
    use crate::construct::StructureKind;
    let (_, _, mut sim) = setup();
    let site = build_site(&sim, StructureKind::Wall);
    sim.add_player(1, "builder", Team::Red, false);
    sim.spawn_at(1, site + Vec3::new(0.0, 0.0, 8.0), 0.0);
    let carry_and_build = |sim: &mut ContinentSim, kind, at| {
        sim.player_mut(1).unwrap().carrying = Some(kind);
        sim.request(1, Request::Build { kind, at, yaw: 0.0 });
    };
    carry_and_build(&mut sim, StructureKind::Wall, site);
    assert_eq!(sim.structures.len(), 1);
    // It stops a body walking into it (from 8 m in front, walking north).
    sim.step();
    for _ in 0..60 * 4 {
        sim.set_cmd(
            1,
            PlayerCmd {
                stick: Vec2::new(0.0, -1.0),
                yaw: 0.0,
                ..Default::default()
            },
        );
        sim.step();
    }
    let z = sim.player(1).unwrap().feet().z;
    assert!(z > site.z + 0.4, "stopped at the wall: {z} vs {}", site.z);
    // And shots: an enemy's bolts wear it down, not who's behind it.
    sim.add_player(2, "foe", Team::Blue, false);
    sim.spawn_at(2, site - Vec3::new(0.0, 0.0, 10.0), std::f32::consts::PI);
    sim.player_mut(2).unwrap().protect = 0.0;
    let me_hp = sim.player(1).unwrap().health;
    for _ in 0..60 * 3 {
        let e = sim.player(2).unwrap().eye();
        let d = (site + Vec3::Y * 1.5 - e).normalize();
        sim.set_cmd(
            2,
            PlayerCmd {
                yaw: yaw_of(d),
                pitch: d.y.asin(),
                fire: true,
                ..Default::default()
            },
        );
        sim.step();
    }
    assert!(sim.structures[0].health < 100.0, "hit");
    assert_eq!(sim.player(1).unwrap().health, me_hp, "the wall took them");
    sim.remove_player(2);
    // A tube: dead team mates spawn at it.
    let tube_at = site + Vec3::new(0.0, 0.0, -12.0);
    sim.player_mut(1).unwrap().body.feet = site + Vec3::new(4.0, 0.0, -12.0);
    carry_and_build(&mut sim, StructureKind::Tube, tube_at);
    let tube = sim
        .structures
        .iter()
        .find(|s| s.kind == StructureKind::Tube)
        .expect("a tube")
        .id;
    sim.add_player(3, "mate", Team::Red, false);
    sim.player_mut(3).unwrap().dead = Some(RESPAWN_TIME);
    sim.request(3, Request::Spawn(SpawnPoint::Tube(tube)));
    let at = sim.player(3).unwrap();
    assert!(at.alive());
    assert!((at.feet() - tube_at).length() < 4.0);
}

#[test]
fn a_bastion_fires_on_enemies_and_engineers_mend_it() {
    use crate::construct::StructureKind;
    let (_, _, mut sim) = setup();
    let site = build_site(&sim, StructureKind::Bastion);
    sim.add_player(1, "eng", Team::Red, false);
    sim.spawn_at(1, site + Vec3::new(4.0, 0.0, 0.0), 0.0);
    let p = sim.player_mut(1).unwrap();
    p.loadout = loadout(
        ArmorKind::Medium,
        Weapon::Rifle,
        Weapon::RepairGun,
        Kit::None,
    );
    p.refill();
    p.carrying = Some(StructureKind::Bastion);
    sim.request(
        1,
        Request::Build {
            kind: StructureKind::Bastion,
            at: site,
            yaw: 0.0,
        },
    );
    assert_eq!(sim.structures.len(), 1);
    // An enemy 40 m off in the open.
    sim.add_player(2, "foe", Team::Blue, false);
    let far = site + Vec3::new(0.0, 0.0, 40.0);
    let far = Vec3::new(far.x, sim.world.ground(far.x, far.z, 500.0).unwrap(), far.z);
    sim.spawn_at(2, far, 0.0);
    sim.player_mut(2).unwrap().protect = 0.0;
    for _ in 0..60 * 4 {
        sim.step();
    }
    let hurt = sim.player(2).is_none_or(|p| !p.alive() || p.health < 100.0);
    assert!(hurt, "the bastion fired");
    // Mended by the engineer's repair gun.
    sim.structures[0].health = 50.0;
    for _ in 0..60 * 3 {
        let e = sim.player(1).unwrap().eye();
        let d = (site + Vec3::Y * 1.5 - e).normalize();
        sim.set_cmd(
            1,
            PlayerCmd {
                yaw: yaw_of(d),
                pitch: d.y.asin(),
                slot: 2,
                fire: true,
                ..Default::default()
            },
        );
        sim.step();
    }
    assert!(
        sim.structures[0].health > 60.0,
        "{}",
        sim.structures[0].health
    );
}

#[test]
fn spotting_marks_an_enemy_for_the_faction_and_pays_the_spotter() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    let ground = |sim: &ContinentSim, p: Vec3| {
        Vec3::new(p.x, sim.world.ground(p.x, p.z, p.y + 50.0).unwrap(), p.z)
    };
    sim.add_player(1, "spotter", Team::Red, false);
    sim.add_player(2, "mate", Team::Red, false);
    sim.add_player(3, "foe", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(2, ground(&sim, at + Vec3::new(5.0, 0.0, 0.0)), 0.0);
    let foe = ground(&sim, at + Vec3::new(0.0, 0.0, -60.0));
    sim.spawn_at(3, foe, 0.0);
    let flagged = |sim: &ContinentSim| {
        sim.snapshot_for(2, 0)
            .players
            .iter()
            .find(|p| p.id == 3)
            .is_some_and(|p| p.has(flags::SPOTTED))
    };
    assert!(!flagged(&sim));
    // Aiming at it (a little off) and spotting.
    let e = sim.player(1).unwrap().eye();
    let d = (foe + Vec3::new(0.8, 1.0, 0.0) - e).normalize();
    sim.set_cmd(
        1,
        PlayerCmd {
            yaw: yaw_of(d),
            pitch: d.y.asin(),
            ..Default::default()
        },
    );
    sim.step();
    sim.request(1, Request::Spot);
    assert!(flagged(&sim), "the team mate sees it spotted");
    // The team mate's kill pays the spotter.
    sim.awards.clear();
    let i = sim.players.iter().position(|p| p.id == 3).unwrap();
    sim.kill(i, 2, crate::Cause::Fall, false);
    assert!(
        sim.awards
            .iter()
            .any(|a| a.0 == 1 && a.1 == crate::progress::Award::Spot)
    );
    // It wears off.
    sim.spawn_at(3, foe, 0.0);
    sim.request(1, Request::Spot);
    for _ in 0..(60.0 * (spot::SPOT_TIME + 0.5)) as usize {
        sim.step();
    }
    assert!(!flagged(&sim));
}

#[test]
fn forcefields_stop_their_own_factions_shots() {
    let (_, _, mut sim) = setup();
    let (b, shield) = (0..sim.bases.len())
        .find_map(|b| sim.map.bases[b].shields.first().map(|s| (b, s.collider)))
        .expect("a base with a forcefield");
    sim.set_base(b as BaseIdx, Some(Team::Red), None, [false; 3]);
    let (lo, hi) = shield.bounds();
    let mid = (lo + hi) / 2.0;
    // From 4 m off its thin side, square on.
    let thin = if hi.x - lo.x < hi.z - lo.z {
        Vec3::X
    } else {
        Vec3::Z
    };
    let mut feet = mid + thin * 4.0;
    feet.y = sim
        .world
        .ground(feet.x, feet.z, mid.y + 2.0)
        .unwrap_or(lo.y);
    sim.add_player(1, "owner", Team::Red, false);
    sim.spawn_at(1, feet, 0.0);
    sim.player_mut(1).unwrap().protect = 0.0;
    let e = sim.player(1).unwrap().eye();
    let d = (Vec3::new(mid.x, e.y, mid.z) - e).normalize();
    sim.set_cmd(
        1,
        PlayerCmd {
            yaw: yaw_of(d),
            pitch: 0.0,
            fire: true,
            ..Default::default()
        },
    );
    let mut on_shield = false;
    for _ in 0..30 {
        sim.step();
        on_shield |= sim.fx.iter().any(|(_, f)| {
            matches!(
                f,
                crate::snapshot::Fx::Impact {
                    what: ImpactOn::Shield,
                    ..
                }
            )
        });
    }
    assert!(on_shield, "the owner's shot stopped at its own forcefield");
}

#[test]
fn emp_smoke_and_flash_grenades() {
    use crate::weapons::GrenadeKind;
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    let ground = |sim: &ContinentSim, p: Vec3| {
        Vec3::new(p.x, sim.world.ground(p.x, p.z, p.y + 50.0).unwrap(), p.z)
    };
    sim.add_player(1, "thrower", Team::Red, false);
    sim.add_player(2, "foe", Team::Blue, false);
    sim.spawn_at(1, at, 0.0);
    let foe = ground(&sim, at + Vec3::new(0.0, 0.0, -4.0));
    sim.spawn_at(2, foe, 0.0);
    let p = sim.player_mut(2).unwrap();
    p.protect = 0.0;
    p.loadout = loadout(ArmorKind::Stealth, Weapon::Rifle, Weapon::Medgun, Kit::None);
    p.refill();
    p.cloaked = true;
    // EMP: its armour gone, its cloak down and drained; no damage.
    sim.utility_burst(GrenadeKind::Emp, foe + Vec3::Y, 1, Team::Red);
    let q = sim.player(2).unwrap();
    assert_eq!(q.armor, 0.0);
    assert!(!q.cloaked && q.cloak == 0.0);
    assert_eq!(q.health, 100.0);
    // Flash: blinded (and told).
    sim.out.clear();
    sim.utility_burst(
        GrenadeKind::Flash,
        foe + Vec3::new(0.0, 0.5, 2.0),
        1,
        Team::Red,
    );
    assert!(sim.player(2).unwrap().flashed > 0.0);
    assert!(
        sim.out
            .iter()
            .any(|o| matches!(o.ev, SimEvent::Flashed { player: 2, .. }))
    );
    // Smoke: nobody sees through it, until it's gone.
    let (a, b) = (at + Vec3::Y * 1.6, foe + Vec3::Y * 1.6);
    assert!(!sim.smoked(a, b));
    sim.utility_burst(GrenadeKind::Smoke, (a + b) / 2.0, 1, Team::Red);
    assert!(sim.smoked(a, b));
    for _ in 0..(60.0 * (weapons::SMOKE_TIME + 0.5)) as usize {
        sim.step();
    }
    assert!(!sim.smoked(a, b));
    // Thrown, a smoke grenade flies as one and bounces.
    let p = sim.player_mut(1).unwrap();
    p.loadout.grenade = GrenadeKind::Smoke;
    p.refill();
    let mut cmd = PlayerCmd::default();
    sim.set_cmd(1, cmd);
    sim.step();
    cmd.throws += 1;
    sim.set_cmd(1, cmd);
    sim.step();
    assert!(
        sim.projectiles
            .iter()
            .any(|p| p.ammo == Ammo::Utility(GrenadeKind::Smoke))
    );
}

#[test]
fn light_armour_jets_and_bailing_out_falls_under_a_chute() {
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    sim.add_player(1, "jumper", Team::Red, false);
    sim.spawn_at(1, at, 0.0);
    let p = sim.player_mut(1).unwrap();
    p.loadout.armor = ArmorKind::Light;
    p.refill();
    p.protect = 0.0;
    // A jump, then the jets held: higher than a jump goes.
    let mut cmd = PlayerCmd::default();
    sim.set_cmd(1, cmd);
    sim.step();
    cmd.jumps += 1;
    cmd.jet = true;
    let mut top: f32 = 0.0;
    for _ in 0..90 {
        sim.set_cmd(1, cmd);
        sim.step();
        top = top.max(sim.player(1).unwrap().feet().y - at.y);
    }
    let jump_top = player::JUMP_SPEED * player::JUMP_SPEED / (2.0 * 9.81);
    assert!(
        top > jump_top + 1.5,
        "jetted to {top} m (a jump: {jump_top})"
    );
    assert!(sim.player(1).unwrap().body.fuel < 0.6);
    // Medium armour: no jets.
    for _ in 0..240 {
        sim.set_cmd(1, PlayerCmd::default());
        sim.step();
    }
    sim.player_mut(1).unwrap().loadout.armor = ArmorKind::Medium;
    let floor = sim.player(1).unwrap().feet().y;
    let mut cmd = sim.player(1).unwrap().cmd;
    cmd.jumps += 1;
    cmd.jet = true;
    let mut top: f32 = 0.0;
    for _ in 0..90 {
        sim.set_cmd(1, cmd);
        sim.step();
        top = top.max(sim.player(1).unwrap().feet().y - floor);
    }
    assert!(top < jump_top + 0.3, "no jets: {top}");
    // Out of a VTOL 80 m up: under a chute, landing unhurt.
    let vtol = sim.add_vehicle(VehicleKind::Vtol, Team::Red, at + Vec3::Y * 80.0, 0.0);
    sim.vehicles.last_mut().unwrap().motion.grounded = false;
    // (Healed of the jets' fall.)
    let p = sim.player_mut(1).unwrap();
    p.protect = 0.0;
    p.health = 100.0;
    assert!(sim.bot_board(1, vtol, 0));
    sim.request(1, Request::ExitVehicle);
    assert!(sim.player(1).unwrap().body.chute);
    sim.remove_vehicle(vtol);
    for _ in 0..60 * 25 {
        sim.set_cmd(1, PlayerCmd::default());
        sim.step();
    }
    let p = sim.player(1).unwrap();
    assert!(
        p.alive() && p.health == 100.0,
        "landed unhurt: {}",
        p.health
    );
    assert!(!p.body.chute && p.body.on_ground());
}

#[test]
fn vehicle_loadouts_choose_the_gun_and_the_defence() {
    use crate::vehicle::{Defense, VehicleLoadout};
    let (_, _, mut sim) = setup();
    let at = open_ground(&sim);
    let tank = sim.add_vehicle(VehicleKind::LightTank, Team::Red, at, 0.0);
    let vi = sim.vehicles.len() - 1;
    let auto = VehicleLoadout {
        weapon: 1,
        defense: Defense::FireSuppression,
    };
    assert!(VehicleKind::LightTank.loadout_valid(auto));
    assert!(!VehicleKind::LightTank.loadout_valid(VehicleLoadout {
        weapon: 1,
        defense: Defense::Flares,
    }));
    sim.vehicles[vi].load_out(auto);
    assert_eq!(sim.vehicles[vi].gun_spec(0).ammo, Ammo::Autocannon);
    assert_eq!(sim.vehicles[vi].guns[0].ammo, 160);
    // The driver fires the autocannon; the deploy key puts the fire out.
    sim.add_player(1, "driver", Team::Red, false);
    sim.spawn_at(1, at + Vec3::new(3.0, 0.0, 0.0), 0.0);
    assert!(sim.bot_board(1, tank, 0));
    sim.vehicles[vi].health = 40.0;
    let mut cmd = PlayerCmd::default();
    sim.set_cmd(1, cmd);
    sim.step();
    cmd.deploys += 1;
    cmd.fire = true;
    sim.set_cmd(1, cmd);
    sim.step();
    let v = sim.vehicle(tank).unwrap();
    assert!((v.health - (40.0 + crate::vehicle::FIRE_SUPPRESSION)).abs() < 1.0);
    assert!(v.flare_wait > 0.0);
    assert!(
        sim.projectiles
            .iter()
            .any(|p| p.ammo == Ammo::Autocannon && p.owner == 1)
    );
    // Plating: a fifth less damage.
    sim.vehicles[vi].load_out(VehicleLoadout {
        weapon: 0,
        defense: Defense::Plating,
    });
    sim.vehicles[vi].health = 100.0;
    sim.damage_vehicle(tank, 10.0, crate::types::HAZARD);
    assert!((sim.vehicle(tank).unwrap().health - 92.0).abs() < 1e-3);
}

/// A capture point base on a continent of some world, with an enemy allowed to take it.
fn points_setup() -> (ContinentSim, BaseIdx, Team, Team) {
    for seed in 0..40 {
        let def = WorldDef::generate(WorldParams { seed, contested: 6 });
        let st = WorldState::initial(&def);
        for cd in &def.continents {
            for (b, site) in cd.bases.iter().enumerate() {
                let b = b as BaseIdx;
                let owner = st.base(cd.id, b).owner;
                let Some(enemy) = Team::ALL
                    .into_iter()
                    .find(|&t| owner.is_some() && st.can_hack(&def, cd.id, b, t))
                else {
                    continue;
                };
                if site.capture != crate::worldgen::Capture::Points {
                    continue;
                }
                let map = Arc::new(ContinentMap::build(Arc::new(cd.clone()), false));
                let mut sim = ContinentSim::new(cd.id, map);
                sync(&def, &st, &mut sim);
                return (sim, b, owner.unwrap(), enemy);
            }
        }
    }
    panic!("no capture point base to take");
}

#[test]
fn standing_on_two_points_reports_a_majority_and_retaking_one_ends_it() {
    let (mut sim, b, owner, enemy) = points_setup();
    let pts = sim.map.bases[b as usize].points.clone();
    assert_eq!(pts.len(), 3);
    assert!(
        sim.bases[b as usize]
            .points
            .iter()
            .all(|p| p.owner == Some(owner))
    );
    // (The first step tells the server nobody holds two.)
    // (Its turrets down: nobody shoots.)
    sim.bases[b as usize].generator = 0.0;
    let told = |sim: &mut ContinentSim| -> Vec<Option<Team>> {
        std::mem::take(&mut sim.majority)
            .into_iter()
            .filter(|m| m.0 == b)
            .map(|m| m.1)
            .collect()
    };
    sim.step();
    assert_eq!(told(&mut sim), vec![None]);
    sim.add_player(1, "a", enemy, false);
    sim.add_player(2, "b", enemy, false);
    sim.spawn_at(1, pts[0].stand, 0.0);
    sim.spawn_at(2, pts[2].stand, 0.0);
    let secs = |s: f32| (s / TICK) as usize;
    for _ in 0..secs(crate::capture::POINT_TIME + 1.0) {
        sim.step();
    }
    let rt = &sim.bases[b as usize];
    assert_eq!(rt.points[0].owner, Some(enemy));
    assert_eq!(rt.points[2].owner, Some(enemy));
    assert_eq!(rt.points[1].owner, Some(owner));
    assert_eq!(told(&mut sim), vec![Some(enemy)]);
    assert_eq!(sim.player(1).unwrap().score, progress::CAPTURE_XP);
    let snap = sim.bases[b as usize].points[0];
    assert_eq!(snap.by, None);
    // The owners take A back (its taker gone): the majority is gone.
    sim.add_player(3, "c", owner, false);
    sim.spawn_at(
        1,
        sim.map.bases[b as usize].center + Vec3::new(0.0, 0.0, 300.0),
        0.0,
    );
    sim.spawn_at(3, pts[0].stand, 0.0);
    for _ in 0..secs(crate::capture::POINT_TIME + 1.0) {
        sim.step();
    }
    assert_eq!(sim.bases[b as usize].points[0].owner, Some(owner));
    assert_eq!(told(&mut sim), vec![None]);
    // The base changing hands: its points are all its new owner's.
    sim.set_base(b, Some(enemy), None, [false; 3]);
    assert!(
        sim.bases[b as usize]
            .points
            .iter()
            .all(|p| p.owner == Some(enemy))
    );
}

#[test]
fn ziplines_carry_riders_up_and_jumping_lets_go() {
    let (_, _, mut sim) = setup_on(true);
    assert!(!sim.map.caves.lines.is_empty(), "no ziplines in the caves");
    let z = sim.map.caves.lines[0];
    sim.add_player(1, "a", Team::Red, false);
    // On the roof at the bottom end, looking along it, out of reach: refused.
    let yaw = crate::types::yaw_of(z.dir());
    sim.spawn_at(1, z.land_a + Vec3::new(0.0, 0.0, 0.0), yaw);
    sim.players[0].protect = 0.0;
    let far = z.land_a - z.dir().with_y(0.0).normalize() * 6.0;
    sim.spawn_at(1, Vec3::new(far.x, z.land_a.y, far.z), yaw);
    sim.request(1, Request::Zip(0));
    assert!(sim.player(1).unwrap().zip.is_none());
    sim.spawn_at(1, z.land_a, yaw);
    sim.request(1, Request::Zip(0));
    assert!(sim.player(1).unwrap().zip.is_some(), "didn't grab it");
    // Up it and put down on the rim, unhurt.
    let mut secs = 0.0;
    while sim.player(1).unwrap().zip.is_some() && secs < 30.0 {
        sim.step();
        secs += TICK;
        let p = sim.player(1).unwrap();
        if let Some(r) = p.zip {
            let want = z.at(r.s) - Vec3::Y * crate::zip::ZIP_HANG;
            assert!((p.feet() - want).length() < 0.01);
        }
    }
    let p = sim.player(1).unwrap();
    assert!(p.zip.is_none() && secs < 20.0, "{secs} s");
    assert!(
        (p.feet() - z.land_b).length() < 0.5,
        "{} vs {}",
        p.feet(),
        z.land_b
    );
    for _ in 0..60 {
        sim.step();
    }
    let p = sim.player(1).unwrap();
    assert!(p.alive() && p.health == 100.0);
    assert!((p.feet() - z.land_b).length() < 0.5);
    // Back down; halfway, a jump lets go and it falls.
    sim.request(1, Request::Zip(0));
    assert!(sim.player(1).unwrap().zip.is_some_and(|r| r.sign < 0.0));
    for _ in 0..(z.len() / 2.0 / crate::zip::ZIP_SPEED / TICK) as usize {
        sim.step();
    }
    let mid = sim.player(1).unwrap().feet();
    let mut cmd = sim.player(1).unwrap().cmd;
    cmd.jumps += 1;
    sim.set_cmd(1, cmd);
    sim.step();
    let p = sim.player(1).unwrap();
    assert!(p.zip.is_none());
    for _ in 0..30 {
        sim.step();
    }
    assert!(sim.player(1).unwrap().feet().y < mid.y - 0.5);
}
