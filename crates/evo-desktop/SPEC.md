# Evo Desktop UI specification

## Scope

This spec covers the presentation and user interaction owned by `evo-desktop`. The canonical observation/work model, capture, storage formats, voice protocol, permissions, and restoration APIs remain owned by their existing crates and are not changed by this UI pass.

## Product contract

- Home presents only witnessed work supplied by the runtime. It does not invent activity, location, project type, pinning, or recency. Work that the engine does not consider eligible remains distinct from eligible work.
- Continue remains an explicit request to restore the currently eligible work. Search narrows the existing list; selection navigates to the corresponding detail screen where the existing identity bridge permits.
- Capture status and permission controls reflect the daemon's reported state. Accessibility controls open macOS System Settings and ask the worker to re-check; they do not grant permission themselves.
- First run explains what the desktop can and cannot promise. It describes local desktop storage without implying that every optional agent capability is enabled.
- Settings offer Dusk (default), Day, and Auto appearance; show the selected appearance, capture state, local storage location, privacy/trust information, basic usage help, and local diagnostics. Appearance is presentation preference, not canonical work state.
- Voice/notch behavior remains owned by the already-existing agent/notch runtime. The desktop must not imply its window starts a microphone listener. Any state not available to this desktop is stated as unavailable rather than inferred.
- Existing empty, storage-error, capture-limited, permission-required, and restoration outcome states remain honest; no disabled/decorative control suggests an unimplemented action.

## Visual system

The product direction is **Evo's Clearing**: Dusk by default, an optional warm-paper Day palette, and Auto following the host's reported system appearance. Leaf green is the only product accent; status errors and warnings remain distinguishable. Instrument Serif and JetBrains Mono are bundled under their existing SIL Open Font License. UI text uses the macOS system face with a bundled/system fallback. Spacing, type roles, contrast, reduced surprise motion, and keyboard dismissal are centralized in theme/UI primitives.

## Trust boundaries (current build)

- The desktop's canonical store is under `~/Library/Application Support/Evo/storage` unless `EVO_STORAGE_ROOT` is explicitly used for development/testing.
- Capture is performed by the Evo worker process and is subject to platform permissions and runtime source availability. The shell only reports the worker's state.
- The desktop window does not start the microphone. The optional `evo-agent` voice worker can capture microphone audio and run local transcription when built/configured with voice features and models; it uses a local Unix socket to communicate with the daemon.
- There is no desktop account, sync, analytics, or diagnostic-upload setting in this build. This spec does not claim that all platform processes or bundled dependencies are incapable of network access.
- Accessibility is granted by macOS, not by Evo; the UI can open the relevant privacy pane and restart/recheck the worker.

## Accessibility and interaction

Controls keep their labels visible, use a shared emphasis hierarchy, and reflow rather than clip. Status is expressed with text as well as color. Modals are dismissible with Escape. Keyboard and pointer interactions should preserve the same explicit confirmation behavior. Keep animation short and nonessential; do not communicate state by color alone.

## Verification

Run workspace formatting, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and available desktop interaction/UI checks. Report actual failures and platform limitations. Passing a build is not a claim of launch readiness for optional features or unverified macOS integration.
