//! Squads on the flat screen: the squad panel (P, or SQUAD in the menu) to start, join and
//! leave squads (the leader also hands on the lead and puts members out); the leader's
//! waypoint (T: where one aims; or a click on the map), shown as a beam of light over the land,
//! on the map and the minimap; the squad's list in the HUD's top left (health, or where they
//! are), names over the squad mates, and over the faction's downed a medic's cross while a
//! medic may still revive them; the spot key (Q), and a red mark over the enemies the faction
//! spotted (src/vrui.rs has the VR panel; the beam and the crosses show in VR too).

use bevy::prelude::*;
use ps_net::protocol::{SQUAD_MAX, SquadCmd};
use ps_sim::PlayerId;
use ps_sim::snapshot::flags;

use crate::desktop::{MainCamera, Overlays};
use crate::game::{Game, Mover};
use crate::materials::{GlowMaterial, NeonUniform};
use crate::neonui::{self, NeonButton};
use crate::settings::{Action, Inputs, Settings};

/// The squad's colour (its mates, its waypoint), apart from the factions'.
pub const SQUAD_RGB: Vec3 = Vec3::new(1.0, 0.85, 0.15);
pub const SQUAD: Color = Color::srgb(1.0, 0.88, 0.3);
/// The downed's cross (a medic's green).
pub const DOWNED_RGB: Vec3 = Vec3::new(0.25, 1.0, 0.45);

/// How high the waypoint's beam goes (m).
const BEAM: f32 = 300.0;
/// Names over squad mates this near (m).
const TAG_RANGE: f32 = 300.0;
/// Floating labels at most (squad mates, the downed, the waypoint).
const TAGS: usize = 24;

#[derive(Resource, Default)]
pub struct SquadPanel {
    pub open: bool,
    /// What the panel was built from (rebuilt when it changes).
    built: Option<String>,
}

#[derive(Component)]
struct PanelRoot;

#[derive(Component, Clone)]
enum SquadAct {
    Cmd(SquadCmd),
    Close,
}

#[derive(Component)]
struct HudRoot;

/// The squad's list (top left).
#[derive(Component)]
struct ListText;

/// A floating label over something in the world.
#[derive(Component)]
struct Tag(usize);

#[derive(Component)]
struct WaypointBeam;

/// The downed's crosses (pooled).
#[derive(Component)]
struct Cross(usize);

#[derive(Resource)]
struct Marks {
    bar: Handle<Mesh>,
    cross: Handle<GlowMaterial>,
}

/// The panel, the HUD's list, the labels and the keys: on the flat screen.
pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadPanel>()
            .add_systems(
                Update,
                (
                    keys.after(crate::pause::toggle),
                    clicks,
                    build_panel,
                    spawn_hud,
                    hud,
                )
                    .chain()
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, clear.run_if(not(resource_exists::<Game>)));
    }
}

/// The marks in the world (the waypoint's beam, the downed's crosses): on the flat screen and
/// in VR alike.
pub struct SquadMarksPlugin;

impl Plugin for SquadMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_marks)
            .add_systems(
                Update,
                (beam, crosses).chain().run_if(resource_exists::<Game>),
            )
            .add_systems(Update, hide_marks.run_if(not(resource_exists::<Game>)));
    }
}

fn make_marks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let beam = glows.add(GlowMaterial {
        neon: NeonUniform::fixed(SQUAD_RGB, 5.0),
        overlay: false,
    });
    commands.spawn((
        WaypointBeam,
        Mesh3d(meshes.add(Cuboid::new(0.5, BEAM, 0.5))),
        MeshMaterial3d(beam),
        Transform::default(),
        Visibility::Hidden,
    ));
    commands.insert_resource(Marks {
        bar: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        cross: glows.add(GlowMaterial {
            neon: NeonUniform::fixed(DOWNED_RGB, 6.0),
            overlay: false,
        }),
    });
}

#[allow(clippy::type_complexity)]
fn clear(
    mut commands: Commands,
    mut panel: ResMut<SquadPanel>,
    mut overlays: ResMut<Overlays>,
    roots: Query<Entity, Or<(With<PanelRoot>, With<HudRoot>)>>,
) {
    panel.open = false;
    panel.built = None;
    overlays.squad = false;
    for e in &roots {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::type_complexity)]
