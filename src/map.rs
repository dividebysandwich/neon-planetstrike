//! The map (M): on the left the world, every continent coloured by who holds it, the warp
//! gates' links between them and the contested ones marked; on the right a continent (the one
//! the player is on, or one picked on the left): its land, its bases in their owners' colours
//! with their lattice links and hacks, the warp gates and where they lead, the player and the
//! faction's vehicles, one's squad mates and the squad's waypoint (its leader clicks the land to
//! put it there). Dead, it's the spawn screen: click a base of one's own faction (here or on
//! another continent), a deployed spawn bus, the squad leader, or the sanctuary; downed, it
//! says how long a medic may still revive one.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use ps_sim::snapshot::vflags;
use ps_sim::terrain::{HALF, MAP_SIZE, SEA};
use ps_sim::vehicle::VehicleKind;
use ps_sim::worldgen::{BaseKind, Node as LNode};
use ps_sim::{BaseIdx, ContinentId, Request, SpawnPoint, Team};

use crate::desktop::Overlays;
use crate::game::{Arrived, Game, Mover};
use crate::materials::team_color;
use crate::neonui::{self, NeonButton};
use crate::settings::{Action, Inputs, Settings};

const WORLD_PX: f32 = 380.0;
const MAP_PX: f32 = 560.0;

#[derive(Resource, Default)]
pub struct MapState {
    pub open: bool,
    /// The continent shown on the right (none: the one the player is on).
    pub view: Option<ContinentId>,
    image: Option<(ContinentId, Handle<Image>)>,
    rebuild: f32,
    /// Opened as the spawn screen (closes on spawning).
    spawn_screen: bool,
    built_for: Option<(bool, Option<ContinentId>, Option<ContinentId>)>,
}

#[derive(Component)]
struct MapRoot;

#[derive(Component)]
struct MeMarker;

#[derive(Component, Clone, Copy)]
enum MapClick {
    Continent(ContinentId),
    Spawn(SpawnPoint),
    /// The continent's land (the squad leader puts the waypoint where it's clicked).
    Land(ContinentId),
    Close,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapState>()
            .add_systems(
                Update,
                (make_image, toggle, clicks, build, me_marker)
                    .chain()
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, close.run_if(not(resource_exists::<Game>)));
    }
}

fn close(
    mut commands: Commands,
    mut st: ResMut<MapState>,
    roots: Query<Entity, With<MapRoot>>,
    mut overlays: ResMut<Overlays>,
) {
    st.open = false;
    st.built_for = None;
    overlays.map = false;
    for e in &roots {
        commands.entity(e).despawn();
    }
}

fn color_of(t: Option<Team>) -> Color {
    let c = team_color(t);
    Color::linear_rgb(c.x, c.y, c.z)
}

/// The continent's land and roads as a picture (when the player arrives).
fn make_image(
    game: Res<Game>,
    mut st: ResMut<MapState>,
    mut arrived: MessageReader<Arrived>,
    mut images: ResMut<Assets<Image>>,
) {
    if arrived.read().last().is_none() {
        return;
    }
    let Some(map) = &game.map else { return };
    st.image = Some((map.def.id, images.add(land_image(map, 256))));
    st.built_for = None;
}

