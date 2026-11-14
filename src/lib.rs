//! Neon Planetstrike: three factions fight over continents linked by warp gates, in VR or on a
//! flat screen.
//!
//! ```text
//! neon-planetstrike [--vr | --gl-vr | --flat] [--name NAME] [--faction red|blue|green|auto]
//!          [--offline] [--seed N] [--world small|medium|large] [--bots N]
//!          [--difficulty easy|normal|hard]
//!          [--host [PORT]] [--public] [--server-name NAME] [--connect HOST[:PORT]]
//!          [--screenshot FILE --after SECS] [--quit-after SECS]
//! ```
//!
//! Offline and hosted worlds run the server here (`ps_server`) and join it; `--connect` joins
//! another. Modes as in the start: VR when a headset is there (src/xr.rs, src/glvr), else the
//! flat screen (src/desktop.rs).

pub mod actors;
pub mod assets;
pub mod audio;
pub mod bases;
pub mod build;
pub mod chat;
pub mod deploy;
pub mod desktop;
pub mod eyes;
pub mod flora;
pub mod freecam;
pub mod fx;
pub mod game;
#[cfg(all(feature = "glvr", target_os = "linux"))]
pub mod glvr;
pub mod holster;
pub mod hud;
pub mod hull;
pub mod interact;
pub mod login;
pub mod map;
pub mod materials;
pub mod menu;
pub mod minimap;
pub mod neonui;
pub mod noderadar;
pub mod objectives;
pub mod options;
pub mod pause;
pub mod reflections;
pub mod rig;
pub mod scores;
pub mod seatview;
pub mod segtext;
pub mod settings;
pub mod squad;
pub mod stats;
pub mod tail;
pub mod terrain;
pub mod vehicles;
pub mod vfont;
pub mod viewkick;
pub mod vr;
pub mod vrui;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod wheel;
#[cfg(windows)]
mod win_console;
pub mod worldbar;
#[cfg(feature = "xr")]
pub mod xr;

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use ps_sim::Team;
use ps_sim::bots::Difficulty;
use ps_sim::worldgen::SizeChoice;

use crate::game::{Game, Hosting};

/// The browser build (docs/web.md): no VR, no hosting, joining through the master server's relay.
pub const WEB: bool = cfg!(target_arch = "wasm32");
/// The browser build without WebGPU draws with WebGL2.
pub const WEBGL2: bool = cfg!(all(target_arch = "wasm32", not(feature = "webgpu")));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Desktop,
    Vr,
    GlVr,
}

#[derive(Clone, Debug, Resource)]
pub struct Args {
    pub mode: Mode,
    pub seed: u64,
    /// Bots on each contested continent (offline and hosted worlds).
    pub bots: usize,
    pub name: String,
    pub name_given: bool,
    pub host: Option<u16>,
    pub public: bool,
    pub server_name: Option<String>,
    pub connect: Option<String>,
    pub difficulty: Difficulty,
    pub arena: SizeChoice,
    pub team: Option<Team>,
    pub left_handed: bool,
    pub screenshot: Option<(String, Vec<f32>)>,
    pub quit_after: Option<f32>,
    /// Show the start menu (nothing chosen on the command line).
    pub menu: bool,
}