fn hide_marks(mut marks: Query<&mut Visibility, Or<(With<WaypointBeam>, With<Cross>)>>) {
    for mut v in &mut marks {
        *v = Visibility::Hidden;
    }
}

/// The squad key opens and closes the panel (Esc closes it too); the leader's waypoint key
/// puts the waypoint where the view meets the land or a wall (in the sky: takes it away).
#[allow(clippy::too_many_arguments)]
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    mut game: ResMut<Game>,
    mut panel: ResMut<SquadPanel>,
    mut overlays: ResMut<Overlays>,
    cams: Query<&Transform, With<MainCamera>>,
    mut dev: Local<u8>,
    mut spot_ticks: Local<u32>,
) {
    // Testing: PS_SQUAD=1 starts a squad once in the game, then (leading it, alive) puts the
    // waypoint 60 m ahead.
    if std::env::var("PS_SQUAD").is_ok_and(|v| v == "1") && game.ready() {
        if *dev == 0 && game.my_squad().is_none() {
            game.squad(SquadCmd::Create);
            *dev = 1;
        } else if *dev == 1
            && game.leading()
            && game.alive()
            && let (Some(c), Some(m)) = (game.continent, game.mine())
        {
            let yaw = game.me_snap().map_or(0.0, |p| p.yaw);
            let ahead = m.feet + ps_sim::look_dir(yaw, 0.0) * 60.0;
            game.squad(SquadCmd::Waypoint(Some((c, ahead))));
            *dev = 2;
        }
    }
    let i = Inputs {
        keys: &keys,
        mouse: &mouse,
        wheel: default(),
    };
    if panel.open {
        if keys.just_pressed(KeyCode::Escape) || settings.just_pressed(Action::Squad, &i) {
            panel.open = false;
        }
    } else if !overlays.any() && game.connected && settings.just_pressed(Action::Squad, &i) {
        panel.open = true;
    }
    overlays.squad = panel.open;
    // The spot key: the enemy aimed at, for the faction (the server finds it).
    // (Testing: PS_SPOT=1 spots every second.)
    *spot_ticks = spot_ticks.wrapping_add(1);
    let auto = std::env::var("PS_SPOT").is_ok_and(|v| v == "1") && spot_ticks.is_multiple_of(60);
    if !overlays.any() && game.alive() && (auto || settings.just_pressed(Action::Spot, &i)) {
        game.request(ps_sim::Request::Spot);
    }
    if overlays.any() || !game.alive() || !settings.just_pressed(Action::Waypoint, &i) {
        return;
    }
    if !game.leading() {
        return;
    }
    let (Ok(cam), Some(c)) = (cams.single(), game.continent) else {
        return;
    };
    let from = cam.translation;
    let hit = game
        .world
        .ray(from, from + cam.forward() * 2000.0)
        .map(|h| h.1);
    game.squad(SquadCmd::Waypoint(hit.map(|at| (c, at))));
}

fn clicks(
    mut game: ResMut<Game>,
    mut panel: ResMut<SquadPanel>,
    q: Query<(&Interaction, &SquadAct), Changed<Interaction>>,
) {
    for (i, a) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match a {
            SquadAct::Cmd(c) => game.squad(c.clone()),
            SquadAct::Close => panel.open = false,
        }
    }
}

/// A member's state as the squad's list and panel show it: its health here, downed, dead,
/// or the continent it's on.
fn status(game: &Game, id: PlayerId) -> String {
    if let Some(p) = game.snap.players.iter().find(|p| p.id == id) {
        return if p.has(flags::DOWNED) {
            "DOWNED".into()
        } else if p.has(flags::DEAD) {
            "DEAD".into()
        } else {
            format!("{:.0}", p.health)
        };
    }
    if Some(id) == game.me {
        return if game.alive() { "" } else { "DEAD" }.into();
    }
    match game.whereabouts.get(&id) {
        Some(&c) if Some(c) != game.continent => game.def.continents[c as usize].name.clone(),
        _ => String::new(),
    }
}

