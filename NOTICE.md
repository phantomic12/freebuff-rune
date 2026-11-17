# Notices

## This repository contains no Freebuff code

`freebuff-rune` is an independent user interface written from scratch in Rust. It does not
include, embed, or redistribute any part of Freebuff Desktop, Codebuff, or their bundles.

Everything in this repository is either:

- original Rust source (`app/`, `scripts/`, `docs/`),
- small reading tools (`tools/`) that operate on a copy of the installed app **on the machine
  that already has it**, or
- the vendored framework below.

### Relationship to Freebuff Desktop

The port was studied from an installed copy of Freebuff Desktop: its published interface design
(tokens, geometry, pane composition) and the observable behaviour of its local orchestrator
(boot lines, ports, HTTP surface). **No Freebuff source code is included, copied, or
redistributed here.** The fixtures in `app/src/fixtures.rs` are fictional sample content, and the
shell speaks only to the user's own locally-installed orchestrator when the user runs it.

The Apache License 2.0 below is the same license Freebuff Desktop itself ships under (its
`LICENSE` names Freebuff, Inc.), chosen so the derivation carries no ambiguity about what may be
reused. That choice grants no trademark rights: "Freebuff" is used nominatively to describe the
compatible product; neither this project nor its authors are affiliated with or endorsed by
Freebuff or Codebuff.

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