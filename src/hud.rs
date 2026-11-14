//! The flat-screen HUD: health and armor bars, the weapon and its ammo (or the vehicle's or
//! turret's health and gun, a harvester's load), the crosshair with its hit marker (and, seen
//! from a gun's seat, a ring where the gun is turning to), what the use key does, the base the
//! player is at (owner, hack countdown), the faction's bases under attack, the kill feed (and
//! revives), the
//! chat, short messages (refusals, captures, no firing in a warp gate, a new Battle Rank), and
//! the XP as it's earned ("+100 KILL") under the crosshair. The minimap is src/minimap.rs, the
//! scoreboard src/scores.rs.

use std::collections::VecDeque;

use bevy::prelude::*;
use ps_sim::state::Change;
use ps_sim::{BaseIdx, Cause, SimEvent, Team};

use crate::game::{Game, GameEvent, LocalCmd, Mover, WorldChange};
use crate::interact::UseHint;
use crate::materials::team_color;
use crate::neonui;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
enum Line {
    Health,
    Armor,
    VehicleHealth,
    Cargo,
    Silo,
    Weapon,
    Hint,
    Base,
    Alerts,
    Feed,
    Message,
    /// XP lately earned.
    Xp,
    Chat,
    Top,
    /// Killed: by whom, with what, from how far, how they stood.
    Killcam,
}

#[derive(Component)]
enum Bar {
    Health,
    Armor,
    /// Seated: the vehicle's or turret's health, and a harvester's load.
    Vehicle,
    Cargo,
    /// Inside a base's walls: its silo's resources.
    Silo,
}

/// A lock-on warning's colour.
const WARN_COLOR: Color = Color::srgb(1.0, 0.2, 0.15);

/// XP's colour.
const XP_COLOR: Color = Color::srgb(0.55, 1.0, 0.95);
/// How long XP shows (s).
const XP_TIME: f32 = 2.5;

/// The silo bar's colour (amber is the cargo's).
const SILO_COLOR: Color = Color::srgb(1.0, 0.5, 0.12);

/// A bar's row (hidden when it doesn't apply).
#[derive(Component)]
struct BarRow(u8);

/// Where the gun is turning to (seen from its seat).
#[derive(Component)]
struct AimMarker;

#[derive(Component)]
struct Crosshair;

/// A box round the vehicle one's launcher (or missiles) locks on to, and its words.
#[derive(Component)]
struct LockBox;

#[derive(Component)]
struct LockText;

/// Being locked on: the warning under the crosshair.
#[derive(Component)]
struct Warning;

/// The hit marker: an X of four bars (`HitBar`), scaled and coloured by what was hit.
#[derive(Component)]
struct HitMark;

#[derive(Component)]
struct HitBar;

/// The screen's edges reddened by damage taken (and pulsing at low health).
#[derive(Component)]
struct Vignette;

/// A red arc on a ring round the crosshair, towards where a hit came from.
#[derive(Component)]
struct HurtArc(usize);

/// How many recent hits show where they came from.
const HURT_ARCS: usize = 4;
/// How long they show (s).
const HURT_ARC_TIME: f32 = 1.2;

#[derive(Resource, Default)]
struct HudState {
    feed: VecDeque<(String, f32)>,
    message: Option<(String, f32)>,
    /// XP earned lately: what for, how much, when (the same reason close together adds up).
    xp: VecDeque<(&'static str, u32, f32)>,
    /// The Battle Rank last seen (a new one is announced).
    br: Option<u8>,
    /// The last hit given: when, and whether it killed, was a headshot, hit a vehicle.
    hit: f32,
    kill: bool,
    headshot: bool,
    vehicle: bool,
    /// How hurt (0..1) as of `pain.1`, wearing off.
    pain: (f32, f32),
    /// Recent hits taken: where from and when.
    hurts: VecDeque<(Vec3, f32)>,
    /// Blinded by a flash grenade: until when, and for how long in all (s).
    flash: (f32, f32),
}

/// The white of a flash grenade, over everything.
#[derive(Component)]
struct FlashOverlay;

impl HudState {
    fn pain(&self, now: f32) -> f32 {
        self.pain.0 * (-(now - self.pain.1).max(0.0) / 0.5).exp()
    }
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_systems(
                Update,
                (
                    spawn_hud,
                    (events, update, feedback, locks, flash)
                        .chain()
                        .run_if(resource_exists::<Game>),
                ),
            )
            .add_systems(Update, hide.run_if(not(resource_exists::<Game>)));
    }
}

fn color_of(t: Option<Team>) -> Color {
    let c = team_color(t);
    Color::linear_rgb(c.x, c.y, c.z)
}

