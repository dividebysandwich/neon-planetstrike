//! The world: continents linked by warp gates, each with bases joined in a lattice. All of it
//! follows from the server's seed (`WorldDef::generate`), so clients rebuild it from the seed
//! the server sends.
//!
//! - Three sanctuaries, one per faction, each with one home base and two warp gates to two
//!   different contested continents.
//! - `contested` continents with two or three warp gates each, on a planet map (a disc) where
//!   gates link near neighbours without crossing links.
//! - On each contested continent a center base and 5–8 outer bases, in a lattice (a relative
//!   neighbourhood graph); each warp gate links to its nearest outer bases. Roads follow the
//!   lattice (`road_links`, built into the land by `roads.rs`).

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::noise::Seq;
use crate::roads::{RoadEnd, RoadLink};
use crate::terrain::{Pad, Terrain};
use crate::theme::Theme;
use crate::types::Team;

pub type ContinentId = u8;
/// A base's index on its continent.
pub type BaseIdx = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldParams {
    pub seed: u64,
    /// Contested continents (besides the three sanctuaries).
    pub contested: u8,
}

impl Default for WorldParams {
    fn default() -> Self {
        WorldParams {
            seed: 1,
            contested: 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseKind {
    /// An outer base.
    Outpost,
    /// The continent's center base: whoever holds it holds the continent.
    Center,
    /// A sanctuary's home base (never captured).
    Home,
    /// A tower near a base: no lattice, captured at once by solving its console, a spawn
    /// bunker under it.
    Tower,
}

/// What an outpost is besides a base: holding one (its generator up) gives its faction a
/// benefit on the continent (`ContinentSim::benefit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Facility {
    /// Heavy tanks, interceptors and dropships may be ordered.
    TechPlant,
    /// Respawning takes half as long.
    BioLab,
    /// The faction's wall turrets take half damage and fire faster.
    AmpStation,
    /// Enemies near the faction's bases show on its minimaps.
    Interlink,
}

impl Facility {
    pub const ALL: [Facility; 4] = [
        Facility::TechPlant,
        Facility::BioLab,
        Facility::AmpStation,
        Facility::Interlink,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Facility::TechPlant => "Tech Plant",
            Facility::BioLab => "Bio Lab",
            Facility::AmpStation => "Amp Station",
            Facility::Interlink => "Interlink",
        }
    }

    pub fn benefit(self) -> &'static str {
        match self {
            Facility::TechPlant => "heavy tanks, interceptors and dropships",
            Facility::BioLab => "respawning twice as fast",
            Facility::AmpStation => "tougher, faster-firing turrets",
            Facility::Interlink => "enemies near your bases on the minimap",
        }
    }
}

impl ContinentDef {
    /// Base `b`'s facility: the outposts take the four in turn (from a start that differs by
    /// continent); the center, towers and sanctuaries have none.
    pub fn facility(&self, b: BaseIdx) -> Option<Facility> {
        if self.bases.get(b as usize)?.kind != BaseKind::Outpost {
            return None;
        }
        let k = self.bases[..b as usize]
            .iter()
            .filter(|s| s.kind == BaseKind::Outpost)
            .count();
        Some(Facility::ALL[(k + self.id as usize) % Facility::ALL.len()])
    }
}

/// How a base is taken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capture {
    /// Solving the console in its core room (a hack that runs out, at a tower at once).
    #[default]
    Console,
    /// Three capture points (A, B, C): a faction holding two of them takes it when its
    /// capture runs out (`crate::capture`).
    Points,
}

#[derive(Clone, Debug)]
pub struct BaseSite {
    pub name: String,
    pub kind: BaseKind,
    pub pos: Vec2,
    /// Which way the main building's front faces (radians about +Y, 0 = facing -Z).
    pub facing: f32,
    pub capture: Capture,
}

