//! Infantry weapons, armor and loadouts. Every number about a weapon lives in its `Stats`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weapon {
    /// Automatic pulse rifle (the lasertag blaster grown up).
    Rifle,
    /// Close range: a spread of pellets.
    Scatter,
    /// Marksman rifle: one fast, hard-hitting bolt, a scope.
    Lance,
    /// Anti-vehicle rockets, a little splash.
    Striker,
    /// Semi-automatic pistol (everyone's sidearm).
    Pistol,
    /// The heal gun: a beam that heals team mates.
    Medgun,
    /// The repair gun: a beam that repairs vehicles, turrets and generators.
    RepairGun,
    /// Anti-air missiles: held on an aircraft, it locks on; its missile homes on it.
    Skyhook,
    /// The red faction's rifle: heavy hitting, slower.
    Ember,
    /// The blue faction's rifle: fast, accurate bolts.
    Volt,
    /// The green faction's rifle: a big magazine, quick fire.
    Hive,
    /// A MAX's arms against infantry: twin chainguns.
    Pounder,
    /// A MAX's arms against armour: twin cannons, a little splash.
    Mauler,
    /// A MAX's arms against aircraft: flak bursting near them.
    Burster,
}

/// What a weapon's shot is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotKind {
    /// Projectiles (`pellets` of them) flying at `speed`.
    Bolt,
    /// A beam reaching `range` (no projectile).
    Beam,
}

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub name: &'static str,
    pub shot: ShotKind,
    /// Damage per pellet (health points at full effect; healing for the medgun, per second).
    pub damage: f32,
    /// Damage against vehicles and turrets, per pellet (vehicle health percent).
    pub vs_vehicle: f32,
    pub pellets: u8,
    /// Spread (radians, cone half-angle) per pellet when hip firing standing still
    /// (`spread_for` widens it on the move and narrows it aiming down the sights).
    pub spread: f32,
    pub speed: f32,
    /// Seconds between shots.
    pub interval: f32,
    pub automatic: bool,
    pub mag: u16,
    /// Rounds carried besides the magazine (filled at terminals).
    pub reserve: u16,
    pub reload: f32,
    /// Falloff: full damage up to `near`, `far_factor` of it at `far` and beyond.
    pub near: f32,
    pub far: f32,
    pub far_factor: f32,
    /// Seconds a projectile lives.
    pub life: f32,
    /// Splash radius (0: none) and the damage at its middle.
    pub splash: f32,
    pub splash_damage: f32,
    pub headshot: f32,
    /// Projectiles fall (m/s²).
    pub gravity: f32,
}

const BASE: Stats = Stats {
    name: "",
    shot: ShotKind::Bolt,
    damage: 10.0,
    vs_vehicle: 0.2,
    pellets: 1,
    spread: 0.012,
    speed: 220.0,
    interval: 0.1,
    automatic: true,
    mag: 30,
    reserve: 180,
    reload: 2.0,
    near: 40.0,
    far: 140.0,
    far_factor: 0.6,
    life: 2.0,
    splash: 0.0,
    splash_damage: 0.0,
    headshot: 2.0,
    gravity: 0.0,
};

impl Weapon {
    pub const ALL: [Weapon; 14] = [
        Weapon::Rifle,
        Weapon::Scatter,
        Weapon::Lance,
        Weapon::Ember,
        Weapon::Volt,
        Weapon::Hive,
        Weapon::Striker,
        Weapon::Pistol,
        Weapon::Medgun,
        Weapon::RepairGun,
        Weapon::Skyhook,
        Weapon::Pounder,
        Weapon::Mauler,
        Weapon::Burster,
    ];
    /// Each faction's own rifle (only its players carry it).
    pub fn faction_rifle(team: crate::Team) -> Weapon {
        match team {
            crate::Team::Red => Weapon::Ember,
            crate::Team::Blue => Weapon::Volt,
            crate::Team::Green => Weapon::Hive,
        }
    }

    /// The rifles a faction's players may carry: the common ones and its own.
    pub fn rifles_for(team: crate::Team) -> [Weapon; 4] {
        [
            Weapon::Rifle,
            Weapon::Scatter,
            Weapon::Lance,
            Weapon::faction_rifle(team),
        ]
    }

