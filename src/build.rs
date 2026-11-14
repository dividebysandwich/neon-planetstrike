//! Construction on the flat screen. E at one's faction's harvester opens its construction
//! terminal: the structures and what each costs of the harvester's load; picking one pulls it
//! (the server takes the cost) and one carries it. Carrying, its ghost shows where the view
//! meets the ground, facing the builder: green where it may go and draw on a mobile silo, amber
//! where it would have no silo to draw on (it would decay), red where it may not go (in a base,
//! out of reach). A click puts it down; the right button or Esc puts it away (still carried:
//! N takes it out again). (VR: the terminal's panel in src/vrui.rs, placing on the command
//! wheel, src/wheel.rs.)

use bevy::prelude::*;
use ps_sim::Request;
use ps_sim::VehicleId;
use ps_sim::construct::{BUILD_REACH, NO_BUILD, SILO_RANGE, StructureKind};
use ps_sim::worldgen::BaseKind;

use crate::desktop::{MainCamera, Overlays};
use crate::game::Game;
use crate::interact::{Target, UseHint};
use crate::materials::{GlowMaterial, NeonUniform, team_color};
use crate::neonui::{self, NeonButton};
use crate::settings::{Action, Inputs, Settings};

/// The construction terminal open (at that harvester), and the carried structure being placed.
#[derive(Resource, Default)]
pub struct Building {
    pub open: Option<VehicleId>,
    pub placing: Option<StructureKind>,
    /// What was carried last frame (a new one goes straight to placing).
    carried: Option<StructureKind>,
    built: Option<String>,
}

#[derive(Component)]
struct PanelRoot;

#[derive(Component, Clone, Copy)]
enum BuildAct {
    Pull(StructureKind),
    Close,
}

/// The ghost of what's being placed (one per kind, shown while placing it).
#[derive(Component)]
struct Ghost(StructureKind);

#[derive(Resource)]
struct GhostLook {
    ok: Handle<GlowMaterial>,
    warn: Handle<GlowMaterial>,
    bad: Handle<GlowMaterial>,
}

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Building>()
            .add_systems(Startup, make_ghosts)
            .add_systems(
                Update,
                (keys.after(crate::pause::toggle), clicks, panel, ghost)
                    .chain()
                    .run_if(resource_exists::<Game>),
            )
            .add_systems(Update, clear.run_if(not(resource_exists::<Game>)));
    }
}

/// Whether the local player's faction has facility `f`'s benefit on its continent (as the
/// sim has it: a base of the kind held, its generator up; a sanctuary has them all).
pub fn benefit(game: &Game, f: ps_sim::worldgen::Facility) -> bool {
    let Some(map) = game.map.as_ref() else {
        return false;
    };
    let c = map.def.id;
    (0..map.def.bases.len()).any(|b| {
        game.state.base(c, b as ps_sim::BaseIdx).owner == Some(game.team)
            && (map.def.bases[b].kind == BaseKind::Home
                || (map.def.facility(b as ps_sim::BaseIdx) == Some(f)
                    && game.snap.bases.get(b).is_none_or(|s| s.generator > 0.0)))
    })
}

/// The name of the base nearest `at` on the continent.
pub fn nearest_base(game: &Game, at: Vec3) -> Option<String> {
    let map = game.map.as_ref()?;
    map.bases
        .iter()
        .enumerate()
        .min_by(|a, b| {
            (a.1.center - at)
                .length()
                .total_cmp(&(b.1.center - at).length())
        })
        .map(|(b, _)| map.def.bases[b].name.clone())
}

/// The structure the local player carries (pulled at a harvester, not yet put down).
pub fn carrying(game: &Game) -> Option<StructureKind> {
    game.mine()
        .and_then(|m| m.carrying)
        .filter(|_| game.alive())
}

/// A harvester's load as the snapshot has it (resources).
pub fn load(game: &Game, v: VehicleId) -> f32 {
    game.vehicle(v).map_or(0.0, |v| v.cargo as f32)
}

/// Whether `at` is where nobody builds: in a base, or by a tower (as the server reckons it).
pub fn no_build(game: &Game, at: Vec3) -> bool {
    game.map.as_ref().is_some_and(|m| {
        m.bases.iter().any(|b| {
            let r = if b.kind == BaseKind::Tower {
                30.0
            } else {
                b.half * NO_BUILD
            };
            Vec2::new(b.center.x - at.x, b.center.z - at.z).length() < r
        })
    })
}

