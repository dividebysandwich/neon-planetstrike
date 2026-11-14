//! The VR panels, built from neon meshes (flat quads and text from src/vfont.rs) so both VR
//! renderers draw them (the GL headset renderer has no UI, text or textures):
//!
//! - out of a game: the start screen (the offline world, the first servers of the internet
//!   list, quit);
//! - dead: the spawn points (as the map's SPAWN AT list, `map::spawn_choices`);
//! - at a terminal or console: the equipment, the vehicles, the console's puzzle (the same
//!   choices as the flat panels, `interact::apply`);
//! - the menu (the left menu button): resume, the map, the squad (start, join, leave one),
//!   suicide (to spawn elsewhere), leave the game, quit.
//!
//! A panel appears in front of the eyes and stays there (in the tracking space) while it's up,
//! drawn over the world (overlay materials: no wall or vehicle hides it).
//! Each hand points a laser at it; pulling the trigger on a button presses it, and the gun
//! doesn't fire meanwhile.
//!
//! In a game, a small plate over the left wrist shows health and armor, the weapon's ammo (or
//! the vehicle's state, a harvester's load), what the use trigger would do and the faction's
//! bases under attack; the minimap (src/minimap.rs) is on the back of the left hand. Seated in
//! a vehicle or a turret, a ring shows where the gun points (another where it's turning to)
//! and a line under the view the vehicle's state.
//!
//! The map (the left stick clicked, or MAP in the menu): the continent, with its bases by owner,
//! hacks, the lattice, the gates and an arrow where one is, and the world's continents.
//!
//! Testing: `PS_VR_MENU=1` opens the menu at the start, `PS_VR_MAP=1` the map.

use bevy::prelude::*;
use ps_sim::{Request, SpawnPoint};

use crate::desktop::Overlays;
use crate::game::{Game, LocalCmd};
use crate::interact::{Panel, Pick, Target};
use crate::materials::{AccentMaterial, NeonUniform, team_color};
use crate::menu::{InternetList, LeaveGame, MenuAct, MenuModel, MenuRoot, Outcome, Screen};
use crate::vr::{XrInput, XrRig};

/// Metres from the eyes.
const DISTANCE: f32 = 1.3;
/// Panel width (m).
const WIDTH: f32 = 1.1;
const PAD: f32 = 0.04;
const BUTTON_H: f32 = 0.065;
const GAP: f32 = 0.012;
const CELL: f32 = 0.045;

/// What a button does.
#[derive(Clone, Debug, PartialEq)]
enum Act {
    Menu(MenuAct),
    Pick(Pick),
    Spawn(SpawnPoint),
    Resume,
    Suicide,
    Leave,
    Quit,
    Map(bool),
    /// The squad panel open or closed.
    SquadPanel(bool),
    /// The scoreboard open or closed.
    ScoresPanel(bool),
    /// One's stats open or closed.
    StatsPanel(bool),
    /// The login form (its keyboard, fields, buttons).
    Login(crate::login::LoginAct),
    Squad(ps_net::protocol::SquadCmd),
}

/// A puzzle cell: its colour (a piece's), inside the square, the piece.
type Cell = (Option<Vec3>, bool, Option<u8>);

/// A panel's contents, top to bottom.
#[derive(Clone, Debug, PartialEq)]
enum Row {
    Title(String),
    Text(String),
    /// Buttons in `cols` columns: label, act, lit (the current choice).
    Buttons(Vec<(String, Act, bool)>, usize),
    /// The console puzzle's grid: rows of cells, each (colour, inside the square, the piece).
    Cells(Vec<Vec<Cell>>),
    /// The map: the continent and the world (rebuilt when its signature changes: owners,
    /// hacks).
    Map(String),
}

struct Button {
    act: Act,
    center: Vec2,
    half: Vec2,
    entity: Entity,
}

#[derive(Resource, Default)]
pub(crate) struct VrUi {
    /// The contents shown (rebuilt when they change).
    shown: Option<Vec<Row>>,
    root: Option<Entity>,
    /// The panel's height (m).
    height: f32,
    /// The panel's place in the tracking space.
    pose: Transform,
    buttons: Vec<Button>,
    hovered: Option<usize>,
    lasers: [Option<Entity>; 2],
    trigger_held: [bool; 2],
    menu_held: bool,
    /// The menu is open.
    menu: bool,
    /// The squad panel is open.
    pub(crate) squad: bool,
    /// The scoreboard is open (the command wheel's SCORES).
    pub(crate) scores: bool,
    /// One's stats are open.
    pub(crate) stats: bool,
    /// The map is open, and the stick click that toggles it is held.
    pub(crate) map: bool,
    stick_held: bool,
    /// `PS_VR_MAP`: open the map once in a game.
    map_env: bool,
    /// The map's "you are here" arrow: its entity, and the panel point of the continent's
    /// (-BOUND, -BOUND) corner and metres per panel metre.
    marker: Option<(Entity, Vec2, f32)>,
    /// The continent's land as a coloured mesh, for the map (made once per continent).
    land: Option<(ps_sim::ContinentId, Handle<Mesh>)>,
    asked_list: bool,
    /// The panel was placed by a tracked head (not the default one before tracking starts).
    placed_by_head: bool,
}

#[derive(Resource)]
pub(crate) struct Mats {
    pub(crate) back: Handle<AccentMaterial>,
    edge: Handle<AccentMaterial>,
    pub(crate) button: Handle<AccentMaterial>,
    pub(crate) lit: Handle<AccentMaterial>,
    hover: Handle<AccentMaterial>,
    pub(crate) text: Handle<AccentMaterial>,
    pub(crate) dim: Handle<AccentMaterial>,
    laser: Handle<AccentMaterial>,
    pub(crate) quad: Handle<Mesh>,
    beam: Handle<Mesh>,
}

pub struct VrUiPlugin;

impl Plugin for VrUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VrUi {
            menu: std::env::var("PS_VR_MENU").is_ok(),
            map_env: std::env::var("PS_VR_MAP").is_ok(),
            ..default()
        })
        .add_systems(Startup, make_mats)
        .add_systems(
            Update,
            (
                crate::wheel::pick,
                contents,
                point,
                wrist,
                seat_hud,
                hand_map,
                crate::wheel::draw,
                crate::objectives::vr_marks,
            )
                .chain()
                .after(crate::game::InputSet)
                .after(crate::seatview::SeatViewSet)
                .run_if(resource_exists::<XrRig>),
        );
    }
}

fn make_mats(
    mut commands: Commands,
    mut mats: ResMut<Assets<AccentMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let m = |c: Vec3, i: f32, mats: &mut Assets<AccentMaterial>| {
        mats.add(AccentMaterial {
            neon: NeonUniform::fixed(c, i),
            overlay: true,
        })
    };
    commands.insert_resource(Mats {
        back: m(Vec3::new(0.01, 0.01, 0.02), 1.0, &mut mats),
        edge: m(Vec3::new(1.0, 0.3, 0.8), 2.0, &mut mats),
        button: m(Vec3::new(0.06, 0.07, 0.12), 1.0, &mut mats),
        lit: m(Vec3::new(0.15, 0.4, 0.6), 1.0, &mut mats),
        hover: m(Vec3::new(0.5, 0.2, 0.6), 1.0, &mut mats),
        text: m(Vec3::new(0.9, 0.95, 1.0), 1.5, &mut mats),
        dim: m(Vec3::new(0.5, 0.55, 0.7), 1.0, &mut mats),
        laser: m(Vec3::new(1.0, 0.3, 0.8), 3.0, &mut mats),
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        beam: meshes.add(Cuboid::new(0.003, 0.003, 1.0)),
    });
}

