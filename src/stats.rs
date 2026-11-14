//! One's stats as the server keeps them (its account's, or its name's): STATS in the pause menu
//! asks for them and shows the counters, the main ones first (kills, deaths, the ratio,
//! accuracy, headshots, time played), then kills by weapon. (VR: the menu's STATS, in
//! src/vrui.rs, from `lines`.)

use bevy::prelude::*;

use crate::desktop::Overlays;
use crate::game::Game;
use crate::neonui::{self, NeonButton};

#[derive(Resource, Default)]
pub struct StatsPanel {
    pub open: bool,
    built: Option<String>,
}

#[derive(Component)]
struct StatsRoot;

#[derive(Component, Clone, Copy)]
struct CloseStats;

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StatsPanel>().add_systems(
            Update,
            (keys, panel).chain().run_if(resource_exists::<Game>),
        );
    }
}

/// The stats' lines: the main ones, then kills by what (most first).
pub fn lines(stats: &[(String, i64)]) -> Vec<String> {
    let get = |k: &str| stats.iter().find(|s| s.0 == k).map_or(0, |s| s.1);
    let (kills, deaths) = (get("kills"), get("deaths"));
    let (shots, hits) = (get("shots"), get("hits"));
    let secs = get("seconds_played");
    let mut out = vec![
        format!(
            "Kills {kills}   Deaths {deaths}   K/D {:.2}",
            kills as f32 / deaths.max(1) as f32
        ),
        format!(
            "Accuracy {:.0}%   Headshots {}   Shots {shots}",
            hits as f32 * 100.0 / shots.max(1) as f32,
            get("headshots")
        ),
        format!(
            "Vehicles destroyed {}   Revives {}   Hacks {}   Points {}   Built {}",
            get("vehicle_kills"),
            get("revives"),
            get("hacks") + get("terminal_hacks"),
            get("points"),
            get("structures_built")
        ),
        format!("Played {}h {:02}m", secs / 3600, secs / 60 % 60),
    ];
    let mut by: Vec<(&str, i64)> = stats
        .iter()
        .filter_map(|(k, v)| k.strip_prefix("kills:").map(|w| (w, *v)))
        .collect();
    by.sort_by_key(|b| std::cmp::Reverse(b.1));
    if !by.is_empty() {
        out.push("Kills by weapon:".into());
        for (w, n) in by.iter().take(10) {
            out.push(format!("   {w}  {n}"));
        }
    }
    out
}

fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut st: ResMut<StatsPanel>,
    mut overlays: ResMut<Overlays>,
    q: Query<&Interaction, (Changed<Interaction>, With<CloseStats>)>,
) {
    if st.open
        && (keys.just_pressed(KeyCode::Escape) || q.iter().any(|i| *i == Interaction::Pressed))
    {
        st.open = false;
    }
    overlays.stats = st.open;
}

fn panel(
    mut commands: Commands,
    game: Res<Game>,
    mut st: ResMut<StatsPanel>,
    roots: Query<Entity, With<StatsRoot>>,
) {
    let sig = st.open.then(|| format!("{:?}", game.stats));
    if sig == st.built && (sig.is_none() || !roots.is_empty()) {
        return;
    }
    st.built = sig;
    for e in &roots {
        commands.entity(e).despawn();
    }
    if !st.open {
        return;
    }
    let color = neonui::CYAN;
    commands
        .spawn((StatsRoot, neonui::backdrop(neonui::BACKDROP, 22)))
        .with_children(|r| {
            r.spawn(neonui::panel(color, Some(620.0)))
                .with_children(|p| {
                    neonui::title(p, "STATS", 32.0, color);
                    match &game.stats {
                        Some(s) => {
                            for l in lines(s) {
                                p.spawn(neonui::text(l, 16.0, neonui::TEXT));
                            }
                        }
                        None => {
                            p.spawn(neonui::text("Asking the server...", 16.0, neonui::DIM));
                        }
                    }
                    neonui::button(
                        p,
                        "CLOSE",
                        CloseStats,
                        NeonButton::new(neonui::DIM),
                        Some(160.0),
                        16.0,
                    );
                });
        });
}
