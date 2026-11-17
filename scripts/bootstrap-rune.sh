#!/usr/bin/env bash
# Rune is pre-release and consumed by revision, and the workspace's `[patch.crates-io]` entries
# point into Rune's own vendored crates — so a fresh clone cannot build until `vendor/rune` exists
# at the recorded revision. This fetches exactly that, and nothing else.
#
#   ./scripts/bootstrap-rune.sh          # clone at the pinned revision
#   ./scripts/bootstrap-rune.sh --check  # verify an existing clone, clone nothing
#
# Set RUNE_REMOTE to clone from a mirror.
set -euo pipefail
cd "$(dirname "$0")/.."

rev="$(tr -d '[:space:]' < vendor/rune-rev.txt)"
remote="${RUNE_REMOTE:-https://github.com/anomalyco/rune}"
dest="vendor/rune"

check_only=0
[ "${1:-}" = "--check" ] && check_only=1

if [ -d "$dest/.git" ]; then
	have="$(git -C "$dest" rev-parse HEAD)"
	if [ "$have" = "$rev" ]; then
		echo "vendor/rune already at $rev"
		exit 0
	fi
	if [ "$check_only" = 1 ]; then
		echo "vendor/rune is at $have, expected $rev" >&2
		exit 1
	fi
	echo "vendor/rune is at $have; switching to $rev"
	git -C "$dest" fetch --depth 1 origin "$rev"
	git -C "$dest" checkout --detach "$rev"
	exit 0
fi

[ "$check_only" = 1 ] && { echo "vendor/rune is missing" >&2; exit 1; }

echo "cloning $remote at $rev"
mkdir -p vendor
git init -q "$dest"
git -C "$dest" remote add origin "$remote"
git -C "$dest" fetch --depth 1 origin "$rev"
git -C "$dest" checkout -q --detach "$rev"
echo "vendor/rune at $rev"
echo "next: cargo build -p freebuff-rune   (first build compiles ~190 crates)"
