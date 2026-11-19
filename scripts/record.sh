#!/usr/bin/env bash
# Record what the desktop app's orchestrator actually serves, as JSONL.
#
#   scripts/record.sh                 # launch OUR shell with the recorder loaded
#   scripts/record.sh --app           # launch the installed Electron app instead
#   OUT=rec/today.jsonl scripts/record.sh
#
# Either way the orchestrator inherits BUN_OPTIONS from the shell process, so the recorder loads
# inside the server without patching the install. Then drive the app as a user would, close it, and
# read the log: every request with its path, headers, body shape and status.
#
# This is how the unrecoverable part becomes recoverable. The shipped route table only exposes auth,
# appearance, hosted-workspace and telemetry; threads, chat and the event stream exist behind the
# launch token, and their request shapes can only be learned by watching a real session. Sign in
# first if you want those, and mind the note at the top of tools/record-orchestrator.mjs: the file
# can contain conversation content.
set -euo pipefail
cd "$(dirname "$0")/.."

out=${OUT:-rec/orchestrator-$(date +%Y%m%d-%H%M%S).jsonl}
mkdir -p "$(dirname "$out")"

# Bun takes a --preload on the command line or via BUN_OPTIONS; the shell passes its environment to
# the orchestrator it spawns, so BUN_OPTIONS is enough. The path must be absolute and Windows-shaped
# for Bun, which does not read a POSIX path.
recorder=$(cd tools && pwd -W 2>/dev/null || pwd)/record-orchestrator.mjs
export BUN_OPTIONS="--preload $recorder"
export FREEBUFF_RECORD_OUT="$(cd "$(dirname "$out")" && pwd -W 2>/dev/null || pwd)/$(basename "$out")"

echo "recording to $out" >&2

if [ "${1:-}" = "--app" ]; then
  # The installed app picks its own loopback port and its own profile; nothing to configure.
  app=${FREEBUFF_APP_EXE:-"$LOCALAPPDATA/Programs/@codebufffreebuff-desktop/Freebuff.exe"}
  [ -x "$app" ] || { echo "no installed app at $app (set FREEBUFF_APP_EXE)" >&2; exit 1; }
  "$app"
else
  ./scripts/run.sh "$@"
fi

echo "recorded $(grep -c '' "$out" 2>/dev/null || echo 0) lines to $out" >&2
