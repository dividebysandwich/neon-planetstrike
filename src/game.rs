//! The running game: a connection to a server (a remote one, or the one this game runs itself
//! for an offline or hosted world, `ps_server`), the world's state as the server tells it, the
//! continent the player is on (built here from the seed, on a thread of its own), and what the
//! player sees of it: snapshots played back a little behind (`Interp`) with the player's own
//! movement and driving predicted (`Mover`). Stepped in `FixedUpdate` at the sim's tick; the
//! rest of the client reads `Game::snap` and the `GameEvent` / `FxEvent` messages.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use bevy::prelude::*;
use ps_net::client::{Client, ClientEvent};
use ps_net::protocol::{ScoreRow, SquadCmd, SquadInfo};
use ps_sim::collide::World;
use ps_sim::map::ContinentMap;
use ps_sim::player::Body;
use ps_sim::snapshot::{Fx, MeSnap, PlayerSnap, VehicleSnap};
use ps_sim::state::WorldState;
use ps_sim::vehicle::{Motion, VehicleKind};
use ps_sim::{
    ContinentId, PlayerCmd, PlayerId, Request, SimEvent, Snapshot, Team, VehicleId, WorldDef,
    WorldParams,
};

/// A world this game runs itself: offline (only this player), or hosted for others.
#[derive(Clone, Debug, PartialEq)]
pub struct Hosting {
    /// Others may join: on the local network (and the internet when `internet`).
    pub open: bool,
    pub internet: bool,
    pub name: String,
    pub port: u16,
}

#[derive(Resource)]
pub struct Game {
    pub client: Client,
    #[cfg(not(target_arch = "wasm32"))]
    pub server: Option<ps_server::Handle>,
    pub me: Option<PlayerId>,
    pub team: Team,
    pub server_name: String,
    pub def: Arc<WorldDef>,
    pub state: WorldState,
    /// Where the server has the player.
    pub continent: Option<ContinentId>,
    /// The continent built for drawing and prediction (`continent`'s, once it's ready).
    pub map: Option<Arc<ContinentMap>>,
    /// Its collision with the vehicles shown (for the prediction).
    pub world: World,
    loading: Option<Loading>,
    /// The state shown (interpolated), and the one before.
    pub snap: Snapshot,
    pub prev: Snapshot,
    pub interp: Interp,
    pub names: HashMap<PlayerId, (String, Team)>,
    /// The continent each human player is on (from the roster).
    pub whereabouts: HashMap<PlayerId, ContinentId>,
    /// The faction's squads (the server's).
    pub squads: Vec<SquadInfo>,
    /// The player's XP in all (the server's; none until it says).
    pub xp: Option<u64>,
    /// Human players' Battle Ranks (from the roster).
    pub ranks: HashMap<PlayerId, u8>,
    /// The scoreboard: everyone on the continent (the server's, every few seconds).
    pub scores: Vec<ScoreRow>,
    pub population: Vec<[u16; 3]>,
    /// The human players on each continent by faction (overpopulation).
    pub humans: Vec<[u16; 3]>,
    /// The alert under way, and when it was heard of (s, real time: its time left counts down).
    pub alert: Option<(ps_net::protocol::AlertInfo, f32)>,
    /// A notice for the HUD (an alert's end), and when (s).
    pub notice: Option<(String, f32)>,
    /// The server wants a login (its name), and why the last one failed (src/login.rs).
    pub login: Option<(String, Option<String>)>,
    /// One's stats as the server last told them (counters by key).
    pub stats: Option<Vec<(String, i64)>>,
    /// The last death: who did it (the killcam's).
    pub death: Option<Death>,
    pub status: String,
    /// The latest puzzle shown at a console: base, seed, pieces.
    pub puzzle: Option<(u8, u32, Vec<u8>)>,
    /// Said lately: who, their faction, what, and whether to the squad only.
    pub chat: VecDeque<(String, Team, String, bool)>,
    pub connected: bool,
    /// The time of day (0..1, 0.5 noon), from the server and counted on here.
    pub day: f32,
}

struct Loading {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    continent: ContinentId,
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<ContinentMap>>,
}

impl Game {
    pub fn new(client: Client) -> Game {
        let def = Arc::new(WorldDef::generate(WorldParams::default()));
        let state = WorldState::initial(&def);
        Game {
            client,
            #[cfg(not(target_arch = "wasm32"))]
            server: None,
            me: None,
            team: Team::Red,
            server_name: String::new(),
            def,
            state,
            continent: None,
            map: None,
            world: World::default(),
            loading: None,
            snap: Snapshot::default(),
            prev: Snapshot::default(),
            interp: Interp::default(),
            names: HashMap::new(),
            population: Vec::new(),
            humans: Vec::new(),
            alert: None,
            notice: None,
            login: None,
            stats: None,
            death: None,
            status: "Connecting...".into(),
            puzzle: None,
            chat: VecDeque::new(),
            whereabouts: HashMap::new(),
            squads: Vec::new(),
            xp: None,
            ranks: HashMap::new(),
            scores: Vec::new(),
            connected: false,
            day: 0.35,
        }
    }

