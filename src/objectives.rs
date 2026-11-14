//! Objective markers: what matters to the player's faction on its continent, each with a word
//! and the distance. The consoles: HACK (one it may hack: an enemy's or a neutral base its
//! lattice reaches, or a tower), DEFEND (one of its own being hacked), HOLD (one it is hacking:
//! keep the enemy off the console until the hack is done). Capture points: CAPTURE A (one not
//! its faction's at a base it may take), DEFEND B (one of its own taken or being taken), HOLD C
//! (one it holds while its capture runs). Near the player: generators and wall turrets to
//! DESTROY (at a base it may take) or REPAIR (its own, damaged). On the flat screen a mark on
//! the objective (a diamond on a console, a circle on a point, a square on a generator or a
//! turret; pinned to the screen's edge, pointing the way, when it's out of view); in VR a
//! floating label over it, seen through walls.

use bevy::prelude::*;
use ps_sim::BaseIdx;

use crate::desktop::{MainCamera, Overlays};
use crate::game::Game;
use crate::neonui;

/// Markers at most (the nearest, the bases' consoles and points first).
const MARKS: usize = 10;
/// How near generators and turrets are marked (m).
const NEAR_KIT: f32 = 250.0;
/// How far in from the screen's edge a pinned marker sits (px), and how far over the bottom
/// (the HUD's lines there).
const EDGE: f32 = 40.0;
const BOTTOM: f32 = 190.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    Hack,
    Defend,
    Hold,
    Capture,
    Destroy,
    Repair,
}

impl Objective {
    pub const ALL: [Objective; 6] = [
        Objective::Hack,
        Objective::Defend,
        Objective::Hold,
        Objective::Capture,
        Objective::Destroy,
        Objective::Repair,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Objective::Hack => "HACK",
            Objective::Defend => "DEFEND",
            Objective::Hold => "HOLD",
            Objective::Capture => "CAPTURE",
            Objective::Destroy => "DESTROY",
            Objective::Repair => "REPAIR",
        }
    }

    pub fn rgb(self) -> Vec3 {
        match self {
            Objective::Hack | Objective::Capture => Vec3::new(1.0, 0.7, 0.15),
            Objective::Defend => Vec3::new(1.0, 0.2, 0.15),
            Objective::Hold => Vec3::new(0.3, 1.0, 0.5),
            Objective::Destroy => Vec3::new(1.0, 0.45, 0.1),
            Objective::Repair => Vec3::new(0.2, 0.85, 1.0),
        }
    }

    fn color(self) -> Color {
        let c = self.rgb();
        Color::srgb(c.x, c.y, c.z)
    }
}

/// What an objective is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjKind {
    Console,
    Point(u8),
    Generator,
    Turret,
}

/// An objective: its base, where (its mark goes a little over it), what it is and what to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obj {
    pub base: BaseIdx,
    pub at: Vec3,
    pub kind: ObjKind,
    pub what: Objective,
}

impl Obj {
    /// Its word (with the point's letter).
    pub fn label(&self) -> String {
        match self.kind {
            ObjKind::Point(i) => format!(
                "{} {}",
                self.what.word(),
                ps_sim::base::POINT_NAMES[i as usize % 3]
            ),
            _ => self.what.word().to_string(),
        }
    }

    fn rank(&self) -> u8 {
        match self.kind {
            ObjKind::Console | ObjKind::Point(_) => 0,
            ObjKind::Generator => 1,
            ObjKind::Turret => 2,
        }
    }
}

