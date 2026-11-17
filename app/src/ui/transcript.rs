//! The conversation: user bubbles on the right, the agent's prose at the measure, tool rows as
//! muted one-liners, and the approval card that holds the thread open — `.thread-transcript`,
//! `.messages`, `.user-message-bubble`, `.agent-box`, `.tool-row`.

use rune::{Align, Cx, El, col, el, row, spacer, text};

use crate::model::{Act, Item, State, Status, Thread};
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{chip, dot};

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok) -> El {
    let thread = s.thread();
    let mut scroll = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .items_center()
        .pt(tk::SP7)
        .pb(tk::SP7)
        .gap(tk::SP6)
        .scroll(rune::ScrollSpec { stick_bottom: true });

    if thread.items.is_empty() {
        return scroll.child(empty(s, t));
    }

    for item in &thread.items {
        scroll = scroll.child(match item {
            Item::User(body) => user_bubble(t, body),
            Item::Agent(body) => agent_text(t, body),
            Item::Activity(act, label) => activity_row(t, *act, label),
            Item::Approval(title, body) => approval_card(t, title, body),
        });
    }
    let _ = thread;
    scroll
}

fn measure_col() -> El {
    col().w_full().max_w(tk::CHAT_W).min_w(0.0).gap(tk::SP5)
}

fn user_bubble(t: &Tok, body: &str) -> El {
    row().w_full().max_w(tk::CHAT_W).justify_end().child(
        col()
            .max_w(tk::CHAT_W * 0.82)
            .px(tk::SP5)
            .py(tk::SP4)
            .radius(tk::R_LG)
            .fill(t.bubble)
            .child(
                text(body.to_string())
                    .font(tk::message())
                    .color(t.text)
                    .wrap(),
            ),
    )
}

fn agent_text(t: &Tok, body: &str) -> El {
    measure_col().items_start().child(
        text(body.to_string())
            .font(tk::message())
            .color(t.text)
            .wrap(),
    )
}

fn activity_row(t: &Tok, act: Act, label: &str) -> El {
    let glyph = match act {
        Act::Read => icons::list(t.faint, 11.0),
        Act::Run => icons::terminal(t.faint, 11.0),
        Act::Edit => icons::split(t.faint, 11.0),
        Act::Search => icons::search(t.faint, 11.0),
        Act::Think => icons::squares(t.faint, 11.0),
    };
    measure_col().align(Align::Start).child(
        row()
            .items_center()
            .gap(tk::SP3)
            .child(glyph)
            .child(text(label.to_string()).font(tk::ui()).color(t.faint)),
    )
}

/// The card that stops the turn until the person answers. Its buttons are the app's loudest
/// affordance after the composer's send.
fn approval_card(t: &Tok, title: &str, body: &str) -> El {
    measure_col()
        .border(1.0, t.border)
        .radius(tk::R_LG)
        .padding(tk::SP5)
        .gap(tk::SP4)
        .fill(t.surface)
        .child(
            row()
                .items_center()
                .gap(tk::SP3)
                .child(dot(t.premium, 6.0))
                .child(text(title.to_string()).font(tk::label()).color(t.premium)),
        )
        .child(
            text(body.to_string())
                .font(tk::body())
                .color(t.muted)
                .wrap(),
        )
        .child(
            row()
                .items_center()
                .gap(tk::SP3)
                .child(
                    el().px(tk::SP5)
                        .h(28.0)
                        .radius(tk::R_SM)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .fill(t.primary_action)
                        .hover(wash(t.text, 0.08))
                        .child(text("Approve").font(tk::ui()).color(t.primary_action_text)),
                )
                .child(
                    el().px(tk::SP5)
                        .h(28.0)
                        .radius(tk::R_SM)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .border(1.0, t.control_border)
                        .hover(wash(t.text, 0.06))
                        .child(text("Deny").font(tk::ui()).color(t.muted)),
                ),
        )
}

/// A new thread's first screen: the wordmark, a line of copy and the starter prompts.
fn empty(s: &State, t: &Tok) -> El {
    let starters = [
        "Explain this repository's architecture",
        "Fix the failing test in the latest change",
        "Find where the build reads its config",
    ];
    let mut col = col()
        .w_full()
        .max_w(tk::CHAT_W)
        .items_center()
        .gap(tk::SP5)
        .pt(58.0)
        .child(icons::logo(t.brand, 34.0))
        .child(
            text("What should Freebuff build?")
                .font(tk::heading())
                .color(t.text),
        )
        .child(
            text(format!(
                "Working in {} · {}",
                s.projects[s.active.0].name, s.location
            ))
            .font(tk::body())
            .color(t.muted),
        )
        .child(spacer());
    for prompt in starters {
        col = col.child(
            el().px(tk::SP5)
                .py(tk::SP3)
                .radius(999.0)
                .border(1.0, t.control_border)
                .fill(t.raised)
                .cursor(rune::Cursor::Pointer)
                .hover(wash(t.text, 0.06))
                .child(text(prompt).font(tk::ui()).color(t.text)),
        );
    }
    col
}

/// The thread's own header strip: status, branch, and the view switches at the right.
pub fn header(s: &State, t: &Tok) -> El {
    let thread: &Thread = s.thread();
    let label = match thread.status {
        Status::Working => "Working",
        Status::NeedsYou => "Needs you",
        Status::Failed => "Failed",
        Status::Done => "Done",
        Status::Idle => "Idle",
    };
    let mut bar = row()
        .items_center()
        .gap(tk::SP3)
        .w_full()
        .h(tk::PANEL_HEAD_H)
        .px(tk::SP5)
        .child(
            text(thread.title.clone())
                .font(tk::title())
                .color(t.text)
                .truncate(),
        )
        .child(chip(t, label));
    if let Some(branch) = &thread.branch {
        bar = bar
            .child(icons::branch(t.faint, 11.0))
            .child(text(branch.clone()).font(tk::caption()).color(t.faint));
    }
    bar.child(spacer())
        .child(icons::list(t.muted, 13.0))
        .child(icons::terminal(t.muted, 13.0))
        .child(icons::folder(t.muted, 13.0))
        .child(icons::split(t.muted, 13.0))
}
