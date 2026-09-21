#!/usr/bin/env python3
"""Decode Evo's append-only canonical logs for diagnosis.

Framing (evo-storage): each record is `len\n<bytes>\n`.
Record bodies are line-oriented `key=value`, with text hex-encoded.

Read-only. Never writes to a storage root.
"""

import sys
import os
from collections import Counter, defaultdict


def read_records(path):
    """Yield each framed record body as bytes."""
    with open(path, "rb") as fh:
        data = fh.read()
    offset = 0
    while offset < len(data):
        line_end = data.find(b"\n", offset)
        if line_end == -1:
            break
        try:
            length = int(data[offset:line_end])
        except ValueError:
            break
        start = line_end + 1
        end = start + length
        if end > len(data):
            break
        yield data[start:end]
        offset = end + 1


def parse_kv(record):
    """Parse a record body into (record_type, list_of_pairs)."""
    text = record.decode("utf-8", errors="replace")
    lines = text.split("\n")
    rtype = lines[0] if lines else ""
    pairs = []
    for line in lines[1:]:
        if "=" in line:
            k, v = line.split("=", 1)
            pairs.append((k, v))
    return rtype, pairs


def unhex(value):
    try:
        return bytes.fromhex(value).decode("utf-8", errors="replace")
    except ValueError:
        return f"<bad-hex:{value[:24]}>"


def decode_observations(path):
    """Return a list of decoded observation dicts."""
    out = []
    for record in read_records(path):
        rtype, pairs = parse_kv(record)
        if rtype != "observation-v1":
            continue
        obs = {"schema": None, "version": None, "source": None,
               "at": None, "context": {}, "facts": []}
        pending_ctx_key = None
        pending_fact_name = None
        for k, v in pairs:
            if k == "id_hex":
                obs["id"] = unhex(v)
            elif k == "schema_name_hex":
                obs["schema"] = unhex(v)
            elif k == "schema_version":
                obs["version"] = v
            elif k == "source_hex":
                obs["source"] = unhex(v)
            elif k == "observed_at":
                sign, rest = v[0], v[1:]
                secs, nanos = rest.split(":")
                t = int(secs) + int(nanos) / 1e9
                obs["at"] = t if sign == "+" else -t
            elif k == "context_key_hex":
                pending_ctx_key = unhex(v)
            elif k == "context_value_hex":
                obs["context"][pending_ctx_key] = unhex(v)
            elif k == "fact_name_hex":
                pending_fact_name = unhex(v)
            elif k == "fact_value_hex":
                obs["facts"].append((pending_fact_name, unhex(v)))
            elif k == "fact_value_int":
                obs["facts"].append((pending_fact_name, int(v)))
            elif k == "fact_value_bool":
                obs["facts"].append((pending_fact_name, v == "1"))
        out.append(obs)
    return out


def decode_workspaces(path):
    """Return list of workspace records (stale previous-generation log)."""
    out = []
    for record in read_records(path):
        rtype, pairs = parse_kv(record)
        if not rtype.startswith("workspace-"):
            continue
        ws = {"version": rtype, "id": None, "lifecycle": None,
              "attachments": [], "snapshots": 0}
        in_snapshot = False
        for k, v in pairs:
            if k == "workspace_id_hex":
                ws["id"] = unhex(v)
            elif k == "lifecycle" and not in_snapshot:
                ws["lifecycle"] = v
            elif k == "snapshot_count":
                ws["snapshots"] = int(v)
                in_snapshot = True
            elif k == "attachment_artifact_hex" and not in_snapshot:
                ws["attachments"].append(unhex(v))
        out.append(ws)
    return out


def decode_restorations(path):
    out = []
    for record in read_records(path):
        rtype, pairs = parse_kv(record)
        if rtype != "restoration-v1":
            continue
        r = {"workspace": None, "outcome": None, "resume": None,
             "context_count": 0, "blockers": 0, "next_step": False,
             "surface": 0, "missing_reason": None}
        for k, v in pairs:
            if k == "workspace_id_hex":
                r["workspace"] = unhex(v)
            elif k == "outcome":
                r["outcome"] = v
            elif k == "resume_point_present":
                r["resume"] = v == "1"
            elif k == "resume_point_artifact_hex":
                r["resume_artifact"] = unhex(v)
            elif k == "context_count":
                r["context_count"] = int(v)
            elif k == "blocker_count":
                r["blockers"] = int(v)
            elif k == "next_step_present":
                r["next_step"] = v == "1"
            elif k == "continuation_surface_count":
                r["surface"] = int(v)
            elif k == "next_step_missing_reason_hex" and v:
                r["missing_reason"] = unhex(v)
        out.append(r)
    return out