    /// A MAX's arms (its only weapon).
    pub const MAX_ARMS: [Weapon; 3] = [Weapon::Pounder, Weapon::Mauler, Weapon::Burster];
    /// What may go in the rifle, sidearm and special slots.
    pub const RIFLES: [Weapon; 3] = [Weapon::Rifle, Weapon::Scatter, Weapon::Lance];
    pub const SIDEARMS: [Weapon; 1] = [Weapon::Pistol];
    pub const SPECIALS: [Weapon; 4] = [
        Weapon::Striker,
        Weapon::Skyhook,
        Weapon::Medgun,
        Weapon::RepairGun,
    ];

    pub fn stats(self) -> Stats {
        match self {
            Weapon::Rifle => Stats {
                name: "Pulse Rifle",
                damage: 12.0,
                interval: 0.11,
                ..BASE
            },
            Weapon::Scatter => Stats {
                name: "Scatter Gun",
                damage: 11.0,
                pellets: 9,
                spread: 0.07,
                speed: 160.0,
                interval: 0.75,
                automatic: false,
                mag: 6,
                reserve: 36,
                reload: 2.6,
                near: 8.0,
                far: 30.0,
                far_factor: 0.2,
                life: 0.4,
                headshot: 1.5,
                ..BASE
            },
            Weapon::Lance => Stats {
                name: "Prism Lance",
                damage: 75.0,
                vs_vehicle: 1.0,
                spread: 0.006,
                speed: 700.0,
                interval: 1.1,
                automatic: false,
                mag: 5,
                reserve: 30,
                reload: 3.0,
                near: 400.0,
                far: 800.0,
                far_factor: 0.8,
                life: 1.5,
                headshot: 2.0,
                ..BASE
            },
            Weapon::Striker => Stats {
                name: "Striker",
                damage: 60.0,
                vs_vehicle: 28.0,
                spread: 0.01,
                speed: 75.0,
                interval: 1.0,
                automatic: false,
                mag: 1,
                reserve: 9,
                reload: 2.2,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                life: 5.0,
                splash: 4.0,
                splash_damage: 45.0,
                headshot: 1.0,
                gravity: 2.0,
                ..BASE
            },
            Weapon::Skyhook => Stats {
                name: "Skyhook",
                damage: 30.0,
                vs_vehicle: 22.0,
                spread: 0.01,
                speed: 90.0,
                interval: 1.0,
                automatic: false,
                mag: 1,
                reserve: 7,
                reload: 2.6,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                life: 7.0,
                splash: 3.0,
                splash_damage: 30.0,
                headshot: 1.0,
                gravity: 0.0,
                ..BASE
            },
            Weapon::Ember => Stats {
                name: "Ember Carbine",
                damage: 16.0,
                spread: 0.014,
                speed: 200.0,
                interval: 0.14,
                mag: 24,
                reserve: 168,
                ..BASE
            },
            Weapon::Volt => Stats {
                name: "Volt Rifle",
                damage: 12.0,
                spread: 0.007,
                speed: 340.0,
                interval: 0.115,
                mag: 30,
                reserve: 180,
                far: 170.0,
                ..BASE
            },
            Weapon::Hive => Stats {
                name: "Hive Repeater",
                damage: 10.0,
                spread: 0.016,
                interval: 0.085,
                mag: 50,
                reserve: 250,
                reload: 2.6,
                ..BASE
            },
            Weapon::Pounder => Stats {
                name: "Pounder",
                damage: 13.0,
                vs_vehicle: 0.5,
                spread: 0.02,
                speed: 240.0,
                interval: 0.075,
                mag: 120,
                reserve: 600,
                reload: 3.0,
                near: 50.0,
                far: 160.0,
                far_factor: 0.6,
                headshot: 1.5,
                ..BASE
            },
            Weapon::Mauler => Stats {
                name: "Mauler",
                damage: 35.0,
                vs_vehicle: 7.0,
                spread: 0.008,
                speed: 150.0,
                interval: 0.55,
                automatic: true,
                mag: 12,
                reserve: 72,
                reload: 3.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                life: 3.0,
                splash: 3.0,
                splash_damage: 25.0,
                headshot: 1.0,
                gravity: 3.0,
                ..BASE
            },
            Weapon::Burster => Stats {
                name: "Burster",
                damage: 8.0,
                vs_vehicle: 1.6,
                spread: 0.01,
                speed: 260.0,
                interval: 0.22,
                automatic: true,
                mag: 40,
                reserve: 240,
                reload: 3.0,
                near: 300.0,
                far: 600.0,
                far_factor: 0.7,
                life: 2.2,
                splash: 4.0,
                splash_damage: 6.0,
                headshot: 1.0,
                gravity: 2.0,
                ..BASE
            },
            Weapon::Pistol => Stats {
                name: "Sidearm",
                damage: 16.0,
                spread: 0.016,
                interval: 0.22,
                automatic: false,
                mag: 12,
                reserve: 72,
                reload: 1.4,
                near: 20.0,
                far: 80.0,
                far_factor: 0.5,
                ..BASE
            },
            Weapon::RepairGun => Stats {
                name: "Repair Gun",
                shot: ShotKind::Beam,
                // Vehicle / turret / generator percent per second.
                damage: 0.0,
                vs_vehicle: 8.0,
                spread: 0.0,
                speed: 0.0,
                interval: 0.1,
                automatic: true,
                mag: 100,
                reserve: 300,
                reload: 1.5,
                near: 14.0,
                far: 14.0,
                far_factor: 1.0,
                ..BASE
            },
            Weapon::Medgun => Stats {
                name: "Medgun",
                shot: ShotKind::Beam,
                // Health per second on a team mate; vehicle percent per second.
                damage: 18.0,
                vs_vehicle: 6.0,
                spread: 0.0,
                speed: 0.0,
                interval: 0.1,
                automatic: true,
                mag: 100,
                reserve: 300,
                reload: 1.5,
                near: 18.0,
                far: 18.0,
                far_factor: 1.0,
                ..BASE
            },
        }
    }

