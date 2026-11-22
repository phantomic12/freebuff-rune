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

The thread/chat/event request shapes are not in the shipped files: the interface that drives them
is the signed hot bundle (`orchestrator-bundles/state.json`: `{"current":null,"bad":[...]}` on this
install — none present). They are, however, recoverable from a running session, and §8 is the
result of doing it.

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
boolean the footer shows, and enough to draw a signed-in profile once there is a session. A
signed-in boot is where the rest of the surface shows up: §8 records one, and lists what is still
out of reach (the message stream, which needs a turn run). Treat a recording as sensitive; it can
hold conversation content, which is why the shapes in §8 are written with the values dropped.

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
| transcript as parts | `Part`, `Message`, `Thread::read` (`app/src/model.rs`) ← §9 |
| cursor paging window | `Thread::history` → `ui/transcript.rs` (`load-earlier` row) |
| hosted-workspace client | `Endpoint::{bootstrap, authorize, projects, threads, read}` (`app/src/remote.rs`) ← §10 |
| live sync into the sidebar | `spawn_sync` (`app/src/main.rs`) → `State::apply_snapshot`, `apply_transcript` |

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

## 8. The hosted-workspace surface, as recorded

The shipped route table in `orchestrator.js` lists four route groups; what the renderer actually
calls was learned by recording a real boot, not by reading the bundle. `tools/record-orchestrator.mjs`
is a Bun preload that wraps `Bun.serve` and writes one JSONL line per exchange (`scripts/record.sh
--app` launches the installed app with it). One signed-in boot, 83 exchanges, no window driven:

```
GET  /healthz                             200  {ok, launchId, pid, port}
GET  /                                    200  the renderer's index.html
GET  /assets/*, /fonts/*                  200  the UI bundle and font
GET  /api/auth/status                     200  {authed, user:{id,name,email}, login:{phase}, sessionRefunds:{accountId, items[]}}
GET  /api/hosted-workspace/status         200  polled 23 times in that boot
POST /api/diagnostics/renderer-health     202  {rendererPrivateKb, rendererWorkingSetKb, sampledAt, systemFreeKb, systemTotalKb}
POST /api/hosted-workspace/*              200/409
```

The interesting part is the second group: it is **one endpoint per resource with an `operation`
field**, not REST verbs. Every body carries `accountId`, and the reply is the whole slice of state
the caller needs, not a mutation result — the shell re-renders from it.

| Endpoint | `operation` values seen | Reply, top level |
|---|---|---|
| `POST /api/hosted-workspace/local-conversations` | `authorize`, `projects`, `threads`, `workspace`, `save-workspace` | `{projects[], threads[], nextOffset}`, `{ok}`, `{workspace}` |
| `POST /api/hosted-workspace/status` | — (`trigger: project-selection`, `automatic-retry`) | `{accountId, deviceId, enabled, projectSetupCompleted, connections[], localProjects[]}` |
| `POST /api/hosted-workspace/runtime` | — | `{choices:[cloud, local], location, workspace:{projectName, projectPath, directory, branch, worktree}, supportsOpener, supportsWorktreeTitle, supportsWorktreeSourceBranch}` |
| `POST /api/hosted-workspace/browser` | `action.action: status` | `{contexts[]}` each with `{projectId, threadId, tabs[], history[], profiles[], activeTabId, defaults:{colorScheme, profileId, zoom, viewport}}` |
| `POST /api/hosted-workspace/git` | `changes`, `summary` (+ `scope: all`, `live`) | `{branch, files[], totals:{adds, dels, files}, scope}` |
| `POST /api/hosted-workspace/tasks` | `list` | `{error}` here — needs a reconnected folder |
| `POST /api/hosted-workspace/elevations` | — | `[]` |
| `POST /api/hosted-workspace/native-agents` | `status` | `{agents[]:{harness, installed, connected, signedIn, canInstall, needsUpdate, models[]:{id, label, tagline, efforts[]}}}` |
| `POST /api/hosted-workspace/appearance` | — | `{installationId, preferences:{selectedPreset, customPresets[]}}` |
| `POST /api/hosted-workspace/profile-notice` | — | `{notice}` |
| `POST /api/hosted-workspace/sites-context` | — | `{error}` when the folder is not reconnected |

A thread record, which is what the sidebar renders and what this port's fixtures imitate:

```json
{"id": "<uuid>", "title": "…", "draft": false, "activeAt": 1791322132776,
 "running": false, "archived": true, "canContinue": true,
 "harness": "codebuff", "mode": "build"}
```