/// What the panel should show now (None: no panel).
fn wanted(
    ui: &mut VrUi,
    game: Option<&Game>,
    panel: Option<&Panel>,
    model: Option<&mut MenuModel>,
    login: Option<&crate::login::LoginForm>,
) -> Option<Vec<Row>> {
    // A server that keeps accounts: logging in first.
    if let (Some(g), Some(form)) = (game, login)
        && g.login.is_some()
    {
        return Some(login_rows(g, form));
    }
    let Some(game) = game else {
        // The start screen.
        let model = model?;
        if !ui.asked_list {
            ui.asked_list = true;
            model.apply(&MenuAct::Go(Screen::Internet));
        }
        let mut rows = vec![
            Row::Title("NEON PLANETSTRIKE".into()),
            Row::Buttons(
                vec![("OFFLINE WORLD".into(), Act::Menu(MenuAct::StartBots), false)],
                1,
            ),
        ];
        match &model.internet {
            InternetList::Listed(l) if !l.is_empty() => {
                rows.push(Row::Text("Internet servers".into()));
                rows.push(Row::Buttons(
                    l.iter()
                        .take(6)
                        .map(|s| {
                            (
                                format!("{} ({})", s.report.name, s.report.players.len()),
                                Act::Menu(MenuAct::Join(s.addr.clone())),
                                false,
                            )
                        })
                        .collect(),
                    2,
                ));
            }
            InternetList::Asking => rows.push(Row::Text("Asking for servers...".into())),
            _ => {}
        }
        rows.push(Row::Buttons(vec![("QUIT".into(), Act::Quit, false)], 1));
        return Some(rows);
    };
    if ui.menu {
        let rank = game.xp.map_or(String::new(), |xp| {
            let br = ps_sim::progress::battle_rank(xp);
            format!(
                "BATTLE RANK {br} ({:.0}%)   {xp} XP   K {} D {} A {}",
                ps_sim::progress::rank_progress(xp) * 100.0,
                game.mine().map_or(0, |m| m.kills),
                game.mine().map_or(0, |m| m.deaths),
                game.mine().map_or(0, |m| m.assists),
            )
        });
        let mut acts = vec![
            ("RESUME".into(), Act::Resume, false),
            ("MAP".into(), Act::Map(true), false),
            ("SQUAD".into(), Act::SquadPanel(true), false),
            ("STATS".into(), Act::StatsPanel(true), false),
        ];
        acts.extend([
            ("SUICIDE (NEW SPAWN)".into(), Act::Suicide, false),
            ("LEAVE GAME".into(), Act::Leave, false),
            ("QUIT".into(), Act::Quit, false),
        ]);
        let mut rows = vec![Row::Title("MENU".into()), Row::Text(rank)];
        if let Some((a, _)) = &game.alert {
            let h = crate::hud::holdings(game, a.continent);
            rows.push(Row::Text(format!(
                "ALERT on {}: most bases win   R {} B {} G {}",
                game.def.continents[a.continent as usize].name, h[0], h[1], h[2]
            )));
        }
        rows.push(Row::Buttons(acts, 1));
        return Some(rows);
    }
    if ui.squad {
        return Some(squad_rows(game));
    }
    if ui.scores {
        return Some(score_rows(game));
    }
    if ui.stats {
        let mut rows = vec![Row::Title("STATS".into())];
        match &game.stats {
            Some(s) => rows.extend(crate::stats::lines(s).into_iter().map(Row::Text)),
            None => rows.push(Row::Text("Asking the server...".into())),
        }
        rows.push(Row::Buttons(
            vec![("CLOSE".into(), Act::StatsPanel(false), false)],
            1,
        ));
        return Some(rows);
    }
    if let Some(panel) = panel
        && let Some(t) = panel.open
    {
        return Some(terminal(game, panel, t));
    }
    if game.ready() && !game.alive() && game.mine().is_some() {
        let left = game.mine().and_then(|m| m.dead).unwrap_or(0.0);
        let downed = game.mine().and_then(|m| m.downed);
        let title = if let Some((w, k)) = downed {
            if k > 0.0 {
                format!("BEING REVIVED {:.0}%", k * 100.0)
            } else {
                format!("DOWNED: A MEDIC MAY REVIVE YOU ({:.0} S)", w.ceil())
            }
        } else if left > 0.0 {
            format!("CHOOSE A SPAWN POINT ({:.0} S)", left.ceil())
        } else {
            "CHOOSE A SPAWN POINT".into()
        };
        let buttons = crate::map::spawn_choices(game)
            .into_iter()
            .map(|(l, p)| (l, Act::Spawn(p), false))
            .collect();
        let mut rows = vec![Row::Title(title)];
        // Who did it (the flat screen's killcam).
        if let Some(d) = game.death.as_ref().filter(|d| Some(d.by) != game.me) {
            rows.push(Row::Text(format!(
                "KILLED BY {}: {}, {:.0} m (their health {:.0}, armour {:.0})",
                game.name_of(d.by),
                d.cause.name(),
                d.distance,
                d.health,
                d.armor
            )));
        }
        rows.push(Row::Buttons(buttons, 2));
        return Some(rows);
    }
    if ui.map
        && game.ready()
        && let Some(c) = game.continent
    {
        let cd = &game.def.continents[c as usize];
        let st = &game.state;
        // Its signature: what changes the drawing (owners, hacks), not the marker.
        let mut sig = format!("{c}");
        for b in &st.continents[c as usize].bases {
            sig.push_str(&format!("{:?}{:?}", b.owner, b.hack.map(|h| h.team)));
        }
        for cs in &st.continents {
            sig.push_str(&format!("{:?}", cs.owner));
        }
        return Some(vec![
            Row::Title(format!("{}  -  {:?}", cd.name.to_uppercase(), cd.theme)),
            Row::Map(sig),
            Row::Buttons(vec![("CLOSE".into(), Act::Map(false), false)], 1),
        ]);
    }
    None
}

/// The wrist plate: its lines as last built, and its entity.
#[derive(Default)]
struct Wrist {
    lines: Vec<String>,
    root: Option<Entity>,
}

/// The wrist's lines now.
fn wrist_lines(game: &Game, hint: Option<&crate::interact::UseHint>) -> Vec<String> {
    let Some(me) = game.mine() else {
        return Vec::new();
    };
    if me.dead.is_some() {
        return Vec::new();
    }
    let mut out = vec![format!("HP {:.0}   AR {:.0}", me.health, me.armor)];
    match game
        .seat()
        .and_then(|(v, s)| game.vehicle(v).map(|v| (v, s)))
    {
        Some((v, s)) => {
            out.push(format!(
                "{} {:.0}%{}",
                v.kind.name(),
                v.health,
                if s == 0 { "" } else { "  gunner" }
            ));
            if v.kind == ps_sim::vehicle::VehicleKind::Harvester {
                let c = v.cargo as f32 / ps_sim::sim::CARGO;
                let n = (c * 10.0).round().clamp(0.0, 10.0) as usize;
                out.push(format!(
                    "CARGO [{}{}] {:.0}%",
                    "#".repeat(n),
                    "-".repeat(10 - n),
                    c * 100.0
                ));
            }
        }
        None => {
            let g = me.guns[(me.slot as usize).min(me.guns.len() - 1)];
            out.push(format!(
                "{}  {} / {}",
                me.loadout.slot(me.slot).stats().name,
                g.mag,
                g.reserve
            ));
        }
    }
    if let Some(t) = hint.and_then(|h| h.0)
        && t != crate::interact::Target::Leave
    {
        out.push(t.hint());
    }
    if let Some(k) = me.carrying {
        out.push(format!("Carrying {} (wheel: PLACE)", k.name()));
    }
    let attacked = game
        .state
        .continents
        .iter()
        .flat_map(|c| c.bases.iter())
        .filter(|b| b.owner == Some(game.team) && b.hack.is_some())
        .count();
    if attacked > 0 {
        out.push(format!("{attacked} base(s) under attack"));
    }
    out
}

/// The plate over the left wrist, following the hand.
#[allow(clippy::too_many_arguments)]
fn wrist(
    mut commands: Commands,
    mut w: Local<Wrist>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    mats: Option<Res<Mats>>,
    game: Option<Res<Game>>,
    hint: Option<Res<crate::interact::UseHint>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(mats) = mats else { return };
    let lines = game
        .as_deref()
        .map(|g| wrist_lines(g, hint.as_deref()))
        .unwrap_or_default();
    let grip = input.hands[0].grip;
    if lines != w.lines {
        if let Some(r) = w.root.take() {
            commands.entity(r).despawn();
        }
        w.lines = lines.clone();
        if !lines.is_empty() {
            let root = commands
                .spawn((Transform::default(), Visibility::default(), ChildOf(rig.0)))
                .id();
            let (h, step, width) = (0.011, 0.017, 0.15);
            let height = step * lines.len() as f32 + 0.012;
            commands.spawn((
                Mesh3d(mats.quad.clone()),
                MeshMaterial3d(mats.back.clone()),
                Transform::from_xyz(0.0, -height / 2.0 + 0.006, -0.001)
                    .with_scale(Vec3::new(width, height, 1.0)),
                ChildOf(root),
            ));
            for (i, l) in lines.iter().enumerate() {
                let mesh = meshes.add(crate::vfont::mesh(l, h, 600).to_mesh());
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(if i == 0 {
                        mats.text.clone()
                    } else {
                        mats.dim.clone()
                    }),
                    Transform::from_xyz(-width / 2.0 + 0.008, -(i as f32) * step - h / 2.0, 0.001),
                    ChildOf(root),
                ));
            }
            w.root = Some(root);
        }
    }
    // Over the back of the left wrist, tilted towards the eyes.
    if let (Some(root), Some(g)) = (w.root, grip) {
        let t = g * Transform::from_xyz(0.0, 0.05, 0.1).with_rotation(Quat::from_rotation_x(-0.9));
        if let Ok(mut rt) = transforms.get_mut(root) {
            *rt = t;
        }
    }
}