    /// The spread (cone half-angle) of a shot: the weapon's own, a fraction of it aiming down
    /// the sights (the lance's scope most), crouching steadier, more running (`moving`: the
    /// speed over the walking speed) and much more in the air.
    pub fn spread_for(self, aiming: bool, crouch: bool, moving: f32, airborne: bool) -> f32 {
        let s = self.stats();
        // (The scatter gun's pellets spread whatever one does.)
        if self == Weapon::Scatter {
            return s.spread * if airborne { 1.3 } else { 1.0 };
        }
        let mut k = 1.0;
        if aiming {
            k *= if self == Weapon::Lance { 0.08 } else { 0.45 };
        }
        if crouch {
            k *= 0.7;
        }
        k *= 1.0 + 1.2 * moving.clamp(0.0, 1.6);
        if airborne {
            k *= 2.5;
        }
        s.spread * k
    }

    /// Damage of a pellet that has flown `dist` metres.
    pub fn damage_at(self, dist: f32) -> f32 {
        let s = self.stats();
        let k = if dist <= s.near {
            1.0
        } else if dist >= s.far {
            s.far_factor
        } else {
            1.0 + (s.far_factor - 1.0) * (dist - s.near) / (s.far - s.near)
        };
        s.damage * k
    }
}

/// Armor worn: more protection, slower feet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorKind {
    Light,
    Medium,
    Heavy,
    /// The infiltration suit: little armour, a cloak (the deploy key), and it hacks enemy
    /// terminals and turrets.
    Stealth,
    /// The MAX exosuit: heavy armour, slow, one big weapon (`Weapon::MAX_ARMS`); it doesn't
    /// drive or man guns, solve consoles or take medics' heals (engineers repair it), and
    /// taking one costs the base resources (`MAX_COST`).
    Max,
}

/// Resources a MAX takes from the base's silo.
pub const MAX_COST: f32 = 40.0;
/// The cloak: seconds it lasts from full, and how fast it fills again (per second).
pub const CLOAK_TIME: f32 = 12.0;
pub const CLOAK_REFILL: f32 = 0.08;
/// Enemies see a cloaked player only this close (m), and faint.
pub const CLOAK_SEEN: f32 = 10.0;
/// Seconds of hacking (standing at it) an enemy terminal or turret takes, and how long it
/// then works for the hacker's faction.
pub const HACK_TIME: f32 = 5.0;
pub const HACKED_FOR: f32 = 300.0;

