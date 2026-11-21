// BUN_OPTIONS preload that records every HTTP exchange the orchestrator serves, as JSONL.
//
// The desktop shell spawns its Bun orchestrator with an inherited environment, so this file can be
// injected without touching the install:
//
//   export BUN_OPTIONS="--preload $(cygpath -m "$PWD/tools/record-orchestrator.mjs")"
//   export FREEBUFF_RECORD_OUT="$PWD/orchestrator-recording.jsonl"
//   scripts/run.sh                       # our own shell's boot traffic
//   start Freebuff.exe                   # the real Electron app's traffic
//
// It wraps Bun.serve, so it sees exactly what the server sees: path, method, headers, request body,
// status, response body. Nothing is filtered by the recorder itself except secrets — see redact().
//
// Caveats, stated rather than hidden:
//   * GET request bodies are not read (they are almost always absent, and reading them is costly).
//   * Streaming responses are tapped rather than skipped: the body is re-emitted to the app
//     unchanged while a copy of each frame (SSE `event:`/`data:` or newline-delimited JSON, which is
//     what localStream() uses) is recorded, bounded by FREEBUFF_RECORD_STREAM_FRAMES.
//   * Uploads (POST with a file) have their body length recorded, not their bytes.
//   * The file grows to FREEBUFF_RECORD_MAX bytes and then stops; keep it bounded.
//   * This records loopback traffic of the desktop app. It is a diagnostics tool, not a toy: delete
//     the file when you are done, and never share it — it may contain conversation content.

import { appendFileSync, statSync } from "node:fs";

const OUT = process.env.FREEBUFF_RECORD_OUT ?? "orchestrator-recording.jsonl";
const MAX = Number(process.env.FREEBUFF_RECORD_MAX ?? 20 * 1024 * 1024);
const CLEANED = /^(authorization|cookie|x-freebuff-launch-id|set-cookie)$/i;
// A body whose keys or values look like credentials is recorded as a shape, never as content.
const SECRET_VALUE = /(token|secret|bearer|password|cookie)/i;
// Long strings in an otherwise-fine body are truncated: an id is a shape, its full value is not.
const LONG_STRING = Number(process.env.FREEBUFF_RECORD_LONG ?? 48);
// Streaming bodies are unbounded in principle, so a stream records at most this many frames and
// then keeps flowing to the app untouched.
const MAX_FRAMES = Number(process.env.FREEBUFF_RECORD_STREAM_FRAMES ?? 400);

let bytes = safeBytes();
let lines = 0;
let stopped = false;

function safeBytes() {
  try {
    return statSync(OUT).size;
  } catch {
    return 0;
  }
}

function emit(entry) {
  if (stopped) return;
  lines += 1;
  const line = JSON.stringify(entry) + "\n";
  bytes += Buffer.byteLength(line);
  if (bytes > MAX) {
    emitNote("recorder hit its byte budget; stopping");
    stopped = true;
    return;
  }
  try {
    appendFileSync(OUT, line);
  } catch {
    // No point interrupting the app because the recorder cannot write.
  }
}

function emitNote(note) {
  try {
    appendFileSync(OUT, JSON.stringify({ ts: Date.now(), note }) + "\n");
  } catch {
    /* ignore */
  }
}

function clampHeaders(headers) {
  const out = {};
  for (const [k, v] of headers) {
    if (CLEANED.test(k)) {
      out[k] = "<redacted>";
      continue;
    }
    out[k] = v.length > 512 ? v.slice(0, 512) + "…" : v;
  }
  return out;
}

function shapeOf(text, contentType) {
  if (SECRET_VALUE.test(text)) {
    // A token-shaped body is still worth recording as a shape, never as content.
    return { kind: "body-with-possible-secret", bytes: text.length };
  }
  if (contentType?.includes("json")) {
    if (text.length > 4096) return { kind: "json-truncated", bytes: text.length, head: text.slice(0, 512) };
    try {
      return { kind: "json", value: shorten(JSON.parse(text)) };
    } catch {
      return { kind: "json-unparsed", bytes: text.length, head: text.slice(0, 512) };
    }
  }
  return { kind: "text", bytes: text.length, head: text.slice(0, 512) };
}

// Walk a parsed body and cut long strings, keeping the shape readable.
function shorten(value, depth = 0) {
  if (depth > 6) return "…";
  if (typeof value === "string") return value.length > LONG_STRING ? value.slice(0, LONG_STRING) + "…" : value;
  if (Array.isArray(value)) return value.slice(0, 12).map((v) => shorten(v, depth + 1));
  if (value && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) {
      out[k] = SECRET_VALUE.test(k) ? "<redacted>" : shorten(v, depth + 1);
    }
    return out;
  }
  return value;
}

