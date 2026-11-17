# freebuff-rune

Freebuff Desktop's user interface, rebuilt natively with [Rune](https://github.com/anomalyco/rune)
instead of Electron + Chromium + Node.

Freebuff's own repository does not ship the desktop shell, so the port is sourced from the
**installed artifact** (`%LOCALAPPDATA%\Programs\@codebufffreebuff-desktop`):

| Evidence | Used for |
|---|---|
| `resources/orchestrator/ui/index.html`, `assets/index-*.js` | the shell's own boot/consent page (see the note below) |
| `assets/index-*.css` | every colour, spacing, radius, type size and pane width |
| `resources/app.asar` → `electron/*.cjs` | window chrome behaviour (`titleBarOverlay`, drag region, caption buttons) |
| `resources/orchestrator/orchestrator.js` | the boot contract and the loopback API this shell now speaks |
| a screenshot of the running app | the layout the shell was showing (its *content* is not copied — see the fixtures note) |

One correction to the first row: the shipped `assets/index-*.js` is **not** the app's interface.
It is a small splash/consent bundle (auth, diagnostics and telemetry calls only — no thread, chat
or project endpoints), while the 274 KB stylesheet beside it is the whole design system. The full
runtime UI arrives as a signed *hot bundle* (`orchestrator-bundles/state.json`), and this install
has none (`"current": null`). So the layout evidence is the screenshot plus the stylesheet, and
the data contract is the orchestrator's own code — not a decompiled renderer.

Nothing from those artifacts is copied into this repository; `extracted/` and
`upstream-freebuff/` are gitignored reference material.

## What is a port, not a re-implementation

The Rust code reproduces the app's geometry from its own tokens, not by eye:

| Token | Value | Where it now lives |
|---|---|---|
| `--tabbar-height` | 44 | `tokens::TABBAR_H` + `Titlebar::new(44.0)` |
| `--shell-rail-width` | 52 | `tokens::RAIL_W` |
| project sidebar | 254 | `tokens::SIDEBAR_W` |
| `--chat-content-max-width` | 720 | `tokens::CHAT_W` |
| `--explorer-width` | 440 | `tokens::PANEL_W` |
| `--space-1…8`, `--radius-xs…xl` | 2…32, 4…16 | `tokens::SP*`, `tokens::R_*` |
| `--bg`, `--chrome`, `--surface`, `--border`, `--brand`, … | dark + light | `tokens::tok(dark)` |

`cargo test` asserts those numbers against the laid-out tree, so a regression in layout fails a
test instead of needing an eyeball.

## Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ title bar 44pt: ⌫ 〉  ⌈Code│Chat⌉  [tabs…] [+]      [panel][list][+]  │  drawn by the app
├────┬──────────────┬────────────────────────────┬────────────────────┤
│rail│  sidebar 254 │  conversation (max 720)    │  Overview 440      │
│ 52 │  Freebuff    │  thread header             │  Overview│Files    │
│    │  ⌕ New chat  │  user bubble / agent prose │  Environment       │
│    │  New project │  tool rows / approval card │  Context ▓▓░░ 20%  │
│    │  Projects…   │                            │                    │
│  U │  streak      │  ad slot                   │                    │
│    │              │  composer ──────────────── │                    │
└────┴──────────────┴────────────────────────────┴────────────────────┘
```

Files: `app/src/tokens.rs` (the design system), `model.rs` (state), `fixtures.rs` (the seed
content), and `ui/{chrome,rail,sidebar,transcript,composer,panel,settings,icons}.rs`.

## Build setup

```bash
./scripts/bootstrap-rune.sh     # fetch Rune at the pinned revision (vendor/rune)
cargo build -p freebuff-rune
```

Rune is pre-release and consumed by revision, so it is not on crates.io. `scripts/bootstrap-rune.sh`
clones it at the revision in `vendor/rune-rev.txt` (`--check` verifies an existing clone without
network). The clone is 181 MB and is not committed; the workspace root repeats Rune's
`[patch.crates-io]` entries for its vendored `wgpu-hal`, `winit`, `parley_engine` and
`accesskit_*` — a dependency's own `[patch]` table does not apply to the workspace that consumes
it, so without those entries the build fails resolving `parley_engine`'s features.

## Running it

```bash
cargo run -p freebuff-rune                 # the window
cargo test -p freebuff-rune                # layout + interaction checks
```

Because every Rune app runs headless, the shell is inspectable from a terminal — something the
Electron build could not do:

```bash
./target/debug/freebuff-rune.exe --snap shots/dark.png --size 1440x900
./target/debug/freebuff-rune.exe --snap shots/light.png --scene light
./target/debug/freebuff-rune.exe --snap shots/settings.png --scene settings
./target/debug/freebuff-rune.exe --snap shots/empty.png --scene empty
./target/debug/freebuff-rune.exe --snap shots/bare.png --scene bare
./target/debug/freebuff-rune.exe --snap shots/backend.png --scene backend
./target/debug/freebuff-rune.exe --tree shots/tree.json     # rects, fills, roles, text
```

Scenes: `light`, `settings`, `empty`, `panel-closed`, `bare`, `backend`.

## Platforms

Windows is the target and the only platform exercised: the custom 44-point title bar relies on the
Windows caption buttons Rune draws, and the window has been verified there. `Config::discover`
also knows where macOS and Linux install an Electron app (`~/Applications`, `/Applications`,
`~/.local/share`, `/opt`, `/usr/lib`) and which Bun binary name to expect, but no one has run it on
either yet. CI runs on `windows-latest` only, for the same reason.

Two switches control the orchestrator at boot:

- `FREEBUFF_NO_ORCHESTRATOR=1` — never spawn one (fixtures only).
- `FREEBUFF_SNAP_BACKEND=1` — in a snapshot run, boot a real orchestrator and wait up to 90 s for
  a settled state first, so a snapshot shows the live line rather than a promise:

  ```bash
  FREEBUFF_SNAP_BACKEND=1 ./target/debug/freebuff-rune.exe --snap shots/backend-live.png
  ```

`FREEBUFF_RESOURCES_DIR`, `FREEBUFF_BUN_PATH` and `FREEBUFF_DESKTOP_STATE_PATH` override what
`orchestrator::Config::discover` finds, the same overrides upstream honours.

## The orchestrator, brought back

The data in the real app lives in a Bun process the Electron shell spawns
(`resources/orchestrator/orchestrator.js`). This shell spawns the same process with the same boot
contract, recovered from `electron/main.cjs` and `electron/orchestrator-startup.cjs`:

| Step | Upstream | Here |
|---|---|---|
| pick a port | bind-and-retry loop, `PORT` in the env | `free_port()`, `PORT` in the env |
| profile | `FREEBUFF_DESKTOP_STATE_PATH` (default `~/.config/freebuff-desktop/state.json`) | same env, same default |
| secrets | one stdin line, never the environment | `{"shellLifetimeToken","launchId"}` on stdin |
| lifetime | HMAC challenge/response on `FREEBUFF_SHELL_LIFETIME_PORT`, `ping` every 3 s | same handshake, same ping period |
| boot progress | `[orchestrator-phase] <step>` on stdout | `Live::Booting(phase)` in the footer |
| ready | `[orchestrator-ready] {port,pid}` then `/healthz` | 30 s / 60 s budgets, then `/healthz` |
| API calls | `x-freebuff-launch-id: <token>` | the same header on every `/api/` call |
| shutdown | the shell's socket closing | the same: killing this process stops the orchestrator |

Verified against the installed app, not assumed:

- a real boot reaches `[orchestrator-ready]`, answers `/healthz` (`ok`) and reports
  `{"authed":false}` on a throwaway profile, which the footer draws as “Orchestrator · signed out”;
- `taskkill /F` on this shell (no destructor, no cleanup) stops the orchestrator within 2 s;
- a second shell against the profile the installed app already holds fails loudly: exit code 75,
  “Another Freebuff orchestrator is already using this Desktop state profile”, shown in the
  sidebar as a red line under “Orchestrator failed”.

The status line above the invite block is this port's own addition: the shell owns that process,
so the shell is where its state belongs. Off / starting / ready / signed out / failed, with the
reason under a failure.

```bash
FREEBUFF_LIVE_TEST=1 cargo test -p freebuff-rune -- --nocapture the_shell_boots_the_real
```

That test boots the installed orchestrator against a scratch profile and waits for `/healthz`;
it is off by default because it spawns Bun and takes about a second of warm cache (a cold one is
closer to ten).

## What actually works

- Custom title bar that is the drag region; thread tabs (open-tab list, close, `+`), history
  buttons, the Code/Chat switch, panel toggles, and Rune's Windows caption buttons over the
  trailing edge.
- Sidebar selection switching threads, each with its own transcript and context percentage.
- A real text composer: Rune's edit model, caret, selection, paste, backspace, Enter to submit,
  placeholder, growing height, and a send button that enables only with text.
- Settings sheet: light/dark switch that re-tokenises the whole shell live, plus the shell rows.
- Boot, supervise and shut down a real Freebuff orchestrator, and show what it reached.
- Scroll areas with stick-to-bottom on the transcript.
- Accessibility roles/labels on the shell, tabs, panel, composer field and backend status.

Tests cover: pane geometry, tab press → thread switch, sidebar open → new tab, panel toggle,
typing/backspace/Enter through the edit model, the boot-line parsers, plain and chunked HTTP
replies, the footer's five states, and (gated) a real orchestrator boot.

## Repository layout

```
app/src/            the shell: tokens, model, fixtures, orchestrator client, ui/*
app/src/ui/         chrome, rail, sidebar, transcript, composer, panel, settings, icons, widgets
docs/port-notes.md  the recovered contracts (boot, handshake, launch token, API surface)
scripts/            bootstrap-rune.sh — Rune at the pinned revision
tools/              three read-only helpers used against the installed app; nothing ships from it
```

No Freebuff code is redistributed here; see `NOTICE.md`. Licensed MIT (`LICENSE`).

## What is mocked, and the next step

The shell now owns the orchestrator process and speaks its boot protocol, but none of its data
yet:

- Threads, messages and tool rows still come from `fixtures.rs`; `State::send` appends a canned
  reply. `/api/auth/status` is the only payload this port reads.
- The fixtures are **invented sample content**, not anyone's projects: the app's screenshot showed
  real names, and a published port cannot carry them. The sponsored card's copy is placeholder
  text in the app's own voice, not a real ad; the CTA is inert.
- The thread, chat and event routes are reachable but **unmapped**. The interface that drives them
  is the hot bundle this install does not have, so the request shapes are not recoverable from the
  shipped files; `/api/threads` (create) and `/api/thread/:id/message` are all the bundle's own
  analytics names for. Guessing them from a screenshot would be invention, so nothing does.
- No auth flow, no updates, no hosted browser/terminal panels.
- The Overview panel is the app's pre-run state (Environment + Context).

The next step is to map that API from one real exchange: run the installed app, drive one thread
while a loopback proxy records the request/response pairs, and turn the recorded pair into a
client here. Everything under it — spawn, health, lifetime, launch token — is already in place.
