//! Players on screen, following `Game::snap`: the floating-part avatars (an animal head, the
//! vest, gloves, and the body between them posed by `crate::rig`), with the weapon in hand.
//! VR players show their tracked head and hands; everyone else's are worked out from where
//! they look. Players in vehicles are hidden inside them. One's own avatar shows only the gun
//! and the gloves holding it (in VR both gloves, and the legs). A MAX is drawn a size up with
//! armour plating over the torso; a cloaked player is only a faint shimmer.

use std::collections::HashMap;

use bevy::prelude::*;
use ps_sim::snapshot::{PlayerSnap, flags};
use ps_sim::weapons::Weapon;
use ps_sim::{PlayerId, Team};

use crate::assets::{Model, Models};
use crate::game::{DrawSet, Game, LocalCmd, Mover, alpha};
use crate::materials::{AccentMaterial, GlowMaterial, Palette};

#[derive(Component)]
pub struct Avatar(pub PlayerId);

/// How much bigger than its model a head is drawn, grown about the skull top.
const HEAD_SCALE: f32 = 1.5;
const HEAD_CROWN: f32 = 0.15;

struct AvatarParts {
    root: Entity,
    body: Entity,
    head: Entity,
    /// The animal's head and eyes (under `head`; hidden in a MAX's helmet).
    face: Entity,
    hands: [Entity; 2],
    hand_looks: [[Entity; 2]; 2],
    neck: Entity,
    pelvis: Entity,
    arms: [[Entity; 2]; 2],
    legs: [[Entity; 3]; 2],
    gait: crate::rig::Gait,
    tail: Vec<Entity>,
    animal: usize,
    swing: crate::tail::Tail,
    /// The guns under `gun` (in the gun hand), each spawned when first held.
    gun: Entity,
    guns: Vec<Gun>,
    /// How far the held gun's turning parts have turned, and how fast (radians, per second).
    spin: f32,
    spin_rate: f32,
    team: Team,
    /// The exosuit, once the player has worn one.
    max: Option<MaxParts>,
    /// The cloak's shimmer.
    shimmer: Entity,
}

/// A gun's model under an avatar's `gun`, and its turning parts (`spawn_spins`).
struct Gun {
    weapon: Weapon,
    entity: Entity,
    spins: Vec<Entity>,
}

/// A MAX's exosuit (tools/models/max.py), drawn in place of the infantry parts and posed by
/// the same rig with `crate::rig::MAX` proportions.
struct MaxParts {
    torso: Entity,
    /// Under the avatar's `head`.
    helmet: Entity,
    pauldrons: [Entity; 2],
    arms: [[Entity; 2]; 2],
    /// Under the avatar's `hands`: the gauntlets (a hand in an arm gun's cradle has none).
    fists: [Entity; 2],
    pelvis: Entity,
    legs: [[Entity; 3]; 2],
}

/// How fast a firing gun's barrels turn (radians per second), and how quickly they get there.
const SPIN_RATE: f32 = 30.0;
const SPIN_UP: f32 = 6.0;

/// How much bigger a MAX is drawn.
const MAX_SCALE: f32 = 1.15;

/// The cloak's shimmer: a faint column of light.
#[derive(Resource)]
struct CloakLook {
    mesh: Handle<Mesh>,
    mat: Handle<GlowMaterial>,
}

fn make_cloak_look(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    commands.insert_resource(CloakLook {
        mesh: meshes.add(Capsule3d::new(0.32, 1.1)),
        mat: glows.add(GlowMaterial {
            neon: crate::materials::NeonUniform::fixed(Vec3::new(0.5, 0.7, 1.0), 0.25),
            overlay: false,
        }),
    });
}

#[derive(Resource, Default)]
struct Views {
    players: HashMap<PlayerId, AvatarParts>,
    continent: Option<u8>,
}

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Views>()
            .add_systems(Startup, make_cloak_look)
            .add_systems(
                Update,
                (sync_players.run_if(resource_exists::<Game>), clear)
                    .chain()
                    .in_set(DrawSet),
            );
    }
}

fn clear(mut commands: Commands, game: Option<Res<Game>>, mut views: ResMut<Views>) {
    if game.is_none() && !views.players.is_empty() {
        for (_, v) in views.players.drain() {
            commands.entity(v.root).despawn();
            commands.entity(v.gun).despawn();
            commands.entity(v.shimmer).despawn();
        }
    }
}

