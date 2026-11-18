#!/usr/bin/env bash
# Launch the shell in a real window (release build for smoothness).
#
#   scripts/run.sh                  # spawn the installed Freebuff orchestrator, then show the window
#   FREEBUFF_NO_ORCHESTRATOR=1 scripts/run.sh   # fixtures only, start instantly
#   scripts/run.sh --scene light    # any flag the binary takes
#
# What the window does: it finds the installed Freebuff Desktop resources, starts that app's Bun
# orchestrator under a scratch Desktop profile, keeps it alive over the lifetime socket and shows
# the boot phase in the sidebar footer — "Starting…", then "Orchestrator ready", or the reason it
# stopped. Closing the window stops the orchestrator.
set -euo pipefail
cd "$(dirname "$0")/.."

bin=${BIN:-target/release/freebuff-rune}
if [ ! -x "$bin" ]; then
  echo "building $bin" >&2
  cargo build --release -p freebuff-rune
fi

exec "$bin" "$@"