    /// A world run here (offline or hosted), with this game joined to it.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn local(
        params: WorldParams,
        bots: usize,
        skill: f32,
        host: Option<Hosting>,
        name: &str,
        team: Option<Team>,
    ) -> std::io::Result<Game> {
        let open = host.as_ref().is_some_and(|h| h.open);
        let bind = match &host {
            Some(h) if h.open => format!("0.0.0.0:{}", h.port),
            _ => "127.0.0.1:0".to_string(),
        };
        let state_path = crate::settings::data_dir()
            .map(|d| d.join(format!("world_{}_{}.ron", params.seed, params.contested)));
        let handle = ps_server::start(ps_server::Config {
            bind,
            name: host
                .as_ref()
                .map_or_else(|| "Offline world".to_string(), |h| h.name.clone()),
            params,
            bots,
            skill,
            max_players: if open { 64 } else { 1 },
            state_path,
            master: host.as_ref().filter(|h| h.internet).map(|_| {
                std::env::var("PS_MASTER").unwrap_or_else(|_| ps_net::DEFAULT_MASTER.to_string())
            }),
            lan: open,
            port_map: host.as_ref().is_some_and(|h| h.internet),
            dedicated: false,
            snapshot_threads: 1,
            verbose: true,
            accounts: None,
        })?;
        let client = Client::connect(&format!("127.0.0.1:{}", handle.port), name, team)?;
        let mut g = Game::new(client);
        g.server = Some(handle);
        Ok(g)
    }

    pub fn is_local(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.server.is_some();
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// A hosting game's state (where it can be reached).
    pub fn hosting(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.server.as_ref().map(|s| s.status());
        #[cfg(target_arch = "wasm32")]
        None
    }

    pub fn name_of(&self, id: PlayerId) -> String {
        match self.names.get(&id) {
            Some((n, _)) => n.clone(),
            None => ps_sim::bots::bot_name(id),
        }
    }

    /// The local player as last shown.
    pub fn me_snap(&self) -> Option<&PlayerSnap> {
        let me = self.me?;
        self.snap.players.iter().find(|p| p.id == me)
    }

    /// The local player's own state (newest).
    pub fn mine(&self) -> Option<&MeSnap> {
        self.snap.me.as_ref()
    }

    pub fn alive(&self) -> bool {
        self.mine().is_some_and(|m| m.dead.is_none())
    }

    /// The vehicle the player sits in and the seat.
    pub fn seat(&self) -> Option<(VehicleId, u8)> {
        self.mine().and_then(|m| m.seat)
    }

    pub fn vehicle(&self, id: VehicleId) -> Option<&VehicleSnap> {
        self.snap.vehicles.iter().find(|v| v.id == id)
    }

    /// Ready to play: the continent built.
    pub fn ready(&self) -> bool {
        self.map.is_some() && self.loading.is_none()
    }

    pub fn request(&mut self, r: Request) {
        self.client.send_request(r);
    }

    /// The squad the player is in.
    pub fn my_squad(&self) -> Option<&SquadInfo> {
        let me = self.me?;
        self.squads.iter().find(|s| s.members.contains(&me))
    }

    /// Whether `id` is in the player's squad (not the player itself).
    pub fn squad_mate(&self, id: PlayerId) -> bool {
        Some(id) != self.me && self.my_squad().is_some_and(|s| s.members.contains(&id))
    }

    /// Whether the player leads its squad.
    pub fn leading(&self) -> bool {
        self.my_squad().is_some_and(|s| Some(s.leader) == self.me)
    }

    /// The squad's waypoint, if it's on the continent the player is on.
    pub fn waypoint_here(&self) -> Option<Vec3> {
        let (c, at) = self.my_squad()?.waypoint?;
        (Some(c) == self.continent).then_some(at)
    }

    pub fn squad(&mut self, cmd: SquadCmd) {
        self.client.send_squad(cmd);
    }

    /// Leaves the server (and stops the one run here).
    pub fn close(&mut self) {
        self.client.close();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(mut s) = self.server.take() {
            s.stop();
        }
    }

    fn start_loading(&mut self, c: ContinentId) {
        let def = Arc::new(self.def.continents[c as usize].clone());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(ContinentMap::build(def, true));
            });
            self.loading = Some(Loading {
                continent: c,
                rx: std::sync::Mutex::new(rx),
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            // (No threads in the browser: built at once.)
            let map = ContinentMap::build(def, true);
            self.loaded(c, map);
        }
    }

    fn loaded(&mut self, c: ContinentId, map: ContinentMap) {
        if self.continent != Some(c) {
            return;
        }
        self.world = map.world.clone();
        self.map = Some(Arc::new(map));
        self.loading = None;
        self.interp.reset();
        self.snap = Snapshot::default();
        self.prev = Snapshot::default();
    }
}

/// Snapshots are drawn a few ticks behind the newest, interpolating between the two around
/// that moment, so others move smoothly however the packets arrive. The delay follows the
/// measured jitter. Events and effects wait until the view reaches the tick they came with.
#[derive(Default)]
pub struct Interp {
    buffer: VecDeque<Snapshot>,
    pub view: f64,
    clock: u64,
    base: Option<f64>,
    jitter: f64,
    pending: VecDeque<(u64, SimEvent)>,
    fx: VecDeque<(u64, Fx)>,
}

const MIN_DELAY: f64 = 3.0;
const MAX_DELAY: f64 = 14.0;

impl Interp {
    pub fn view_tick(&self) -> u64 {
        self.view.max(0.0) as u64
    }

    pub fn reset(&mut self) {
        *self = Interp::default();
    }

    pub fn newest(&self) -> Option<&Snapshot> {
        self.buffer.back()
    }

