//! The game's messages: postcard encoded (compact, no field names), LZ4 compressed when large.

use crate::delta::SnapDelta;
use glam::Vec3;
use ps_sim::state::{Change, WorldState};
use ps_sim::{ContinentId, PlayerCmd, PlayerId, Request, SimEvent, Team, WorldParams};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const PROTOCOL: u16 = 12;
/// Seconds a whole day takes.
pub const DAY_LENGTH: f32 = 1800.0;
pub const DEFAULT_PORT: u16 = 27430;
/// How many of its latest commands a client sends with each input (a lost packet loses
/// nothing).
pub const RECENT: usize = 4;
/// Players in a squad at most.
pub const SQUAD_MAX: usize = 10;

pub type SquadId = u16;

/// What a player does about squads (the server keeps them, by faction, across continents).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SquadCmd {
    /// Start a squad (leaving the one it's in) and lead it.
    Create,
    Join(SquadId),
    Leave,
    /// The leader: put the squad's waypoint here (none: take it away).
    Waypoint(Option<(ContinentId, Vec3)>),
    /// The leader: hand the lead to a member.
    Promote(PlayerId),
    /// The leader: put a member out of the squad.
    Kick(PlayerId),
}

/// A squad as its faction is told of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SquadInfo {
    pub id: SquadId,
    pub leader: PlayerId,
    /// The leader first.
    pub members: Vec<PlayerId>,
    pub waypoint: Option<(ContinentId, Vec3)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToServer {
    Hello {
        protocol: u16,
        name: String,
        /// The faction asked for (None: the smallest).
        team: Option<Team>,
    },
    /// The client's latest commands, oldest first, by sequence number (one per tick).
    Input {
        recent: Vec<(u32, PlayerCmd)>,
        /// The newest snapshot the client has (the baseline for the next deltas).
        ack: u64,
    },
    Request(Request),
    Chat(String),
    /// Said to one's squad only.
    SquadChat(String),
    /// Said to one's faction only (VR's callouts).
    TeamChat(String),
    Squad(SquadCmd),
    /// Logging in to (or registering on) a server with accounts: the account's name and the
    /// secret from its password (`login_secret`; the password itself never goes).
    Login {
        name: String,
        secret: String,
        register: bool,
    },
    /// One's stats (answered with `ToClient::Stats`).
    Stats,
    Bye,
}

/// A player as the roster lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RosterEntry {
    pub id: PlayerId,
    pub name: String,
    pub team: Team,
    pub continent: Option<ContinentId>,
    /// Battle Rank.
    pub br: u8,
}

/// A line of the scoreboard: a player (or bot) on the continent and how it's doing this
/// session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreRow {
    pub id: PlayerId,
    pub team: Team,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    pub score: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToClient {
    Welcome {
        id: PlayerId,
        params: WorldParams,
        team: Team,
        server: String,
    },
    Reject(String),
    /// The server keeps accounts: log in or register (`ToServer::Login`) before playing.
    LoginRequired {
        server: String,
    },
    /// The login or registration didn't go through: why.
    LoginFailed(String),
    /// One's stats as the server keeps them (counters by key; none without an account).
    Stats(Vec<(String, i64)>),
    /// Who holds what (reliable; again every so often).
    World(WorldState),
    /// Ownership changes (reliable).
    Changes(Vec<Change>),
    /// The player is now on continent `continent` (load it; snapshots from it follow).
    Enter {
        continent: ContinentId,
    },
    /// What the player sees of its continent (unreliable), as a delta (`crate::delta`).
    Snapshot {
        continent: ContinentId,
        snap: Box<SnapDelta>,
    },
    /// What happened (reliable).
    Events(Vec<SimEvent>),
    /// The human players (bots go by `ps_sim::bots::bot_name`).
    Roster(Vec<RosterEntry>),
    /// Players and bots on each continent by faction, and the human players alone.
    Population {
        all: Vec<[u16; 3]>,
        humans: Vec<[u16; 3]>,
    },
    /// The alert under way (reliable; again every so often, and none when it ends).
    Alert(Option<AlertInfo>),
    /// An alert ended: its continent, and the faction that won it (none: a draw).
    AlertEnded {
        continent: ContinentId,
        winner: Option<Team>,
    },
    Chat {
        from: PlayerId,
        text: String,
        /// Said to the squad only.
        squad: bool,
    },
    /// The faction's squads (reliable; again whenever one changes).
    Squads(Vec<SquadInfo>),
    /// The player's XP in all (on joining, and as it grows).
    Progress(u64),
    /// Everyone on the player's continent (every few seconds).
    Scores(Vec<ScoreRow>),
    /// The time of day (0..1: 0 midnight, 0.5 noon).
    Clock(f32),
    Shutdown(String),
}

