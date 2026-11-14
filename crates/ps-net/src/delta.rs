//! Snapshots as deltas. The client acknowledges the newest snapshot it has (with its input);
//! the server keeps what it sent each client and sends each new snapshot as the difference to
//! the acknowledged one: only the entities that changed (as they'd go over the wire, quantized)
//! and which ones left. Far players and vehicles aren't refreshed every snapshot: within `NEAR`
//! always, then every second, beyond `MID` every third; in between they're "held" (the client
//! keeps showing the last it heard, and smooths over once the next arrives).
//!
//! Both sides keep the same record of each snapshot: a held entity is recorded as the
//! baseline had it, so a later delta against that record means the same on both sides.

use std::collections::VecDeque;

use glam::Vec3;
use ps_sim::snapshot::{
    BaseSnap, DeploySnap, Fx, MeSnap, NodeSnap, PlayerSnap, StructSnap, TurretSnap, VehicleSnap,
};
use ps_sim::{PlayerId, Snapshot, VehicleId};
use serde::{Deserialize, Serialize};

/// Snapshots kept on either side (about two seconds at 30 a second).
const KEEP: usize = 64;
/// Within this (m) everything is refreshed every snapshot.
pub const NEAR: f32 = 200.0;
/// Within this every second snapshot, beyond every third.
pub const MID: f32 = 500.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListDelta<T> {
    /// New or changed since the baseline.
    pub changed: Vec<T>,
    /// Keys of those gone since the baseline.
    pub removed: Vec<u32>,
}

