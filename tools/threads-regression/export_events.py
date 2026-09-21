#!/usr/bin/env python3
"""Export Evo's canonical event store into the evo-threads engine input format.

Read-only against the live corpus. Produces the JSON array-of-arrays the
regression harness (crates/evo-threads/src/bin/regression.rs) consumes:

    [timestamp_ms, origin, interaction, resource, optional_weight]

Mapping rules (the adapter's contract with the engine):
- origin: the old store has no provenance column; everything maps to "Person"
  except download_watcher rows, which are the person's own downloads.
- interaction: "download" event_kind -> "Downloaded"; "bucket" rows (the
  input-activity counters, once the daemon records them) -> "Typed" with
  weight = key count; terminal focus rows -> "Focused"; everything else
  (URL/window observations) -> "Focused" with the row's dwell_ms as the
  weight, capped at one attention interval.
- resource: COALESCE(normalized_url, window_title), EXCEPT bucket rows,
  which carry the document-grain subject in `subject` when present and fall
  back to the title. Downloaded-file resource names are normalized: macOS
  and browsers mint "doc (1).pdf", "doc.pdf.pdf" and "doc (1).pdf (1).pdf"
  for what is one file re-downloaded; the engine is deliberately
  name-agnostic, so the adapter owes it stable identities.

Usage:
  python3 tools/threads-regression/export_events.py [output.json] [since_epoch_ms]
"""

import json
import re
import sqlite3
import sys
from pathlib import Path

DEFAULT_DB = Path.home() / "Library" / "Application Support" / "evo" / "evo.db"
DEFAULT_OUT = Path(__file__).parent / "sample_events.json"
DEFAULT_SINCE = 1_788_480_000_000  # 2026-09-05, the labeled window

MAX_DWELL_WEIGHT = 600_000  # one max_attention_interval, in ms


def normalize_download(resource: str) -> str:
    prev = None
    stem = resource
    while prev != stem:
        prev = stem
        stem = re.sub(r"\s*\(\d+\)(?=\.|$)", "", stem)
        stem = re.sub(r"(\.\w+)\1$", r"\1", stem)
    return stem


def main() -> None:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    since = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_SINCE

    db = sqlite3.connect(f"file:{DEFAULT_DB}?mode=ro", uri=True)
    # Schema-adaptive: the store gains a document-grain `subject` column and
    # an input-counter `bucket` event kind when the daemon is rebuilt with
    # the capture-layer changes; until then the legacy columns stand.
    columns = {row[1] for row in db.execute("PRAGMA table_info(canonical_events)")}
    subject_expr = (
        "COALESCE(NULLIF(TRIM(subject), ''), normalized_url, window_title)"
        if "subject" in columns
        else "COALESCE(normalized_url, window_title)"
    )
    rows = db.execute(
        f"""
        SELECT ts_ms, {subject_expr}, app_id, event_kind, input_chars, dwell_ms
        FROM canonical_events
        WHERE ts_ms > ?
        ORDER BY ts_ms
        """,
        (since,),
    ).fetchall()

    events = []
    for ts, resource, app, kind, keys, dwell in rows:
        if resource is None:
            continue
        resource = resource.strip()
        if kind == "download":
            events.append([ts, "Person", "Downloaded", normalize_download(resource)])
        elif kind == "bucket" and keys and keys > 0:
            # Content-free input counters: the production signal the engine
            # was starving for. Weight = key count; the clicks and scrolls
            # re-arm attention via the Focused events around them.
            events.append([ts, "Person", "Typed", resource, keys])
        else:
            weight = min(dwell or 0, MAX_DWELL_WEIGHT)
            events.append([ts, "Person", "Focused", resource, weight])

    out.write_text(json.dumps(events))
    print(f"exported {len(events)} events -> {out}")


if __name__ == "__main__":
    main()
