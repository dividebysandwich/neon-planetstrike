//! VR's command wheel: Y held (past a tap, which spots) brings up a ring of commands round the
//! left hand; the left stick picks one (walking and driving wait meanwhile) and letting go of Y
//! does it (the stick centred: nothing). What's on it depends on what one is doing: the squad
//! leader's waypoint (where one looks), switching seats in a vehicle, the map, the scoreboard,
//! the squad panel, placing a structure carried, the cloak (an infiltrator on foot), and callouts to
//! the faction (a medic, repairs, a ride, attack or defend here: said with the nearest base's
//! name).

use bevy::prelude::*;
use ps_net::protocol::SquadCmd;

use crate::game::{Game, LocalCmd};
use crate::vr::{VrState, XrInput, XrRig};
use crate::vrui::{Mats, VrUi};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    Waypoint,
    SwitchSeat,
    Map,
    Scores,
    Squad,
    /// Put down the structure carried (6 m ahead, facing one).
    Place,
    Cloak,
    Call(Call),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Call {
    Medic,
    Repair,
    Ride,
    Attack,
    Defend,
}

impl Call {
    fn text(self) -> &'static str {
        match self {
            Call::Medic => "Need a medic",
            Call::Repair => "Need repairs",
            Call::Ride => "Need a ride",
            Call::Attack => "Attack",
            Call::Defend => "Defend",
        }
    }
}

impl Slot {
    pub fn label(self) -> &'static str {
        match self {
            Slot::Waypoint => "WAYPOINT",
            Slot::SwitchSeat => "SWITCH SEAT",
            Slot::Map => "MAP",
            Slot::Scores => "SCORES",
            Slot::Squad => "SQUAD",
            Slot::Place => "PLACE",
            Slot::Cloak => "CLOAK",
            Slot::Call(Call::Medic) => "MEDIC!",
            Slot::Call(Call::Repair) => "REPAIRS!",
            Slot::Call(Call::Ride) => "RIDE!",
            Slot::Call(Call::Attack) => "ATTACK HERE",
            Slot::Call(Call::Defend) => "DEFEND HERE",
        }
    }
}

/// What's on the wheel now (from the top, clockwise).
pub fn slots(game: &Game) -> Vec<Slot> {
    let mut out = Vec::new();
    if game.leading() {
        out.push(Slot::Waypoint);
    }
    if let Some((v, s)) = game.seat()
        && game
            .vehicle(v)
            .is_some_and(|v| (0..v.seats.len()).any(|k| k != s as usize && v.seats[k].is_none()))
    {
        out.push(Slot::SwitchSeat);
    }
    out.extend([Slot::Map, Slot::Scores]);
    if crate::build::carrying(game).is_some() {
        out.push(Slot::Place);
    }
    if game.seat().is_none()
        && game
            .mine()
            .is_some_and(|m| m.loadout.armor == ps_sim::weapons::ArmorKind::Stealth)
    {
        out.push(Slot::Cloak);
    }
    out.push(Slot::Squad);
    out.extend(
        [
            Call::Medic,
            Call::Repair,
            Call::Ride,
            Call::Attack,
            Call::Defend,
        ]
        .map(Slot::Call),
    );
    out
}

/// The slot the stick points at (none near the middle).
pub fn picked(stick: Vec2, n: usize) -> Option<usize> {
    if stick.length() < 0.5 || n == 0 {
        return None;
    }
    // (Clockwise from the top.)
    let a = stick.x.atan2(stick.y).rem_euclid(std::f32::consts::TAU);
    let step = std::f32::consts::TAU / n as f32;
    Some(((a / step).round() as usize) % n)
}

/// Where a slot sits on the ring (its unit direction, x right, y up).
fn slot_dir(i: usize, n: usize) -> Vec2 {
    let a = i as f32 / n as f32 * std::f32::consts::TAU;
    Vec2::new(a.sin(), a.cos())
}