/// A picture of the continent's land, `n` pixels square (north, -Z, at the top).
pub fn land_image(map: &ps_sim::map::ContinentMap, n: usize) -> Image {
    let lk = crate::terrain::look(map.def.theme, map.def.sanctuary);
    let mut data = Vec::with_capacity(n * n * 4);
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 + 0.5) / n as f32 * MAP_SIZE - HALF;
            let z = (j as f32 + 0.5) / n as f32 * MAP_SIZE - HALF;
            let h = map.terrain.height(x, z);
            let c = if h < map.terrain.water(x, z) || h < SEA {
                lk.water * 3.0 + Vec3::splat(0.02)
            } else {
                let k = (h / 120.0).clamp(0.0, 1.0);
                let band = if (h / lk.contour_every).fract() < 0.12 {
                    0.25
                } else {
                    0.0
                };
                (lk.low.lerp(lk.high, k) * 4.0 + lk.contour * band).min(Vec3::ONE)
            };
            // The roads.
            let road = 1.0 - ((map.terrain.road_distance(x, z) - 6.0) / 4.0).clamp(0.0, 1.0);
            let c = c.lerp(lk.contour * 0.8 + Vec3::splat(0.15), road * 0.85);
            let to8 = |v: f32| (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8;
            data.extend([to8(c.x), to8(c.y), to8(c.z), 255]);
        }
    }
    Image::new(
        Extent3d {
            width: n as u32,
            height: n as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    game: Res<Game>,
    mut st: ResMut<MapState>,
    mut overlays: ResMut<Overlays>,
    time: Res<Time>,
    real: Res<Time<Real>>,
) {
    let i = Inputs {
        keys: &keys,
        mouse: &mouse,
        wheel: default(),
    };
    // (The killcam first: the spawn screen after it.)
    let dead = game.ready()
        && !game.alive()
        && game.mine().is_some()
        && game.killcam(real.elapsed_secs()).is_none();
    if dead {
        // The spawn screen.
        if !st.open {
            st.open = true;
            st.view = None;
        }
        st.spawn_screen = true;
    } else if std::mem::take(&mut st.spawn_screen) {
        st.open = false;
    } else if settings.just_pressed(Action::Map, &i)
        && !overlays.terminal
        && !overlays.menu
        && !overlays.chat
    {
        st.open = !st.open;
        st.view = None;
    } else if st.open && keys.just_pressed(KeyCode::Escape) {
        st.open = false;
    }
    overlays.map = st.open;
    // Refreshed now and then while open (owners, hacks, positions).
    st.rebuild -= time.delta_secs();
}

fn clicks(
    mut game: ResMut<Game>,
    mut st: ResMut<MapState>,
    q: Query<(&Interaction, &MapClick, Option<&RelativeCursorPosition>), Changed<Interaction>>,
) {
    for (i, c, rel) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match *c {
            MapClick::Continent(c) => {
                st.view = Some(c);
                st.built_for = None;
            }
            MapClick::Spawn(p) => game.request(Request::Spawn(p)),
            MapClick::Land(c) => {
                // (The node's middle is 0, its edges +-0.5.)
                if let Some(n) = rel.and_then(|r| r.normalized) {
                    let xz = (n + Vec2::splat(0.5)) * MAP_SIZE - Vec2::splat(HALF);
                    let y = if Some(c) == game.continent {
                        game.world.land(xz.x, xz.y).unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    game.squad(ps_net::protocol::SquadCmd::Waypoint(Some((
                        c,
                        Vec3::new(xz.x, y, xz.y),
                    ))));
                    st.rebuild = 0.0;
                }
            }
            MapClick::Close => st.open = false,
        }
    }
}

/// An absolutely placed line between two points (px) of its parent.
fn line(p: &mut ChildSpawnerCommands, a: Vec2, b: Vec2, w: f32, color: Color) {
    let d = b - a;
    let len = d.length();
    let mid = (a + b) / 2.0;
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(mid.x - len / 2.0),
            top: Val::Px(mid.y - w / 2.0),
            width: Val::Px(len),
            height: Val::Px(w),
            ..default()
        },
        UiTransform {
            rotation: Rot2::radians(d.y.atan2(d.x)),
            ..default()
        },
        BackgroundColor(color),
    ));
}

fn label(p: &mut ChildSpawnerCommands, at: Vec2, s: &str, size: f32, color: Color) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(at.x),
            top: Val::Px(at.y),
            ..default()
        },
        neonui::text(s, size, color),
    ));
}