impl Args {
    pub fn parse() -> Args {
        let mut a = Args {
            mode: Mode::Auto,
            seed: rand::random::<u32>() as u64 % 100_000,
            bots: 24,
            name: Args::default_name(),
            name_given: false,
            host: None,
            public: false,
            server_name: None,
            connect: None,
            difficulty: Difficulty::default(),
            arena: SizeChoice::default(),
            team: None,
            left_handed: std::env::var("PS_LEFT_HANDED").is_ok_and(|v| v == "1"),
            screenshot: None,
            quit_after: None,
            menu: true,
        };
        #[cfg(target_arch = "wasm32")]
        if let Some(name) = web::param("name").map(|n| settings::clean_name(&n))
            && !name.is_empty()
        {
            a.name = name;
            a.name_given = true;
        }
        let argv: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        let next = |i: &mut usize| -> Option<String> {
            *i += 1;
            argv.get(*i).cloned()
        };
        let mut after = vec![3.0];
        let mut shot = None;
        let mut chosen = false;
        while i < argv.len() {
            match argv[i].as_str() {
                "--vr" => a.mode = Mode::Vr,
                "--flat" => a.mode = Mode::Desktop,
                "--gl-vr" => a.mode = Mode::GlVr,
                "--offline" => chosen = true,
                "--seed" => {
                    a.seed = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(a.seed);
                    chosen = true;
                }
                "--bots" => a.bots = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(a.bots),
                "--name" => {
                    if let Some(n) = next(&mut i) {
                        a.name = n;
                        a.name_given = true;
                    }
                }
                "--host" => {
                    let port = argv.get(i + 1).and_then(|v| v.parse().ok());
                    if port.is_some() {
                        i += 1;
                    }
                    a.host = Some(port.unwrap_or(ps_net::protocol::DEFAULT_PORT));
                    chosen = true;
                }
                "--public" => a.public = true,
                "--server-name" => a.server_name = next(&mut i),
                "--connect" => {
                    a.connect = next(&mut i);
                    chosen = true;
                }
                "--difficulty" => match next(&mut i).as_deref().and_then(Difficulty::parse) {
                    Some(d) => a.difficulty = d,
                    None => eprintln!("--difficulty: easy, normal or hard"),
                },
                "--world" => match next(&mut i).as_deref().and_then(SizeChoice::parse) {
                    Some(s) => a.arena = s,
                    None => eprintln!("--world: small, medium or large"),
                },
                "--faction" | "--team" => {
                    match next(&mut i).as_deref().map(str::to_ascii_lowercase) {
                        Some(t) if t == "red" => a.team = Some(Team::Red),
                        Some(t) if t == "blue" => a.team = Some(Team::Blue),
                        Some(t) if t == "green" => a.team = Some(Team::Green),
                        Some(t) if t == "auto" => a.team = None,
                        _ => eprintln!("--faction: red, blue, green or auto"),
                    }
                }
                "--left-handed" => a.left_handed = true,
                "--screenshot" => shot = next(&mut i),
                "--after" => {
                    after = next(&mut i)
                        .map(|v| v.split(',').filter_map(|t| t.parse().ok()).collect())
                        .unwrap_or(after)
                }
                "--quit-after" => a.quit_after = next(&mut i).and_then(|v| v.parse().ok()),
                other => eprintln!("unknown argument {other}"),
            }
            i += 1;
        }
        a.screenshot = shot.map(|s| (s, after));
        a.menu = !chosen;
        a
    }

    pub fn default_name() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .map(|n| settings::clean_name(&n))
            .ok()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Player".into())
    }

    /// The player renamed in the options (`name`: empty for the default).
    pub fn set_name(
        &mut self,
        settings: &mut settings::Settings,
        menu: Option<&mut menu::MenuModel>,
        name: &str,
    ) {
        let name = settings::clean_name(name);
        if name == settings.name && !self.name_given {
            return;
        }
        let before = self.hosted_name();
        settings.name = name.clone();
        settings.save();
        self.name = if name.is_empty() {
            Args::default_name()
        } else {
            name
        };
        self.name_given = false;
        if let Some(m) = menu
            && self.server_name.is_none()
            && m.server_name == before
        {
            m.server_name = self.hosted_name();
        }
    }

    pub fn hosted_name(&self) -> String {
        self.server_name
            .clone()
            .unwrap_or_else(|| format!("{}'s world", self.name))
    }
}

/// Whether this is a Steam Frame: SteamOS on arm64 (Valve's other SteamOS machines are x86-64).
fn steam_frame() -> bool {
    cfg!(all(target_os = "linux", target_arch = "aarch64"))
        && std::fs::read_to_string("/etc/os-release").is_ok_and(|s| {
            s.lines()
                .any(|l| l.trim() == "ID=steamos" || l.trim() == "ID=\"steamos\"")
        })
}

/// Whether a VR compositor is running: SteamVR's (vrcompositor) or Monado's service. Asking
/// OpenXR itself would start SteamVR wherever it is installed.
fn compositor_running() -> bool {
    const NAMES: [&str; 2] = ["vrcompositor", "monado-service"];
    #[cfg(target_os = "linux")]
    {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR")
            && std::path::Path::new(&dir).join("monado_comp_ipc").exists()
        {
            return true;
        }
        let Ok(procs) = std::fs::read_dir("/proc") else {
            return false;
        };
        procs.flatten().any(|p| {
            std::fs::read_to_string(p.path().join("comm")).is_ok_and(|c| NAMES.contains(&c.trim()))
        })
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/NH", "/FI", "IMAGENAME eq vrcompositor.exe"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("vrcompositor"))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = NAMES;
        false
    }
}

