//! A base's structures, from its site.
//!
//! - **Bases** (outposts, center bases, sanctuary homes): a square compound of tall walls you
//!   can walk on (ramps up along the inside of every wall), two vehicle gateways closed to enemy
//!   vehicles by forcefields (infantry walk through), wall turrets, the resource silo, and the
//!   main building in the middle: on the ground floor the spawn room (a forcefield across its
//!   doorway lets only the owners through), the hall with terminals, the core room with the
//!   console that is hacked and the generator room powering the base; usually an upper floor of
//!   offices up a stair ramp; a roof reached from outside. Furnished with crates, racks and
//!   desks. In the yard: the vehicle pads with their terminals and the rearm pads.
//! - **Towers**: three storeys up ramps inside, the capture console on the top floor, turrets
//!   on the roof, a spawn bunker below ground, and an apron round the foot.
//!
//! Ramps are drawn as stairs. The buildings and walls are dressed with drawn-only `Detail`s
//! (angled cornices, plinths, piers and buttresses, framed and glazed windows, string courses,
//! vents, gateway pylons) that nothing collides with.
//!
//! Laid out in the site's frame (its front towards -Z) and turned by quarter turns, so every
//! collider stays axis-aligned (the details needn't be).

use glam::{Vec2, Vec3};

use crate::collide::{Collider, round_prism};
use crate::noise::Seq;
use crate::worldgen::{BaseKind, BaseSite};

/// What a structure box is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Wall,
    Parapet,
    Building,
    Roof,
    Floor,
    Interior,
    Ramp,
    Pillar,
    /// Furniture: crates, racks, desks.
    Decor,
}

/// What a detail is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    /// The walls' plating.
    Body,
    /// Dark glass.
    Glass,
    /// A neon strip in the owner's colour.
    Trim,
}

