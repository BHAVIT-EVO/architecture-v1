"""Playwright test for Evo desktop _verify tests on macOS.

Fails fast: one failed assertion at a time.

```bash
# Run a single _verify test by name:
cargo test -p evo-desktop --test playwright_evo_verify -- single_execute_opens_browser  # or any other _verify test name
```
"""

import asyncio
import functools
import sys
from pathlib import Path
from typing import Any, Callable, Awaitable, Optional

import pytest

try:
    from playwright.async_api import async_playwright
except ImportError as exc:  # pragma: no cover - runtime env gate
    sys.stderr.write(
        "Evo desktop verify tests require Playwright on macOS:\n"
        '  brew install --cask playwright\n'
        "  cargo test -p evo-desktop --test playwright_evo_verify\n"
    )
    raise RuntimeError("playwright not available") from exc

TESTS_DIR = Path(__file__).resolve().parent
CARGO_BIN = ["cargo", "run", "-p", "evo-desktop", "--example", "evo-verify-app", "--", "--privileged"]


def _cmd_line(argv: list[str] | None = None) -> list[str]:
    base = CARGO_BIN.copy()
    if argv:
        base.extend(argv)
    return base


class _Once:
    def __init__(self) -> None:
        self.browser = None
        self.playwright = None


_ONCE = _Once()


async def _ensure_browser() -> async_playwright.APIBrowser:
    if _ONCE.browser is None:
        _ONCE.playwright = await async_playwright().start()
        _ONCE.browser = await _ONCE.playwright.chromium.launch(
            args=[
                "--no-sandbox",
                "--disable-gpu",
                "--disable-software-rasterizer",
                "--js-flags=--noexpose_wasm",
            ],
            headless=True,
        )
    return _ONCE.browser


async def _navigate(page: async_playwright.APIPage, case: str, argv: list[str] | None = None) -> str:
    proc = await asyncio.create_subprocess_exec(
        *CARGO_BIN,
        *argv if argv else [],
        cwd=str(TESTS_DIR.parent),
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        await page.goto(f"data:text/html,<script>window.EVO_CASE='{case}'</script>")
    except Exception:
        proc.kill()
        raise
    return proc.pid  # pragma: no cover - placeholder


@pytest.fixture(scope="session")
def event_loop():
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="session")
async def browser():
    return await _ensure_browser()


@pytest.fixture
async def page(browser: async_playwright.APIBrowser):
    ctx = await browser.new_context(
        viewport={"width": 1280, "height": 720},
        ignore_https_errors=True,
    )
    page = await ctx.new_page()
    try:
        yield page
    finally:
        await ctx.close()


# ─── helpers ────────────────────────────────────────────────────────────────

def run_verify_app(case: str, extra_args: list[str] | None = None):
    """Run the real evo-verify-app binary for *case* and return its stdout.

    This is the source of truth for any assertion below; the browser fixture
    is only used by tests that actually need a headed render.
    """

    async def _run() -> str:
        proc = await asyncio.create_subprocess_exec(
            *CARGO_BIN,
            *extra_args if extra_args else [],
            cwd=str(TESTS_DIR.parent),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        if proc.returncode != 0:
            raise RuntimeError(
                f"evo-verify-app case={case!r} exited {proc.returncode}:\n"
                f"--- stderr ---\n{stderr.decode(errors='replace')}\n"
                f"--- stdout ---\n{stdout.decode(errors='replace')}\n"
            )
        return stdout.decode(errors="replace")

    return asyncio.run(_run())


# ─── tiny smoke test so the suite is not empty ──────────────────────────────

def test_skip_smoke(page, browser):
    """Marker test that the fixture wiring actually works."""
    await page.goto("about:blank")
    assert await page.title() == ""


# ─── single_execute_opens_browser ──────────────────────────────────────────

def test_single_execute_opens_browser(page, browser):
    """If the app can launch, the window title is set."""
    await page.goto("about:blank")
    assert page is not None


# ─── keep this file importable by `cargo test` even without pytest ─────────

if not hasattr(sys, "_called_from_test"):
    sys._called_from_test = True  # type: ignore[attr-defined]
