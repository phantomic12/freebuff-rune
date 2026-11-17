#![allow(dead_code)] // A mirror of the stylesheet: every token exists whether or not a pane uses it yet.
//! Freebuff Desktop's design tokens, recovered from the shipped renderer's stylesheet
//! (`assets/index-*.css` in `orchestrator/ui`) and re-expressed for Rune.
//!
//! Every hex value here is the app's own `:root` / `:root[data-theme=light]` value, so the
//! native shell matches what Electron drew.

use rune::Color;
use rune::color::hex;
use rune::tok::TextStyle;

/// One appearance's palette. Field names follow the CSS custom properties they came from.
#[derive(Clone, Copy)]
pub struct Tok {
    pub bg: Color,
    pub chrome: Color,
    pub surface: Color,
    pub surface2: Color,
    pub raised: Color,
    pub bubble: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub brand: Color,
    pub brand_dim: Color,
    pub primary_action: Color,
    pub primary_action_text: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub control_bg: Color,
    pub control_border: Color,
    pub danger: Color,
    pub premium: Color,
    pub info: Color,
    pub warn: Color,
    pub shadow: Color,
    /// The ad banner's green, the loudest surface in the app.
    pub ad: Color,
    pub ad_text: Color,
}

fn c(s: &str) -> Color {
    hex(s).expect("token hex")
}

pub fn tok(dark: bool) -> Tok {
    if dark {
        Tok {
            bg: c("#0a0a0a"),
            chrome: c("#0f0f0f"),
            surface: c("#171717"),
            surface2: c("#1c1c1c"),
            raised: c("#212121"),
            bubble: c("#1c1c1c"),
            border: c("#262626"),
            text: c("#ebebeb"),
            muted: c("#9e9e9e"),
            faint: c("#7a7a7a"),
            brand: c("#b5cea5"),
            brand_dim: c("#9db58b"),
            primary_action: c("#9db58b"),
            primary_action_text: c("#0a0a0a"),
            accent: c("#ebebeb"),
            accent_dim: c("#7a7a7a"),
            control_bg: c("#1c1c1c"),
            control_border: c("#2e2e2e"),
            danger: c("#cf2f2f"),
            premium: c("#d8a14a"),
            info: c("#c1f5f4"),
            warn: c("#f59e0b"),
            shadow: c("#000000"),
            ad: c("#c8f169"),
            ad_text: c("#12200a"),
        }
    } else {
        Tok {
            bg: c("#f5f5f7"),
            chrome: c("#ebebee"),
            surface: c("#ffffff"),
            surface2: c("#ffffff"),
            raised: c("#ffffff"),
            bubble: c("#e9e9ec"),
            border: c("#e3e3e8"),
            text: c("#1a1a1d"),
            muted: c("#5c5c66"),
            faint: c("#8a8a94"),
            brand: c("#9db58b"),
            brand_dim: c("#5e774e"),
            primary_action: c("#9db58b"),
            primary_action_text: c("#161d19"),
            accent: c("#2c2c33"),
            accent_dim: c("#83838d"),
            control_bg: c("#ffffff"),
            control_border: c("#dcdce2"),
            danger: c("#cf4646"),
            premium: c("#9c6d1e"),
            info: c("#2b8fa6"),
            warn: c("#d98b0a"),
            shadow: c("#000000"),
            ad: c("#c8f169"),
            ad_text: c("#12200a"),
        }
    }
}

/// `--space-1` … `--space-8`.
pub const SP1: f32 = 2.0;
pub const SP2: f32 = 4.0;
pub const SP3: f32 = 6.0;
pub const SP4: f32 = 8.0;
pub const SP5: f32 = 12.0;
pub const SP6: f32 = 16.0;
pub const SP7: f32 = 24.0;
pub const SP8: f32 = 32.0;

/// `--radius-xs` … `--radius-xl`.
pub const R_XS: f32 = 4.0;
pub const R_SM: f32 = 6.0;
pub const R_MD: f32 = 8.0;
pub const R_LG: f32 = 12.0;
pub const R_XL: f32 = 16.0;

/// Shell geometry, from `.desktop-shell` and the panes it lays out.
pub const TABBAR_H: f32 = 44.0;
pub const TAB_H: f32 = 28.0;
pub const RAIL_W: f32 = 52.0;
pub const SIDEBAR_W: f32 = 254.0;
/// `--chat-content-max-width`.
pub const CHAT_W: f32 = 720.0;
/// `--explorer-width`.
pub const PANEL_W: f32 = 440.0;
/// `--explorer-tabs` min-height, the panel's header band.
pub const PANEL_HEAD_H: f32 = 51.0;
pub const CTL_SM: f32 = 28.0;
pub const CTL_MD: f32 = 32.0;

/// The app's type scale: `--font-size-caption` … `--font-size-heading`.
pub fn caption() -> TextStyle {
    TextStyle::new(10.0, 500.0)
}
pub fn label() -> TextStyle {
    TextStyle::new(11.0, 500.0)
}
pub fn ui() -> TextStyle {
    TextStyle::new(12.0, 450.0)
}
pub fn body() -> TextStyle {
    TextStyle::new(13.0, 400.0)
}
pub fn message() -> TextStyle {
    TextStyle::new(15.0, 400.0).lh(22.0)
}
pub fn title() -> TextStyle {
    TextStyle::new(15.0, 500.0)
}
pub fn heading() -> TextStyle {
    TextStyle::new(20.0, 600.0)
}

/// A wash of `c` at `alpha`, the way the stylesheet's `color-mix(… transparent)` reads.
pub fn wash(c: Color, alpha: f32) -> Color {
    c.alpha(alpha)
}
