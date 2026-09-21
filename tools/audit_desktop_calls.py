#!/usr/bin/env python3
"""Static self-audit for evo-desktop.

No Rust toolchain is available in this environment, so `cargo check` cannot
run. This substitutes the checks a compiler would catch first: a call to a
function that does not exist, and a call with the wrong number of arguments.

Three passes:

1. every `module::name(` call site, against the `fn` signatures of that module —
   unresolved names and arity mismatches;
2. every `module::CONSTANT`, `Type::Variant` and `Type { field: … }` reference,
   against the constants, enum variants and struct fields actually declared;
3. every bare name used inside a `#[cfg(test)] mod tests` block, against what
   that module defines or imports — `cargo test` compiles those modules too, so
   a test calling a helper that was renamed is a build failure passes 1 and 2
   cannot see.

It is deliberately conservative: generics, closures and trailing commas are
handled, but it makes no attempt at type checking. It is not a substitute for
`cargo check` — it is what can be verified without one.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "crates/evo-desktop/src"
MODULES = ["ui", "theme", "state", "home", "shell", "detail", "daemon", "app"]


def blank(text: str, start: int, end: int, keep: str = "") -> str:
    """Replace a span with `keep` plus padding, preserving every newline.

    Line numbers have to survive blanking or every reported location is wrong,
    and a blanked string literal has to leave *something* behind or the
    argument it was would vanish from the count.
    """
    span = text[start:end]
    padded = keep + " " * max(0, len(span) - len(keep))
    return "".join(
        "\n" if original == "\n" else replacement
        for original, replacement in zip(span, padded)
    )


def strip_strings_and_comments(text: str) -> str:
    """Blank out string/char literals and comments so they cannot match.

    A string literal collapses to a single `_`, so it still counts as one
    argument; a comment collapses to whitespace.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        two = text[i : i + 2]
        if two == "//":
            j = text.find("\n", i)
            j = n if j == -1 else j
            out.append(blank(text, i, j))
            i = j
        elif two == "/*":
            j = text.find("*/", i + 2)
            j = n if j == -1 else j + 2
            out.append(blank(text, i, j))
            i = j
        elif text[i] == '"':
            # Raw strings (r"…", r#"…"#) do not appear in this crate; plain and
            # escaped literals do.
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(blank(text, i, j, keep="_"))
            i = j
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


IDENT_TAIL = re.compile(r"[A-Za-z0-9_:>]")


def depth_walk(text: str, index: int, depth: int, angle: int) -> tuple[int, int]:
    """Update bracket and angle-bracket depth for the character at `index`.

    Rust's `<`/`>` are ambiguous, so an angle bracket only counts when it
    directly follows an identifier (`Vec<…>`, `HashMap<…>`), and `->` / `=>` /
    `>=` never close one. Without this, every `match` arm inside an argument
    closure would look like a closing bracket.
    """
    ch = text[index]
    previous = text[index - 1] if index > 0 else " "
    if ch in "([{":
        depth += 1
    elif ch in ")]}":
        depth -= 1
    elif ch == "<":
        if IDENT_TAIL.match(previous):
            angle += 1
    elif ch == ">":
        if previous not in "-=" and angle > 0:
            angle -= 1
    return depth, angle


def split_args(argtext: str) -> list[str]:
    """Split a parameter/argument list on top-level commas."""
    depth = 0
    angle = 0
    parts: list[str] = []
    current: list[str] = []
    for index, ch in enumerate(argtext):
        depth, angle = depth_walk(argtext, index, depth, angle)
        if ch == "," and depth == 0 and angle == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(ch)
    tail = "".join(current).strip()
    if tail:
        parts.append(tail)
    return [p.strip() for p in parts if p.strip()]


def balanced_slice(text: str, open_index: int) -> tuple[str, int]:
    """Return the contents of the (…) starting at `open_index`, and its end."""
    depth = 0
    angle = 0
    i = open_index
    n = len(text)
    while i < n:
        depth, angle = depth_walk(text, i, depth, angle)
        if depth == 0 and i > open_index:
            return text[open_index + 1 : i], i
        i += 1
    return text[open_index + 1 :], n


