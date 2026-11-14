//! The planetstrike server: the whole world on one machine. A hub thread owns the network
//! endpoint and the world state (who holds which base and continent, the hacks under way);
//! every continent runs on its own thread (`continent.rs`), with a few more building its
//! snapshots when it is busy.
//!
//! - Players join on their faction's sanctuary and spawn at any base their faction holds, on
//!   any continent; they warp between continents through the gates.
//! - Bots fight only on contested continents, so many per continent split between the
//!   factions with a stake there, making room for that faction's humans.
//! - The world state is kept on disk (`Config::state_path`) and survives restarts.
//!
//! The dedicated server (`main.rs`) and the game's own local and hosted games use this.

pub mod accounts;
pub mod alerts;
pub mod continent;
pub mod profiles;
pub mod squads;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use continent::{Arrive, Continent, FromContinent, Outbound, ToContinent, TransferTo};
use ps_net::protocol::{PROTOCOL, RosterEntry, SquadCmd, ToClient, ToServer, decode, encode};
use ps_net::transport::{Channel, ConnId, Endpoint, NetEvent};
use ps_net::{LanAnnouncer, PortMapper, ReportPlayer, Reporter, ServerReport};
use ps_sim::sim::Player;
use ps_sim::state::{Change, WorldState};
use ps_sim::{
    BaseIdx, ContinentId, Deny, PlayerId, Request, SimEvent, SpawnPoint, Team, WorldDef,
    WorldParams,
};

#[derive(Clone, Debug)]
pub struct Config {
    /// Where to listen ("0.0.0.0:27430"; port 0: any free one).
    pub bind: String,
    pub name: String,
    pub params: WorldParams,
    /// Bots on each contested continent (split between the factions there).
    pub bots: usize,
    /// The bots' skill, 0..1.
    pub skill: f32,
    pub max_players: usize,
    /// Where the world state is kept (none: not kept).
    pub state_path: Option<PathBuf>,
    /// Report to this master server (none: unlisted).
    pub master: Option<String>,
    /// Announce on the local network.
    pub lan: bool,
    /// Open the port on the router (UPnP / NAT-PMP).
    pub port_map: bool,
    pub dedicated: bool,
    /// Threads per continent building snapshots.
    pub snapshot_threads: usize,
    /// Print what happens.
    pub verbose: bool,
    /// Player accounts' database (log in or register before playing; none: anyone plays, by
    /// any name).
    pub accounts: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: format!("0.0.0.0:{}", ps_net::protocol::DEFAULT_PORT),
            name: "Planetstrike server".into(),
            params: WorldParams::default(),
            bots: 24,
            skill: 0.5,
            max_players: 200,
            state_path: None,
            master: None,
            lan: false,
            port_map: false,
            dedicated: true,
            snapshot_threads: 2,
            verbose: false,
            accounts: None,
        }
    }
}

/// A running server; stops when dropped.
pub struct Handle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub port: u16,
    /// What the server is up to (for a hosting game's menus).
    pub status: Arc<Mutex<String>>,
}

