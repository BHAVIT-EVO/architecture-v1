// The tray menu and the four state panels: empty, error, offline, denied.
// Loading is drawn by the scene (pale outline objects), never a spinner.

import { STATES, TRAY } from "./copy.js";

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

let tray = null;

export function closeTray() {
  tray?.remove();
  tray = null;
}

export function openTray({ anchor, listening, paused, onPause, onOpen, onHelp, onDiagnostics }) {
  closeTray();
  const items = [
    [TRAY.status(listening), null],
    [paused ? TRAY.resume : TRAY.pause, onPause],
    [TRAY.open, onOpen],
    [TRAY.help, onHelp],
    [TRAY.diagnostics, onDiagnostics],
    [TRAY.quit, null, TRAY.quitHint],
  ];
  tray = el("div", { class: "tray-menu", role: "menu", "aria-label": "Evo menu" });
  for (const [label, action, hint] of items) {
    if (!action) {
      tray.append(el("div", { class: "tray-item is-static", role: "presentation" },
        el("strong", {}, label), hint ? el("small", {}, hint) : null));
      continue;
    }
    tray.append(el("button", { class: "tray-item", role: "menuitem", onclick: () => { closeTray(); action(); } }, label));
  }
  const rect = anchor.getBoundingClientRect();
  tray.style.left = `${Math.round(rect.left)}px`;
  tray.style.top = `${Math.round(rect.bottom + 8)}px`;
  document.body.append(tray);
  setTimeout(() => document.addEventListener("click", closeTray, { once: true }), 0);
}

/** Replaces the Home card with one of the four states. */
export function showState(kind, { onAction } = {}) {
  clearState();
  const copy = STATES[kind];
  const panel = el("div", { class: `panel-card state-panel state-${kind}`, role: "status" },
    el("h2", {}, copy.title),
    copy.body ? el("p", {}, copy.body) : null,
    copy.action ? el("button", { class: "btn-primary", onclick: onAction }, copy.action) : null);
  document.querySelector(".left").prepend(panel);
  document.body.dataset.state = kind;
  return panel;
}

export function clearState() {
  document.querySelector(".state-panel")?.remove();
  delete document.body.dataset.state;
}
