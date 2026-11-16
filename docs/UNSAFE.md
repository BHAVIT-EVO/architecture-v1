# Unsafe and panic audit

This is the F8 audit of the paths that can be reached from bridge commands,
the `evo-app` scheme handler, voice startup, and the launch agent.

## Reachable fallible operations

| Path | Operation | Handling |
|---|---|---|
| `crates/evo-desktop/src/bridge.rs` | Continue duplicate-guard mutex | Returns `duplicate_guard_unavailable` instead of panicking |
| `crates/evo-desktop/src/webshell.rs` | Bridge inbox mutex | Logs an explicit refusal and drops only the malformed/unavailable message |
| `crates/evo-desktop/src/bin/evo-agent-install.rs` | launchd bootstrap retry result | Uses the final command status directly; reports a typed install error |
| `crates/evo-voice/src/transcribe.rs` | Whisper state initialization | Returns a transcription error if initialization did not produce state |
| `crates/evo-voice/src/driver.rs` | Transcription worker spawn | Records the spawn error and emits a failed voice outcome |
| `crates/evo-voice/src/driver.rs` | Missing command-capture timestamp | Ends the capture cleanly instead of panicking |

Test fixtures still use `unwrap` and `expect` for assertions. They are not
reachable from the application binaries and are intentionally excluded from
the production inventory.

## Unsafe blocks

The macOS UI integration uses `objc`, Core Foundation, Accessibility, and
AppKit FFI. Unsafe blocks are confined to these boundary modules:

- `crates/evo-desktop/src/webshell.rs`
- `crates/evo-desktop/src/websurface.rs`
- `crates/evo-desktop/src/pod_surface.rs`
- `crates/evo-desktop/src/presence.rs`
- `crates/evo-desktop/src/notch_window.rs`
- `crates/evo-desktop/src/bin/evo-agent.rs`
- `crates/evo-desktop/src/state.rs`

Each boundary operation must have a nearby `SAFETY:` comment describing the
pointer validity, selector contract, thread requirement, or ownership rule.
The existing FFI helpers centralize repeated contracts; new unsafe code must
be added through those helpers rather than calling Objective-C symbols inline.

The audit deliberately does not classify test-only fixture setup as a
production failure. Re-run the production search with:

```sh
rg 'unwrap\\(|expect\\(' crates/evo-desktop/src crates/evo-voice/src \
  --glob '*.rs'
```