fn build_panel(
    mut commands: Commands,
    game: Res<Game>,
    mut panel: ResMut<SquadPanel>,
    roots: Query<Entity, With<PanelRoot>>,
) {
    let sig = panel
        .open
        .then(|| format!("{:?}{:?}{:?}", game.squads, game.whereabouts, game.names));
    if sig == panel.built && (sig.is_none() || !roots.is_empty()) {
        return;
    }
    panel.built = sig;
    for e in &roots {
        commands.entity(e).despawn();
    }
    if !panel.open {
        return;
    }
    let me = game.me;
    let mine = game.my_squad().cloned();
    let leading = game.leading();
    let where_of = |id: PlayerId| {
        game.whereabouts.get(&id).map_or(String::new(), |&c| {
            game.def.continents[c as usize].name.clone()
        })
    };
    commands
        .spawn((PanelRoot, neonui::backdrop(neonui::BACKDROP, 18)))
        .with_children(|r| {
            r.spawn(neonui::panel(SQUAD, Some(620.0)))
                .with_children(|p| {
                    neonui::title(p, "SQUAD", 34.0, SQUAD);
                    match &mine {
                        Some(s) => {
                            p.spawn(neonui::text(
                                format!("YOUR SQUAD  {}/{SQUAD_MAX}", s.members.len()),
                                18.0,
                                neonui::DIM,
                            ));
                            for &m in &s.members {
                                p.spawn(neonui::row(10.0)).with_children(|row| {
                                    let lead = if m == s.leader { "* " } else { "" };
                                    let you = if Some(m) == me { " (you)" } else { "" };
                                    row.spawn((
                                        Node {
                                            width: Val::Px(300.0),
                                            ..default()
                                        },
                                        neonui::text(
                                            format!(
                                                "{lead}{}{you}  {}",
                                                game.name_of(m),
                                                where_of(m)
                                            ),
                                            16.0,
                                            if m == s.leader { SQUAD } else { neonui::TEXT },
                                        ),
                                    ));
                                    if leading && Some(m) != me {
                                        neonui::button(
                                            row,
                                            "LEAD",
                                            SquadAct::Cmd(SquadCmd::Promote(m)),
                                            NeonButton::new(SQUAD),
                                            None,
                                            13.0,
                                        );
                                        neonui::button(
                                            row,
                                            "KICK",
                                            SquadAct::Cmd(SquadCmd::Kick(m)),
                                            NeonButton::new(neonui::PINK),
                                            None,
                                            13.0,
                                        );
                                    }
                                });
                            }
                            let hint = if leading {
                                "You lead: [T] puts the waypoint where you aim (at the sky: \
                                 takes it away), or click the map."
                            } else {
                                "Spawn on your leader from the spawn screen."
                            };
                            p.spawn(neonui::text(hint, 13.0, neonui::DIM));
                            p.spawn(neonui::text(
                                "Chat to the squad: start the line with /s",
                                13.0,
                                neonui::DIM,
                            ));
                            p.spawn(neonui::row(12.0)).with_children(|row| {
                                if leading && s.waypoint.is_some() {
                                    neonui::button(
                                        row,
                                        "CLEAR WAYPOINT",
                                        SquadAct::Cmd(SquadCmd::Waypoint(None)),
                                        NeonButton::new(SQUAD),
                                        None,
                                        15.0,
                                    );
                                }
                                neonui::button(
                                    row,
                                    "LEAVE SQUAD",
                                    SquadAct::Cmd(SquadCmd::Leave),
                                    NeonButton::new(neonui::PINK),
                                    None,
                                    15.0,
                                );
                            });
                        }
                        None => {
                            p.spawn(neonui::text("You're in no squad.", 16.0, neonui::DIM));
                            neonui::button(
                                p,
                                "CREATE SQUAD",
                                SquadAct::Cmd(SquadCmd::Create),
                                NeonButton::new(SQUAD),
                                Some(280.0),
                                18.0,
                            );
                        }
                    }
                    let others: Vec<_> = game
                        .squads
                        .iter()
                        .filter(|s| Some(s.id) != mine.as_ref().map(|m| m.id))
                        .collect();
                    if !others.is_empty() {
                        p.spawn(neonui::gap(4.0));
                        p.spawn(neonui::text("YOUR FACTION'S SQUADS", 18.0, neonui::DIM));
                        for s in others {
                            p.spawn(neonui::row(10.0)).with_children(|row| {
                                row.spawn((
                                    Node {
                                        width: Val::Px(360.0),
                                        ..default()
                                    },
                                    neonui::text(
                                        format!(
                                            "{}'s squad  {}/{SQUAD_MAX}  {}",
                                            game.name_of(s.leader),
                                            s.members.len(),
                                            where_of(s.leader)
                                        ),
                                        16.0,
                                        neonui::TEXT,
                                    ),
                                ));
                                if s.members.len() < SQUAD_MAX {
                                    neonui::button(
                                        row,
                                        "JOIN",
                                        SquadAct::Cmd(SquadCmd::Join(s.id)),
                                        NeonButton::new(SQUAD),
                                        None,
                                        14.0,
                                    );
                                }
                            });
                        }
                    }
                    neonui::button(
                        p,
                        "CLOSE (P)",
                        SquadAct::Close,
                        NeonButton::new(neonui::DIM),
                        Some(200.0),
                        16.0,
                    );
                });
        });
}

