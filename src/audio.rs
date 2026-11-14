//! Sounds (assets/sounds, synthesized by tools/gen_sfx.py): shots, impacts and explosions in
//! space around the listener, vehicles' engines (loops following them), footsteps, and the
//! world's news (bases captured and lost, hacks), the local player's hits and kills.
//!
//! Spatial sounds fade with the square of the distance in units of `SPATIAL_SCALE` metres'
//! reciprocal; loud ones (cannons, explosions) carry further with their own scale.

use std::collections::HashMap;

use bevy::audio::{DefaultSpatialScale, PlaybackMode, SpatialScale, Volume};
use bevy::prelude::*;
use ps_sim::snapshot::ImpactOn;
use ps_sim::state::Change;
use ps_sim::vehicle::VehicleKind;
use ps_sim::weapons::{Ammo, Weapon};
use ps_sim::{PlayerId, SimEvent, VehicleId};

use crate::fx::{Boom, LocalShots};
use crate::game::{FxEvent, Game, GameEvent, WorldChange};
use crate::settings::Settings;

macro_rules! sounds {
    ($($f:ident),* $(,)?) => {
        #[derive(Resource)]
        #[allow(dead_code)]
        struct Sounds {
            $($f: Handle<AudioSource>,)*
            steps: [Handle<AudioSource>; 3],
        }
        fn load(mut commands: Commands, assets: Res<AssetServer>) {
            let s = |n: &str| assets.load(format!("sounds/{n}.ogg"));
            commands.insert_resource(Sounds {
                $($f: s(stringify!($f)),)*
                steps: [s("footstep_1"), s("footstep_2"), s("footstep_3")],
            });
        }
    };
}

sounds!(
    rifle_fire,
    shotgun,
    lance_crack,
    striker_fire,
    pistol_fire,
    medgun_loop,
    reload_long,
    grenade_throw,
    grenade_bounce,
    cannon_light,
    cannon_heavy,
    flak_fire,
    airburst,
    rocket_fire,
    chaingun,
    missile_fire,
    air_cannon,
    lock_tone,
    lock_warning,
    flares,
    cloak,
    kit_down,
    hack_done,
    silo_warning,
    reload,
    reload_done,
    empty,
    terminal_select,
    sensor_pulse,
    jet_thrust,
    chute_open,
    turret_wall_fire,
    explosion_small,
    explosion_large,
    impact_armor,
    impact_ground,
    hitmarker,
    kill_confirm,
    hurt,
    death,
    spawn,
    heal,
    fall_thud,
    deny,
    vehicle_spawn,
    deploy,
    undeploy,
    warp,
    hack_start,
    hack_stopped,
    hack_tick,
    base_captured,
    base_lost,
    continent_captured,
    alert,
    terminal_open,
    engine_light_tank,
    engine_heavy_tank,
    engine_bus,
    engine_vtol,
    puzzle_solved,
    piece_slide,
);

const SPATIAL_SCALE: f32 = 0.12;

/// The listener for the camera or head (ears 20 cm apart; swapped: rodio pans backwards).
pub fn listener() -> SpatialListener {
    let ears = SpatialListener::new(0.2);
    SpatialListener {
        left_ear_offset: ears.right_ear_offset,
        right_ear_offset: ears.left_ear_offset,
    }
}

/// An engine loop following vehicle `.0`.
#[derive(Component)]
struct Engine(VehicleId);

#[derive(Resource, Default)]
struct Walked {
    last: HashMap<PlayerId, (Vec3, f32)>,
    n: usize,
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Walked>()
            .insert_resource(DefaultSpatialScale(SpatialScale::new(SPATIAL_SCALE)))
            .add_systems(Startup, load)
            .add_systems(
                Update,
                (
                    shots, booms, events, changes, footsteps, engines, locks, cues,
                )
                    .after(crate::fx::FxSet)
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, stop_engines.run_if(not(resource_exists::<Game>)));
    }
}

