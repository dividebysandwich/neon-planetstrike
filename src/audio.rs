//! Sounds (assets/sounds, synthesized by tools/gen_sfx.py): shots, impacts and explosions in
//! space around the listener, vehicles' engines (loops following them), footsteps, and the
//! world's news (bases captured and lost, hacks), the local player's hits and kills.
//!
//! Spatial sounds fade with the square of the distance in units of `SPATIAL_SCALE` metres'
//! reciprocal; loud ones (cannons, explosions) carry further with their own scale. Shots and
//! blasts fade over to a dull, rolling "tail" with distance (`Out::layered`), so a battle is
//! heard from afar. One-shots go through `Out::play`, which keeps to `MAX_VOICES` at a time (the
//! least important, farthest, oldest making way), drops what is too far to hear and the same
//! sound twice at once.

use std::collections::HashMap;

use bevy::audio::{DefaultSpatialScale, PlaybackMode, SpatialScale, Volume};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use ps_sim::snapshot::{ImpactOn, flags};
use ps_sim::state::Change;
use ps_sim::vehicle::VehicleKind;
use ps_sim::weapons::{Ammo, Weapon};
use ps_sim::{PlayerId, SimEvent, VehicleId};

use crate::fx::{Boom, LocalShots};
use crate::game::{FxEvent, Game, GameEvent, WorldChange};
use crate::settings::Settings;

/// `sounds!(one: a, b; sets: c[3])`: the `Sounds` resource: `a`, `b` from sounds/a.ogg, ...; `c`
/// the variants sounds/c_1.ogg .. c_3.ogg (one picked at random each time, `pick`).
macro_rules! sounds {
    (one: $($f:ident),* $(,)?; sets: $($v:ident[$n:literal]),* $(,)?) => {
        #[derive(Resource)]
        #[allow(dead_code)]
        struct Sounds {
            $($f: Handle<AudioSource>,)*
            $($v: [Handle<AudioSource>; $n],)*
        }
        fn load(mut commands: Commands, assets: Res<AssetServer>) {
            let s = |n: &str| assets.load(format!("sounds/{n}.ogg"));
            commands.insert_resource(Sounds {
                $($f: s(stringify!($f)),)*
                $($v: std::array::from_fn(|i| s(&format!("{}_{}", stringify!($v), i + 1))),)*
            });
        }
    };
}

sounds!(
    one:
    rifle_fire,
    ember_fire,
    volt_fire,
    hive_fire,
    scatter_fire,
    lance_crack,
    striker_fire,
    skyhook_fire,
    pistol_fire,
    medgun_loop,
    repair_loop,
    pounder_fire,
    mauler_fire,
    burster_fire,
    grenade_throw,
    grenade_bounce,
    cannon_light,
    cannon_heavy,
    rail_fire,
    autocannon_fire,
    flak_fire,
    airburst,
    rocket_fire,
    chaingun,
    missile_fire,
    air_cannon,
    turret_wall_fire,
    spitfire_fire,
    tail_small,
    tail_medium,
    tail_heavy,
    lock_tone,
    lock_warning,
    flares,
    cloak,
    kit_down,
    hack_done,
    silo_warning,
    reload_rifle,
    reload_scatter,
    reload_lance,
    reload_launcher,
    reload_pistol,
    reload_heavy,
    reload_done,
    empty,
    terminal_select,
    sensor_pulse,
    jet_thrust,
    chute_open,
    explosion_small,
    explosion_medium,
    explosion_large,
    explosion_wreck,
    impact_shield,
    whiz_heavy,
    casing_shell,
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
    piece_slide;
    sets:
    footstep[3],
    impact_body[2],
    impact_armor[2],
    impact_ground[3],
    impact_metal[2],
    ricochet[3],
    bullet_whiz[3],
    casing[2],
);

/// One of a sound's variants, at random.
fn pick<const N: usize>(set: &[Handle<AudioSource>; N]) -> &Handle<AudioSource> {
    &set[rand::random_range(0..N)]
}

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

/// A beam's loop on player `.0` (`.1`: the repair gun's, else the heal gun's).
#[derive(Component)]
struct BeamLoop(PlayerId, bool);

#[derive(Resource, Default)]
struct Walked {
    last: HashMap<PlayerId, (Vec3, f32)>,
    n: usize,
}

/// A near miss: a projectile passed the listener close by, nearest at `at` (`heavy`: a shell or
/// a rocket, not a bolt). Written by the effects, played here (`whizzes`).
#[derive(Message, Clone, Copy, Debug)]
pub struct Whiz {
    pub at: Vec3,
    pub heavy: bool,
}

