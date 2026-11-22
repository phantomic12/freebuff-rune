//! The conversation: user bubbles on the right, the agent's parts at the measure — prose,
//! folded thinking, tool rows, and the sponsored card that arrives *inside* the flow — with the
//! elevation card that holds the thread open below them. `.thread-transcript`, `.messages`,
//! `.user-message-bubble`, `.agent-box`, `.tool-row`.

use rune::{Align, Cx, El, col, el, row, spacer, text};

use crate::model::{Act, Elevation, Message, Part, Role, State, Status, Thread};
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

    if thread.messages.is_empty() && thread.elevations.is_empty() {
        return scroll.child(empty(s, t));
    }

    // The window a `read` would hand back, not the whole thread: `history` says whether more lies
    // either side, and the top row says so when it does.
    let history = thread.history();
    let page = thread.read(0, 200);
    if history.has_older {
        scroll = scroll.child(
            row()
                .key("load-earlier")
                .items_center()
                .gap(tk::SP3)
                .child(icons::list(t.faint, 11.0))
                .child(
                    text(format!(
                        "{} earlier messages not loaded",
                        history.first_seq - 1
                    ))
                    .font(tk::ui())
                    .color(t.faint),
                ),
        );
    }

    // A message is parts, so each one is drawn part by part in the order the agent produced them.
    for message in page {
        scroll = scroll.child(match message.role {
            Role::User => user_message(t, message),
            Role::Assistant => assistant_message(t, message),
        });
    }
    for elevation in &thread.elevations {
        scroll = scroll.child(approval_card(t, elevation));
    }
    scroll
}

fn measure_col() -> El {
    col().w_full().max_w(tk::CHAT_W).min_w(0.0).gap(tk::SP5)
}

fn user_message(t: &Tok, message: &Message) -> El {
    let mut body = col().gap(tk::SP3).max_w(tk::CHAT_W * 0.82);
    for part in &message.parts {
        if let Part::Text(body_text) = part {
            body = body.child(
                text(body_text.clone())
                    .font(tk::message())
                    .color(t.text)
                    .wrap(),
            );
        }
    }
    row()
        .w_full()
        .max_w(tk::CHAT_W)
        .justify_end()
        .child(
            col()
                .px(tk::SP5)
                .py(tk::SP4)
                .radius(tk::R_LG)
                .fill(t.bubble)
                .child(body),
        )
        .key(format!("msg-{}", message.seq))
}

fn assistant_message(t: &Tok, message: &Message) -> El {
    let mut block = measure_col().items_start().align(Align::Start);
    for (i, part) in message.parts.iter().enumerate() {
        block = block.child(match part {
            Part::Text(body) => text(body.clone()).font(tk::message()).color(t.text).wrap(),
            Part::Reasoning(thought) => reasoning_row(t, message.seq, i, thought),
            Part::Tool { name, input } => tool_row(t, message.seq, i, name, input),
            Part::Ad { title, body, cta } => ad_part(t, message.seq, i, title, body, cta),
        });
    }
    block.key(format!("msg-{}", message.seq))
}

/// Thinking, folded: one muted line with the first words and a chevron, as the app draws it until
/// it is opened. The recorded part carries `open`/`collapse` for exactly this.
fn reasoning_row(t: &Tok, seq: u64, i: usize, thought: &str) -> El {
    measure_col().align(Align::Start).child(
        row()
            .key(format!("reasoning-{seq}-{i}"))
            .items_center()
            .gap(tk::SP3)
            .child(icons::squares(t.faint, 11.0))
            .child(
                text(format!("Thought · {}", first_words(thought, 9)))
                    .font(tk::ui())
                    .color(t.faint)
                    .truncate(),
            ),
    )
}

fn tool_row(t: &Tok, seq: u64, i: usize, name: &str, input: &str) -> El {
    let glyph = match Act::of(name) {
        Act::Read => icons::list(t.faint, 11.0),
        Act::Run => icons::terminal(t.faint, 11.0),
        Act::Edit => icons::split(t.faint, 11.0),
        Act::Search => icons::search(t.faint, 11.0),
        Act::Think => icons::squares(t.faint, 11.0),
    };
    measure_col().align(Align::Start).child(
        row()
            .key(format!("tool-{seq}-{i}"))
            .items_center()
            .gap(tk::SP3)
            .child(glyph)
            .child(text(name.to_string()).font(tk::ui()).color(t.muted))
            .child(
                text(input.to_string())
                    .font(tk::caption())
                    .color(t.faint)
                    .truncate(),
            ),
    )
}

/// The sponsored card as it arrives in a real transcript: a part of the conversation, not a mode.
fn ad_part(t: &Tok, seq: u64, i: usize, title: &str, body: &str, cta: &str) -> El {
    measure_col().align(Align::Start).child(
        col()
            .key(format!("ad-{seq}-{i}"))
            .w_full()
            .border(1.0, tk::wash(t.ad, 0.28))
            .radius(tk::R_MD)
            .padding(tk::SP5)
            .gap(tk::SP3)
            .fill(t.surface)
            .child(
                row()
                    .items_center()
                    .gap(tk::SP3)
                    .child(dot(t.ad, 5.0))
                    .child(text("Sponsored").font(tk::caption()).color(t.ad)),
            )
            .child(text(title.to_string()).font(tk::label()).color(t.text))
            .child(
                text(body.to_string())
                    .font(tk::body())
                    .color(t.muted)
                    .wrap(),
            )
            .child(
                row()
                    .items_center()
                    .gap(tk::SP2)
                    .child(text(cta.to_string()).font(tk::ui()).color(t.ad)),
            ),
    )
}

/// The first `n` words of a thought, for the folded row.
fn first_words(body: &str, n: usize) -> String {
    let words: Vec<&str> = body.split_whitespace().collect();
    if words.len() <= n {
        return body.to_string();
    }
    format!("{}…", words[..n].join(" "))
}

/// The card that stops the turn until the person answers. Its buttons are the app's loudest
/// affordance after the composer's send.
fn approval_card(t: &Tok, elevation: &Elevation) -> El {
    let (title, body) = (&elevation.title, &elevation.body);
    measure_col()
        .key("elevation")
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
                .child(crate::ui::widgets::named(
                    el().px(tk::SP5)
                        .h(28.0)
                        .radius(tk::R_SM)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .fill(t.primary_action)
                        .hover(wash(t.text, 0.08))
                        .child(text("Approve").font(tk::ui()).color(t.primary_action_text)),
                    "Approve",
                ))
                .child(crate::ui::widgets::named(
                    el().px(tk::SP5)
                        .h(28.0)
                        .radius(tk::R_SM)
                        .center()
                        .cursor(rune::Cursor::Pointer)
                        .border(1.0, t.control_border)
                        .hover(wash(t.text, 0.06))
                        .child(text("Deny").font(tk::ui()).color(t.muted)),
                    "Deny",
                )),
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
        col = col.child(crate::ui::widgets::named(
            el().px(tk::SP5)
                .py(tk::SP3)
                .radius(999.0)
                .border(1.0, t.control_border)
                .fill(t.raised)
                .cursor(rune::Cursor::Pointer)
                .hover(wash(t.text, 0.06))
                .child(text(prompt).font(tk::ui()).color(t.text)),
            "Suggestion",
        ));
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