def main():
    root = sys.argv[1] if len(sys.argv) > 1 else "."

    def p(name):
        return os.path.join(root, name)

    print("=" * 78)
    print("OBSERVATION LOG")
    print("=" * 78)
    obs = decode_observations(p("observation.log"))
    print(f"total observations: {len(obs)}")
    schemas = Counter(f"{o['schema']} v{o['version']}" for o in obs)
    print("\nby schema:")
    for name, count in schemas.most_common():
        print(f"  {count:6d}  {name}")

    sources = Counter(o["source"] for o in obs)
    print("\nby source:")
    for name, count in sources.most_common():
        print(f"  {count:6d}  {name}")

    ctx_keys = Counter()
    for o in obs:
        for k in o["context"]:
            ctx_keys[k] += 1
    print(f"\nprovenance context keys present ({len(ctx_keys)} distinct):")
    for name, count in ctx_keys.most_common():
        print(f"  {count:6d}  {name}")
    no_ctx = sum(1 for o in obs if not o["context"])
    print(f"  observations with EMPTY context: {no_ctx} "
          f"({100.0 * no_ctx / max(1, len(obs)):.1f}%)")

    fact_names = Counter()
    for o in obs:
        for name, _ in o["facts"]:
            fact_names[name] += 1
    print("\nfact names:")
    for name, count in fact_names.most_common():
        print(f"  {count:6d}  {name}")

    fact_count_dist = Counter(len(o["facts"]) for o in obs)
    print("\nfacts per observation:")
    for n in sorted(fact_count_dist):
        print(f"  {fact_count_dist[n]:6d} observations have {n} fact(s)")

    # Distinct subjects per schema
    print("\n" + "=" * 78)
    print("DISTINCT WITNESSED SUBJECTS PER SCHEMA")
    print("=" * 78)
    by_schema = defaultdict(Counter)
    for o in obs:
        for name, value in o["facts"]:
            if isinstance(value, str):
                by_schema[o["schema"]][value] += 1
    for schema in sorted(by_schema):
        subjects = by_schema[schema]
        print(f"\n{schema}: {len(subjects)} distinct subjects, "
              f"{sum(subjects.values())} observations")
        for subject, count in subjects.most_common(40):
            display = subject if len(subject) <= 100 else subject[:97] + "..."
            print(f"  {count:5d}  {display}")
        if len(subjects) > 40:
            print(f"  ... and {len(subjects) - 40} more distinct subjects")

    if obs:
        times = [o["at"] for o in obs if o["at"]]
        import datetime
        print("\ntime span:")
        print(f"  first: {datetime.datetime.fromtimestamp(min(times))}")
        print(f"  last:  {datetime.datetime.fromtimestamp(max(times))}")
        span_h = (max(times) - min(times)) / 3600
        print(f"  span:  {span_h:.1f} hours")

    # Stale previous-generation logs
    for label, fname, decoder in (
        ("WORKSPACE LOG (previous generation, stale)", "workspace.log", decode_workspaces),
        ("RESTORATION LOG (previous generation, stale)", "restoration.log", decode_restorations),
    ):
        if not os.path.exists(p(fname)):
            continue
        print("\n" + "=" * 78)
        print(label)
        print("=" * 78)
        records = decoder(p(fname))
        print(f"total records: {len(records)}")
        if fname == "workspace.log":
            ids = set(r["id"] for r in records)
            print(f"distinct workspace ids: {len(ids)}")
            vers = Counter(r["version"] for r in records)
            for v, c in vers.most_common():
                print(f"  {c:6d}  {v}")
            # Union attachments per workspace across all delta records
            members = defaultdict(set)
            for r in records:
                for a in r["attachments"]:
                    members[r["id"]].add(a)
            sizes = Counter(len(v) for v in members.values())
            print(f"\nworkspace size distribution "
                  f"(union of attachments across delta records):")
            for n in sorted(sizes):
                print(f"  {sizes[n]:6d} workspaces have {n} artifact(s)")
            single = sizes.get(1, 0) + sizes.get(0, 0)
            print(f"  single-or-zero-artifact workspaces: {single} of "
                  f"{len(members)} ({100.0 * single / max(1, len(members)):.1f}%)")
            all_artifacts = set()
            for v in members.values():
                all_artifacts |= v
            print(f"  distinct artifacts across all workspaces: {len(all_artifacts)}")
        else:
            outcomes = Counter(r["outcome"] for r in records)
            print("\noutcomes:")
            for o, c in outcomes.most_common():
                print(f"  {c:6d}  {o}")
            print(f"\nrestoration content:")
            print(f"  with resume point:        "
                  f"{sum(1 for r in records if r['resume'])}")
            print(f"  with next step:           "
                  f"{sum(1 for r in records if r['next_step'])}")
            ctx = Counter(r["context_count"] for r in records)
            print(f"  context_count distribution: {dict(ctx)}")
            surf = Counter(r["surface"] for r in records)
            print(f"  continuation_surface_count: {dict(surf)}")
            reasons = Counter(r["missing_reason"] for r in records
                             if r["missing_reason"])
            print(f"\ndistinct missing-reason strings: {len(reasons)}")
            for reason, c in reasons.most_common(5):
                print(f"  {c:6d}  {reason[:300]}")

    print("\n" + "=" * 78)
    print("ARTIFACT LOG")
    print("=" * 78)
    if os.path.exists(p("artifact.log")):
        arts = []
        for record in read_records(p("artifact.log")):
            arts.append(record.decode("utf-8", errors="replace").strip())
        print(f"total records: {len(arts)}")
        print(f"distinct artifact ids: {len(set(arts))}")


if __name__ == "__main__":
    main()