/// A spent case landed at `at` (`shell`: a cannon's, not a gun's). Written by the effects,
/// played here (`casings`).
#[derive(Message, Clone, Copy, Debug)]
pub struct Casing {
    pub at: Vec3,
    pub shell: bool,
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Walked>()
            .init_resource::<Voices>()
            .add_message::<Whiz>()
            .add_message::<Casing>()
            .insert_resource(DefaultSpatialScale(SpatialScale::new(SPATIAL_SCALE)))
            .add_systems(Startup, load)
            .add_systems(
                Update,
                (
                    hear,
                    (
                        shots, booms, events, changes, footsteps, engines, beams, locks, cues,
                        whizzes, casings,
                    ),
                )
                    .chain()
                    .after(crate::fx::FxSet)
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, stop_loops.run_if(not(resource_exists::<Game>)));
    }
}

/// How many one-shot sounds play at once at most.
const MAX_VOICES: usize = 40;
/// Sounds further than this many times their range aren't played.
const CULL: f32 = 5.0;
/// The same sound again within this long (s) and this near (m) isn't played twice.
const DEDUPE: (f32, f32) = (0.035, 2.5);
/// The same for the far-off tails (many guns firing far away: one tail does for them).
const DEDUPE_TAIL: (f32, f32) = (0.12, 15.0);

/// How much a sound matters when there are too many: the least first to go.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Prio {
    Ambient,
    Impact,
    Fire,
    Explosion,
    /// The player's own sounds and the interface's: never dropped for others'.
    Own,
}

/// A one-shot sound playing (despawned when it ends).
#[derive(Component)]
struct Voice;

struct Live {
    entity: Entity,
    sound: AssetId<AudioSource>,
    at: Option<Vec3>,
    prio: Prio,
    born: f32,
}

/// The one-shots playing, where the listener is and the time (for `Out::play`).
#[derive(Resource, Default)]
struct Voices {
    live: Vec<Live>,
    ears: Vec3,
    now: f32,
}

/// Where the listener is; the voices that ended forgotten.
fn hear(
    mut v: ResMut<Voices>,
    time: Res<Time>,
    ears: Query<&GlobalTransform, With<SpatialListener>>,
    alive: Query<(), With<Voice>>,
) {
    v.now = time.elapsed_secs();
    if let Some(t) = ears.iter().next() {
        v.ears = t.translation();
    }
    let now = v.now;
    // (A voice just spawned isn't in the world yet: its commands are still queued.)
    v.live
        .retain(|l| now - l.born < 0.2 || (alive.contains(l.entity) && now - l.born < 15.0));
}

/// Which voice makes way for a new one of `prio`, `dist` m away, when `live` is full: the
/// least important (the lowest priority, of those the farthest, of those the oldest), unless
/// the new one matters less still.
fn victim(live: &[Live], ears: Vec3, prio: Prio, dist: f32) -> Option<usize> {
    let far = |l: &Live| l.at.map_or(0.0, |p| p.distance(ears));
    let (i, l) = live.iter().enumerate().min_by(|(_, a), (_, b)| {
        a.prio
            .cmp(&b.prio)
            .then(far(b).total_cmp(&far(a)))
            .then(a.born.total_cmp(&b.born))
    })?;
    (l.prio < prio || (l.prio == prio && far(l) >= dist)).then_some(i)
}

/// The one-shot sounds' way out: `play`.
#[derive(SystemParam)]
struct Out<'w, 's> {
    commands: Commands<'w, 's>,
    voices: ResMut<'w, Voices>,
    settings: Option<Res<'w, Settings>>,
}

impl Out<'_, '_> {
    fn gain(&self) -> f32 {
        crate::settings::sound_gain(self.settings.as_deref())
    }

    /// Plays `sound` at `at` (none: in the head), heard up to about `range` metres at full.
    /// False if it wasn't played (too far, a duplicate, or too many more important ones).
    fn play(
        &mut self,
        sound: &Handle<AudioSource>,
        at: Option<Vec3>,
        volume: f32,
        range: f32,
        prio: Prio,
    ) -> bool {
        self.play_once(sound, at, volume, range, prio, DEDUPE)
    }

