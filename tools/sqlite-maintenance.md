# Evo SQLite maintenance runbook

The live store (`~/Library/Application Support/evo/evo.db`) is written by the
`evod` daemon binary. On 2026-09-09 its WAL had grown to 4.7 GB because no
connection ever set `journal_size_limit` and the file was never truncated:
frames were checkpointed into the main database all along, but a WAL file is
only ever *shrunk* by an explicit `TRUNCATE` checkpoint.

## What was done (2026-09-09)

1. Diagnosed with `PRAGMA wal_checkpoint(PASSIVE)`:
   `busy=0 log_frames=109 ckpt_frames=1` — nothing pinned, tiny active
   region, so the 4.7 GB was dead never-truncated history.
2. `PRAGMA wal_checkpoint(TRUNCATE)` — first attempt returned `busy=1`
   (the daemon held the write lock); one retry between the daemon's
   transactions succeeded. WAL: 4702 MB -> 0 MB. Integrity check: `ok`.

## One-time reclaim (safe to rerun anytime)

```bash
python3 - <<'EOF'
import sqlite3, os, time
db = os.path.expanduser("~/Library/Application Support/evo/evo.db")
for attempt in range(12):
    con = sqlite3.connect(db, timeout=2)
    try:
        busy, _, _ = con.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()
        if busy == 0:
            print("truncated")
            break
    finally:
        con.close()
    time.sleep(3)
EOF
```

Every attempt is safe: `busy=1` is a no-op, never a corruption risk.

## The durable fix (daemon rebuild)

The reclaim is a stopgap: the WAL regrows until the *writer* sets limits.
When the daemon is next rebuilt from source, its SQLite connection must run
these pragmas once on open:

```sql
PRAGMA journal_mode       = WAL;
PRAGMA synchronous        = NORMAL;       -- safe in WAL; fsync per checkpoint
PRAGMA wal_autocheckpoint = 1000;         -- ~4 MB of writes per auto-checkpoint
PRAGMA journal_size_limit = 536870912;    -- 512 MB hard ceiling on the WAL file
PRAGMA busy_timeout       = 5000;
```

`journal_size_limit` is the seatbelt: after any successful checkpoint the
file is truncated back under the ceiling, so even a pathological writer can
never again accumulate gigabytes. Group-committing observation rows (200
events or 500 ms per transaction, whichever first) shrinks WAL growth a
further 5-10x because page dirtiness amortizes across the batch.

Scheduled checkpoints, from the daemon, logged:

- Hourly, when OS idle >= 60 s and the WAL exceeds 256 MB:
  `PRAGMA wal_checkpoint(RESTART)`.
- Nightly, after `settle` and before rendering the morning's resume
  bundles: `PRAGMA wal_checkpoint(TRUNCATE)`; log `(log_frames, ckpt_frames)`.

Gauges worth alerting on: `wal_bytes > 512 MB for > 1 h` (seatbelt holding,
something pins) and a `busy=1` streak longer than 6 attempts.

## Why the log is the real escape hatch

Once the observation log (§3 of the capture-layer design) becomes the
source of truth, the SQLite file is a derived cache whose only writer is a
projector that can be re-run from the log wholesale. A wedged database then
stops being an incident: drop it and re-project.