    fn receive(&mut self, mut s: Snapshot) {
        if self.buffer.back().is_some_and(|b| b.tick >= s.tick) {
            return;
        }
        for f in s.fx.drain(..) {
            self.fx.push_back((s.tick, f));
        }
        let offset = self.clock as f64 - s.tick as f64;
        let base = self.base.map_or(offset, |b| (b + 0.002).min(offset));
        self.base = Some(base);
        let late = offset - base;
        self.jitter = if late > self.jitter {
            late
        } else {
            self.jitter * 0.995 + late * 0.005
        };
        if self.buffer.is_empty() {
            self.view = s.tick as f64 - MIN_DELAY;
        }
        self.smooth_held(&s);
        self.buffer.push_back(s);
        while self.buffer.len() > 64 {
            self.buffer.pop_front();
        }
    }

    /// Far players and vehicles aren't refreshed every snapshot (`ps_net::delta`): the
    /// snapshots in between, still waiting to be shown, get them on the way to `s`'s.
    fn smooth_held(&mut self, s: &Snapshot) {
        for q in s.players.iter().filter(|q| !s.held_players.contains(&q.id)) {
            let Some(last) = self.buffer.iter().rposition(|b| {
                b.players
                    .iter()
                    .any(|p| p.id == q.id && !b.held_players.contains(&q.id))
            }) else {
                continue;
            };
            let (t0, p0) = {
                let b = &self.buffer[last];
                let p = b.players.iter().find(|p| p.id == q.id).unwrap();
                (b.tick, (p.feet, p.yaw, p.pitch))
            };
            if (q.feet - p0.0).length() > 40.0 || s.tick <= t0 {
                continue;
            }
            for b in self.buffer.iter_mut().skip(last + 1) {
                if !b.held_players.contains(&q.id) {
                    continue;
                }
                let k = (b.tick - t0) as f32 / (s.tick - t0) as f32;
                if let Some(p) = b.players.iter_mut().find(|p| p.id == q.id) {
                    p.feet = p0.0.lerp(q.feet, k);
                    p.yaw = lerp_angle(p0.1, q.yaw, k);
                    p.pitch = p0.2 + (q.pitch - p0.2) * k;
                }
            }
        }
        for q in s
            .vehicles
            .iter()
            .filter(|q| !s.held_vehicles.contains(&q.id))
        {
            let Some(last) = self.buffer.iter().rposition(|b| {
                b.vehicles
                    .iter()
                    .any(|v| v.id == q.id && !b.held_vehicles.contains(&q.id))
            }) else {
                continue;
            };
            let (t0, v0) = {
                let b = &self.buffer[last];
                let v = b.vehicles.iter().find(|v| v.id == q.id).unwrap();
                (b.tick, (v.pos, v.rot))
            };
            if (q.pos - v0.0).length() > 150.0 || s.tick <= t0 {
                continue;
            }
            for b in self.buffer.iter_mut().skip(last + 1) {
                if !b.held_vehicles.contains(&q.id) {
                    continue;
                }
                let k = (b.tick - t0) as f32 / (s.tick - t0) as f32;
                if let Some(v) = b.vehicles.iter_mut().find(|v| v.id == q.id) {
                    v.pos = v0.0.lerp(q.pos, k);
                    v.rot = v0.1.slerp(q.rot, k);
                }
            }
        }
    }

