//! The client side: a connection to a server, sending this player's commands and receiving
//! the world.

use web_time::Instant;

use ps_sim::state::{Change, WorldState};
use ps_sim::{ContinentId, PlayerCmd, PlayerId, Request, SimEvent, Snapshot, Team, WorldParams};

use crate::protocol::{
    PROTOCOL, RECENT, RosterEntry, ScoreRow, SquadCmd, SquadInfo, ToClient, ToServer, decode,
    encode,
};
use crate::transport::{Channel, Endpoint, NetEvent};

#[derive(Clone, Debug)]
pub enum ClientEvent {
    Welcome {
        id: PlayerId,
        params: WorldParams,
        team: Team,
        server: String,
    },
    World(WorldState),
    Changes(Vec<Change>),
    Enter(ContinentId),
    Snapshot(ContinentId, Box<Snapshot>),
    Events(Vec<SimEvent>),
    Roster(Vec<RosterEntry>),
    Population {
        all: Vec<[u16; 3]>,
        humans: Vec<[u16; 3]>,
    },
    Alert(Option<crate::protocol::AlertInfo>),
    /// The server wants a login (`Client::send_login`).
    LoginRequired(String),
    LoginFailed(String),
    Stats(Vec<(String, i64)>),
    AlertEnded {
        continent: ps_sim::ContinentId,
        winner: Option<ps_sim::Team>,
    },
    /// Who, what, and whether to the squad only.
    Chat(PlayerId, String, bool),
    Squads(Vec<SquadInfo>),
    Progress(u64),
    Scores(Vec<ScoreRow>),
    Clock(f32),
    Disconnected(String),
}

pub struct Client {
    endpoint: Endpoint,
    name: String,
    team: Option<Team>,
    seq: u32,
    recent: Vec<(u32, PlayerCmd)>,
    last_tick: u64,
    deltas: crate::delta::DeltaReceiver,
    pub connected: bool,
    /// Snapshot messages received and their bytes (as sent, compressed).
    pub snapshot_count: u64,
    pub snapshot_bytes: u64,
}

impl Client {
    /// Starts connecting to `addr` ("host:port"; the default port if none is given).
    pub fn connect(addr: &str, name: &str, team: Option<Team>) -> std::io::Result<Client> {
        let addr = if addr.contains(':') {
            addr.to_string()
        } else {
            format!("{addr}:{}", crate::protocol::DEFAULT_PORT)
        };
        Ok(Client::over(Endpoint::client(&addr)?, name, team))
    }

    /// Starts connecting through the master server's relay at `url` (a WebSocket URL,
    /// [`crate::master::relay_url`]): the browser build's way to a game server.
    #[cfg(target_arch = "wasm32")]
    pub fn relayed(url: &str, name: &str, team: Option<Team>) -> std::io::Result<Client> {
        Ok(Client::over(Endpoint::relay(url)?, name, team))
    }

    /// Starts connecting over an endpoint made elsewhere ([`Endpoint::client_over`]).
    pub fn over(endpoint: Endpoint, name: &str, team: Option<Team>) -> Client {
        Client {
            endpoint,
            name: name.to_string(),
            team,
            seq: 0,
            recent: Vec::new(),
            last_tick: 0,
            deltas: Default::default(),
            connected: false,
            snapshot_count: 0,
            snapshot_bytes: 0,
        }
    }

    pub fn rtt(&self) -> f32 {
        self.endpoint.rtt(0)
    }

    /// This tick's command.
    pub fn send_input(&mut self, cmd: PlayerCmd) {
        if !self.connected {
            return;
        }
        self.seq += 1;
        self.recent.push((self.seq, cmd));
        if self.recent.len() > RECENT {
            self.recent.remove(0);
        }
        let msg = ToServer::Input {
            recent: self.recent.clone(),
            ack: self.deltas.newest(),
        };
        self.endpoint.send(0, Channel::Unreliable, &encode(&msg));
        self.endpoint.flush(Instant::now());
    }

    /// A request (reliable).
    pub fn send_request(&mut self, r: Request) {
        if !self.connected {
            return;
        }
        self.endpoint
            .send(0, Channel::Reliable, &encode(&ToServer::Request(r)));
        self.endpoint.flush(Instant::now());
    }

    pub fn send_chat(&mut self, text: &str) {
        self.chat(text, false);
    }

    /// Says something to one's squad only.
    pub fn send_squad_chat(&mut self, text: &str) {
        self.chat(text, true);
    }

    /// The name the game was joined under.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Logs in (or registers) as `name` with `password` (only its secret goes).
    pub fn send_login(&mut self, name: &str, password: &str, register: bool) {
        if !self.connected {
            return;
        }
        let msg = ToServer::Login {
            name: name.trim().to_string(),
            secret: crate::protocol::login_secret(name, password),
            register,
        };
        self.endpoint.send(0, Channel::Reliable, &encode(&msg));
        self.endpoint.flush(Instant::now());
    }

