//! Bars over the objectives, in the world: the generators' and wall turrets' health, the
//! capture points' (who holds each, and who is taking it, how far; a letter over it and a
//! ring on the ground round its zone), and a hack's or capture's progress over the console.
//! Turned to the eyes, bigger the further, shown near (or while something is happening).
//! Drawn with the neon accent material, so the headset's own renderer shows them too.

use bevy::prelude::*;
use ps_sim::base::POINT_NAMES;
use ps_sim::{BaseIdx, Team};

use crate::desktop::MainCamera;
use crate::game::Game;
use crate::materials::{AccentMaterial, NeonUniform, team_color};

/// What a bar shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarOf {
    Generator(BaseIdx),
    /// A wall turret: its base and index there.
    Turret(BaseIdx, u8),
    Point(BaseIdx, u8),
    /// A hack or capture under way.
    Console(BaseIdx),
}

/// A bar: what it shows, and how wide it is (m, near).
#[derive(Component)]
pub struct WorldBar {
    pub of: BarOf,
    pub width: f32,
}

#[derive(Component)]
struct BarBack;
#[derive(Component)]
struct BarFill;
#[derive(Component)]
struct BarLetter;
/// A capture point's ring on the ground.
#[derive(Component)]
struct PointRing(BaseIdx, u8);

/// The bars' colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Team(Option<Team>),
    Dim(Option<Team>),
    Good,
    Warn,
    Bad,
    Dark,
    White,
}

impl Tone {
    fn index(self) -> usize {
        let t = |o: Option<Team>| o.map_or(3, |t| t.index());
        match self {
            Tone::Team(o) => t(o),
            Tone::Dim(o) => 4 + t(o),
            Tone::Good => 8,
            Tone::Warn => 9,
            Tone::Bad => 10,
            Tone::Dark => 11,
            Tone::White => 12,
        }
    }

    fn health(frac: f32) -> Tone {
        if frac > 0.6 {
            Tone::Good
        } else if frac > 0.25 {
            Tone::Warn
        } else {
            Tone::Bad
        }
    }
}

#[derive(Resource)]
pub struct BarAssets {
    quad: Handle<Mesh>,
    /// A quad from x = 0 to 1 (scaled by the share it fills).
    fill: Handle<Mesh>,
    ring: Handle<Mesh>,
    letters: [Handle<Mesh>; 3],
    tones: Vec<Handle<AccentMaterial>>,
}

impl BarAssets {
    fn tone(&self, t: Tone) -> Handle<AccentMaterial> {
        self.tones[t.index()].clone()
    }
}

pub struct WorldBarPlugin;

impl Plugin for WorldBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, make_assets).add_systems(
            Update,
            update.run_if(resource_exists::<Game>.and_then(resource_exists::<BarAssets>)),
        );
    }
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
) {
    let mut tones = Vec::new();
    for t in [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None] {
        tones.push(accents.add(AccentMaterial {
            neon: NeonUniform::fixed(team_color(t), 3.0),
            overlay: false,
        }));
    }
    for t in [Some(Team::Red), Some(Team::Blue), Some(Team::Green), None] {
        tones.push(accents.add(AccentMaterial {
            neon: NeonUniform::fixed(team_color(t) * 0.25, 1.0),
            overlay: false,
        }));
    }
    for (c, k) in [
        (Vec3::new(0.2, 1.0, 0.35), 3.0),
        (Vec3::new(1.0, 0.6, 0.05), 3.0),
        (Vec3::new(1.0, 0.08, 0.05), 3.5),
        (Vec3::new(0.03, 0.03, 0.05), 1.0),
        (Vec3::ONE, 4.0),
    ] {
        tones.push(accents.add(AccentMaterial {
            neon: NeonUniform::fixed(c, k),
            overlay: false,
        }));
    }
    let letters = [0, 1, 2].map(|i| {
        let s = POINT_NAMES[i].to_string();
        let h = 1.0;
        let w = crate::vfont::width(&s, h, 800);
        let mut m = crate::vfont::mesh(&s, h, 800);
        for p in m.pos.iter_mut() {
            p[0] -= w / 2.0;
        }
        meshes.add(m.to_mesh())
    });
    commands.insert_resource(BarAssets {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        fill: meshes.add(
            Rectangle::new(1.0, 1.0)
                .mesh()
                .build()
                .translated_by(Vec3::X * 0.5),
        ),
        ring: meshes.add(Annulus::new(0.94, 1.0).mesh().resolution(48).build()),
        letters,
        tones,
    });
}