impl BaseSite {
    /// Radius of the flat ground the base stands on.
    pub fn pad_radius(&self) -> f32 {
        match self.kind {
            BaseKind::Outpost => 96.0,
            BaseKind::Center => 130.0,
            BaseKind::Home => 148.0,
            BaseKind::Tower => 20.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gate {
    pub pos: Vec2,
    /// The continent and gate (index there) it leads to.
    pub to: ContinentId,
    pub to_gate: u8,
}

pub const GATE_PAD: f32 = 88.0;

/// A lattice node: a base or a warp gate on the same continent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Node {
    Base(BaseIdx),
    Gate(u8),
}

#[derive(Clone, Debug)]
pub struct ContinentDef {
    pub id: ContinentId,
    pub name: String,
    pub theme: Theme,
    /// The faction whose sanctuary this is.
    pub sanctuary: Option<Team>,
    /// Where it sits on the planet map (unit disc).
    pub map_pos: Vec2,
    pub bases: Vec<BaseSite>,
    pub gates: Vec<Gate>,
    pub lattice: Vec<(Node, Node)>,
    /// The terrain's seed.
    pub terrain_seed: u32,
}

impl ContinentDef {
    pub fn center_base(&self) -> Option<BaseIdx> {
        self.bases
            .iter()
            .position(|b| matches!(b.kind, BaseKind::Center | BaseKind::Home))
            .map(|i| i as BaseIdx)
    }

    /// The lattice neighbours of `node`.
    pub fn neighbours(&self, node: Node) -> impl Iterator<Item = Node> + '_ {
        self.lattice.iter().filter_map(move |&(a, b)| {
            if a == node {
                Some(b)
            } else if b == node {
                Some(a)
            } else {
                None
            }
        })
    }

    /// The flat pads under the bases and gates (bases first, in order).
    pub fn pads(&self) -> Vec<Pad> {
        self.bases
            .iter()
            .map(|b| Pad {
                center: b.pos,
                radius: b.pad_radius(),
                blend: if b.kind == BaseKind::Tower {
                    30.0
                } else {
                    90.0
                },
            })
            .chain(self.gates.iter().map(|g| Pad {
                center: g.pos,
                radius: GATE_PAD,
                blend: 70.0,
            }))
            .collect()
    }

    /// Where a lattice node stands.
    pub fn node_pos(&self, node: Node) -> Vec2 {
        match node {
            Node::Base(b) => self.bases[b as usize].pos,
            Node::Gate(g) => self.gates[g as usize].pos,
        }
    }

    /// The roads to build: one along each lattice link, from a gateway of each base (the one
    /// facing the other end) and into each warp gate between its pillars.
    pub fn road_links(&self) -> Vec<RoadLink> {
        let pads = self.pads();
        let end = |node: Node, towards: Vec2| -> RoadEnd {
            match node {
                Node::Base(b) => {
                    let site = &self.bases[b as usize];
                    let to = (towards - site.pos).normalize_or_zero();
                    let (at, out) = crate::base::gateways(site)
                        .into_iter()
                        .max_by(|x, y| x.1.dot(to).total_cmp(&y.1.dot(to)))
                        .unwrap_or((site.pos, to));
                    // Out to just beyond the pad's edge.
                    let radius = pads[b as usize].radius;
                    RoadEnd {
                        at,
                        out,
                        reach: radius + 12.0 - at.distance(site.pos),
                    }
                }
                Node::Gate(g) => {
                    let c = self.gates[g as usize].pos;
                    let to = towards - c;
                    let a = to.y.atan2(to.x);
                    // Through the gap between pillars nearest that way.
                    let n = crate::base::GATE_PILLARS as f32;
                    let first = 0.26 + std::f32::consts::PI / n;
                    let step = std::f32::consts::TAU / n;
                    let a = first + ((a - first) / step).round() * step;
                    let out = Vec2::new(a.cos(), a.sin());
                    let core = crate::base::GATE_CORE + 4.0;
                    RoadEnd {
                        at: c + out * core,
                        out,
                        reach: GATE_PAD + 12.0 - core,
                    }
                }
            }
        };
        let mut links: Vec<(Node, Node)> = self.lattice.clone();
        // The short ones first: the long ones then join them.
        links.sort_by(|x, y| {
            let l = |(a, b): (Node, Node)| self.node_pos(a).distance(self.node_pos(b));
            l(*x).total_cmp(&l(*y))
        });
        links
            .into_iter()
            .map(|(a, b)| RoadLink {
                a: end(a, self.node_pos(b)),
                b: end(b, self.node_pos(a)),
            })
            .collect()
    }

    /// The continent's land (a second or so of work).
    pub fn terrain(&self) -> Terrain {
        Terrain::generate(
            self.theme,
            self.terrain_seed,
            &self.pads(),
            &self.road_links(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct WorldDef {
    pub params: WorldParams,
    pub continents: Vec<ContinentDef>,
}

const NAMES: [&str; 24] = [
    "Velkara",
    "Tharsis Rim",
    "Myrrhen",
    "Corvalis",
    "Quillon",
    "Zhedra",
    "Ondara",
    "Paxillon",
    "Siverra",
    "Kaldrith",
    "Lumora",
    "Brakkon",
    "Feyhold",
    "Nimbrel",
    "Ostravel",
    "Draskor",
    "Halvine",
    "Iridune",
    "Marrowmere",
    "Tessaline",
    "Vorquen",
    "Ylsabeth",
    "Graventh",
    "Solveig Reach",
];
const BASE_NAMES: [&str; 30] = [
    "Alpha", "Bravo", "Cobalt", "Delta", "Ember", "Flux", "Gamma", "Helix", "Ion", "Jolt",
    "Krypton", "Lumen", "Mirage", "Nova", "Onyx", "Prism", "Quasar", "Radiant", "Static", "Tesla",
    "Umbra", "Vector", "Wraith", "Xenon", "Yield", "Zenith", "Argon", "Boreal", "Cipher", "Dynamo",
];

/// Do segments a-b and c-d cross (not just touch at an end)?
fn crosses(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let o = |p: Vec2, q: Vec2, r: Vec2| (q - p).perp_dot(r - p);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    ((d1 > 1e-6 && d2 < -1e-6) || (d1 < -1e-6 && d2 > 1e-6))
        && ((d3 > 1e-6 && d4 < -1e-6) || (d3 < -1e-6 && d4 > 1e-6))
}

/// The links of the planet map: (continent, continent) pairs. Sanctuaries are 0..3.
fn link_continents(pos: &[Vec2], rng: &mut Seq, max_degree: usize) -> Option<Vec<(usize, usize)>> {
    let n = pos.len();
    let mut deg = vec![0usize; n];
    let mut links: Vec<(usize, usize)> = Vec::new();
    let ok_to_add = |links: &Vec<(usize, usize)>, a: usize, b: usize| {
        !links.contains(&(a.min(b), a.max(b)))
            && !links.iter().any(|&(c, d)| {
                c != a && c != b && d != a && d != b && crosses(pos[a], pos[b], pos[c], pos[d])
            })
    };
    // Sanctuaries: two links each, to their nearest contested continents not yet linked to
    // another sanctuary if possible.
    for s in 0..3 {
        let mut near: Vec<usize> = (3..n).collect();
        near.sort_by(|&a, &b| {
            let pa = pos[a].distance(pos[s]) + if deg[a] > 0 { 0.6 } else { 0.0 };
            let pb = pos[b].distance(pos[s]) + if deg[b] > 0 { 0.6 } else { 0.0 };
            pa.total_cmp(&pb)
        });
        let mut added = 0;
        for &c in &near {
            if added == 2 {
                break;
            }
            if deg[c] < max_degree && ok_to_add(&links, s, c) {
                links.push((s.min(c), s.max(c)));
                deg[s] += 1;
                deg[c] += 1;
                added += 1;
            }
        }
        if added < 2 {
            return None;
        }
    }
    // Contested continents: shortest non-crossing links first, while both have room.
    let mut cand: Vec<(f32, usize, usize)> = Vec::new();
    for a in 3..n {
        for b in a + 1..n {
            cand.push((pos[a].distance(pos[b]) * rng.range(0.9, 1.1), a, b));
        }
    }
    cand.sort_by(|x, y| x.0.total_cmp(&y.0));
    // First a spanning structure (connect components), then fill up degrees to 2.
    let mut comp: Vec<usize> = (0..n).collect();
    fn find(c: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while c[r] != r {
            r = c[r];
        }
        c[x] = r;
        r
    }
    for &(a, b) in &links {
        let (ra, rb) = (find(&mut comp, a), find(&mut comp, b));
        comp[ra] = rb;
    }
    for &(_, a, b) in &cand {
        let (ra, rb) = (find(&mut comp, a), find(&mut comp, b));
        if ra != rb && deg[a] < max_degree && deg[b] < max_degree && ok_to_add(&links, a, b) {
            links.push((a, b));
            deg[a] += 1;
            deg[b] += 1;
            comp[ra] = rb;
        }
    }
    for &(_, a, b) in &cand {
        if (deg[a] < 2 || deg[b] < 2)
            && deg[a] < max_degree
            && deg[b] < max_degree
            && ok_to_add(&links, a, b)
        {
            links.push((a, b));
            deg[a] += 1;
            deg[b] += 1;
        }
    }
    let root = find(&mut comp, 0);
    let connected = (0..n).all(|i| find(&mut comp, i) == root);
    let degrees_ok =
        (3..n).all(|i| (2..=max_degree).contains(&deg[i])) && (0..3).all(|i| deg[i] == 2);
    (connected && degrees_ok).then_some(links)
}

/// Relative neighbourhood graph of `pts`: a and b are linked unless some c is nearer to both.
fn rng_graph(pts: &[Vec2]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for a in 0..pts.len() {
        for b in a + 1..pts.len() {
            let d = pts[a].distance(pts[b]);
            let blocked = (0..pts.len()).any(|c| {
                c != a && c != b && pts[a].distance(pts[c]) < d && pts[b].distance(pts[c]) < d
            });
            if !blocked {
                out.push((a, b));
            }
        }
    }
    out
}

impl WorldDef {
    pub fn generate(params: WorldParams) -> WorldDef {
        let contested = params.contested.clamp(3, 18) as usize;
        let n = contested + 3;
        let mut attempt = 0u64;
        let (pos, links) = loop {
            let mut rng = Seq::new(params.seed, 0x5eed + attempt);
            attempt += 1;
            let mut pos: Vec<Vec2> = (0..3)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 3.0;
                    Vec2::new(a.cos(), a.sin())
                })
                .collect();
            let min_sep = (1.3 / (contested as f32).sqrt()).min(0.55);
            let mut tries = 0;
            while pos.len() < n && tries < 5000 {
                tries += 1;
                let p = Vec2::new(rng.range(-0.8, 0.8), rng.range(-0.8, 0.8));
                if p.length() > 0.78 || pos.iter().any(|q| q.distance(p) < min_sep) {
                    continue;
                }
                pos.push(p);
            }
            if pos.len() < n {
                continue;
            }
            let max_degree = if attempt > 400 { 4 } else { 3 };
            if let Some(links) = link_continents(&pos, &mut rng, max_degree) {
                break (pos, links);
            }
            assert!(attempt < 2000, "no world for {params:?}");
        };
        let mut rng = Seq::new(params.seed, 0xc0);
        // Names: shuffled.
        let mut names: Vec<&str> = NAMES.to_vec();
        for i in (1..names.len()).rev() {
            let j = rng.int(0, i as i32 + 1) as usize;
            names.swap(i, j);
        }
        // Themes: each contested theme used in turn (shuffled), so neighbours differ.
        let mut themes = Theme::CONTESTED.to_vec();
        for i in (1..themes.len()).rev() {
            let j = rng.int(0, i as i32 + 1) as usize;
            themes.swap(i, j);
        }
        let mut continents: Vec<ContinentDef> = (0..n)
            .map(|i| {
                let sanctuary = (i < 3).then(|| Team::ALL[i]);
                ContinentDef {
                    id: i as ContinentId,
                    name: match sanctuary {
                        Some(t) => format!("{} Sanctuary", t.name()),
                        None => names[(i - 3) % names.len()].to_string(),
                    },
                    theme: if i < 3 {
                        Theme::Haven
                    } else {
                        themes[(i - 3) % themes.len()]
                    },
                    sanctuary,
                    map_pos: pos[i],
                    bases: Vec::new(),
                    gates: Vec::new(),
                    lattice: Vec::new(),
                    terrain_seed: rng.next_u64() as u32,
                }
            })
            .collect();
        // Gates: one per link at each end, towards the other continent on the planet map.
        for &(a, b) in &links {
            let (ga, gb) = (continents[a].gates.len(), continents[b].gates.len());
            continents[a].gates.push(Gate {
                pos: Vec2::ZERO,
                to: b as ContinentId,
                to_gate: gb as u8,
            });
            continents[b].gates.push(Gate {
                pos: Vec2::ZERO,
                to: a as ContinentId,
                to_gate: ga as u8,
            });
        }
        #[allow(clippy::needless_range_loop)]
        for c in 0..n {
            let mut rng = Seq::new(params.seed, 0x1000 + c as u64);
            let here = continents[c].map_pos;
            let mut angles: Vec<f32> = continents[c]
                .gates
                .iter()
                .map(|g| {
                    let d = pos[g.to as usize] - here;
                    // Map +y is north (-z in the world).
                    d.x.atan2(-d.y)
                })
                .collect();
            spread_angles(&mut angles, 1.4);
            for (g, a) in continents[c].gates.iter_mut().zip(&angles) {
                let r = 1480.0;
                g.pos = Vec2::new(a.sin(), -a.cos()) * r;
            }
            let gates: Vec<Vec2> = continents[c].gates.iter().map(|g| g.pos).collect();
            let def = &mut continents[c];
            if def.sanctuary.is_some() {
                def.bases.push(BaseSite {
                    name: "Home".into(),
                    kind: BaseKind::Home,
                    pos: Vec2::ZERO,
                    facing: 0.0,
                    capture: Capture::Console,
                });
                def.lattice = (0..gates.len())
                    .map(|g| (Node::Base(0), Node::Gate(g as u8)))
                    .collect();
                continue;
            }
            // Center base, then outposts by rejection sampling.
            let center = Vec2::new(rng.range(-150.0, 150.0), rng.range(-150.0, 150.0));
            let mut sites = vec![center];
            let want = rng.int(5, 9) as usize;
            let mut tries = 0;
            while sites.len() < want + 1 && tries < 4000 {
                tries += 1;
                let a = rng.range(0.0, std::f32::consts::TAU);
                let r = rng.range(520.0, 1420.0);
                let p = Vec2::new(a.cos(), a.sin()) * r;
                if sites.iter().any(|q| q.distance(p) < 600.0)
                    || gates.iter().any(|g| g.distance(p) < 420.0)
                {
                    continue;
                }
                sites.push(p);
            }
            let mut used = Vec::new();
            for (i, &p) in sites.iter().enumerate() {
                let name = loop {
                    let n = BASE_NAMES[rng.int(0, BASE_NAMES.len() as i32) as usize];
                    if !used.contains(&n) {
                        used.push(n);
                        break n;
                    }
                };
                // Facing the continent's middle (the center base: a random way).
                let facing = if i == 0 {
                    rng.range(0.0, std::f32::consts::TAU)
                } else {
                    let d = -p;
                    (-d.x).atan2(-d.y)
                };
                def.bases.push(BaseSite {
                    name: if i == 0 {
                        format!("{} Central", def.name)
                    } else {
                        name.to_string()
                    },
                    kind: if i == 0 {
                        BaseKind::Center
                    } else {
                        BaseKind::Outpost
                    },
                    pos: p,
                    facing,
                    capture: Capture::Console,
                });
            }
            // The lattice between the bases, never straight from a gate to the center.
            for (a, b) in rng_graph(&sites) {
                def.lattice
                    .push((Node::Base(a as BaseIdx), Node::Base(b as BaseIdx)));
            }
            for (g, gp) in gates.iter().enumerate() {
                let mut near: Vec<usize> = (1..sites.len()).collect();
                near.sort_by(|&a, &b| sites[a].distance(*gp).total_cmp(&sites[b].distance(*gp)));
                let links = if near.len() > 3 && rng.chance(0.5) {
                    2
                } else {
                    1
                };
                for &b in near.iter().take(links) {
                    def.lattice
                        .push((Node::Gate(g as u8), Node::Base(b as BaseIdx)));
                }
            }
            // Towers: one near most bases (two near the center), away from everything else.
            let mut towers: Vec<(Vec2, usize)> = Vec::new();
            for (bi, &site) in sites.iter().enumerate() {
                let want = if bi == 0 {
                    2
                } else if rng.chance(0.75) {
                    1
                } else {
                    0
                };
                let (mut tries, mut made) = (0, 0);
                while made < want && tries < 200 {
                    tries += 1;
                    let a = rng.range(0.0, std::f32::consts::TAU);
                    let p = site + Vec2::new(a.cos(), a.sin()) * rng.range(250.0, 340.0);
                    if p.length() > 1600.0
                        || sites.iter().any(|q| q.distance(p) < 230.0)
                        || gates.iter().any(|g| g.distance(p) < 200.0)
                        || towers.iter().any(|t| t.0.distance(p) < 260.0)
                    {
                        continue;
                    }
                    towers.push((p, bi));
                    made += 1;
                }
            }
            for (p, bi) in towers {
                let facing = rng.int(0, 4) as f32 * std::f32::consts::FRAC_PI_2;
                let stem = def.bases[bi].name.trim_end_matches(" Central").to_string();
                // The centre has two: told apart by where they stand (north is -Z).
                let name = if bi == 0 {
                    let d = p - sites[bi];
                    let side = if d.x.abs() > d.y.abs() {
                        if d.x > 0.0 { "East" } else { "West" }
                    } else if d.y > 0.0 {
                        "South"
                    } else {
                        "North"
                    };
                    format!("{stem} {side} Tower")
                } else {
                    format!("{stem} Tower")
                };
                def.bases.push(BaseSite {
                    name,
                    kind: BaseKind::Tower,
                    pos: p,
                    facing,
                    capture: Capture::Console,
                });
            }
        }
        for c in &mut continents {
            assign_capture(c, params.seed);
        }
        WorldDef { params, continents }
    }

    pub fn sanctuary(&self, team: Team) -> &ContinentDef {
        &self.continents[team.index()]
    }

    /// The other end of gate `gate` on continent `c`.
    pub fn gate_target(&self, c: ContinentId, gate: u8) -> (ContinentId, u8) {
        let g = &self.continents[c as usize].gates[gate as usize];
        (g.to, g.to_gate)
    }

    /// Continents linked to `c` by a warp gate.
    pub fn neighbours(&self, c: ContinentId) -> impl Iterator<Item = ContinentId> + '_ {
        self.continents[c as usize].gates.iter().map(|g| g.to)
    }
}

/// A third of a contested continent's outposts (rounded up) are taken at capture points;
/// which ones follows from a hash, not the continent's random stream (that would move
/// everything drawn after it).
fn assign_capture(def: &mut ContinentDef, seed: u64) {
    let mut outposts: Vec<usize> = (0..def.bases.len())
        .filter(|&b| def.bases[b].kind == BaseKind::Outpost)
        .collect();
    let key = (seed as u32) ^ ((seed >> 32) as u32) ^ 0xca97;
    outposts.sort_by_key(|&b| crate::noise::hash2(def.id as i32, b as i32, key));
    let n = outposts.len().div_ceil(3);
    for &b in &outposts[..n] {
        def.bases[b].capture = Capture::Points;
    }
}

/// Pushes angles apart until neighbours are at least `min` radians apart.
fn spread_angles(angles: &mut [f32], min: f32) {
    use std::f32::consts::TAU;
    for _ in 0..50 {
        let mut moved = false;
        for i in 0..angles.len() {
            for j in 0..angles.len() {
                if i == j {
                    continue;
                }
                let mut d = (angles[j] - angles[i]).rem_euclid(TAU);
                if d > std::f32::consts::PI {
                    d -= TAU;
                }
                if d.abs() < min {
                    let push = (min - d.abs()) * 0.5 + 0.01;
                    let s = if d >= 0.0 { 1.0 } else { -1.0 };
                    angles[j] += s * push;
                    angles[i] -= s * push;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worlds_follow_the_rules() {
        for seed in 0..40u64 {
            for contested in [4u8, 6, 8, 10] {
                let w = WorldDef::generate(WorldParams { seed, contested });
                assert_eq!(w.continents.len(), contested as usize + 3);
                for c in &w.continents {
                    let g = c.gates.len();
                    if c.sanctuary.is_some() {
                        assert_eq!(g, 2, "seed {seed}: sanctuary gates");
                        let a = c.gates[0].to;
                        let b = c.gates[1].to;
                        assert_ne!(a, b);
                        assert!(a >= 3 && b >= 3, "sanctuaries link to contested continents");
                        assert_eq!(c.bases.len(), 1);
                    } else {
                        assert!((2..=4).contains(&g), "seed {seed}: {} gates", g);
                        assert!(c.bases.len() >= 5, "seed {seed}: {} bases", c.bases.len());
                        assert_eq!(c.bases[0].kind, BaseKind::Center);
                        // A third of the outposts (rounded up) at capture points; nothing
                        // else.
                        let outposts = c
                            .bases
                            .iter()
                            .filter(|b| b.kind == BaseKind::Outpost)
                            .count();
                        let points: Vec<&BaseSite> = c
                            .bases
                            .iter()
                            .filter(|b| b.capture == Capture::Points)
                            .collect();
                        assert_eq!(points.len(), outposts.div_ceil(3));
                        assert!(points.iter().all(|b| b.kind == BaseKind::Outpost));
                        // Every gate reaches the center through the lattice, never directly.
                        for gi in 0..g {
                            assert!(
                                !c.neighbours(Node::Gate(gi as u8))
                                    .any(|n| n == Node::Base(0))
                            );
                            assert!(reaches(c, Node::Gate(gi as u8), Node::Base(0)));
                        }
                    }
                    // Gates come in pairs.
                    for (i, gate) in c.gates.iter().enumerate() {
                        let back = &w.continents[gate.to as usize].gates[gate.to_gate as usize];
                        assert_eq!(back.to, c.id);
                        assert_eq!(back.to_gate as usize, i);
                    }
                }
                // Everything connected.
                let mut seen = vec![false; w.continents.len()];
                let mut stack = vec![0u8];
                while let Some(c) = stack.pop() {
                    if std::mem::replace(&mut seen[c as usize], true) {
                        continue;
                    }
                    stack.extend(w.neighbours(c));
                }
                assert!(seen.iter().all(|&s| s), "seed {seed}");
            }
        }
    }

    fn reaches(c: &ContinentDef, from: Node, to: Node) -> bool {
        let mut seen = Vec::new();
        let mut stack = vec![from];
        while let Some(n) = stack.pop() {
            if n == to {
                return true;
            }
            if seen.contains(&n) {
                continue;
            }
            seen.push(n);
            stack.extend(c.neighbours(n));
        }
        false
    }

    #[test]
    fn same_seed_same_world() {
        let a = WorldDef::generate(WorldParams {
            seed: 7,
            contested: 6,
        });
        let b = WorldDef::generate(WorldParams {
            seed: 7,
            contested: 6,
        });
        for (x, y) in a.continents.iter().zip(&b.continents) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.lattice, y.lattice);
            assert_eq!(x.terrain_seed, y.terrain_seed);
            assert_eq!(x.bases.len(), y.bases.len());
        }
    }
}

/// How big a world to make: the contested continents (besides the sanctuaries).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SizeChoice {
    Small,
    #[default]
    Medium,
    Large,
}

impl SizeChoice {
    pub const ALL: [SizeChoice; 3] = [SizeChoice::Small, SizeChoice::Medium, SizeChoice::Large];

    pub fn name(self) -> &'static str {
        match self {
            SizeChoice::Small => "small",
            SizeChoice::Medium => "medium",
            SizeChoice::Large => "large",
        }
    }

    pub fn contested(self) -> u8 {
        match self {
            SizeChoice::Small => 4,
            SizeChoice::Medium => 6,
            SizeChoice::Large => 9,
        }
    }

    pub fn parse(s: &str) -> Option<SizeChoice> {
        SizeChoice::ALL.into_iter().find(|d| d.name() == s)
    }
}