/// Spawns a model's parts under `parent`, materials by name; returns the parts named `hit_N`.
pub fn spawn_model(
    commands: &mut Commands,
    parent: Entity,
    model: &Model,
    palette: &Palette,
    team: Option<Team>,
) -> Vec<(Entity, u8)> {
    let mut hits = Vec::new();
    for (mat, mesh) in &model.parts {
        let mut e = commands.spawn((Mesh3d(mesh.clone()), Transform::IDENTITY, ChildOf(parent)));
        if let Some(m) = palette.surface(mat, team) {
            e.insert(MeshMaterial3d(m));
            continue;
        }
        match mat.as_str() {
            "body" => {
                e.insert(MeshMaterial3d(palette.avatar.clone()));
            }
            "fur_light" => {
                e.insert(MeshMaterial3d(palette.fur.clone()));
            }
            "sclera" => {
                e.insert(MeshMaterial3d(palette.sclera.clone()));
            }
            "glow" | "eye_glow" => {
                e.insert(MeshMaterial3d::<GlowMaterial>(palette.glow_for(team)));
            }
            "halo_white" => {
                e.insert(MeshMaterial3d::<GlowMaterial>(palette.glow_for(None)));
            }
            "eyes" => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.eyes_for(team)));
            }
            "flow" => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.flow_for(team)));
            }
            "glow_white" | "muzzle" => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.accent_for(None)));
            }
            m => {
                e.insert(MeshMaterial3d::<AccentMaterial>(palette.accent_for(team)));
                if let Some(n) = m.strip_prefix("hit_").and_then(|n| n.parse().ok()) {
                    hits.push((e.id(), n));
                }
            }
        }
    }
    hits
}

/// A cloakable avatar's shimmer material and part lists (kept for `crate::eyes`).
pub type Ghost<'a> = (
    &'a Handle<GlowMaterial>,
    &'a mut Vec<Entity>,
    &'a mut Vec<Entity>,
);

/// The model of a weapon held (the blaster when there is none).
/// The gun in a player's hand (every avatar's): for what comes out of its muzzle (src/fx.rs).
#[derive(Component)]
pub struct GunOf(pub PlayerId);

pub fn weapon_model(models: &Models, w: Weapon) -> Model {
    let name = weapon_mesh(w);
    if models.has(name) {
        models.get(name)
    } else {
        models.blaster.clone()
    }
}

pub fn weapon_mesh(w: Weapon) -> &'static str {
    match w {
        Weapon::Rifle => "rifle",
        Weapon::Ember => "ember",
        Weapon::Volt => "volt",
        Weapon::Hive => "hive",
        Weapon::Scatter => "scatter",
        Weapon::Lance => "lance",
        Weapon::Striker => "striker",
        Weapon::Pistol => "pistol",
        Weapon::Medgun => "medgun",
        Weapon::RepairGun => "repair_gun",
        Weapon::Skyhook => "skyhook",
        Weapon::Pounder => "pounder",
        Weapon::Mauler => "mauler",
        Weapon::Burster => "burster",
    }
}

/// A MAX's arm guns: worn on the forearm, the hand inside their cradle.
fn arm_gun(w: Weapon) -> bool {
    matches!(w, Weapon::Pounder | Weapon::Mauler | Weapon::Burster)
}

/// A gun's turning parts (a rotary gun's barrel clusters), spawned under its entity `gun`:
/// the mesh `<mesh>_spin`, one at each node `<mesh>_spin_N`, to be turned about their Z.
pub fn spawn_spins(
    commands: &mut Commands,
    gun: Entity,
    models: &Models,
    palette: &Palette,
    team: Option<Team>,
    w: Weapon,
) -> Vec<Entity> {
    let name = format!("{}_spin", weapon_mesh(w));
    if !models.has(&name) {
        return Vec::new();
    }
    let model = models.get(&name);
    (0..)
        .map_while(|i| models.nodes.get(&format!("{name}_{i}")).copied())
        .map(|at| {
            let e = commands
                .spawn((
                    Transform::from_translation(at),
                    Visibility::default(),
                    ChildOf(gun),
                ))
                .id();
            spawn_model(commands, e, &model, palette, team);
            e
        })
        .collect()
}

