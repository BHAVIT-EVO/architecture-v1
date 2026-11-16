# ADR 0001 — A native macOS shell, not a web view

**Status:** accepted (Phase 0)

## Context

The mockup is HTML, so a web view (Tauri, Electron) would have rendered it
almost for free. Evo needs a window that is not an app window: an
always-on-top panel above the menu bar, a menu-bar item, and a microphone
indicator that survives losing focus.

## Decision

The desktop app is Rust with egui/eframe and direct AppKit calls through the
Objective-C runtime. There is no Tauri, no Electron and no HTML shell. The one
WebKit use is an optional child view for a work's own web pages
(`websurface.rs`).

## Consequences

- The mockup's canvas has to be ported (ADR 0002), and every component is
  rebuilt in egui from the same tokens.
- The notch can be an `NSPanel` at level 27 with its own thread, which a web
  view cannot do.
- Measurements are taken in two places: the native surface (egui) and a web
  reference implementation used for the design proof (ADR 0005).
