//! Continent themes: how the land is shaped and what grows and stands on it. The client draws
//! each theme with its own palette, sky and scenery meshes (src/terrain.rs, src/flora.rs).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Theme {
    /// Rolling dunes of dark sand, glowing crystal clusters and obelisks.
    SynthDunes,
    /// Steep green hills and rugged highlands under bioluminescent palms, ferns and flowers.
    NeonJungle,
    /// Ice ridges, shards and spires under an aurora.
    Glacier,
    /// Volcanic: basalt columns, craters and glowing vents.
    Ember,
    /// Low wetlands of giant glowing mushrooms and reeds.
    FungalMarsh,
    /// Terraced mesas scored with glowing contour lines, circuit trees and monoliths.
    CircuitMesa,
    /// A faction's sanctuary: a calm plateau with neat trees and lamps.
    Haven,
    /// Canyon caves: a rock plateau cut by roofed canyons along the roads, the bases in
    /// sinkholes; crystals and fungi glowing in the dark (`crate::caves`).
    HollowReach,
}

/// How a theme shapes its land (metres).
#[derive(Clone, Copy, Debug)]
pub struct TerrainStyle {
    /// Rolling hills: amplitude and feature size.
    pub hill_amp: f32,
    pub hill_scale: f32,
    /// Mountain ridges: amplitude and feature size (they rise in some regions only).
    pub ridge_amp: f32,
    pub ridge_scale: f32,
    /// How far the land is pushed sideways by a slow noise (twisting the shapes).
    pub warp: f32,
    /// Terraces: step height (0: none).
    pub terrace: f32,
    /// Dunes along one direction: amplitude (0: none) and wavelength.
    pub dune_amp: f32,
    pub dune_len: f32,
    /// Craters: depth (0: none) and spacing.
    pub crater_depth: f32,
    pub crater_scale: f32,
    /// Height of the land above the sea at the coast and inland on average.
    pub base: f32,
    /// Crags: rugged highlands too steep to drive over, covering about `crag_cover` of the
    /// land (0: none) away from bases and gates; their height and feature size.
    pub crag_cover: f32,
    pub crag_amp: f32,
    pub crag_scale: f32,
}

/// A piece of scenery. The ones with a `collider` stop players, vehicles and shots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SceneryKind {
    Rock,
    Boulder,
    Crystal,
    Obelisk,
    Palm,
    Fern,
    Flower,
    IceSpire,
    IceShard,
    Basalt,
    Vent,
    DeadTree,
    Mushroom,
    Toadstool,
    Reed,
    CircuitTree,
    Monolith,
    Shrub,
    Grass,
    Tree,
    Lamp,
}

impl SceneryKind {
    /// The collider at scale 1: radius and height of an upright prism (None: walked through).
    pub fn collider(self) -> Option<(f32, f32)> {
        use SceneryKind::*;
        match self {
            Rock => Some((1.2, 1.2)),
            Boulder => Some((3.0, 3.2)),
            Crystal => Some((0.9, 3.5)),
            Obelisk => Some((1.3, 12.0)),
            Palm => Some((0.35, 9.0)),
            IceSpire => Some((2.2, 18.0)),
            IceShard => Some((0.9, 4.0)),
            Basalt => Some((1.6, 6.0)),
            DeadTree => Some((0.3, 6.0)),
            Mushroom => Some((0.7, 9.0)),
            CircuitTree => Some((0.4, 8.0)),
            Monolith => Some((1.5, 10.0)),
            Tree => Some((0.35, 8.0)),
            Lamp => Some((0.2, 5.0)),
            Fern | Flower | Vent | Toadstool | Reed | Shrub | Grass => None,
        }
    }
}