impl Handle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    pub fn status(&self) -> String {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the server: binds its port now (so the error comes back here), then runs on its own
/// threads.
pub fn start(cfg: Config) -> std::io::Result<Handle> {
    let endpoint = Endpoint::server(&cfg.bind, cfg.max_players)?;
    let port = endpoint.local_addr().map_or(0, |a| a.port());
    let stop = Arc::new(AtomicBool::new(false));
    let status = Arc::new(Mutex::new(String::new()));
    let (s2, st2) = (stop.clone(), status.clone());
    let thread = std::thread::Builder::new()
        .name("ps-hub".into())
        .spawn(move || Hub::new(cfg, endpoint, port, st2).run(&s2))?;
    Ok(Handle {
        stop,
        thread: Some(thread),
        port,
        status,
    })
}

struct PInfo {
    conn: ConnId,
    name: String,
    team: Team,
    continent: Option<ContinentId>,
    kills: u16,
    deaths: u16,
    /// XP in all (the profile's, or the account's).
    xp: u64,
    /// The account it logged in to (a server with accounts).
    account: Option<i64>,
    /// When it joined (its time played goes into its stats as it leaves).
    joined: Instant,
}

struct Hub {
    cfg: Config,
    def: Arc<WorldDef>,
    state: WorldState,
    endpoint: Endpoint,
    port: u16,
    status: Arc<Mutex<String>>,
    links: Vec<(Sender<ToContinent>, Option<JoinHandle<()>>)>,
    from: Receiver<FromContinent>,
    out: Receiver<Outbound>,
    players: HashMap<PlayerId, PInfo>,
    conns: HashMap<ConnId, PlayerId>,
    squads: squads::Squads,
    profiles: profiles::Profiles,
    next_id: Arc<AtomicU16>,
    population: Vec<[u16; 3]>,
    alerts: alerts::Alerts,
    /// Player accounts (a server with them), and the connections waiting to log in (with the
    /// faction they asked for).
    accounts: Option<accounts::Accounts>,
    pending: HashMap<ConnId, Option<Team>>,
    /// The bots on each continent and their factions (as the continents say).
    bots: Vec<Vec<(PlayerId, Team)>>,
    /// Each continent's structures (kept in `structures.ron` beside the world's state).
    structures: Vec<Vec<ps_sim::construct::SavedStructure>>,
    structures_dirty: bool,
    ready: usize,
    dirty: bool,
    reporter: Option<Reporter>,
    lan: Option<LanAnnouncer>,
    mapper: Option<PortMapper>,
    started: Instant,
}

impl Hub {
    fn new(cfg: Config, endpoint: Endpoint, port: u16, status: Arc<Mutex<String>>) -> Hub {
        let def = Arc::new(WorldDef::generate(cfg.params));
        let kept = cfg
            .state_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| WorldState::from_ron(&s))
            .filter(|s| s.fits(&def));
        // (A new world is written out at once.)
        let dirty = kept.is_none();
        let state = kept.unwrap_or_else(|| WorldState::initial(&def));
        let (hub_tx, from) = channel();
        let (out_tx, out) = channel();
        let next_id = Arc::new(AtomicU16::new(1));
        let links: Vec<(Sender<ToContinent>, Option<JoinHandle<()>>)> = (0..def.continents.len())
            .map(|c| {
                let (tx, rx) = channel();
                let cont = Continent {
                    id: c as ContinentId,
                    def: def.clone(),
                    inbox: rx,
                    hub: hub_tx.clone(),
                    out: out_tx.clone(),
                    next_id: next_id.clone(),
                    skill: cfg.skill,
                    threads: cfg.snapshot_threads.max(1),
                };
                let h = std::thread::Builder::new()
                    .name(format!("ps-continent-{c}"))
                    .spawn(move || cont.run())
                    .ok();
                (tx, h)
            })
            .collect();
        let reporter = cfg.master.as_ref().map(|m| Reporter::new(m));
        let lan = cfg.lan.then(LanAnnouncer::new);
        let mapper = cfg.port_map.then(|| PortMapper::open(port));
        let n = def.continents.len();
        let accounts = cfg
            .accounts
            .as_ref()
            .and_then(|p| match accounts::Accounts::open(p) {
                Ok(a) => Some(a),
                Err(e) => {
                    eprintln!("accounts ({}): {e}: playing without", p.display());
                    None
                }
            });
        // The structures kept (none if the world was new).
        let structures: Vec<Vec<ps_sim::construct::SavedStructure>> = cfg
            .state_path
            .as_ref()
            .filter(|_| !dirty)
            .and_then(|p| std::fs::read_to_string(p.with_file_name("structures.ron")).ok())
            .and_then(|s| ron::from_str(&s).ok())
            .filter(|v: &Vec<Vec<_>>| v.len() == n)
            .unwrap_or_else(|| vec![Vec::new(); n]);
        for (c, list) in structures.iter().enumerate() {
            if !list.is_empty() {
                let _ = links[c].0.send(ToContinent::Structures(list.clone()));
            }
        }
        let profiles = profiles::Profiles::load(
            cfg.state_path
                .as_ref()
                .map(|p| p.with_file_name("profiles.ron")),
        );
        Hub {
            cfg,
            def,
            state,
            endpoint,
            port,
            status,
            links,
            from,
            out,
            players: HashMap::new(),
            conns: HashMap::new(),
            squads: squads::Squads::default(),
            profiles,
            next_id,
            population: vec![[0; 3]; n],
            alerts: alerts::Alerts::new(Instant::now()),
            accounts,
            pending: HashMap::new(),
            structures,
            structures_dirty: false,
            bots: vec![Vec::new(); n],
            ready: 0,
            dirty,
            reporter,
            lan,
            mapper,
            started: Instant::now(),
        }
    }

    fn log(&self, line: &str) {
        if self.cfg.verbose {
            println!("{line}");
        }
    }

    fn send(&mut self, conn: ConnId, reliable: bool, msg: &ToClient) {
        let ch = if reliable {
            Channel::Reliable
        } else {
            Channel::Unreliable
        };
        self.endpoint.send(conn, ch, &encode(msg));
    }

    fn broadcast(&mut self, msg: &ToClient) {
        let bytes = encode(msg);
        let conns: Vec<ConnId> = self.players.values().map(|p| p.conn).collect();
        for c in conns {
            self.endpoint.send(c, Channel::Reliable, &bytes);
        }
    }

    fn to_continent(&self, c: ContinentId, msg: ToContinent) {
        if let Some((tx, _)) = self.links.get(c as usize) {
            let _ = tx.send(msg);
        }
    }

