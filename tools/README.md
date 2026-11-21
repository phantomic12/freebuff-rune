# tools

Four helpers used to read the reference artifacts this port was built from. They operate on
files that are **never committed** (`extracted/`, `upstream-freebuff/` are gitignored), and on a
copy of Freebuff Desktop the user already has installed. They are provenance, not build steps:
nothing here runs in CI or in the app.

| Tool | What it answers |
|---|---|
| `extract-asar.mjs <app.asar> <out>` | Unpacks an Electron `app.asar` without Electron: parses the header's JSON and copies each file out. |
| `scan.mjs <file> <word…>` | Counts and locates strings in a minified bundle — how the orchestrator's route list and the boot phase names were found. |
| `shot-uia.ps1` | Screenshots a running `freebuff-rune` window through `System.Windows.Automation` (PowerShell, no `Add-Type -TypeDefinition`, which some AV products block). |
| `record-orchestrator.mjs` | A `BUN_OPTIONS` preload that records every exchange the orchestrator serves, as JSONL: bodies shortened to their shape, credentials redacted, streams tapped frame by frame. Driven by `scripts/record.sh`. |

Typical use:

```bash
node tools/extract-asar.mjs \
  "$LOCALAPPDATA/Programs/@codebufffreebuff-desktop/resources/app.asar" extracted/app
node tools/scan.mjs extracted/orchestrator.js text/event-stream
powershell -NoProfile -ExecutionPolicy Bypass -File tools/shot-uia.ps1
```

The recorder's knobs: `FREEBUFF_RECORD_OUT` (file), `FREEBUFF_RECORD_MAX` (byte budget, default
20 MB), `FREEBUFF_RECORD_LONG` (string cut-off, default 48 — raise it when the run has to replay an
id back), `FREEBUFF_RECORD_STREAM_FRAMES` (frames per stream, default 400). It records loopback
traffic from a signed-in app, so treat its output as sensitive and delete it after the shapes are
written down; `rec/` is gitignored for that reason.

Findings from these runs live in `docs/port-notes.md`. The scripts are deliberately dumb: they
read, they print, they do not modify the installed app.
