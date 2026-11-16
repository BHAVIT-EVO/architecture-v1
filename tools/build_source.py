#!/usr/bin/env python3
"""Builds site/source.html — the code page.

    python3 tools/build_source.py

Every excerpt on that page is extracted from the working tree at build time.
If a file moves, or a line range no longer exists, the build fails instead of
publishing something plausible-looking. That is the point: a judge should be
able to trust that what is on the page is what is in the repository.

Nothing from boundary.json is read here.
"""
import html
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SRC = os.path.join(ROOT, "site", "src")
OUT = os.path.join(ROOT, "site", "source.html")

PAT = re.compile(r'<pre class="code" data-file="([^"]+)" data-lines="(\d+)-(\d+)"></pre>')


def read(name):
    with open(os.path.join(SRC, name), encoding="utf-8") as f:
        return f.read()


def excerpt(path, a, b):
    full = os.path.join(ROOT, path)
    if not os.path.exists(full):
        raise SystemExit("missing source file: " + path)
    with open(full, encoding="utf-8") as f:
        lines = f.read().splitlines()
    if b > len(lines):
        raise SystemExit("%s has %d lines, asked for %d-%d" % (path, len(lines), a, b))

    is_py = path.endswith(".py")
    out = ['<span class="f">%s · %d–%d</span>' % (path, a, b)]
    for line in lines[a - 1:b]:
        esc = html.escape(line)
        stripped = line.lstrip()
        if stripped.startswith("//") or (is_py and stripped.startswith("#")):
            esc = '<span class="c">%s</span>' % esc
        out.append(esc)
    return '<pre class="code">' + "\n".join(out) + "</pre>"


def build():
    head = read("00-head.html")
    head = head.replace("<title>Evo — the boundary layer for agents</title>",
                        "<title>Evo — source</title>")
    head = head.replace(
        '<meta name="description" content="Evo derives where one body of work ends and the next begins — on the device, without a model — so an agent can act on a real machine without a human labelling every folder first.">',
        '<meta name="description" content="Real excerpts from the Evo engine: the declaration enum, the prohibition, the gate, the no-network test, the MCP surface.">')

    body = read("source-body.html")
    body, n = PAT.subn(lambda m: excerpt(m.group(1), int(m.group(2)), int(m.group(3))), body)
    if "data-file=" in body:
        raise SystemExit("a code block was left unfilled")

    doc = head + body + "</body>\n</html>\n"
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(doc)
    print("wrote", os.path.relpath(OUT, ROOT), len(doc), "bytes", "·", n, "excerpts")


if __name__ == "__main__":
    build()