/// The objectives on the player's continent, the bases' consoles and points first, then
/// generators and turrets near `from`; nearest first.
pub fn objectives(game: &Game, from: Vec3) -> Vec<Obj> {
    let (Some(map), Some(c)) = (game.map.as_ref(), game.continent) else {
        return Vec::new();
    };
    let team = game.team;
    let mut out: Vec<Obj> = Vec::new();
    for (b, l) in map.bases.iter().enumerate() {
        let b = b as BaseIdx;
        let st = game.state.base(c, b);
        let ours = st.owner == Some(team);
        let may = game.state.can_hack(&game.def, c, b, team);
        let our_hack = st.hack.is_some_and(|h| h.team == team);
        if let Some((at, _)) = l.console {
            let what = match st.hack {
                Some(h) if ours && h.team != team => Some(Objective::Defend),
                Some(h) if h.team == team => Some(Objective::Hold),
                _ if may => Some(Objective::Hack),
                _ => None,
            };
            if let Some(what) = what {
                out.push(Obj {
                    base: b,
                    at,
                    kind: ObjKind::Console,
                    what,
                });
            }
        }
        let points = game.snap.bases.get(b as usize).and_then(|s| s.points);
        for (i, pt) in l.points.iter().enumerate() {
            let Some(p) = points.map(|p| p[i]) else { break };
            let mine = p.owner == Some(team);
            let threat = p.by.is_some_and(|t| t != team) || p.contested;
            let what = if ours {
                (!mine || threat).then_some(Objective::Defend)
            } else if our_hack && mine {
                Some(if threat {
                    Objective::Defend
                } else {
                    Objective::Hold
                })
            } else if (may || our_hack) && !mine {
                Some(Objective::Capture)
            } else {
                None
            };
            if let Some(what) = what {
                out.push(Obj {
                    base: b,
                    at: pt.pylon + Vec3::Y * (ps_sim::base::PYLON_H + 0.2),
                    kind: ObjKind::Point(i as u8),
                    what,
                });
            }
        }
        // Generators and turrets near: to take down at a base we may take, to mend at ours.
        if (l.center - from).length() > NEAR_KIT + l.half {
            continue;
        }
        let enemy = !ours && may && st.owner.is_some();
        if let (Some(g), Some(s)) = (l.generator, game.snap.bases.get(b as usize)) {
            let what = if enemy && s.generator > 0.0 {
                Some(Objective::Destroy)
            } else if ours && s.generator < 100.0 {
                Some(Objective::Repair)
            } else {
                None
            };
            if let Some(what) = what
                && (g - from).length() < NEAR_KIT
            {
                out.push(Obj {
                    base: b,
                    at: g + Vec3::Y * 2.0,
                    kind: ObjKind::Generator,
                    what,
                });
            }
        }
        for t in game.snap.turrets.iter().filter(|t| t.base == b) {
            let Some(&at) = l.turrets.get(t.index as usize) else {
                continue;
            };
            let what = if enemy && t.health > 0.0 && t.team != Some(team) {
                Some(Objective::Destroy)
            } else if ours && t.health < 100.0 {
                Some(Objective::Repair)
            } else {
                None
            };
            if let Some(what) = what
                && (at - from).length() < NEAR_KIT * 0.6
            {
                out.push(Obj {
                    base: b,
                    at: at + Vec3::Y * ps_sim::base::TURRET_PIVOT,
                    kind: ObjKind::Turret,
                    what,
                });
            }
        }
    }
    // The nearest of the bases' own objectives, then (in the room left, at least a few) the
    // nearest generators and turrets.
    out.sort_by(|a, b| (a.at - from).length().total_cmp(&(b.at - from).length()));
    let (main, kit): (Vec<Obj>, Vec<Obj>) = out.into_iter().partition(|o| o.rank() == 0);
    let keep = main.len().min(MARKS - kit.len().min(4));
    let mut out: Vec<Obj> = main.into_iter().take(keep).collect();
    out.extend(kit.into_iter().take(MARKS - keep));
    out.sort_by(|a, b| (a.at - from).length().total_cmp(&(b.at - from).length()));
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
    mut marks: Query<
        (
            &Mark,
            &mut Node,
            &mut BorderColor,
            &mut UiTransform,
            &mut Visibility,
        ),
        Without<MarkText>,
    >,
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
    let mut spots: Vec<Option<(Vec2, bool, Obj, f32)>> = list
        .iter()
        .map(|o| {
            let (c, t) = cam?;
            let (mut p, on) = screen_spot(c, t, o.at + Vec3::Y * 1.2, size)?;
            if !on {
                // (Pinned ones keep above the HUD's bottom lines.)
                p.y = p.y.min(size.y - BOTTOM);
            }
            Some((p, on, *o, (o.at - me).length()))
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
    for (m, mut n, mut border, mut turn, mut vis) in &mut marks {
        let Some(Some((p, _, o, _))) = spots.get(m.0) else {
            set_vis(&mut vis, false);
            continue;
        };
        set_vis(&mut vis, true);
        // A diamond on a console, a circle on a point, a square on the rest.
        let (size, deg, round) = match o.kind {
            ObjKind::Console => (14.0, 45.0, 0.0),
            ObjKind::Point(_) => (16.0, 0.0, 8.0),
            ObjKind::Generator => (13.0, 0.0, 0.0),
            ObjKind::Turret => (10.0, 0.0, 0.0),
        };
        n.width = Val::Px(size);
        n.height = Val::Px(size);
        n.border_radius = BorderRadius::all(Val::Px(round));
        n.left = Val::Px(p.x - size / 2.0);
        n.top = Val::Px(p.y - size / 2.0);
        let rot = Rot2::degrees(deg);
        if turn.rotation != rot {
            *turn = UiTransform::from_rotation(rot);
        }
        *border = BorderColor::all(o.what.color());
    }
    for (m, mut n, mut text, mut color, mut vis) in &mut texts {
        let Some(Some((p, on, o, d))) = spots.get(m.0) else {
            set_vis(&mut vis, false);
            continue;
        };
        set_vis(&mut vis, true);
        let what = o.what;
        let s = if *on {
            format!("{}\n{:.0} m", o.label(), d)
        } else {
            o.label()
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
    mats: Option<[Handle<crate::materials::AccentMaterial>; 6]>,
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
            Objective::ALL.map(|o| {
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
                .map(|o| (o.at, o.what, (o.at - me).length(), o.label()))
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
    for (i, (at, what, d, label)) in list.iter().enumerate() {
        let (at, what, d) = (*at, *what, *d);
        let text = format!("{label} {:.0} m", (d / 10.0).round() * 10.0);
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
            let idx = Objective::ALL.iter().position(|o| *o == what).unwrap_or(0);
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