/// The mode to start in without one on the command line: VR on a Steam Frame (our own GL
/// renderer, foveated) or while a VR compositor runs (bevy_mod_openxr, or the GL renderer if the
/// runtime only offers OpenGL), the flat screen otherwise.
fn auto_mode() -> Mode {
    let frame = steam_frame();
    if !frame && !compositor_running() {
        return Mode::Desktop;
    }
    #[cfg(all(feature = "glvr", target_os = "linux"))]
    if frame && glvr::runtime_available() {
        return Mode::GlVr;
    }
    #[cfg(feature = "xr")]
    if xr::runtime_available() {
        return Mode::Vr;
    }
    #[cfg(all(feature = "glvr", target_os = "linux"))]
    if glvr::runtime_available() {
        return Mode::GlVr;
    }
    Mode::Desktop
}

pub fn run() {
    #[cfg(windows)]
    win_console::leave_explorer_console();
    let mut args = Args::parse();
    let settings = settings::Settings::load();
    if !args.name_given && !settings.name.is_empty() {
        args.name = settings.name.clone();
    }
    if args.mode == Mode::Auto {
        args.mode = auto_mode();
        eprintln!("starting in {:?} mode", args.mode);
    }
    let mut app = App::new();
    // In the browser the assets are beside the page (web/index.html copies them there).
    let asset_root = if WEB {
        "assets".to_string()
    } else {
        assets::asset_dir().to_string_lossy().to_string()
    };
    let vr = match args.mode {
        #[cfg(feature = "xr")]
        Mode::Vr => xr::add_plugins(&mut app, &asset_root),
        #[cfg(all(feature = "glvr", target_os = "linux"))]
        Mode::GlVr => glvr::add_plugins(&mut app, &asset_root),
        _ => false,
    };
    if !vr {
        {
            if args.mode != Mode::Desktop {
                warn!("no headset for {:?}; playing on the flat screen", args.mode);
                args.mode = Mode::Desktop;
            }
            app.add_plugins(
                DefaultPlugins
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Neon Planetstrike".into(),
                            // The browser build draws into the page's canvas (web/index.html),
                            // as big as the page, and keeps the browser's keys (F5, Tab, ...)
                            // for the game.
                            canvas: WEB.then(|| "#planetstrike".into()),
                            fit_canvas_to_parent: true,
                            // Testing: PS_NOVSYNC=1 (a hidden window isn't held to the
                            // compositor's pace).
                            present_mode: if std::env::var("PS_NOVSYNC").is_ok() {
                                bevy::window::PresentMode::AutoNoVsync
                            } else {
                                bevy::window::PresentMode::AutoVsync
                            },
                            prevent_default_event_handling: true,
                            ..default()
                        }),
                        ..default()
                    })
                    .set(AssetPlugin {
                        file_path: asset_root,
                        // In the browser each asset is fetched: no .meta files to ask for
                        // (a 404 each).
                        meta_check: if WEB {
                            bevy::asset::AssetMetaCheck::Never
                        } else {
                            default()
                        },
                        ..default()
                    }),
            );
            app.add_plugins((
                desktop::DesktopPlugin,
                viewkick::ViewKickPlugin,
                hud::HudPlugin,
                menu::MenuPlugin,
                neonui::NeonUiPlugin,
                pause::PausePlugin,
                options::OptionsPlugin,
                reflections::ReflectionsPlugin,
                freecam::FreeCamPlugin,
                map::MapPlugin,
                minimap::MinimapPlugin,
                noderadar::NodeRadarPlugin,
                chat::ChatPlugin,
                squad::SquadPlugin,
                scores::ScoresPlugin,
            ))
            .add_plugins((
                build::BuildPlugin,
                objectives::ObjectivesPlugin,
                stats::StatsPlugin,
            ));
        }
    }
    app.add_plugins((
        materials::MaterialsPlugin,
        game::GamePlugin,
        seatview::SeatViewPlugin,
        menu::MenuModelPlugin,
        hull::HullPlugin,
        terrain::TerrainPlugin,
        flora::FloraPlugin,
        bases::BasesPlugin,
        worldbar::WorldBarPlugin,
        actors::ActorsPlugin,
        vehicles::VehiclesPlugin,
        // (What the use key or trigger does: VR's panels show its choices too.)
        interact::InteractPlugin,
        fx::FxPlugin,
        eyes::EyesPlugin,
        audio::SoundPlugin,
    ))
    .add_plugins((
        squad::SquadMarksPlugin,
        deploy::DeployPlugin,
        login::LoginPlugin,
    ))
    .insert_resource(ClearColor(Color::linear_rgb(0.002, 0.002, 0.006)))
    .insert_resource(GlobalAmbientLight {
        color: Color::linear_rgb(0.6, 0.65, 1.0),
        brightness: 80.0,
        ..default()
    })
    .insert_resource(args.clone())
    .insert_resource(settings)
    .add_message::<menu::LeaveGame>()
    .init_resource::<desktop::Overlays>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        viewkick::feel_test
            .before(game::PredictSet)
            .run_if(|| std::env::var("PS_FEEL_TEST").is_ok()),
    )
    .add_systems(Update, start_game.run_if(resource_added::<menu::Start>))
    // In Last: a set's run condition is checked once, so taking the Game away mid-Update
    // would leave the rest of a chain gated on it running without it.
    .add_systems(Last, leave_game);
    if let Some((path, times)) = args.screenshot.clone() {
        app.add_systems(
            Update,
            move |mut commands: Commands, time: Res<Time<Real>>, mut done: Local<usize>| {
                if *done < times.len() && time.elapsed_secs() > times[*done] {
                    let file = if times.len() > 1 {
                        path.replace(".png", &format!("_{}.png", *done))
                    } else {
                        path.clone()
                    };
                    *done += 1;
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(file));
                }
            },
        );
    }
    // Testing: PS_FPS=1 logs the frame rate.
    if std::env::var("PS_FPS").is_ok() {
        app.add_plugins((
            bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
            bevy::diagnostic::LogDiagnosticsPlugin::default(),
        ));
    }
    if let Some(secs) = args.quit_after {
        app.add_systems(
            Update,
            move |time: Res<Time<Real>>, mut exit: MessageWriter<AppExit>| {
                if time.elapsed() > Duration::from_secs_f32(secs) {
                    exit.write(AppExit::Success);
                }
            },
        );
    }
    app.run();
}