impl ArmorKind {
    pub const ALL: [ArmorKind; 5] = [
        ArmorKind::Light,
        ArmorKind::Medium,
        ArmorKind::Heavy,
        ArmorKind::Stealth,
        ArmorKind::Max,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ArmorKind::Light => "Light",
            ArmorKind::Medium => "Medium",
            ArmorKind::Heavy => "Heavy",
            ArmorKind::Stealth => "Infiltrator",
            ArmorKind::Max => "MAX",
        }
    }

    pub fn max(self) -> f32 {
        match self {
            ArmorKind::Light => 30.0,
            ArmorKind::Medium => 60.0,
            ArmorKind::Heavy => 100.0,
            ArmorKind::Stealth => 15.0,
            ArmorKind::Max => 250.0,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            ArmorKind::Light => 1.0,
            ArmorKind::Medium => 0.92,
            ArmorKind::Heavy => 0.8,
            ArmorKind::Stealth => 1.0,
            ArmorKind::Max => 0.62,
        }
    }
}

/// The grenade one carries (picked at an equipment terminal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrenadeKind {
    /// Fragmentation: a blast that hurts.
    #[default]
    Frag,
    /// Strips armour, drops cloaks and drains them, and hurts enemy deployables; no damage.
    Emp,
    /// A cloud (`SMOKE_TIME`) that nobody sees through.
    Smoke,
    /// Blinds the enemies in sight of it for a while (`FLASH_TIME`).
    Flash,
}

impl GrenadeKind {
    pub const ALL: [GrenadeKind; 4] = [
        GrenadeKind::Frag,
        GrenadeKind::Emp,
        GrenadeKind::Smoke,
        GrenadeKind::Flash,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GrenadeKind::Frag => "Frag",
            GrenadeKind::Emp => "EMP",
            GrenadeKind::Smoke => "Smoke",
            GrenadeKind::Flash => "Flash",
        }
    }

    /// What it flies as.
    pub fn ammo(self) -> Ammo {
        match self {
            GrenadeKind::Frag => Ammo::Grenade,
            k => Ammo::Utility(k),
        }
    }
}

/// An EMP's reach and the armour it strips; a smoke cloud's radius and how long it lasts; how
/// far a flash blinds and for how long.
pub const EMP_RADIUS: f32 = 8.0;
pub const EMP_ARMOR: f32 = 60.0;
pub const SMOKE_RADIUS: f32 = 6.0;
pub const SMOKE_TIME: f32 = 12.0;
pub const FLASH_RADIUS: f32 = 12.0;
pub const FLASH_TIME: f32 = 3.0;

/// An engineer's deployable (with the repair gun): placed with the deploy key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kit {
    #[default]
    None,
    /// A small automatic turret firing at enemy infantry.
    Spitfire,
    /// Anti-tank mines: they go off under enemy ground vehicles.
    Mines,
    /// Refills the ammunition of everyone of its faction near it.
    AmmoPack,
}

impl Kit {
    pub const ALL: [Kit; 4] = [Kit::None, Kit::Spitfire, Kit::Mines, Kit::AmmoPack];

    pub fn name(self) -> &'static str {
        match self {
            Kit::None => "None",
            Kit::Spitfire => "Spitfire Turret",
            Kit::Mines => "AT Mines",
            Kit::AmmoPack => "Ammo Pack",
        }
    }

    /// How many one carries (refilled at an equipment terminal), and may have out at once.
    pub fn count(self) -> u8 {
        match self {
            Kit::None => 0,
            Kit::Spitfire | Kit::AmmoPack => 1,
            Kit::Mines => 3,
        }
    }
}

/// What a player carries (picked at an equipment terminal): a rifle, a sidearm and a
/// special weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Loadout {
    pub armor: ArmorKind,
    pub rifle: Weapon,
    pub sidearm: Weapon,
    pub special: Weapon,
    /// The engineer's deployable (with the repair gun only).
    pub kit: Kit,
    pub grenade: GrenadeKind,
}

impl Default for Loadout {
    fn default() -> Self {
        Loadout {
            armor: ArmorKind::Medium,
            rifle: Weapon::Rifle,
            sidearm: Weapon::Pistol,
            special: Weapon::Medgun,
            kit: Kit::None,
            grenade: GrenadeKind::Frag,
        }
    }
}

