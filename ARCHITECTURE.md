# Architecture

Evo is a native macOS shell over a local, deterministic runtime. Nothing in the
default build talks to the network.

## The shape

```
witness (capture)  →  observation log  →  daemon (canonical owner)  →  ledger
                                                     │
                                             desktop shell (egui)
                                                     │
                                   Home · scene · notch · settings · tray
```

| Layer | Crate | Owns |
|---|---|---|
| Capture | `evo-capture` | window/AX evidence, input counters; never writes canonical state |
| Observation | `evo-observation` | candidate validation, canonicalization, provenance |
| Artifact | `evo-artifact` | what a piece of work *is* (identity, integrity) |
| Ledger | `evo-ledger` | `ledger.db`, ordered canonical records |
| Threads | `evo-threads` | grouping, assignment, retrieval over the ledger |
| Workspace | `evo-workspace` | snapshots of "where the work is" |
| Restoration | `evo-restoration` | how a work is brought back |
| Execution | `evo-execution` | launching apps, tabs and windows within scope |
| Voice | `evo-voice` | mic, wake word, transcription — all on device |
| Daemon | `evo-daemon` | the runtime owner; validates every declaration |
| Desktop | `evo-desktop` | the window, the notch, the menu bar, the scene |

The desktop shell duplicates no semantics. It reads a projection from the
daemon and renders it.

## Where the interface lives

Two surfaces share one design language:

- **Native** (`crates/evo-desktop`): Home, the notch window (`NSPanel`), the
  menu-bar item, Settings, Help, Diagnostics. Painted with egui and the
  Objective-C runtime; there is no HTML.
- **Web** (`ui/`, `site/`): the design-system reference (`ui/gallery.html`),
  the scene port (`ui/scene/`), the Home shell used for measurement
  (`ui/app/`), and the public website (`site/`).

`ui/tokens.css` and `ui/scene/` are the single source for colour, type and the
scene; both surfaces read from them.

## Data and trust boundaries

- **Audio** never leaves RAM. It is transcribed by a local whisper model and
  dropped.
- **Text** goes over a per-user Unix socket to the local daemon.
- **Storage** is `~/Library/Application Support/Evo/storage/` — see
  [`PRIVACY.md`](PRIVACY.md).
- **Permissions** are decided in `crates/evo-desktop/src/security/permissions.rs`:
  kill switch → scope → mode. See [`SECURITY.md`](SECURITY.md).
- **Commands** are argument arrays only (`security::paths::CommandLine`); paths
  are canonicalized and containment-checked before use.

## Tests

- Unit tests beside each module (`cargo test --workspace`).
- Integration tests: `crates/evo-desktop/tests/journeys.rs` (first run, first
  wake, open a recent, pause, refused permission), `trust_constraints.rs`
  (no network, no screen capture, voice opt-in only).
- Web: `node --test "home/*.test.js"` in `ui/`.
- Measurement harnesses live in `notch_window.rs` and print their percentiles.

## Decisions

Recorded in [`docs/adr/`](docs/adr/).