/// Where a weapon's muzzle is in its frame.
pub fn muzzle_of(models: &Models, w: Weapon) -> Vec3 {
    models
        .nodes
        .get(&format!("{}_muzzle", weapon_mesh(w)))
        .copied()
        .unwrap_or(models.muzzle)
}

fn spawn_avatar(
    commands: &mut Commands,
    p: &PlayerSnap,
    models: &Models,
    palette: &Palette,
    look: &CloakLook,
) -> AvatarParts {
    let team = Some(p.team);
    let root = commands
        .spawn((
            Avatar(p.id),
            Transform::from_translation(p.feet),
            Visibility::default(),
        ))
        .id();
    let part = |commands: &mut Commands, model: &Model, parent: Entity| {
        let e = commands
            .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(parent)))
            .id();
        spawn_model(commands, e, model, palette, team);
        e
    };
    let body = part(commands, &models.torso, root);
    let shimmer = commands
        .spawn((
            Mesh3d(look.mesh.clone()),
            MeshMaterial3d(look.mat.clone()),
            Transform::from_translation(p.feet),
            Visibility::Hidden,
        ))
        .id();
    let head = commands
        .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(root)))
        .id();
    let animal_index = p.id as usize % models.heads.len().max(1);
    let grown = commands
        .spawn((
            Transform::from_xyz(0.0, HEAD_CROWN * (1.0 - HEAD_SCALE), 0.0)
                .with_scale(Vec3::splat(HEAD_SCALE)),
            Visibility::default(),
            ChildOf(head),
        ))
        .id();
    if let Some(animal) = models.heads.get(animal_index) {
        spawn_model(commands, grown, &animal.model, palette, team);
        crate::eyes::spawn_eyes(
            commands,
            grown,
            animal,
            models,
            palette,
            team,
            p.id as u64,
            None,
        );
    }
    let mut hand_looks = [[Entity::PLACEHOLDER; 2]; 2];
    let hands = [0, 1].map(|i| {
        let h = commands
            .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(root)))
            .id();
        hand_looks[i] = [
            part(commands, &models.hands[i], h),
            part(commands, &models.grips[i], h),
        ];
        h
    });
    let b = &models.body;
    let neck = part(commands, &b.neck, root);
    let pelvis = part(commands, &b.pelvis, root);
    let arms = [0, 1].map(|_| {
        [
            part(commands, &b.upper_arm, root),
            part(commands, &b.forearm, root),
        ]
    });
    let legs = [0, 1].map(|_| {
        [
            part(commands, &b.thigh, root),
            part(commands, &b.shin, root),
            part(commands, &b.boot, root),
        ]
    });
    let tail = models
        .heads
        .get(animal_index)
        .and_then(|h| h.tail.as_ref())
        .map(|t| t.pieces.iter().map(|m| part(commands, m, root)).collect())
        .unwrap_or_default();
    let gun = commands
        .spawn((Transform::IDENTITY, Visibility::default(), GunOf(p.id)))
        .id();
    AvatarParts {
        root,
        body,
        head,
        face: grown,
        hands,
        hand_looks,
        neck,
        pelvis,
        arms,
        legs,
        gait: default(),
        tail,
        animal: animal_index,
        swing: default(),
        gun,
        guns: Vec::new(),
        spin: 0.0,
        spin_rate: 0.0,
        team: p.team,
        max: None,
        shimmer,
    }
}

/// The exosuit's parts under the avatar `v` (none if the models are missing: the MAX is then
/// drawn as infantry, a size up).
fn spawn_max(
    commands: &mut Commands,
    v: &AvatarParts,
    models: &Models,
    palette: &Palette,
) -> Option<MaxParts> {
    if !models.has("max_torso") {
        return None;
    }
    let team = Some(v.team);
    let part = |commands: &mut Commands, name: &str, parent: Entity| {
        let e = commands
            .spawn((Transform::IDENTITY, Visibility::Hidden, ChildOf(parent)))
            .id();
        spawn_model(commands, e, &models.get(name), palette, team);
        e
    };
    Some(MaxParts {
        torso: part(commands, "max_torso", v.root),
        helmet: part(commands, "max_helmet", v.head),
        pauldrons: [0, 1].map(|_| part(commands, "max_pauldron", v.root)),
        arms: [0, 1].map(|_| {
            [
                part(commands, "max_upper_arm", v.root),
                part(commands, "max_forearm", v.root),
            ]
        }),
        fists: v.hands.map(|h| part(commands, "max_fist", h)),
        pelvis: part(commands, "max_pelvis", v.root),
        legs: [0, 1].map(|_| {
            [
                part(commands, "max_thigh", v.root),
                part(commands, "max_shin", v.root),
                part(commands, "max_boot", v.root),
            ]
        }),
    })
}

