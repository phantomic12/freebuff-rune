//! Remembering the window's size between runs.
//!
//! Freebuff Desktop opens at whatever size the OS gives it and forgets it on quit; the Electron
//! shell leaves that to Chromium. A native window has to do it itself, so this writes one small
//! file — two numbers — and reads it back at startup.
//!
//! What it does not do: position. Rune can *set* a window's position but not report it, so there is
//! nothing to remember yet; a restored size with a default position is honest, an invented one
//! would move someone's window somewhere they did not put it.

use std::io::Write;
use std::path::PathBuf;

/// Below this a window cannot show the three panes; above it a stray value would fill a wall.
const MIN: (f32, f32) = (480.0, 360.0);
const MAX: (f32, f32) = (12_000.0, 12_000.0);

/// `~/.config/freebuff-rune/window.json`, next to nothing else of ours.
pub fn path() -> PathBuf {
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    };
    home.unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("freebuff-rune")
        .join("window.json")
}

/// A size worth restoring, or none. Shared with the test so the bounds are checked once.
pub fn sane(w: f32, h: f32) -> Option<(f32, f32)> {
    let ok = |v: f32, min: f32, max: f32| v.is_finite() && v >= min && v <= max;
    (ok(w, MIN.0, MAX.0) && ok(h, MIN.1, MAX.1)).then_some((w, h))
}

/// Read the last size. A missing, unreadable or nonsensical file is not an error: the window opens
/// at its default, which is what a first run does anyway.
pub fn load_at(file: &std::path::Path) -> Option<(f32, f32)> {
    let text = std::fs::read_to_string(file).ok()?;
    sane(number(&text, "\"w\"")?, number(&text, "\"h\"")?)
}

pub fn save_at(file: &std::path::Path, w: f32, h: f32) -> std::io::Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write then rename: a half-written file after a crash would lose the next start's geometry.
    let tmp = file.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(format!("{{\"w\":{w},\"h\":{h}}}\n").as_bytes())?;
    }
    std::fs::rename(&tmp, file)
}

pub fn load() -> Option<(f32, f32)> {
    load_at(&path())
}

pub fn save(w: f32, h: f32) {
    // Nothing here is worth failing a quit over.
    let _ = save_at(&path(), w, h);
}

/// The number after `key`: strictly `"key": <number>`. A value that is not a number (`NaN`, a
/// string, a nested object) yields none rather than the first digits it can find elsewhere.
fn number(json: &str, key: &str) -> Option<f32> {
    let rest = json.get(json.find(key)? + key.len()..)?.trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| {
            !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' || c == '+')
        })
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_size_comes_back() {
        let dir = std::env::temp_dir().join("freebuff-rune-window-test");
        let file = dir.join("window.json");
        let _ = std::fs::remove_file(&file);

        save_at(&file, 1180.0, 820.0).expect("write");
        assert_eq!(load_at(&file), Some((1180.0, 820.0)));
        assert_eq!(
            std::fs::read_to_string(&file).expect("read").trim(),
            "{\"w\":1180,\"h\":820}"
        );

        // Sizes that cannot be a window are refused rather than restored.
        save_at(&file, 10.0, 820.0).expect("write");
        assert_eq!(load_at(&file), None);
        save_at(&file, f32::NAN, 820.0).expect("write");
        assert_eq!(load_at(&file), None);

        // A truncated or foreign file is not fatal.
        std::fs::write(&file, "{\"w\":").expect("write");
        assert_eq!(load_at(&file), None);
        std::fs::write(&file, "{}").expect("write");
        assert_eq!(load_at(&file), None);
        assert_eq!(load_at(&dir.join("absent.json")), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