    fn run(mut self, stop: &AtomicBool) {
        let mut last_world = Instant::now();
        let mut last_hacks = Instant::now();
        let mut last_quota = Instant::now() - Duration::from_secs(10);
        let mut last_pop = Instant::now();
        let mut last_save = Instant::now();
        let mut last_report = Instant::now() - Duration::from_secs(60);
        let mut last_clock = Instant::now();
        self.push_bases();
        let msg = format!(
            "\"{}\" on UDP {}: seed {}, {} continents",
            self.cfg.name,
            self.port,
            self.def.params.seed,
            self.def.continents.len()
        );
        self.log(&msg);
        *self.status.lock().unwrap() = msg;
        while !stop.load(Ordering::Relaxed) {
            let now = Instant::now();
            for ev in self.endpoint.poll(now) {
                self.net_event(ev);
            }
            while let Ok(m) = self.from.try_recv() {
                self.on_continent(m);
            }
            while let Ok(o) = self.out.try_recv() {
                let ch = if o.reliable {
                    Channel::Reliable
                } else {
                    Channel::Unreliable
                };
                self.endpoint.send(o.conn, ch, &o.bytes);
            }
            if last_hacks.elapsed() >= Duration::from_secs(1) {
                let dt = last_hacks.elapsed().as_secs_f32();
                last_hacks = Instant::now();
                let mut changes = Vec::new();
                for c in 0..self.def.continents.len() {
                    changes.extend(self.state.tick_hacks(&self.def, c as ContinentId, dt));
                }
                if !changes.is_empty() {
                    self.changed(changes);
                }
            }
            if last_quota.elapsed() >= Duration::from_secs(2) && self.ready == self.links.len() {
                last_quota = Instant::now();
                self.quotas();
            }
            if last_pop.elapsed() >= Duration::from_secs(2) {
                last_pop = Instant::now();
                let p = self.population_msg();
                self.broadcast(&p);
                self.tick_alerts();
            }
            if last_clock.elapsed() >= Duration::from_secs(20) {
                last_clock = Instant::now();
                let c = ToClient::Clock(self.day());
                self.broadcast(&c);
            }
            if last_world.elapsed() >= Duration::from_secs(15) {
                last_world = Instant::now();
                let w = ToClient::World(self.state.clone());
                self.broadcast(&w);
            }
            if (self.dirty || self.profiles.dirty || self.structures_dirty)
                && last_save.elapsed() >= Duration::from_secs(5)
            {
                last_save = Instant::now();
                self.save();
            }
            if last_report.elapsed() >= Duration::from_secs(2) {
                last_report = Instant::now();
                self.report();
            }
            self.endpoint.flush(Instant::now());
            std::thread::sleep(Duration::from_millis(1));
        }
        // Shutting down: tell everyone, keep the world, stop the continents.
        self.broadcast(&ToClient::Shutdown("The server stopped".into()));
        self.endpoint.flush(Instant::now());
        self.save();
        for (tx, _) in &self.links {
            let _ = tx.send(ToContinent::Stop);
        }
        for (_, h) in &mut self.links {
            if let Some(h) = h.take() {
                let _ = h.join();
            }
        }
        if let Some(mut l) = self.lan.take() {
            l.stop();
        }
        if let Some(mut r) = self.reporter.take() {
            r.stop();
        }
    }

    /// The time of day (0..1), the world starting at morning.
    fn day(&self) -> f32 {
        (0.3 + self.started.elapsed().as_secs_f32() / ps_net::protocol::DAY_LENGTH).fract()
    }

