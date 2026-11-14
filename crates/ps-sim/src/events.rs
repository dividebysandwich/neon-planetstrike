//! What happens on a continent that clients are told reliably (deaths, hits taken and dealt,
//! spawns, puzzles), and what players ask of it (spawning, loadouts, vehicles, consoles).

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::types::{PlayerId, Team, VehicleId};
use crate::vehicle::VehicleKind;
use crate::weapons::{Ammo, Loadout};
use crate::worldgen::{BaseIdx, ContinentId};

/// What killed or hurt someone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cause {
    Weapon(Ammo),
    Fall,
    Roadkill(VehicleKind),
    /// Inside a vehicle that was destroyed.
    Wreck(VehicleKind),
    Turret,
    Suicide,
    /// An engineer's deployable (a spitfire turret, a mine).
    Deployable(crate::weapons::Kit),
}

impl Cause {
    /// What it's called (the kill feed's, the stats' key).
    pub fn name(&self) -> String {
        use crate::weapons::Ammo;
        match self {
            Cause::Weapon(a) => match a {
                Ammo::Gun(w) => w.stats().name.to_string(),
                Ammo::Grenade => "Grenade".into(),
                Ammo::LightShell => "Light Tank".into(),
                Ammo::HeavyShell => "Heavy Tank".into(),
                Ammo::Flak => "Flak".into(),
                Ammo::Rocket => "Rockets".into(),
                Ammo::TurretBolt => "Turret".into(),
                Ammo::Chaingun => "Chaingun".into(),
                Ammo::AirCannon => "Air Cannon".into(),
                Ammo::Missile => "Missile".into(),
                Ammo::Spitfire => "Spitfire".into(),
                Ammo::Mine => "AT Mine".into(),
                Ammo::Utility(k) => format!("{} Grenade", k.name()),
                Ammo::Autocannon => "Autocannon".into(),
                Ammo::Rail => "Railgun".into(),
            },
            Cause::Fall => "Fall".into(),
            Cause::Roadkill(k) => format!("{} (roadkill)", k.name()),
            Cause::Wreck(k) => format!("{} wreck", k.name()),
            Cause::Turret => "Turret".into(),
            Cause::Suicide => "Suicide".into(),
            Cause::Deployable(k) => k.name().into(),
        }
    }
}

/// Where a player asks to spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnPoint {
    Base {
        continent: ContinentId,
        base: BaseIdx,
    },
    /// A deployed spawn bus on the player's continent.
    Bus(VehicleId),
    /// A forward spawn tube (a structure) of the player's faction on its continent.
    Tube(u32),
    /// Beside the player's squad leader (or in a free seat of its vehicle), on whatever
    /// continent it is (the server checks the squad).
    Squad(PlayerId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Deny {
    TooFar,
    NotYours,
    PadBusy,
    Cooldown(u16),
    NotLinked,
    NoHack,
    NotDead,
    NotReady,
    Full,
    TooMany,
    Sanctuary,
    /// The base's generator is down.
    Unpowered,
    /// The base's silo hasn't enough resources.
    NoResources,
    /// Not in that player's squad, or it doesn't lead one.
    NotInSquad,
    /// The squad leader is dead, manning a turret or flying with no free seat.
    LeaderUnavailable,
    /// A MAX can't (drive, man a gun, work a console, hack).
    Max,
    /// Only an infiltrator hacks.
    NotInfiltrator,
    /// Already the player's faction's.
    Already,
    /// No room to put a deployable down here.
    NoRoom,
    /// Too many of the player's faction on that continent already (by humans).
    Overpopulated,
    /// Needs a facility its faction holds on the continent.
    NeedsFacility(crate::worldgen::Facility),
    /// Not near enough a base of its faction's to build for it.
    NoBase,
    /// The base has as many structures (or a tube) as it may.
    TooManyStructures,
    /// Only engineers (the repair gun) build.
    NotEngineer,
    /// Already carrying a structure.
    Carrying,
    /// Not carrying that structure.
    NotCarrying,
    /// No building in (or by) a base.
    NoBuildZone,
}

impl Deny {
    pub fn text(&self) -> String {
        match self {
            Deny::TooFar => "Too far away".into(),
            Deny::NotYours => "Your faction doesn't hold this".into(),
            Deny::PadBusy => "The pad is occupied".into(),
            Deny::Cooldown(s) => format!("Available in {s} s"),
            Deny::NotLinked => "Not linked to your lattice".into(),
            Deny::NoHack => "No hack to stop".into(),
            Deny::NotDead => "You are alive".into(),
            Deny::NotReady => "Not yet".into(),
            Deny::Full => "No free seat".into(),
            Deny::TooMany => "Too many vehicles here".into(),
            Deny::Sanctuary => "Can't warp into an enemy sanctuary".into(),
            Deny::Unpowered => "The base's generator is down".into(),
            Deny::NoResources => "Not enough resources in the base's silo".into(),
            Deny::NotInSquad => "Not your squad's leader".into(),
            Deny::LeaderUnavailable => "Can't spawn on the squad leader now".into(),
            Deny::Max => "A MAX can't do that".into(),
            Deny::NotInfiltrator => "Only an infiltrator can hack that".into(),
            Deny::Already => "Your faction already has that".into(),
            Deny::NoRoom => "No room to put it down here".into(),
            Deny::Overpopulated => {
                "Your faction is overpopulated there: try another continent".into()
            }
            Deny::NeedsFacility(f) => format!("Needs a {} held by your faction here", f.name()),
            Deny::NoBase => "Too far from a base of your faction's".into(),
            Deny::TooManyStructures => "That base has all the structures it may".into(),
            Deny::NotEngineer => "Only engineers (the repair gun) build".into(),
            Deny::Carrying => "You're carrying a structure already: put it down first".into(),
            Deny::NotCarrying => "Pull a structure at a harvester's terminal first".into(),
            Deny::NoBuildZone => "No building in a base".into(),
        }
    }
}

/// A player's requests (reliable).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Request {
    Spawn(SpawnPoint),
    /// At an equipment terminal: take this loadout (refilling ammo and armor).
    Loadout(Loadout),
    /// At a medical terminal: heal.
    Heal,
    EnterVehicle(VehicleId),
    /// Man a turret (its index on the continent).
    EnterTurret(u16),
    ExitVehicle,
    SpawnVehicle {
        base: BaseIdx,
        terminal: u8,
        kind: VehicleKind,
        /// Its main gun's choice and its defensive slot.
        loadout: crate::vehicle::VehicleLoadout,
    },
    /// The base's console: show its puzzle, or poke a piece of it.
    ViewConsole(BaseIdx),
    Poke {
        base: BaseIdx,
        piece: u8,
    },
    Suicide,
    /// An infiltrator starts hacking an enemy terminal or turret (standing at it for
    /// `weapons::HACK_TIME`).
    Hack(HackTarget),
    /// Spot the enemy one aims at for one's faction.
    Spot,
    /// At its faction's harvester `harvester`: take structure `kind` from its construction
    /// terminal (its load paying), to carry and put down.
    Pull {
        harvester: VehicleId,
        kind: crate::construct::StructureKind,
    },
    /// Put the structure carried (`kind`) down at `at` (on the ground there) turned by `yaw`.
    Build {
        kind: crate::construct::StructureKind,
        at: Vec3,
        yaw: f32,
    },
}