FN_RE = re.compile(r"\bfn\s+([a-z_][a-z0-9_]*)\s*(?:<[^>(]*>)?\s*\(")


def definitions(path: Path) -> dict[str, set[int]]:
    """Every `fn name(...)` in a file, mapped to the arities seen."""
    text = strip_strings_and_comments(path.read_text())
    found: dict[str, set[int]] = {}
    for match in FN_RE.finditer(text):
        name = match.group(1)
        args_text, _ = balanced_slice(text, match.end() - 1)
        args = split_args(args_text)
        # `self` receivers are not passed at a `module::fn()` call site.
        args = [a for a in args if not re.match(r"^(&(mut\s+)?)?(self|mut\s+self)\b", a)]
        found.setdefault(name, set()).add(len(args))
    return found


CALL_RE = re.compile(r"\b(" + "|".join(MODULES) + r")::([a-z_][a-z0-9_]*)\s*\(")


def call_sites(path: Path) -> list[tuple[int, str, str, int]]:
    """Every `module::name(...)` call: (line, module, name, argument count)."""
    text = strip_strings_and_comments(path.read_text())
    calls = []
    for match in CALL_RE.finditer(text):
        module, name = match.group(1), match.group(2)
        args_text, _ = balanced_slice(text, match.end() - 1)
        line = text.count("\n", 0, match.start()) + 1
        calls.append((line, module, name, len(split_args(args_text))))
    return calls


# ── Constants, enum variants and struct fields ──────────────────────────

CONST_RE = re.compile(r"\bpub const ([A-Z][A-Z0-9_]*)\s*:")
TYPE_RE = re.compile(r"\bpub (struct|enum) ([A-Z][A-Za-z0-9_]*)")


def type_bodies(text: str) -> dict[str, tuple[str, str]]:
    """Each `pub struct`/`pub enum` name → (kind, body text)."""
    bodies: dict[str, tuple[str, str]] = {}
    for match in TYPE_RE.finditer(text):
        kind, name = match.group(1), match.group(2)
        brace = text.find("{", match.end())
        semi = text.find(";", match.end())
        if brace == -1 or (semi != -1 and semi < brace):
            bodies[name] = (kind, "")  # tuple or unit struct
            continue
        body, _ = balanced_slice(text, brace)
        bodies[name] = (kind, body)
    return bodies


def members(kind: str, body: str) -> set[str]:
    """Field names of a struct, or variant names of an enum."""
    if kind == "enum":
        return {
            match.group(1)
            for match in re.finditer(r"^\s*([A-Z][A-Za-z0-9_]*)", body, re.MULTILINE)
        }
    return {
        match.group(1)
        for match in re.finditer(r"^\s*(?:pub\s+)?([a-z_][a-z0-9_]*)\s*:", body, re.MULTILINE)
    }


def symbol_table(paths: list[Path]) -> tuple[dict[str, set[str]], dict[str, set[str]], dict[str, str]]:
    """(constants per module, members per type, defining module per type)."""
    constants: dict[str, set[str]] = {}
    type_members: dict[str, set[str]] = {}
    owner: dict[str, str] = {}
    for path in paths:
        text = strip_strings_and_comments(path.read_text())
        constants[path.stem] = {m.group(1) for m in CONST_RE.finditer(text)}
        for name, (kind, body) in type_bodies(text).items():
            type_members[name] = members(kind, body)
            owner[name] = path.stem
    return constants, type_members, owner


PATH_CONST_RE = re.compile(r"\b(" + "|".join(MODULES) + r")::([A-Z][A-Z0-9_]*)\b(?!\s*\()")
VARIANT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)::([A-Z][A-Za-z0-9_]*)\b")
LITERAL_RE = re.compile(r"\b(?:([a-z_][a-z0-9_]*)::)?([A-Z][A-Za-z0-9_]*)\s*\{")