    fn advance(&mut self) -> Option<Snapshot> {
        self.clock += 1;
        let newest = self.buffer.back()?.tick as f64;
        let delay = (MIN_DELAY + self.jitter * 1.2).clamp(MIN_DELAY, MAX_DELAY);
        let target = newest - delay;
        let error = target - self.view;
        self.view += if error.abs() > 30.0 {
            error + 1.0
        } else {
            1.0 + (error * 0.05).clamp(-0.15, 0.15)
        };
        let view = self.view.min(newest);
        while self.buffer.len() > 2 && self.buffer[1].tick as f64 <= view {
            self.buffer.pop_front();
        }
        let a = &self.buffer[0];
        let b = self.buffer.get(1).unwrap_or(a);
        let k = if b.tick > a.tick {
            ((view - a.tick as f64) / (b.tick - a.tick) as f64).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        let mut out = blend(a, b, k, view);
        // One's own state as of the newest snapshot.
        out.me = self.buffer.back().and_then(|n| n.me.clone());
        Some(out)
    }

    fn due(&mut self) -> (Vec<SimEvent>, Vec<Fx>) {
        let view = self.view_tick();
        let mut ev = Vec::new();
        while self.pending.front().is_some_and(|(t, _)| *t <= view) {
            ev.push(self.pending.pop_front().unwrap().1);
        }
        let mut fx = Vec::new();
        while self.fx.front().is_some_and(|(t, _)| *t <= view) {
            fx.push(self.fx.pop_front().unwrap().1);
        }
        (ev, fx)
    }
}

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let d = (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + d * t
}

fn blend(a: &Snapshot, b: &Snapshot, k: f32, view: f64) -> Snapshot {
    let mut out = Snapshot {
        tick: view.max(0.0) as u64,
        ..Default::default()
    };
    out.players = b
        .players
        .iter()
        .map(|q| match a.players.iter().find(|p| p.id == q.id) {
            Some(p) if (q.feet - p.feet).length() < 6.0 => {
                let mut r = q.clone();
                r.feet = p.feet.lerp(q.feet, k);
                r.yaw = lerp_angle(p.yaw, q.yaw, k);
                r.pitch = p.pitch + (q.pitch - p.pitch) * k;
                if let (Some(pv), Some(qv)) = (&p.vr, &mut r.vr) {
                    qv.head.pos = pv.head.pos.lerp(qv.head.pos, k);
                    qv.head.rot = pv.head.rot.slerp(qv.head.rot, k);
                    for i in 0..2 {
                        qv.hands[i].pos = pv.hands[i].pos.lerp(qv.hands[i].pos, k);
                        qv.hands[i].rot = pv.hands[i].rot.slerp(qv.hands[i].rot, k);
                    }
                }
                r
            }
            _ => q.clone(),
        })
        .collect();
    out.vehicles = b
        .vehicles
        .iter()
        .map(|q| match a.vehicles.iter().find(|p| p.id == q.id) {
            Some(p) if (q.pos - p.pos).length() < 30.0 => {
                let mut r = q.clone();
                r.pos = p.pos.lerp(q.pos, k);
                r.rot = p.rot.slerp(q.rot, k);
                for (i, aim) in r.aim.iter_mut().enumerate() {
                    if let Some(pa) = p.aim.get(i) {
                        let yaw = lerp_angle(pa.0 as f32 / 1e4, aim.0 as f32 / 1e4, k);
                        let pitch = pa.1 as f32 / 1e4 + (aim.1 as f32 - pa.1 as f32) / 1e4 * k;
                        *aim = ((yaw * 1e4) as i16, (pitch * 1e4) as i16);
                    }
                }
                r
            }
            _ => q.clone(),
        })
        .collect();
    out.turrets = b
        .turrets
        .iter()
        .map(|q| {
            let mut r = *q;
            if let Some(p) = a
                .turrets
                .iter()
                .find(|p| p.base == q.base && p.index == q.index)
            {
                r.yaw = lerp_angle(p.yaw, q.yaw, k);
                r.pitch = p.pitch + (q.pitch - p.pitch) * k;
            }
            r
        })
        .collect();
    // (Nothing to smooth: the bases' power and silos, and the resource nodes, as the newer
    // has them.)
    out.bases = b.bases.clone();
    out.nodes = b.nodes.clone();
    // Structures: a Bastion's aim smoothed.
    out.structures = b
        .structures
        .iter()
        .map(|q| {
            let mut r = *q;
            if let Some(p) = a.structures.iter().find(|p| p.id == q.id) {
                r.aim_yaw = lerp_angle(p.aim_yaw, q.aim_yaw, k);
                r.aim_pitch = p.aim_pitch + (q.aim_pitch - p.aim_pitch) * k;
            }
            r
        })
        .collect();
    // Deployables: a spitfire's aim smoothed.
    out.deployables = b
        .deployables
        .iter()
        .map(|q| {
            let mut r = *q;
            if let Some(p) = a.deployables.iter().find(|p| p.id == q.id) {
                r.yaw = lerp_angle(p.yaw, q.yaw, k);
                r.pitch = p.pitch + (q.pitch - p.pitch) * k;
            }
            r
        })
        .collect();
    out
}

/// What the local player wants this tick (desktop or VR input fills it in).
#[derive(Resource, Default)]
pub struct LocalCmd {
    pub cmd: PlayerCmd,
    /// The trigger was pulled since the last tick (a quick click isn't lost between ticks).
    pub fire_latch: bool,
    /// Physical walking (VR) since the last frame's prediction, world XZ metres.
    pub walk_frame: Vec2,
    /// The input is the player's (no menu or map open).
    pub active: bool,
    /// Testing (`PS_DEV_VEHICLE`): the vehicle was ordered.
    pub dev_vehicle_sent: bool,
    /// Testing (`PS_DEV_LOADOUT`): the loadout was asked for; (`PS_DEV_DEPLOY`) when.
    pub dev_loadout_sent: Option<f64>,
    pub dev_deployed: bool,
    pub dev_built: bool,
}

/// The local player's own prediction: its body on foot, or the vehicle it drives.
#[derive(Resource, Default)]
pub struct Mover {
    pub body: Body,
    pub valid: bool,
    pub continent: Option<ContinentId>,
    since_moved: f32,
    trail: VecDeque<(f32, Vec3)>,
    /// Driving: the vehicle's predicted motion.
    pub drive: Option<(VehicleId, VehicleKind, Motion)>,
    /// Just landed on foot: how fast it was falling (m/s; the view's dip takes it).
    pub landed: Option<f32>,
    drive_trail: VecDeque<(f32, Vec3)>,
}

const TRAIL_TIME: f32 = 1.0;
const TRAIL_SLACK: f32 = 2.0;

fn lost(now: Vec3, trail: &VecDeque<(f32, Vec3)>, server: Vec3, slack: f32) -> bool {
    (now - server).length() > slack && trail.iter().all(|(_, f)| (*f - server).length() > slack)
}

/// Drawing the actors (after the prediction and the VR poses).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DrawSet;

/// Prediction runs after the input and before anything that places the camera or hands.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PredictSet;

/// Systems that fill in `LocalCmd` before the step.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

#[derive(Message, Clone, Debug)]
pub struct GameEvent(pub SimEvent);

#[derive(Message, Clone, Copy, Debug)]
pub struct FxEvent(pub Fx);

/// The world changed (ownership, a hack).
#[derive(Message, Clone, Copy, Debug)]
pub struct WorldChange(pub ps_sim::state::Change);

/// The player arrived on a continent (built and ready).
#[derive(Message, Clone, Copy, Debug)]
pub struct Arrived(pub ContinentId);

/// The spot button was pressed (VR; the flat screen's key sends its request itself).
#[derive(Message)]
pub struct SpotPressed;

fn send_spots(mut game: ResMut<Game>, mut spots: MessageReader<SpotPressed>) {
    if spots.read().count() > 0 && game.alive() {
        game.request(ps_sim::Request::Spot);
    }
}

/// How long the killcam looks at the killer before the spawn screen (s).
pub const KILLCAM_TIME: f32 = 4.0;

/// One's last death as the server told it (`SimEvent::DeathInfo`).
#[derive(Clone, Debug)]
pub struct Death {
    pub by: PlayerId,
    pub cause: ps_sim::Cause,
    pub distance: f32,
    pub killer_at: Option<Vec3>,
    pub health: f32,
    pub armor: f32,
    /// When (s, real time).
    pub at: f32,
}

impl Game {
    /// The killcam's moment (dead, lately, by someone else): who, and where they are now (the
    /// snapshot's, else where they were).
    pub fn killcam(&self, now: f32) -> Option<(&Death, Option<Vec3>)> {
        let d = self.death.as_ref()?;
        if self.alive() || now - d.at > KILLCAM_TIME || Some(d.by) == self.me {
            return None;
        }
        let at = self
            .snap
            .players
            .iter()
            .find(|p| p.id == d.by && p.seat.is_none())
            .map(|p| p.feet + Vec3::Y * 1.4)
            .or(d.killer_at);
        Some((d, at))
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(ps_sim::TICK as f64))
            .init_resource::<LocalCmd>()
            .init_resource::<Mover>()
            .configure_sets(
                Update,
                (PredictSet.after(InputSet), DrawSet.after(PredictSet)),
            )
            .add_message::<GameEvent>()
            .add_message::<FxEvent>()
            .add_message::<WorldChange>()
            .add_message::<Arrived>()
            .add_message::<SpotPressed>()
            .add_systems(Update, send_spots.run_if(resource_exists::<Game>))
            .add_systems(
                Update,
                (check_loaded, predict.in_set(PredictSet)).run_if(resource_exists::<Game>),
            )
            .add_systems(FixedUpdate, step_game.run_if(resource_exists::<Game>));
    }
}