fn spawn_hud(mut commands: Commands, q: Query<(), With<HudRoot>>, game: Option<Res<Game>>) {
    if !q.is_empty() || game.is_none() {
        return;
    }
    let abs = |left: Option<f32>, right: Option<f32>, top: Option<f32>, bottom: Option<f32>| Node {
        position_type: PositionType::Absolute,
        left: left.map_or(Val::Auto, Val::Px),
        right: right.map_or(Val::Auto, Val::Px),
        top: top.map_or(Val::Auto, Val::Px),
        bottom: bottom.map_or(Val::Auto, Val::Px),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        ..default()
    };
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(5),
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            // Behind it all: the damage vignette, and the arcs towards hits.
            r.spawn((
                Vignette,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundGradient::default(),
                Pickable::IGNORE,
            ));
            for i in 0..HURT_ARCS {
                r.spawn((
                    HurtArc(i),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        width: Val::Px(70.0),
                        height: Val::Px(7.0),
                        margin: UiRect::new(Val::Px(-35.0), Val::ZERO, Val::Px(-3.5), Val::ZERO),
                        border_radius: BorderRadius::all(Val::Px(3.5)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    UiTransform::IDENTITY,
                    Visibility::Hidden,
                    Pickable::IGNORE,
                ));
            }
            // Bottom left: health, armor, weapon.
            r.spawn(abs(Some(24.0), None, None, Some(24.0)))
                .with_children(|c| {
                    for (k, (bar, line, col)) in [
                        (Bar::Health, Line::Health, neonui::GREEN),
                        (Bar::Armor, Line::Armor, neonui::CYAN),
                        (Bar::Vehicle, Line::VehicleHealth, neonui::MAGENTA),
                        (Bar::Cargo, Line::Cargo, neonui::AMBER),
                        (Bar::Silo, Line::Silo, SILO_COLOR),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        c.spawn((neonui::row(10.0), BarRow(k as u8)))
                            .with_children(|row| {
                                row.spawn((
                                    Node {
                                        width: Val::Px(220.0),
                                        height: Val::Px(12.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BorderColor::all(col.with_alpha(0.6)),
                                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        bar,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        BackgroundColor(col),
                                    ));
                                });
                                row.spawn((line, neonui::text("", 16.0, col)));
                            });
                    }
                    c.spawn((Line::Weapon, neonui::text("", 20.0, neonui::TEXT)));
                });
            r.spawn(abs(Some(24.0), None, None, Some(120.0)))
                .with_children(|c| {
                    c.spawn((Line::Chat, neonui::text("", 14.0, neonui::TEXT)));
                });
            // Top: the continent and the base here; the alerts below it.
            r.spawn(Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Px(14.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|c| {
                c.spawn((Line::Top, neonui::text("", 16.0, neonui::DIM)));
                c.spawn((Line::Base, neonui::text_w("", 22.0, neonui::TEXT, 700)));
                c.spawn((Line::Alerts, neonui::text("", 16.0, neonui::AMBER)));
            });
            // Top right: the kill feed.
            r.spawn(abs(None, Some(24.0), Some(20.0), None))
                .with_children(|c| {
                    c.spawn((Line::Feed, neonui::text("", 15.0, neonui::TEXT)));
                });
            // The middle: crosshair, hit marker, hint, messages.
            r.spawn(Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    Crosshair,
                    Node {
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                ));
                c.spawn((
                    LockBox,
                    Node {
                        position_type: PositionType::Absolute,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(neonui::AMBER),
                    Visibility::Hidden,
                ))
                .with_children(|b| {
                    b.spawn((
                        LockText,
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Percent(100.0),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                        neonui::text_w("", 13.0, neonui::AMBER, 700),
                    ));
                });
                c.spawn((
                    AimMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(11.0)),
                        ..default()
                    },
                    BorderColor::all(neonui::AMBER.with_alpha(0.8)),
                    Visibility::Hidden,
                ));
                c.spawn((
                    HitMark,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        ..default()
                    },
                    UiTransform::IDENTITY,
                ))
                .with_children(|m| {
                    // (Each bar along its diagonal, out from a gap in the middle.)
                    for (dx, dy) in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
                        let (len, at) = (11.0, 8.0);
                        m.spawn((
                            HitBar,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(18.0 + dx * at - len / 2.0),
                                top: Val::Px(18.0 + dy * at - 1.5),
                                width: Val::Px(len),
                                height: Val::Px(3.0),
                                ..default()
                            },
                            UiTransform::from_rotation(Rot2::degrees(45.0 * dx * dy)),
                            BackgroundColor(Color::NONE),
                        ));
                    }
                });
                c.spawn((Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(62.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                },))
                    .with_children(|m| {
                        m.spawn((Line::Hint, neonui::text("", 18.0, neonui::TEXT)));
                        m.spawn((Line::Message, neonui::text_w("", 22.0, neonui::AMBER, 700)));
                        m.spawn((Line::Killcam, neonui::text_w("", 24.0, WARN_COLOR, 800)));
                        m.spawn((Line::Xp, neonui::text_w("", 16.0, XP_COLOR, 700)));
                        m.spawn((Warning, neonui::text_w("", 24.0, WARN_COLOR, 800)));
                    });
            });
        });
}

