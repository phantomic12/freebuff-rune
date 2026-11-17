# Upstream submission

Fork and branch, ready to open:

| | |
|---|---|
| Fork | `phantomic12/freebuff` (GitHub, via `gh repo fork`) |
| Branch | `docs/desktop-shell-contract` — rebased on upstream `main` (`23eff4e`), 1 ahead, 0 behind |
| Issue | **<https://github.com/CodebuffAI/freebuff/issues/1685>** (open) |
| PR | **<https://github.com/CodebuffAI/freebuff/pull/1686>** (open, `MERGEABLE`, PR-hygiene check passed) |
| File | `docs/desktop-shell-contract.md` (4.4 KB) |
| Local clone | `upstream-freebuff/` — `origin` = upstream, `fork` = ours |

Verification done:

- hygiene rules (`.github/workflows/pr-hygiene.yml`): title `Document the desktop orchestrator's
  shell contract` — 47 chars / 6 words, not branch-like, no WIP marker; description ≥ 30 chars;
  only `docs/` touched — none of `web/`, `freebuff/web/`, `packages/internal|billing|bigquery/`.
- the patch (`0001-…patch`) applies clean on a fresh `origin/main`.
- the branch exists on the fork with the expected commit (`3f3d505`) and file.

## Open the PR

```bash
gh pr create --repo CodebuffAI/freebuff \
  --head phantomic12:docs/desktop-shell-contract \
  --title "Document the desktop orchestrator's shell contract" \
  --body-file pr/03-pr-body-desktop-shell-contract.md
```

Compare URL (works as soon as the branch is pushed, i.e. now):
<https://github.com/CodebuffAI/freebuff/compare/main...phantomic12:docs/desktop-shell-contract>

## Order matters

`pr/01-issue-desktop-native-shell.md` is the issue that should be opened (area: Desktop app)
*before or alongside* the PR — the PR body says so explicitly, because the maintainers' triage
closes unilateral product-direction changes, and this PR sets up the conversation rather than
runs past it. The docs are the in-scope half; the native shell is the follow-up the issue asks
about.

If the fork is ever re-created, re-add the remote and rebase:

```bash
cd upstream-freebuff
git remote add fork https://github.com/phantomic12/freebuff.git
git fetch origin main && git rebase origin/main
git push -u fork docs/desktop-shell-contract
```