// Activity, permissions and data — the trust surface. Rows say what happened,
// when, and why; anything reversible offers Undo.

const STORE = "evo.activity";
const PERMS = "evo.permissions";
const PAUSED = "evo.paused";

const el = (tag, props = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") n.className = v;
    else if (k === "html") n.innerHTML = v;
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

/** Records one thing Evo did, so Activity can show it honestly. */
export function logEvent({ what, why, undo }) {
  const events = read(STORE, []);
  events.unshift({ what, why, undo, at: Date.now() });
  localStorage.setItem(STORE, JSON.stringify(events.slice(0, 100)));
}

export function undoEvent(event) {
  if (!event.undo) return false;
  const overrides = read("evo.home.overrides", { hidden: [], renamed: {}, merged: [] });
  const { kind, id } = event.undo;
  if (kind === "hide") overrides.hidden = (overrides.hidden ?? []).filter((x) => x !== id);
  if (kind === "merge") overrides.merged = (overrides.merged ?? []).filter((x) => x !== id);
  if (kind === "rename") delete overrides.renamed[id];
  localStorage.setItem("evo.home.overrides", JSON.stringify(overrides));
  // Rows are re-read from storage, so match on the identity fields, not the object.
  const events = read(STORE, []).filter((e) => !(e.at === event.at && e.what === event.what));
  localStorage.setItem(STORE, JSON.stringify(events));
  return true;
}

export function isPaused() {
  return localStorage.getItem(PAUSED) === "1";
}

const when = (at) => {
  const mins = Math.round((Date.now() - at) / 60_000);
  if (mins < 1) return "Just now";
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} h ago`;
  return `${Math.round(hours / 24)} d ago`;
};

const sheet = (title) => {
  const body = el("div", { class: "body" });
  const close = el("button", { class: "icon-button", "aria-label": "Close", onclick: () => scrim.remove() },
    el("span", { html: '<svg width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M5 5l8 8M13 5l-8 8"/></svg>' }));
  const scrim = el("div", { class: "scrim" },
    el("div", { class: "sheet", role: "dialog", "aria-label": title },
      el("header", {}, el("h2", {}, title), el("span", { class: "sp" }), close),
      body));
  scrim.addEventListener("click", (e) => { if (e.target === scrim) scrim.remove(); });
  return { scrim, body };
};

export function openActivity({ works, onResume }) {
  const { scrim, body } = sheet("Activity");
  const tabs = ["Activity", "Permissions", "Data"];
  const tabRow = el("div", { class: "tabs" });
  const seg = el("div", { class: "segmented", role: "tablist" });
  const buttons = tabs.map((t) =>
    el("button", { role: "tab", "aria-pressed": String(t === "Activity"), onclick: () => show(t) }, t));
  buttons.forEach((b) => seg.append(b));
  tabRow.append(seg);
  scrim.querySelector(".sheet").insertBefore(tabRow, body);

  function show(tab) {
    buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.textContent === tab)));
    body.replaceChildren();
    if (tab === "Activity") activityTab();
    if (tab === "Permissions") permissionsTab();
    if (tab === "Data") dataTab();
  }

  function pauseButton() {
    const paused = isPaused();
    return el("button", {
      class: `pause-all${paused ? " is-paused" : ""}`,
      onclick: () => {
        const next = !isPaused();
        localStorage.setItem(PAUSED, next ? "1" : "0");
        onResume?.(!next);
        show("Activity");
      },
    },
      el("span", { html: paused
        ? '<svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M7 5v10l8-5z"/></svg>'
        : '<svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M8 5v10M14 5v10"/></svg>' }),
      el("span", {}, el("strong", {}, paused ? "Resume everything" : "Pause everything"),
        el("small", {}, paused
          ? "Capture and voice are off. Resume when you're ready."
          : "Stops capture, listening and every action, everywhere.")));
  }

  function activityTab() {
    body.append(pauseButton());
    const events = read(STORE, []);
    if (!events.length) {
      body.append(el("p", { class: "none", style: "color:var(--muted);padding:18px 2px" },
        "Nothing yet. What Evo does appears here, with the reason."));
      return;
    }
    for (const event of events) {
      body.append(el("div", { class: "row-line" },
        el("span", {}, el("strong", {}, event.what), el("small", {}, `${when(event.at)} · ${event.why}`)),
        el("span", { class: "side" }, event.undo
          ? el("button", {
              class: "btn-ghost",
              style: "height:32px;padding:0 14px",
              onclick: () => { undoEvent(event); show("Activity"); onResume?.(true); },
            }, "Undo")
          : null)));
    }
  }

  function permissionsTab() {
    const saved = read(PERMS, { mode: "risky", scopes: {} });
    const modes = [
      ["ask", "Ask every time", "Evo asks before it reopens anything."],
      ["risky", "Ask for risky actions", "Quiet work goes ahead; anything that changes files asks."],
      ["scope", "Act within scope", "Evo acts on what is allowed below without asking."],
    ];
    body.append(el("p", { class: "section-label" }, "Mode"));
    const list = el("div", { class: "modes", role: "radiogroup", "aria-label": "Permission mode" });
    for (const [id, name, hint] of modes) {
      list.append(el("button", {
        class: "mode", role: "radio", "aria-checked": String(saved.mode === id),
        onclick: () => { saved.mode = id; localStorage.setItem(PERMS, JSON.stringify(saved)); show("Permissions"); },
      }, el("span", { class: "dot" }), el("span", {}, name, el("small", {}, hint))));
    }
    body.append(list);

    const groups = [
      ["Folders", ["Documents", "Desktop", "Downloads"]],
      ["Apps", ["Claude", "Chrome", "Finder", "Terminal"]],
      ["Sites", ["claude.ai", "youtube.com", "github.com"]],
    ];
    for (const [label, items] of groups) {
      body.append(el("p", { class: "section-label" }, label));
      for (const item of items) {
        const on = saved.scopes[item] ?? item !== "Downloads";
        body.append(el("div", { class: "row-line" },
          el("span", {}, el("strong", {}, item), el("small", {}, on ? "Evo can see this" : "Off")),
          el("span", { class: "side" },
            el("button", {
              class: "switch", role: "switch", "aria-checked": String(on),
              onclick: () => {
                saved.scopes[item] = !on;
                localStorage.setItem(PERMS, JSON.stringify(saved));
                show("Permissions");
              },
            }, el("span", { class: "track" })))));
      }
    }
  }

  function dataTab() {
    body.append(el("div", { class: "row-line" },
      el("span", {}, el("strong", {}, "Export your history"),
        el("small", {}, "A JSON file with your works, settings and activity.")),
      el("span", { class: "side" },
        el("button", {
          class: "btn-ghost", style: "height:36px",
          onclick: () => {
            const dump = {
              exported: new Date().toISOString(),
              works: read("evo.home.overrides", {}),
              settings: read("evo.settings", {}),
              permissions: read(PERMS, {}),
              activity: read(STORE, []),
            };
            const blob = new Blob([JSON.stringify(dump, null, 2)], { type: "application/json" });
            const a = el("a", { href: URL.createObjectURL(blob), download: "evo-history.json" });
            document.body.append(a);
            a.click();
            a.remove();
          },
        }, "Export"))));

    body.append(el("div", { class: "row-line" },
      el("span", {}, el("strong", {}, "Delete everything"),
        el("small", {}, "Removes your works, settings, permissions and activity from this Mac.")),
      el("span", { class: "side" },
        el("button", {
          class: "btn-ghost danger", style: "height:36px",
          onclick: (e) => {
            const button = e.currentTarget;
            if (button.dataset.armed) {
              localStorage.clear();
              location.reload();
              return;
            }
            button.dataset.armed = "1";
            button.textContent = "Delete everything?";
            setTimeout(() => { delete button.dataset.armed; button.textContent = "Delete"; }, 4000);
          },
        }, "Delete"))));
  }

  show("Activity");
  document.body.append(scrim);
  const onKey = (e) => { if (e.key === "Escape") { document.removeEventListener("keydown", onKey, true); scrim.remove(); } };
  document.addEventListener("keydown", onKey, true);
  return { close: () => scrim.remove(), show };
}
