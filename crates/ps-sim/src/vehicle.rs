//! Vehicles: what each kind is (seats, guns, hull, speed) and how it moves. Ground vehicles
//! follow the land (tracked ones turn on the spot, the wheeled bus turns as it rolls); the
//! aircraft (the VTOL, the interceptor, the dropship) fly: Space hovers up, W/S speed up and
//! slow down, the mouse yaws and pitches, A/D roll.
//! The server and the driver's client step the same physics (`Motion::step`), the driver's
//! prediction being accepted when plausible.

use glam::{EulerRot, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::collide::{Collider, World};
use crate::map::ContinentMap;
use crate::types::DriveCmd;
use crate::weapons::Ammo;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VehicleKind {
    LightTank,
    LightAaa,
    HeavyTank,
    SpawnBus,
    Vtol,
    /// Mines resource nodes with its beam and brings them to a base's silo.
    Harvester,
    /// A light, fast buggy: a driver and a gunner.
    Rover,
    /// A fast fighter: a nose cannon and homing missiles against aircraft.
    Interceptor,
    /// A transport: a pilot, two side gunners and passengers (squads spawn into its seats).
    Dropship,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionKind {
    Tracked,
    Wheeled,
    Flying,
}

#[derive(Clone, Copy, Debug)]
pub struct GunSpec {
    pub ammo: Ammo,
    /// Where it fires from: the turret's pivot (hull frame) and the muzzle ahead of it.
    pub pivot: Vec3,
    pub barrel: f32,
    /// Barrels it alternates between (spread sideways this far apart).
    pub barrels: u8,
    pub barrel_gap: f32,
    pub interval: f32,
    pub ammo_max: u16,
    /// Turret traverse (rad/s), and pitch limits.
    pub turn_rate: f32,
    pub pitch_min: f32,
    pub pitch_max: f32,
    /// Fixed forward (the VTOL's pods): aims where the craft points.
    pub fixed: bool,
    pub spread: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SeatSpec {
    /// The occupant's eyes, hull frame.
    pub eye: Vec3,
    /// The gun this seat fires (an index into `Spec::guns`).
    pub gun: Option<usize>,
    /// Drives / flies.
    pub driver: bool,
    /// A second gun fired with the other trigger (the interceptor's missiles).
    pub alt_gun: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub name: &'static str,
    pub motion: MotionKind,
    /// The hull's half extents (hull frame: x across, y up from the bottom, z front to back,
    /// the front towards -Z).
    pub half: Vec3,
    pub max_speed: f32,
    pub reverse: f32,
    pub accel: f32,
    pub turn_rate: f32,
    /// Steepest climb (rise over run).
    pub climb: f32,
    pub seats: Vec<SeatSpec>,
    pub guns: Vec<GunSpec>,
    /// Seconds between spawning one at a terminal (per player).
    pub cooldown: f32,
}

impl VehicleKind {
    pub const ALL: [VehicleKind; 9] = [
        VehicleKind::Rover,
        VehicleKind::LightTank,
        VehicleKind::LightAaa,
        VehicleKind::HeavyTank,
        VehicleKind::SpawnBus,
        VehicleKind::Vtol,
        VehicleKind::Interceptor,
        VehicleKind::Dropship,
        VehicleKind::Harvester,
    ];

    /// Flies.
    /// Needs a Tech Plant held on the continent (or the sanctuary) to be ordered.
    pub fn advanced(self) -> bool {
        matches!(
            self,
            VehicleKind::HeavyTank | VehicleKind::Interceptor | VehicleKind::Dropship
        )
    }

    pub fn aircraft(self) -> bool {
        matches!(
            self,
            VehicleKind::Vtol | VehicleKind::Interceptor | VehicleKind::Dropship
        )
    }

    /// The resources spawning one takes from the base's silo.
    pub fn cost(self) -> f32 {
        match self {
            VehicleKind::LightTank | VehicleKind::LightAaa => 40.0,
            VehicleKind::HeavyTank => 70.0,
            VehicleKind::SpawnBus => 50.0,
            VehicleKind::Vtol => 60.0,
            VehicleKind::Harvester => 0.0,
            VehicleKind::Rover => 15.0,
            VehicleKind::Interceptor => 50.0,
            VehicleKind::Dropship => 80.0,
        }
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn spec(self) -> Spec {
        let cannon = |ammo, interval, ammo_max, turn_rate| GunSpec {
            ammo,
            pivot: Vec3::new(0.0, 1.85, 0.3),
            barrel: 3.2,
            barrels: 1,
            barrel_gap: 0.0,
            interval,
            ammo_max,
            turn_rate,
            pitch_min: -0.15,
            pitch_max: 0.45,
            fixed: false,
            spread: 0.005,
        };
        match self {
            VehicleKind::LightTank => Spec {
                name: "Light Tank",
                motion: MotionKind::Tracked,
                half: Vec3::new(1.6, 1.25, 2.8),
                max_speed: 17.0,
                reverse: 7.0,
                accel: 7.0,
                turn_rate: 1.3,
                climb: 0.9,
                seats: vec![SeatSpec {
                    eye: Vec3::new(0.0, 2.4, 0.3),
                    gun: Some(0),
                    driver: true,
                    alt_gun: None,
                }],
                guns: vec![cannon(Ammo::LightShell, 1.3, 40, 1.4)],
                cooldown: 60.0,
            },
            VehicleKind::LightAaa => Spec {
                name: "Light AAA",
                motion: MotionKind::Tracked,
                half: Vec3::new(1.6, 1.25, 2.8),
                max_speed: 16.0,
                reverse: 7.0,
                accel: 7.0,
                turn_rate: 1.3,
                climb: 0.9,
                seats: vec![SeatSpec {
                    eye: Vec3::new(0.0, 2.4, 0.4),
                    gun: Some(0),
                    driver: true,
                    alt_gun: None,
                }],
                guns: vec![GunSpec {
                    ammo: Ammo::Flak,
                    pivot: Vec3::new(0.0, 1.9, 0.4),
                    barrels: 4,
                    barrel_gap: 0.32,
                    barrel: 2.4,
                    interval: 0.09,
                    ammo_max: 600,
                    turn_rate: 2.2,
                    pitch_min: -0.1,
                    pitch_max: 1.4,
                    spread: 0.018,
                    ..cannon(Ammo::Flak, 0.09, 600, 2.2)
                }],
                cooldown: 60.0,
            },
            VehicleKind::HeavyTank => Spec {
                name: "Heavy Tank",
                motion: MotionKind::Tracked,
                half: Vec3::new(2.2, 1.5, 3.8),
                max_speed: 11.0,
                reverse: 5.0,
                accel: 4.0,
                turn_rate: 0.7,
                climb: 0.8,
                seats: vec![
                    SeatSpec {
                        eye: Vec3::new(0.0, 2.8, 0.5),
                        gun: Some(0),
                        driver: true,
                        alt_gun: None,
                    },
                    SeatSpec {
                        eye: Vec3::new(0.0, 3.4, 1.4),
                        gun: Some(1),
                        driver: false,
                        alt_gun: None,
                    },
                ],
                guns: vec![
                    GunSpec {
                        pivot: Vec3::new(0.0, 2.2, 0.5),
                        barrel: 4.4,
                        ..cannon(Ammo::HeavyShell, 2.2, 30, 0.7)
                    },
                    GunSpec {
                        ammo: Ammo::Chaingun,
                        pivot: Vec3::new(0.0, 3.0, 1.2),
                        barrel: 1.1,
                        barrels: 1,
                        barrel_gap: 0.0,
                        interval: 0.1,
                        ammo_max: 500,
                        turn_rate: 3.0,
                        pitch_min: -0.3,
                        pitch_max: 1.0,
                        fixed: false,
                        spread: 0.02,
                    },
                ],
                cooldown: 90.0,
            },
            VehicleKind::SpawnBus => Spec {
                name: "Spawn Bus",
                motion: MotionKind::Wheeled,
                half: Vec3::new(1.7, 1.8, 5.0),
                max_speed: 19.0,
                reverse: 8.0,
                accel: 5.0,
                turn_rate: 0.9,
                climb: 0.75,
                seats: (0..5)
                    .map(|i| SeatSpec {
                        eye: Vec3::new(
                            if i == 0 {
                                -0.7
                            } else if i % 2 == 0 {
                                0.7
                            } else {
                                -0.7
                            },
                            2.4,
                            if i == 0 {
                                -3.8
                            } else {
                                -1.2 + ((i - 1) / 2) as f32 * 2.4
                            },
                        ),
                        gun: None,
                        driver: i == 0,
                        alt_gun: None,
                    })
                    .collect(),
                guns: vec![],
                cooldown: 120.0,
            },
            VehicleKind::Harvester => Spec {
                name: "Harvester",
                motion: MotionKind::Wheeled,
                half: Vec3::new(2.0, 1.8, 5.5),
                max_speed: 13.0,
                reverse: 6.0,
                accel: 4.0,
                turn_rate: 0.8,
                climb: 0.7,
                seats: vec![SeatSpec {
                    eye: Vec3::new(-0.7, 3.0, -4.0),
                    gun: Some(0),
                    driver: true,
                    alt_gun: None,
                }],
                // (The harvesting beam: aimed like a turret, fires no projectile.)
                guns: vec![GunSpec {
                    ammo: Ammo::Chaingun,
                    pivot: Vec3::new(0.0, 3.2, -2.0),
                    barrel: 2.0,
                    barrels: 1,
                    barrel_gap: 0.0,
                    interval: 0.1,
                    ammo_max: 1,
                    turn_rate: 2.0,
                    pitch_min: -0.6,
                    pitch_max: 0.5,
                    fixed: false,
                    spread: 0.0,
                }],
                cooldown: 30.0,
            },
            VehicleKind::Rover => Spec {
                name: "Rover",
                motion: MotionKind::Wheeled,
                half: Vec3::new(1.25, 0.95, 2.3),
                max_speed: 30.0,
                reverse: 9.0,
                accel: 11.0,
                turn_rate: 1.6,
                climb: 0.95,
                seats: vec![
                    SeatSpec {
                        eye: Vec3::new(-0.4, 1.75, -0.4),
                        gun: None,
                        driver: true,
                        alt_gun: None,
                    },
                    SeatSpec {
                        eye: Vec3::new(0.0, 2.35, 1.2),
                        gun: Some(0),
                        driver: false,
                        alt_gun: None,
                    },
                ],
                guns: vec![GunSpec {
                    ammo: Ammo::Chaingun,
                    pivot: Vec3::new(0.0, 1.85, 1.2),
                    barrel: 1.1,
                    barrels: 1,
                    barrel_gap: 0.0,
                    interval: 0.11,
                    ammo_max: 400,
                    turn_rate: 3.5,
                    pitch_min: -0.3,
                    pitch_max: 0.9,
                    fixed: false,
                    spread: 0.022,
                }],
                cooldown: 20.0,
            },
            VehicleKind::Interceptor => Spec {
                name: "Interceptor",
                motion: MotionKind::Flying,
                half: Vec3::new(3.0, 1.1, 4.0),
                max_speed: 85.0,
                reverse: 0.0,
                accel: 22.0,
                turn_rate: 2.1,
                climb: 10.0,
                seats: vec![SeatSpec {
                    eye: Vec3::new(0.0, 1.75, -1.6),
                    gun: Some(0),
                    driver: true,
                    alt_gun: Some(1),
                }],
                guns: vec![
                    GunSpec {
                        ammo: Ammo::AirCannon,
                        pivot: Vec3::new(0.0, 0.9, -2.5),
                        barrel: 1.6,
                        barrels: 2,
                        barrel_gap: 0.8,
                        interval: 0.09,
                        ammo_max: 500,
                        turn_rate: 0.0,
                        pitch_min: 0.0,
                        pitch_max: 0.0,
                        fixed: true,
                        spread: 0.01,
                    },
                    GunSpec {
                        ammo: Ammo::Missile,
                        pivot: Vec3::new(0.0, 0.6, -0.5),
                        barrel: 1.0,
                        barrels: 2,
                        barrel_gap: 4.4,
                        interval: 1.2,
                        ammo_max: 8,
                        turn_rate: 0.0,
                        pitch_min: 0.0,
                        pitch_max: 0.0,
                        fixed: true,
                        spread: 0.0,
                    },
                ],
                cooldown: 90.0,
            },
            VehicleKind::Dropship => Spec {
                name: "Dropship",
                motion: MotionKind::Flying,
                half: Vec3::new(4.2, 2.0, 7.0),
                max_speed: 42.0,
                reverse: 0.0,
                accel: 9.0,
                turn_rate: 0.9,
                climb: 10.0,
                seats: (0..10)
                    .map(|i| {
                        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                        let (eye, gun) = match i {
                            0 => (Vec3::new(0.0, 3.0, -5.6), None),
                            1 | 2 => (Vec3::new(side * 3.2, 2.6, 0.5), Some(i - 1)),
                            _ => (
                                Vec3::new(side * 1.0, 2.2, -2.0 + ((i - 3) / 2) as f32 * 1.6),
                                None,
                            ),
                        };
                        SeatSpec {
                            eye,
                            gun,
                            driver: i == 0,
                            alt_gun: None,
                        }
                    })
                    .collect(),
                guns: [-1.0f32, 1.0]
                    .map(|side| GunSpec {
                        ammo: Ammo::Chaingun,
                        pivot: Vec3::new(side * 3.6, 1.9, 0.5),
                        barrel: 1.3,
                        barrels: 1,
                        barrel_gap: 0.0,
                        interval: 0.1,
                        ammo_max: 600,
                        turn_rate: 3.0,
                        pitch_min: -1.0,
                        pitch_max: 0.6,
                        fixed: false,
                        spread: 0.02,
                    })
                    .to_vec(),
                cooldown: 150.0,
            },
            VehicleKind::Vtol => Spec {
                name: "VTOL",
                motion: MotionKind::Flying,
                half: Vec3::new(3.6, 1.4, 4.2),
                max_speed: 55.0,
                reverse: 0.0,
                accel: 14.0,
                turn_rate: 1.6,
                climb: 10.0,
                seats: vec![SeatSpec {
                    eye: Vec3::new(0.0, 1.9, -2.6),
                    gun: Some(0),
                    driver: true,
                    alt_gun: None,
                }],
                guns: vec![GunSpec {
                    ammo: Ammo::Rocket,
                    pivot: Vec3::new(0.0, 0.8, -1.0),
                    barrel: 2.0,
                    barrels: 2,
                    barrel_gap: 5.2,
                    interval: 0.28,
                    ammo_max: 40,
                    turn_rate: 0.0,
                    pitch_min: 0.0,
                    pitch_max: 0.0,
                    fixed: true,
                    spread: 0.016,
                }],
                cooldown: 90.0,
            },
        }
    }
}

/// How a vehicle is moving.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    /// The hull's bottom middle.
    pub pos: Vec3,
    pub rot: Quat,
    pub vel: Vec3,
    /// Ground vehicles: heading (yaw) apart from the land's tilt; aircraft: the forward speed
    /// asked for.
    pub yaw: f32,
    pub throttle: f32,
    /// On the ground (aircraft: landed).
    pub grounded: bool,
}

impl Motion {
    pub fn at(pos: Vec3, yaw: f32) -> Motion {
        Motion {
            pos,
            rot: Quat::from_rotation_y(yaw),
            vel: Vec3::ZERO,
            yaw,
            throttle: 0.0,
            grounded: true,
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.rot * Vec3::NEG_Z
    }

    /// A point in the hull's frame, in the world.
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.pos + self.rot * local
    }

    /// One step of `dt`: driving by `cmd` (none: nobody driving) over `world`. Returns the
    /// speed of an impact it took (ground or obstacle), for damage.
    pub fn step(
        &mut self,
        spec: &Spec,
        cmd: Option<&DriveCmd>,
        deployed: bool,
        world: &World,
        dt: f32,
    ) -> Option<f32> {
        match spec.motion {
            MotionKind::Flying => self.fly(spec, cmd, world, dt),
            _ => self.drive(spec, cmd, deployed, world, dt),
        }
    }

    fn ground_at(world: &World, p: Vec3, reach: f32) -> f32 {
        world.ground(p.x, p.z, p.y + reach).unwrap_or(-50.0)
    }

    fn drive(
        &mut self,
        spec: &Spec,
        cmd: Option<&DriveCmd>,
        deployed: bool,
        world: &World,
        dt: f32,
    ) -> Option<f32> {
        let none = DriveCmd::default();
        let c = if deployed {
            &none
        } else {
            cmd.unwrap_or(&none)
        };
        // (Ground vehicles keep their forward speed in `throttle`.)
        let speed = self.throttle;
        let target = if c.throttle >= 0.0 {
            c.throttle * spec.max_speed
        } else {
            c.throttle * spec.reverse
        };
        let mut new_speed = if self.grounded {
            let a = if target.abs() < speed.abs() || target.signum() != speed.signum() {
                spec.accel * 1.6
            } else {
                spec.accel
            };
            speed + (target - speed).clamp(-a * dt, a * dt)
        } else {
            speed
        };
        // Turning: tracks on the spot, wheels only while rolling.
        let turn = match spec.motion {
            MotionKind::Tracked => c.steer * spec.turn_rate,
            _ => c.steer * spec.turn_rate * (new_speed / 6.0).clamp(-1.0, 1.0),
        };
        if self.grounded {
            self.yaw -= turn * dt;
        }
        let fwd = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        // Too steep uphill slows to a crawl.
        let probe = self.pos + fwd * (spec.half.z + 1.0) * new_speed.signum();
        let rise = Self::ground_at(world, probe, 3.0) - self.pos.y;
        let run = spec.half.z + 1.0;
        if rise / run > spec.climb && new_speed.abs() > 1.0 {
            new_speed *= 1.0 - (2.5 * dt).min(1.0);
        }
        self.throttle = new_speed;
        let mut horiz = Vec2::new(self.vel.x, self.vel.z);
        let want = Vec2::new(fwd.x, fwd.z) * new_speed;
        if self.grounded {
            // Grip: the sideways slide dies quickly.
            horiz = horiz.lerp(want, (8.0 * dt).min(1.0));
        }
        let mut p = self.pos + Vec3::new(horiz.x, 0.0, horiz.y) * dt;
        // Obstacles: the hull as two circles pushed out of the colliders.
        let r = spec.half.x;
        let mut hit = None;
        for k in [-1.0f32, 1.0] {
            let off = fwd * (spec.half.z - r).max(0.0) * k;
            let mut c = p + off;
            let before = c;
            push_circle(world, &mut c, r, spec.half.y * 2.0);
            if (c - before).length_squared() > 1e-6 {
                p += c - before;
                let into = horiz.length();
                if into > 4.0 {
                    hit = Some(hit.unwrap_or(0.0f32).max(into));
                }
                horiz *= 0.5;
                self.throttle *= 0.5;
            }
        }
        p = ContinentMap::clamp(p);
        // Follow the land: the hull's height and tilt from four corners.
        let (hx, hz) = (spec.half.x * 0.9, spec.half.z * 0.9);
        let right = Vec3::new(-fwd.z, 0.0, fwd.x);
        let reach = 1.2;
        let corner = |dx: f32, dz: f32| {
            let q = p + right * dx - fwd * dz;
            Self::ground_at(world, q, reach)
        };
        let (fl, fr, bl, br) = (
            corner(-hx, -hz),
            corner(hx, -hz),
            corner(-hx, hz),
            corner(hx, hz),
        );
        let ground = (fl + fr + bl + br) * 0.25;
        let mut vy = self.vel.y;
        if p.y > ground + 0.3 {
            vy -= crate::collide::GRAVITY * dt;
            p.y += vy * dt;
            self.grounded = false;
        }
        if p.y <= ground + 0.3 {
            if !self.grounded && vy < -10.0 {
                hit = Some(hit.unwrap_or(0.0).max(-vy));
            }
            p.y = ground;
            vy = 0.0;
            self.grounded = true;
        }
        let pitch = (((fl + fr) - (bl + br)) * 0.5 / (2.0 * hz)).atan();
        let roll = (((fr + br) - (fl + bl)) * 0.5 / (2.0 * hx)).atan();
        let tilt = Quat::from_euler(EulerRot::YXZ, self.yaw, pitch, roll);
        self.rot = if self.grounded {
            self.rot.slerp(tilt, (10.0 * dt).min(1.0))
        } else {
            self.rot
        };
        self.vel = Vec3::new(horiz.x, vy, horiz.y);
        self.pos = p;
        hit
    }

    fn fly(&mut self, spec: &Spec, cmd: Option<&DriveCmd>, world: &World, dt: f32) -> Option<f32> {
        let none = DriveCmd::default();
        let c = cmd.unwrap_or(&none);
        let piloted = cmd.is_some();
        // Speed asked for: W/S change it.
        self.throttle = (self.throttle + c.throttle * spec.accel * dt).clamp(0.0, spec.max_speed);
        if !piloted {
            self.throttle = (self.throttle - spec.accel * dt).max(0.0);
        }
        // Turning: the mouse yaws and pitches, A/D roll; banking turns too.
        // (Per tick, as far as the craft's agility allows: the VTOL 0.08 rad.)
        let most = spec.turn_rate * 0.05;
        let (yaw_in, pitch_in) = (c.turn.x.clamp(-most, most), c.turn.y.clamp(-most, most));
        let roll_in = -c.steer * 1.8 * dt;
        let mut rot = self.rot;
        if !self.grounded || c.lift > 0.0 {
            rot = rot
                * Quat::from_rotation_y(yaw_in)
                * Quat::from_rotation_x(pitch_in)
                * Quat::from_rotation_z(roll_in);
        }
        // Banking turns the nose (coordinated turn), and the craft levels itself out gently
        // when not rolled by hand.
        let (yaw, pitch, roll) = rot.to_euler(EulerRot::YXZ);
        let speed_k = (self.throttle / 20.0).clamp(0.0, 1.0);
        let yaw = yaw + roll * 0.9 * speed_k * dt;
        let roll = if c.steer == 0.0 {
            roll * (1.0 - (1.2 * dt).min(1.0))
        } else {
            roll
        };
        let pitch = pitch.clamp(-1.3, 1.3);
        let roll = roll.clamp(-1.2, 1.2);
        rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        // Thrust: forward speed along the nose, hover lift up, sinking when slow without lift.
        let fwd = rot * Vec3::NEG_Z;
        let lift = c.lift * 9.0;
        let sink = if c.lift <= 0.0 {
            -5.0 * (1.0 - speed_k)
        } else {
            0.0
        };
        let target = fwd * self.throttle + Vec3::Y * (lift + sink);
        let mut vel = self.vel.lerp(target, (1.6 * dt).min(1.0));
        if self.grounded && c.lift <= 0.0 && self.throttle < 1.0 {
            vel = Vec3::ZERO;
        }
        let mut p = self.pos + vel * dt;
        p = ContinentMap::clamp(p);
        p.y = p.y.min(700.0);
        let mut hit = None;
        // A roof overhead (a cave's): the craft stops under it.
        let tall = spec.half.y * 2.0;
        if let Some(c) = world.ceiling(p.x, p.z, self.pos.y + tall - 0.1)
            && p.y + tall > c
        {
            if vel.y > 10.0 {
                hit = Some(hit.unwrap_or(0.0f32).max(vel.y));
            }
            p.y = c - tall - 0.01;
            vel.y = vel.y.min(0.0);
        }
        // Obstacles (one circle for the fuselage; the wings ride over).
        let before = p;
        push_circle(world, &mut p, spec.half.x * 0.6, spec.half.y * 2.0);
        if (p - before).length() > 1e-3 && vel.length() > 12.0 {
            hit = Some(vel.length());
            vel *= 0.3;
        }
        let ground = Self::ground_at(world, p + Vec3::Y * 2.0, 2.0);
        if p.y <= ground {
            let down = -vel.y;
            let level = pitch.abs() < 0.5 && roll.abs() < 0.5;
            if down > 7.0 || vel.length() > 22.0 || !level {
                hit = Some(hit.unwrap_or(0.0f32).max(vel.length()));
            }
            p.y = ground;
            vel.y = vel.y.max(0.0);
            self.grounded = c.lift <= 0.0;
            if self.grounded {
                vel = Vec3::ZERO;
                self.throttle = 0.0;
                rot = Quat::from_rotation_y(yaw);
            }
        } else {
            self.grounded = false;
        }
        self.rot = rot;
        self.vel = vel;
        self.pos = p;
        self.yaw = yaw;
        hit
    }
}

/// Pushes a vertical circle (radius `r`, from `p.y` up `h`) out of the world's colliders.
pub fn push_circle(world: &World, p: &mut Vec3, r: f32, h: f32) {
    for c in world.near_colliders(Vec2::new(p.x - r, p.z - r), Vec2::new(p.x + r, p.z + r)) {
        let (min, max) = c.bounds();
        // Low things are driven over.
        if max.y <= p.y + 0.9 || min.y >= p.y + h {
            continue;
        }
        if let Collider::Prism { n, pts, .. } = c {
            if let Some(push) = Collider::prism_push(n, &pts, Vec2::new(p.x, p.z), r) {
                p.x += push.x;
                p.z += push.y;
            }
            continue;
        }
        if let Collider::Ramp { .. } = c {
            continue;
        }
        let nx = p.x.clamp(min.x, max.x);
        let nz = p.z.clamp(min.z, max.z);
        let (dx, dz) = (p.x - nx, p.z - nz);
        let d2 = dx * dx + dz * dz;
        if d2 >= r * r {
            continue;
        }
        if d2 > 1e-8 {
            let d = d2.sqrt();
            p.x += dx / d * (r - d);
            p.z += dz / d * (r - d);
        } else {
            let opts = [
                (p.x - min.x + r, Vec2::new(-1.0, 0.0)),
                (max.x - p.x + r, Vec2::new(1.0, 0.0)),
                (p.z - min.z + r, Vec2::new(0.0, -1.0)),
                (max.z - p.z + r, Vec2::new(0.0, 1.0)),
            ];
            let (d, n) = opts
                .iter()
                .copied()
                .fold(opts[0], |a, b| if b.0 < a.0 { b } else { a });
            p.x += n.x * d;
            p.z += n.y * d;
        }
    }
}

/// The distance along the segment `a`..`b` (fraction) where it enters a vehicle's hull.
pub fn hit_hull(m: &Motion, half: Vec3, a: Vec3, b: Vec3) -> Option<f32> {
    let len = (b - a).length();
    if len < 1e-6 {
        return None;
    }
    let inv = m.rot.inverse();
    let lo = inv * (a - m.pos);
    let ld = inv * ((b - a) / len);
    let min = Vec3::new(-half.x, 0.0, -half.z);
    let max = Vec3::new(half.x, half.y * 2.0, half.z);
    if lo.cmpge(min).all() && lo.cmple(max).all() {
        return Some(0.0);
    }
    crate::collide::ray_box(lo, ld, min, max, len).map(|(t, _)| t / len)
}

/// How far `p` is from the hull's box (0 inside).
pub fn hull_distance(m: &Motion, half: Vec3, p: Vec3) -> f32 {
    let l = m.rot.inverse() * (p - m.pos);
    let min = Vec3::new(-half.x, 0.0, -half.z);
    let max = Vec3::new(half.x, half.y * 2.0, half.z);
    (l.clamp(min, max) - l).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> World {
        World::new(
            vec![Collider::Box {
                min: Vec3::new(-500.0, -1.0, -500.0),
                max: Vec3::new(500.0, 0.0, 500.0),
            }],
            None,
        )
    }

    #[test]
    fn tanks_drive_and_turn_on_the_spot() {
        let w = flat();
        let spec = VehicleKind::LightTank.spec();
        let mut m = Motion::at(Vec3::ZERO, 0.0);
        let go = DriveCmd {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            m.step(&spec, Some(&go), false, &w, 1.0 / 60.0);
        }
        assert!(
            m.pos.z < -50.0 && m.pos.x.abs() < 0.5,
            "{} {} {}",
            m.pos,
            m.vel,
            m.grounded
        );
        assert!((m.vel.length() - spec.max_speed).abs() < 1.0);
        let turn = DriveCmd {
            steer: 1.0,
            ..Default::default()
        };
        let mut m = Motion::at(Vec3::ZERO, 0.0);
        for _ in 0..60 {
            m.step(&spec, Some(&turn), false, &w, 1.0 / 60.0);
        }
        assert!(
            m.pos.length() < 0.5 && m.yaw.abs() > 1.0,
            "{} {}",
            m.pos,
            m.yaw
        );
    }

    #[test]
    fn tanks_stop_at_walls() {
        let mut w = flat();
        w = World::new(
            [
                w.colliders.clone(),
                vec![Collider::Box {
                    min: Vec3::new(-20.0, 0.0, -32.0),
                    max: Vec3::new(20.0, 8.0, -30.0),
                }],
            ]
            .concat(),
            None,
        );
        let spec = VehicleKind::HeavyTank.spec();
        let mut m = Motion::at(Vec3::ZERO, 0.0);
        let go = DriveCmd {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..600 {
            m.step(&spec, Some(&go), false, &w, 1.0 / 60.0);
        }
        assert!(m.pos.z > -30.0, "{}", m.pos);
    }

    #[test]
    fn vtols_take_off_hover_and_fly() {
        let w = flat();
        let spec = VehicleKind::Vtol.spec();
        let mut m = Motion::at(Vec3::ZERO, 0.0);
        let up = DriveCmd {
            lift: 1.0,
            ..Default::default()
        };
        for _ in 0..240 {
            m.step(&spec, Some(&up), false, &w, 1.0 / 60.0);
        }
        assert!(m.pos.y > 10.0 && !m.grounded, "{}", m.pos);
        let h = m.pos.y;
        let go = DriveCmd {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            m.step(&spec, Some(&go), false, &w, 1.0 / 60.0);
        }
        // Flying forward at speed holds its height (roughly) and goes north.
        assert!(m.pos.z < -100.0, "{}", m.pos);
        assert!((m.pos.y - h).abs() < 25.0, "{} vs {h}", m.pos.y);
    }

    #[test]
    fn vtols_stop_under_a_roof() {
        let w = World::new(
            vec![
                flat().colliders[0],
                Collider::Box {
                    min: Vec3::new(-100.0, 14.0, -100.0),
                    max: Vec3::new(100.0, 18.0, 100.0),
                },
            ],
            None,
        );
        let spec = VehicleKind::Vtol.spec();
        let mut m = Motion::at(Vec3::ZERO, 0.0);
        let up = DriveCmd {
            lift: 1.0,
            ..Default::default()
        };
        for _ in 0..600 {
            m.step(&spec, Some(&up), false, &w, 1.0 / 60.0);
            assert!(m.pos.y + spec.half.y * 2.0 <= 14.0 + 1e-3, "{}", m.pos);
        }
        assert!(m.pos.y > 5.0, "{}", m.pos);
    }

    #[test]
    fn rovers_outrun_tanks_and_interceptors_outfly_vtols() {
        let w = flat();
        let go = DriveCmd {
            throttle: 1.0,
            ..Default::default()
        };
        let run = |kind: VehicleKind, lift: bool| {
            let spec = kind.spec();
            let mut m = Motion::at(Vec3::ZERO, 0.0);
            if lift {
                let up = DriveCmd {
                    lift: 1.0,
                    ..Default::default()
                };
                for _ in 0..240 {
                    m.step(&spec, Some(&up), false, &w, 1.0 / 60.0);
                }
            }
            for _ in 0..600 {
                m.step(&spec, Some(&go), false, &w, 1.0 / 60.0);
            }
            -m.pos.z
        };
        let (rover, tank) = (
            run(VehicleKind::Rover, false),
            run(VehicleKind::LightTank, false),
        );
        assert!(rover > tank * 1.4, "rover {rover} vs tank {tank}");
        let (fighter, vtol) = (
            run(VehicleKind::Interceptor, true),
            run(VehicleKind::Vtol, true),
        );
        assert!(fighter > vtol * 1.3, "interceptor {fighter} vs vtol {vtol}");
        let dropship = run(VehicleKind::Dropship, true);
        assert!(dropship > 100.0 && dropship < vtol, "dropship {dropship}");
        for k in VehicleKind::ALL {
            let s = k.spec();
            assert_eq!(s.motion == MotionKind::Flying, k.aircraft(), "{k:?}");
            assert!(s.seats[0].driver);
            for seat in &s.seats {
                for g in [seat.gun, seat.alt_gun].into_iter().flatten() {
                    assert!(g < s.guns.len(), "{k:?}");
                }
            }
        }
    }
}

/// A vehicle's defensive slot (picked at the terminal): used with the deploy key by the driver
/// (plating works by itself).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Defense {
    #[default]
    None,
    /// Aircraft: flares (locks break, missiles go astray).
    Flares,
    /// A smoke screen round it (nobody sees or locks on through it).
    Smoke,
    /// Puts out the fire: a quick repair.
    FireSuppression,
    /// Armour plating: a fifth less damage, always.
    Plating,
}

impl Defense {
    pub fn name(self) -> &'static str {
        match self {
            Defense::None => "None",
            Defense::Flares => "Flares",
            Defense::Smoke => "Smoke Screen",
            Defense::FireSuppression => "Fire Suppression",
            Defense::Plating => "Armour Plating",
        }
    }

    /// Seconds before it may be used again.
    pub fn cooldown(self) -> f32 {
        match self {
            Defense::Flares => 12.0,
            Defense::Smoke => 20.0,
            Defense::FireSuppression => 30.0,
            Defense::None | Defense::Plating => 0.0,
        }
    }
}

/// Health (percent) fire suppression puts back; what share of damage plating keeps off.
pub const FIRE_SUPPRESSION: f32 = 30.0;
pub const PLATING: f32 = 0.2;

/// A vehicle as ordered: its main gun's choice (`VehicleKind::gun_options`, 0 the standard
/// one) and its defensive slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VehicleLoadout {
    pub weapon: u8,
    pub defense: Defense,
}

/// A choice of main gun: what it fires, how often, how much it carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GunOption {
    pub name: &'static str,
    pub ammo: Ammo,
    pub interval: f32,
    pub ammo_max: u16,
}

