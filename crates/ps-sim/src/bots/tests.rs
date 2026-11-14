use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::base::STOREY;
use crate::map::ContinentMap;
use crate::state::WorldState;
use crate::worldgen::{BaseKind, WorldDef, WorldParams};
use crate::{SimEvent, snapshot::Fx, weapons::Ammo};

/// A front continent with its bases' state from the world's.
fn setup(seed: u64) -> (WorldDef, WorldState, ContinentSim) {
    // (On the open land these were made for.)
    let def = WorldDef::generate(WorldParams { seed, contested: 6 }).without_caves();
    let st = WorldState::initial(&def);
    let c = (3..def.continents.len() as u8)
        .find(|&c| st.continents[c as usize].owner.is_some() && st.contested(&def, c))
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

/// What a run of bots did.
#[derive(Debug, Default)]
struct Report {
    /// Bot-seconds spent trying to walk (not fighting) without getting 2 m in 5 s, by where.
    stuck: HashMap<&'static str, u32>,
    /// Bot-seconds trying to walk for 20 s without getting 6 m from where they were.
    loiter: u32,
    /// Bot-seconds on foot trying to walk.
    walking: u32,
    /// Samples (once a second) of two bots of a faction on foot within 1 m of each other.
    bunched: u32,
    /// Infantry shots (pellets) and hits on infantry, by weapon.
    shots: HashMap<String, u32>,
    hits: HashMap<String, u32>,
    kills: u32,
    hacks: u32,
}

impl Report {
    fn stuck_total(&self) -> u32 {
        self.stuck.values().sum()
    }
}

fn where_is(sim: &ContinentSim, p: Vec3) -> &'static str {
    for b in &sim.map.bases {
        if b.in_building(p) {
            return if b.kind == BaseKind::Tower {
                "tower"
            } else {
                "building"
            };
        }
    }
    for b in &sim.map.bases {
        if b.kind != BaseKind::Tower && b.inside(p) {
            return "compound";
        }
    }
    if sim
        .map
        .bases
        .iter()
        .any(|b| (b.center - p).length() < b.half + 30.0)
    {
        return "near base";
    }
    "open"
}

