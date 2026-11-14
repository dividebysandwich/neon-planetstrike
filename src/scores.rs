//! The scoreboard (Tab, held): everyone on the player's continent by faction, most XP this
//! session first: Battle Rank (players; bots have none), kills, deaths, assists, score. Over
//! it, the player's own Battle Rank, XP to the next and certification points.

use bevy::prelude::*;
use ps_sim::Team;
use ps_sim::progress;

use crate::desktop::Overlays;
use crate::game::Game;
use crate::materials::team_color;
use crate::neonui;
use crate::settings::{Action, Inputs, Settings};

#[derive(Component)]
struct ScoresRoot;

/// When the board was last built (s): it's rebuilt now and then while it's up.
#[derive(Resource, Default)]
struct Built(Option<f32>);

pub struct ScoresPlugin;

impl Plugin for ScoresPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Built>().add_systems(Update, show);
    }
}

fn color_of(t: Team) -> Color {
    let c = team_color(Some(t));
    Color::linear_rgb(c.x, c.y, c.z)
}

#[allow(clippy::too_many_arguments)]
fn show(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    overlays: Res<Overlays>,
    game: Option<Res<Game>>,
    time: Res<Time>,
    mut built: ResMut<Built>,
    roots: Query<Entity, With<ScoresRoot>>,
) {
    let i = Inputs {
        keys: &keys,
        mouse: &mouse,
        wheel: default(),
    };
    let want = game.is_some()
        && !overlays.chat
        && !overlays.options
        && (settings.pressed(Action::Scores, &i)
            // Testing: PS_SCORES=1 keeps it up.
            || std::env::var("PS_SCORES").is_ok_and(|v| v == "1"));
    let now = time.elapsed_secs();
    if want && built.0.is_some_and(|t| now - t < 0.5) {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let (true, Some(game)) = (want, game) else {
        built.0 = None;
        return;
    };
    built.0 = Some(now);
    let mine = game.team;
    commands
        .spawn((
            ScoresRoot,
            neonui::backdrop(neonui::BACKDROP, 20),
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            r.spawn(neonui::panel(color_of(mine), None))
                .with_children(|p| {
                    let here = game
                        .continent
                        .map_or(String::new(), |c| game.def.continents[c as usize].name.clone());
                    neonui::title(p, &format!("SCORES  -  {here}"), 28.0, color_of(mine));
                    if let Some(xp) = game.xp {
                        let br = progress::battle_rank(xp);
                        let next = if br < progress::MAX_BR {
                            format!("{} XP to BR {}", progress::xp_for(br + 1) - xp, br + 1)
                        } else {
                            "highest rank".into()
                        };
                        p.spawn(neonui::text(
                            format!(
                                "BATTLE RANK {br}   -   {xp} XP   -   {next}   -   {} certification points",
                                progress::cert_points(br)
                            ),
                            15.0,
                            neonui::TEXT,
                        ));
                    }
                    // The faction's continent locks' discount, and the alert under way.
                    let off = crate::hud::lock_discount(&game);
                    if off > 0.0 {
                        p.spawn(neonui::text(
                            format!(
                                "Continents held: vehicles, MAXes and structures cost {:.0}% less",
                                off * 100.0
                            ),
                            14.0,
                            neonui::GREEN,
                        ));
                    }
                    if let Some((a, _)) = &game.alert {
                        let h = crate::hud::holdings(&game, a.continent);
                        p.spawn(neonui::text(
                            format!(
                                "ALERT on {}: the most bases win  -  RED {}  BLUE {}  GREEN {}",
                                game.def.continents[a.continent as usize].name,
                                h[0],
                                h[1],
                                h[2]
                            ),
                            14.0,
                            neonui::AMBER,
                        ));
                    }
                    p.spawn(neonui::row(28.0)).with_children(|row| {
                        for t in Team::ALL {
                            let mut list: Vec<_> =
                                game.scores.iter().filter(|s| s.team == t).collect();
                            if list.is_empty() {
                                continue;
                            }
                            list.sort_by(|a, b| b.score.cmp(&a.score).then(b.kills.cmp(&a.kills)));
                            row.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(3.0),
                                min_width: Val::Px(300.0),
                                ..default()
                            })
                            .with_children(|col| {
                                let total: u32 = list.iter().map(|s| s.score).sum();
                                col.spawn(neonui::text_w(
                                    format!("{}  ({})   {total}", t.upper(), list.len()),
                                    17.0,
                                    color_of(t),
                                    800,
                                ));
                                cells(col, ["", "BR", "K", "D", "A", "XP"].map(String::from), 12.0, neonui::DIM);
                                for s in list.iter().take(24) {
                                    let br = game
                                        .ranks
                                        .get(&s.id)
                                        .map_or("-".to_string(), |b| b.to_string());
                                    let mut name = game.name_of(s.id);
                                    name.truncate(16);
                                    let color = if Some(s.id) == game.me {
                                        neonui::MAGENTA
                                    } else if game.squad_mate(s.id) {
                                        crate::squad::SQUAD
                                    } else {
                                        neonui::TEXT
                                    };
                                    cells(
                                        col,
                                        [
                                            name,
                                            br,
                                            s.kills.to_string(),
                                            s.deaths.to_string(),
                                            s.assists.to_string(),
                                            s.score.to_string(),
                                        ],
                                        13.0,
                                        color,
                                    );
                                }
                            });
                        }
                    });
                });
        });
}

/// Column widths (px): the name, then the numbers (right aligned).
const WIDTHS: [f32; 6] = [150.0, 34.0, 34.0, 34.0, 34.0, 60.0];

/// A row of the board, its cells in their columns.
fn cells(p: &mut ChildSpawnerCommands, texts: [String; 6], size: f32, color: Color) {
    p.spawn(Node::default()).with_children(|row| {
        for (i, (t, w)) in texts.into_iter().zip(WIDTHS).enumerate() {
            row.spawn(Node {
                width: Val::Px(w),
                justify_content: if i == 0 {
                    JustifyContent::Start
                } else {
                    JustifyContent::End
                },
                ..default()
            })
            .with_children(|c| {
                c.spawn(neonui::text(t, size, color));
            });
        }
    });
}
