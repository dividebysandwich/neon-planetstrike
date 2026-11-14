//! Shots, impacts and explosions on screen. Everyone else's projectiles come as `Fx::Shot`
//! and fly here until their `Fx::Impact` (or the end of their life); one's own the server
//! doesn't send back: they're fired here at once when the trigger is pulled (`LocalShots`),
//! so they leave the gun without a round trip's delay. They also end here, on what they meet
//! as this client sees it (the land, a wall, a forcefield, an enemy body, vehicle or turret),
//! with their impact at once (`LocalImpacts`); the server's word on that impact, a round trip
//! and the playback delay later, is then passed over. (An impact the client didn't foresee
//! still shows when the server's word comes.)
//!
//! Impacts make particles (`Particle`): a bullet kicks up dust from the ground and sparks in
//! its faction's colour, rings hot sparks off armour, flashes on a forcefield; whatever
//! explodes makes a flash, a fireball, smoke rising, embers flying and (on the ground) dust
//! blown outwards. Flashes, fire, smoke and dust are soft particles (`ParticleMaterial`) on
//! quads turned to the viewer: alpha blended smoke and dust, additive fire and flashes.

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use ps_sim::snapshot::{Fx, ImpactOn};
use ps_sim::vehicle::VehicleKind;
use ps_sim::weapons::{Ammo, ShotKind, Weapon};
use ps_sim::{PlayerId, Team, VehicleId, look_dir};

use crate::game::{FxEvent, Game, LocalCmd, Mover};
use crate::materials::{AccentMaterial, GlowMaterial, NeonUniform, ParticleMaterial, team_color};

/// A missile homing on a vehicle (as the server has it fly).
#[derive(Component)]
struct Homing(VehicleId);

#[derive(Component)]
struct Projectile {
    id: u32,
    ammo: Ammo,
    vel: Vec3,
    gravity: f32,
    left: f32,
}

#[derive(Component)]
struct Blast {
    age: f32,
    life: f32,
    size: f32,
}

/// A particle: it flies (slowed by drag, pulled down by gravity), grows or shrinks from
/// `size.0` to `size.1` over its life, and fades through its `Fade` materials.
#[derive(Component)]
struct Particle {
    vel: Vec3,
    gravity: f32,
    drag: f32,
    age: f32,
    life: f32,
    size: (f32, f32),
}

/// A soft particle on a quad, turned to face the viewer each frame (rolled by this).
#[derive(Component)]
struct Billboard(f32);

/// A streak (`PARTICLE_STREAK`): a quad lying along `dir`, its bright end that way, turned
/// about it to the viewer; `long` times as long as it is wide. One that `flies` lies along
/// its particle's flight.
#[derive(Component)]
struct Streak {
    dir: Vec3,
    long: f32,
    flies: bool,
}

/// What goes with a projectile (its light, its engine's glow), `back` metres behind it, and
/// goes when it does. (`blink`: a grenade's light, of this size, flickering.)
#[derive(Component)]
struct Follow {
    target: Entity,
    back: f32,
    blink: Option<f32>,
}

/// A projectile's trail: a puff of smoke every so many metres flown (`left` more at most).
#[derive(Component)]
struct Trail {
    every: f32,
    flown: f32,
    left: u16,
    size: f32,
    life: f32,
}

/// A particle fading through materials: smoke (`FxAssets::puff`) or fire (`FxAssets::flame`).
#[derive(Component, Clone, Copy, PartialEq)]
enum Fade {
    Smoke,
    Dust,
    Fire,
    /// A trail's or a muzzle's thin grey smoke.
    Wisp,
}

const FADE_STEPS: usize = 6;

#[derive(Resource)]
struct FxAssets {
    tracer: Handle<Mesh>,
    shell: Handle<Mesh>,
    grenade: Handle<Mesh>,
    /// The soft particles' quad (2 m across: scaled by a particle's radius).
    quad: Handle<Mesh>,
    chip: Handle<Mesh>,
    /// A blast's fireball and its smoke.
    fire: Handle<ParticleMaterial>,
    smoke: Handle<ParticleMaterial>,
    spark: [Handle<AccentMaterial>; 4],
    /// A muzzle flash in each faction's colour (and none's).
    muzzle: [Handle<ParticleMaterial>; 4],
    /// Smoke and dust, thinner and thinner; fire, dimmer and dimmer.
    puff: [Handle<ParticleMaterial>; FADE_STEPS],
    dust: [Handle<ParticleMaterial>; FADE_STEPS],
    flame: [Handle<ParticleMaterial>; FADE_STEPS],
    /// Hot debris and sparks (chips), and white-hot flashes.
    hot: Handle<GlowMaterial>,
    flash: Handle<ParticleMaterial>,
    shield: Handle<ParticleMaterial>,
    /// Streaks in each faction's colour (tracers' light, a flash's spikes) and white hot.
    streak: [Handle<ParticleMaterial>; 4],
    hot_streak: Handle<ParticleMaterial>,
    /// A rocket's engine.
    engine: Handle<ParticleMaterial>,
    wisp: [Handle<ParticleMaterial>; FADE_STEPS],
}

/// How dear a particle is when there are many (`FxBudget`): the heart of an effect (a flash,
/// a fireball) always shows; its body (smoke, trails) thins out under load and far off; its
/// garnish (sparks, embers, dust, casings) goes first.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tier {
    Core,
    Body,
    Garnish,
}

/// How far (m) from the viewer the garnish is still made, and the body in full.
const GARNISH_REACH: f32 = 120.0;
const BODY_REACH: f32 = 250.0;
/// The particles' cap on a flat screen and in the headset (one draw each there).
const CAP: usize = 1400;
const CAP_VR: usize = 700;

/// The particles alive (every one an entity) against their cap: what may still be made.
#[derive(Resource)]
pub struct FxBudget {
    pub live: usize,
    pub cap: usize,
    /// The most alive at once so far (testing: `PS_FPS=1` logs it).
    pub peak: usize,
    /// The viewer's eye.
    pub eye: Vec3,
}

impl Default for FxBudget {
    fn default() -> Self {
        FxBudget {
            live: 0,
            cap: CAP,
            peak: 0,
            eye: Vec3::ZERO,
        }
    }
}

impl FxBudget {
    fn load(&self) -> f32 {
        self.live as f32 / self.cap.max(1) as f32
    }

    /// Whether a particle of `tier` at `at` is made (counted if so).
    fn take(&mut self, tier: Tier, at: Vec3) -> bool {
        let far = at.distance_squared(self.eye);
        let load = self.load();
        let ok = match tier {
            Tier::Core => true,
            Tier::Body => {
                load < 1.0
                    && (!(load > 0.8 || far > BODY_REACH * BODY_REACH)
                        || rand::random::<f32>() < 0.5)
            }
            Tier::Garnish => load <= 0.6 && far <= GARNISH_REACH * GARNISH_REACH,
        };
        if ok {
            self.live += 1;
        }
        ok
    }
}

/// What the effects are made with.
struct Ctx<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    fx: &'a FxAssets,
    budget: &'a mut FxBudget,
    game: &'a Game,
}

/// The local player's guns, as this client reckons them (for firing at once).
#[derive(Resource, Default)]
pub struct LocalShots {
    cooldown: f32,
    prev_fire: bool,
    next_id: u32,
    /// Shots fired here this frame (for the sounds): ammo and where.
    pub fired: Vec<(Ammo, Vec3)>,
    barrel: u8,
    /// The seat's second gun (missiles).
    alt_cooldown: f32,
    alt_barrel: u8,
}

/// The projectiles flying (by id), and their shooters' factions.
#[derive(Resource, Default)]
struct Flying(HashMap<u32, (Entity, Option<Team>)>);

/// One's own shots' impacts shown here before the server's word: where, what, when (s).
#[derive(Resource, Default)]
struct LocalImpacts(VecDeque<(Vec3, Ammo, f32)>);

/// How long (s) and how far (m, besides a blast's radius) the server's word on an impact
/// may come after and away from one shown here, to be that one.
const MATCH_TIME: f32 = 1.5;
const MATCH_DIST: f32 = 3.0;

/// Effects for the audio (src/audio.rs) and the HUD: what hit where.
#[derive(Message, Clone, Copy, Debug)]
pub struct Boom {
    pub at: Vec3,
    pub ammo: Option<Ammo>,
    pub on: ImpactOn,
    pub wreck: Option<VehicleKind>,
}

