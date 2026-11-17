

**What problem are you trying to solve?**

The desktop app is Electron: Chromium + Node. That costs a full browser runtime for what is a
client of a local HTTP server — the window process is mostly idle between turns, yet it carries a
browser's memory and startup cost, and the renderer is only reachable from inside an Electron
window. I rebuilt the shell outside Electron to see how much of that is avoidable, and to check
whether the UI layer can be verified without launching a GUI.

The result is a working native shell: the same tokens, geometry and panes drawn by Rune's GPU and
text stack, no Chromium, no Node. It boots the *own* installed orchestrator unchanged — same
spawn, same stdin bootstrap line, same lifetime handshake, same `/healthz` budgets — and shows the
orchestrator's state in the sidebar. Panes are asserted headlessly from a test harness
(`--snap` renders a PNG, `--tree` dumps rects), which the Electron shell cannot do.

**Where it stops, precisely:** it draws the interface and supervises the process, but it reads
exactly one payload, `/api/auth/status`. Threads, chat streaming and ads still come from
fixtures, because the request shapes of those endpoints live in the shell code that is not in this
repository — a shipped install exposes the compiled bundle, not the client that drives it.

**What would you like to happen?**

Either of these, and I am happy with whichever you prefer:

1. **Take the contract as documentation.** `docs/desktop-shell-contract.md` (PR ready) records
   what a desktop shell must do to drive the orchestrator: the boot-phase whitelist, the two
   readiness budgets, the stdin bootstrap line, the HMAC lifetime socket, the
   `x-freebuff-launch-id` gate on `/api/*`. That is useful to anyone writing a client, and it
   needs no Rust and no new toolchain.
2. **Consider the native shell as an experiment.** With the contract above as the map, the
   remaining work is one recorded API session per feature (threads, streaming, ads) — a loopback
   proxy in front of a running app records the pair, and the shapes become implementable. I would
   open that as a separate PR once the shapes are known, under whatever path you name, with Rune
   pinned in your build instead of vendored.

If the direction is wrong for the product, say so and I will stop at the documentation.

**Area:** Desktop app

**Contribution:** I would be willing to open a PR for this

---

I can attach a screenshot of the native window beside the Electron one, the `--tree` dump the
tests assert against, and a short recording (spawn → phases → ready → footer "Orchestrator
ready"; then killing the shell and the orchestrator stopping within 2 s) on request, or put the
prototype in a repo if you want to read the code — I did not want to publish one before asking
whether the direction makes sense.
