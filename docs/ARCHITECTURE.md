# Peregrine — Architecture

This is the developer-facing companion to the [README](../README.md). It describes how
the code is organized and, more importantly, the **one invariant that must never
break**: the egress chokepoint.

## The stack

- **Shell:** [Tauri 2](https://tauri.app) — a ~5 MB, low-privilege desktop app. Chosen
  because Peregrine's whole promise is "small, local, auditable." Installs entirely in
  user space (no admin).
- **UI:** React + TypeScript (Vite), in the webview.
- **Backend:** Rust (`src-tauri/`), which owns all privileged capability —
  filesystem, network, and (later) the local database.

## The egress chokepoint — the invariant

```
   ┌──────────────────────────────┐
   │  Web UI (React)              │   CSP: connect-src 'self'
   │  — network-sandboxed —       │   → CANNOT reach the internet
   └──────────────┬───────────────┘
                  │  Tauri command (IPC) only
                  ▼
   ┌──────────────────────────────┐
   │  Rust backend (src-tauri)    │   The ONLY code that can touch the network.
   │  ┌────────────────────────┐  │
   │  │ Egress guard           │  │   Every outbound request is checked against
   │  │  · allowlist           │  │   the allowlist and written to the Activity
   │  │  · Activity log        │  │   log BEFORE a byte leaves.
   │  └───────────┬────────────┘  │
   └──────────────┼───────────────┘
                  ▼
        chosen model endpoint  +  read-only connected sources
        (nothing else)
```

**Rule:** the web UI never makes network calls. It is sandboxed by the Content Security
Policy in `src-tauri/tauri.conf.json` (`connect-src 'self'`). Anything that needs the
network goes through a `#[tauri::command]` in Rust, and every such command routes through
the egress guard. This is what makes "watch the wire and verify us" true rather than
aspirational. **Do not add network access to the frontend, and do not add a Rust network
call that bypasses the guard.**

## Modules (target shape)

| Layer | Home | Responsibility |
|---|---|---|
| Coworker core | `src/` + `coworker_reply` command | Chat, project review, trajectory, brainstorm |
| Career-memory store | Rust + local encrypted SQLite | `signals` and `accomplishments`, with provenance |
| Model router | Rust | Bring-your-own-model / local model; the trust dial |
| Egress guard | Rust | Allowlist + Activity log — the chokepoint above |
| Renderers | `src/` | Résumé / LinkedIn / self-review views over the store |
| Connectors | Rust (read) + UI (review-and-publish) | One-click *read*; human always publishes |

## Current status (v0 skeleton)

What exists today:

- Tauri + React shell that builds and runs.
- `coworker_reply` command — returns an **honest placeholder** (no model wired yet).
- `activity_log` command — returns empty, because nothing has left the machine.
- Trust posture surfaced in the UI (egress badge + Activity panel).

What's intentionally *not* here yet: the SQLite store, the model router, real
connectors. Those land next, each behind the chokepoint above.

## Build order (why this sequence)

1. **Prove the shell** (done) — toolchain builds and runs end-to-end.
2. **Career-memory store** — the moat is worthless without somewhere to put it.
3. **Model router + trust dial** — bring-your-own-model, egress-guarded.
4. **Accomplishment engine** — cluster → quantify → score.
5. **Renderers**, then **connectors** (read first, publish always human-in-the-loop).

The coworker has to be genuinely useful *before* the career layer matters — so quality
of (1)–(3) comes before breadth of (5).