fn check_loaded(mut game: ResMut<Game>, mut arrived: MessageWriter<Arrived>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(l) = &game.loading else { return };
        let got = l.rx.lock().ok().and_then(|rx| rx.try_recv().ok());
        if let Some(map) = got {
            let c = l.continent;
            game.loaded(c, map);
            if game.ready() {
                arrived.write(Arrived(c));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (&mut game, &mut arrived);
    }
}

#[allow(clippy::too_many_arguments)]
fn step_game(
    mut game: ResMut<Game>,
    mut local: ResMut<LocalCmd>,
    mut events: MessageWriter<GameEvent>,
    mut fxs: MessageWriter<FxEvent>,
    mut changes: MessageWriter<WorldChange>,
    mut arrived: MessageWriter<Arrived>,
    mut leave: MessageWriter<crate::menu::LeaveGame>,
    time: Res<Time<Real>>,
) {
    let game = &mut *game;
    let mut cmd = local.cmd;
    if std::mem::take(&mut local.fire_latch) {
        cmd.fire = true;
    }
    local.cmd.drive.turn = Vec2::ZERO;
    cmd.view_tick = game.interp.view_tick();
    if !game.ready() {
        cmd = PlayerCmd {
            yaw: cmd.yaw,
            pitch: cmd.pitch,
            jumps: cmd.jumps,
            reloads: cmd.reloads,
            throws: cmd.throws,
            deploys: cmd.deploys,
            seat_switches: cmd.seat_switches,
            ..Default::default()
        };
    }
    game.client.send_input(cmd);
    game.state.count_down(ps_sim::TICK);
    game.day = (game.day + ps_sim::TICK / ps_net::protocol::DAY_LENGTH).fract();
    // Testing: PS_DAY=0.0 (midnight) .. 0.5 (noon) fixes the time of day.
    if let Some(d) = std::env::var("PS_DAY")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        game.day = d;
    }
    for ev in game.client.poll() {
        match ev {
            ClientEvent::Welcome {
                id,
                params,
                team,
                server,
            } => {
                info!("joined {server} as player {id} ({})", team.name());
                game.login = None;
                game.me = Some(id);
                game.team = team;
                game.server_name = server;
                game.connected = true;
                game.status.clear();
                if game.def.params != params {
                    game.def = Arc::new(WorldDef::generate(params));
                    game.state = WorldState::initial(&game.def);
                }
            }
            ClientEvent::World(w) => {
                if w.fits(&game.def) {
                    game.state = w;
                }
            }
            ClientEvent::Changes(cs) => {
                for c in cs {
                    let def = game.def.clone();
                    game.state.apply(&def, &c);
                    changes.write(WorldChange(c));
                }
            }
            ClientEvent::Enter(c) => {
                if game.continent != Some(c) || game.map.is_none() {
                    game.continent = Some(c);
                    game.map = None;
                    game.start_loading(c);
                    if game.ready() {
                        arrived.write(Arrived(c));
                    }
                }
            }
            ClientEvent::Snapshot(c, s) => {
                if Some(c) == game.continent && game.ready() {
                    game.interp.receive(*s);
                }
            }
            ClientEvent::Events(evs) => {
                let tick = game.interp.newest().map_or(0, |s| s.tick);
                for e in evs {
                    // Puzzles and refusals show at once.
                    match &e {
                        SimEvent::Puzzle { base, seed, at } => {
                            game.puzzle = Some((*base, *seed, at.clone()));
                            events.write(GameEvent(e));
                        }
                        SimEvent::Denied(_) | SimEvent::Spawned { .. } => {
                            events.write(GameEvent(e));
                        }
                        _ => game.interp.pending.push_back((tick, e)),
                    }
                }
            }
            ClientEvent::Roster(r) => {
                game.whereabouts = r
                    .iter()
                    .filter_map(|e| Some((e.id, e.continent?)))
                    .collect();
                game.ranks = r.iter().map(|e| (e.id, e.br)).collect();
                game.names = r.into_iter().map(|e| (e.id, (e.name, e.team))).collect();
            }
            ClientEvent::Squads(s) => game.squads = s,
            ClientEvent::Progress(x) => game.xp = Some(x),
            ClientEvent::Scores(s) => game.scores = s,
            ClientEvent::Population { all, humans } => {
                game.population = all;
                game.humans = humans;
            }
            ClientEvent::LoginRequired(server) => game.login = Some((server, None)),
            ClientEvent::LoginFailed(why) => {
                if let Some(l) = game.login.as_mut() {
                    l.1 = Some(why);
                }
            }
            ClientEvent::Stats(s) => game.stats = Some(s),
            ClientEvent::Alert(a) => game.alert = a.map(|a| (a, time.elapsed_secs())),
            ClientEvent::AlertEnded { continent, winner } => {
                let name = game
                    .def
                    .continents
                    .get(continent as usize)
                    .map_or(String::new(), |c| c.name.clone());
                let text = match winner {
                    Some(t) if t == game.team => format!("ALERT WON: {name} is yours"),
                    Some(t) => format!("ALERT LOST: {} won {name}", t.upper()),
                    None => format!("ALERT ON {} ENDED IN A DRAW", name.to_uppercase()),
                };
                game.notice = Some((text, time.elapsed_secs()));
            }
            ClientEvent::Clock(t) => game.day = t,
            ClientEvent::Chat(from, text, squad) => {
                let name = game.name_of(from);
                let team = game.names.get(&from).map_or(Team::Red, |n| n.1);
                game.chat.push_back((name, team, text, squad));
                while game.chat.len() > 8 {
                    game.chat.pop_front();
                }
            }
            ClientEvent::Disconnected(why) => {
                warn!("disconnected: {why}");
                game.status = format!("Disconnected: {why}");
                game.connected = false;
                leave.write(crate::menu::LeaveGame(Some(why)));
            }
        }
    }
    // Testing: PS_AUTOSPAWN=home spawns at the sanctuary when dead, =front at a base of the
    // faction's on a contested continent (=C,B at base B of continent C).
    if let Ok(spec) = std::env::var("PS_AUTOSPAWN")
        && game.ready()
        && game.mine().is_some_and(|m| m.dead == Some(0.0))
        && game.interp.view_tick().is_multiple_of(120)
    {
        let team = game.team;
        let pick = match spec.as_str() {
            "home" => Some((team.index() as ContinentId, 0)),
            "front" => (3..game.def.continents.len()).find_map(|c| {
                let c = c as ContinentId;
                if !game.state.contested(&game.def, c) {
                    return None;
                }
                game.state.continents[c as usize]
                    .bases
                    .iter()
                    .position(|b| b.owner == Some(team))
                    .map(|b| (c, b as u8))
            }),
            s => s
                .split_once(',')
                .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                .or_else(|| {
                    let c = game
                        .def
                        .continents
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(s))?;
                    let b = game.state.continents[c.id as usize]
                        .bases
                        .iter()
                        .position(|b| b.owner == Some(team))?;
                    Some((c.id, b as u8))
                }),
        };
        if let Some((continent, base)) = pick.or(Some((team.index() as ContinentId, 0))) {
            game.client
                .send_request(Request::Spawn(ps_sim::SpawnPoint::Base { continent, base }));
        }
    }
    // Testing: PS_DEV_VEHICLE=light_tank|light_aaa|heavy_tank|spawn_bus|vtol orders one at
    // the nearest vehicle terminal of the faction's (with PS_CHEAT=1 on the server), once.
    if let Ok(kind) = std::env::var("PS_DEV_VEHICLE")
        && game.alive()
        && game.seat().is_none()
        && !local.dev_vehicle_sent
        && let Some(map) = &game.map
        && let Some(me) = game.mine()
    {
        let kind = ps_sim::vehicle::VehicleKind::ALL
            .into_iter()
            .find(|k| crate::vehicles::prefix(*k) == kind);
        let term = map
            .bases
            .iter()
            .enumerate()
            .flat_map(|(b, l)| {
                l.terminals
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.kind == ps_sim::base::TerminalKind::Vehicle)
                    .map(move |(i, t)| (b, i, (t.pos - me.feet).length()))
            })
            .filter(|(b, _, _)| game.state.base(map.def.id, *b as u8).owner == Some(game.team))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let (Some(kind), Some((b, t, _))) = (kind, term) {
            game.client.send_request(Request::SpawnVehicle {
                base: b as u8,
                terminal: t as u8,
                kind,
                loadout: kind.default_loadout(),
            });
            local.dev_vehicle_sent = true;
        }
    }
    // Testing: PS_DEV_LOADOUT=max|burster|mauler|stealth|spitfire|mines|ammo takes that
    // loadout once spawned (with PS_CHEAT=1 on the server: anywhere); PS_DEV_DEPLOY=1 then
    // presses the deploy key (cloak, deployable) a second later.
    if let Ok(which) = std::env::var("PS_DEV_LOADOUT")
        && game.alive()
        && game.seat().is_none()
        && local.dev_loadout_sent.is_none()
    {
        use ps_sim::weapons::{ArmorKind, Kit, Loadout, Weapon};
        let base = Loadout::default();
        let l = match which.as_str() {
            "max" => Loadout {
                armor: ArmorKind::Max,
                rifle: Weapon::Pounder,
                ..base
            },
            "burster" | "mauler" => Loadout {
                armor: ArmorKind::Max,
                rifle: if which == "burster" {
                    Weapon::Burster
                } else {
                    Weapon::Mauler
                },
                ..base
            },
            "stealth" => Loadout {
                armor: ArmorKind::Stealth,
                ..base
            },
            k => Loadout {
                special: Weapon::RepairGun,
                kit: match k {
                    "mines" => Kit::Mines,
                    "ammo" => Kit::AmmoPack,
                    _ => Kit::Spitfire,
                },
                ..base
            },
        };
        game.client.send_request(Request::Loadout(l));
        local.dev_loadout_sent = Some(time.elapsed_secs_f64());
    }
    if std::env::var("PS_DEV_DEPLOY").is_ok_and(|v| v == "1")
        && !local.dev_deployed
        && local
            .dev_loadout_sent
            .is_some_and(|t| time.elapsed_secs_f64() > t + 1.0)
    {
        local.cmd.deploys += 1;
        local.dev_deployed = true;
    }
    // Testing: PS_DEV_BUILD=wall|bastion|tube|silo (an engineer: with PS_DEV_LOADOUT=spitfire) asks
    // to build one at eight spots 25 m round the player, two seconds after the loadout (the
    // server refuses those with no room).
    if let Ok(which) = std::env::var("PS_DEV_BUILD")
        && !local.dev_built
        && local
            .dev_loadout_sent
            .is_some_and(|t| time.elapsed_secs_f64() > t + 2.0)
        && let Some(me) = game.mine()
    {
        use ps_sim::construct::StructureKind;
        let kind = match which.as_str() {
            "bastion" => StructureKind::Bastion,
            "tube" => StructureKind::Tube,
            "silo" => StructureKind::Silo,
            _ => StructureKind::Wall,
        };
        let feet = me.feet;
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU;
            let at = feet + Vec3::new(a.cos(), 0.0, a.sin()) * 25.0;
            let y = game.world.ground(at.x, at.z, at.y + 10.0).unwrap_or(at.y);
            game.client.send_request(Request::Build {
                kind,
                at: Vec3::new(at.x, y, at.z),
                yaw: a,
            });
        }
        local.dev_built = true;
    }
    if let Some(s) = game.interp.advance() {
        game.prev = std::mem::replace(&mut game.snap, s);
    }
    let (evs, fx) = game.interp.due();
    for e in evs {
        // The killcam's: who killed one, and how they stood.
        if let SimEvent::DeathInfo {
            by,
            cause,
            distance,
            killer_at,
            killer_health,
            killer_armor,
        } = &e
        {
            game.death = Some(Death {
                by: *by,
                cause: *cause,
                distance: *distance,
                killer_at: *killer_at,
                health: *killer_health,
                armor: *killer_armor,
                at: time.elapsed_secs(),
            });
        }
        events.write(GameEvent(e));
    }
    for f in fx {
        fxs.write(FxEvent(f));
    }
    // The vehicles shown, for the prediction's collision (not the one driven).
    let driving = game.seat().filter(|s| s.1 == 0).map(|s| s.0);
    let boxes = game
        .snap
        .vehicles
        .iter()
        .filter(|v| Some(v.id) != driving && !v.kind.aircraft())
        .map(|v| {
            let mut veh = ps_sim::sim::Vehicle::new(v.id, v.kind, v.team, v.pos, 0.0);
            veh.motion.rot = v.rot;
            veh.bounds()
        })
        // (And the structures built.)
        .chain(
            game.snap
                .structures
                .iter()
                .map(|s| s.kind.collider(s.pos, s.yaw)),
        )
        .collect();
    game.world.set_dynamic(boxes);
}

