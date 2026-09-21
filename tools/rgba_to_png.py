#!/usr/bin/env python3
"""Convert raw premultiplied-RGBA framebuffer dumps into PNGs.

`cargo run --example qa_capture` writes `<name>_<w>x<h>.rgba` files straight out
of the shell's own colour attachment. This turns them into PNGs so they can be
looked at. Nothing but the standard library is used: the workspace has no image
encoder among its dependencies and adding one is out of scope.

The bytes come back premultiplied, so they are un-premultiplied here before
being written; skipping that step darkens every translucent pixel.
"""

import re
import struct
import sys
import zlib
from pathlib import Path

NAME = re.compile(r"^(?P<stem>.+)_(?P<w>\d+)x(?P<h>\d+)\.rgba$")


def chunk(tag: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + tag
        + payload
        + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
    )


def unpremultiply(raw: bytes) -> bytearray:
    out = bytearray(raw)
    for i in range(0, len(out), 4):
        a = out[i + 3]
        if a == 0:
            out[i] = out[i + 1] = out[i + 2] = 0
        elif a != 255:
            for c in range(3):
                out[i + c] = min(255, (out[i + c] * 255 + a // 2) // a)
    return out


def write_png(path: Path, width: int, height: int, rgba: bytes) -> None:
    pixels = unpremultiply(rgba)
    stride = width * 4
    # One filter byte (0 = None) per scanline, as the PNG spec requires.
    scanlines = b"".join(
        b"\x00" + bytes(pixels[y * stride : (y + 1) * stride]) for y in range(height)
    )
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(scanlines, 6))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(png)


def main() -> int:
    source = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/evo-qa-shots")
    dumps = sorted(source.glob("*.rgba"))
    if not dumps:
        print(f"no .rgba dumps in {source}")
        return 1
    for dump in dumps:
        match = NAME.match(dump.name)
        if not match:
            print(f"skip (unparsable name): {dump.name}")
            continue
        width, height = int(match["w"]), int(match["h"])
        raw = dump.read_bytes()
        expected = width * height * 4
        if len(raw) != expected:
            print(f"skip {dump.name}: {len(raw)} bytes, expected {expected}")
            continue
        out = dump.with_name(f"{match['stem']}.png")
        write_png(out, width, height, raw)
        print(f"{out.name}  {width}x{height}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
