//! Neon Planetstrike without a renderer: the world (continents, warp gates, base lattice) and
//! its state, continent terrain and scenery, bases, infantry, vehicles, projectiles and bots.
//! Run by the dedicated server (one `ContinentSim` per continent, each on its own thread) and,
//! for prediction and drawing, by the client.

pub mod avatar;
pub mod base;
pub mod bots;
pub mod capture;
pub mod caves;
pub mod collide;
pub mod construct;
pub mod events;
pub mod map;
pub mod nav;
pub mod noise;
pub mod player;
pub mod progress;
pub mod puzzle;
pub mod roads;
pub mod scenery;
pub mod sim;
pub mod snapshot;
pub mod state;
pub mod terrain;
pub mod theme;
pub mod types;
pub mod vehicle;
pub mod weapons;
pub mod worldgen;
pub mod zip;

pub use events::{
    Audience, Cause, Deny, HackTarget, Outgoing, Request, SimEvent, SpawnPoint, SpotTarget,
};
pub use sim::{ContinentSim, TICK};
pub use snapshot::Snapshot;
pub use state::WorldState;
pub use types::*;
pub use worldgen::{BaseIdx, ContinentId, WorldDef, WorldParams};