#[allow(clippy::too_many_arguments)]
fn build(
    mut commands: Commands,
    game: Res<Game>,
    mut st: ResMut<MapState>,
    roots: Query<Entity, With<MapRoot>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let dead = !game.alive();
    // Smaller windows (a browser tab): the panels shrink to fit (about 1360 x 660 at full size).
    let fit = windows.single().map_or(1.0, |w| {
        (w.width() / 1360.0)
            .min((w.height() - 90.0) / 660.0)
            .clamp(0.4, 1.0)
    });
    let key = (st.open, st.view, game.continent);
    if st.built_for == Some((key.0, key.1, key.2)) && st.rebuild > 0.0 {
        return;
    }
    st.built_for = Some(key);
    st.rebuild = 1.0;
    for e in &roots {
        commands.entity(e).despawn();
    }
    if !st.open {
        return;
    }
    let team = game.team;
    let tc = color_of(Some(team));
    let here = game.continent;
    let shown = st.view.or(here).unwrap_or(0);
    let def = &game.def;
    let respawn = game.mine().and_then(|m| m.dead).unwrap_or(0.0);
    commands
        .spawn((MapRoot, neonui::backdrop(neonui::BACKDROP_DEEP, 15)))
        .with_children(|r| {
            let downed = game.mine().and_then(|m| m.downed);
            let title = if let Some((left, k)) = downed.filter(|_| dead) {
                if k > 0.0 {
                    format!("BEING REVIVED  {:.0}%", k * 100.0)
                } else if respawn > 0.0 {
                    format!(
                        "DOWNED: A MEDIC MAY REVIVE YOU ({:.0} s)  -  OR SPAWN IN {:.0} s",
                        left.ceil(),
                        respawn.ceil()
                    )
                } else {
                    format!(
                        "DOWNED: A MEDIC MAY REVIVE YOU ({:.0} s)  -  OR SPAWN",
                        left.ceil()
                    )
                }
            } else if dead {
                if respawn > 0.0 {
                    format!("CHOOSE A SPAWN POINT  ({:.0} s)", respawn.ceil())
                } else {
                    "CHOOSE A SPAWN POINT".to_string()
                }
            } else {
                "MAP".to_string()
            };
            neonui::title(r, &title, 30.0, tc);
            r.spawn((neonui::row(24.0), UiTransform::from_scale(Vec2::splat(fit))))
                .with_children(|row| {
                    // The world.
                    row.spawn(neonui::panel(tc, None)).with_children(|p| {
                        p.spawn(neonui::text("THE WORLD", 18.0, neonui::DIM));
                        p.spawn(Node {
                            width: Val::Px(WORLD_PX),
                            height: Val::Px(WORLD_PX),
                            ..default()
                        })
                        .with_children(|w| {
                            let at = |c: &ps_sim::worldgen::ContinentDef| {
                                Vec2::new(c.map_pos.x, -c.map_pos.y) * (WORLD_PX * 0.42)
                                    + Vec2::splat(WORLD_PX / 2.0)
                            };
                            for c in &def.continents {
                                for g in &c.gates {
                                    if g.to > c.id {
                                        line(
                                            w,
                                            at(c),
                                            at(&def.continents[g.to as usize]),
                                            2.0,
                                            Color::srgba(0.6, 0.7, 1.0, 0.5),
                                        );
                                    }
                                }
                            }
                            for c in &def.continents {
                                let owner = game.state.continents[c.id as usize].owner;
                                let contested = game.state.contested(def, c.id);
                                let size = if c.sanctuary.is_some() { 30.0 } else { 38.0 };
                                let p = at(c);
                                let mut b = NeonButton::new(color_of(owner));
                                b.selected = Some(c.id) == here || st.view == Some(c.id);
                                w.spawn((
                                    Button,
                                    MapClick::Continent(c.id),
                                    b,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(p.x - size / 2.0),
                                        top: Val::Px(p.y - size / 2.0),
                                        width: Val::Px(size),
                                        height: Val::Px(size),
                                        border: UiRect::all(Val::Px(if contested {
                                            4.0
                                        } else {
                                            2.0
                                        })),
                                        border_radius: BorderRadius::all(Val::Px(size / 2.0)),
                                        ..default()
                                    },
                                    BorderColor::all(if contested {
                                        neonui::AMBER
                                    } else {
                                        color_of(owner)
                                    }),
                                    BackgroundColor(color_of(owner).with_alpha(0.35)),
                                ));
                                let mut name = c.name.clone();
                                if Some(c.id) == here {
                                    name.push_str(" *");
                                }
                                // Who's there (players and bots, by faction), and the
                                // faction's being overpopulated there.
                                if let Some(pop) = game.population.get(c.id as usize)
                                    && pop.iter().any(|&n| n > 0)
                                {
                                    name.push_str(&format!("  {}/{}/{}", pop[0], pop[1], pop[2]));
                                }
                                if overpop(&game, c.id) {
                                    name.push_str(" OVERPOP");
                                }
                                label(
                                    w,
                                    p + Vec2::new(-size / 2.0, size / 2.0 + 2.0),
                                    &name,
                                    12.0,
                                    neonui::TEXT,
                                );
                            }
                        });
                        p.spawn(neonui::text(
                            "Amber ring: contested.  * You are here.",
                            12.0,
                            neonui::DIM,
                        ));
                    });
                    // The continent.
                    let cd = &def.continents[shown as usize];
                    row.spawn(neonui::panel(tc, None)).with_children(|p| {
                        p.spawn(neonui::text(
                            format!("{}  -  {}", cd.name.to_uppercase(), cd.theme.name()),
                            18.0,
                            neonui::DIM,
                        ));
                        let mut area = p.spawn(Node {
                            width: Val::Px(MAP_PX),
                            height: Val::Px(MAP_PX),
                            ..default()
                        });
                        if let Some((c, img)) = &st.image
                            && *c == shown
                        {
                            area.insert(ImageNode::new(img.clone()));
                        } else {
                            area.insert(BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 0.9)));
                        }
                        if game.leading() && !dead {
                            area.insert((
                                Button,
                                MapClick::Land(shown),
                                RelativeCursorPosition::default(),
                            ));
                        }
                        area.with_children(|m| {
                            let px = |v: Vec2| (v + Vec2::splat(HALF)) / MAP_SIZE * MAP_PX;
                            let node_at = |n: LNode| match n {
                                LNode::Base(b) => px(cd.bases[b as usize].pos),
                                LNode::Gate(g) => px(cd.gates[g as usize].pos),
                            };
                            for &(a, b) in &cd.lattice {
                                line(
                                    m,
                                    node_at(a),
                                    node_at(b),
                                    2.0,
                                    Color::srgba(1.0, 1.0, 1.0, 0.45),
                                );
                            }
                            for g in &cd.gates {
                                let p = px(g.pos);
                                let to = &def.continents[g.to as usize];
                                m.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(p.x - 9.0),
                                        top: Val::Px(p.y - 9.0),
                                        width: Val::Px(18.0),
                                        height: Val::Px(18.0),
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    UiTransform {
                                        rotation: Rot2::degrees(45.0),
                                        ..default()
                                    },
                                    BorderColor::all(neonui::CYAN),
                                    BackgroundColor(
                                        color_of(game.state.continents[g.to as usize].owner)
                                            .with_alpha(0.5),
                                    ),
                                ));
                                label(
                                    m,
                                    p + Vec2::new(12.0, -8.0),
                                    &format!("to {}", to.name),
                                    12.0,
                                    neonui::CYAN,
                                );
                            }
                            for (bi, b) in cd.bases.iter().enumerate() {
                                let st_b = game.state.base(shown, bi as BaseIdx);
                                let p = px(b.pos);
                                let size = match b.kind {
                                    BaseKind::Tower => 11.0,
                                    BaseKind::Outpost => 18.0,
                                    _ => 26.0,
                                };
                                let ours = st_b.owner == Some(team);
                                let mut e = m.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(p.x - size / 2.0),
                                        top: Val::Px(p.y - size / 2.0),
                                        width: Val::Px(size),
                                        height: Val::Px(size),
                                        border: UiRect::all(Val::Px(if st_b.hack.is_some() {
                                            4.0
                                        } else {
                                            2.0
                                        })),
                                        border_radius: BorderRadius::all(Val::Px(3.0)),
                                        ..default()
                                    },
                                    BorderColor::all(if st_b.hack.is_some() {
                                        neonui::AMBER
                                    } else {
                                        Color::WHITE
                                    }),
                                    BackgroundColor(color_of(st_b.owner)),
                                ));
                                if dead && ours {
                                    e.insert((
                                        Button,
                                        MapClick::Spawn(SpawnPoint::Base {
                                            continent: shown,
                                            base: bi as BaseIdx,
                                        }),
                                    ));
                                }
                                let mut name = b.name.clone();
                                if let Some(f) = cd.facility(bi as BaseIdx) {
                                    name = format!("{name} ({})", f.name());
                                }
                                if let Some(h) = st_b.hack {
                                    name = format!("{name} [{} {:.0}s]", h.team.upper(), h.left);
                                }
                                label(
                                    m,
                                    p + Vec2::new(size / 2.0 + 3.0, -7.0),
                                    &name,
                                    12.0,
                                    neonui::TEXT,
                                );
                            }
                            // The squad's waypoint.
                            if let Some((wc, at)) = game.my_squad().and_then(|s| s.waypoint)
                                && wc == shown
                            {
                                let p = px(Vec2::new(at.x, at.z));
                                m.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(p.x - 8.0),
                                        top: Val::Px(p.y - 8.0),
                                        width: Val::Px(16.0),
                                        height: Val::Px(16.0),
                                        border: UiRect::all(Val::Px(3.0)),
                                        ..default()
                                    },
                                    UiTransform {
                                        rotation: Rot2::degrees(45.0),
                                        ..default()
                                    },
                                    BorderColor::all(crate::squad::SQUAD),
                                ));
                                label(
                                    m,
                                    p + Vec2::new(11.0, -8.0),
                                    "WAYPOINT",
                                    12.0,
                                    crate::squad::SQUAD,
                                );
                            }
                            if Some(shown) == here {
                                for p in &game.snap.players {
                                    if !game.squad_mate(p.id)
                                        || p.has(ps_sim::snapshot::flags::DEAD)
                                    {
                                        continue;
                                    }
                                    let at = px(Vec2::new(p.feet.x, p.feet.z));
                                    m.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(at.x - 5.0),
                                            top: Val::Px(at.y - 5.0),
                                            width: Val::Px(10.0),
                                            height: Val::Px(10.0),
                                            border_radius: BorderRadius::all(Val::Px(5.0)),
                                            ..default()
                                        },
                                        BackgroundColor(crate::squad::SQUAD),
                                    ));
                                    label(
                                        m,
                                        at + Vec2::new(7.0, -7.0),
                                        &game.name_of(p.id),
                                        11.0,
                                        crate::squad::SQUAD,
                                    );
                                }
                                for v in &game.snap.vehicles {
                                    if v.team != team {
                                        continue;
                                    }
                                    let p = px(Vec2::new(v.pos.x, v.pos.z));
                                    let bus = v.kind == VehicleKind::SpawnBus
                                        && v.flags & vflags::DEPLOYED != 0;
                                    let size = if bus { 12.0 } else { 7.0 };
                                    let mut e = m.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(p.x - size / 2.0),
                                            top: Val::Px(p.y - size / 2.0),
                                            width: Val::Px(size),
                                            height: Val::Px(size),
                                            border_radius: BorderRadius::all(Val::Px(size / 2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(if bus { neonui::GREEN } else { tc }),
                                    ));
                                    if bus && dead {
                                        e.insert((Button, MapClick::Spawn(SpawnPoint::Bus(v.id))));
                                    }
                                }
                                m.spawn((
                                    MeMarker,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(12.0),
                                        height: Val::Px(12.0),
                                        border: UiRect::all(Val::Px(2.0)),
                                        border_radius: BorderRadius::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    BorderColor::all(Color::WHITE),
                                    BackgroundColor(neonui::MAGENTA),
                                ));
                            }
                        });
                    });
                    // Spawn points (dead).
                    if dead {
                        row.spawn(neonui::panel(tc, Some(300.0)))
                            .with_children(|p| {
                                p.spawn(neonui::text("SPAWN AT", 18.0, neonui::DIM));
                                for (label, point) in spawn_choices(&game) {
                                    let color = if matches!(point, SpawnPoint::Bus(_)) {
                                        neonui::GREEN
                                    } else if matches!(point, SpawnPoint::Squad(_)) {
                                        crate::squad::SQUAD
                                    } else {
                                        tc
                                    };
                                    neonui::button(
                                        p,
                                        &label,
                                        MapClick::Spawn(point),
                                        NeonButton::new(color),
                                        Some(240.0),
                                        14.0,
                                    );
                                }
                            });
                    }
                });
            if !dead {
                neonui::button(
                    r,
                    "CLOSE (M)",
                    MapClick::Close,
                    NeonButton::new(neonui::DIM),
                    Some(160.0),
                    16.0,
                );
            }
        });
}

