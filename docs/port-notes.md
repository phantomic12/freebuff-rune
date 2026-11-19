# Port notes: the recovered Freebuff Desktop contracts

Every fact here was recovered from the installed app, not from documentation (there is none for
the shell). Sources are the files under `resources/` of the installed desktop app:

```
resources/app.asar            → extracted/app/electron/*.cjs, *.mjs, *.ts
resources/orchestrator/       → orchestrator.js (11 MB Bun bundle, `ui/`, `wasm/`)
resources/bun/                → bun.exe, bun-baseline.exe, ripgrep
```

Hunt method: `tools/extract-asar.mjs` + `tools/scan.mjs` over the bundles, then live probes of a
hand-started orchestrator.

## 1. Two processes, one contract

The desktop app is a front-end to a Bun server:

```
Electron shell ──spawn──▶ orchestrator (bun resources/orchestrator/orchestrator.js, cwd=…/orchestrator)
      │                        │  prints [orchestrator-phase] <step> on stdout
      │                        └─▶ http://127.0.0.1:<PORT>   HTTP + SSE
      └── serves ui/ (Chromium window)
```

The shell is replaceable: it is everything *outside* orchestrator.js. This port replaces the
shell with Rust; the orchestrator is unchanged and still spawns from the installed app's own
`resources/` when the user has Freebuff Desktop.

## 2. Boot sequence (electron/orchestrator-startup.cjs)

`BOOT_PHASES` in order: `started, bootstrap, profile_lock, auth, recover_setups, terminals,
listen, projects_boot_start, projects_boot_end`. Only this whitelist is reported; anything else
on stdout is ignored.

| Line | Meaning |
|---|---|
| `[orchestrator-phase] <step>` | one whitelisted boot step finished |
| `[orchestrator-ready] {"launchId":…,"pid":…,"port":…}` | HTTP server is up |

Budgets (`READY_LINE_BUDGET_MS = 30_000`, `HEALTH_BUDGET_MS = 60_000` in the shipped code):

1. spawn → ready line:  30 s
2. ready line → first healthy `GET /healthz` (`x-freebuff-launch-id` header):  60 s

`/healthz` answers plain `ok` when the orchestrator has no launch id, or
`{"ok":true,"launchId":…,"pid":…,"port":…}` when it does.

## 3. The launch handshake (src/server/launch-bootstrap.ts, electron/main.cjs)

- The shell binds a **shell-lifetime server** on 127.0.0.1 and passes its port as
  `FREEBUFF_SHELL_LIFETIME_PORT`. When the socket closes, the orchestrator shuts down — this is
  how a killed shell does not orphan a server.
- Secrets (launch id, bridge tokens) are passed on the **first stdin line**, not in the
  environment: another process of the same user can read a child's starting environment.
  Line shape: `{"shellLifetimeToken":…,"launchId":…,"cdpBridgeToken":…,"mcpConsentToken":…}`,
  or a bare token string.
- Lifetime handshake (`electron/shell-lifetime.cjs`), exactly:

  1. server writes `challenge\n` (32 random bytes, base64url);
  2. client writes `clientChallenge:HMAC-SHA256(token, "client:"+challenge+":"+clientChallenge)\n`
     (both base64url);
  3. server proves it with `HMAC-SHA256(token, "server:"+challenge+":"+clientChallenge)`;
  4. then `ping\n` every 3 s. The orchestrator tolerates 10 s of silence.

- **Launch token gate**: every `/api/*` path is refused with `401 unauthorized` unless the
  request carries `x-freebuff-launch-id: <launchId>` (or the `freebuff_launch_<port>` cookie).
  The header value is the launchId the shell put on the stdin line. A hand-started orchestrator
  has no launch id and admits everything (verified: `/api/auth/status` returned live JSON).

## 4. Environment

| Variable | Purpose |
|---|---|
| `PORT` | loopback port to bind |
| `FREEBUFF_DESKTOP_STATE_PATH` | profile: `~/.config/freebuff-desktop/state.json` (all platforms) |
| `FREEBUFF_PROFILE_LOCK_WAIT_MS` | how long to wait on the profile lock another orchestrator holds |
| `FREEBUFF_SHELL_LIFETIME_PORT` | lifetime server port |
| `FREEBUFF_UI_DIR` | where `ui/` lives (served at `/`) |
| `CODEBUFF_WASM_DIR`, `CODEBUFF_TREE_SITTER_WASM_PATH` | tree-sitter wasm for code features |
| `FREEBUFF_BUN_PATH` | override the bundled Bun |

Exit 75 with `Another Freebuff orchestrator is already using this Desktop state profile` is the
profile lock — verified live against a second shell.

## 5. Local API surface

