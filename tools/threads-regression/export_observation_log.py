#!/usr/bin/env python3
"""Export Evo's canonical Observation log into the evo-threads engine input format.

Read-only against the live corpus. Produces the JSON array-of-arrays the
regression harness (crates/evo-threads/src/bin/regression.rs) consumes:

    [timestamp_ms, origin, interaction, resource, optional_weight]

This is the observation-log twin of export_events.py (which reads the legacy
SQLite store). The mapping mirrors crates/evo-daemon/src/threads.rs
`project_observation` — the same projection the live read path and the shadow
gate use — so the regression corpus is scored against engine input exactly as
production feeds it:

- OBS-WINDOW-FOCUS-GAINED -> "Focused"; the dwell weight is the gap to the
  next event, capped at one attention interval (600000 ms), exactly as the
  daemon bridge computes it.
- OBS-URL-NAVIGATED       -> "Visited" (weight = gap to next event, capped).
- OBS-INPUT-ACTIVITY      -> "Typed", weight = keys; all-zero buckets never
  existed (absence is the zero).
- OBS-FILE-SAVED          -> "Mutated" (the engine's churn gate judges it).
- OBS-WORK-GROUPED        -> "Declared" SameWork (first fact, CoMember fact).

Unmapped concepts (RepositoryMembership, CommitMade, WorkDesignated,
SurfaceNamed) project nothing, exactly as the bridge drops them.

Usage:
  python3 tools/threads-regression/export_observation_log.py \
      [output.json] [since_epoch_ms] [--until epoch_ms] [--exclude-declared]

The committed corpus (tools/threads-regression/sample_events.json) is the
input-capture era of the log: the window where OBS-INPUT-ACTIVITY events
exist, because that is the era whose attention is disambiguated — the
precondition for treating the engine's placement of a resource as a testable
claim. Regenerate it with the matching flags (see docs/BUILD-LOG.md).
"""

import json
import sys
from pathlib import Path

DEFAULT_LOG = Path.home() / "Library" / "Application Support" / "evo" / "storage" / "observation.log"
DEFAULT_OUT = Path(__file__).parent / "sample_events.json"
DEFAULT_SINCE = 0  # the whole log; the committed window is chosen by review

MAX_INTERVAL = 600_000  # one max_attention_interval, in ms


def unhex(value: str) -> str:
    return bytes.fromhex(value).decode("utf-8", "replace")


def parse_record(record: str):
    schema = None
    ts_ms = None
    first_text = None
    facts = []  # (name, kind, value)
    pending_name = None
    for line in record.splitlines():
        if line.startswith("schema_name_hex="):
            schema = unhex(line.split("=", 1)[1])
        elif line.startswith("observed_at="):
            raw = line.split("=", 1)[1].lstrip("+")
            secs, nanos = raw.split(":")
            ts_ms = int(secs) * 1000 + int(nanos) // 1_000_000
        elif line.startswith("fact_name_hex="):
            pending_name = unhex(line.split("=", 1)[1])
        elif line.startswith("fact_value_kind="):
            kind = line.split("=", 1)[1]
        elif line.startswith("fact_value_hex="):
            value = unhex(line.split("=", 1)[1])
            facts.append((pending_name, "Text", value))
            pending_name = None
        elif line.startswith("fact_value_int="):
            facts.append((pending_name, "Integer", int(line.split("=", 1)[1])))
            pending_name = None
    return schema, ts_ms, facts


