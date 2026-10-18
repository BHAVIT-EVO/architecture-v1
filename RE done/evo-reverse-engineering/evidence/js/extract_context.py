#!/usr/bin/env python3
"""Single-pass keyword context extractor over a file or directory tree.

Much faster than repeated `grep -o '.{0,70}term.{0,70}'` scans of large bundles:
opens each file once, finds all terms in one pass, and dedupes results.
"""
import os
import re
import sys

TEXT_EXT = (".js", ".mjs", ".cjs", ".ts", ".json", ".html", ".css")


def iter_text_files(root):
    if os.path.isfile(root):
        yield root
        return
    for dirpath, _dirnames, filenames in os.walk(root):
        for fn in filenames:
            if fn.lower().endswith(TEXT_EXT):
                yield os.path.join(dirpath, fn)


def main(root, terms, limit):
    pattern = re.compile("|".join(terms), re.IGNORECASE)
    seen = {}
    for path in iter_text_files(root):
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as fh:
                text = fh.read()
        except OSError:
            continue
        for m in pattern.finditer(text):
            start = max(0, m.start() - 90)
            end = min(len(text), m.end() + 90)
            snippet = re.sub(r"\s+", " ", text[start:end]).strip()
            key = (m.group(0).lower(), snippet)
            seen.setdefault(key, path)
    by_term = {}
    for (term, snippet), path in seen.items():
        by_term.setdefault(term, []).append((snippet, path))
    for term in sorted(by_term):
        print(f"\n===== {term} ({len(by_term[term])} unique) =====")
        for snippet, path in by_term[term][:limit]:
            print(f"  [{os.path.basename(path)}] {snippet}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2].split(","), int(sys.argv[3]) if len(sys.argv) > 3 else 8)
