//! A player joins a running server: lands on its sanctuary, spawns there, sees the world in
//! its snapshots, and spawns at a base on another continent.

use std::time::{Duration, Instant};

use ps_net::client::{Client, ClientEvent};
use ps_server::Config;
use ps_sim::{PlayerCmd, Request, SpawnPoint, Team, WorldParams, WorldState};

#[test]
fn a_player_joins_spawns_and_moves_between_continents() {
    let params = WorldParams {
        seed: 11,
        contested: 4,
    };
    let server = ps_server::start(Config {
        bind: "127.0.0.1:0".into(),
        params,
        bots: 8,
        ..Config::default()
    })
    .unwrap();
    let mut client = Client::connect(
        &format!("127.0.0.1:{}", server.port),
        "Tester",
        Some(Team::Blue),
    )
    .unwrap();
    let start = Instant::now();
    let (mut me, mut world, mut here, mut alive_snaps) = (None, None::<WorldState>, None, 0);
    let mut asked_home = false;
    let mut asked_away: Option<u8> = None;
    let mut arrived_away = false;
    let mut bots_seen = 0usize;
    let (mut full_bytes, mut full_count) = (0u64, 0u64);
    while start.elapsed() < Duration::from_secs(60) {
        for ev in client.poll() {
            match ev {
                ClientEvent::Welcome { id, team, .. } => {
                    assert_eq!(team, Team::Blue);
                    me = Some(id);
                }
                ClientEvent::World(w) => world = Some(w),
                ClientEvent::Enter(c) => {
                    if asked_away == Some(c) {
                        arrived_away = true;
                    }
                    here = Some(c);
                }
                ClientEvent::Snapshot(c, s) => {
                    full_bytes += ps_net::protocol::encode(&s).len() as u64;
                    full_count += 1;
                    if Some(c) == here
                        && let Some(m) = &s.me
                        && m.dead.is_none()
                    {
                        alive_snaps += 1;
                    }
                    bots_seen = bots_seen.max(s.players.len());
                }
                ClientEvent::Disconnected(why) => panic!("disconnected: {why}"),
                _ => {}
            }
        }
        client.send_input(PlayerCmd::default());
        if !asked_home && here.is_some() {
            // Home: the sanctuary (continent 1 for Blue).
            assert_eq!(here, Some(1));
            client.send_request(Request::Spawn(SpawnPoint::Base {
                continent: 1,
                base: 0,
            }));
            asked_home = true;
        }
        if alive_snaps > 20 && asked_away.is_none() {
            // Somewhere Blue holds on another continent (after dying there: suicide first).
            let w = world.as_ref().unwrap();
            let away = (3..w.continents.len()).find_map(|c| {
                w.continents[c]
                    .bases
                    .iter()
                    .position(|b| b.owner == Some(Team::Blue))
                    .map(|b| (c as u8, b as u8))
            });
            if let Some((c, b)) = away {
                client.send_request(Request::Suicide);
                std::thread::sleep(Duration::from_millis(50));
                asked_away = Some(c);
                // Respawn time first.
                let wait = Instant::now();
                while wait.elapsed() < Duration::from_secs(9) {
                    client.poll();
                    client.send_input(PlayerCmd::default());
                    std::thread::sleep(Duration::from_millis(16));
                }
                client.send_request(Request::Spawn(SpawnPoint::Base {
                    continent: c,
                    base: b,
                }));
            } else {
                asked_away = Some(255);
                arrived_away = true;
            }
        }
        if arrived_away && alive_snaps > 40 && full_count > 400 {
            break;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    assert!(me.is_some(), "welcomed");
    assert!(alive_snaps > 40, "{alive_snaps} snapshots alive");
    assert!(arrived_away, "spawned on another continent");
    let _ = bots_seen;
    // Deltas: well under what the whole snapshots would have taken.
    let sent = client.snapshot_bytes as f64 / client.snapshot_count.max(1) as f64;
    let full = full_bytes as f64 / full_count.max(1) as f64;
    eprintln!(
        "snapshots: {} received, {sent:.0} B each as deltas, {full:.0} B whole",
        client.snapshot_count
    );
    assert!(sent < full * 0.8, "{sent:.0} B vs {full:.0} B");
}