    fn save(&mut self) {
        if self.profiles.dirty {
            self.profiles.save();
        }
        if std::mem::take(&mut self.structures_dirty)
            && let Some(path) = &self.cfg.state_path
            && let Ok(text) = ron::ser::to_string(&self.structures)
        {
            let path = path.with_file_name("structures.ron");
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
        self.dirty = false;
        if let Some(path) = &self.cfg.state_path {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, self.state.to_ron()).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }

    fn net_event(&mut self, ev: NetEvent) {
        match ev {
            NetEvent::Connected(_) => {}
            NetEvent::Disconnected(conn, why) => {
                self.pending.remove(&conn);
                if let Some(id) = self.conns.remove(&conn) {
                    let played = self
                        .players
                        .get(&id)
                        .map_or(0, |p| p.joined.elapsed().as_secs() as i64);
                    self.add_stats(id, &[("seconds_played".into(), played)]);
                    if let Some(p) = self.players.remove(&id) {
                        self.log(&format!("{} left ({why})", p.name));
                        if let Some(c) = p.continent {
                            self.to_continent(c, ToContinent::Leave(id));
                        }
                    }
                    if let Some(t) = self.squads.leave(id) {
                        self.send_squads(t);
                    }
                    self.roster();
                }
            }
            NetEvent::Message(conn, _, bytes) => {
                let Some(msg) = decode::<ToServer>(&bytes) else {
                    return;
                };
                match msg {
                    ToServer::Hello {
                        protocol,
                        name,
                        team,
                    } => self.hello(conn, protocol, &name, team),
                    ToServer::Input { recent, ack } => {
                        if let Some(&id) = self.conns.get(&conn)
                            && let Some(c) = self.players.get(&id).and_then(|p| p.continent)
                        {
                            self.to_continent(c, ToContinent::Input(id, recent, ack));
                        }
                    }
                    ToServer::Request(r) => {
                        if let Some(&id) = self.conns.get(&conn) {
                            self.request(id, r);
                        }
                    }
                    ToServer::Chat(text) => {
                        if let Some(&id) = self.conns.get(&conn) {
                            let text: String = text.chars().take(160).collect();
                            self.broadcast(&ToClient::Chat {
                                from: id,
                                text,
                                squad: false,
                            });
                        }
                    }
                    ToServer::SquadChat(text) => {
                        if let Some(&id) = self.conns.get(&conn)
                            && let Some(s) = self.squads.of(id)
                        {
                            let text: String = text.chars().take(160).collect();
                            let msg = encode(&ToClient::Chat {
                                from: id,
                                text,
                                squad: true,
                            });
                            let conns: Vec<ConnId> = s
                                .members
                                .iter()
                                .filter_map(|m| self.players.get(m).map(|p| p.conn))
                                .collect();
                            for c in conns {
                                self.endpoint.send(c, Channel::Reliable, &msg);
                            }
                        }
                    }
                    ToServer::TeamChat(text) => {
                        if let Some(&id) = self.conns.get(&conn)
                            && let Some(team) = self.players.get(&id).map(|p| p.team)
                        {
                            let text: String = text.chars().take(160).collect();
                            let msg = encode(&ToClient::Chat {
                                from: id,
                                text,
                                squad: false,
                            });
                            let conns: Vec<ConnId> = self
                                .players
                                .values()
                                .filter(|p| p.team == team)
                                .map(|p| p.conn)
                                .collect();
                            for c in conns {
                                self.endpoint.send(c, Channel::Reliable, &msg);
                            }
                        }
                    }
                    ToServer::Squad(cmd) => {
                        if let Some(&id) = self.conns.get(&conn) {
                            self.squad(id, cmd);
                        }
                    }
                    ToServer::Login {
                        name,
                        secret,
                        register,
                    } => self.login(conn, &name, &secret, register),
                    ToServer::Stats => {
                        if let Some(&id) = self.conns.get(&conn) {
                            let stats = self.stats_of(id);
                            self.send(conn, true, &ToClient::Stats(stats));
                        }
                    }
                    ToServer::Bye => self.endpoint.close(conn, "Bye"),
                }
            }
        }
    }

    fn hello(&mut self, conn: ConnId, protocol: u16, name: &str, team: Option<Team>) {
        if protocol != PROTOCOL {
            self.send(
                conn,
                true,
                &ToClient::Reject(format!(
                    "The server plays protocol {PROTOCOL}, the game {protocol}: update"
                )),
            );
            self.endpoint.close(conn, "protocol");
            return;
        }
        if self.conns.contains_key(&conn) {
            return;
        }
        // A server with accounts: a login first.
        if self.accounts.is_some() {
            self.pending.insert(conn, team);
            let server = self.cfg.name.clone();
            self.send(conn, true, &ToClient::LoginRequired { server });
            return;
        }
        self.join(conn, name, team, None);
    }

    /// A login (or registration) on a server with accounts: in if it holds.
    fn login(&mut self, conn: ConnId, name: &str, secret: &str, register: bool) {
        let Some(&team) = self.pending.get(&conn) else {
            return;
        };
        let Some(acc) = self.accounts.as_mut() else {
            return;
        };
        let got = if register {
            acc.register(name.trim(), secret)
        } else {
            acc.login(name.trim(), secret)
        };
        let got = got.and_then(|a| {
            if self.players.values().any(|p| p.account == Some(a.id)) {
                Err(format!("\"{}\" is playing already", a.name))
            } else {
                Ok(a)
            }
        });
        match got {
            Ok(a) => {
                self.pending.remove(&conn);
                self.log(&format!(
                    "{} {}",
                    a.name,
                    if register { "registered" } else { "logged in" }
                ));
                self.join(conn, &a.name.clone(), team, Some(a));
            }
            Err(why) => self.send(conn, true, &ToClient::LoginFailed(why)),
        }
    }

    /// Player `name` (an account's, or as given) joins the game.
    fn join(
        &mut self,
        conn: ConnId,
        name: &str,
        team: Option<Team>,
        account: Option<accounts::Account>,
    ) {
        // The smallest faction unless asked.
        let team = team.unwrap_or_else(|| {
            *Team::ALL
                .iter()
                .min_by_key(|t| self.players.values().filter(|p| p.team == **t).count())
                .unwrap()
        });
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let name: String = name.trim().chars().take(24).collect();
        let name = if name.is_empty() {
            format!("Player {id}")
        } else {
            name
        };
        self.log(&format!("{name} joined {}", team.name()));
        let home = team.index() as ContinentId;
        self.players.insert(
            id,
            PInfo {
                conn,
                name: name.clone(),
                team,
                continent: Some(home),
                kills: 0,
                deaths: 0,
                xp: account
                    .as_ref()
                    .map_or_else(|| self.profiles.get(&name).xp, |a| a.xp),
                account: account.as_ref().map(|a| a.id),
                joined: Instant::now(),
            },
        );
        self.conns.insert(conn, id);
        self.send(
            conn,
            true,
            &ToClient::Welcome {
                id,
                params: self.def.params,
                team,
                server: self.cfg.name.clone(),
            },
        );
        let w = ToClient::World(self.state.clone());
        self.send(conn, true, &w);
        self.send(conn, true, &ToClient::Enter { continent: home });
        let mut p = Player::new(id, &name, team, false);
        p.dead = Some(ps_sim::sim::RESPAWN_TIME);
        self.to_continent(
            home,
            ToContinent::Join {
                player: Box::new(p),
                conn: Some(conn),
                at: Arrive::Choosing,
            },
        );
        self.roster();
        let pop = self.population_msg();
        self.send(conn, true, &pop);
        let alert = ToClient::Alert(self.alerts.info(Instant::now()));
        self.send(conn, true, &alert);
        let c = ToClient::Clock(self.day());
        self.send(conn, true, &c);
        let s = ToClient::Squads(self.squads.info(team));
        self.send(conn, true, &s);
        let xp = self.players.get(&id).map_or(0, |p| p.xp);
        self.send(conn, true, &ToClient::Progress(xp));
    }

    fn squad(&mut self, id: PlayerId, cmd: SquadCmd) {
        let Some(team) = self.players.get(&id).map(|p| p.team) else {
            return;
        };
        let n = self.def.continents.len();
        if let Some(t) = self.squads.apply(id, team, cmd, n) {
            self.send_squads(t);
        }
    }

    /// Tells faction `team` of its squads.
    fn send_squads(&mut self, team: Team) {
        let msg = encode(&ToClient::Squads(self.squads.info(team)));
        let conns: Vec<ConnId> = self
            .players
            .values()
            .filter(|p| p.team == team)
            .map(|p| p.conn)
            .collect();
        for c in conns {
            self.endpoint.send(c, Channel::Reliable, &msg);
        }
    }

    fn request(&mut self, id: PlayerId, r: Request) {
        let Some(info) = self.players.get(&id) else {
            return;
        };
        let Some(here) = info.continent else { return };
        let team = info.team;
        let conn = info.conn;
        // Not onto another continent where one's faction is overpopulated.
        let target = match r {
            Request::Spawn(SpawnPoint::Base { continent, .. }) => Some(continent),
            Request::Spawn(SpawnPoint::Squad(leader)) => {
                self.players.get(&leader).and_then(|l| l.continent)
            }
            _ => None,
        };
        if let Some(c) = target.filter(|&c| c != here)
            && self.overpopulated(c, team)
        {
            self.send(
                conn,
                true,
                &ToClient::Events(vec![SimEvent::Denied(Deny::Overpopulated)]),
            );
            return;
        }
        if let Request::Spawn(SpawnPoint::Squad(leader)) = r {
            // Only on the leader of one's own squad, wherever it is.
            let there = self
                .squads
                .of(id)
                .filter(|s| s.leader() == leader && leader != id)
                .and_then(|_| self.players.get(&leader))
                .and_then(|l| l.continent);
            let Some(there) = there else {
                self.send(
                    conn,
                    true,
                    &ToClient::Events(vec![SimEvent::Denied(Deny::NotInSquad)]),
                );
                return;
            };
            if there != here {
                self.to_continent(
                    here,
                    ToContinent::Transfer(id, TransferTo::Squad(there, leader)),
                );
                return;
            }
        }
        if let Request::Spawn(SpawnPoint::Base { continent, base }) = r
            && continent != here
        {
            let ok = self
                .state
                .continents
                .get(continent as usize)
                .and_then(|c| c.bases.get(base as usize))
                .is_some_and(|b| b.owner == Some(team));
            if !ok {
                self.send(
                    conn,
                    true,
                    &ToClient::Events(vec![SimEvent::Denied(Deny::NotYours)]),
                );
                return;
            }
            self.to_continent(
                here,
                ToContinent::Transfer(id, TransferTo::Spawn(continent, base)),
            );
            return;
        }
        self.to_continent(here, ToContinent::Request(id, r));
    }

    fn on_continent(&mut self, m: FromContinent) {
        match m {
            FromContinent::Ready(_) => self.ready += 1,
            FromContinent::Stats(list) => {
                // (Summed by player and counter first: one write each.)
                let mut by: HashMap<PlayerId, HashMap<String, i64>> = HashMap::new();
                for (id, key, n) in list {
                    *by.entry(id).or_default().entry(key).or_default() += n;
                }
                for (id, add) in by {
                    let add: Vec<(String, i64)> = add.into_iter().collect();
                    self.add_stats(id, &add);
                }
            }
            FromContinent::Structures { continent, list } => {
                if let Some(kept) = self.structures.get_mut(continent as usize)
                    && *kept != list
                {
                    *kept = list;
                    self.structures_dirty = true;
                }
            }
            FromContinent::Awards(list) => {
                let mut ranked_up = false;
                let mut told: Vec<PlayerId> = Vec::new();
                for (id, reason, xp) in list {
                    let Some(info) = self.players.get_mut(&id) else {
                        continue;
                    };
                    let before = ps_sim::progress::battle_rank(info.xp);
                    info.xp = match (info.account, self.accounts.as_mut()) {
                        (Some(u), Some(acc)) => acc.add_xp(u, xp),
                        _ => self.profiles.add(&info.name, reason, xp).xp,
                    };
                    ranked_up |= ps_sim::progress::battle_rank(info.xp) != before;
                    if !told.contains(&id) {
                        told.push(id);
                    }
                }
                for id in told {
                    if let Some(info) = self.players.get(&id) {
                        let (conn, xp) = (info.conn, info.xp);
                        self.send(conn, true, &ToClient::Progress(xp));
                    }
                }
                if ranked_up {
                    self.roster();
                }
            }
            FromContinent::Solved {
                continent,
                base,
                team,
                by,
            } => {
                let ch = self.state.console_solved(&self.def, continent, base, team);
                if !ch.is_empty() {
                    let who = self
                        .players
                        .get(&by)
                        .map_or_else(|| ps_sim::bots::bot_name(by), |p| p.name.clone());
                    self.log(&format!(
                        "{who} ({}) solved a console: {:?}",
                        team.name(),
                        ch[0]
                    ));
                    self.changed(ch);
                }
            }
            FromContinent::Majority {
                continent,
                base,
                team,
            } => {
                let ch = self.state.points_held(&self.def, continent, base, team);
                if !ch.is_empty() {
                    self.log(&format!("capture points: {:?}", ch[0]));
                    self.changed(ch);
                }
            }
            FromContinent::Transferred { player, to } => {
                let (c, at) = match to {
                    TransferTo::Spawn(c, b) => (c, Arrive::Base(b)),
                    TransferTo::Squad(c, leader) => (c, Arrive::Squad(leader)),
                };
                {
                    let id = player.id;
                    let conn = self.players.get(&id).map(|p| p.conn);
                    if let Some(info) = self.players.get_mut(&id) {
                        info.continent = Some(c);
                    }
                    if let Some(conn) = conn {
                        self.send(conn, true, &ToClient::Enter { continent: c });
                    }
                    self.to_continent(c, ToContinent::Join { player, conn, at });
                    self.roster();
                }
            }
            FromContinent::Warped {
                from,
                gate,
                vehicle,
                riders,
            } => {
                let (to, to_gate) = self.def.gate_target(from, gate);
                let mut with_conns = Vec::new();
                for p in riders {
                    let conn = self.players.get(&p.id).map(|i| i.conn);
                    if let Some(info) = self.players.get_mut(&p.id) {
                        info.continent = Some(to);
                    }
                    if let Some(conn) = conn {
                        self.send(conn, true, &ToClient::Enter { continent: to });
                    }
                    with_conns.push((p, conn));
                }
                match vehicle {
                    Some(v) => self.to_continent(
                        to,
                        ToContinent::ArriveVehicle {
                            vehicle: v,
                            riders: with_conns,
                            gate: to_gate,
                        },
                    ),
                    None => {
                        for (p, conn) in with_conns {
                            self.to_continent(
                                to,
                                ToContinent::Join {
                                    player: Box::new(p),
                                    conn,
                                    at: Arrive::Gate(to_gate),
                                },
                            );
                        }
                    }
                }
                self.roster();
            }
            FromContinent::Population {
                continent,
                counts,
                humans,
                bots,
                resources,
                drained,
            } => {
                if let Some(b) = self.bots.get_mut(continent as usize) {
                    *b = bots;
                }
                if let Some(cs) = self.state.continents.get_mut(continent as usize) {
                    for (b, r) in cs.bases.iter_mut().zip(resources) {
                        if (b.resources - r).abs() > 0.5 {
                            self.dirty = true;
                        }
                        b.resources = r;
                    }
                }
                let mut changes = Vec::new();
                for b in drained {
                    changes.extend(self.state.drain(continent, b));
                }
                if !changes.is_empty() {
                    self.changed(changes);
                }
                if let Some(p) = self.population.get_mut(continent as usize) {
                    *p = counts;
                }
                for (id, k, d) in humans {
                    if let Some(info) = self.players.get_mut(&id) {
                        info.kills = k;
                        info.deaths = d;
                    }
                }
            }
        }
    }

    /// Adds to human player `id`'s stats: its account's, or its profile's.
    fn add_stats(&mut self, id: PlayerId, add: &[(String, i64)]) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        match (p.account, self.accounts.as_mut()) {
            (Some(u), Some(acc)) => acc.add_stats(u, add),
            _ => self.profiles.add_stats(&p.name.clone(), add),
        }
    }