/// Whether a structure at `at` would have a mobile silo of the faction's to draw on.
pub fn silo_near(game: &Game, at: Vec3) -> bool {
    game.snap.structures.iter().any(|s| {
        s.kind == StructureKind::Silo
            && s.team == game.team
            && s.fill > 0
            && (s.pos - at).length() < SILO_RANGE
    })
}

/// What a spot is like for a structure: may it go there, and would it have a silo.
pub fn spot_state(game: &Game, kind: StructureKind, at: Vec3, in_reach: bool) -> (bool, bool) {
    let ok = in_reach && !no_build(game, at);
    let fed = kind == StructureKind::Silo || silo_near(game, at);
    (ok, fed)
}

fn make_ghosts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let mut glow = |c: Vec3| {
        glows.add(GlowMaterial {
            neon: NeonUniform::fixed(c, 0.6),
            overlay: false,
        })
    };
    let ok = glow(Vec3::new(0.2, 1.0, 0.4));
    let warn = glow(Vec3::new(1.0, 0.7, 0.1));
    let bad = glow(Vec3::new(1.0, 0.15, 0.1));
    for k in StructureKind::ALL {
        let (h, height) = k.size();
        commands.spawn((
            Ghost(k),
            Mesh3d(meshes.add(Cuboid::new(h.x * 2.0, height, h.y * 2.0))),
            MeshMaterial3d(ok.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(GhostLook { ok, warn, bad });
}

fn clear(
    mut commands: Commands,
    mut b: ResMut<Building>,
    mut overlays: ResMut<Overlays>,
    roots: Query<Entity, With<PanelRoot>>,
    mut ghosts: Query<&mut Visibility, With<Ghost>>,
) {
    *b = Building::default();
    overlays.build = false;
    overlays.placing = false;
    for e in &roots {
        commands.entity(e).despawn();
    }
    for mut v in &mut ghosts {
        *v = Visibility::Hidden;
    }
}

/// Where the view meets the ground, the yaw facing the builder, and whether that's in reach.
fn aimed(game: &Game, cam: &Transform) -> Option<(Vec3, f32, bool)> {
    let from = cam.translation;
    let fwd = cam.forward();
    let p = game.world.ray(from, from + *fwd * (BUILD_REACH + 25.0))?.1;
    let me = game.mine()?.feet;
    let yaw = ps_sim::yaw_of(Vec3::new(me.x - p.x, 0.0, me.z - p.z));
    Some((p, yaw, (p - me).length() <= BUILD_REACH))
}

/// The construction terminal: opened from src/interact.rs (E at a harvester).
pub fn open_terminal(b: &mut Building, harvester: VehicleId) {
    b.open = Some(harvester);
    b.built = None;
}

#[allow(clippy::too_many_arguments)]
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    hint: Res<UseHint>,
    mut game: ResMut<Game>,
    mut b: ResMut<Building>,
    mut overlays: ResMut<Overlays>,
    cams: Query<&Transform, With<MainCamera>>,
) {
    let i = Inputs {
        keys: &keys,
        mouse: &mouse,
        wheel: default(),
    };
    let carried = carrying(&game);
    // A structure newly carried: out to place it at once.
    if carried.is_some() && carried != b.carried {
        b.placing = carried;
        b.open = None;
    }
    b.carried = carried;
    if carried.is_none() {
        b.placing = None;
    }
    let build_key = settings.just_pressed(Action::Build, &i);
    if let Some(v) = b.open {
        // (Walking away closes it.)
        if keys.just_pressed(KeyCode::Escape)
            || hint.0 != Some(Target::Construction(v))
            || !game.alive()
        {
            b.open = None;
        }
    } else if let Some(kind) = b.placing {
        if keys.just_pressed(KeyCode::Escape) || build_key || mouse.just_pressed(MouseButton::Right)
        {
            b.placing = None;
            overlays.hold_fire = true;
        } else if mouse.just_pressed(MouseButton::Left)
            && let Ok(cam) = cams.single()
            && let Some((at, yaw, _)) = aimed(&game, cam)
        {
            game.request(Request::Build { kind, at, yaw });
            overlays.hold_fire = true;
        }
    } else if build_key && carried.is_some() && !overlays.any() {
        b.placing = carried;
    }
    overlays.build = b.open.is_some();
    overlays.placing = b.placing.is_some();
}

fn clicks(
    mut game: ResMut<Game>,
    mut b: ResMut<Building>,
    q: Query<(&Interaction, &BuildAct), Changed<Interaction>>,
    mut overlays: ResMut<Overlays>,
) {
    for (i, a) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match *a {
            BuildAct::Pull(kind) => {
                if let Some(harvester) = b.open {
                    game.request(Request::Pull { harvester, kind });
                }
                b.open = None;
                overlays.hold_fire = true;
            }
            BuildAct::Close => b.open = None,
        }
    }
    overlays.build = b.open.is_some();
}

fn panel(
    mut commands: Commands,
    game: Res<Game>,
    mut b: ResMut<Building>,
    roots: Query<Entity, With<PanelRoot>>,
) {
    let sig = b
        .open
        .map(|v| format!("{v} {:.0} {:?}", load(&game, v), carrying(&game)));
    if sig == b.built && (sig.is_none() || !roots.is_empty()) {
        return;
    }
    b.built = sig;
    for e in &roots {
        commands.entity(e).despawn();
    }
    let Some(v) = b.open else { return };
    let tc = team_color(Some(game.team));
    let color = Color::linear_rgb(tc.x, tc.y, tc.z);
    let have = load(&game, v);
    let off = 1.0 - crate::hud::lock_discount(&game);
    commands
        .spawn((PanelRoot, neonui::backdrop(neonui::BACKDROP, 18)))
        .with_children(|r| {
            r.spawn(neonui::panel(color, Some(600.0)))
                .with_children(|p| {
                    neonui::title(p, "CONSTRUCTION", 32.0, color);
                    p.spawn(neonui::text(
                        format!("The harvester's load: {have:.0}"),
                        16.0,
                        neonui::TEXT,
                    ));
                    if let Some(k) = carrying(&game) {
                        p.spawn(neonui::text(
                            format!("Carrying a {}: put it down first (N)", k.name()),
                            15.0,
                            neonui::AMBER,
                        ));
                    }
                    for k in StructureKind::ALL {
                        let cost = k.cost() * off;
                        let afford = have >= cost && carrying(&game).is_none();
                        let neon = NeonButton::new(if afford { color } else { neonui::DIM });
                        neonui::button(
                            p,
                            &format!("{}   ({cost:.0})", k.name()),
                            BuildAct::Pull(k),
                            neon,
                            Some(460.0),
                            18.0,
                        );
                    }
                    p.spawn(neonui::text(
                        "Structures draw on a mobile silo of your faction's within 120 m and decay \
                     without one. Park a harvester by a silo to fill it.",
                        13.0,
                        neonui::DIM,
                    ));
                    neonui::button(
                        p,
                        "CLOSE",
                        BuildAct::Close,
                        NeonButton::new(neonui::DIM),
                        Some(160.0),
                        16.0,
                    );
                });
        });
}

/// The ghost of what's being placed, where it would go.
#[allow(clippy::type_complexity)]
fn ghost(
    game: Res<Game>,
    b: Res<Building>,
    look: Option<Res<GhostLook>>,
    cams: Query<&Transform, With<MainCamera>>,
    mut ghosts: Query<
        (
            &Ghost,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<GlowMaterial>,
        ),
        Without<MainCamera>,
    >,
) {
    let Some(look) = look else { return };
    let spot = b
        .placing
        .and_then(|k| Some((k, aimed(&game, cams.single().ok()?)?)));
    for (g, mut t, mut v, mut m) in &mut ghosts {
        let Some((k, (at, yaw, reach))) = spot.filter(|s| s.0 == g.0) else {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
            continue;
        };
        *v = Visibility::Visible;
        t.translation = at + Vec3::Y * k.size().1 * 0.5;
        t.rotation = Quat::from_rotation_y(yaw);
        let want = match spot_state(&game, k, at, reach) {
            (false, _) => &look.bad,
            (true, false) => &look.warn,
            (true, true) => &look.ok,
        };
        if m.0 != *want {
            m.0 = want.clone();
        }
    }
}
