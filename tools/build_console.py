#!/usr/bin/env python3
"""Builds site/demo.html — the console.

    python3 tools/build_console.py

The console shares the report's design system on purpose: same paper, same
type, same rules. It reuses site/src/00-head.html wholesale, then appends the
console body and its script. Data comes from build_site.py, so the console and
the report can never disagree about the eight invented works.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from build_site import build_data, STOP, VOCAB  # noqa: E402

SRC = os.path.join(ROOT, "site", "src")
OUT = os.path.join(ROOT, "site", "demo.html")


def read(name):
    path = os.path.join(SRC, name)
    with open(path, encoding="utf-8") as f:
        return f.read()


def build():
    data = build_data()
    head = read("00-head.html")
    head = head.replace("<title>Evo — the boundary layer for agents</title>",
                        "<title>Evo — the console</title>")
    head = head.replace(
        '<meta name="description" content="Evo derives where one body of work ends and the next begins — on the device, without a model — so an agent can act on a real machine without a human labelling every folder first.">',
        '<meta name="description" content="The console: eight invented bodies of work, two claimed, six refused, and a gate you can trip.">')

    html = head + read("console-body.html") + read("console-js.html")
    html = html.replace("__CORPUS__", json.dumps(data, separators=(",", ":")))
    html = html.replace("__STOP__", " ".join(sorted(STOP)))
    html = html.replace("__SYN__", json.dumps(VOCAB, separators=(",", ":")))

    for token in ("__CORPUS__", "__STOP__", "__SYN__"):
        if token in html:
            raise SystemExit("unreplaced token survived: " + token)

    with open(OUT, "w", encoding="utf-8") as f:
        f.write(html)

    print("wrote", os.path.relpath(OUT, ROOT), len(html), "bytes")
    print("works:", len(data["works"]), "claimed:",
          [w["label"] for w in data["works"] if w["claimed"]])


if __name__ == "__main__":
    build()