    /// Player `id`'s stats: its account's, or (without one) its profile's counters.
    fn stats_of(&self, id: PlayerId) -> Vec<(String, i64)> {
        let Some(p) = self.players.get(&id) else {
            return Vec::new();
        };
        match (p.account, self.accounts.as_ref()) {
            (Some(u), Some(acc)) => acc.stats(u),
            _ => self.profiles.counters(&p.name),
        }
    }

    /// The human players on each continent, by faction.
    fn humans(&self) -> Vec<[u16; 3]> {
        let mut out = vec![[0u16; 3]; self.def.continents.len()];
        for p in self.players.values() {
            if let Some(c) = p.continent.and_then(|c| out.get_mut(c as usize)) {
                c[p.team.index()] += 1;
            }
        }
        out
    }

    fn population_msg(&self) -> ToClient {
        ToClient::Population {
            all: self.population.clone(),
            humans: self.humans(),
        }
    }

    /// Whether `team` is overpopulated on continent `c` (by humans, against the factions with
    /// a stake there).
    fn overpopulated(&self, c: ContinentId, team: Team) -> bool {
        let h = self.humans().get(c as usize).copied().unwrap_or_default();
        let rivals = self.state.factions_on(&self.def, c);
        ps_net::protocol::overpopulated(h, team, &rivals)
    }

