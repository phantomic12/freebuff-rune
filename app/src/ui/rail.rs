//! The left rail: `--shell-rail-width` wide, 36-point buttons with 8-point gaps, and the profile
//! disc at its foot — `.shell-navigation`, `.shell-nav-button`.

use rune::{Cx, El, col, el, spacer, text};

use crate::model::State;
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::button_surface;

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok) -> El {
    col()
        .key("rail")
        .w(tk::RAIL_W)
        .h_full()
        .items_center()
        .py(tk::SP4)
        .gap(tk::SP4)
        .child(nav(t, icons::home(t.muted, 18.0), true, |s| {
            s.settings_open = false
        }))
        .child(nav(t, icons::panels(t.muted, 18.0), false, |s| {
            s.sidebar_open = !s.sidebar_open
        }))
        .child(nav(t, icons::squares(t.muted, 18.0), false, |_s| {}))
        .child(nav(t, icons::gift(t.muted, 18.0), false, |_s| {}))
        .child(spacer())
        .child(nav(t, icons::gear(t.muted, 18.0), s.settings_open, |s| {
            s.settings_open = !s.settings_open
        }))
        .child(avatar(s, t))
}

fn nav(t: &Tok, glyph: El, active: bool, on: impl Fn(&mut State) + 'static) -> El {
    let mut b = button_surface(t, 36.0, tk::R_MD);
    if active {
        b = b.fill(wash(t.text, 0.09));
    }
    b.child(glyph).on_press(on)
}

/// The signed-in profile disc. Freebuff draws the account's initial here.
fn avatar(_s: &State, t: &Tok) -> El {
    el().size(30.0, 30.0)
        .radius(15.0)
        .center()
        .fill(wash(t.text, 0.14))
        .border(1.0, wash(t.text, 0.12))
        .cursor(rune::Cursor::Pointer)
        .hover(wash(t.text, 0.2))
        .child(text("U").font(tk::label()).color(t.text))
}
