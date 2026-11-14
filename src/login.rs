//! Logging in to a server that keeps accounts (`Game::login`): before playing, the account's
//! name and password, or registering a new one (the password typed twice, to catch a typo).
//! Tab (or a click) moves between the fields, Enter goes; the password shows as dots and only a
//! secret derived from it goes to the server (`ps_net::protocol::login_secret`). The form is
//! shared with VR's panel (src/vrui.rs, typed on its keyboard).

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::desktop::Overlays;
use crate::game::Game;
use crate::neonui::{self, NeonButton};

/// The form: its fields, the one typed into, registering or logging in; what it says.
#[derive(Resource, Default)]
pub struct LoginForm {
    pub name: String,
    pub password: String,
    pub again: String,
    /// 0 the name, 1 the password, 2 it again (registering).
    pub focus: u8,
    pub register: bool,
    /// A problem found before sending (the passwords differ, a field empty).
    pub problem: Option<String>,
    /// Sent, waiting for the server.
    pub sent: bool,
    /// VR's keyboard: capitals.
    pub caps: bool,
    shown: Option<String>,
    started: bool,
}

/// What the form's buttons (and VR's keys) do.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum LoginAct {
    Field(u8),
    Mode(bool),
    Submit,
    Leave,
    /// VR's keyboard.
    Char(char),
    Back,
    Caps,
}

#[derive(Component)]
struct LoginRoot;

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginForm>().add_systems(
            Update,
            (keys, clicks, panel)
                .chain()
                .run_if(resource_exists::<Game>),
        );
    }
}

/// The longest name and password the form takes.
const MAX_NAME: usize = 24;
const MAX_PASSWORD: usize = 64;

impl LoginForm {
    /// Does what `a` says (typing, switching, sending).
    pub fn act(
        &mut self,
        a: LoginAct,
        game: &mut Game,
        leave: &mut MessageWriter<crate::menu::LeaveGame>,
    ) {
        match a {
            LoginAct::Field(f) => self.focus = f.min(if self.register { 2 } else { 1 }),
            LoginAct::Mode(r) => {
                self.register = r;
                self.problem = None;
                self.again.clear();
                self.focus = self.focus.min(if r { 2 } else { 1 });
            }
            LoginAct::Char(c) => {
                let c = if self.caps { c.to_ascii_uppercase() } else { c };
                self.type_char(c)
            }
            LoginAct::Caps => self.caps = !self.caps,
            LoginAct::Back => {
                self.field().pop();
            }
            LoginAct::Leave => {
                leave.write(crate::menu::LeaveGame(None));
            }
            LoginAct::Submit => self.submit(game),
        }
    }

    fn field(&mut self) -> &mut String {
        match self.focus {
            0 => &mut self.name,
            1 => &mut self.password,
            _ => &mut self.again,
        }
    }

    fn type_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        let max = if self.focus == 0 {
            MAX_NAME
        } else {
            MAX_PASSWORD
        };
        let f = self.field();
        if f.chars().count() < max {
            f.push(c);
        }
    }

    fn submit(&mut self, game: &mut Game) {
        self.problem = if self.name.trim().chars().count() < 3 {
            Some("A name of 3 characters or more".into())
        } else if self.password.is_empty() {
            Some("The password, please".into())
        } else if self.register && self.password.chars().count() < 6 {
            Some("A password of 6 characters or more".into())
        } else if self.register && self.password != self.again {
            self.again.clear();
            self.focus = 2;
            Some("The passwords differ: type it again".into())
        } else {
            None
        };
        if self.problem.is_some() {
            return;
        }
        game.client
            .send_login(&self.name, &self.password, self.register);
        if let Some(l) = game.login.as_mut() {
            l.1 = None;
        }
        self.sent = true;
    }

    /// What the form shows of each field (the passwords as dots).
    pub fn shown(&self, f: u8) -> String {
        match f {
            0 => self.name.clone(),
            1 => "*".repeat(self.password.chars().count()),
            _ => "*".repeat(self.again.chars().count()),
        }
    }

    /// The line under the fields: a problem, the server's answer, or waiting.
    pub fn status(&self, game: &Game) -> Option<String> {
        self.problem
            .clone()
            .or_else(|| game.login.as_ref().and_then(|l| l.1.clone()))
            .or_else(|| self.sent.then(|| "...".to_string()))
    }
}