/// A bar over `at` for `of` (under `parent`).
pub fn spawn_bar(
    commands: &mut Commands,
    assets: &BarAssets,
    parent: Entity,
    of: BarOf,
    at: Vec3,
    width: f32,
) {
    let bar = commands
        .spawn((
            WorldBar { of, width },
            Transform::from_translation(at),
            Visibility::Hidden,
            ChildOf(parent),
        ))
        .id();
    let h = 0.22;
    commands.spawn((
        BarBack,
        Mesh3d(assets.quad.clone()),
        MeshMaterial3d(assets.tone(Tone::Dark)),
        Transform::from_scale(Vec3::new(width + 0.08, h + 0.08, 1.0)),
        ChildOf(bar),
    ));
    commands.spawn((
        BarFill,
        Mesh3d(assets.fill.clone()),
        MeshMaterial3d(assets.tone(Tone::Good)),
        Transform::from_xyz(-width / 2.0, 0.0, 0.01).with_scale(Vec3::new(width, h, 1.0)),
        ChildOf(bar),
    ));
    if let BarOf::Point(_, i) = of {
        commands.spawn((
            BarLetter,
            Mesh3d(assets.letters[i as usize % 3].clone()),
            MeshMaterial3d(assets.tone(Tone::White)),
            Transform::from_xyz(0.0, 0.75, 0.0).with_scale(Vec3::splat(0.9)),
            ChildOf(bar),
        ));
    }
}

/// A capture point's ring on the ground round its zone (radius `r`).
pub fn spawn_ring(
    commands: &mut Commands,
    assets: &BarAssets,
    parent: Entity,
    b: BaseIdx,
    i: u8,
    at: Vec3,
    r: f32,
) {
    commands.spawn((
        PointRing(b, i),
        Mesh3d(assets.ring.clone()),
        MeshMaterial3d(assets.tone(Tone::White)),
        Transform::from_translation(at + Vec3::Y * 0.06)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(r)),
        ChildOf(parent),
    ));
}

/// What a bar shows now: whether, how full, the fill's and the back's colours, the letter's.
struct Look {
    frac: f32,
    fill: Tone,
    back: Tone,
    letter: Tone,
}

