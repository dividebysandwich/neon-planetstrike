//! What the use key does where the player stands: enter the vehicle in front of it (leave the
//! one it sits in), heal at a medical terminal, or open a panel: the equipment terminal's
//! loadouts, the vehicle terminal's vehicles, or a base's console with its puzzle (solved to
//! hack the base, or by its owners to stop a hack). An infiltrator at an enemy terminal or
//! turret hacks it (standing still at it). The HUD shows what it would do (`UseHint`).

use bevy::prelude::*;
use ps_sim::base::{TerminalKind, USE_REACH};
use ps_sim::puzzle::Puzzle;
use ps_sim::vehicle::{VehicleKind, hull_distance};
use ps_sim::weapons::{ArmorKind, Kit, Loadout, Weapon};
use ps_sim::{BaseIdx, HackTarget, Request, VehicleId};

use crate::desktop::{Overlays, UsePressed};
use crate::game::{Game, LocalCmd, Mover};
use crate::materials::team_color;
use crate::neonui::{self, NeonButton};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Leave,
    Enter(VehicleId, VehicleKind),
    Heal,
    Equipment,
    Vehicles(BaseIdx, u8),
    Console(BaseIdx),
    Turret(u16),
    Hack(HackTarget),
    /// A harvester of one's faction's: its construction terminal.
    Construction(VehicleId),
}

impl Target {
    pub fn hint(&self) -> String {
        match self {
            Target::Leave => "Leave the vehicle".into(),
            Target::Enter(_, k) => format!("Enter the {}", k.name()),
            Target::Heal => "Heal".into(),
            Target::Equipment => "Equipment terminal".into(),
            Target::Vehicles(..) => "Vehicle terminal".into(),
            Target::Console(_) => "Base console".into(),
            Target::Turret(_) => "Man the turret".into(),
            Target::Hack(HackTarget::Terminal { .. }) => "Hack the terminal".into(),
            Target::Hack(HackTarget::Turret(_)) => "Hack the turret".into(),
            Target::Construction(_) => "Construction terminal".into(),
        }
    }
}

/// What the use key would do now.
#[derive(Resource, Default)]
pub struct UseHint(pub Option<Target>);

/// The panel open, and the loadout being put together.
#[derive(Resource, Default)]
pub struct Panel {
    pub open: Option<Target>,
    pub loadout: Option<Loadout>,
    /// The vehicle terminal: the vehicle picked and its loadout (ordered with `Pick::Order`).
    pub vehicle: Option<(VehicleKind, ps_sim::vehicle::VehicleLoadout)>,
    rev: u32,
    built: u32,
    puzzle_seen: Option<(u8, u32, Vec<u8>)>,
    /// Opened this frame (the key that opened it doesn't close it).
    fresh: bool,
}

#[derive(Component)]
struct PanelRoot;

/// A choice on a panel (the flat screen's buttons, and VR's: src/vrui.rs).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Armor(ArmorKind),
    Rifle(Weapon),
    Sidearm(Weapon),
    Special(Weapon),
    Kit(Kit),
    Grenade(ps_sim::weapons::GrenadeKind),
    /// Pull a structure from the harvester's construction terminal (VR's panel).
    Pull(ps_sim::construct::StructureKind),
    Equip,
    Vehicle(VehicleKind),
    VehicleGun(u8),
    VehicleDefense(ps_sim::vehicle::Defense),
    /// Order the vehicle picked.
    Order,
    Piece(u8),
    Close,
}

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UseHint>()
            .init_resource::<Panel>()
            .add_systems(
                Update,
                (hint, on_use, keys, clicks, build_panel)
                    .chain()
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, close_all.run_if(not(resource_exists::<Game>)));
    }
}

