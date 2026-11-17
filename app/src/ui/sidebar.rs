//! The project sidebar: wordmark, search, the two create actions, then projects with their
//! threads — `.project-sidebar`, `.project-sidebar-brand`, `.sidebar-search-button`.

use rune::{Cx, El, col, el, row, spacer, text};

use crate::model::{State, Status, Thread};
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{dot, hover_row};

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok) -> El {
    col()
        .key("sidebar")
        .w(tk::SIDEBAR_W)
        .h_full()
        .min_h(0.0)
        .py(tk::SP4)
        .gap(0.0)
        .child(brand(s, t))
        .child(actions(t))
        .child(projects_list(s, t))
        .child(footer(s, t))
}

fn brand(_s: &State, t: &Tok) -> El {
    row()
        .items_center()
        .gap(tk::SP3)
        .w_full()
        .px(tk::SP6)
        .pb(tk::SP5)
        .child(icons::logo(t.brand, 16.0))
        .child(text("Freebuff").font(tk::title()).color(t.text))
        .child(spacer())
        .child(
            el().size(24.0, 24.0)
                .radius(tk::R_SM)
                .center()
                .cursor(rune::Cursor::Pointer)
                .hover(wash(t.text, 0.08))
                .child(icons::search(t.muted, 14.0)),
        )
}

fn actions(t: &Tok) -> El {
    col()
        .w_full()
        .px(tk::SP3)
        .gap(1.0)
        .child(action_row(
            t,
            icons::plus(t.muted, 14.0),
            "New chat",
            |s: &mut State| s.draft.clear(),
        ))
        .child(action_row(
            t,
            icons::folder(t.muted, 14.0),
            "New project",
            |_s| {},
        ))
        .child(
            row()
                .items_center()
                .w_full()
                .px(tk::SP4)
                .pt(tk::SP5)
                .pb(tk::SP2)
                .child(text("Projects").font(tk::label()).color(t.faint)),
        )
}

fn action_row(t: &Tok, glyph: El, label: &str, on: impl Fn(&mut State) + 'static) -> El {
    hover_row(t, tk::R_SM)
        .items_center()
        .gap(tk::SP3)
        .h(28.0)
        .px(tk::SP4)
        .on_press(on)
        .child(glyph)
        .child(text(label).font(tk::body()).color(t.text))
}

fn projects_list(s: &State, t: &Tok) -> El {
    let mut list = col()
        .w_full()
        .min_h(0.0)
        .grow(1.0)
        .gap(0.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        });
    list = list.px(tk::SP3).pb(tk::SP5);
    for (p, project) in s.projects.iter().enumerate() {
        list = list.child(project_head(t, &project.name));
        if project.threads.is_empty() {
            list = list.child(empty_row(t));
            continue;
        }
        for (i, thread) in project.threads.iter().enumerate() {
            list = list.child(thread_row(t, s, thread, project.threads.len() == 1, p, i));
        }
    }
    list
}

fn project_head(t: &Tok, name: &str) -> El {
    hover_row(t, tk::R_SM)
        .items_center()
        .gap(tk::SP3)
        .h(26.0)
        .px(tk::SP4)
        .mt(tk::SP3)
        .child(icons::folder(t.faint, 12.0))
        .child(
            text(name.to_string())
                .font(tk::ui())
                .color(t.muted)
                .truncate(),
        )
}

fn thread_row(t: &Tok, s: &State, thread: &Thread, only: bool, p: usize, i: usize) -> El {
    let on = s.active == (p, i) && !s.settings_open;
    let status = match thread.status {
        Status::Working => Some(t.brand),
        Status::NeedsYou => Some(t.premium),
        Status::Failed => Some(t.danger),
        Status::Done => Some(tk::wash(t.brand, 0.5)),
        Status::Idle => None,
    };
    let mut r = hover_row(t, tk::R_SM)
        .key(format!("thread-{p}-{i}"))
        .items_center()
        .gap(tk::SP3)
        .h(26.0)
        .px(tk::SP4)
        .pl(tk::SP7)
        .on_press(move |s: &mut State| s.open_thread(p, i));
    if on {
        r = r.fill(wash(t.text, 0.09));
    }
    if let Some(c) = status {
        r = r.child(dot(c, 5.0));
    }
    r.child(
        text(thread.title.clone())
            .font(tk::ui())
            .color(if on {
                t.text
            } else if only {
                t.muted
            } else {
                t.faint
            })
            .truncate(),
    )
}

fn empty_row(t: &Tok) -> El {
    row().items_center().h(26.0).px(tk::SP4).pl(tk::SP7).child(
        text("No chats yet")
            .font(tk::ui())
            .color(tk::wash(t.faint, 0.8)),
    )
}

/// The streak and invite block that closes the sidebar.
fn footer(s: &State, t: &Tok) -> El {
    col()
        .w_full()
        .gap(tk::SP3)
        .pt(tk::SP4)
        .px(tk::SP6)
        .border_sides([1.0, 0.0, 0.0, 0.0], wash(t.border, 0.7))
        .child(backend(s, t))
        .child(
            row()
                .items_center()
                .gap(tk::SP3)
                .child(icons::gift(t.muted, 13.0))
                .child(text("Invite friends").font(tk::ui()).color(t.muted))
                .child(spacer())
                .child(
                    text(format!("{}/{}", s.invites.0, s.invites.1))
                        .font(tk::caption())
                        .color(t.faint),
                ),
        )
        .child(
            row()
                .items_center()
                .gap(tk::SP3)
                .child(streak_dots(s, t))
                .child(
                    text(format!("{} day streak", s.streak_days))
                        .font(tk::caption())
                        .color(t.faint),
                )
                .child(spacer()),
        )
}

/// The orchestrator's own line, above the streak: the shell owns that process now, so the shell
/// is where its state is visible. A dot carries the colour, the text stays at caption size.
fn backend(s: &State, t: &Tok) -> El {
    use crate::orchestrator::Tone;
    let live = &s.backend;
    let tone = match live.tone() {
        Tone::Work => t.warn,
        Tone::Good => t.brand,
        Tone::Warn => t.warn,
        Tone::Bad => t.danger,
        Tone::Idle => t.faint,
    };
    let mut block = col().w_full().gap(2.0).child(
        row()
            .key("backend-status")
            .role("status")
            .label(live.detail())
            .items_center()
            .gap(tk::SP3)
            .child(dot(wash(tone, 0.9), 5.0))
            .child(
                text(live.label())
                    .key("backend-status-text")
                    .font(tk::caption())
                    .color(t.faint),
            )
            .child(spacer()),
    );
    if let Some(reason) = live.reason() {
        block = block.child(
            text(reason)
                .key("backend-status-reason")
                .font(tk::caption())
                .color(wash(t.danger, 0.8)),
        );
    }
    block
}

/// Seven pips, filled for the days the streak has run.
fn streak_dots(s: &State, t: &Tok) -> El {
    let mut r = row().items_center().gap(2.0);
    for i in 0..7 {
        let lit = (i as u32) < s.streak_days.min(7);
        r = r.child(dot(
            if lit {
                tk::wash(t.brand, 0.85)
            } else {
                wash(t.text, 0.12)
            },
            4.0,
        ));
    }
    r
}
