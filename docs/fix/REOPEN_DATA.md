# Reopen data audit

This audit uses a copy of the real storage at
`EVO_TEST_HOME=/Users/bhavitsaini/.copilot/session-state/78d577e8-721d-451b-a333-a1b26d74490e/r1-test-home`.
The original `~/Library/Application Support/Evo` was read only and was not
modified.

The six newest rows in the copied `ledger.db` are ordered by `end_ms DESC`.
For URL rows, “exists” means a non-empty URL target was stored; this audit does
not make a network request. For file rows, existence was checked with `test -e`.

| Work ID | URL | Path | App | Window/title | Target exists |
|---:|---|---|---|---|---|
| 1 | — | — | Freebuff | Freebuff Desktop | no: no target |
| 97 | — | `/Users/bhavitsaini/Downloads/generate_unhinged.py` | zcode | Downloaded: generate_unhinged.py | yes: path exists |
| 96 | — | — | zcode | ZCode | no: no target |
| 95 | `https://chatgpt.com/g/g-p-69cbe7237a088191b24cdfa70bb6e6ea/c/6a60fc1c-6cc8-83e8-81b6-1006452af89e` | — | google chrome | evo - Evo Architecture Design | yes: URL stored |
| 94 | — | — | antigravity ide | Antigravity IDE | no: no target |
| 93 | `https://claude.ai/chat/f5c116b5-5fea-4f64-bfda-ef61c96c886e` | — | google chrome | [ai-chat] Complete kundli analysis and remedies | yes: URL stored |

## Capture conclusion

The copied data contains openable file and URL targets. It contains no
openable target for app-only/window-only work (IDs 1, 96, and 94): those rows
have an app name and title but no URL or path. Capture must record the
application bundle identifier or bundle path for app work, and should retain
the exact window title separately for optional window restoration. A title
alone is not a launch target.

The file target in ID 97 still exists in this audit copy. A moved-file fixture
must therefore be tested separately; it is not evidence that the stored real
file is currently missing.

## Evidence

```text
SOURCE: /Users/bhavitsaini/Library/Application Support/Evo
EVO_TEST_HOME: /Users/bhavitsaini/.copilot/session-state/78d577e8-721d-451b-a333-a1b26d74490e/r1-test-home
PATH_EXISTS /Users/bhavitsaini/Downloads/generate_unhinged.py
```

The storage schema used for this report is `intervals`, with
`task_key_app`, `task_key_title`, `task_key_doc`, `urls_json`, and
`titles_json`. No source files or original storage were changed.