The packaged orchestrator's route table is four groups: `appearanceRoutes, authRoutes,
hostedWorkspaceRoutes, diagnosticsRoutes` (found as `routes = [...]` in
`src/server/http/routes.ts`). Everything else the UI needs — threads, chat, events, terminal,
ads, `/api/v1/*` — lives behind the launch token and, on a signed-out profile, 404s.

Verified against the running signed-in app: `/api/threads`, `/api/events` → `401 unauthorized`
without the launch id (same on any `/api/*`).

What the four groups actually contain, read from the bundle — none of them carries threads:

| group | routes |
|---|---|
| `appearanceRoutes` | `POST /api/appearance/wallpapers`, `GET /api/appearance/wallpapers/:filename` |
| `authRoutes` | `/api/auth/status`, `login/start`, `login/cancel`, `logout` |
| `hostedWorkspaceRoutes` | workspace handoff, all of it behind a signed-in session |
| `diagnosticsRoutes` | `POST /api/telemetry/funnel` and its siblings — write-only |

So the theme is a renderer-local setting (nothing to sync), and the only readable state a signed-out
profile offers is `/api/auth/status` — which is exactly what the port uses.

What the shipped shell UI itself calls (all of it):
`/api/auth/status`, `/api/auth/login/start`, `/api/auth/login/cancel`, `/api/auth/logout`,
`/api/diagnostics/renderer-boot`, `/api/telemetry/funnel`. Its network helpers are `Dm(url, opts)`
(fetch + 30 s timeout) and `RS(url, body)` (fire-and-forget POST), both adding
`x-freebuff-launch-id`.

The thread/chat/event request shapes are not recoverable from the shipped files: the interface
that drives them is the signed hot bundle (`orchestrator-bundles/state.json`:
`{"current":null,"bad":[...]}` on this install — none present).

They *are* recoverable from a running session, which is what `tools/record-orchestrator.mjs` is for.
It is a `BUN_OPTIONS` preload: the shell hands its environment to the orchestrator it spawns, so the
recorder loads inside the server without patching the install. It wraps `Bun.serve` and writes one
JSONL line per exchange — path, method, headers with credentials redacted, request body shape,
status, response body shape (streaming responses are logged as metadata only, since reading them
would consume the live channel). `scripts/record.sh` sets that up and launches either this shell or
the installed app.

What it records with nothing signed in:

```json
{"method":"GET","path":"/healthz","res":{"status":200,
  "body":{"ok":true,"launchId":"…","pid":3028,"port":62587}}}
{"method":"GET","path":"/api/auth/status","res":{"status":200,
  "body":{"authed":false,"user":null,"login":{"phase":"idle"},
          "sessionRefunds":{"accountId":null,"items":[]}}}}
```

So `/api/auth/status` carries `user`, `login.phase` and `sessionRefunds` — more than the signed-out
boolean the footer shows, and enough to draw a signed-in profile once there is a session. Thread,
chat and event traffic needs a signed-in run: `scripts/record.sh --app` after closing the app, sign
in, drive one thread, close it. Treat the recording as sensitive; it can hold conversation content.

## 6. What the port implements

| Contract piece | Where |
|---|---|
| spawn + port + phase lines | `app/src/orchestrator.rs` (`launch`, `parse_phase`) |
| ready line + health budgets | `parse_ready`, `poll_health` (30 s / 60 s) |
| stdin bootstrap line | `run` writes `{"shellLifetimeToken","launchId"}` |
| lifetime server + HMAC handshake | `Lifetime::serve`, `keep_alive` (sha2/hmac) |
| launch token on every call | `request(... launch_id)` → `x-freebuff-launch-id` |
| profile + overrides | `Config::discover`, `state_path` |
| live status in the UI | `Live` → sidebar footer (`ui/sidebar.rs`) |

Verified: real boot to `/healthz`, signed-out detection via `/api/auth/status`, orchestrator
exits ≤2 s after the shell is killed, profile-lock failure surfaced with its stderr line.
## 7. Keyboard and window memory

The menu's accelerators survive the port. `app-menu.cjs` (in `app.asar`) declares them as
`CmdOrCtrl+` chords; the native shell binds the same set on Rune's `on_key`, so they work with
the composer focused, which is exactly the case a native app has to defend:

| Menu accelerator | The shell does |
|---|---|
| `CmdOrCtrl+,` | open/close Settings |
| `CmdOrCtrl+T` | open a thread that has no tab yet |
| `CmdOrCtrl+Shift+T` | reopen the last closed tab |
| `CmdOrCtrl+W` | close the active tab |
| `CmdOrCtrl+Shift+W` | close the window |

Two menu entries are deliberately left out: `CmdOrCtrl+R` reloads a page that no longer exists
(the state is the app), and `CmdOrCtrl+O` needs a folder picker this shell does not have yet.
A real *new* thread (`CmdOrCtrl+T`) needs the orchestrator — issue #1685's subject — so while
the fixtures stand in, the chord opens the project's next untabbed thread. Rune delivers a
chord with Shift as the shifted character (`Key::Command('T')`), which the tests encode.

The window's size is remembered across runs. Electron left that to Chromium; a native window
must do it itself. `app/src/window_state.rs` writes two numbers to
`~/.config/freebuff-rune/window.json` (atomic: write to a temp then rename, so a crash cannot
leave a half file), applies the saved size as the native window is created, and refuses a size
that cannot be a window. Position is *not* persisted: Rune can set a window's position but not
read it back, so there is nothing to remember yet — a restored size at the default position is
honest, an invented one would move the window somewhere the person did not put it.