/// The terminal and console panels' rows (as src/interact.rs's flat ones).
fn terminal(game: &Game, panel: &Panel, t: Target) -> Vec<Row> {
    use ps_sim::vehicle::VehicleKind;
    use ps_sim::weapons::{ArmorKind, Kit, Weapon};
    let close = Row::Buttons(vec![("CLOSE".into(), Act::Pick(Pick::Close), false)], 1);
    match t {
        Target::Construction(v) => {
            use ps_sim::construct::StructureKind;
            let have = crate::build::load(game, v);
            let off = 1.0 - crate::hud::lock_discount(game);
            let mut rows = vec![
                Row::Title("CONSTRUCTION".into()),
                Row::Text(format!("The harvester's load: {have:.0}")),
            ];
            if let Some(k) = crate::build::carrying(game) {
                rows.push(Row::Text(format!(
                    "Carrying a {}: place it from the wheel (hold Y)",
                    k.name()
                )));
            }
            rows.push(Row::Buttons(
                StructureKind::ALL
                    .iter()
                    .map(|k| {
                        (
                            format!("{} ({:.0})", k.name().to_uppercase(), k.cost() * off),
                            Act::Pick(Pick::Pull(*k)),
                            false,
                        )
                    })
                    .collect(),
                1,
            ));
            rows.push(Row::Text(
                "They draw on a mobile silo within 120 m; without one they decay".into(),
            ));
            rows.push(close);
            rows
        }
        Target::Equipment => {
            let l = panel.loadout.unwrap_or_default();
            let weapons = |list: &[Weapon], cur: Weapon, f: fn(Weapon) -> Pick| {
                Row::Buttons(
                    list.iter()
                        .map(|w| (w.stats().name.to_string(), Act::Pick(f(*w)), *w == cur))
                        .collect(),
                    3,
                )
            };
            let mut rows = vec![
                Row::Title("EQUIPMENT".into()),
                Row::Text("Armor".into()),
                Row::Buttons(
                    ArmorKind::ALL
                        .iter()
                        .map(|a| {
                            (
                                a.name().to_string(),
                                Act::Pick(Pick::Armor(*a)),
                                *a == l.armor,
                            )
                        })
                        .collect(),
                    3,
                ),
            ];
            if l.max() {
                rows.push(Row::Text("Arms".into()));
                rows.push(weapons(&Weapon::MAX_ARMS, l.rifle, Pick::Rifle));
                rows.push(Row::Text(crate::interact::armor_note(ArmorKind::Max)));
            } else {
                if l.armor == ArmorKind::Stealth {
                    rows.push(Row::Text(
                        crate::interact::armor_note(ArmorKind::Stealth)
                            .replace("[B]", "The right stick pulled back"),
                    ));
                }
                rows.push(Row::Text("Rifle".into()));
                rows.push(weapons(
                    &Weapon::rifles_for(game.team),
                    l.rifle,
                    Pick::Rifle,
                ));
                rows.push(Row::Text("Sidearm".into()));
                rows.push(weapons(&Weapon::SIDEARMS, l.sidearm, Pick::Sidearm));
                rows.push(Row::Text("Special".into()));
                rows.push(weapons(&Weapon::SPECIALS, l.special, Pick::Special));
                rows.push(Row::Text("Grenade".into()));
                rows.push(Row::Buttons(
                    ps_sim::weapons::GrenadeKind::ALL
                        .iter()
                        .map(|g| {
                            (
                                g.name().to_string(),
                                Act::Pick(Pick::Grenade(*g)),
                                *g == l.grenade,
                            )
                        })
                        .collect(),
                    4,
                ));
                if l.special == Weapon::RepairGun {
                    rows.push(Row::Text(
                        "Engineer's kit (right stick back puts it down)".into(),
                    ));
                    rows.push(Row::Buttons(
                        Kit::ALL
                            .iter()
                            .map(|k| (k.name().to_string(), Act::Pick(Pick::Kit(*k)), *k == l.kit))
                            .collect(),
                        2,
                    ));
                }
            }
            rows.push(Row::Buttons(
                vec![
                    ("EQUIP".into(), Act::Pick(Pick::Equip), true),
                    ("CLOSE".into(), Act::Pick(Pick::Close), false),
                ],
                2,
            ));
            rows
        }
        Target::Vehicles(..) => {
            let picked = panel.vehicle;
            let mut rows = vec![
                Row::Title("VEHICLES".into()),
                Row::Buttons(
                    VehicleKind::ALL
                        .iter()
                        .map(|k| {
                            (
                                k.name().to_string(),
                                Act::Pick(Pick::Vehicle(*k)),
                                picked.is_some_and(|p| p.0 == *k),
                            )
                        })
                        .collect(),
                    2,
                ),
            ];
            if let Some((k, l)) = picked {
                let guns = k.gun_options();
                if guns.len() > 1 {
                    rows.push(Row::Text("Main gun".into()));
                    rows.push(Row::Buttons(
                        guns.iter()
                            .enumerate()
                            .map(|(i, g)| {
                                (
                                    g.name.to_string(),
                                    Act::Pick(Pick::VehicleGun(i as u8)),
                                    l.weapon == i as u8,
                                )
                            })
                            .collect(),
                        2,
                    ));
                }
                rows.push(Row::Text("Defence (X in the driver's seat)".into()));
                rows.push(Row::Buttons(
                    k.defenses()
                        .iter()
                        .map(|d| {
                            (
                                d.name().to_string(),
                                Act::Pick(Pick::VehicleDefense(*d)),
                                l.defense == *d,
                            )
                        })
                        .collect(),
                    3,
                ));
                rows.push(Row::Buttons(
                    vec![
                        (
                            format!("ORDER {}", k.name().to_uppercase()),
                            Act::Pick(Pick::Order),
                            true,
                        ),
                        ("CLOSE".into(), Act::Pick(Pick::Close), false),
                    ],
                    2,
                ));
            } else {
                rows.push(close);
            }
            rows
        }
        Target::Console(b) => {
            let name = game
                .map
                .as_ref()
                .map_or(String::new(), |m| m.def.bases[b as usize].name.clone());
            let mut rows = vec![
                Row::Title(format!("{} CORE", name.to_uppercase())),
                Row::Text("Slide every piece into the square".into()),
            ];
            if let Some((pb, seed, at)) = &game.puzzle
                && *pb == b
            {
                rows.push(Row::Cells(puzzle_cells(*seed, at)));
            } else {
                rows.push(Row::Text("Connecting...".into()));
            }
            rows.push(close);
            rows
        }
        _ => vec![close],
    }
}