/// Lock-ons: a box round what one's launcher or missiles lock on to (amber while locking,
/// red once locked), and the warning when an enemy locks on to one's vehicle or a missile is
/// coming.
#[allow(clippy::type_complexity)]
fn locks(
    game: Res<Game>,
    time: Res<Time>,
    cams: Query<(&Camera, &GlobalTransform), With<crate::desktop::MainCamera>>,
    mut boxes: Query<(&mut Node, &mut BorderColor, &mut Visibility), With<LockBox>>,
    mut texts: Query<(&mut Text, &mut TextColor), (With<LockText>, Without<Warning>)>,
    mut warn: Query<&mut Text, (With<Warning>, Without<LockText>)>,
) {
    let me = game.mine().filter(|m| m.dead.is_none());
    let shown = me.and_then(|m| m.lock).and_then(|(vid, k)| {
        let v = game.vehicle(vid)?;
        let (cam, t) = cams.single().ok()?;
        let c = v.pos + v.rot * Vec3::Y * v.kind.spec().half.y;
        let px = cam.world_to_viewport(t, c).ok()?;
        let d = (c - t.translation()).length().max(1.0);
        let size = (900.0 * v.kind.spec().half.length() / d).clamp(26.0, 200.0);
        Some((px, size, k))
    });
    for (mut n, mut border, mut vis) in &mut boxes {
        let Some((px, size, k)) = shown else {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        n.left = Val::Px(px.x - size / 2.0);
        n.top = Val::Px(px.y - size / 2.0);
        n.width = Val::Px(size);
        n.height = Val::Px(size);
        let c = if k >= 1.0 { WARN_COLOR } else { neonui::AMBER };
        *border = BorderColor::all(c);
        for (mut t, mut tc) in &mut texts {
            let s = if k >= 1.0 {
                "LOCKED".to_string()
            } else {
                format!("LOCKING {:.0}%", k * 100.0)
            };
            if t.0 != s {
                t.0 = s;
            }
            tc.0 = c;
        }
    }
    // (Blinking.)
    let on = (time.elapsed_secs() * 4.0) as i64 % 2 == 0;
    let s = match me.map_or(0, |m| m.warning) {
        2 => {
            let flares = me.is_some_and(|m| m.flare_wait <= 0.0)
                && game
                    .seat()
                    .and_then(|(v, s)| game.vehicle(v).map(|v| (v, s)))
                    .is_some_and(|(v, s)| {
                        s == 0 && v.loadout.defense == ps_sim::vehicle::Defense::Flares
                    });
            if flares {
                "MISSILE INCOMING  -  [B] FLARES"
            } else {
                "MISSILE INCOMING"
            }
        }
        1 if on => "LOCK WARNING",
        _ => "",
    };
    for mut t in &mut warn {
        if t.0 != s {
            t.0 = s.to_string();
        }
    }
}

fn hide(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn cause_name(c: &Cause) -> String {
    c.name()
}

fn events(
    game: Res<Game>,
    time: Res<Time>,
    mut hud: ResMut<HudState>,
    mut events: MessageReader<GameEvent>,
    mut changes: MessageReader<WorldChange>,
) {
    let now = time.elapsed_secs();
    // A new Battle Rank.
    if let Some(xp) = game.xp {
        let br = ps_sim::progress::battle_rank(xp);
        if hud.br.is_some_and(|b| br > b) {
            hud.message = Some((format!("BATTLE RANK {br}"), now));
        }
        hud.br = Some(br);
    }
    for GameEvent(e) in events.read() {
        match e {
            SimEvent::Xp { reason, xp } => {
                let name = reason.name();
                match hud.xp.back_mut() {
                    Some(last) if last.0 == name && now - last.2 < 0.6 => {
                        last.1 += xp;
                        last.2 = now;
                    }
                    _ => hud.xp.push_back((name, *xp, now)),
                }
                while hud.xp.len() > 5 {
                    hud.xp.pop_front();
                }
            }
            SimEvent::Killed {
                victim,
                by,
                cause,
                headshot,
            } => {
                let line = if by == victim || *by == ps_sim::HAZARD {
                    format!("{} [{}]", game.name_of(*victim), cause_name(cause))
                } else {
                    format!(
                        "{} [{}{}] {}",
                        game.name_of(*by),
                        cause_name(cause),
                        if *headshot { ", head" } else { "" },
                        game.name_of(*victim)
                    )
                };
                hud.feed.push_back((line, now));
                while hud.feed.len() > 6 {
                    hud.feed.pop_front();
                }
            }
            SimEvent::HitConfirm {
                by,
                kill,
                headshot,
                vehicle,
            } if Some(*by) == game.me => {
                // (A kill's marker isn't cut short by the next hit.)
                if !(hud.kill && now - hud.hit < 0.3) || *kill {
                    hud.hit = now;
                    hud.kill = *kill;
                    hud.headshot = *headshot;
                    hud.vehicle = *vehicle;
                }
            }
            SimEvent::Hurt {
                victim,
                amount,
                from,
                ..
            } if Some(*victim) == game.me => {
                hud.pain = ((hud.pain(now) + amount / 60.0).min(0.7), now);
                // (Not falls, nor one's own blasts at one's feet: no direction.)
                let at = game.mine().map_or(*from, |m| m.feet);
                if Vec2::new(from.x - at.x, from.z - at.z).length() > 1.5 {
                    hud.hurts.push_back((*from, now));
                    while hud.hurts.len() > HURT_ARCS {
                        hud.hurts.pop_front();
                    }
                }
            }
            SimEvent::Denied(d) => hud.message = Some((d.text(), now)),
            SimEvent::LoadoutTaken { .. } => hud.message = Some(("Loadout equipped".into(), now)),
            SimEvent::Healed { .. } => hud.message = Some(("Healed".into(), now)),
            SimEvent::Revived { player, by } => {
                let line = format!("{} revived {}", game.name_of(*by), game.name_of(*player));
                if Some(*player) == game.me {
                    hud.message = Some((format!("Revived by {}", game.name_of(*by)), now));
                } else if Some(*by) == game.me {
                    hud.message = Some((format!("You revived {}", game.name_of(*player)), now));
                }
                hud.feed.push_back((line, now));
                while hud.feed.len() > 6 {
                    hud.feed.pop_front();
                }
            }
            SimEvent::Hacked { by, team, what } => {
                let thing = match what {
                    ps_sim::HackTarget::Terminal { .. } => "a terminal",
                    ps_sim::HackTarget::Turret(_) => "a turret",
                };
                if Some(*by) == game.me {
                    hud.message = Some((format!("You hacked {thing}"), now));
                } else if *team != game.team {
                    hud.message = Some((format!("An enemy infiltrator hacked {thing}"), now));
                }
            }
            SimEvent::Built { by, kind, .. } if Some(*by) == game.me => {
                hud.message = Some((format!("{} built", kind.name()), now));
            }
            SimEvent::Flashed { player, secs } if Some(*player) == game.me => {
                hud.flash = (now + secs, *secs);
            }
            SimEvent::SiloLow { at, empty } => {
                let near = crate::build::nearest_base(&game, *at)
                    .map_or(String::new(), |n| format!(" by {n}"));
                hud.message = Some((
                    if *empty {
                        format!("A mobile silo{near} is EMPTY: its structures are decaying")
                    } else {
                        format!("A mobile silo{near} is running low")
                    },
                    now,
                ));
            }
            SimEvent::StructureDown { kind, team, .. } => {
                let whose = if *team == game.team { "Our" } else { "Enemy" };
                hud.message = Some((format!("{whose} {} destroyed", kind.name()), now));
            }
            SimEvent::Deployed { by, kind, .. } if Some(*by) == game.me => {
                hud.message = Some((format!("{} down", kind.name()), now));
            }
            _ => {}
        }
    }
    for WorldChange(c) in changes.read() {
        let name = |c: u8, b: BaseIdx| {
            game.def
                .continents
                .get(c as usize)
                .and_then(|d| d.bases.get(b as usize))
                .map_or(String::new(), |b| b.name.clone())
        };
        let points = |c: u8, b: BaseIdx| {
            game.def
                .continents
                .get(c as usize)
                .and_then(|d| d.bases.get(b as usize))
                .is_some_and(|b| b.capture == ps_sim::worldgen::Capture::Points)
        };
        let msg = match *c {
            Change::BaseCaptured {
                continent,
                base,
                team,
                ..
            } if Some(continent) == game.continent => {
                Some(format!("{} took {}", team.upper(), name(continent, base)))
            }
            Change::ContinentCaptured {
                continent, team, ..
            } => Some(format!(
                "{} captured {}",
                team.upper(),
                game.def.continents[continent as usize].name
            )),
            Change::HackStarted {
                continent,
                base,
                team,
            } if Some(continent) == game.continent => Some(format!(
                "{} is {} {}",
                team.upper(),
                if points(continent, base) {
                    "capturing"
                } else {
                    "hacking"
                },
                name(continent, base)
            )),
            Change::HackStopped {
                continent, base, ..
            } if Some(continent) == game.continent => Some(format!(
                "{} of {} stopped",
                if points(continent, base) {
                    "Capture"
                } else {
                    "Hack"
                },
                name(continent, base)
            )),
            _ => None,
        };
        if let Some(m) = msg {
            hud.message = Some((m, now));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    game: Res<Game>,
    hud: Res<HudState>,
    time: Res<Time>,
    hint: Res<UseHint>,
    mover: Res<Mover>,
    local: Res<LocalCmd>,
    mut lines: Query<(&Line, &mut Text, &mut TextColor)>,
    mut bars: Query<(&Bar, &mut Node), Without<AimMarker>>,
    mut rows: Query<(&BarRow, &mut Visibility), (Without<Crosshair>, Without<AimMarker>)>,
    mut cross: Query<&mut Visibility, (With<Crosshair>, Without<AimMarker>)>,
    mut aim: Query<(&mut Node, &mut Visibility), With<AimMarker>>,
    chat_line: Res<crate::chat::ChatLine>,
    real_time: Res<Time<Real>>,
    (view, look, cams): (
        Res<crate::seatview::SeatView>,
        Res<crate::desktop::Look>,
        Query<(&Camera, &Transform), With<crate::desktop::MainCamera>>,
    ),
) {
    let now = time.elapsed_secs();
    let me = game.mine();
    let alive = game.alive();
    let (health, armor, armor_max) = me.map_or((0.0, 0.0, 1.0), |m| {
        (m.health, m.armor, m.loadout.armor.max())
    });
    // Seated: the vehicle's (or turret's) health, and a harvester's load.
    let seat_health = if !alive || !view.active {
        None
    } else if view.turret.is_some() {
        game.snap
            .turrets
            .iter()
            .find(|t| t.occupant == game.me)
            .map(|t| t.health)
    } else {
        view.vehicle.and_then(|v| game.vehicle(v)).map(|v| v.health)
    };
    let cargo = view.cargo.filter(|_| alive);
    // Inside a base's walls (one with a silo): how full the silo is (0..1).
    let silo = game.map.as_ref().filter(|_| alive).and_then(|m| {
        let at = if view.active {
            view.eye
        } else if mover.valid {
            mover.body.feet
        } else {
            me?.feet
        };
        let b = m
            .bases
            .iter()
            .position(|b| b.silo.is_some() && b.inside(at))?;
        let s = game.snap.bases.get(b)?;
        Some(s.resources as f32 / ps_sim::state::SILO_CAP)
    });
    for (b, mut n) in &mut bars {
        let k = match b {
            Bar::Health => health / 100.0,
            Bar::Armor => armor / armor_max.max(1.0),
            Bar::Vehicle => seat_health.unwrap_or(0.0) / 100.0,
            Bar::Cargo => cargo.unwrap_or(0.0),
            Bar::Silo => silo.unwrap_or(0.0),
        };
        n.width = Val::Percent((k * 100.0).clamp(0.0, 100.0));
    }
    for (r, mut v) in &mut rows {
        let on = match r.0 {
            2 => seat_health.is_some(),
            3 => cargo.is_some(),
            4 => silo.is_some(),
            _ => true,
        };
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    // From a gun's seat (inside), a ring where the gunner wants it to point: the crosshair is
    // where it points now.
    let marker = view
        .want
        .filter(|_| alive && !look.third_person)
        .filter(|w| w.angle_between(view.rot * Vec3::NEG_Z) > 0.015)
        .and_then(|w| {
            let (cam, t) = cams.single().ok()?;
            cam.world_to_viewport(&GlobalTransform::from(*t), t.translation + w * 300.0)
                .ok()
        });
    for (mut n, mut v) in &mut aim {
        match marker {
            Some(p) => {
                n.left = Val::Px(p.x - 11.0);
                n.top = Val::Px(p.y - 11.0);
                if *v != Visibility::Visible {
                    *v = Visibility::Visible;
                }
            }
            None => {
                if *v != Visibility::Hidden {
                    *v = Visibility::Hidden;
                }
            }
        }
    }
    let in_bubble = alive
        && game.map.as_ref().is_some_and(|m| {
            let at = if view.active {
                view.eye
            } else {
                mover.body.feet + Vec3::Y
            };
            m.gates.iter().any(|g| ps_sim::base::in_gate_bubble(*g, at))
        });
    for mut v in &mut cross {
        *v = if alive {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let c = game.continent.unwrap_or(0);
    let here = game.map.as_ref();
    for (line, mut text, mut color) in &mut lines {
        let s = match line {
            Line::Health => format!("{:.0}", health),
            Line::Armor => format!("{:.0}", armor),
            Line::VehicleHealth => format!("{:.0}%", seat_health.unwrap_or(0.0)),
            Line::Cargo => {
                let harvesting = view
                    .vehicle
                    .and_then(|v| game.vehicle(v))
                    .is_some_and(|v| v.flags & ps_sim::snapshot::vflags::HARVESTING != 0);
                format!(
                    "{:.0}/{} {}",
                    cargo.unwrap_or(0.0) * ps_sim::sim::CARGO,
                    ps_sim::sim::CARGO as u32,
                    if harvesting {
                        "HARVESTING"
                    } else if cargo.unwrap_or(0.0) >= 1.0 {
                        "FULL: TO A SILO"
                    } else {
                        "CARGO"
                    }
                )
            }
            Line::Silo => {
                let k = silo.unwrap_or(0.0);
                color.0 = if k < 0.2 {
                    Color::srgb(1.0, 0.25, 0.2)
                } else {
                    SILO_COLOR
                };
                format!(
                    "{:.0}/{} SILO{}",
                    k * ps_sim::state::SILO_CAP,
                    ps_sim::state::SILO_CAP as u32,
                    if k <= 0.0 {
                        ": EMPTY"
                    } else if k < 0.2 {
                        ": LOW"
                    } else {
                        ""
                    }
                )
            }
            Line::Weapon => match (me, game.seat()) {
                (Some(m), _) if m.turret.is_some() => {
                    "Wall Turret   Gunner   [V] camera".to_string()
                }
                (Some(m), Some((v, seat))) => game.vehicle(v).map_or(String::new(), |v| {
                    let spec = v.kind.spec();
                    let role = if seat == 0 {
                        "Driver"
                    } else if spec.seats[seat as usize].gun.is_some() {
                        "Gunner"
                    } else {
                        "Passenger"
                    };
                    // (The harvester's beam needs no rounds.)
                    let ammo = match m.seat_ammo {
                        Some(_) if v.kind == ps_sim::vehicle::VehicleKind::Harvester => {
                            String::new()
                        }
                        Some((0, _)) => "   NO AMMO: REARM AT A PAD".to_string(),
                        Some((n, max)) => format!("   {n} / {max}"),
                        None => String::new(),
                    };
                    // The second gun (missiles), and an aircraft's flares.
                    let missiles = m.alt_ammo.map_or(String::new(), |(n, max)| {
                        format!("   Missiles {n}/{max} [RMB]")
                    });
                    // The driver's defensive slot (plating works by itself).
                    use ps_sim::vehicle::Defense;
                    let d = v.loadout.defense;
                    let flares = if seat == 0 && !matches!(d, Defense::None | Defense::Plating) {
                        if m.flare_wait > 0.0 {
                            format!("   {} {:.0} s", d.name(), m.flare_wait.ceil())
                        } else {
                            format!("   {} [B]", d.name())
                        }
                    } else if d == Defense::Plating {
                        "   Plated".to_string()
                    } else {
                        String::new()
                    };
                    format!(
                        "{}{ammo}{missiles}{flares}   {role}   [V] camera",
                        spec.name
                    )
                }),
                (Some(m), None) => {
                    let slot = (local.cmd.slot as usize).min(ps_sim::weapons::SLOTS - 1);
                    let w = m.loadout.slot(slot as u8);
                    let g = m.guns[slot];
                    let reload = if g.reload > 0.0 { "  RELOADING" } else { "" };
                    let grenades = if m.loadout.max() {
                        String::new()
                    } else {
                        format!("   {} grenades {}", m.loadout.grenade.name(), m.grenades)
                    };
                    // The suit's and the kit's: the cloak, a hack, deployables left.
                    let class = if let Some(k) = m.hack {
                        format!("   HACKING {:.0}%", k * 100.0)
                    } else if m.loadout.armor == ps_sim::weapons::ArmorKind::Stealth {
                        let on = game
                            .me_snap()
                            .is_some_and(|p| p.has(ps_sim::snapshot::flags::CLOAKED));
                        format!(
                            "   Cloak {:.0}% [B]{}",
                            m.cloak * 100.0,
                            if on { " ON" } else { "" }
                        )
                    } else if m.loadout.kit() != ps_sim::weapons::Kit::None {
                        format!("   {} x{} [B]", m.loadout.kit().name(), m.kit_left)
                    } else {
                        String::new()
                    };
                    // Light armour's jump jets (hold jump in the air).
                    let class = if m.loadout.armor == ps_sim::weapons::ArmorKind::Light {
                        format!("{class}   Jets {:.0}%", m.fuel * 100.0)
                    } else {
                        class
                    };
                    // A structure carried (from a harvester's terminal).
                    let class = match m.carrying {
                        Some(k) => format!("{class}   CARRYING {} [N] place", k.name()),
                        None => class,
                    };
                    format!(
                        "{}  {} / {}{}{grenades}{class}",
                        w.stats().name,
                        g.mag,
                        g.reserve,
                        reload,
                    )
                }
                _ => String::new(),
            },
            Line::Hint => match (alive, hint.0) {
                _ if in_bubble => "Weapons are disabled inside the warp gate".to_string(),
                // (Leaving a seat needs no prompt.)
                (true, Some(crate::interact::Target::Leave)) => String::new(),
                (true, Some(t)) => format!("[E] {}", t.hint()),
                _ => String::new(),
            },
            Line::Top => match here {
                Some(m) => {
                    let pop = game.population.get(c as usize).copied().unwrap_or_default();
                    let rank = game.xp.map_or(String::new(), |xp| {
                        format!(
                            "   |   BR {} ({:.0}%)",
                            ps_sim::progress::battle_rank(xp),
                            ps_sim::progress::rank_progress(xp) * 100.0
                        )
                    });
                    format!(
                        "{}  -  {}   |   RED {}  BLUE {}  GREEN {}   |   K {} / D {} / A {}{rank}",
                        m.def.name,
                        m.def.theme.name(),
                        pop[0],
                        pop[1],
                        pop[2],
                        me.map_or(0, |m| m.kills),
                        me.map_or(0, |m| m.deaths),
                        me.map_or(0, |m| m.assists),
                    )
                }
                None => "Warping...".into(),
            },
            Line::Base => {
                let at = mover
                    .valid
                    .then_some(mover.body.feet)
                    .or(me.map(|m| m.feet));
                let base = here.zip(at).and_then(|(m, p)| {
                    m.bases
                        .iter()
                        .enumerate()
                        .find(|(_, b)| (b.center - p).length() < b.half * 1.8)
                        .map(|(i, _)| i as BaseIdx)
                });
                match (here, base) {
                    (Some(m), Some(b)) => {
                        let st = game.state.base(c, b);
                        color.0 = color_of(st.owner);
                        let owner = st.owner.map_or("NEUTRAL", |t| t.upper());
                        let power = game.snap.bases.get(b as usize).map_or(String::new(), |s| {
                            if m.bases[b as usize].silo.is_some() && silo.is_none() {
                                format!("   GEN {:.0}%  SILO {}", s.generator, s.resources)
                            } else if m.bases[b as usize].silo.is_some() {
                                // (The silo's bar shows the rest.)
                                format!("   GEN {:.0}%", s.generator)
                            } else {
                                String::new()
                            }
                        });
                        let hack = st.hack.map_or(String::new(), |h| {
                            format!(
                                "   HACKED BY {}  {}:{:02}",
                                h.team.upper(),
                                (h.left / 60.0) as u32,
                                h.left as u32 % 60
                            )
                        });
                        format!(
                            "{} ({}){}{}",
                            m.def.bases[b as usize].name, owner, hack, power
                        )
                    }
                    _ => String::new(),
                }
            }
            Line::Alerts => {
                // The alert under way; the faction's bases being hacked, anywhere.
                let mut out = Vec::new();
                if let Some((a, heard)) = &game.alert {
                    let left = (a.left as f32 - (now - heard)).max(0.0) as u32;
                    let h = holdings(&game, a.continent);
                    out.push(format!(
                        "ALERT  {}: hold the most bases  {}:{:02}   RED {}  BLUE {}  GREEN {}",
                        game.def.continents[a.continent as usize]
                            .name
                            .to_uppercase(),
                        left / 60,
                        left % 60,
                        h[0],
                        h[1],
                        h[2]
                    ));
                }
                for (ci, cs) in game.state.continents.iter().enumerate() {
                    for (bi, b) in cs.bases.iter().enumerate() {
                        if b.owner == Some(game.team)
                            && let Some(h) = b.hack
                        {
                            out.push(format!(
                                "{} / {} under attack ({:.0} s)",
                                game.def.continents[ci].name,
                                game.def.continents[ci].bases[bi].name,
                                h.left
                            ));
                        }
                    }
                }
                out.truncate(3);
                out.join("\n")
            }
            Line::Feed => hud
                .feed
                .iter()
                .filter(|(_, t)| now - t < 8.0)
                .map(|(l, _)| l.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            Line::Killcam => match game.killcam(real_time.elapsed_secs()) {
                Some((d, _)) => format!(
                    "KILLED BY {}\n{}  -  {:.0} m  -  their health {:.0}, armour {:.0}",
                    game.name_of(d.by),
                    d.cause.name(),
                    d.distance,
                    d.health,
                    d.armor
                ),
                None => String::new(),
            },
            Line::Message => match (&hud.message, &game.notice) {
                // (An alert's end shows longer, unless something newer.)
                (_, Some((n, t)))
                    if now - t < 6.0 && hud.message.as_ref().is_none_or(|m| m.1 < *t) =>
                {
                    n.clone()
                }
                (Some((m, t)), _) if now - t < 3.5 => m.clone(),
                _ => String::new(),
            },
            Line::Xp => hud
                .xp
                .iter()
                .filter(|x| now - x.2 < XP_TIME)
                .map(|(name, xp, _)| format!("+{xp} {name}"))
                .collect::<Vec<_>>()
                .join("\n"),
            Line::Chat => {
                let mut lines: Vec<String> = game
                    .chat
                    .iter()
                    .map(|(n, t, s, squad)| {
                        if *squad {
                            format!("[SQUAD] {n}: {s}")
                        } else {
                            format!("[{}] {n}: {s}", t.upper())
                        }
                    })
                    .collect();
                if let Some(t) = &chat_line.0 {
                    // The line being typed, with a blinking caret.
                    let caret = if (now * 2.0) as i64 % 2 == 0 {
                        "_"
                    } else {
                        " "
                    };
                    lines.push(format!("Say: {t}{caret}"));
                }
                lines.join("\n")
            }
        };
        if text.0 != s {
            text.0 = s;
        }
    }
}

/// The hit marker, the damage vignette and the arcs towards hits taken.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn feedback(
    game: Res<Game>,
    hud: Res<HudState>,
    time: Res<Time>,
    mut mark: Query<&mut UiTransform, (With<HitMark>, Without<HurtArc>)>,
    mut bars: Query<&mut BackgroundColor, (With<HitBar>, Without<HurtArc>)>,
    mut vignette: Query<&mut BackgroundGradient, With<Vignette>>,
    mut arcs: Query<
        (
            &HurtArc,
            &mut UiTransform,
            &mut BackgroundColor,
            &mut Visibility,
        ),
        Without<HitMark>,
    >,
    cams: Query<&GlobalTransform, With<crate::desktop::MainCamera>>,
) {
    let now = time.elapsed_secs();
    let alive = game.alive();
    // The hit marker: white; amber and bigger on the head; dim cyan on a vehicle; magenta,
    // popping in and lingering, on a kill.
    let since = now - hud.hit;
    let life = if hud.kill { 0.5 } else { 0.3 };
    let a = (1.0 - since / life).clamp(0.0, 1.0);
    let color = if hud.kill {
        neonui::MAGENTA
    } else if hud.headshot {
        neonui::AMBER
    } else if hud.vehicle {
        neonui::CYAN.with_alpha(0.8)
    } else {
        Color::WHITE
    };
    let mut scale = if hud.headshot { 1.35 } else { 1.0 };
    if hud.kill {
        scale *= 1.0 + 0.5 * (1.0 - since / 0.15).max(0.0);
    }
    for mut t in &mut mark {
        t.scale = Vec2::splat(scale);
    }
    let bar = color.with_alpha(color.alpha() * a);

    for mut b in &mut bars {
        if b.0 != bar {
            b.0 = bar;
        }
    }
    // The vignette: the pain wearing off, and a slow pulse at low health.
    let health = game.mine().map_or(100.0, |m| m.health);
    let low = if alive && health < 30.0 {
        let k = (30.0 - health) / 30.0;
        (0.12 + 0.2 * k) * (0.6 + 0.4 * (now * std::f32::consts::TAU).sin())
    } else {
        0.0
    };
    let pain = if alive { hud.pain(now).max(low) } else { 0.0 };
    for mut g in &mut vignette {
        let want = if pain < 0.005 {
            Vec::new()
        } else {
            vec![Gradient::Radial(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::new(Color::srgba(0.9, 0.02, 0.06, 0.0), Val::Percent(58.0)),
                    ColorStop::new(
                        Color::srgba(0.9, 0.02, 0.06, pain * 0.3),
                        Val::Percent(85.0),
                    ),
                    ColorStop::new(
                        Color::srgba(1.0, 0.05, 0.1, pain * 0.75),
                        Val::Percent(100.0),
                    ),
                ],
            ))]
        };
        if g.0 != want {
            g.0 = want;
        }
    }
    // The arcs: clockwise from the top as the hit came from the camera's left or right.
    let (cam_at, cam_yaw) = cams.single().map_or((Vec3::ZERO, 0.0), |c| {
        (c.translation(), ps_sim::yaw_of(c.rotation() * Vec3::NEG_Z))
    });
    for (arc, mut t, mut c, mut v) in &mut arcs {
        let hit = hud
            .hurts
            .iter()
            .rev()
            .nth(arc.0)
            .filter(|(_, at)| alive && now - at < HURT_ARC_TIME);
        let want = if hit.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        let Some(&(from, at)) = hit else { continue };
        let a = -ps_sim::wrap_angle(ps_sim::yaw_of(from - cam_at) - cam_yaw);
        let r = 150.0;
        t.translation = Val2::px(r * a.sin(), -r * a.cos());
        t.rotation = Rot2::radians(a);
        let fade = 1.0 - (now - at) / HURT_ARC_TIME;
        c.0 = Color::srgba(1.0, 0.1, 0.12, 0.85 * fade.sqrt());
    }
}

/// The outposts and center each faction holds on continent `c` (as an alert counts them).
pub fn holdings(game: &Game, c: u8) -> [u16; 3] {
    let mut out = [0u16; 3];
    let Some(cd) = game.def.continents.get(c as usize) else {
        return out;
    };
    for (b, site) in cd.bases.iter().enumerate() {
        if matches!(
            site.kind,
            ps_sim::worldgen::BaseKind::Outpost | ps_sim::worldgen::BaseKind::Center
        ) && let Some(t) = game.state.base(c, b as BaseIdx).owner
        {
            out[t.index()] += 1;
        }
    }
    out
}

/// The discount the faction's continent locks give it (0..0.3), as the server reckons it.
pub fn lock_discount(game: &Game) -> f32 {
    let locked = game
        .state
        .continents
        .iter()
        .enumerate()
        .filter(|(c, s)| s.owner == Some(game.team) && game.def.continents[*c].sanctuary.is_none())
        .count();
    0.1 * locked.min(3) as f32
}

/// A flash grenade's white, fading as the blindness wears off.
fn flash(
    mut commands: Commands,
    hud: Res<HudState>,
    time: Res<Time>,
    mut q: Query<(Entity, &mut BackgroundColor), With<FlashOverlay>>,
) {
    let now = time.elapsed_secs();
    let left = hud.flash.0 - now;
    let k = if left > 0.0 && hud.flash.1 > 0.0 {
        // (Full white most of the while, then fading.)
        (left / hud.flash.1 * 1.6).min(1.0)
    } else {
        0.0
    };
    match q.single_mut() {
        Ok((e, mut bg)) => {
            if k <= 0.0 {
                commands.entity(e).despawn();
            } else {
                bg.0 = Color::srgba(1.0, 1.0, 0.97, k);
            }
        }
        Err(_) if k > 0.0 => {
            commands.spawn((
                FlashOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 0.97, k)),
                GlobalZIndex(50),
                Pickable::IGNORE,
            ));
        }
        Err(_) => {}
    }
}