fn spawn_hud(mut commands: Commands, q: Query<(), With<HudRoot>>) {
    if !q.is_empty() {
        return;
    }
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(4),
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            r.spawn((
                ListText,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    top: Val::Px(20.0),
                    ..default()
                },
                neonui::text("", 14.0, SQUAD),
            ));
            for i in 0..TAGS {
                r.spawn((
                    Tag(i),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    neonui::text_w("", 13.0, SQUAD, 700),
                    Visibility::Hidden,
                ));
            }
        });
}

/// The squad's list, and the floating labels: squad mates' names, the downed, the waypoint.
#[allow(clippy::type_complexity)]
fn hud(
    game: Res<Game>,
    overlays: Res<Overlays>,
    mover: Res<Mover>,
    cams: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut list: Query<&mut Text, (With<ListText>, Without<Tag>)>,
    mut tags: Query<(&Tag, &mut Node, &mut Text, &mut TextColor, &mut Visibility)>,
) {
    if let Ok(mut t) = list.single_mut() {
        let s = match game.my_squad() {
            Some(s) if !overlays.map => {
                let mut out = format!("SQUAD  {}/{SQUAD_MAX}", s.members.len());
                for &m in &s.members {
                    let lead = if m == s.leader { "* " } else { "  " };
                    out.push_str(&format!(
                        "\n{lead}{}  {}",
                        game.name_of(m),
                        status(&game, m)
                    ));
                }
                if let Some(at) = game.waypoint_here() {
                    let from = if mover.valid {
                        mover.body.feet
                    } else {
                        game.mine().map_or(at, |m| m.feet)
                    };
                    out.push_str(&format!(
                        "\nWAYPOINT  {:.0} m",
                        Vec2::new(at.x - from.x, at.z - from.z).length()
                    ));
                }
                out
            }
            _ => String::new(),
        };
        if t.0 != s {
            t.0 = s;
        }
    }
    // The labels: (where, what, colour).
    let mut want: Vec<(Vec3, String, Color)> = Vec::new();
    let cam = cams.single().ok();
    let eye = cam.map_or(Vec3::ZERO, |c| c.1.translation());
    if game.ready() && !overlays.map {
        for p in &game.snap.players {
            if Some(p.id) == game.me || p.team != game.team {
                continue;
            }
            let d = (p.feet - eye).length();
            if p.has(flags::DOWNED) && d < TAG_RANGE {
                want.push((
                    p.feet + Vec3::Y * 1.6,
                    format!("+ {}", game.name_of(p.id)),
                    Color::linear_rgb(DOWNED_RGB.x, DOWNED_RGB.y, DOWNED_RGB.z),
                ));
            } else if !p.has(flags::DEAD) && game.squad_mate(p.id) && d < TAG_RANGE && d > 2.0 {
                let up = if p.seat.is_some() { 3.2 } else { 2.2 };
                want.push((p.feet + Vec3::Y * up, game.name_of(p.id), SQUAD));
            }
        }
        // Enemies the faction spotted (or its interlink sees): a red mark over them.
        let spotted = Color::srgb(1.0, 0.25, 0.2);
        for p in &game.snap.players {
            if p.team != game.team
                && p.has(flags::SPOTTED)
                && !p.has(flags::DEAD)
                && p.seat.is_none()
                && (p.feet - eye).length() < 600.0
            {
                want.push((p.feet + Vec3::Y * 2.4, "!".into(), spotted));
            }
        }
        for v in &game.snap.vehicles {
            if v.team != game.team && v.flags & ps_sim::snapshot::vflags::SPOTTED != 0 {
                let up = v.kind.spec().half.y * 2.0 + 1.5;
                want.push((
                    v.pos + Vec3::Y * up,
                    format!("! {}", v.kind.name()),
                    spotted,
                ));
            }
        }
        if let Some(at) = game.waypoint_here() {
            // (From the player, as in the squad's list: the camera may be far behind.)
            let me_at = if mover.valid {
                mover.body.feet
            } else {
                game.mine().map_or(eye, |m| m.feet)
            };
            let ground = game.world.land(at.x, at.z).map_or(at.y, |g| g.max(at.y));
            want.push((
                Vec3::new(at.x, ground + 6.0, at.z),
                format!(
                    "WAYPOINT {:.0} m",
                    Vec2::new(at.x - me_at.x, at.z - me_at.z).length()
                ),
                SQUAD,
            ));
        }
    }
    for (tag, mut node, mut text, mut color, mut vis) in &mut tags {
        let shown = want.get(tag.0).and_then(|(at, s, c)| {
            let (cam, t) = cam?;
            let px = cam.world_to_viewport(t, *at).ok()?;
            Some((px, s, *c))
        });
        let v = if shown.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != v {
            *vis = v;
        }
        let Some((px, s, c)) = shown else { continue };
        // (Centred over the spot, roughly: the text's width isn't known here.)
        node.left = Val::Px(px.x - s.chars().count() as f32 * 4.5);
        node.top = Val::Px(px.y - 8.0);
        if text.0 != *s {
            text.0 = s.clone();
        }
        color.0 = c;
    }
}