/// Letting go of Y on the wheel: the slot picked is done.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pick(
    mut vr: ResMut<VrState>,
    mut ui: ResMut<VrUi>,
    game: Option<ResMut<Game>>,
    mut local: ResMut<LocalCmd>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    globals: Query<&GlobalTransform>,
) {
    let Some(stick) = vr.wheel_pick.take() else {
        return;
    };
    let Some(mut game) = game else { return };
    let list = slots(&game);
    let Some(slot) = picked(stick, list.len()).map(|i| list[i]) else {
        return;
    };
    match slot {
        Slot::Map => ui.map = true,
        Slot::Scores => ui.scores = true,
        Slot::Squad => ui.squad = true,
        Slot::Place => {
            if let (Some(kind), Some(me)) = (crate::build::carrying(&game), game.me_snap()) {
                // 6 m ahead of where one faces, turned to face one.
                let yaw = me.yaw;
                let ahead = me.feet + ps_sim::look_dir(yaw, 0.0) * 6.0;
                let y = game
                    .world
                    .ground(ahead.x, ahead.z, ahead.y + 3.0)
                    .unwrap_or(ahead.y);
                game.request(ps_sim::Request::Build {
                    kind,
                    at: Vec3::new(ahead.x, y, ahead.z),
                    yaw: yaw + std::f32::consts::PI,
                });
            }
        }
        Slot::Cloak => local.cmd.deploys += 1,
        Slot::SwitchSeat => {
            if let Some((vid, s)) = game.seat()
                && let Some(v) = game.vehicle(vid)
            {
                let n = v.seats.len();
                if let Some(next) = (1..n)
                    .map(|k| (s as usize + k) % n)
                    .find(|&k| v.seats[k].is_none())
                {
                    local.cmd.seat = next as u8;
                    local.cmd.seat_switches += 1;
                }
            }
        }
        Slot::Waypoint => {
            // Where the head looks, out to the land or a wall (the sky: none).
            let (Some(head), Ok(rig_t), Some(c)) = (input.head, globals.get(rig.0), game.continent)
            else {
                return;
            };
            let head = rig_t.compute_transform() * head;
            let from = head.translation;
            let hit = game
                .world
                .ray(from, from + head.forward() * 2000.0)
                .map(|h| h.1);
            game.squad(SquadCmd::Waypoint(hit.map(|at| (c, at))));
        }
        Slot::Call(call) => {
            let near = game.mine().and_then(|m| {
                let map = game.map.as_ref()?;
                map.bases
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        (a.1.center - m.feet)
                            .length()
                            .total_cmp(&(b.1.center - m.feet).length())
                    })
                    .map(|(b, _)| map.def.bases[b].name.clone())
            });
            let text = match near {
                Some(n) => format!("{} near {n}", call.text()),
                None => call.text().to_string(),
            };
            game.client.send_team_chat(&text);
        }
    }
}

/// The wheel's look: built as it opens and when the pick changes.
#[derive(Default)]
pub struct Shown {
    root: Option<Entity>,
    key: Option<(Vec<Slot>, Option<usize>)>,
}

/// The ring of commands round the left hand, facing the eyes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    mut commands: Commands,
    mut shown: Local<Shown>,
    vr: Res<VrState>,
    input: Res<XrInput>,
    rig: Res<XrRig>,
    mats: Option<Res<Mats>>,
    game: Option<Res<Game>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
) {
    let (Some(mats), Some(game)) = (mats, game) else {
        return;
    };
    let open = vr.wheel_open() && game.alive();
    let key = open.then(|| {
        let list = slots(&game);
        let pick = picked(vr.wheel_stick, list.len());
        (list, pick)
    });
    if key != shown.key {
        if let Some(r) = shown.root.take() {
            commands.entity(r).despawn();
        }
        if let Some((list, pick)) = &key {
            let root = commands
                .spawn((Transform::default(), Visibility::default(), ChildOf(rig.0)))
                .id();
            let n = list.len();
            let (h, w) = (0.0085, 0.085);
            // (Room round the ring for every slot.)
            let radius = (n as f32 * (w + 0.008) / std::f32::consts::TAU).max(0.1);
            for (i, s) in list.iter().enumerate() {
                let at = slot_dir(i, n) * radius;
                let lit = *pick == Some(i);
                commands.spawn((
                    Mesh3d(mats.quad.clone()),
                    MeshMaterial3d(if lit {
                        mats.lit.clone()
                    } else {
                        mats.button.clone()
                    }),
                    Transform::from_xyz(at.x, at.y, -0.001).with_scale(Vec3::new(w, 0.022, 1.0)),
                    ChildOf(root),
                ));
                let label = s.label();
                let tw = crate::vfont::width(label, h, 600);
                let mesh = meshes.add(crate::vfont::mesh(label, h, 600).to_mesh());
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(if lit {
                        mats.text.clone()
                    } else {
                        mats.dim.clone()
                    }),
                    Transform::from_xyz(at.x - tw / 2.0, at.y - h / 2.0, 0.001),
                    ChildOf(root),
                ));
            }
            // The middle: what letting go does.
            let mid = pick.map_or("(cancel)", |i| list[i].label());
            let tw = crate::vfont::width(mid, h, 600);
            let mesh = meshes.add(crate::vfont::mesh(mid, h, 600).to_mesh());
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mats.text.clone()),
                Transform::from_xyz(-tw / 2.0, -h / 2.0, 0.001),
                ChildOf(root),
            ));
            shown.root = Some(root);
        }
        shown.key = key;
    }
    // Over the left hand, facing the eyes (in the tracking space).
    if let (Some(root), Some(g), Some(head)) = (shown.root, input.hands[0].grip, input.head) {
        let at = g.translation + Vec3::Y * 0.12;
        let away = at + (at - head.translation);
        let t = Transform::from_translation(at).looking_at(away, Vec3::Y);
        if let Ok(mut rt) = transforms.get_mut(root) {
            *rt = t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stick_picks_round_the_ring_clockwise_from_the_top() {
        assert_eq!(picked(Vec2::new(0.0, 0.2), 8), None);
        assert_eq!(picked(Vec2::new(0.0, 1.0), 8), Some(0));
        assert_eq!(picked(Vec2::new(1.0, 0.0), 8), Some(2));
        assert_eq!(picked(Vec2::new(0.0, -1.0), 8), Some(4));
        assert_eq!(picked(Vec2::new(-1.0, 0.0), 8), Some(6));
        // Just left of the top: still the top one.
        assert_eq!(picked(Vec2::new(-0.1, 1.0), 8), Some(0));
        for i in 0..7 {
            assert_eq!(picked(slot_dir(i, 7), 7), Some(i));
        }
    }
}
