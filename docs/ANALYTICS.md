# Analytics and crash reports

Both are **off by default**. Nothing is sent from this build — there is no
client to send with (see `crates/evo-desktop/tests/trust_constraints.rs`), so
an opted-in record is written next to the log and goes nowhere until a
transport exists and this page says so.

## The event list

Exactly these names are accepted (`crates/evo-desktop/src/observability.rs`,
`EVENTS`). Any other name is refused by `Analytics::record`, so a caller cannot
invent a metric.

| Event | When | Fields |
|---|---|---|
| `app_launch` | the window starts | version, crash reports on/off |
| `onboarding_finished` | the fourth setup step completes | none |
| `work_opened` | a work is brought forward | none |
| `search_used` | the ⌘K dialog returns a result | none |
| `voice_wake` | the wake word is detected | none |
| `voice_failed` | the voice engine could not start or died | reason code |
| `pause_toggled` | the kill switch flips | state |
| `report_opened` | Report a problem is opened | none |

Never included: work titles, file paths, window titles, transcripts, audio,
site names, app names, or any identifier that could name a person. The values
that are written go through `field_value`, which shortens `$HOME` to `~` and
truncates at 200 characters.

## Crash reports

- The hook always prints `EVO-PANIC: <message> at <file>:<line>` to stderr and
  then runs the previous hook, so a panic is never swallowed.
- With crash reports on (`EVO_CRASH_REPORTS=1`, or the preference the app
  writes), the same record is appended to `crashes.jsonl` beside the log.
- The record carries the panic message and the source location only. No
  backtrace with registers, no environment, no files.

## Logs

`~/Library/Application Support/Evo/logs/evo.jsonl`, one JSON object per line:

```json
{"level":"info","event":"app_launch","version":"0.1.0","crash_reports":"off"}
```

Rotated at 1 MiB into `evo.1.jsonl` and `evo.2.jsonl`; older archives are
removed. A missing or unwritable directory makes `write` return `false` rather
than panic, so a full disk cannot take the UI down.

## Turning it on and off

- Analytics: `Analytics::new(false)` unless the user opts in in Settings →
  Privacy and Trust.
- Crash reports: the environment flag above, wired in `main.rs`.
- Both are visible in Diagnostics (log files, bytes, crash count).
