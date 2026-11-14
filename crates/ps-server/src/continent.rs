//! A continent's thread: its simulation stepped at `TICK`, its players' commands queued one a
//! tick, its bots, and what it sends: events as they happen and each player's snapshot (built
//! on a few scoped threads when there are many players).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use ps_net::delta::DeltaSender;
use ps_net::protocol::{ScoreRow, ToClient, encode};
use ps_net::transport::ConnId;
use ps_sim::bots::Bots;
use ps_sim::map::ContinentMap;
use ps_sim::sim::{Player, Vehicle, Warper};
use ps_sim::state::{Hack, WorldState};
use ps_sim::{
    Audience, BaseIdx, ContinentId, ContinentSim, Deny, PlayerCmd, PlayerId, Request, SimEvent,
    TICK, Team, WorldDef,
};

/// Snapshots go out every this many ticks.
pub const SEND_EVERY: u64 = 2;
/// Commands queued per player at most (beyond: the oldest go; presses are counts, so none is
/// lost).
const QUEUE: usize = 4;

/// Where a transferred player goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransferTo {
    /// Spawning at a base of another continent.
    Spawn(ContinentId, BaseIdx),
    /// Spawning on the squad leader (this player) on another continent.
    Squad(ContinentId, PlayerId),
}

/// Where a player joining a continent appears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arrive {
    /// Dead, choosing where to spawn.
    Choosing,
    /// Spawning at this base.
    Base(BaseIdx),
    /// Through this gate.
    Gate(u8),
    /// On its squad leader (or choosing, if it can't be).
    Squad(PlayerId),
}

#[allow(clippy::type_complexity)]
pub enum ToContinent {
    Join {
        player: Box<Player>,
        conn: Option<ConnId>,
        at: Arrive,
    },
    /// The player left the game.
    Leave(PlayerId),
    /// Take the player off this continent and hand it back (`FromContinent::Transferred`).
    Transfer(PlayerId, TransferTo),
    /// Commands, and the newest snapshot the client has.
    Input(PlayerId, Vec<(u32, PlayerCmd)>, u64),
    Request(PlayerId, Request),
    /// Every base's owner, hack, who may hack it and its silo; per gate the owner beyond it and
    /// which factions have something to take there.
    Bases(
        Vec<(Option<Team>, Option<Hack>, [bool; 3], f32)>,
        Vec<(Option<Team>, [bool; 3])>,
    ),
    BotQuota([usize; 3]),
    /// What vehicles, MAXes and structures cost each faction (its locks' discount).
    Costs([f32; 3]),
    /// The structures kept over a restart (once, at the start).
    Structures(Vec<ps_sim::construct::SavedStructure>),
    /// The human-led squads here and their bots (`ps_sim::bots::HumanSquad`).
    BotSquads(Vec<ps_sim::bots::HumanSquad>),
    ArriveVehicle {
        vehicle: Box<Vehicle>,
        riders: Vec<(Player, Option<ConnId>)>,
        gate: u8,
    },
    Stop,
}

pub enum FromContinent {
    /// The continent's structures as they are (to keep over a restart).
    Structures {
        continent: ContinentId,
        list: Vec<ps_sim::construct::SavedStructure>,
    },
    Solved {
        continent: ContinentId,
        base: BaseIdx,
        team: Team,
        by: PlayerId,
    },
    Transferred {
        player: Box<Player>,
        to: TransferTo,
    },
    /// Something went through gate `gate` (already off this continent).
    Warped {
        from: ContinentId,
        gate: u8,
        vehicle: Option<Box<Vehicle>>,
        riders: Vec<Player>,
    },
    /// Players and bots here by faction, and the humans' stats.
    Population {
        continent: ContinentId,
        counts: [u16; 3],
        humans: Vec<(PlayerId, u16, u16)>,
        /// The bots here and their factions.
        bots: Vec<(PlayerId, Team)>,
        /// Every base's silo, and the bases whose silo ran dry.
        resources: Vec<f32>,
        drained: Vec<BaseIdx>,
    },
    Ready(ContinentId),
    /// XP given to the humans here: whom, what for, how much.
    Awards(Vec<(PlayerId, ps_sim::progress::Award, u32)>),
    /// The humans' stats here: whose, which counter, how much.
    Stats(Vec<(PlayerId, String, i64)>),
}

/// A packet for the hub to send.
pub struct Outbound {
    pub conn: ConnId,
    pub reliable: bool,
    pub bytes: Vec<u8>,
}

struct Human {
    conn: ConnId,
    queue: VecDeque<PlayerCmd>,
    last_seq: u32,
    last: PlayerCmd,
    sent_tick: u64,
    events: Vec<SimEvent>,
    deltas: DeltaSender,
}