/// The effects' systems (the audio follows them).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FxSet;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalShots>()
            .init_resource::<Flying>()
            .init_resource::<LocalImpacts>()
            .init_resource::<FxBudget>()
            .add_message::<Boom>()
            .add_systems(Startup, make_assets)
            .add_systems(
                Update,
                (
                    count,
                    local_fire,
                    receive,
                    fly,
                    follow,
                    blasts,
                    particles,
                    face_viewer,
                    flares,
                    jets_and_chutes,
                )
                    .chain()
                    .in_set(FxSet)
                    .after(crate::game::PredictSet)
                    .run_if(resource_exists::<Game>),
            );
    }
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut parts: ResMut<Assets<ParticleMaterial>>,
) {
    let spark = [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None].map(|t| {
        accents.add(AccentMaterial {
            neon: NeonUniform::fixed(team_color(t), 9.0),
            overlay: false,
        })
    });
    let fade = |k: usize| 1.0 - k as f32 / FADE_STEPS as f32;
    let smoke_of = |c: Vec3, a: f32, parts: &mut Assets<ParticleMaterial>| {
        std::array::from_fn(|k| parts.add(ParticleMaterial::puff(c, a * fade(k), k as f32 * 3.7)))
    };
    let puff = smoke_of(Vec3::new(0.025, 0.025, 0.03), 0.8, &mut parts);
    let dust = smoke_of(Vec3::new(0.11, 0.1, 0.09), 0.6, &mut parts);
    let flame = std::array::from_fn(|k| {
        parts.add(ParticleMaterial::glow(
            Vec3::new(1.0, 0.38, 0.06) * 3.0,
            fade(k) * fade(k),
        ))
    });
    let muzzle = [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None]
        .map(|t| parts.add(ParticleMaterial::glow(team_color(t) * 4.0, 1.0)));
    let streak = [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None]
        .map(|t| parts.add(ParticleMaterial::streak(team_color(t) * 6.0, 1.0)));
    let wisp = smoke_of(Vec3::new(0.2, 0.2, 0.22), 0.6, &mut parts);
    commands.insert_resource(FxAssets {
        tracer: meshes.add(Cuboid::new(0.05, 0.05, 1.6)),
        shell: meshes.add(Sphere::new(0.18).mesh().ico(1).unwrap()),
        grenade: meshes.add(Sphere::new(0.09).mesh().ico(2).unwrap()),
        quad: meshes.add(Rectangle::new(2.0, 2.0)),
        chip: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        fire: parts.add(ParticleMaterial::glow(Vec3::new(1.0, 0.45, 0.1) * 3.0, 1.0)),
        smoke: parts.add(ParticleMaterial::puff(
            Vec3::new(0.025, 0.025, 0.03),
            0.7,
            11.0,
        )),
        spark,
        muzzle,
        puff,
        dust,
        flame,
        hot: glows.add(GlowMaterial {
            neon: NeonUniform::fixed(Vec3::new(1.0, 0.85, 0.55), 8.0),
            overlay: false,
        }),
        flash: parts.add(ParticleMaterial::glow(Vec3::new(1.0, 0.8, 0.5) * 4.0, 1.0)),
        shield: parts.add(ParticleMaterial::glow(Vec3::new(0.4, 0.8, 1.0) * 2.5, 0.8)),
        streak,
        hot_streak: parts.add(ParticleMaterial::streak(
            Vec3::new(1.0, 0.7, 0.35) * 5.0,
            1.0,
        )),
        engine: parts.add(ParticleMaterial::glow(
            Vec3::new(1.0, 0.55, 0.15) * 4.0,
            1.0,
        )),
        wisp,
    });
}

fn team_of(game: &Game, id: PlayerId) -> Option<Team> {
    game.snap
        .players
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.team)
        .or_else(|| game.names.get(&id).map(|n| n.1))
}

/// What a projectile looks like flying.
#[derive(Clone, Copy, PartialEq)]
enum Flight {
    /// A bolt: a tracer and its light.
    Bolt,
    /// A shell: a hot ball leaving a thin wake.
    Shell,
    /// A rocket or missile: its engine's glow and a trail of smoke.
    Rocket,
    Grenade,
}

fn flight_of(ammo: Ammo) -> Flight {
    match ammo {
        Ammo::Grenade | Ammo::Utility(_) => Flight::Grenade,
        Ammo::LightShell | Ammo::HeavyShell | Ammo::Gun(Weapon::Mauler) => Flight::Shell,
        Ammo::Rocket | Ammo::Missile | Ammo::Gun(Weapon::Skyhook) | Ammo::Gun(Weapon::Striker) => {
            Flight::Rocket
        }
        _ => Flight::Bolt,
    }
}

fn spawn_projectile(
    c: &mut Ctx,
    id: u32,
    ammo: Ammo,
    from: Vec3,
    vel: Vec3,
    team: Option<Team>,
) -> Entity {
    let b = ammo.ballistics();
    let (commands, fx) = (&mut *c.commands, c.fx);
    let flight = flight_of(ammo);
    let mesh = match flight {
        Flight::Grenade => fx.grenade.clone(),
        Flight::Shell | Flight::Rocket => fx.shell.clone(),
        Flight::Bolt => fx.tracer.clone(),
    };
    let ti = team.map_or(3, |t| t.index());
    let mat = fx.spark[ti].clone();
    // (Bolts stretched with their speed; a grenade a ball.)
    let len = if b.bounces {
        1.0
    } else {
        (vel.length() / 140.0).clamp(0.4, 4.0)
    };
    let dir = vel.normalize_or(Vec3::NEG_Z);
    let mut e = commands.spawn((
        Projectile {
            id,
            ammo,
            vel,
            gravity: b.gravity,
            left: b.life,
        },
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(from)
            .looking_to(dir, Vec3::Y)
            .with_scale(Vec3::new(1.0, 1.0, len)),
    ));
    match flight {
        Flight::Rocket => {
            e.insert(Trail {
                every: 1.1,
                flown: -1.5,
                left: 40,
                size: 0.42,
                life: 1.1,
            });
        }
        Flight::Shell => {
            e.insert(Trail {
                every: 2.2,
                flown: -3.0,
                left: 24,
                size: 0.26,
                life: 0.45,
            });
        }
        _ => {}
    }
    let e = e.id();
    // What goes with it: a bolt's light drawn out behind it, an engine's or a hot shell's
    // glow, a grenade's flicker.
    let (back, blink, streak, mat, scale) = match flight {
        Flight::Bolt => {
            let half = 1.5 * len;
            let wide = if b.damage > 30.0 { 0.16 } else { 0.11 };
            (
                half * 0.55,
                None,
                Some(half / wide),
                fx.streak[ti].clone(),
                Vec3::new(half, wide, wide),
            )
        }
        Flight::Rocket => (0.35, None, None, fx.engine.clone(), Vec3::splat(0.5)),
        Flight::Shell => (0.2, None, None, fx.flash.clone(), Vec3::splat(0.4)),
        Flight::Grenade => (
            0.0,
            Some(0.2),
            None,
            fx.muzzle[ti].clone(),
            Vec3::splat(0.2),
        ),
    };
    let mut glow = commands.spawn((
        Follow {
            target: e,
            back,
            blink,
        },
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(mat),
        Transform::from_translation(from - dir * back).with_scale(scale),
    ));
    match streak {
        Some(long) => glow.insert(Streak {
            dir,
            long,
            flies: false,
        }),
        None => glow.insert(Billboard(0.0)),
    };
    e
}

/// A muzzle flash's make: what a gun's looks like.
struct Flash {
    /// The glow at the muzzle (its radius, m) and how long it all lasts (s).
    core: f32,
    life: f32,
    /// The spike ahead (its length, m) and how wide (half, m).
    ahead: f32,
    wide: f32,
    /// Spikes out sideways (a muzzle brake's) or, for `cone`, fanned out ahead (a shotgun's).
    spikes: u8,
    spike: f32,
    cone: bool,
    /// White hot (a cannon's) and not the shooter's colour.
    hot: bool,
    /// Puffs of smoke left at the muzzle.
    smoke: u8,
    /// A launcher's blast out of its back.
    back: bool,
    /// Dust blown off the ground under it (a cannon near the ground).
    dust: bool,
}

const FLASH: Flash = Flash {
    core: 0.22,
    life: 0.06,
    ahead: 0.55,
    wide: 0.07,
    spikes: 0,
    spike: 0.0,
    cone: false,
    hot: false,
    smoke: 0,
    back: false,
    dust: false,
};