fn me_marker(game: Res<Game>, mover: Res<Mover>, mut q: Query<&mut Node, With<MeMarker>>) {
    let at = if mover.valid {
        Some(mover.body.feet)
    } else {
        game.mine().map(|m| m.feet)
    };
    let Some(at) = at else { return };
    for mut n in &mut q {
        let p = (Vec2::new(at.x, at.z) + Vec2::splat(HALF)) / MAP_SIZE * MAP_PX;
        n.left = Val::Px(p.x - 6.0);
        n.top = Val::Px(p.y - 6.0);
    }
}

/// Where the player may spawn now: the sanctuary, the faction's bases at the front (and on this
/// continent), and its deployed spawn buses; labelled.
pub fn spawn_choices(game: &Game) -> Vec<(String, SpawnPoint)> {
    let team = game.team;
    let def = &game.def;
    let here = game.continent;
    let mut out = vec![(
        "Sanctuary".to_string(),
        SpawnPoint::Base {
            continent: team.index() as ContinentId,
            base: 0,
        },
    )];
    for (ci, cs) in game.state.continents.iter().enumerate().skip(3) {
        for (bi, b) in cs.bases.iter().enumerate() {
            if b.owner != Some(team) || out.len() >= 10 {
                continue;
            }
            // Bases at the front: next to something the faction doesn't hold.
            let cd = &def.continents[ci];
            let front = game.state.contested(def, ci as ContinentId)
                || cd.neighbours(LNode::Base(bi as BaseIdx)).any(|n| match n {
                    LNode::Base(o) => cs.bases[o as usize].owner != Some(team),
                    LNode::Gate(_) => false,
                });
            if !front && Some(ci as u8) != here {
                continue;
            }
            let full = if Some(ci as u8) != here && overpop(game, ci as ContinentId) {
                "  OVERPOP"
            } else {
                ""
            };
            out.push((
                match cd.facility(bi as BaseIdx) {
                    Some(f) => format!("{} / {} ({}){full}", cd.name, cd.bases[bi].name, f.name()),
                    None => format!("{} / {}{full}", cd.name, cd.bases[bi].name),
                },
                SpawnPoint::Base {
                    continent: ci as ContinentId,
                    base: bi as BaseIdx,
                },
            ));
        }
    }
    for v in &game.snap.vehicles {
        if v.team == team && v.kind == VehicleKind::SpawnBus && v.flags & vflags::DEPLOYED != 0 {
            out.push(("Spawn bus".to_string(), SpawnPoint::Bus(v.id)));
        }
    }
    // The faction's forward spawn tubes here.
    if let Some(map) = &game.map {
        for s in &game.snap.structures {
            if s.team == team && s.kind == ps_sim::construct::StructureKind::Tube {
                let _ = map;
                let near = crate::build::nearest_base(game, s.pos)
                    .map_or(String::new(), |n| format!(" (by {n})"));
                out.push((format!("Forward tube{near}"), SpawnPoint::Tube(s.id)));
            }
        }
    }
    // On the squad leader (wherever it is; the server says if it can't be now).
    if let Some(s) = game.my_squad()
        && Some(s.leader) != game.me
    {
        out.insert(
            0,
            (
                format!("Squad leader ({})", game.name_of(s.leader)),
                SpawnPoint::Squad(s.leader),
            ),
        );
    }
    out
}

/// Whether the player's faction is overpopulated on continent `c` (the server refuses spawns
/// onto it from elsewhere).
pub fn overpop(game: &Game, c: ContinentId) -> bool {
    let Some(&h) = game.humans.get(c as usize) else {
        return false;
    };
    let rivals = game.state.factions_on(&game.def, c);
    ps_net::protocol::overpopulated(h, game.team, &rivals)
}
