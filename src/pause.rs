//! The in-game menu (Esc): resume, options, the squad panel, suicide (to spawn somewhere else), leave the game.
//! The game goes on meanwhile (it's a shared world).

use bevy::prelude::*;

use crate::desktop::Overlays;
use crate::game::Game;
use crate::menu::LeaveGame;
use crate::neonui::{self, NeonButton};

#[derive(Component)]
struct PauseRoot;

#[derive(Component, Clone, Copy)]
enum PauseAct {
    Resume,
    Options,
    Squad,
    Stats,
    Suicide,
    Leave,
}

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (toggle, buttons, show)
                .chain()
                .run_if(resource_exists::<Game>),
        )
        .add_systems(Update, hide_all.run_if(not(resource_exists::<Game>)));
    }
}

fn hide_all(
    mut commands: Commands,
    q: Query<Entity, With<PauseRoot>>,
    mut overlays: ResMut<Overlays>,
) {
    overlays.menu = false;
    for e in &q {
        commands.entity(e).despawn();
    }
}

pub fn toggle(keys: Res<ButtonInput<KeyCode>>, mut overlays: ResMut<Overlays>) {
    if !keys.just_pressed(KeyCode::Escape) || overlays.options {
        return;
    }
    if overlays.menu {
        overlays.menu = false;
    } else if !overlays.map && !overlays.terminal && !overlays.chat && !overlays.squad {
        overlays.menu = true;
    }
}

fn buttons(
    q: Query<(&Interaction, &PauseAct), Changed<Interaction>>,
    mut overlays: ResMut<Overlays>,
    mut leave: MessageWriter<LeaveGame>,
    mut game: ResMut<Game>,
    mut squad: ResMut<crate::squad::SquadPanel>,
    mut stats: ResMut<crate::stats::StatsPanel>,
) {
    for (i, a) in &q {
        if *i != Interaction::Pressed {
            continue;
        }
        match a {
            PauseAct::Resume => overlays.menu = false,
            PauseAct::Options => overlays.options = true,
            PauseAct::Squad => {
                overlays.menu = false;
                squad.open = true;
            }
            PauseAct::Stats => {
                overlays.menu = false;
                stats.open = true;
                game.stats = None;
                game.client.ask_stats();
            }
            PauseAct::Suicide => {
                overlays.menu = false;
                game.request(ps_sim::Request::Suicide);
            }
            PauseAct::Leave => {
                overlays.menu = false;
                leave.write(LeaveGame(None));
            }
        }
    }
}

fn show(
    mut commands: Commands,
    overlays: Res<Overlays>,
    game: Res<Game>,
    q: Query<Entity, With<PauseRoot>>,
) {
    let want = overlays.menu && !overlays.options;
    if want == !q.is_empty() {
        return;
    }
    for e in &q {
        commands.entity(e).despawn();
    }
    if !want {
        return;
    }
    let host = game.hosting();
    commands
        .spawn((PauseRoot, neonui::backdrop(neonui::BACKDROP, 18)))
        .with_children(|r| {
            r.spawn(neonui::panel(neonui::MAGENTA, Some(460.0)))
                .with_children(|p| {
                    neonui::title(p, "MENU", 34.0, neonui::MAGENTA);
                    p.spawn(neonui::text(game.server_name.clone(), 16.0, neonui::DIM));
                    if let Some(h) = host {
                        p.spawn(neonui::text(h, 13.0, neonui::DIM));
                    }
                    neonui::button(
                        p,
                        "RESUME",
                        PauseAct::Resume,
                        NeonButton::new(neonui::CYAN),
                        Some(280.0),
                        20.0,
                    );
                    neonui::button(
                        p,
                        "OPTIONS",
                        PauseAct::Options,
                        NeonButton::new(neonui::VIOLET),
                        Some(280.0),
                        20.0,
                    );
                    neonui::button(
                        p,
                        "SQUAD (P)",
                        PauseAct::Squad,
                        NeonButton::new(crate::squad::SQUAD),
                        Some(280.0),
                        20.0,
                    );
                    neonui::button(
                        p,
                        "STATS",
                        PauseAct::Stats,
                        NeonButton::new(neonui::CYAN),
                        Some(280.0),
                        20.0,
                    );
                    if game.alive() {
                        neonui::button(
                            p,
                            "SUICIDE (CHOOSE A NEW SPAWN)",
                            PauseAct::Suicide,
                            NeonButton::new(neonui::AMBER),
                            Some(280.0),
                            16.0,
                        );
                    }
                    neonui::button(
                        p,
                        "LEAVE",
                        PauseAct::Leave,
                        NeonButton::new(neonui::PINK),
                        Some(280.0),
                        20.0,
                    );
                });
        });
}