    /// `play`, but not the same sound twice within `dedupe` (s, m).
    fn play_once(
        &mut self,
        sound: &Handle<AudioSource>,
        at: Option<Vec3>,
        volume: f32,
        range: f32,
        prio: Prio,
        dedupe: (f32, f32),
    ) -> bool {
        let gain = self.gain();
        let v = &mut *self.voices;
        let range = range.max(1.0);
        let dist = at.map_or(0.0, |p| p.distance(v.ears));
        if volume <= 0.0 || dist > range * CULL {
            return false;
        }
        let id = sound.id();
        let now = v.now;
        let same = |l: &Live| {
            l.sound == id
                && now - l.born < dedupe.0
                && match (l.at, at) {
                    (Some(a), Some(b)) => a.distance(b) < dedupe.1,
                    (None, None) => true,
                    _ => false,
                }
        };
        if v.live.iter().any(same) {
            return false;
        }
        if v.live.len() >= MAX_VOICES {
            let Some(i) = victim(&v.live, v.ears, prio, dist) else {
                return false;
            };
            self.commands.entity(v.live[i].entity).try_despawn();
            v.live.swap_remove(i);
        }
        let entity = self
            .commands
            .spawn((
                Voice,
                AudioPlayer(sound.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(volume * gain),
                    spatial: at.is_some(),
                    spatial_scale: Some(SpatialScale::new(1.0 / range)),
                    speed: 0.95 + rand::random::<f32>() * 0.1,
                    ..default()
                },
                Transform::from_translation(at.unwrap_or_default()),
            ))
            .id();
        v.live.push(Live {
            entity,
            sound: id,
            at,
            prio,
            born: now,
        });
        true
    }

    /// A shot or blast at `at`: its own sound `near` by, fading over to its far-off `tail` with
    /// the distance (from half its `range`, all tail from twice it; the tail carries four times
    /// as far).
    fn layered(
        &mut self,
        near: &Handle<AudioSource>,
        tail: Option<&Handle<AudioSource>>,
        at: Vec3,
        volume: f32,
        range: f32,
        prio: Prio,
    ) {
        let k = match tail {
            Some(_) => far_mix(at.distance(self.voices.ears), range),
            None => 0.0,
        };
        if volume * (1.0 - k) > 0.02 {
            self.play(near, Some(at), volume * (1.0 - k), range, prio);
        }
        if let Some(tail) = tail
            && volume * k > 0.02
        {
            let prio = prio.min(Prio::Impact);
            self.play_once(tail, Some(at), volume * k, range * 4.0, prio, DEDUPE_TAIL);
        }
    }
}

/// How much of a shot's far-off tail is heard `dist` m away (the rest its own sound): none up
/// to half its `range`, all from twice it.
fn far_mix(dist: f32, range: f32) -> f32 {
    let t = ((dist - 0.5 * range) / (1.5 * range)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    time: Res<Time>,
    mut st: Local<LockSounds>,
) {
    let Some(s) = sounds else { return };
    let now = time.elapsed_secs();
    let me = game.mine().filter(|m| m.dead.is_none());
    if let Some((_, k)) = me.and_then(|m| m.lock)
        && now >= st.beep
    {
        let locked = k >= 1.0;
        out.play(&s.lock_tone, None, 0.5, 1.0, Prio::Own);
        st.beep = now + if locked { 0.25 } else { 0.5 };
    }
    let warning = me.map_or(0, |m| m.warning);
    if warning > 0 && now >= st.warn {
        out.play(&s.lock_warning, None, 0.7, 1.0, Prio::Own);
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
            out.play(&s.flares, Some(v.pos), 0.9, 40.0, Prio::Fire);
        }
    }
    st.flaring = flaring;
    let cloaked: Vec<ps_sim::PlayerId> = game
        .snap
        .players
        .iter()
        .filter(|p| p.has(flags::CLOAKED))
        .map(|p| p.id)
        .collect();
    for p in &game.snap.players {
        if cloaked.contains(&p.id) != st.cloaked.contains(&p.id) {
            let mine = Some(p.id) == game.me;
            let at = (!mine).then_some(p.feet);
            let prio = if mine { Prio::Own } else { Prio::Ambient };
            out.play(&s.cloak, at, 0.7, 15.0, prio);
        }
    }
    st.cloaked = cloaked;
}

/// A shot's far-off sound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tail {
    /// Not heard from afar (a thrown grenade, a beam).
    No,
    Small,
    Medium,
    Heavy,
}

impl Tail {
    fn sound(self, s: &Sounds) -> Option<&Handle<AudioSource>> {
        match self {
            Tail::No => None,
            Tail::Small => Some(&s.tail_small),
            Tail::Medium => Some(&s.tail_medium),
            Tail::Heavy => Some(&s.tail_heavy),
        }
    }
}

