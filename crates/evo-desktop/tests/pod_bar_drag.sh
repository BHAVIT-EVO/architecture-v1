#!/usr/bin/env bash
set -euo pipefail

# Smallest verifier for the pod-bar drag-to-learn wiring.
# - img and rect must agree on the taught frame's width and height.
# - Unit is pixels; caller converts to screen points if needed.

create () {
  local out_dir="${1:-.}"
  local img="${2:-untitled.png}"
  local rect="${3:-0,0,320,200}"

  mkdir -p "$out_dir" || true
  python3 - <<'PY' "$out_dir" "$img" "$rect"
import os, struct, zlib, sys

out_dir, img_name, rect_text = sys.argv[1:4]
x, y, w, h = (int(v) for v in rect_text.split(","))

def chunk(kind, data):
    payload = kind + data
    return struct.pack("<I", len(data)) + payload + struct.pack("<I", zlib.crc32(payload) & 0xFFFFFFFF)

def raw_chunk(data):
    return chunk(b"IDAT", zlib.compress(data))

# Minimal 320x200 RGBA PNG: transparent background, one red pixel at (10,10).
width, height = w, h
rows = bytearray()
for _ in range(height):
    rows.append(0)  # filter none
    rows.extend(b"\x00\x00\x00\x00" * width)

# stamp a red pixel so the file is visibly non-empty
px = 10
row = bytearray([0]) + b"\x00\x00\x00\x00" * px + b"\xff\x00\x00\xff" + b"\x00\x00\x00\x00" * (width - px - 1)
rows = bytearray([0]) + b"\x00\x00\x00\x00" * width  # placeholder above, rebuild properly
rows = bytearray()
for row_idx in range(height):
    row_bytes = bytearray([0])
    row_bytes.extend(b"\x00\x00\x00\x00" * width)
    if row_idx == 10 and px < width:
        off = 1 + px * 4
        row_bytes[off:off+4] = b"\xff\x00\x00\xff"
    rows.extend(row_bytes)

ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
sig = b"\x89PNG\r\n\x1a\n"
ihdr_chunk = chunk(b"IHDR", ihdr)
idat_chunk = raw_chunk(bytes(rows))
iend_chunk = chunk(b"IEND", b"")

with open(os.path.join(out_dir, img_name), "wb") as f:
    f.write(sig)
    f.write(ihdr_chunk)
    f.write(idat_chunk)
    f.write(iend_chunk)
PY
}

main () {
  create "$@"
}

main "$@"