/// What a theme scatters, how densely (items per hectare where it may grow) and at what scale.
#[derive(Clone, Copy, Debug)]
pub struct Scatter {
    pub kind: SceneryKind,
    pub per_ha: f32,
    pub scale: (f32, f32),
    /// Only on land this flat (max slope, rise over run) and this high above the sea.
    pub max_slope: f32,
    pub min_height: f32,
    /// Clumping: 0 even, 1 in dense groves with bare land between.
    pub clump: f32,
}

const fn s(kind: SceneryKind, per_ha: f32, lo: f32, hi: f32, clump: f32) -> Scatter {
    Scatter {
        kind,
        per_ha,
        scale: (lo, hi),
        max_slope: 0.7,
        min_height: 1.5,
        clump,
    }
}

impl Theme {
    pub const CONTESTED: [Theme; 7] = [
        Theme::SynthDunes,
        Theme::NeonJungle,
        Theme::Glacier,
        Theme::Ember,
        Theme::FungalMarsh,
        Theme::CircuitMesa,
        Theme::HollowReach,
    ];

    /// Whether its land is canyon caves (`crate::caves`).
    pub fn caves(self) -> bool {
        self == Theme::HollowReach
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::SynthDunes => "Synth Dunes",
            Theme::NeonJungle => "Neon Jungle",
            Theme::Glacier => "Glacier",
            Theme::Ember => "Ember",
            Theme::FungalMarsh => "Fungal Marsh",
            Theme::CircuitMesa => "Circuit Mesa",
            Theme::Haven => "Sanctuary",
            Theme::HollowReach => "Hollow Reach",
        }
    }

    pub fn style(self) -> TerrainStyle {
        let base = TerrainStyle {
            hill_amp: 30.0,
            hill_scale: 600.0,
            ridge_amp: 0.0,
            ridge_scale: 900.0,
            warp: 150.0,
            terrace: 0.0,
            dune_amp: 0.0,
            dune_len: 90.0,
            crater_depth: 0.0,
            crater_scale: 350.0,
            base: 6.0,
            crag_cover: 0.0,
            crag_amp: 0.0,
            crag_scale: 300.0,
        };
        // From flattest to most rugged: Fungal Marsh, Synth Dunes, Circuit Mesa, Ember, Neon
        // Jungle, Glacier.
        match self {
            Theme::SynthDunes => TerrainStyle {
                hill_amp: 25.0,
                ridge_amp: 45.0,
                ridge_scale: 1200.0,
                dune_amp: 5.0,
                ..base
            },
            Theme::NeonJungle => TerrainStyle {
                hill_amp: 55.0,
                hill_scale: 450.0,
                ridge_amp: 70.0,
                ridge_scale: 700.0,
                warp: 220.0,
                crag_cover: 0.5,
                crag_amp: 90.0,
                crag_scale: 300.0,
                ..base
            },
            Theme::Glacier => TerrainStyle {
                hill_amp: 35.0,
                ridge_amp: 120.0,
                ridge_scale: 800.0,
                warp: 120.0,
                base: 10.0,
                crag_cover: 0.6,
                crag_amp: 130.0,
                crag_scale: 420.0,
                ..base
            },
            Theme::Ember => TerrainStyle {
                hill_amp: 30.0,
                ridge_amp: 60.0,
                crater_depth: 22.0,
                crater_scale: 320.0,
                crag_cover: 0.3,
                crag_amp: 60.0,
                crag_scale: 260.0,
                ..base
            },
            Theme::FungalMarsh => TerrainStyle {
                hill_amp: 14.0,
                hill_scale: 500.0,
                ridge_amp: 20.0,
                warp: 260.0,
                base: 2.5,
                ..base
            },
            Theme::CircuitMesa => TerrainStyle {
                hill_amp: 50.0,
                hill_scale: 700.0,
                ridge_amp: 35.0,
                terrace: 12.0,
                warp: 90.0,
                base: 8.0,
                crag_cover: 0.35,
                crag_amp: 70.0,
                crag_scale: 380.0,
                ..base
            },
            Theme::Haven => TerrainStyle {
                hill_amp: 20.0,
                hill_scale: 800.0,
                ridge_amp: 40.0,
                base: 8.0,
                ..base
            },
            // (Gentle land: the canyons and the plateau over them come after, `caves::raise`.)
            Theme::HollowReach => TerrainStyle {
                hill_amp: 12.0,
                hill_scale: 600.0,
                warp: 200.0,
                base: 6.0,
                ..base
            },
        }
    }

    /// What grows and stands on this theme's land.
    pub fn scatter(self) -> &'static [Scatter] {
        use SceneryKind::*;
        match self {
            Theme::SynthDunes => {
                const L: &[Scatter] = &[
                    s(Rock, 0.6, 0.6, 1.8, 0.3),
                    s(Crystal, 0.5, 0.6, 1.6, 0.8),
                    s(Obelisk, 0.03, 0.8, 1.4, 0.2),
                    s(Grass, 4.0, 0.6, 1.2, 0.7),
                ];
                L
            }
            Theme::NeonJungle => {
                const L: &[Scatter] = &[
                    s(Palm, 3.0, 0.8, 1.4, 0.7),
                    s(Fern, 10.0, 0.6, 1.4, 0.6),
                    s(Flower, 5.0, 0.6, 1.2, 0.8),
                    s(Boulder, 0.15, 0.6, 1.4, 0.3),
                    s(Rock, 0.5, 0.6, 1.5, 0.3),
                ];
                L
            }
            Theme::Glacier => {
                const L: &[Scatter] = &[
                    s(IceSpire, 0.08, 0.6, 1.4, 0.5),
                    s(IceShard, 1.0, 0.5, 1.5, 0.7),
                    s(Boulder, 0.25, 0.6, 1.5, 0.3),
                    s(Rock, 0.6, 0.6, 1.5, 0.3),
                ];
                L
            }
            Theme::Ember => {
                const L: &[Scatter] = &[
                    s(Basalt, 0.6, 0.6, 1.6, 0.8),
                    s(DeadTree, 0.8, 0.7, 1.3, 0.6),
                    s(Vent, 0.5, 0.6, 1.5, 0.6),
                    s(Rock, 0.8, 0.6, 1.6, 0.3),
                ];
                L
            }
            Theme::FungalMarsh => {
                const L: &[Scatter] = &[
                    s(Mushroom, 1.6, 0.6, 1.6, 0.7),
                    s(Toadstool, 8.0, 0.5, 1.4, 0.7),
                    s(Reed, 14.0, 0.7, 1.3, 0.6),
                    s(Rock, 0.3, 0.6, 1.4, 0.3),
                ];
                L
            }
            Theme::CircuitMesa => {
                const L: &[Scatter] = &[
                    s(CircuitTree, 1.2, 0.7, 1.4, 0.6),
                    s(Monolith, 0.06, 0.7, 1.3, 0.3),
                    s(Shrub, 5.0, 0.6, 1.3, 0.6),
                    s(Rock, 0.5, 0.6, 1.5, 0.3),
                ];
                L
            }
            Theme::Haven => {
                const L: &[Scatter] = &[
                    s(Tree, 1.5, 0.8, 1.3, 0.5),
                    s(Shrub, 4.0, 0.6, 1.3, 0.5),
                    s(Grass, 4.0, 0.6, 1.2, 0.5),
                ];
                L
            }
            // (Low enough to stand under the caves' roofs.)
            Theme::HollowReach => {
                const L: &[Scatter] = &[
                    s(Crystal, 0.8, 0.6, 1.5, 0.8),
                    s(Mushroom, 0.4, 0.5, 1.0, 0.7),
                    s(Toadstool, 3.0, 0.5, 1.3, 0.7),
                    s(Boulder, 0.15, 0.6, 1.3, 0.3),
                    s(Rock, 0.6, 0.6, 1.5, 0.3),
                    s(Vent, 0.3, 0.6, 1.4, 0.6),
                ];
                L
            }
        }
    }
}