/// The flash of what fires `ammo` (none for what's thrown or laid).
fn flash_of(ammo: Ammo) -> Option<Flash> {
    Some(match ammo {
        Ammo::Grenade | Ammo::Utility(_) | Ammo::Mine => return None,
        Ammo::Gun(w) => match w {
            Weapon::Medgun | Weapon::RepairGun => return None,
            Weapon::Pistol => Flash {
                core: 0.16,
                ahead: 0.32,
                wide: 0.05,
                ..FLASH
            },
            Weapon::Rifle | Weapon::Volt | Weapon::Hive => Flash {
                spikes: 2,
                spike: 0.2,
                ..FLASH
            },
            Weapon::Ember => Flash {
                core: 0.27,
                ahead: 0.6,
                wide: 0.09,
                spikes: 4,
                spike: 0.24,
                ..FLASH
            },
            Weapon::Scatter => Flash {
                core: 0.32,
                life: 0.08,
                ahead: 0.5,
                wide: 0.1,
                spikes: 5,
                spike: 0.6,
                cone: true,
                smoke: 1,
                ..FLASH
            },
            Weapon::Lance => Flash {
                core: 0.3,
                life: 0.1,
                ahead: 1.7,
                wide: 0.09,
                spikes: 4,
                spike: 0.35,
                ..FLASH
            },
            Weapon::Striker | Weapon::Skyhook => Flash {
                core: 0.45,
                life: 0.1,
                ahead: 0.6,
                wide: 0.16,
                hot: true,
                smoke: 2,
                back: true,
                ..FLASH
            },
            Weapon::Pounder => Flash {
                core: 0.28,
                ahead: 0.75,
                wide: 0.09,
                spikes: 2,
                spike: 0.28,
                ..FLASH
            },
            Weapon::Mauler => Flash {
                core: 0.5,
                life: 0.09,
                ahead: 1.1,
                wide: 0.16,
                spikes: 2,
                spike: 0.6,
                hot: true,
                smoke: 1,
                ..FLASH
            },
            Weapon::Burster => Flash {
                core: 0.36,
                ahead: 0.8,
                wide: 0.11,
                spikes: 2,
                spike: 0.35,
                hot: true,
                ..FLASH
            },
        },
        Ammo::LightShell => Flash {
            core: 1.4,
            life: 0.13,
            ahead: 3.4,
            wide: 0.45,
            spikes: 2,
            spike: 1.8,
            hot: true,
            smoke: 4,
            dust: true,
            ..FLASH
        },
        Ammo::HeavyShell => Flash {
            core: 2.1,
            life: 0.16,
            ahead: 5.0,
            wide: 0.65,
            spikes: 4,
            spike: 2.6,
            hot: true,
            smoke: 7,
            dust: true,
            ..FLASH
        },
        Ammo::Rail => Flash {
            core: 1.4,
            life: 0.18,
            ahead: 7.5,
            wide: 0.35,
            spikes: 4,
            spike: 0.9,
            dust: true,
            ..FLASH
        },
        Ammo::Flak => Flash {
            core: 0.45,
            life: 0.07,
            ahead: 1.1,
            wide: 0.15,
            spikes: 2,
            spike: 0.45,
            hot: true,
            ..FLASH
        },
        Ammo::Rocket | Ammo::Missile => Flash {
            core: 0.55,
            life: 0.1,
            ahead: 0.5,
            wide: 0.18,
            hot: true,
            smoke: 2,
            back: true,
            ..FLASH
        },
        Ammo::TurretBolt => Flash {
            core: 0.45,
            life: 0.08,
            ahead: 1.3,
            wide: 0.15,
            spikes: 2,
            spike: 0.5,
            ..FLASH
        },
        Ammo::Spitfire => Flash {
            core: 0.2,
            ahead: 0.45,
            ..FLASH
        },
        Ammo::Chaingun => Flash {
            core: 0.3,
            ahead: 0.85,
            wide: 0.1,
            spikes: 2,
            spike: 0.3,
            ..FLASH
        },
        Ammo::AirCannon | Ammo::Autocannon => Flash {
            core: 0.42,
            life: 0.07,
            ahead: 1.2,
            wide: 0.14,
            spikes: 2,
            spike: 0.5,
            hot: true,
            ..FLASH
        },
    })
}

/// A spike of a muzzle flash: a streak from `at` out along `dir`, `len` long, brightest at
/// the muzzle.
#[allow(clippy::too_many_arguments)]
fn spike(
    c: &mut Ctx,
    mat: &Handle<ParticleMaterial>,
    at: Vec3,
    dir: Vec3,
    len: f32,
    wide: f32,
    life: f32,
    carry: Vec3,
) {
    let quad = c.fx.quad.clone();
    if let Some(mut e) = particle(
        c,
        Tier::Core,
        &quad,
        at + dir * len * 0.5,
        carry,
        0.0,
        0.0,
        life,
        (wide, wide * 0.6),
    ) {
        e.insert((
            Streak {
                dir: -dir,
                long: len * 0.5 / wide,
                flies: false,
            },
            MeshMaterial3d(mat.clone()),
        ));
    }
}

/// A gun's muzzle flash at `at`, firing `ammo` along `dir`: a glow, spikes of flame ahead and
/// out of its brake, and for the big ones smoke, a launcher's back blast and dust blown off
/// the ground. (`carry`: the shooter's own speed, which the flash keeps.)
fn muzzle_fx(c: &mut Ctx, at: Vec3, dir: Vec3, ammo: Ammo, team: Option<Team>, carry: Vec3) {
    let Some(f) = flash_of(ammo) else { return };
    let fx = c.fx;
    let ti = team.map_or(3, |t| t.index());
    let (glow, streak) = if f.hot {
        (fx.flash.clone(), fx.hot_streak.clone())
    } else {
        (fx.muzzle[ti].clone(), fx.streak[ti].clone())
    };
    // (A little jitter: no two flashes alike.)
    let k = rnd(0.85, 1.15);
    c.budget.live += 1;
    c.commands.spawn((
        Blast {
            age: 0.0,
            life: f.life,
            size: f.core * k,
        },
        Billboard(rnd(0.0, 6.3)),
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(glow),
        Transform::from_translation(at + dir * f.core * 0.3).with_scale(Vec3::splat(0.05)),
    ));
    spike(c, &streak, at, dir, f.ahead * k, f.wide, f.life, carry);
    if f.spikes > 0 {
        let side = dir.any_orthonormal_vector();
        let roll = rnd(0.0, 6.3);
        for i in 0..f.spikes {
            let a = roll + i as f32 / f.spikes as f32 * std::f32::consts::TAU;
            let out = Quat::from_axis_angle(dir, a) * side;
            // (A shotgun's: fanned out ahead; a brake's: out sideways, swept a little forward.)
            let d = if f.cone {
                (dir + out * rnd(0.2, 0.45)).normalize()
            } else {
                (out + dir * 0.35).normalize()
            };
            spike(
                c,
                &streak,
                at,
                d,
                f.spike * rnd(0.7, 1.2),
                f.wide * 0.7,
                f.life,
                carry,
            );
        }
    }
    let wisp = fx.wisp[0].clone();
    for _ in 0..f.smoke {
        let v =
            carry * 0.3 + dir * rnd(1.0, 4.0) * f.core + random_dir(false) * 0.5 + Vec3::Y * 0.4;
        if let Some(mut e) = soft(
            c,
            Tier::Body,
            &wisp,
            at + dir * f.core * rnd(0.2, 1.0),
            v,
            -0.3,
            2.5,
            rnd(0.7, 1.3),
            (f.core * 0.35, f.core * rnd(0.9, 1.4)),
        ) {
            e.insert(Fade::Wisp);
        }
    }
    if f.back {
        // The blast out of the launcher's back: a lick of flame and smoke thrown out behind.
        spike(c, &streak, at - dir * 0.9, -dir, 1.3, 0.2, 0.1, carry);
        for _ in 0..3 {
            if let Some(mut e) = soft(
                c,
                Tier::Body,
                &wisp,
                at - dir * rnd(1.0, 1.6),
                carry * 0.3 - dir * rnd(3.0, 7.0) + random_dir(false) * 0.8,
                -0.3,
                3.0,
                rnd(0.6, 1.0),
                (0.2, rnd(0.6, 0.9)),
            ) {
                e.insert(Fade::Wisp);
            }
        }
    }
    if f.dust
        && let Some(g) = c.game.world.ground(at.x, at.z, at.y)
        && at.y - g < 3.0
    {
        let dust = fx.dust[0].clone();
        let under = Vec3::new(at.x, g + 0.2, at.z) + dir * f.core;
        for i in 0..8 {
            let a = i as f32 / 8.0 * std::f32::consts::TAU + rnd(0.0, 0.6);
            let out = Vec3::new(a.cos(), 0.0, a.sin());
            if let Some(mut e) = soft(
                c,
                Tier::Garnish,
                &dust,
                under + out * f.core * 0.6,
                out * f.core * rnd(2.5, 4.5) + Vec3::Y * 0.5,
                0.0,
                2.4,
                rnd(0.8, 1.3),
                (f.core * 0.4, f.core * rnd(0.9, 1.3)),
            ) {
                e.insert(Fade::Dust);
            }
        }
    }
}

