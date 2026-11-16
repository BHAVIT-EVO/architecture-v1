# Evo — Privacy

Last updated 13 November 2026. This file is the plain-language source of truth
for what Evo processes and where it goes. `PROJECT_BRIEF.md` §9 and
`DESIGN_LAW.md` §6 govern how it is shown in the app; if a claim here stops
being true, the claim changes before the release ships.

## The short version

Nothing leaves this Mac. Evo has no account, no server, no sync and no
analytics. Audio and text are processed locally; microphone buffers are dropped
after transcription.

## What Evo processes

| Data | Why | Where it lives |
|---|---|---|
| The title and source of the window you are in | to offer the work back to you | in memory, then Evo's local store |
| The document a window belongs to (path) | to reopen it at the right place | Evo's local store |
| Microphone audio while Evo listens | to detect “Hey Evo” and hear the command | memory only, dropped after transcription |
| Your typed or spoken command | to match a work and act | memory only |
| Your settings, permissions and activity | so the app behaves as you chose | Evo's local store |

Evo does **not** capture the screen, record keystrokes, record system audio, or
collect a device identifier.

## Where it lives

```
~/Library/Application Support/Evo/storage/
├── observation.log     window observations (plain text)
├── artifact.log        files and documents seen
├── ledger.db           the work ledger (SQLite)
├── daemon.status       the daemon's status line
├── pods/               pod membership
└── preferences/        settings and permissions
```

The directory is mode 700. Logs and per-user sockets are mode 600. The data is
still readable by other processes running as your user, so “private on this Mac”
would overclaim. There is no encryption at rest today.

## Where audio and text go

- Microphone → CoreAudio (cpal) → a 16 kHz buffer in memory.
- Wake word → openWakeWord ONNX models loaded **from beside the executable**.
- Speech → whisper.cpp on Metal (`whisper-base.en.bin`, also beside the executable).
- Text → a per-user Unix socket in the temporary directory, then the local daemon.

No model is downloaded at runtime. No HTTP or TLS client is linked into the
workspace; `crates/evo-desktop/tests/trust_constraints.rs` fails the build if
one appears, if screen-capture or OCR crates are added, or if a non-loopback
socket is introduced.

The optional `fastembed` feature is off by default. If it is enabled, it can
fetch model assets; builds that ship must leave it off or this document changes.

## Permissions

Evo asks per scope, never globally.

- **Scopes**: folders, apps, sites.
- **Actions**: open, read, write, run.
- **Modes**: ask every time, ask for risky actions (default), act within scope.

The decision function lives in `crates/evo-desktop/src/security/permissions.rs`
and is enforced in this order: kill switch → scope → mode. A target outside
every scope is refused in every mode. Writing and running are the risky
actions; opening and reading are not. Nodes above the scope (a parent folder)
are not granted by a child scope.

## The kill switch

One switch, shared by the notch, the menu bar, Settings and the keyboard
shortcut. Pausing denies every permission decision, calls pause on the CoreAudio
input stream, turns the listening indicator to **Paused**, and queues nothing.
Resuming calls play on the same stream. If the agent is not running, the state
is **Off**.

## Activity

Every action Evo takes is written to Activity with what, when and why, newest
first, capped at 100 entries. Hiding, merging and renaming offer Undo, once.
Activity stores work titles, never transcripts, and shortens your home
directory to `~`.

## Reports

Report a problem shows the exact payload before anything happens: app version,
macOS version, microphone state and the last few activity lines. Nothing is
sent until you approve it, and you can copy it instead of sending.

## Export and delete

- **Export** writes a JSON file with your works, settings, permissions and
  activity to a location you choose.
- **Delete everything** removes the works, settings, permissions and activity
  from this Mac, and makes the next launch a first launch.

## Secrets

This build holds no API keys or tokens, so there is nothing to put in the
keychain yet. When an integration needs a secret it goes in the macOS keychain
(`Security.framework`), never in a file store, and the app reads it at the
moment of use.

## Dependencies and supply chain

`cargo audit` and `cargo deny` are the intended gate on every dependency
change; `clippy -D warnings` and `cargo test` run with them. Unsafe blocks are
reviewed where they appear, and the Objective-C runtime work in
`notch_window.rs` and `presence.rs` is the only place Evo talks to AppKit.

## What would change these claims

- Adding sync, accounts or crash uploads.
- Enabling `fastembed` or any runtime model download.
- Adding an integration that stores a token.
- Writing audio to disk.

Each of those changes this file, the “Stays on this Mac” chip, and the Privacy
and Trust screen in the same commit.

## Measurement record

The isolated bridge self-test passed on this machine:

```text
pause bridge check: {"data":{"capture":"stopped","listening":false,"paused":true,"voice_state":"paused"},"id":3,"ok":true}
quit bridge check: {"data":{"quitting":true},"id":4,"ok":true}
selftest OK
```

The required live network measurement was not captured because the installed
`com.evo.agent` launchd job was not present when this check ran. Do not treat
“Nothing leaves this Mac” as a measured release claim until the following
manual check is completed while saying a real command:

```sh
launchctl kickstart -k gui/$(id -u)/com.evo.agent
PID="$(launchctl print gui/$(id -u)/com.evo.agent | awk '/pid =/ {print $3; exit}')"
lsof -a -p "$PID" -i
nettop -P -L 10 -p "$PID" -J bytes_in,bytes_out
```

Expected result: no non-loopback network rows and zero network bytes during
the command. Paste the actual output here before shipping.

## Needs-human checks

- Light menu bar: launch Evo, close the window, and verify the template tray
  icon remains visible and readable.
- Dark menu bar: repeat with macOS in Dark appearance.
- Launch at login: enable the login item in System Settings, log out/in, and
  verify the tray item and agent appear.