pub struct Continent {
    pub id: ContinentId,
    pub def: Arc<WorldDef>,
    pub inbox: Receiver<ToContinent>,
    pub hub: Sender<FromContinent>,
    pub out: Sender<Outbound>,
    pub next_id: Arc<AtomicU16>,
    pub skill: f32,
    pub threads: usize,
}

impl Continent {
    pub fn run(self) {
        let cdef = Arc::new(self.def.continents[self.id as usize].clone());
        let map = Arc::new(ContinentMap::build(cdef, false));
        let mut sim = ContinentSim::new(self.id, map);
        let mut bots = Bots::new(self.skill, self.def.params.seed ^ self.id as u64);
        let mut humans: HashMap<PlayerId, Human> = HashMap::new();
        let mut quota = [0usize; 3];
        let _ = self.hub.send(FromContinent::Ready(self.id));
        let mut next = Instant::now();
        let mut last_pop = Instant::now();
        let mut last_scores = Instant::now();
        let mut last_kept = Instant::now();
        loop {
            // Messages.
            while let Ok(msg) = self.inbox.try_recv() {
                match msg {
                    ToContinent::Stop => return,
                    ToContinent::Join { player, conn, at } => {
                        let id = player.id;
                        let bot = player.bot;
                        sim.insert_player(*player);
                        if let Some(conn) = conn {
                            humans.insert(id, Human::new(conn));
                        } else if bot {
                            bots.adopt(id);
                        }
                        match at {
                            Arrive::Choosing => {}
                            Arrive::Squad(leader) => {
                                sim.request(id, Request::Spawn(ps_sim::SpawnPoint::Squad(leader)))
                            }
                            Arrive::Base(b) => {
                                if let Some((p, yaw)) = sim.base_spawn(b as usize) {
                                    sim.spawn_at(id, p, yaw);
                                }
                            }
                            Arrive::Gate(g) => {
                                let (p, yaw) = sim.arrival(g, false);
                                let alive = sim.player(id).is_some_and(|p| p.alive());
                                if alive {
                                    if let Some(pl) = sim.player_mut(id) {
                                        pl.body = ps_sim::player::Body::at(p);
                                        pl.cmd.yaw = yaw;
                                    }
                                    sim.out.push(ps_sim::Outgoing {
                                        to: Audience::Player(id),
                                        ev: SimEvent::Spawned {
                                            player: id,
                                            pos: p,
                                            yaw,
                                        },
                                    });
                                } else {
                                    sim.spawn_at(id, p, yaw);
                                }
                            }
                        }
                    }
                    ToContinent::Leave(id) => {
                        sim.remove_player(id);
                        humans.remove(&id);
                    }
                    ToContinent::Transfer(id, to) => {
                        let ok = match to {
                            TransferTo::Spawn(..) | TransferTo::Squad(..) => sim.may_spawn(id),
                        };
                        if ok {
                            if let Some(p) = sim.remove_player(id) {
                                humans.remove(&id);
                                let _ = self.hub.send(FromContinent::Transferred {
                                    player: Box::new(p),
                                    to,
                                });
                            }
                        } else if let Some(h) = humans.get_mut(&id) {
                            h.events.push(SimEvent::Denied(Deny::NotReady));
                        }
                    }
                    ToContinent::Input(id, recent, ack) => {
                        if let Some(h) = humans.get_mut(&id) {
                            h.deltas.ack(ack);
                            for (seq, cmd) in recent {
                                if seq > h.last_seq {
                                    h.last_seq = seq;
                                    h.queue.push_back(cmd);
                                }
                            }
                            while h.queue.len() > QUEUE {
                                h.queue.pop_front();
                            }
                        }
                    }
                    ToContinent::Request(id, r) => sim.request(id, r),
                    ToContinent::Bases(bases, gates) => {
                        for (b, (owner, hack, hackable, res)) in bases.into_iter().enumerate() {
                            sim.set_base(b as BaseIdx, owner, hack, hackable);
                            sim.set_resources(b as BaseIdx, res);
                        }
                        sim.gate_owners = gates.iter().map(|g| g.0).collect();
                        sim.gate_fronts = gates.iter().map(|g| g.1).collect();
                    }
                    ToContinent::BotQuota(q) => quota = q,
                    ToContinent::Costs(k) => sim.cost_factor = k,
                    ToContinent::Structures(list) => sim.restore_structures(list),
                    ToContinent::BotSquads(list) => bots.human_squads = list,
                    ToContinent::ArriveVehicle {
                        vehicle,
                        riders,
                        gate,
                    } => {
                        let (at, yaw) = sim.arrival(gate, true);
                        let mut players = Vec::new();
                        for (p, conn) in riders {
                            if let Some(conn) = conn {
                                humans.insert(p.id, Human::new(conn));
                            } else if p.bot {
                                bots.adopt(p.id);
                            }
                            players.push(p);
                        }
                        let ids: Vec<PlayerId> = players.iter().map(|p| p.id).collect();
                        sim.insert_vehicle(*vehicle, players, at, yaw);
                        for id in ids {
                            sim.out.push(ps_sim::Outgoing {
                                to: Audience::Player(id),
                                ev: SimEvent::Spawned {
                                    player: id,
                                    pos: at,
                                    yaw,
                                },
                            });
                        }
                    }
                }
            }
            let active =
                !humans.is_empty() || quota.iter().any(|&q| q > 0) || bots.ids().next().is_some();
            if !active {
                std::thread::sleep(Duration::from_millis(20));
                next = Instant::now();
                continue;
            }
            // Bots to their quota.
            for t in Team::ALL {
                let have = bots.count(&sim, t);
                let want = quota[t.index()];
                if have < want {
                    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                    bots.add(&mut sim, id, t);
                } else if have > want {
                    bots.remove_one(&mut sim, t);
                }
            }
            // One command per player per tick.
            for (&id, h) in humans.iter_mut() {
                if let Some(c) = h.queue.pop_front() {
                    h.last = c;
                }
                sim.set_cmd(id, h.last);
            }
            bots.think(&mut sim);
            sim.step();
            // What changes the world goes to the hub.
            for (base, team, by) in std::mem::take(&mut sim.solved) {
                let _ = self.hub.send(FromContinent::Solved {
                    continent: self.id,
                    base,
                    team,
                    by,
                });
            }
            // XP: the humans' to the hub (bots keep only their score).
            let awards: Vec<_> = std::mem::take(&mut sim.awards)
                .into_iter()
                .filter(|a| humans.contains_key(&a.0))
                .collect();
            if !awards.is_empty() {
                let _ = self.hub.send(FromContinent::Awards(awards));
            }
            let stats = std::mem::take(&mut sim.stats);
            if !stats.is_empty() {
                let _ = self.hub.send(FromContinent::Stats(stats));
            }
            self.warps(&mut sim, &mut humans);
            // Events to those they're for.
            for o in std::mem::take(&mut sim.out) {
                for (id, h) in humans.iter_mut() {
                    let to_them = match o.to {
                        Audience::All => true,
                        Audience::Player(p) => p == *id,
                        Audience::Team(t) => sim.player(*id).is_some_and(|p| p.team == t),
                        Audience::Near(at, r) => {
                            sim.viewer_pos(*id).is_some_and(|p| (p - at).length() < r)
                        }
                    };
                    if to_them {
                        h.events.push(o.ev.clone());
                    }
                }
            }
            for h in humans.values_mut() {
                if !h.events.is_empty() {
                    let msg = ToClient::Events(std::mem::take(&mut h.events));
                    let _ = self.out.send(Outbound {
                        conn: h.conn,
                        reliable: true,
                        bytes: encode(&msg),
                    });
                }
            }
            if sim.tick.is_multiple_of(SEND_EVERY) {
                self.snapshots(&sim, &mut humans);
            }
            if last_scores.elapsed() > Duration::from_secs(2) && !humans.is_empty() {
                last_scores = Instant::now();
                let rows = sim
                    .players
                    .iter()
                    .map(|p| ScoreRow {
                        id: p.id,
                        team: p.team,
                        kills: p.kills,
                        deaths: p.deaths,
                        assists: p.assists,
                        score: p.score,
                    })
                    .collect();
                let bytes = encode(&ToClient::Scores(rows));
                for h in humans.values() {
                    let _ = self.out.send(Outbound {
                        conn: h.conn,
                        reliable: true,
                        bytes: bytes.clone(),
                    });
                }
            }
            // The structures, for the server to keep (every ten seconds).
            if last_kept.elapsed() > Duration::from_secs(10) {
                last_kept = Instant::now();
                let _ = self.hub.send(FromContinent::Structures {
                    continent: self.id,
                    list: sim.saved_structures(),
                });
            }
            if last_pop.elapsed() > Duration::from_secs(1) {
                last_pop = Instant::now();
                let mut counts = [0u16; 3];
                for p in &sim.players {
                    counts[p.team.index()] += 1;
                }
                let stats = sim
                    .players
                    .iter()
                    .filter(|p| !p.bot)
                    .map(|p| (p.id, p.kills, p.deaths))
                    .collect();
                let _ = self.hub.send(FromContinent::Population {
                    continent: self.id,
                    counts,
                    humans: stats,
                    bots: sim
                        .players
                        .iter()
                        .filter(|p| p.bot)
                        .map(|p| (p.id, p.team))
                        .collect(),
                    resources: sim.bases.iter().map(|b| b.resources).collect(),
                    drained: std::mem::take(&mut sim.drained),
                });
            }
            next += Duration::from_secs_f32(TICK);
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else if now - next > Duration::from_millis(250) {
                next = now;
            }
        }
    }