def main() -> None:
    argv = sys.argv[1:]
    out = DEFAULT_OUT
    since = DEFAULT_SINCE
    until = None
    exclude_declared = False
    positional = []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--until" and i + 1 < len(argv):
            until = int(argv[i + 1])
            i += 2
        elif arg == "--exclude-declared":
            exclude_declared = True
            i += 1
        elif arg.isdigit():
            since = int(arg)
            i += 1
        else:
            positional.append(arg)
            i += 1
    out = Path(positional[0]) if positional else DEFAULT_OUT
    log = DEFAULT_LOG

    data = log.read_bytes()
    records = []
    offset = 0
    while offset < len(data):
        nl = data.find(b"\n", offset)
        if nl < 0:
            break
        try:
            length = int(data[offset:nl].decode().strip())
        except ValueError:
            break
        start = nl + 1
        end = start + length
        if end > len(data):
            break
        records.append(data[start:end].decode("utf-8", "replace"))
        offset = end + 1

    # First pass: (ts_ms, interaction, resource, weight) for attention events,
    # so the next-event gap — the live bridge's dwell computation — is exact.
    projected = []
    saves = []
    declarations = []
    for record in records:
        schema, ts_ms, facts = parse_record(record)
        if schema is None or ts_ms is None or ts_ms <= since:
            continue
        if until is not None and ts_ms > until:
            continue
        if schema == "OBS-WINDOW-FOCUS-GAINED":
            subject = next((v for (n, k, v) in facts if n == "WindowFocusGained" and k == "Text"), None)
            if subject:
                projected.append([ts_ms, "Focused", subject])
        elif schema == "OBS-URL-NAVIGATED":
            subject = next((v for (n, k, v) in facts if n == "URLNavigated" and k == "Text"), None)
            if subject:
                projected.append([ts_ms, "Visited", subject])
        elif schema == "OBS-INPUT-ACTIVITY":
            subject = next((v for (n, k, v) in facts if n == "InputActivity" and k == "Text"), None)
            keys = next((v for (n, k, v) in facts if n == "Keys" and k == "Integer"), 0)
            if subject and keys > 0:
                projected.append([ts_ms, "Typed", subject, keys])
        elif schema == "OBS-FILE-SAVED":
            subject = next((v for (n, k, v) in facts if n == "FileSaved" and k == "Text"), None)
            if subject:
                saves.append([ts_ms, "Mutated", subject])
        elif schema == "OBS-WORK-GROUPED":
            if exclude_declared:
                continue
            first = next((v for (n, k, v) in facts if n == "WorkGrouped" and k == "Text"), None)
            second = next((v for (n, k, v) in facts if n == "CoMember" and k == "Text"), None)
            if first and second:
                declarations.append([ts_ms, "Declared", "", (first, second)])

    projected.sort(key=lambda row: row[0])
    saves.sort(key=lambda row: row[0])
    declarations.sort(key=lambda row: row[0])
    # Declarations travel in the same stream as everything else: the person's
    # word is an event too, and the engine folds it in timestamp order.
    everything = projected + saves + declarations
    everything.sort(key=lambda row: row[0])

    # Dwell = gap to the next event (any event), capped — the bridge's rule,
    # applied at the same boundary (engine input), only here in the exporter
    # because the corpus is a snapshot of the log.
    events = []
    for index, row in enumerate(everything):
        ts, interaction, resource = row[0], row[1], row[2]
        if interaction == "Declared":
            (a, b) = row[3]
            events.append([ts, "Person", "Declared", "", None, {"SameWork": [a, b]}])
            continue
        weight = None
        if interaction in ("Focused", "Visited"):
            nxt = everything[index + 1][0] if index + 1 < len(everything) else ts
            weight = max(min(nxt - ts, MAX_INTERVAL), 0)
        events.append([ts, "Person", interaction, resource, weight])

    # The regression binary reads [ts, origin, interaction, resource, weight]
    # and ignores extra slots; declarations need the pair, so they carry a
    # 6th element. Strip the detail slot for non-declaration rows to keep the
    # file canonical.
    canonical = []
    for row in events:
        if row[1] == "Person" and row[2] == "Declared":
            (a, b) = row[5]["SameWork"]
            canonical.append([row[0], "Person", "Declared", "", None, a, b])
        else:
            canonical.append(row[:5])

    out.write_text(json.dumps(canonical))
    kinds = {}
    for row in canonical:
        kinds[row[2]] = kinds.get(row[2], 0) + 1
    print(f"exported {len(canonical)} events -> {out}")
    print("  " + ", ".join(f"{k}={v}" for k, v in sorted(kinds.items())))


if __name__ == "__main__":
    main()
