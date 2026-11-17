//! The shell: title bar over a row of rail, sidebar, conversation and Overview panel. This is the
//! Electron app's `.desktop-shell` geometry, rebuilt from Rune elements.

pub mod chrome;
pub mod composer;
pub mod icons;
pub mod panel;
pub mod rail;
pub mod settings;
pub mod sidebar;
pub mod transcript;
pub mod widgets;

use rune::{Cx, El, col, row};

use crate::model::State;
use crate::tokens::{self as tk, tok};

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let t = tok(s.dark);
    let insets = cx.chrome_insets();
    cx.caption_color(t.text);

    let mut body = row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(rail::view(s, cx, &t));
    if s.sidebar_open {
        body = body.child(sidebar::view(s, cx, &t));
    }
    body = body.child(conversation(s, cx, &t));
    if s.panel_open {
        body = body.child(panel::view(s, cx, &t));
    }

    let mut root = col()
        .w_full()
        .h_full()
        .fill(t.bg)
        .child(chrome::view(s, cx, &t, insets))
        .child(body);

    if s.settings_open {
        root = root.child(settings::overlay(s, &t));
    }
    root
}

/// The centre column: the thread's header, its transcript, and the composer.
fn conversation(s: &State, cx: &mut Cx<State>, t: &tk::Tok) -> El {
    col()
        .grow(1.0)
        .w_full()
        .min_h(0.0)
        .fill(t.bg)
        .child(transcript::header(s, t))
        .child(transcript::view(s, cx, t))
        .child(composer::view(s, cx, t))
}
