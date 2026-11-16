# ADR 0002 — The web UI shell

**Status:** accepted (S9-1). Supersedes ADR 0001 (native egui shell).

## Context

The new interface was built in HTML/CSS/JS because the design law is stated in
those terms (a mockup, a 1100×806 pixel diff, bundled fonts). Porting it to egui
would either drift from the measured design or cost a second implementation of
every screen. WKWebView already exists in the app: `websurface.rs` renders a
work's own pages inside the window, with per-work data stores.

## Decision

The app's main window hosts the new HTML UI in a WKWebView.

- The page is `evo-app://app/app/home.html`, served by a `WKURLSchemeHandler`
  whose root is `Contents/Resources/ui`. Nothing is read from `file://` or the
  network.
- Navigation policy lives in `webshell::navigation_allowed`: only
  `evo-app://app/…` is allowed; `https:`, `http:`, `file:`, `javascript:` and
  `data:` are refused. It is enforced today by the scheme handler (which has no
  way to serve another scheme) and by `ui/app/shell.js` in the page (link
  clicks, form submits and `window.open`). A `WKNavigationDelegate` was
  implemented and **removed**: on this macOS, WebKit 26.4 crashes inside
  `NavigationClient::decidePolicyForNavigationAction` when a hand-built
  delegate implements that method, twice, with crash reports. Until that is
  solved with a supported binding, a JS-initiated `location` change to
  `https:` is not blocked. This is a known gap, not a claim.
- CSP is declared in the page (`default-src 'self'; script-src 'self'`) and no
  script is inline, so `eval` is never permitted.
- Reality enters through one bridge: the shell injects `window.evo.works` from
  `evo_daemon::threads::display_threads_from_storage` as a document-start user
  script. The page falls back to its local sample corpus only when that object
  is absent (a plain browser).
- The eframe window still exists and owns the title bar, traffic lights and
  window behaviour; a 28 px native drag strip above the web view makes the top
  band a drag region. `-webkit-app-region` is not used: WebKit does not
  implement it.
- The egui interface stays compiled and remains the default. `EVO_UI=web`
  selects the web shell; nothing is deleted before the swap is approved.

## Security model

| Boundary | Rule |
|---|---|
| Read access | only files under the canonicalised `ui` root; `..`, percent-encoded traversal and symlinks that leave the root are refused |
| Navigation | only the `evo-app` scheme with host `app`; everything else cancelled before load |
| Scripts | no inline script, no `eval`, `script-src 'self'` |
| Remote content | none: no remote URL in any asset (`grep` in CI-style check), `connect-src 'none'` |
| Window | `window.open` blocked; web inspector enabled only in debug builds |
| Backend | Rust stays the only enforcer of permissions, paths and the kill switch; the page cannot call anything but the injected read-only snapshot |

## Rollback

`EVO_UI` defaults to `native`, so the shipped app is unchanged until the switch
is flipped. The web shell is additive: deleting `webshell.rs`, its one `if` in
`main.rs`, and `crates/evo-desktop/ui/` returns the app to its pre-S9 state with
no other edits. The `evo-app` scheme is installed only on the configuration of
the window that asks for it.