fn puzzle_cells(seed: u32, at: &[u8]) -> Vec<Vec<Cell>> {
    let puzzle = ps_sim::puzzle::Puzzle::generate(seed);
    let n = ps_sim::puzzle::N as i8;
    let d = ps_sim::puzzle::DEPTH as i8;
    let hues = [
        Vec3::new(1.0, 0.3, 0.6),
        Vec3::new(0.2, 0.9, 1.0),
        Vec3::new(1.0, 0.8, 0.2),
        Vec3::new(0.5, 1.0, 0.4),
        Vec3::new(0.7, 0.4, 1.0),
        Vec3::new(1.0, 0.5, 0.2),
        Vec3::new(0.4, 0.6, 1.0),
        Vec3::new(0.9, 0.9, 0.9),
    ];
    (-d..n + d)
        .rev()
        .map(|r| {
            (-d..n + d)
                .map(|c| {
                    let inside = (0..n).contains(&c) && (0..n).contains(&r);
                    let piece = puzzle.piece_at(at, c, r);
                    (
                        piece.map(|i| hues[i % hues.len()]),
                        inside,
                        piece.map(|i| i as u8),
                    )
                })
                .collect()
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn contents(
    mut commands: Commands,
    mut ui: ResMut<VrUi>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    mats: Option<Res<Mats>>,
    game: Option<Res<Game>>,
    panel: Option<Res<Panel>>,
    model: Option<ResMut<MenuModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    local: Res<LocalCmd>,
    login: Option<Res<crate::login::LoginForm>>,
) {
    let Some(mats) = mats else { return };
    let local_yaw = local.cmd.yaw;
    // The left menu button: the menu, in a game.
    let menu_now = input.hands[0].menu || input.hands[1].menu;
    if menu_now && !ui.menu_held && game.is_some() {
        ui.menu = !ui.menu;
    }
    ui.menu_held = menu_now;
    // The left stick clicked without pushing it (pushed, the click sprints): the map.
    let l = input.hands[0];
    let click = l.stick_click && l.stick.length() < 0.35;
    if click && !ui.stick_held && game.is_some() {
        ui.map = !ui.map;
    }
    ui.stick_held = l.stick_click;
    if game.is_none() {
        ui.menu = false;
        ui.map = false;
    } else if std::mem::take(&mut ui.map_env) {
        ui.map = true;
    }
    let want = wanted(
        &mut ui,
        game.as_deref(),
        panel.as_deref(),
        model.map(|m| m.into_inner()),
        login.as_deref(),
    );
    if want == ui.shown {
        // The map's arrow: where one is, which way one looks.
        if let (Some((e, corner, k)), Some(g)) = (ui.marker, game.as_deref())
            && let Some(me) = g.mine()
        {
            let p = me.feet;
            let at = corner
                + Vec2::new(
                    (p.x + ps_sim::terrain::BOUND) / k,
                    -(p.z + ps_sim::terrain::BOUND) / k,
                );
            let yaw = local_yaw;
            commands.entity(e).insert(
                Transform::from_xyz(at.x, at.y, 0.006).with_rotation(Quat::from_rotation_z(yaw)),
            );
        }
        // Up but out of view (or placed before the head was tracked): in front again.
        if let (Some(root), Some(head)) = (ui.root, input.head) {
            let to = ui.pose.translation - head.translation;
            let f = head.rotation * Vec3::NEG_Z;
            let off = Vec3::new(to.x, 0.0, to.z)
                .normalize_or_zero()
                .dot(Vec3::new(f.x, 0.0, f.z).normalize_or_zero());
            if !ui.placed_by_head || off < 0.26 {
                let pose = place(&head, ui.height);
                ui.pose = pose;
                ui.placed_by_head = true;
                commands.entity(root).insert(pose);
            }
        }
        return;
    }
    let was_up = ui.shown.is_some();
    if let Some(r) = ui.root.take() {
        commands.entity(r).despawn();
    }
    ui.buttons.clear();
    ui.hovered = None;
    ui.marker = None;
    ui.shown = want.clone();
    let Some(rows) = want else { return };
    // Placed in front of the eyes when it comes up (it stays put while it's up).
    let place_new = !was_up;
    let team = game.as_ref().map(|g| g.team);
    let edge = if team.is_some() {
        accents.add(AccentMaterial {
            neon: NeonUniform::fixed(team_color(team), 2.0),
            overlay: true,
        })
    } else {
        mats.edge.clone()
    };
    let root = commands
        .spawn((ui.pose, Visibility::default(), ChildOf(rig.0)))
        .id();
    ui.root = Some(root);
    // The rows go down from the top edge of `content`, which is moved up by half the height
    // once that is known (the root at the panel's middle).
    let content = commands
        .spawn((Transform::default(), Visibility::default(), ChildOf(root)))
        .id();
    let mut y = -PAD;
    let inner = WIDTH - 2.0 * PAD;
    let text = |commands: &mut Commands,
                meshes: &mut Assets<Mesh>,
                s: &str,
                h: f32,
                at: Vec2,
                mat: &Handle<AccentMaterial>| {
        let w = crate::vfont::width(s, h, 600);
        let mesh = meshes.add(crate::vfont::mesh(s, h, 600).to_mesh());
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(at.x - w / 2.0, at.y - h / 2.0, 0.006),
            ChildOf(content),
        ));
    };
    let mut buttons = Vec::new();
    for row in &rows {
        match row {
            Row::Title(s) => {
                let h = 0.045;
                y -= h / 2.0 + GAP;
                text(
                    &mut commands,
                    &mut meshes,
                    s,
                    h,
                    Vec2::new(0.0, y),
                    &mats.text,
                );
                y -= h / 2.0 + GAP;
            }
            Row::Text(s) => {
                let h = 0.025;
                y -= h / 2.0 + GAP;
                text(
                    &mut commands,
                    &mut meshes,
                    s,
                    h,
                    Vec2::new(0.0, y),
                    &mats.dim,
                );
                y -= h / 2.0;
            }
            Row::Buttons(list, cols) => {
                let cols = (*cols).max(1);
                let w = (inner - GAP * (cols - 1) as f32) / cols as f32;
                for (i, (label, act, lit)) in list.iter().enumerate() {
                    let (c, r) = (i % cols, i / cols);
                    let center = Vec2::new(
                        -inner / 2.0 + w / 2.0 + c as f32 * (w + GAP),
                        y - GAP - BUTTON_H / 2.0 - r as f32 * (BUTTON_H + GAP),
                    );
                    let half = Vec2::new(w / 2.0, BUTTON_H / 2.0);
                    let e = commands
                        .spawn((
                            Mesh3d(mats.quad.clone()),
                            MeshMaterial3d(if *lit {
                                mats.lit.clone()
                            } else {
                                mats.button.clone()
                            }),
                            Transform::from_xyz(center.x, center.y, 0.003)
                                .with_scale(Vec3::new(w, BUTTON_H, 1.0)),
                            ChildOf(content),
                        ))
                        .id();
                    // Long labels shrink to fit.
                    let mut h = 0.026;
                    while h > 0.012 && crate::vfont::width(label, h, 600) > w - 0.02 {
                        h *= 0.9;
                    }
                    text(&mut commands, &mut meshes, label, h, center, &mats.text);
                    buttons.push(Button {
                        act: act.clone(),
                        center,
                        half,
                        entity: e,
                    });
                }
                let lines = list.len().div_ceil(cols);
                y -= GAP + lines as f32 * (BUTTON_H + GAP);
            }
            Row::Map(_) => {
                if let Some(g) = game.as_deref()
                    && let Some(m) = g.map.as_ref()
                {
                    let land = match &ui.land {
                        Some((c, h)) if Some(*c) == g.continent => h.clone(),
                        _ => {
                            let h = meshes.add(land_mesh(m).to_mesh());
                            ui.land = g.continent.map(|c| (c, h.clone()));
                            h
                        }
                    };
                    let used = draw_map(
                        &mut commands,
                        content,
                        &mut meshes,
                        &mut accents,
                        &mats,
                        g,
                        m,
                        land,
                        y - GAP,
                        &mut ui.marker,
                    );
                    y -= GAP + used;
                }
            }
            Row::Cells(grid) => {
                let n = grid.first().map_or(0, |r| r.len());
                let x0 = -(n as f32) * CELL / 2.0;
                for (r, line) in grid.iter().enumerate() {
                    for (c, (colour, inside, piece)) in line.iter().enumerate() {
                        let center = Vec2::new(
                            x0 + (c as f32 + 0.5) * CELL,
                            y - GAP - (r as f32 + 0.5) * CELL,
                        );
                        let mat = match colour {
                            Some(col) => accents.add(AccentMaterial {
                                neon: NeonUniform::fixed(*col, 1.0),
                                overlay: true,
                            }),
                            None if *inside => mats.button.clone(),
                            None => continue,
                        };
                        let e = commands
                            .spawn((
                                Mesh3d(mats.quad.clone()),
                                MeshMaterial3d(mat),
                                Transform::from_xyz(center.x, center.y, 0.003)
                                    .with_scale(Vec3::new(CELL * 0.92, CELL * 0.92, 1.0)),
                                ChildOf(content),
                            ))
                            .id();
                        if let Some(p) = piece {
                            buttons.push(Button {
                                act: Act::Pick(Pick::Piece(*p)),
                                center,
                                half: Vec2::splat(CELL / 2.0),
                                entity: e,
                            });
                        }
                    }
                }
                y -= GAP + grid.len() as f32 * CELL;
            }
        }
    }
    let height = -y + PAD;
    if place_new {
        ui.placed_by_head = input.head.is_some();
        let head = input
            .head
            .unwrap_or(Transform::from_xyz(0.0, ps_sim::player::EYE, 0.0));
        ui.pose = place(&head, height);
    } else {
        // Same top-centre: the height may have changed.
        let top = ui.pose.translation + ui.pose.rotation * Vec3::new(0.0, ui.height / 2.0, 0.0);
        ui.pose.translation = top - ui.pose.rotation * Vec3::new(0.0, height / 2.0, 0.0);
    }
    ui.height = height;
    // The backing and its edge.
    commands.spawn((
        Mesh3d(mats.quad.clone()),
        MeshMaterial3d(mats.back.clone()),
        Transform::from_xyz(0.0, -height / 2.0, 0.0).with_scale(Vec3::new(WIDTH, height, 1.0)),
        ChildOf(content),
    ));
    for (at, size) in [
        (Vec2::new(0.0, 0.0), Vec2::new(WIDTH, 0.006)),
        (Vec2::new(0.0, -height), Vec2::new(WIDTH, 0.006)),
        (
            Vec2::new(-WIDTH / 2.0, -height / 2.0),
            Vec2::new(0.006, height),
        ),
        (
            Vec2::new(WIDTH / 2.0, -height / 2.0),
            Vec2::new(0.006, height),
        ),
    ] {
        commands.spawn((
            Mesh3d(mats.quad.clone()),
            MeshMaterial3d(edge.clone()),
            Transform::from_xyz(at.x, at.y, 0.002).with_scale(size.extend(1.0)),
            ChildOf(content),
        ));
    }
    // The rows were laid out down from the panel's top edge: the root at its centre.
    commands
        .entity(content)
        .insert(Transform::from_xyz(0.0, height / 2.0, 0.0));
    commands.entity(root).insert(ui.pose);
    debug!(
        "vr panel: {} rows, {} buttons, {:.2} m tall, at {:.2} (head {:?})",
        rows.len(),
        buttons.len(),
        height,
        ui.pose.translation,
        input.head.map(|h| h.translation)
    );
    ui.buttons = buttons
        .into_iter()
        .map(|mut b| {
            b.center.y += height / 2.0;
            b
        })
        .collect();
}

/// A panel `height` tall in front of `head` (tracking space), its middle a little under the
/// eyes, facing them (the panel's front is +Z).
fn place(head: &Transform, height: f32) -> Transform {
    let f = head.rotation * Vec3::NEG_Z;
    let flat = Vec3::new(f.x, 0.0, f.z).normalize_or(Vec3::NEG_Z);
    let at = head.translation + flat * DISTANCE - Vec3::Y * (0.15 + height * 0.1);
    // (Looking away from the eyes puts the front, +Z, towards them.)
    Transform::from_translation(at).looking_at(at + flat, Vec3::Y)
}

/// The hands' lasers on the panel, and the trigger pressing the button pointed at.
#[allow(clippy::too_many_arguments)]
fn point(
    mut commands: Commands,
    mut ui: ResMut<VrUi>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    mats: Option<Res<Mats>>,
    mut local: ResMut<LocalCmd>,
    mut game: Option<ResMut<Game>>,
    mut panel: Option<ResMut<Panel>>,
    mut overlays: ResMut<Overlays>,
    mut model: Option<ResMut<MenuModel>>,
    menu_roots: Query<Entity, With<MenuRoot>>,
    mut leave: MessageWriter<LeaveGame>,
    mut exit: MessageWriter<AppExit>,
    mut transforms: Query<&mut Transform>,
    mut materials: Query<&mut MeshMaterial3d<AccentMaterial>>,
    mut login: Option<ResMut<crate::login::LoginForm>>,
) {
    let Some(mats) = mats else { return };
    let up = ui.root.is_some();
    // The gun holds its fire while a panel is up.
    if up {
        local.cmd.fire = false;
    }
    let inv = ui.pose.compute_affine().inverse();
    let mut hovered = None;
    let mut pressed = None;
    for h in 0..2 {
        let trigger = input.hands[h].trigger > 0.6;
        let edge = trigger && !ui.trigger_held[h];
        ui.trigger_held[h] = trigger;
        let aim = input.hands[h].aim.filter(|_| up);
        let Some(aim) = aim else {
            if let Some(l) = ui.lasers[h].take() {
                commands.entity(l).despawn();
            }
            continue;
        };
        let o = aim.translation;
        let d = aim.rotation * Vec3::NEG_Z;
        let (lo, ld) = (inv.transform_point3(o), inv.transform_vector3(d));
        let mut length = 1.5;
        if ld.z < -1e-4 {
            let t = -lo.z / ld.z;
            if t > 0.0 {
                let p = (lo + ld * t).truncate();
                length = t;
                if let Some(i) = ui
                    .buttons
                    .iter()
                    .position(|b| (p - b.center).abs().cmple(b.half).all())
                {
                    hovered = Some(i);
                    if edge {
                        pressed = Some(i);
                    }
                }
            }
        }
        let laser = *ui.lasers[h].get_or_insert_with(|| {
            commands
                .spawn((
                    Mesh3d(mats.beam.clone()),
                    MeshMaterial3d(mats.laser.clone()),
                    Transform::default(),
                    ChildOf(rig.0),
                ))
                .id()
        });
        let t = Transform::from_translation(o + d * length / 2.0)
            .looking_to(d, Vec3::Y)
            .with_scale(Vec3::new(1.0, 1.0, length));
        if let Ok(mut lt) = transforms.get_mut(laser) {
            *lt = t;
        } else {
            commands.entity(laser).insert(t);
        }
    }
    // Hover: lit.
    if hovered != ui.hovered {
        for (i, b) in ui.buttons.iter().enumerate() {
            if Some(i) == hovered || Some(i) == ui.hovered {
                let lit = matches!(ui.shown.as_ref(), Some(rows) if is_lit(rows, &b.act));
                if let Ok(mut m) = materials.get_mut(b.entity)
                    && !matches!(b.act, Act::Pick(Pick::Piece(_)))
                {
                    m.0 = if Some(i) == hovered {
                        mats.hover.clone()
                    } else if lit {
                        mats.lit.clone()
                    } else {
                        mats.button.clone()
                    };
                }
            }
        }
        ui.hovered = hovered;
    }
    let Some(i) = pressed else { return };
    let act = ui.buttons[i].act.clone();
    match act {
        Act::Login(a) => {
            if let (Some(form), Some(g)) = (login.as_deref_mut(), game.as_deref_mut()) {
                form.act(a, g, &mut leave);
            }
        }
        Act::Menu(a) => {
            if let Some(m) = model.as_deref_mut() {
                match m.apply(&a) {
                    Outcome::Start(s) => {
                        for e in &menu_roots {
                            commands.entity(e).despawn();
                        }
                        commands.insert_resource(s);
                    }
                    Outcome::Quit => {
                        exit.write(AppExit::Success);
                    }
                    _ => {}
                }
            }
        }
        Act::Pick(p) => {
            if let (Some(g), Some(pn)) = (game.as_deref_mut(), panel.as_deref_mut()) {
                crate::interact::apply(p, g, pn, &mut overlays);
            }
        }
        Act::Spawn(s) => {
            if let Some(g) = game.as_deref_mut() {
                g.request(Request::Spawn(s));
            }
        }
        Act::Resume => ui.menu = false,
        Act::Suicide => {
            ui.menu = false;
            if let Some(g) = game.as_deref_mut() {
                g.request(Request::Suicide);
            }
        }
        Act::Map(open) => {
            ui.map = open;
            ui.menu = false;
        }
        Act::SquadPanel(open) => {
            ui.squad = open;
            ui.menu = false;
        }
        Act::ScoresPanel(open) => {
            ui.scores = open;
            ui.menu = false;
        }
        Act::StatsPanel(open) => {
            ui.stats = open;
            ui.menu = false;
            if open && let Some(g) = game.as_deref_mut() {
                g.stats = None;
                g.client.ask_stats();
            }
        }
        Act::Squad(c) => {
            if let Some(g) = game.as_deref_mut() {
                g.squad(c);
            }
        }
        Act::Leave => {
            ui.menu = false;
            leave.write(LeaveGame(None));
        }
        Act::Quit => {
            exit.write(AppExit::Success);
        }
    }
}

/// Logging in (or registering) in VR: the fields (pointed at to type into), a keyboard, the
/// buttons.
fn login_rows(game: &Game, form: &crate::login::LoginForm) -> Vec<Row> {
    use crate::login::LoginAct;
    let mut rows = vec![
        Row::Title(if form.register { "REGISTER" } else { "LOG IN" }.into()),
        Row::Buttons(
            vec![
                (
                    "LOG IN".into(),
                    Act::Login(LoginAct::Mode(false)),
                    !form.register,
                ),
                (
                    "REGISTER".into(),
                    Act::Login(LoginAct::Mode(true)),
                    form.register,
                ),
            ],
            2,
        ),
    ];
    let fields: &[(&str, u8)] = if form.register {
        &[("NAME", 0), ("PASSWORD", 1), ("AGAIN", 2)]
    } else {
        &[("NAME", 0), ("PASSWORD", 1)]
    };
    rows.push(Row::Buttons(
        fields
            .iter()
            .map(|&(label, f)| {
                let caret = if form.focus == f { "_" } else { "" };
                (
                    format!("{label}: {}{caret}", form.shown(f)),
                    Act::Login(LoginAct::Field(f)),
                    form.focus == f,
                )
            })
            .collect(),
        1,
    ));
    for line in ["1234567890", "qwertyuiop", "asdfghjkl-", "zxcvbnm_.@"] {
        rows.push(Row::Buttons(
            line.chars()
                .map(|c| {
                    let shown = if form.caps { c.to_ascii_uppercase() } else { c };
                    (shown.to_string(), Act::Login(LoginAct::Char(c)), false)
                })
                .collect(),
            line.len(),
        ));
    }
    rows.push(Row::Buttons(
        vec![
            ("CAPS".into(), Act::Login(LoginAct::Caps), form.caps),
            ("SPACE".into(), Act::Login(LoginAct::Char(' ')), false),
            ("DELETE".into(), Act::Login(LoginAct::Back), false),
        ],
        3,
    ));
    if let Some(s) = form.status(game) {
        rows.push(Row::Text(s));
    }
    rows.push(Row::Buttons(
        vec![
            (
                if form.register { "REGISTER" } else { "LOG IN" }.into(),
                Act::Login(LoginAct::Submit),
                true,
            ),
            ("LEAVE".into(), Act::Login(LoginAct::Leave), false),
        ],
        2,
    ));
    rows
}

/// The scoreboard (as the flat screen's, shorter): each faction's best on the continent.
fn score_rows(game: &Game) -> Vec<Row> {
    let mut rows = vec![Row::Title("SCORES".into())];
    if let Some(xp) = game.xp {
        rows.push(Row::Text(format!(
            "BATTLE RANK {}   {xp} XP",
            ps_sim::progress::battle_rank(xp)
        )));
    }
    for t in ps_sim::Team::ALL {
        let mut list: Vec<_> = game.scores.iter().filter(|s| s.team == t).collect();
        if list.is_empty() {
            continue;
        }
        list.sort_by_key(|s| std::cmp::Reverse(s.score));
        let total: u32 = list.iter().map(|s| s.score).sum();
        rows.push(Row::Text(format!(
            "{}  ({})  {total} XP",
            t.upper(),
            list.len()
        )));
        for s in list.iter().take(5) {
            let mut name = game.name_of(s.id);
            name.truncate(14);
            rows.push(Row::Text(format!(
                "   {name}   K {}  D {}  A {}   {}",
                s.kills, s.deaths, s.assists, s.score
            )));
        }
    }
    rows.push(Row::Buttons(
        vec![("CLOSE".into(), Act::ScoresPanel(false), false)],
        1,
    ));
    rows
}

/// The squad panel: one's squad (the leader can hand on the lead) and leaving it, or starting
/// one; the faction's other squads to join.
fn squad_rows(game: &Game) -> Vec<Row> {
    use ps_net::protocol::{SQUAD_MAX, SquadCmd};
    let mut rows = vec![Row::Title("SQUAD".into())];
    let mine = game.my_squad();
    match mine {
        Some(s) => {
            let leading = game.leading();
            rows.push(Row::Text(format!(
                "YOUR SQUAD {}/{SQUAD_MAX}",
                s.members.len()
            )));
            let names: Vec<String> = s
                .members
                .iter()
                .map(|&m| {
                    let lead = if m == s.leader { "* " } else { "" };
                    format!("{lead}{}", game.name_of(m).to_uppercase())
                })
                .collect();
            rows.push(Row::Text(names.join("  ")));
            let mut b = Vec::new();
            if leading {
                for &m in s.members.iter().filter(|&&m| Some(m) != game.me) {
                    b.push((
                        format!("LEAD: {}", game.name_of(m).to_uppercase()),
                        Act::Squad(SquadCmd::Promote(m)),
                        false,
                    ));
                }
                if s.waypoint.is_some() {
                    b.push((
                        "CLEAR WAYPOINT".into(),
                        Act::Squad(SquadCmd::Waypoint(None)),
                        false,
                    ));
                }
            }
            b.push(("LEAVE SQUAD".into(), Act::Squad(SquadCmd::Leave), false));
            rows.push(Row::Buttons(b, 2));
        }
        None => rows.push(Row::Buttons(
            vec![("CREATE SQUAD".into(), Act::Squad(SquadCmd::Create), false)],
            1,
        )),
    }
    let others: Vec<(String, Act, bool)> = game
        .squads
        .iter()
        .filter(|s| Some(s.id) != mine.map(|m| m.id) && s.members.len() < SQUAD_MAX)
        .take(8)
        .map(|s| {
            (
                format!(
                    "JOIN {} ({}/{SQUAD_MAX})",
                    game.name_of(s.leader).to_uppercase(),
                    s.members.len()
                ),
                Act::Squad(SquadCmd::Join(s.id)),
                false,
            )
        })
        .collect();
    if !others.is_empty() {
        rows.push(Row::Text("YOUR FACTION'S SQUADS".into()));
        rows.push(Row::Buttons(others, 2));
    }
    rows.push(Row::Buttons(
        vec![("CLOSE".into(), Act::SquadPanel(false), false)],
        1,
    ));
    rows
}

fn is_lit(rows: &[Row], act: &Act) -> bool {
    rows.iter().any(|r| match r {
        Row::Buttons(list, _) => list.iter().any(|(_, a, lit)| a == act && *lit),
        _ => false,
    })
}

/// Size of the continent's square on the map panel, and of the world graph's (m).
const MAP_S: f32 = 0.64;
const WORLD_S: f32 = 0.34;
/// Cells along the map's land.
const LAND_N: usize = 72;

/// The continent's land for the map: a grid of quads coloured by height (water blue), in a
/// unit square (x right, y up: north, -Z, at the top), drawn in vertex colours.
fn land_mesh(m: &ps_sim::map::ContinentMap) -> crate::assets::MeshData {
    let b = ps_sim::terrain::BOUND;
    let t = &m.terrain;
    let mut d = crate::assets::MeshData::default();
    let step = 1.0 / LAND_N as f32;
    for j in 0..LAND_N {
        for i in 0..LAND_N {
            let (u, v) = ((i as f32 + 0.5) * step, (j as f32 + 0.5) * step);
            let (x, z) = (-b + u * 2.0 * b, -b + v * 2.0 * b);
            let land = t.height(x, z);
            let water = t.water(x, z);
            let col = if water > land + 0.3 {
                [0.03, 0.08, 0.22, 1.0]
            } else {
                let k = (land / 160.0).clamp(0.0, 1.0);
                let c = Vec3::new(0.07, 0.09, 0.11).lerp(Vec3::new(0.32, 0.36, 0.42), k);
                [c.x, c.y, c.z, 1.0]
            };
            let (x0, x1) = (i as f32 * step, (i + 1) as f32 * step);
            let (y0, y1) = (-(j as f32) * step, -((j + 1) as f32) * step);
            let n = d.pos.len() as u32;
            for (px, py) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                d.pos.push([px, py, 0.0]);
                d.nrm.push([0.0, 0.0, 1.0]);
                d.uv.push([0.0, 0.0]);
                d.col.push(col);
            }
            d.idx.extend([n, n + 2, n + 1, n, n + 3, n + 2]);
        }
    }
    d
}