/// What a player spotted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpotTarget {
    Player(PlayerId),
    Vehicle(VehicleId),
}

/// What an infiltrator hacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HackTarget {
    Terminal {
        base: BaseIdx,
        index: u8,
    },
    /// A base turret (its index on the continent).
    Turret(u16),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SimEvent {
    Killed {
        victim: PlayerId,
        by: PlayerId,
        cause: Cause,
        headshot: bool,
    },
    /// The victim was hurt (`from`: where it came from).
    Hurt {
        victim: PlayerId,
        by: PlayerId,
        amount: f32,
        from: Vec3,
    },
    /// The shooter hit something.
    HitConfirm {
        by: PlayerId,
        vehicle: bool,
        headshot: bool,
        kill: bool,
    },
    Spawned {
        player: PlayerId,
        pos: Vec3,
        yaw: f32,
    },
    Healed {
        player: PlayerId,
    },
    /// XP for the player this goes to.
    Xp {
        reason: crate::progress::Award,
        xp: u32,
    },
    /// To the one killed: who did it, with what, from how far, and how the killer stood
    /// (its health and armour; where it was) - the killcam's.
    DeathInfo {
        by: PlayerId,
        cause: Cause,
        distance: f32,
        killer_at: Option<Vec3>,
        killer_health: f32,
        killer_armor: f32,
    },
    /// A medic (`by`) brought a downed player back where it fell.
    Revived {
        player: PlayerId,
        by: PlayerId,
    },
    /// An infiltrator (`by`, of `team`) hacked an enemy terminal or turret: it works for `team`
    /// for `weapons::HACKED_FOR`.
    Hacked {
        by: PlayerId,
        team: Team,
        what: HackTarget,
    },
    /// A player spotted an enemy for its faction.
    Spotted {
        by: PlayerId,
        what: SpotTarget,
    },
    /// A structure was built (`by`, of `team`).
    Built {
        by: PlayerId,
        team: Team,
        kind: crate::construct::StructureKind,
        at: Vec3,
    },
    /// The player was blinded by a flash grenade for `secs`.
    Flashed {
        player: PlayerId,
        secs: f32,
    },
    /// A mobile silo of the faction's is running low, or ran dry (its structures decay).
    SiloLow {
        at: Vec3,
        empty: bool,
    },
    /// A structure was destroyed.
    StructureDown {
        kind: crate::construct::StructureKind,
        team: Team,
        at: Vec3,
    },
    /// The player's deployable went down (`kind` at `at`).
    Deployed {
        by: PlayerId,
        kind: crate::weapons::Kit,
        at: Vec3,
    },
    Denied(Deny),
    VehicleSpawned {
        id: VehicleId,
        kind: VehicleKind,
        by: PlayerId,
    },
    VehicleDestroyed {
        id: VehicleId,
        kind: VehicleKind,
        by: PlayerId,
    },
    /// A console's puzzle: its seed and the pieces' positions (sent to those using it).
    Puzzle {
        base: BaseIdx,
        seed: u32,
        at: Vec<u8>,
    },
    /// A console was solved: by whom and their faction.
    ConsoleSolved {
        base: BaseIdx,
        by: PlayerId,
        team: Team,
    },
    LoadoutTaken {
        player: PlayerId,
    },
    /// A base's generator went down, or came back up.
    Generator {
        base: BaseIdx,
        up: bool,
    },
}

/// Who an event goes to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Audience {
    All,
    Player(PlayerId),
    Team(Team),
    Near(Vec3, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outgoing {
    pub to: Audience,
    pub ev: SimEvent,
}
