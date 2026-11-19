//! The keyboard shortcuts the Electron shell had, as chords on Rune's key events.
//!
//! Taken from the installed app's own menu (`app.asar` → `electron/app-menu.cjs`): the accelerators
//! there are `CmdOrCtrl+,`, `CmdOrCtrl+T`, `CmdOrCtrl+Shift+T`, `CmdOrCtrl+W`, `CmdOrCtrl+Shift+W`,
//! `CmdOrCtrl+O`, `CmdOrCtrl+R` and `CmdOrCtrl+Q`. Two of those do not survive the port: reload
//! reloads a page that no longer exists (the state is the app), and open-project needs a folder
//! picker this shell does not have yet. The rest are here.
//!
//! `mod` is the platform's primary modifier, so these read Ctrl on Windows and Linux and ⌘ on
//! macOS, which is what `CmdOrCtrl` meant.

use rune::{Chord, KeyEvent, Tx};

use crate::model::State;

/// What a chord asks for. Kept apart from the matching so the intent is testable without a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Settings,
    NewTab,
    ReopenTab,
    CloseTab,
    CloseWindow,
}

const BINDINGS: [(&str, Action); 5] = [
    ("mod+,", Action::Settings),
    ("mod+t", Action::NewTab),
    ("mod+shift+t", Action::ReopenTab),
    ("mod+w", Action::CloseTab),
    ("mod+shift+w", Action::CloseWindow),
];

fn bindings() -> &'static [(Chord, Action)] {
    static PARSED: std::sync::OnceLock<Vec<(Chord, Action)>> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| {
        BINDINGS
            .iter()
            .map(|(chord, action)| {
                // The literals are in this file and covered by a test, so a parse failure is a
                // programming error rather than something a user can trigger.
                let chord = Chord::parse(chord).expect("binding literal parses");
                (chord, *action)
            })
            .collect()
    })
}

/// The action for a key event, if any. `platform` is explicit so the tests can check the Windows
/// and macOS readings of the same chord.
pub fn action_for_on(ev: &KeyEvent, platform: rune::KeyPlatform) -> Option<Action> {
    bindings()
        .iter()
        .find(|(chord, _)| chord.matches_on(ev, platform))
        .map(|(_, action)| *action)
}

pub fn action_for(ev: &KeyEvent) -> Option<Action> {
    action_for_on(ev, rune::KeyPlatform::current())
}

/// The app's `on_key`: true when the chord was ours and the key should not travel further.
pub fn handle(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let Some(action) = action_for(ev) else {
        return false;
    };
    match action {
        Action::Settings => s.settings_open = !s.settings_open,
        Action::NewTab => s.new_tab(),
        Action::ReopenTab => {
            if !s.reopen_tab() {
                return false;
            }
        }
        Action::CloseTab => s.close_active_tab(),
        Action::CloseWindow => {
            tx.close_window();
            return true;
        }
    }
    // Returning true marks the key as ours, which redraws with the state it just changed.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Key, KeyPlatform, Mods};

    fn ev(key: Key, mods: Mods) -> KeyEvent {
        KeyEvent { key, mods }
    }

    fn cmd() -> Mods {
        Mods {
            cmd: true,
            ..Mods::default()
        }
    }

    fn cmd_shift() -> Mods {
        Mods {
            cmd: true,
            shift: true,
            ..Mods::default()
        }
    }

    #[test]
    fn every_binding_literal_parses() {
        for (chord, _) in BINDINGS {
            Chord::parse(chord).unwrap_or_else(|e| panic!("{chord}: {e}"));
        }
        assert_eq!(bindings().len(), BINDINGS.len());
    }

    #[test]
    fn the_menu_accelerators_map_to_actions() {
        let on =
            |c: char, mods: Mods| action_for_on(&ev(Key::Command(c), mods), KeyPlatform::Windows);

        // With Shift down the platform reports the shifted character, so the chords are checked
        // the way a real key event arrives.
        assert_eq!(on(',', cmd()), Some(Action::Settings));
        assert_eq!(on('t', cmd()), Some(Action::NewTab));
        assert_eq!(on('T', cmd_shift()), Some(Action::ReopenTab));
        assert_eq!(on('w', cmd()), Some(Action::CloseTab));
        assert_eq!(on('W', cmd_shift()), Some(Action::CloseWindow));

        // A bare letter is typing, and Shift must not make `mod+w` read as the window close.
        assert_eq!(on('w', Mods::default()), None);
        assert_eq!(
            on('w', cmd_shift()),
            Some(Action::CloseWindow),
            "shift+mod+w is the window chord, not close-tab"
        );
    }
}
