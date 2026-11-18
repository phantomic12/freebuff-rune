//! Freebuff Desktop's shell, rebuilt natively on Rune: the same panes, tokens and geometry the
//! Electron renderer drew, with Rune's own GPU renderer and text stack instead of Chromium.
//!
//! Check it headlessly, the way the Electron app could not be checked:
//!   cargo run -p freebuff-rune -- --snap shot.png
//!   cargo run -p freebuff-rune -- --snap shot.png --scene light
//!   cargo run -p freebuff-rune -- --tree tree.json

mod fixtures;
mod model;
mod orchestrator;
#[cfg(test)]
mod tests;
mod tokens;
mod ui;

use model::State;
use rune::{App, AppSpec, Cx, El, Titlebar};
use tokens::TABBAR_H;

fn view(s: &State, cx: &mut Cx<State>) -> El {
    ui::view(s, cx)
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
                    items: Vec::new(),
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
                }
            }),
            ("panel-closed", |s: &mut State| s.panel_open = false),
            ("bare", |s: &mut State| {
                s.panel_open = false;
                s.sidebar_open = false;
            }),
        ],
        ..AppSpec::new("Freebuff", (1440.0, 900.0), State::seed, view)
    };

    // The app draws its own 44-point tab bar and keeps its window controls over it; on Windows
    // that replaces the native caption entirely and Rune draws the caption buttons.
    let app = App::new(spec).titlebar(Titlebar::new(TABBAR_H));

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
                orchestrator::launch(cfg, move |live| {
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
