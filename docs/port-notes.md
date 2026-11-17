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

What the shipped shell UI itself calls (all of it):
`/api/auth/status`, `/api/auth/login/start`, `/api/auth/login/cancel`, `/api/auth/logout`,
`/api/diagnostics/renderer-boot`, `/api/telemetry/funnel`. Its network helpers are `Dm(url, opts)`
(fetch + 30 s timeout) and `RS(url, body)` (fire-and-forget POST), both adding
`x-freebuff-launch-id`.

The thread/chat/event request shapes are not recoverable from the shipped files: the interface
that drives them is the signed hot bundle (`orchestrator-bundles/state.json`:
`{"current":null,"bad":[…]} ` on this install — none present). The next step is a loopback proxy
in front of a real session, recording one exchange per feature.

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