fn spawn_blast(c: &mut Ctx, at: Vec3, size: f32, life: f32) {
    let (commands, fx) = (&mut *c.commands, c.fx);
    c.budget.live += if size > 2.0 { 3 } else { 1 };
    commands.spawn((
        Blast {
            age: 0.0,
            life,
            size,
        },
        Billboard(rnd(0.0, 6.3)),
        Mesh3d(fx.quad.clone()),
        MeshMaterial3d(fx.fire.clone()),
        Transform::from_translation(at).with_scale(Vec3::splat(0.1)),
    ));
    if size > 2.0 {
        commands.spawn((
            Blast {
                age: 0.0,
                life: life * 3.0,
                size: size * 1.4,
            },
            Billboard(rnd(0.0, 6.3)),
            Mesh3d(fx.quad.clone()),
            MeshMaterial3d(fx.smoke.clone()),
            Transform::from_translation(at + Vec3::Y * size * 0.5).with_scale(Vec3::splat(0.1)),
        ));
        commands.spawn((
            Blast {
                age: 0.0,
                life: life * 0.6,
                size: 0.0,
            },
            PointLight {
                color: Color::linear_rgb(1.0, 0.5, 0.15),
                intensity: 4_000_000.0 * size,
                range: size * 8.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(at + Vec3::Y),
        ));
    }
}

/// A random unit vector (`up`: in the upper half).
fn random_dir(up: bool) -> Vec3 {
    let v = Vec3::new(
        rand::random::<f32>() * 2.0 - 1.0,
        rand::random::<f32>() * 2.0 - 1.0,
        rand::random::<f32>() * 2.0 - 1.0,
    )
    .normalize_or(Vec3::Y);
    if up {
        Vec3::new(v.x, v.y.abs(), v.z)
    } else {
        v
    }
}

fn rnd(lo: f32, hi: f32) -> f32 {
    lo + rand::random::<f32>() * (hi - lo)
}

/// A particle of `tier`, if the budget has room for it.
#[allow(clippy::too_many_arguments)]
fn particle<'a>(
    c: &'a mut Ctx,
    tier: Tier,
    mesh: &Handle<Mesh>,
    at: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    life: f32,
    size: (f32, f32),
) -> Option<EntityCommands<'a>> {
    if !c.budget.take(tier, at) {
        return None;
    }
    let mut e = c.commands.spawn((
        Particle {
            vel,
            gravity,
            drag,
            age: 0.0,
            life,
            size,
        },
        Mesh3d(mesh.clone()),
        Transform::from_translation(at)
            .with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                rnd(0.0, 6.3),
                rnd(0.0, 6.3),
                rnd(0.0, 6.3),
            ))
            .with_scale(Vec3::splat(size.0.max(0.001))),
    ));
    e.insert(Visibility::default());
    Some(e)
}

/// A soft particle (`FxAssets::quad`, facing the viewer) of `size` radius growing to the second.
#[allow(clippy::too_many_arguments)]
fn soft<'a>(
    c: &'a mut Ctx,
    tier: Tier,
    mat: &Handle<ParticleMaterial>,
    at: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    life: f32,
    size: (f32, f32),
) -> Option<EntityCommands<'a>> {
    let quad = c.fx.quad.clone();
    let mut e = particle(c, tier, &quad, at, vel, gravity, drag, life, size)?;
    e.insert((Billboard(rnd(0.0, 6.3)), MeshMaterial3d(mat.clone())));
    Some(e)
}

/// Sparks flying from `at` (up, off the ground, when `up`).
fn sparks(
    c: &mut Ctx,
    at: Vec3,
    n: usize,
    speed: f32,
    mat: Option<&Handle<AccentMaterial>>,
    up: bool,
) {
    let (chip, hot) = (c.fx.chip.clone(), c.fx.hot.clone());
    for _ in 0..n {
        let v = random_dir(up) * speed * rnd(0.5, 1.2);
        let size = rnd(0.04, 0.08);
        let Some(mut e) = particle(
            c,
            Tier::Garnish,
            &chip,
            at,
            v,
            9.8,
            1.5,
            rnd(0.2, 0.45),
            (size, 0.0),
        ) else {
            continue;
        };
        match mat {
            Some(m) => e.insert(MeshMaterial3d(m.clone())),
            None => e.insert(MeshMaterial3d(hot.clone())),
        };
    }
}

/// Smoke or dust: puffs round `at`, rising and spreading.
#[allow(clippy::too_many_arguments)]
fn puffs(c: &mut Ctx, tier: Tier, at: Vec3, n: usize, size: f32, life: f32, rise: f32, kind: Fade) {
    let mat = if kind == Fade::Dust {
        c.fx.dust[0].clone()
    } else {
        c.fx.puff[0].clone()
    };
    for _ in 0..n {
        let off = random_dir(true) * size * 0.4;
        let v = Vec3::Y * rise * rnd(0.6, 1.3) + random_dir(false) * size * 0.6;
        if let Some(mut e) = soft(
            c,
            tier,
            &mat,
            at + off,
            v,
            -0.3,
            1.2,
            life * rnd(0.75, 1.25),
            (size * 0.45, size * rnd(1.1, 1.5)),
        ) {
            e.insert(kind);
        }
    }
}

/// An explosion of `size` (its splash radius) at `at`: a white flash, a fireball, debris,
/// smoke rising and (on the ground) dust blown out round it. (`light`: a flak burst's, of which
/// there are many: a flash, a little smoke and a few sparks.)
fn explosion(c: &mut Ctx, at: Vec3, size: f32, ground: bool, light: bool) {
    let s = size.max(0.8);
    let (flash, flame, dust) = (
        c.fx.flash.clone(),
        c.fx.flame[0].clone(),
        c.fx.dust[0].clone(),
    );
    soft(
        c,
        Tier::Core,
        &flash,
        at,
        Vec3::ZERO,
        0.0,
        0.0,
        0.14,
        (s * 0.35, s * 0.8),
    );
    if light {
        puffs(
            c,
            Tier::Body,
            at,
            2,
            s * 0.9,
            rnd(1.2, 1.8),
            0.3,
            Fade::Smoke,
        );
        sparks(c, at, 4, 9.0, None, false);
        return;
    }
    for _ in 0..5 {
        let off = random_dir(true) * s * 0.2;
        if let Some(mut e) = soft(
            c,
            Tier::Core,
            &flame,
            at + off,
            Vec3::Y * s * 0.6 + random_dir(false) * s * 0.5,
            -1.0,
            2.0,
            rnd(0.35, 0.6),
            (s * 0.25, s * rnd(0.6, 0.85)),
        ) {
            e.insert(Fade::Fire);
        }
    }
    let debris = (6.0 + s * 2.0).min(22.0) as usize;
    for _ in 0..debris {
        let v = random_dir(ground) * rnd(6.0, 14.0) * s.sqrt();
        // (Embers: glowing, fading as they fly.)
        let size = rnd(0.06, 0.14) * s.sqrt();
        if let Some(mut e) = soft(
            c,
            Tier::Garnish,
            &flame,
            at,
            v,
            14.0,
            0.6,
            rnd(0.5, 1.1),
            (size, size * 0.4),
        ) {
            e.insert(Fade::Fire);
        }
    }
    puffs(
        c,
        Tier::Body,
        at + Vec3::Y * s * 0.2,
        (2.0 + s * 0.8).min(8.0) as usize,
        s * 0.9,
        rnd(1.6, 2.4),
        s * 0.5,
        Fade::Smoke,
    );
    if ground {
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU + rnd(0.0, 0.5);
            let out = Vec3::new(a.cos(), 0.0, a.sin());
            if let Some(mut e) = soft(
                c,
                Tier::Garnish,
                &dust,
                at + out * s * 0.3,
                out * s * rnd(2.0, 3.0) + Vec3::Y * 0.6,
                0.0,
                2.2,
                rnd(0.9, 1.4),
                (s * 0.3, s * 0.7),
            ) {
                e.insert(Fade::Dust);
            }
        }
    }
}