    /// Gives `xp` for `reason` to human player `id` (its XP popup too).
    fn give_xp(&mut self, id: PlayerId, reason: ps_sim::progress::Award, xp: u32) {
        let Some(info) = self.players.get_mut(&id) else {
            return;
        };
        info.xp = match (info.account, self.accounts.as_mut()) {
            (Some(u), Some(acc)) => acc.add_xp(u, xp),
            _ => self.profiles.add(&info.name, reason, xp).xp,
        };
        let (conn, total) = (info.conn, info.xp);
        self.send(
            conn,
            true,
            &ToClient::Events(vec![SimEvent::Xp { reason, xp }]),
        );
        self.send(conn, true, &ToClient::Progress(total));
    }

    /// Alerts starting and ending: everyone told; the winners paid.
    fn tick_alerts(&mut self) {
        let now = Instant::now();
        match self
            .alerts
            .tick(now, &self.def, &self.state, &self.population)
        {
            Some(alerts::AlertEvent::Started(c)) => {
                let name = self.def.continents[c as usize].name.clone();
                self.log(&format!("alert on {name}"));
                let a = ToClient::Alert(self.alerts.info(now));
                self.broadcast(&a);
            }
            Some(alerts::AlertEvent::Ended(c, winner)) => {
                let name = self.def.continents[c as usize].name.clone();
                self.log(&format!(
                    "alert on {name} ended: {}",
                    winner.map_or("a draw", |t| t.name())
                ));
                self.broadcast(&ToClient::Alert(None));
                self.broadcast(&ToClient::AlertEnded {
                    continent: c,
                    winner,
                });
                if let Some(w) = winner {
                    let paid: Vec<(PlayerId, u32)> = self
                        .players
                        .iter()
                        .filter(|(_, p)| p.team == w)
                        .map(|(id, p)| {
                            let xp = if p.continent == Some(c) {
                                ps_sim::progress::ALERT_XP
                            } else {
                                ps_sim::progress::ALERT_XP_AWAY
                            };
                            (*id, xp)
                        })
                        .collect();
                    for (id, xp) in paid {
                        self.give_xp(id, ps_sim::progress::Award::Alert, xp);
                    }
                }
            }
            None => {
                // (Its time left, now and then.)
                if self.alerts.active.is_some() {
                    let a = ToClient::Alert(self.alerts.info(now));
                    self.broadcast(&a);
                }
            }
        }
    }

