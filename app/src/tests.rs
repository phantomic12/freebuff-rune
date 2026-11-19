//! Headless checks: the shell lays out the app's geometry, a tab press switches threads, and the
//! composer really edits text and submits it. These run through the same `Runtime` the window
//! uses, with a fixed virtual clock.

use crate::model::State;
use crate::ui;
use rune::{Env, Key, KeyEvent, Mods, Runtime};

fn runtime(state: State) -> Runtime<State> {
    let env = Env {
        system_dark: true,
        scale: 2.0,
    };
    let mut rt = Runtime::new(state, ui::view, (1440.0, 900.0), env);
    rt.frame(0.0);
    rt
}

fn tap(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no element {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn the_shell_lays_out_the_electron_geometry() {
    let rt = runtime(State::seed());
    let ([_, _, rail_w, rail_h], _) = rt.presented("rail").expect("rail");
    let ([side_x, _, side_w, _], _) = rt.presented("sidebar").expect("sidebar");
    let ([panel_x, _, panel_w, _], _) = rt.presented("overview-panel").expect("panel");
    let ([_, _, _, bar_h], _) = rt.presented("titlebar").expect("titlebar");

    assert_eq!(rail_w, 52.0, "`--shell-rail-width: 52px`");
    assert_eq!(rail_h, 900.0 - 44.0, "the rail starts under the tab bar");
    assert_eq!(side_x, 52.0, "the sidebar sits right of the rail");
    assert_eq!(side_w, 254.0, "the project sidebar's width");
    assert_eq!(panel_w, 440.0, "`--explorer-width: 440px`");
    assert_eq!(panel_x + panel_w, 1440.0, "the panel is flush right");
    assert_eq!(bar_h, 44.0, "`--tabbar-height: 44px`");
}

#[test]
fn a_press_on_an_open_tab_switches_threads() {
    let mut rt = runtime(State::seed());
    assert_eq!(rt.state.active, (0, 0));

    tap(&mut rt, "tab-2-0");
    assert_eq!(rt.state.active, (2, 0), "the tab press selects its thread");
    assert!(rt.state.open_tabs.contains(&(2, 0)));

    // A sidebar row for a thread with no tab opens one.
    tap(&mut rt, "thread-3-0");
    assert_eq!(rt.state.active, (3, 0));
    assert!(
        rt.state.open_tabs.contains(&(3, 0)),
        "opening a thread from the sidebar gives it a tab"
    );
}

#[test]
fn the_composer_edits_and_submits_real_text() {
    let mut rt = runtime(State::seed());
    let before = rt.state.thread().items.len();

    tap(&mut rt, "composer-field");
    rt.paste("ship the native shell");
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.draft.text, "ship the native shell");

    // Backspace edits through the same edit model the window uses.
    rt.key(KeyEvent {
        key: Key::Backspace,
        mods: Mods::default(),
    });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.draft.text, "ship the native shel");

    rt.key(KeyEvent {
        key: Key::Text("l".into()),
        mods: Mods::default(),
    });
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    rt.frame(1.0 / 60.0);

    assert_eq!(rt.state.draft.text, "", "Enter clears the field");
    assert_eq!(
        rt.state.thread().items.len(),
        before + 3,
        "the message, a tool row and the agent's reply"
    );
}

#[test]
fn the_panel_toggle_closes_the_overview_pane() {
    let mut rt = runtime(State::seed());
    assert!(rt.state.panel_open);
    assert!(rt.find("overview-panel").is_some());

    tap(&mut rt, "panel-toggle");
    assert!(
        !rt.state.panel_open,
        "the title bar's panel button toggles it"
    );
    assert!(rt.find("overview-panel").is_none());

    tap(&mut rt, "panel-toggle");
    assert!(
        rt.presented("overview-panel").is_some(),
        "and brings it back"
    );
}

// ---- the orchestrator boot contract ---------------------------------------------------------