/// When the next lock beep and warning may go, and the aircraft whose flares were burning.
#[derive(Default)]
struct LockSounds {
    beep: f32,
    warn: f32,
    flaring: Vec<VehicleId>,
    /// Players cloaked in the last snapshot (a shimmer as one cloaks or shows again).
    cloaked: Vec<ps_sim::PlayerId>,
}

/// Lock-ons: a beep while one's launcher locks (quicker once locked), a warning while an enemy
/// locks on to one's vehicle (more urgent with a missile coming); aircraft's flares going.
fn locks(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    time: Res<Time>,
    mut st: Local<LockSounds>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let now = time.elapsed_secs();
    let me = game.mine().filter(|m| m.dead.is_none());
    if let Some((_, k)) = me.and_then(|m| m.lock)
        && now >= st.beep
    {
        let locked = k >= 1.0;
        play(&mut commands, gain, &s.lock_tone, None, 0.5, 1.0);
        st.beep = now + if locked { 0.25 } else { 0.5 };
    }
    let warning = me.map_or(0, |m| m.warning);
    if warning > 0 && now >= st.warn {
        play(&mut commands, gain, &s.lock_warning, None, 0.7, 1.0);
        st.warn = now + if warning == 2 { 0.6 } else { 1.2 };
    }
    let flaring: Vec<VehicleId> = game
        .snap
        .vehicles
        .iter()
        .filter(|v| v.flags & ps_sim::snapshot::vflags::FLARES != 0)
        .map(|v| v.id)
        .collect();
    for v in &game.snap.vehicles {
        if flaring.contains(&v.id) && !st.flaring.contains(&v.id) {
            play(&mut commands, gain, &s.flares, Some(v.pos), 0.9, 40.0);
        }
    }
    st.flaring = flaring;
    let cloaked: Vec<ps_sim::PlayerId> = game
        .snap
        .players
        .iter()
        .filter(|p| p.has(ps_sim::snapshot::flags::CLOAKED))
        .map(|p| p.id)
        .collect();
    for p in &game.snap.players {
        if cloaked.contains(&p.id) != st.cloaked.contains(&p.id) {
            let at = (Some(p.id) != game.me).then_some(p.feet);
            play(&mut commands, gain, &s.cloak, at, 0.7, 15.0);
        }
    }
    st.cloaked = cloaked;
}

/// Plays `sound` at `at` (none: in the head), heard up to about `range` metres at full.
fn play(
    commands: &mut Commands,
    gain: f32,
    sound: &Handle<AudioSource>,
    at: Option<Vec3>,
    volume: f32,
    range: f32,
) {
    commands.spawn((
        AudioPlayer(sound.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(volume * gain),
            spatial: at.is_some(),
            spatial_scale: Some(SpatialScale::new(1.0 / range.max(1.0))),
            speed: 0.95 + rand::random::<f32>() * 0.1,
            ..default()
        },
        Transform::from_translation(at.unwrap_or_default()),
    ));
}