/// The "you are here" arrow: pointing +Y.
fn arrow_mesh() -> crate::assets::MeshData {
    let mut d = crate::assets::MeshData::default();
    for p in [[0.0, 0.016, 0.0], [-0.009, -0.01, 0.0], [0.009, -0.01, 0.0]] {
        d.pos.push(p);
        d.nrm.push([0.0, 0.0, 1.0]);
        d.uv.push([0.0, 0.0]);
    }
    d.idx.extend([0, 1, 2]);
    d
}

fn flat(accents: &mut Assets<AccentMaterial>, c: Vec3, i: f32) -> Handle<AccentMaterial> {
    accents.add(AccentMaterial {
        neon: NeonUniform::fixed(c, i),
        overlay: true,
    })
}

/// A thin line from `a` to `b` (panel metres).
#[allow(clippy::too_many_arguments)]
fn line(
    commands: &mut Commands,
    parent: Entity,
    quad: &Handle<Mesh>,
    mat: Handle<AccentMaterial>,
    a: Vec2,
    b: Vec2,
    w: f32,
    z: f32,
) {
    let d = b - a;
    let mid = (a + b) / 2.0;
    commands.spawn((
        Mesh3d(quad.clone()),
        MeshMaterial3d(mat),
        Transform::from_xyz(mid.x, mid.y, z)
            .with_rotation(Quat::from_rotation_z(d.y.atan2(d.x)))
            .with_scale(Vec3::new(d.length(), w, 1.0)),
        ChildOf(parent),
    ));
}

