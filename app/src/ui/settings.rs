//! Settings, drawn as a modal sheet over the shell — the app's own settings surface, reduced to
//! the appearance controls that a native shell has to keep (`--font-size-ui`, the theme switch).

use rune::{Color, El, col, el, row, spacer, text};

use crate::model::State;
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{button_surface, hline};

pub fn overlay(s: &State, t: &Tok) -> El {
    let mut scrim = el()
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(wash(Color::BLACK, 0.5))
        .on_press(|s: &mut State| s.settings_open = false);
    scrim = scrim.child(
        el().absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .center()
            // Swallow presses on the sheet so they do not reach the scrim behind it.
            .on_press(|_s: &mut State| {})
            .child(sheet(s, t)),
    );
    scrim
}

fn sheet(s: &State, t: &Tok) -> El {
    col()
        .w(520.0)
        .max_h(560.0)
        .radius(tk::R_XL)
        .fill(t.surface)
        .border(1.0, t.control_border)
        .shadow(12.0, 40.0, wash(t.shadow, 0.45))
        .child(header(t))
        .child(hline(t))
        .child(
            col()
                .w_full()
                .gap(tk::SP6)
                .padding(tk::SP7)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(appearance(s, t))
                .child(shell_section(t)),
        )
}

fn header(t: &Tok) -> El {
    row()
        .items_center()
        .w_full()
        .px(tk::SP7)
        .h(56.0)
        .child(text("Settings").font(tk::title()).color(t.text))
        .child(spacer())
        .child(
            button_surface(t, 28.0, tk::R_SM)
                .hover(wash(t.text, 0.08))
                .on_press(|s: &mut State| s.settings_open = false)
                .child(icons::cross(t.muted, 13.0)),
        )
}

fn appearance(s: &State, t: &Tok) -> El {
    let opt = |label: &'static str, dark: bool| {
        let on = s.dark == dark;
        el().grow(1.0)
            .h(34.0)
            .radius(tk::R_SM)
            .center()
            .cursor(rune::Cursor::Pointer)
            .fill(if on { t.raised } else { Color::TRANSPARENT })
            .border(
                1.0,
                if on {
                    t.control_border
                } else {
                    Color::TRANSPARENT
                },
            )
            .on_press(move |s: &mut State| s.dark = dark)
            .child(
                text(label)
                    .font(tk::ui())
                    .color(if on { t.text } else { t.muted }),
            )
    };
    col()
        .w_full()
        .gap(tk::SP4)
        .child(text("Appearance").font(tk::label()).color(t.faint))
        .child(
            row()
                .w_full()
                .items_center()
                .gap(2.0)
                .padding(2.0)
                .radius(tk::R_MD)
                .fill(wash(t.text, 0.06))
                .child(opt("Light", false))
                .child(opt("Dark", true)),
        )
}

fn shell_section(t: &Tok) -> El {
    col()
        .w_full()
        .gap(tk::SP4)
        .child(text("Shell").font(tk::label()).color(t.faint))
        .child(
            col()
                .w_full()
                .radius(tk::R_MD)
                .border(1.0, t.control_border)
                .fill(t.surface2)
                .child(setting_row(
                    t,
                    "Native renderer",
                    "Rune draws the window; no Chromium, no Node",
                    None,
                ))
                .child(hline(t))
                .child(setting_row(
                    t,
                    "Interface scale",
                    "Follows the display's scale factor",
                    Some("100%"),
                ))
                .child(hline(t))
                .child(setting_row(
                    t,
                    "Orchestrator",
                    "Local process serving threads and tools",
                    Some("127.0.0.1"),
                )),
        )
}

fn setting_row(t: &Tok, label: &str, detail: &str, value: Option<&str>) -> El {
    row()
        .items_center()
        .w_full()
        .gap(tk::SP4)
        .px(tk::SP5)
        .py(tk::SP4)
        .child(
            col()
                .gap(1.0)
                .child(text(label).font(tk::body()).color(t.text))
                .child(text(detail).font(tk::caption()).color(t.faint)),
        )
        .child(spacer())
        .child(match value {
            Some(v) => text(v).font(tk::ui()).color(t.muted),
            None => row().items_center().gap(tk::SP2).child(
                el().w(28.0).h(16.0).radius(8.0).fill(t.brand).child(
                    el().absolute(14.0, 2.0)
                        .size(12.0, 12.0)
                        .radius(6.0)
                        .fill(t.primary_action_text),
                ),
            ),
        })
}
