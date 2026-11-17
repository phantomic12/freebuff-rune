<!--
PR body for the ready-to-send branch `docs/desktop-shell-contract` (patch in this directory).
Title is the commit subject; both satisfy .github/workflows/pr-hygiene.yml (≥15 chars, ≥3 words,
a real description, no forbidden paths).
-->

**Title:** Document the desktop orchestrator's shell contract

---

**What this changes**

Adds `docs/desktop-shell-contract.md`: the boundary between the desktop shell and the
orchestrator it starts, written down once instead of re-derived per client.

It covers:

- the two-process shape (free port, `127.0.0.1`, cwd, stdout as the progress channel);
- the nine boot-phase names and the ready line, with the shell's two budgets (30 s spawn → ready,
  60 s ready → `/healthz`);
- the stdin bootstrap line, and why secrets are not in the environment;
- the HMAC lifetime socket — challenge/response, 3 s pings, 10 s tolerance — which is how a
  killed shell takes its server down instead of orphaning it;
- the `x-freebuff-launch-id` (or `freebuff_launch_<port>` cookie) gate that answers
  `401 unauthorized` on every `/api/*` path;
- the environment variables a shell sets, including the profile path and the lock-wait that fails
  with exit 75.

**Why**

Everything above is observable from a shipped install but undocumented, and the observations are
easy to get subtly wrong — the phase whitelist, the two different timeouts, and the fact that the
launch id travels on stdin rather than in the environment are all things a second implementer
would otherwise rediscover by trial and error. This is the documentation half of an experiment
that replaced the shell with a native UI; the shell code is not part of this PR, and the issue
linked below asks whether it should be.

No product behaviour changes: documentation only. It touches no path outside `docs/`.

Filed as an issue first (area: Desktop app) because a native shell is a direction question, not a
docs question — if the answer is "not the direction", this page is still true and still useful.