fn fire_sound(s: &Sounds, ammo: Ammo) -> (&Handle<AudioSource>, f32, f32) {
    match ammo {
        Ammo::Gun(Weapon::Rifle) => (&s.rifle_fire, 0.7, 12.0),
        Ammo::Gun(Weapon::Scatter) => (&s.shotgun, 0.9, 14.0),
        Ammo::Gun(Weapon::Lance) => (&s.lance_crack, 1.0, 30.0),
        Ammo::Gun(Weapon::Striker) => (&s.striker_fire, 0.9, 20.0),
        Ammo::Gun(Weapon::Pistol) => (&s.pistol_fire, 0.7, 10.0),
        Ammo::Gun(Weapon::Medgun) | Ammo::Gun(Weapon::RepairGun) => (&s.medgun_loop, 0.3, 6.0),
        Ammo::Grenade => (&s.grenade_throw, 0.6, 6.0),
        Ammo::LightShell => (&s.cannon_light, 1.0, 40.0),
        Ammo::HeavyShell => (&s.cannon_heavy, 1.0, 50.0),
        Ammo::Flak => (&s.flak_fire, 0.6, 25.0),
        Ammo::Rocket => (&s.rocket_fire, 0.8, 30.0),
        Ammo::TurretBolt => (&s.turret_wall_fire, 0.7, 20.0),
        Ammo::Chaingun => (&s.chaingun, 0.7, 20.0),
        Ammo::AirCannon => (&s.air_cannon, 0.7, 25.0),
        Ammo::Missile | Ammo::Gun(Weapon::Skyhook) => (&s.missile_fire, 0.9, 25.0),
        Ammo::Gun(Weapon::Pounder) => (&s.chaingun, 0.8, 18.0),
        Ammo::Gun(Weapon::Mauler) => (&s.cannon_light, 0.7, 30.0),
        Ammo::Gun(Weapon::Burster) => (&s.flak_fire, 0.6, 25.0),
        Ammo::Spitfire => (&s.chaingun, 0.4, 12.0),
        Ammo::Mine => (&s.rocket_fire, 0.0, 1.0),
        Ammo::Utility(_) => (&s.grenade_throw, 0.6, 6.0),
        Ammo::Autocannon => (&s.air_cannon, 0.8, 25.0),
        Ammo::Rail => (&s.lance_crack, 1.0, 45.0),
        Ammo::Gun(Weapon::Ember) => (&s.rifle_fire, 0.8, 13.0),
        Ammo::Gun(Weapon::Volt) => (&s.rifle_fire, 0.65, 12.0),
        Ammo::Gun(Weapon::Hive) => (&s.rifle_fire, 0.6, 11.0),
    }
}

fn shots(
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    local: Res<LocalShots>,
    mut fx: MessageReader<FxEvent>,
    mut grenades: Local<std::collections::VecDeque<u32>>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    for &(ammo, _) in &local.fired {
        let (h, v, _) = fire_sound(&s, ammo);
        play(&mut commands, gain, h, None, v * 0.8, 1.0);
    }
    for FxEvent(f) in fx.read() {
        if let ps_sim::snapshot::Fx::Shot { id, ammo, from, .. } = *f {
            // (A grenade's flight sent again after a bounce: the bounce, not the throw.)
            if ammo.ballistics().bounces {
                if grenades.contains(&id) {
                    play(
                        &mut commands,
                        gain,
                        &s.grenade_bounce,
                        Some(from),
                        0.6,
                        10.0,
                    );
                    continue;
                }
                grenades.push_back(id);
                if grenades.len() > 64 {
                    grenades.pop_front();
                }
            }
            let (h, v, range) = fire_sound(&s, ammo);
            play(&mut commands, gain, h, Some(from), v, range);
        }
    }
}

fn booms(
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    mut booms: MessageReader<Boom>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    for b in booms.read() {
        if b.wreck.is_some() {
            play(
                &mut commands,
                gain,
                &s.explosion_large,
                Some(b.at),
                1.0,
                80.0,
            );
            continue;
        }
        let Some(ammo) = b.ammo else { continue };
        let bal = ammo.ballistics();
        if bal.splash > 0.0 {
            let big = bal.splash >= 5.0;
            let (h, v, r) = match (ammo, big) {
                (Ammo::Flak, _) => (&s.airburst, 0.7, 30.0),
                (_, true) => (&s.explosion_large, 0.8, 50.0),
                _ => (&s.explosion_small, 0.9, 35.0),
            };
            play(&mut commands, gain, h, Some(b.at), v, r);
        } else {
            let h = match b.on {
                ImpactOn::Vehicle => &s.impact_armor,
                _ => &s.impact_ground,
            };
            play(&mut commands, gain, h, Some(b.at), 0.4, 6.0);
        }
    }
}

