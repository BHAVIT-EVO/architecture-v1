# The bridge

The page talks to Rust through one channel and one only: `window.webkit
.messageHandlers.evo`. `ui/app/bridge.js` is the only file that touches it. In
a normal browser there is no handler, and `bridge.js` serves the local sample
corpus instead — never in the app: `scripts/build-app.sh` fails if
`ui/data/works.json` is bundled.

## Shape

Request (page → Rust), one JSON **string** per message (`postMessage` takes
the serialised object; the handler parses it):

```json
{ "id": 7, "cmd": "rename_work", "work": 12, "title": "The pitch deck" }
```

Reply (Rust → page), always with the same `id`:

```json
{ "id": 7, "ok": true, "data": { "work": 12, "title": "The pitch deck", "forwarded": true } }
{ "id": 7, "ok": false, "error": { "code": "paused", "message": "Evo is paused. Resume before opening anything." } }
```

Rust calls `window.__evoReply(json)` for every message, including refusals, so
no promise can hang. `bridge.js` ignores replies whose id it does not know
(late answers after a timeout).

## The allowlist

| Command | Fields | Returns | What it does |
|---|---|---|---|
| `get_works` | — | `{ works: [{ id, title, source, last_active_ms, resumable }], total }` | `display_threads_from_storage`, then the canonical-index fallback when the engine groups nothing; names and hidden ids applied from `ui-state.json` |
| `continue_work` | `work` | `{ status, items: [{ label, ok, reason }], hint, action }` | checks the stored targets, uses plain `open` for files, folders and URLs, records every attempt in Activity, and returns `opening`, `opened`, `partly_opened`, or `couldnt_open`; duplicate requests within three seconds return `opening` with `hint: "Already opening"` |
| `rename_work` | `work`, `title` | `{ work, title, forwarded }` | saves the name in `ui-state.json`, and forwards it as a `Naming` declaration when the daemon is up |
| `merge_works` | `first`, `second` | `{ merged }` | requires a shared witnessed resource (`grouping_subjects_from_storage`), then `submit_grouping`; a pairing without evidence is refused with `no_shared_evidence` |
| `hide_work` | `work`, `undo` | `{ hidden, changed, work }` | persisted and reversible: `undo: true` removes exactly that id |
| `search` | `query` | `{ results: [{ id, title, score, matched, start_ms }], query }` | `Ledger::search` (FTS5) over `ledger.db` |
| `get_status` | — | `{ paused, capture, listening }` | `paused` from disk, `capture` from the daemon lock, `listening` honestly `false` (the desktop shell does not own the mic) |
| `get_voice_events` | — | `{ events: [{ type, ... } ] }` | Reads the agent-owned event queue. Types are `listening`, `live_transcript`, `thinking`, `choices` (at most three), `opened`, `not_found`, `no_speech`, `mic_denied`, and `agent_error`. |
| `pause` / `resume` | — | `{ paused }` | writes the kill switch; every later `continue_work` reads it from disk |
| `get_about` | — | `{ version, build_time, commit }` | `CARGO_PKG_VERSION` and the `EVO_BUILD_TIME` stamp `build.rs` writes |

Anything else is `unknown_command`. There is no generic invoke, no `eval`, and
no command takes a path.

`continue_work.items[].reason` is one of `file_moved`, `app_missing`,
`no_target`, `permission_needed`, `browser_not_running`, or `other`. The
single `action` is `Find the file`, `Allow access`, or `Try again`.

## Limits

| Limit | Value | On breach |
|---|---|---|
| Message size | 8 KiB | `too_large`, refused unread |
| `title` | 120 characters, non-blank after trim | `invalid_args` |
| `query` | 200 characters, non-blank | `invalid_args` |
| Unknown keys | none allowed per command | `invalid_args` |
| Wrong types | e.g. `"work": "nine"` | `invalid_args` |
| Malformed JSON / not an object | — | `invalid_json` / `invalid_shape` |
| Missing `id` | — | `missing_id` (page cannot correlate, so it is refused) |

## Error codes

`unknown_command`, `invalid_json`, `invalid_shape`, `missing_id`,
`missing_command`, `invalid_args`, `too_large`, `paused`, `no_shared_evidence`,
`rejected` (daemon declined), `backend` (I/O, socket, database).

## Events (Rust → page)

The installed agent writes `agent.build-id` after startup. The desktop compares
it with its own version/build stamp and kickstarts `com.evo.agent` when the
values differ, preventing an old agent from handling a new app's commands.
Microphone startup failures are emitted as `mic_denied` when the operating
system reports permission-like failure; the page offers the Microphone pane in
System Settings. Other startup failures remain `agent_error`.

Reserved and delivered through `window.__evoEvent`, which re-dispatches as an
`evo-event` DOM event: `works_changed`, `listening`, `paused`. They are
informational only. The page never relies on them for safety: a stale page
claiming `paused: false` still gets `paused` from `continue_work`.

## Who may talk

The `WKScriptMessageHandler` accepts a message only when

1. it comes from the **main frame** (`frameInfo.isMainFrame`), and
2. that frame's URL passes `navigation_allowed` — `evo-app://app/…` only.

Anything else is dropped before parsing, with a log line. Commands run on a
worker thread; replies are injected on the UI thread.

## What the page never receives

No file paths, no environment values, no tokens, no transcripts, no window
titles of other apps beyond the work name already shown on Home, and nothing
from `~/Library/Application Support` except counts and flags. Permissions and
the kill switch are decided in Rust; the page has no command that grants
anything.