/// Seconds a lock-on takes (the target held in the launcher's cone, `LOCK_CONE`).
pub const LOCK_TIME: f32 = 1.5;
/// The cone's half angle (rad) a lock-on target has to stay in.
pub const LOCK_CONE: f32 = 0.12;
/// How far a lock-on reaches: aircraft (the Skyhook, the interceptor's missiles), and
/// ground vehicles (the Striker, aimed down its sights).
pub const LOCK_RANGE_AIR: f32 = 500.0;
pub const LOCK_RANGE_GROUND: f32 = 300.0;
/// How fast a homing missile turns (rad/s).
pub const HOMING_TURN: f32 = 2.2;

impl Ammo {
    /// Homes on a locked target: against aircraft (true) or ground vehicles (false).
    pub fn lock_on(self) -> Option<bool> {
        match self {
            Ammo::Gun(Weapon::Skyhook) | Ammo::Missile => Some(true),
            Ammo::Gun(Weapon::Striker) => Some(false),
            _ => None,
        }
    }
}

/// Weapon slots: 0 rifle, 1 sidearm, 2 special.
pub const SLOTS: usize = 3;

impl Loadout {
    /// Whether a player of `team` may take it.
    pub fn valid(&self, team: crate::Team) -> bool {
        let max = self.armor == ArmorKind::Max;
        let first = if max {
            Weapon::MAX_ARMS.contains(&self.rifle)
        } else {
            Weapon::rifles_for(team).contains(&self.rifle)
        };
        first
            && Weapon::SIDEARMS.contains(&self.sidearm)
            && Weapon::SPECIALS.contains(&self.special)
            && (self.kit == Kit::None || (self.special == Weapon::RepairGun && !max))
    }

    /// A MAX has only its arms (slot 0).
    pub fn max(&self) -> bool {
        self.armor == ArmorKind::Max
    }

    /// The slots one may switch to.
    pub fn usable(&self, slot: u8) -> bool {
        slot == 0 || (!self.max() && (slot as usize) < SLOTS)
    }

    /// The deployable carried (a MAX carries none).
    pub fn kit(&self) -> Kit {
        if self.max() { Kit::None } else { self.kit }
    }

    pub fn slot(&self, slot: u8) -> Weapon {
        match slot {
            1 => self.sidearm,
            2 => self.special,
            _ => self.rifle,
        }
    }

    pub fn weapons(&self) -> [Weapon; SLOTS] {
        [self.rifle, self.sidearm, self.special]
    }
}

pub const GRENADES: u8 = 2;
pub const GRENADE_FUSE: f32 = 2.5;
pub const GRENADE_RADIUS: f32 = 7.0;
pub const GRENADE_DAMAGE: f32 = 80.0;
pub const GRENADE_SPEED: f32 = 18.0;

/// Damage `amount` to a body with `health` and `armor`: armor takes two thirds while it lasts.
/// Returns the new (health, armor).
pub fn apply_damage(health: f32, armor: f32, amount: f32) -> (f32, f32) {
    let to_armor = (amount * 2.0 / 3.0).min(armor);
    let to_health = amount - to_armor;
    ((health - to_health).max(0.0), armor - to_armor)
}

/// Falls faster than this (m/s) hurt.
pub const FALL_SAFE: f32 = 13.0;
/// Damage per m/s over `FALL_SAFE` (bypassing armor).
pub const FALL_DAMAGE: f32 = 9.0;

/// What a projectile is: every projectile in the game flies by its ammo's `Ballistics`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ammo {
    Gun(Weapon),
    Grenade,
    /// The light tank's cannon.
    LightShell,
    /// The heavy tank's cannon.
    HeavyShell,
    /// The light AAA's airbursting flak.
    Flak,
    /// The VTOL's unguided rockets.
    Rocket,
    /// A base wall turret's bolt.
    TurretBolt,
    /// The heavy tank gunner's machine gun.
    Chaingun,
    /// The interceptor's nose cannon.
    AirCannon,
    /// The interceptor's homing missile (against aircraft).
    Missile,
    /// An engineer's spitfire turret's bolt.
    Spitfire,
    /// An anti-tank mine going off (its blast only; nothing flies).
    Mine,
    /// An EMP, smoke or flash grenade.
    Utility(GrenadeKind),
    /// A vehicle's autocannon (the light tank's, the AAA's, the rover's choice).
    Autocannon,
    /// The heavy tank's railgun (its choice): fast, hard, slow to fire.
    Rail,
}

