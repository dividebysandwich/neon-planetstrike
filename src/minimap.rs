//! The minimap: the land round the player (north up on the flat screen, the player's arrow
//! turning; in VR turned the way the hand points), with the bases in their
//! owners' colours, the warp gates, resource nodes, the faction's players and vehicles (squad
//! mates in the squad's colour, the downed in a medic's green), the squad's waypoint (at the
//! edge when it's further), enemy vehicles in sight and enemies giving themselves away by
//! firing. On the flat screen in the
//! HUD's bottom right corner (here); in VR on the back of the left hand (src/vrui.rs), from
//! the same `blips`.

use bevy::prelude::*;
use ps_sim::BaseIdx;
use ps_sim::snapshot::{flags, vflags};
use ps_sim::terrain::{HALF, MAP_SIZE};

use crate::game::{Game, Mover};
use crate::materials::team_color;

/// What the minimap shows round the player (m, from the middle to the edge).
pub const RANGE: f32 = 260.0;
/// The flat screen's minimap (px).
const PX: f32 = 210.0;

/// Something on the minimap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlipKind {
    Base,
    Gate,
    Node,
    Friend,
    /// A squad mate.
    Squad,
    /// A team mate a medic may revive.
    Downed,
    /// The squad's waypoint.
    Waypoint,
    FriendVehicle,
    Enemy,
    EnemyVehicle,
}

#[derive(Clone, Copy, Debug)]
pub struct Blip {
    /// World XZ.
    pub at: Vec2,
    pub kind: BlipKind,
    pub color: Vec3,
}

impl BlipKind {
    /// Its size relative to a player's dot.
    pub fn size(self) -> f32 {
        match self {
            BlipKind::Base => 2.6,
            BlipKind::Gate => 2.4,
            BlipKind::Node => 1.4,
            BlipKind::FriendVehicle | BlipKind::EnemyVehicle => 1.6,
            BlipKind::Friend | BlipKind::Enemy => 1.0,
            BlipKind::Squad | BlipKind::Downed => 1.3,
            BlipKind::Waypoint => 2.2,
        }
    }
}

/// Where the player is (world) and which way it faces (yaw).
pub fn here(game: &Game, mover: &Mover, view: &crate::seatview::SeatView) -> Option<(Vec3, f32)> {
    let me = game.me_snap()?;
    let at = if view.active {
        view.eye
    } else if mover.valid {
        mover.body.feet
    } else {
        me.feet
    };
    let yaw = if view.active {
        ps_sim::yaw_of(view.rot * Vec3::NEG_Z)
    } else {
        me.yaw
    };
    Some((at, yaw))
}

/// What's on the minimap within `range` of `at`.
pub fn blips(game: &Game, at: Vec3, range: f32) -> Vec<Blip> {
    let mut out = Vec::new();
    let Some(map) = &game.map else { return out };
    let near = |p: Vec3| Vec2::new(p.x - at.x, p.z - at.z).length() < range * 1.5;
    let push = |out: &mut Vec<Blip>, p: Vec3, kind: BlipKind, color: Vec3| {
        if near(p) {
            out.push(Blip {
                at: Vec2::new(p.x, p.z),
                kind,
                color,
            });
        }
    };
    let c = map.def.id;
    for (bi, b) in map.bases.iter().enumerate() {
        let owner = game.state.base(c, bi as BaseIdx).owner;
        push(&mut out, b.center, BlipKind::Base, team_color(owner));
    }
    for g in &map.gates {
        push(&mut out, *g, BlipKind::Gate, Vec3::new(0.4, 0.8, 1.0));
    }
    for n in &game.snap.nodes {
        push(&mut out, n.pos, BlipKind::Node, Vec3::new(1.0, 0.6, 0.1));
    }
    let mine = game.team;
    for v in &game.snap.vehicles {
        if v.flags & vflags::CLOAKED != 0 && v.team != mine {
            continue;
        }
        let kind = if v.team == mine {
            BlipKind::FriendVehicle
        } else {
            BlipKind::EnemyVehicle
        };
        push(&mut out, v.pos, kind, team_color(Some(v.team)));
    }
    for p in &game.snap.players {
        if Some(p.id) == game.me || p.seat.is_some() {
            continue;
        }
        if p.has(flags::DOWNED) && p.team == mine {
            push(&mut out, p.feet, BlipKind::Downed, crate::squad::DOWNED_RGB);
            continue;
        }
        if p.has(flags::DEAD) {
            continue;
        }
        if game.squad_mate(p.id) {
            push(&mut out, p.feet, BlipKind::Squad, crate::squad::SQUAD_RGB);
        } else if p.team == mine {
            push(&mut out, p.feet, BlipKind::Friend, team_color(Some(p.team)));
        } else if p.has(flags::FIRING) || p.has(flags::SPOTTED) {
            push(&mut out, p.feet, BlipKind::Enemy, team_color(Some(p.team)));
        }
    }
    // The waypoint: at the edge, the way to go, when it's beyond the range.
    if let Some(w) = game.waypoint_here() {
        let d = Vec2::new(w.x - at.x, w.z - at.z);
        let d = if d.length() > range * 0.92 {
            d.normalize() * range * 0.92
        } else {
            d
        };
        out.push(Blip {
            at: Vec2::new(at.x, at.z) + d,
            kind: BlipKind::Waypoint,
            color: crate::squad::SQUAD_RGB,
        });
    }
    out
}

/// A blip's colour on screen.
fn ui_color(c: Vec3, kind: BlipKind) -> Color {
    let k = if matches!(kind, BlipKind::Base | BlipKind::Gate) {
        0.85
    } else {
        1.0
    };
    Color::linear_rgb(c.x * k, c.y * k, c.z * k)
}