#[test]
fn boot_lines_parse_the_way_the_electron_shell_reads_them() {
    use crate::orchestrator as o;

    assert_eq!(
        o::parse_phase("[orchestrator-phase] listen"),
        Some("listen")
    );
    assert_eq!(
        o::parse_phase("[orchestrator-phase] projects_boot_end"),
        Some("projects_boot_end")
    );
    assert_eq!(
        o::parse_phase("[orchestrator-phase] something-new"),
        None,
        "only the phases upstream whitelists reach the UI"
    );
    assert_eq!(
        o::parse_phase("freebuff-desktop orchestrator listening"),
        None
    );

    // The real line from the installed app's stdout.
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"launchId":null,"pid":32704,"port":7781}"#),
        Some((7781, 32704))
    );
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"port": 60111}"#),
        Some((60111, 0))
    );
    assert_eq!(o::parse_ready("[orchestrator-ready] {"), None);

    assert!(o::parse_authed(r#"{"authed":true,"user":{"id":"u1"}}"#));
    assert!(!o::parse_authed(r#"{"authed":false,"user":null}"#));
    assert!(!o::parse_authed(r#"{"authed": false}"#));
    assert!(!o::parse_authed("not json"));
}

#[test]
fn an_http_reply_reads_plain_and_chunked_bodies() {
    use crate::orchestrator::parse_response;

    // `/healthz` with no launch id answers a bare `ok`.
    let (status, body) =
        parse_response(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok").expect("plain");
    assert_eq!(status, 200);
    assert_eq!(body, "ok");

    // Bun streams `/api/` replies, so the reader has to join the chunks.
    let raw = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n8\r\n{\"authed\r\n8\r\n\":false}\r\n0\r\n\r\n";
    let (status, body) = parse_response(raw).expect("chunked");
    assert_eq!(status, 200);
    assert_eq!(body, "{\"authed\":false}");

    let (status, _) = parse_response(b"HTTP/1.1 404 Not Found\r\n\r\n{\"error\":\"not found\"}")
        .expect("error reply");
    assert_eq!(status, 404);
}

#[test]
fn the_footer_says_what_the_orchestrator_is_doing() {
    use crate::orchestrator::Live;

    use crate::orchestrator::Tone;
    let cases = [
        (Live::Off, "Orchestrator off", Tone::Idle),
        (
            Live::Booting("listen".into()),
            "Starting · listen",
            Tone::Work,
        ),
        (
            Live::Ready {
                port: 7781,
                pid: 9,
                authed: false,
            },
            "Orchestrator · signed out",
            Tone::Warn,
        ),
        (
            Live::Ready {
                port: 7781,
                pid: 9,
                authed: true,
            },
            "Orchestrator ready",
            Tone::Good,
        ),
        (
            Live::Failed("exit 1".into()),
            "Orchestrator failed",
            Tone::Bad,
        ),
    ];
    for (live, label, tone) in cases {
        assert_eq!(live.label(), label);
        assert_eq!(live.tone(), tone);
    }
    assert_eq!(
        Live::Ready {
            port: 7781,
            pid: 9,
            authed: true
        }
        .detail(),
        "Freebuff orchestrator ready on port 7781 as process 9, signed in"
    );
}

#[test]
fn the_sidebar_footer_shows_the_orchestrator_line() {
    let mut state = State::seed();
    state.backend = crate::orchestrator::Live::Booting("auth".into());
    let rt = runtime(state);
    assert!(
        rt.find("backend-status").is_some(),
        "the shell's own status line"
    );
}

/// Boots the orchestrator the installed app ships, against a throwaway profile, and waits for
/// `/healthz` — the whole boot contract end to end. Off by default: it spawns Bun and takes about
/// thirty seconds, so run it deliberately:
///
/// ```sh
/// FREEBUFF_LIVE_TEST=1 cargo test -p freebuff-rune -- --nocapture the_shell_boots_the_real
/// ```
#[test]
fn the_shell_boots_the_real_orchestrator() {
    use crate::orchestrator::{Config, Live, launch};
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    if std::env::var_os("FREEBUFF_LIVE_TEST").is_none() {
        return;
    }
    let mut cfg = Config::discover().expect("the installed Freebuff Desktop resources");
    // The installed app may be running and holding the real profile's lock.
    let scratch = std::env::temp_dir().join("freebuff-rune-live");
    std::fs::create_dir_all(&scratch).expect("scratch profile");
    cfg.state = scratch.join("state.json");

    let (tx, rx) = channel();
    launch(cfg, move |live| {
        let _ = tx.send(live);
    });

    let mut ready = None;
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Live::Ready { port, pid, authed }) => {
                ready = Some((port, pid, authed));
                break;
            }
            Ok(Live::Failed(why)) => panic!("the orchestrator did not boot: {why}"),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let (port, pid, authed) = ready.expect("the orchestrator answered /healthz");
    assert!(port > 1024, "it bound the port the shell picked");
    assert!(!authed, "a throwaway profile is signed out");
    if pid > 0 {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status();
    }
}

#[test]
fn the_boot_line_parsers_survive_the_ugly_cases() {
    use crate::orchestrator as o;

    // A Windows pipe can deliver `\r\n`; `BufRead::lines` strips it, but these parsers are also
    // used on raw text (recordings, other clients), so they take a trimmed line either way.
    assert_eq!(
        o::parse_phase("[orchestrator-phase] listen\r"),
        Some("listen")
    );
    assert_eq!(
        o::parse_phase("   [orchestrator-phase] auth   "),
        Some("auth")
    );
    assert_eq!(o::parse_phase(""), None);
    assert_eq!(
        o::parse_phase("[orchestrator-ready] {}"),
        None,
        "the wrong prefix is not a phase"
    );

    // The ready line, in the shapes a different build might print it.
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"pid":32704,"launchId":"x","port":7781}"#),
        Some((7781, 32704)),
        "key order is not guaranteed"
    );
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"launchId":{"port":1},"port":"60111","pid":7}"#),
        Some((60111, 7)),
        "a string port still reads, and a decoy `port` in a nested value is not taken"
    );
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"port":70000,"pid":7}"#),
        None,
        "a port that cannot be one is a misread, not a wrap to 4464"
    );
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"port":0}"#),
        None,
        "port 0 means the server let the OS pick, which is not what we asked for"
    );
    assert_eq!(
        o::parse_ready(r#"[orchestrator-ready] {"port":7781,"pid":0}"#),
        Some((7781, 0)),
        "an older orchestrator without a pid still boots"
    );
    assert_eq!(o::parse_ready("[orchestrator-ready] no json here"), None);

    // `/api/auth/status`: the top-level flag wins over anything nested behind it.
    assert!(o::parse_authed(
        "{\"authed\": true, \"user\": {\"id\": \"u1\"}}"
    ));
    assert!(!o::parse_authed("{\"authed\": false, \"user\": null}"));
    assert!(
        !o::parse_authed(r#"{"authed":false,"user":{"authed":true}}"#),
        "the first `authed` is the real one; a nested one must not flip the answer"
    );
    assert!(
        !o::parse_authed(r#"{"authenticated":true}"#),
        "substring is not the field"
    );
}

#[test]
fn the_failure_tail_keeps_its_last_line_without_splitting_a_character() {
    use crate::orchestrator::push_tail;

    // 3 000 chars of a multi-byte character straddles the cut: the old code panicked here.
    let mut tail = String::new();
    push_tail(&mut tail, &"é".repeat(2600));
    push_tail(&mut tail, "[orchestrator] another instance has the profile");
    assert!(
        tail.len() < 3100,
        "the tail is bounded: {} bytes",
        tail.len()
    );
    assert!(
        tail.ends_with("has the profile\n"),
        "the line the footer shows survives: {tail:?}"
    );
}

#[test]
fn every_button_has_a_name() {
    // An icon-only control without a label is announced as an image with no name. The shell has
    // 40-odd buttons, most of them glyphs, so the invariant is worth a test rather than a habit.
    let rt = runtime(State::seed());
    let elements = rt.elements();
    let unnamed: Vec<String> = elements
        .iter()
        .filter(|e| e.role == "button" && e.label.as_deref().map_or(true, str::is_empty))
        .map(|e| format!("{} ({})", e.key.as_deref().unwrap_or("?"), e.path))
        .collect();
    assert!(unnamed.is_empty(), "unlabelled buttons: {unnamed:?}");
    let buttons = elements.iter().filter(|e| e.role == "button").count();
    assert!(
        buttons >= 40,
        "expected the whole shell's buttons, saw {buttons}"
    );
    // The glyphs inside them are decorative: `icons::lines` and `icons::solid` mark themselves
    // `a11y_hidden`, so the accessibility tree (what `--tree` dumps) carries no nameless images
    // even though they still draw. That is the layer to check, not this list.
}
