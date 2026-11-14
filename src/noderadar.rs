//! The harvester's node radar (flat screen): while driving or riding a harvester, a round
//! scope left of the minimap shows the resource nodes out to `RANGE`, turned the way the
//! harvester faces (up: ahead), with a sweep going round. A node's blip grows with what's
//! left in it; one in the beam's reach shows green; one past the scope's edge sits on the
//! rim. Below it: how far the nearest node is, and which way.

use bevy::prelude::*;

use crate::game::Game;
use crate::neonui;

/// How far the scope sees (m, middle to rim): about as far as the server tells of nodes.
const RANGE: f32 = 1100.0;
/// The scope (px across).
const PX: f32 = 180.0;
/// Inside the scope's 2 px rim (px): where its children are placed.
const INNER: f32 = PX - 4.0;
/// One sweep round (s).
const SWEEP: f32 = 2.5;

#[derive(Component)]
struct RadarRoot;

#[derive(Component)]
struct RadarScope;

#[derive(Component)]
struct RadarSweep;

#[derive(Component)]
struct RadarLabel;

/// A pointer on the rim towards one of the nearest nodes (`.0`: 0 the nearest): a square the
/// size of the scope turned towards it, with an arc (`RadarPointerArc`) at its top.
#[derive(Component)]
struct RadarPointer(usize);

#[derive(Component)]
struct RadarPointerArc(usize);

/// How many of the nearest nodes get a pointer, and how far off (m) a pointer fades to its
/// faintest.
const POINTERS: usize = 3;
const POINTER_FAR: f32 = 1600.0;

/// The blips' nodes, reused frame to frame.
#[derive(Resource, Default)]
struct RadarState {
    pool: Vec<Entity>,
}

pub struct NodeRadarPlugin;

impl Plugin for NodeRadarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RadarState>()
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

fn hide(mut commands: Commands, q: Query<Entity, With<RadarRoot>>, mut st: ResMut<RadarState>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    st.pool.clear();
}