/// Runs `per_team` bots a faction for `secs` on the front continent of world `seed`.
fn run(seed: u64, per_team: usize, secs: u32, skill: f32, verbose: bool) -> Report {
    let (def, mut st, mut sim) = setup(seed);
    let factions = st.factions_on(&def, sim.id);
    let mut bots = Bots::new(skill, seed);
    let mut id = 10;
    for &t in &factions {
        for _ in 0..per_team {
            bots.add(&mut sim, id, t);
            id += 1;
        }
    }
    let mut r = Report::default();
    // A bot's position each second, and how many of the last seconds it tried to walk.
    let mut trail: HashMap<PlayerId, (Vec<Vec3>, u32)> = HashMap::new();
    let mut trying: HashMap<PlayerId, u32> = HashMap::new();
    let mut fighting: HashMap<PlayerId, u32> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut spots: HashMap<(i32, i32, i32), u32> = HashMap::new();
    for tick in 0..60 * secs as u64 {
        bots.think(&mut sim);
        sim.step();
        for o in std::mem::take(&mut sim.out) {
            match o.ev {
                SimEvent::Killed { .. } => r.kills += 1,
                SimEvent::HitConfirm {
                    by, vehicle: false, ..
                } => {
                    if let Some(p) = sim.player(by).filter(|p| p.afoot()) {
                        *r.hits.entry(format!("{:?}", p.weapon())).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
        for (b, team, _) in std::mem::take(&mut sim.solved) {
            if !st.console_solved(&def, sim.id, b, team).is_empty() {
                r.hacks += 1;
            }
        }
        for (_, f) in &sim.fx {
            if let Fx::Shot {
                id, owner, ammo, ..
            } = f
                && seen.insert(*id)
                && let Ammo::Gun(w) = ammo
                && sim.player(*owner).is_some_and(|p| p.afoot())
            {
                *r.shots.entry(format!("{w:?}")).or_default() += 1;
            }
        }
        for b in &bots.bots {
            let Some(p) = sim.player(b.id) else { continue };
            if p.afoot() && p.cmd.stick.length() > 0.1 {
                *trying.entry(b.id).or_default() += 1;
            }
            if b.target.is_some() {
                *fighting.entry(b.id).or_default() += 1;
            }
        }
        if tick % 60 == 59 {
            for b in &bots.bots {
                let Some(p) = sim.player(b.id) else { continue };
                let t = trying.remove(&b.id).unwrap_or(0);
                let f = fighting.remove(&b.id).unwrap_or(0);
                let e = trail.entry(b.id).or_default();
                if !p.afoot() || t < 54 || f > 0 {
                    *e = (Vec::new(), 0);
                    continue;
                }
                r.walking += 1;
                e.0.push(p.feet());
                e.1 += 1;
                if e.0.len() > 21 {
                    e.0.remove(0);
                }
                if e.0.len() == 21 {
                    let from = e.0[0];
                    if e.0.iter().all(|q| (*q - from).length() < 6.0) {
                        r.loiter += 1;
                        if verbose && r.loiter % 5 == 1 {
                            println!(
                                "{} loitering at {:.1} ({}) goal {:?} walk {:?} stick {:?}",
                                b.id,
                                p.feet(),
                                where_is(&sim, p.feet()),
                                b.goal,
                                b.walk.as_ref().map(|w| (w.base, w.inward, w.next)),
                                p.cmd.stick
                            );
                        }
                    }
                }
                if e.0.len() >= 6 {
                    let last = &e.0[e.0.len() - 6..];
                    let from = last[0];
                    let far = last
                        .iter()
                        .map(|q| (*q - from).length())
                        .fold(0.0, f32::max);
                    if far < 2.0 {
                        *r.stuck.entry(where_is(&sim, p.feet())).or_default() += 1;
                        let f = p.feet();
                        *spots
                            .entry(((f.x / 4.0) as i32, (f.y / 4.0) as i32, (f.z / 4.0) as i32))
                            .or_default() += 1;
                        if verbose && e.1 % 10 == 6 {
                            let base = sim
                                .map
                                .bases
                                .iter()
                                .min_by(|a, c| {
                                    (a.center - f).length().total_cmp(&(c.center - f).length())
                                })
                                .unwrap();
                            println!(
                                "{} stuck at {:.1} ({}; {:?} {:.1} from its center) goal {:?} path {:?}",
                                b.id,
                                f,
                                where_is(&sim, f),
                                base.kind,
                                f - base.center,
                                b.goal,
                                b.walk.as_ref().map(|w| (w.base, w.inward, w.next))
                            );
                        }
                    }
                }
            }
            let afoot: Vec<(Team, Vec3)> = bots
                .bots
                .iter()
                .filter_map(|b| sim.player(b.id))
                .filter(|p| p.afoot())
                .map(|p| (p.team, p.feet()))
                .collect();
            for (i, a) in afoot.iter().enumerate() {
                for c in &afoot[i + 1..] {
                    if a.0 == c.0 && (a.1 - c.1).length() < 1.0 {
                        r.bunched += 1;
                    }
                }
            }
        }
        if tick % 60 == 0 {
            st.tick_hacks(&def, sim.id, 1.0);
            sync(&def, &st, &mut sim);
        }
    }
    if verbose {
        let mut s: Vec<_> = spots.into_iter().collect();
        s.sort_by_key(|x| std::cmp::Reverse(x.1));
        println!("worst spots (4 m cells): {:?}", &s[..s.len().min(10)]);
    }
    r
}

/// The harness: bots on a front for a few minutes; how long they stand stuck, how they bunch,
/// how well they shoot. `PS_BOT_HARNESS=secs` for a longer run with details, `PS_BOT_SEEDS=a,b`
/// for other worlds.
#[test]
fn bots_dont_stand_stuck() {
    let secs = std::env::var("PS_BOT_HARNESS")
        .ok()
        .and_then(|s| s.parse().ok());
    let verbose = secs.is_some();
    let mut total = Report::default();
    let seeds: Vec<u64> = std::env::var("PS_BOT_SEEDS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or(vec![11, 23]);
    for seed in seeds {
        let r = run(seed, 8, secs.unwrap_or(300), 0.6, verbose);
        println!(
            "seed {seed}: stuck {} (20 s: {}) of {} walking bot-s {:?}, bunched {}, kills {}, hacks {}",
            r.stuck_total(),
            r.loiter,
            r.walking,
            r.stuck,
            r.bunched,
            r.kills,
            r.hacks
        );
        println!("  shots {:?} hits {:?}", r.shots, r.hits);
        total.walking += r.walking;
        total.loiter += r.loiter;
        total.bunched += r.bunched;
        for (k, v) in r.stuck {
            *total.stuck.entry(k).or_default() += v;
        }
    }
    let share = total.stuck_total() as f32 / total.walking.max(1) as f32;
    println!("stuck {:.1}% of walking time", share * 100.0);
    // (Before bots found their way about towers: 11% stuck, 23% without 6 m of headway in
    // 20 s, bunched 1800 times in 15 minutes.)
    assert!(share < 0.01, "stuck {:.1}% of the time", share * 100.0);
    assert!(total.loiter * 100 < total.walking, "{total:?}");
    assert!(total.bunched * 50 < total.walking, "{total:?}");
}

/// A continent where every base is `team`'s (no turrets firing, nobody else): a tower on it.
fn quiet(team: Team) -> (ContinentSim, BaseIdx) {
    let (_, _, mut sim) = setup(11);
    for b in 0..sim.bases.len() {
        sim.set_base(b as BaseIdx, Some(team), None, [false; 3]);
    }
    let t = sim
        .map
        .bases
        .iter()
        .position(|l| l.kind == BaseKind::Tower)
        .expect("a tower") as BaseIdx;
    (sim, t)
}

/// Red bots placed at `at`, going for `goal`, for `secs`.
fn walk_bots(sim: &mut ContinentSim, at: &[Vec3], goal: Goal, secs: u32) -> Bots {
    let mut bots = Bots::new(0.6, 3);
    for (i, &p) in at.iter().enumerate() {
        let id = 50 + i as PlayerId;
        bots.add(sim, id, Team::Red);
        sim.spawn_at(id, p, 0.0);
    }
    for _ in 0..60 * secs {
        for b in &mut bots.bots {
            b.goal = Some(goal);
            b.rethink = 1e6;
        }
        bots.think(sim);
        sim.step();
    }
    bots
}

#[test]
fn bots_find_their_way_out_of_a_tower() {
    let (mut sim, t) = quiet(Team::Red);
    let l = sim.map.bases[t as usize].clone();
    let (console, _) = l.console.unwrap();
    // In the corners of the top floor, the first floor and the bunker, going far off.
    let up = console.y - l.center.y;
    let at = [
        l.center + Vec3::new(7.0, up, -7.0),
        l.center + Vec3::new(-7.0, up, 7.0),
        l.center + Vec3::new(7.0, up / 2.0, -7.0),
        l.spawns[0].0,
    ];
    let away = l.center + Vec3::new(120.0, 0.0, 0.0);
    let bots = walk_bots(&mut sim, &at, Goal::Go(away), 40);
    for b in &bots.bots {
        let p = sim.player(b.id).unwrap().feet();
        assert!(
            !l.in_building(p),
            "{} still in the tower at {}",
            b.id,
            p - l.center
        );
        assert!(
            (p - l.center).length() > 20.0,
            "{} at {}",
            b.id,
            p - l.center
        );
    }
}

#[test]
fn a_squad_goes_up_a_tower_to_its_console_and_spreads_out() {
    let (mut sim, t) = quiet(Team::Red);
    // Hacked by another faction: Red goes to stop it, from the tower's bunker.
    let hack = crate::state::Hack {
        team: Team::Blue,
        left: 300.0,
    };
    sim.set_base(t, Some(Team::Red), Some(hack), [false; 3]);
    let l = sim.map.bases[t as usize].clone();
    let (console, _) = l.console.unwrap();
    let at: Vec<Vec3> = l.spawns.iter().map(|s| s.0).collect();
    let bots = walk_bots(&mut sim, &at, Goal::Console(t), 40);
    let feet: Vec<Vec3> = bots
        .bots
        .iter()
        .map(|b| sim.player(b.id).unwrap().feet())
        .collect();
    assert!(
        feet.iter().any(|p| (*p - console).length() < USE_REACH),
        "nobody at the console: {:?}",
        feet.iter().map(|p| *p - l.center).collect::<Vec<_>>()
    );
    // All upstairs, none on top of another.
    for (i, a) in feet.iter().enumerate() {
        assert!(a.y > console.y - STOREY - 0.5, "{} still low", a - l.center);
        for b in &feet[i + 1..] {
            assert!((*a - *b).length() > 1.0, "bunched at {}", a - l.center);
        }
    }
}

#[test]
fn hit_chances_want_reach_a_clear_line_and_the_aim_on_it() {
    use crate::collide::{Collider, World};
    // A wall 30 m off along +X.
    let wall = Collider::Box {
        min: Vec3::new(29.0, -5.0, -5.0),
        max: Vec3::new(30.0, 5.0, 5.0),
    };
    let w = World::new(vec![wall], None);
    let eye = Vec3::ZERO;
    let r = BODY_HIT_RADIUS;
    let chance = |gun: Weapon, at: Vec3, aim: Vec3| {
        hit_chance(
            &w,
            gun,
            // (Aimed down the sights, as bots fight past close quarters.)
            gun.spread_for(true, false, 0.0, false),
            eye,
            aim.normalize(),
            at,
            r,
            reach(gun),
            1.0,
        )
    };
    let north = |d: f32| Vec3::new(0.0, 0.0, -d);
    // On it, near: likely; far past its reach: never.
    assert!(chance(Weapon::Rifle, north(20.0), north(1.0)) > 0.9);
    assert_eq!(chance(Weapon::Rifle, north(200.0), north(1.0)), 0.0);
    assert!(chance(Weapon::Lance, north(200.0), north(1.0)) > 0.9);
    // The scatter gun only up close.
    assert!(chance(Weapon::Scatter, north(8.0), north(1.0)) > 0.5);
    assert_eq!(chance(Weapon::Scatter, north(60.0), north(1.0)), 0.0);
    // Not through a wall.
    let behind = Vec3::new(40.0, 0.0, 0.0);
    assert_eq!(chance(Weapon::Rifle, behind, behind), 0.0);
    // Not aimed a few degrees off.
    let off = Vec3::new(0.08, 0.0, -1.0);
    assert_eq!(chance(Weapon::Rifle, north(30.0), off), 0.0);
    // Clumsy bots fire at worse odds than sharp ones.
    let (easy, easy_judge) = discipline(Difficulty::Easy.skill());
    let (hard, hard_judge) = discipline(Difficulty::Hard.skill());
    assert!(easy < hard && easy_judge < hard_judge);
    // (Aimed just off the edge of the spread's reach to it, as a sharp bot sees it.)
    let edge = (r / 30.0).atan() + Weapon::Rifle.stats().spread * 0.95;
    let a = edge / hard_judge;
    let slightly = Vec3::new(a.sin(), 0.0, -a.cos());
    let judged = |judge: f32| {
        hit_chance(
            &w,
            Weapon::Rifle,
            Weapon::Rifle.stats().spread,
            eye,
            slightly,
            north(30.0),
            r,
            120.0,
            judge,
        )
    };
    assert!(judged(easy_judge) >= easy && judged(hard_judge) < hard);
}

/// Somewhere in the open, far from every base.
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
fn bots_hold_fire_out_of_reach_and_close_in() {
    let (mut sim, _) = quiet(Team::Red);
    let at = open_ground(&sim);
    // A bot with a scatter gun, and an enemy standing 90 m off (past its pistol's reach too).
    let mut bots = Bots::new(0.6, 3);
    bots.add(&mut sim, 50, Team::Red);
    sim.add_player(60, "x", Team::Blue, false);
    sim.player_mut(50).unwrap().loadout.rifle = Weapon::Scatter;
    let ground = |sim: &ContinentSim, p: Vec3| {
        Vec3::new(p.x, sim.world.ground(p.x, p.z, p.y + 50.0).unwrap(), p.z)
    };
    sim.spawn_at(50, at, 0.0);
    let far = ground(&sim, at + Vec3::new(0.0, 0.0, -90.0));
    sim.spawn_at(60, far, 0.0);
    for p in &mut sim.players {
        p.protect = 0.0;
    }
    // (In sight of each other on this ground.)
    let eye = |sim: &ContinentSim, id| sim.player(id).unwrap().eye();
    assert!(sim.world.clear(eye(&sim, 50), eye(&sim, 60)));
    let mut seen = std::collections::HashSet::new();
    let mut fired = Vec::new();
    let mut nearest = f32::MAX;
    for _ in 0..60 * 30 {
        bots.bots[0].goal = Some(Goal::Go(at));
        bots.bots[0].rethink = 1e6;
        bots.think(&mut sim);
        sim.step();
        let (me, them) = (sim.player(50).unwrap(), sim.player(60).unwrap());
        if !them.alive() {
            break;
        }
        let d = (them.eye() - me.eye()).length();
        nearest = nearest.min(d);
        for (_, f) in &sim.fx {
            if let Fx::Shot {
                id,
                owner: 50,
                ammo,
                ..
            } = f
                && seen.insert(*id)
            {
                fired.push((*ammo, d));
            }
        }
    }
    assert!(
        nearest < reach(Weapon::Scatter),
        "didn't close in: {nearest}"
    );
    assert!(!fired.is_empty(), "never fired");
    for (ammo, d) in fired {
        match ammo {
            Ammo::Gun(Weapon::Scatter) => assert!(d < reach(Weapon::Scatter) + 2.0, "{d}"),
            Ammo::Gun(Weapon::Pistol) => assert!(d < reach(Weapon::Pistol) + 2.0, "{d}"),
            other => panic!("fired {other:?} at a body"),
        }
    }
}

#[test]
fn rockets_go_at_armour_not_bodies() {
    let (mut sim, _) = quiet(Team::Red);
    let at = open_ground(&sim);
    let eye = at + Vec3::Y * 1.65;
    sim.add_player(60, "x", Team::Blue, false);
    sim.spawn_at(60, at + Vec3::new(0.0, 0.0, -50.0), 0.0);
    assert!(armour_near(&sim, Team::Red, eye).is_none());
    let tank = at + Vec3::new(80.0, 0.0, 0.0);
    let tank = Vec3::new(
        tank.x,
        sim.world.ground(tank.x, tank.z, 500.0).unwrap(),
        tank.z,
    );
    sim.vehicles.push(crate::sim::Vehicle::new(
        900,
        VehicleKind::LightTank,
        Team::Blue,
        tank,
        0.0,
    ));
    let (p, _, size) = armour_near(&sim, Team::Red, eye).expect("the tank");
    assert!((p - tank).length() < 2.0 && size > 1.0);
    // Too far for rockets.
    sim.vehicles[0].motion.pos = at + Vec3::new(400.0, 0.0, 0.0);
    assert!(armour_near(&sim, Team::Red, eye).is_none());
}

#[test]
fn bot_medics_revive_the_downed_near_them() {
    let (_, _, mut sim) = setup(11);
    // In the open, away from every base.
    let at = (0..64)
        .map(|k| {
            let a = k as f32 / 64.0 * std::f32::consts::TAU;
            Vec3::new(a.cos() * 700.0, 0.0, a.sin() * 700.0)
        })
        .find(|p| {
            sim.map
                .bases
                .iter()
                .all(|b| (b.center - *p).length() > 350.0)
        })
        .map(|p| Vec3::new(p.x, sim.world.ground(p.x, p.z, 500.0).unwrap(), p.z))
        .unwrap();
    let mut bots = Bots::new(0.6, 5);
    bots.add(&mut sim, 10, Team::Red);
    sim.spawn_at(10, at, 0.0);
    let medic = sim.player_mut(10).unwrap();
    medic.loadout.special = Weapon::Medgun;
    medic.refill();
    sim.add_player(2, "fallen", Team::Red, false);
    sim.spawn_at(2, at + Vec3::new(9.0, 0.0, 6.0), 0.0);
    let i = sim.players.iter().position(|p| p.id == 2).unwrap();
    sim.kill(i, crate::types::HAZARD, crate::Cause::Fall, false);
    let mut t = 0.0;
    while !sim.player(2).unwrap().alive() && t < 20.0 {
        bots.think(&mut sim);
        sim.step();
        t += crate::sim::TICK;
    }
    assert!(sim.player(2).unwrap().alive(), "revived in {t:.1} s");
}

#[test]
fn bot_interceptors_lock_on_to_aircraft_and_fire_missiles() {
    let (mut sim, _) = quiet(Team::Red);
    let at = open_ground(&sim) + Vec3::Y * 120.0;
    let mut bots = Bots::new(0.8, 7);
    bots.add(&mut sim, 50, Team::Red);
    sim.spawn_at(50, at, 0.0);
    let mine = sim.add_vehicle(VehicleKind::Interceptor, Team::Red, at, 0.0);
    sim.vehicles.last_mut().unwrap().motion.grounded = false;
    assert!(sim.bot_board(50, mine, 0));
    // An enemy VTOL hanging 300 m off, a little to one side.
    let them = sim.add_vehicle(
        VehicleKind::Vtol,
        Team::Blue,
        at + Vec3::new(80.0, 10.0, -300.0),
        0.0,
    );
    let hold = sim.vehicle(them).unwrap().motion.pos;
    let mut fired = false;
    for _ in 0..60 * 20 {
        if let Some(v) = sim.vehicles.iter_mut().find(|v| v.id == them) {
            v.motion.pos = hold;
            v.motion.vel = Vec3::ZERO;
            v.motion.grounded = false;
        }
        bots.think(&mut sim);
        sim.step();
        if sim
            .projectiles
            .iter()
            .any(|p| p.owner == 50 && p.homing == Some(them))
        {
            fired = true;
            break;
        }
    }
    assert!(fired, "no missile at the VTOL");
}

#[test]
fn bots_shoot_enemy_structures_with_nobody_to_fight() {
    use crate::construct::StructureKind;
    let (mut sim, _) = quiet(Team::Red);
    let at = open_ground(&sim);
    let mut bots = Bots::new(0.8, 5);
    bots.add(&mut sim, 50, Team::Red);
    sim.spawn_at(50, at, 0.0);
    // A blue wall 25 m off.
    let wall = at + Vec3::new(0.0, 0.0, -25.0);
    let wall = Vec3::new(
        wall.x,
        sim.world.ground(wall.x, wall.z, 500.0).unwrap(),
        wall.z,
    );
    sim.restore_structures(vec![crate::construct::SavedStructure {
        kind: StructureKind::Wall,
        team: Team::Blue,
        pos: wall.to_array(),
        yaw: 0.0,
        health: 100.0,
        resources: 0.0,
    }]);
    for _ in 0..60 * 8 {
        bots.bots[0].goal = Some(Goal::Go(at));
        bots.bots[0].rethink = 1e6;
        bots.think(&mut sim);
        sim.step();
    }
    // (Unsupplied it decays too: well under what decay alone takes.)
    let h = sim.structures.first().map_or(0.0, |s| s.health);
    assert!(
        h < 100.0 - crate::construct::DECAY * 8.0 - 2.0,
        "shot at: {h}"
    );
}

#[test]
fn infiltrator_bots_hack_enemy_terminals() {
    let (_, _, mut sim) = setup(11);
    let (b, t) = sim
        .map
        .bases
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind != BaseKind::Home && l.kind != BaseKind::Tower)
        .find_map(|(b, l)| {
            l.terminals
                .iter()
                .position(|t| t.kind == crate::base::TerminalKind::Equipment)
                .map(|t| (b, t))
        })
        .unwrap();
    sim.set_base(b as BaseIdx, Some(Team::Blue), None, [false; 3]);
    let term = sim.map.bases[b].terminals[t].pos;
    let mut bots = Bots::new(0.8, 3);
    bots.add(&mut sim, 50, Team::Red);
    let p = sim.player_mut(50).unwrap();
    p.loadout.armor = ArmorKind::Stealth;
    p.refill();
    sim.spawn_at(50, term + Vec3::new(1.0, 0.0, 1.0), 0.0);
    for _ in 0..60 * 8 {
        bots.bots[0].goal = Some(Goal::Hold(b as BaseIdx));
        bots.bots[0].rethink = 1e6;
        bots.think(&mut sim);
        sim.step();
    }
    let hacked =
        (0..sim.map.bases[b].terminals.len()).any(|i| sim.terminal_team(b, i) == Some(Team::Red));
    assert!(hacked, "a terminal hacked");
}

#[test]
fn bots_in_a_humans_squad_go_to_its_waypoint() {
    let (mut sim, _) = quiet(Team::Red);
    let at = open_ground(&sim);
    let mut bots = Bots::new(0.8, 9);
    bots.add(&mut sim, 50, Team::Red);
    sim.add_player(1, "leader", Team::Red, false);
    sim.spawn_at(1, at, 0.0);
    sim.spawn_at(50, at + Vec3::new(3.0, 0.0, 0.0), 0.0);
    let way = at + Vec3::new(0.0, 0.0, 60.0);
    let way = Vec3::new(way.x, sim.world.ground(way.x, way.z, 500.0).unwrap(), way.z);
    bots.human_squads = vec![HumanSquad {
        leader: 1,
        bots: vec![50],
        waypoint: Some(way),
    }];
    for _ in 0..60 * 15 {
        bots.think(&mut sim);
        sim.step();
    }
    let d = (sim.player(50).unwrap().feet() - way).length();
    assert!(d < 35.0, "at the waypoint: {d} m off");
}

#[test]
fn bots_take_capture_points_and_defenders_go_to_them() {
    // A capture point base someone may take, its turrets down.
    let mut found = None;
    'search: for seed in 0..40 {
        let def = WorldDef::generate(WorldParams { seed, contested: 6 });
        let st = WorldState::initial(&def);
        for cd in &def.continents {
            for (b, site) in cd.bases.iter().enumerate() {
                let b = b as BaseIdx;
                let owner = st.base(cd.id, b).owner;
                if site.capture != crate::worldgen::Capture::Points || owner.is_none() {
                    continue;
                }
                if let Some(t) = Team::ALL
                    .into_iter()
                    .find(|&t| st.can_hack(&def, cd.id, b, t))
                {
                    found = Some((def.clone(), st.clone(), cd.id, b, owner.unwrap(), t));
                    break 'search;
                }
            }
        }
    }
    let (def, st, c, b, owner, enemy) = found.expect("a capture point base");
    let map = Arc::new(ContinentMap::build(
        Arc::new(def.continents[c as usize].clone()),
        false,
    ));
    let mut sim = ContinentSim::new(c, map);
    sync(&def, &st, &mut sim);
    // Nothing else to take; nobody shooting.
    for o in 0..sim.bases.len() {
        if o != b as usize {
            sim.bases[o].hackable = [false; 3];
        }
    }
    sim.bases[b as usize].generator = 0.0;
    let l = sim.map.bases[b as usize].clone();
    let mut bots = Bots::new(0.6, 5);
    for i in 0..4 {
        let id = 60 + i;
        bots.add(&mut sim, id, enemy);
        let out = l.points[0].path[0] + Vec3::new(i as f32 * 2.0, 0.0, 0.0);
        sim.spawn_at(id, out, 0.0);
    }
    let mut took = None;
    for tick in 0..60 * 240 {
        bots.think(&mut sim);
        sim.step();
        for (base, t) in std::mem::take(&mut sim.majority) {
            if base == b && t.is_some() {
                took = Some(tick as f32 / 60.0);
            }
        }
        if took.is_some() {
            break;
        }
    }
    let took = took.unwrap_or_else(|| {
        let at: Vec<String> = bots
            .bots
            .iter()
            .map(|bot| {
                format!(
                    "{:?} {}",
                    bot.goal,
                    sim.player(bot.id).unwrap().feet() - l.center
                )
            })
            .collect();
        panic!(
            "no majority: points {:?}, bots {at:?}",
            sim.bases[b as usize].points
        )
    });
    println!("two points taken in {took:.0} s");
    // The owners' bots, there now, mostly go for the points the enemy holds.
    let mut defenders = Bots::new(0.6, 6);
    for i in 0..6 {
        let id = 90 + i;
        defenders.add(&mut sim, id, owner);
        sim.spawn_at(id, l.center + Vec3::new(i as f32, 0.0, -40.0), 0.0);
    }
    defenders.think(&mut sim);
    let goals: Vec<Option<Goal>> = defenders.bots.iter().map(|b| b.goal).collect();
    let to_points = goals
        .iter()
        .filter(|g| {
            matches!(g, Some(Goal::Point(g, i))
                if *g == b && sim.bases[b as usize].points[*i as usize].owner == Some(enemy))
        })
        .count();
    assert!(to_points >= 2, "{goals:?}");
}
