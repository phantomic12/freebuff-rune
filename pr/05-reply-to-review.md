Fair on both counts. Pushed a provenance section (`c1d1b35`); here is what it says and what I chose not to do.

**Build and artifacts.** Written against installed **Freebuff Desktop `0.0.203`**. The doc now records the SHA-256 of each file a claim was read from — `resources/app.asar`, `app/electron/orchestrator-startup.cjs`, `shell-lifetime.cjs`, `launch-bootstrap.cjs`, `resources/orchestrator/orchestrator.js` — so a diff of the doc against a newer install is a hash comparison first, then a grep.

**Per-claim pointers.** A table maps every constant to a file and line and quotes the literal, so the spot-check is mechanical rather than a re-derivation:

| claim | literal |
|---|---|
| nine phases, in order | `orchestrator-startup.cjs:22`, `const BOOT_PHASES = new Set([` |
| 30 s spawn → ready | `orchestrator-startup.cjs:35`, `const READY_LINE_BUDGET_MS = 30_000` |
| 60 s ready → `/healthz` | `orchestrator-startup.cjs:37`, `const HEALTH_BUDGET_MS = 60_000` |
| exit 75 | `orchestrator.js`, `var PROFILE_BUSY_EXIT = 75` … `process.exit(PROFILE_BUSY_EXIT)` |
| `/api/*` gate | `orchestrator.js`, `pathname.startsWith("/api/")`, `LAUNCH_TOKEN_HEADER = "x-freebuff-launch-id"` |

The bundle still carries its source paths, which is the better pointer for your side: the phase list is `src/server/boot-phase.ts` and the lock and exit code are `src/server/services/orchestrator-lock.ts` — the shell's own comment says the caller's copy and `boot-phase.ts` are held equal by a test, so a mismatch there is a real finding, not a doc bug.

**Nothing needs a build to check.** Two greps, both in the doc:

```bash
npx @electron/asar extract "<install>/resources/app.asar" app
grep -n "READY_LINE_BUDGET_MS\|HEALTH_BUDGET_MS" app/electron/orchestrator-startup.cjs
grep -n "PROFILE_BUSY_EXIT\|invalid launch id\|LAUNCH_TOKEN_HEADER" \
  "<install>/resources/orchestrator/orchestrator.js"
```

**On staleness.** I agree these are exactly the details that are either right or subtly wrong, so the doc now says out loud that nothing is negotiated — no handshake carries a protocol version, so a build that renames a phase or moves a budget breaks a client silently. It separates the durable shape (phase line on stdout, ready line with a launch id, stdin bootstrap, HMAC lifetime socket, launch-id header) from the constants that should be re-read per release, and it states that where the page disagrees with a newer build, the build wins.

If you would rather not carry versioned specifics in a permanent doc at all, the alternative is to cut the constants and keep only the shape — say so and I will trim it; I kept them because a harness that does not know the budgets cannot fail a slow boot.

**On sequencing**, taken: the page no longer leans on the native-shell question. The new `Scope` section says the facts are useful to a test harness or a diagnostics client whether or not a native shell is ever built, and that nothing in it argues for a shell. The companion issue stays the place where that direction is decided — it is now referenced as context only, not as justification.
