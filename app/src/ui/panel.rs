//! The right panel (`--explorer-width: 440px`): its tab strip and the Overview page — Environment
//! and Context, as the app shows them before a thread has run anything.

use rune::{Cx, El, col, el, row, spacer, text};

use crate::model::State;
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{hline, hover_row};

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok) -> El {
    col()
        .key("overview-panel")
        .w(tk::PANEL_W)
        .h_full()
        .min_h(0.0)
        .fill(t.surface)
        .border_sides([0.0, 0.0, 0.0, 1.0], wash(t.border, 0.9))
        .child(tab_strip(t))
        .child(
            col()
                .w_full()
                .min_h(0.0)
                .grow(1.0)
                .gap(tk::SP6)
                .padding(tk::SP6)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(environment(s, t))
                .child(context(s, t)),
        )
}

fn tab_strip(t: &Tok) -> El {
    let mut strip = row()
        .items_center()
        .gap(tk::SP2)
        .w_full()
        .min_h(tk::PANEL_HEAD_H)
        .px(tk::SP5)
        .border_sides([0.0, 0.0, 1.0, 0.0], wash(t.border, 0.9));
    for (label, on) in [("Overview", true), ("Files", false), ("Changes", false)] {
        let mut tab = hover_row(t, tk::R_SM)
            .h(26.0)
            .items_center()
            .px(tk::SP4)
            .gap(tk::SP3);
        if on {
            tab = tab.fill(wash(t.text, 0.07));
        }
        strip = strip.child(
            tab.shrink()
                .child(
                    text(label)
                        .font(tk::ui())
                        .color(if on { t.text } else { t.muted })
                        .truncate(),
                )
                .child(
                    el().size(14.0, 14.0)
                        .radius(4.0)
                        .center()
                        .hover(wash(t.text, 0.12))
                        .child(icons::cross(t.faint, 9.0)),
                ),
        );
    }
    strip.child(spacer()).child(
        el().size(24.0, 24.0)
            .radius(tk::R_SM)
            .center()
            .cursor(rune::Cursor::Pointer)
            .hover(wash(t.text, 0.08))
            .child(icons::plus(t.muted, 13.0)),
    )
}

fn section_title(label: &str, t: &Tok) -> El {
    text(label.to_string()).font(tk::label()).color(t.faint)
}

fn environment(s: &State, t: &Tok) -> El {
    let project = &s.projects[s.active.0].name;
    col()
        .w_full()
        .gap(tk::SP4)
        .child(section_title("Environment", t))
        .child(
            col()
                .w_full()
                .gap(tk::SP3)
                .padding(tk::SP4)
                .radius(tk::R_MD)
                .fill(t.surface2)
                .border(1.0, t.control_border)
                .child(
                    row()
                        .items_center()
                        .gap(tk::SP3)
                        .child(icons::globe(t.muted, 13.0))
                        .child(text("This computer").font(tk::ui()).color(t.text))
                        .child(spacer())
                        .child(text(s.location).font(tk::caption()).color(t.faint)),
                )
                .child(hline(t))
                .child(
                    row()
                        .items_center()
                        .gap(tk::SP3)
                        .child(icons::folder(t.muted, 13.0))
                        .child(
                            text(project.clone())
                                .font(tk::ui())
                                .color(t.text)
                                .truncate(),
                        ),
                ),
        )
}

fn context(s: &State, t: &Tok) -> El {
    let used = s.thread().context;
    col()
        .w_full()
        .gap(tk::SP4)
        .child(
            row()
                .items_center()
                .child(section_title("Context", t))
                .child(spacer())
                .child(
                    text(format!("{}%", used.round() as i32))
                        .font(tk::caption())
                        .color(t.muted),
                ),
        )
        .child(
            el().w_full()
                .h(4.0)
                .radius(2.0)
                .fill(wash(t.text, 0.1))
                .child(
                    el().absolute(0.0, 0.0)
                        .w_frac((used / 100.0).clamp(0.0, 1.0))
                        .h(4.0)
                        .radius(2.0)
                        .fill(t.brand),
                ),
        )
}