`harness` is one of `codebuff`, `claude-code`, `codex` — the thread remembers which agent runs it,
and `mode` is `build` (the app's Code/Chat switch is a different thing: it is the
`mode` of the *composer*, and `build` is what the recorded threads carry).

**409 is a state, not a failure.** Any resource operation that needs a local folder answers
`409 {"error": "Reconnect this local folder to use its tools"}` when the workspace connection is
not up. The renderer draws that string; it is not a toast-worthy error.

**What is still not recorded:** the live turn stream — see §9, which reads a real transcript and
explains why a turn never crosses loopback at all. The recorder now taps streams rather than
skipping them (SSE frames by `event:`/`data:`, newline-delimited JSON by line, since that is what
`localStream` in `main.cjs` returns), bounded by `FREEBUFF_RECORD_STREAM_FRAMES`, but on this app
the tap has nothing to catch: the streams that exist are the Publish progress and the hosted page's
own network, and only the first is local.

Also learned the hard way, and worth writing down because it changes how the app can be observed:
`app/electron/debug-switches.cjs` refuses to start a packaged build if `--remote-debugging-port`,
`-pipe` or `-address` appears in argv (case-folded, and `/`-spelled on Windows), because such a
switch would let any process running as the user drive the signed-in app. A DevTools endpoint is
therefore not available on the installed app; the orchestrator preload is. The shell exits before
its first log line when handed one of those flags.

## 9. Reading a transcript, and where the turn actually happens

The gap §8 left open — the message stream — turned out not to be a gap in the recorder. It is not
on loopback at all, and finding that out is worth more than the stream would have been.

### The window is a remote web app

`hosted-workspace.cjs` opens the workspace with `POST /api/hosted-workspace/bootstrap {launchId}`
and loads whatever `bootstrap.url` says, in a session partition keyed by `bootstrap.accountId`,
carrying `bootstrap.sessionToken`. The accepted origins are
`staging-2-auth.freebuffweb.site`, `staging-auth.freebuff.com` and `freebuff.com`. The bundled
`resources/orchestrator/ui/` is not that page: it is the splash and the sign-in page, and it calls
exactly six endpoints (`/api/auth/*`, `/api/diagnostics/renderer-boot`, `/api/telemetry/funnel`).

So the local orchestrator is a **capability bridge** for a page that lives on the vendor's origin:
browser panes, git, files, terminal, elevations, local history. The page cannot call it directly —
every call goes through an `ipcMain.handle` in the main process, which is where the
`x-freebuff-launch-id` header is added and where the per-operation argument whitelist lives
(`hosted-local-conversations.cjs` re-checks it: an operation it does not know, or an extra field,
is a thrown error).

**What that means for a native shell**, and it is the honest answer to the issue's question: a port
cannot stop at re-implementing loopback. The thing the person is looking at is a web app on
`freebuff.com`, so a native UI has to be a *client of the cloud API* — the product-shaped decision,
not a rendering exercise. Loopback is the small, well-specified half.

### The transcript is a pull, not a subscription

`POST /api/hosted-workspace/local-conversations` has these operations, from
`hosted-local-conversations.cjs`'s `fields` table (each with its exact allowed arguments):

| operation | arguments |
|---|---|
| `authorize` | — (once per window, before any other operation) |
| `projects` | `offset` |
| `threads` | `projectId`, `offset` |
| `workspace` | — |
| `save-workspace` | `tabs`, `activeId` |
| `search` | `projectId`, `query` |
| `read` | `projectId`, `threadId`, `before`, `after` |
| `migrate` | `projectId`, `threadId`, `priority`, `retry` |
| `attachment` | `projectId`, `threadId`, `attachmentId` |

`read`, recorded against a real thread, answers with the whole conversation window:

```json
{
  "thread": {"id","title","activeAt","running","archived","canContinue","harness","mode"},
  "history": {"firstSeq","lastSeq","totalMessages","messageOffset","userOffset",
              "loadedBytes","hasOlder","hasNewer"},
  "messages": [
    {"seq": 42, "role": "user" | "assistant", "inputId": "…",
     "parts": [{"id","kind": "text" | "reasoning" | "tool" | "ad", …}]}
  ],
  "models": {"model","effort","harness","cliModels":[],
             "catalog": {"version","protocol","issuedAt","refreshAt","fetchId",
                         "fallbackKey","recommendedKey","hideModelPicker","plansUrl",
                         "showAllModels": {"default"},
                         "sections": [{"id","label","tooltip"}],
                         "rows": [{"key","handle","displayName","tagline","contextWindow",
                                   "efforts","defaultEffort","reasoningEffort","premium",
                                   "access","multimodal","dataUse","badges","legacyDigests",
                                   "placement","compaction","lockedLabel","opensAt","peakPills"}]}},
  "runtime": {"choices","location","locked"},
  "queued": [], "questions": [], "elevations": []
}
```

