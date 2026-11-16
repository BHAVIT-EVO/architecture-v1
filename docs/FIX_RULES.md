# FIX RULES
- Find the root cause before changing code: reproduce it and show the failing evidence (log line, failing test, measured value). No guess-fixes.
- Every fix gets a regression test that failed before and passes after. Show both runs.
- Never touch my real Evo data. Use the isolation mechanism from F1 (EVO_TEST_HOME).
- Do only the numbered task. No refactors, no redesign, no unrelated edits. If you find something else broken, append it to docs/fix/FOUND.md and move on.
- Small commits, one per fix: "fix(<area>): <what>".
- Never say fixed or working without evidence from this session. If you cannot test it, write NEEDS-HUMAN with exact steps for me.
- End every session with a reply in exactly this format:
  1) ROOT CAUSE (one paragraph, with evidence)
  2) CHANGES (files and what changed)
  3) EVIDENCE (before and after output, test names)
  4) STILL BROKEN OR UNVERIFIED
  5) NEEDS-HUMAN (exact steps for me)

  ## Added rules
- Never write "remains as follow-up" or "due to scope". Finish every numbered step, or stop with BLOCKED: the exact error, file and line, and what you tried.
- Paste real command output for every claim (cargo test, selftest.sh, grep). No summaries.
- A claim may appear in the UI only if a test or measurement proves it.

## Added rules (round 3)
- Do not run graphify or refresh any graph in fix sessions. Your reply starts with the five-part format, never with a tool summary.
- Do the numbered steps in order. If a step is impossible, write BLOCKED with the exact error and continue with the next step that doesn't depend on it.
- Never mark a step done without pasting its output in this reply.