fn events(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    mut events: MessageReader<GameEvent>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let me = game.me;
    for GameEvent(e) in events.read() {
        match e {
            SimEvent::HitConfirm { by, kill, .. } if Some(*by) == me => {
                let h = if *kill { &s.kill_confirm } else { &s.hitmarker };
                play(&mut commands, gain, h, None, 0.6, 1.0);
            }
            SimEvent::Hurt { victim, amount, .. } if Some(*victim) == me => {
                play(
                    &mut commands,
                    gain,
                    &s.hurt,
                    None,
                    (amount / 30.0).clamp(0.3, 1.0),
                    1.0,
                );
            }
            SimEvent::Killed { victim, .. } if Some(*victim) == me => {
                play(&mut commands, gain, &s.death, None, 0.9, 1.0);
            }
            SimEvent::Spawned { player, .. } if Some(*player) == me => {
                play(&mut commands, gain, &s.spawn, None, 0.6, 1.0);
            }
            SimEvent::Healed { .. } => play(&mut commands, gain, &s.heal, None, 0.7, 1.0),
            SimEvent::Revived { player, by } if Some(*player) == me || Some(*by) == me => {
                play(&mut commands, gain, &s.heal, None, 0.8, 0.8)
            }
            SimEvent::LoadoutTaken { .. } => {
                play(&mut commands, gain, &s.terminal_open, None, 0.7, 1.0)
            }
            SimEvent::Denied(_) => play(&mut commands, gain, &s.deny, None, 0.6, 1.0),
            SimEvent::Deployed { at, by, .. } => {
                let at = (Some(*by) != me).then_some(*at);
                play(&mut commands, gain, &s.kit_down, at, 0.8, 15.0);
            }
            SimEvent::Built { at, .. } => {
                play(&mut commands, gain, &s.kit_down, Some(*at), 1.0, 30.0)
            }
            SimEvent::SiloLow { at, .. } => {
                play(&mut commands, gain, &s.silo_warning, Some(*at), 0.9, 60.0);
                play(&mut commands, gain, &s.silo_warning, None, 0.35, 1.0);
            }
            SimEvent::Spotted { by, .. } => {
                let v = if Some(*by) == me { 0.6 } else { 0.25 };
                play(&mut commands, gain, &s.sensor_pulse, None, v, 1.0);
            }
            SimEvent::Hacked { by, .. } if Some(*by) == me => {
                play(&mut commands, gain, &s.hack_done, None, 0.8, 1.0)
            }
            SimEvent::VehicleSpawned { .. } => {
                play(&mut commands, gain, &s.vehicle_spawn, None, 0.7, 1.0)
            }
            SimEvent::Puzzle { .. } => play(&mut commands, gain, &s.piece_slide, None, 0.4, 1.0),
            SimEvent::ConsoleSolved { .. } => {
                play(&mut commands, gain, &s.puzzle_solved, None, 0.7, 1.0)
            }
            _ => {}
        }
    }
}

/// The world's news: one's own faction's bases and continents won and lost, hacks.
fn changes(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    mut changes: MessageReader<WorldChange>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let here = game.continent;
    for WorldChange(c) in changes.read() {
        match *c {
            Change::ContinentCaptured { team, from, .. } => {
                if team == game.team {
                    play(&mut commands, gain, &s.continent_captured, None, 0.8, 1.0);
                } else if from == Some(game.team) {
                    play(&mut commands, gain, &s.base_lost, None, 0.8, 1.0);
                }
            }
            Change::BaseCaptured {
                continent,
                team,
                from,
                ..
            } if Some(continent) == here => {
                if team == game.team {
                    play(&mut commands, gain, &s.base_captured, None, 0.7, 1.0);
                } else if from == Some(game.team) {
                    play(&mut commands, gain, &s.base_lost, None, 0.7, 1.0);
                }
            }
            Change::HackStarted {
                continent,
                team,
                base,
            } => {
                let ours = game.state.base(continent, base).owner == Some(game.team);
                if ours {
                    play(&mut commands, gain, &s.alert, None, 0.7, 1.0);
                } else if team == game.team && Some(continent) == here {
                    play(&mut commands, gain, &s.hack_start, None, 0.7, 1.0);
                }
            }
            Change::HackStopped { continent, .. } if Some(continent) == here => {
                play(&mut commands, gain, &s.hack_stopped, None, 0.6, 1.0);
            }
            _ => {}
        }
    }
}

