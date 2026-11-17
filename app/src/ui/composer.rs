//! The composer: one rounded box holding the field, then a row of controls under it. A sponsored
//! card sits directly above, as `.composer` and `.sponsored-thread-note` lay out in the app.

use rune::{Cx, El, col, el, row, spacer, text, text_input};

use crate::fixtures::{AD_BODY, AD_CTA, AD_TITLE};
use crate::model::{State, Status};
use crate::tokens::{self as tk, Tok, wash};
use crate::ui::icons;
use crate::ui::widgets::{button_surface, control_pill};

pub fn view(s: &State, _cx: &mut Cx<State>, t: &Tok) -> El {
    col()
        .w_full()
        .items_center()
        .pt(tk::SP3)
        .pb(tk::SP5)
        .gap(tk::SP4)
        .child(ad_card(t))
        .child(
            col()
                .w_full()
                .max_w(tk::CHAT_W + 2.0 * tk::SP7)
                .px(tk::SP7)
                .child(box_(s, t)),
        )
}

/// Freebuff is ad-supported: the slot above the composer is part of the product, not chrome.
fn ad_card(t: &Tok) -> El {
    row()
        .w_full()
        .max_w(tk::CHAT_W + 2.0 * tk::SP7)
        .px(tk::SP7)
        .child(
            col()
                .w_full()
                .gap(tk::SP2)
                .px(tk::SP5)
                .py(tk::SP4)
                .radius(tk::R_LG)
                .fill(t.ad)
                .child(
                    row()
                        .items_center()
                        .gap(tk::SP3)
                        .child(text(AD_TITLE).font(tk::ui()).color(t.ad_text).truncate())
                        .child(spacer())
                        .child(
                            el().px(4.0)
                                .h(14.0)
                                .radius(3.0)
                                .center()
                                .fill(wash(t.ad_text, 0.18))
                                .child(text("Ad").font(tk::caption()).color(t.ad_text)),
                        ),
                )
                .child(
                    text(AD_BODY)
                        .font(tk::caption())
                        .color(wash(t.ad_text, 0.78))
                        .max_lines(1)
                        .truncate(),
                )
                .child(
                    row().items_center().gap(tk::SP3).mt(tk::SP1).child(
                        el().px(tk::SP4)
                            .h(22.0)
                            .radius(tk::R_SM)
                            .center()
                            .cursor(rune::Cursor::Pointer)
                            .fill(wash(t.ad_text, 0.9))
                            .child(text(AD_CTA).font(tk::label()).color(t.ad)),
                    ),
                ),
        )
}

fn box_(s: &State, t: &Tok) -> El {
    col()
        .key("composer")
        .w_full()
        .gap(tk::SP3)
        .px(tk::SP5)
        .pt(tk::SP5)
        .pb(tk::SP4)
        .radius(tk::R_LG)
        .fill(t.surface2)
        .border(1.0, t.control_border)
        .shadow(1.0, 3.0, wash(t.shadow, 0.25))
        .child(
            text_input(&s.draft, tk::message())
                .key("composer-field")
                .label("Message Freebuff")
                .w_full()
                .min_h(22.0)
                .max_h(160.0)
                .color(t.text)
                .caret(t.brand, wash(t.text, 0.18))
                .placeholder("Ask anything, @ to mention files", wash(t.text, 0.55))
                .on_edit(|s: &mut State| &mut s.draft)
                .on_submit(|s: &mut State, _tx| s.send()),
        )
        .child(controls(s, t))
}

fn controls(s: &State, t: &Tok) -> El {
    let can_send = !s.draft.text.trim().is_empty();
    let mut row = row()
        .items_center()
        .gap(tk::SP3)
        .w_full()
        .child(
            button_surface(t, 24.0, tk::R_SM)
                .border(1.0, t.control_border)
                .child(icons::plus(t.muted, 13.0)),
        )
        .child(
            control_pill(t, s.model)
                .child(spacer())
                .child(icons::chevron_down(t.faint, 11.0)),
        )
        .child(control_pill(t, s.location))
        .child(spacer());

    let running = s.thread().status == Status::Working;
    if running {
        row = row.child(
            button_surface(t, 30.0, tk::R_SM)
                .fill(wash(t.text, 0.1))
                .child(icons::stop(t.text, 11.0))
                .on_press(|s: &mut State| {
                    let thread = s.thread_mut();
                    thread.running = false;
                    thread.status = Status::Idle;
                }),
        );
    }
    row.child(
        el().size(30.0, 30.0)
            .radius(tk::R_SM)
            .center()
            .cursor(rune::Cursor::Pointer)
            .fill(if can_send {
                t.primary_action
            } else {
                wash(t.text, 0.1)
            })
            .hover(wash(t.text, 0.08))
            .on_press(move |s: &mut State| s.send())
            .child(icons::arrow_up(
                if can_send {
                    t.primary_action_text
                } else {
                    t.faint
                },
                14.0,
            )),
    )
}