fn keys(
    mut form: ResMut<LoginForm>,
    mut game: ResMut<Game>,
    mut input: MessageReader<KeyboardInput>,
    mut overlays: ResMut<Overlays>,
    mut leave: MessageWriter<crate::menu::LeaveGame>,
) {
    overlays.login = game.login.is_some();
    if game.login.is_none() {
        input.clear();
        return;
    }
    // (Once: the name the game was started with.)
    if !form.started {
        form.started = true;
        form.name = game.client.name().chars().take(MAX_NAME).collect();
        form.focus = if form.name.is_empty() { 0 } else { 1 };
    }
    // The server answered (a failure): typing again.
    if game.login.as_ref().is_some_and(|l| l.1.is_some()) {
        form.sent = false;
    }
    for k in input.read() {
        if !k.state.is_pressed() {
            continue;
        }
        let a = match &k.logical_key {
            Key::Enter => Some(LoginAct::Submit),
            Key::Tab => {
                let n = if form.register { 3 } else { 2 };
                Some(LoginAct::Field((form.focus + 1) % n))
            }
            Key::Backspace => Some(LoginAct::Back),
            Key::Escape => Some(LoginAct::Leave),
            Key::Space => Some(LoginAct::Char(' ')),
            Key::Character(c) => {
                for ch in c.chars() {
                    form.type_char(ch);
                }
                None
            }
            _ => None,
        };
        if let Some(a) = a {
            form.act(a, &mut game, &mut leave);
        }
    }
}

fn clicks(
    mut form: ResMut<LoginForm>,
    mut game: ResMut<Game>,
    q: Query<(&Interaction, &LoginAct), Changed<Interaction>>,
    mut leave: MessageWriter<crate::menu::LeaveGame>,
) {
    for (i, a) in &q {
        if *i == Interaction::Pressed {
            form.act(*a, &mut game, &mut leave);
        }
    }
}

fn panel(
    mut commands: Commands,
    game: Res<Game>,
    mut form: ResMut<LoginForm>,
    roots: Query<Entity, With<LoginRoot>>,
) {
    let sig = game.login.as_ref().map(|(server, _)| {
        format!(
            "{server} {} {} {} {} {} {:?}",
            form.name,
            form.shown(1),
            form.shown(2),
            form.focus,
            form.register,
            form.status(&game)
        )
    });
    if sig == form.shown && (sig.is_none() || !roots.is_empty()) {
        return;
    }
    form.shown = sig;
    for e in &roots {
        commands.entity(e).despawn();
    }
    let Some((server, _)) = &game.login else {
        return;
    };
    let color = neonui::CYAN;
    commands
        .spawn((LoginRoot, neonui::backdrop(neonui::BACKDROP, 30)))
        .with_children(|r| {
            r.spawn(neonui::panel(color, Some(520.0)))
                .with_children(|p| {
                    neonui::title(
                        p,
                        if form.register { "REGISTER" } else { "LOG IN" },
                        32.0,
                        color,
                    );
                    p.spawn(neonui::text(
                        format!("{server} keeps accounts: log in, or register one"),
                        15.0,
                        neonui::DIM,
                    ));
                    p.spawn(neonui::row(10.0)).with_children(|row| {
                        let mut b = NeonButton::new(color);
                        b.selected = !form.register;
                        neonui::button(row, "LOG IN", LoginAct::Mode(false), b, Some(170.0), 16.0);
                        let mut b = NeonButton::new(color);
                        b.selected = form.register;
                        neonui::button(row, "REGISTER", LoginAct::Mode(true), b, Some(170.0), 16.0);
                    });
                    let fields: &[(&str, u8)] = if form.register {
                        &[("Name", 0), ("Password", 1), ("Password again", 2)]
                    } else {
                        &[("Name", 0), ("Password", 1)]
                    };
                    for &(label, f) in fields {
                        p.spawn(neonui::text(label, 14.0, neonui::DIM));
                        let mut b =
                            NeonButton::new(if form.focus == f { color } else { neonui::DIM });
                        b.selected = form.focus == f;
                        let caret = if form.focus == f { "_" } else { "" };
                        neonui::button(
                            p,
                            &format!("{}{caret}", form.shown(f)),
                            LoginAct::Field(f),
                            b,
                            Some(440.0),
                            18.0,
                        );
                    }
                    if let Some(s) = form.status(&game) {
                        p.spawn(neonui::text(s, 15.0, neonui::AMBER));
                    }
                    p.spawn(neonui::row(16.0)).with_children(|row| {
                        neonui::button(
                            row,
                            if form.register { "REGISTER" } else { "LOG IN" },
                            LoginAct::Submit,
                            NeonButton::new(neonui::GREEN),
                            Some(200.0),
                            18.0,
                        );
                        neonui::button(
                            row,
                            "LEAVE",
                            LoginAct::Leave,
                            NeonButton::new(neonui::DIM),
                            Some(140.0),
                            16.0,
                        );
                    });
                    p.spawn(neonui::text(
                        "Tab: next field   Enter: go   Esc: leave",
                        12.0,
                        neonui::DIM,
                    ));
                });
        });
}