/// An ammo's shot: its sound, its far-off tail, how loud, and the metres it is heard at full.
fn fire_sound(s: &Sounds, ammo: Ammo) -> (&Handle<AudioSource>, Tail, f32, f32) {
    use Tail::*;
    match ammo {
        Ammo::Gun(Weapon::Rifle) => (&s.rifle_fire, Small, 0.7, 12.0),
        Ammo::Gun(Weapon::Ember) => (&s.ember_fire, Small, 0.75, 14.0),
        Ammo::Gun(Weapon::Volt) => (&s.volt_fire, Small, 0.7, 12.0),
        Ammo::Gun(Weapon::Hive) => (&s.hive_fire, Small, 0.7, 11.0),
        Ammo::Gun(Weapon::Scatter) => (&s.scatter_fire, Small, 0.9, 16.0),
        Ammo::Gun(Weapon::Lance) => (&s.lance_crack, Medium, 1.0, 30.0),
        Ammo::Gun(Weapon::Striker) => (&s.striker_fire, Medium, 0.9, 20.0),
        Ammo::Gun(Weapon::Skyhook) => (&s.skyhook_fire, Medium, 0.9, 22.0),
        Ammo::Gun(Weapon::Pistol) => (&s.pistol_fire, Small, 0.7, 10.0),
        // (The beams' loops follow their players: `beams`.)
        Ammo::Gun(Weapon::Medgun) | Ammo::Gun(Weapon::RepairGun) => (&s.medgun_loop, No, 0.0, 1.0),
        Ammo::Gun(Weapon::Pounder) => (&s.pounder_fire, Small, 0.8, 18.0),
        Ammo::Gun(Weapon::Mauler) => (&s.mauler_fire, Medium, 0.9, 30.0),
        Ammo::Gun(Weapon::Burster) => (&s.burster_fire, Small, 0.75, 25.0),
        Ammo::Grenade | Ammo::Utility(_) => (&s.grenade_throw, No, 0.6, 6.0),
        Ammo::LightShell => (&s.cannon_light, Medium, 1.0, 40.0),
        Ammo::HeavyShell => (&s.cannon_heavy, Heavy, 1.0, 50.0),
        Ammo::Rail => (&s.rail_fire, Heavy, 1.0, 50.0),
        Ammo::Autocannon => (&s.autocannon_fire, Small, 0.8, 28.0),
        Ammo::Flak => (&s.flak_fire, Small, 0.7, 28.0),
        Ammo::Rocket => (&s.rocket_fire, Medium, 0.8, 30.0),
        Ammo::Missile => (&s.missile_fire, Medium, 0.9, 28.0),
        Ammo::AirCannon => (&s.air_cannon, Small, 0.75, 25.0),
        Ammo::Chaingun => (&s.chaingun, Small, 0.7, 20.0),
        Ammo::TurretBolt => (&s.turret_wall_fire, Medium, 0.75, 24.0),
        Ammo::Spitfire => (&s.spitfire_fire, No, 0.6, 12.0),
        // (A mine doesn't fire: its blast is its sound.)
        Ammo::Mine => (&s.rocket_fire, No, 0.0, 1.0),
    }
}

fn shots(
    mut out: Out,
    sounds: Option<Res<Sounds>>,
    local: Res<LocalShots>,
    mut fx: MessageReader<FxEvent>,
    mut grenades: Local<std::collections::VecDeque<u32>>,
) {
    let Some(s) = sounds else { return };
    // One's own: in the head; the big guns' tails rolling back from the land.
    let mut mine: Vec<Ammo> = Vec::new();
    for &(ammo, _) in &local.fired {
        if mine.contains(&ammo) {
            continue;
        }
        mine.push(ammo);
        let (h, tail, v, _) = fire_sound(&s, ammo);
        out.play(h, None, v * 0.8, 1.0, Prio::Own);
        if let (Tail::Medium | Tail::Heavy, Some(t)) = (tail, tail.sound(&s)) {
            out.play_once(t, None, v * 0.25, 1.0, Prio::Own, DEDUPE_TAIL);
        }
    }
    // Others': one sound a shot, however many projectiles it was (a scatter gun's pellets each
    // come as a `Shot`).
    let mut theirs: Vec<(PlayerId, Ammo)> = Vec::new();
    for FxEvent(f) in fx.read() {
        if let ps_sim::snapshot::Fx::Shot {
            id,
            owner,
            ammo,
            from,
            ..
        } = *f
        {
            // (A grenade's flight sent again after a bounce: the bounce, not the throw.)
            if ammo.ballistics().bounces {
                if grenades.contains(&id) {
                    out.play(&s.grenade_bounce, Some(from), 0.6, 10.0, Prio::Ambient);
                    continue;
                }
                grenades.push_back(id);
                if grenades.len() > 64 {
                    grenades.pop_front();
                }
            }
            if theirs.contains(&(owner, ammo)) {
                continue;
            }
            theirs.push((owner, ammo));
            let (h, tail, v, range) = fire_sound(&s, ammo);
            out.layered(h, tail.sound(&s), from, v, range, Prio::Fire);
        }
    }
}