/// Predicts the local player's feet (on foot) or vehicle (driving).
fn predict(
    game: Res<Game>,
    mut local: ResMut<LocalCmd>,
    mut mover: ResMut<Mover>,
    time: Res<Time>,
    mut events: MessageReader<GameEvent>,
) {
    let walk = std::mem::take(&mut local.walk_frame);
    let mut spawned = None;
    for e in events.read() {
        if let SimEvent::Spawned { player, pos, .. } = e.0
            && Some(player) == game.me
        {
            spawned = Some(pos);
        }
    }
    let dt = time.delta_secs().min(0.05);
    let now = time.elapsed_secs();
    let Some(me) = game.mine().cloned() else {
        mover.valid = false;
        mover.drive = None;
        local.cmd.feet = None;
        local.cmd.vehicle = None;
        return;
    };
    // (On a zipline the server carries us: no prediction.)
    if !game.ready() || me.dead.is_some() || me.turret.is_some() || me.zip.is_some() {
        mover.valid = false;
        mover.drive = None;
        local.cmd.feet = None;
        local.cmd.vehicle = None;
        return;
    }
    // Driving: predict the vehicle.
    if let Some((vid, 0)) = me.seat {
        local.cmd.feet = None;
        mover.valid = false;
        let Some(v) = game.vehicle(vid).cloned() else {
            local.cmd.vehicle = None;
            return;
        };
        let newest = game
            .interp
            .newest()
            .and_then(|s| s.vehicles.iter().find(|x| x.id == vid))
            .map_or(v.pos, |x| x.pos);
        let fresh = match &mover.drive {
            Some((id, _, m)) => {
                *id != vid
                    || lost(
                        m.pos,
                        &mover.drive_trail,
                        newest,
                        6.0 + m.vel.length() * 0.4,
                    )
            }
            None => true,
        };
        if fresh {
            let (yaw, _, _) = v.rot.to_euler(EulerRot::YXZ);
            let mut m = Motion::at(newest, yaw);
            m.rot = v.rot;
            m.vel = v.vel;
            m.throttle = if v.kind.aircraft() {
                v.vel.dot(v.rot * Vec3::NEG_Z).max(0.0)
            } else {
                v.vel.dot(v.rot * Vec3::NEG_Z)
            };
            m.grounded = !v.kind.aircraft() || v.vel.length() < 0.5;
            mover.drive = Some((vid, v.kind, m));
            mover.drive_trail.clear();
        }
        let deployed = v.flags & ps_sim::snapshot::vflags::DEPLOYED != 0;
        let drive = local.cmd.drive;
        let (_, kind, m) = mover.drive.as_mut().unwrap();
        let spec = kind.spec();
        m.step(&spec, Some(&drive), deployed, &game.world, dt);
        let pos = m.pos;
        local.cmd.vehicle = Some((m.pos, m.rot, m.vel));
        while mover
            .drive_trail
            .front()
            .is_some_and(|(t, _)| now - *t > TRAIL_TIME)
        {
            mover.drive_trail.pop_front();
        }
        mover.drive_trail.push_back((now, pos));
        return;
    }
    mover.drive = None;
    local.cmd.vehicle = None;
    if me.seat.is_some() {
        // A passenger or gunner: the vehicle moves us.
        local.cmd.feet = None;
        mover.valid = false;
        return;
    }
    mover.since_moved = if spawned.is_some() {
        0.0
    } else {
        mover.since_moved + dt
    };
    let server = game
        .interp
        .newest()
        .and_then(|s| s.me.as_ref())
        .map_or(me.feet, |m| m.feet);
    if !mover.valid
        || mover.continent != game.continent
        || spawned.is_some()
        || (lost(mover.body.feet, &mover.trail, server, TRAIL_SLACK) && mover.since_moved > 0.5)
    {
        mover.body = Body::at(spawned.unwrap_or(server));
        mover.body.vy = 0.0;
        mover.body.jumps_seen = Some(local.cmd.jumps);
        mover.valid = true;
        mover.continent = game.continent;
        mover.trail.clear();
    }
    // Jump jets (light armour) and a bailed-out chute, as the server has them.
    mover.body.can_jet = me.loadout.armor == ps_sim::weapons::ArmorKind::Light;
    if me.chute {
        mover.body.chute = true;
    }
    let mut cmd = local.cmd;
    // (A MAX doesn't run: as the server has it.)
    if me.loadout.max() {
        cmd.sprint = false;
    }
    let speed = ps_sim::player::move_speed(&cmd, me.loadout.armor.speed());
    let world = &game.world;
    if let Some(v) = mover.body.step(world, &cmd, speed, dt) {
        mover.landed = Some(v);
    }
    if walk != Vec2::ZERO {
        let (f, _) = world.move_body(mover.body.feet, walk, mover.body.vy, 0.0);
        mover.body.feet = f;
    }
    while mover
        .trail
        .front()
        .is_some_and(|(t, _)| now - *t > TRAIL_TIME)
    {
        mover.trail.pop_front();
    }
    let feet = mover.body.feet;
    mover.trail.push_back((now, feet));
    local.cmd.feet = Some(feet);
}