    /// Whatever entered a gate's core goes on (if its faction may go there) or is turned back.
    fn warps(&self, sim: &mut ContinentSim, humans: &mut HashMap<PlayerId, Human>) {
        let mut done: Vec<Warper> = Vec::new();
        for (w, gate) in std::mem::take(&mut sim.warps) {
            if done.contains(&w) {
                continue;
            }
            done.push(w);
            let (to, _) = self.def.gate_target(self.id, gate);
            let team = match w {
                Warper::Player(id) => sim.player(id).map(|p| p.team),
                Warper::Vehicle(id) => sim.vehicle(id).map(|v| v.team),
            };
            let Some(team) = team else { continue };
            if !WorldState::can_warp(&self.def, to, team) {
                // Turned back out of the core.
                let (at, yaw) = sim.arrival(gate, matches!(w, Warper::Vehicle(_)));
                match w {
                    Warper::Player(id) => {
                        if let Some(p) = sim.player_mut(id) {
                            p.body = ps_sim::player::Body::at(at);
                            p.cmd.yaw = yaw;
                        }
                        sim.out.push(ps_sim::Outgoing {
                            to: Audience::Player(id),
                            ev: SimEvent::Spawned {
                                player: id,
                                pos: at,
                                yaw,
                            },
                        });
                        sim.out.push(ps_sim::Outgoing {
                            to: Audience::Player(id),
                            ev: SimEvent::Denied(Deny::Sanctuary),
                        });
                    }
                    Warper::Vehicle(id) => {
                        if let Some(v) = sim.vehicles.iter_mut().find(|v| v.id == id) {
                            v.motion.pos = at;
                            v.motion.vel = -v.motion.vel * 0.3;
                        }
                    }
                }
                continue;
            }
            match w {
                Warper::Player(id) => {
                    if let Some(p) = sim.remove_player(id) {
                        humans.remove(&id);
                        let _ = self.hub.send(FromContinent::Warped {
                            from: self.id,
                            gate,
                            vehicle: None,
                            riders: vec![p],
                        });
                    }
                }
                Warper::Vehicle(id) => {
                    if let Some((v, riders)) = sim.remove_vehicle(id) {
                        for r in &riders {
                            humans.remove(&r.id);
                        }
                        let _ = self.hub.send(FromContinent::Warped {
                            from: self.id,
                            gate,
                            vehicle: Some(Box::new(v)),
                            riders,
                        });
                    }
                }
            }
        }
    }