/// Where a player's head and hands are (relative to the feet) when the snapshot has no VR
/// poses: from the look, the gun held in front of the right shoulder.
pub fn flat_poses(yaw: f32, pitch: f32, crouch: bool, w: Weapon) -> (Transform, [Transform; 2]) {
    let look = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
    let level = Vec3::Y * ps_sim::player::eye_height(crouch);
    let eye = ps_sim::avatar::eye_on_neck(level, look);
    let head = Transform::from_translation(eye).with_rotation(look);
    let long = !matches!(w, Weapon::Pistol | Weapon::Medgun);
    // (A MAX's arm gun: lower and further out, so its bulk stays out of the view; the free
    // fist at the side.)
    let arm = arm_gun(w);
    let right_at = if arm {
        Vec3::new(0.24, -0.42, -0.4)
    } else {
        Vec3::new(0.17, -0.27, -0.34)
    };
    let right = Transform::from_translation(eye + look * right_at).with_rotation(look);
    let (left_at, roll) = if arm {
        (Vec3::new(-0.3, -0.42, -0.3), 0.0)
    } else if long {
        (Vec3::new(-0.02, -0.25, -0.6), -0.6)
    } else {
        (Vec3::new(0.08, -0.32, -0.3), -0.6)
    };
    let left = Transform::from_translation(eye + look * left_at)
        .with_rotation(look * Quat::from_rotation_z(roll));
    (head, [left, right])
}

/// The guns' grip axis (bottom to top) in their frame: tilted back 22 degrees, as the gripping
/// gloves are (tools/models/weapons.py).
const GRIP_AXIS: Vec3 = Vec3::new(0.0, 0.927_184, -0.374_607);