function record(req, res, t0, reqBody, resText) {
  const url = new URL(req.url);
  const contentType = res.headers?.get?.("content-type") ?? "";
  emit({
    ts: Date.now(),
    ms: Math.round((performance.now() - t0) * 10) / 10,
    method: req.method,
    path: url.pathname + (url.search || ""),
    req: {
      headers: clampHeaders(req.headers),
      body: reqBody === null ? null : shapeOf(reqBody, req.headers.get("content-type") ?? ""),
    },
    res: {
      status: res.status,
      headers: clampHeaders(res.headers),
      body: resText === null ? null : shapeOf(resText, contentType),
    },
  });
}

// Read a streamed body without changing it: each chunk goes to the app first, then a copy is
// buffered and cut into frames. SSE frames end at a blank line; newline-delimited JSON at a newline.
function tapStream(res, label, path) {
  const state = { buffer: "", frames: [], sent: 0, note: null };
  const flush = () => {
    if (state.note || state.frames.length === 0) return;
    emit({ ts: Date.now(), path, stream: label, status: res.status, frames: state.frames });
    state.frames = [];
  };
  const decoder = new TextDecoder();
  const sse = /text\/event-stream/i.test(res.headers?.get?.("content-type") ?? "");
  const sep = sse ? "\n\n" : "\n";
  const tap = new TransformStream({
    transform(chunk, controller) {
      controller.enqueue(chunk);
      if (state.note) return;
      let text;
      try {
        text = decoder.decode(chunk, { stream: true });
      } catch {
        return;
      }
      state.buffer += text;
      for (;;) {
        const at = state.buffer.indexOf(sep);
        if (at < 0) break;
        const frame = state.buffer.slice(0, at);
        state.buffer = state.buffer.slice(at + sep.length);
        if (!frame.trim()) continue;
        state.frames.push(frameShape(frame, sse));
        if (++state.sent > MAX_FRAMES) {
          state.note = `stream stopped after ${MAX_FRAMES} frames`;
          emit({ ts: Date.now(), path, stream: label, note: state.note });
          state.frames = [];
          return;
        }
        if (state.frames.length >= 40) flush();
      }
      if (state.buffer.length > 64 * 1024) {
        state.frames.push({ kind: "unterminated", bytes: state.buffer.length });
        state.buffer = "";
      }
    },
    flush() {
      if (state.buffer.trim()) state.frames.push(frameShape(state.buffer, sse));
      flush();
    },
  });
  return new Response(res.body.pipeThrough(tap), res);
}

function frameShape(frame, sse) {
  if (sse) {
    const event = /^event:\s*(.+)$/m.exec(frame)?.[1] ?? null;
    const data = /^data:\s*([\s\S]*)$/m.exec(frame)?.[1] ?? "";
    return { event, data: dataShape(data) };
  }
  return { data: dataShape(frame) };
}

function dataShape(text) {
  const trimmed = text.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      return { kind: "json", value: shorten(JSON.parse(trimmed)) };
    } catch {
      return { kind: "json-unparsed", bytes: trimmed.length, head: trimmed.slice(0, 240) };
    }
  }
  return { kind: "text", bytes: trimmed.length, head: trimmed.slice(0, 240) };
}

const serve = Bun.serve;
Bun.serve = (options) => {
  const inner = options?.fetch;
  if (typeof inner === "function") {
    options.fetch = async (req, server) => {
      const t0 = performance.now();
      let reqBody = null;
      if (!["GET", "HEAD"].includes(req.method)) {
        try {
          reqBody = await req.clone().text();
        } catch {
          reqBody = null;
        }
      }
      let res;
      try {
        res = await inner(req, server);
      } catch (err) {
        emit({ ts: Date.now(), error: String(err?.message ?? err) });
        throw err;
      }
      const contentType = res.headers?.get?.("content-type") ?? "";
      // A stream must not be cloned and awaited: that would hold the app's body until the stream
      // ends, which for a live run is never. Hand the tap back immediately instead.
      const streamed =
        /text\/event-stream/i.test(contentType) || contentType.includes("application/x-ndjson");
      if (streamed && res.body !== null) {
        record(req, res, t0, reqBody, null);
        return tapStream(res, contentType.includes("ndjson") ? "ndjson" : "sse", new URL(req.url).pathname);
      }
      let resText = null;
      if (res.body !== null) {
        try {
          // A clone tees the body, so the app still receives all of it.
          resText = await res.clone().text();
        } catch {
          resText = null;
        }
      }
      record(req, res, t0, reqBody, resText);
      return res;
    };
  }
  return serve(options);
};

process.on("exit", () => emitNote(`recorder closed after ${lines} exchanges`));
