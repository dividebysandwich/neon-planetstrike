//! Objective markers: the consoles of the bases on the player's continent that matter to its
//! faction, each with a word: HACK (one it may hack: an enemy's or a neutral base its lattice
//! reaches, or a tower), DEFEND (one of its own being hacked), HOLD (one it is hacking: keep the
//! enemy off the console until the hack is done); and the distance. On the flat screen a
//! diamond on the console (pinned to the screen's edge, pointing the way, when it's out of
//! view); in VR a floating label over the console, seen through walls.

use bevy::prelude::*;
use ps_sim::BaseIdx;

use crate::desktop::{MainCamera, Overlays};
use crate::game::Game;
use crate::neonui;

/// Markers at most (the nearest).
const MARKS: usize = 6;
/// How far in from the screen's edge a pinned marker sits (px), and how far over the bottom
/// (the HUD's lines there).
const EDGE: f32 = 40.0;
const BOTTOM: f32 = 190.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    Hack,
    Defend,
    Hold,
}

impl Objective {
    pub fn word(self) -> &'static str {
        match self {
            Objective::Hack => "HACK",
            Objective::Defend => "DEFEND",
            Objective::Hold => "HOLD",
        }
    }

    pub fn rgb(self) -> Vec3 {
        match self {
            Objective::Hack => Vec3::new(1.0, 0.7, 0.15),
            Objective::Defend => Vec3::new(1.0, 0.2, 0.15),
            Objective::Hold => Vec3::new(0.3, 1.0, 0.5),
        }
    }

    fn color(self) -> Color {
        let c = self.rgb();
        Color::srgb(c.x, c.y, c.z)
    }
}

/// The objectives on the player's continent: base, its console, what to do; nearest first.
pub fn objectives(game: &Game, from: Vec3) -> Vec<(BaseIdx, Vec3, Objective)> {
    let (Some(map), Some(c)) = (game.map.as_ref(), game.continent) else {
        return Vec::new();
    };
    let team = game.team;
    let mut out: Vec<(BaseIdx, Vec3, Objective)> = map
        .bases
        .iter()
        .enumerate()
        .filter_map(|(b, l)| {
            let b = b as BaseIdx;
            let (at, _) = l.console?;
            let st = game.state.base(c, b);
            let what = match st.hack {
                Some(h) if st.owner == Some(team) && h.team != team => Objective::Defend,
                Some(h) if h.team == team => Objective::Hold,
                _ if game.state.can_hack(&game.def, c, b, team) => Objective::Hack,
                _ => return None,
            };
            Some((b, at, what))
        })
        .collect();
    out.sort_by(|a, b| (a.1 - from).length().total_cmp(&(b.1 - from).length()));
    out.truncate(MARKS);
    out
}

#[derive(Component)]
struct Mark(usize);

#[derive(Component)]
struct MarkText(usize);

#[derive(Component)]
struct MarksRoot;

pub struct ObjectivesPlugin;

impl Plugin for ObjectivesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn, place).chain().run_if(resource_exists::<Game>),
        )
        .add_systems(Update, clear.run_if(not(resource_exists::<Game>)));
    }
}

fn clear(mut commands: Commands, q: Query<Entity, With<MarksRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn spawn(mut commands: Commands, q: Query<(), With<MarksRoot>>) {
    if !q.is_empty() {
        return;
    }
    commands
        .spawn((
            MarksRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(3),
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            for i in 0..MARKS {
                // A diamond: a square turned a quarter.
                r.spawn((
                    Mark(i),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(14.0),
                        height: Val::Px(14.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::degrees(45.0)),
                    BorderColor::all(Color::WHITE),
                    Visibility::Hidden,
                ));
                r.spawn((
                    MarkText(i),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    neonui::text_w("", 12.0, Color::WHITE, 700),
                    TextLayout::justify(Justify::Center),
                    Visibility::Hidden,
                ));
            }
        });
}

/// Where a point shows on the screen: on it, or pinned to its edge (the way to it).
fn screen_spot(cam: &Camera, t: &GlobalTransform, at: Vec3, size: Vec2) -> Option<(Vec2, bool)> {
    let local = t.affine().inverse().transform_point3(at);
    let behind = local.z > 0.0;
    if !behind
        && let Ok(p) = cam.world_to_viewport(t, at)
        && p.x > EDGE
        && p.y > EDGE
        && p.x < size.x - EDGE
        && p.y < size.y - EDGE
    {
        return Some((p, true));
    }
    // Off the screen: from its middle towards it (behind: the side it's on, low).
    let mid = size / 2.0;
    let mut d = Vec2::new(local.x, -local.y);
    if behind {
        d.y = d.y.abs().max(0.3 * d.x.abs()) + 1.0;
    }
    let d = d.normalize_or(Vec2::Y);
    let half = mid - Vec2::splat(EDGE);
    let k = (half.x / d.x.abs().max(1e-4)).min(half.y / d.y.abs().max(1e-4));
    Some((mid + d * k, false))
}

#[allow(clippy::type_complexity)]
fn place(
    game: Res<Game>,
    overlays: Res<Overlays>,
    cams: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut marks: Query<(&Mark, &mut Node, &mut BorderColor, &mut Visibility), Without<MarkText>>,
    mut texts: Query<
        (
            &MarkText,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut Visibility,
        ),
        Without<Mark>,
    >,
) {
    let cam = cams.single().ok();
    let size = cam
        .and_then(|(c, _)| c.logical_viewport_size())
        .unwrap_or(Vec2::new(1280.0, 720.0));
    let eye = cam.map_or(Vec3::ZERO, |c| c.1.translation());
    let me = game.mine().map_or(eye, |m| m.feet);
    let list = if game.ready() && game.alive() && !overlays.map {
        objectives(&game, me)
    } else {
        Vec::new()
    };
    let mut spots: Vec<Option<(Vec2, bool, Objective, f32)>> = list
        .iter()
        .map(|(_, at, what)| {
            let (c, t) = cam?;
            let (mut p, on) = screen_spot(c, t, *at + Vec3::Y * 1.2, size)?;
            if !on {
                // (Pinned ones keep above the HUD's bottom lines.)
                p.y = p.y.min(size.y - BOTTOM);
            }
            Some((p, on, *what, (*at - me).length()))
        })
        .collect();
    // Markers on top of each other: the later ones moved along (down, or up near the bottom).
    for i in 0..spots.len() {
        let Some((mut p, ..)) = spots[i] else {
            continue;
        };
        for _ in 0..MARKS {
            let crowded = spots[..i]
                .iter()
                .flatten()
                .any(|q| (q.0 - p).length() < 34.0);
            if !crowded {
                break;
            }
            p.y += if p.y > size.y - BOTTOM - 40.0 {
                -36.0
            } else {
                36.0
            };
        }
        if let Some(s) = spots[i].as_mut() {
            s.0 = p;
        }
    }
    let set_vis = |v: &mut Visibility, on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    };
    for (m, mut n, mut border, mut vis) in &mut marks {
        let Some(Some((p, _, what, _))) = spots.get(m.0) else {
            set_vis(&mut vis, false);
            continue;
        };
        set_vis(&mut vis, true);
        n.left = Val::Px(p.x - 7.0);
        n.top = Val::Px(p.y - 7.0);
        *border = BorderColor::all(what.color());
    }
    for (m, mut n, mut text, mut color, mut vis) in &mut texts {
        let Some(Some((p, on, what, d))) = spots.get(m.0) else {
            set_vis(&mut vis, false);
            continue;
        };
        set_vis(&mut vis, true);
        let s = if *on {
            format!("{}\n{:.0} m", what.word(), d)
        } else {
            what.word().to_string()
        };
        // (Centred under the diamond, roughly: the text's width isn't known here.)
        let w = 7.5 * s.lines().map(str::len).max().unwrap_or(0) as f32;
        n.left = Val::Px(p.x - w / 2.0);
        n.top = Val::Px(p.y + 10.0);
        if text.0 != s {
            text.0 = s;
        }
        color.0 = what.color();
    }
}

