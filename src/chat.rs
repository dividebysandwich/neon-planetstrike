//! Typing a chat message (desktop): the chat key (Enter) opens a line at the bottom left, Enter
//! sends it to everyone (a line starting "/s " to one's squad only), Esc drops it. While it's open the game's keys are the line's.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::desktop::Overlays;
use crate::game::Game;
use crate::settings::{Action, Inputs, Settings};

/// The line being typed (None: chat closed).
#[derive(Resource, Default)]
pub struct ChatLine(pub Option<String>);

const MAX: usize = 160;

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        // After everything else in the frame: the Esc or Enter that closes the line isn't
        // also seen by the pause menu or the game that frame.
        app.init_resource::<ChatLine>()
            .add_systems(PostUpdate, typing);
    }
}

#[allow(clippy::too_many_arguments)]
fn typing(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<Settings>,
    mut input: MessageReader<KeyboardInput>,
    mut line: ResMut<ChatLine>,
    mut overlays: ResMut<Overlays>,
    game: Option<ResMut<Game>>,
    mut said: Local<bool>,
) {
    let Some(mut game) = game else {
        line.0 = None;
        overlays.chat = false;
        input.clear();
        return;
    };
    // Testing: PS_SAY=text says it once in the game, and leaves it typed in the line.
    if !*said
        && game.alive()
        && let Ok(t) = std::env::var("PS_SAY")
    {
        *said = true;
        game.client.send_chat(&t);
        line.0 = Some(t);
        overlays.chat = true;
    }
    let Some(text) = line.0.as_mut() else {
        // Closed: the chat key opens it (when nothing else is open).
        let i = Inputs {
            keys: &keys,
            mouse: &mouse,
            wheel: Default::default(),
        };
        if !overlays.any() && game.connected && settings.just_pressed(Action::Chat, &i) {
            line.0 = Some(String::new());
            overlays.chat = true;
        }
        // The key that opened it isn't typed.
        input.clear();
        return;
    };
    let mut done = None;
    for k in input.read() {
        if !k.state.is_pressed() {
            continue;
        }
        match &k.logical_key {
            Key::Enter => done = Some(true),
            Key::Escape => done = Some(false),
            Key::Backspace => {
                text.pop();
            }
            Key::Space => {
                if text.chars().count() < MAX {
                    text.push(' ');
                }
            }
            Key::Character(c) => {
                for ch in c.chars().filter(|c| !c.is_control()) {
                    if text.chars().count() < MAX {
                        text.push(ch);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(send) = done {
        if send {
            let t = text.trim().to_string();
            // ("/s ..." goes to the squad only.)
            if let Some(sq) = t.strip_prefix("/s ") {
                game.client.send_squad_chat(sq.trim());
            } else if !t.is_empty() {
                game.client.send_chat(&t);
            }
        }
        line.0 = None;
        overlays.chat = false;
    }
}
