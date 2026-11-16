# Contributing

Evo is a small app with strong rules. Read these before a change:

1. [`PROJECT_BRIEF.md`](PROJECT_BRIEF.md) — what the product is.
2. [`docs/DESIGN_LAW.md`](docs/DESIGN_LAW.md) — how it looks, moves and speaks.
3. [`PRIVACY.md`](PRIVACY.md) and [`SECURITY.md`](SECURITY.md) — what it may do.

If a change contradicts the design law, the law wins unless the law is amended
first (and the amendment is in the same pull request).

## Before you write

- One screen or component at a time. Files stay under about 300 lines.
- No new dependency without a reason in the pull request body; no HTTP client,
  no telemetry SDK, no screen-capture crate (`trust_constraints.rs` will fail).
- Permission is asked per scope. A new capability gets a scope, an action, and
  a row in the permission UI — not a settings toggle.

## Commands

```sh
cargo fmt --check                 # formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # unit + integration
cargo test -p evo-desktop --test trust_constraints
cargo test -p evo-desktop --test journeys
cd ui && node --test "home/*.test.js"
cargo audit                       # advisories; see SECURITY.md for the open one
```

Everything above runs in CI (`.github/workflows/ci.yml`). A pull request that
skips a check says why in its body.

## Style

- Plain sentences, sentence case, second person; buttons are verbs; no
  exclamation marks, no emoji, no internal terms (`GLOSSARY.md`).
- Errors say what happened and what to do; empty states teach in one sentence
  with one action.
- User-visible strings live in a copy module (`crates/evo-desktop/src/copy.rs`,
  `ui/app/copy.js`). No inline strings in new surfaces.
- No `unwrap`/`expect` in a user-facing path; return a typed error and show it.
- Every `unsafe` block gets a comment saying what invariant it relies on.

## Privacy

Never log a transcript, a file path, a window title or an email address. The
home directory is shortened to `~` by `observability::field_value`. If a change
means audio, text or a report could leave the device, update `PRIVACY.md`, the
"Stays on this Mac" copy and the trust-constraints test in the same commit.

## Commits

Small, named for the phase (`phase-5: gate the wake pipeline`). Describe why in
the body, not just what.