/// What a projectile does where it hits.
fn impact_fx(c: &mut Ctx, at: Vec3, ammo: Ammo, what: ImpactOn, team: Option<Team>) {
    let b = ammo.ballistics();
    let spark = c.fx.spark[team.map_or(3, |t| t.index())].clone();
    if b.splash > 0.0 {
        let light = matches!(ammo, Ammo::Flak | Ammo::Gun(Weapon::Burster));
        explosion(c, at, b.splash * 0.7, what == ImpactOn::Ground, light);
        return;
    }
    // (A heavier gun, a bigger hit.)
    let k = (b.damage / 12.0).clamp(0.6, 2.5);
    match what {
        ImpactOn::Ground => {
            puffs(c, Tier::Garnish, at, 2, 0.6 * k, 0.6, 0.8, Fade::Dust);
            sparks(c, at, 4, 5.0, Some(&spark), true);
        }
        ImpactOn::Body => sparks(c, at, 7, 4.0, Some(&spark), false),
        ImpactOn::Vehicle => {
            sparks(c, at, 8, 7.0, None, false);
            let flash = c.fx.flash.clone();
            soft(
                c,
                Tier::Core,
                &flash,
                at,
                Vec3::ZERO,
                0.0,
                0.0,
                0.08,
                (0.2, 0.5 * k),
            );
        }
        ImpactOn::Shield => {
            let shield = c.fx.shield.clone();
            soft(
                c,
                Tier::Core,
                &shield,
                at,
                Vec3::ZERO,
                0.0,
                0.0,
                0.25,
                (0.3, 1.4 * k),
            );
            sparks(c, at, 4, 3.0, Some(&spark), false);
        }
        ImpactOn::Air => {
            puffs(c, Tier::Body, at, 2, 0.8 * k, 0.8, 0.3, Fade::Smoke);
        }
    }
}

/// The budget's count of what's alive, its cap and the viewer's eye (and, testing with
/// `PS_FPS=1`, a line a second on how many particles there are).
#[allow(clippy::too_many_arguments)]
fn count(
    mut budget: ResMut<FxBudget>,
    time: Res<Time>,
    particles: Query<(), With<Particle>>,
    blasts: Query<(), With<Blast>>,
    cams: Query<&GlobalTransform, With<crate::desktop::MainCamera>>,
    vr: Option<Res<crate::vr::VrState>>,
    xr: Option<Res<crate::vr::XrInput>>,
    mut log: Local<(f32, bool, bool)>,
) {
    budget.live = particles.iter().len() + blasts.iter().len();
    budget.peak = budget.peak.max(budget.live);
    budget.cap = if xr.is_some() { CAP_VR } else { CAP };
    budget.eye = match (vr, xr) {
        (Some(vr), Some(xr)) => (vr.rig * xr.head.unwrap_or_default()).translation,
        _ => cams.single().map_or(budget.eye, |c| c.translation()),
    };
    if !log.1 {
        *log = (0.0, true, std::env::var("PS_FPS").is_ok());
    }
    if log.2 {
        log.0 += time.delta_secs();
        if log.0 >= 1.0 {
            log.0 = 0.0;
            info!(
                "fx: {} particles (peak {}, cap {})",
                budget.live, budget.peak, budget.cap
            );
        }
    }
}

/// Turns the soft particles to the viewer (the camera, or in VR the head), and the streaks
/// about their length to it.
fn face_viewer(
    budget: Res<FxBudget>,
    mut q: Query<(&mut Transform, &Billboard)>,
    mut streaks: Query<(&mut Transform, &Streak), Without<Billboard>>,
) {
    let eye = budget.eye;
    for (mut t, b) in &mut q {
        let to = t.translation - eye;
        if to.length_squared() > 1e-6 {
            // (+Z, the quad's face, towards the eye.)
            t.rotation =
                Transform::IDENTITY.looking_to(to, Vec3::Y).rotation * Quat::from_rotation_z(b.0);
        }
    }
    for (mut t, s) in &mut streaks {
        let x = s.dir.normalize_or(Vec3::X);
        let to = eye - t.translation;
        let z = (to - x * to.dot(x)).normalize_or(x.any_orthonormal_vector());
        t.rotation = Quat::from_mat3(&Mat3::from_cols(x, z.cross(x), z));
    }
}

/// What goes with a projectile keeps behind it, and goes when it's gone.
fn follow(
    mut commands: Commands,
    time: Res<Time>,
    targets: Query<(&Transform, &Projectile), Without<Follow>>,
    mut q: Query<(Entity, &Follow, &mut Transform, Option<&mut Streak>)>,
) {
    let flicker = 0.65 + 0.35 * (time.elapsed_secs() * 20.0).sin();
    for (e, f, mut t, streak) in &mut q {
        let Ok((at, p)) = targets.get(f.target) else {
            commands.entity(e).despawn();
            continue;
        };
        let dir = p.vel.normalize_or(Vec3::NEG_Z);
        t.translation = at.translation - dir * f.back;
        if let Some(mut s) = streak {
            s.dir = dir;
        }
        if let Some(size) = f.blink {
            t.scale = Vec3::splat(size * flicker);
        }
    }
}

type ParticleParts = (
    Entity,
    &'static mut Particle,
    &'static mut Transform,
    Option<&'static Fade>,
    Option<&'static mut MeshMaterial3d<ParticleMaterial>>,
    Option<&'static mut Streak>,
);

/// Particles fly, grow or shrink, fade and go.
fn particles(
    mut commands: Commands,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut q: Query<ParticleParts>,
) {
    let Some(fx) = fx else { return };
    let dt = time.delta_secs();
    for (e, mut p, mut t, fade, mat, streak) in &mut q {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = p.age / p.life;
        let drag = (1.0 - p.drag * dt).max(0.0);
        p.vel *= drag;
        p.vel.y -= p.gravity * dt;
        t.translation += p.vel * dt;
        let size = (p.size.0 + (p.size.1 - p.size.0) * k.sqrt()).max(0.001);
        t.scale = Vec3::splat(size);
        if let Some(mut s) = streak {
            if s.flies && p.vel.length_squared() > 1e-4 {
                s.dir = p.vel.normalize();
            }
            t.scale.x = size * s.long;
        }
        if let (Some(&fade), Some(mut m)) = (fade, mat) {
            let step = ((k * FADE_STEPS as f32) as usize).min(FADE_STEPS - 1);
            let want = match fade {
                Fade::Smoke => &fx.puff[step],
                Fade::Dust => &fx.dust[step],
                Fade::Fire => &fx.flame[step],
                Fade::Wisp => &fx.wisp[step],
            };
            if m.0 != *want {
                m.0 = want.clone();
            }
        }
    }
}

