//! VR: the weapons holstered on the body (src/vr.rs draws from them), each with a faint ring
//! that brightens while a hand is in reach; and the view's edges glowing red when hurt (and
//! pulsing at low health), held in front of the eyes.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use ps_sim::weapons::Weapon;
use ps_sim::{SimEvent, Team};

use crate::actors::{spawn_model, weapon_model};
use crate::assets::Models;
use crate::game::{Game, GameEvent};
use crate::materials::{AccentMaterial, GlowMaterial, NeonUniform, Palette, team_color};
use crate::vr::{VrState, XrInput, XrRig};

pub struct HolsterPlugin;

impl Plugin for HolsterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_holsters.run_if(
                resource_exists::<Palette>
                    .and_then(resource_exists::<Models>)
                    .and_then(not(resource_exists::<Holsters>)),
            ),
        )
        .add_systems(
            Update,
            (holsters, pain)
                .after(crate::vr::place_rig)
                .before(crate::game::DrawSet)
                .run_if(resource_exists::<Game>.and_then(resource_exists::<Holsters>)),
        );
    }
}

/// One holster: its gun (rebuilt when the loadout changes) and ring.
struct Slot {
    root: Entity,
    gun: Entity,
    ring: Entity,
    shows: Option<(Weapon, Team)>,
}

#[derive(Resource)]
struct Holsters {
    slots: [Slot; 3],
    dim: Handle<AccentMaterial>,
    bright: Handle<AccentMaterial>,
    team: Option<Team>,
    /// The red glow round the view, its material, and how hurt (0..1 as of the time).
    pain: Entity,
    pain_mat: Handle<GlowMaterial>,
    hurt: (f32, f32),
}