/// The map under `top`: the continent (land, lattice, gates, bases by owner, hacks, the
/// arrow) on the left, the world (continents by owner, gates' links) on the right. Its height.
#[allow(clippy::too_many_arguments)]
fn draw_map(
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    accents: &mut Assets<AccentMaterial>,
    mats: &Mats,
    game: &Game,
    m: &ps_sim::map::ContinentMap,
    land: Handle<Mesh>,
    top: f32,
    marker: &mut Option<(Entity, Vec2, f32)>,
) -> f32 {
    use ps_sim::worldgen::{BaseKind, Node};
    let b = ps_sim::terrain::BOUND;
    let inner = WIDTH - 2.0 * PAD;
    let corner = Vec2::new(-inner / 2.0, top);
    let k = 2.0 * b / MAP_S;
    let at = |p: Vec3| corner + Vec2::new((p.x + b) / k, -(p.z + b) / k);
    let mut tinted = NeonUniform::fixed(Vec3::ONE, 1.0);
    tinted.params.x = 3.0;
    let land_mat = accents.add(AccentMaterial {
        neon: tinted,
        overlay: true,
    });
    commands.spawn((
        Mesh3d(land),
        MeshMaterial3d(land_mat),
        Transform::from_xyz(corner.x, corner.y, 0.002).with_scale(Vec3::new(MAP_S, MAP_S, 1.0)),
        ChildOf(parent),
    ));
    let c = game.continent.unwrap_or(0);
    let st = &game.state.continents[c as usize];
    let node = |n: Node| match n {
        Node::Base(i) => m.bases[i as usize].center,
        Node::Gate(g) => m.gates[g as usize],
    };
    // The lattice.
    let link = flat(accents, Vec3::new(0.6, 0.65, 0.8), 0.8);
    for (a, bb) in &m.def.lattice {
        line(
            commands,
            parent,
            &mats.quad,
            link.clone(),
            at(node(*a)),
            at(node(*bb)),
            0.002,
            0.003,
        );
    }
    // Gates.
    let gate = flat(accents, Vec3::new(0.2, 0.9, 1.0), 2.0);
    for g in &m.gates {
        let p = at(*g);
        commands.spawn((
            Mesh3d(mats.quad.clone()),
            MeshMaterial3d(gate.clone()),
            Transform::from_xyz(p.x, p.y, 0.004)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
                .with_scale(Vec3::new(0.016, 0.016, 1.0)),
            ChildOf(parent),
        ));
    }
    // Bases: by owner (white: nobody's), an amber frame while hacked; names under the
    // bigger ones.
    let amber = flat(accents, Vec3::new(1.0, 0.6, 0.1), 2.5);
    for (i, layout) in m.bases.iter().enumerate() {
        let s = &st.bases[i];
        let size = match layout.kind {
            BaseKind::Tower => 0.012,
            BaseKind::Outpost => 0.022,
            _ => 0.03,
        };
        let p = at(layout.center);
        let col = match s.owner {
            Some(t) => team_color(Some(t)),
            None => Vec3::splat(0.85),
        };
        if s.hack.is_some() {
            commands.spawn((
                Mesh3d(mats.quad.clone()),
                MeshMaterial3d(amber.clone()),
                Transform::from_xyz(p.x, p.y, 0.004).with_scale(Vec3::splat(size + 0.01)),
                ChildOf(parent),
            ));
        }
        commands.spawn((
            Mesh3d(mats.quad.clone()),
            MeshMaterial3d(flat(accents, col, 1.5)),
            Transform::from_xyz(p.x, p.y, 0.005).with_scale(Vec3::splat(size)),
            ChildOf(parent),
        ));
        if layout.kind != BaseKind::Tower {
            let name = &m.def.bases[i].name;
            let h = 0.011;
            let w = crate::vfont::width(name, h, 500);
            commands.spawn((
                Mesh3d(meshes.add(crate::vfont::mesh(name, h, 500).to_mesh())),
                MeshMaterial3d(mats.dim.clone()),
                Transform::from_xyz(p.x - w / 2.0, p.y - size / 2.0 - h - 0.004, 0.005),
                ChildOf(parent),
            ));
        }
    }
    // The arrow (placed every frame).
    let arrow = commands
        .spawn((
            Mesh3d(meshes.add(arrow_mesh().to_mesh())),
            MeshMaterial3d(flat(accents, Vec3::new(1.0, 0.95, 0.3), 3.0)),
            Transform::from_xyz(corner.x, corner.y, 0.006),
            ChildOf(parent),
        ))
        .id();
    *marker = Some((arrow, corner, k));
    // The world: continents on the planet map's disc, by owner; this one ringed.
    let wc = Vec2::new(inner / 2.0 - WORLD_S / 2.0, top - WORLD_S / 2.0);
    let wp = |p: Vec2| wc + Vec2::new(p.x, -p.y) * (WORLD_S / 2.0 - 0.03);
    let wlink = flat(accents, Vec3::new(0.4, 0.45, 0.7), 0.8);
    for cd in &game.def.continents {
        for g in &cd.gates {
            if g.to > cd.id {
                let other = &game.def.continents[g.to as usize];
                line(
                    commands,
                    parent,
                    &mats.quad,
                    wlink.clone(),
                    wp(cd.map_pos),
                    wp(other.map_pos),
                    0.002,
                    0.003,
                );
            }
        }
    }
    for (ci, cd) in game.def.continents.iter().enumerate() {
        let p = wp(cd.map_pos);
        let owner = game.state.continents[ci].owner;
        let col = owner.map_or(Vec3::splat(0.6), |t| team_color(Some(t)));
        if Some(ci as u8) == game.continent {
            commands.spawn((
                Mesh3d(mats.quad.clone()),
                MeshMaterial3d(flat(accents, Vec3::new(1.0, 0.95, 0.3), 2.5)),
                Transform::from_xyz(p.x, p.y, 0.004).with_scale(Vec3::splat(0.034)),
                ChildOf(parent),
            ));
        }
        commands.spawn((
            Mesh3d(mats.quad.clone()),
            MeshMaterial3d(flat(accents, col, 1.5)),
            Transform::from_xyz(p.x, p.y, 0.005).with_scale(Vec3::splat(0.026)),
            ChildOf(parent),
        ));
        let h = 0.011;
        let w = crate::vfont::width(&cd.name, h, 500);
        commands.spawn((
            Mesh3d(meshes.add(crate::vfont::mesh(&cd.name, h, 500).to_mesh())),
            MeshMaterial3d(mats.dim.clone()),
            Transform::from_xyz(p.x - w / 2.0, p.y - 0.03, 0.005),
            ChildOf(parent),
        ));
    }
    MAP_S
}