#[derive(Component)]
struct MiniRoot;

#[derive(Component)]
struct MiniLand;

/// The player's arrow (turned to where it faces).
#[derive(Component)]
struct MiniMe;

/// The minimap's own picture of the land: finer than the map's.
const IMAGE_PX: usize = 768;

#[derive(Resource, Default)]
struct MiniState {
    /// The blips' nodes (reused frame to frame), and the land's picture (its continent).
    pool: Vec<Entity>,
    image: Option<(ps_sim::ContinentId, Handle<Image>)>,
}

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiniState>()
            .add_systems(
                Update,
                (spawn, update)
                    .chain()
                    .after(crate::seatview::SeatViewSet)
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, hide.run_if(not(resource_exists::<Game>)));
    }
}

fn hide(mut commands: Commands, q: Query<Entity, With<MiniRoot>>, mut st: ResMut<MiniState>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    st.pool.clear();
    st.image = None;
}

fn spawn(mut commands: Commands, q: Query<(), With<MiniRoot>>) {
    if !q.is_empty() {
        return;
    }
    commands
        .spawn((
            MiniRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                width: Val::Px(PX + 4.0),
                height: Val::Px(PX + 4.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(crate::neonui::CYAN.with_alpha(0.55)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.85)),
            GlobalZIndex(5),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|r| {
            r.spawn((
                MiniLand,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(PX),
                    height: Val::Px(PX),
                    ..default()
                },
            ));
            // The player in the middle: a diamond with a line pointing where it faces.
            r.spawn((
                MiniMe,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(PX / 2.0 - 12.0),
                    top: Val::Px(PX / 2.0 - 12.0),
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                GlobalZIndex(6),
            ))
            .with_children(|m| {
                m.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(7.0),
                        top: Val::Px(7.0),
                        width: Val::Px(10.0),
                        height: Val::Px(10.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::degrees(45.0)),
                    BorderColor::all(Color::WHITE),
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                ));
                m.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(11.0),
                        top: Val::Px(-4.0),
                        width: Val::Px(2.0),
                        height: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                ));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    game: Res<Game>,
    mover: Res<Mover>,
    view: Res<crate::seatview::SeatView>,
    mut images: ResMut<Assets<Image>>,
    mut st: ResMut<MiniState>,
    mut root: Query<(Entity, &mut Visibility), With<MiniRoot>>,
    mut me_q: Query<&mut UiTransform, With<MiniMe>>,
    mut land: Query<(Entity, Option<&mut ImageNode>), With<MiniLand>>,
    mut nodes: Query<(&mut Node, &mut BackgroundColor), (Without<MiniLand>, Without<MiniMe>)>,
    mut vis: Query<&mut Visibility, Without<MiniRoot>>,
) {
    let here = here(&game, &mover, &view).filter(|_| game.alive() && game.ready());
    let Ok((root_e, mut rv)) = root.single_mut() else {
        return;
    };
    let want = if here.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *rv != want {
        *rv = want;
    }
    let Some((at, yaw)) = here else { return };
    let Some(map) = &game.map else { return };
    let Ok((land_e, image_node)) = land.single_mut() else {
        return;
    };
    // The land: a picture of the continent, of which the part round the player is shown.
    if st.image.as_ref().is_none_or(|i| i.0 != map.def.id) {
        let h = images.add(crate::map::land_image(map, IMAGE_PX));
        st.image = Some((map.def.id, h.clone()));
        commands.entity(land_e).insert(ImageNode::new(h));
        return;
    }
    let px_per_m = IMAGE_PX as f32 / MAP_SIZE;
    let c = (Vec2::new(at.x, at.z) + Vec2::splat(HALF)) * px_per_m;
    let half = RANGE * px_per_m;
    if let Some(mut img) = image_node {
        img.rect = Some(Rect::from_center_half_size(c, Vec2::splat(half)));
    }
    // North up; the arrow turns with the player (yaw: anticlockwise from north).
    if let Ok(mut t) = me_q.single_mut() {
        t.rotation = Rot2::radians(-yaw);
    }
    let to_px = |p: Vec2| (p - Vec2::new(at.x, at.z)) / (2.0 * RANGE) * PX + Vec2::splat(PX / 2.0);
    let list = blips(&game, at, RANGE);
    while st.pool.len() < list.len() {
        let e = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                ChildOf(root_e),
            ))
            .id();
        st.pool.push(e);
    }
    for (i, &e) in st.pool.iter().enumerate() {
        let shown = list.get(i).filter(|b| {
            let p = to_px(b.at);
            p.x > -6.0 && p.y > -6.0 && p.x < PX + 6.0 && p.y < PX + 6.0
        });
        if let Ok(mut v) = vis.get_mut(e) {
            let want = if shown.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *v != want {
                *v = want;
            }
        }
        let Some(b) = shown else { continue };
        if let Ok((mut n, mut bg)) = nodes.get_mut(e) {
            let s = 5.0 * b.kind.size();
            let p = to_px(b.at);
            n.left = Val::Px(p.x - s / 2.0);
            n.top = Val::Px(p.y - s / 2.0);
            n.width = Val::Px(s);
            n.height = Val::Px(s);
            let square = matches!(
                b.kind,
                BlipKind::Base
                    | BlipKind::FriendVehicle
                    | BlipKind::EnemyVehicle
                    | BlipKind::Waypoint
            );
            n.border_radius = if square {
                BorderRadius::ZERO
            } else {
                BorderRadius::all(Val::Px(s / 2.0))
            };
            bg.0 = ui_color(b.color, b.kind);
        }
    }
}
