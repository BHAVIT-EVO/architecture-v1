# GLOSSARY

Every internal word the interface shows **today**, the plain word that replaces it, and what the word means in the code. Screen words only — a term that never reaches a screen is not listed.

Status: proposed for approval. Nothing here is applied yet.

| On screen today | Say instead | What it means in code |
|---|---|---|
| "Capture" (Settings) | **Watching** | `DaemonStatus::Capturing`: the capture worker is recording observations. |
| "Capture running" / "partially running" / "unavailable" / "failed" | **Watching / partially watching / not watching / stopped** | `daemon::DaemonStatus::label()` over `CaptureStatus`. |
| "Capture worker" | **Evo's background helper** | The `evo-daemon` process that writes observations (`daemon.rs`). |
| "Evo is recording what it witnesses." | "Evo is watching what you do." | `copy::SETTINGS_CAPTURE_RUNNING`. Nothing is audio- or screen-recorded. |
| "Some capture sources are unavailable; window focus is not permitted." | "Evo can't see which window is in front. Accessibility permission is off." | `PartialCapture { detail }`. |
| "Evo cannot observe your work yet." | "Evo isn't watching yet." | `DaemonStatus::Unavailable`. |
| "witnessed" / "Witnessed" (tab) | **saw** / **What Evo saw** | `evo-observation` evidence accepted from a granted source; `detail.rs` RecordTab::Witnessed. |
| "Nothing witnessed yet" | "Nothing here yet." | Empty projection (`state.rs`). |
| "Evo has not witnessed anything in this work yet." | "Evo hasn't seen anything in this work yet." | Empty snapshot list. |
| "Loading witnessed work…" | "Loading your work…" | `copy::HOME_LOADING`. |
| "record" / "its record" / "Evo could not read its record" | **history** / "Evo couldn't read your history" | The append-only local store: `observation.log` + `artifact.log` written by `evo-daemon::persistence`. |
| "Your record lives on this Mac" | "Your history stays on this Mac." | First-run storage step (`copy::FIRST_RUN_STORAGE_TITLE`). |
| "reading its local work record" | "reading your history" | `shell::loading`. |
| "workspace" | **work** | `evo_workspace::Workspace` — one body of work. |
| "Preparing your workspace" | "Opening your windows" | `ContinuePhase::Preparing` (`detail.rs`). |
| "Work tiles" / "workspace layout" (Settings) | "Window layout" | `evo_pods::layout::LayoutId` preference. |
| "pod" / "In: {pod}" | **Open: {work}** | A `Pod` is an entered work: other apps hide, its windows rise (`evo-pods`). |
| "Leave pod" / "Eject" | **Leave** | `DockAction::Eject`; restores the desktop exactly. |
| "Next pod ⌘⇧E" / "Pod Bar ⌥Space" | "Next work" / "Work switcher" | Pod cycling and the strip (`presence.rs`, `pod_bar_drag.rs`). |
| "Pod not entered: restoration failed." | "Evo couldn't open that work." | `PodHost::enter_at` returned no successful target. |
| "Library" (dock) | **Home** | `DockAction::Library` → `ShellView::Home`. |
| "stack" / "More in the stack" / "Hide the stack" | **the rest** / "Show the rest" | `stack_open`: works beyond the third card (`rack.rs`). |
| "Surface" (Record tab) / "witnessed surface" | **What Evo saw** / "thing Evo saw" | `RecordTab::Surface`; a witnessed window/page/resource. |
| "Exact surface no longer available." | "That page or window isn't open any more." | `ScopedOutcome::Failed` from the daemon. |
| "declared" / "commit-derived" / "inferred" | "you said so" / "from your commits" / "worked out by Evo" | `MemberStateSource::{Declared, Derived, Inferred}` in `skipped_members`. |
| "Skipped members: 2 declared, 3 inferred" | "3 things Evo isn't sure about" | `DisplayThread.skipped_members`. |
| "artifact" — "Opened 1 artifact" (notch detail) | "Opened 1 thing" | `evo-artifact::Artifact`: one file, page or document. |
| "daemon" — "The daemon did not report…" | "Evo's background helper" | `evo_daemon`, the always-on process. |
| "next refresh" / "on the next refresh" | "in a moment" | The projection is rebuilt from the log; there is no refresh the person performs. |
| "Continuable" / "Restorable" / "Remembered" (work view eyebrow) | "Ready to continue" / "Can reopen" / "Only remembered" | `evo_engagement::Standing`. |
| "Looks related to X (3 shared sessions) — merge?" | "This looks like the same work as X. Join them?" | `rack::related_proposal`; `sittings` is already the plain word for a shared stretch of attention. |
| "Restore" (notch chip) | **Continue** | `OptionAction::Restore`; the dock and cards already say Continue. |
| "Browse" (notch chip) | unchanged | `OptionAction::OpenRack`. |
| "Diagnostics", "Privacy & trust", "Coming soon", "Not available in this build" | unchanged | Screen labels and an honest absence marker. |
| "Stays on this Mac" (not built yet) | keep, only while true | Audio and text are processed and stored locally; see SPEC §5 for the four conditions. |

## Words that stay exactly as they are

Continue · Watching · Leave · Home · Search · Settings · Help · Diagnostics · Dusk / Day / Auto · Evo.

## Never on a screen

"user", "AI-powered", "seamless", "unlock", "magic", "Oops", "Something went wrong", exclamation marks, emoji. Say what happened, then what to do.