impl VehicleKind {
    /// The main gun's choices (gun 0's): the standard one first. One choice: none to make.
    pub fn gun_options(self) -> Vec<GunOption> {
        // (A vehicle without a gun, the bus, has the one empty choice.)
        let std = |name| match self.spec().guns.first() {
            Some(g) => GunOption {
                name,
                ammo: g.ammo,
                interval: g.interval,
                ammo_max: g.ammo_max,
            },
            None => GunOption {
                name: "None",
                ammo: Ammo::Chaingun,
                interval: 1.0,
                ammo_max: 0,
            },
        };
        match self {
            VehicleKind::LightTank => vec![
                std("Cannon"),
                GunOption {
                    name: "Autocannon",
                    ammo: Ammo::Autocannon,
                    interval: 0.22,
                    ammo_max: 160,
                },
            ],
            VehicleKind::HeavyTank => vec![
                std("Heavy Cannon"),
                GunOption {
                    name: "Railgun",
                    ammo: Ammo::Rail,
                    interval: 3.2,
                    ammo_max: 24,
                },
            ],
            VehicleKind::LightAaa => vec![
                std("Flak"),
                GunOption {
                    name: "Autocannon",
                    ammo: Ammo::Autocannon,
                    interval: 0.22,
                    ammo_max: 160,
                },
            ],
            VehicleKind::Vtol => vec![
                std("Rockets"),
                GunOption {
                    name: "Nose Cannon",
                    ammo: Ammo::AirCannon,
                    interval: 0.1,
                    ammo_max: 400,
                },
            ],
            VehicleKind::Rover => vec![
                std("Chaingun"),
                GunOption {
                    name: "Autocannon",
                    ammo: Ammo::Autocannon,
                    interval: 0.25,
                    ammo_max: 140,
                },
            ],
            _ => vec![std("Standard")],
        }
    }