fn spawn_holsters(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
) {
    let dim = accents.add(AccentMaterial {
        overlay: false,
        neon: NeonUniform::fixed(team_color(None), 0.4),
    });
    let bright = accents.add(AccentMaterial {
        overlay: false,
        neon: NeonUniform::fixed(team_color(None), 2.5),
    });
    let torus = meshes.add(Torus::new(0.062, 0.07));
    let slots = std::array::from_fn(|n| {
        let root = commands
            .spawn((
                Name::new(format!("holster {n}")),
                Transform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        let gun = commands
            .spawn((Transform::IDENTITY, Visibility::default(), ChildOf(root)))
            .id();
        let ring = commands
            .spawn((
                Mesh3d(torus.clone()),
                MeshMaterial3d(dim.clone()),
                Transform::IDENTITY,
                Visibility::Hidden,
            ))
            .id();
        Slot {
            root,
            gun,
            ring,
            shows: None,
        }
    });
    let pain_mat = glows.add(GlowMaterial {
        neon: NeonUniform::fixed(Vec3::new(1.0, 0.03, 0.05), 0.0),
        overlay: false,
    });
    let pain = commands
        .spawn((
            Name::new("pain"),
            Mesh3d(meshes.add(annulus(0.065, 0.4, 48))),
            MeshMaterial3d(pain_mat.clone()),
            Transform::IDENTITY,
            Visibility::Hidden,
        ))
        .id();
    commands.insert_resource(Holsters {
        slots,
        dim,
        bright,
        team: None,
        pain,
        pain_mat,
        hurt: (0.0, 0.0),
    });
}

/// A flat ring facing +Z (in the XY plane) from `inner` to `outer`: a halo strip, glowing at
/// its outer edge (uv.y 0) and fading towards its inner (uv.y 1).
fn annulus(inner: f32, outer: f32, segments: usize) -> Mesh {
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    for i in 0..=segments {
        let a = i as f32 / segments as f32 * TAU;
        let d = Vec3::new(a.cos(), a.sin(), 0.0);
        pos.push((d * outer).to_array());
        pos.push((d * inner).to_array());
        let u = i as f32 / segments as f32;
        uv.push([u, 0.0]);
        uv.push([u, 1.0]);
    }
    let mut idx = Vec::new();
    for i in 0..segments as u32 {
        let (o0, i0, o1, i1) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        idx.extend_from_slice(&[o0, o1, i0, i0, o1, i1]);
    }
    let n = pos.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(idx))
}

fn set_vis(vis: &mut Query<&mut Visibility>, e: Entity, on: bool) {
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
}

#[allow(clippy::too_many_arguments)]
fn holsters(
    mut commands: Commands,
    mut h: ResMut<Holsters>,
    vr: Res<VrState>,
    game: Res<Game>,
    models: Res<Models>,
    palette: Res<Palette>,
    mut accents: ResMut<Assets<AccentMaterial>>,
    mut transforms: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
    mut mats: Query<&mut MeshMaterial3d<AccentMaterial>>,
) {
    let h = &mut *h;
    let me = game.mine();
    let team = me.map(|_| game.team);
    if team != h.team {
        h.team = team;
        for (m, k) in [(&h.dim, 0.4), (&h.bright, 2.5)] {
            if let Some(mut m) = accents.get_mut(m) {
                m.neon = NeonUniform::fixed(team_color(team), k);
            }
        }
    }
    let on_foot = game.alive() && game.seat().is_none() && me.is_some_and(|m| m.turret.is_none());
    for (n, slot) in h.slots.iter_mut().enumerate() {
        // The loadout's weapon for this slot, in the team's colours.
        let shows = me.map(|m| (m.loadout.slot(n as u8), game.team));
        if shows != slot.shows {
            slot.shows = shows;
            commands.entity(slot.gun).despawn_children();
            if let Some((w, t)) = shows {
                spawn_model(
                    &mut commands,
                    slot.gun,
                    &weapon_model(&models, w),
                    &palette,
                    Some(t),
                );
            }
        }
        let at = vr.holsters_world[n];
        let drawn = vr.drawn.is_some_and(|(_, s)| s as usize == n);
        let show = on_foot && at.is_some() && shows.is_some() && !drawn;
        set_vis(&mut vis, slot.root, show);
        set_vis(&mut vis, slot.ring, show);
        let Some(at) = at else { continue };
        if let Ok(mut t) = transforms.get_mut(slot.root) {
            *t = at;
        }
        if let Ok(mut t) = transforms.get_mut(slot.ring) {
            // (Level with the floor, round the grip.)
            *t = Transform::from_translation(at.translation);
        }
        let near = vr.near.contains(&Some(n as u8));
        if let Ok(mut m) = mats.get_mut(slot.ring) {
            let want = if near { &h.bright } else { &h.dim };
            if m.0 != *want {
                m.0 = want.clone();
            }
        }
    }
}

/// The red glow round the view: flaring with each hit taken and wearing off, and pulsing
/// slowly at low health.
#[allow(clippy::too_many_arguments)]
fn pain(
    mut h: ResMut<Holsters>,
    mut events: MessageReader<GameEvent>,
    game: Res<Game>,
    time: Res<Time>,
    input: Res<XrInput>,
    vr: Res<VrState>,
    rig: Option<Res<XrRig>>,
    mut glows: ResMut<Assets<GlowMaterial>>,
    mut transforms: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
) {
    let now = time.elapsed_secs();
    let fade = |(p, at): (f32, f32)| p * (-(now - at).max(0.0) / 0.5).exp();
    for GameEvent(e) in events.read() {
        if let SimEvent::Hurt { victim, amount, .. } = e
            && Some(*victim) == game.me
        {
            h.hurt = ((fade(h.hurt) + amount / 60.0).min(0.8), now);
        }
    }
    let alive = game.alive();
    let health = game.mine().map_or(100.0, |m| m.health);
    let low = if alive && health < 30.0 {
        let k = (30.0 - health) / 30.0;
        (0.12 + 0.2 * k) * (0.6 + 0.4 * (now * TAU).sin())
    } else {
        0.0
    };
    let p = if alive { fade(h.hurt).max(low) } else { 0.0 };
    let show = p > 0.005 && rig.is_some() && input.head.is_some();
    set_vis(&mut vis, h.pain, show);
    if !show {
        return;
    }
    if let Some(mut m) = glows.get_mut(&h.pain_mat) {
        m.neon.color.w = p * 5.0;
    }
    // A hand's breadth in front of the eyes, turning with the head.
    let head = vr.rig * input.head.unwrap_or_default();
    if let Ok(mut t) = transforms.get_mut(h.pain) {
        *t = head * Transform::from_xyz(0.0, 0.0, -0.12);
    }
}