# Types whose members live outside this crate, so their variants and fields
# cannot be checked here.
FOREIGN = {"Self", "Ok", "Err", "Some", "None"}


def item_problems(path: Path, constants, type_members, owner) -> list[str]:
    """Unknown constants, enum variants and struct-literal fields."""
    text = strip_strings_and_comments(path.read_text())
    found: list[str] = []

    def where(index: int) -> int:
        return text.count("\n", 0, index) + 1

    for match in PATH_CONST_RE.finditer(text):
        module, name = match.group(1), match.group(2)
        if module == path.stem or module not in constants:
            continue
        if name not in constants[module] and name not in type_members:
            found.append(f"{path.name}:{where(match.start())}: {module}::{name} is not defined")

    for match in VARIANT_RE.finditer(text):
        type_name, variant = match.group(1), match.group(2)
        if type_name in FOREIGN or type_name not in type_members:
            continue
        if not type_members[type_name]:
            continue  # associated items on a struct, not enum variants
        if owner.get(type_name) == path.stem:
            pass  # still worth checking: a typo is a typo
        if variant not in type_members[type_name]:
            found.append(
                f"{path.name}:{where(match.start())}: {type_name}::{variant} is not a variant "
                f"(have {', '.join(sorted(type_members[type_name]))})"
            )

    for match in LITERAL_RE.finditer(text):
        type_name = match.group(2)
        if type_name in FOREIGN or type_name not in type_members:
            continue
        body, _ = balanced_slice(text, match.end() - 1)
        if ".." in body and "=>" not in body:
            continue  # a struct update or rest pattern: incomplete by design
        supplied = {
            field.split(":")[0].strip()
            for field in split_args(body)
            if re.match(r"^[a-z_][a-z0-9_]*\s*(:|,|$)", field.strip())
        }
        unknown = supplied - type_members[type_name]
        if unknown:
            found.append(
                f"{path.name}:{where(match.start())}: {type_name} has no field(s) "
                f"{', '.join(sorted(unknown))}"
            )
    return found


# ── Same-file references inside `#[cfg(test)] mod tests` ────────────────

TEST_MOD_RE = re.compile(r"#\[cfg\(test\)\]\s*mod tests\s*\{")
BARE_CALL_RE = re.compile(r"(?<![\w:.])([a-z_][a-z0-9_]*)\s*\(")
BARE_CONST_RE = re.compile(r"(?<![\w:.])([A-Z][A-Z0-9_]{1,})(?![\w(])")
IMPORT_RE = re.compile(r"\buse\s+([A-Za-z0-9_:{}, *]+);")

# Rust keywords that can be followed by `(` — `let (a, b) = …`, `for (i, x) in
# …`, `match (a, b) { … }` — and so look exactly like calls to this parser.
KEYWORDS = {
    "let", "for", "if", "while", "match", "return", "in", "move", "as", "else",
    "loop", "break", "continue", "and", "or", "not",
}

# Rust attributes read as calls, and macros/std items that resolve outside the
# file by definition. Anything here is *not* evidence of a problem.
NOT_A_LOCAL_FN = KEYWORDS | {
    "cfg", "derive", "test", "allow", "expect", "assert", "assert_eq",
    "assert_ne", "panic", "format", "vec", "matches", "println", "eprintln",
    "write", "writeln", "Some", "None", "Ok", "Err",
}
NOT_A_LOCAL_CONST = {"MAX", "MIN", "EPSILON", "INFINITY", "ZERO", "NONE"}


def imported_names(text: str) -> set[str]:
    """Every name brought in by a `use` statement, at any nesting depth."""
    names: set[str] = set()
    for match in IMPORT_RE.finditer(text):
        for token in re.split(r"[:{},\s]+", match.group(1)):
            if token and token != "*":
                names.add(token)
    return names