    /// The defensive slot's choices: the standard one first.
    pub fn defenses(self) -> &'static [Defense] {
        match self {
            // (The bus's deploy key deploys it.)
            VehicleKind::SpawnBus => &[Defense::None, Defense::Plating],
            k if k.aircraft() => &[Defense::Flares, Defense::FireSuppression, Defense::Plating],
            _ => &[Defense::Smoke, Defense::FireSuppression, Defense::Plating],
        }
    }

    pub fn default_loadout(self) -> VehicleLoadout {
        VehicleLoadout {
            weapon: 0,
            defense: self.defenses()[0],
        }
    }

    /// Whether `l` is one of its choices.
    pub fn loadout_valid(self, l: VehicleLoadout) -> bool {
        (l.weapon as usize) < self.gun_options().len() && self.defenses().contains(&l.defense)
    }

    /// Gun `g` as loaded out (the main gun's choice applied).
    pub fn gun_spec(self, l: VehicleLoadout, g: usize) -> GunSpec {
        let mut gs = self.spec().guns[g];
        if g == 0
            && l.weapon > 0
            && let Some(o) = self.gun_options().get(l.weapon as usize)
        {
            gs.ammo = o.ammo;
            gs.interval = o.interval;
            gs.ammo_max = o.ammo_max;
        }
        gs
    }
}

#[cfg(test)]
mod loadout_tests {
    use super::*;

    #[test]
    fn every_vehicle_has_a_valid_standard_loadout() {
        for k in VehicleKind::ALL {
            assert!(!k.gun_options().is_empty(), "{k:?}");
            assert!(k.loadout_valid(VehicleLoadout {
                weapon: 0,
                defense: k.defenses()[0],
            }));
        }
    }
}
