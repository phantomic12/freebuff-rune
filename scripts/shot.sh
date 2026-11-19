#!/usr/bin/env bash
# Render the shell to PNGs — the same layout the window shows, at 2x, so the files are postable.
#
#   scripts/shot.sh                 # shots/{dark,light,empty,settings,panel-closed}.png
#   SIZE=1600x1000 scripts/shot.sh  # another window size
#   OUT=shots/pr scripts/shot.sh    # another output directory
#
# Runs against fixtures by design: a screenshot should not depend on a login. For the one that
# shows a live orchestrator line, see the last block.
set -euo pipefail
cd "$(dirname "$0")/.."

bin=${BIN:-target/release/freebuff-rune}
size=${SIZE:-1440x900}
out=${OUT:-shots}

# Build the profile the binary path implies, so passing BIN never builds the wrong one.
if [ ! -x "$bin" ] && [ ! -f "$bin.exe" ]; then
  case "$bin" in
    *release*) echo "building release" >&2; cargo build --release -p freebuff-rune ;;
    *) echo "building debug" >&2; cargo build -p freebuff-rune ;;
  esac
fi

mkdir -p "$out"

# The default state is the dark theme, so it takes no scene name.
"$bin" --snap "$out/dark.png" --size "$size"
for scene in light empty settings panel-closed; do
  "$bin" --scene "$scene" --snap "$out/$scene.png" --size "$size"
done

# The same tree the tests assert against: pane rects in points, not pixels.
"$bin" --tree "$out/tree.json"

# The sidebar footer with a real orchestrator behind it. This spawns the installed app's
# orchestrator under a scratch profile, so it needs a Freebuff Desktop install and takes up to
# ~30 s; it exits with the state it reached either way.
if [ "${LIVE:-0}" = "1" ]; then
  FREEBUFF_SNAP_BACKEND=1 "$bin" --scene backend --snap "$out/backend-live.png" --size "$size"
fi

printf '%s\n' "$out"/*.png | sed 's/^/wrote /'