fn footsteps(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    mut walked: ResMut<Walked>,
    settings: Option<Res<Settings>>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let walked = &mut *walked;
    for p in &game.snap.players {
        if p.seat.is_some() || p.has(ps_sim::snapshot::flags::DEAD) {
            continue;
        }
        let entry = walked.last.entry(p.id).or_insert((p.feet, 0.0));
        let d = Vec2::new(p.feet.x - entry.0.x, p.feet.z - entry.0.z).length();
        let step = if d < 1.5 && (p.feet.y - entry.0.y).abs() < 0.5 {
            d
        } else {
            0.0
        };
        entry.0 = p.feet;
        entry.1 += step;
        if entry.1 > 1.0 && !p.has(ps_sim::snapshot::flags::AIR) {
            entry.1 = 0.0;
            walked.n += 1;
            let mine = Some(p.id) == game.me;
            play(
                &mut commands,
                gain,
                &s.steps[walked.n % 3],
                (!mine).then_some(p.feet),
                if mine { 0.3 } else { 0.6 },
                1.0,
            );
        }
    }
    walked
        .last
        .retain(|id, _| game.snap.players.iter().any(|p| p.id == *id));
}

/// Engine loops on the vehicles seen, louder and higher with speed.
fn engines(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    mut q: Query<(
        Entity,
        &Engine,
        &mut Transform,
        Option<&mut SpatialAudioSink>,
    )>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    for (e, eng, mut t, sink) in &mut q {
        match game.vehicle(eng.0) {
            Some(v) => {
                t.translation = v.pos;
                if let Some(mut sink) = sink {
                    let speed = v.vel.length() / v.kind.spec().max_speed.max(1.0);
                    sink.set_speed(0.8 + speed * 0.5);
                    sink.set_volume(Volume::Linear(gain * (0.35 + speed * 0.5)));
                }
            }
            None => commands.entity(e).despawn(),
        }
    }
    for v in &game.snap.vehicles {
        if q.iter().any(|(_, e, ..)| e.0 == v.id) {
            continue;
        }
        let h = match v.kind {
            VehicleKind::LightTank | VehicleKind::LightAaa => &s.engine_light_tank,
            VehicleKind::HeavyTank => &s.engine_heavy_tank,
            VehicleKind::SpawnBus | VehicleKind::Harvester | VehicleKind::Rover => &s.engine_bus,
            VehicleKind::Vtol | VehicleKind::Interceptor | VehicleKind::Dropship => &s.engine_vtol,
        };
        commands.spawn((
            Engine(v.id),
            AudioPlayer(h.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(0.4 * gain),
                spatial: true,
                spatial_scale: Some(SpatialScale::new(1.0 / 25.0)),
                ..default()
            },
            Transform::from_translation(v.pos),
        ));
    }
}