fn look(game: &Game, of: BarOf, d: f32, blink: bool) -> Option<Look> {
    let c = game.continent?;
    let map = game.map.as_ref()?;
    match of {
        BarOf::Generator(b) => {
            let g = game.snap.bases.get(b as usize)?.generator / 100.0;
            (d < 60.0 || (g < 1.0 && d < 250.0)).then(|| Look {
                frac: g,
                fill: Tone::health(g),
                back: Tone::Dark,
                letter: Tone::White,
            })
        }
        BarOf::Turret(b, i) => {
            let t = game
                .snap
                .turrets
                .iter()
                .find(|t| t.base == b && t.index == i)?;
            let h = t.health / 100.0;
            let hacked = t.team != game.state.base(c, b).owner;
            (d < 35.0 || ((h < 1.0 || hacked) && d < 150.0)).then(|| Look {
                frac: h,
                fill: if hacked {
                    Tone::Team(t.team)
                } else {
                    Tone::health(h)
                },
                back: Tone::Dark,
                letter: Tone::White,
            })
        }
        BarOf::Point(b, i) => {
            let p = game.snap.bases.get(b as usize)?.points?[i as usize];
            (d < 450.0).then(|| Look {
                frac: p.progress as f32 / 100.0,
                fill: if p.contested {
                    if blink { Tone::White } else { Tone::Team(p.by) }
                } else {
                    Tone::Team(p.by)
                },
                back: Tone::Dim(p.owner),
                letter: Tone::Team(p.owner),
            })
        }
        BarOf::Console(b) => {
            let h = game.state.base(c, b).hack?;
            let full = ps_sim::state::hack_time(&game.def, c, b);
            let _ = map;
            (d < 300.0).then(|| Look {
                frac: (1.0 - h.left / full).clamp(0.0, 1.0),
                fill: Tone::Team(Some(h.team)),
                back: Tone::Dark,
                letter: Tone::White,
            })
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update(
    game: Res<Game>,
    time: Res<Time>,
    assets: Res<BarAssets>,
    cams: Query<&GlobalTransform, With<MainCamera>>,
    vr: Option<Res<crate::vr::XrInput>>,
    rig: Option<Res<crate::vr::XrRig>>,
    globals: Query<&GlobalTransform, Without<WorldBar>>,
    mut bars: Query<(
        &WorldBar,
        &GlobalTransform,
        &mut Transform,
        &mut Visibility,
        &Children,
    )>,
    mut parts: Query<
        (
            &mut Transform,
            &mut MeshMaterial3d<AccentMaterial>,
            Has<BarFill>,
            Has<BarBack>,
            Has<BarLetter>,
        ),
        (Without<WorldBar>, Without<PointRing>),
    >,
    mut rings: Query<(&PointRing, &mut MeshMaterial3d<AccentMaterial>), Without<WorldBar>>,
) {
    // The eyes: the headset's, else the camera's.
    let head = vr
        .as_ref()
        .and_then(|i| i.head)
        .zip(rig.as_ref().and_then(|r| globals.get(r.0).ok()))
        .map(|(h, r)| (r.compute_transform() * h).translation);
    let Some(eye) = head.or_else(|| cams.iter().next().map(|t| t.translation())) else {
        return;
    };
    let blink = (time.elapsed_secs() * 4.0).sin() > 0.0;
    let live = game.ready() && game.alive();
    for (bar, g, mut t, mut vis, children) in &mut bars {
        let at = g.translation();
        let d = (at - eye).length();
        let shown = if live {
            look(&game, bar.of, d, blink)
        } else {
            None
        };
        let want = if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        let Some(l) = shown else { continue };
        // Turned to the eyes (upright), as big to the eye from up to 25 m off.
        let to = Vec3::new(eye.x - at.x, 0.0, eye.z - at.z);
        let k = (d / 25.0).clamp(1.0, 6.0);
        t.rotation = Quat::from_rotation_y(f32::atan2(to.x, to.z));
        t.scale = Vec3::splat(k);
        for &child in children {
            let Ok((mut ct, mut mat, fill, back, letter)) = parts.get_mut(child) else {
                continue;
            };
            let tone = if fill {
                ct.scale.x = (l.frac * bar.width).max(0.001);
                l.fill
            } else if back {
                l.back
            } else if letter {
                l.letter
            } else {
                continue;
            };
            let h = assets.tone(tone);
            if mat.0 != h {
                mat.0 = h;
            }
        }
    }
    for (r, mut mat) in &mut rings {
        let p = game
            .snap
            .bases
            .get(r.0 as usize)
            .and_then(|s| s.points)
            .map(|p| p[r.1 as usize]);
        let tone = match p {
            Some(p) if p.contested && blink => Tone::White,
            Some(p) => Tone::Team(p.owner),
            None => Tone::Dim(None),
        };
        let h = assets.tone(tone);
        if mat.0 != h {
            mat.0 = h;
        }
    }
}
