//! The title bar: window drag, history, the Code/Chat switch, the thread tabs and the panel
//! toggles — `.shell-tab-row`, `.tabs`, `.tab` and `.shell-tab-trailing` in the stylesheet.

use rune::{ChromeInsets, Color, Cx, El, el, row, text};

use crate::model::{Mode, State, Status};
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{
    button_surface, button_surface_active, dot, hover_row, icon_button, named,
};

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok, insets: ChromeInsets) -> El {
    let leading = if insets.leading > 0.0 {
        insets.leading
    } else {
        tk::SP4
    };

    let mut strip = row()
        .key("titlebar")
        .items_center()
        .gap(tk::SP4)
        .w_full()
        .h(tk::TABBAR_H)
        .pl(leading)
        .pr(insets.trailing + tk::SP4)
        .fill(t.chrome)
        // The drag band sits behind the content, as `.shell-titlebar-drag` does, so a press on a
        // tab still selects it while a press on empty space moves the window.
        .child(
            el().absolute(0.0, 0.0)
                .w_full()
                .h(tk::TABBAR_H)
                .window_drag(),
        )
        .child(history(s, t))
        .child(mode_switch(s, t))
        // The tab strip grows into whatever the leading cluster leaves, so it needs no spacer of
        // its own: a second growing child would split that space and squeeze the tabs.
        .child(tabs(s, t))
        .child(trailing(s, t));
    strip = strip.child(
        el().absolute(0.0, tk::TABBAR_H - 1.0)
            .w_full()
            .h(1.0)
            .fill(t.border),
    );
    strip
}

fn history(s: &State, t: &Tok) -> El {
    let _ = s;
    row()
        .items_center()
        .gap(tk::SP1)
        .child(
            icon_button(t, tk::CTL_SM, tk::R_SM, "Back").child(icons::chevron_left(t.muted, 14.0)),
        )
        .child(
            icon_button(t, tk::CTL_SM, tk::R_SM, "Forward")
                .opacity(0.45)
                .child(icons::chevron_right(t.muted, 14.0)),
        )
}

/// Freebuff's Code / Chat segmented control.
fn mode_switch(s: &State, t: &Tok) -> El {
    let seg = |label: &'static str, mode: Mode| {
        let on = s.mode == mode;
        el().px(9.0)
            .h(22.0)
            .radius(6.0)
            .center()
            .cursor(rune::Cursor::Pointer)
            .fill(if on { t.raised } else { Color::TRANSPARENT })
            .label_static(label)
            .child(
                text(label)
                    .font(tk::ui())
                    .color(if on { t.text } else { t.muted }),
            )
            .on_press(move |s: &mut State| s.mode = mode)
    };
    row()
        .items_center()
        .gap(2.0)
        .padding(2.0)
        .radius(8.0)
        .fill(wash(t.text, 0.06))
        .child(seg("Code", Mode::Code))
        .child(seg("Chat", Mode::Chat))
}

fn tabs(s: &State, t: &Tok) -> El {
    let mut strip = row()
        .items_center()
        .gap(2.0)
        .min_w(0.0)
        .grow(1.0)
        .clip_soft(0.0);
    for (p, i) in &s.open_tabs {
        let on = s.active == (*p, *i);
        strip = strip.child(tab_chip(t, s.tab_thread(*p, *i), on, *p, *i));
    }
    strip
        .child(icon_button(t, tk::CTL_SM, tk::R_SM, "New thread").child(icons::plus(t.muted, 14.0)))
        .child(
            icon_button(t, tk::CTL_SM, tk::R_SM, "All threads")
                .child(icons::chevron_down(t.faint, 13.0)),
        )
}

fn tab_chip(t: &Tok, thread: &crate::model::Thread, on: bool, p: usize, i: usize) -> El {
    let status = match thread.status {
        Status::Working => Some(t.brand),
        Status::NeedsYou => Some(t.premium),
        Status::Failed => Some(t.danger),
        Status::Done => Some(tk::wash(t.brand, 0.55)),
        Status::Idle => None,
    };
    let mut chip = hover_row(t, tk::R_SM)
        .key(format!("tab-{p}-{i}"))
        .label(thread.title.clone())
        .items_center()
        .gap(tk::SP3)
        .h(tk::TAB_H)
        .px(tk::SP4)
        .shrink()
        .min_w(56.0)
        .max_w(236.0)
        .on_press(move |s: &mut State| s.open_thread(p, i));
    if on {
        chip = chip.fill(t.raised).border(1.0, t.border);
    }
    if let Some(c) = status {
        chip = chip.child(dot(c, 6.0));
    } else {
        chip = chip.child(icons::terminal(t.faint, 12.0));
    }
    chip.child(
        text(thread.title.clone())
            .font(tk::ui())
            .color(if on { t.text } else { t.muted })
            .truncate(),
    )
    .child(named(
        el().size(16.0, 16.0)
            .radius(4.0)
            .center()
            .cursor(rune::Cursor::Pointer)
            .hover(wash(t.text, 0.12))
            .child(icons::cross(t.faint, 10.0))
            .on_press(move |s: &mut State| s.close_tab(p, i)),
        "Close thread",
    ))
}

fn trailing(s: &State, t: &Tok) -> El {
    row()
        .items_center()
        .gap(tk::SP2)
        .child(named(
            button_surface_active(t, tk::CTL_SM, tk::R_SM, s.panel_open)
                .key("panel-toggle")
                .child(icons::panels(t.muted, 14.0))
                .on_press(|s: &mut State| s.panel_open = !s.panel_open),
            "Toggle panel",
        ))
        .child(named(
            button_surface(t, tk::CTL_SM, tk::R_SM)
                .child(icons::list(t.muted, 14.0))
                .on_press(|s: &mut State| s.settings_open = true),
            "Settings",
        ))
        .child(named(
            button_surface(t, tk::CTL_SM, tk::R_SM)
                .child(icons::plus(t.muted, 14.0))
                .on_press(|_s: &mut State| {}),
            "More",
        ))
}