    /// What vehicles, MAXes and structures cost each faction: 10 % less for each continent it
    /// has locked (not counting sanctuaries), at most 30 %.
    fn cost_factors(&self) -> [f32; 3] {
        Team::ALL.map(|t| {
            let locked = self
                .state
                .continents
                .iter()
                .enumerate()
                .filter(|(c, s)| s.owner == Some(t) && self.def.continents[*c].sanctuary.is_none())
                .count();
            1.0 - 0.1 * locked.min(3) as f32
        })
    }

    /// The world changed: tell everyone, and the continents what follows for them.
    fn changed(&mut self, changes: Vec<Change>) {
        for ch in &changes {
            if let Change::ContinentCaptured {
                continent, team, ..
            } = ch
            {
                let name = self.def.continents[*continent as usize].name.clone();
                self.log(&format!("{} captured {name}", team.name()));
                // Those of the faction there earn the lock.
                let there: Vec<PlayerId> = self
                    .players
                    .iter()
                    .filter(|(_, p)| p.team == *team && p.continent == Some(*continent))
                    .map(|(id, _)| *id)
                    .collect();
                for id in there {
                    self.give_xp(id, ps_sim::progress::Award::Lock, ps_sim::progress::LOCK_XP);
                }
            }
        }
        self.broadcast(&ToClient::Changes(changes));
        self.push_bases();
        self.dirty = true;
    }