/// An alert: a faction-wide objective for a while. Whoever holds the most of the continent's
/// bases (outposts and its center) when the time is up wins it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertInfo {
    pub continent: ContinentId,
    /// Seconds left.
    pub left: u32,
}

/// The secret a login sends for `password` (with the account's `name`, any case): the password
/// never leaves the game.
pub fn login_secret(name: &str, password: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"neon-planetstrike login\0");
    h.update(name.trim().to_lowercase().as_bytes());
    h.update(b"\0");
    h.update(password.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The fewest humans of a faction on a continent before it may count as overpopulated.
pub const OVERPOP_MIN: u16 = 4;

/// Whether `team` is overpopulated on a continent with `humans` per faction, against the other
/// factions with a stake there (`rivals`): it has at least `OVERPOP_MIN`, and half again as many
/// plus two as the smallest of them.
pub fn overpopulated(humans: [u16; 3], team: Team, rivals: &[Team]) -> bool {
    let mine = humans[team.index()];
    let least = rivals
        .iter()
        .filter(|t| **t != team)
        .map(|t| humans[t.index()])
        .min();
    match least {
        Some(least) => mine >= OVERPOP_MIN && mine as f32 >= least as f32 * 1.5 + 2.0,
        None => false,
    }
}

/// A message's bytes: a flag (1 compressed) and postcard.
pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    let Ok(raw) = postcard::to_allocvec(msg) else {
        return Vec::new();
    };
    if raw.len() > 200 {
        let mut out = vec![1];
        out.extend(lz4_flex::compress_prepend_size(&raw));
        out
    } else {
        let mut out = Vec::with_capacity(raw.len() + 1);
        out.push(0);
        out.extend(raw);
        out
    }
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    let (&flag, body) = bytes.split_first()?;
    match flag {
        0 => postcard::from_bytes(body).ok(),
        1 => {
            let raw = lz4_flex::decompress_size_prepended(body).ok()?;
            postcard::from_bytes(&raw).ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec3};
    use ps_sim::snapshot::{PlayerSnap, flags};
    use ps_sim::weapons::Weapon;

    #[test]
    fn overpopulation_wants_numbers_and_a_lead() {
        let both = [Team::Red, Team::Blue];
        // Few: never.
        assert!(!overpopulated([3, 0, 0], Team::Red, &both));
        // 8 against 4: 8 >= 4 * 1.5 + 2.
        assert!(overpopulated([8, 4, 0], Team::Red, &both));
        assert!(!overpopulated([7, 4, 0], Team::Red, &both));
        // The smaller side never is; nor anyone alone with no rivals.
        assert!(!overpopulated([8, 4, 0], Team::Blue, &both));
        assert!(!overpopulated([9, 0, 0], Team::Red, &[Team::Red]));
    }

    #[test]
    fn round_trip_and_size() {
        let players: Vec<PlayerSnap> = (0..40)
            .map(|i| PlayerSnap {
                id: i,
                team: Team::ALL[i as usize % 3],
                feet: Vec3::new(i as f32 * 13.7 - 200.0, 30.25, 400.0 - i as f32 * 3.1),
                yaw: 1.0,
                pitch: -0.2,
                flags: flags::SPRINT,
                weapon: Weapon::Rifle,
                health: 87.0,
                seat: None,
                vr: None,
            })
            .collect();
        let msg = ToClient::Snapshot {
            continent: 4,
            snap: Box::new(SnapDelta {
                tick: 123_456,
                players: crate::delta::ListDelta {
                    changed: players,
                    removed: Vec::new(),
                },
                ..Default::default()
            }),
        };
        let bytes = encode(&msg);
        // 40 players in well under 1 KB a snapshot.
        assert!(bytes.len() < 900, "{} bytes", bytes.len());
        let back: ToClient = decode(&bytes).unwrap();
        let ToClient::Snapshot { snap, .. } = back else {
            panic!()
        };
        assert_eq!(snap.players.changed.len(), 40);
        assert!(
            (snap.players.changed[3].feet - Vec3::new(3.0 * 13.7 - 200.0, 30.25, 400.0 - 9.3))
                .length()
                < 0.01
        );
        let input = ToServer::Input {
            recent: (0..RECENT as u32)
                .map(|i| {
                    (
                        i,
                        PlayerCmd {
                            stick: Vec2::new(0.0, -1.0),
                            yaw: 1.0,
                            feet: Some(Vec3::new(10.0, 2.0, 3.0)),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
            ack: 123_456,
        };
        let bytes = encode(&input);
        assert!(bytes.len() < 400, "{} bytes", bytes.len());
        assert_eq!(decode::<ToServer>(&bytes), Some(input));
    }
}