/// VR: a floating label over each objective's console (its word and distance), turned to the
/// eyes, drawn over everything (it shows through walls).
#[derive(Default)]
pub(crate) struct VrMarks {
    shown: Vec<(Entity, String, Objective)>,
    mats: Option<[Handle<crate::materials::AccentMaterial>; 3]>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn vr_marks(
    mut commands: Commands,
    mut st: Local<VrMarks>,
    game: Option<Res<Game>>,
    input: Res<crate::vr::XrInput>,
    rig: Res<crate::vr::XrRig>,
    globals: Query<&GlobalTransform>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<crate::materials::AccentMaterial>>,
    mut transforms: Query<&mut Transform>,
) {
    let mats = st
        .mats
        .get_or_insert_with(|| {
            [Objective::Hack, Objective::Defend, Objective::Hold].map(|o| {
                accents.add(crate::materials::AccentMaterial {
                    neon: crate::materials::NeonUniform::fixed(o.rgb(), 2.0),
                    overlay: true,
                })
            })
        })
        .clone();
    let head = input
        .head
        .zip(globals.get(rig.0).ok())
        .map(|(h, r)| (r.compute_transform() * h).translation);
    let list = match (game.as_deref(), head) {
        (Some(g), Some(_)) if g.ready() && g.alive() => {
            let me = g.mine().map_or(Vec3::ZERO, |m| m.feet);
            objectives(g, me)
                .into_iter()
                .map(|(_, at, what)| (at, what, (at - me).length()))
                .collect()
        }
        _ => Vec::new(),
    };
    // Labels as many as the objectives, their text as it is now.
    while st.shown.len() > list.len() {
        if let Some((e, ..)) = st.shown.pop() {
            commands.entity(e).despawn();
        }
    }
    for (i, &(at, what, d)) in list.iter().enumerate() {
        let text = format!("{} {:.0} m", what.word(), (d / 10.0).round() * 10.0);
        let same = st.shown.get(i).is_some_and(|s| s.1 == text && s.2 == what);
        if !same {
            if i < st.shown.len() {
                let (e, ..) = st.shown.remove(i);
                commands.entity(e).despawn();
            }
            // (Bigger the further: about as big to the eye at any distance.)
            let h = 0.06;
            let w = crate::vfont::width(&text, h, 700);
            let mut mesh = crate::vfont::mesh(&text, h, 700);
            for p in mesh.pos.iter_mut() {
                p[0] -= w / 2.0;
            }
            let idx = match what {
                Objective::Hack => 0,
                Objective::Defend => 1,
                Objective::Hold => 2,
            };
            let e = commands
                .spawn((
                    Mesh3d(meshes.add(mesh.to_mesh())),
                    MeshMaterial3d(mats[idx].clone()),
                    Transform::from_translation(at),
                ))
                .id();
            st.shown.insert(i, (e, text, what));
        }
        if let (Some(head), Some((e, ..))) = (head, st.shown.get(i))
            && let Ok(mut t) = transforms.get_mut(*e)
        {
            let pos = at + Vec3::Y * 2.2;
            let scale = ((pos - head).length() / 12.0).clamp(1.0, 25.0);
            *t = Transform::from_translation(pos)
                .looking_at(pos + (pos - head), Vec3::Y)
                .with_scale(Vec3::splat(scale));
        }
    }
}
