//! A server with accounts: a player must register (or log in) before it plays; a wrong
//! password is refused, as is a second login to an account in play; the account keeps its
//! XP, and its stats are asked for.

use std::time::{Duration, Instant};

use ps_net::client::{Client, ClientEvent};
use ps_server::Config;
use ps_sim::{PlayerId, Team, WorldParams};

#[derive(Default, Debug)]
struct Seen {
    asked: bool,
    failed: Option<String>,
    id: Option<PlayerId>,
    stats: Option<Vec<(String, i64)>>,
}

fn pump(c: &mut Client, s: &mut Seen) {
    for ev in c.poll() {
        match ev {
            ClientEvent::LoginRequired(_) => s.asked = true,
            ClientEvent::LoginFailed(why) => s.failed = Some(why),
            ClientEvent::Welcome { id, .. } => s.id = Some(id),
            ClientEvent::Stats(st) => s.stats = Some(st),
            ClientEvent::Disconnected(why) => panic!("disconnected: {why}"),
            _ => {}
        }
    }
}

/// Pumps `c` until `done` (or a few seconds pass).
fn until(c: &mut Client, s: &mut Seen, done: impl Fn(&Seen) -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        pump(c, s);
        if done(s) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn players_register_and_log_in_before_playing() {
    let dir = std::env::temp_dir().join(format!("ps-accounts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("accounts.db");
    let _ = std::fs::remove_file(&db);
    let server = ps_server::start(Config {
        bind: "127.0.0.1:0".into(),
        params: WorldParams {
            seed: 11,
            contested: 4,
        },
        bots: 0,
        accounts: Some(db.clone()),
        ..Config::default()
    })
    .unwrap();
    let addr = format!("127.0.0.1:{}", server.port);
    // Asked to log in; no game till then.
    let mut a = Client::connect(&addr, "Ace", Some(Team::Red)).unwrap();
    let mut sa = Seen::default();
    assert!(until(&mut a, &mut sa, |s| s.asked), "asked to log in");
    assert!(sa.id.is_none());
    // Logging in to an account that doesn't exist: refused; registering: in.
    a.send_login("Ace", "hunter22", false);
    assert!(until(&mut a, &mut sa, |s| s.failed.is_some()), "refused");
    a.send_login("Ace", "hunter22", true);
    assert!(until(&mut a, &mut sa, |s| s.id.is_some()), "in: {sa:?}");
    // Another: a wrong password, then the account already in play.
    let mut b = Client::connect(&addr, "Ace", Some(Team::Red)).unwrap();
    let mut sb = Seen::default();
    assert!(until(&mut b, &mut sb, |s| s.asked));
    b.send_login("ACE", "wrong-one", false);
    assert!(until(&mut b, &mut sb, |s| s.failed.is_some()));
    assert!(sb.failed.as_deref().unwrap().contains("Wrong"), "{sb:?}");
    sb.failed = None;
    b.send_login("ace", "hunter22", false);
    assert!(until(&mut b, &mut sb, |s| s.failed.is_some()));
    assert!(sb.failed.as_deref().unwrap().contains("playing"), "{sb:?}");
    assert!(sb.id.is_none());
    // Stats (none yet).
    a.ask_stats();
    assert!(until(&mut a, &mut sa, |s| s.stats.is_some()));
    drop(server);
    let _ = std::fs::remove_dir_all(&dir);
}
