//! Freebuff Desktop's shell, rebuilt natively on Rune: the same panes, tokens and geometry the
//! Electron renderer drew, with Rune's own GPU renderer and text stack instead of Chromium.
//!
//! Check it headlessly, the way the Electron app could not be checked:
//!   cargo run -p freebuff-rune -- --snap shot.png
//!   cargo run -p freebuff-rune -- --snap shot.png --scene light
//!   cargo run -p freebuff-rune -- --tree tree.json

mod fixtures;
mod keys;
mod model;
mod orchestrator;
mod remote;
#[cfg(test)]
mod tests;
mod tokens;
mod ui;
mod vault;
mod window_state;

use model::State;
use rune::{App, AppSpec, Cx, El, Titlebar};
use tokens::TABBAR_H;

fn view(s: &State, cx: &mut Cx<State>) -> El {
    ui::view(s, cx)
}

/// The menu's accelerators, routed app-wide: they work with the composer focused.
fn shortcuts(s: &mut State, ev: &rune::KeyEvent, tx: &mut rune::Tx) -> bool {
    keys::handle(s, ev, tx)
}

/// Read the orchestrator's own data into the shell: the account, every project, its threads, and
/// then a transcript each time the window asks for one. A background thread, because the window
/// never waits on the network; the fixtures stay on screen until the first snapshot lands.
fn spawn_sync(handle: rune::AppHandle<State>, port: u16, launch: String) {
    let endpoint = remote::Endpoint { port, launch };
    std::thread::spawn(move || {
        let snapshot = match endpoint.snapshot() {
            Ok(snapshot) => snapshot,
            Err(why) => {
                // A signed-in boot that cannot list projects is worth one line, not a dialog.
                eprintln!("freebuff-rune: sync failed: {why}");
                return;
            }
        };
        // The thread the shell puts on screen first is the one whose transcript it needs.
        let first = snapshot.projects.first().and_then(|(project, threads)| {
            threads.first().map(|thread| model::Want {
                project: project.id.clone(),
                thread: thread.id.clone(),
            })
        });
        let (wants, asked) = std::sync::mpsc::channel::<model::Want>();
        let ask = wants.clone();
        let projects = snapshot.projects;
        handle.update(move |s, _| s.apply_snapshot(projects, wants));
        eprintln!("freebuff-rune: synced the orchestrator's projects");
        if let Some(want) = first {
            let _ = ask.send(want);
        }

        while let Ok(want) = asked.recv() {
            match endpoint.read(&snapshot.account, &want.project, &want.thread, 0) {
                Ok(transcript) => {
                    let (project, thread) = (want.project, want.thread);
                    handle.update(move |s, _| s.apply_transcript(&project, &thread, transcript));
                }
                Err(why) => eprintln!("freebuff-rune: read {}: {why}", want.thread),
            }
        }
    });
}

fn main() {
    let spec = AppSpec::<State> {
        scenes: &[
            ("light", |s: &mut State| s.dark = false),
            ("empty", |s: &mut State| {
                s.active = (6, 0);
                s.projects[6].threads.push(model::Thread {
                    title: "New chat".into(),
                    status: model::Status::Idle,
                    messages: Vec::new(),
                    elevations: Vec::new(),
                    total_messages: 0,
                    remote: None,
                    context: 0.0,
                    branch: None,
                    running: false,
                });
            }),
            ("settings", |s: &mut State| s.settings_open = true),
            ("backend", |s: &mut State| {
                s.backend = orchestrator::Live::Ready {
                    port: 7781,
                    pid: 32704,
                    authed: true,
                    launch: "snapshot".into(),
                }
            }),
            ("panel-closed", |s: &mut State| s.panel_open = false),
            ("bare", |s: &mut State| {
                s.panel_open = false;
                s.sidebar_open = false;
            }),
        ],
        on_key: Some(shortcuts),
        ..AppSpec::new("Freebuff", (1440.0, 900.0), State::seed, view)
    };

    // The app draws its own 44-point tab bar and keeps its window controls over it; on Windows
    // that replaces the native caption entirely and Rune draws the caption buttons.
    let mut app = App::new(spec).titlebar(Titlebar::new(TABBAR_H));

    // Remember the size the window was last left at. `on_window` fires as the native window is
    // created, so the saved size applies before the first frame rather than as a visible jump;
    // `on_frame_drawn` is the only place the current size is observable. Neither runs headless, so
    // a snapshot keeps exactly the size it was asked for.
    let restored = window_state::load();
    if let Some((w, h)) = restored {
        app = app.on_window(move |handle: &rune::WindowHandle| handle.resize_instant((w, h)));
    }
    let mut last = restored;
    let mut written = std::time::Instant::now();
    app = app.on_frame_drawn(move |rt: &rune::Runtime<State>| {
        let size = Some(rt.viewport);
        if size == last || written.elapsed() < std::time::Duration::from_millis(900) {
            return;
        }
        if let Some((w, h)) = window_state::sane(rt.viewport.0, rt.viewport.1) {
            window_state::save(w, h);
            last = size;
            written = std::time::Instant::now();
        }
    });

    // The data in the real app comes from the Bun orchestrator the Electron shell spawns. This
    // shell spawns the same one, with the same boot contract (see `orchestrator.rs`), and shows
    // what it reaches in the sidebar footer.
    //
    // A snapshot run stays hermetic unless `FREEBUFF_SNAP_BACKEND` asks for the real thing; then
    // it boots the orchestrator and waits, so a snapshot can show the live line instead of a
    // promise. Either way the window never waits: the boot runs on its own thread.
    let headless = std::env::args().any(|a| a.starts_with("--snap") || a.starts_with("--tree"));
    let snap_backend = headless && std::env::var_os("FREEBUFF_SNAP_BACKEND").is_some();
    if (!headless || snap_backend) && std::env::var_os("FREEBUFF_NO_ORCHESTRATOR").is_none() {
        match orchestrator::Config::discover() {
            Some(cfg) => {
                let handle = app.handle();
                let (tx, rx) = std::sync::mpsc::channel();
                let sync_handle = handle.clone();
                orchestrator::launch(cfg, move |live| {
                    // The boot says it is signed in: read the person's projects, then let the
                    // window ask for a transcript as it opens threads.
                    if let orchestrator::Live::Ready {
                        authed,
                        port,
                        launch,
                        ..
                    } = &live
                    {
                        if *authed {
                            spawn_sync(sync_handle.clone(), *port, launch.clone());
                        } else {
                            // No account, no remote data: the fixtures are what this shell can
                            // show, and saying so once is better than an empty sidebar.
                            eprintln!(
                                "freebuff-rune: the orchestrator is signed out; showing the \
                                 fixture thread"
                            );
                        }
                    }
                    let settled = matches!(
                        live,
                        orchestrator::Live::Ready { .. } | orchestrator::Live::Failed(_)
                    );
                    let seen = live.clone();
                    handle.update(move |s, _| s.backend = live.clone());
                    if settled {
                        // The same line the footer shows, on stderr: the window is a GUI, and a
                        // one-line trace of what it reached is the only text a terminal sees.
                        eprintln!("freebuff-rune: backend {}", seen.detail());
                        let _ = tx.send(seen);
                    }
                });
                if snap_backend && rx.recv_timeout(std::time::Duration::from_secs(90)).is_err() {
                    eprintln!("freebuff-rune: backend still booting");
                }
            }
            None => eprintln!(
                "freebuff-rune: no Freebuff Desktop install found; running on fixtures \
                 (set FREEBUFF_RESOURCES_DIR to point at one)"
            ),
        }
    }

    app.run();
}
