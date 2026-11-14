//! What the player sees from a seat: a vehicle's or a wall turret's. Inside, the view is the
//! gun's: a tank's (and the heavy tank gunner's, a wall turret's) turns and tilts with its
//! barrel, the harvester's, the bus's and the VTOL's look ahead along the hull. Nothing of the
//! vehicle or turret itself is drawn from inside (src/vehicles.rs, src/bases.rs): the HUD
//! shows what matters.
//!
//! The gun's aim is predicted here as the sim turns it (towards the look, at its traverse
//! rate), so the view follows the mouse or stick without a round trip; it's put right when
//! the snapshots disagree. The desktop camera (src/desktop.rs), the VR rig (src/vr.rs) and the
//! HUD's aim marker read `SeatView`.

use bevy::prelude::*;
use ps_sim::base::{TURRET_EYE, TURRET_PIVOT};
use ps_sim::map::ContinentMap;
use ps_sim::sim::{TURRET_PITCH, TURRET_TURN};
use ps_sim::vehicle::{MotionKind, VehicleKind};
use ps_sim::{VehicleId, look_dir, wrap_angle, yaw_of};

use crate::game::{Game, LocalCmd, Mover, PredictSet};

/// The seat whose gun is predicted.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Vehicle(VehicleId, u8),
    Turret(u16),
}

#[derive(Resource, Default, Debug)]
pub struct SeatView {
    /// In a vehicle's seat or a turret's.
    pub active: bool,
    /// The eyes (world) and where the view looks: along the barrel or the hull.
    pub eye: Vec3,
    pub rot: Quat,
    /// The view turned only about the vertical (for the VR rig), except the VTOL pilot's,
    /// which banks and pitches with the craft.
    pub level: Quat,
    /// The barrel's pivot (world), and the direction the gunner wants it to point (the seat
    /// has a gun one aims: not the VTOL's fixed pods, nor the harvester's beam).
    pub pivot: Vec3,
    pub want: Option<Vec3>,
    /// What not to draw (seen from inside): the vehicle or the turret.
    pub vehicle: Option<VehicleId>,
    pub turret: Option<u16>,
    /// A harvester: what it carries (0..1), and its beam is on.
    pub cargo: Option<f32>,
    /// The predicted gun: its seat, yaw and pitch (relative to the hull; a turret's: world).
    gun: Option<(Key, f32, f32)>,
}

pub struct SeatViewPlugin;

impl Plugin for SeatViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SeatView>().add_systems(
            Update,
            update
                .in_set(SeatViewSet)
                .after(PredictSet)
                .run_if(resource_exists::<Game>),
        );
    }
}

/// `SeatView` is up to date after this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeatViewSet;

/// Turret `t`'s mount (its index on the continent: the bases' turrets in order).
pub fn turret_mount(map: &ContinentMap, t: usize) -> Option<Vec3> {
    let mut k = t;
    for l in &map.bases {
        if k < l.turrets.len() {
            return Some(l.turrets[k]);
        }
        k -= l.turrets.len();
    }
    None
}

/// Whether the player sees the seat from inside (VR always; the flat screen unless the
/// camera is behind): then the vehicle or turret isn't drawn.
pub fn from_inside(view: &SeatView, look: Option<&crate::desktop::Look>) -> bool {
    view.active && look.is_none_or(|l| !l.third_person)
}

/// Steps `from` towards `to` (radians) by at most `step`.
fn approach(from: f32, to: f32, step: f32) -> f32 {
    from + (to - from).clamp(-step, step)
}