fn stop_engines(mut commands: Commands, q: Query<Entity, With<Engine>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// What the cues last saw of the player: the gun in hand reloading, its rounds, the trigger,
/// a hack under way, a structure carried; when the dry click and the hack's tick last went; and
/// who else was reloading.
#[derive(Default)]
struct Cues {
    reloading: bool,
    slot: u8,
    trigger: bool,
    click: f32,
    hack_tick: f32,
    carrying: Option<ps_sim::construct::StructureKind>,
    others: Vec<PlayerId>,
    /// When the jets' roar last went (one's own, others'), and who was under a chute.
    jet: f32,
    jets: f32,
    chutes: Vec<PlayerId>,
}

#[allow(clippy::too_many_arguments)]
/// The player's own cues: reloading (starting and done), the trigger on an empty gun, a hack's
/// tick, a structure pulled at a terminal; and others' reloads where they are.
fn cues(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    local: Res<crate::game::LocalCmd>,
    mover: Res<crate::game::Mover>,
    time: Res<Time>,
    mut st: Local<Cues>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let now = time.elapsed_secs();
    let me = game
        .mine()
        .filter(|m| m.dead.is_none() && m.seat.is_none() && m.turret.is_none());
    if let Some(m) = me {
        let slot = m.slot.min(ps_sim::weapons::SLOTS as u8 - 1);
        let g = m.guns[slot as usize];
        let reloading = g.reload > 0.0;
        if slot == st.slot {
            if reloading && !st.reloading {
                let long = g.reload > 2.0;
                let h = if long { &s.reload_long } else { &s.reload };
                play(&mut commands, gain, h, None, 0.6, 1.0);
            } else if !reloading && st.reloading && g.mag > 0 {
                play(&mut commands, gain, &s.reload_done, None, 0.6, 1.0);
            }
        }
        // The trigger on an empty gun (nothing left to reload either): a dry click, not too
        // often.
        let trigger = local.cmd.fire;
        let beam = m.loadout.slot(slot).stats().shot == ps_sim::weapons::ShotKind::Beam;
        if trigger
            && g.mag == 0
            && !reloading
            && !beam
            && (!st.trigger || now - st.click > 0.5)
            && now - st.click > 0.2
        {
            play(&mut commands, gain, &s.empty, None, 0.5, 1.0);
            st.click = now;
        }
        st.trigger = trigger;
        st.reloading = reloading;
        st.slot = slot;
        if m.hack.is_some() && now - st.hack_tick > 0.5 {
            play(&mut commands, gain, &s.hack_tick, None, 0.4, 1.0);
            st.hack_tick = now;
        }
        if m.carrying.is_some() && st.carrying.is_none() {
            play(&mut commands, gain, &s.terminal_select, None, 0.7, 1.0);
        }
        st.carrying = m.carrying;
    } else {
        st.reloading = false;
        st.trigger = false;
        st.carrying = None;
    }
    // Others reloading near.
    let reloading: Vec<PlayerId> = game
        .snap
        .players
        .iter()
        .filter(|p| Some(p.id) != game.me && p.has(ps_sim::snapshot::flags::RELOAD))
        .map(|p| p.id)
        .collect();
    for p in &game.snap.players {
        if reloading.contains(&p.id) && !st.others.contains(&p.id) {
            play(
                &mut commands,
                gain,
                &s.reload,
                Some(p.feet + Vec3::Y),
                0.5,
                8.0,
            );
        }
    }
    st.others = reloading;
    // Jump jets roaring; chutes opening.
    use ps_sim::snapshot::flags;
    if now - st.jets > 0.22 {
        st.jets = now;
        for p in &game.snap.players {
            if Some(p.id) != game.me && p.has(flags::JET) {
                play(&mut commands, gain, &s.jet_thrust, Some(p.feet), 0.7, 20.0);
            }
        }
    }
    if mover.valid && mover.body.jetting && now - st.jet > 0.22 {
        st.jet = now;
        play(&mut commands, gain, &s.jet_thrust, None, 0.5, 1.0);
    }
    let chutes: Vec<PlayerId> = game
        .snap
        .players
        .iter()
        .filter(|p| p.has(flags::CHUTE))
        .map(|p| p.id)
        .collect();
    for p in &game.snap.players {
        if chutes.contains(&p.id) && !st.chutes.contains(&p.id) {
            let at = (Some(p.id) != game.me).then_some(p.feet);
            play(&mut commands, gain, &s.chute_open, at, 0.8, 30.0);
        }
    }
    st.chutes = chutes;
}