/// Seated in VR (a vehicle, a turret): a ring where the gun points, another where the stick
/// is turning it to, and a line under the view with the vehicle's state (a harvester's load).
#[derive(Default)]
struct SeatHud {
    ring: Option<Handle<Mesh>>,
    reticle: Option<Entity>,
    target: Option<Entity>,
    amber: Option<Handle<AccentMaterial>>,
    line: Option<(String, Entity)>,
}

/// The ring's distance (m) and radii there.
const RETICLE_AT: f32 = 25.0;

#[allow(clippy::too_many_arguments)]
fn seat_hud(
    mut commands: Commands,
    mut st: Local<SeatHud>,
    mats: Option<Res<Mats>>,
    view: Res<crate::seatview::SeatView>,
    game: Option<Res<Game>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut q: Query<(&mut Transform, &mut Visibility)>,
) {
    let Some(mats) = mats else { return };
    let ring = st
        .ring
        .get_or_insert_with(|| meshes.add(Annulus::new(0.32, 0.46).mesh().resolution(32)))
        .clone();
    let amber = st
        .amber
        .get_or_insert_with(|| flat(&mut accents, Vec3::new(1.0, 0.6, 0.15), 2.0))
        .clone();
    let make = |commands: &mut Commands, mat: Handle<AccentMaterial>| {
        commands
            .spawn((
                Mesh3d(ring.clone()),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id()
    };
    let reticle = *st
        .reticle
        .get_or_insert_with(|| make(&mut commands, mats.text.clone()));
    let target = *st
        .target
        .get_or_insert_with(|| make(&mut commands, amber.clone()));
    let alive = game.as_deref().is_some_and(|g| g.alive());
    let on = alive && view.active;
    // Facing the eyes (the ring's front is +Z).
    let at = |dir: Vec3| {
        let p = view.eye + dir * RETICLE_AT;
        Transform::from_translation(p).looking_at(p + dir, view.level * Vec3::Y)
    };
    let fwd = view.rot * Vec3::NEG_Z;
    // (A gun one aims, or the VTOL's pods: not the bus, nor the harvester.)
    let vtol = game
        .as_deref()
        .and_then(|g| g.seat())
        .is_some_and(|(v, s)| {
            s == 0
                && game
                    .as_deref()
                    .and_then(|g| g.vehicle(v))
                    .is_some_and(|v| v.kind.aircraft())
        });
    let gun = on && (view.want.is_some() || vtol);
    if let Ok((mut t, mut v)) = q.get_mut(reticle) {
        *v = if gun {
            *t = at(fwd);
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let want = view.want.filter(|w| on && w.angle_between(fwd) > 0.015);
    if let Ok((mut t, mut v)) = q.get_mut(target) {
        *v = match want {
            Some(w) => {
                *t = at(w).with_scale(Vec3::splat(0.6));
                Visibility::Visible
            }
            None => Visibility::Hidden,
        };
    }
    // The line under the view.
    let text = match game.as_deref() {
        Some(g) if on => {
            let mut s = if view.turret.is_some() {
                let h = g
                    .snap
                    .turrets
                    .iter()
                    .find(|t| t.occupant == g.me)
                    .map_or(0.0, |t| t.health);
                format!("WALL TURRET {h:.0}%")
            } else {
                view.vehicle
                    .and_then(|v| g.vehicle(v))
                    .map_or(String::new(), |v| {
                        format!("{} {:.0}%", v.kind.name().to_uppercase(), v.health)
                    })
            };
            if let Some(c) = view.cargo {
                let n = (c * 10.0).round().clamp(0.0, 10.0) as usize;
                s += &format!(
                    "   CARGO [{}{}] {:.0}%",
                    "#".repeat(n),
                    "-".repeat(10 - n),
                    c * 100.0
                );
            }
            s
        }
        _ => String::new(),
    };
    if st.line.as_ref().is_none_or(|l| l.0 != text) {
        if let Some((_, e)) = st.line.take() {
            commands.entity(e).despawn();
        }
        if !text.is_empty() {
            let h = 0.028;
            let mut m = crate::vfont::mesh(&text, h, 600);
            let w = m.pos.iter().map(|p| p[0]).fold(0.0f32, f32::max);
            for p in &mut m.pos {
                p[0] -= w / 2.0;
            }
            let e = commands
                .spawn((
                    Mesh3d(meshes.add(m.to_mesh())),
                    MeshMaterial3d(mats.text.clone()),
                    Transform::default(),
                    Visibility::Visible,
                ))
                .id();
            st.line = Some((text, e));
        }
    }
    if let Some((_, e)) = &st.line
        && let Ok((mut t, _)) = q.get_mut(*e)
    {
        *t = Transform::from_translation(view.eye + view.level * Vec3::new(0.0, -0.42, -1.1))
            .with_rotation(view.level * Quat::from_rotation_x(0.35));
    }
}

/// The minimap on the back of the left hand (src/minimap.rs): a disc of the land round the
/// player turned the way the hand points (like a compass), the blips, an arrow for the player.
#[derive(Default)]
struct HandMap {
    root: Option<Entity>,
    content: Option<Entity>,
    /// The land's disc: the point it's round, and its entity.
    land: Option<(Vec3, Entity)>,
    me: Option<Entity>,
    pool: Vec<Entity>,
    mats: Vec<(Vec3, Handle<AccentMaterial>)>,
}

/// The disc's radius on the hand (m), and the land's cells across it.
const HAND_MAP_R: f32 = 0.055;
const HAND_MAP_N: usize = 22;

/// The land round `c` within `range`, as a disc of coloured cells (x east, y north), radius 1.
fn local_land(m: &ps_sim::map::ContinentMap, c: Vec3, range: f32) -> crate::assets::MeshData {
    let mut d = crate::assets::MeshData::default();
    let n = HAND_MAP_N;
    let step = 2.0 / n as f32;
    for j in 0..n {
        for i in 0..n {
            let (u, v) = (
                -1.0 + (i as f32 + 0.5) * step,
                1.0 - (j as f32 + 0.5) * step,
            );
            if u * u + v * v > 1.05 {
                continue;
            }
            let (x, z) = (c.x + u * range, c.z - v * range);
            let land = m.terrain.height(x, z);
            let col = if m.terrain.water(x, z) > land + 0.3 {
                [0.03, 0.08, 0.22, 1.0]
            } else {
                let k = (land / 160.0).clamp(0.0, 1.0);
                let c = Vec3::new(0.06, 0.08, 0.1).lerp(Vec3::new(0.3, 0.34, 0.4), k);
                [c.x, c.y, c.z, 1.0]
            };
            let (x0, y0) = (u - step / 2.0, v + step / 2.0);
            let (x1, y1) = (u + step / 2.0, v - step / 2.0);
            let base = d.pos.len() as u32;
            for (px, py) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                d.pos.push([px, py, 0.0]);
                d.nrm.push([0.0, 0.0, 1.0]);
                d.uv.push([0.0, 0.0]);
                d.col.push(col);
            }
            d.idx
                .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    d
}

#[allow(clippy::too_many_arguments)]
fn hand_map(
    mut commands: Commands,
    mut st: Local<HandMap>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    vr: Res<crate::vr::VrState>,
    mats: Option<Res<Mats>>,
    game: Option<Res<Game>>,
    mover: Option<Res<crate::game::Mover>>,
    view: Res<crate::seatview::SeatView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut q: Query<(&mut Transform, &mut Visibility)>,
) {
    let Some(mats) = mats else { return };
    let here = game
        .as_deref()
        .zip(mover.as_deref())
        .filter(|(g, _)| g.alive() && g.ready())
        .and_then(|(g, m)| crate::minimap::here(g, m, &view).map(|h| (g, h)));
    let grip = input.hands[0].grip;
    let root = *st.root.get_or_insert_with(|| {
        commands
            .spawn((Transform::default(), Visibility::Hidden, ChildOf(rig.0)))
            .id()
    });
    let (Some((game, (at, yaw))), Some(g)) = (here, grip) else {
        if let Ok((_, mut v)) = q.get_mut(root) {
            *v = Visibility::Hidden;
        }
        return;
    };
    if st.content.is_none() {
        // The plate behind, its rim, and the turning content with the player's arrow.
        let disc = meshes.add(Circle::new(HAND_MAP_R * 1.08).mesh().resolution(40));
        commands.spawn((
            Mesh3d(disc),
            MeshMaterial3d(mats.back.clone()),
            Transform::from_xyz(0.0, 0.0, -0.001),
            ChildOf(root),
        ));
        let rim = meshes.add(
            Annulus::new(HAND_MAP_R * 1.04, HAND_MAP_R * 1.1)
                .mesh()
                .resolution(40),
        );
        commands.spawn((
            Mesh3d(rim),
            MeshMaterial3d(mats.edge.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0005),
            ChildOf(root),
        ));
        let content = commands
            .spawn((Transform::default(), Visibility::default(), ChildOf(root)))
            .id();
        let me = commands
            .spawn((
                Mesh3d(meshes.add(arrow_mesh().to_mesh())),
                MeshMaterial3d(mats.text.clone()),
                Transform::from_xyz(0.0, 0.0, 0.002).with_scale(Vec3::splat(0.45)),
                ChildOf(content),
            ))
            .id();
        st.content = Some(content);
        st.me = Some(me);
    }
    let content = st.content.unwrap();
    // On the back of the left hand (+X of the left grip), its top towards the fingers.
    let (gx, gy, gz) = (
        g.rotation * Vec3::X,
        g.rotation * Vec3::Y,
        g.rotation * Vec3::Z,
    );
    let rot = Quat::from_mat3(&Mat3::from_cols(-gy, -gz, gx));
    if let Ok((mut t, mut v)) = q.get_mut(root) {
        *t = Transform::from_translation(g.translation + gx * 0.045 + gz * 0.02).with_rotation(rot);
        *v = Visibility::Visible;
    }
    // Turned the way the hand points: north up when it points north.
    let fwd = vr.rig.rotation * g.rotation * Vec3::NEG_Z;
    let hand_yaw = ps_sim::yaw_of(fwd);
    if let Ok((mut t, _)) = q.get_mut(content) {
        t.rotation = Quat::from_rotation_z(-hand_yaw);
    }
    let range = crate::minimap::RANGE;
    let k = HAND_MAP_R / range;
    let to_map = |p: Vec2| Vec2::new(p.x - at.x, -(p.y - at.z)) * k;
    // The land, made again every so many metres.
    let rebuild = st
        .land
        .as_ref()
        .is_none_or(|(c, _)| Vec2::new(c.x - at.x, c.z - at.z).length() > 15.0);
    if rebuild && let Some(map) = &game.map {
        if let Some((_, e)) = st.land.take() {
            commands.entity(e).despawn();
        }
        let mut tinted = NeonUniform::fixed(Vec3::ONE, 1.0);
        tinted.params.x = 3.0;
        let mat = accents.add(AccentMaterial {
            neon: tinted,
            overlay: true,
        });
        let e = commands
            .spawn((
                Mesh3d(meshes.add(local_land(map, at, range).to_mesh())),
                MeshMaterial3d(mat),
                Transform::from_scale(Vec3::new(HAND_MAP_R, HAND_MAP_R, 1.0)),
                ChildOf(content),
            ))
            .id();
        st.land = Some((at, e));
    }
    if let Some((c, e)) = &st.land
        && let Ok((mut t, _)) = q.get_mut(*e)
    {
        let off = to_map(Vec2::new(c.x, c.z));
        t.translation = Vec3::new(off.x, off.y, 0.0);
    }
    if let Some(me) = st.me
        && let Ok((mut t, _)) = q.get_mut(me)
    {
        t.rotation = Quat::from_rotation_z(yaw);
    }
    let list = crate::minimap::blips(game, at, range);
    while st.pool.len() < list.len() {
        let e = commands
            .spawn((
                Mesh3d(mats.quad.clone()),
                MeshMaterial3d(mats.text.clone()),
                Transform::default(),
                Visibility::Hidden,
                ChildOf(content),
            ))
            .id();
        st.pool.push(e);
    }
    for i in 0..st.pool.len() {
        let e = st.pool[i];
        let Some(b) = list.get(i) else {
            if let Ok((_, mut v)) = q.get_mut(e) {
                *v = Visibility::Hidden;
            }
            continue;
        };
        let p = to_map(b.at);
        let inside = p.length() < HAND_MAP_R;
        let mat = match st.mats.iter().find(|(c, _)| (*c - b.color).length() < 0.01) {
            Some((_, m)) => m.clone(),
            None => {
                let m = flat(&mut accents, b.color, 2.0);
                st.mats.push((b.color, m.clone()));
                m
            }
        };
        commands.entity(e).insert(MeshMaterial3d(mat));
        if let Ok((mut t, mut v)) = q.get_mut(e) {
            let s = 0.0035 * b.kind.size();
            *t = Transform::from_xyz(p.x, p.y, 0.001).with_scale(Vec3::new(s, s, 1.0));
            *v = if inside {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