fn close_all(
    mut panel: ResMut<Panel>,
    mut overlays: ResMut<Overlays>,
    mut commands: Commands,
    roots: Query<Entity, With<PanelRoot>>,
) {
    if panel.open.is_some() {
        panel.open = None;
        overlays.terminal = false;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// The nearest thing to use.
fn hint(game: Res<Game>, mover: Res<Mover>, local: Res<LocalCmd>, mut hint: ResMut<UseHint>) {
    hint.0 = None;
    if !game.alive() {
        return;
    }
    if game.seat().is_some() || game.mine().is_some_and(|m| m.turret.is_some()) {
        hint.0 = Some(Target::Leave);
        return;
    }
    let Some(map) = &game.map else { return };
    if !mover.valid {
        return;
    }
    let feet = mover.body.feet;
    let eye = feet + Vec3::Y * ps_sim::player::eye_height(local.cmd.crouch);
    let mut best: Option<(f32, Target)> = None;
    let mut offer = |d: f32, t: Target| {
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, t));
        }
    };
    for v in &game.snap.vehicles {
        if v.team != game.team
            || (v.seats.iter().all(|s| s.is_some()) && v.kind != VehicleKind::Harvester)
        {
            continue;
        }
        let m = ps_sim::vehicle::Motion {
            pos: v.pos,
            rot: v.rot,
            ..ps_sim::vehicle::Motion::at(v.pos, 0.0)
        };
        let half = v.kind.spec().half;
        let d = hull_distance(&m, half, eye);
        if d < 3.2 {
            offer(d, Target::Enter(v.id, v.kind));
        }
        // A harvester's construction terminal: at its back (ahead of getting in there).
        if v.kind == VehicleKind::Harvester {
            let back = v.pos + v.rot * Vec3::new(0.0, 1.0, half.z + 0.5);
            let d = (back - eye).length();
            if d < 2.8 {
                offer(d - 2.0, Target::Construction(v.id));
            }
        }
    }
    let c = map.def.id;
    let me = game.mine();
    let spy = me.is_some_and(|m| m.loadout.armor == ArmorKind::Stealth);
    let max = me.is_some_and(|m| m.loadout.max());
    let mut ti = 0u16;
    for (bi, b) in map.bases.iter().enumerate() {
        for (i, &mount) in b.turrets.iter().enumerate() {
            let d = (mount + Vec3::Y * 1.0 - eye).length();
            let snap = game
                .snap
                .turrets
                .iter()
                .find(|t| t.base == bi as BaseIdx && t.index == i as u8);
            // (Whose it is now: a hack's faction, else the base's.)
            let team = snap.map_or(game.state.base(c, bi as BaseIdx).owner, |t| t.team);
            let up = snap.is_some_and(|t| t.health > 0.0);
            let free = snap.is_some_and(|t| t.occupant.is_none());
            // (The mount stands on its pillar, over the wall.)
            if team == Some(game.team) && up && free && d < 6.0 && !max {
                offer(d, Target::Turret(ti));
            } else if spy && up && team.is_some_and(|t| t != game.team) && d < 6.0 {
                offer(d, Target::Hack(HackTarget::Turret(ti)));
            }
            ti += 1;
        }
    }
    for (bi, b) in map.bases.iter().enumerate() {
        if (b.center - feet).length() > b.half * 1.6 {
            continue;
        }
        let owner = game.state.base(c, bi as BaseIdx).owner;
        let hacks = game
            .snap
            .bases
            .get(bi)
            .map_or((0, None), |s| (s.hacked, s.hacker));
        for (ti, t) in b.terminals.iter().enumerate() {
            let d = (t.pos - feet).length();
            if d >= USE_REACH + 0.4 {
                continue;
            }
            // (A hacked terminal is its hacker's faction's.)
            let team = if ti < 16 && hacks.0 & (1 << ti) != 0 {
                hacks.1
            } else {
                owner
            };
            if team == Some(game.team) {
                offer(
                    d,
                    match t.kind {
                        TerminalKind::Equipment => Target::Equipment,
                        TerminalKind::Medical => Target::Heal,
                        TerminalKind::Vehicle => Target::Vehicles(bi as BaseIdx, ti as u8),
                    },
                );
            } else if spy && team.is_some() {
                offer(
                    d,
                    Target::Hack(HackTarget::Terminal {
                        base: bi as BaseIdx,
                        index: ti as u8,
                    }),
                );
            }
        }
        if let Some((at, _)) = b.console {
            let d = (at - feet).length();
            if d < USE_REACH + 0.4 {
                offer(d, Target::Console(bi as BaseIdx));
            }
        }
    }
    hint.0 = best.map(|b| b.1);
}