def test_reference_problems(path: Path) -> list[str]:
    """Names a test module uses that resolve nowhere it could see them.

    `cargo test` compiles these modules too, so a test calling a helper that
    was renamed or deleted is a build failure the two passes above would miss —
    they only look at `module::name(` call sites, and a test module says
    `use super::*` and then calls bare names.
    """
    text = strip_strings_and_comments(path.read_text())
    match = TEST_MOD_RE.search(text)
    if not match:
        return []
    brace = text.find("{", match.end() - 1)
    body, _ = balanced_slice(text, brace)
    # `body` is the text after the opening brace, so an offset inside it maps
    # back to the file at `brace + 1 + offset`.
    def where(offset: int) -> int:
        return text.count("\n", 0, brace + 1 + offset) + 1

    own_fns = {m.group(1) for m in re.finditer(r"\bfn\s+([a-z_][a-z0-9_]*)", text)}
    own_consts = {m.group(1) for m in CONST_RE.finditer(text)}
    own_consts |= {m.group(1) for m in re.finditer(r"\bconst ([A-Z][A-Z0-9_]*)\s*:", text)}
    imported = imported_names(text)
    bindings = {m.group(1) for m in re.finditer(r"\blet (?:mut )?([a-z_][a-z0-9_]*)", body)}
    # A name used as `x.name(…)` anywhere is a method, not a free function.
    methods = set(re.findall(r"\.\s*([a-z_][a-z0-9_]*)\s*\(", body))

    found: list[str] = []
    for m in BARE_CALL_RE.finditer(body):
        name = m.group(1)
        if name in own_fns or name in bindings or name in imported:
            continue
        if name in methods or name in NOT_A_LOCAL_FN:
            continue
        found.append(
            f"{path.name}:{where(m.start())}: "
            f"test calls {name}(), which is not defined or imported here"
        )
    for m in BARE_CONST_RE.finditer(body):
        name = m.group(1)
        if name in own_consts or name in imported or name in NOT_A_LOCAL_CONST:
            continue
        found.append(
            f"{path.name}:{where(m.start())}: "
            f"test uses {name}, which is not defined or imported here"
        )
    return found


def main() -> int:
    defs = {module: definitions(SRC / f"{module}.rs") for module in MODULES}
    paths = sorted(SRC.glob("*.rs"))
    problems: list[str] = []
    checked = 0

    for path in paths:
        for line, module, name, arity in call_sites(path):
            # `self::` style intra-module calls resolve against their own file.
            if module == path.stem:
                continue
            checked += 1
            table = defs.get(module, {})
            if name not in table:
                problems.append(
                    f"{path.name}:{line}: {module}::{name} is not defined in {module}.rs"
                )
            elif arity not in table[name]:
                expected = "/".join(str(a) for a in sorted(table[name]))
                problems.append(
                    f"{path.name}:{line}: {module}::{name} called with {arity} "
                    f"arg(s), defined with {expected}"
                )

    print(f"pass 1: {checked} cross-module call sites")
    calls_bad = len(problems)
    for problem in problems:
        print(f"  MISMATCH {problem}")
    if not calls_bad:
        print("  no unresolved names, no arity mismatches")

    constants, type_members, owner = symbol_table(paths)
    items: list[str] = []
    for path in paths:
        items.extend(item_problems(path, constants, type_members, owner))
    print(
        f"pass 2: {len(constants)} modules, {len(type_members)} types "
        f"({sum(len(v) for v in constants.values())} constants)"
    )
    for problem in items:
        print(f"  MISMATCH {problem}")
    if not items:
        print("  no unknown constants, variants or struct fields")

    tests: list[str] = []
    for path in paths:
        tests.extend(test_reference_problems(path))
    modules_with_tests = sum(
        1 for path in paths if TEST_MOD_RE.search(strip_strings_and_comments(path.read_text()))
    )
    print(f"pass 3: {modules_with_tests} test modules")
    for problem in tests:
        print(f"  UNRESOLVED {problem}")
    if not tests:
        print("  every test reference resolves in its own module")

    return 1 if (problems or items or tests) else 0


if __name__ == "__main__":
    sys.exit(main())
