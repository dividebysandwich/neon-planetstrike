//! The planetstrike dedicated server: the whole world (continents, warp gates, bases) on this
//! machine, each continent on its own thread, listed on the master server unless `--private`
//! and announced on the local network unless `--no-lan`.
//!
//!     ps-server [--port 27430] [--name NAME] [--seed N] [--continents 6] [--bots 24]
//!               [--difficulty easy|normal|hard] [--state FILE] [--max-players 200]
//!               [--master URL] [--private] [--no-lan] [--snapshot-threads 2] [--quiet]
//!               [--accounts FILE | --no-accounts]
//!
//! The seed decides the continents, their bases and lattice; the world's state (who holds
//! what) is kept in `--state` (default `world_state.ron`) and picked up again on restart if it
//! belongs to the same seed and world size. Players log in to accounts (or register) kept in
//! `--accounts` (default `accounts.db`, SQLite) unless `--no-accounts`.

use std::path::PathBuf;

use ps_server::Config;
use ps_sim::WorldParams;

fn main() {
    let mut cfg = Config {
        state_path: Some(PathBuf::from("world_state.ron")),
        accounts: Some(PathBuf::from("accounts.db")),
        master: Some(
            std::env::var("PS_MASTER").unwrap_or_else(|_| ps_net::DEFAULT_MASTER.to_string()),
        ),
        lan: true,
        verbose: true,
        ..Config::default()
    };
    let mut port = ps_net::protocol::DEFAULT_PORT;
    let mut seed: Option<u64> = None;
    let mut contested = WorldParams::default().contested;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut num = |what: &str| -> Option<u64> {
            let v = it.next().and_then(|v| v.parse().ok());
            if v.is_none() {
                eprintln!("{what} takes a number");
            }
            v
        };
        match arg.as_str() {
            "--port" => port = num("--port").map_or(port, |v| v as u16),
            "--seed" => seed = num("--seed"),
            "--continents" => {
                contested = num("--continents").map_or(contested, |v| v.clamp(3, 18) as u8)
            }
            "--bots" => cfg.bots = num("--bots").map_or(cfg.bots, |v| v as usize),
            "--max-players" => {
                cfg.max_players = num("--max-players").map_or(cfg.max_players, |v| v as usize)
            }
            "--snapshot-threads" => {
                cfg.snapshot_threads =
                    num("--snapshot-threads").map_or(cfg.snapshot_threads, |v| v.max(1) as usize)
            }
            "--name" => cfg.name = it.next().unwrap_or(cfg.name),
            "--accounts" => cfg.accounts = it.next().map(PathBuf::from),
            "--no-accounts" => cfg.accounts = None,
            "--difficulty" => {
                cfg.skill = match it.next().as_deref() {
                    Some("easy") => 0.2,
                    Some("normal") => 0.5,
                    Some("hard") => 0.85,
                    _ => {
                        eprintln!("--difficulty takes easy, normal or hard");
                        cfg.skill
                    }
                }
            }
            "--state" => cfg.state_path = it.next().map(PathBuf::from),
            "--master" => cfg.master = it.next(),
            "--private" => cfg.master = None,
            "--no-lan" => cfg.lan = false,
            "--quiet" => cfg.verbose = false,
            "-h" | "--help" => {
                println!(
                    "ps-server [--port N] [--name NAME] [--seed N] [--continents N] [--bots N] \
                     [--difficulty easy|normal|hard] [--state FILE] [--max-players N] \
                     [--master URL] [--private] [--no-lan] [--snapshot-threads N] [--quiet]"
                );
                return;
            }
            other => eprintln!("unknown argument {other}"),
        }
    }
    // Without a seed: the one the kept state was made with, else a new one.
    let seed = seed.unwrap_or_else(|| {
        cfg.state_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| ps_sim::WorldState::from_ron(&s))
            .map_or_else(rand::random::<u32>, |s| s.params.seed as u32) as u64
    });
    cfg.params = WorldParams { seed, contested };
    cfg.bind = format!("0.0.0.0:{port}");
    let handle = match ps_server::start(cfg) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("can't listen on UDP port {port}: {e}");
            std::process::exit(1);
        }
    };
    let _ = handle.port;
    // Runs until killed (the state is saved every few seconds after a change).
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
