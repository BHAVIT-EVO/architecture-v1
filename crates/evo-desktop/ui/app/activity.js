// Activity, permissions and data — the trust surface.
//
// Everything shown here comes from Rust (docs/BRIDGE.md): the permission set,
// the kill switch and the activity log are read with `get_privacy`, changed
// with `set_permission_mode` / `set_scope` / `pause` / `resume`, and undone
// with `undo_activity`. This file decides nothing and enforces nothing; it
// asks, and paints the answer.
//
// In a normal browser there is no backend, so the sheet says that instead of
// inventing entries.

import { api, isApp } from "./bridge.js";
import { iconSvgPath, iconSvg } from "./card-view.js";

const LOCAL = "evo.activity";

const el = (tag, props = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") n.className = v;
    else if (k.startsWith("on")) n.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) n.setAttribute(k, v === true ? "" : v);
  }
  for (const kid of kids.flat()) if (kid != null) n.append(kid.nodeType ? kid : document.createTextNode(kid));
  return n;
};

const read = (key, fallback) => {
  try { return JSON.parse(localStorage.getItem(key) || "null") ?? fallback; }
  catch { return fallback; }
};

// ── the kill switch, as this page last heard it from Rust ────────────────
let paused = false;
export const isPaused = () => paused;

/** Asks Rust for the authoritative state. The page never guesses it. */
export async function refreshStatus() {
  try {
    const status = await api.status();
    paused = Boolean(status?.paused);
  } catch {
    paused = false;
  }
  return paused;
}

/** Asks Rust to flip the switch; the answer is what the page believes. */
export async function togglePause() {
  const status = paused ? await api.resume() : await api.pause();
  paused = Boolean(status?.paused);
  return paused;
}

/** Rust records actions itself. Only the browser preview keeps a local log. */
export function logEvent({ what, why }) {
  if (isApp) return;
  const events = read(LOCAL, []);
  events.unshift({ what, why, at: Date.now() });
  localStorage.setItem(LOCAL, JSON.stringify(events.slice(0, 100)));
}

