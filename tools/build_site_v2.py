#!/usr/bin/env python3
"""Builds site/index.html — the narrative site.

    python3 tools/build_site_v2.py

The story site is four source fragments assembled into one self-contained
file:

    site/src/00-head.html   document head, font faces, the whole design system
    site/src/10-body.html   the chapters, in order
    site/src/20-js.html     the scroll engine, scenes, gate, ask box

The synthetic corpus is injected from build_site.py, so the narrative site and
the console demo can never disagree about the eight invented works. The real
corpus in boundary.json is never read here and never published.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from build_site import build_data, STOP, VOCAB  # noqa: E402

SRC = os.path.join(ROOT, "site", "src")
OUT = os.path.join(ROOT, "site", "index.html")

PARTS = ["00-head.html", "10-body.html", "20-js.html"]


def read(name):
    path = os.path.join(SRC, name)
    if not os.path.exists(path):
        raise SystemExit("missing source fragment: %s" % path)
    with open(path, encoding="utf-8") as f:
        return f.read()


def build():
    data = build_data()
    corpus = json.dumps(data, separators=(",", ":"))
    stop = " ".join(sorted(STOP))
    syn = json.dumps(VOCAB, separators=(",", ":"))

    html = "".join(read(p) for p in PARTS)
    html = html.replace("__CORPUS__", corpus)
    html = html.replace("__STOP__", stop)
    html = html.replace("__SYN__", syn)

    for token in ("__CORPUS__", "__STOP__", "__SYN__"):
        if token in html:
            raise SystemExit("unreplaced token survived: " + token)

    with open(OUT, "w", encoding="utf-8") as f:
        f.write(html)

    claimed = [w["label"] for w in data["works"] if w["claimed"]]
    print("wrote", os.path.relpath(OUT, ROOT), len(html), "bytes")
    print("works:", len(data["works"]), "claimed:", claimed)
    print("events:", data["totalEvents"], "stream points:", len(data["stream"]))


if __name__ == "__main__":
    build()