impl<T> Default for ListDelta<T> {
    fn default() -> Self {
        ListDelta {
            changed: Vec::new(),
            removed: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapDelta {
    pub tick: u64,
    /// The snapshot this is relative to (0: none, all of it is here).
    pub base: u64,
    pub players: ListDelta<PlayerSnap>,
    pub held_players: Vec<PlayerId>,
    pub vehicles: ListDelta<VehicleSnap>,
    pub held_vehicles: Vec<VehicleId>,
    pub turrets: ListDelta<TurretSnap>,
    /// The bases' state (None: as in the baseline).
    pub bases: Option<Vec<BaseSnap>>,
    pub nodes: ListDelta<NodeSnap>,
    pub deployables: ListDelta<DeploySnap>,
    pub structures: ListDelta<StructSnap>,
    pub me: Option<MeSnap>,
    pub fx: Vec<Fx>,
}

trait Keyed: Clone + Serialize {
    fn key(&self) -> u32;
}

impl Keyed for PlayerSnap {
    fn key(&self) -> u32 {
        self.id as u32
    }
}

impl Keyed for VehicleSnap {
    fn key(&self) -> u32 {
        self.id as u32
    }
}

impl Keyed for TurretSnap {
    fn key(&self) -> u32 {
        (self.base as u32) << 8 | self.index as u32
    }
}

impl Keyed for NodeSnap {
    fn key(&self) -> u32 {
        self.id
    }
}

impl Keyed for StructSnap {
    fn key(&self) -> u32 {
        self.id
    }
}

impl Keyed for DeploySnap {
    fn key(&self) -> u32 {
        self.id
    }
}

/// Equal as they'd go over the wire.
fn same<T: Serialize>(a: &T, b: &T) -> bool {
    let mut x = [0u8; 512];
    let mut y = [0u8; 512];
    match (postcard::to_slice(a, &mut x), postcard::to_slice(b, &mut y)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// `now` against `base`: the delta, the record (as both sides will have it) and the held keys.
fn diff<T: Keyed>(
    now: &[T],
    base: Option<&[T]>,
    hold: impl Fn(&T) -> bool,
) -> (ListDelta<T>, Vec<T>, Vec<u32>) {
    let mut d = ListDelta::default();
    let mut record = Vec::with_capacity(now.len());
    let mut held = Vec::new();
    for e in now {
        let k = e.key();
        match base.and_then(|b| b.iter().find(|x| x.key() == k)) {
            Some(b) if same(b, e) => record.push(b.clone()),
            Some(b) if hold(e) => {
                record.push(b.clone());
                held.push(k);
            }
            _ => {
                record.push(e.clone());
                d.changed.push(e.clone());
            }
        }
    }
    if let Some(base) = base {
        d.removed = base
            .iter()
            .map(Keyed::key)
            .filter(|k| !now.iter().any(|e| e.key() == *k))
            .collect();
    }
    (d, record, held)
}

/// The baseline with the delta applied (sorted by key).
fn apply<T: Keyed>(base: Option<&[T]>, d: ListDelta<T>) -> Vec<T> {
    let mut out: Vec<T> = base
        .unwrap_or_default()
        .iter()
        .filter(|e| !d.removed.contains(&e.key()))
        .cloned()
        .collect();
    for e in d.changed {
        let k = e.key();
        match out.iter_mut().find(|x| x.key() == k) {
            Some(x) => *x = e,
            None => out.push(e),
        }
    }
    out.sort_by_key(Keyed::key);
    out
}

fn sorted<T: Keyed>(mut v: Vec<T>) -> Vec<T> {
    v.sort_by_key(Keyed::key);
    v
}

/// The server's side, one per client.
#[derive(Default)]
pub struct DeltaSender {
    sent: VecDeque<Snapshot>,
    acked: u64,
    round: u64,
}

impl DeltaSender {
    /// The client has snapshot `tick`.
    pub fn ack(&mut self, tick: u64) {
        if tick > self.acked && self.sent.iter().any(|s| s.tick == tick) {
            self.acked = tick;
            while self.sent.front().is_some_and(|s| s.tick < tick) {
                self.sent.pop_front();
            }
        }
    }

    /// `snap` (seen from `eye`) as a delta against what the client has acknowledged.
    pub fn delta(&mut self, snap: Snapshot, eye: Vec3) -> SnapDelta {
        self.round += 1;
        let round = self.round;
        let base = self.sent.front().filter(|s| s.tick == self.acked);
        let due = |at: Vec3, key: u32| {
            let d = (at - eye).length();
            if d < NEAR {
                return true;
            }
            let every = if d < MID { 2 } else { 3 };
            (round + key as u64).is_multiple_of(every)
        };
        let (players, rp, hp) = diff(
            &sorted(snap.players),
            base.map(|b| b.players.as_slice()),
            |p| !due(p.feet, p.key()),
        );
        let (vehicles, rv, hv) = diff(
            &sorted(snap.vehicles),
            base.map(|b| b.vehicles.as_slice()),
            |v| !due(v.pos, v.key()),
        );
        let (turrets, rt, _) = diff(
            &sorted(snap.turrets),
            base.map(|b| b.turrets.as_slice()),
            |_| false,
        );
        let (nodes, rn, _) = diff(
            &sorted(snap.nodes),
            base.map(|b| b.nodes.as_slice()),
            |_| false,
        );
        let (deployables, rd, _) = diff(
            &sorted(snap.deployables),
            base.map(|b| b.deployables.as_slice()),
            |_| false,
        );
        let (structures, rs, _) = diff(
            &sorted(snap.structures),
            base.map(|b| b.structures.as_slice()),
            |_| false,
        );
        let bases = match base {
            Some(b) if b.bases.len() == snap.bases.len() && same(&b.bases, &snap.bases) => None,
            _ => Some(snap.bases.clone()),
        };
        let delta = SnapDelta {
            tick: snap.tick,
            base: base.map_or(0, |b| b.tick),
            players,
            held_players: hp.into_iter().map(|k| k as PlayerId).collect(),
            vehicles,
            held_vehicles: hv.into_iter().map(|k| k as VehicleId).collect(),
            turrets,
            bases,
            nodes,
            deployables,
            structures,
            me: snap.me,
            fx: snap.fx,
        };
        self.sent.push_back(Snapshot {
            tick: snap.tick,
            players: rp,
            vehicles: rv,
            turrets: rt,
            bases: snap.bases,
            nodes: rn,
            deployables: rd,
            structures: rs,
            ..Default::default()
        });
        while self.sent.len() > KEEP {
            self.sent.pop_front();
        }
        delta
    }
}

/// The client's side.
#[derive(Default)]
pub struct DeltaReceiver {
    records: VecDeque<Snapshot>,
    shown: Option<Snapshot>,
}

impl DeltaReceiver {
    /// The newest snapshot to acknowledge (0: none).
    pub fn newest(&self) -> u64 {
        self.records.back().map_or(0, |s| s.tick)
    }

    /// The whole snapshot, or None if its baseline is gone (the server will fall back).
    pub fn apply(&mut self, d: SnapDelta) -> Option<Snapshot> {
        let base = if d.base == 0 {
            None
        } else {
            Some(self.records.iter().find(|s| s.tick == d.base)?)
        };
        let bases = match d.bases {
            Some(b) => b,
            None => base?.bases.clone(),
        };
        let record = Snapshot {
            tick: d.tick,
            players: apply(base.map(|b| b.players.as_slice()), d.players),
            vehicles: apply(base.map(|b| b.vehicles.as_slice()), d.vehicles),
            turrets: apply(base.map(|b| b.turrets.as_slice()), d.turrets),
            bases,
            nodes: apply(base.map(|b| b.nodes.as_slice()), d.nodes),
            deployables: apply(base.map(|b| b.deployables.as_slice()), d.deployables),
            structures: apply(base.map(|b| b.structures.as_slice()), d.structures),
            ..Default::default()
        };
        // Shown: held ones as last heard of.
        let mut shown = record.clone();
        if let Some(last) = &self.shown {
            for p in shown.players.iter_mut() {
                if d.held_players.contains(&p.id)
                    && let Some(l) = last.players.iter().find(|l| l.id == p.id)
                {
                    *p = l.clone();
                }
            }
            for v in shown.vehicles.iter_mut() {
                if d.held_vehicles.contains(&v.id)
                    && let Some(l) = last.vehicles.iter().find(|l| l.id == v.id)
                {
                    *v = l.clone();
                }
            }
        }
        shown.held_players = d.held_players;
        shown.held_vehicles = d.held_vehicles;
        self.shown = Some(shown.clone());
        shown.me = d.me;
        shown.fx = d.fx;
        // Later deltas are against this baseline or newer.
        let cut = d.base;
        while self.records.front().is_some_and(|s| s.tick < cut) {
            self.records.pop_front();
        }
        self.records.push_back(record);
        while self.records.len() > KEEP {
            self.records.pop_front();
        }
        Some(shown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_sim::Team;
    use ps_sim::weapons::Weapon;

    fn player(id: PlayerId, x: f32) -> PlayerSnap {
        PlayerSnap {
            id,
            team: Team::Red,
            feet: Vec3::new(x, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            flags: 0,
            weapon: Weapon::Rifle,
            health: 100.0,
            seat: None,
            vr: None,
        }
    }

    fn snap(tick: u64, players: Vec<PlayerSnap>) -> Snapshot {
        Snapshot {
            tick,
            players,
            bases: vec![BaseSnap {
                generator: 100.0,
                resources: 600,
                hacked: 0,
                hacker: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn deltas_rebuild_the_snapshots() {
        let mut tx = DeltaSender::default();
        let mut rx = DeltaReceiver::default();
        // Nothing acknowledged: all of it.
        let d = tx.delta(snap(2, vec![player(1, 0.0), player(2, 10.0)]), Vec3::ZERO);
        assert_eq!(d.base, 0);
        assert_eq!(d.players.changed.len(), 2);
        let s = rx.apply(d).unwrap();
        assert_eq!(s.players.len(), 2);
        tx.ack(rx.newest());
        // One moved, one stayed, one came, the bases are as they were.
        let d = tx.delta(
            snap(4, vec![player(1, 1.0), player(2, 10.0), player(3, 5.0)]),
            Vec3::ZERO,
        );
        assert_eq!(d.base, 2);
        assert_eq!(d.players.changed.len(), 2);
        assert!(d.bases.is_none());
        let s = rx.apply(d).unwrap();
        assert_eq!(s.players.len(), 3);
        assert!((s.players[0].feet.x - 1.0).abs() < 0.01);
        // Not acknowledged (lost): still against 2. One left.
        let d = tx.delta(snap(6, vec![player(1, 2.0), player(3, 5.0)]), Vec3::ZERO);
        assert_eq!(d.base, 2);
        assert_eq!(d.players.removed, vec![2]);
        let s = rx.apply(d).unwrap();
        assert_eq!(
            s.players.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![1, 3]
        );
        // A baseline the client never had: dropped.
        let mut rx2 = DeltaReceiver::default();
        let d = tx.delta(snap(8, vec![player(1, 3.0)]), Vec3::ZERO);
        assert!(rx2.apply(d).is_none());
    }

    #[test]
    fn far_players_are_held_between_refreshes() {
        let mut tx = DeltaSender::default();
        let mut rx = DeltaReceiver::default();
        let far = |t: u64, x: f32| snap(t, vec![player(7, 800.0 + x)]);
        let s = rx.apply(tx.delta(far(2, 0.0), Vec3::ZERO)).unwrap();
        assert!(s.held_players.is_empty());
        let mut refreshed = 0;
        let mut held = 0;
        for i in 1..=12u64 {
            tx.ack(rx.newest());
            let s = rx
                .apply(tx.delta(far(2 + i * 2, i as f32), Vec3::ZERO))
                .unwrap();
            assert_eq!(s.players.len(), 1);
            if s.held_players.contains(&7) {
                held += 1;
            } else {
                refreshed += 1;
                assert!((s.players[0].feet.x - (800.0 + i as f32)).abs() < 0.01);
            }
        }
        assert_eq!(refreshed, 4);
        assert_eq!(held, 8);
    }

    #[test]
    fn deltas_are_small() {
        let mut tx = DeltaSender::default();
        let mut rx = DeltaReceiver::default();
        let crowd = |t: u64, moving: usize| {
            snap(
                t,
                (0..40)
                    .map(|i| {
                        player(
                            i,
                            i as f32 * 3.0 + if (i as usize) < moving { t as f32 } else { 0.0 },
                        )
                    })
                    .collect(),
            )
        };
        let full = crate::protocol::encode(&tx.delta(crowd(2, 40), Vec3::ZERO));
        rx.apply(tx.delta(crowd(4, 5), Vec3::ZERO)).unwrap();
        tx.ack(4);
        let d = tx.delta(crowd(6, 5), Vec3::ZERO);
        let small = crate::protocol::encode(&d);
        assert!(
            small.len() * 3 < full.len(),
            "{} vs {}",
            small.len(),
            full.len()
        );
    }
}