fn on_use(
    mut game: ResMut<Game>,
    hint: Res<UseHint>,
    mut panel: ResMut<Panel>,
    mut overlays: ResMut<Overlays>,
    mut uses: MessageReader<UsePressed>,
    building: Option<ResMut<crate::build::Building>>,
) {
    if uses.read().last().is_none() {
        return;
    }
    // (A panel open: the press is the panel's, VR's trigger pointing at it.)
    if panel.open.is_some() || overlays.menu {
        return;
    }
    let Some(t) = hint.0 else { return };
    match t {
        Target::Leave => game.request(Request::ExitVehicle),
        Target::Enter(v, _) => game.request(Request::EnterVehicle(v)),
        Target::Heal => game.request(Request::Heal),
        Target::Turret(t) => game.request(Request::EnterTurret(t)),
        Target::Hack(h) => game.request(Request::Hack(h)),
        // (The flat screen's panel is src/build.rs's; VR's is this one's.)
        Target::Construction(v) => match building {
            Some(mut b) => crate::build::open_terminal(&mut b, v),
            None => open(&mut panel, &mut overlays, t),
        },
        Target::Console(b) => {
            game.request(Request::ViewConsole(b));
            open(&mut panel, &mut overlays, t);
        }
        Target::Equipment | Target::Vehicles(..) => {
            panel.loadout = game.mine().map(|m| m.loadout);
            panel.vehicle = None;
            open(&mut panel, &mut overlays, t);
        }
    }
}

fn open(panel: &mut Panel, overlays: &mut Overlays, t: Target) {
    panel.open = Some(t);
    panel.rev += 1;
    panel.fresh = true;
    overlays.terminal = true;
}

pub fn close(panel: &mut Panel, overlays: &mut Overlays) {
    panel.open = None;
    panel.rev += 1;
    overlays.terminal = false;
}

/// Esc or E closes the panel; walking away or dying too.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<Game>,
    hint: Res<UseHint>,
    mut panel: ResMut<Panel>,
    mut overlays: ResMut<Overlays>,
) {
    let Some(t) = panel.open else { return };
    let fresh = std::mem::take(&mut panel.fresh);
    if keys.just_pressed(KeyCode::Escape)
        || (keys.just_pressed(KeyCode::KeyE) && !fresh)
        || !game.alive()
        || hint.0 != Some(t)
    {
        close(&mut panel, &mut overlays);
    }
    // A new state of the console's puzzle: shown anew.
    if let Target::Console(b) = t
        && game.puzzle.as_ref().is_some_and(|p| p.0 == b)
        && panel.puzzle_seen != game.puzzle
    {
        panel.puzzle_seen = game.puzzle.clone();
        panel.rev += 1;
    }
}

fn clicks(
    mut game: ResMut<Game>,
    mut panel: ResMut<Panel>,
    mut overlays: ResMut<Overlays>,
    q: Query<(&Interaction, &Pick), Changed<Interaction>>,
) {
    for (i, pick) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        if apply(*pick, &mut game, &mut panel, &mut overlays) {
            return;
        }
    }
}

/// Does what choice `pick` on the open panel says; true if that closed the panel.
pub fn apply(pick: Pick, game: &mut Game, panel: &mut Panel, overlays: &mut Overlays) -> bool {
    let l = panel.loadout.get_or_insert_with(Loadout::default);
    match pick {
        Pick::Armor(a) => {
            // A MAX's arms in the first slot, or a rifle back out of one.
            if a == ArmorKind::Max && !Weapon::MAX_ARMS.contains(&l.rifle) {
                l.rifle = Weapon::Pounder;
            } else if a != ArmorKind::Max && !Weapon::rifles_for(game.team).contains(&l.rifle) {
                l.rifle = Weapon::Rifle;
            }
            l.armor = a;
        }
        Pick::Rifle(w) => l.rifle = w,
        Pick::Sidearm(w) => l.sidearm = w,
        Pick::Special(w) => {
            l.special = w;
            if w != Weapon::RepairGun {
                l.kit = Kit::None;
            }
        }
        Pick::Kit(k) => l.kit = k,
        Pick::Grenade(g) => l.grenade = g,
        Pick::Pull(kind) => {
            if let Some(Target::Construction(harvester)) = panel.open {
                game.request(Request::Pull { harvester, kind });
            }
            close(panel, overlays);
            return true;
        }
        Pick::Equip => {
            let l = *l;
            game.request(Request::Loadout(l));
            close(panel, overlays);
            return true;
        }
        Pick::Vehicle(k) => panel.vehicle = Some((k, k.default_loadout())),
        Pick::VehicleGun(g) => {
            if let Some((_, l)) = panel.vehicle.as_mut() {
                l.weapon = g;
            }
        }
        Pick::VehicleDefense(d) => {
            if let Some((_, l)) = panel.vehicle.as_mut() {
                l.defense = d;
            }
        }
        Pick::Order => {
            if let (Some(Target::Vehicles(base, terminal)), Some((kind, loadout))) =
                (panel.open, panel.vehicle)
            {
                game.request(Request::SpawnVehicle {
                    base,
                    terminal,
                    kind,
                    loadout,
                });
            }
            close(panel, overlays);
            return true;
        }
        Pick::Piece(p) => {
            if let Some(Target::Console(base)) = panel.open {
                game.request(Request::Poke { base, piece: p });
            }
        }
        Pick::Close => {
            close(panel, overlays);
            return true;
        }
    }
    panel.rev += 1;
    false
}

