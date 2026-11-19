//! Shared shell pieces: the icon buttons, chips and dots that every pane repeats.

use rune::{Color, Cursor, El, el, row, text};

use crate::tokens::{Tok, caption, ui, wash};

/// A pressable rounded surface with hover and press washes: the shell's default control.
pub fn button_surface(t: &Tok, size: f32, radius: f32) -> El {
    el().size(size, size)
        .radius(radius)
        .center()
        .cursor(Cursor::Pointer)
        .hover(wash(t.text, 0.07))
        .press(wash(t.text, 0.11))
}

/// An icon-only button: [`button_surface`] plus the role and name a screen reader needs. Without
/// them the glyph is announced as "image" with no name, which is what an unlabelled control is.
pub fn icon_button(t: &Tok, size: f32, radius: f32, label: &'static str) -> El {
    button_surface(t, size, radius)
        .role("button")
        .label_static(label)
}

/// The same name for a control already built by hand (an active fill, a border, a key).
pub fn named(e: El, label: &'static str) -> El {
    e.role("button").label_static(label)
}

/// The same, already filled as active: the rail's selected button, a toggled panel.
pub fn button_surface_active(t: &Tok, size: f32, radius: f32, active: bool) -> El {
    let mut b = button_surface(t, size, radius);
    if active {
        b = b.fill(wash(t.text, 0.09));
    }
    b
}

/// A hover-only row: sidebar rows, the tab strip's chips.
pub fn hover_row(t: &Tok, radius: f32) -> El {
    el().w_full()
        .radius(radius)
        .cursor(Cursor::Pointer)
        .hover(wash(t.text, 0.06))
        .press(wash(t.text, 0.1))
}

/// A status dot: the tab's state marker, the sidebar's branch marker.
pub fn dot(c: Color, size: f32) -> El {
    el().size(size, size).radius(size / 2.0).fill(c)
}

/// A small label chip: `Ad`, `Local`, tool names.
pub fn chip(t: &Tok, label: &str) -> El {
    row()
        .items_center()
        .px(5.0)
        .h(16.0)
        .radius(4.0)
        .fill(wash(t.text, 0.1))
        .child(text(label).font(caption()).color(t.muted))
}

/// A bordered, pressable pill that reads as a control: the composer's model and location.
pub fn control_pill(t: &Tok, label: &str) -> El {
    row()
        .items_center()
        .gap(4.0)
        .px(7.0)
        .h(24.0)
        .radius(6.0)
        .cursor(Cursor::Pointer)
        .hover(wash(t.text, 0.06))
        .child(text(label).font(ui()).color(t.text))
}

/// A hairline of the shell's border colour.
pub fn hline(t: &Tok) -> El {
    el().h(1.0).w_full().fill(t.border)
}