/// A piece of the architecture that is drawn but not solid (cornices, buttresses, plinths,
/// window frames and panes, vents): a section of four corners at its start (`pts[..4]`, in
/// order round it) swept to the same four at its end (`pts[4..]`), in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    pub pts: [Vec3; 8],
    pub finish: Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalKind {
    /// Pick a loadout; also refills ammo and armor.
    Equipment,
    /// Heals to full.
    Medical,
    /// Spawns vehicles on the pad next to it.
    Vehicle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terminal {
    pub kind: TerminalKind,
    pub pos: Vec3,
    /// The way its screen faces (yaw, 0 = -Z): towards whoever uses it.
    pub yaw: f32,
    /// Vehicle terminals: the pad they spawn on.
    pub pad: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shield {
    pub collider: Collider,
}

/// A light the client hangs: a floodlight on the walls (pointing at `aim`), or a room's
/// ceiling light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    pub pos: Vec3,
    pub aim: Option<Vec3>,
}

#[derive(Clone, Debug)]
pub struct BaseLayout {
    pub kind: BaseKind,
    pub center: Vec3,
    /// Quarter turns of the base's frame.
    pub quarter: u8,
    /// The compound's half width (to the walls' outer face; a tower's footprint).
    pub half: f32,
    pub wall_h: f32,
    pub colliders: Vec<Collider>,
    pub styles: Vec<Style>,
    /// Team forcefields bodies of other factions don't pass (spawn rooms, the tower bunker).
    pub shields: Vec<Shield>,
    /// Forcefields across the gateways: other factions' vehicles don't pass.
    pub gate_fields: Vec<Collider>,
    /// Spawn points (feet, yaw).
    pub spawns: Vec<(Vec3, f32)>,
    /// The console (hacked to capture the base): where it stands and faces.
    pub console: Option<(Vec3, f32)>,
    pub terminals: Vec<Terminal>,
    /// Vehicle pads (middle, yaw vehicles face).
    pub pads: Vec<(Vec3, f32)>,
    pub rearm: Vec<Vec3>,
    /// Turrets' mounts.
    pub turrets: Vec<Vec3>,
    /// The generator (its middle on the floor) and the resource silo (its foot).
    pub generator: Option<Vec3>,
    pub silo: Option<Vec3>,
    pub lamps: Vec<Lamp>,
    /// The architecture's drawn-only details.
    pub details: Vec<Detail>,
    /// Areas (XZ min, max) where the land isn't solid (a tower's bunker under it). The land
    /// isn't drawn over them either: its cells touching one are left out, and the tower's
    /// apron covers the ground between.
    pub holes: Vec<(Vec2, Vec2)>,
    /// The inside of the main building (for "inside" tests: min, max).
    pub building: (Vec3, Vec3),
    pub storeys: u8,
    /// Ways through the base for bots: from outside the front gateway to the console, and from
    /// the spawn room out through the front gateway.
    pub to_console: Vec<Vec3>,
    pub from_spawn: Vec<Vec3>,
}

pub const WALL_T: f32 = 4.0;
pub const WALL_H: f32 = 12.0;
pub const GATEWAY: f32 = 18.0;
pub const BLD_HALF: Vec2 = Vec2::new(18.0, 14.0);
pub const BLD_C: Vec2 = Vec2::new(0.0, 8.0);
/// A storey's height (floor to floor) and the floor slabs' thickness.
pub const STOREY: f32 = 5.4;
pub const SLAB: f32 = 0.4;
pub const T: f32 = 0.6;
pub const DOOR_W: f32 = 5.0;
pub const DOOR_H: f32 = 4.0;
pub const RAMP_LEN: f32 = 24.0;
pub const RAMP_W: f32 = 3.5;
/// The rearm pad's reach, and the vehicle pads'.
pub const REARM_RADIUS: f32 = 8.0;
pub const PAD_RADIUS: f32 = 7.0;
/// How close a player stands to use a terminal or console.
pub const USE_REACH: f32 = 2.6;
/// A turret's pivot above its mount, its barrels' length, and where its gunner's eyes are
/// from the pivot. Mounts stand on pillars `TURRET_RISE` over the wall (or roof) for a view
/// down past its foot.
pub const TURRET_PIVOT: f32 = 2.4;
pub const TURRET_RISE: f32 = 3.5;
pub const TURRET_BARREL: f32 = 2.6;
pub const TURRET_EYE: Vec3 = Vec3::new(0.0, 0.5, 0.6);
/// The generator's half extents, and the silo's radius and height.
pub const GENERATOR_HALF: Vec3 = Vec3::new(2.0, 1.75, 2.0);
pub const SILO_R: f32 = 4.0;
pub const SILO_H: f32 = 12.0;
/// A harvester unloads within this of a silo.
pub const SILO_REACH: f32 = 16.0;
/// A tower's half width, storeys and bunker depth.
pub const TOWER_HALF: f32 = 8.0;

struct Builder {
    origin: Vec3,
    q: u8,
    out: BaseLayout,
}

fn rot(q: u8, v: Vec2) -> Vec2 {
    match q % 4 {
        0 => v,
        1 => Vec2::new(v.y, -v.x),
        2 => Vec2::new(-v.x, -v.y),
        _ => Vec2::new(-v.y, v.x),
    }
}

/// An opening in a wall: along it from its start (`off`, `w`), from `lo` up to `hi` above the
/// wall's foot.
#[derive(Clone, Copy)]
struct Gap {
    off: f32,
    w: f32,
    lo: f32,
    hi: f32,
}

fn door(off: f32, w: f32, h: f32) -> Gap {
    Gap {
        off,
        w,
        lo: 0.0,
        hi: h,
    }
}

impl Builder {
    /// A point in the base's frame to the world.
    fn p(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let r = rot(self.q, Vec2::new(x, z));
        self.origin + Vec3::new(r.x, y, r.y)
    }

    fn yaw(&self, local: f32) -> f32 {
        local + self.q as f32 * std::f32::consts::FRAC_PI_2
    }

    fn world_box(&self, min: Vec3, max: Vec3) -> Collider {
        let a = self.p(min.x, min.y, min.z);
        let b = self.p(max.x, max.y, max.z);
        Collider::Box {
            min: a.min(b),
            max: a.max(b),
        }
    }

    fn boxed(&mut self, min: Vec3, max: Vec3, style: Style) {
        let c = self.world_box(min, max);
        self.out.colliders.push(c);
        self.out.styles.push(style);
    }

    /// A ramp climbing towards local direction `dir` (0 +Z, 1 +X, 2 -Z, 3 -X).
    fn ramp(&mut self, min: Vec3, max: Vec3, dir: u8) {
        let a = self.p(min.x, min.y, min.z);
        let b = self.p(max.x, max.y, max.z);
        self.out.colliders.push(Collider::Ramp {
            min: a.min(b),
            max: a.max(b),
            dir: (dir + self.q) % 4,
        });
        self.out.styles.push(Style::Ramp);
    }

    /// A wall from `a` to `b` (along x or z), `t` thick, `y0`..`y1` high, with openings.
    #[allow(clippy::too_many_arguments)]
    fn wall(&mut self, a: Vec2, b: Vec2, t: f32, y0: f32, y1: f32, gaps: &[Gap], style: Style) {
        let along_x = (b.y - a.y).abs() < 1e-3;
        let len = if along_x { b.x - a.x } else { b.y - a.y };
        let mut cuts = gaps.to_vec();
        cuts.sort_by(|x, y| x.off.total_cmp(&y.off));
        // Pieces: (start, end, bottom, top) along the wall.
        let mut pieces: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut at = 0.0;
        for g in &cuts {
            if g.off > at {
                pieces.push((at, g.off, y0, y1));
            }
            if g.lo > 0.0 {
                pieces.push((g.off, g.off + g.w, y0, y0 + g.lo));
            }
            if y0 + g.hi < y1 {
                pieces.push((g.off, g.off + g.w, y0 + g.hi, y1));
            }
            at = g.off + g.w;
        }
        if at < len {
            pieces.push((at, len, y0, y1));
        }
        for (s, e, lo, hi) in pieces {
            let (min, max) = if along_x {
                (
                    Vec3::new(a.x + s, lo, a.y - t / 2.0),
                    Vec3::new(a.x + e, hi, a.y + t / 2.0),
                )
            } else {
                (
                    Vec3::new(a.x - t / 2.0, lo, a.y + s),
                    Vec3::new(a.x + t / 2.0, hi, a.y + e),
                )
            };
            self.boxed(min, max, style);
        }
    }

    /// A floor slab over the rectangle, with holes cut (rectangles, x0, z0, x1, z1).
    fn slab(
        &mut self,
        r: (f32, f32, f32, f32),
        y: f32,
        holes: &[(f32, f32, f32, f32)],
        style: Style,
    ) {
        // Split the rectangle by the holes' x and z edges into a grid, keep cells not in a hole.
        let mut xs = vec![r.0, r.2];
        let mut zs = vec![r.1, r.3];
        for h in holes {
            xs.extend([h.0.clamp(r.0, r.2), h.2.clamp(r.0, r.2)]);
            zs.extend([h.1.clamp(r.1, r.3), h.3.clamp(r.1, r.3)]);
        }
        xs.sort_by(f32::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        zs.sort_by(f32::total_cmp);
        zs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        for i in 0..xs.len() - 1 {
            // Merge runs along z into one box per column.
            let mut run: Option<f32> = None;
            for j in 0..zs.len() {
                let solid = j < zs.len() - 1 && {
                    let (cx, cz) = ((xs[i] + xs[i + 1]) / 2.0, (zs[j] + zs[j + 1]) / 2.0);
                    !holes
                        .iter()
                        .any(|h| cx > h.0 && cx < h.2 && cz > h.1 && cz < h.3)
                };
                match (solid, run) {
                    (true, None) => run = Some(zs[j]),
                    (false, Some(z0)) => {
                        self.boxed(
                            Vec3::new(xs[i], y - SLAB, z0),
                            Vec3::new(xs[i + 1], y, zs[j]),
                            style,
                        );
                        run = None;
                    }
                    _ => {}
                }
            }
        }
    }

    fn lamp(&mut self, x: f32, y: f32, z: f32) {
        let pos = self.p(x, y, z);
        self.out.lamps.push(Lamp { pos, aim: None });
    }

    fn flood(&mut self, at: (f32, f32, f32), aim: (f32, f32, f32)) {
        let pos = self.p(at.0, at.1, at.2);
        let aim = self.p(aim.0, aim.1, aim.2);
        self.out.lamps.push(Lamp {
            pos,
            aim: Some(aim),
        });
    }

    fn terminal(&mut self, kind: TerminalKind, x: f32, y: f32, z: f32, yaw: f32) {
        let pos = self.p(x, y, z);
        let yaw = self.yaw(yaw);
        self.out.terminals.push(Terminal {
            kind,
            pos,
            yaw,
            pad: None,
        });
    }

    fn decor(&mut self, x: f32, y: f32, z: f32, size: Vec3) {
        self.boxed(
            Vec3::new(x - size.x / 2.0, y, z - size.z / 2.0),
            Vec3::new(x + size.x / 2.0, y + size.y, z + size.z / 2.0),
            Style::Decor,
        );
    }

    fn pts(&self, list: &[(f32, f32)], y: f32) -> Vec<Vec3> {
        list.iter().map(|&(x, z)| self.p(x, y, z)).collect()
    }

    /// A drawn-only piece along face `f` from `s` to `e`, its section `sec` (out from the face,
    /// height) round it; mitred where it reaches the face's corners.
    fn run(&mut self, f: &Face, s: f32, e: f32, sec: Sec, finish: Finish) {
        let (s, e) = (s.max(0.0), e.min(f.len));
        if e - s < 0.05 {
            return;
        }
        let (ms, me) = (s <= 1e-3, e >= f.len - 1e-3);
        let mut pts = [Vec3::ZERO; 8];
        for (k, &(o, y)) in sec.iter().enumerate() {
            let a = f.at(s - if ms { o } else { 0.0 }, o);
            let b = f.at(e + if me { o } else { 0.0 }, o);
            pts[k] = self.p(a.x, y, a.y);
            pts[k + 4] = self.p(b.x, y, b.y);
        }
        self.out.details.push(Detail { pts, finish });
    }

    /// `run` along the whole face but for the stretches `cuts`.
    fn runs(&mut self, f: &Face, cuts: &[(f32, f32)], sec: Sec, finish: Finish) {
        let mut cuts = cuts.to_vec();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut at = 0.0;
        for (s, e) in cuts {
            self.run(f, at, s, sec, finish);
            at = at.max(e);
        }
        self.run(f, at, f.len, sec, finish);
    }
}

/// A section swept along a face: four corners (out from the face, height), in order round it.
type Sec = [(f32, f32); 4];

/// A building's outer face, for its details: from its corner `a` (base frame XZ) along `u`,
/// facing `out`, `len` to the next corner; its doorways and windows (along it from `a`), and
/// stretches kept clear above the roof line (where a ramp comes up).
struct Face {
    a: Vec2,
    u: Vec2,
    out: Vec2,
    len: f32,
    doors: Vec<(f32, f32, f32)>,
    windows: Vec<(f32, f32, f32, f32)>,
    clear: Vec<(f32, f32)>,
}

impl Face {
    fn at(&self, along: f32, out: f32) -> Vec2 {
        self.a + self.u * along + self.out * out
    }

    /// The face of a wall from `a` to `b` (its middle line, `t` thick) facing `out`, with the
    /// wall's gaps (doors from the foot, the rest windows).
    fn of(a: Vec2, b: Vec2, t: f32, out: Vec2, gaps: &[Gap]) -> Face {
        let u = (b - a).normalize();
        let a = a + out * (t / 2.0) - u * (t / 2.0);
        let len = a.distance(b + out * (t / 2.0) + u * (t / 2.0));
        let sh = t / 2.0;
        Face {
            a,
            u,
            out,
            len,
            doors: gaps
                .iter()
                .filter(|g| g.lo <= 0.0)
                .map(|g| (g.off + sh, g.off + sh + g.w, g.hi))
                .collect(),
            windows: gaps
                .iter()
                .filter(|g| g.lo > 0.0)
                .map(|g| (g.off + sh, g.off + sh + g.w, g.lo, g.hi))
                .collect(),
            clear: Vec::new(),
        }
    }

    /// Whether `s`..`e` along it keeps `m` away from its doorways.
    fn free(&self, s: f32, e: f32, m: f32) -> bool {
        s > 0.8
            && e < self.len - 0.8
            && self
                .doors
                .iter()
                .map(|d| (d.0, d.1))
                .chain(self.clear.iter().copied())
                .all(|(a, z)| e + m < a || s - m > z)
    }
}

/// How a facade is dressed.
struct Dress {
    /// Its roof line (the walls' top) and the storeys' floors above the ground.
    top: f32,
    storeys: u8,
    /// Glazed windows along the ground floor, and pilasters between the bays.
    glazed: bool,
    bays: bool,
}

/// The architecture over a building's (or tower's) outer faces: an angled plinth at the foot,
/// corner piers, pilasters between the bays, framed windows (glazed on the ground floor),
/// framed doorways, a string course at every floor, vents, and an angled cornice leaning in
/// to the roof's parapet with a neon band below it.
fn facade(b: &mut Builder, faces: &[Face], d: &Dress) {
    use Finish::{Body, Glass, Trim};
    let yb = d.top - 1.2;
    let ct = d.top + 1.7;
    for f in faces {
        let doors: Vec<(f32, f32)> = f.doors.iter().map(|d| (d.0 - 0.5, d.1 + 0.5)).collect();
        // The plinth: a sloped course at the foot, broken by the doorways.
        b.runs(
            f,
            &doors,
            [(-0.05, 0.0), (0.55, 0.0), (0.2, 0.75), (-0.05, 0.75)],
            Body,
        );
        // Corner piers.
        for (s, e) in [(0.0, 1.1), (f.len - 1.1, f.len)] {
            b.run(
                f,
                s,
                e,
                [(-0.05, 0.0), (0.4, 0.0), (0.4, yb), (-0.05, yb)],
                Body,
            );
        }
        // Doorways: jambs, a sloped lintel with a lit strip under its front.
        for &(s, e, h) in &f.doors {
            for (js, je) in [(s - 0.45, s), (e, e + 0.45)] {
                b.run(
                    f,
                    js,
                    je,
                    [(-0.05, 0.0), (0.35, 0.0), (0.35, h + 0.2), (-0.05, h + 0.2)],
                    Body,
                );
            }
            b.run(
                f,
                s - 0.6,
                e + 0.6,
                [(-0.05, h), (0.65, h), (0.65, h + 0.25), (-0.05, h + 0.75)],
                Body,
            );
            b.run(
                f,
                s,
                e,
                [(0.3, h - 0.07), (0.5, h - 0.07), (0.5, h), (0.3, h)],
                Trim,
            );
        }
        // Windows: the wall's openings, and glazed ones along the ground floor.
        let mut wins: Vec<(f32, f32, f32, f32, bool)> = f
            .windows
            .iter()
            .map(|w| (w.0, w.1, w.2, w.3, false))
            .collect();
        let mut bays = Vec::new();
        let n = ((f.len - 2.0) / 7.0) as usize;
        for i in 0..n {
            let s = 2.3 + i as f32 * 7.0;
            bays.push(s);
            if d.glazed && f.free(s - 0.4, s + 3.9, 0.4) {
                wins.push((s, s + 3.5, 1.5, 3.2, true));
            } else if d.glazed && f.free(s + 0.6, s + 2.9, 0.3) {
                // No room for a window: a vent.
                vent(b, f, s + 0.6, s + 2.9, 0.9);
            }
        }
        for &(s, e, lo, hi, glass) in &wins {
            if glass {
                b.run(
                    f,
                    s,
                    e,
                    [(-0.05, lo), (0.06, lo), (0.06, hi), (-0.05, hi)],
                    Glass,
                );
                let m = (s + e) / 2.0;
                b.run(
                    f,
                    m - 0.07,
                    m + 0.07,
                    [(-0.05, lo), (0.14, lo), (0.14, hi), (-0.05, hi)],
                    Body,
                );
            }
            for (js, je) in [(s - 0.22, s), (e, e + 0.22)] {
                b.run(
                    f,
                    js,
                    je,
                    [(-0.05, lo), (0.25, lo), (0.25, hi), (-0.05, hi)],
                    Body,
                );
            }
            // A sloped sill, and a hood over it with a lit strip underneath.
            b.run(
                f,
                s - 0.4,
                e + 0.4,
                [
                    (-0.05, lo - 0.3),
                    (0.42, lo - 0.3),
                    (0.42, lo - 0.14),
                    (-0.05, lo),
                ],
                Body,
            );
            b.run(
                f,
                s - 0.4,
                e + 0.4,
                [(-0.05, hi), (0.5, hi), (0.5, hi + 0.12), (-0.05, hi + 0.5)],
                Body,
            );
            b.run(
                f,
                s,
                e,
                [(0.28, hi - 0.06), (0.44, hi - 0.06), (0.44, hi), (0.28, hi)],
                Trim,
            );
        }
        // Pilasters between the bays, their tops sloping out under the cornice.
        if d.bays {
            for &s in &bays {
                let m = s - 1.75;
                if m > 2.0 && f.free(m - 0.45, m + 0.45, 0.3) {
                    b.run(
                        f,
                        m - 0.45,
                        m + 0.45,
                        [
                            (-0.05, 0.5),
                            (0.45, 0.5),
                            (0.45, yb - 0.75),
                            (-0.05, yb - 0.1),
                        ],
                        Body,
                    );
                }
            }
        }
        // A string course at every floor above the ground, a neon line under its lip.
        for k in 1..d.storeys {
            let y = k as f32 * STOREY;
            b.runs(
                f,
                &[],
                [
                    (-0.05, y - 0.3),
                    (0.3, y - 0.15),
                    (0.3, y + 0.08),
                    (-0.05, y + 0.08),
                ],
                Body,
            );
            b.runs(
                f,
                &[],
                [(0.3, y - 0.12), (0.34, y - 0.12), (0.34, y), (0.3, y)],
                Trim,
            );
        }
        // The cornice: leaning in from its lip to the parapet's top, a neon band across its
        // slope and another under it.
        let slope = |y: f32| 1.35 - 1.1 * (y - yb) / (ct - yb);
        b.runs(
            f,
            &f.clear,
            [(-0.05, yb), (1.35, yb), (0.25, ct), (-0.05, ct)],
            Body,
        );
        let (y0, y1) = (yb + 0.45, yb + 0.7);
        b.runs(
            f,
            &f.clear,
            [
                (slope(y0) - 0.02, y0),
                (slope(y0) + 0.04, y0),
                (slope(y1) + 0.04, y1),
                (slope(y1) - 0.02, y1),
            ],
            Trim,
        );
        b.runs(
            f,
            &f.clear,
            [
                (-0.05, yb - 0.3),
                (0.32, yb - 0.3),
                (0.32, yb - 0.16),
                (-0.05, yb - 0.16),
            ],
            Trim,
        );
    }
}

/// A vent grille on face `f` from `s` to `e`, its foot at `y`: a frame and slats.
fn vent(b: &mut Builder, f: &Face, s: f32, e: f32, y: f32) {
    use Finish::Body;
    b.run(
        f,
        s,
        e,
        [(-0.05, y), (0.12, y), (0.12, y + 1.1), (-0.05, y + 1.1)],
        Body,
    );
    for k in 0..4 {
        let h = y + 0.18 + k as f32 * 0.24;
        b.run(
            f,
            s + 0.12,
            e - 0.12,
            [
                (0.1, h),
                (0.26, h - 0.05),
                (0.26, h + 0.06),
                (0.1, h + 0.12),
            ],
            Body,
        );
    }
}

/// Which quarter turn the site's facing comes to.
fn quarter(site: &BaseSite) -> u8 {
    ((site.facing / std::f32::consts::FRAC_PI_2).round() as i32).rem_euclid(4) as u8
}

/// Half the compound's (the tower's) width.
fn half_of(kind: BaseKind) -> f32 {
    match kind {
        BaseKind::Outpost => 64.0,
        BaseKind::Center => 88.0,
        BaseKind::Home => 100.0,
        BaseKind::Tower => TOWER_HALF,
    }
}

/// The compound's vehicle gateways, front and back (none for a tower): the middle of each on
/// the wall's outer face, and which way is out.
pub fn gateways(site: &BaseSite) -> Vec<(Vec2, Vec2)> {
    if site.kind == BaseKind::Tower {
        return Vec::new();
    }
    let (q, half) = (quarter(site), half_of(site.kind));
    [-1.0f32, 1.0]
        .iter()
        .map(|&s| {
            let out = rot(q, Vec2::new(0.0, s));
            (site.pos + out * half, out)
        })
        .collect()
}

/// The structures of the site, its pad at height `y`.
pub fn layout(site: &BaseSite, y: f32, seed: u32) -> BaseLayout {
    let q = quarter(site);
    let half = half_of(site.kind);
    let origin = Vec3::new(site.pos.x, y, site.pos.y);
    let mut b = Builder {
        origin,
        q,
        out: BaseLayout {
            kind: site.kind,
            center: origin,
            quarter: q,
            half,
            wall_h: WALL_H,
            colliders: Vec::new(),
            styles: Vec::new(),
            shields: Vec::new(),
            gate_fields: Vec::new(),
            spawns: Vec::new(),
            console: None,
            terminals: Vec::new(),
            pads: Vec::new(),
            rearm: Vec::new(),
            turrets: Vec::new(),
            generator: None,
            silo: None,
            lamps: Vec::new(),
            details: Vec::new(),
            holes: Vec::new(),
            building: (Vec3::ZERO, Vec3::ZERO),
            storeys: 1,
            to_console: Vec::new(),
            from_spawn: Vec::new(),
        },
    };
    let mut rng = Seq::new(seed as u64, 0xba5e);
    if site.kind == BaseKind::Tower {
        tower(&mut b);
        return b.out;
    }
    let storeys = match site.kind {
        BaseKind::Outpost if rng.chance(0.45) => 1,
        _ => 2,
    };
    b.out.storeys = storeys;
    compound(&mut b, half, site.kind);
    building(&mut b, storeys, site.kind != BaseKind::Home, &mut rng);
    // Paths (base frame): the front gateway, the building's front door, the hall, the doors.
    let c = BLD_C;
    let (z0, z1) = (c.y - BLD_HALF.y, c.y + BLD_HALF.y);
    let (x0, x1) = (c.x - BLD_HALF.x, c.x + BLD_HALF.x);
    let inner = half - WALL_T;
    let gate_out = (0.0, -half - 8.0);
    let gate_in = (0.0, -inner + 5.0);
    let door_out = (0.0, z0 - 3.0);
    let door_in = (0.0, z0 + 2.5);
    let hall = (x1 - 16.0, z1 - 15.0);
    let core_door = (x1 - 7.0, z1 - 12.0 - 1.5);
    let core_in = (x1 - 7.0, z1 - 12.0 + 2.0);
    let console_front = (x1 - 7.0, z1 - 5.0 - 1.6);
    b.out.to_console = b.pts(
        &[
            gate_out,
            gate_in,
            door_out,
            door_in,
            hall,
            core_door,
            core_in,
            console_front,
        ],
        0.0,
    );
    let sdoor_x = x0 + 6.0;
    let sz = z1 - 11.0;
    b.out.from_spawn = b.pts(
        &[
            (sdoor_x, sz + 2.0),
            (sdoor_x, sz - 2.0),
            door_in,
            door_out,
            gate_in,
            gate_out,
        ],
        0.0,
    );
    // The yard: vehicle pads with their terminals, rearm pads, the silo.
    type Spots = &'static [(f32, f32)];
    let (pads, rearm, silo): (Spots, Spots, (f32, f32)) = match site.kind {
        BaseKind::Outpost => (&[(-34.0, -36.0)], &[(34.0, -36.0)], (42.0, 34.0)),
        _ => (
            &[(-40.0, -46.0), (40.0, 46.0)],
            &[(42.0, -46.0), (-42.0, 46.0)],
            (58.0, -4.0),
        ),
    };
    for &(x, z) in pads {
        let i = b.out.pads.len();
        let at = b.p(x, 0.0, z);
        let yaw = b.yaw(0.0);
        b.out.pads.push((at, yaw));
        // The terminal beside the pad, facing the pad.
        let tx = x + if x < 0.0 { 11.0 } else { -11.0 };
        let pos = b.p(tx, 0.0, z);
        let yaw = b.yaw(if x < 0.0 {
            -std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        });
        b.out.terminals.push(Terminal {
            kind: TerminalKind::Vehicle,
            pos,
            yaw,
            pad: Some(i),
        });
    }
    for &(x, z) in rearm {
        let at = b.p(x, 0.0, z);
        b.out.rearm.push(at);
    }
    let s = b.p(silo.0, 0.0, silo.1);
    b.out.silo = Some(s);
    b.out
        .colliders
        .push(round_prism(s - Vec3::Y, SILO_R, SILO_H + 1.0, 8, 0.0));
    b.out.styles.push(Style::Pillar);
    b.out
}

/// The walls with their walkways, gateways front and back with their forcefields, ramps up
/// along every wall, turrets and floodlights.
fn compound(b: &mut Builder, half: f32, kind: BaseKind) {
    let inner = half - WALL_T;
    let mid = half - WALL_T / 2.0;
    let gate = [door(half - GATEWAY / 2.0, GATEWAY, 100.0)];
    for (z, gaps) in [(-mid, &gate[..]), (mid, &gate[..])] {
        b.wall(
            Vec2::new(-half, z),
            Vec2::new(half, z),
            WALL_T,
            -2.0,
            WALL_H,
            gaps,
            Style::Wall,
        );
        let pz = if z < 0.0 { -half + 0.3 } else { half - 0.3 };
        b.wall(
            Vec2::new(-half, pz),
            Vec2::new(half, pz),
            0.6,
            WALL_H,
            WALL_H + 1.3,
            gaps,
            Style::Parapet,
        );
        // The gateway's forcefield.
        let f = b.world_box(
            Vec3::new(-GATEWAY / 2.0, 0.0, z - 0.4),
            Vec3::new(GATEWAY / 2.0, WALL_H, z + 0.4),
        );
        b.out.gate_fields.push(f);
    }
    for x in [-mid, mid] {
        b.wall(
            Vec2::new(x, -half + WALL_T),
            Vec2::new(x, half - WALL_T),
            WALL_T,
            -2.0,
            WALL_H,
            &[],
            Style::Wall,
        );
        let px = if x < 0.0 { -half + 0.3 } else { half - 0.3 };
        b.wall(
            Vec2::new(px, -half + WALL_T),
            Vec2::new(px, half - WALL_T),
            0.6,
            WALL_H,
            WALL_H + 1.3,
            &[],
            Style::Parapet,
        );
    }
    ramparts(b, half);
    // Ramps (stairs) up the inside of every wall, two on each, climbing from near the corners.
    let (a0, a1) = (-inner + 6.0, -inner + 6.0 + RAMP_LEN);
    for x in [-inner, inner - RAMP_W] {
        b.ramp(Vec3::new(x, 0.0, a0), Vec3::new(x + RAMP_W, WALL_H, a1), 0);
        b.ramp(
            Vec3::new(x, 0.0, -a1),
            Vec3::new(x + RAMP_W, WALL_H, -a0),
            2,
        );
    }
    for z in [-inner, inner - RAMP_W] {
        b.ramp(Vec3::new(a0, 0.0, z), Vec3::new(a1, WALL_H, z + RAMP_W), 1);
        b.ramp(
            Vec3::new(-a1, 0.0, z),
            Vec3::new(-a0, WALL_H, z + RAMP_W),
            3,
        );
    }
    // Turrets: the corners and beside the gateways (and the side walls' middles on the big
    // bases).
    let g = GATEWAY / 2.0 + 4.0;
    let mut mounts = vec![
        (-mid, -mid),
        (mid, -mid),
        (-mid, mid),
        (mid, mid),
        (-g, -mid),
        (g, -mid),
    ];
    if kind != BaseKind::Outpost {
        mounts.extend([(-mid, 0.0), (mid, 0.0), (-g, mid), (g, mid)]);
    }
    for (x, z) in mounts {
        let p = b.p(x, WALL_H + TURRET_RISE, z);
        b.out.turrets.push(p);
    }
    // Floodlights on the corners, lighting the yard.
    for (x, z) in [(-mid, -mid), (mid, -mid), (-mid, mid), (mid, mid)] {
        b.flood((x * 0.97, WALL_H + 3.5, z * 0.97), (x * 0.3, 0.0, z * 0.3));
    }
}

/// The compound walls' dress: sloped buttresses along the outside, a corbelled fascia under
/// the walkway's edge with a neon line along it, and tall pylons either side of the gateways,
/// inside and out.
fn ramparts(b: &mut Builder, half: f32) {
    use Finish::{Body, Trim};
    let inner = half - WALL_T;
    let g = GATEWAY / 2.0;
    let h = WALL_H;
    // Outer faces, then inner ones (front, back, left, right), the gateways midway on the
    // front and back.
    let mut faces = Vec::new();
    // (The inner faces' "out" points into the yard; no piece on them reaches a corner, where
    // the mitres would be the wrong way round.)
    for (outside, r) in [(true, half), (false, inner)] {
        let sgn = if outside { 1.0 } else { -1.0 };
        for (a, u, out, gate) in [
            (Vec2::new(-r, -r), Vec2::X, Vec2::NEG_Y, true),
            (Vec2::new(-r, r), Vec2::X, Vec2::Y, true),
            (Vec2::new(-r, -r), Vec2::Y, Vec2::NEG_X, false),
            (Vec2::new(r, -r), Vec2::Y, Vec2::X, false),
        ] {
            let len = 2.0 * r;
            let doors = if gate {
                vec![(r - g, r + g, h)]
            } else {
                Vec::new()
            };
            faces.push((
                outside,
                Face {
                    a,
                    u,
                    out: out * sgn,
                    len,
                    doors,
                    windows: Vec::new(),
                    clear: Vec::new(),
                },
            ));
        }
    }
    for (outside, f) in &faces {
        let gate: Vec<(f32, f32)> = f.doors.iter().map(|d| (d.0, d.1)).collect();
        // Pylons either side of the gateway, leaning in, banded.
        let foot = if *outside { -1.0 } else { 0.0 };
        let o = |y: f32| 1.6 - 0.5 * (y - foot) / (h + 3.0 - foot);
        for &(s, e) in &gate {
            for (ps, pe) in [(s - 2.6, s), (e, e + 2.6)] {
                b.run(
                    f,
                    ps,
                    pe,
                    [
                        (-0.05, foot),
                        (1.6, foot),
                        (o(h + 3.0), h + 3.0),
                        (-0.05, h + 3.0),
                    ],
                    Body,
                );
                b.run(
                    f,
                    ps - 0.1,
                    pe + 0.1,
                    [
                        (-0.05, h + 3.0),
                        (1.25, h + 3.0),
                        (1.0, h + 3.4),
                        (-0.05, h + 3.4),
                    ],
                    Body,
                );
                for y in [3.0, 7.0, h + 2.2] {
                    b.run(
                        f,
                        ps + 0.3,
                        pe - 0.3,
                        [
                            (o(y) - 0.02, y),
                            (o(y) + 0.04, y),
                            (o(y + 0.2) + 0.04, y + 0.2),
                            (o(y + 0.2) - 0.02, y + 0.2),
                        ],
                        Trim,
                    );
                }
            }
        }
        if !*outside {
            continue;
        }
        // Buttresses, sloping up into the corbels.
        let mut s = 10.0;
        while s < f.len - 10.0 {
            if gate
                .iter()
                .all(|&(a, e)| s + 1.0 < a - 4.0 || s - 1.0 > e + 4.0)
            {
                b.run(
                    f,
                    s - 1.0,
                    s + 1.0,
                    [
                        (-0.05, -1.0),
                        (1.8, -1.0),
                        (0.35, h - 1.6),
                        (-0.05, h - 1.0),
                    ],
                    Body,
                );
            }
            s += 16.0;
        }
        // The corbel and the fascia over it, up to the parapet's top, a neon line along it.
        b.runs(
            f,
            &gate,
            [
                (-0.05, h - 1.6),
                (0.1, h - 1.6),
                (0.7, h - 0.4),
                (-0.05, h - 0.4),
            ],
            Body,
        );
        b.runs(
            f,
            &gate,
            [
                (-0.05, h - 0.4),
                (0.7, h - 0.4),
                (0.7, h + 1.35),
                (-0.05, h + 1.35),
            ],
            Body,
        );
        b.runs(
            f,
            &gate,
            [
                (0.7, h + 0.2),
                (0.74, h + 0.2),
                (0.74, h + 0.34),
                (0.7, h + 0.34),
            ],
            Trim,
        );
    }
}

/// The main building around `BLD_C` (its front towards -Z), `storeys` high.
fn building(b: &mut Builder, storeys: u8, console: bool, rng: &mut Seq) {
    let c = BLD_C;
    let (hx, hz) = (BLD_HALF.x, BLD_HALF.y);
    let (x0, x1, z0, z1) = (c.x - hx, c.x + hx, c.y - hz, c.y + hz);
    let top = STOREY * storeys as f32;
    let middle = |len: f32| door(len / 2.0 - DOOR_W / 2.0, DOOR_W, DOOR_H);
    // Windows on the upper storey: a row of gaps from 1.2 m to 3.6 m above its floor.
    let windows = |len: f32| -> Vec<Gap> {
        let n = (len / 7.0) as usize;
        (0..n)
            .map(|i| Gap {
                off: 2.0 + i as f32 * 7.0,
                w: 3.5,
                lo: STOREY + 1.2,
                hi: STOREY + 3.6,
            })
            .filter(|g| g.off + g.w < len - 1.0)
            .collect()
    };
    // (Windows that would cut into a door are left out.)
    let with = |mut a: Vec<Gap>, b2: Vec<Gap>| {
        let keep: Vec<Gap> = b2
            .into_iter()
            .filter(|w| {
                a.iter()
                    .all(|d| w.off + w.w < d.off - 0.5 || w.off > d.off + d.w + 0.5)
            })
            .collect();
        a.extend(keep);
        a
    };
    let upper = |len: f32| {
        if storeys > 1 {
            windows(len)
        } else {
            Vec::new()
        }
    };
    // Outer walls: doors front and back, and on the right side into the generator room.
    let front = with(vec![middle(2.0 * hx)], upper(2.0 * hx));
    let back = with(vec![middle(2.0 * hx)], upper(2.0 * hx));
    let left = upper(2.0 * hz);
    let right = with(vec![door(4.0, DOOR_W, DOOR_H)], upper(2.0 * hz));
    let sides = [
        (Vec2::new(x0, z0), Vec2::new(x1, z0), Vec2::NEG_Y, &front),
        (Vec2::new(x0, z1), Vec2::new(x1, z1), Vec2::Y, &back),
        (Vec2::new(x0, z0), Vec2::new(x0, z1), Vec2::NEG_X, &left),
        (Vec2::new(x1, z0), Vec2::new(x1, z1), Vec2::X, &right),
    ];
    let mut faces = Vec::new();
    for (a, e, out, gaps) in sides {
        b.wall(a, e, T, 0.0, top, gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, gaps));
    }
    // (The roof's ramp comes up the back, left of the door.)
    faces[1]
        .clear
        .push((4.0 + T / 2.0 - 0.2, 8.0 + T / 2.0 + 0.2));
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys,
            glazed: true,
            bays: true,
        },
    );
    // The upper floor (with the stairwell over the stair ramp), and the roof.
    let stair = (x0, z0, x0 + RAMP_W + 0.5, z0 + 12.0);
    if storeys > 1 {
        b.slab((x0, z0, x1, z1), STOREY, &[stair], Style::Floor);
        b.ramp(
            Vec3::new(x0 + T / 2.0, 0.0, z0 + 1.0),
            Vec3::new(x0 + T / 2.0 + RAMP_W, STOREY, z0 + 11.0),
            0,
        );
    }
    b.slab(
        (x0 - T / 2.0, z0 - T / 2.0, x1 + T / 2.0, z1 + T / 2.0),
        top + 0.6,
        &[],
        Style::Roof,
    );
    // A parapet round the roof.
    let r = top + 0.6;
    b.wall(
        Vec2::new(x0, z0),
        Vec2::new(x1, z0),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x0, z1),
        Vec2::new(x1, z1),
        0.4,
        r,
        r + 1.0,
        &[door(4.0, 4.0, 2.0)],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x0, z0),
        Vec2::new(x0, z1),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    b.wall(
        Vec2::new(x1, z0),
        Vec2::new(x1, z1),
        0.4,
        r,
        r + 1.0,
        &[],
        Style::Parapet,
    );
    // The roof's ramp up the back (to the parapet's gap).
    let len = r * 2.0;
    b.ramp(
        Vec3::new(x0 + 4.0, 0.0, z1 + T / 2.0),
        Vec3::new(x0 + 8.0, r, z1 + T / 2.0 + len),
        2,
    );
    // On the roof: vent stacks and plant on the right (clear of the ramp's landing), an
    // antenna mast.
    b.decor(x1 - 5.0, r, z0 + 5.0, Vec3::new(4.0, 1.4, 3.0));
    b.decor(x1 - 5.0, r + 1.4, z0 + 5.0, Vec3::new(2.4, 0.6, 2.0));
    b.decor(x1 - 4.0, r, z1 - 6.0, Vec3::new(2.4, 1.0, 4.0));
    b.decor(c.x + 4.0, r, z1 - 3.0, Vec3::new(1.6, 2.2, 1.6));
    let mast = b.p(x1 - 2.5, r, c.y);
    b.out.colliders.push(round_prism(mast, 0.3, 7.0, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    // The spawn room: the back left corner, its doorway towards the hall closed by a
    // forcefield.
    let sx = x0 + 12.0;
    let sz = z1 - 11.0;
    let sdoor = door(4.5, 3.0, 3.2);
    b.wall(
        Vec2::new(x0, sz),
        Vec2::new(sx, sz),
        T,
        0.0,
        STOREY,
        &[sdoor],
        Style::Interior,
    );
    b.wall(
        Vec2::new(sx, sz),
        Vec2::new(sx, z1),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    let f0 = b.p(x0 + 4.5, 0.0, sz - 0.15);
    let f1 = b.p(x0 + 7.5, 3.2, sz + 0.15);
    b.out.shields.push(Shield {
        collider: Collider::Box {
            min: f0.min(f1),
            max: f0.max(f1),
        },
    });
    for i in 0..6 {
        let (px, pz) = (
            x0 + 2.4 + (i % 3) as f32 * 3.4,
            z1 - 2.4 - (i / 3) as f32 * 3.6,
        );
        let at = b.p(px, 0.0, pz);
        let yaw = b.yaw(0.0);
        b.out.spawns.push((at, yaw));
    }
    let r90 = std::f32::consts::FRAC_PI_2;
    b.terminal(TerminalKind::Equipment, x0 + 1.0, 0.0, sz + 2.5, -r90);
    b.terminal(TerminalKind::Medical, sx - 1.0, 0.0, z1 - 3.0, r90);
    // The core room: the back right corner, one doorway into the hall; server racks.
    let cx = x1 - 13.0;
    let cz = z1 - 12.0;
    b.wall(
        Vec2::new(cx, cz),
        Vec2::new(x1, cz),
        T,
        0.0,
        STOREY,
        &[door(4.0, 4.0, DOOR_H)],
        Style::Interior,
    );
    b.wall(
        Vec2::new(cx, cz),
        Vec2::new(cx, z1),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    if console {
        let at = b.p(x1 - 7.0, 0.0, z1 - 5.0);
        let yaw = b.yaw(0.0);
        b.out.console = Some((at, yaw));
        b.out.colliders.push(round_prism(at, 0.6, 1.1, 6, 0.0));
        b.out.styles.push(Style::Pillar);
    }
    for k in 0..3 {
        b.decor(
            x1 - 1.0,
            0.0,
            cz + 3.0 + k as f32 * 2.6,
            Vec3::new(1.2, 2.4, 2.0),
        );
    }
    b.decor(cx + 1.2, 0.0, z1 - 2.0, Vec3::new(1.2, 2.4, 2.0));
    // The generator room: the front right corner, a doorway from the hall and the outer door.
    let gx = x1 - 11.0;
    let gz = z0 + 10.0;
    b.wall(
        Vec2::new(gx, z0),
        Vec2::new(gx, gz),
        T,
        0.0,
        STOREY,
        &[door(4.0, 3.5, DOOR_H)],
        Style::Interior,
    );
    b.wall(
        Vec2::new(gx, gz),
        Vec2::new(x1, gz),
        T,
        0.0,
        STOREY,
        &[],
        Style::Interior,
    );
    let gen_at = b.p(x1 - 5.5, 0.0, z0 + 6.0);
    b.out.generator = Some(gen_at);
    let gh = GENERATOR_HALF;
    let gen_c = b.world_box(
        Vec3::new(x1 - 5.5 - gh.x, 0.0, z0 + 6.0 - gh.z),
        Vec3::new(x1 - 5.5 + gh.x, gh.y * 2.0, z0 + 6.0 + gh.z),
    );
    b.out.colliders.push(gen_c);
    b.out.styles.push(Style::Decor);
    // The hall: terminals, pillars, crates.
    b.terminal(TerminalKind::Equipment, x0 + 1.0, 0.0, z0 + 14.0, -r90);
    b.terminal(TerminalKind::Medical, x0 + 1.0, 0.0, z0 + 17.5, -r90);
    for (px, pz) in [(c.x - 6.0, c.y - 1.0), (c.x + 4.0, c.y - 1.0)] {
        let p = b.p(px, 0.0, pz);
        b.out.colliders.push(round_prism(p, 0.5, STOREY, 8, 0.0));
        b.out.styles.push(Style::Pillar);
    }
    for _ in 0..4 {
        // Crates stacked against the walls (out of the paths).
        let (x, z) = match rng.int(0, 3) {
            0 => (rng.range(x0 + 6.0, cx - 2.0), z0 + 1.2),
            1 => (rng.range(sx + 2.0, cx - 2.0), z1 - 1.2),
            _ => (gx - 1.5, rng.range(gz + 1.5, cz - 1.5)),
        };
        let s = rng.range(1.0, 1.8);
        b.decor(x, 0.0, z, Vec3::splat(s));
        if rng.chance(0.5) {
            b.decor(x, s, z, Vec3::splat(s * 0.8));
        }
    }
    b.lamp(c.x - 3.0, STOREY - 0.6, c.y - 4.0);
    b.lamp(x0 + 6.0, STOREY - 0.6, z1 - 5.0);
    b.lamp(x1 - 7.0, STOREY - 0.6, z1 - 6.0);
    b.lamp(x1 - 5.5, STOREY - 0.6, z0 + 5.0);
    if storeys > 1 {
        // Upstairs: offices behind a partition, desks and consoles.
        let y = STOREY;
        let px = c.x + 2.0;
        b.wall(
            Vec2::new(px, z0),
            Vec2::new(px, z1),
            T,
            y,
            y + STOREY - SLAB,
            &[door(10.0, 3.0, DOOR_H), door(20.0, 3.0, DOOR_H)],
            Style::Interior,
        );
        b.wall(
            Vec2::new(px, c.y + 2.0),
            Vec2::new(x1, c.y + 2.0),
            T,
            y,
            y + STOREY - SLAB,
            &[door(5.0, 3.0, DOOR_H)],
            Style::Interior,
        );
        for k in 0..3 {
            b.decor(
                px + 4.0 + k as f32 * 4.0,
                y,
                z0 + 4.0,
                Vec3::new(2.4, 0.9, 1.0),
            );
            b.decor(
                px + 4.0 + k as f32 * 4.0,
                y,
                z1 - 4.0,
                Vec3::new(2.4, 0.9, 1.0),
            );
        }
        b.decor(x0 + 10.0, y, z1 - 1.0, Vec3::new(4.0, 2.2, 1.0));
        b.decor(x0 + 14.0, y, c.y, Vec3::new(1.4, 1.4, 1.4));
        b.terminal(TerminalKind::Equipment, x0 + 1.0, y, z1 - 4.0, -r90);
        b.lamp(c.x - 8.0, top - 0.6, c.y);
        b.lamp(c.x + 10.0, top - 0.6, z0 + 6.0);
        b.lamp(c.x + 10.0, top - 0.6, z1 - 6.0);
    }
    let a = b.p(x0, 0.0, z0);
    let e = b.p(x1, top, z1);
    b.out.building = (a.min(e), a.max(e));
}

/// A tower: three storeys up ramps inside, the console on the top floor, turrets on the roof
/// and the spawn bunker under the ground.
fn tower(b: &mut Builder) {
    let h = TOWER_HALF;
    let top = STOREY * 3.0;
    let r90 = std::f32::consts::FRAC_PI_2;
    // Outer walls: a door front and back on the ground floor, slits above.
    let slits = |len: f32| -> Vec<Gap> {
        let mut g = vec![door(len / 2.0 - 2.0, 4.0, DOOR_H)];
        for k in 1..3 {
            g.push(Gap {
                off: 3.0,
                w: 2.0,
                lo: k as f32 * STOREY + 1.2,
                hi: k as f32 * STOREY + 2.6,
            });
            g.push(Gap {
                off: len - 5.0,
                w: 2.0,
                lo: k as f32 * STOREY + 1.2,
                hi: k as f32 * STOREY + 2.6,
            });
        }
        g
    };
    let (fb, sides) = (slits(2.0 * h), Vec::new());
    let mut faces = Vec::new();
    for (a, e, out, gaps) in [
        (Vec2::new(-h, -h), Vec2::new(h, -h), Vec2::NEG_Y, &fb),
        (Vec2::new(-h, h), Vec2::new(h, h), Vec2::Y, &fb),
        (Vec2::new(-h, -h), Vec2::new(-h, h), Vec2::NEG_X, &sides),
        (Vec2::new(h, -h), Vec2::new(h, h), Vec2::X, &sides),
    ] {
        b.wall(a, e, T, 0.0, top, gaps, Style::Building);
        faces.push(Face::of(a, e, T, out, gaps));
    }
    facade(
        b,
        &faces,
        &Dress {
            top,
            storeys: 3,
            glazed: false,
            bays: false,
        },
    );
    // The blank sides: a vent low down and a conduit up to the cornice.
    for f in &faces[2..] {
        vent(b, f, f.len / 2.0 - 1.4, f.len / 2.0 + 1.4, 1.0);
        let m = f.len / 2.0 + 3.2;
        b.run(
            f,
            m - 0.18,
            m + 0.18,
            [
                (-0.05, 0.6),
                (0.36, 0.6),
                (0.36, top - 1.5),
                (-0.05, top - 1.5),
            ],
            Finish::Body,
        );
        for y in [2.0, 6.0, 10.0] {
            b.run(
                f,
                m - 0.26,
                m + 0.26,
                [(-0.05, y), (0.44, y), (0.44, y + 0.25), (-0.05, y + 0.25)],
                Finish::Body,
            );
        }
    }
    // The apron round its foot (over the land's cut-out round the bunker).
    let (w, a) = (h + T / 2.0, h + 5.5);
    for (min, max) in [
        (Vec3::new(-a, -0.5, -a), Vec3::new(a, 0.02, -w)),
        (Vec3::new(-a, -0.5, w), Vec3::new(a, 0.02, a)),
        (Vec3::new(-a, -0.5, -w), Vec3::new(-w, 0.02, w)),
        (Vec3::new(w, -0.5, -w), Vec3::new(a, 0.02, w)),
    ] {
        b.boxed(min, max, Style::Floor);
    }
    // Stairwells: over each ramp, ending where the ramp reaches the floor above.
    let left = (-h, -7.0, -h + RAMP_W + 0.5, 4.0);
    let right = (h - RAMP_W - 0.5, -4.0, h, 7.0);
    let down = (h - RAMP_W - 0.5, -6.0, h, 6.0);
    // The ground floor over the bunker (the stairwell down on the right).
    b.slab((-h, -h, h, h), 0.02, &[down], Style::Floor);
    // Floors 1 and 2 and the roof, each with its stairwell.
    b.slab((-h, -h, h, h), STOREY, &[left], Style::Floor);
    b.slab((-h, -h, h, h), STOREY * 2.0, &[right], Style::Floor);
    b.slab(
        (-h - T / 2.0, -h - T / 2.0, h + T / 2.0, h + T / 2.0),
        top + 0.6,
        &[left],
        Style::Roof,
    );
    let (lx0, lx1) = (-h + T / 2.0, -h + T / 2.0 + RAMP_W);
    let (rx0, rx1) = (h - T / 2.0 - RAMP_W, h - T / 2.0);
    b.ramp(Vec3::new(lx0, 0.0, -6.0), Vec3::new(lx1, STOREY, 4.0), 0);
    b.ramp(
        Vec3::new(rx0, STOREY, -4.0),
        Vec3::new(rx1, STOREY * 2.0, 6.0),
        2,
    );
    b.ramp(
        Vec3::new(lx0, STOREY * 2.0, -6.0),
        Vec3::new(lx1, top + 0.6, 4.0),
        0,
    );
    // Roof parapet, two turrets on its corners.
    let r = top + 0.6;
    for (a, e) in [((-h, -h), (h, -h)), ((-h, h), (h, h))] {
        b.wall(
            Vec2::new(a.0, a.1),
            Vec2::new(e.0, e.1),
            0.4,
            r,
            r + 1.0,
            &[],
            Style::Parapet,
        );
    }
    for x in [-h, h] {
        b.wall(
            Vec2::new(x, -h),
            Vec2::new(x, h),
            0.4,
            r,
            r + 1.0,
            &[],
            Style::Parapet,
        );
    }
    for (x, z) in [(h - 2.5, -h + 2.5), (-h + 2.5, h - 2.5)] {
        let p = b.p(x, r + TURRET_RISE, z);
        b.out.turrets.push(p);
    }
    // The console: the top floor, by the far wall.
    let y2 = STOREY * 2.0;
    let at = b.p(0.0, y2, 4.5);
    let yaw = b.yaw(0.0);
    b.out.console = Some((at, yaw));
    b.out.colliders.push(round_prism(at, 0.6, 1.1, 6, 0.0));
    b.out.styles.push(Style::Pillar);
    b.decor(-4.0, y2, 6.5, Vec3::new(2.0, 1.0, 1.0));
    b.decor(3.0, STOREY, -6.5, Vec3::new(1.5, 1.5, 1.5));
    // The bunker: walls, floor, the ramp down, a forcefield at its foot.
    let d = -STOREY;
    b.slab((-h, -h, h, h), d, &[], Style::Floor);
    for (a, e) in [((-h, -h), (h, -h)), ((-h, h), (h, h))] {
        b.wall(
            Vec2::new(a.0, a.1),
            Vec2::new(e.0, e.1),
            T,
            d - 1.0,
            0.0,
            &[],
            Style::Interior,
        );
    }
    for x in [-h, h] {
        b.wall(
            Vec2::new(x, -h),
            Vec2::new(x, h),
            T,
            d - 1.0,
            0.0,
            &[],
            Style::Interior,
        );
    }
    b.ramp(Vec3::new(rx0, d, -6.0), Vec3::new(rx1, 0.0, 5.0), 2);
    let f0 = b.p(rx0, d, 5.2);
    let f1 = b.p(rx1, d + 3.0, 5.5);
    b.out.shields.push(Shield {
        collider: Collider::Box {
            min: f0.min(f1),
            max: f0.max(f1),
        },
    });
    for (px, pz) in [(-4.5, -3.5), (-4.5, 1.0), (-1.0, -3.5), (-1.0, 1.0)] {
        let at = b.p(px, d, pz);
        let yaw = b.yaw(0.0);
        b.out.spawns.push((at, yaw));
    }
    b.terminal(TerminalKind::Equipment, -h + 1.0, d, 5.5, -r90);
    b.terminal(
        TerminalKind::Medical,
        -h + 4.0,
        d,
        h - 1.0,
        std::f32::consts::PI,
    );
    b.lamp(0.0, -0.8, 0.0);
    b.lamp(0.0, STOREY - 0.6, 0.0);
    b.lamp(0.0, STOREY * 2.0 - 0.6, 0.0);
    b.lamp(0.0, top - 0.6, 0.0);
    let hole = {
        let a = b.p(-h - 0.5, 0.0, -h - 0.5);
        let e = b.p(h + 0.5, 0.0, h + 0.5);
        (
            Vec2::new(a.x.min(e.x), a.z.min(e.z)),
            Vec2::new(a.x.max(e.x), a.z.max(e.z)),
        )
    };
    b.out.holes.push(hole);
    b.out.storeys = 3;
    let a = b.p(-h, d, -h);
    let e = b.p(h, top, h);
    b.out.building = (a.min(e), a.max(e));
    // Bots: in through the front door, up the ramps to the console; out of the bunker.
    let ramp_l = (lx0 + RAMP_W / 2.0, 0.0);
    let rr = rx0 + RAMP_W / 2.0;
    b.out.to_console = vec![
        b.p(0.0, 0.0, -h - 5.0),
        b.p(0.0, 0.0, -h + 1.5),
        b.p(ramp_l.0, 0.0, -7.0),
        b.p(ramp_l.0, STOREY, 5.5),
        b.p(rr, STOREY, 7.0),
        b.p(rr, y2, -5.5),
        b.p(1.0, y2, -6.0),
        b.p(0.0, y2, 2.8),
    ];
    b.out.from_spawn = vec![
        b.p(-2.0, d, 6.5),
        b.p(rr, d, 6.5),
        b.p(rr, 0.0, -6.8),
        b.p(0.0, 0.0, -h + 1.5),
        b.p(0.0, 0.0, -h - 5.0),
    ];
}

/// The warp gate's pillars around its middle `at`.
pub fn gate_colliders(at: Vec3) -> Vec<Collider> {
    (0..GATE_PILLARS)
        .map(|i| {
            let a = i as f32 / GATE_PILLARS as f32 * std::f32::consts::TAU + 0.26;
            let p = at + Vec3::new(a.cos(), 0.0, a.sin()) * GATE_RING;
            round_prism(p - Vec3::Y, 2.4, GATE_H + 1.0, 6, a)
        })
        .collect()
}

/// The warp gate's ring of pillars (count, radius, height) and its core: what enters the core
/// (within `GATE_CORE` of the middle, below `GATE_H`) is warped.
pub const GATE_PILLARS: usize = 8;
pub const GATE_RING: f32 = 40.0;
pub const GATE_H: f32 = 55.0;
pub const GATE_CORE: f32 = 14.0;
/// The gate's bubble (the shell over it: an ellipsoid this wide and high): nobody fires inside
/// it, and no shot goes through it either way.
pub const GATE_BUBBLE_R: f32 = 38.0;
pub const GATE_BUBBLE_H: f32 = 48.0;

/// Whether `p` is inside the bubble of the gate whose middle is `gate`.
pub fn in_gate_bubble(gate: Vec3, p: Vec3) -> bool {
    let d = p - gate;
    if d.y < -4.0 {
        return false;
    }
    let (x, y, z) = (
        d.x / GATE_BUBBLE_R,
        d.y.max(0.0) / GATE_BUBBLE_H,
        d.z / GATE_BUBBLE_R,
    );
    x * x + y * y + z * z < 1.0
}

impl BaseLayout {
    /// Whether `p` is inside the compound's walls (a tower's footprint).
    pub fn inside(&self, p: Vec3) -> bool {
        (p.x - self.center.x).abs() < self.half && (p.z - self.center.z).abs() < self.half
    }

    /// Whether `p` is inside the main building (the tower).
    pub fn in_building(&self, p: Vec3) -> bool {
        let (a, b) = self.building;
        p.x > a.x && p.x < b.x && p.z > a.z && p.z < b.z && p.y < b.y + 0.5 && p.y > a.y - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::World;

    #[test]
    fn spawns_are_free_and_walk_out_of_the_building() {
        for (kind, facing) in [
            (BaseKind::Outpost, 0.0),
            (BaseKind::Center, 1.6),
            (BaseKind::Home, 3.1),
            (BaseKind::Outpost, -1.5),
            (BaseKind::Tower, 1.6),
        ] {
            for seed in 0..4 {
                let site = BaseSite {
                    name: "t".into(),
                    kind,
                    pos: Vec2::new(100.0, -50.0),
                    facing,
                };
                let l = layout(&site, 10.0, seed);
                assert_eq!(l.colliders.len(), l.styles.len());
                let mut cs = l.colliders.clone();
                if kind != BaseKind::Tower {
                    cs.push(Collider::Box {
                        min: Vec3::new(-500.0, 9.0, -500.0),
                        max: Vec3::new(500.0, 10.0, 500.0),
                    });
                }
                let w = World::new(cs, None);
                for &(s, _) in &l.spawns {
                    let mut p = s;
                    assert!(
                        !w.colliders
                            .iter()
                            .any(|c| crate::collide::push_out(c, &mut p)),
                        "{kind:?} spawn {s} blocked"
                    );
                    assert!(l.in_building(s), "{kind:?} spawn {s} outside");
                    // Something to stand on.
                    let g = w.ground(s.x, s.z, s.y + 0.2).unwrap();
                    assert!((g - s.y).abs() < 0.05, "{kind:?} spawn {s} floor {g}");
                }
                assert_eq!(l.console.is_some(), kind != BaseKind::Home);
                assert!(
                    l.terminals
                        .iter()
                        .any(|t| t.kind == TerminalKind::Equipment)
                );
                if kind != BaseKind::Tower {
                    assert!(l.terminals.iter().any(|t| t.kind == TerminalKind::Vehicle));
                    assert!(l.generator.is_some() && l.silo.is_some());
                    assert_eq!(l.gate_fields.len(), 2);
                } else {
                    assert_eq!(l.holes.len(), 1);
                }
            }
        }
    }

    #[test]
    fn gateways_are_where_the_forcefields_are() {
        for q in 0..4 {
            let site = BaseSite {
                name: "t".into(),
                kind: BaseKind::Outpost,
                pos: Vec2::new(100.0, -50.0),
                facing: q as f32 * std::f32::consts::FRAC_PI_2,
            };
            let l = layout(&site, 0.0, 1);
            let gw = gateways(&site);
            assert_eq!(gw.len(), 2);
            for f in &l.gate_fields {
                let Collider::Box { min, max } = *f else {
                    panic!()
                };
                let mid = Vec2::new(min.x + max.x, min.z + max.z) / 2.0;
                // Each forcefield just inside a gateway, the way out away from the middle.
                assert!(
                    gw.iter()
                        .any(|(at, out)| at.distance(mid) < WALL_T
                            && (*at - site.pos).dot(*out) > 0.0),
                    "{q}: {mid} {gw:?}"
                );
            }
        }
    }

    #[test]
    fn bots_paths_reach_the_console() {
        // Walking each path point to point gets to the console (up the ramps in a tower).
        for kind in [BaseKind::Outpost, BaseKind::Center, BaseKind::Tower] {
            let site = BaseSite {
                name: "t".into(),
                kind,
                pos: Vec2::ZERO,
                facing: 0.0,
            };
            let l = layout(&site, 0.0, 1);
            let mut cs = l.colliders.clone();
            if kind == BaseKind::Tower {
                // (The land around the tower; the bunker's hole in it.)
                for (a, e) in [
                    (Vec2::new(-300.0, -300.0), Vec2::new(300.0, -8.5)),
                    (Vec2::new(-300.0, 8.5), Vec2::new(300.0, 300.0)),
                    (Vec2::new(-300.0, -8.5), Vec2::new(-8.5, 8.5)),
                    (Vec2::new(8.5, -8.5), Vec2::new(300.0, 8.5)),
                ] {
                    cs.push(Collider::Box {
                        min: Vec3::new(a.x, -1.0, a.y),
                        max: Vec3::new(e.x, 0.0, e.y),
                    });
                }
            } else {
                cs.push(Collider::Box {
                    min: Vec3::new(-300.0, -1.0, -300.0),
                    max: Vec3::new(300.0, 0.0, 300.0),
                });
            }
            let w = World::new(cs, None);
            let mut feet = l.to_console[0];
            let mut vy = 0.0;
            for &target in &l.to_console[1..] {
                for _ in 0..1200 {
                    let d = Vec2::new(target.x - feet.x, target.z - feet.z);
                    if d.length() < 0.4 {
                        break;
                    }
                    let step = d.normalize() * (5.0f32 / 60.0).min(d.length());
                    (feet, vy) = w.move_body(feet, step, vy, 1.0 / 60.0);
                }
                assert!(
                    Vec2::new(target.x - feet.x, target.z - feet.z).length() < 0.6,
                    "{kind:?}: stuck at {feet} on the way to {target}"
                );
            }
            let (console, _) = l.console.unwrap();
            assert!(
                (console.y - feet.y).abs() < 0.3,
                "{kind:?}: at {feet}, console {console}"
            );
            assert!(
                (console - feet).length() < USE_REACH,
                "{kind:?}: {feet} vs {console}"
            );
        }
    }
}