    fn push_bases(&mut self) {
        for (ci, cd) in self.def.continents.iter().enumerate() {
            let c = ci as ContinentId;
            let bases = (0..cd.bases.len())
                .map(|b| {
                    let st = self.state.base(c, b as BaseIdx);
                    let hackable =
                        Team::ALL.map(|t| self.state.can_hack(&self.def, c, b as BaseIdx, t));
                    (st.owner, st.hack, hackable, st.resources)
                })
                .collect();
            let gates = cd
                .gates
                .iter()
                .map(|g| {
                    let far = &self.def.continents[g.to as usize];
                    let front = Team::ALL.map(|t| {
                        WorldState::can_warp(&self.def, g.to, t)
                            && (0..far.bases.len())
                                .any(|b| self.state.can_hack(&self.def, g.to, b as BaseIdx, t))
                    });
                    (self.state.continents[g.to as usize].owner, front)
                })
                .collect();
            self.to_continent(c, ToContinent::Bases(bases, gates));
            self.to_continent(c, ToContinent::Costs(self.cost_factors()));
        }
    }

    /// Bots in human-led squads: filled (one place left for a human), the factions told, and
    /// each continent told which of its bots follow whom.
    fn squad_bots(&mut self) {
        let bots: Vec<(PlayerId, Team, ContinentId)> = self
            .bots
            .iter()
            .enumerate()
            .flat_map(|(c, list)| list.iter().map(move |&(id, t)| (id, t, c as ContinentId)))
            .collect();
        let players = &self.players;
        let changed = self
            .squads
            .fill_bots(|id| players.get(&id).map(|p| p.continent), &bots);
        for t in changed {
            self.send_squads(t);
        }
        for c in 0..self.def.continents.len() {
            let c = c as ContinentId;
            let list = self
                .squads
                .all()
                .iter()
                .filter(|s| self.players.get(&s.leader()).and_then(|p| p.continent) == Some(c))
                .map(|s| ps_sim::bots::HumanSquad {
                    leader: s.leader(),
                    bots: s
                        .members
                        .iter()
                        .copied()
                        .filter(|m| !self.players.contains_key(m))
                        .collect(),
                    waypoint: s.waypoint.filter(|w| w.0 == c).map(|w| w.1),
                })
                .filter(|s| !s.bots.is_empty())
                .collect();
            self.to_continent(c, ToContinent::BotSquads(list));
        }
    }

    /// Bots per faction on each continent: only contested ones, making room for humans.
    fn quotas(&mut self) {
        self.squad_bots();
        for ci in 0..self.def.continents.len() {
            let c = ci as ContinentId;
            let mut q = [0usize; 3];
            if self.cfg.bots > 0 && self.state.contested(&self.def, c) {
                let factions = self.state.factions_on(&self.def, c);
                let per = self.cfg.bots / factions.len().max(1);
                for t in factions {
                    let humans = self
                        .players
                        .values()
                        .filter(|p| p.team == t && p.continent == Some(c))
                        .count();
                    q[t.index()] = per.saturating_sub(humans);
                }
            }
            self.to_continent(c, ToContinent::BotQuota(q));
        }
    }

    fn roster(&mut self) {
        let list = self
            .players
            .iter()
            .map(|(id, p)| RosterEntry {
                id: *id,
                name: p.name.clone(),
                team: p.team,
                continent: p.continent,
                br: ps_sim::progress::battle_rank(p.xp),
            })
            .collect();
        self.broadcast(&ToClient::Roster(list));
    }

    fn report(&mut self) {
        if self.reporter.is_none() && self.lan.is_none() {
            return;
        }
        let mut held = [0u32; 3];
        for c in &self.state.continents {
            if let Some(t) = c.owner {
                held[t.index()] += 1;
            }
        }
        let bots: u16 = self
            .population
            .iter()
            .map(|p| p.iter().sum::<u16>())
            .sum::<u16>()
            .saturating_sub(self.players.len() as u16);
        let port = self
            .mapper
            .as_ref()
            .and_then(|m| m.external_port())
            .unwrap_or(self.port);
        let report = ServerReport {
            protocol: PROTOCOL,
            game_version: env!("CARGO_PKG_VERSION").into(),
            name: self.cfg.name.clone(),
            port,
            dedicated: self.cfg.dedicated,
            map_seed: self.def.params.seed,
            mode: "conquest".into(),
            arena: format!("{} continents", self.def.continents.len()),
            players: self
                .players
                .values()
                .map(|p| ReportPlayer {
                    name: p.name.clone(),
                    team: p.team.name().to_lowercase(),
                })
                .collect(),
            max_players: self.cfg.max_players as u16,
            bots,
            bots_enabled: self.cfg.bots > 0,
            scores: held,
            time_left_s: 0,
        };
        if let Some(l) = &self.lan {
            l.update(report.clone());
        }
        if let Some(r) = &self.reporter {
            r.update(report);
            for line in r.take_log() {
                self.log(&format!("master: {line}"));
            }
        }
    }
}