fn update(
    game: Res<Game>,
    mover: Res<Mover>,
    local: Res<LocalCmd>,
    time: Res<Time>,
    mut view: ResMut<SeatView>,
) {
    let dt = time.delta_secs();
    let gun = view.gun;
    *view = SeatView::default();
    let Some(me) = game.mine().filter(|m| m.dead.is_none()) else {
        return;
    };
    let look = look_dir(local.cmd.yaw, local.cmd.pitch);
    // A wall turret.
    if let Some(t) = me.turret
        && let Some(mount) = game.map.as_ref().and_then(|m| turret_mount(m, t as usize))
    {
        let snap = game.snap.turrets.iter().find(|s| s.occupant == game.me);
        let (mut yaw, mut pitch) = match gun {
            Some((Key::Turret(k), y, p)) if k == t => (y, p),
            _ => snap.map_or((local.cmd.yaw, 0.0), |s| (s.yaw, s.pitch)),
        };
        if let Some(s) = snap
            && (wrap_angle(s.yaw - yaw).abs() > 0.4 || (s.pitch - pitch).abs() > 0.4)
        {
            (yaw, pitch) = (s.yaw, s.pitch);
        }
        let step = TURRET_TURN * dt;
        yaw = wrap_angle(yaw + wrap_angle(local.cmd.yaw - yaw).clamp(-step, step));
        let want_pitch = local.cmd.pitch.clamp(TURRET_PITCH.0, TURRET_PITCH.1);
        pitch = approach(pitch, want_pitch, step);
        let pivot = mount + Vec3::Y * TURRET_PIVOT;
        let turn = Quat::from_rotation_y(yaw);
        view.active = true;
        view.eye = pivot + turn * TURRET_EYE;
        view.rot = turn * Quat::from_rotation_x(pitch);
        view.level = turn;
        view.pivot = pivot;
        view.want = Some(look);
        view.turret = Some(t);
        view.gun = Some((Key::Turret(t), yaw, pitch));
        return;
    }
    let Some((vid, seat)) = me.seat else { return };
    let Some(v) = game.vehicle(vid) else { return };
    let spec = v.kind.spec();
    let Some(seat_spec) = spec.seats.get(seat as usize) else {
        return;
    };
    let (pos, vrot) = match &mover.drive {
        Some((id, _, m)) if *id == vid => (m.pos, m.rot),
        _ => (v.pos, v.rot),
    };
    let hull_yaw = Quat::from_rotation_y(yaw_of(vrot * Vec3::NEG_Z));
    view.active = true;
    view.vehicle = Some(vid);
    view.eye = pos + vrot * seat_spec.eye;
    view.rot = vrot;
    view.level = if spec.motion == MotionKind::Flying && seat == 0 {
        vrot
    } else {
        hull_yaw
    };
    view.pivot = view.eye;
    if v.kind == VehicleKind::Harvester {
        view.cargo = Some(v.cargo as f32 / ps_sim::sim::CARGO);
    }
    // A gun one aims (not the VTOL's fixed pods, nor the harvester's beam): the view is the
    // barrel's.
    let Some(gi) = seat_spec.gun else { return };
    let gs = v.kind.gun_spec(v.loadout, gi);
    if gs.fixed || v.kind == VehicleKind::Harvester {
        return;
    }
    let key = Key::Vehicle(vid, seat);
    let snap = v.aim.get(gi).map(|a| (a.0 as f32 / 1e4, a.1 as f32 / 1e4));
    let (mut yaw, mut pitch) = match gun {
        Some((k, y, p)) if k == key => (y, p),
        _ => snap.unwrap_or((0.0, 0.0)),
    };
    if let Some(s) = snap
        && (wrap_angle(s.0 - yaw).abs() > 0.4 || (s.1 - pitch).abs() > 0.4)
    {
        (yaw, pitch) = s;
    }
    // As the sim turns it: towards the look, relative to the hull, at the traverse rate.
    let local_look = vrot.inverse() * look;
    let step = gs.turn_rate * dt;
    yaw = wrap_angle(yaw + wrap_angle(yaw_of(local_look) - yaw).clamp(-step, step));
    let want_pitch = local_look.y.clamp(-1.0, 1.0).asin();
    pitch = approach(pitch, want_pitch.clamp(gs.pitch_min, gs.pitch_max), step);
    let turn = Quat::from_rotation_y(yaw);
    let aim = turn * Quat::from_rotation_x(pitch);
    // The eyes turn with the turret about its pivot.
    view.pivot = pos + vrot * gs.pivot;
    view.eye = pos + vrot * (gs.pivot + turn * (seat_spec.eye - gs.pivot));
    view.rot = vrot * aim;
    view.level = Quat::from_rotation_y(yaw_of(view.rot * Vec3::NEG_Z));
    view.want = Some(look);
    view.gun = Some((key, yaw, pitch));
}