/// How a projectile flies and what it does.
#[derive(Clone, Copy, Debug)]
pub struct Ballistics {
    pub speed: f32,
    pub gravity: f32,
    pub life: f32,
    /// Damage to infantry at full effect (falloff as `Stats`), and to vehicles (percent).
    pub damage: f32,
    pub vs_vehicle: f32,
    pub near: f32,
    pub far: f32,
    pub far_factor: f32,
    pub headshot: f32,
    pub splash: f32,
    pub splash_damage: f32,
    /// Splash against vehicles (percent at the middle).
    pub splash_vehicle: f32,
    /// Bursts this close to an aircraft (flak).
    pub airburst: f32,
    /// Bounces instead of hitting the ground, exploding when its life runs out (grenades).
    pub bounces: bool,
}

impl Ammo {
    pub fn ballistics(self) -> Ballistics {
        let base = Ballistics {
            speed: 200.0,
            gravity: 0.0,
            life: 2.0,
            damage: 10.0,
            vs_vehicle: 0.2,
            near: 40.0,
            far: 140.0,
            far_factor: 0.6,
            headshot: 2.0,
            splash: 0.0,
            splash_damage: 0.0,
            splash_vehicle: 0.0,
            airburst: 0.0,
            bounces: false,
        };
        match self {
            Ammo::Gun(w) => {
                let s = w.stats();
                // (The Burster's flak bursts near aircraft, as the AAA's.)
                let airburst = if w == Weapon::Burster { 6.0 } else { 0.0 };
                Ballistics {
                    airburst,
                    speed: s.speed,
                    gravity: s.gravity,
                    life: s.life,
                    damage: s.damage,
                    vs_vehicle: s.vs_vehicle,
                    near: s.near,
                    far: s.far,
                    far_factor: s.far_factor,
                    headshot: s.headshot,
                    splash: s.splash,
                    splash_damage: s.splash_damage,
                    splash_vehicle: s.vs_vehicle * 0.3,
                    ..base
                }
            }
            Ammo::Grenade => Ballistics {
                speed: GRENADE_SPEED,
                gravity: 9.81,
                life: GRENADE_FUSE,
                damage: 0.0,
                vs_vehicle: 0.0,
                headshot: 1.0,
                splash: GRENADE_RADIUS,
                splash_damage: GRENADE_DAMAGE,
                splash_vehicle: 4.0,
                bounces: true,
                ..base
            },
            Ammo::LightShell => Ballistics {
                speed: 160.0,
                gravity: 4.0,
                life: 4.0,
                damage: 70.0,
                vs_vehicle: 14.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                headshot: 1.0,
                splash: 5.0,
                splash_damage: 55.0,
                splash_vehicle: 4.0,
                ..base
            },
            Ammo::HeavyShell => Ballistics {
                speed: 180.0,
                gravity: 4.0,
                life: 4.0,
                damage: 100.0,
                vs_vehicle: 24.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                headshot: 1.0,
                splash: 6.0,
                splash_damage: 70.0,
                splash_vehicle: 6.0,
                ..base
            },
            Ammo::Flak => Ballistics {
                speed: 260.0,
                gravity: 2.0,
                life: 2.2,
                damage: 14.0,
                vs_vehicle: 2.2,
                near: 300.0,
                far: 600.0,
                far_factor: 0.7,
                headshot: 1.0,
                splash: 5.0,
                splash_damage: 10.0,
                splash_vehicle: 2.2,
                airburst: 7.0,
                ..base
            },
            Ammo::Rocket => Ballistics {
                speed: 140.0,
                gravity: 1.0,
                life: 3.0,
                damage: 45.0,
                vs_vehicle: 7.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                headshot: 1.0,
                splash: 4.5,
                splash_damage: 40.0,
                splash_vehicle: 3.0,
                ..base
            },
            Ammo::TurretBolt => Ballistics {
                speed: 180.0,
                life: 2.0,
                damage: 9.0,
                vs_vehicle: 1.2,
                near: 80.0,
                far: 250.0,
                far_factor: 0.7,
                headshot: 1.0,
                ..base
            },
            Ammo::AirCannon => Ballistics {
                speed: 300.0,
                life: 2.0,
                damage: 16.0,
                vs_vehicle: 1.1,
                near: 150.0,
                far: 500.0,
                far_factor: 0.6,
                headshot: 1.0,
                ..base
            },
            Ammo::Missile => Ballistics {
                speed: 150.0,
                life: 6.0,
                damage: 40.0,
                vs_vehicle: 26.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                headshot: 1.0,
                splash: 3.0,
                splash_damage: 30.0,
                splash_vehicle: 6.0,
                ..base
            },
            Ammo::Spitfire => Ballistics {
                speed: 200.0,
                life: 0.4,
                damage: 8.0,
                vs_vehicle: 0.2,
                near: 25.0,
                far: 60.0,
                far_factor: 0.6,
                headshot: 1.5,
                ..base
            },
            Ammo::Utility(k) => Ballistics {
                speed: GRENADE_SPEED,
                gravity: 9.81,
                life: GRENADE_FUSE,
                damage: 0.0,
                vs_vehicle: 0.0,
                headshot: 1.0,
                // (No damage: what it does is `ContinentSim::utility_burst`'s.)
                splash: match k {
                    GrenadeKind::Emp => EMP_RADIUS,
                    GrenadeKind::Smoke => SMOKE_RADIUS,
                    _ => FLASH_RADIUS,
                },
                splash_damage: 0.0,
                splash_vehicle: 0.0,
                bounces: true,
                ..base
            },
            Ammo::Autocannon => Ballistics {
                speed: 230.0,
                gravity: 2.0,
                life: 2.5,
                damage: 28.0,
                vs_vehicle: 3.2,
                near: 300.0,
                far: 700.0,
                far_factor: 0.8,
                headshot: 1.0,
                splash: 1.5,
                splash_damage: 15.0,
                splash_vehicle: 1.0,
                airburst: 3.0,
                ..base
            },
            Ammo::Rail => Ballistics {
                speed: 650.0,
                life: 2.0,
                damage: 110.0,
                vs_vehicle: 32.0,
                near: 1000.0,
                far: 2000.0,
                far_factor: 1.0,
                headshot: 1.0,
                ..base
            },
            Ammo::Mine => Ballistics {
                speed: 0.0,
                life: 0.0,
                damage: 0.0,
                vs_vehicle: 0.0,
                headshot: 1.0,
                splash: 5.0,
                splash_damage: 90.0,
                splash_vehicle: 13.5,
                ..base
            },
            Ammo::Chaingun => Ballistics {
                speed: 240.0,
                life: 2.0,
                damage: 14.0,
                vs_vehicle: 0.6,
                near: 60.0,
                far: 200.0,
                far_factor: 0.6,
                headshot: 1.5,
                ..base
            },
        }
    }