const when = (at) => {
  if (!at) return "Just now";
  const secs = Math.max(0, Math.round(Date.now() / 1000 - at));
  if (secs < 60) return "Just now";
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} h ago`;
  return `${Math.round(hours / 24)} d ago`;
};

const bytes = (n) => {
  if (!n) return "0 KB";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
};

const sheet = (title) => {
  const body = el("div", { class: "body" });
  const close = el("button", { class: "icon-button", "aria-label": "Close", onclick: () => scrim.remove() },
    iconSvgPath(document, "M5 5l8 8M13 5l-8 8"));
  const scrim = el("div", { class: "scrim" },
    el("div", { class: "sheet", role: "dialog", "aria-label": title },
      el("header", {}, el("h2", {}, title), el("span", { class: "sp" }), close),
      body));
  scrim.addEventListener("click", (e) => { if (e.target === scrim) scrim.remove(); });
  return { scrim, body };
};

const modeLabel = { ask: "Ask every time", risky: "Ask for risky actions", scope: "Act within scope" };

export function openActivity({ onChange } = {}) {
  const { scrim, body } = sheet("Activity");
  const tabs = ["Activity", "Permissions", "Data"];
  const seg = el("div", { class: "segmented", role: "tablist" });
  const buttons = tabs.map((t) =>
    el("button", { role: "tab", "aria-pressed": String(t === "Activity"), onclick: () => show(t) }, t));
  buttons.forEach((b) => seg.append(b));
  scrim.querySelector(".sheet").insertBefore(el("div", { class: "tabs" }, seg), body);

  let privacy = null;
  let failed = null;
  // A tab that was switched away from while it was reading must not paint
  // into the next tab's body.
  let token = 0;
  const current = (mine) => mine === token;

  async function load() {
    try {
      privacy = await api.privacy();
      failed = null;
    } catch (error) {
      failed = error.message;
      privacy = null;
    }
    paused = Boolean(privacy?.paused);
    return privacy;
  }

  function show(tab) {
    token += 1;
    const mine = token;
    buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.textContent === tab)));
    body.replaceChildren();
    if (tab === "Activity") activityTab(mine);
    if (tab === "Permissions") permissionsTab(mine);
    if (tab === "Data") dataTab(mine);
  }

  function note(text) {
    return el("p", { class: "none", style: "color:var(--muted);padding:14px 2px;font-size:13px" }, text);
  }

  function pauseButton() {
    const on = paused;
    return el("button", {
      class: `pause-all${on ? " is-paused" : ""}`,
      onclick: async () => {
        try {
          await togglePause();
          await load();
          show("Activity");
          onChange?.();
        } catch (error) {
          body.prepend(note(error.message));
        }
      },
    },
      el("span", {}, on
        ? iconSvgPath(document, "M7 5v10l8-5z", 18)
        : iconSvgPath(document, "M8 5v10M14 5v10", 18)),
      el("span", {}, el("strong", {}, on ? "Resume everything" : "Pause everything"),
        el("small", {}, on
          ? "Capture and voice are off. Resume when you're ready."
          : "Stops capture, listening and every action, everywhere.")));
  }

  async function activityTab(mine = token) {
    body.append(pauseButton());
    body.append(el("button", {
      class: "btn-ghost",
      style: "height:34px;margin:10px 0",
      onclick: async () => {
        try {
          await api.putThingsBack();
          onChange?.();
          show("Activity");
        } catch (error) {
          body.prepend(note(error.message));
        }
      },
    }, "Put things back"));
    body.append(note("Reading…"));
    const data = await load();
    if (!current(mine)) return;
    body.lastChild.remove();
    if (!data) {
      if (isApp) body.append(note(`The store didn't answer. ${failed ?? ""}`.trim()));
      else body.append(note("Activity is kept by the Evo app; this preview cannot read it."));
      return;
    }
    const events = data.activity ?? [];
    if (!events.length) {
      body.append(note("Nothing yet. What Evo does appears here, with the reason."));
      return;
    }
    for (const event of events) {
      body.append(el("div", { class: "row-line" },
        el("span", {}, el("strong", {}, event.action), el("small", {}, `${when(event.at)}, ${event.reason}`)),
        el("span", { class: "side" }, event.undo
          ? el("button", {
              class: "btn-ghost",
              style: "height:32px;padding:0 14px",
              onclick: async (e) => {
                e.currentTarget.disabled = true;
                try {
                  await api.undoActivity(event.at);
                  await load();
                  show("Activity");
                  onChange?.();
                } catch (error) {
                  e.currentTarget.disabled = false;
                  body.prepend(note(error.message));
                }
              },
            }, "Undo")
          : null)));
    }
  }

  async function permissionsTab(mine = token) {
    body.append(note("Reading…"));
    const data = await load();
    if (!current(mine)) return;
    body.lastChild.remove();
    if (!data) {
      body.append(note(isApp ? "The store didn't answer." : "Permissions live in the app's settings store."));
      return;
    }
    const modes = [
      ["ask", "Ask every time", "Evo asks before it reopens anything."],
      ["risky", "Ask for risky actions", "Quiet work goes ahead; anything that changes files asks."],
      ["scope", "Act within scope", "Evo acts on what is allowed below without asking."],
    ];
    body.append(el("p", { class: "section-label" }, "Mode"));
    const list = el("div", { class: "modes", role: "radiogroup", "aria-label": "Permission mode" });
    for (const [id, name, hint] of modes) {
      list.append(el("button", {
        class: "mode", role: "radio", "aria-checked": String(data.mode === id),
        onclick: async () => {
          try {
            await api.setMode(id);
            await load();
            show("Permissions");
            onChange?.();
          } catch (error) {
            body.prepend(note(error.message));
          }
        },
      }, el("span", { class: "dot" }), el("span", {}, name, el("small", {}, hint))));
    }
    body.append(list);

    const groups = [
      ["Folders", data.scopes.filter((s) => s.kind === "folder")],
      ["Apps", data.scopes.filter((s) => s.kind === "app")],
      ["Sites", data.scopes.filter((s) => s.kind === "site")],
    ];
    for (const [label, items] of groups) {
      body.append(el("p", { class: "section-label" }, label));
      for (const scope of items) {
        body.append(el("div", { class: "row-line" },
          el("span", {}, el("strong", {}, scope.value), el("small", {}, scope.allowed ? "Evo can see this" : "Off")),
          el("span", { class: "side" },
            el("button", {
              class: "switch", role: "switch", "aria-checked": String(scope.allowed),
              onclick: async () => {
                try {
                  await api.setScope(scope.kind, scope.value, !scope.allowed);
                  await load();
                  show("Permissions");
                  onChange?.();
                } catch (error) {
                  body.prepend(note(error.message));
                }
              },
            }, el("span", { class: "track" })))));
      }
    }
  }

  async function dataTab(mine = token) {
    const data = await load();
    if (!current(mine)) return;
    const where = data ? `${data.storage.path}, ${bytes(data.storage.bytes)}` : "unavailable";

    const result = el("p", { class: "why", hidden: true, style: "color:var(--muted);font-size:12.5px" });
    body.append(el("div", { class: "row-line" },
      el("span", {}, el("strong", {}, "Export your data"),
        el("small", {}, `A JSON file with your works, settings and activity. Stored in ${where}.`)),
      el("span", { class: "side" },
        el("button", {
          class: "btn-ghost", style: "height:36px",
          onclick: async (e) => {
            e.currentTarget.disabled = true;
            try {
              const copy = await api.exportData();
              result.hidden = false;
              result.textContent = `Wrote ${copy.path}, ${bytes(copy.bytes)}, ${copy.works} works`;
            } catch (error) {
              result.hidden = false;
              result.textContent = error.message;
            } finally {
              e.currentTarget.disabled = false;
            }
          },
        }, "Export"))));
    body.append(result);

    const deleted = el("p", { class: "why", hidden: true, style: "color:var(--muted);font-size:12.5px" });
    body.append(el("div", { class: "row-line" },
      el("span", {}, el("strong", {}, "Delete everything"),
        el("small", {}, "Removes the ledger, the settings and the activity from this Mac.")),
      el("span", { class: "side" },
        el("button", {
          class: "btn-ghost danger", style: "height:36px",
          onclick: async (e) => {
            const button = e.currentTarget;
            if (!button.dataset.armed) {
              button.dataset.armed = "1";
              button.textContent = "Delete everything?";
              setTimeout(() => { delete button.dataset.armed; button.textContent = "Delete"; }, 4000);
              return;
            }
            button.disabled = true;
            try {
              const gone = await api.deleteAll();
              deleted.hidden = false;
              deleted.textContent = `Deleted ${gone.removed.length} files. quit and reopen Evo.`;
              onChange?.();
            } catch (error) {
              deleted.hidden = false;
              deleted.textContent = error.message;
              button.disabled = false;
            }
          },
        }, "Delete"))));
    body.append(deleted);
  }

  show("Activity");
  document.body.append(scrim);
  const onKey = (e) => { if (e.key === "Escape") { document.removeEventListener("keydown", onKey, true); scrim.remove(); } };
  document.addEventListener("keydown", onKey, true);
  return { close: () => scrim.remove(), show };
}