/// Whether a shot that hit "the ground" at `at` hit something built (a wall, a deck, a roof)
/// rather than the land: it is well above the land there.
fn on_structure(game: &Game, at: Vec3) -> bool {
    game.map
        .as_ref()
        .is_some_and(|m| at.y > m.terrain.height(at.x, at.z) + 0.4)
}

fn booms(
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    mut booms: MessageReader<Boom>,
) {
    let Some(s) = sounds else { return };
    for b in booms.read() {
        if b.wreck.is_some() {
            let tail = Some(&s.tail_heavy);
            out.layered(&s.explosion_wreck, tail, b.at, 1.0, 80.0, Prio::Explosion);
            continue;
        }
        let Some(ammo) = b.ammo else { continue };
        let splash = ammo.ballistics().splash;
        if splash > 0.0 {
            // By the blast's size: a small shell's, a rocket's or light shell's, a heavy shell's.
            let (h, tail, v, r) = match ammo {
                Ammo::Flak | Ammo::Gun(Weapon::Burster) => (&s.airburst, &s.tail_small, 0.7, 30.0),
                _ if splash < 3.5 => (&s.explosion_small, &s.tail_medium, 0.9, 35.0),
                _ if splash < 6.0 => (&s.explosion_medium, &s.tail_medium, 0.9, 45.0),
                _ => (&s.explosion_large, &s.tail_heavy, 0.9, 55.0),
            };
            out.layered(h, Some(tail), b.at, v, r, Prio::Explosion);
            continue;
        }
        // By what it hit.
        let h = match b.on {
            ImpactOn::Vehicle => pick(&s.impact_armor),
            ImpactOn::Body => pick(&s.impact_body),
            ImpactOn::Shield => &s.impact_shield,
            ImpactOn::Ground if on_structure(&game, b.at) => {
                if rand::random::<f32>() < 0.2 {
                    out.play(pick(&s.ricochet), Some(b.at), 0.5, 10.0, Prio::Ambient);
                }
                pick(&s.impact_metal)
            }
            ImpactOn::Ground => pick(&s.impact_ground),
            // (Spent in the air: nothing to hear.)
            ImpactOn::Air => continue,
        };
        out.play(h, Some(b.at), 0.5, 8.0, Prio::Impact);
    }
}

fn events(
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    mut events: MessageReader<GameEvent>,
) {
    let Some(s) = sounds else { return };
    let me = game.me;
    for GameEvent(e) in events.read() {
        match e {
            SimEvent::HitConfirm { by, kill, .. } if Some(*by) == me => {
                let h = if *kill { &s.kill_confirm } else { &s.hitmarker };
                out.play(h, None, 0.6, 1.0, Prio::Own);
            }
            SimEvent::Hurt { victim, amount, .. } if Some(*victim) == me => {
                out.play(
                    &s.hurt,
                    None,
                    (amount / 30.0).clamp(0.3, 1.0),
                    1.0,
                    Prio::Own,
                );
            }
            SimEvent::Killed { victim, .. } if Some(*victim) == me => {
                out.play(&s.death, None, 0.9, 1.0, Prio::Own);
            }
            SimEvent::Spawned { player, .. } if Some(*player) == me => {
                out.play(&s.spawn, None, 0.6, 1.0, Prio::Own);
            }
            SimEvent::Healed { .. } => {
                out.play(&s.heal, None, 0.7, 1.0, Prio::Own);
            }
            SimEvent::Revived { player, by } if Some(*player) == me || Some(*by) == me => {
                out.play(&s.heal, None, 0.8, 0.8, Prio::Own);
            }
            SimEvent::LoadoutTaken { .. } => {
                out.play(&s.terminal_open, None, 0.7, 1.0, Prio::Own);
            }
            SimEvent::Denied(_) => {
                out.play(&s.deny, None, 0.6, 1.0, Prio::Own);
            }
            SimEvent::Deployed { at, by, .. } => {
                let mine = Some(*by) == me;
                let prio = if mine { Prio::Own } else { Prio::Ambient };
                out.play(&s.kit_down, (!mine).then_some(*at), 0.8, 15.0, prio);
            }
            SimEvent::Built { at, .. } => {
                out.play(&s.kit_down, Some(*at), 1.0, 30.0, Prio::Ambient);
            }
            SimEvent::SiloLow { at, .. } => {
                out.play(&s.silo_warning, Some(*at), 0.9, 60.0, Prio::Fire);
                out.play(&s.silo_warning, None, 0.35, 1.0, Prio::Own);
            }
            SimEvent::Spotted { by, .. } => {
                let v = if Some(*by) == me { 0.6 } else { 0.25 };
                out.play(&s.sensor_pulse, None, v, 1.0, Prio::Own);
            }
            SimEvent::Hacked { by, .. } if Some(*by) == me => {
                out.play(&s.hack_done, None, 0.8, 1.0, Prio::Own);
            }
            SimEvent::VehicleSpawned { .. } => {
                out.play(&s.vehicle_spawn, None, 0.7, 1.0, Prio::Own);
            }
            SimEvent::Puzzle { .. } => {
                out.play(&s.piece_slide, None, 0.4, 1.0, Prio::Own);
            }
            SimEvent::ConsoleSolved { .. } => {
                out.play(&s.puzzle_solved, None, 0.7, 1.0, Prio::Own);
            }
            _ => {}
        }
    }
}