/// Interpolation factor between `prev` and `snap` for this frame.
pub fn alpha(time: &Time<Fixed>) -> f32 {
    time.overstep_fraction().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(tick: u64, x: f32) -> Snapshot {
        Snapshot {
            tick,
            players: vec![PlayerSnap {
                id: 1,
                team: Team::Red,
                feet: Vec3::new(x, 0.0, 0.0),
                yaw: 0.0,
                pitch: 0.0,
                flags: 0,
                weapon: ps_sim::weapons::Weapon::Rifle,
                health: 100.0,
                seat: None,
                vr: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn jittery_snapshots_play_back_smoothly() {
        let mut interp = Interp::default();
        let mut sent = 0u64;
        let mut xs = Vec::new();
        let arrivals = [0usize, 0, 2, 1, 0, 3, 0, 1, 2, 0, 0, 1];
        for t in 0..600u64 {
            let n = if t % 2 == 0 {
                arrivals[(t as usize / 2) % arrivals.len()]
            } else {
                0
            };
            for _ in 0..n {
                sent += 2;
                interp.receive(snap(sent, sent as f32 * 0.05));
            }
            while sent < t.saturating_sub(8) {
                sent += 2;
                interp.receive(snap(sent, sent as f32 * 0.05));
            }
            if let Some(s) = interp.advance() {
                xs.push(s.players[0].feet.x);
            }
        }
        let steps: Vec<f32> = xs.windows(2).skip(100).map(|w| w[1] - w[0]).collect();
        let (lo, hi) = steps
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &s| (a.min(s), b.max(s)));
        assert!(lo > 0.03 && hi < 0.07, "steady: {lo}..{hi}");
    }
}