fn setup(mut commands: Commands, args: Res<Args>, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(assets::load_models(&mut meshes));
    commands.spawn((
        DirectionalLight {
            illuminance: 1500.0,
            color: Color::linear_rgb(0.7, 0.75, 1.0),
            ..default()
        },
        Transform::from_xyz(0.4, 1.0, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    if !args.menu {
        commands.insert_resource(match &args.connect {
            Some(addr) => menu::Start::Join(addr.clone()),
            None => menu::Start::Local {
                host: args.host.map(|port| Hosting {
                    open: true,
                    internet: args.public,
                    name: args.hosted_name(),
                    port,
                }),
                bots: args.bots,
                difficulty: args.difficulty,
                arena: args.arena,
                seed: args.seed,
                team: args.team,
            },
        });
    }
}

fn start_game(
    mut commands: Commands,
    args: Res<Args>,
    start: Res<menu::Start>,
    mut leave: MessageWriter<menu::LeaveGame>,
) {
    commands.remove_resource::<menu::Start>();
    let game = match start.clone() {
        menu::Start::Join(addr) => join(&addr, &args.name, args.team).map(Game::new),
        #[cfg(not(target_arch = "wasm32"))]
        menu::Start::Local {
            host,
            bots,
            difficulty,
            arena,
            seed,
            team,
        } => {
            let params = ps_sim::WorldParams {
                seed,
                contested: arena.contested(),
            };
            info!("world {seed} ({}), {bots} bots per front", arena.name());
            Game::local(params, bots, difficulty.skill(), host, &args.name, team)
        }
        #[cfg(target_arch = "wasm32")]
        menu::Start::Local { .. } => Err(std::io::Error::other("no local worlds in the browser")),
    };
    match game {
        Ok(g) => commands.insert_resource(g),
        Err(e) => {
            error!("can't start: {e}");
            leave.write(menu::LeaveGame(Some(e.to_string())));
        }
    }
}

/// Starts connecting: over UDP, or in the browser through the master server's relay.
fn join(addr: &str, name: &str, team: Option<Team>) -> std::io::Result<ps_net::client::Client> {
    #[cfg(target_arch = "wasm32")]
    return ps_net::client::Client::relayed(addr, name, team);
    #[cfg(not(target_arch = "wasm32"))]
    ps_net::client::Client::connect(addr, name, team)
}

fn leave_game(
    mut commands: Commands,
    mut leave: MessageReader<menu::LeaveGame>,
    game: Option<ResMut<Game>>,
    mut menu: Option<ResMut<menu::MenuModel>>,
) {
    let Some(why) = leave.read().last().cloned() else {
        return;
    };
    if let Some(mut g) = game {
        g.close();
        commands.remove_resource::<Game>();
    }
    if let (Some(m), Some(why)) = (menu.as_deref_mut(), why.0) {
        m.join_error = Some(why);
        m.rev += 1;
    }
    commands.insert_resource(menu::ShowMenu);
}