    fn snapshots(&self, sim: &ContinentSim, humans: &mut HashMap<PlayerId, Human>) {
        let mut list: Vec<(PlayerId, ConnId, u64, &mut DeltaSender)> = humans
            .iter_mut()
            .map(|(id, h)| (*id, h.conn, h.sent_tick, &mut h.deltas))
            .collect();
        let continent = self.id;
        let build = |chunk: &mut [(PlayerId, ConnId, u64, &mut DeltaSender)]| -> Vec<Outbound> {
            chunk
                .iter_mut()
                .map(|(id, conn, since, deltas)| {
                    let (id, conn) = (*id, *conn);
                    let snap = deltas.delta(
                        sim.snapshot_for(id, *since),
                        sim.viewer_pos(id).unwrap_or_default(),
                    );
                    Outbound {
                        conn,
                        reliable: false,
                        bytes: encode(&ToClient::Snapshot {
                            continent,
                            snap: Box::new(snap),
                        }),
                    }
                })
                .collect()
        };
        let packets: Vec<Outbound> = if list.len() > 8 && self.threads > 1 {
            let per = list.len().div_ceil(self.threads);
            std::thread::scope(|s| {
                let handles: Vec<_> = list
                    .chunks_mut(per)
                    .map(|c| s.spawn(move || build(c)))
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|h| h.join().unwrap_or_default())
                    .collect()
            })
        } else {
            build(&mut list)
        };
        for p in packets {
            let _ = self.out.send(p);
        }
        for h in humans.values_mut() {
            h.sent_tick = sim.tick;
        }
    }
}

impl Human {
    fn new(conn: ConnId) -> Human {
        Human {
            conn,
            queue: VecDeque::new(),
            last_seq: 0,
            last: PlayerCmd::default(),
            sent_tick: 0,
            events: Vec::new(),
            deltas: DeltaSender::default(),
        }
    }
}