/// The world's news: one's own faction's bases and continents won and lost, hacks.
fn changes(
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    mut changes: MessageReader<WorldChange>,
) {
    let Some(s) = sounds else { return };
    let here = game.continent;
    let mut news = |h: &Handle<AudioSource>, v: f32| {
        out.play(h, None, v, 1.0, Prio::Own);
    };
    for WorldChange(c) in changes.read() {
        match *c {
            Change::ContinentCaptured { team, from, .. } => {
                if team == game.team {
                    news(&s.continent_captured, 0.8);
                } else if from == Some(game.team) {
                    news(&s.base_lost, 0.8);
                }
            }
            Change::BaseCaptured {
                continent,
                team,
                from,
                ..
            } if Some(continent) == here => {
                if team == game.team {
                    news(&s.base_captured, 0.7);
                } else if from == Some(game.team) {
                    news(&s.base_lost, 0.7);
                }
            }
            Change::HackStarted {
                continent,
                team,
                base,
            } => {
                let ours = game.state.base(continent, base).owner == Some(game.team);
                if ours {
                    news(&s.alert, 0.7);
                } else if team == game.team && Some(continent) == here {
                    news(&s.hack_start, 0.7);
                }
            }
            Change::HackStopped { continent, .. } if Some(continent) == here => {
                news(&s.hack_stopped, 0.6);
            }
            _ => {}
        }
    }
}