fn spawn(mut commands: Commands, q: Query<(), With<RadarRoot>>) {
    if !q.is_empty() {
        return;
    }
    let ring = |d: f32, a: f32| {
        (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((INNER - d) / 2.0),
                top: Val::Px((INNER - d) / 2.0),
                width: Val::Px(d),
                height: Val::Px(d),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(d / 2.0)),
                ..default()
            },
            BorderColor::all(neonui::AMBER.with_alpha(a)),
        )
    };
    commands
        .spawn((
            RadarRoot,
            Node {
                position_type: PositionType::Absolute,
                // (Left of the minimap: src/minimap.rs.)
                right: Val::Px(24.0 + 214.0 + 18.0),
                bottom: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            GlobalZIndex(5),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|r| {
            r.spawn((RadarLabel, neonui::text("", 15.0, neonui::AMBER)));
            r.spawn((
                RadarScope,
                Node {
                    width: Val::Px(PX),
                    height: Val::Px(PX),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(PX / 2.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(neonui::AMBER.with_alpha(0.7)),
                BackgroundColor(Color::srgba(0.02, 0.01, 0.0, 0.8)),
            ))
            .with_children(|s| {
                s.spawn(ring(PX * 2.0 / 3.0, 0.25));
                s.spawn(ring(PX / 3.0, 0.25));
                // The cross hairs, and a tick ahead.
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(INNER / 2.0 - 0.5),
                        top: Val::Px(0.0),
                        width: Val::Px(1.0),
                        height: Val::Px(INNER),
                        ..default()
                    },
                    BackgroundColor(neonui::AMBER.with_alpha(0.15)),
                ));
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(INNER / 2.0 - 0.5),
                        width: Val::Px(INNER),
                        height: Val::Px(1.0),
                        ..default()
                    },
                    BackgroundColor(neonui::AMBER.with_alpha(0.15)),
                ));
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(INNER / 2.0 - 1.5),
                        top: Val::Px(0.0),
                        width: Val::Px(3.0),
                        height: Val::Px(9.0),
                        ..default()
                    },
                    BackgroundColor(neonui::AMBER.with_alpha(0.8)),
                ));
                // The sweep: a square the size of the scope turning, a line from its middle
                // to its top.
                s.spawn((
                    RadarSweep,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(INNER),
                        height: Val::Px(INNER),
                        ..default()
                    },
                    UiTransform::IDENTITY,
                ))
                .with_children(|w| {
                    for (k, a) in [(0.0, 0.5), (1.0, 0.25), (2.0, 0.12)] {
                        w.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(INNER / 2.0 - 1.0 - k * 2.0),
                                top: Val::Px(0.0),
                                width: Val::Px(2.0),
                                height: Val::Px(INNER / 2.0),
                                ..default()
                            },
                            BackgroundColor(neonui::AMBER.with_alpha(a)),
                        ));
                    }
                });
                // Pointers on the rim towards the nearest nodes.
                for i in 0..POINTERS {
                    s.spawn((
                        RadarPointer(i),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(INNER),
                            height: Val::Px(INNER),
                            ..default()
                        },
                        UiTransform::IDENTITY,
                        Visibility::Hidden,
                        GlobalZIndex(6),
                    ))
                    .with_children(|w| {
                        w.spawn((
                            RadarPointerArc(i),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(INNER / 2.0 - 15.0),
                                top: Val::Px(1.0),
                                width: Val::Px(30.0),
                                height: Val::Px(5.0),
                                border_radius: BorderRadius::all(Val::Px(2.5)),
                                ..default()
                            },
                            BackgroundColor(neonui::AMBER),
                        ));
                    });
                }
                // The harvester in the middle.
                s.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(INNER / 2.0 - 4.0),
                        top: Val::Px(INNER / 2.0 - 4.0),
                        width: Val::Px(8.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::degrees(45.0)),
                    BackgroundColor(Color::WHITE),
                    GlobalZIndex(7),
                ));
            });
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update(
    mut commands: Commands,
    game: Res<Game>,
    time: Res<Time>,
    view: Res<crate::seatview::SeatView>,
    mut st: ResMut<RadarState>,
    mut root: Query<&mut Visibility, With<RadarRoot>>,
    scope: Query<Entity, With<RadarScope>>,
    mut sweep: Query<&mut UiTransform, With<RadarSweep>>,
    mut label: Query<(&mut Text, &mut TextColor), With<RadarLabel>>,
    mut nodes: Query<
        (&mut Node, &mut BackgroundColor),
        (Without<RadarScope>, Without<RadarPointerArc>),
    >,
    mut vis: Query<&mut Visibility, (Without<RadarRoot>, Without<RadarPointer>)>,
    mut pointers: Query<
        (&RadarPointer, &mut UiTransform, &mut Visibility),
        (Without<RadarSweep>, Without<RadarRoot>),
    >,
    mut arcs: Query<(&RadarPointerArc, &mut Node, &mut BackgroundColor), Without<RadarScope>>,
) {
    let on = game.alive() && game.ready() && view.active && view.cargo.is_some();
    let Ok(mut rv) = root.single_mut() else {
        return;
    };
    let want = if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *rv != want {
        *rv = want;
    }
    if !on {
        return;
    }
    let Ok(scope_e) = scope.single() else { return };
    let now = time.elapsed_secs();
    if let Ok(mut t) = sweep.single_mut() {
        t.rotation = Rot2::radians(now / SWEEP * std::f32::consts::TAU);
    }
    // Heading up: the harvester's own facing (not the gunner's look) at the top.
    let at = view.eye;
    let fwd = view.level * Vec3::NEG_Z;
    let fwd = Vec2::new(fwd.x, fwd.z).normalize_or(Vec2::NEG_Y);
    // (Screen: x right, y down; ahead is up.)
    let right = Vec2::new(-fwd.y, fwd.x);
    let to_scope = |d: Vec2| Vec2::new(d.dot(right), -d.dot(fwd));
    let reach = ps_sim::sim::HARVEST_REACH;
    let r_px = INNER / 2.0;
    let list: Vec<(Vec2, f32, u16)> = game
        .snap
        .nodes
        .iter()
        .map(|n| {
            let d = Vec2::new(n.pos.x - at.x, n.pos.z - at.z);
            (to_scope(d), d.length(), n.amount)
        })
        .collect();
    while st.pool.len() < list.len() {
        let e = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                GlobalZIndex(6),
                ChildOf(scope_e),
            ))
            .id();
        st.pool.push(e);
    }
    for (i, &e) in st.pool.iter().enumerate() {
        let b = list.get(i);
        if let Ok(mut v) = vis.get_mut(e) {
            let want = if b.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *v != want {
                *v = want;
            }
        }
        let Some(&(dir, dist, amount)) = b else {
            continue;
        };
        let Ok((mut n, mut bg)) = nodes.get_mut(e) else {
            continue;
        };
        let outside = dist > RANGE;
        let p = if outside {
            dir.normalize_or_zero() * (r_px - 4.0)
        } else {
            dir / RANGE * r_px
        };
        let in_reach = dist < reach;
        // (Bigger the more it holds, up to the biggest a node grows.)
        let big = ps_sim::sim::NODE_SIZES[2] * ps_sim::sim::CARGO;
        let mut s = 5.0 + (amount as f32 / big).clamp(0.0, 1.0) * 6.0;
        if outside {
            s = 5.0;
        }
        if in_reach {
            s += 2.0 * (now * 6.0).sin().abs();
        }
        let c = Vec2::splat(INNER / 2.0) + p;
        n.left = Val::Px(c.x - s / 2.0);
        n.top = Val::Px(c.y - s / 2.0);
        n.width = Val::Px(s);
        n.height = Val::Px(s);
        n.border_radius = BorderRadius::all(Val::Px(s / 2.0));
        // (Brightest just behind the sweep, as it passes.)
        let ang = (p.x.atan2(-p.y) - now / SWEEP * std::f32::consts::TAU)
            .rem_euclid(std::f32::consts::TAU);
        let glow = 0.65 + 0.35 * (1.0 - ang / std::f32::consts::TAU).powi(3);
        bg.0 = if in_reach {
            neonui::GREEN
        } else if outside {
            neonui::AMBER.with_alpha(0.45)
        } else {
            neonui::AMBER.with_alpha(glow)
        };
    }
    // Pointers on the rim towards the nearest nodes (not one already in reach): the nearer,
    // the brighter and longer.
    let mut near: Vec<(Vec2, f32)> = list
        .iter()
        .filter(|n| n.1 >= reach)
        .map(|n| (n.0, n.1))
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    let pointer_k = |d: f32| (1.0 - (d - reach) / POINTER_FAR).clamp(0.0, 1.0);
    for (p, mut t, mut v) in &mut pointers {
        let want = match near.get(p.0) {
            Some(&(dir, _)) => {
                t.rotation = Rot2::radians(dir.x.atan2(-dir.y));
                Visibility::Inherited
            }
            None => Visibility::Hidden,
        };
        if *v != want {
            *v = want;
        }
    }
    for (a, mut n, mut bg) in &mut arcs {
        let Some(&(_, d)) = near.get(a.0) else {
            continue;
        };
        let k = pointer_k(d);
        let w = 14.0 + 26.0 * k;
        let h = if a.0 == 0 { 5.0 } else { 4.0 };
        n.left = Val::Px(INNER / 2.0 - w / 2.0);
        n.width = Val::Px(w);
        n.height = Val::Px(h);
        bg.0 = neonui::AMBER.with_alpha(0.2 + 0.8 * k);
    }
    // The nearest node: how far, and which way (an arrow on the clock, ahead up).
    if let Ok((mut text, mut color)) = label.single_mut() {
        let nearest = list.iter().min_by(|a, b| a.1.total_cmp(&b.1));
        let (s, c) = match nearest {
            None => ("NO NODES IN RANGE".to_string(), neonui::DIM),
            Some(&(_, d, _)) if d < reach => ("NODE IN REACH".to_string(), neonui::GREEN),
            Some(&(dir, d, _)) => {
                let ways = [
                    "AHEAD",
                    "AHEAD RIGHT",
                    "RIGHT",
                    "BEHIND RIGHT",
                    "BEHIND",
                    "BEHIND LEFT",
                    "LEFT",
                    "AHEAD LEFT",
                ];
                let a = dir.x.atan2(-dir.y).rem_euclid(std::f32::consts::TAU);
                let k = ((a / std::f32::consts::FRAC_PI_4).round() as usize) % 8;
                (format!("NODE {:.0} m {}", d, ways[k]), neonui::AMBER)
            }
        };
        if text.0 != s {
            text.0 = s;
        }
        if color.0 != c {
            color.0 = c;
        }
    }
}