/// The waypoint's beam of light, on the land under it.
fn beam(game: Res<Game>, mut q: Query<(&mut Transform, &mut Visibility), With<WaypointBeam>>) {
    let Ok((mut t, mut v)) = q.single_mut() else {
        return;
    };
    let at = game.waypoint_here().filter(|_| game.ready());
    let want = if at.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *v != want {
        *v = want;
    }
    if let Some(at) = at {
        let ground = game.world.land(at.x, at.z).map_or(at.y, |g| g.min(at.y));
        t.translation = Vec3::new(at.x, ground + BEAM / 2.0, at.z);
    }
}

/// A slowly turning cross over each of the faction's downed (a medic may revive them).
fn crosses(
    mut commands: Commands,
    game: Res<Game>,
    marks: Option<Res<Marks>>,
    time: Res<Time>,
    mut q: Query<(&Cross, &mut Transform, &mut Visibility)>,
) {
    let Some(marks) = marks else { return };
    let downed: Vec<Vec3> = game
        .snap
        .players
        .iter()
        .filter(|p| p.team == game.team && Some(p.id) != game.me && p.has(flags::DOWNED))
        .map(|p| p.feet)
        .collect();
    let have = q.iter().count();
    for i in have..downed.len() {
        commands
            .spawn((Cross(i), Transform::default(), Visibility::Hidden))
            .with_children(|c| {
                for (w, h) in [(0.9, 0.26), (0.26, 0.9)] {
                    c.spawn((
                        Mesh3d(marks.bar.clone()),
                        MeshMaterial3d(marks.cross.clone()),
                        Transform::from_scale(Vec3::new(w, h, 0.12)),
                    ));
                }
            });
    }
    let t = time.elapsed_secs();
    for (c, mut tr, mut v) in &mut q {
        let at = downed.get(c.0);
        let want = if at.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        if let Some(at) = at {
            let bob = (t * 2.0 + c.0 as f32).sin() * 0.08;
            tr.translation = *at + Vec3::Y * (0.9 + bob);
            tr.rotation = Quat::from_rotation_y(t * 1.5);
        }
    }
}
