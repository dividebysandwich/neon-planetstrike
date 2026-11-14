//! Engineers' deployables and structures on screen, following `Game::snap`: spitfire turrets
//! (a stand and a head turning to its aim), anti-tank mines, ammo packs; built walls, Bastion
//! turrets (likewise a head on its bunker) and forward spawn tubes; in their faction's colours.

use std::collections::HashMap;

use bevy::prelude::*;
use ps_sim::snapshot::DeploySnap;
use ps_sim::weapons::Kit;

use crate::actors::spawn_model;
use crate::assets::Models;
use crate::game::{DrawSet, Game};
use crate::materials::Palette;

struct Shown {
    root: Entity,
    /// A spitfire's turning head.
    head: Option<Entity>,
}

#[derive(Resource, Default)]
struct Deployables(HashMap<u32, Shown>);

#[derive(Resource, Default)]
struct Structures(HashMap<u32, Shown>);

pub struct DeployPlugin;

impl Plugin for DeployPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Deployables>()
            .init_resource::<Structures>()
            .add_systems(Update, (sync, sync_structures).in_set(DrawSet));
    }
}

fn spawn(commands: &mut Commands, d: &DeploySnap, models: &Models, palette: &Palette) -> Shown {
    let team = Some(d.team);
    let root = commands
        .spawn((
            Transform::from_translation(d.pos).with_rotation(Quat::from_rotation_y(d.yaw)),
            Visibility::default(),
        ))
        .id();
    let part = |commands: &mut Commands, name: &str, parent: Entity, at: Vec3| {
        let e = commands
            .spawn((
                Transform::from_translation(at),
                Visibility::default(),
                ChildOf(parent),
            ))
            .id();
        spawn_model(commands, e, &models.get(name), palette, team);
        e
    };
    let head = match d.kind {
        Kit::Spitfire => {
            part(commands, "spitfire_base", root, Vec3::ZERO);
            Some(part(
                commands,
                "spitfire_head",
                root,
                Vec3::Y * ps_sim::sim::kit::SPITFIRE_PIVOT,
            ))
        }
        Kit::Mines => {
            part(commands, "at_mine", root, Vec3::ZERO);
            None
        }
        Kit::AmmoPack | Kit::None => {
            part(commands, "ammo_pack", root, Vec3::ZERO);
            None
        }
    };
    Shown { root, head }
}

fn sync(
    mut commands: Commands,
    game: Option<Res<Game>>,
    models: Res<Models>,
    palette: Res<Palette>,
    mut shown: ResMut<Deployables>,
    mut transforms: Query<&mut Transform>,
) {
    let list: &[DeploySnap] = game.as_deref().map_or(&[], |g| &g.snap.deployables);
    shown.0.retain(|id, s| {
        let keep = list.iter().any(|d| d.id == *id);
        if !keep {
            commands.entity(s.root).despawn();
        }
        keep
    });
    for d in list {
        if !shown.0.contains_key(&d.id) {
            debug!("deployable {} {:?} at {}", d.id, d.kind, d.pos);
        }
        let s = shown
            .0
            .entry(d.id)
            .or_insert_with(|| spawn(&mut commands, d, &models, &palette));
        // (The head turns to its aim in the world; the stand stays as it was put down.)
        if let Some(head) = s.head
            && let Ok(root) = transforms.get(s.root).map(|t| t.rotation)
            && let Ok(mut t) = transforms.get_mut(head)
        {
            t.rotation =
                root.inverse() * Quat::from_rotation_y(d.yaw) * Quat::from_rotation_x(d.pitch);
        }
    }
}

fn spawn_structure(
    commands: &mut Commands,
    st: &ps_sim::snapshot::StructSnap,
    models: &Models,
    palette: &Palette,
) -> Shown {
    use ps_sim::construct::{BASTION_PIVOT, StructureKind};
    let team = Some(st.team);
    let root = commands
        .spawn((
            Transform::from_translation(st.pos).with_rotation(Quat::from_rotation_y(st.yaw)),
            Visibility::default(),
        ))
        .id();
    let part = |commands: &mut Commands, name: &str, at: Vec3| {
        let e = commands
            .spawn((
                Transform::from_translation(at),
                Visibility::default(),
                ChildOf(root),
            ))
            .id();
        spawn_model(commands, e, &models.get(name), palette, team);
        e
    };
    let head = match st.kind {
        // (A silo's "head": its fill, scaled to how full it is.)
        StructureKind::Silo => {
            part(commands, "mobile_silo", Vec3::ZERO);
            Some(part(commands, "mobile_silo_fill", Vec3::Y * SILO_FILL_AT))
        }
        StructureKind::Wall => {
            part(commands, "build_wall", Vec3::ZERO);
            None
        }
        StructureKind::Bastion => {
            part(commands, "bastion_base", Vec3::ZERO);
            Some(part(commands, "bastion_head", Vec3::Y * BASTION_PIVOT))
        }
        StructureKind::Tube => {
            part(commands, "forward_tube", Vec3::ZERO);
            None
        }
    };
    Shown { root, head }
}

fn sync_structures(
    mut commands: Commands,
    game: Option<Res<Game>>,
    models: Res<Models>,
    palette: Res<Palette>,
    mut shown: ResMut<Structures>,
    mut transforms: Query<&mut Transform>,
) {
    let list: &[ps_sim::snapshot::StructSnap] = game.as_deref().map_or(&[], |g| &g.snap.structures);
    shown.0.retain(|id, s| {
        let keep = list.iter().any(|d| d.id == *id);
        if !keep {
            commands.entity(s.root).despawn();
        }
        keep
    });
    for st in list {
        let s = shown
            .0
            .entry(st.id)
            .or_insert_with(|| spawn_structure(&mut commands, st, &models, &palette));
        if let Some(head) = s.head
            && let Ok(mut t) = transforms.get_mut(head)
        {
            if st.kind == ps_sim::construct::StructureKind::Silo {
                // The fill in the window: as tall as the silo is full (nothing when empty).
                let k = st.fill as f32 / 100.0;
                t.scale = Vec3::new(1.0, (SILO_FILL_H * k).max(1e-4), 1.0);
                t.translation.y = if k > 0.0 { SILO_FILL_AT } else { -10.0 };
            } else {
                t.rotation = Quat::from_rotation_y(st.aim_yaw - st.yaw)
                    * Quat::from_rotation_x(st.aim_pitch);
            }
        }
    }
}

/// A mobile silo's fill window: its bottom over the ground, and its height (m).
const SILO_FILL_AT: f32 = 1.405;
const SILO_FILL_H: f32 = 1.895;