    /// Asks for one's stats (`ClientEvent::Stats`).
    pub fn ask_stats(&mut self) {
        if !self.connected {
            return;
        }
        self.endpoint
            .send(0, Channel::Reliable, &encode(&ToServer::Stats));
        self.endpoint.flush(Instant::now());
    }

    /// Says something to one's faction only.
    pub fn send_team_chat(&mut self, text: &str) {
        if !self.connected || text.trim().is_empty() {
            return;
        }
        let text: String = text.chars().take(160).collect();
        self.endpoint
            .send(0, Channel::Reliable, &encode(&ToServer::TeamChat(text)));
        self.endpoint.flush(Instant::now());
    }

    fn chat(&mut self, text: &str, squad: bool) {
        if !self.connected || text.trim().is_empty() {
            return;
        }
        let text: String = text.chars().take(160).collect();
        let msg = if squad {
            ToServer::SquadChat(text)
        } else {
            ToServer::Chat(text)
        };
        self.endpoint.send(0, Channel::Reliable, &encode(&msg));
        self.endpoint.flush(Instant::now());
    }

    /// Squads: create, join, leave, the waypoint, the lead (reliable).
    pub fn send_squad(&mut self, cmd: SquadCmd) {
        if !self.connected {
            return;
        }
        self.endpoint
            .send(0, Channel::Reliable, &encode(&ToServer::Squad(cmd)));
        self.endpoint.flush(Instant::now());
    }

    pub fn poll(&mut self) -> Vec<ClientEvent> {
        let now = Instant::now();
        let mut out = Vec::new();
        for ev in self.endpoint.poll(now) {
            match ev {
                NetEvent::Connected(_) => {
                    self.connected = true;
                    let hello = ToServer::Hello {
                        protocol: PROTOCOL,
                        name: self.name.clone(),
                        team: self.team,
                    };
                    self.endpoint.send(0, Channel::Reliable, &encode(&hello));
                }
                NetEvent::Disconnected(_, why) => {
                    self.connected = false;
                    out.push(ClientEvent::Disconnected(why));
                }
                NetEvent::Message(_, _, bytes) => match decode::<ToClient>(&bytes) {
                    Some(ToClient::Snapshot { continent, snap }) => {
                        self.snapshot_count += 1;
                        self.snapshot_bytes += bytes.len() as u64;
                        // Late arrivals are dropped.
                        if snap.tick > self.last_tick
                            && let Some(snap) = self.deltas.apply(*snap)
                        {
                            self.last_tick = snap.tick;
                            out.push(ClientEvent::Snapshot(continent, Box::new(snap)));
                        }
                    }
                    Some(ToClient::Welcome {
                        id,
                        params,
                        team,
                        server,
                    }) => out.push(ClientEvent::Welcome {
                        id,
                        params,
                        team,
                        server,
                    }),
                    Some(ToClient::World(w)) => out.push(ClientEvent::World(w)),
                    Some(ToClient::Changes(c)) => out.push(ClientEvent::Changes(c)),
                    Some(ToClient::Enter { continent }) => {
                        // A new continent: its ticks are its own.
                        self.last_tick = 0;
                        self.deltas = Default::default();
                        out.push(ClientEvent::Enter(continent));
                    }
                    Some(ToClient::Events(e)) => out.push(ClientEvent::Events(e)),
                    Some(ToClient::Roster(r)) => out.push(ClientEvent::Roster(r)),
                    Some(ToClient::Population { all, humans }) => {
                        out.push(ClientEvent::Population { all, humans })
                    }
                    Some(ToClient::Alert(a)) => out.push(ClientEvent::Alert(a)),
                    Some(ToClient::LoginRequired { server }) => {
                        out.push(ClientEvent::LoginRequired(server))
                    }
                    Some(ToClient::LoginFailed(why)) => out.push(ClientEvent::LoginFailed(why)),
                    Some(ToClient::Stats(s)) => out.push(ClientEvent::Stats(s)),
                    Some(ToClient::AlertEnded { continent, winner }) => {
                        out.push(ClientEvent::AlertEnded { continent, winner })
                    }
                    Some(ToClient::Chat { from, text, squad }) => {
                        out.push(ClientEvent::Chat(from, text, squad));
                    }
                    Some(ToClient::Squads(s)) => out.push(ClientEvent::Squads(s)),
                    Some(ToClient::Progress(x)) => out.push(ClientEvent::Progress(x)),
                    Some(ToClient::Scores(s)) => out.push(ClientEvent::Scores(s)),
                    Some(ToClient::Clock(t)) => out.push(ClientEvent::Clock(t)),
                    Some(ToClient::Reject(why)) | Some(ToClient::Shutdown(why)) => {
                        out.push(ClientEvent::Disconnected(why));
                    }
                    None => {}
                },
            }
        }
        self.endpoint.flush(now);
        out
    }

    /// Leaves: says so, then closes the connection (both go out now: there may be no later
    /// flush).
    pub fn close(&mut self) {
        self.endpoint
            .send(0, Channel::Reliable, &encode(&ToServer::Bye));
        self.endpoint.flush(Instant::now());
        self.endpoint.close(0, "Bye");
        self.endpoint.flush(Instant::now());
        self.connected = false;
    }
}
