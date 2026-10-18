#!/usr/bin/env python3
"""Minimal ASAR extractor: pulls only text-ish files (js/json/mjs/cjs/ts/html/css).

ASAR layout: [u32 header_size_align][u32 header_size_actual][u32 json_size][u32 json_size_actual]
then the JSON directory header (with 4-byte aligned size prefix echoed by size fields),
then the file payloads concatenated in header order.
"""
import json
import os
import struct
import sys

WANT = (".js", ".mjs", ".cjs", ".json", ".ts", ".html", ".css", ".map")


def parse_header(f):
    """ASAR/Chromium-Pickle header.

    u32@0  = 4 (payload size of the following field)
    u32@4  = pickle total size
    u32@8  = aligned header (JSON) size -- file offsets are relative to 8 + this
    u32@12 = exact JSON byte length
    """
    f.seek(0)
    # u32@4 is the payload size of the header pickle: 4 (length field) + aligned JSON.
    # JSON itself begins at 16; u32@12 is its exact length. Payload data begins at 8 + u32@4.
    _, pickle_payload, aligned_len, json_len = struct.unpack("<IIII", f.read(16))
    blob = f.read(aligned_len)
    json_bytes = blob[:json_len] if json_len else blob
    # Payload data begins at 8 (size pickle) + pickle_payload (header pickle).
    return json.loads(json_bytes.decode("utf-8", "replace")), 8 + pickle_payload


def walk(node, prefix, out):
    for name, meta in (node.get("files") or {}).items():
        path = f"{prefix}/{name}"
        if meta.get("files") is not None:
            walk(meta, path, out)
            continue
        if "link" in meta or meta.get("size") is None or meta.get("offset") is None:
            continue  # symlink / unpacked stub: no inline payload
        if name.lower().endswith(WANT):
            # asar v3 stores offsets as decimal strings to survive JSON precision.
            out.append((path, int(meta["offset"]), int(meta["size"])))


def main(asar_path, dest):
    os.makedirs(dest, exist_ok=True)
    with open(asar_path, "rb") as f:
        # parse_header already returns the absolute payload base, not a size.
        header, base = parse_header(f)
        files = []
        walk(header, "", files)
        written = 0
        total = 0
        skipped = 0
        for path, offset, length in files:
            # offsets are relative to the end of the header block
            f.seek(base + offset)
            data = f.read(length)
            if len(data) != length:
                skipped += 1
                continue
            target = os.path.join(dest, path.lstrip("/"))
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with open(target, "wb") as out:
                out.write(data)
            written += 1
            total += length
        if skipped:
            print(f"  warning: {skipped} truncated entries skipped")
    print(f"{os.path.basename(asar_path)}: extracted {written} text files ({total/1e6:.1f} MB) -> {dest}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
