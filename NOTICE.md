# Notices

## This repository contains no Freebuff code

`freebuff-rune` is an independent user interface written from scratch in Rust. It does not
include, embed, or redistribute any part of Freebuff Desktop, Codebuff, or their bundles.

Everything in this repository is either:

- original Rust source (`app/`, `scripts/`, `docs/`),
- small reading tools (`tools/`) that operate on a copy of the installed app **on the machine
  that already has it**, or
- the vendored framework below.

Freebuff Desktop itself is a third-party product; the artifacts it installs stay on the user's
machine in `extracted/` and `upstream-freebuff/`, both gitignored, and are not distributed with
this repository. Fixtures in `app/src/fixtures.rs` are fictional sample content, not the app's
data. The shell talks to the user's own locally installed orchestrator at their request; it ships
none of it.

## Rune

`vendor/rune` is a clone of [anomalyco/rune](https://github.com/anomalyco/rune) at the revision in
`vendor/rune-rev.txt`, fetched by `scripts/bootstrap-rune.sh` and not committed here.

```
MIT License

Copyright (c) 2026 Anomaly
```

Rune's own vendored dependencies (`vendor/rune/vendor/*`) keep their upstream licenses, which are
carried in that clone.

## Fonts

The app draws with system fonts through Rune's text stack. No font files are included here; the
type names in `app/src/tokens.rs` are references, not redistributed assets.