/// One's own off hand on a two-handed gun (flat screen, first person): the left glove's pose in
/// the gun's frame (the gun hand's; grip at the origin), and whether it grips a fore grip (the
/// gripping glove, turned to the fore grip's slant) or cups the handguard from below (the open
/// glove, palm up and in, thumb along the left side). None for guns held in one hand: the
/// pistol, the medgun, and a MAX's arm guns (worn on the forearm).
fn support(w: Weapon) -> Option<(Transform, bool)> {
    // The open left glove's palm faces +X: turned up and towards the gun.
    let cup = Quat::from_rotation_z(1.05) * Quat::from_rotation_y(-0.25);
    let palm = |at: Vec3| Some((Transform::from_translation(at).with_rotation(cup), false));
    match w {
        // Under the handguard (the rifle's and lance's), the pump, the repair gun's nozzle.
        Weapon::Rifle => palm(Vec3::new(-0.024, 0.042, -0.24)),
        // The factions' rifles: under the Ember's heat shroud, the Volt's slim handguard,
        // the Hive's helical magazine.
        Weapon::Ember => palm(Vec3::new(-0.03, 0.044, -0.25)),
        Weapon::Volt => palm(Vec3::new(-0.018, 0.038, -0.24)),
        Weapon::Hive => palm(Vec3::new(-0.024, 0.004, -0.22)),
        Weapon::Lance => palm(Vec3::new(-0.022, 0.04, -0.3)),
        Weapon::Scatter => palm(Vec3::new(-0.019, 0.012, -0.205)),
        Weapon::RepairGun => palm(Vec3::new(-0.029, 0.04, -0.21)),
        // The launchers' fore grip: a bar from (y 0.09, z -0.3) down to (-0.02, -0.32).
        Weapon::Striker | Weapon::Skyhook => {
            let axis = Vec3::new(0.0, 0.11, 0.02).normalize();
            Some((
                Transform::from_xyz(0.0, 0.035, -0.31)
                    .with_rotation(Quat::from_rotation_arc(GRIP_AXIS, axis)),
                true,
            ))
        }
        Weapon::Pistol | Weapon::Medgun | Weapon::Pounder | Weapon::Mauler | Weapon::Burster => {
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_players(
    mut commands: Commands,
    game: Res<Game>,
    mut views: ResMut<Views>,
    models: Res<Models>,
    palette: Res<Palette>,
    local: Res<LocalCmd>,
    mover: Res<Mover>,
    time: Res<Time<Fixed>>,
    real: Res<Time>,
    xr: Option<Res<crate::vr::XrInput>>,
    sway: Option<Res<crate::viewkick::Sway>>,
    look: Option<Res<CloakLook>>,
    mut transforms: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(look) = look else { return };
    let vr = xr.is_some();
    let a = alpha(&time);
    let views = &mut *views;
    if views.continent != game.continent {
        views.continent = game.continent;
        for (_, v) in views.players.drain() {
            commands.entity(v.root).despawn();
            commands.entity(v.gun).despawn();
            commands.entity(v.shimmer).despawn();
        }
    }
    let gone: Vec<PlayerId> = views
        .players
        .iter()
        .filter(|(id, v)| {
            !game
                .snap
                .players
                .iter()
                .any(|p| p.id == **id && p.team == v.team)
        })
        .map(|(id, _)| *id)
        .collect();
    for id in gone {
        if let Some(v) = views.players.remove(&id) {
            commands.entity(v.root).despawn();
            commands.entity(v.gun).despawn();
            commands.entity(v.shimmer).despawn();
        }
    }
    let set_vis = |vis: &mut Query<&mut Visibility>, e: Entity, on: bool| {
        if let Ok(mut v) = vis.get_mut(e) {
            let want = if on {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *v != want {
                *v = want;
            }
        }
    };
    for p in &game.snap.players {
        let v = views
            .players
            .entry(p.id)
            .or_insert_with(|| spawn_avatar(&mut commands, p, &models, &palette, &look));
        let prev = game.prev.players.iter().find(|q| q.id == p.id);
        // (Testing: PS_SHOW_SELF=1 draws one's own avatar as anyone else's, for a PS_VIEW.)
        let me = Some(p.id) == game.me && std::env::var("PS_SHOW_SELF").is_err();
        let mine = me && mover.valid;
        // (Not oneself manning a turret: the view is the turret's.)
        let in_turret = me && game.mine().is_some_and(|m| m.turret.is_some());
        let alive = !p.has(flags::DEAD) && p.seat.is_none() && (!me || game.alive()) && !in_turret;
        // Cloaked: only the shimmer (oneself: the gun as ever).
        let cloaked = alive && p.has(flags::CLOAKED) && !me;
        set_vis(&mut vis, v.root, alive && !cloaked);
        set_vis(&mut vis, v.gun, alive && !cloaked);
        set_vis(&mut vis, v.shimmer, cloaked);
        if cloaked {
            let feet = prev.map_or(p.feet, |q| q.feet.lerp(p.feet, a));
            if let Ok(mut t) = transforms.get_mut(v.shimmer) {
                t.translation = feet + Vec3::Y * 0.9;
            }
            continue;
        }
        if !alive {
            continue;
        }
        // A MAX: a size up (not oneself, whose view it would upset), in its exosuit (spawned
        // the first time; shown from the next frame).
        let max = p.has(flags::MAX);
        let scale = if max && !me { MAX_SCALE } else { 1.0 };
        if max && v.max.is_none() {
            v.max = spawn_max(&mut commands, v, &models, &palette);
        }
        let suit = max && v.max.is_some();
        let prop = if suit {
            &crate::rig::MAX
        } else {
            &crate::rig::INFANTRY
        };
        let feet = if mine {
            mover.body.feet
        } else {
            prev.map_or(p.feet, |q| q.feet.lerp(p.feet, a))
        };
        let (yaw, pitch, crouch) = if mine {
            (local.cmd.yaw, local.cmd.pitch, local.cmd.crouch)
        } else {
            (
                prev.map_or(p.yaw, |q| crate::game::lerp_angle(q.yaw, p.yaw, a)),
                p.pitch,
                p.has(flags::CROUCH),
            )
        };
        // The gun in the hand; one's own, the one switched to here (testing: PS_DEV_HELD=<mesh
        // name>, e.g. striker, shows another in one's own hand).
        let weapon = match game.mine() {
            Some(m) if mine => std::env::var("PS_DEV_HELD")
                .ok()
                .and_then(|n| Weapon::ALL.iter().copied().find(|&w| weapon_mesh(w) == n))
                .unwrap_or(m.loadout.slot(local.cmd.slot)),
            _ => p.weapon,
        };
        let (head, hands) = match (&p.vr, mine.then_some(local.cmd.vr).flatten()) {
            (_, Some(vr)) => (
                Transform::from_translation(vr.head.pos).with_rotation(vr.head.rot),
                [0, 1].map(|i| {
                    Transform::from_translation(vr.hands[i].pos).with_rotation(vr.hands[i].rot)
                }),
            ),
            (Some(vr), None) => (
                Transform::from_translation(vr.head.pos).with_rotation(vr.head.rot),
                [0, 1].map(|i| {
                    Transform::from_translation(vr.hands[i].pos).with_rotation(vr.hands[i].rot)
                }),
            ),
            _ => flat_poses(yaw, pitch, crouch, weapon),
        };
        let mut hands = hands;
        // (One's own gun on a flat screen: with the camera's bob, swaying and kicking.)
        if me
            && !vr
            && let Some(sway) = sway.as_deref().filter(|s| s.on_foot)
        {
            let (shift, turn) = sway.gun_shift(yaw, pitch);
            for h in &mut hands {
                h.translation += shift;
            }
            hands[1].rotation = turn * hands[1].rotation;
        }
        // Which hand holds the gun (VR: none while it's holstered, src/holster.rs).
        let holding = match (mine.then_some(local.cmd.vr).flatten(), &p.vr) {
            (Some(v), _) => v.gun_hand.map(|h| h.index()),
            (None, Some(v)) => v.gun_hand.map(|h| h.index()),
            (None, None) => Some(1),
        };
        let gun_hand = holding.unwrap_or(1);
        set_vis(&mut vis, v.gun, holding.is_some());
        if let Ok(mut t) = transforms.get_mut(v.root) {
            t.translation = feet;
            t.scale = Vec3::splat(scale);
        }
        set_vis(&mut vis, v.body, !me && !suit);
        set_vis(&mut vis, v.head, !me);
        set_vis(&mut vis, v.face, !suit);
        for e in [v.body].into_iter().chain(v.max.as_ref().map(|m| m.torso)) {
            if let Ok(mut t) = transforms.get_mut(e) {
                t.translation = ps_sim::avatar::chest_top(head.translation, head.rotation);
                t.rotation = Quat::from_rotation_y(yaw);
            }
        }
        if let Some(m) = &v.max {
            set_vis(&mut vis, m.torso, !me && suit);
            set_vis(&mut vis, m.helmet, suit);
        }
        if let Ok(mut t) = transforms.get_mut(v.head) {
            *t = head;
        }
        // One's own gloves: in VR both (the gun hand gripping); on the flat screen the gun
        // hand gripping the gun, and on a two-handed gun the other steadying it there (so
        // both sway and kick with it).
        let steady = (me && !vr)
            .then_some(holding)
            .flatten()
            .and_then(|_| support(weapon));
        if let Some((t, _)) = steady {
            hands[1 - gun_hand] = hands[gun_hand] * t;
        }
        let off = 1 - gun_hand;
        for (i, &h) in v.hands.iter().enumerate() {
            let steadies = i == off && steady.is_some();
            let grips = holding == Some(i) || (steadies && steady.is_some_and(|s| s.1));
            set_vis(&mut vis, h, !me || vr || holding == Some(i) || steadies);
            set_vis(&mut vis, v.hand_looks[i][0], !grips && !suit);
            set_vis(&mut vis, v.hand_looks[i][1], grips && !suit);
            // (A MAX's gauntlet, but on the hand inside an arm gun's cradle.)
            if let Some(m) = &v.max {
                let cradled = holding == Some(i) && arm_gun(weapon);
                set_vis(&mut vis, m.fists[i], suit && !cradled);
            }
            if let Ok(mut t) = transforms.get_mut(h) {
                *t = hands[i];
            }
        }
        let mut place = |e: Entity, t: Transform, on: bool| {
            set_vis(&mut vis, e, on);
            if let Ok(mut tr) = transforms.get_mut(e) {
                *tr = t;
            }
        };
        let chest =
            Transform::from_translation(ps_sim::avatar::chest_top(head.translation, head.rotation))
                .with_rotation(Quat::from_rotation_y(yaw));
        // (A MAX's gaiter hangs from its helmet instead of the neck.)
        place(
            v.neck,
            crate::rig::neck(head.translation, head.rotation),
            !me && !suit,
        );
        for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
            let [upper, fore] = crate::rig::arm(prop, &chest, side, &hands[i]);
            // (A MAX with no suit to draw: its upper arms as ever.)
            place(v.arms[i][0], upper, !me && !suit);
            place(v.arms[i][1], fore, !me && !suit);
            if let Some(m) = &v.max {
                place(m.arms[i][0], upper, !me && suit);
                place(m.arms[i][1], fore, !me && suit);
                // The pauldron rides the shoulder, turning part of the way with the arm (the
                // left one is the right one turned half round).
                let turn = chest.rotation.slerp(upper.rotation, 0.35);
                let flip = if side < 0.0 {
                    Quat::from_rotation_y(std::f32::consts::PI)
                } else {
                    Quat::IDENTITY
                };
                let at = Transform::from_translation(upper.translation).with_rotation(turn * flip);
                place(m.pauldrons[i], at, !me && suit);
            }
        }
        if real.delta_secs() > 0.0 {
            let legs = v
                .gait
                .update(prop, real.delta_secs(), feet, chest.translation, yaw);
            let lower = !me || vr;
            place(v.pelvis, legs.pelvis, lower && !suit);
            for i in 0..2 {
                place(v.legs[i][0], legs.thigh[i], lower && !suit);
                place(v.legs[i][1], legs.shin[i], lower && !suit);
                place(v.legs[i][2], legs.boot[i], lower && !suit);
            }
            if let Some(m) = &v.max {
                place(m.pelvis, legs.pelvis, lower && suit);
                for i in 0..2 {
                    place(m.legs[i][0], legs.thigh[i], lower && suit);
                    place(m.legs[i][1], legs.shin[i], lower && suit);
                    place(m.legs[i][2], legs.boot[i], lower && suit);
                }
            }
            if let Some(model) = models.heads.get(v.animal).and_then(|h| h.tail.as_ref()) {
                let pelvis = Transform::from_translation(feet + legs.pelvis.translation)
                    .with_rotation(legs.pelvis.rotation);
                let poses = v.swing.update(model, real.delta_secs(), &pelvis);
                for (&e, pose) in v.tail.iter().zip(&poses) {
                    let t = Transform::from_translation(pose.translation - feet)
                        .with_rotation(pose.rotation);
                    place(e, t, (!me || vr) && !suit);
                }
            }
        }
        // The gun in the gun hand (world): its model spawned when first held, its barrels
        // turning while it fires.
        if !v.guns.iter().any(|g| g.weapon == weapon) {
            let team = Some(v.team);
            let entity = commands
                .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(v.gun)))
                .id();
            spawn_model(
                &mut commands,
                entity,
                &weapon_model(&models, weapon),
                &palette,
                team,
            );
            let spins = spawn_spins(&mut commands, entity, &models, &palette, team, weapon);
            v.guns.push(Gun {
                weapon,
                entity,
                spins,
            });
        }
        let dt = real.delta_secs();
        let want = if p.has(flags::FIRING) { SPIN_RATE } else { 0.0 };
        v.spin_rate += (want - v.spin_rate) * (1.0 - (-dt * SPIN_UP).exp());
        v.spin = (v.spin + v.spin_rate * dt).rem_euclid(std::f32::consts::TAU);
        for g in &v.guns {
            set_vis(&mut vis, g.entity, g.weapon == weapon);
            if g.weapon == weapon {
                for (k, &e) in g.spins.iter().enumerate() {
                    if let Ok(mut t) = transforms.get_mut(e) {
                        // (The two of a pair turn against each other.)
                        let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                        t.rotation = Quat::from_rotation_z(v.spin * dir);
                    }
                }
            }
        }
        if let Ok(mut t) = transforms.get_mut(v.gun) {
            let h = hands[gun_hand];
            t.translation = feet + h.translation * scale;
            t.rotation = h.rotation;
            t.scale = Vec3::splat(scale);
        }
    }
}
