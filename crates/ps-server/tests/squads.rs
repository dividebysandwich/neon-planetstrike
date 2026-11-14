//! Two players of a faction make a squad; the leader spawns on a contested continent, and the
//! other spawns on the leader from its sanctuary (the server moves it there). Squad chat goes
//! to the squad.

use std::time::{Duration, Instant};

use glam::Vec3;
use ps_net::client::{Client, ClientEvent};
use ps_net::protocol::{SquadCmd, SquadInfo};
use ps_server::Config;
use ps_sim::{
    ContinentId, PlayerCmd, PlayerId, Request, SpawnPoint, Team, WorldParams, WorldState,
};

#[derive(Default)]
struct Seen {
    id: Option<PlayerId>,
    world: Option<WorldState>,
    here: Option<ContinentId>,
    /// Alive at these feet, on `here`.
    feet: Option<Vec3>,
    squads: Vec<SquadInfo>,
    squad_chat: Vec<String>,
    xp: Option<u64>,
}

fn pump(c: &mut Client, s: &mut Seen) {
    for ev in c.poll() {
        match ev {
            ClientEvent::Welcome { id, .. } => s.id = Some(id),
            ClientEvent::World(w) => s.world = Some(w),
            ClientEvent::Enter(c) => {
                s.here = Some(c);
                s.feet = None;
            }
            ClientEvent::Snapshot(c, snap) => {
                if Some(c) == s.here {
                    s.feet = snap.me.filter(|m| m.dead.is_none()).map(|m| m.feet);
                }
            }
            ClientEvent::Squads(q) => s.squads = q,
            ClientEvent::Progress(x) => s.xp = Some(x),
            ClientEvent::Chat(_, text, true) => s.squad_chat.push(text),
            ClientEvent::Disconnected(why) => panic!("disconnected: {why}"),
            _ => {}
        }
    }
    c.send_input(PlayerCmd::default());
}

#[test]
fn squad_members_spawn_on_their_leader_across_continents() {
    let server = ps_server::start(Config {
        bind: "127.0.0.1:0".into(),
        params: WorldParams {
            seed: 11,
            contested: 4,
        },
        bots: 0,
        ..Config::default()
    })
    .unwrap();
    let addr = format!("127.0.0.1:{}", server.port);
    let mut a = Client::connect(&addr, "Leader", Some(Team::Blue)).unwrap();
    let mut b = Client::connect(&addr, "Member", Some(Team::Blue)).unwrap();
    let (mut sa, mut sb) = (Seen::default(), Seen::default());
    let start = Instant::now();
    let mut step = 0;
    let mut away = None;
    while start.elapsed() < Duration::from_secs(60) {
        pump(&mut a, &mut sa);
        pump(&mut b, &mut sb);
        match step {
            // Both in: the leader starts a squad.
            0 if sa.id.is_some() && sb.id.is_some() && sa.world.is_some() => {
                a.send_squad(SquadCmd::Create);
                step = 1;
            }
            // The member sees it and joins.
            1 if sb.squads.len() == 1 => {
                b.send_squad(SquadCmd::Join(sb.squads[0].id));
                step = 2;
            }
            // The leader goes to a base Blue holds on a contested continent.
            2 if sa.squads.first().is_some_and(|s| s.members.len() == 2) => {
                assert_eq!(sa.squads[0].leader, sa.id.unwrap());
                let w = sa.world.as_ref().unwrap();
                let (c, base) = (3..w.continents.len())
                    .find_map(|c| {
                        w.continents[c]
                            .bases
                            .iter()
                            .position(|b| b.owner == Some(Team::Blue))
                            .map(|b| (c as ContinentId, b as u8))
                    })
                    .expect("Blue holds a base away from home");
                a.send_request(Request::Spawn(SpawnPoint::Base { continent: c, base }));
                away = Some(c);
                step = 3;
            }
            // Once it's there, the member (still on its sanctuary) spawns on it.
            3 if sa.here == away && sa.feet.is_some() => {
                assert_eq!(sb.here, Some(Team::Blue.index() as ContinentId));
                b.send_request(Request::Spawn(SpawnPoint::Squad(sa.id.unwrap())));
                a.send_squad_chat("on me");
                step = 4;
            }
            4 if sb.here == away
                && let (Some(fa), Some(fb)) = (sa.feet, sb.feet) =>
            {
                assert!((fa - fb).length() < 4.0, "{fa:?} vs {fb:?}");
                step = 5;
            }
            // The leader's XP for the spawn on it reaches it.
            5 if !sb.squad_chat.is_empty() && sa.xp.is_some_and(|x| x > 0) => break,
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    assert_eq!(step, 5, "got as far as step {step}");
    assert_eq!(sb.squad_chat, vec!["on me".to_string()]);
    assert_eq!(sa.xp, Some(ps_sim::progress::SPAWN_XP as u64));
}