fn footsteps(
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    mut walked: ResMut<Walked>,
) {
    let Some(s) = sounds else { return };
    let walked = &mut *walked;
    for p in &game.snap.players {
        if p.seat.is_some() || p.has(flags::DEAD) {
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
        if entry.1 > 1.0 && !p.has(flags::AIR) {
            entry.1 = 0.0;
            walked.n += 1;
            let mine = Some(p.id) == game.me;
            out.play(
                &s.footstep[walked.n % 3],
                (!mine).then_some(p.feet),
                if mine { 0.3 } else { 0.6 },
                1.0,
                if mine { Prio::Own } else { Prio::Ambient },
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

/// The heal and repair guns' beams: a loop on each player whose beam is on, where the gun is.
fn beams(
    mut commands: Commands,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    settings: Option<Res<Settings>>,
    mut q: Query<(Entity, &BeamLoop, &mut Transform)>,
) {
    let Some(s) = sounds else { return };
    let gain = crate::settings::sound_gain(settings.as_deref());
    let beaming = |id: PlayerId| {
        game.snap
            .players
            .iter()
            .find(|p| p.id == id && p.has(flags::BEAM) && !p.has(flags::DEAD))
    };
    let hand = |feet: Vec3| feet + Vec3::Y * 1.3;
    for (e, b, mut t) in &mut q {
        match beaming(b.0) {
            Some(p) if (p.weapon == Weapon::RepairGun) == b.1 => t.translation = hand(p.feet),
            _ => commands.entity(e).despawn(),
        }
    }
    for p in &game.snap.players {
        if beaming(p.id).is_none() || q.iter().any(|(_, b, _)| b.0 == p.id) {
            continue;
        }
        let repair = p.weapon == Weapon::RepairGun;
        let h = if repair {
            &s.repair_loop
        } else {
            &s.medgun_loop
        };
        commands.spawn((
            BeamLoop(p.id, repair),
            AudioPlayer(h.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(0.6 * gain),
                spatial: true,
                spatial_scale: Some(SpatialScale::new(1.0 / 8.0)),
                ..default()
            },
            Transform::from_translation(hand(p.feet)),
        ));
    }
}

fn stop_loops(
    mut commands: Commands,
    engines: Query<Entity, With<Engine>>,
    beams: Query<Entity, With<BeamLoop>>,
) {
    for e in engines.iter().chain(&beams) {
        commands.entity(e).despawn();
    }
}

/// A near miss going by (`Whiz`).
fn play_whiz(out: &mut Out, s: &Sounds, at: Vec3, heavy: bool) {
    if heavy {
        out.play(&s.whiz_heavy, Some(at), 0.9, 8.0, Prio::Fire);
    } else {
        out.play(pick(&s.bullet_whiz), Some(at), 0.8, 5.0, Prio::Fire);
    }
}

/// A spent case landing (`Casing`).
fn play_casing(out: &mut Out, s: &Sounds, at: Vec3, shell: bool) {
    if shell {
        out.play(&s.casing_shell, Some(at), 0.7, 6.0, Prio::Ambient);
    } else {
        out.play(pick(&s.casing), Some(at), 0.6, 4.0, Prio::Ambient);
    }
}

fn whizzes(mut out: Out, sounds: Option<Res<Sounds>>, mut whizzes: MessageReader<Whiz>) {
    let Some(s) = sounds else { return };
    for w in whizzes.read() {
        play_whiz(&mut out, &s, w.at, w.heavy);
    }
}

fn casings(mut out: Out, sounds: Option<Res<Sounds>>, mut casings: MessageReader<Casing>) {
    let Some(s) = sounds else { return };
    for c in casings.read() {
        play_casing(&mut out, &s, c.at, c.shell);
    }
}

/// A weapon's reload: by its mechanism.
fn reload_sound(s: &Sounds, w: Weapon) -> &Handle<AudioSource> {
    match w {
        Weapon::Rifle
        | Weapon::Ember
        | Weapon::Volt
        | Weapon::Hive
        | Weapon::Medgun
        | Weapon::RepairGun => &s.reload_rifle,
        Weapon::Scatter => &s.reload_scatter,
        Weapon::Lance => &s.reload_lance,
        Weapon::Striker | Weapon::Skyhook => &s.reload_launcher,
        Weapon::Pistol => &s.reload_pistol,
        Weapon::Pounder | Weapon::Mauler | Weapon::Burster => &s.reload_heavy,
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
    mut out: Out,
    game: Res<Game>,
    sounds: Option<Res<Sounds>>,
    local: Res<crate::game::LocalCmd>,
    mover: Res<crate::game::Mover>,
    time: Res<Time>,
    mut st: Local<Cues>,
) {
    let Some(s) = sounds else { return };
    let now = time.elapsed_secs();
    let me = game
        .mine()
        .filter(|m| m.dead.is_none() && m.seat.is_none() && m.turret.is_none());
    if let Some(m) = me {
        let slot = m.slot.min(ps_sim::weapons::SLOTS as u8 - 1);
        let g = m.guns[slot as usize];
        let weapon = m.loadout.slot(slot);
        let reloading = g.reload > 0.0;
        if slot == st.slot {
            if reloading && !st.reloading {
                out.play(reload_sound(&s, weapon), None, 0.7, 1.0, Prio::Own);
            } else if !reloading && st.reloading && g.mag > 0 {
                out.play(&s.reload_done, None, 0.6, 1.0, Prio::Own);
            }
        }
        // The trigger on an empty gun (nothing left to reload either): a dry click, not too
        // often.
        let trigger = local.cmd.fire;
        let beam = weapon.stats().shot == ps_sim::weapons::ShotKind::Beam;
        if trigger
            && g.mag == 0
            && !reloading
            && !beam
            && (!st.trigger || now - st.click > 0.5)
            && now - st.click > 0.2
        {
            out.play(&s.empty, None, 0.5, 1.0, Prio::Own);
            st.click = now;
        }
        st.trigger = trigger;
        st.reloading = reloading;
        st.slot = slot;
        if m.hack.is_some() && now - st.hack_tick > 0.5 {
            out.play(&s.hack_tick, None, 0.4, 1.0, Prio::Own);
            st.hack_tick = now;
        }
        if m.carrying.is_some() && st.carrying.is_none() {
            out.play(&s.terminal_select, None, 0.7, 1.0, Prio::Own);
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
        .filter(|p| Some(p.id) != game.me && p.has(flags::RELOAD))
        .map(|p| p.id)
        .collect();
    for p in &game.snap.players {
        if reloading.contains(&p.id) && !st.others.contains(&p.id) {
            let at = Some(p.feet + Vec3::Y);
            out.play(reload_sound(&s, p.weapon), at, 0.6, 8.0, Prio::Ambient);
        }
    }
    st.others = reloading;
    // Jump jets roaring; chutes opening.
    if now - st.jets > 0.22 {
        st.jets = now;
        for p in &game.snap.players {
            if Some(p.id) != game.me && p.has(flags::JET) {
                out.play(&s.jet_thrust, Some(p.feet), 0.7, 20.0, Prio::Ambient);
            }
        }
    }
    if mover.valid && mover.body.jetting && now - st.jet > 0.22 {
        st.jet = now;
        out.play(&s.jet_thrust, None, 0.5, 1.0, Prio::Own);
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
            let mine = Some(p.id) == game.me;
            let prio = if mine { Prio::Own } else { Prio::Ambient };
            out.play(&s.chute_open, (!mine).then_some(p.feet), 0.8, 30.0, prio);
        }
    }
    st.chutes = chutes;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(n: u32, prio: Prio, at: Option<Vec3>, born: f32) -> Live {
        Live {
            entity: Entity::from_raw_u32(n).unwrap(),
            sound: AssetId::default(),
            at,
            prio,
            born,
        }
    }

    #[test]
    fn far_shots_fade_over_to_their_tails() {
        assert_eq!(far_mix(0.0, 20.0), 0.0);
        assert_eq!(far_mix(10.0, 20.0), 0.0);
        assert!((far_mix(25.0, 20.0) - 0.5).abs() < 1e-5);
        assert_eq!(far_mix(40.0, 20.0), 1.0);
        assert_eq!(far_mix(400.0, 20.0), 1.0);
        let mut last = 0.0;
        for d in 0..100 {
            let k = far_mix(d as f32, 20.0);
            assert!(k >= last);
            last = k;
        }
    }

    #[test]
    fn the_least_important_voice_makes_way() {
        let at = |d: f32| Some(Vec3::new(d, 0.0, 0.0));
        let voices = [
            live(1, Prio::Own, None, 0.0),
            live(2, Prio::Fire, at(10.0), 1.0),
            live(3, Prio::Impact, at(5.0), 2.0),
            live(4, Prio::Impact, at(50.0), 3.0),
            live(5, Prio::Impact, at(50.0), 2.5),
        ];
        // The lowest priority, the farthest of those, the oldest of those.
        assert_eq!(victim(&voices, Vec3::ZERO, Prio::Fire, 100.0), Some(4));
        // One as unimportant makes it go only if it is nearer.
        assert_eq!(victim(&voices, Vec3::ZERO, Prio::Impact, 20.0), Some(4));
        assert_eq!(victim(&voices, Vec3::ZERO, Prio::Impact, 80.0), None);
        // Nothing goes for something less important than all.
        assert_eq!(victim(&voices, Vec3::ZERO, Prio::Ambient, 1.0), None);
        // One's own sounds stay whatever comes.
        let own = [live(1, Prio::Own, None, 0.0)];
        assert_eq!(victim(&own, Vec3::ZERO, Prio::Explosion, 0.0), None);
        assert_eq!(victim(&own, Vec3::ZERO, Prio::Own, 0.0), Some(0));
    }

    #[test]
    fn every_ammo_has_its_sound() {
        // (The table's match is exhaustive; this checks the levels make sense.)
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<AudioSource>()
            .add_systems(Startup, load);
        app.update();
        let s = app.world().resource::<Sounds>();
        let mut guns = Vec::new();
        for w in Weapon::ALL {
            let (h, tail, v, range) = fire_sound(s, Ammo::Gun(w));
            let beam = w.stats().shot == ps_sim::weapons::ShotKind::Beam;
            assert_eq!(v > 0.0, !beam, "{w:?}");
            assert!(range >= 1.0);
            if !beam {
                assert!(tail != Tail::No, "{w:?}");
                assert!(!guns.contains(&h.id()), "{w:?} shares its sound");
                guns.push(h.id());
            }
        }
    }
}