/// One's own shots, fired here at once.
#[allow(clippy::too_many_arguments)]
fn local_fire(
    mut commands: Commands,
    game: Res<Game>,
    local: Res<LocalCmd>,
    mover: Res<Mover>,
    fx: Option<Res<FxAssets>>,
    mut shots: ResMut<LocalShots>,
    time: Res<Time>,
    models: Res<crate::assets::Models>,
    view: Res<crate::seatview::SeatView>,
    mut budget: ResMut<FxBudget>,
) {
    shots.fired.clear();
    let Some(fx) = fx else { return };
    let c = &mut Ctx {
        commands: &mut commands,
        fx: &fx,
        budget: &mut budget,
        game: &game,
    };
    let dt = time.delta_secs();
    shots.cooldown = (shots.cooldown - dt).max(0.0);
    let fire = local.cmd.fire && local.active;
    let edge = fire && !shots.prev_fire;
    shots.prev_fire = fire;
    let Some(me) = game.mine().cloned() else {
        return;
    };
    // A locked target, for the missile about to go.
    let locked = me.lock.filter(|l| l.1 >= 1.0).map(|l| l.0);
    // The seat's second gun (the interceptor's missiles) on the other trigger.
    shots.alt_cooldown = (shots.alt_cooldown - dt).max(0.0);
    if local.cmd.alt
        && local.active
        && shots.alt_cooldown <= 0.0
        && me.dead.is_none()
        && game.ready()
        && me.alt_ammo.is_some_and(|a| a.0 > 0)
        && let Some((vid, seat)) = me.seat
        && let Some(v) = game.vehicle(vid)
        && let Some(gi) = v
            .kind
            .spec()
            .seats
            .get(seat as usize)
            .and_then(|s| s.alt_gun)
    {
        let gs = v.kind.gun_spec(v.loadout, gi);
        shots.alt_cooldown = gs.interval;
        shots.alt_barrel = (shots.alt_barrel + 1) % gs.barrels.max(1);
        let (pos, rot) = match &mover.drive {
            Some((id, _, m)) if *id == vid => (m.pos, m.rot),
            _ => (v.pos, v.rot),
        };
        let side = (shots.alt_barrel as f32 - (gs.barrels - 1) as f32 * 0.5) * gs.barrel_gap;
        let from = pos + rot * (gs.pivot + Vec3::new(side, 0.0, -gs.barrel));
        let vel = rot * Vec3::NEG_Z * gs.ammo.ballistics().speed + v.vel;
        shots.next_id = shots.next_id.wrapping_add(1);
        let id = u32::MAX - shots.next_id % 100_000;
        let e = spawn_projectile(c, id, gs.ammo, from, vel, Some(game.team));
        muzzle_fx(c, from, rot * Vec3::NEG_Z, gs.ammo, Some(game.team), v.vel);
        if let Some(t) = locked {
            c.commands.entity(e).insert(Homing(t));
        }
        shots.fired.push((gs.ammo, from));
    }
    if me.dead.is_some() || !game.ready() || !fire || shots.cooldown > 0.0 {
        return;
    }
    // (Nobody fires inside a warp gate's bubble.)
    let at = if view.active {
        view.eye
    } else {
        mover.body.feet + Vec3::Y
    };
    if game
        .map
        .as_ref()
        .is_some_and(|m| m.gates.iter().any(|g| ps_sim::base::in_gate_bubble(*g, at)))
    {
        return;
    }
    let team = Some(game.team);
    let look = look_dir(local.cmd.yaw, local.cmd.pitch);
    // A wall turret: its bolts from its barrels, turn about.
    if me.turret.is_some() && view.active {
        let powered = game
            .snap
            .turrets
            .iter()
            .find(|t| t.occupant == game.me)
            .is_some_and(|t| t.powered);
        if !powered {
            return;
        }
        shots.cooldown = ps_sim::sim::TURRET_INTERVAL;
        shots.barrel = (shots.barrel + 1) % 2;
        let d = view.rot * Vec3::NEG_Z;
        let side = view.rot * Vec3::X * if shots.barrel == 0 { 0.55 } else { -0.55 };
        let from = view.pivot + d * ps_sim::base::TURRET_BARREL + side;
        let vel = d * Ammo::TurretBolt.ballistics().speed;
        shots.next_id = shots.next_id.wrapping_add(1);
        let id = u32::MAX - shots.next_id % 100_000;
        spawn_projectile(c, id, Ammo::TurretBolt, from, vel, team);
        muzzle_fx(c, from, d, Ammo::TurretBolt, team, Vec3::ZERO);
        shots.fired.push((Ammo::TurretBolt, from));
        return;
    }
    match me.seat {
        None => {
            let slot = (local.cmd.slot as usize).min(ps_sim::weapons::SLOTS - 1);
            let w = me.loadout.slot(slot as u8);
            let s = w.stats();
            let g = me.guns[slot];
            if g.mag == 0 || g.reload > 0.0 || (!s.automatic && !edge) || s.shot == ShotKind::Beam {
                return;
            }
            shots.cooldown = s.interval;
            let (muzzle, dir) = match &local.cmd.vr {
                // VR: from the gun in the hand, along its barrel.
                Some(vr) => (mover.body.feet + vr.muzzle.pos, vr.muzzle.forward()),
                None => {
                    // (From the gun's muzzle, converging on the crosshair's line.)
                    let eye =
                        mover.body.feet + Vec3::Y * ps_sim::player::eye_height(local.cmd.crouch);
                    let (_, hands) = crate::actors::flat_poses(
                        local.cmd.yaw,
                        local.cmd.pitch,
                        local.cmd.crouch,
                        w,
                    );
                    let muzzle = mover.body.feet
                        + hands[1].translation
                        + hands[1].rotation * crate::actors::muzzle_of(&models, w);
                    let aim = game
                        .world
                        .ray(eye, eye + look * 600.0)
                        .map_or(eye + look * 600.0, |h| h.1);
                    (muzzle, (aim - muzzle).normalize_or(look))
                }
            };
            let moving = mover.body.vel.length() / ps_sim::player::WALK_SPEED;
            let spread = w.spread_for(
                local.cmd.alt,
                local.cmd.crouch,
                moving,
                !mover.body.on_ground(),
            );
            for _ in 0..s.pellets.min(6) {
                let d = (dir
                    + Vec3::new(
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                        rand::random::<f32>() - 0.5,
                    ) * spread
                        * 2.0)
                    .normalize();
                shots.next_id = shots.next_id.wrapping_add(1);
                let id = u32::MAX - shots.next_id % 100_000;
                let e = spawn_projectile(c, id, Ammo::Gun(w), muzzle, d * s.speed, team);
                if let Some(t) = locked.filter(|_| Ammo::Gun(w).lock_on().is_some()) {
                    c.commands.entity(e).insert(Homing(t));
                }
            }
            let carry = Vec3::new(mover.body.vel.x, 0.0, mover.body.vel.y);
            muzzle_fx(c, muzzle, dir, Ammo::Gun(w), team, carry);
            shots.fired.push((Ammo::Gun(w), muzzle));
        }
        Some((vid, seat)) => {
            let Some(v) = game.vehicle(vid) else { return };
            // (The harvester's beam is no gun: src/vehicles.rs draws it.)
            if v.kind == VehicleKind::Harvester {
                return;
            }
            let spec = v.kind.spec();
            let Some(gi) = spec.seats.get(seat as usize).and_then(|s| s.gun) else {
                return;
            };
            // (Out of rounds: the server fires nothing, so neither does the screen.)
            if me.seat_ammo.is_some_and(|a| a.0 == 0) {
                return;
            }
            let gs = v.kind.gun_spec(v.loadout, gi);
            shots.cooldown = gs.interval;
            let (pos, rot) = match &mover.drive {
                Some((id, _, m)) if *id == vid => (m.pos, m.rot),
                _ => (v.pos, v.rot),
            };
            let (yaw, pitch) = v
                .aim
                .get(gi)
                .map_or((0.0, 0.0), |a| (a.0 as f32 / 1e4, a.1 as f32 / 1e4));
            let aim = if gs.fixed {
                Quat::IDENTITY
            } else if view.want.is_some() && view.vehicle == Some(vid) {
                // (As the view has it: predicted.)
                rot.inverse() * view.rot
            } else {
                Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
            };
            shots.barrel = (shots.barrel + 1) % gs.barrels.max(1);
            let side = if gs.barrels > 1 {
                (shots.barrel as f32 - (gs.barrels - 1) as f32 * 0.5) * gs.barrel_gap
            } else {
                0.0
            };
            let from = pos + rot * (gs.pivot + aim * Vec3::new(side, 0.0, -gs.barrel));
            let dir = rot * aim * Vec3::NEG_Z;
            let vel = dir * gs.ammo.ballistics().speed + v.vel;
            shots.next_id = shots.next_id.wrapping_add(1);
            let id = u32::MAX - shots.next_id % 100_000;
            spawn_projectile(c, id, gs.ammo, from, vel, team);
            muzzle_fx(c, from, dir, gs.ammo, team, v.vel);
            shots.fired.push((gs.ammo, from));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn receive(
    mut commands: Commands,
    game: Res<Game>,
    fx: Option<Res<FxAssets>>,
    mut events: MessageReader<FxEvent>,
    mut flying: ResMut<Flying>,
    mut booms: MessageWriter<Boom>,
    mut local: ResMut<LocalImpacts>,
    time: Res<Time>,
    mut shots: Query<(Entity, &mut Projectile, &mut Transform, Option<&Homing>)>,
    mut budget: ResMut<FxBudget>,
) {
    let Some(fx) = fx else { return };
    let c = &mut Ctx {
        commands: &mut commands,
        fx: &fx,
        budget: &mut budget,
        game: &game,
    };
    let now = time.elapsed_secs();
    // Who has flashed this frame (a scatter gun's pellets come as a shot each).
    let mut flashed: Vec<(PlayerId, Ammo)> = Vec::new();
    while local.0.front().is_some_and(|l| now - l.2 > MATCH_TIME) {
        local.0.pop_front();
    }
    for FxEvent(f) in events.read() {
        match *f {
            Fx::Shot {
                id,
                owner,
                ammo,
                from,
                vel,
                target,
            } => {
                let team = team_of(&game, owner);
                // A grenade's new flight after a bounce: the one flying follows it.
                if let Some(&(e, _)) = flying.0.get(&id) {
                    if let Ok((_, mut p, mut t, _)) = shots.get_mut(e) {
                        p.vel = vel;
                        t.translation = from;
                    }
                    continue;
                }
                let many = matches!(ammo, Ammo::Gun(w) if w.stats().pellets > 1);
                if !(many && flashed.contains(&(owner, ammo))) {
                    muzzle_fx(
                        c,
                        from,
                        vel.normalize_or(Vec3::NEG_Z),
                        ammo,
                        team,
                        Vec3::ZERO,
                    );
                    flashed.push((owner, ammo));
                }
                let e = spawn_projectile(c, id, ammo, from, vel, team);
                if let Some(t) = target {
                    c.commands.entity(e).insert(Homing(t));
                }
                flying.0.insert(id, (e, team));
            }
            Fx::Impact { id, ammo, at, what } => {
                // (A shot not seen flying is one's own: the server doesn't send those.)
                let team = match flying.0.remove(&id) {
                    Some((e, team)) => {
                        c.commands.entity(e).try_despawn();
                        team
                    }
                    None => {
                        // One's own: shown already if this client saw it coming.
                        let reach = MATCH_DIST + ammo.ballistics().splash;
                        if let Some(i) = local
                            .0
                            .iter()
                            .position(|l| l.1 == ammo && (l.0 - at).length() < reach)
                        {
                            let l = local.0.remove(i);
                            debug!(
                                "own impact shown {:.0} ms before the server's word",
                                l.map_or(0.0, |l| (now - l.2) * 1000.0)
                            );
                            continue;
                        }
                        debug!("impact of a shot not seen flying, or not foreseen here: {what:?}");
                        Some(game.team)
                    }
                };
                show_impact(c, &mut booms, at, ammo, what, team);
            }
            Fx::Wreck { at, kind } => {
                spawn_blast(c, at, 7.0, 0.9);
                explosion(c, at, 6.0, true, false);
                booms.write(Boom {
                    at,
                    ammo: None,
                    on: ImpactOn::Vehicle,
                    wreck: Some(kind),
                });
            }
            // (A turret's flash is made with its bolt's `Shot`, which says where it points.)
            Fx::Muzzle { .. } => {}
        }
    }
}

/// An impact's effects: the blast's light, the particles, the sound (`Boom`).
fn show_impact(
    c: &mut Ctx,
    booms: &mut MessageWriter<Boom>,
    at: Vec3,
    ammo: Ammo,
    what: ImpactOn,
    team: Option<Team>,
) {
    let b = ammo.ballistics();
    if let Ammo::Utility(k) = ammo {
        utility_fx(c, at, k);
    } else {
        if b.splash > 0.0 {
            // (The light of the blast.)
            spawn_blast(c, at, b.splash * 0.6, 0.5);
        }
        impact_fx(c, at, ammo, what, team);
    }
    booms.write(Boom {
        at,
        ammo: Some(ammo),
        on: what,
        wreck: None,
    });
}

/// What one's own shot meets going from `a` to `b`, as this client sees it (as the server
/// reckons it: the land and walls, forcefields but the faction's own, enemy bodies on foot,
/// enemy vehicles' hulls and the enemy's standing turrets): where and what.
fn own_hit(game: &Game, a: Vec3, b: Vec3) -> Option<(Vec3, ImpactOn)> {
    let team = game.team;
    let seg = b - a;
    let mut best: Option<(f32, ImpactOn)> = None;
    let mut take = |f: f32, on: ImpactOn| {
        if best.is_none_or(|h| f < h.0) {
            best = Some((f, on));
        }
    };
    let vehicles: Vec<(ps_sim::vehicle::Motion, Vec3, Team)> = game
        .snap
        .vehicles
        .iter()
        .map(|v| {
            let mut veh = ps_sim::sim::Vehicle::new(v.id, v.kind, v.team, v.pos, 0.0);
            veh.motion.rot = v.rot;
            (veh.motion, v.kind.spec().half, v.team)
        })
        .collect();
    if let Some((f, at, _)) = game.world.ray(a, b) {
        // (The vehicles' boxes in the client's world aren't the server's walls: their hulls
        // below are.)
        let on_vehicle = vehicles.iter().any(|(m, half, _)| {
            ps_sim::vehicle::hit_hull(
                m,
                *half + Vec3::splat(0.3),
                at - seg.normalize_or(Vec3::Y) * 0.5,
                at + seg.normalize_or(Vec3::Y) * 0.5,
            )
            .is_some()
        });
        if !on_vehicle {
            let shield = game.world.shields.iter().any(|(c, _)| {
                let (lo, hi) = c.bounds();
                at.cmpge(lo - 0.05).all() && at.cmple(hi + 0.05).all()
            });
            take(
                f,
                if shield {
                    ImpactOn::Shield
                } else {
                    ImpactOn::Ground
                },
            );
        }
    }
    for p in &game.snap.players {
        if p.team == team
            || Some(p.id) == game.me
            || p.seat.is_some()
            || p.has(ps_sim::snapshot::flags::DEAD)
        {
            continue;
        }
        let crouch = p.has(ps_sim::snapshot::flags::CROUCH);
        if let Some((f, _)) = ps_sim::player::hit_body(p.feet, crouch, a, b) {
            take(f, ImpactOn::Body);
        }
    }
    for (m, half, t) in &vehicles {
        if *t == team {
            continue;
        }
        if let Some(f) = ps_sim::vehicle::hit_hull(m, *half, a, b) {
            take(f, ImpactOn::Vehicle);
        }
    }
    if let Some(map) = &game.map {
        for t in &game.snap.turrets {
            if t.health <= 0.0 || t.team.is_none_or(|o| o == team) {
                continue;
            }
            let Some(pos) = map
                .bases
                .get(t.base as usize)
                .and_then(|l| l.turrets.get(t.index as usize))
            else {
                continue;
            };
            let c = *pos + Vec3::Y * ps_sim::base::TURRET_PIVOT;
            let k = ((c - a).dot(seg) / seg.length_squared().max(1e-6)).clamp(0.0, 1.0);
            if (a + seg * k - c).length() < 1.6 {
                take(k, ImpactOn::Vehicle);
            }
        }
    }
    best.map(|(f, on)| (a + seg * f, on))
}

type ProjectileParts = (
    Entity,
    &'static mut Projectile,
    &'static mut Transform,
    Option<&'static Homing>,
    Option<&'static mut Trail>,
);

#[allow(clippy::too_many_arguments)]
fn fly(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut flying: ResMut<Flying>,
    mut local: ResMut<LocalImpacts>,
    mut booms: MessageWriter<Boom>,
    mut q: Query<ProjectileParts>,
    mut budget: ResMut<FxBudget>,
) {
    let Some(fx) = fx else { return };
    let c = &mut Ctx {
        commands: &mut commands,
        fx: &fx,
        budget: &mut budget,
        game: &game,
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    for (e, mut p, mut t, homing, trail) in &mut q {
        p.left -= dt;
        p.vel.y -= p.gravity * dt;
        // A missile turns towards its target (and goes on straight once flares lead it off).
        if let Some(h) = homing
            && let Some(v) = game.vehicle(h.0)
        {
            if v.flags & ps_sim::snapshot::vflags::FLARES != 0 {
                c.commands.entity(e).remove::<Homing>();
            } else {
                let middle = v.pos + v.rot * Vec3::Y * v.kind.spec().half.y;
                p.vel = ps_sim::sim::home(p.vel, middle - t.translation, dt);
            }
        }
        let mut next = t.translation + p.vel * dt;
        // A grenade bounces off the land and what stands on it (the server's word on its new
        // flight follows).
        if p.ammo.ballistics().bounces
            && let Some((_, at, n)) = game.world.ray(t.translation, next)
        {
            let vn = p.vel.dot(n);
            p.vel = (p.vel - n * vn * 1.6) * 0.55;
            next = at + n * 0.05;
        }
        // One's own end on what they meet, with their impact now (the server's word on it
        // comes later, and is passed over then).
        let own = p.id > u32::MAX - 100_001;
        let hit = if own && !p.ammo.ballistics().bounces {
            own_hit(&game, t.translation, next)
        } else {
            None
        };
        if let Some((at, what)) = hit {
            c.commands.entity(e).despawn();
            show_impact(c, &mut booms, at, p.ammo, what, Some(game.team));
            local.0.push_back((at, p.ammo, now));
            continue;
        }
        if p.left <= 0.0 {
            c.commands.entity(e).despawn();
            flying.0.remove(&p.id);
            continue;
        }
        // Its trail: puffs of smoke left along the way.
        if let Some(mut tr) = trail {
            let step = next - t.translation;
            tr.flown += step.length();
            let dir = step.normalize_or(Vec3::NEG_Z);
            let wisp = fx.wisp[0].clone();
            let mut made = 0;
            while tr.flown >= tr.every && tr.left > 0 && made < 4 {
                tr.flown -= tr.every;
                tr.left -= 1;
                made += 1;
                if let Some(mut e) = soft(
                    c,
                    Tier::Body,
                    &wisp,
                    next - dir * tr.flown + random_dir(false) * tr.size * 0.25,
                    random_dir(false) * 0.5 + Vec3::Y * 0.3,
                    -0.2,
                    1.0,
                    tr.life * rnd(0.8, 1.25),
                    (tr.size * 0.5, tr.size * rnd(1.4, 1.9)),
                ) {
                    e.insert(Fade::Wisp);
                }
            }
            // (Not making up later for what a slow frame missed.)
            tr.flown = tr.flown.min(tr.every);
        }
        t.translation = next;
        if p.vel.length_squared() > 1e-4 {
            t.look_to(p.vel.normalize(), Vec3::Y);
        }
    }
}

fn blasts(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Blast, &mut Transform, Option<&mut PointLight>)>,
) {
    let dt = time.delta_secs();
    for (e, mut b, mut t, light) in &mut q {
        b.age += dt;
        let k = (b.age / b.life).min(1.0);
        if k >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        t.scale = Vec3::splat((b.size * (0.3 + 0.7 * k.sqrt())).max(0.01));
        if let Some(mut l) = light {
            l.intensity *= 1.0 - (dt * 6.0).min(0.9);
        }
    }
}

/// Aircraft's flares: while they burn, white-hot sparks thrown out behind and falling.
fn flares(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut next: Local<f32>,
    mut budget: ResMut<FxBudget>,
) {
    let Some(fx) = fx else { return };
    let c = &mut Ctx {
        commands: &mut commands,
        fx: &fx,
        budget: &mut budget,
        game: &game,
    };
    *next -= time.delta_secs();
    if *next > 0.0 {
        return;
    }
    *next = 0.07;
    for v in &game.snap.vehicles {
        if v.flags & ps_sim::snapshot::vflags::FLARES == 0 {
            continue;
        }
        let back = v.pos + v.rot * Vec3::new(0.0, v.kind.spec().half.y, v.kind.spec().half.z);
        for _ in 0..3 {
            let out = Vec3::new(rnd(-1.0, 1.0), rnd(-0.6, 0.4), rnd(0.2, 1.0)).normalize();
            let vel = v.vel * 0.5 + v.rot * out * rnd(10.0, 18.0);
            let size = rnd(0.25, 0.4);
            if let Some(mut e) = particle(
                c,
                Tier::Body,
                &fx.chip,
                back,
                vel,
                6.0,
                0.6,
                1.4,
                (size, 0.05),
            ) {
                e.insert(MeshMaterial3d(fx.hot.clone()));
            }
        }
    }
}

/// The colour of a faction's shots.
pub fn shot_color(team: Option<Team>) -> Vec3 {
    team_color(team)
}

/// An EMP's blue burst, a flash's white one, a smoke grenade's cloud (puffs that hang as long
/// as the sim's cloud does).
fn utility_fx(c: &mut Ctx, at: Vec3, k: ps_sim::weapons::GrenadeKind) {
    let (commands, fx) = (&mut *c.commands, c.fx);
    use ps_sim::weapons::{GrenadeKind, SMOKE_RADIUS, SMOKE_TIME};
    let burst = |commands: &mut Commands, mat: Handle<ParticleMaterial>, size: f32, life: f32| {
        commands.spawn((
            Blast {
                age: 0.0,
                life,
                size,
            },
            Billboard(rnd(0.0, 6.3)),
            Mesh3d(fx.quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(at + Vec3::Y * 0.5).with_scale(Vec3::splat(0.1)),
        ));
    };
    match k {
        GrenadeKind::Frag => {}
        GrenadeKind::Emp => burst(commands, fx.shield.clone(), 7.0, 0.5),
        GrenadeKind::Flash => burst(commands, fx.flash.clone(), 9.0, 0.25),
        GrenadeKind::Smoke => {
            for i in 0..14 {
                let a = i as f32 * 2.4;
                let r = SMOKE_RADIUS * 0.55 * ((i % 4) as f32 / 3.0);
                let off = Vec3::new(a.cos() * r, 0.6 + (i % 3) as f32 * 1.1, a.sin() * r);
                commands.spawn((
                    Blast {
                        age: 0.0,
                        life: SMOKE_TIME,
                        size: SMOKE_RADIUS * 0.75,
                    },
                    Billboard(rnd(0.0, 6.3)),
                    Mesh3d(fx.quad.clone()),
                    MeshMaterial3d(fx.smoke.clone()),
                    Transform::from_translation(at + off).with_scale(Vec3::splat(0.1)),
                ));
            }
        }
    }
}

/// Players' jump jets (flames under their feet while they fire, one's own as predicted) and
/// chutes (a glowing canopy over whoever falls under one).
#[allow(clippy::too_many_arguments)]
fn jets_and_chutes(
    mut commands: Commands,
    game: Res<Game>,
    mover: Res<crate::game::Mover>,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut next: Local<f32>,
    mut canopies: Local<std::collections::HashMap<PlayerId, Entity>>,
    mut transforms: Query<&mut Transform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut canopy_mesh: Local<Option<Handle<Mesh>>>,
    mut budget: ResMut<FxBudget>,
) {
    use ps_sim::snapshot::flags;
    let Some(fx) = fx else { return };
    let mesh = canopy_mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.4).mesh().ico(1).unwrap()))
        .clone();
    let me = game.me;
    let feet_of = |p: &ps_sim::snapshot::PlayerSnap| {
        if Some(p.id) == me && mover.valid {
            mover.body.feet
        } else {
            p.feet
        }
    };
    // Flames.
    *next -= time.delta_secs();
    if *next <= 0.0 {
        *next = 0.05;
        for p in &game.snap.players {
            let jetting = if Some(p.id) == me && mover.valid {
                mover.body.jetting
            } else {
                p.has(flags::JET)
            };
            if !jetting || p.seat.is_some() {
                continue;
            }
            let at = feet_of(p) + Vec3::Y * 0.9;
            for side in [-0.18f32, 0.18] {
                let back = Quat::from_rotation_y(p.yaw) * Vec3::new(side, 0.0, 0.2);
                let vel = Vec3::new(rnd(-0.6, 0.6), rnd(-7.0, -4.0), rnd(-0.6, 0.6));
                let c = &mut Ctx {
                    commands: &mut commands,
                    fx: &fx,
                    budget: &mut budget,
                    game: &game,
                };
                if let Some(mut e) = particle(
                    c,
                    Tier::Garnish,
                    &fx.chip,
                    at + back,
                    vel,
                    0.0,
                    0.25,
                    0.2,
                    (rnd(0.06, 0.1), 0.02),
                ) {
                    e.insert(MeshMaterial3d(fx.hot.clone()));
                }
            }
        }
    }
    // Canopies.
    let chuting: Vec<(PlayerId, Vec3)> = game
        .snap
        .players
        .iter()
        .filter(|p| {
            if Some(p.id) == me && mover.valid {
                mover.body.chute
            } else {
                p.has(flags::CHUTE)
            }
        })
        .map(|p| (p.id, feet_of(p)))
        .collect();
    canopies.retain(|id, e| {
        let keep = chuting.iter().any(|c| c.0 == *id);
        if !keep {
            commands.entity(*e).try_despawn();
        }
        keep
    });
    for (id, feet) in chuting {
        let at = feet + Vec3::Y * 3.4;
        let e = *canopies.entry(id).or_insert_with(|| {
            commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(fx.hot.clone()),
                    Transform::from_translation(at).with_scale(Vec3::new(1.0, 0.35, 1.0)),
                ))
                .id()
        });
        if let Ok(mut t) = transforms.get_mut(e) {
            t.translation = at;
        }
    }
}