Three things in there are worth naming:

- **`before` / `after` are cursors, not offsets.** The reply says which way more history lies
  (`hasOlder`, `hasNewer`) and how much it loaded (`loadedBytes`), so a client pages by sequence
  number and never asks for the whole thread.
- **A message is parts, not text.** `kind: "tool"` carries `toolName` plus the raw tool `input`
  (for a shell tool: `command`, `process_type`, `timeout_seconds`); `kind: "reasoning"` is the
  visible thinking; `kind: "text"` is the prose. A transcript renderer that assumes a string per
  message cannot draw this UI.
- **`kind: "ad"` is a part too.** Twelve of them came back in one thread, each
  `{ad: {title, adText, cta, url, favicon, clickUrl, impUrl, placementId}}`. The ads ship inside
  the transcript as an ordinary part — which settles the question of whether a port should offer a
  switch: upstream has no toggle to mirror, because the creative is content.

`queued`, `questions` and `elevations` are the three queues the window draws beside the transcript:
a message waiting to be sent, a question the agent asked, and an approval request. All empty here.

### So what is still unrecorded

The live token stream of a running turn — and it cannot be recorded from this side, because the
turn is between the hosted page and `freebuff.com`. The recorder sees no `text/event-stream` on
loopback across 190 recorded exchanges, and that is the correct result rather than a shortcoming:
loopback never carries a turn. What loopback does carry is what the shell contributes to one:
`runtime` (where it will run), `prepare-chat` / `release-chat` (claims the folder for a thread),
`files`, `git`, `browser` and `terminal` (the tools the agent asks the shell for).

## 10. Reading the person's own threads

§9 said a native shell can only be a client of what the orchestrator answers. This is that client,
written and tested: `app/src/remote.rs` makes the four calls a reading window needs, and the model
is what they land in.

| Call | Body | Lands in |
|---|---|---|
| `POST /api/hosted-workspace/bootstrap` | `{launchId}` | the account id (§9: the reply also carries the hosted `url`) |
| `local-conversations` | `{operation:"authorize", accountId}` | — (the window's one-time handshake) |
| `local-conversations` | `{operation:"projects", accountId, offset}` | sidebar projects |
| `local-conversations` | `{operation:"threads", accountId, projectId, offset}` | sidebar threads |
| `local-conversations` | `{operation:"read", accountId, projectId, threadId, after}` | the transcript window |

Every one of them is a POST of one object and carries `x-freebuff-launch-id`; a non-2xx answer is
the orchestrator's own sentence (`{"error": "Reconnect this local folder to use its tools"}` is a
*state*, not a crash) and surfaces as the text of an `Err`.

`spawn_sync` in `app/src/main.rs` runs on a background thread once the boot says it is *signed in*:
bootstrap → authorize → projects → threads, then one update that replaces the fixtures with the
real sidebar and hands the window a channel. Opening a thread sends `Want { project, thread }` on
that channel; the same thread answers with `read` and updates the model, so the window never waits
on a socket and the person can keep clicking while it loads. A signed-out boot says so once on
stderr and leaves the fixtures alone — nothing pretends to be data it isn't.

Three rules the model keeps, because a real account will violate the obvious assumptions:

- **A project with no threads is not a sidebar entry**, and an account with no projects is one
  placeholder (`No projects yet` / `Nothing open yet`). Every renderer here indexes
  `projects[active]`, so an empty vector would be a panic, not an empty state.
- **A transcript may arrive after the person moved on**, so the update names its project *and*
  thread ids and is dropped if that thread is no longer in the model.
- **Only the visible window is fetched** (`after: 0`, the first page). `history.hasOlder` is what
  the `load-earlier` row draws; paging backwards is the same call with `before`.

What this cannot do is *write*: sending a turn is the hosted page's `POST` to the cloud, not a
loopback operation (§9), so the composer still stops at the model. That is the product-shaped gap
issue #1685 is about, and it is now the only one left in the shell.