    /// Damage to infantry after `dist` metres of flight.
    pub fn damage_at(self, dist: f32) -> f32 {
        let b = self.ballistics();
        let k = if dist <= b.near {
            1.0
        } else if dist >= b.far {
            b.far_factor
        } else {
            1.0 + (b.far_factor - 1.0) * (dist - b.near) / (b.far - b.near)
        };
        b.damage * k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_soaks_two_thirds() {
        let (h, a) = apply_damage(100.0, 60.0, 30.0);
        assert!((h - 90.0).abs() < 1e-4 && (a - 40.0).abs() < 1e-4);
        // Armor runs out: the rest hits health.
        let (h, a) = apply_damage(100.0, 10.0, 30.0);
        assert!((h - 80.0).abs() < 1e-4 && a == 0.0);
        let (h, _) = apply_damage(10.0, 0.0, 30.0);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn spread_grows_on_the_move_and_shrinks_aiming() {
        let still = Weapon::Rifle.spread_for(false, false, 0.0, false);
        assert!(Weapon::Rifle.spread_for(true, false, 0.0, false) < still);
        assert!(Weapon::Rifle.spread_for(false, false, 1.0, false) > still * 1.5);
        assert!(Weapon::Rifle.spread_for(false, false, 0.0, true) > still * 2.0);
        assert!(Weapon::Lance.spread_for(true, true, 0.0, false) < 0.001);
    }

    #[test]
    fn falloff() {
        let r = Weapon::Rifle;
        assert_eq!(r.damage_at(10.0), 12.0);
        assert!((r.damage_at(140.0) - 7.2).abs() < 1e-4);
        assert!(r.damage_at(90.0) < 12.0 && r.damage_at(90.0) > 7.2);
    }
}
