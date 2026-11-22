//! Headless checks: the shell lays out the app's geometry, a tab press switches threads, and the
//! composer really edits text and submits it. These run through the same `Runtime` the window
//! uses, with a fixed virtual clock.

use crate::model::State;
use crate::ui;
use rune::{Env, Key, KeyEvent, Mods, Runtime};
use std::io::{Read, Write};

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
    let before = rt.state.thread().messages.len();

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
        rt.state.thread().messages.len(),
        before + 2,
        "the person's message and the agent's reply"
    );
    let last = rt.state.thread().messages.last().expect("reply");
    assert_eq!(last.role, crate::model::Role::Assistant);
    assert!(
        last.parts
            .iter()
            .any(|p| matches!(p, crate::model::Part::Reasoning(_))),
        "the reply carries a reasoning part, as a real one does"
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
                launch: String::new(),
            },
            "Orchestrator · signed out",
            Tone::Warn,
        ),
        (
            Live::Ready {
                port: 7781,
                pid: 9,
                authed: true,
                launch: String::new(),
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
            authed: true,
            launch: String::new()
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
            Ok(Live::Ready {
                port, pid, authed, ..
            }) => {
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

    // The RPC client against the real server: whatever a signed-out profile answers, it must be
    // the orchestrator's own JSON sentence, not a framing or parsing failure.
    let endpoint = crate::remote::Endpoint {
        port,
        launch: "0000000000000000".into(),
    };
    let answered = endpoint.bootstrap();
    eprintln!("freebuff-rune: bootstrap answered {answered:?}");
    if let Err(why) = &answered {
        assert!(
            !why.contains("unreadable reply") && !why.contains("malformed HTTP"),
            "the request reached the orchestrator and its reply parsed: {why}"
        );
    }
    let _ = endpoint.snapshot();
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
        .filter(|e| e.role == "button" && e.label.as_deref().is_none_or(str::is_empty))
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

#[test]
fn the_menu_chords_reach_the_app_with_the_composer_focused() {
    use crate::keys;

    let mut rt = runtime(State::seed());
    rt.on_key = Some(keys::handle);

    // Typing goes to the composer, which is the point: the chords still work from there.
    tap(&mut rt, "composer-field");
    let before = rt.state.open_tabs.len();
    assert_eq!(before, 3);

    let chord = |c: char, shift: bool| KeyEvent {
        key: Key::Command(if shift { c.to_ascii_uppercase() } else { c }),
        mods: Mods {
            cmd: true,
            ctrl: true,
            shift,
            ..Mods::default()
        },
    };

    rt.key(chord('w', false));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.open_tabs.len(), before - 1, "mod+w closes the tab");

    rt.key(chord('T', true));
    rt.frame(1.0 / 60.0);
    assert_eq!(
        rt.state.open_tabs.len(),
        before,
        "mod+shift+t brings it back"
    );

    rt.key(chord('t', false));
    rt.frame(1.0 / 60.0);
    assert_eq!(
        rt.state.open_tabs.len(),
        before + 1,
        "mod+t opens a thread of the project"
    );

    rt.key(chord(',', false));
    rt.frame(1.0 / 60.0);
    assert!(rt.state.settings_open, "mod+, opens settings");
    rt.key(chord(',', false));
    rt.frame(1.0 / 60.0);
    assert!(!rt.state.settings_open, "and closes them again");
}

#[test]
fn the_transcript_is_parts_and_pages_by_cursor() {
    use crate::model::{Part, Role};

    let s = State::seed();
    let thread = &s.projects[0].threads[0];

    // Four messages, and every part kind the recorded payload carries appears in them.
    assert_eq!(thread.messages.len(), 4);
    let kinds: Vec<&str> = thread
        .messages
        .iter()
        .flat_map(|m| m.parts.iter())
        .map(|p| match p {
            Part::Text(_) => "text",
            Part::Reasoning(_) => "reasoning",
            Part::Tool { .. } => "tool",
            Part::Ad { .. } => "ad",
        })
        .collect();
    for kind in ["text", "reasoning", "tool", "ad"] {
        assert!(kinds.contains(&kind), "no {kind} part in the seed thread");
    }
    assert_eq!(thread.messages[0].role, Role::User);

    // `history` is what a `read` reports, and it says which way more lies.
    let h = thread.history();
    assert_eq!((h.first_seq, h.last_seq, h.total), (1, 4, 4));
    assert!(!h.has_older && !h.has_newer, "the whole thread is loaded");

    // Paging by cursor: the messages after `after`, at most `limit` of them.
    let seqs = |after: u64, limit: usize| -> Vec<u64> {
        thread.read(after, limit).iter().map(|m| m.seq).collect()
    };
    assert_eq!(seqs(0, 2), vec![1, 2]);
    assert_eq!(seqs(2, 2), vec![3, 4]);
    assert_eq!(
        seqs(4, 2),
        Vec::<u64>::new(),
        "nothing after the last message"
    );
    assert_eq!(seqs(0, 0), Vec::<u64>::new(), "a zero window loads nothing");

    // A window that starts mid-thread is the case the real payload describes with `hasOlder`.
    let mut partial = State::seed();
    let t = &mut partial.projects[0].threads[0];
    t.messages.drain(..2);
    t.total_messages = 9;
    let h = t.history();
    assert_eq!((h.first_seq, h.last_seq, h.total), (3, 4, 9));
    assert!(h.has_older, "the thread holds messages before this window");
    assert!(h.has_newer, "and after it");
}

#[test]
fn every_part_kind_reaches_the_screen() {
    let rt = runtime(State::seed());
    // The four kinds, each found by the key its renderer sets.
    assert!(rt.find("msg-1").is_some(), "the person's message");
    assert!(rt.find("reasoning-2-0").is_some(), "folded thinking");
    assert!(rt.find("tool-3-0").is_some(), "a tool row");
    assert!(
        rt.find("ad-3-3").is_some(),
        "the sponsored part in the flow"
    );
    assert!(
        rt.find("elevation").is_some(),
        "the request holding the thread open"
    );
}

/// A fake orchestrator on a loopback port: it answers `local-conversations` and `bootstrap` with
/// the recorded shape and remembers every call, so the RPC client is exercised end to end with no
/// Bun, no profile and no network.
struct FakeOrchestrator {
    port: u16,
    calls: std::sync::Arc<std::sync::Mutex<Vec<(String, String, String)>>>,
}

impl FakeOrchestrator {
    fn start(threads_body: &'static str, read_body: &'static str) -> Self {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a loopback port");
        let port = listener.local_addr().unwrap().port();
        let calls: std::sync::Arc<std::sync::Mutex<Vec<(String, String, String)>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = calls.clone();
        std::thread::spawn(move || {
            for _ in 0..6 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut all = Vec::new();
                while all.windows(4).all(|w| w != b"\r\n\r\n") {
                    let mut buf = [0u8; 4096];
                    let Ok(n) = stream.read(&mut buf) else { break };
                    if n == 0 {
                        break;
                    }
                    all.extend_from_slice(&buf[..n]);
                }
                let Some(head_end) = all.windows(4).position(|w| w == b"\r\n\r\n") else {
                    continue;
                };
                let head = String::from_utf8_lossy(&all[..head_end]).into_owned();
                let mut lines = head.lines();
                let first = lines.next().unwrap_or_default();
                let path = first
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_string();
                let launch = lines
                    .find(|l| l.to_ascii_lowercase().starts_with("x-freebuff-launch-id:"))
                    .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_string()))
                    .unwrap_or_default();
                let body = String::from_utf8_lossy(&all[head_end + 4..]).into_owned();
                recorded
                    .lock()
                    .unwrap()
                    .push((path.clone(), launch, body.clone()));
                let reply = if path.ends_with("/bootstrap") {
                    r#"{"accountId":"acct-7","url":"https://app.freebuff.web/workspace"}"#
                } else if body.contains(r#""operation":"authorize""#) {
                    r#"{"ok":true}"#
                } else if body.contains(r#""operation":"projects""#) {
                    r#"{"projects":[{"id":"proj-alpha","name":"alpha"},{"id":"proj-beta","name":"beta"}]}"#
                } else if body.contains(r#""operation":"threads""#) && body.contains("proj-beta") {
                    // Each project has its own threads; the harness comes from the record.
                    r#"{"threads":[{"id":"thr-9","title":"Explain the wire","harness":"claude-code"}]}"#
                } else if body.contains(r#""operation":"threads""#) {
                    threads_body
                } else {
                    read_body
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                        reply.len()
                    )
                    .as_bytes(),
                );
            }
        });
        FakeOrchestrator { port, calls }
    }

    fn seen(&self) -> Vec<(String, String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[test]
fn the_sync_reads_the_orchestrators_own_data() {
    let fake = FakeOrchestrator::start(
        r#"{"threads":[{"id":"thr-1","title":"Fix the build","running":true,"harness":"codebuff"},{"id":"thr-2","title":"Explain the wire","running":false,"harness":"claude-code"}]}"#,
        r#"{
            "thread":{"id":"thr-1","title":"Fix the build"},
            "history":{"firstSeq":1,"lastSeq":3,"totalMessages":3,"hasOlder":false,"hasNewer":false},
            "messages":[
                {"seq":1,"role":"user","parts":[{"kind":"text","text":"why is the loop slow?"}]},
                {"seq":2,"role":"assistant","parts":[
                    {"kind":"reasoning","text":"the iterator re-reads the file every pass"},
                    {"kind":"tool","toolName":"read_file","input":{"path":"src/loop.ts"}},
                    {"kind":"text","text":"cached, it is 40x faster"}
                ]},
                {"seq":3,"role":"assistant","parts":[
                    {"kind":"ad","ad":{"title":"Loop Profiler","adText":"Find the hot path in minutes.","cta":"Try free"}},
                    {"kind":"unknown","text":"dropped, not drawn"}
                ]}
            ],
            "elevations":[{"title":"Overwrite src/loop.ts?","body":"The plan touches one file."}]
        }"#,
    );
    let endpoint = crate::remote::Endpoint {
        port: fake.port,
        launch: "launch-9".into(),
    };

    let snapshot = endpoint.snapshot().expect("projects and threads list");
    assert_eq!(snapshot.account, "acct-7");
    assert_eq!(snapshot.projects.len(), 2, "every project");
    assert_eq!(snapshot.projects[0].1[0].title, "Fix the build");
    assert!(snapshot.projects[0].1[0].running);
    assert_eq!(snapshot.projects[1].1[0].harness, "claude-code");

    let transcript = endpoint
        .read("acct-7", "proj-alpha", "thr-1", 0)
        .expect("a transcript");
    assert_eq!(transcript.title, "Fix the build");
    assert_eq!(transcript.messages.len(), 3);
    assert_eq!(
        transcript.messages[1].parts,
        vec![
            crate::model::Part::Reasoning("the iterator re-reads the file every pass".into()),
            crate::model::Part::Tool {
                name: "read_file".into(),
                input: "src/loop.ts".into(),
            },
            crate::model::Part::Text("cached, it is 40x faster".into()),
        ]
    );
    assert!(matches!(
        &transcript.messages[2].parts[0],
        crate::model::Part::Ad { title, body, cta }
            if title == "Loop Profiler" && body == "Find the hot path in minutes." && cta == "Try free"
    ));
    assert_eq!(transcript.elevations.len(), 1);
    assert_eq!(transcript.history.total, 3);
    assert!(!transcript.history.has_older);

    // Every call rode the launch header the boot gave it, on the hosted-workspace tree.
    for (path, launch, body) in &fake.seen() {
        assert_eq!(launch, "launch-9", "the launch id rides every call");
        assert!(
            path.starts_with("/api/hosted-workspace/"),
            "{path} is on the hosted-workspace tree"
        );
        if path.ends_with("/local-conversations") {
            assert!(
                body.contains(r#""accountId":"acct-7""#),
                "{path} names the account"
            );
        }
    }
}

#[test]
fn a_snapshot_lands_in_the_model_and_a_thread_opened_asks_for_its_transcript() {
    let fake = FakeOrchestrator::start(
        r#"{"threads":[{"id":"thr-1","title":"one"}]}"#,
        r#"{"thread":{"title":"one"},"history":{"lastSeq":1,"totalMessages":1},"messages":[{"seq":1,"role":"assistant","parts":[{"kind":"text","text":"hello from the orchestrator"}]}],"elevations":[]}"#,
    );
    let endpoint = crate::remote::Endpoint {
        port: fake.port,
        launch: "launch-9".into(),
    };
    let snapshot = endpoint.snapshot().unwrap();

    let mut state = State::seed();
    let (wants, asked) = std::sync::mpsc::channel::<crate::model::Want>();
    state.apply_snapshot(snapshot.projects.clone(), wants);

    // The sidebar is now the orchestrator's: one project, one real thread, no fixtures.
    assert_eq!(
        state.projects.len(),
        2,
        "each project the orchestrator named"
    );
    assert_eq!(state.projects[0].name, "alpha");
    let remote = state.projects[0].threads[0].remote.clone().unwrap();
    assert_eq!(remote.thread, "thr-1");
    assert!(state.sync.is_some(), "a working account is connected");

    // Opening that thread asks the sync thread for its transcript.
    state.open_thread(0, 0);
    let want = asked
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the open asks for a read");
    assert_eq!(
        (want.project.as_str(), want.thread.as_str()),
        ("proj-alpha", "thr-1")
    );

    // And the reply lands where the renderer looks.
    let transcript = endpoint
        .read(&snapshot.account, &want.project, &want.thread, 0)
        .unwrap();
    state.apply_transcript(&want.project, &want.thread, transcript);
    assert_eq!(state.projects[0].threads[0].messages.len(), 1);
    assert_eq!(state.projects[0].threads[0].total_messages, 1);
    assert_eq!(state.projects[0].threads[0].title, "one");

    let rt = runtime(state);
    assert!(rt.find("msg-1").is_some(), "the fetched message is drawn");
}

#[test]
fn an_account_with_nothing_in_it_still_renders() {
    let mut state = State::seed();
    let (wants, asked) = std::sync::mpsc::channel::<crate::model::Want>();
    state.apply_snapshot(Vec::new(), wants);
    assert_eq!(
        state.projects.len(),
        1,
        "one placeholder, not an empty vector"
    );
    assert_eq!(state.projects[0].name, "No projects yet");
    assert!(state.sync.is_some(), "the account is still connected");

    state.open_thread(0, 0);
    assert!(
        asked.try_recv().is_err(),
        "a placeholder has no remote thread to read"
    );

    let rt = runtime(state);
    assert!(rt.find("sidebar").is_some() && rt.find("composer").is_some());
    // An empty thread draws the landing state, which is the one place a name is safe to read.
    assert!(rt.find("rail").is_some(), "the window still lays out");
}