fn build_panel(
    mut commands: Commands,
    game: Res<Game>,
    mut panel: ResMut<Panel>,
    roots: Query<Entity, With<PanelRoot>>,
) {
    if panel.built == panel.rev {
        return;
    }
    panel.built = panel.rev;
    for e in &roots {
        commands.entity(e).despawn();
    }
    let Some(t) = panel.open else { return };
    let tc = team_color(Some(game.team));
    let color = Color::linear_rgb(tc.x, tc.y, tc.z);
    let loadout = panel.loadout.unwrap_or_default();
    commands
        .spawn((PanelRoot, neonui::backdrop(neonui::BACKDROP, 20)))
        .with_children(|r| {
            r.spawn(neonui::panel(color, Some(760.0)))
                .with_children(|p| match t {
                    Target::Equipment => {
                        neonui::title(p, "EQUIPMENT", 30.0, color);
                        p.spawn(neonui::text("Armor", 16.0, neonui::DIM));
                        p.spawn(neonui::row(10.0)).with_children(|row| {
                            for a in ArmorKind::ALL {
                                let mut b = NeonButton::new(color);
                                b.selected = loadout.armor == a;
                                let label = format!("{} ({:.0})", a.name(), a.max());
                                neonui::button(row, &label, Pick::Armor(a), b, None, 16.0);
                            }
                        });
                        let rifles = Weapon::rifles_for(game.team);
                        let max = loadout.max();
                        p.spawn(neonui::text(
                            if max { "Arms" } else { "Rifle" },
                            16.0,
                            neonui::DIM,
                        ));
                        p.spawn(neonui::row(10.0)).with_children(|row| {
                            let list: &[Weapon] = if max { &Weapon::MAX_ARMS } else { &rifles };
                            for &w in list {
                                let mut b = NeonButton::new(color);
                                b.selected = loadout.rifle == w;
                                neonui::button(row, w.stats().name, Pick::Rifle(w), b, None, 16.0);
                            }
                        });
                        if max {
                            p.spawn(neonui::text(
                                &armor_note(ArmorKind::Max),
                                14.0,
                                neonui::AMBER,
                            ));
                        } else {
                            if loadout.armor == ArmorKind::Stealth {
                                p.spawn(neonui::text(
                                    &armor_note(ArmorKind::Stealth),
                                    14.0,
                                    neonui::AMBER,
                                ));
                            }
                            p.spawn(neonui::text("Sidearm", 16.0, neonui::DIM));
                            p.spawn(neonui::row(10.0)).with_children(|row| {
                                for w in Weapon::SIDEARMS {
                                    let mut b = NeonButton::new(color);
                                    b.selected = loadout.sidearm == w;
                                    neonui::button(
                                        row,
                                        w.stats().name,
                                        Pick::Sidearm(w),
                                        b,
                                        None,
                                        16.0,
                                    );
                                }
                            });
                            p.spawn(neonui::text("Special", 16.0, neonui::DIM));
                            p.spawn(neonui::row(10.0)).with_children(|row| {
                                for w in Weapon::SPECIALS {
                                    let mut b = NeonButton::new(color);
                                    b.selected = loadout.special == w;
                                    neonui::button(
                                        row,
                                        w.stats().name,
                                        Pick::Special(w),
                                        b,
                                        None,
                                        16.0,
                                    );
                                }
                            });
                            p.spawn(neonui::text("Grenade", 16.0, neonui::DIM));
                            p.spawn(neonui::row(10.0)).with_children(|row| {
                                for g in ps_sim::weapons::GrenadeKind::ALL {
                                    let mut b = NeonButton::new(color);
                                    b.selected = loadout.grenade == g;
                                    neonui::button(row, g.name(), Pick::Grenade(g), b, None, 16.0);
                                }
                            });
                            if loadout.special == Weapon::RepairGun {
                                p.spawn(neonui::text(
                                    "Engineer's kit  ([B] puts it down)",
                                    16.0,
                                    neonui::DIM,
                                ));
                                p.spawn(neonui::row(10.0)).with_children(|row| {
                                    for k in Kit::ALL {
                                        let mut b = NeonButton::new(color);
                                        b.selected = loadout.kit == k;
                                        let label = if k.count() > 1 {
                                            format!("{} x{}", k.name(), k.count())
                                        } else {
                                            k.name().to_string()
                                        };
                                        neonui::button(row, &label, Pick::Kit(k), b, None, 16.0);
                                    }
                                });
                            }
                        }
                        p.spawn(neonui::gap(8.0));
                        p.spawn(neonui::row(16.0)).with_children(|row| {
                            neonui::button(
                                row,
                                "EQUIP",
                                Pick::Equip,
                                NeonButton::new(neonui::GREEN),
                                Some(180.0),
                                20.0,
                            );
                            neonui::button(
                                row,
                                "CLOSE",
                                Pick::Close,
                                NeonButton::new(neonui::DIM),
                                Some(140.0),
                                18.0,
                            );
                        });
                    }
                    Target::Vehicles(..) => {
                        neonui::title(p, "VEHICLES", 30.0, color);
                        let tech =
                            crate::build::benefit(&game, ps_sim::worldgen::Facility::TechPlant);
                        for k in VehicleKind::ALL {
                            let s = k.spec();
                            // (The advanced ones need a Tech Plant held here.)
                            let locked = k.advanced() && !tech;
                            let label = format!(
                                "{}   ({} seat{}){}",
                                k.name(),
                                s.seats.len(),
                                if s.seats.len() > 1 { "s" } else { "" },
                                if locked { "   NEEDS A TECH PLANT" } else { "" }
                            );
                            let mut b = NeonButton::new(if locked { neonui::DIM } else { color });
                            b.selected = panel.vehicle.is_some_and(|v| v.0 == k);
                            neonui::button(p, &label, Pick::Vehicle(k), b, Some(460.0), 18.0);
                        }
                        // The one picked: its gun and its defensive slot.
                        if let Some((k, l)) = panel.vehicle {
                            let guns = k.gun_options();
                            if guns.len() > 1 {
                                p.spawn(neonui::text("Main gun", 15.0, neonui::DIM));
                                p.spawn(neonui::row(10.0)).with_children(|row| {
                                    for (i, g) in guns.iter().enumerate() {
                                        let mut b = NeonButton::new(color);
                                        b.selected = l.weapon == i as u8;
                                        neonui::button(
                                            row,
                                            g.name,
                                            Pick::VehicleGun(i as u8),
                                            b,
                                            None,
                                            15.0,
                                        );
                                    }
                                });
                            }
                            p.spawn(neonui::text(
                                "Defence ([B] in the driver's seat)",
                                15.0,
                                neonui::DIM,
                            ));
                            p.spawn(neonui::row(10.0)).with_children(|row| {
                                for &d in k.defenses() {
                                    let mut b = NeonButton::new(color);
                                    b.selected = l.defense == d;
                                    neonui::button(
                                        row,
                                        d.name(),
                                        Pick::VehicleDefense(d),
                                        b,
                                        None,
                                        15.0,
                                    );
                                }
                            });
                        }
                        p.spawn(neonui::row(16.0)).with_children(|row| {
                            if let Some((k, _)) = panel.vehicle {
                                neonui::button(
                                    row,
                                    &format!("ORDER {}", k.name().to_uppercase()),
                                    Pick::Order,
                                    NeonButton::new(neonui::GREEN),
                                    Some(300.0),
                                    18.0,
                                );
                            }
                            neonui::button(
                                row,
                                "CLOSE",
                                Pick::Close,
                                NeonButton::new(neonui::DIM),
                                Some(140.0),
                                18.0,
                            );
                        });
                    }
                    Target::Construction(_) => {}
                    Target::Console(b) => {
                        let c = game.continent.unwrap_or(0);
                        let st = game.state.base(c, b);
                        let ours = st.owner == Some(game.team);
                        let name = game
                            .map
                            .as_ref()
                            .map_or(String::new(), |m| m.def.bases[b as usize].name.clone());
                        neonui::title(p, &format!("{} CORE", name.to_uppercase()), 28.0, color);
                        let line = match (ours, st.hack) {
                            (true, Some(h)) => format!(
                                "Being hacked by {}: {:.0} s left. Solve to stop it!",
                                h.team.upper(),
                                h.left
                            ),
                            (true, None) => "Secure. Nothing to do here.".to_string(),
                            (false, Some(h)) if h.team == game.team => {
                                format!("Your faction's hack: {:.0} s left. Hold the base!", h.left)
                            }
                            (false, _) => {
                                "Slide every piece into the square to hack the base.".to_string()
                            }
                        };
                        p.spawn(neonui::text(line, 16.0, neonui::TEXT));
                        if let Some((pb, seed, at)) = &game.puzzle
                            && *pb == b
                        {
                            puzzle_grid(p, *seed, at, color);
                        } else {
                            p.spawn(neonui::text("Connecting...", 16.0, neonui::DIM));
                        }
                        neonui::button(
                            p,
                            "CLOSE",
                            Pick::Close,
                            NeonButton::new(neonui::DIM),
                            Some(140.0),
                            18.0,
                        );
                    }
                    _ => {}
                });
        });
}

/// The sliding puzzle as a grid of cells: the square in the middle, the pieces out round it.
fn puzzle_grid(p: &mut ChildSpawnerCommands, seed: u32, at: &[u8], color: Color) {
    let puzzle = Puzzle::generate(seed);
    let n = ps_sim::puzzle::N as i8;
    let d = ps_sim::puzzle::DEPTH as i8;
    let cell = 42.0;
    let hues = [
        Color::srgb(1.0, 0.3, 0.6),
        Color::srgb(0.2, 0.9, 1.0),
        Color::srgb(1.0, 0.8, 0.2),
        Color::srgb(0.5, 1.0, 0.4),
        Color::srgb(0.7, 0.4, 1.0),
        Color::srgb(1.0, 0.5, 0.2),
        Color::srgb(0.4, 0.6, 1.0),
        Color::srgb(0.9, 0.9, 0.9),
    ];
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|g| {
        for r in (-d..n + d).rev() {
            g.spawn(neonui::row(2.0)).with_children(|row| {
                for c in -d..n + d {
                    let inside = (0..n).contains(&c) && (0..n).contains(&r);
                    let piece = puzzle.piece_at(at, c, r);
                    let bg = match piece {
                        Some(i) => hues[i % hues.len()],
                        None if inside => color.with_alpha(0.15),
                        None => Color::NONE,
                    };
                    let mut e = row.spawn((
                        Node {
                            width: Val::Px(cell),
                            height: Val::Px(cell),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(bg),
                        BorderColor::all(if inside {
                            color.with_alpha(0.5)
                        } else {
                            Color::NONE
                        }),
                    ));
                    if let Some(i) = piece {
                        e.insert((Button, Pick::Piece(i as u8)));
                    }
                }
            });
        }
    });
}

/// What a suit is about, under its choices.
pub fn armor_note(a: ArmorKind) -> String {
    match a {
        ArmorKind::Max => format!(
            "MAX: {:.0} resources from the base's silo. No vehicles' controls or guns, no \
             consoles; engineers mend it.",
            ps_sim::weapons::MAX_COST
        ),
        ArmorKind::Stealth => format!(
            "Infiltrator: [B] cloaks ({:.0} s); use an enemy terminal or turret to hack it.",
            ps_sim::weapons::CLOAK_TIME
        ),
        _ => String::new(),
    }
}
